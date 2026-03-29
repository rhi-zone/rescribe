//! VimWiki parser.

use crate::ast::*;

/// Parse a VimWiki string into a [`VimwikiDoc`].
pub fn parse(input: &str) -> (VimwikiDoc, Vec<Diagnostic>) {
    let mut p = Parser::new(input);
    let blocks = p.parse();
    (VimwikiDoc { blocks, span: Span::NONE }, vec![])
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

            // Comment line (skip)
            if line.trim_start().starts_with("%% ") || line.trim() == "%%" {
                self.pos += 1;
                continue;
            }

            // Heading = Title = to ====== Title ======
            if let Some(block) = self.try_parse_heading(line) {
                blocks.push(block);
                self.pos += 1;
                continue;
            }

            // Horizontal rule (4+ dashes)
            if line.trim().chars().all(|c| c == '-') && line.trim().len() >= 4 {
                blocks.push(Block::HorizontalRule { span: Span::NONE });
                self.pos += 1;
                continue;
            }

            // Preformatted block {{{ ... }}}
            if line.trim_start().starts_with("{{{") && !is_inline_preformatted(line) {
                blocks.push(self.parse_preformatted());
                continue;
            }

            let trimmed = line.trim_start();

            // Definition list (; term\n: definition)
            if trimmed.starts_with("; ") {
                blocks.push(self.parse_definition_list());
                continue;
            }

            // Unordered list (* or -)
            if (trimmed.starts_with("* ") && !trimmed.starts_with("**"))
                || trimmed.starts_with("- ")
            {
                blocks.push(self.parse_list(false));
                continue;
            }

            // Ordered list (# or 1. or a))
            if trimmed.starts_with("# ") || self.is_ordered_list_item(line) {
                blocks.push(self.parse_list(true));
                continue;
            }

            // Blockquote (lines starting with >)
            if trimmed.starts_with("> ") || trimmed == ">" {
                blocks.push(self.parse_blockquote());
                continue;
            }

            // Table
            if trimmed.starts_with('|') {
                blocks.push(self.parse_table());
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
        let level = trimmed.chars().take_while(|&c| c == '=').count();
        if level == 0 || level > 6 {
            return None;
        }

        // Check for matching trailing =
        let trailing = trimmed.chars().rev().take_while(|&c| c == '=').count();
        if trailing < level {
            return None;
        }

        // Guard against degenerate input like a lone "=" (level + trailing > len).
        if level + trailing > trimmed.len() {
            return None;
        }

        // Extract content
        let content = &trimmed[level..trimmed.len() - trailing].trim();
        if content.is_empty() {
            return None;
        }

        let inlines = parse_inline(content);
        Some(Block::Heading { level, inlines, span: Span::NONE })
    }

    fn parse_preformatted(&mut self) -> Block {
        let mut content = String::new();
        let first_line = self.lines[self.pos].trim_start();

        // Check for language after {{{
        let language = if first_line.len() > 3 {
            let after = first_line[3..].trim();
            if !after.is_empty() {
                // Handle class="lang" style
                let lang = if let Some(rest) = after.strip_prefix("class=\"") {
                    rest.strip_suffix('"').unwrap_or(rest).to_string()
                } else {
                    after.to_string()
                };
                Some(lang)
            } else {
                None
            }
        } else {
            None
        };

        self.pos += 1;

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            if line.trim() == "}}}" {
                self.pos += 1;
                break;
            }
            if !content.is_empty() {
                content.push('\n');
            }
            content.push_str(line);
            self.pos += 1;
        }

        Block::CodeBlock { language, content, span: Span::NONE }
    }

    fn is_ordered_list_item(&self, line: &str) -> bool {
        let trimmed = line.trim_start();
        // Check for # style
        if trimmed.starts_with("# ") {
            return true;
        }
        // Check for 1. style
        if let Some(dot_pos) = trimmed.find(". ") {
            let prefix = &trimmed[..dot_pos];
            return prefix.chars().all(|c| c.is_ascii_digit());
        }
        // Check for a) style
        if let Some(paren_pos) = trimmed.find(") ") {
            let prefix = &trimmed[..paren_pos];
            return prefix.len() == 1 && prefix.chars().all(|c| c.is_ascii_lowercase());
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

            // Check if still in list at same or deeper level
            if indent < base_indent {
                break;
            }

            let is_bullet = (trimmed.starts_with("* ") && !trimmed.starts_with("**"))
                || trimmed.starts_with("- ");
            let is_hash = trimmed.starts_with("# ");
            let is_numbered = self.is_ordered_list_item(line);

            if !is_bullet && !is_numbered && !is_hash {
                break;
            }

            // Extract item content
            let content = if is_bullet || is_hash {
                &trimmed[2..]
            } else if let Some(pos) = trimmed.find(". ") {
                &trimmed[pos + 2..]
            } else if let Some(pos) = trimmed.find(") ") {
                &trimmed[pos + 2..]
            } else {
                break;
            };

            // Check for checkbox [ ] or [X]
            let (checkbox_state, actual_content) = if let Some(rest) = content.strip_prefix("[ ] ")
            {
                (Some(false), rest)
            } else if let Some(rest) = content
                .strip_prefix("[X] ")
                .or_else(|| content.strip_prefix("[x] "))
            {
                (Some(true), rest)
            } else {
                (None, content)
            };

            let inlines = parse_inline(actual_content);
            items.push(ListItem {
                checked: checkbox_state,
                inlines,
                span: Span::NONE,
            });
            self.pos += 1;
        }

        Block::List { ordered, items, span: Span::NONE }
    }

    fn parse_blockquote(&mut self) -> Block {
        let mut content = String::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            let trimmed = line.trim_start();

            if !trimmed.starts_with('>') {
                break;
            }

            let text = if let Some(rest) = trimmed.strip_prefix("> ") {
                rest
            } else if trimmed == ">" {
                ""
            } else {
                break;
            };

            if !content.is_empty() {
                content.push(' ');
            }
            content.push_str(text);
            self.pos += 1;
        }

        let inlines = parse_inline(&content);
        Block::Blockquote { inlines, span: Span::NONE }
    }

    fn parse_table(&mut self) -> Block {
        let mut rows = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            let trimmed = line.trim();

            if !trimmed.starts_with('|') {
                break;
            }

            // Check for separator row (|---|---|)
            if trimmed.contains("---") {
                self.pos += 1;
                continue;
            }

            let mut cells = Vec::new();
            let parts: Vec<&str> = trimmed.split('|').collect();

            for part in &parts[1..] {
                // Skip empty trailing part
                if part.trim().is_empty() && parts.last() == Some(part) {
                    continue;
                }
                let inlines = parse_inline(part.trim());
                cells.push(inlines);
            }

            if !cells.is_empty() {
                rows.push(TableRow { cells, span: Span::NONE });
            }
            self.pos += 1;
        }

        Block::Table { rows, span: Span::NONE }
    }

    fn parse_definition_list(&mut self) -> Block {
        let mut items = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            let trimmed = line.trim_start();

            if !trimmed.starts_with("; ") {
                break;
            }

            let term_text = &trimmed[2..];
            let term = parse_inline(term_text);
            self.pos += 1;

            // Expect : definition line
            let desc = if self.pos < self.lines.len() {
                let dline = self.lines[self.pos].trim_start();
                if let Some(rest) = dline.strip_prefix(": ") {
                    self.pos += 1;
                    parse_inline(rest)
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            };

            items.push(DefinitionItem { term, desc, span: Span::NONE });
        }

        Block::DefinitionList { items, span: Span::NONE }
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
                || (trimmed.starts_with("{{{") && !is_inline_preformatted(line))
                || (trimmed.starts_with("* ") && !trimmed.starts_with("**"))
                || trimmed.starts_with("- ")
                || trimmed.starts_with("# ")
                || self.is_ordered_list_item(line)
                || trimmed.starts_with("> ")
                || trimmed.starts_with('|')
                || trimmed.starts_with("; ")
                || trimmed.starts_with("%% ")
                || trimmed == "%%"
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

/// Public wrapper for batch module use.
pub(crate) fn is_inline_preformatted_pub(line: &str) -> bool {
    is_inline_preformatted(line)
}

/// Check if a line with `{{{` is an inline preformatted span (contains `}}}` on the same line).
fn is_inline_preformatted(line: &str) -> bool {
    if let Some(start) = line.find("{{{") {
        line[start + 3..].contains("}}}")
    } else {
        false
    }
}

pub(crate) fn parse_inline(text: &str) -> Vec<Inline> {
    let mut inlines = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Bold *text*
        if chars[i] == '*'
            && i + 1 < chars.len()
            && chars[i + 1] != '*'
            && chars[i + 1] != ' '
            && let Some((end, content)) = find_closing(&chars, i + 1, '*')
        {
            if !current.is_empty() {
                inlines.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            let inner = parse_inline(&content);
            inlines.push(Inline::Bold(inner, Span::NONE));
            i = end + 1;
            continue;
        }

        // Italic _text_
        if chars[i] == '_'
            && i + 1 < chars.len()
            && chars[i + 1] != ' '
            && let Some((end, content)) = find_closing(&chars, i + 1, '_')
        {
            if !current.is_empty() {
                inlines.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            let inner = parse_inline(&content);
            inlines.push(Inline::Italic(inner, Span::NONE));
            i = end + 1;
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

        // Superscript ^text^
        if chars[i] == '^'
            && i + 1 < chars.len()
            && chars[i + 1] != ' '
            && let Some((end, content)) = find_closing(&chars, i + 1, '^')
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

        // Subscript ,,text,,
        if chars[i] == ','
            && i + 1 < chars.len()
            && chars[i + 1] == ','
            && let Some((end, content)) = find_double_closing(&chars, i + 2, ',')
        {
            if !current.is_empty() {
                inlines.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            let inner = parse_inline(&content);
            inlines.push(Inline::Subscript(inner, Span::NONE));
            i = end + 2;
            continue;
        }

        // Inline preformatted {{{text}}}
        if chars[i] == '{'
            && i + 2 < chars.len()
            && chars[i + 1] == '{'
            && chars[i + 2] == '{'
            && let Some((end, content)) = find_triple_closing(&chars, i + 3, '}')
        {
            if !current.is_empty() {
                inlines.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            inlines.push(Inline::Code(content, Span::NONE));
            i = end + 3;
            continue;
        }

        // Code `text`
        if chars[i] == '`'
            && let Some((end, content)) = find_closing(&chars, i + 1, '`')
        {
            if !current.is_empty() {
                inlines.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            inlines.push(Inline::Code(content, Span::NONE));
            i = end + 1;
            continue;
        }

        // Wiki link [[link]] or [[link|description]]
        if chars[i] == '['
            && i + 1 < chars.len()
            && chars[i + 1] == '['
            && let Some((end, url, label)) = parse_wiki_link(&chars, i)
        {
            if !current.is_empty() {
                inlines.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            inlines.push(Inline::Link { url, label, span: Span::NONE });
            i = end;
            continue;
        }

        // Image {{image.png}} or {{image.png|alt}} or {{image.png|alt|style}}
        if chars[i] == '{'
            && i + 1 < chars.len()
            && chars[i + 1] == '{'
            && (i + 2 >= chars.len() || chars[i + 2] != '{')
            && let Some((end, url, alt, style)) = parse_image(&chars, i)
        {
            if !current.is_empty() {
                inlines.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            inlines.push(Inline::Image { url, alt, style, span: Span::NONE });
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

fn find_closing(chars: &[char], start: usize, marker: char) -> Option<(usize, String)> {
    let mut i = start;
    let mut content = String::new();

    while i < chars.len() {
        if chars[i] == marker
            && (i + 1 >= chars.len() || chars[i + 1] == ' ' || !chars[i + 1].is_alphanumeric())
        {
            return Some((i, content));
        }
        content.push(chars[i]);
        i += 1;
    }
    None
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

fn find_triple_closing(chars: &[char], start: usize, marker: char) -> Option<(usize, String)> {
    let mut i = start;
    let mut content = String::new();

    while i + 2 < chars.len() {
        if chars[i] == marker && chars[i + 1] == marker && chars[i + 2] == marker {
            return Some((i, content));
        }
        content.push(chars[i]);
        i += 1;
    }
    None
}

fn parse_wiki_link(chars: &[char], start: usize) -> Option<(usize, String, String)> {
    // [[link]] or [[link|description]]
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

fn parse_image(chars: &[char], start: usize) -> Option<(usize, String, Option<String>, Option<String>)> {
    // {{image.png}} or {{image.png|alt}} or {{image.png|alt|style}}
    if start + 1 >= chars.len() || chars[start] != '{' || chars[start + 1] != '{' {
        return None;
    }

    let mut i = start + 2;
    let mut content = String::new();

    while i < chars.len() {
        if chars[i] == '}' && i + 1 < chars.len() && chars[i + 1] == '}' {
            let parts: Vec<&str> = content.splitn(3, '|').collect();
            let url = parts[0].to_string();
            let alt = parts.get(1).map(|s| s.to_string());
            let style = parts.get(2).map(|s| s.to_string());
            return Some((i + 2, url, alt, style));
        }
        content.push(chars[i]);
        i += 1;
    }
    None
}
