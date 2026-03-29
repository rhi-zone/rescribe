use crate::ast::*;

/// Parse a DokuWiki string into a [`DokuwikiDoc`] and a list of diagnostics.
pub fn parse(input: &str) -> (DokuwikiDoc, Vec<Diagnostic>) {
    let mut p = Parser::new(input);
    let blocks = p.parse();
    (DokuwikiDoc { blocks }, p.diagnostics)
}

struct Parser<'a> {
    lines: Vec<&'a str>,
    pos: usize,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        let lines: Vec<&str> = input.lines().collect();
        Self {
            lines,
            pos: 0,
            diagnostics: Vec::new(),
        }
    }

    fn parse(&mut self) -> Vec<Block> {
        let mut blocks = Vec::new();

        while self.pos < self.lines.len() {
            let prev_pos = self.pos;
            if let Some(block) = self.parse_block() {
                blocks.push(block);
            }
            if self.pos == prev_pos {
                self.pos += 1;
            }
        }

        blocks
    }

    fn current_line(&self) -> Option<&'a str> {
        self.lines.get(self.pos).copied()
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn parse_block(&mut self) -> Option<Block> {
        let line = self.current_line()?;

        // Skip blank lines
        if line.trim().is_empty() {
            self.advance();
            return None;
        }

        let trimmed = line.trim();

        // Heading: ====== H1 ====== (6 =), ===== H2 ===== (5 =), etc.
        if trimmed.starts_with('=') && trimmed.ends_with('=') {
            return Some(self.parse_heading());
        }

        // Code block: <code> or <code lang>
        if trimmed.starts_with("<code") && (trimmed.len() == 5 || trimmed.as_bytes().get(5).is_some_and(|&b| b == b'>' || b == b' ')) {
            return Some(self.parse_code_block());
        }

        // File block: <file>
        if trimmed.starts_with("<file") && (trimmed.len() == 5 || trimmed.as_bytes().get(5).is_some_and(|&b| b == b'>' || b == b' ')) {
            return Some(self.parse_file_block());
        }

        // HTML block: <html>
        if trimmed.starts_with("<html") && (trimmed.len() == 5 || trimmed.as_bytes().get(5).is_some_and(|&b| b == b'>' || b == b' ')) {
            return Some(self.parse_raw_block("html"));
        }

        // PHP block: <php>
        if trimmed.starts_with("<php") && (trimmed.len() == 4 || trimmed.as_bytes().get(4).is_some_and(|&b| b == b'>' || b == b' ')) {
            return Some(self.parse_raw_block("php"));
        }

        // Table: starts with ^ or |
        if trimmed.starts_with('^') || trimmed.starts_with('|') {
            return Some(self.parse_table());
        }

        // List: starts with spaces and * or -
        if line.starts_with("  ")
            && (line.trim_start().starts_with("* ") || line.trim_start().starts_with("- "))
        {
            return Some(self.parse_list());
        }

        // Definition list: ; term
        if trimmed.starts_with(';') {
            return Some(self.parse_definition_list());
        }

        // Blockquote: > text
        if trimmed.starts_with('>') {
            return Some(self.parse_blockquote());
        }

        // Horizontal rule: ----
        if trimmed == "----" {
            self.advance();
            return Some(Block::HorizontalRule(Span::NONE));
        }

        // Control macros: ~~NOTOC~~, ~~NOCACHE~~
        if trimmed.starts_with("~~") && trimmed.ends_with("~~") && trimmed.len() > 4 {
            let name = &trimmed[2..trimmed.len() - 2];
            if name.chars().all(|c| c.is_ascii_uppercase()) {
                self.advance();
                return Some(Block::Macro {
                    name: name.to_string(),
                    span: Span::NONE,
                });
            }
        }

        // Default: paragraph
        Some(self.parse_paragraph())
    }

    fn parse_heading(&mut self) -> Block {
        let line = self.current_line().unwrap();
        self.advance();

        let trimmed = line.trim();

        // Count leading = signs
        let leading = trimmed.chars().take_while(|c| *c == '=').count();
        // DokuWiki uses 6 = for H1, 5 for H2, etc.
        let level = (7 - leading.min(6)) as u8;

        // Extract content between = signs
        let content = trimmed
            .trim_start_matches('=')
            .trim_end_matches('=')
            .trim();

        Block::Heading {
            level,
            inlines: parse_inline(content),
            span: Span::NONE,
        }
    }

    fn parse_code_block(&mut self) -> Block {
        let line = self.current_line().unwrap();
        self.advance();

        let lang = extract_tag_attr(line, "code");

        let mut content = String::new();
        while let Some(line) = self.current_line() {
            if line.contains("</code>") {
                self.advance();
                break;
            }
            if !content.is_empty() {
                content.push('\n');
            }
            content.push_str(line);
            self.advance();
        }

        Block::CodeBlock {
            language: lang,
            content,
            span: Span::NONE,
        }
    }

    fn parse_file_block(&mut self) -> Block {
        let line = self.current_line().unwrap();
        self.advance();

        // <file lang filename> or <file lang> or <file>
        let attr = extract_tag_attr(line, "file");
        let (language, filename) = if let Some(a) = attr {
            let parts: Vec<&str> = a.splitn(2, ' ').collect();
            if parts.len() == 2 {
                (Some(parts[0].to_string()), Some(parts[1].to_string()))
            } else {
                (Some(parts[0].to_string()), None)
            }
        } else {
            (None, None)
        };

        let mut content = String::new();
        while let Some(line) = self.current_line() {
            if line.contains("</file>") {
                self.advance();
                break;
            }
            if !content.is_empty() {
                content.push('\n');
            }
            content.push_str(line);
            self.advance();
        }

        Block::FileBlock {
            language,
            filename,
            content,
            span: Span::NONE,
        }
    }

    fn parse_raw_block(&mut self, format: &str) -> Block {
        let end_tag = format!("</{}>", format);
        self.advance();

        let mut content = String::new();
        while let Some(line) = self.current_line() {
            if line.contains(&end_tag) {
                self.advance();
                break;
            }
            if !content.is_empty() {
                content.push('\n');
            }
            content.push_str(line);
            self.advance();
        }

        Block::RawBlock {
            format: format.to_string(),
            content,
            span: Span::NONE,
        }
    }

    fn parse_table(&mut self) -> Block {
        let mut rows = Vec::new();

        while let Some(line) = self.current_line() {
            let trimmed = line.trim();
            if !trimmed.starts_with('^') && !trimmed.starts_with('|') {
                break;
            }

            let is_header = trimmed.starts_with('^');
            let mut cells = Vec::new();

            // Parse cells: split by ^ or | delimiters
            let chars: Vec<char> = trimmed.chars().collect();
            let mut i = 1; // skip leading delimiter
            let mut cell_text = String::new();

            while i < chars.len() {
                if chars[i] == '^' || chars[i] == '|' {
                    let cell_content = cell_text.trim().to_string();
                    cells.push(TableCell {
                        inlines: parse_inline(&cell_content),
                    });
                    cell_text.clear();
                } else {
                    cell_text.push(chars[i]);
                }
                i += 1;
            }

            if !cells.is_empty() {
                rows.push(TableRow { cells, is_header });
            }
            self.advance();
        }

        Block::Table {
            rows,
            span: Span::NONE,
        }
    }

    fn parse_list(&mut self) -> Block {
        let first_char = self
            .current_line()
            .and_then(|l| l.trim_start().chars().next());
        let ordered = first_char == Some('-');

        let items = self.parse_list_items(1);

        Block::List {
            ordered,
            items,
            span: Span::NONE,
        }
    }

    fn parse_list_items(&mut self, expected_depth: usize) -> Vec<ListItem> {
        let mut items: Vec<ListItem> = Vec::new();

        while let Some(line) = self.current_line() {
            if !line.starts_with("  ") {
                break;
            }
            let trimmed = line.trim_start();
            if !trimmed.starts_with("* ") && !trimmed.starts_with("- ") {
                break;
            }

            // Calculate depth: number of leading 2-space indents
            let indent = line.len() - line.trim_start().len();
            let depth = indent / 2;

            if depth < expected_depth {
                break;
            }

            if depth > expected_depth {
                // Nested list — parse as children of the last item
                let nested_ordered = trimmed.starts_with('-');
                let nested_items = self.parse_list_items(depth);
                if let Some(last) = items.last_mut() {
                    last.children.push(Block::List {
                        ordered: nested_ordered,
                        items: nested_items,
                        span: Span::NONE,
                    });
                }
                continue;
            }

            // Same depth: regular item
            let content = trimmed[2..].trim_start();
            let item_inlines = parse_inline(content);
            items.push(ListItem {
                inlines: item_inlines,
                children: Vec::new(),
            });
            self.advance();
        }

        items
    }

    fn parse_definition_list(&mut self) -> Block {
        let mut items = Vec::new();

        while let Some(line) = self.current_line() {
            let trimmed = line.trim();
            if let Some(stripped) = trimmed.strip_prefix(';') {
                let term_text = stripped.trim();
                let term = parse_inline(term_text);
                self.advance();

                // Look for : definition line
                let desc = if let Some(next_line) = self.current_line() {
                    let next_trimmed = next_line.trim();
                    if let Some(desc_stripped) = next_trimmed.strip_prefix(':') {
                        let desc_text = desc_stripped.trim();
                        self.advance();
                        parse_inline(desc_text)
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };

                items.push(DefinitionItem { term, desc });
            } else {
                break;
            }
        }

        Block::DefinitionList {
            items,
            span: Span::NONE,
        }
    }

    fn parse_blockquote(&mut self) -> Block {
        let mut lines = Vec::new();

        while let Some(line) = self.current_line() {
            let trimmed = line.trim();
            if !trimmed.starts_with('>') {
                break;
            }
            let content = trimmed[1..].trim_start();
            lines.push(content);
            self.advance();
        }

        let text = lines.join(" ");
        Block::Blockquote {
            children: vec![Block::Paragraph {
                inlines: parse_inline(&text),
                span: Span::NONE,
            }],
            span: Span::NONE,
        }
    }

    fn parse_paragraph(&mut self) -> Block {
        let mut lines = Vec::new();

        while let Some(line) = self.current_line() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                break;
            }
            // Stop at block-level elements
            if (trimmed.starts_with('=') && trimmed.ends_with('='))
                || (trimmed.starts_with("<code") && (trimmed.len() == 5 || trimmed.as_bytes().get(5).is_some_and(|&b| b == b'>' || b == b' ')))
                || (trimmed.starts_with("<file") && (trimmed.len() == 5 || trimmed.as_bytes().get(5).is_some_and(|&b| b == b'>' || b == b' ')))
                || (trimmed.starts_with("<html") && (trimmed.len() == 5 || trimmed.as_bytes().get(5).is_some_and(|&b| b == b'>' || b == b' ')))
                || (trimmed.starts_with("<php") && (trimmed.len() == 4 || trimmed.as_bytes().get(4).is_some_and(|&b| b == b'>' || b == b' ')))
                || (line.starts_with("  ") && (trimmed.starts_with("* ") || trimmed.starts_with("- ")))
                || trimmed.starts_with('>')
                || trimmed.starts_with('^')
                || trimmed.starts_with('|')
                || trimmed.starts_with(';')
                || trimmed == "----"
                || (trimmed.starts_with("~~") && trimmed.ends_with("~~") && trimmed.len() > 4)
            {
                break;
            }
            lines.push(trimmed);
            self.advance();
        }

        let text = lines.join(" ");
        Block::Paragraph {
            inlines: parse_inline(&text),
            span: Span::NONE,
        }
    }
}

