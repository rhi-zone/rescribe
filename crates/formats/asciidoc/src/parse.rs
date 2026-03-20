//! AsciiDoc parser.

use crate::ast::{
    AsciiDoc, Block, DefinitionItem, Diagnostic, ImageData, Inline, Span,
};

// ── Public API ────────────────────────────────────────────────────────────────

/// Parse an AsciiDoc string into an [`AsciiDoc`] document.
///
/// Parsing is always infallible: malformed constructs produce diagnostics
/// rather than errors.
pub fn parse(input: &str) -> (AsciiDoc, Vec<Diagnostic>) {
    let mut p = Parser::new(input);
    let (blocks, attributes) = p.parse_document();
    let doc = AsciiDoc {
        blocks,
        attributes,
        span: Span::NONE,
    };
    (doc, p.diagnostics)
}

// ── Parser ────────────────────────────────────────────────────────────────────

pub(crate) struct Parser<'a> {
    lines: Vec<&'a str>,
    line_idx: usize,
    attributes: std::collections::HashMap<String, String>,
    pub(crate) diagnostics: Vec<Diagnostic>,
}

impl<'a> Parser<'a> {
    pub(crate) fn new(input: &'a str) -> Self {
        let lines: Vec<&str> = input.lines().collect();
        Self {
            lines,
            line_idx: 0,
            attributes: std::collections::HashMap::new(),
            diagnostics: Vec::new(),
        }
    }

    fn current_line(&self) -> Option<&'a str> {
        self.lines.get(self.line_idx).copied()
    }

    fn advance_line(&mut self) {
        self.line_idx += 1;
    }

    fn is_eof(&self) -> bool {
        self.line_idx >= self.lines.len()
    }

    fn is_blank_line(&self) -> bool {
        self.current_line()
            .map(|l| l.trim().is_empty())
            .unwrap_or(true)
    }

    fn skip_blank_lines(&mut self) {
        while !self.is_eof() && self.is_blank_line() {
            self.advance_line();
        }
    }

    pub(crate) fn parse_document(
        &mut self,
    ) -> (Vec<Block>, std::collections::HashMap<String, String>) {
        let mut blocks = Vec::new();

        while !self.is_eof() {
            self.skip_blank_lines();
            if self.is_eof() {
                break;
            }

            if let Some(block) = self.try_parse_block() {
                blocks.push(block);
            } else {
                // Fallback: skip line to prevent infinite loop
                self.advance_line();
            }
        }

        (blocks, std::mem::take(&mut self.attributes))
    }

    fn try_parse_block(&mut self) -> Option<Block> {
        let line = self.current_line()?;

        // Document attributes (:attr: value)
        if line.starts_with(':') && line.len() > 1 {
            if let Some((attr, value)) = self.try_parse_attribute(line) {
                self.attributes.insert(attr, value);
                self.advance_line();
                return self.try_parse_block();
            }
        }

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
        if !rest.starts_with(' ') && !rest.is_empty() {
            return None;
        }

        let title = rest.trim();
        self.advance_line();

        let inlines = parse_inline_content(title);
        Some(Block::Heading {
            level,
            inlines,
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

        match first_attr.as_deref() {
            Some("NOTE") | Some("TIP") | Some("WARNING") | Some("IMPORTANT") | Some("CAUTION") => {
                let admonition_type = first_attr.unwrap().to_lowercase();
                let content = self.collect_paragraph_content();
                let inlines = parse_inline_content(&content);
                Some(Block::Div {
                    class: Some(format!("admonition {}", admonition_type)),
                    children: vec![Block::Paragraph {
                        inlines,
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
                    children: vec![Block::Paragraph {
                        inlines,
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
                    children: vec![Block::Paragraph {
                        inlines,
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

        let patterns = ["----", "====", "****", "____", "++++", "...."];

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
                    children: vec![Block::Paragraph {
                        inlines,
                        span: Span::NONE,
                    }],
                    span: Span::NONE,
                }
            }
            '*' => {
                let inlines = parse_inline_content(&content);
                Block::Div {
                    class: Some("sidebar".to_string()),
                    children: vec![Block::Paragraph {
                        inlines,
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
                    children: vec![Block::Paragraph {
                        inlines,
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

            // Check for start of new block
            if line.starts_with('=')
                || line.starts_with('[')
                || self.is_delimiter_line(line)
                || line.starts_with("image::")
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
        let mut items = Vec::new();

        // First item
        let inlines = parse_inline_content(first_content);
        items.push(vec![Block::Paragraph {
            inlines,
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
                    span: Span::NONE,
                }]);
                self.advance_line();
            } else {
                // Check for continuation
                if line.starts_with(' ') || line.starts_with('\t') {
                    // Continuation - append to last item's last paragraph
                    if let Some(last_item) = items.last_mut() {
                        let more_inlines = parse_inline_content(trimmed);
                        // Append to the last paragraph in the item
                        if let Some(Block::Paragraph { inlines, .. }) = last_item.last_mut() {
                            inlines.extend(more_inlines);
                        }
                    }
                    self.advance_line();
                } else {
                    break;
                }
            }
        }

        Block::List {
            ordered: false,
            items,
            span: Span::NONE,
        }
    }

    fn parse_ordered_list(&mut self, marker: &str, first_content: &str) -> Block {
        let mut items = Vec::new();

        // First item
        let inlines = parse_inline_content(first_content);
        items.push(vec![Block::Paragraph {
            inlines,
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
            span: Span::NONE,
        }
    }

    fn parse_ordered_list_numbered(&mut self, first_content: &str) -> Block {
        let mut items = Vec::new();

        // First item
        let inlines = parse_inline_content(first_content);
        items.push(vec![Block::Paragraph {
            inlines,
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

        let inlines = parse_inline_content(&content);
        Some(Block::Paragraph {
            inlines,
            span: Span::NONE,
        })
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

        // Line break: + at end of line
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
            {
                break;
            }
            // Check for image: or link:
            if c == 'i' || c == 'l' || c == 'h' {
                let remaining: String = chars[pos..].iter().take(8).collect();
                if remaining.starts_with("image:")
                    || remaining.starts_with("link:")
                    || remaining.starts_with("https://")
                    || remaining.starts_with("http://")
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
            span: Span::NONE,
        },
    ))
}

fn parse_link_macro(chars: &[char], start: usize) -> Option<(usize, Inline)> {
    // link:url[text]
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
    let link_text: String = chars[text_start..pos].iter().collect();
    pos += 1; // Skip ]

    let text = if link_text.is_empty() {
        url.clone()
    } else {
        link_text
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
            span: Span::NONE,
        },
    ))
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
            span: Span::NONE,
        },
    ))
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
