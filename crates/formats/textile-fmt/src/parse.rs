//! Textile parser — infallible, returns (TextileDoc, Vec<Diagnostic>).

use crate::ast::{Block, Diagnostic, Inline, Span, TableCell, TableRow, TextileDoc};

/// Parse a Textile string into a [`TextileDoc`] plus diagnostics.
/// Never panics; always returns a (possibly partial) document.
pub fn parse(input: &str) -> (TextileDoc, Vec<Diagnostic>) {
    let mut p = Parser::new(input);
    let blocks = p.parse_blocks();
    let doc = TextileDoc {
        blocks,
        span: Span::new(0, input.len()),
    };
    (doc, p.diagnostics)
}

// ── Parser ────────────────────────────────────────────────────────────────────

struct Parser<'a> {
    src: &'a str,
    lines: Vec<&'a str>,
    /// Byte offset of the start of each line within `src`.
    line_offsets: Vec<usize>,
    pos: usize,
    pub diagnostics: Vec<Diagnostic>,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        let mut offsets = Vec::new();
        let mut current = 0usize;
        let lines: Vec<&str> = input
            .split('\n')
            .map(|line| {
                // split('\n') loses the newline; we record the byte start
                let s = line;
                offsets.push(current);
                // +1 for the '\n' that split consumed (except possibly at end)
                current += line.len() + 1;
                s
            })
            .collect();
        Self {
            src: input,
            lines,
            line_offsets: offsets,
            pos: 0,
            diagnostics: Vec::new(),
        }
    }

    /// Byte offset of the start of line `line_idx` in the source.
    fn line_start(&self, line_idx: usize) -> usize {
        self.line_offsets
            .get(line_idx)
            .copied()
            .unwrap_or(self.src.len())
    }

    /// Byte offset of the end of line `line_idx` (exclusive, not including newline).
    fn line_end(&self, line_idx: usize) -> usize {
        let start = self.line_start(line_idx);
        start + self.lines.get(line_idx).map(|l| l.len()).unwrap_or(0)
    }

    fn parse_blocks(&mut self) -> Vec<Block> {
        let mut nodes = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            if line.trim().is_empty() {
                self.pos += 1;
                continue;
            }

            // Block code bc. or bc..
            if line.starts_with("bc.") {
                nodes.push(self.parse_code_block());
                continue;
            }

            // Blockquote bq.
            if line.starts_with("bq.") {
                nodes.push(self.parse_blockquote());
                continue;
            }

            // Pre block pre.
            if line.starts_with("pre.") {
                nodes.push(self.parse_pre_block());
                continue;
            }

            // Heading h1. to h6.
            if let Some(node) = self.try_parse_heading(line) {
                nodes.push(node);
                self.pos += 1;
                continue;
            }

            // Table
            if line.trim_start().starts_with('|') {
                nodes.push(self.parse_table());
                continue;
            }

            // List
            if line.trim_start().starts_with("* ")
                || line.trim_start().starts_with("# ")
                || line.trim_start().starts_with("** ")
                || line.trim_start().starts_with("## ")
            {
                nodes.push(self.parse_list());
                continue;
            }

            // Regular paragraph p. or just text
            nodes.push(self.parse_paragraph());
        }

        nodes
    }

    fn try_parse_heading(&self, line: &str) -> Option<Block> {
        let line_start = self.line_start(self.pos);
        for level in 1..=6u8 {
            let prefix = format!("h{}.", level);
            if line.starts_with(&prefix) {
                let content = line[prefix.len()..].trim();
                let inline_nodes = parse_inline(content, line_start + prefix.len());
                return Some(Block::Heading {
                    level,
                    inlines: inline_nodes,
                    span: Span::new(line_start, self.line_end(self.pos)),
                });
            }
        }
        None
    }

    fn parse_code_block(&mut self) -> Block {
        let block_start = self.line_start(self.pos);
        let first_line = self.lines[self.pos];
        let extended = first_line.starts_with("bc..");

        let content_start = if extended { 4 } else { 3 };
        let mut content = String::new();

        let first_content = first_line[content_start..].trim();
        if !first_content.is_empty() {
            content.push_str(first_content);
            content.push('\n');
        }
        self.pos += 1;

        if extended {
            while self.pos < self.lines.len() {
                let line = self.lines[self.pos];
                if line.trim().is_empty() {
                    break;
                }
                content.push_str(line);
                content.push('\n');
                self.pos += 1;
            }
        }

        let block_end = self.line_end(self.pos.saturating_sub(1));
        Block::CodeBlock {
            content: content.trim_end().to_string(),
            span: Span::new(block_start, block_end),
        }
    }

    fn parse_pre_block(&mut self) -> Block {
        let block_start = self.line_start(self.pos);
        let first_line = self.lines[self.pos];
        let extended = first_line.starts_with("pre..");

        let content_start = if extended { 5 } else { 4 };
        let mut content = String::new();

        let first_content = first_line[content_start..].trim();
        if !first_content.is_empty() {
            content.push_str(first_content);
            content.push('\n');
        }
        self.pos += 1;

        if extended {
            while self.pos < self.lines.len() {
                let line = self.lines[self.pos];
                if line.trim().is_empty() {
                    break;
                }
                content.push_str(line);
                content.push('\n');
                self.pos += 1;
            }
        }

        let block_end = self.line_end(self.pos.saturating_sub(1));
        Block::CodeBlock {
            content: content.trim_end().to_string(),
            span: Span::new(block_start, block_end),
        }
    }

    fn parse_blockquote(&mut self) -> Block {
        let block_start = self.line_start(self.pos);
        let first_line = self.lines[self.pos];
        let extended = first_line.starts_with("bq..");

        let content_start = if extended { 4 } else { 3 };
        let mut text = String::new();

        let first_content = first_line[content_start..].trim();
        if !first_content.is_empty() {
            text.push_str(first_content);
        }
        self.pos += 1;

        if extended {
            while self.pos < self.lines.len() {
                let line = self.lines[self.pos];
                if line.trim().is_empty() {
                    break;
                }
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(line.trim());
                self.pos += 1;
            }
        }

        let block_end = self.line_end(self.pos.saturating_sub(1));
        let inline_nodes = parse_inline(&text, block_start + content_start);
        Block::Blockquote {
            inlines: inline_nodes,
            span: Span::new(block_start, block_end),
        }
    }

    fn parse_table(&mut self) -> Block {
        let block_start = self.line_start(self.pos);
        let mut rows = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            if !line.trim_start().starts_with('|') {
                break;
            }

            let row = self.parse_table_row(line, self.line_start(self.pos));
            rows.push(row);
            self.pos += 1;
        }

        let block_end = self.line_end(self.pos.saturating_sub(1));
        Block::Table {
            rows,
            span: Span::new(block_start, block_end),
        }
    }

    fn parse_table_row(&self, line: &str, line_start: usize) -> TableRow {
        let mut cells = Vec::new();
        let trimmed = line.trim();

        let inner = trimmed.trim_start_matches('|').trim_end_matches('|');
        let parts: Vec<&str> = inner.split('|').collect();

        let mut offset = line_start + (trimmed.len() - inner.len());
        for part in parts {
            let part_trimmed = part.trim();
            let is_header = part_trimmed.starts_with("_.");
            let cell_content = if is_header {
                part_trimmed[2..].trim()
            } else {
                part_trimmed
            };

            let cell_start = offset;
            let cell_end = cell_start + part.len();
            let inline_nodes = parse_inline(cell_content, cell_start);
            cells.push(TableCell {
                is_header,
                inlines: inline_nodes,
                span: Span::new(cell_start, cell_end),
            });
            offset += part.len() + 1; // +1 for the '|' separator
        }

        TableRow {
            cells,
            span: Span::new(line_start, line_start + line.len()),
        }
    }

    fn parse_list(&mut self) -> Block {
        let block_start = self.line_start(self.pos);
        let first_line = self.lines[self.pos];
        let trimmed = first_line.trim_start();
        let ordered = trimmed.starts_with('#');

        let pos_before = self.pos;
        let (items, _) = self.parse_list_at_level(1, ordered);
        // Guard: if parse_list_at_level didn't advance pos (e.g. first line is at
        // level > 1 so it immediately broke), advance by one to prevent the caller
        // from looping on the same line indefinitely.
        if self.pos == pos_before {
            self.pos += 1;
        }
        let block_end = self.line_end(self.pos.saturating_sub(1));
        Block::List {
            ordered,
            items,
            span: Span::new(block_start, block_end),
        }
    }

    fn parse_list_at_level(&mut self, level: usize, ordered: bool) -> (Vec<Vec<Block>>, bool) {
        let marker = if ordered { '#' } else { '*' };
        let mut items = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            let trimmed = line.trim_start();

            let marker_count = trimmed.chars().take_while(|&c| c == marker).count();

            if marker_count == 0 {
                let other_marker = if ordered { '*' } else { '#' };
                let other_count = trimmed.chars().take_while(|&c| c == other_marker).count();
                if other_count == 0 {
                    break;
                }
                if other_count <= level {
                    break;
                }
            }

            if marker_count < level {
                break;
            }

            if marker_count == level
                && trimmed.len() > marker_count
                && trimmed.chars().nth(marker_count) == Some(' ')
            {
                let content = trimmed[marker_count + 1..].trim();
                let line_start = self.line_start(self.pos);
                let inline_nodes = parse_inline(content, line_start + marker_count + 1);
                let para = Block::Paragraph {
                    inlines: inline_nodes,
                    span: Span::new(line_start, self.line_end(self.pos)),
                };
                let mut item_children = vec![para];

                self.pos += 1;

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
                        let (nested_items, _) =
                            self.parse_list_at_level(next_marker_count, ordered);
                        let nested_start = self.line_start(self.pos);
                        item_children.push(Block::List {
                            ordered,
                            items: nested_items,
                            span: Span::new(nested_start, self.line_end(self.pos.saturating_sub(1))),
                        });
                    } else if next_other_count > level {
                        let (nested_items, _) =
                            self.parse_list_at_level(next_other_count, !ordered);
                        let nested_start = self.line_start(self.pos);
                        item_children.push(Block::List {
                            ordered: !ordered,
                            items: nested_items,
                            span: Span::new(nested_start, self.line_end(self.pos.saturating_sub(1))),
                        });
                    }
                }

                items.push(item_children);
            } else if marker_count > level {
                break;
            } else {
                self.pos += 1;
            }
        }

        (items, ordered)
    }

    fn parse_paragraph(&mut self) -> Block {
        let block_start = self.line_start(self.pos);
        let mut text = String::new();
        let first_line = self.lines[self.pos];

        let first_content = first_line
            .strip_prefix("p.")
            .map(|s| s.trim())
            .unwrap_or_else(|| first_line.trim());

        text.push_str(first_content);
        self.pos += 1;

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            if line.trim().is_empty() {
                break;
            }

            if line.starts_with("h1.")
                || line.starts_with("h2.")
                || line.starts_with("h3.")
                || line.starts_with("h4.")
                || line.starts_with("h5.")
                || line.starts_with("h6.")
                || line.starts_with("bc.")
                || line.starts_with("bq.")
                || line.starts_with("pre.")
                || line.starts_with("p.")
                || line.trim_start().starts_with('|')
                || line.trim_start().starts_with("* ")
                || line.trim_start().starts_with("# ")
            {
                break;
            }

            if !text.is_empty() {
                text.push(' ');
            }
            text.push_str(line.trim());
            self.pos += 1;
        }

        let block_end = self.line_end(self.pos.saturating_sub(1));
        let inline_nodes = parse_inline(&text, block_start);
        Block::Paragraph {
            inlines: inline_nodes,
            span: Span::new(block_start, block_end),
        }
    }
}

