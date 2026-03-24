//! AsciiDoc parser.

use crate::ast::{
    AsciiDoc, Block, DefinitionItem, Diagnostic, ImageData, Inline, Span,
};
use crate::events::{collect_block_events, OwnedEvent};
use std::collections::VecDeque;

// ── Public API ────────────────────────────────────────────────────────────────

/// Parse an AsciiDoc string into an [`AsciiDoc`] document.
///
/// Parsing is always infallible: malformed constructs produce diagnostics
/// rather than errors.
pub fn parse(input: &str) -> (AsciiDoc, Vec<Diagnostic>) {
    let mut iter = EventIter::new(input);
    let (blocks, attributes, diagnostics) = crate::events::collect_doc_from_iter(&mut iter);
    let doc = AsciiDoc { blocks, attributes, span: Span::NONE };
    (doc, diagnostics)
}

// ── Parser ────────────────────────────────────────────────────────────────────

pub struct EventIter<'a> {
    lines: Vec<&'a str>,
    line_idx: usize,
    pub(crate) attributes: std::collections::HashMap<String, String>,
    pub(crate) diagnostics: Vec<Diagnostic>,
    /// Pending block id from `[#id]` — applied to the next block and cleared.
    pending_id: Option<String>,
    /// Pending block role from `[.role]` — applied to the next block and cleared.
    pending_role: Option<String>,
    /// Pending list style from `[loweralpha]` etc. — applied to the next list block and cleared.
    pending_list_style: Option<String>,
    // ── Iterator state ────────────────────────────────────────────────────────
    pub(crate) event_buf: VecDeque<OwnedEvent>,
    iter_started: bool,
    iter_done: bool,
}

impl<'a> EventIter<'a> {
    pub fn new(input: &'a str) -> Self {
        let lines: Vec<&str> = input.lines().collect();
        Self {
            lines,
            line_idx: 0,
            attributes: std::collections::HashMap::new(),
            diagnostics: Vec::new(),
            pending_id: None,
            pending_role: None,
            pending_list_style: None,
            event_buf: VecDeque::new(),
            iter_started: false,
            iter_done: false,
        }
    }

