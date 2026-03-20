//! Muse parser.

use crate::ast::{Block, Diagnostic, Inline, MuseDoc, Span};

// ── Public API ────────────────────────────────────────────────────────────────

/// Parse a Muse string into a [`MuseDoc`], returning any diagnostics.
///
/// This function is infallible — malformed input produces diagnostics, not errors.
pub fn parse(input: &str) -> (MuseDoc, Vec<Diagnostic>) {
    let mut p = Parser::new(input);
    let blocks = p.parse();
    let doc = MuseDoc {
        blocks,
        span: Span::new(0, input.len()),
    };
    (doc, p.diagnostics)
}

// ── Parser ────────────────────────────────────────────────────────────────────

struct Parser<'a> {
    lines: Vec<&'a str>,
    pos: usize,
    pub diagnostics: Vec<Diagnostic>,
    /// Byte offset of the start of each line.
    line_offsets: Vec<usize>,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        let lines: Vec<&'a str> = input.lines().collect();
        // Compute byte offset of each line start.
        let mut offsets = Vec::with_capacity(lines.len());
        let mut offset = 0usize;
        for line in &lines {
            offsets.push(offset);
            offset += line.len() + 1; // +1 for the newline
        }
        Self {
            lines,
            pos: 0,
            diagnostics: Vec::new(),
            line_offsets: offsets,
        }
    }

    fn line_start(&self, idx: usize) -> usize {
        self.line_offsets.get(idx).copied().unwrap_or(0)
    }

    fn line_span(&self, idx: usize) -> Span {
        let start = self.line_start(idx);
        let end = start + self.lines.get(idx).map(|l| l.len()).unwrap_or(0);
        Span::new(start, end)
    }

    fn parse(&mut self) -> Vec<Block> {
        let mut nodes = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            if line.trim().is_empty() {
                self.pos += 1;
                continue;
            }

            // Example block <example>...</example>
            if line.trim_start().starts_with("<example>") {
                nodes.push(self.parse_example_block());
                continue;
            }

            // Verse block <verse>...</verse>
            if line.trim_start().starts_with("<verse>") {
                nodes.push(self.parse_verse_block());
                continue;
            }

            // Quote block <quote>...</quote>
            if line.trim_start().starts_with("<quote>") {
                nodes.push(self.parse_quote_block());
                continue;
            }

            // Heading * to *****
            if let Some(node) = self.try_parse_heading(line) {
                nodes.push(node);
                self.pos += 1;
                continue;
            }

            // Horizontal rule (4+ dashes)
            if line.trim().starts_with("----") {
                let span = self.line_span(self.pos);
                nodes.push(Block::HorizontalRule { span });
                self.pos += 1;
                continue;
            }

            // Unordered list (space before -)
            if line.starts_with(" - ") || line.starts_with("  - ") {
                nodes.push(self.parse_unordered_list());
                continue;
            }

            // Ordered list (space before number)
            if self.is_ordered_list_item(line) {
                nodes.push(self.parse_ordered_list());
                continue;
            }

            // Definition list (term ::)
            if line.contains(" :: ") {
                nodes.push(self.parse_definition_list());
                continue;
            }

            // Indented code block
            if line.starts_with("  ") && !line.trim().is_empty() {
                nodes.push(self.parse_indented_code());
                continue;
            }

            // Regular paragraph
            nodes.push(self.parse_paragraph());
        }

        nodes
    }

    fn try_parse_heading(&self, line: &str) -> Option<Block> {
        // Muse headings: * to *****
        let level = line.chars().take_while(|&c| c == '*').count();

        if level > 0 && level <= 5 && line.len() > level && line.chars().nth(level) == Some(' ') {
            let content = line[level + 1..].trim();
            let span = self.line_span(self.pos);
            let inline_nodes = parse_inline(content, span.start + level + 1);

            return Some(Block::Heading {
                level: level as u8,
                inlines: inline_nodes,
                span,
            });
        }
        None
    }

    fn parse_example_block(&mut self) -> Block {
        let start = self.line_start(self.pos);
        let mut content = String::new();
        let first_line = self.lines[self.pos];

        // Get content after <example> on same line
        if let Some(pos) = first_line.find("<example>") {
            let after = &first_line[pos + 9..];
            if let Some(end) = after.find("</example>") {
                content.push_str(&after[..end]);
                self.pos += 1;
                let span = Span::new(start, self.line_start(self.pos));
                return Block::CodeBlock { content, span };
            }
            if !after.trim().is_empty() {
                content.push_str(after.trim());
                content.push('\n');
            }
        }
        self.pos += 1;

        // Multi-line
        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            if line.contains("</example>") {
                if let Some(pos) = line.find("</example>") {
                    content.push_str(&line[..pos]);
                }
                self.pos += 1;
                break;
            }
            content.push_str(line);
            content.push('\n');
            self.pos += 1;
        }

        let end = self.line_start(self.pos);
        Block::CodeBlock {
            content: content.trim_end().to_string(),
            span: Span::new(start, end),
        }
    }

    fn parse_verse_block(&mut self) -> Block {
        let start = self.line_start(self.pos);
        let mut content = String::new();
        let first_line = self.lines[self.pos];

        if let Some(pos) = first_line.find("<verse>") {
            let after = &first_line[pos + 7..];
            if let Some(end) = after.find("</verse>") {
                content.push_str(&after[..end]);
                self.pos += 1;
                let span = Span::new(start, self.line_start(self.pos));
                let inline = parse_inline(&content, start);
                return Block::Blockquote {
                    children: vec![Block::Paragraph {
                        inlines: inline,
                        span,
                    }],
                    span,
                };
            }
            if !after.trim().is_empty() {
                content.push_str(after.trim());
                content.push('\n');
            }
        }
        self.pos += 1;

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            if line.contains("</verse>") {
                if let Some(pos) = line.find("</verse>") {
                    content.push_str(&line[..pos]);
                }
                self.pos += 1;
                break;
            }
            content.push_str(line);
            content.push('\n');
            self.pos += 1;
        }

        let end = self.line_start(self.pos);
        let span = Span::new(start, end);
        let inline = parse_inline(content.trim_end(), start);
        Block::Blockquote {
            children: vec![Block::Paragraph {
                inlines: inline,
                span,
            }],
            span,
        }
    }

    fn parse_quote_block(&mut self) -> Block {
        let start = self.line_start(self.pos);
        let mut content = String::new();
        let first_line = self.lines[self.pos];

        if let Some(pos) = first_line.find("<quote>") {
            let after = &first_line[pos + 7..];
            if let Some(end) = after.find("</quote>") {
                content.push_str(&after[..end]);
                self.pos += 1;
                let span = Span::new(start, self.line_start(self.pos));
                let inline = parse_inline(&content, start);
                return Block::Blockquote {
                    children: vec![Block::Paragraph {
                        inlines: inline,
                        span,
                    }],
                    span,
                };
            }
            if !after.trim().is_empty() {
                content.push_str(after.trim());
                content.push('\n');
            }
        }
        self.pos += 1;

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            if line.contains("</quote>") {
                if let Some(pos) = line.find("</quote>") {
                    content.push_str(&line[..pos]);
                }
                self.pos += 1;
                break;
            }
            content.push_str(line);
            content.push('\n');
            self.pos += 1;
        }

        let end = self.line_start(self.pos);
        let span = Span::new(start, end);
        let inline = parse_inline(content.trim_end(), start);
        Block::Blockquote {
            children: vec![Block::Paragraph {
                inlines: inline,
                span,
            }],
            span,
        }
    }

    fn is_ordered_list_item(&self, line: &str) -> bool {
        if line.starts_with(' ') {
            let trimmed = line.trim_start();
            if let Some(dot_pos) = trimmed.find(". ") {
                let num = &trimmed[..dot_pos];
                return num.chars().all(|c| c.is_ascii_digit());
            }
        }
        false
    }

    fn parse_unordered_list(&mut self) -> Block {
        let start = self.line_start(self.pos);
        let mut items = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            if !line.starts_with(" - ") && !line.starts_with("  - ") {
                break;
            }

            let item_span = self.line_span(self.pos);
            let content = line.trim_start()[2..].trim();
            let inline_nodes = parse_inline(content, item_span.start + line.find(content).unwrap_or(0));
            items.push(vec![Block::Paragraph {
                inlines: inline_nodes,
                span: item_span,
            }]);
            self.pos += 1;
        }

        let end = self.line_start(self.pos);
        Block::List {
            ordered: false,
            items,
            span: Span::new(start, end),
        }
    }

    fn parse_ordered_list(&mut self) -> Block {
        let start = self.line_start(self.pos);
        let mut items = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            if !self.is_ordered_list_item(line) {
                break;
            }

            let item_span = self.line_span(self.pos);
            let trimmed = line.trim_start();
            if let Some(dot_pos) = trimmed.find(". ") {
                let content = &trimmed[dot_pos + 2..];
                let inline_nodes = parse_inline(content, item_span.start + line.find(content).unwrap_or(0));
                items.push(vec![Block::Paragraph {
                    inlines: inline_nodes,
                    span: item_span,
                }]);
            }
            self.pos += 1;
        }

        let end = self.line_start(self.pos);
        Block::List {
            ordered: true,
            items,
            span: Span::new(start, end),
        }
    }

    fn parse_definition_list(&mut self) -> Block {
        let start = self.line_start(self.pos);
        let mut items = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            if !line.contains(" :: ") {
                break;
            }

            let item_span = self.line_span(self.pos);
            if let Some(sep_pos) = line.find(" :: ") {
                let term = &line[..sep_pos];
                let desc = &line[sep_pos + 4..];

                let term_inlines = parse_inline(term.trim(), item_span.start);
                let desc_block = Block::Paragraph {
                    inlines: parse_inline(desc.trim(), item_span.start + sep_pos + 4),
                    span: item_span,
                };

                items.push((term_inlines, vec![desc_block]));
            }
            self.pos += 1;
        }

        let end = self.line_start(self.pos);
        Block::DefinitionList {
            items,
            span: Span::new(start, end),
        }
    }

    fn parse_indented_code(&mut self) -> Block {
        let start = self.line_start(self.pos);
        let mut content = String::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            if !line.starts_with("  ") && !line.trim().is_empty() {
                break;
            }

            if let Some(stripped) = line.strip_prefix("  ") {
                content.push_str(stripped);
                content.push('\n');
            }
            self.pos += 1;
        }

        let end = self.line_start(self.pos);
        Block::CodeBlock {
            content: content.trim_end().to_string(),
            span: Span::new(start, end),
        }
    }

    fn parse_paragraph(&mut self) -> Block {
        let start = self.line_start(self.pos);
        let mut text = String::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            if line.trim().is_empty() {
                break;
            }

            // Check for block elements - but not **bold**
            let is_heading = line.chars().take_while(|&c| c == '*').count() > 0
                && line.chars().find(|&c| c != '*') == Some(' ');
            if is_heading
                || line.starts_with("----")
                || line.starts_with(" - ")
                || (line.starts_with("  ") && !line.trim().is_empty())
                || line.contains(" :: ")
                || line.trim_start().starts_with('<')
                || self.is_ordered_list_item(line)
            {
                break;
            }

            if !text.is_empty() {
                text.push(' ');
            }
            text.push_str(line.trim());
            self.pos += 1;
        }

        let end = self.line_start(self.pos);
        let span = Span::new(start, end);
        let inline_nodes = parse_inline(&text, start);
        Block::Paragraph {
            inlines: inline_nodes,
            span,
        }
    }
}

