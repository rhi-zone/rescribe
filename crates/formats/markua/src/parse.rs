//! Markua parser — infallible, returns (MarkuaDoc, Vec<Diagnostic>).

use crate::ast::{Block, Diagnostic, Inline, MarkuaDoc, Span};

/// Parse a Markua string into a [`MarkuaDoc`].
///
/// Always succeeds — malformed markup is tolerated and may produce diagnostics.
pub fn parse(input: &str) -> (MarkuaDoc, Vec<Diagnostic>) {
    let diagnostics = Vec::new();
    let offsets = line_byte_offsets(input);
    let mut p = Parser::new(input, &offsets);
    let blocks = p.parse();
    let doc = MarkuaDoc {
        blocks,
        span: Span::new(0, input.len()),
    };
    (doc, diagnostics)
}

fn line_byte_offsets(input: &str) -> Vec<usize> {
    let mut offsets = vec![0usize];
    let mut pos = 0;
    for ch in input.chars() {
        pos += ch.len_utf8();
        if ch == '\n' {
            offsets.push(pos);
        }
    }
    offsets
}

struct Parser<'a> {
    lines: Vec<&'a str>,
    offsets: &'a [usize],
    pos: usize,
    input_len: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str, offsets: &'a [usize]) -> Self {
        Self {
            lines: input.lines().collect(),
            offsets,
            pos: 0,
            input_len: input.len(),
        }
    }

    fn line_start(&self, line_idx: usize) -> usize {
        self.offsets.get(line_idx).copied().unwrap_or(self.input_len)
    }

    fn parse(&mut self) -> Vec<Block> {
        let mut nodes = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            if line.trim().is_empty() {
                self.pos += 1;
                continue;
            }

            // ATX headings: # Title
            if let Some(block) = self.try_parse_atx_heading(line) {
                nodes.push(block);
                self.pos += 1;
                continue;
            }

            // Scene break: * * * or - - - or *** or ---
            if self.is_scene_break(line) {
                let span = Span::new(self.line_start(self.pos), self.line_start(self.pos + 1));
                nodes.push(Block::HorizontalRule { span });
                self.pos += 1;
                continue;
            }

            // Fenced code block
            if line.trim_start().starts_with("```") || line.trim_start().starts_with("~~~") {
                nodes.push(self.parse_fenced_code_block());
                continue;
            }

            // Markua special blocks: A>, B>, W>, T>, E>, D>, Q>, I>
            if let Some(block_type) = Self::get_special_block_type(line) {
                nodes.push(self.parse_special_block(block_type));
                continue;
            }

            // Blockquote: > text
            if line.trim_start().starts_with("> ") || line.trim_start() == ">" {
                nodes.push(self.parse_blockquote());
                continue;
            }

            // Unordered list: - or * or +
            let trimmed = line.trim_start();
            if (trimmed.starts_with("- ")
                || trimmed.starts_with("* ")
                || trimmed.starts_with("+ "))
                && !self.is_scene_break(line)
            {
                nodes.push(self.parse_list(false));
                continue;
            }

            // Ordered list: 1. or 1)
            if self.is_ordered_list_item(line) {
                nodes.push(self.parse_list(true));
                continue;
            }

            // Regular paragraph
            nodes.push(self.parse_paragraph());
        }

        nodes
    }

    fn try_parse_atx_heading(&self, line: &str) -> Option<Block> {
        let trimmed = line.trim_start();
        if !trimmed.starts_with('#') {
            return None;
        }

        let level = trimmed.chars().take_while(|&c| c == '#').count();
        if level == 0 || level > 6 {
            return None;
        }

        let rest = &trimmed[level..];
        if !rest.is_empty() && !rest.starts_with(' ') {
            return None;
        }

        let title = rest.trim().trim_end_matches('#').trim();
        let inlines = parse_inline(title);
        let span = Span::new(self.line_start(self.pos), self.line_start(self.pos + 1));

        Some(Block::Heading {
            level: level as u8,
            inlines,
            span,
        })
    }

    fn is_scene_break(&self, line: &str) -> bool {
        let trimmed = line.trim();
        if trimmed.len() < 3 {
            return false;
        }
        let chars: Vec<char> = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
        if chars.len() < 3 {
            return false;
        }
        let first = chars[0];
        (first == '*' || first == '-' || first == '_') && chars.iter().all(|&c| c == first)
    }

    fn get_special_block_type(line: &str) -> Option<String> {
        let trimmed = line.trim_start();
        let prefixes = [
            ("A> ", "aside"),
            ("B> ", "blurb"),
            ("W> ", "warning"),
            ("T> ", "tip"),
            ("E> ", "error"),
            ("D> ", "discussion"),
            ("Q> ", "question"),
            ("I> ", "information"),
        ];
        for (prefix, block_type) in prefixes {
            if trimmed.starts_with(prefix) {
                return Some(block_type.to_string());
            }
        }
        None
    }

    fn parse_special_block(&mut self, block_type: String) -> Block {
        let start = self.line_start(self.pos);
        let prefix = match block_type.as_str() {
            "aside" => "A> ",
            "blurb" => "B> ",
            "warning" => "W> ",
            "tip" => "T> ",
            "error" => "E> ",
            "discussion" => "D> ",
            "question" => "Q> ",
            "information" => "I> ",
            _ => {
                return Block::Paragraph {
                    inlines: Vec::new(),
                    span: Span::new(self.line_start(self.pos), self.line_start(self.pos + 1)),
                };
            }
        };

        let mut content = String::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            let trimmed = line.trim_start();

            if let Some(rest) = trimmed.strip_prefix(prefix) {
                if !content.is_empty() {
                    content.push(' ');
                }
                content.push_str(rest);
                self.pos += 1;
            } else if trimmed.is_empty() {
                self.pos += 1;
                break;
            } else {
                break;
            }
        }

        let end = self.line_start(self.pos);
        let inlines = parse_inline(&content);
        Block::SpecialBlock {
            block_type,
            inlines,
            span: Span::new(start, end),
        }
    }

    fn parse_fenced_code_block(&mut self) -> Block {
        let start = self.line_start(self.pos);
        let first_line = self.lines[self.pos].trim_start();
        let fence_char = first_line.chars().next().unwrap_or('`');
        let fence_len = first_line.chars().take_while(|&c| c == fence_char).count();

        let info_string = first_line[fence_len..].trim();
        let language = if info_string.is_empty() {
            None
        } else {
            Some(
                info_string
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_string(),
            )
        };

        self.pos += 1;
        let mut content = String::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            let trimmed = line.trim_start();

            if trimmed.starts_with(fence_char)
                && trimmed.chars().take_while(|&c| c == fence_char).count() >= fence_len
            {
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
            language,
            span: Span::new(start, end),
        }
    }

    fn parse_blockquote(&mut self) -> Block {
        let start = self.line_start(self.pos);
        let mut content = String::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            let trimmed = line.trim_start();

            if let Some(rest) = trimmed.strip_prefix("> ") {
                if !content.is_empty() {
                    content.push(' ');
                }
                content.push_str(rest);
                self.pos += 1;
            } else if trimmed == ">" {
                self.pos += 1;
            } else if trimmed.is_empty() {
                self.pos += 1;
                break;
            } else {
                break;
            }
        }

        let end = self.line_start(self.pos);
        let inlines = parse_inline(&content);
        let para = Block::Paragraph {
            inlines,
            span: Span::NONE,
        };
        Block::Blockquote {
            children: vec![para],
            span: Span::new(start, end),
        }
    }

    fn is_ordered_list_item(&self, line: &str) -> bool {
        let trimmed = line.trim_start();
        let mut chars = trimmed.chars();
        let mut has_digit = false;

        while let Some(c) = chars.next() {
            if c.is_ascii_digit() {
                has_digit = true;
            } else if has_digit && (c == '.' || c == ')') {
                match chars.next() {
                    Some(' ') | None => return true,
                    _ => return false,
                }
            } else {
                return false;
            }
        }
        false
    }

    fn parse_list(&mut self, ordered: bool) -> Block {
        let start = self.line_start(self.pos);
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

            let is_bullet = trimmed.starts_with("- ")
                || trimmed.starts_with("* ")
                || trimmed.starts_with("+ ");
            let is_numbered = self.is_ordered_list_item(line);

            if !is_bullet && !is_numbered {
                break;
            }

            let content = if is_bullet {
                &trimmed[2..]
            } else {
                let marker_end = trimmed.find(". ").or_else(|| trimmed.find(") "));
                if let Some(pos) = marker_end {
                    &trimmed[pos + 2..]
                } else {
                    // Marker with no trailing space (e.g. "3." at end of line):
                    // advance past this line to avoid an infinite loop.
                    self.pos += 1;
                    continue;
                }
            };

            let item_span =
                Span::new(self.line_start(self.pos), self.line_start(self.pos + 1));
            let inlines = parse_inline(content);
            let para = Block::Paragraph {
                inlines,
                span: item_span,
            };
            items.push(vec![para]);
            self.pos += 1;
        }

        let end = self.line_start(self.pos);
        Block::List {
            ordered,
            items,
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

            let trimmed = line.trim_start();
            if self.try_parse_atx_heading(line).is_some()
                || self.is_scene_break(line)
                || trimmed.starts_with("```")
                || trimmed.starts_with("~~~")
                || trimmed.starts_with("> ")
                || Self::get_special_block_type(line).is_some()
                || trimmed.starts_with("- ")
                || trimmed.starts_with("* ")
                || trimmed.starts_with("+ ")
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
        let inlines = parse_inline(&text);
        Block::Paragraph {
            inlines,
            span: Span::new(start, end),
        }
    }
}

