//! Jira wiki markup parser.

use crate::ast::*;

/// Parse a Jira string into a [`JiraDoc`].
pub fn parse(input: &str) -> (JiraDoc, Vec<Diagnostic>) {
    let mut p = Parser::new(input);
    let blocks = p.parse().unwrap_or_default();
    (JiraDoc { blocks, span: Span::NONE }, vec![])
}

struct Parser<'a> {
    lines: Vec<&'a str>,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        let lines: Vec<&str> = input.lines().collect();
        Self { lines, pos: 0 }
    }

    fn parse(&mut self) -> Result<Vec<Block>, String> {
        let mut blocks = Vec::new();

        while self.pos < self.lines.len() {
            let prev_pos = self.pos;
            if let Some(block) = self.parse_block()? {
                blocks.push(block);
            }
            // Safety: prevent infinite loop when parse_block consumed nothing.
            if self.pos == prev_pos {
                self.pos += 1;
            }
        }

        Ok(blocks)
    }

    fn current_line(&self) -> Option<&'a str> {
        self.lines.get(self.pos).copied()
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn parse_block(&mut self) -> Result<Option<Block>, String> {
        let line = match self.current_line() {
            Some(l) => l,
            None => return Ok(None),
        };

        // Skip blank lines
        if line.trim().is_empty() {
            self.advance();
            return Ok(None);
        }

        // Heading: h1. to h6.
        for level in 1..=6u8 {
            let prefix = format!("h{}. ", level);
            if let Some(rest) = line.strip_prefix(prefix.as_str()) {
                self.advance();
                return Ok(Some(Block::Heading {
                    level,
                    inlines: self.parse_inline(rest),
                    span: Span::NONE,
                }));
            }
        }

        // Code block: {code} or {code:lang}
        if line.starts_with("{code") {
            return Ok(Some(self.parse_code_block()?));
        }

        // Noformat block: {noformat}
        if line.trim() == "{noformat}" {
            return Ok(Some(self.parse_noformat_block()?));
        }

        // Quote block: {quote}
        if line.trim() == "{quote}" {
            return Ok(Some(self.parse_quote_block()?));
        }

        // Panel: {panel} or {panel:...}
        if line.starts_with("{panel") {
            return Ok(Some(self.parse_panel_block()?));
        }

        // Lists: * or # (but not ** which could be bold at start-of-line if not followed by space)
        if self.is_list_line(line) {
            return Ok(Some(self.parse_list()?));
        }

        // Table: starts with | or ||
        if line.starts_with('|') {
            return Ok(Some(self.parse_table()?));
        }

        // Horizontal rule: ----
        if line.trim() == "----" {
            self.advance();
            return Ok(Some(Block::HorizontalRule { span: Span::NONE }));
        }

        // Default: paragraph
        Ok(Some(self.parse_paragraph()?))
    }

    fn is_list_line(&self, line: &str) -> bool {
        // A list line starts with one or more * or # followed by a space
        let mut chars = line.chars();
        match chars.next() {
            Some('*') | Some('#') => {}
            _ => return false,
        }
        let first = line.chars().next().unwrap();
        for c in chars {
            if c == first {
                continue;
            }
            return c == ' ';
        }
        false
    }

    fn parse_code_block(&mut self) -> Result<Block, String> {
        let line = self.current_line().unwrap();
        self.advance();

        // Extract language from {code:lang}
        let language = if let Some(rest) = line.strip_prefix("{code:") {
            rest.strip_suffix('}').map(|s| s.to_string())
        } else {
            None
        };

        let mut content = String::new();
        while let Some(line) = self.current_line() {
            if line.trim() == "{code}" {
                self.advance();
                break;
            }
            if !content.is_empty() {
                content.push('\n');
            }
            content.push_str(line);
            self.advance();
        }

        Ok(Block::CodeBlock { content, language, span: Span::NONE })
    }

    fn parse_noformat_block(&mut self) -> Result<Block, String> {
        self.advance(); // Skip {noformat}
        let mut content = String::new();
        while let Some(line) = self.current_line() {
            if line.trim() == "{noformat}" {
                self.advance();
                break;
            }
            if !content.is_empty() {
                content.push('\n');
            }
            content.push_str(line);
            self.advance();
        }
        Ok(Block::Noformat { content, span: Span::NONE })
    }

    fn parse_quote_block(&mut self) -> Result<Block, String> {
        self.advance(); // Skip {quote}
        let mut children = Vec::new();

        while let Some(line) = self.current_line() {
            if line.trim() == "{quote}" {
                self.advance();
                break;
            }
            if line.trim().is_empty() {
                self.advance();
                continue;
            }
            children.push(Block::Paragraph {
                inlines: self.parse_inline(line),
                span: Span::NONE,
            });
            self.advance();
        }

        Ok(Block::Blockquote { children, span: Span::NONE })
    }

    fn parse_panel_block(&mut self) -> Result<Block, String> {
        let line = self.current_line().unwrap();

        // Extract title from {panel:title=...}
        let title = if let Some(rest) = line.strip_prefix("{panel:") {
            let params = rest.strip_suffix('}').unwrap_or(rest);
            // Parse key=value pairs separated by |
            params.split('|').find_map(|pair| {
                let pair = pair.trim();
                if let Some(val) = pair.strip_prefix("title=") {
                    Some(val.to_string())
                } else {
                    None
                }
            })
        } else {
            None
        };

        self.advance(); // Skip {panel...}
        let mut children = Vec::new();

        while let Some(line) = self.current_line() {
            if line.trim() == "{panel}" {
                self.advance();
                break;
            }
            if line.trim().is_empty() {
                self.advance();
                continue;
            }
            children.push(Block::Paragraph {
                inlines: self.parse_inline(line),
                span: Span::NONE,
            });
            self.advance();
        }

        Ok(Block::Panel { title, children, span: Span::NONE })
    }

    fn parse_list(&mut self) -> Result<Block, String> {
        let first_line = self.current_line().unwrap();
        let marker = first_line.chars().next().unwrap();
        let ordered = marker == '#';

        self.parse_list_at_depth(marker, 1, ordered)
    }

    fn parse_list_at_depth(
        &mut self,
        marker: char,
        depth: usize,
        ordered: bool,
    ) -> Result<Block, String> {
        let mut items: Vec<ListItem> = Vec::new();

        while let Some(line) = self.current_line() {
            // Count the marker depth of this line
            let line_marker = line.chars().next().unwrap_or('\0');
            if line_marker != '*' && line_marker != '#' {
                break;
            }
            let line_depth = line.chars().take_while(|&c| c == line_marker).count();

            // Check that the marker character at our depth matches
            if line_depth < depth {
                break; // belongs to parent list
            }

            if line_depth == depth && line_marker == marker {
                let content = line[depth..].trim_start();
                let mut item_children = vec![ListItemContent::Inline(self.parse_inline(content))];
                self.advance();

                // Check for nested sublists
                while let Some(next_line) = self.current_line() {
                    let next_marker = next_line.chars().next().unwrap_or('\0');
                    if next_marker != '*' && next_marker != '#' {
                        break;
                    }
                    let next_depth =
                        next_line.chars().take_while(|&c| c == next_marker).count();
                    if next_depth <= depth {
                        break;
                    }
                    // Nested list
                    let sub_ordered = next_marker == '#';
                    let sublist =
                        self.parse_list_at_depth(next_marker, next_depth, sub_ordered)?;
                    item_children.push(ListItemContent::NestedList(sublist));
                }

                items.push(ListItem { children: item_children });
            } else {
                // Different marker at same depth or deeper — break out
                break;
            }
        }

        Ok(Block::List { ordered, items, span: Span::NONE })
    }

    fn parse_table(&mut self) -> Result<Block, String> {
        let mut rows = Vec::new();

        while let Some(line) = self.current_line() {
            if !line.starts_with('|') {
                break;
            }

            // Check if header row (starts with ||)
            let is_header = line.starts_with("||");
            let cells: Vec<TableCell> = if is_header {
                line.split("||")
                    .filter(|s| !s.is_empty())
                    .map(|cell| TableCell {
                        is_header: true,
                        inlines: self.parse_inline(cell.trim()),
                        span: Span::NONE,
                    })
                    .collect()
            } else {
                line.split('|')
                    .filter(|s| !s.is_empty())
                    .map(|cell| TableCell {
                        is_header: false,
                        inlines: self.parse_inline(cell.trim()),
                        span: Span::NONE,
                    })
                    .collect()
            };

            rows.push(TableRow { cells, span: Span::NONE });
            self.advance();
        }

        Ok(Block::Table { rows, span: Span::NONE })
    }

    fn parse_paragraph(&mut self) -> Result<Block, String> {
        let mut lines = Vec::new();

        while let Some(line) = self.current_line() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                break;
            }
            // Stop at block-level elements
            if trimmed.starts_with("h1. ")
                || trimmed.starts_with("h2. ")
                || trimmed.starts_with("h3. ")
                || trimmed.starts_with("h4. ")
                || trimmed.starts_with("h5. ")
                || trimmed.starts_with("h6. ")
                || trimmed.starts_with("{code")
                || trimmed == "{quote}"
                || trimmed.starts_with("{panel")
                || trimmed == "{noformat}"
                || self.is_list_line(trimmed)
                || trimmed.starts_with('|')
                || trimmed == "----"
            {
                break;
            }
            lines.push(trimmed);
            self.advance();
        }

        let text = lines.join(" ");
        Ok(Block::Paragraph {
            inlines: self.parse_inline(&text),
            span: Span::NONE,
        })
    }

    fn parse_inline(&self, text: &str) -> Vec<Inline> {
        let mut inlines = Vec::new();
        let mut current = String::new();
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // Color macro: {color:xxx}...{color}
            if chars[i] == '{' && i + 6 < chars.len() {
                let rest: String = chars[i..].iter().collect();
                if let Some(color_content) = rest.strip_prefix("{color:") {
                    if let Some(close_brace) = color_content.find('}') {
                        let color = &color_content[..close_brace];
                        let after_open = i + 7 + close_brace; // past {color:xxx}
                        // Find matching {color}
                        let remaining: String = chars[after_open..].iter().collect();
                        if let Some(end_pos) = remaining.find("{color}") {
                            if !current.is_empty() {
                                inlines.push(Inline::Text(current.clone(), Span::NONE));
                                current.clear();
                            }
                            let inner = &remaining[..end_pos];
                            let children = self.parse_inline(inner);
                            inlines.push(Inline::ColorSpan {
                                color: color.to_string(),
                                children,
                                span: Span::NONE,
                            });
                            i = after_open + end_pos + 7; // past {color}
                            continue;
                        }
                    }
                }
            }

            // Monospace: {{text}}
            if i + 1 < chars.len() && chars[i] == '{' && chars[i + 1] == '{' {
                if !current.is_empty() {
                    inlines.push(Inline::Text(current.clone(), Span::NONE));
                    current.clear();
                }
                if let Some((content, end)) = self.find_double_brace(&chars, i + 2) {
                    inlines.push(Inline::Code(content, Span::NONE));
                    i = end + 2;
                    continue;
                }
            }

            // Bold: *text*
            if chars[i] == '*' {
                if !current.is_empty() {
                    inlines.push(Inline::Text(current.clone(), Span::NONE));
                    current.clear();
                }
                if let Some((content, end)) = self.find_delim(&chars, i + 1, '*') {
                    let children = self.parse_inline(&content);
                    inlines.push(Inline::Bold(children, Span::NONE));
                    i = end + 1;
                    continue;
                }
            }

            // Italic: _text_
            if chars[i] == '_' {
                if !current.is_empty() {
                    inlines.push(Inline::Text(current.clone(), Span::NONE));
                    current.clear();
                }
                if let Some((content, end)) = self.find_delim(&chars, i + 1, '_') {
                    let children = self.parse_inline(&content);
                    inlines.push(Inline::Italic(children, Span::NONE));
                    i = end + 1;
                    continue;
                }
            }

            // Strikethrough: -text-
            if chars[i] == '-' {
                if !current.is_empty() {
                    inlines.push(Inline::Text(current.clone(), Span::NONE));
                    current.clear();
                }
                if let Some((content, end)) = self.find_delim(&chars, i + 1, '-') {
                    let children = self.parse_inline(&content);
                    inlines.push(Inline::Strikethrough(children, Span::NONE));
                    i = end + 1;
                    continue;
                }
            }

            // Underline: +text+
            if chars[i] == '+' {
                if !current.is_empty() {
                    inlines.push(Inline::Text(current.clone(), Span::NONE));
                    current.clear();
                }
                if let Some((content, end)) = self.find_delim(&chars, i + 1, '+') {
                    let children = self.parse_inline(&content);
                    inlines.push(Inline::Underline(children, Span::NONE));
                    i = end + 1;
                    continue;
                }
            }

            // Superscript: ^text^
            if chars[i] == '^' {
                if !current.is_empty() {
                    inlines.push(Inline::Text(current.clone(), Span::NONE));
                    current.clear();
                }
                if let Some((content, end)) = self.find_delim(&chars, i + 1, '^') {
                    let children = self.parse_inline(&content);
                    inlines.push(Inline::Superscript(children, Span::NONE));
                    i = end + 1;
                    continue;
                }
            }

            // Subscript: ~text~
            if chars[i] == '~' {
                if !current.is_empty() {
                    inlines.push(Inline::Text(current.clone(), Span::NONE));
                    current.clear();
                }
                if let Some((content, end)) = self.find_delim(&chars, i + 1, '~') {
                    let children = self.parse_inline(&content);
                    inlines.push(Inline::Subscript(children, Span::NONE));
                    i = end + 1;
                    continue;
                }
            }

            // Link: [text|url] or [url]
            if chars[i] == '[' {
                if !current.is_empty() {
                    inlines.push(Inline::Text(current.clone(), Span::NONE));
                    current.clear();
                }
                if let Some((link_content, end)) = self.find_bracket(&chars, i + 1) {
                    let (text, url) = if let Some(pipe_pos) = link_content.find('|') {
                        (&link_content[..pipe_pos], &link_content[pipe_pos + 1..])
                    } else {
                        (link_content.as_str(), link_content.as_str())
                    };
                    inlines.push(Inline::Link {
                        url: url.to_string(),
                        children: vec![Inline::Text(text.to_string(), Span::NONE)],
                        span: Span::NONE,
                    });
                    i = end + 1;
                    continue;
                }
            }

            // Image: !url! or !url|alt!
            if chars[i] == '!' {
                if !current.is_empty() {
                    inlines.push(Inline::Text(current.clone(), Span::NONE));
                    current.clear();
                }
                if let Some((img_content, end)) = self.find_delim(&chars, i + 1, '!') {
                    let (url, alt) = if let Some(pipe_pos) = img_content.find('|') {
                        (
                            &img_content[..pipe_pos],
                            Some(img_content[pipe_pos + 1..].to_string()),
                        )
                    } else {
                        (img_content.as_str(), None)
                    };
                    inlines.push(Inline::Image {
                        url: url.to_string(),
                        alt,
                        span: Span::NONE,
                    });
                    i = end + 1;
                    continue;
                }
            }

            // Mention: @username
            if chars[i] == '@' && i + 1 < chars.len() && chars[i + 1].is_alphanumeric() {
                if !current.is_empty() {
                    inlines.push(Inline::Text(current.clone(), Span::NONE));
                    current.clear();
                }
                let mut name = String::new();
                let mut j = i + 1;
                while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_' || chars[j] == '-' || chars[j] == '.') {
                    name.push(chars[j]);
                    j += 1;
                }
                inlines.push(Inline::Mention(name, Span::NONE));
                i = j;
                continue;
            }

            current.push(chars[i]);
            i += 1;
        }

        if !current.is_empty() {
            inlines.push(Inline::Text(current, Span::NONE));
        }

        inlines
    }

    fn find_delim(&self, chars: &[char], start: usize, delim: char) -> Option<(String, usize)> {
        let mut content = String::new();
        let mut i = start;

        while i < chars.len() {
            if chars[i] == delim {
                if !content.is_empty() {
                    return Some((content, i));
                }
                return None; // empty content — not a valid delimited span
            }
            content.push(chars[i]);
            i += 1;
        }

        None
    }

    fn find_double_brace(&self, chars: &[char], start: usize) -> Option<(String, usize)> {
        let mut content = String::new();
        let mut i = start;

        while i + 1 < chars.len() {
            if chars[i] == '}' && chars[i + 1] == '}' {
                return Some((content, i));
            }
            content.push(chars[i]);
            i += 1;
        }

        None
    }

    fn find_bracket(&self, chars: &[char], start: usize) -> Option<(String, usize)> {
        let mut content = String::new();
        let mut i = start;

        while i < chars.len() {
            if chars[i] == ']' {
                return Some((content, i));
            }
            content.push(chars[i]);
            i += 1;
        }

        None
    }
}