    fn current_line(&self) -> Option<&'a str> {
        self.lines.get(self.line_idx).copied()
    }

    pub(crate) fn advance_line(&mut self) {
        self.line_idx += 1;
    }

    pub(crate) fn is_eof(&self) -> bool {
        self.line_idx >= self.lines.len()
    }

    fn is_blank_line(&self) -> bool {
        self.current_line()
            .map(|l| l.trim().is_empty())
            .unwrap_or(true)
    }

    pub(crate) fn skip_blank_lines(&mut self) {
        while !self.is_eof() && self.is_blank_line() {
            self.advance_line();
        }
    }

    pub(crate) fn take_attributes(&mut self) -> std::collections::HashMap<String, String> {
        std::mem::take(&mut self.attributes)
    }

    pub(crate) fn try_parse_block(&mut self) -> Option<Block> {
        // Skip document attribute lines (:attr: value) iteratively to avoid
        // stack overflow on documents with many consecutive attribute lines.
        loop {
            let line = self.current_line()?;
            if line.starts_with(':') && line.len() > 1 {
                if let Some((attr, value)) = self.try_parse_attribute(line) {
                    self.attributes.insert(attr, value);
                    self.advance_line();
                    continue;
                }
            }
            break;
        }
        let line = self.current_line()?;

        // Section titles (= Title, == Title, etc.)
        if line.starts_with('=') {
            if let Some(heading) = self.try_parse_heading() {
                return Some(heading);
            }
        }

        // Block attributes/admonitions ([NOTE], [source,lang], etc.)
        if line.starts_with('[') && line.ends_with(']') {
            return self.parse_block_with_attributes();
        }

        // Delimited blocks
        if self.is_delimiter_line(line) {
            return self.parse_delimited_block();
        }

        // Table (|=== delimiter)
        if line == "|===" {
            return self.parse_table();
        }

        // Open block (--)
        if line == "--" {
            return self.parse_open_block();
        }

        // Block title (.Title before a block) — but not `. item` ordered list markers
        if line.starts_with('.')
            && line.len() > 1
            && !line.starts_with("..")
            && !line.starts_with(". ")
        {
            return self.parse_block_title();
        }

        // Lists
        if let Some(list) = self.try_parse_list() {
            return Some(list);
        }

        // Horizontal rule
        if line == "'''" || line == "---" || line == "***" {
            self.advance_line();
            return Some(Block::HorizontalRule { span: Span::NONE });
        }

        // Page break
        if line == "<<<" {
            self.advance_line();
            return Some(Block::PageBreak { span: Span::NONE });
        }

        // Block image (image::path[alt])
        if line.starts_with("image::") {
            return self.parse_block_image();
        }

        // Regular paragraph
        self.parse_paragraph()
    }

    fn try_parse_attribute(&self, line: &str) -> Option<(String, String)> {
        if !line.starts_with(':') {
            return None;
        }

        let rest = &line[1..];
        if let Some(colon_idx) = rest.find(':') {
            let attr_name = rest[..colon_idx].trim();
            let value = rest[colon_idx + 1..].trim();

            // Attribute unset (:!attr:)
            if let Some(stripped) = attr_name.strip_prefix('!') {
                return Some((stripped.to_string(), String::new()));
            }

            return Some((attr_name.to_string(), value.to_string()));
        }

        None
    }

    fn try_parse_heading(&mut self) -> Option<Block> {
        let line = self.current_line()?;

        // Count leading = characters
        let level = line.chars().take_while(|&c| c == '=').count();

        // Must have at least one = and be followed by space
        if level == 0 || level > 6 {
            return None;
        }

        let rest = &line[level..];
        // Headings must have a space after the `=` characters: `== Title`.
        // `====` (all `=`, no space) is a delimiter block, not a heading.
        if !rest.starts_with(' ') {
            return None;
        }

        let title = rest.trim();
        self.advance_line();

        let inlines = parse_inline_content(title);
        let id = self.pending_id.take();
        let role = self.pending_role.take();
        Some(Block::Heading {
            level,
            inlines,
            id,
            role,
            span: Span::NONE,
        })
    }

    fn parse_block_with_attributes(&mut self) -> Option<Block> {
        let line = self.current_line()?;

        // Parse attributes inside [...]
        let attrs_content = &line[1..line.len() - 1];
        let attrs: Vec<&str> = attrs_content.split(',').map(|s| s.trim()).collect();

        self.advance_line();

        // Check for admonition blocks
        let first_attr = attrs.first().map(|s| s.to_uppercase());

        // Check for block id: [#my-id] or [#my-id, ...] and role: [.role]
        // Also list style: [loweralpha], [upperroman], [arabic], etc.
        let first = attrs.first().map(|s| s.as_ref()).unwrap_or("");

        // Block id: starts with #
        if let Some(id) = first.strip_prefix('#') {
            self.pending_id = Some(id.trim().to_string());
            // May also have role in subsequent attrs (e.g. [#id,.role])
            for attr in attrs.iter().skip(1) {
                if let Some(role) = attr.strip_prefix('.') {
                    self.pending_role = Some(role.trim().to_string());
                }
            }
            return self.try_parse_block();
        }

        // Block role: starts with .
        if let Some(role) = first.strip_prefix('.') {
            self.pending_role = Some(role.trim().to_string());
            for attr in attrs.iter().skip(1) {
                if let Some(id) = attr.strip_prefix('#') {
                    self.pending_id = Some(id.trim().to_string());
                }
            }
            return self.try_parse_block();
        }

        // List style keywords (loweralpha, upperalpha, lowerroman, upperroman, arabic, etc.)
        let list_style_keywords = [
            "arabic",
            "decimal",
            "loweralpha",
            "upperalpha",
            "lowerroman",
            "upperroman",
            "lowergreek",
        ];
        if list_style_keywords.contains(&first.to_lowercase().as_str()) {
            self.pending_list_style = Some(first.to_string());
            return self.try_parse_block();
        }

        match first_attr.as_deref() {
            Some("NOTE") | Some("TIP") | Some("WARNING") | Some("IMPORTANT") | Some("CAUTION") => {
                let admonition_type = first_attr.unwrap().to_lowercase();
                let content = self.collect_paragraph_content();
                let inlines = parse_inline_content(&content);
                Some(Block::Div {
                    class: Some(format!("admonition {}", admonition_type)),
                    title: None,
                    children: vec![Block::Paragraph {
                        inlines,
                        id: None,
                        role: None,
                        checked: None,
                        span: Span::NONE,
                    }],
                    span: Span::NONE,
                })
            }
            Some("SOURCE") => {
                let language = attrs.get(1).map(|s| s.to_string());

                // Expect delimiter
                self.skip_blank_lines();
                if let Some(delim_line) = self.current_line() {
                    if delim_line.starts_with("----") {
                        let content = self.collect_delimited_content("----");
                        return Some(Block::CodeBlock {
                            content,
                            language,
                            span: Span::NONE,
                        });
                    }
                }

                // Fallback: parse as regular content
                let content = self.collect_paragraph_content();
                Some(Block::CodeBlock {
                    content,
                    language,
                    span: Span::NONE,
                })
            }
            Some("QUOTE") | Some("VERSE") => {
                self.skip_blank_lines();
                let content = self.collect_delimited_content_or_paragraph();
                let inlines = parse_inline_content(&content);
                let attribution = attrs.get(1).map(|s| s.to_string());
                Some(Block::Blockquote {
                    children: vec![Block::Paragraph {
                        inlines,
                        id: None,
                        role: None,
                        checked: None,
                        span: Span::NONE,
                    }],
                    attribution,
                    span: Span::NONE,
                })
            }
            Some("EXAMPLE") => {
                self.skip_blank_lines();
                let content = self.collect_delimited_content_or_paragraph();
                let inlines = parse_inline_content(&content);
                Some(Block::Div {
                    class: Some("example".to_string()),
                    title: None,
                    children: vec![Block::Paragraph {
                        inlines,
                        id: None,
                        role: None,
                        checked: None,
                        span: Span::NONE,
                    }],
                    span: Span::NONE,
                })
            }
            Some("SIDEBAR") => {
                self.skip_blank_lines();
                let content = self.collect_delimited_content_or_paragraph();
                let inlines = parse_inline_content(&content);
                Some(Block::Div {
                    class: Some("sidebar".to_string()),
                    title: None,
                    children: vec![Block::Paragraph {
                        inlines,
                        id: None,
                        role: None,
                        checked: None,
                        span: Span::NONE,
                    }],
                    span: Span::NONE,
                })
            }
            _ => {
                // Unknown attribute — skip and try parsing next element
                self.try_parse_block()
            }
        }
    }

    fn is_delimiter_line(&self, line: &str) -> bool {
        if line.len() < 4 {
            return false;
        }

        let patterns = ["----", "====", "****", "____", "++++", "....", "////"];

        for pattern in patterns {
            if line.starts_with(pattern)
                && line.chars().all(|c| c == pattern.chars().next().unwrap())
            {
                return true;
            }
        }

        false
    }

    fn parse_delimited_block(&mut self) -> Option<Block> {
        let line = self.current_line()?;
        let delimiter = line.to_string();
        let delim_char = delimiter.chars().next()?;

        self.advance_line();

        // Collect content until matching delimiter
        let mut content_lines = Vec::new();
        while !self.is_eof() {
            let content_line = self.current_line().unwrap_or("");
            if content_line == delimiter {
                self.advance_line();
                break;
            }
            content_lines.push(content_line);
            self.advance_line();
        }

        let content = content_lines.join("\n");

        // Comment blocks (////) are dropped — content is not emitted
        if delim_char == '/' {
            return None;
        }

        let block = match delim_char {
            '-' => Block::CodeBlock {
                content,
                language: None,
                span: Span::NONE,
            },
            '=' => {
                let inlines = parse_inline_content(&content);
                Block::Div {
                    class: Some("example".to_string()),
                    title: None,
                    children: vec![Block::Paragraph {
                        inlines,
                        id: None,
                        role: None,
                        checked: None,
                        span: Span::NONE,
                    }],
                    span: Span::NONE,
                }
            }
            '*' => {
                let inlines = parse_inline_content(&content);
                Block::Div {
                    class: Some("sidebar".to_string()),
                    title: None,
                    children: vec![Block::Paragraph {
                        inlines,
                        id: None,
                        role: None,
                        checked: None,
                        span: Span::NONE,
                    }],
                    span: Span::NONE,
                }
            }
            '_' => {
                let inlines = parse_inline_content(&content);
                Block::Blockquote {
                    children: vec![Block::Paragraph {
                        inlines,
                        id: None,
                        role: None,
                        checked: None,
                        span: Span::NONE,
                    }],
                    attribution: None,
                    span: Span::NONE,
                }
            }
            '+' => Block::RawBlock {
                format: "asciidoc".to_string(),
                content,
                span: Span::NONE,
            },
            '.' => Block::CodeBlock {
                content,
                language: None,
                span: Span::NONE,
            },
            _ => {
                let inlines = parse_inline_content(&content);
                Block::Div {
                    class: None,
                    title: None,
                    children: vec![Block::Paragraph {
                        inlines,
                        id: None,
                        role: None,
                        checked: None,
                        span: Span::NONE,
                    }],
                    span: Span::NONE,
                }
            }
        };

        Some(block)
    }

    fn collect_delimited_content(&mut self, delimiter: &str) -> String {
        self.advance_line(); // Skip opening delimiter

        let mut lines = Vec::new();
        while !self.is_eof() {
            let line = self.current_line().unwrap_or("");
            if line.starts_with(delimiter)
                && line.chars().all(|c| c == delimiter.chars().next().unwrap())
            {
                self.advance_line();
                break;
            }
            lines.push(line);
            self.advance_line();
        }

        lines.join("\n")
    }

    fn collect_delimited_content_or_paragraph(&mut self) -> String {
        if let Some(line) = self.current_line() {
            if self.is_delimiter_line(line) {
                let delimiter = line.to_string();
                return self.collect_delimited_content(&delimiter);
            }
        }

        self.collect_paragraph_content()
    }

    pub(crate) fn collect_paragraph_content(&mut self) -> String {
        let mut content = String::new();

        while !self.is_eof() {
            let line = self.current_line().unwrap_or("");

            if line.trim().is_empty() {
                break;
            }

            // Check for start of new block.
            // Only treat `[...]` as a block boundary when the whole line is a block attribute
            // (starts with `[` AND ends with `]`). Inline anchors `[[id]]text` must not break.
            let is_block_attr_line = line.starts_with('[') && line.ends_with(']');
            let trimmed_line = line.trim_start();
            if line.starts_with('=')
                || is_block_attr_line
                || self.is_delimiter_line(line)
                || line.starts_with("image::")
                || line == "--"
                || line == "|==="
                || trimmed_line.starts_with("* ")
                || trimmed_line.starts_with("** ")
                || trimmed_line.starts_with("- ")
                || trimmed_line.starts_with(". ")
                || trimmed_line.starts_with(".. ")
            {
                break;
            }

            if !content.is_empty() {
                content.push(' ');
            }
            content.push_str(line.trim());
            self.advance_line();
        }

        content
    }

    fn try_parse_list(&mut self) -> Option<Block> {
        let line = self.current_line()?;
        let trimmed = line.trim_start();

        // Unordered list: *, **, -, --
        if let Some(rest) = trimmed.strip_prefix("* ") {
            return Some(self.parse_unordered_list("* ", rest));
        }
        if let Some(rest) = trimmed.strip_prefix("** ") {
            return Some(self.parse_unordered_list("** ", rest));
        }
        if let Some(rest) = trimmed.strip_prefix("- ") {
            return Some(self.parse_unordered_list("- ", rest));
        }

        // Ordered list: ., .., 1., a., i.
        if let Some(rest) = trimmed.strip_prefix(". ") {
            return Some(self.parse_ordered_list(". ", rest));
        }
        if let Some(rest) = trimmed.strip_prefix(".. ") {
            return Some(self.parse_ordered_list(".. ", rest));
        }

        // Numbered: 1. 2. etc.
        if let Some(idx) = trimmed.find(". ") {
            let prefix = &trimmed[..idx];
            if prefix.chars().all(|c| c.is_ascii_digit()) {
                let rest = &trimmed[idx + 2..];
                return Some(self.parse_ordered_list_numbered(rest));
            }
        }

        // Description list: term:: definition (but not image::)
        if trimmed.contains("::")
            && !trimmed.starts_with("::")
            && !trimmed.starts_with("image::")
        {
            return self.parse_description_list();
        }

        None
    }

    fn parse_unordered_list(&mut self, marker: &str, first_content: &str) -> Block {
        // Determine the sub-list marker (one more level of '*')
        let sub_marker: Option<&str> = if marker == "* " { Some("** ") } else { None };

        let mut items = Vec::new();

        // First item — detect checklist prefix
        let (checked, text) = parse_checklist_prefix(first_content);
        let inlines = parse_inline_content(text);
        items.push(vec![Block::Paragraph {
            inlines,
            id: None,
            role: None,
            checked,
            span: Span::NONE,
        }]);
        self.advance_line();

        // Subsequent items
        loop {
            if self.is_eof() {
                break;
            }

            let line = self.current_line().unwrap_or("");

            // Skip blank lines between items, but detect `+` continuation
            if line.trim().is_empty() {
                self.advance_line();
                let next = self.current_line().unwrap_or("");
                let next_trimmed = next.trim_start();
                if next_trimmed == "+" {
                    // List continuation: consume `+`, attach next block to last item
                    self.advance_line();
                    if let Some(cont_block) = self.try_parse_block() {
                        if let Some(last_item) = items.last_mut() {
                            last_item.push(cont_block);
                        }
                    }
                    continue;
                }
                // After blank, only continue if next line starts a same-level item
                if next_trimmed.strip_prefix(marker).is_none() {
                    break;
                }
                continue;
            }

            let trimmed = line.trim_start();

            // `+` continuation without preceding blank
            if trimmed == "+" {
                self.advance_line();
                if let Some(cont_block) = self.try_parse_block() {
                    if let Some(last_item) = items.last_mut() {
                        last_item.push(cont_block);
                    }
                }
                continue;
            }

            // Sub-list item (e.g. `** ` when current marker is `* `)
            if let Some(sub) = sub_marker {
                if let Some(sub_content) = trimmed.strip_prefix(sub) {
                    let sub_list = self.parse_unordered_list(sub, sub_content);
                    if let Some(last_item) = items.last_mut() {
                        last_item.push(sub_list);
                    }
                    continue;
                }
            }

            if let Some(rest) = trimmed.strip_prefix(marker) {
                let (checked, text) = parse_checklist_prefix(rest);
                let inlines = parse_inline_content(text);
                items.push(vec![Block::Paragraph {
                    inlines,
                    id: None,
                    role: None,
                    checked,
                    span: Span::NONE,
                }]);
                self.advance_line();
            } else {
                break;
            }
        }

        Block::List {
            ordered: false,
            items,
            style: None,
            span: Span::NONE,
        }
    }

    fn parse_ordered_list(&mut self, marker: &str, first_content: &str) -> Block {
        let style = self.pending_list_style.take();
        let mut items = Vec::new();

        // First item
        let inlines = parse_inline_content(first_content);
        items.push(vec![Block::Paragraph {
            inlines,
            id: None,
            role: None,
            checked: None,
            span: Span::NONE,
        }]);
        self.advance_line();

        // Subsequent items
        while !self.is_eof() {
            self.skip_blank_lines();
            let Some(line) = self.current_line() else {
                break;
            };
            let trimmed = line.trim_start();

            if let Some(rest) = trimmed.strip_prefix(marker) {
                let inlines = parse_inline_content(rest);
                items.push(vec![Block::Paragraph {
                    inlines,
                    id: None,
                    role: None,
                    checked: None,
                    span: Span::NONE,
                }]);
                self.advance_line();
            } else {
                break;
            }
        }

        Block::List {
            ordered: true,
            items,
            style,
            span: Span::NONE,
        }
    }

    fn parse_ordered_list_numbered(&mut self, first_content: &str) -> Block {
        let style = self.pending_list_style.take();
        let mut items = Vec::new();

        // First item
        let inlines = parse_inline_content(first_content);
        items.push(vec![Block::Paragraph {
            inlines,
            id: None,
            role: None,
            checked: None,
            span: Span::NONE,
        }]);
        self.advance_line();

        // Subsequent items
        while !self.is_eof() {
            self.skip_blank_lines();
            let Some(line) = self.current_line() else {
                break;
            };
            let trimmed = line.trim_start();

            if let Some(idx) = trimmed.find(". ") {
                let prefix = &trimmed[..idx];
                if prefix.chars().all(|c| c.is_ascii_digit()) {
                    let rest = &trimmed[idx + 2..];
                    let inlines = parse_inline_content(rest);
                    items.push(vec![Block::Paragraph {
                        inlines,
                        id: None,
                        role: None,
                        checked: None,
                        span: Span::NONE,
                    }]);
                    self.advance_line();
                    continue;
                }
            }
            break;
        }

        Block::List {
            ordered: true,
            items,
            style,
            span: Span::NONE,
        }
    }

    fn parse_description_list(&mut self) -> Option<Block> {
        let mut items = Vec::new();

        while !self.is_eof() {
            let Some(line) = self.current_line() else {
                break;
            };

            if line.trim().is_empty() {
                self.advance_line();
                continue;
            }

            // Find :: separator
            if let Some(sep_idx) = line.find("::") {
                let term = line[..sep_idx].trim();
                let def_start = &line[sep_idx + 2..].trim();

                let term_inlines = parse_inline_content(term);

                self.advance_line();

                // Definition may continue on next lines if indented
                let mut def_content = def_start.to_string();
                while !self.is_eof() {
                    let def_line = self.current_line().unwrap_or("");
                    if def_line.trim().is_empty() {
                        break;
                    }
                    if def_line.starts_with(' ') || def_line.starts_with('\t') {
                        if !def_content.is_empty() {
                            def_content.push(' ');
                        }
                        def_content.push_str(def_line.trim());
                        self.advance_line();
                    } else if def_line.contains("::") && !def_line.starts_with("::") {
                        // Another term
                        break;
                    } else {
                        break;
                    }
                }

                let desc_inlines = parse_inline_content(&def_content);
                items.push(DefinitionItem {
                    term: term_inlines,
                    desc: desc_inlines,
                });
            } else {
                break;
            }
        }

        if items.is_empty() {
            return None;
        }

        Some(Block::DefinitionList {
            items,
            span: Span::NONE,
        })
    }

    fn parse_block_image(&mut self) -> Option<Block> {
        let line = self.current_line()?;

        // image::path[alt,width,height]
        let rest = line.strip_prefix("image::")?;

        let (path, attrs) = if let Some(bracket_start) = rest.find('[') {
            let path = &rest[..bracket_start];
            let attrs_str = &rest[bracket_start + 1..];
            let attrs_end = attrs_str.find(']').unwrap_or(attrs_str.len());
            (path.to_string(), attrs_str[..attrs_end].to_string())
        } else {
            (rest.to_string(), String::new())
        };

        self.advance_line();

        let attrs_parts: Vec<&str> = attrs.split(',').map(|s| s.trim()).collect();
        let alt = attrs_parts
            .first()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let width = attrs_parts
            .get(1)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let height = attrs_parts
            .get(2)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        Some(Block::Figure {
            image: ImageData {
                url: path,
                alt,
                width,
                height,
            },
            span: Span::NONE,
        })
    }

    fn parse_paragraph(&mut self) -> Option<Block> {
        let content = self.collect_paragraph_content();

        if content.is_empty() {
            return None;
        }

        let id = self.pending_id.take();
        let role = self.pending_role.take();
        let inlines = parse_inline_content(&content);
        Some(Block::Paragraph {
            inlines,
            id,
            role,
            checked: None,
            span: Span::NONE,
        })
    }

    /// Parse `|===` table block.
    ///
    /// Supports the common AsciiDoc table syntax:
    /// - One cell per line starting with `|`
    /// - Multiple cells on the same line: `| A | B | C`
    /// - Header row: first group of rows before the first blank line (when followed by more rows)
    fn parse_table(&mut self) -> Option<Block> {
        self.advance_line(); // consume |=== line

        // Collect all logical rows as (cells, line_group_index) pairs.
        // A blank line separates row groups; the first group becomes header rows
        // if there are subsequent groups.
        let mut row_groups: Vec<Vec<Vec<Vec<Inline>>>> = Vec::new(); // groups of rows of cells
        let mut current_group: Vec<Vec<Vec<Inline>>> = Vec::new();
        let mut current_row: Vec<Vec<Inline>> = Vec::new();

        while !self.is_eof() {
            let line = self.current_line().unwrap_or("");
            if line == "|===" {
                self.advance_line();
                break;
            }
            if line.trim().is_empty() {
                // Flush current row and group
                if !current_row.is_empty() {
                    current_group.push(std::mem::take(&mut current_row));
                }
                if !current_group.is_empty() {
                    row_groups.push(std::mem::take(&mut current_group));
                }
                self.advance_line();
                continue;
            }
            if line.starts_with('|') {
                // Split on `|` — each `|` starts a new cell.
                // `| A | B` → cells ["A", "B"]
                // `|A|B` → cells ["A", "B"]
                // `| A | B |` → cells ["A", "B", ""] (trailing empty dropped)
                let parts: Vec<&str> = line.splitn(2, '|').collect();
                let rest = if parts.len() > 1 { parts[1] } else { "" };
                let cell_parts: Vec<&str> = rest.split('|').collect();
                for (i, cell) in cell_parts.iter().enumerate() {
                    let trimmed = cell.trim();
                    // Drop trailing empty cell from `| A | B |`
                    if i == cell_parts.len() - 1 && trimmed.is_empty() {
                        continue;
                    }
                    current_row.push(parse_inline_content(trimmed));
                }
            }
            self.advance_line();
        }

        // Flush remaining
        if !current_row.is_empty() {
            current_group.push(current_row);
        }
        if !current_group.is_empty() {
            row_groups.push(current_group);
        }

        // If there are multiple groups, first group is header
        let has_header = row_groups.len() >= 2;
        let mut rows: Vec<crate::ast::TableRow> = Vec::new();
        for (gi, group) in row_groups.into_iter().enumerate() {
            let is_header = has_header && gi == 0;
            for cells in group {
                rows.push(crate::ast::TableRow { cells, is_header });
            }
        }

        Some(Block::Table {
            rows,
            span: Span::NONE,
        })
    }

    /// Parse `--` open block.
    fn parse_open_block(&mut self) -> Option<Block> {
        self.advance_line(); // skip opening --
        let mut children = Vec::new();
        while !self.is_eof() {
            let line = self.current_line().unwrap_or("");
            if line == "--" {
                self.advance_line();
                break;
            }
            if let Some(block) = self.try_parse_block() {
                children.push(block);
            } else {
                self.advance_line();
            }
        }
        Some(Block::Div {
            class: Some("open".to_string()),
            title: None,
            children,
            span: Span::NONE,
        })
    }

    /// Parse `.Title` block title line — stores title and attaches it to the following block.
    fn parse_block_title(&mut self) -> Option<Block> {
        let line = self.current_line()?;
        let title = line[1..].trim().to_string();
        self.advance_line();
        self.skip_blank_lines();
        let block = self.try_parse_block()?;
        let block = match block {
            Block::Div {
                class,
                children,
                span,
                ..
            } => Block::Div {
                class,
                title: Some(title),
                children,
                span,
            },
            other => other,
        };
        Some(block)
    }
}

