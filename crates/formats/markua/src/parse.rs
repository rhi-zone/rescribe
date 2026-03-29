//! Markua parser — infallible, returns (MarkuaDoc, Vec<Diagnostic>).

use crate::ast::{Block, Diagnostic, Inline, MarkuaDoc, Span, TableRow};

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
        title: None,
        author: None,
        description: None,
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

            // Page break: {pagebreak} or {page-break}
            let trimmed_lower = line.trim().to_lowercase();
            if trimmed_lower == "{pagebreak}" || trimmed_lower == "{page-break}" {
                let span = Span::new(self.line_start(self.pos), self.line_start(self.pos + 1));
                nodes.push(Block::PageBreak { span });
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

            // Table: | cell | cell |
            let trimmed = line.trim_start();
            if trimmed.starts_with('|') {
                nodes.push(self.parse_table());
                continue;
            }

            // Markua special blocks: A>, B>, W>, T>, E>, D>, Q>, I>, X>
            if let Some(block_type) = Self::get_special_block_type(line) {
                nodes.push(self.parse_special_block(block_type));
                continue;
            }

            // Blockquote: > text
            if trimmed.starts_with("> ") || trimmed == ">" {
                nodes.push(self.parse_blockquote());
                continue;
            }

            // Definition list: Term
            // : Definition
            if self.is_definition_list_start() {
                nodes.push(self.parse_definition_list());
                continue;
            }

            // Unordered list: - or * or +
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

            // Regular paragraph (may become a Figure if it's an image-only paragraph)
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
            ("X> ", "exercise"),
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
            "exercise" => "X> ",
            _ => {
                return Block::Paragraph {
                    inlines: Vec::new(),
                    span: Span::new(self.line_start(self.pos), self.line_start(self.pos + 1)),
                };
            }
        };

        let mut content_lines = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            let trimmed = line.trim_start();

            if let Some(rest) = trimmed.strip_prefix(prefix) {
                content_lines.push(rest.to_string());
                self.pos += 1;
            } else if trimmed.is_empty() {
                self.pos += 1;
                break;
            } else {
                break;
            }
        }

        let end = self.line_start(self.pos);
        // Parse the collected content as blocks (rejoin with newlines for sub-parsing)
        let content = content_lines.join("\n");
        let inner_offsets = line_byte_offsets(&content);
        let mut inner_parser = Parser::new(&content, &inner_offsets);
        let children = inner_parser.parse();

        // If no blocks were parsed from the content, create a paragraph with it
        let children = if children.is_empty() && !content.trim().is_empty() {
            vec![Block::Paragraph {
                inlines: parse_inline(content.trim()),
                span: Span::NONE,
            }]
        } else {
            children
        };

        Block::SpecialBlock {
            block_type,
            children,
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
        let mut content_lines = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            let trimmed = line.trim_start();

            if let Some(rest) = trimmed.strip_prefix("> ") {
                content_lines.push(rest.to_string());
                self.pos += 1;
            } else if trimmed == ">" {
                content_lines.push(String::new());
                self.pos += 1;
            } else if trimmed.is_empty() {
                self.pos += 1;
                break;
            } else {
                break;
            }
        }

        let end = self.line_start(self.pos);
        let content = content_lines.join("\n");
        let inner_offsets = line_byte_offsets(&content);
        let mut inner_parser = Parser::new(&content, &inner_offsets);
        let children = inner_parser.parse();

        let children = if children.is_empty() && !content.trim().is_empty() {
            let inlines = parse_inline(content.trim());
            vec![Block::Paragraph {
                inlines,
                span: Span::NONE,
            }]
        } else {
            children
        };

        Block::Blockquote {
            children,
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

    fn parse_table(&mut self) -> Block {
        let start = self.line_start(self.pos);
        let mut rows = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            let trimmed = line.trim();

            if !trimmed.starts_with('|') {
                break;
            }

            // Skip separator rows (| --- | --- |)
            let stripped = trimmed.trim_start_matches('|').trim_end_matches('|');
            let is_separator = stripped
                .split('|')
                .all(|cell| {
                    let c = cell.trim();
                    c.chars().all(|ch| ch == '-' || ch == ':' || ch == ' ') && !c.is_empty()
                });

            if is_separator {
                self.pos += 1;
                continue;
            }

            // Parse cells
            let cells: Vec<Vec<Inline>> = stripped
                .split('|')
                .map(|cell| parse_inline(cell.trim()))
                .collect();

            let row_span = Span::new(self.line_start(self.pos), self.line_start(self.pos + 1));
            rows.push(TableRow {
                cells,
                span: row_span,
            });
            self.pos += 1;
        }

        let end = self.line_start(self.pos);
        Block::Table {
            rows,
            span: Span::new(start, end),
        }
    }

    fn is_definition_list_start(&self) -> bool {
        // Definition list: a term line followed by a line starting with ": "
        if self.pos + 1 >= self.lines.len() {
            return false;
        }
        let next = self.lines[self.pos + 1].trim_start();
        next.starts_with(": ")
    }

    fn parse_definition_list(&mut self) -> Block {
        let start = self.line_start(self.pos);
        let mut items = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            let trimmed = line.trim();

            if trimmed.is_empty() {
                self.pos += 1;
                break;
            }

            // Check if the next line is a definition (starts with ": ")
            if self.pos + 1 < self.lines.len() {
                let next = self.lines[self.pos + 1].trim_start();
                if next.starts_with(": ") {
                    let term = parse_inline(trimmed);
                    self.pos += 1; // move to definition line

                    let mut def_content = String::new();
                    while self.pos < self.lines.len() {
                        let def_line = self.lines[self.pos].trim_start();
                        if let Some(rest) = def_line.strip_prefix(": ") {
                            if !def_content.is_empty() {
                                def_content.push(' ');
                            }
                            def_content.push_str(rest);
                            self.pos += 1;
                        } else {
                            break;
                        }
                    }

                    let def_blocks = vec![Block::Paragraph {
                        inlines: parse_inline(&def_content),
                        span: Span::NONE,
                    }];
                    items.push((term, def_blocks));
                    continue;
                }
            }

            break;
        }

        let end = self.line_start(self.pos);
        Block::DefinitionList {
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

            // Page break check
            let lower = trimmed.to_lowercase();
            if lower == "{pagebreak}" || lower == "{page-break}" {
                break;
            }

            if self.try_parse_atx_heading(line).is_some()
                || self.is_scene_break(line)
                || trimmed.starts_with("```")
                || trimmed.starts_with("~~~")
                || trimmed.starts_with("> ")
                || trimmed.starts_with('|')
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

pub(crate) fn parse_inline(text: &str) -> Vec<Inline> {
    let mut nodes = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Line break: backslash at end or two spaces (handled via \n detection)
        if chars[i] == '\\' && i + 1 < chars.len() && chars[i + 1] == '\n' {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            nodes.push(Inline::LineBreak(Span::NONE));
            i += 2;
            continue;
        }

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

        // Math inline: $expr$ (not $$)
        if chars[i] == '$'
            && (i + 1 < chars.len() && chars[i + 1] != '$')
            && let Some((end, content)) = find_single_marker(&chars, i + 1, '$')
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            nodes.push(Inline::MathInline {
                content,
                span: Span::NONE,
            });
            i = end + 1;
            continue;
        }

        // Footnote ref: ^[text]
        if chars[i] == '^' && i + 1 < chars.len() && chars[i + 1] == '['
            && let Some((end, content)) = find_bracket_content(&chars, i + 1)
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            let inner = parse_inline(&content);
            nodes.push(Inline::FootnoteRef {
                content: inner,
                span: Span::NONE,
            });
            i = end;
            continue;
        }

        // Index term: i[term]
        if chars[i] == 'i' && i + 1 < chars.len() && chars[i + 1] == '['
            && !(i > 0 && chars[i - 1].is_alphanumeric())
            && let Some((end, content)) = find_bracket_content(&chars, i + 1)
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            nodes.push(Inline::IndexTerm {
                term: content,
                span: Span::NONE,
            });
            i = end;
            continue;
        }

        // Superscript: ^text^ (single caret, not footnote ^[)
        if chars[i] == '^' && !(i + 1 < chars.len() && chars[i + 1] == '[')
            && let Some((end, content)) = find_single_marker(&chars, i + 1, '^')
            && !content.is_empty()
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            let inner = parse_inline(&content);
            nodes.push(Inline::Superscript(inner, Span::NONE));
            i = end + 1;
            continue;
        }

        // Subscript: ~text~ (single tilde, not ~~strikethrough)
        if chars[i] == '~' && !(i + 1 < chars.len() && chars[i + 1] == '~')
            && let Some((end, content)) = find_single_marker(&chars, i + 1, '~')
            && !content.is_empty()
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            let inner = parse_inline(&content);
            nodes.push(Inline::Subscript(inner, Span::NONE));
            i = end + 1;
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