/// Extract the attribute from a tag like `<code rust>` -> Some("rust"), `<code>` -> None.
fn extract_tag_attr(line: &str, tag: &str) -> Option<String> {
    let start = line.find('<')?;
    let after = &line[start..];
    let end = after.find('>')?;
    let inner = &after[1..end];
    let parts: Vec<&str> = inner.splitn(2, char::is_whitespace).collect();
    if parts[0] != tag {
        return None;
    }
    if parts.len() > 1 {
        Some(parts[1].trim().to_string())
    } else {
        None
    }
}

/// Parse inline markup from a text string.
pub(crate) fn parse_inline(text: &str) -> Vec<Inline> {
    let mut nodes = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Nowiki: %%...%%
        if i + 1 < chars.len() && chars[i] == '%' && chars[i + 1] == '%' {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            if let Some((content, end)) = find_double_delim(&chars, i + 2, '%') {
                nodes.push(Inline::Nowiki(content, Span::NONE));
                i = end + 2;
                continue;
            }
        }

        // Footnote: ((content))
        if i + 1 < chars.len() && chars[i] == '(' && chars[i + 1] == '(' {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            if let Some((content, end)) = find_double_paren(&chars, i + 2) {
                nodes.push(Inline::FootnoteRef {
                    content,
                    span: Span::NONE,
                });
                i = end + 2;
                continue;
            }
        }

        // Bold: **text**
        if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '*' {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            if let Some((content, end)) = find_double_delim(&chars, i + 2, '*') {
                nodes.push(Inline::Bold(parse_inline(&content), Span::NONE));
                i = end + 2;
                continue;
            }
        }

        // Italic: //text//
        if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '/' {
            // Don't match URLs like http://
            let is_url = i > 0 && chars[i - 1] == ':';
            if !is_url {
                if !current.is_empty() {
                    nodes.push(Inline::Text(current.clone(), Span::NONE));
                    current.clear();
                }
                if let Some((content, end)) = find_double_delim(&chars, i + 2, '/') {
                    nodes.push(Inline::Italic(parse_inline(&content), Span::NONE));
                    i = end + 2;
                    continue;
                }
            }
        }

        // Underline: __text__
        if i + 1 < chars.len() && chars[i] == '_' && chars[i + 1] == '_' {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            if let Some((content, end)) = find_double_delim(&chars, i + 2, '_') {
                nodes.push(Inline::Underline(parse_inline(&content), Span::NONE));
                i = end + 2;
                continue;
            }
        }

        // Monospace: ''text''
        if i + 1 < chars.len() && chars[i] == '\'' && chars[i + 1] == '\'' {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            if let Some((content, end)) = find_double_delim(&chars, i + 2, '\'') {
                nodes.push(Inline::Code(content, Span::NONE));
                i = end + 2;
                continue;
            }
        }

        // Strikethrough: <del>text</del>
        if i + 4 < chars.len() && text[i..].get(..5) == Some("<del>") {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            if let Some(end_pos) = text[i + 5..].find("</del>") {
                let content = &text[i + 5..i + 5 + end_pos];
                nodes.push(Inline::Strikethrough(parse_inline(content), Span::NONE));
                i += 5 + end_pos + 6;
                continue;
            }
        }

        // Superscript: <sup>text</sup>
        if i + 4 < chars.len() && text[i..].get(..5) == Some("<sup>") {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            if let Some(end_pos) = text[i + 5..].find("</sup>") {
                let content = &text[i + 5..i + 5 + end_pos];
                nodes.push(Inline::Superscript(parse_inline(content), Span::NONE));
                i += 5 + end_pos + 6;
                continue;
            }
        }

        // Subscript: <sub>text</sub>
        if i + 4 < chars.len() && text[i..].get(..5) == Some("<sub>") {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            if let Some(end_pos) = text[i + 5..].find("</sub>") {
                let content = &text[i + 5..i + 5 + end_pos];
                nodes.push(Inline::Subscript(parse_inline(content), Span::NONE));
                i += 5 + end_pos + 6;
                continue;
            }
        }

        // Link: [[url|text]]
        if i + 1 < chars.len() && chars[i] == '[' && chars[i + 1] == '[' {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            if let Some((link_content, end)) = find_double_bracket(&chars, i + 2) {
                let (url, link_text) = if let Some(pipe_pos) = link_content.find('|') {
                    (&link_content[..pipe_pos], &link_content[pipe_pos + 1..])
                } else {
                    (link_content.as_str(), link_content.as_str())
                };
                nodes.push(Inline::Link {
                    url: url.to_string(),
                    children: vec![Inline::Text(link_text.to_string(), Span::NONE)],
                    span: Span::NONE,
                });
                i = end + 2;
                continue;
            }
        }

        // Image: {{url|alt}}
        if i + 1 < chars.len() && chars[i] == '{' && chars[i + 1] == '{' {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            if let Some((img_content, end)) = find_double_brace(&chars, i + 2) {
                let (url, alt) = if let Some(pipe_pos) = img_content.find('|') {
                    (
                        &img_content[..pipe_pos],
                        Some(&img_content[pipe_pos + 1..]),
                    )
                } else {
                    (img_content.as_str(), None)
                };
                nodes.push(Inline::Image {
                    url: url.to_string(),
                    alt: alt.map(|s| s.to_string()),
                    span: Span::NONE,
                });
                i = end + 2;
                continue;
            }
        }

        // Line break: \\ followed by space or end of text
        if i + 1 < chars.len() && chars[i] == '\\' && chars[i + 1] == '\\' {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            nodes.push(Inline::LineBreak(Span::NONE));
            i += 2;
            // Skip optional trailing space
            if i < chars.len() && chars[i] == ' ' {
                i += 1;
            }
            continue;
        }

        current.push(chars[i]);
        i += 1;
    }

    if !current.is_empty() {
        nodes.push(Inline::Text(current, Span::NONE));
    }

    nodes
}