// ── Iterator impl ─────────────────────────────────────────────────────────────

impl Iterator for EventIter<'_> {
    type Item = OwnedEvent;

    fn next(&mut self) -> Option<OwnedEvent> {
        if let Some(ev) = self.event_buf.pop_front() {
            return Some(ev);
        }
        if self.iter_done {
            return None;
        }
        if !self.iter_started {
            self.iter_started = true;
            return Some(OwnedEvent::StartDocument);
        }
        loop {
            self.skip_blank_lines();
            if self.is_eof() {
                break;
            }
            if let Some(block) = self.try_parse_block() {
                collect_block_events(&block, &mut self.event_buf);
                if let Some(ev) = self.event_buf.pop_front() {
                    return Some(ev);
                }
                // No events produced (shouldn't happen) — keep looping
            } else {
                self.advance_line();
            }
        }
        self.iter_done = true;
        Some(OwnedEvent::EndDocument)
    }
}

// ── Checklist helper ──────────────────────────────────────────────────────────

/// Detect and strip a checklist prefix from list item content.
///
/// Returns `(checked_state, remaining_text)`:
/// - `[x] ` or `[*] ` → `(Some(true), rest)`
/// - `[ ] ` → `(Some(false), rest)`
/// - anything else → `(None, original)`
fn parse_checklist_prefix(content: &str) -> (Option<bool>, &str) {
    if let Some(rest) = content.strip_prefix("[x] ").or_else(|| content.strip_prefix("[*] ")) {
        (Some(true), rest)
    } else if let Some(rest) = content.strip_prefix("[ ] ") {
        (Some(false), rest)
    } else {
        (None, content)
    }
}

