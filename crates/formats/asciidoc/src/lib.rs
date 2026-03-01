//! AsciiDoc parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-asciidoc` and `rescribe-write-asciidoc` as thin adapter layers.

#![allow(clippy::collapsible_if)]

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct AsciiDocError(pub String);

impl std::fmt::Display for AsciiDocError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AsciiDoc error: {}", self.0)
    }
}

impl std::error::Error for AsciiDocError {}

// ── AST ───────────────────────────────────────────────────────────────────────

/// A parsed AsciiDoc document.
#[derive(Debug, Clone, Default)]
pub struct AsciiDoc {
    pub blocks: Vec<Block>,
    pub attributes: std::collections::HashMap<String, String>,
}

/// Block-level element.
#[derive(Debug, Clone)]
pub enum Block {
    Paragraph {
        inlines: Vec<Inline>,
    },
    Heading {
        level: usize,
        inlines: Vec<Inline>,
    },
    CodeBlock {
        content: String,
        language: Option<String>,
    },
    Blockquote {
        children: Vec<Block>,
        attribution: Option<String>,
    },
    List {
        ordered: bool,
        items: Vec<Vec<Block>>,
    },
    DefinitionList {
        items: Vec<DefinitionItem>,
    },
    HorizontalRule,
    PageBreak,
    Figure {
        image: ImageData,
    },
    /// A generic div block with an optional CSS class.
    Div {
        class: Option<String>,
        children: Vec<Block>,
    },
    RawBlock {
        format: String,
        content: String,
    },
    Table {
        rows: Vec<TableRow>,
    },
}

/// An image (URL + optional alt, width, height).
#[derive(Debug, Clone)]
pub struct ImageData {
    pub url: String,
    pub alt: Option<String>,
    pub width: Option<String>,
    pub height: Option<String>,
}

/// A definition list item (term + description).
#[derive(Debug, Clone)]
pub struct DefinitionItem {
    pub term: Vec<Inline>,
    pub desc: Vec<Inline>,
}

/// A table row.
#[derive(Debug, Clone)]
pub struct TableRow {
    pub cells: Vec<Vec<Inline>>,
    pub is_header: bool,
}

/// Inline element.
#[derive(Debug, Clone)]
pub enum Inline {
    Text(String),
    Strong(Vec<Inline>),
    Emphasis(Vec<Inline>),
    Code(String),
    Superscript(Vec<Inline>),
    Subscript(Vec<Inline>),
    Highlight(Vec<Inline>),
    Strikeout(Vec<Inline>),
    Underline(Vec<Inline>),
    SmallCaps(Vec<Inline>),
    Quoted {
        quote_type: QuoteType,
        children: Vec<Inline>,
    },
    Link {
        url: String,
        children: Vec<Inline>,
    },
    Image(ImageData),
    LineBreak,
    SoftBreak,
    FootnoteRef {
        label: String,
    },
    FootnoteDef {
        label: String,
        children: Vec<Inline>,
    },
    MathInline {
        source: String,
    },
    MathDisplay {
        source: String,
    },
    RawInline {
        format: String,
        content: String,
    },
}

