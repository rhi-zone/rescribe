use crate::ast::*;

/// Parse a DokuWiki string into a [`DokuwikiDoc`] and a list of diagnostics.
pub fn parse(input: &str) -> (DokuwikiDoc, Vec<Diagnostic>) {
    let mut p = Parser::new(input);
    match p.parse() {
        Ok(blocks) => (DokuwikiDoc { blocks }, Vec::new()),
        Err(_) => (DokuwikiDoc::default(), Vec::new()),
    }
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

    fn parse(&mut self) -> Result<Vec<Block>, ()> {
        let mut blocks = Vec::new();

        while self.pos < self.lines.len() {
            if let Some(block) = self.parse_block()? {
                blocks.push(block);
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

    fn parse_block(&mut self) -> Result<Option<Block>, ()> {
        let line = match self.current_line() {
            Some(l) => l,
            None => return Ok(None),
        };

        // Skip blank lines
        if line.trim().is_empty() {
            self.advance();
            return Ok(None);
        }

        let trimmed = line.trim();

        // Heading: ====== H1 ====== (6 =), ===== H2 ===== (5 =), etc.
        if trimmed.starts_with('=') && trimmed.ends_with('=') {
            return Ok(Some(self.parse_heading()));
        }

        // Code block: <code> or <code lang>
        if trimmed.starts_with("<code") {
            return Ok(Some(self.parse_code_block()?));
        }

        // File block: <file>
        if trimmed.starts_with("<file") {
            return Ok(Some(self.parse_code_block()?));
        }

        // List: starts with spaces and * or -
        if line.starts_with("  ")
            && (line.trim_start().starts_with('*') || line.trim_start().starts_with('-'))
        {
            return Ok(Some(self.parse_list()?));
        }

        // Blockquote: > text
        if trimmed.starts_with('>') {
            return Ok(Some(self.parse_blockquote()?));
        }

        // Horizontal rule: ----
        if trimmed == "----" {
            self.advance();
            return Ok(Some(Block::HorizontalRule(Span::NONE)));
        }

        // Default: paragraph
        Ok(Some(self.parse_paragraph()?))
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
        let content = trimmed.trim_start_matches('=').trim_end_matches('=').trim();

        Block::Heading {
            level,
            inlines: self.parse_inline(content),
            span: Span::NONE,
        }
    }

    fn parse_code_block(&mut self) -> Result<Block, ()> {
        let line = self.current_line().unwrap();
        self.advance();

        // Extract language from <code lang> or <file lang>
        let lang = if let Some(start) = line.find('<') {
            let after = &line[start..];
            if let Some(end) = after.find('>') {
                let tag = &after[1..end];
                let parts: Vec<&str> = tag.split_whitespace().collect();
                if parts.len() > 1 {
                    Some(parts[1].to_string())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let end_tag = if line.contains("<file") {
            "</file>"
        } else {
            "</code>"
        };

        let mut content = String::new();
        while let Some(line) = self.current_line() {
            if line.contains(end_tag) {
                self.advance();
                break;
            }
            if !content.is_empty() {
                content.push('\n');
            }
            content.push_str(line);
            self.advance();
        }

        Ok(Block::CodeBlock {
            language: lang,
            content,
            span: Span::NONE,
        })
    }

    fn parse_list(&mut self) -> Result<Block, ()> {
        let mut items = Vec::new();
        let first_char = self
            .current_line()
            .and_then(|l| l.trim_start().chars().next());
        let ordered = first_char == Some('-');

        while let Some(line) = self.current_line() {
            if !line.starts_with("  ") {
                break;
            }
            let trimmed = line.trim_start();
            if !trimmed.starts_with('*') && !trimmed.starts_with('-') {
                break;
            }

            // Get content after marker
            let content = trimmed[1..].trim_start();
            let item_inlines = self.parse_inline(content);
            let item = Block::Paragraph {
                inlines: item_inlines,
                span: Span::NONE,
            };
            items.push(vec![item]);
            self.advance();
        }

        Ok(Block::List { ordered, items, span: Span::NONE })
    }

    fn parse_blockquote(&mut self) -> Result<Block, ()> {
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

        let text = lines.join("\n");
        Ok(Block::Blockquote {
            children: vec![Block::Paragraph {
                inlines: vec![Inline::Text(text, Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
        })
    }

    fn parse_paragraph(&mut self) -> Result<Block, ()> {
        let mut lines = Vec::new();

        while let Some(line) = self.current_line() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                break;
            }
            // Stop at block-level elements
            if (trimmed.starts_with('=') && trimmed.ends_with('='))
                || trimmed.starts_with("<code")
                || trimmed.starts_with("<file")
                || line.starts_with("  ")
                || trimmed.starts_with('>')
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
        let mut nodes = Vec::new();
        let mut current = String::new();
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // Bold: **text**
            if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '*' {
                if !current.is_empty() {
                    nodes.push(Inline::Text(current.clone(), Span::NONE));
                    current.clear();
                }
                if let Some((content, end)) = self.find_double_delim(&chars, i + 2, '*') {
                    nodes.push(Inline::Bold(vec![Inline::Text(content, Span::NONE)], Span::NONE));
                    i = end + 2;
                    continue;
                }
            }

            // Italic: //text//
            if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '/' {
                if !current.is_empty() {
                    nodes.push(Inline::Text(current.clone(), Span::NONE));
                    current.clear();
                }
                if let Some((content, end)) = self.find_double_delim(&chars, i + 2, '/') {
                    nodes.push(Inline::Italic(
                        vec![Inline::Text(content, Span::NONE)],
                        Span::NONE,
                    ));
                    i = end + 2;
                    continue;
                }
            }

            // Underline: __text__
            if i + 1 < chars.len() && chars[i] == '_' && chars[i + 1] == '_' {
                if !current.is_empty() {
                    nodes.push(Inline::Text(current.clone(), Span::NONE));
                    current.clear();
                }
                if let Some((content, end)) = self.find_double_delim(&chars, i + 2, '_') {
                    nodes.push(Inline::Underline(
                        vec![Inline::Text(content, Span::NONE)],
                        Span::NONE,
                    ));
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
                if let Some((content, end)) = self.find_double_delim(&chars, i + 2, '\'') {
                    nodes.push(Inline::Code(content, Span::NONE));
                    i = end + 2;
                    continue;
                }
            }

            // Link: [[url|text]]
            if i + 1 < chars.len() && chars[i] == '[' && chars[i + 1] == '[' {
                if !current.is_empty() {
                    nodes.push(Inline::Text(current.clone(), Span::NONE));
                    current.clear();
                }
                if let Some((link_content, end)) = self.find_double_bracket(&chars, i + 2) {
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
                if let Some((img_content, end)) = self.find_double_brace(&chars, i + 2) {
                    let (url, alt) = if let Some(pipe_pos) = img_content.find('|') {
                        (&img_content[..pipe_pos], Some(&img_content[pipe_pos + 1..]))
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

            current.push(chars[i]);
            i += 1;
        }

        if !current.is_empty() {
            nodes.push(Inline::Text(current, Span::NONE));
        }

        nodes
    }

    fn find_double_delim(
        &self,
        chars: &[char],
        start: usize,
        delim: char,
    ) -> Option<(String, usize)> {
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

    fn find_double_bracket(&self, chars: &[char], start: usize) -> Option<(String, usize)> {
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

    fn find_double_brace(&self, chars: &[char], start: usize) -> Option<(String, usize)> {
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
}