// ── Inline parser ─────────────────────────────────────────────────────────────

pub(crate) fn parse_inline(text: &str, base_offset: usize) -> Vec<Inline> {
    let mut nodes = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    // char_offsets[i] = byte offset of chars[i] within text
    let char_offsets: Vec<usize> = {
        let mut off = 0usize;
        let mut v = Vec::with_capacity(chars.len() + 1);
        for c in &chars {
            v.push(off);
            off += c.len_utf8();
        }
        v.push(off); // sentinel
        v
    };

    let char_abs = |idx: usize| base_offset + char_offsets.get(idx).copied().unwrap_or(text.len());

    let mut text_start = base_offset; // start of `current` text accumulation

    while i < chars.len() {
        // Inline code @...@
        if chars[i] == '@' {
            if !current.is_empty() {
                nodes.push(Inline::Text(
                    current.clone(),
                    Span::new(text_start, char_abs(i)),
                ));
                current.clear();
            }

            let code_start = char_abs(i);
            i += 1;
            let mut code = String::new();
            while i < chars.len() && chars[i] != '@' {
                code.push(chars[i]);
                i += 1;
            }
            if i < chars.len() {
                i += 1; // skip closing @
            }
            let code_end = char_abs(i);
            nodes.push(Inline::Code(code, Span::new(code_start, code_end)));
            text_start = char_abs(i);
            continue;
        }

        // Try to parse formatting markers
        if let Some((new_i, node)) = try_parse_formatting(&chars, i, &mut current, &mut nodes, &char_offsets, base_offset) {
            text_start = char_abs(new_i);
            i = new_i;
            nodes.push(node);
            continue;
        }

        // Link "text":url
        if chars[i] == '"'
            && let Some((link_end, link_text, url)) = parse_textile_link(&chars, i)
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(
                    current.clone(),
                    Span::new(text_start, char_abs(i)),
                ));
                current.clear();
            }
            let link_start = char_abs(i);
            let link_abs_end = char_abs(link_end);
            let text_node = Inline::Text(link_text, Span::new(link_start + 1, link_abs_end));
            nodes.push(Inline::Link {
                url,
                children: vec![text_node],
                span: Span::new(link_start, link_abs_end),
            });
            i = link_end;
            text_start = char_abs(i);
            continue;
        }

        // Image !url!
        if chars[i] == '!'
            && let Some((img_end, url, alt)) = parse_textile_image(&chars, i)
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(
                    current.clone(),
                    Span::new(text_start, char_abs(i)),
                ));
                current.clear();
            }
            let img_span = Span::new(char_abs(i), char_abs(img_end));
            nodes.push(Inline::Image {
                url,
                alt,
                span: img_span,
            });
            i = img_end;
            text_start = char_abs(i);
            continue;
        }

        current.push(chars[i]);
        i += 1;
    }

    if !current.is_empty() {
        nodes.push(Inline::Text(
            current,
            Span::new(text_start, base_offset + text.len()),
        ));
    }

    nodes
}