#[derive(Debug, Clone)]
pub enum QuoteType {
    Single,
    Double,
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Parse an AsciiDoc string into an [`AsciiDoc`] document.
pub fn parse(input: &str) -> Result<AsciiDoc, AsciiDocError> {
    let mut p = Parser::new(input);
    let (blocks, attributes) = p.parse_document();
    Ok(AsciiDoc { blocks, attributes })
}

struct Parser<'a> {
    lines: Vec<&'a str>,
    line_idx: usize,
    attributes: std::collections::HashMap<String, String>,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        let lines: Vec<&str> = input.lines().collect();
        Self {
            lines,
            line_idx: 0,
            attributes: std::collections::HashMap::new(),
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

    fn parse_document(&mut self) -> (Vec<Block>, std::collections::HashMap<String, String>) {
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
            return Some(Block::HorizontalRule);
        }

        // Page break
        if line == "<<<" {
            self.advance_line();
            return Some(Block::PageBreak);
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
        Some(Block::Heading { level, inlines })
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
                    children: vec![Block::Paragraph { inlines }],
                })
            }
            Some("SOURCE") => {
                let language = attrs.get(1).map(|s| s.to_string());

                // Expect delimiter
                self.skip_blank_lines();
                if let Some(delim_line) = self.current_line() {
                    if delim_line.starts_with("----") {
                        let content = self.collect_delimited_content("----");
                        return Some(Block::CodeBlock { content, language });
                    }
                }

                // Fallback: parse as regular content
                let content = self.collect_paragraph_content();
                Some(Block::CodeBlock { content, language })
            }
            Some("QUOTE") | Some("VERSE") => {
                self.skip_blank_lines();
                let content = self.collect_delimited_content_or_paragraph();
                let inlines = parse_inline_content(&content);
                let attribution = attrs.get(1).map(|s| s.to_string());
                Some(Block::Blockquote {
                    children: vec![Block::Paragraph { inlines }],
                    attribution,
                })
            }
            Some("EXAMPLE") => {
                self.skip_blank_lines();
                let content = self.collect_delimited_content_or_paragraph();
                let inlines = parse_inline_content(&content);
                Some(Block::Div {
                    class: Some("example".to_string()),
                    children: vec![Block::Paragraph { inlines }],
                })
            }
            Some("SIDEBAR") => {
                self.skip_blank_lines();
                let content = self.collect_delimited_content_or_paragraph();
                let inlines = parse_inline_content(&content);
                Some(Block::Div {
                    class: Some("sidebar".to_string()),
                    children: vec![Block::Paragraph { inlines }],
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
            },
            '=' => {
                let inlines = parse_inline_content(&content);
                Block::Div {
                    class: Some("example".to_string()),
                    children: vec![Block::Paragraph { inlines }],
                }
            }
            '*' => {
                let inlines = parse_inline_content(&content);
                Block::Div {
                    class: Some("sidebar".to_string()),
                    children: vec![Block::Paragraph { inlines }],
                }
            }
            '_' => {
                let inlines = parse_inline_content(&content);
                Block::Blockquote {
                    children: vec![Block::Paragraph { inlines }],
                    attribution: None,
                }
            }
            '+' => Block::RawBlock {
                format: "asciidoc".to_string(),
                content,
            },
            '.' => Block::CodeBlock {
                content,
                language: None,
            },
            _ => {
                let inlines = parse_inline_content(&content);
                Block::Div {
                    class: None,
                    children: vec![Block::Paragraph { inlines }],
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

    fn collect_paragraph_content(&mut self) -> String {
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
        if trimmed.contains("::") && !trimmed.starts_with("::") && !trimmed.starts_with("image::") {
            return self.parse_description_list();
        }

        None
    }

    fn parse_unordered_list(&mut self, marker: &str, first_content: &str) -> Block {
        let mut items = Vec::new();

        // First item
        let inlines = parse_inline_content(first_content);
        items.push(vec![Block::Paragraph { inlines }]);
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
                items.push(vec![Block::Paragraph { inlines }]);
                self.advance_line();
            } else {
                // Check for continuation
                if line.starts_with(' ') || line.starts_with('\t') {
                    // Continuation - append to last item's last paragraph
                    if let Some(last_item) = items.last_mut() {
                        let more_inlines = parse_inline_content(trimmed);
                        // Append to the last paragraph in the item
                        if let Some(Block::Paragraph { inlines }) = last_item.last_mut() {
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
        }
    }

    fn parse_ordered_list(&mut self, marker: &str, first_content: &str) -> Block {
        let mut items = Vec::new();

        // First item
        let inlines = parse_inline_content(first_content);
        items.push(vec![Block::Paragraph { inlines }]);
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
                items.push(vec![Block::Paragraph { inlines }]);
                self.advance_line();
            } else {
                break;
            }
        }

        Block::List {
            ordered: true,
            items,
        }
    }

    fn parse_ordered_list_numbered(&mut self, first_content: &str) -> Block {
        let mut items = Vec::new();

        // First item
        let inlines = parse_inline_content(first_content);
        items.push(vec![Block::Paragraph { inlines }]);
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
                    items.push(vec![Block::Paragraph { inlines }]);
                    self.advance_line();
                    continue;
                }
            }
            break;
        }

        Block::List {
            ordered: true,
            items,
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

        Some(Block::DefinitionList { items })
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
        })
    }

    fn parse_paragraph(&mut self) -> Option<Block> {
        let content = self.collect_paragraph_content();

        if content.is_empty() {
            return None;
        }

        let inlines = parse_inline_content(&content);
        Some(Block::Paragraph { inlines })
    }
}

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
                    nodes.push(Inline::Strong(parse_inline_content(&text)));
                    pos = end + 2;
                    continue;
                }
            }
            // Single *text*
            if let Some((end, text)) = find_closing_char(&chars, pos + 1, '*') {
                nodes.push(Inline::Strong(parse_inline_content(&text)));
                pos = end + 1;
                continue;
            }
        }

        // Emphasis: _text_ or __text__
        if chars[pos] == '_' {
            // Check for __text__ first
            if pos + 1 < chars.len() && chars[pos + 1] == '_' {
                if let Some((end, text)) = find_double_closing(&chars, pos + 2, '_') {
                    nodes.push(Inline::Emphasis(parse_inline_content(&text)));
                    pos = end + 2;
                    continue;
                }
            }
            // Single _text_
            if let Some((end, text)) = find_closing_char(&chars, pos + 1, '_') {
                nodes.push(Inline::Emphasis(parse_inline_content(&text)));
                pos = end + 1;
                continue;
            }
        }

        // Monospace: `text` or ``text``
        if chars[pos] == '`' {
            // Check for ``text`` first
            if pos + 1 < chars.len() && chars[pos + 1] == '`' {
                if let Some((end, text)) = find_double_closing(&chars, pos + 2, '`') {
                    nodes.push(Inline::Code(text));
                    pos = end + 2;
                    continue;
                }
            }
            // Single `text`
            if let Some((end, text)) = find_closing_char(&chars, pos + 1, '`') {
                nodes.push(Inline::Code(text));
                pos = end + 1;
                continue;
            }
        }

        // Superscript: ^text^
        if chars[pos] == '^' {
            if let Some((end, text)) = find_closing_char(&chars, pos + 1, '^') {
                nodes.push(Inline::Superscript(parse_inline_content(&text)));
                pos = end + 1;
                continue;
            }
        }

        // Subscript: ~text~
        if chars[pos] == '~' {
            if let Some((end, text)) = find_closing_char(&chars, pos + 1, '~') {
                nodes.push(Inline::Subscript(parse_inline_content(&text)));
                pos = end + 1;
                continue;
            }
        }

        // Highlight: #text#
        if chars[pos] == '#' {
            if let Some((end, text)) = find_closing_char(&chars, pos + 1, '#') {
                nodes.push(Inline::Highlight(parse_inline_content(&text)));
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
            nodes.push(Inline::LineBreak);
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
            nodes.push(Inline::Text(text));
        } else if pos == pos_before {
            // No markup matched and the text loop didn't advance — consume the
            // current character literally to guarantee forward progress.
            nodes.push(Inline::Text(chars[pos].to_string()));
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
        Inline::Image(ImageData {
            url: path,
            alt,
            width: None,
            height: None,
        }),
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

        let children = vec![Inline::Text(link_text)];
        return Some((pos, Inline::Link { url, children }));
    }

    // No text - URL is the text
    let children = vec![Inline::Text(url.clone())];
    Some((pos, Inline::Link { url, children }))
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

    let children = vec![Inline::Text(text)];
    Some((pos, Inline::Link { url, children }))
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
    let children = vec![Inline::Text(text)];
    Some((pos, Inline::Link { url, children }))
}