// ── Inline parser ─────────────────────────────────────────────────────────────

/// Parse inline AsciiDoc content into a list of `Inline` elements.
pub fn parse_inline_content(content: &str) -> Vec<Inline> {
    let mut nodes = Vec::new();
    let mut pos = 0;
    let chars: Vec<char> = content.chars().collect();

    while pos < chars.len() {
        // Strong: *text* or **text**
        if chars[pos] == '*' {
            // Check for **text** first
            if pos + 1 < chars.len() && chars[pos + 1] == '*' {
                if let Some((end, text)) = find_double_closing(&chars, pos + 2, '*') {
                    nodes.push(Inline::Strong(
                        parse_inline_content(&text),
                        Span::NONE,
                    ));
                    pos = end + 2;
                    continue;
                }
            }
            // Single *text*
            if let Some((end, text)) = find_closing_char(&chars, pos + 1, '*') {
                nodes.push(Inline::Strong(parse_inline_content(&text), Span::NONE));
                pos = end + 1;
                continue;
            }
        }

        // Emphasis: _text_ or __text__
        if chars[pos] == '_' {
            // Check for __text__ first
            if pos + 1 < chars.len() && chars[pos + 1] == '_' {
                if let Some((end, text)) = find_double_closing(&chars, pos + 2, '_') {
                    nodes.push(Inline::Emphasis(
                        parse_inline_content(&text),
                        Span::NONE,
                    ));
                    pos = end + 2;
                    continue;
                }
            }
            // Single _text_
            if let Some((end, text)) = find_closing_char(&chars, pos + 1, '_') {
                nodes.push(Inline::Emphasis(parse_inline_content(&text), Span::NONE));
                pos = end + 1;
                continue;
            }
        }

        // Monospace: `text` or ``text``
        if chars[pos] == '`' {
            // Check for ``text`` first
            if pos + 1 < chars.len() && chars[pos + 1] == '`' {
                if let Some((end, text)) = find_double_closing(&chars, pos + 2, '`') {
                    nodes.push(Inline::Code(text, Span::NONE));
                    pos = end + 2;
                    continue;
                }
            }
            // Single `text`
            if let Some((end, text)) = find_closing_char(&chars, pos + 1, '`') {
                nodes.push(Inline::Code(text, Span::NONE));
                pos = end + 1;
                continue;
            }
        }

        // Superscript: ^text^
        if chars[pos] == '^' {
            if let Some((end, text)) = find_closing_char(&chars, pos + 1, '^') {
                nodes.push(Inline::Superscript(
                    parse_inline_content(&text),
                    Span::NONE,
                ));
                pos = end + 1;
                continue;
            }
        }

        // Subscript: ~text~
        if chars[pos] == '~' {
            if let Some((end, text)) = find_closing_char(&chars, pos + 1, '~') {
                nodes.push(Inline::Subscript(
                    parse_inline_content(&text),
                    Span::NONE,
                ));
                pos = end + 1;
                continue;
            }
        }

        // [role]#text# or [[id]] anchor
        if chars[pos] == '[' {
            // Inline anchor: [[id]] — must check before [role]#text# since [[...]] starts with [[
            if chars.get(pos + 1) == Some(&'[') {
                if let Some(end) = find_double_bracket_close(&chars, pos + 2) {
                    let id: String = chars[pos + 2..end].iter().collect();
                    nodes.push(Inline::Anchor {
                        id: id.trim().to_string(),
                        span: Span::NONE,
                    });
                    pos = end + 2; // skip closing ]]
                    continue;
                }
            }

            // [role]#text# — semantic highlight/formatting roles
            if let Some((role_end, role)) = find_closing_char(&chars, pos + 1, ']')
                && chars.get(role_end + 1) == Some(&'#')
                && let Some((text_end, text)) = find_closing_char(&chars, role_end + 2, '#')
            {
                let children = parse_inline_content(&text);
                let inline = match role.trim() {
                    "line-through" | "line_through" | "strikethrough" => {
                        Inline::Strikeout(children, Span::NONE)
                    }
                    "underline" => Inline::Underline(children, Span::NONE),
                    "small-caps" | "small_caps" => Inline::SmallCaps(children, Span::NONE),
                    _ => Inline::Highlight(children, Span::NONE),
                };
                nodes.push(inline);
                pos = text_end + 1;
                continue;
            }
            // Not a [role]#text# pattern — emit '[' as text
            nodes.push(Inline::Text { text: "[".to_string(), span: Span::NONE });
            pos += 1;
            continue;
        }

        // Highlight: #text#
        if chars[pos] == '#' {
            if let Some((end, text)) = find_closing_char(&chars, pos + 1, '#') {
                nodes.push(Inline::Highlight(
                    parse_inline_content(&text),
                    Span::NONE,
                ));
                pos = end + 1;
                continue;
            }
        }

        // Inline image: image:path[alt]
        if pos + 6 < chars.len() {
            let prefix: String = chars[pos..pos + 6].iter().collect();
            if prefix == "image:" && !chars.get(pos + 6).map(|c| *c == ':').unwrap_or(false) {
                if let Some((inline, end)) = parse_inline_image(&chars, pos) {
                    nodes.push(inline);
                    pos = end;
                    continue;
                }
            }
        }

        // Link: https://url or link:url[text] or <<anchor>>
        if pos + 4 < chars.len() {
            let prefix: String = chars[pos..].iter().take(8).collect();
            if prefix.starts_with("https://") || prefix.starts_with("http://") {
                if let Some((end, inline)) = parse_url_link(&chars, pos) {
                    nodes.push(inline);
                    pos = end;
                    continue;
                }
            }
        }

        if pos + 5 < chars.len() {
            let prefix: String = chars[pos..pos + 5].iter().collect();
            if prefix == "link:" {
                if let Some((end, inline)) = parse_link_macro(&chars, pos) {
                    nodes.push(inline);
                    pos = end;
                    continue;
                }
            }
        }

        if pos + 2 < chars.len() && chars[pos] == '<' && chars[pos + 1] == '<' {
            if let Some((end, inline)) = parse_xref(&chars, pos) {
                nodes.push(inline);
                pos = end;
                continue;
            }
        }

        // Macro shortcuts: footnote:[...], kbd:[...], btn:[...], pass:[...], menu:...[...]
        if chars[pos] == 'f' || chars[pos] == 'k' || chars[pos] == 'b' || chars[pos] == 'p' || chars[pos] == 'm' {
            let remaining: String = chars[pos..].iter().take(12).collect();

            // footnote:[text]
            if remaining.starts_with("footnote:[") || remaining.starts_with("footnote::[") {
                let skip = if remaining.starts_with("footnote::[") { 11 } else { 10 };
                let content_start = pos + skip;
                if let Some(close_pos) = find_matching_bracket(&chars, content_start) {
                    let content: String = chars[content_start..close_pos].iter().collect();
                    let label = format!("fn{}", nodes.len());
                    let fn_inlines = parse_inline_content(&content);
                    nodes.push(Inline::FootnoteRef {
                        label: label.clone(),
                        span: Span::NONE,
                    });
                    nodes.push(Inline::FootnoteDef {
                        label,
                        children: fn_inlines,
                        span: Span::NONE,
                    });
                    pos = close_pos + 1;
                    continue;
                }
            }

            // kbd:[keys]
            if remaining.starts_with("kbd:[") {
                let content_start = pos + 5;
                if let Some(close_pos) = find_matching_bracket(&chars, content_start) {
                    let content: String = chars[content_start..close_pos].iter().collect();
                    nodes.push(Inline::RawInline {
                        format: "asciidoc".to_string(),
                        content: format!("kbd:[{}]", content),
                        span: Span::NONE,
                    });
                    pos = close_pos + 1;
                    continue;
                }
            }

            // btn:[label]
            if remaining.starts_with("btn:[") {
                let content_start = pos + 5;
                if let Some(close_pos) = find_matching_bracket(&chars, content_start) {
                    let content: String = chars[content_start..close_pos].iter().collect();
                    nodes.push(Inline::RawInline {
                        format: "asciidoc".to_string(),
                        content: format!("btn:[{}]", content),
                        span: Span::NONE,
                    });
                    pos = close_pos + 1;
                    continue;
                }
            }

            // pass:[content] — raw inline passthrough
            if remaining.starts_with("pass:[") {
                let content_start = pos + 6;
                if let Some(close_pos) = find_matching_bracket(&chars, content_start) {
                    let content: String = chars[content_start..close_pos].iter().collect();
                    nodes.push(Inline::RawInline {
                        format: "html".to_string(),
                        content,
                        span: Span::NONE,
                    });
                    pos = close_pos + 1;
                    continue;
                }
            }

            // menu:name[items]
            if remaining.starts_with("menu:") {
                if let Some((end, inline)) = parse_menu_macro(&chars, pos) {
                    nodes.push(inline);
                    pos = end;
                    continue;
                }
            }
        }

        // Inline literal passthrough: +word+ (constrained — + followed immediately by non-space)
        // Only matches when: next char after opening + is non-space, and there's a closing +
        if chars[pos] == '+' {
            let next = chars.get(pos + 1);
            let is_line_break = next.is_none() || next == Some(&' ') || next == Some(&'\n');
            if !is_line_break {
                // Look for closing +
                if let Some((end, text)) = find_closing_char(&chars, pos + 1, '+') {
                    // Closing + must be followed by non-word char or end
                    let after = chars.get(end + 1);
                    let valid_close = after.is_none()
                        || !after.unwrap().is_alphanumeric() && *after.unwrap() != '_';
                    if valid_close {
                        nodes.push(Inline::Code(text, Span::NONE));
                        pos = end + 1;
                        continue;
                    }
                }
            }
        }

        // Line break: + at end of line (pos+1 is space or end)
        if chars[pos] == '+' && (pos + 1 >= chars.len() || chars[pos + 1] == ' ') {
            nodes.push(Inline::LineBreak { span: Span::NONE });
            pos += 1;
            continue;
        }

        // Regular text
        let pos_before = pos;
        let mut text = String::new();
        while pos < chars.len() {
            let c = chars[pos];
            // Stop at potential markup starts
            if c == '*'
                || c == '_'
                || c == '`'
                || c == '^'
                || c == '~'
                || c == '#'
                || c == '+'
                || c == '<'
                || c == '['
            {
                break;
            }
            // Check for macros: image:, link:, https://, http://, footnote:, kbd:, btn:, pass:, menu:
            if c == 'i' || c == 'l' || c == 'h' || c == 'f' || c == 'k' || c == 'b' || c == 'p' || c == 'm' {
                let remaining: String = chars[pos..].iter().take(12).collect();
                if remaining.starts_with("image:")
                    || remaining.starts_with("link:")
                    || remaining.starts_with("https://")
                    || remaining.starts_with("http://")
                    || remaining.starts_with("footnote:[")
                    || remaining.starts_with("footnote::[")
                    || remaining.starts_with("kbd:[")
                    || remaining.starts_with("btn:[")
                    || remaining.starts_with("pass:[")
                    || remaining.starts_with("menu:")
                {
                    break;
                }
            }
            text.push(c);
            pos += 1;
        }

        if !text.is_empty() {
            nodes.push(Inline::Text {
                text,
                span: Span::NONE,
            });
        } else if pos == pos_before {
            // No markup matched and the text loop didn't advance — consume the
            // current character literally to guarantee forward progress.
            nodes.push(Inline::Text {
                text: chars[pos].to_string(),
                span: Span::NONE,
            });
            pos += 1;
        }
    }

    // Merge adjacent text nodes
    merge_text_nodes(&mut nodes);

    nodes
}

