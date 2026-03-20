//! ZimWiki parser.

use crate::ast::*;

/// Parse a ZimWiki string into a [`ZimwikiDoc`].
pub fn parse(input: &str) -> (ZimwikiDoc, Vec<Diagnostic>) {
    let mut p = Parser::new(input);
    let blocks = p.parse();
    (ZimwikiDoc { blocks, span: Span::NONE }, vec![])
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
        let mut blocks = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            if line.trim().is_empty() {
                self.pos += 1;
                continue;
            }

            // Heading ====== Title ====== (more = = lower level, inverted)
            if let Some(block) = self.try_parse_heading(line) {
                blocks.push(block);
                self.pos += 1;
                continue;
            }

            // Horizontal rule (line of only dashes, at least 4)
            if line.trim().chars().all(|c| c == '-') && line.trim().len() >= 4 {
                blocks.push(Block::HorizontalRule { span: Span::NONE });
                self.pos += 1;
                continue;
            }

            // Verbatim block '''...'''
            if line.trim_start().starts_with("'''") {
                blocks.push(self.parse_verbatim_block());
                continue;
            }

            // Unordered list (*)
            let trimmed = line.trim_start();
            if trimmed.starts_with("* ") && !trimmed.starts_with("**") {
                blocks.push(self.parse_list(false));
                continue;
            }

            // Ordered list (1. or a.)
            if self.is_ordered_list_item(line) {
                blocks.push(self.parse_list(true));
                continue;
            }

            // Checkbox list [ ] or [*] or [x]
            if trimmed.starts_with("[ ] ")
                || trimmed.starts_with("[*] ")
                || trimmed.starts_with("[x] ")
            {
                blocks.push(self.parse_checkbox_list());
                continue;
            }

            // Regular paragraph
            blocks.push(self.parse_paragraph());
        }

        blocks
    }

    fn try_parse_heading(&self, line: &str) -> Option<Block> {
        let trimmed = line.trim();

        // Count leading =
        let eq_count = trimmed.chars().take_while(|&c| c == '=').count();
        if !(2..=6).contains(&eq_count) {
            return None;
        }

        // Check for matching trailing =
        let trailing = trimmed.chars().rev().take_while(|&c| c == '=').count();
        if trailing < eq_count {
            return None;
        }

        // Extract content
        let content = &trimmed[eq_count..trimmed.len() - trailing].trim();
        if content.is_empty() {
            return None;
        }

        // ZimWiki heading levels are inverted: ====== = level 1, ===== = level 2, etc.
        let level = 7 - eq_count; // 6 = signs -> level 1, 5 -> level 2, etc.

        let inlines = parse_inline(content);
        Some(Block::Heading {
            level: level as u8,
            inlines,
            span: Span::NONE,
        })
    }

    fn parse_verbatim_block(&mut self) -> Block {
        let mut content = String::new();
        let first_line = self.lines[self.pos].trim_start();

        // Get content after ''' on same line
        if first_line.len() > 3 {
            let after = &first_line[3..];
            if let Some(end_pos) = after.find("'''") {
                content.push_str(&after[..end_pos]);
                self.pos += 1;
                return Block::CodeBlock { content, span: Span::NONE };
            }
            content.push_str(after);
            content.push('\n');
        }
        self.pos += 1;

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            if line.contains("'''") {
                if let Some(pos) = line.find("'''") {
                    content.push_str(&line[..pos]);
                }
                self.pos += 1;
                break;
            }
            content.push_str(line);
            content.push('\n');
            self.pos += 1;
        }

        Block::CodeBlock {
            content: content.trim_end().to_string(),
            span: Span::NONE,
        }
    }

    fn is_ordered_list_item(&self, line: &str) -> bool {
        let trimmed = line.trim_start();
        if let Some(dot_pos) = trimmed.find(". ") {
            let prefix = &trimmed[..dot_pos];
            return prefix.chars().all(|c| c.is_ascii_digit())
                || (prefix.len() == 1 && prefix.chars().all(|c| c.is_ascii_lowercase()));
        }
        false
    }

    fn parse_list(&mut self, ordered: bool) -> Block {
        let mut items = Vec::new();
        let base_indent = self.lines[self.pos].len() - self.lines[self.pos].trim_start().len();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            let indent = line.len() - line.trim_start().len();
            let trimmed = line.trim_start();

            if trimmed.is_empty() {
                self.pos += 1;
                continue;
            }

            if indent < base_indent {
                break;
            }

            let is_bullet = trimmed.starts_with("* ") && !trimmed.starts_with("**");
            let is_numbered = self.is_ordered_list_item(line);

            if !is_bullet && !is_numbered {
                break;
            }

            let content = if is_bullet {
                &trimmed[2..]
            } else if let Some(pos) = trimmed.find(". ") {
                &trimmed[pos + 2..]
            } else {
                break;
            };

            let inlines = parse_inline(content);
            let para = Block::Paragraph { inlines, span: Span::NONE };
            items.push(ListItem {
                checked: None,
                children: vec![para],
                span: Span::NONE,
            });
            self.pos += 1;
        }

        Block::List { ordered, items, span: Span::NONE }
    }

    fn parse_checkbox_list(&mut self) -> Block {
        let mut items = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            let trimmed = line.trim_start();

            let (checked, content) = if let Some(rest) = trimmed.strip_prefix("[ ] ") {
                (Some(false), rest)
            } else if let Some(rest) = trimmed.strip_prefix("[*] ") {
                (Some(true), rest)
            } else if let Some(rest) = trimmed.strip_prefix("[x] ") {
                (Some(true), rest)
            } else {
                break;
            };

            let inlines = parse_inline(content);
            let para = Block::Paragraph { inlines, span: Span::NONE };
            items.push(ListItem {
                checked,
                children: vec![para],
                span: Span::NONE,
            });
            self.pos += 1;
        }

        Block::List {
            ordered: false,
            items,
            span: Span::NONE,
        }
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
            if self.try_parse_heading(line).is_some()
                || (line.trim().chars().all(|c| c == '-') && line.trim().len() >= 4)
                || trimmed.starts_with("'''")
                || (trimmed.starts_with("* ") && !trimmed.starts_with("**"))
                || self.is_ordered_list_item(line)
                || trimmed.starts_with("[ ] ")
                || trimmed.starts_with("[*] ")
                || trimmed.starts_with("[x] ")
            {
                break;
            }

            if !text.is_empty() {
                text.push(' ');
            }
            text.push_str(line.trim());
            self.pos += 1;
        }

        let inlines = parse_inline(&text);
        Block::Paragraph { inlines, span: Span::NONE }
    }
}