/// Merge adjacent text nodes.
fn merge_text_nodes(nodes: &mut Vec<Inline>) {
    let mut i = 0;
    while i + 1 < nodes.len() {
        let is_both_text =
            matches!(&nodes[i], Inline::Text(_)) && matches!(&nodes[i + 1], Inline::Text(_));
        if is_both_text {
            let next_text = match nodes.remove(i + 1) {
                Inline::Text(s) => s,
                _ => unreachable!(),
            };
            if let Inline::Text(ref mut s) = nodes[i] {
                s.push_str(&next_text);
            }
        } else {
            i += 1;
        }
    }
}

// ── Builder ───────────────────────────────────────────────────────────────────

/// Build an AsciiDoc string from an [`AsciiDoc`] document.
pub fn build(doc: &AsciiDoc) -> String {
    let mut ctx = BuildContext::new();
    build_blocks(&doc.blocks, &mut ctx);
    ctx.output
}

struct BuildContext {
    output: String,
    list_depth: usize,
}

impl BuildContext {
    fn new() -> Self {
        Self {
            output: String::new(),
            list_depth: 0,
        }
    }

    fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }
}

fn build_blocks(blocks: &[Block], ctx: &mut BuildContext) {
    for block in blocks {
        build_block(block, ctx);
    }
}