fn find_double_delim(chars: &[char], start: usize, delim: char) -> Option<(String, usize)> {
    let mut i = start;
    let mut content = String::new();

    while i + 1 < chars.len() {
        if chars[i] == delim && chars[i + 1] == delim {
            return Some((content, i));
        }
        content.push(chars[i]);
        i += 1;
    }

    None
}

fn find_double_bracket(chars: &[char], start: usize) -> Option<(String, usize)> {
    let mut i = start;
    let mut content = String::new();

    while i + 1 < chars.len() {
        if chars[i] == ']' && chars[i + 1] == ']' {
            return Some((content, i));
        }
        content.push(chars[i]);
        i += 1;
    }

    None
}

fn find_double_brace(chars: &[char], start: usize) -> Option<(String, usize)> {
    let mut i = start;
    let mut content = String::new();

    while i + 1 < chars.len() {
        if chars[i] == '}' && chars[i + 1] == '}' {
            return Some((content, i));
        }
        content.push(chars[i]);
        i += 1;
    }

    None
}

fn find_double_paren(chars: &[char], start: usize) -> Option<(String, usize)> {
    let mut i = start;
    let mut content = String::new();

    while i + 1 < chars.len() {
        if chars[i] == ')' && chars[i + 1] == ')' {
            return Some((content, i));
        }
        content.push(chars[i]);
        i += 1;
    }

    None
}