// ── Inline parser ─────────────────────────────────────────────────────────────

pub(crate) fn parse_inline(text: &str, base_offset: usize) -> Vec<Inline> {
    let mut nodes = Vec::new();
    let mut current = String::new();
    let mut current_start = base_offset;
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Inline code =...=
        if chars[i] == '='
            && i + 1 < chars.len()
            && let Some((end, content)) = find_closing(&chars, i + 1, '=')
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(
                    current.clone(),
                    Span::new(current_start, base_offset + char_byte_offset(text, i)),
                ));
                current.clear();
            }
            let code_start = base_offset + char_byte_offset(text, i);
            let code_end = base_offset + char_byte_offset(text, end + 1);
            nodes.push(Inline::Code(content, Span::new(code_start, code_end)));
            i = end + 1;
            current_start = base_offset + char_byte_offset(text, i.min(chars.len()));
            continue;
        }

        // Bold **...** (doubled asterisks)
        if chars[i] == '*'
            && i + 1 < chars.len()
            && chars[i + 1] == '*'
            && i + 2 < chars.len()
            && let Some((end, content)) = find_double_closing(&chars, i + 2, '*')
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(
                    current.clone(),
                    Span::new(current_start, base_offset + char_byte_offset(text, i)),
                ));
                current.clear();
            }
            let bold_start = base_offset + char_byte_offset(text, i);
            let bold_end = base_offset + char_byte_offset(text, (end + 2).min(chars.len()));
            let inner = parse_inline(&content, bold_start + 2);
            nodes.push(Inline::Bold(inner, Span::new(bold_start, bold_end)));
            i = end + 2;
            current_start = base_offset + char_byte_offset(text, i.min(chars.len()));
            continue;
        }

        // Emphasis *...*
        if chars[i] == '*'
            && i + 1 < chars.len()
            && chars[i + 1] != '*'
            && (i == 0 || !chars[i - 1].is_alphanumeric())
            && let Some((end, content)) = find_closing(&chars, i + 1, '*')
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(
                    current.clone(),
                    Span::new(current_start, base_offset + char_byte_offset(text, i)),
                ));
                current.clear();
            }
            let em_start = base_offset + char_byte_offset(text, i);
            let em_end = base_offset + char_byte_offset(text, (end + 1).min(chars.len()));
            let inner = parse_inline(&content, em_start + 1);
            nodes.push(Inline::Italic(inner, Span::new(em_start, em_end)));
            i = end + 1;
            current_start = base_offset + char_byte_offset(text, i.min(chars.len()));
            continue;
        }

        // Link [[url][text]] or [[url]]
        if chars[i] == '['
            && i + 1 < chars.len()
            && chars[i + 1] == '['
            && let Some((end, url, link_text)) = parse_muse_link(&chars, i)
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(
                    current.clone(),
                    Span::new(current_start, base_offset + char_byte_offset(text, i)),
                ));
                current.clear();
            }
            let link_start = base_offset + char_byte_offset(text, i);
            let link_end = base_offset + char_byte_offset(text, end.min(chars.len()));
            nodes.push(Inline::Link {
                url,
                children: vec![Inline::Text(link_text, Span::new(link_start, link_end))],
                span: Span::new(link_start, link_end),
            });
            i = end;
            current_start = base_offset + char_byte_offset(text, i.min(chars.len()));
            continue;
        }

        current.push(chars[i]);
        i += 1;
    }

    if !current.is_empty() {
        let end = base_offset + text.len();
        nodes.push(Inline::Text(current, Span::new(current_start, end)));
    }

    nodes
}