fn build_block(block: &Block, ctx: &mut BuildContext) {
    match block {
        Block::Paragraph { inlines } => {
            build_inlines(inlines, ctx);
            ctx.write("\n\n");
        }

        Block::Heading { level, inlines } => {
            // AsciiDoc uses = for headings (more = means deeper level)
            for _ in 0..=*level {
                ctx.write("=");
            }
            ctx.write(" ");
            build_inlines(inlines, ctx);
            ctx.write("\n\n");
        }

        Block::CodeBlock { content, language } => {
            if let Some(lang) = language {
                ctx.write("[source,");
                ctx.write(lang);
                ctx.write("]\n");
            }
            ctx.write("----\n");
            ctx.write(content);
            if !content.ends_with('\n') {
                ctx.write("\n");
            }
            ctx.write("----\n\n");
        }

        Block::Blockquote {
            children,
            attribution: _,
        } => {
            ctx.write("[quote]\n____\n");
            build_blocks(children, ctx);
            ctx.write("____\n\n");
        }

        Block::List { ordered, items } => {
            ctx.list_depth += 1;
            for item_blocks in items {
                // AsciiDoc uses * for unordered, . for ordered (repeated for depth)
                if *ordered {
                    for _ in 0..ctx.list_depth {
                        ctx.write(".");
                    }
                } else {
                    for _ in 0..ctx.list_depth {
                        ctx.write("*");
                    }
                }
                ctx.write(" ");

                // Emit item content — inline paragraphs flatten their content
                for child_block in item_blocks {
                    match child_block {
                        Block::Paragraph { inlines } => {
                            build_inlines(inlines, ctx);
                            ctx.write("\n");
                        }
                        other => build_block(other, ctx),
                    }
                }
            }
            ctx.list_depth -= 1;
            if ctx.list_depth == 0 {
                ctx.write("\n");
            }
        }

        Block::DefinitionList { items } => {
            for item in items {
                build_inlines(&item.term, ctx);
                ctx.write(":: ");
                build_inlines(&item.desc, ctx);
                ctx.write("\n");
            }
            ctx.write("\n");
        }

        Block::HorizontalRule => {
            ctx.write("'''\n\n");
        }

        Block::PageBreak => {
            ctx.write("<<<\n\n");
        }

        Block::Figure { image } => {
            ctx.write("image::");
            ctx.write(&image.url);
            ctx.write("[");
            if let Some(alt) = &image.alt {
                ctx.write(alt);
            }
            ctx.write("]\n\n");
        }

        Block::Div { children, .. } => {
            build_blocks(children, ctx);
        }

        Block::RawBlock { format, content } => {
            if format == "asciidoc" {
                ctx.write(content);
            }
        }

        Block::Table { rows } => {
            ctx.write("|===\n");
            let mut first_row = true;
            for row in rows {
                for cell in &row.cells {
                    ctx.write("| ");
                    build_inlines(cell, ctx);
                    ctx.write(" ");
                }
                ctx.write("\n");

                // Add blank line after header row
                if first_row || row.is_header {
                    ctx.write("\n");
                    first_row = false;
                }
            }
            ctx.write("|===\n\n");
        }
    }
}