fn find_closing_char(chars: &[char], start: usize, close: char) -> Option<(usize, String)> {
    let mut pos = start;
    let mut text = String::new();

    while pos < chars.len() {
        if chars[pos] == close {
            return Some((pos, text));
        }
        text.push(chars[pos]);
        pos += 1;
    }

    None
}

fn find_double_closing(chars: &[char], start: usize, close: char) -> Option<(usize, String)> {
    let mut pos = start;
    let mut text = String::new();

    while pos + 1 < chars.len() {
        if chars[pos] == close && chars[pos + 1] == close {
            return Some((pos, text));
        }
        text.push(chars[pos]);
        pos += 1;
    }

    None
}

fn parse_inline_image(chars: &[char], start: usize) -> Option<(Inline, usize)> {
    // image:path[alt]
    let pos = start + 6; // Skip "image:"

    let mut path_end = pos;
    while path_end < chars.len() && chars[path_end] != '[' {
        path_end += 1;
    }

    if path_end >= chars.len() {
        return None;
    }

    let path: String = chars[pos..path_end].iter().collect();

    // Find closing ]
    let mut attr_end = path_end + 1;
    while attr_end < chars.len() && chars[attr_end] != ']' {
        attr_end += 1;
    }

    let attrs: String = chars[path_end + 1..attr_end].iter().collect();
    let end = if attr_end < chars.len() {
        attr_end + 1
    } else {
        chars.len()
    };

    let alt = if attrs.is_empty() { None } else { Some(attrs) };

    Some((
        Inline::Image(
            ImageData {
                url: path,
                alt,
                width: None,
                height: None,
            },
            Span::NONE,
        ),
        end,
    ))
}