fn try_parse_formatting(
    chars: &[char],
    i: usize,
    current: &mut String,
    nodes: &mut Vec<Inline>,
    char_offsets: &[usize],
    base_offset: usize,
) -> Option<(usize, Inline)> {
    let char_abs = |idx: usize| {
        base_offset + char_offsets.get(idx).copied().unwrap_or_else(|| {
            char_offsets.last().copied().unwrap_or(0)
        })
    };

    let markers: &[(char, char, bool)] = &[
        ('*', '*', true),
        ('_', '_', true),
        ('-', '-', true),
        ('+', '+', true),
        ('^', ' ', false),
        ('~', ' ', false),
    ];

    for &(marker, doubled, check_prev) in markers {
        if chars[i] != marker {
            continue;
        }

        if check_prev && i > 0 && chars[i - 1].is_alphanumeric() {
            continue;
        }

        if i + 1 >= chars.len() || chars[i + 1] == ' ' {
            continue;
        }

        if doubled != ' ' && chars[i + 1] == doubled {
            continue;
        }

        if let Some((end, content)) = find_closing_marker(chars, i + 1, marker) {
            if !current.is_empty() {
                let text_end = char_abs(i);
                // We don't know text_start here, pass dummy — caller manages text_start
                nodes.push(Inline::Text(current.clone(), Span::new(0, text_end)));
                current.clear();
            }
            let fmt_start = char_abs(i);
            let fmt_end = char_abs(end + 1);
            let inner = parse_inline(&content, fmt_start + 1);

            let result = match marker {
                '*' => Inline::Bold(inner, Span::new(fmt_start, fmt_end)),
                '_' => Inline::Italic(inner, Span::new(fmt_start, fmt_end)),
                '-' => Inline::Strikethrough(inner, Span::new(fmt_start, fmt_end)),
                '+' => Inline::Underline(inner, Span::new(fmt_start, fmt_end)),
                '^' => Inline::Superscript(inner, Span::new(fmt_start, fmt_end)),
                '~' => Inline::Subscript(inner, Span::new(fmt_start, fmt_end)),
                _ => return None,
            };

            return Some((end + 1, result));
        }
    }

    None
}