fn build_inlines(inlines: &[Inline], ctx: &mut BuildContext) {
    for inline in inlines {
        build_inline(inline, ctx);
    }
}

fn build_inline(inline: &Inline, ctx: &mut BuildContext) {
    match inline {
        Inline::Text(s) => ctx.write(s),

        Inline::Strong(children) => {
            ctx.write("*");
            build_inlines(children, ctx);
            ctx.write("*");
        }

        Inline::Emphasis(children) => {
            ctx.write("_");
            build_inlines(children, ctx);
            ctx.write("_");
        }

        Inline::Code(s) => {
            ctx.write("`");
            ctx.write(s);
            ctx.write("`");
        }

        Inline::Superscript(children) => {
            ctx.write("^");
            build_inlines(children, ctx);
            ctx.write("^");
        }

        Inline::Subscript(children) => {
            ctx.write("~");
            build_inlines(children, ctx);
            ctx.write("~");
        }

        Inline::Highlight(children) => {
            ctx.write("#");
            build_inlines(children, ctx);
            ctx.write("#");
        }

        Inline::Strikeout(children) => {
            ctx.write("[line-through]#");
            build_inlines(children, ctx);
            ctx.write("#");
        }

        Inline::Underline(children) => {
            ctx.write("[underline]#");
            build_inlines(children, ctx);
            ctx.write("#");
        }

        Inline::SmallCaps(children) => {
            ctx.write("[small-caps]#");
            build_inlines(children, ctx);
            ctx.write("#");
        }

        Inline::Quoted {
            quote_type,
            children,
        } => match quote_type {
            QuoteType::Single => {
                ctx.write("'`");
                build_inlines(children, ctx);
                ctx.write("`'");
            }
            QuoteType::Double => {
                ctx.write("\"`");
                build_inlines(children, ctx);
                ctx.write("`\"");
            }
        },

        Inline::Link { url, children } => {
            ctx.write(url);
            ctx.write("[");
            build_inlines(children, ctx);
            ctx.write("]");
        }

        Inline::Image(img) => {
            ctx.write("image:");
            ctx.write(&img.url);
            ctx.write("[");
            if let Some(alt) = &img.alt {
                ctx.write(alt);
            }
            ctx.write("]");
        }

        Inline::LineBreak => ctx.write(" +\n"),

        Inline::SoftBreak => ctx.write("\n"),

        Inline::FootnoteRef { label } => {
            ctx.write("footnoteref:[");
            ctx.write(label);
            ctx.write("]");
        }

        Inline::FootnoteDef { label, children } => {
            ctx.write("footnotedef:[");
            ctx.write(label);
            ctx.write(",");
            build_inlines(children, ctx);
            ctx.write("]\n");
        }

        Inline::MathInline { source } => {
            ctx.write("stem:[");
            ctx.write(source);
            ctx.write("]");
        }

        Inline::MathDisplay { source } => {
            ctx.write("[stem]\n++++\n");
            ctx.write(source);
            ctx.write("\n++++\n\n");
        }

        Inline::RawInline { format, content } => {
            if format == "asciidoc" {
                ctx.write(content);
            }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_heading() {
        let doc = parse("== Hello World\n\nSome text.").unwrap();
        assert_eq!(doc.blocks.len(), 2);
        assert!(matches!(&doc.blocks[0], Block::Heading { level: 2, .. }));
    }

    #[test]
    fn test_parse_paragraph() {
        let doc = parse("This is a paragraph.\n\nThis is another.").unwrap();
        assert_eq!(doc.blocks.len(), 2);
        assert!(matches!(&doc.blocks[0], Block::Paragraph { .. }));
        assert!(matches!(&doc.blocks[1], Block::Paragraph { .. }));
    }

    #[test]
    fn test_parse_strong() {
        let doc = parse("This is *strong* text.").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Strong(_))));
    }

    #[test]
    fn test_parse_emphasis() {
        let doc = parse("This is _emphasized_ text.").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Emphasis(_))));
    }

    #[test]
    fn test_parse_bullet_list() {
        let doc = parse("* First item\n* Second item\n* Third item").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        let Block::List { ordered, items } = &doc.blocks[0] else {
            panic!("expected list");
        };
        assert!(!ordered);
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn test_parse_numbered_list() {
        let doc = parse(". First item\n. Second item\n. Third item").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        let Block::List { ordered, items } = &doc.blocks[0] else {
            panic!("expected list");
        };
        assert!(ordered);
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn test_parse_code_block() {
        let doc = parse("[source,python]\n----\nprint('hello')\n----").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        let Block::CodeBlock { language, content } = &doc.blocks[0] else {
            panic!("expected code block");
        };
        assert_eq!(language.as_deref(), Some("python"));
        assert!(content.contains("print('hello')"));
    }

    #[test]
    fn test_parse_inline_code() {
        let doc = parse("Use `code here` in text.").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Code(_))));
    }

    #[test]
    fn test_parse_link() {
        let doc = parse("Visit https://example.com[Example Site] for more.").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        let link = inlines.iter().find(|i| matches!(i, Inline::Link { .. }));
        assert!(link.is_some());
        if let Some(Inline::Link { url, .. }) = link {
            assert_eq!(url, "https://example.com");
        }
    }

    #[test]
    fn test_parse_block_image() {
        let doc = parse("image::path/to/image.png[Alt text]").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        let Block::Figure { image } = &doc.blocks[0] else {
            panic!("expected figure");
        };
        assert_eq!(image.url, "path/to/image.png");
        assert_eq!(image.alt.as_deref(), Some("Alt text"));
    }

    #[test]
    fn test_build_paragraph() {
        let doc = AsciiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Hello, world!".into())],
            }],
            attributes: Default::default(),
        };
        let out = build(&doc);
        assert!(out.contains("Hello, world!"));
    }

    #[test]
    fn test_build_heading() {
        let doc = AsciiDoc {
            blocks: vec![Block::Heading {
                level: 1,
                inlines: vec![Inline::Text("Title".into())],
            }],
            attributes: Default::default(),
        };
        let out = build(&doc);
        assert!(out.contains("== Title"));
    }

    #[test]
    fn test_build_code_block() {
        let doc = AsciiDoc {
            blocks: vec![Block::CodeBlock {
                content: "print('hi')".into(),
                language: Some("python".into()),
            }],
            attributes: Default::default(),
        };
        let out = build(&doc);
        assert!(out.contains("[source,python]"));
        assert!(out.contains("----"));
        assert!(out.contains("print('hi')"));
    }

    #[test]
    fn test_build_list() {
        let doc = AsciiDoc {
            blocks: vec![Block::List {
                ordered: false,
                items: vec![
                    vec![Block::Paragraph {
                        inlines: vec![Inline::Text("one".into())],
                    }],
                    vec![Block::Paragraph {
                        inlines: vec![Inline::Text("two".into())],
                    }],
                ],
            }],
            attributes: Default::default(),
        };
        let out = build(&doc);
        assert!(out.contains("* one"));
        assert!(out.contains("* two"));
    }

    #[test]
    fn test_build_strong() {
        let doc = AsciiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Strong(vec![Inline::Text("bold".into())])],
            }],
            attributes: Default::default(),
        };
        let out = build(&doc);
        assert!(out.contains("*bold*"));
    }

    #[test]
    fn test_build_emphasis() {
        let doc = AsciiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Emphasis(vec![Inline::Text("italic".into())])],
            }],
            attributes: Default::default(),
        };
        let out = build(&doc);
        assert!(out.contains("_italic_"));
    }

    #[test]
    fn test_build_inline_code() {
        let doc = AsciiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Code("code".into())],
            }],
            attributes: Default::default(),
        };
        let out = build(&doc);
        assert!(out.contains("`code`"));
    }

    #[test]
    fn test_build_link() {
        let doc = AsciiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "https://example.com".into(),
                    children: vec![Inline::Text("click".into())],
                }],
            }],
            attributes: Default::default(),
        };
        let out = build(&doc);
        assert!(out.contains("https://example.com[click]"));
    }
}