fn parse_url_link(chars: &[char], start: usize) -> Option<(usize, Inline)> {
    let mut pos = start;

    // Collect URL until space or [ or end
    while pos < chars.len() && !chars[pos].is_whitespace() && chars[pos] != '[' {
        pos += 1;
    }

    let url: String = chars[start..pos].iter().collect();

    // Check for [text]
    if pos < chars.len() && chars[pos] == '[' {
        let text_start = pos + 1;
        while pos < chars.len() && chars[pos] != ']' {
            pos += 1;
        }
        let link_text: String = chars[text_start..pos].iter().collect();
        pos += 1; // Skip ]

        let children = vec![Inline::Text {
            text: link_text,
            span: Span::NONE,
        }];
        return Some((
            pos,
            Inline::Link {
                url,
                children,
                target: None,
                span: Span::NONE,
            },
        ));
    }

    // No text - URL is the text
    let children = vec![Inline::Text {
        text: url.clone(),
        span: Span::NONE,
    }];
    Some((
        pos,
        Inline::Link {
            url,
            children,
            target: None,
            span: Span::NONE,
        },
    ))
}

fn parse_link_macro(chars: &[char], start: usize) -> Option<(usize, Inline)> {
    // link:url[text] or link:url[text,window=_blank]
    let url_start = start + 5; // Skip "link:"
    let mut pos = url_start;

    while pos < chars.len() && chars[pos] != '[' {
        pos += 1;
    }

    if pos >= chars.len() {
        return None;
    }

    let url: String = chars[url_start..pos].iter().collect();

    let text_start = pos + 1;
    while pos < chars.len() && chars[pos] != ']' {
        pos += 1;
    }
    let link_attrs: String = chars[text_start..pos].iter().collect();
    pos += 1; // Skip ]

    // Parse display text and optional window= target from attrs
    let (display_text, target) = parse_link_attr_list(&link_attrs);
    let text = if display_text.is_empty() {
        url.clone()
    } else {
        display_text
    };

    let children = vec![Inline::Text {
        text,
        span: Span::NONE,
    }];
    Some((
        pos,
        Inline::Link {
            url,
            children,
            target,
            span: Span::NONE,
        },
    ))
}