pub(crate) fn parse_inline(text: &str) -> Vec<Inline> {
    let mut inlines = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Bold **text** or __text__
        if (chars[i] == '*' || chars[i] == '_')
            && i + 1 < chars.len()
            && chars[i + 1] == chars[i]
            && let Some((end, content)) = find_double_closing(&chars, i + 2, chars[i])
        {
            if !current.is_empty() {
                inlines.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            let inner = parse_inline(&content);
            inlines.push(Inline::Bold(inner, Span::NONE));
            i = end + 2;
            continue;
        }

        // Italic //text//
        if chars[i] == '/'
            && i + 1 < chars.len()
            && chars[i + 1] == '/'
            && let Some((end, content)) = find_double_closing(&chars, i + 2, '/')
        {
            if !current.is_empty() {
                inlines.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            let inner = parse_inline(&content);
            inlines.push(Inline::Italic(inner, Span::NONE));
            i = end + 2;
            continue;
        }

        // Strikethrough ~~text~~
        if chars[i] == '~'
            && i + 1 < chars.len()
            && chars[i + 1] == '~'
            && let Some((end, content)) = find_double_closing(&chars, i + 2, '~')
        {
            if !current.is_empty() {
                inlines.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            let inner = parse_inline(&content);
            inlines.push(Inline::Strikethrough(inner, Span::NONE));
            i = end + 2;
            continue;
        }

        // Subscript _{text}
        if chars[i] == '_'
            && i + 1 < chars.len()
            && chars[i + 1] == '{'
            && let Some((end, content)) = find_brace_closing(&chars, i + 2)
        {
            if !current.is_empty() {
                inlines.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            let inner = parse_inline(&content);
            inlines.push(Inline::Subscript(inner, Span::NONE));
            i = end + 1;
            continue;
        }

        // Superscript ^{text}
        if chars[i] == '^'
            && i + 1 < chars.len()
            && chars[i + 1] == '{'
            && let Some((end, content)) = find_brace_closing(&chars, i + 2)
        {
            if !current.is_empty() {
                inlines.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            let inner = parse_inline(&content);
            inlines.push(Inline::Superscript(inner, Span::NONE));
            i = end + 1;
            continue;
        }

        // Inline code ''text''
        if chars[i] == '\''
            && i + 1 < chars.len()
            && chars[i + 1] == '\''
            && let Some((end, content)) = find_double_closing(&chars, i + 2, '\'')
        {
            if !current.is_empty() {
                inlines.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            inlines.push(Inline::Code(content, Span::NONE));
            i = end + 2;
            continue;
        }

        // Link [[target]] or [[target|label]]
        if chars[i] == '['
            && i + 1 < chars.len()
            && chars[i + 1] == '['
            && let Some((end, url, label)) = parse_link(&chars, i)
        {
            if !current.is_empty() {
                inlines.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            let text_inline = Inline::Text(label, Span::NONE);
            inlines.push(Inline::Link {
                url,
                children: vec![text_inline],
                span: Span::NONE,
            });
            i = end;
            continue;
        }

        // Image {{image.png}}
        if chars[i] == '{'
            && i + 1 < chars.len()
            && chars[i + 1] == '{'
            && let Some((end, url)) = parse_image(&chars, i)
        {
            if !current.is_empty() {
                inlines.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            inlines.push(Inline::Image { url, span: Span::NONE });
            i = end;
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

fn find_double_closing(chars: &[char], start: usize, marker: char) -> Option<(usize, String)> {
    let mut i = start;
    let mut content = String::new();

    while i + 1 < chars.len() {
        if chars[i] == marker && chars[i + 1] == marker {
            return Some((i, content));
        }
        content.push(chars[i]);
        i += 1;
    }
    None
}

fn find_brace_closing(chars: &[char], start: usize) -> Option<(usize, String)> {
    let mut i = start;
    let mut content = String::new();

    while i < chars.len() {
        if chars[i] == '}' {
            return Some((i, content));
        }
        content.push(chars[i]);
        i += 1;
    }
    None
}

fn parse_link(chars: &[char], start: usize) -> Option<(usize, String, String)> {
    if start + 1 >= chars.len() || chars[start] != '[' || chars[start + 1] != '[' {
        return None;
    }

    let mut i = start + 2;
    let mut content = String::new();

    while i < chars.len() {
        if chars[i] == ']' && i + 1 < chars.len() && chars[i + 1] == ']' {
            let (url, label) = if let Some(pipe_pos) = content.find('|') {
                (
                    content[..pipe_pos].to_string(),
                    content[pipe_pos + 1..].to_string(),
                )
            } else {
                (content.clone(), content)
            };
            return Some((i + 2, url, label));
        }
        content.push(chars[i]);
        i += 1;
    }
    None
}

fn parse_image(chars: &[char], start: usize) -> Option<(usize, String)> {
    if start + 1 >= chars.len() || chars[start] != '{' || chars[start + 1] != '{' {
        return None;
    }

    let mut i = start + 2;
    let mut content = String::new();

    while i < chars.len() {
        if chars[i] == '}' && i + 1 < chars.len() && chars[i + 1] == '}' {
            return Some((i + 2, content));
        }
        content.push(chars[i]);
        i += 1;
    }
    None
}
