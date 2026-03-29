use crate::ast::*;

/// Parse a Creole string into a [`CreoleDoc`] and a list of diagnostics.
pub fn parse(input: &str) -> (CreoleDoc, Vec<Diagnostic>) {
    let mut p = Parser::new(input);
    let blocks = p.parse();
    (CreoleDoc { blocks }, Vec::new())
}

struct Parser<'a> {
    lines: Vec<&'a str>,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            lines: input.lines().collect(),
            pos: 0,
        }
    }

    fn parse(&mut self) -> Vec<Block> {
        let mut nodes = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            if line.trim().is_empty() {
                self.pos += 1;
                continue;
            }

            // Nowiki block {{{ ... }}}
            if line.trim_start().starts_with("{{{") {
                nodes.push(self.parse_nowiki_block());
                continue;
            }

            // Heading = to ======
            if let Some(node) = self.try_parse_heading(line) {
                nodes.push(node);
                self.pos += 1;
                continue;
            }

            // Horizontal rule ----
            if line.trim().starts_with("----") {
                nodes.push(Block::HorizontalRule(Span::NONE));
                self.pos += 1;
                continue;
            }

            // Table
            if line.trim_start().starts_with('|') {
                nodes.push(self.parse_table());
                continue;
            }

            // Definition list: ; term\n: definition
            let trimmed = line.trim_start();
            if trimmed.starts_with("; ") || trimmed == ";" {
                nodes.push(self.parse_definition_list());
                continue;
            }

            // Blockquote (> prefix)
            if trimmed.starts_with("> ") || trimmed == ">" {
                nodes.push(self.parse_blockquote());
                continue;
            }

            // List - but not bold **text**
            if (trimmed.starts_with('*') && !trimmed.starts_with("**"))
                || (trimmed.starts_with('#') && !trimmed.starts_with("##"))
            {
                nodes.push(self.parse_list());
                continue;
            }

            // Paragraph (or unrecognised line — always advance pos to prevent infinite loop).
            let prev_pos = self.pos;
            nodes.push(self.parse_paragraph());
            if self.pos == prev_pos {
                self.pos += 1;
            }
        }

        nodes
    }

    fn try_parse_heading(&self, line: &str) -> Option<Block> {
        let trimmed = line.trim_start();
        let level = trimmed.chars().take_while(|&c| c == '=').count();

        if level > 0 && level <= 6 {
            let rest = trimmed[level..].trim();
            // Remove trailing = if present
            let content = rest.trim_end_matches('=').trim();
            let inline_nodes = Self::parse_inline(content);

            return Some(Block::Heading {
                level: level as u8,
                inlines: inline_nodes,
                span: Span::NONE,
            });
        }
        None
    }

    fn parse_nowiki_block(&mut self) -> Block {
        let first_line = self.lines[self.pos];
        let content_start = first_line.find("{{{").unwrap() + 3;
        let mut content = String::new();

        // Check if it ends on the same line
        if let Some(end_pos) = first_line[content_start..].find("}}}") {
            content.push_str(&first_line[content_start..content_start + end_pos]);
            self.pos += 1;
        } else {
            // Multi-line nowiki
            if content_start < first_line.len() {
                content.push_str(&first_line[content_start..]);
                content.push('\n');
            }
            self.pos += 1;

            while self.pos < self.lines.len() {
                let line = self.lines[self.pos];
                if let Some(end_pos) = line.find("}}}") {
                    content.push_str(&line[..end_pos]);
                    self.pos += 1;
                    break;
                } else {
                    content.push_str(line);
                    content.push('\n');
                    self.pos += 1;
                }
            }
        }

        Block::CodeBlock { content, span: Span::NONE }
    }

    fn parse_table(&mut self) -> Block {
        let mut rows = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            if !line.trim_start().starts_with('|') {
                break;
            }

            let row = self.parse_table_row(line);
            rows.push(row);
            self.pos += 1;
        }

        Block::Table { rows, span: Span::NONE }
    }

    fn parse_table_row(&self, line: &str) -> TableRow {
        let mut cells = Vec::new();
        let trimmed = line.trim();

        // Split by | but skip empty first/last
        let parts: Vec<&str> = trimmed.split('|').collect();

        for part in parts {
            if part.is_empty() {
                continue;
            }

            let is_header = part.starts_with('=');
            let cell_content = if is_header {
                part[1..].trim()
            } else {
                part.trim()
            };

            let inline_nodes = Self::parse_inline(cell_content);
            cells.push(TableCell {
                is_header,
                inlines: inline_nodes,
                span: Span::NONE,
            });
        }

        TableRow { cells, span: Span::NONE }
    }

    fn parse_definition_list(&mut self) -> Block {
        let mut items = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            let trimmed = line.trim_start();

            if !trimmed.starts_with("; ") && trimmed != ";" {
                // Check if next line is a definition (: prefix)
                break;
            }

            // Parse the term
            let term_text = if trimmed == ";" { "" } else { trimmed[2..].trim() };
            let term = Self::parse_inline(term_text);
            self.pos += 1;

            // Parse the description (: prefix on the next line)
            let desc = if self.pos < self.lines.len() {
                let next = self.lines[self.pos].trim_start();
                if let Some(stripped) = next.strip_prefix(": ") {
                    let desc_text = stripped.trim();
                    self.pos += 1;
                    Self::parse_inline(desc_text)
                } else if next == ":" {
                    self.pos += 1;
                    Vec::new()
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            };

            items.push(DefinitionItem { term, desc });
        }

        Block::DefinitionList { items, span: Span::NONE }
    }

    fn parse_blockquote(&mut self) -> Block {
        let mut children = Vec::new();
        let mut text = String::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            let trimmed = line.trim_start();

            if let Some(stripped) = trimmed.strip_prefix("> ") {
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(stripped);
                self.pos += 1;
            } else if trimmed == ">" {
                // Empty blockquote line — flush current paragraph if any
                if !text.is_empty() {
                    let inlines = Self::parse_inline(&text);
                    children.push(Block::Paragraph { inlines, span: Span::NONE });
                    text.clear();
                }
                self.pos += 1;
            } else {
                break;
            }
        }

        if !text.is_empty() {
            let inlines = Self::parse_inline(&text);
            children.push(Block::Paragraph { inlines, span: Span::NONE });
        }

        Block::Blockquote { children, span: Span::NONE }
    }

    fn parse_list(&mut self) -> Block {
        let first_line = self.lines[self.pos];
        let first_char = first_line.trim_start().chars().next().unwrap();
        let ordered = first_char == '#';

        let (items, _) = self.parse_list_at_level(1, ordered);
        Block::List { ordered, items, span: Span::NONE }
    }

    fn parse_list_at_level(&mut self, level: usize, ordered: bool) -> (Vec<Vec<Block>>, usize) {
        let marker = if ordered { '#' } else { '*' };
        let mut items = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            let trimmed = line.trim_start();

            // Count markers
            let marker_count = trimmed.chars().take_while(|&c| c == marker).count();

            if marker_count == 0 {
                // Check for other list type
                let other_marker = if ordered { '*' } else { '#' };
                let other_count = trimmed.chars().take_while(|&c| c == other_marker).count();
                if other_count == 0 {
                    break;
                }
                // Different list type at same level - break
                if other_count == level {
                    break;
                }
            }

            if marker_count < level {
                break;
            }

            if marker_count == level {
                let content = trimmed[marker_count..].trim();
                let inline_nodes = Self::parse_inline(content);
                let para = Block::Paragraph {
                    inlines: inline_nodes,
                    span: Span::NONE,
                };
                let mut item_children = vec![para];

                self.pos += 1;

                // Check for nested list
                if self.pos < self.lines.len() {
                    let next_line = self.lines[self.pos];
                    let next_trimmed = next_line.trim_start();
                    let next_marker_count =
                        next_trimmed.chars().take_while(|&c| c == marker).count();
                    let other_marker = if ordered { '*' } else { '#' };
                    let next_other_count = next_trimmed
                        .chars()
                        .take_while(|&c| c == other_marker)
                        .count();

                    if next_marker_count > level {
                        let (nested, _) = self.parse_list_at_level(next_marker_count, ordered);
                        item_children.push(Block::List {
                            ordered,
                            items: nested,
                            span: Span::NONE,
                        });
                    } else if next_other_count > 0 {
                        let (nested, _) = self.parse_list_at_level(next_other_count, !ordered);
                        item_children.push(Block::List {
                            ordered: !ordered,
                            items: nested,
                            span: Span::NONE,
                        });
                    }
                }

                items.push(item_children);
            } else if marker_count > level {
                // Nested list - handled by item above
                break;
            }
        }

        (items, level)
    }

    fn parse_paragraph(&mut self) -> Block {
        let mut text = String::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            if line.trim().is_empty() {
                break;
            }

            // Check for block elements
            let trimmed = line.trim_start();
            if trimmed.starts_with('=')
                || trimmed.starts_with("----")
                || trimmed.starts_with('|')
                || (trimmed.starts_with('*') && !trimmed.starts_with("**"))
                || (trimmed.starts_with('#') && !trimmed.starts_with("##"))
                || trimmed.starts_with("{{{")
                || trimmed.starts_with("> ")
                || trimmed == ">"
                || trimmed.starts_with("; ")
            {
                break;
            }

            if !text.is_empty() {
                text.push(' ');
            }
            text.push_str(line.trim());
            self.pos += 1;
        }

        let inline_nodes = Self::parse_inline(&text);
        Block::Paragraph {
            inlines: inline_nodes,
            span: Span::NONE,
        }
    }

    fn parse_inline(text: &str) -> Vec<Inline> {
        let mut nodes = Vec::new();
        let mut current = String::new();
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // Escape character ~
            if chars[i] == '~' && i + 1 < chars.len() {
                current.push(chars[i + 1]);
                i += 2;
                continue;
            }

            // Line break \\
            if i + 1 < chars.len() && chars[i] == '\\' && chars[i + 1] == '\\' {
                if !current.is_empty() {
                    nodes.push(Inline::Text(current.clone(), Span::NONE));
                    current.clear();
                }
                nodes.push(Inline::LineBreak(Span::NONE));
                i += 2;
                continue;
            }

            // Inline nowiki {{{...}}}
            if i + 2 < chars.len()
                && chars[i] == '{'
                && chars[i + 1] == '{'
                && chars[i + 2] == '{'
            {
                if !current.is_empty() {
                    nodes.push(Inline::Text(current.clone(), Span::NONE));
                    current.clear();
                }

                i += 3;
                let mut code = String::new();
                while i + 2 < chars.len() {
                    if chars[i] == '}' && chars[i + 1] == '}' && chars[i + 2] == '}' {
                        i += 3;
                        break;
                    }
                    code.push(chars[i]);
                    i += 1;
                }
                nodes.push(Inline::Code(code, Span::NONE));
                continue;
            }

            // Bold **...**
            if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '*' {
                if !current.is_empty() {
                    nodes.push(Inline::Text(current.clone(), Span::NONE));
                    current.clear();
                }

                i += 2;
                let mut bold_text = String::new();
                while i + 1 < chars.len() {
                    if chars[i] == '*' && chars[i + 1] == '*' {
                        i += 2;
                        break;
                    }
                    bold_text.push(chars[i]);
                    i += 1;
                }
                let inner = Self::parse_inline(&bold_text);
                nodes.push(Inline::Bold(inner, Span::NONE));
                continue;
            }

            // Italic //...//
            if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '/' {
                // Make sure we're not in a URL (preceded by :)
                let preceded_by_colon = i > 0 && chars[i - 1] == ':';
                if !preceded_by_colon {
                    if !current.is_empty() {
                        nodes.push(Inline::Text(current.clone(), Span::NONE));
                        current.clear();
                    }

                    i += 2;
                    let mut italic_text = String::new();
                    while i + 1 < chars.len() {
                        if chars[i] == '/' && chars[i + 1] == '/' {
                            i += 2;
                            break;
                        }
                        italic_text.push(chars[i]);
                        i += 1;
                    }
                    let inner = Self::parse_inline(&italic_text);
                    nodes.push(Inline::Italic(inner, Span::NONE));
                    continue;
                }
            }

            // Link [[url|text]] or [[url]]
            if i + 1 < chars.len() && chars[i] == '[' && chars[i + 1] == '[' {
                if !current.is_empty() {
                    nodes.push(Inline::Text(current.clone(), Span::NONE));
                    current.clear();
                }

                i += 2;
                let mut link_content = String::new();
                while i + 1 < chars.len() {
                    if chars[i] == ']' && chars[i + 1] == ']' {
                        i += 2;
                        break;
                    }
                    link_content.push(chars[i]);
                    i += 1;
                }

                let (url, link_text) = if let Some(pipe_pos) = link_content.find('|') {
                    (
                        &link_content[..pipe_pos],
                        link_content[pipe_pos + 1..].to_string(),
                    )
                } else {
                    (link_content.as_str(), link_content.clone())
                };

                let text_node = Inline::Text(link_text, Span::NONE);
                nodes.push(Inline::Link {
                    url: url.to_string(),
                    children: vec![text_node],
                    span: Span::NONE,
                });
                continue;
            }

            // Image {{url|alt}} or {{url}}
            if i + 1 < chars.len() && chars[i] == '{' && chars[i + 1] == '{' {
                // Not inline nowiki (checked above)
                if i + 2 < chars.len() && chars[i + 2] != '{' {
                    if !current.is_empty() {
                        nodes.push(Inline::Text(current.clone(), Span::NONE));
                        current.clear();
                    }

                    i += 2;
                    let mut img_content = String::new();
                    while i + 1 < chars.len() {
                        if chars[i] == '}' && chars[i + 1] == '}' {
                            i += 2;
                            break;
                        }
                        img_content.push(chars[i]);
                        i += 1;
                    }

                    let (url, alt) = if let Some(pipe_pos) = img_content.find('|') {
                        (
                            &img_content[..pipe_pos],
                            Some(img_content[pipe_pos + 1..].to_string()),
                        )
                    } else {
                        (img_content.as_str(), None)
                    };

                    nodes.push(Inline::Image {
                        url: url.to_string(),
                        alt,
                        span: Span::NONE,
                    });
                    continue;
                }
            }

            current.push(chars[i]);
            i += 1;
        }

        if !current.is_empty() {
            nodes.push(Inline::Text(current, Span::NONE));
        }

        nodes
    }
}