fn find_bracket_content(chars: &[char], start: usize) -> Option<(usize, String)> {
    if start >= chars.len() || chars[start] != '[' {
        return None;
    }
    let mut i = start + 1;
    let mut content = String::new();
    let mut depth = 1;

    while i < chars.len() {
        if chars[i] == '[' {
            depth += 1;
        } else if chars[i] == ']' {
            depth -= 1;
            if depth == 0 {
                return Some((i + 1, content));
            }
        }
        content.push(chars[i]);
        i += 1;
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

// ── EventIter ─────────────────────────────────────────────────────────────────

/// Frame for the lazy event iterator.
enum Frame {
    Event(crate::events::OwnedMarkuaEvent),
    Blocks(std::vec::IntoIter<Block>),
    Inlines(std::vec::IntoIter<Inline>),
    ListItems(std::vec::IntoIter<Vec<Block>>),
    TableRows(std::vec::IntoIter<TableRow>),
    TableCells(std::vec::IntoIter<Vec<Inline>>),
    DefinitionItems(std::vec::IntoIter<(Vec<Inline>, Vec<Block>)>),
}

/// Pull-based event iterator over a Markua document.
///
/// The parser IS the iterator: `EventIter` holds the parser state;
/// `next()` advances it and returns one event.
pub struct EventIter<'a> {
    #[allow(dead_code)]
    lines: Vec<&'a str>,
    #[allow(dead_code)]
    pos: usize,
    pub diagnostics: Vec<Diagnostic>,
    frame_stack: Vec<Frame>,
    pub(crate) done: bool,
}

impl<'a> EventIter<'a> {
    pub fn new(input: &'a str) -> Self {
        let offsets = line_byte_offsets(input);
        let mut p = Parser::new(input, &offsets);
        let blocks = p.parse();
        let mut iter = Self {
            lines: input.lines().collect(),
            pos: 0,
            diagnostics: Vec::new(),
            frame_stack: Vec::new(),
            done: false,
        };
        if !blocks.is_empty() {
            iter.frame_stack.push(Frame::Blocks(blocks.into_iter()));
        }
        iter
    }

    fn expand_block(&mut self, block: Block) {
        use crate::events::MarkuaEvent;
        match block {
            Block::Paragraph { inlines, .. } => {
                self.frame_stack.push(Frame::Event(MarkuaEvent::EndParagraph));
                if !inlines.is_empty() {
                    self.frame_stack.push(Frame::Inlines(inlines.into_iter()));
                }
                self.frame_stack.push(Frame::Event(MarkuaEvent::StartParagraph));
            }
            Block::Heading { level, inlines, .. } => {
                self.frame_stack.push(Frame::Event(MarkuaEvent::EndHeading));
                if !inlines.is_empty() {
                    self.frame_stack.push(Frame::Inlines(inlines.into_iter()));
                }
                self.frame_stack.push(Frame::Event(MarkuaEvent::StartHeading { level }));
            }
            Block::CodeBlock { content, language, .. } => {
                self.frame_stack.push(Frame::Event(MarkuaEvent::CodeBlock {
                    language,
                    content: std::borrow::Cow::Owned(content),
                }));
            }
            Block::Blockquote { children, .. } => {
                self.frame_stack.push(Frame::Event(MarkuaEvent::EndBlockquote));
                if !children.is_empty() {
                    self.frame_stack.push(Frame::Blocks(children.into_iter()));
                }
                self.frame_stack.push(Frame::Event(MarkuaEvent::StartBlockquote));
            }
            Block::List { ordered, items, .. } => {
                self.frame_stack.push(Frame::Event(MarkuaEvent::EndList));
                if !items.is_empty() {
                    self.frame_stack.push(Frame::ListItems(items.into_iter()));
                }
                self.frame_stack.push(Frame::Event(MarkuaEvent::StartList { ordered }));
            }
            Block::Table { rows, .. } => {
                self.frame_stack.push(Frame::Event(MarkuaEvent::EndTable));
                if !rows.is_empty() {
                    self.frame_stack.push(Frame::TableRows(rows.into_iter()));
                }
                self.frame_stack.push(Frame::Event(MarkuaEvent::StartTable));
            }
            Block::HorizontalRule { .. } => {
                self.frame_stack.push(Frame::Event(MarkuaEvent::HorizontalRule));
            }
            Block::SpecialBlock { block_type, children, .. } => {
                self.frame_stack.push(Frame::Event(MarkuaEvent::EndSpecialBlock));
                if !children.is_empty() {
                    self.frame_stack.push(Frame::Blocks(children.into_iter()));
                }
                self.frame_stack.push(Frame::Event(MarkuaEvent::StartSpecialBlock { kind: block_type }));
            }
            Block::DefinitionList { items, .. } => {
                self.frame_stack.push(Frame::Event(MarkuaEvent::EndDefinitionList));
                if !items.is_empty() {
                    self.frame_stack.push(Frame::DefinitionItems(items.into_iter()));
                }
                self.frame_stack.push(Frame::Event(MarkuaEvent::StartDefinitionList));
            }
            Block::PageBreak { .. } => {
                self.frame_stack.push(Frame::Event(MarkuaEvent::PageBreak));
            }
            Block::Figure { caption, body, .. } => {
                self.frame_stack.push(Frame::Event(MarkuaEvent::EndFigure));
                self.expand_block(*body);
                if !caption.is_empty() {
                    self.frame_stack.push(Frame::Event(MarkuaEvent::EndCaption));
                    self.frame_stack.push(Frame::Inlines(caption.into_iter()));
                    self.frame_stack.push(Frame::Event(MarkuaEvent::StartCaption));
                }
                self.frame_stack.push(Frame::Event(MarkuaEvent::StartFigure));
            }
        }
    }

    fn expand_inline(&mut self, inline: Inline) {
        use crate::events::MarkuaEvent;
        match inline {
            Inline::Text(text, _) => {
                self.frame_stack.push(Frame::Event(MarkuaEvent::Text(std::borrow::Cow::Owned(text))));
            }
            Inline::Strong(children, _) => {
                self.frame_stack.push(Frame::Event(MarkuaEvent::EndStrong));
                if !children.is_empty() {
                    self.frame_stack.push(Frame::Inlines(children.into_iter()));
                }
                self.frame_stack.push(Frame::Event(MarkuaEvent::StartStrong));
            }
            Inline::Emphasis(children, _) => {
                self.frame_stack.push(Frame::Event(MarkuaEvent::EndEmphasis));
                if !children.is_empty() {
                    self.frame_stack.push(Frame::Inlines(children.into_iter()));
                }
                self.frame_stack.push(Frame::Event(MarkuaEvent::StartEmphasis));
            }
            Inline::Strikethrough(children, _) => {
                self.frame_stack.push(Frame::Event(MarkuaEvent::EndStrikethrough));
                if !children.is_empty() {
                    self.frame_stack.push(Frame::Inlines(children.into_iter()));
                }
                self.frame_stack.push(Frame::Event(MarkuaEvent::StartStrikethrough));
            }
            Inline::Subscript(children, _) => {
                self.frame_stack.push(Frame::Event(MarkuaEvent::EndSubscript));
                if !children.is_empty() {
                    self.frame_stack.push(Frame::Inlines(children.into_iter()));
                }
                self.frame_stack.push(Frame::Event(MarkuaEvent::StartSubscript));
            }
            Inline::Superscript(children, _) => {
                self.frame_stack.push(Frame::Event(MarkuaEvent::EndSuperscript));
                if !children.is_empty() {
                    self.frame_stack.push(Frame::Inlines(children.into_iter()));
                }
                self.frame_stack.push(Frame::Event(MarkuaEvent::StartSuperscript));
            }
            Inline::Underline(children, _) => {
                self.frame_stack.push(Frame::Event(MarkuaEvent::EndUnderline));
                if !children.is_empty() {
                    self.frame_stack.push(Frame::Inlines(children.into_iter()));
                }
                self.frame_stack.push(Frame::Event(MarkuaEvent::StartUnderline));
            }
            Inline::SmallCaps(children, _) => {
                self.frame_stack.push(Frame::Event(MarkuaEvent::EndSmallCaps));
                if !children.is_empty() {
                    self.frame_stack.push(Frame::Inlines(children.into_iter()));
                }
                self.frame_stack.push(Frame::Event(MarkuaEvent::StartSmallCaps));
            }
            Inline::Code(content, _) => {
                self.frame_stack.push(Frame::Event(MarkuaEvent::InlineCode(std::borrow::Cow::Owned(content))));
            }
            Inline::Link { url, children, .. } => {
                self.frame_stack.push(Frame::Event(MarkuaEvent::EndLink));
                if !children.is_empty() {
                    self.frame_stack.push(Frame::Inlines(children.into_iter()));
                }
                self.frame_stack.push(Frame::Event(MarkuaEvent::StartLink { url }));
            }
            Inline::Image { url, alt, .. } => {
                self.frame_stack.push(Frame::Event(MarkuaEvent::Image { url, alt }));
            }
            Inline::LineBreak(_) => {
                self.frame_stack.push(Frame::Event(MarkuaEvent::LineBreak));
            }
            Inline::SoftBreak(_) => {
                self.frame_stack.push(Frame::Event(MarkuaEvent::SoftBreak));
            }
            Inline::FootnoteRef { content, .. } => {
                self.frame_stack.push(Frame::Event(MarkuaEvent::EndFootnoteRef));
                if !content.is_empty() {
                    self.frame_stack.push(Frame::Inlines(content.into_iter()));
                }
                self.frame_stack.push(Frame::Event(MarkuaEvent::StartFootnoteRef));
            }
            Inline::IndexTerm { term, .. } => {
                self.frame_stack.push(Frame::Event(MarkuaEvent::IndexTerm { term }));
            }
            Inline::MathInline { content, .. } => {
                self.frame_stack.push(Frame::Event(MarkuaEvent::MathInline { content }));
            }
        }
    }
}

impl<'a> Iterator for EventIter<'a> {
    type Item = crate::events::MarkuaEvent<'a>;

    fn next(&mut self) -> Option<crate::events::MarkuaEvent<'a>> {
        loop {
            match self.frame_stack.pop() {
                Some(Frame::Event(ev)) => return Some(ev),
                Some(Frame::Blocks(mut iter)) => {
                    if let Some(block) = iter.next() {
                        self.frame_stack.push(Frame::Blocks(iter));
                        self.expand_block(block);
                    }
                    continue;
                }
                Some(Frame::Inlines(mut iter)) => {
                    if let Some(inline) = iter.next() {
                        self.frame_stack.push(Frame::Inlines(iter));
                        self.expand_inline(inline);
                    }
                    continue;
                }
                Some(Frame::ListItems(mut iter)) => {
                    if let Some(item_blocks) = iter.next() {
                        self.frame_stack.push(Frame::ListItems(iter));
                        self.frame_stack.push(Frame::Event(crate::events::MarkuaEvent::EndListItem));
                        if !item_blocks.is_empty() {
                            self.frame_stack.push(Frame::Blocks(item_blocks.into_iter()));
                        }
                        self.frame_stack.push(Frame::Event(crate::events::MarkuaEvent::StartListItem));
                    }
                    continue;
                }
                Some(Frame::TableRows(mut iter)) => {
                    if let Some(row) = iter.next() {
                        self.frame_stack.push(Frame::TableRows(iter));
                        self.frame_stack.push(Frame::Event(crate::events::MarkuaEvent::EndTableRow));
                        if !row.cells.is_empty() {
                            self.frame_stack.push(Frame::TableCells(row.cells.into_iter()));
                        }
                        self.frame_stack.push(Frame::Event(crate::events::MarkuaEvent::StartTableRow));
                    }
                    continue;
                }
                Some(Frame::TableCells(mut iter)) => {
                    if let Some(cell_inlines) = iter.next() {
                        self.frame_stack.push(Frame::TableCells(iter));
                        self.frame_stack.push(Frame::Event(crate::events::MarkuaEvent::EndTableCell));
                        if !cell_inlines.is_empty() {
                            self.frame_stack.push(Frame::Inlines(cell_inlines.into_iter()));
                        }
                        self.frame_stack.push(Frame::Event(crate::events::MarkuaEvent::StartTableCell));
                    }
                    continue;
                }
                Some(Frame::DefinitionItems(mut iter)) => {
                    if let Some((term, def_blocks)) = iter.next() {
                        self.frame_stack.push(Frame::DefinitionItems(iter));
                        self.frame_stack.push(Frame::Event(crate::events::MarkuaEvent::EndDefinitionDesc));
                        if !def_blocks.is_empty() {
                            self.frame_stack.push(Frame::Blocks(def_blocks.into_iter()));
                        }
                        self.frame_stack.push(Frame::Event(crate::events::MarkuaEvent::StartDefinitionDesc));
                        self.frame_stack.push(Frame::Event(crate::events::MarkuaEvent::EndDefinitionTerm));
                        if !term.is_empty() {
                            self.frame_stack.push(Frame::Inlines(term.into_iter()));
                        }
                        self.frame_stack.push(Frame::Event(crate::events::MarkuaEvent::StartDefinitionTerm));
                    }
                    continue;
                }
                None => {
                    self.done = true;
                    return None;
                }
            }
        }
    }
}