// ── Inline parser ─────────────────────────────────────────────────────────────

fn parse_inline(text: &str) -> Vec<Inline> {
    let mut nodes = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Strong: **text** or __text__
        if i + 1 < chars.len()
            && ((chars[i] == '*' && chars[i + 1] == '*')
                || (chars[i] == '_' && chars[i + 1] == '_'))
        {
            let marker = chars[i];
            if let Some((end, content)) = find_double_marker(&chars, i + 2, marker) {
                if !current.is_empty() {
                    nodes.push(Inline::Text(current.clone(), Span::NONE));
                    current.clear();
                }
                let inner = parse_inline(&content);
                nodes.push(Inline::Strong(inner, Span::NONE));
                i = end + 2;
                continue;
            }
        }

        // Strikethrough: ~~text~~
        if i + 1 < chars.len()
            && chars[i] == '~'
            && chars[i + 1] == '~'
            && let Some((end, content)) = find_double_marker(&chars, i + 2, '~')
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            let inner = parse_inline(&content);
            nodes.push(Inline::Strikethrough(inner, Span::NONE));
            i = end + 2;
            continue;
        }

        // Emphasis: *text* or _text_
        if chars[i] == '*' || chars[i] == '_' {
            let marker = chars[i];
            if let Some((end, content)) = find_single_marker(&chars, i + 1, marker) {
                if !current.is_empty() {
                    nodes.push(Inline::Text(current.clone(), Span::NONE));
                    current.clear();
                }
                let inner = parse_inline(&content);
                nodes.push(Inline::Emphasis(inner, Span::NONE));
                i = end + 1;
                continue;
            }
        }

        // Inline code: `code`
        if chars[i] == '`'
            && let Some((end, content)) = find_backtick_content(&chars, i)
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            nodes.push(Inline::Code(content, Span::NONE));
            i = end;
            continue;
        }

        // Image: ![alt](url) — check before Link
        if chars[i] == '!'
            && i + 1 < chars.len()
            && chars[i + 1] == '['
            && let Some((end, alt, url)) = parse_link(&chars, i + 1)
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            nodes.push(Inline::Image {
                url,
                alt,
                span: Span::NONE,
            });
            i = end;
            continue;
        }

        // Link: [text](url)
        if chars[i] == '['
            && let Some((end, link_text, url)) = parse_link(&chars, i)
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            let text_nodes = parse_inline(&link_text);
            nodes.push(Inline::Link {
                url,
                children: text_nodes,
                span: Span::NONE,
            });
            i = end;
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