/// Get byte offset of the i-th character in `text`.
fn char_byte_offset(text: &str, char_idx: usize) -> usize {
    text.char_indices()
        .nth(char_idx)
        .map(|(b, _)| b)
        .unwrap_or(text.len())
}

fn find_closing(chars: &[char], start: usize, marker: char) -> Option<(usize, String)> {
    let mut i = start;
    let mut content = String::new();

    while i < chars.len() {
        if chars[i] == marker {
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

fn parse_muse_link(chars: &[char], start: usize) -> Option<(usize, String, String)> {
    // [[url][text]] or [[url]]
    if start + 1 >= chars.len() || chars[start] != '[' || chars[start + 1] != '[' {
        return None;
    }

    let mut i = start + 2;
    let mut url = String::new();

    // Collect URL until ] or [
    while i < chars.len() && chars[i] != ']' && chars[i] != '[' {
        url.push(chars[i]);
        i += 1;
    }

    if i >= chars.len() {
        return None;
    }

    // Check for link text
    if chars[i] == ']' && i + 1 < chars.len() && chars[i + 1] == '[' {
        i += 2;
        let mut text = String::new();
        while i < chars.len() && chars[i] != ']' {
            text.push(chars[i]);
            i += 1;
        }
        if i + 1 < chars.len() && chars[i] == ']' && chars[i + 1] == ']' {
            return Some((i + 2, url, text));
        }
    } else if chars[i] == ']' && i + 1 < chars.len() && chars[i + 1] == ']' {
        // No link text, use URL
        return Some((i + 2, url.clone(), url));
    }

    None
}