fn find_closing_marker(chars: &[char], start: usize, marker: char) -> Option<(usize, String)> {
    let mut i = start;
    let mut content = String::new();

    while i < chars.len() {
        if chars[i] == marker && (i + 1 >= chars.len() || !chars[i + 1].is_alphanumeric()) {
            return Some((i, content));
        }
        content.push(chars[i]);
        i += 1;
    }
    None
}

fn parse_textile_link(chars: &[char], start: usize) -> Option<(usize, String, String)> {
    if chars[start] != '"' {
        return None;
    }

    let mut i = start + 1;
    let mut link_text = String::new();

    while i < chars.len() && chars[i] != '"' {
        link_text.push(chars[i]);
        i += 1;
    }

    if i >= chars.len() || chars[i] != '"' {
        return None;
    }
    i += 1;

    if i >= chars.len() || chars[i] != ':' {
        return None;
    }
    i += 1;

    let mut url = String::new();
    while i < chars.len() && !chars[i].is_whitespace() {
        url.push(chars[i]);
        i += 1;
    }

    if url.is_empty() {
        return None;
    }

    Some((i, link_text, url))
}

fn parse_textile_image(chars: &[char], start: usize) -> Option<(usize, String, Option<String>)> {
    if chars[start] != '!' {
        return None;
    }

    let mut i = start + 1;
    let mut url = String::new();
    let mut alt = None;

    while i < chars.len() && chars[i] != '!' && chars[i] != '(' {
        url.push(chars[i]);
        i += 1;
    }

    if i >= chars.len() {
        return None;
    }

    if chars[i] == '(' {
        i += 1;
        let mut alt_text = String::new();
        while i < chars.len() && chars[i] != ')' {
            alt_text.push(chars[i]);
            i += 1;
        }
        if i < chars.len() && chars[i] == ')' {
            alt = Some(alt_text);
            i += 1;
        }
    }

    if i >= chars.len() || chars[i] != '!' {
        return None;
    }
    i += 1;

    if url.is_empty() {
        return None;
    }

    Some((i, url, alt))
}