fn find_double_marker(chars: &[char], start: usize, marker: char) -> Option<(usize, String)> {
    let mut i = start;
    let mut content = String::new();

    while i + 1 < chars.len() {
        if chars[i] == marker && chars[i + 1] == marker {
            if !content.is_empty() {
                return Some((i, content));
            }
            return None;
        }
        content.push(chars[i]);
        i += 1;
    }
    None
}

fn find_single_marker(chars: &[char], start: usize, marker: char) -> Option<(usize, String)> {
    let mut i = start;
    let mut content = String::new();

    while i < chars.len() {
        if chars[i] == marker {
            if i + 1 < chars.len() && chars[i + 1] == marker {
                content.push(chars[i]);
                i += 1;
                continue;
            }
            if !content.is_empty() {
                return Some((i, content));
            }
            return None;
        }
        content.push(chars[i]);
        i += 1;
    }
    None
}

fn find_backtick_content(chars: &[char], start: usize) -> Option<(usize, String)> {
    let mut backtick_count = 0;
    let mut i = start;
    while i < chars.len() && chars[i] == '`' {
        backtick_count += 1;
        i += 1;
    }

    let mut content = String::new();
    while i < chars.len() {
        if chars[i] == '`' {
            let mut closing_count = 0;
            while i < chars.len() && chars[i] == '`' {
                closing_count += 1;
                i += 1;
            }
            if closing_count == backtick_count {
                return Some((i, content.trim().to_string()));
            }
            for _ in 0..closing_count {
                content.push('`');
            }
        } else {
            content.push(chars[i]);
            i += 1;
        }
    }
    None
}

fn parse_link(chars: &[char], start: usize) -> Option<(usize, String, String)> {
    if chars[start] != '[' {
        return None;
    }

    let mut i = start + 1;
    let mut link_text = String::new();

    while i < chars.len() && chars[i] != ']' {
        link_text.push(chars[i]);
        i += 1;
    }

    if i >= chars.len() {
        return None;
    }

    i += 1; // Skip ]

    if i >= chars.len() {
        return None;
    }

    if chars[i] == '(' {
        i += 1;
        let mut url = String::new();
        while i < chars.len() && chars[i] != ')' {
            url.push(chars[i]);
            i += 1;
        }
        if i < chars.len() {
            return Some((i + 1, link_text, url));
        }
    }

    None
}