/// Parse AsciiDoc link attribute list: `"display text,window=_blank"`.
/// Returns `(display_text, target)`.
fn parse_link_attr_list(attrs: &str) -> (String, Option<String>) {
    // Split on first comma: positional display text, then named attrs
    let (display_part, named_part) = match attrs.find(',') {
        Some(idx) => (&attrs[..idx], Some(&attrs[idx + 1..])),
        None => (attrs, None),
    };
    let display_text = display_part.trim().to_string();
    let mut target = None;

    if let Some(named) = named_part {
        for kv in named.split(',') {
            let kv = kv.trim();
            if let Some(v) = kv.strip_prefix("window=") {
                target = Some(v.trim_matches('"').trim_matches('\'').to_string());
            }
        }
    }

    (display_text, target)
}

fn parse_xref(chars: &[char], start: usize) -> Option<(usize, Inline)> {
    // <<anchor>> or <<anchor,text>>
    let anchor_start = start + 2; // Skip "<<"
    let mut pos = anchor_start;

    while pos + 1 < chars.len() && !(chars[pos] == '>' && chars[pos + 1] == '>') {
        pos += 1;
    }

    if pos + 1 >= chars.len() {
        return None;
    }

    let content: String = chars[anchor_start..pos].iter().collect();
    pos += 2; // Skip ">>"

    let (anchor, text) = if let Some(comma_idx) = content.find(',') {
        (
            content[..comma_idx].to_string(),
            content[comma_idx + 1..].trim().to_string(),
        )
    } else {
        (content.clone(), content)
    };

    let url = format!("#{}", anchor);
    let children = vec![Inline::Text {
        text,
        span: Span::NONE,
    }];
    Some((
        pos,
        Inline::Link {
            url,
            children,
            target: None,
            span: Span::NONE,
        },
    ))
}

/// Find the closing `]]` for an inline anchor, starting after the opening `[[`.
/// Returns the index of the first `]` of `]]`.
fn find_double_bracket_close(chars: &[char], start: usize) -> Option<usize> {
    let mut pos = start;
    while pos + 1 < chars.len() {
        if chars[pos] == ']' && chars[pos + 1] == ']' {
            return Some(pos);
        }
        pos += 1;
    }
    None
}

/// Find the closing `]` for a macro bracket argument, starting after the `[`.
/// Returns the index of `]`.
fn find_matching_bracket(chars: &[char], start: usize) -> Option<usize> {
    let mut pos = start;
    let mut depth = 1usize;
    while pos < chars.len() {
        match chars[pos] {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(pos);
                }
            }
            _ => {}
        }
        pos += 1;
    }
    None
}

/// Parse `menu:Name[Item > SubItem]` inline macro.
fn parse_menu_macro(chars: &[char], start: usize) -> Option<(usize, Inline)> {
    // Advance past "menu:"
    let name_start = start + 5;
    let mut pos = name_start;

    // Collect menu name (up to '[')
    while pos < chars.len() && chars[pos] != '[' {
        pos += 1;
    }

    if pos >= chars.len() {
        return None;
    }

    let menu_name: String = chars[name_start..pos].iter().collect();
    let attr_start = pos + 1;

    if let Some(close_pos) = find_matching_bracket(chars, attr_start) {
        let items: String = chars[attr_start..close_pos].iter().collect();
        let content = if items.is_empty() {
            menu_name
        } else {
            format!("{}>{}", menu_name, items)
        };
        Some((
            close_pos + 1,
            Inline::RawInline {
                format: "asciidoc".to_string(),
                content: format!("menu:{}", content),
                span: Span::NONE,
            },
        ))
    } else {
        None
    }
}

/// Merge adjacent text nodes.
fn merge_text_nodes(nodes: &mut Vec<Inline>) {
    let mut i = 0;
    while i + 1 < nodes.len() {
        let is_both_text = matches!(&nodes[i], Inline::Text { .. })
            && matches!(&nodes[i + 1], Inline::Text { .. });
        if is_both_text {
            let next_text = match nodes.remove(i + 1) {
                Inline::Text { text, .. } => text,
                _ => unreachable!(),
            };
            if let Inline::Text { text, .. } = &mut nodes[i] {
                text.push_str(&next_text);
            }
        } else {
            i += 1;
        }
    }
}
