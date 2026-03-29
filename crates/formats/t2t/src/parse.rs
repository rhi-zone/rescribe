//! txt2tags parser.

use crate::ast::{Block, Diagnostic, Inline, Span, T2tDoc};

/// Parse a txt2tags string into a [`T2tDoc`] and a list of diagnostics.
///
/// Parsing is always infallible — malformed constructs produce diagnostics.
pub fn parse(input: &str) -> (T2tDoc, Vec<Diagnostic>) {
    let mut p = Parser::new(input);
    let (title, author, date) = p.try_parse_header();
    let blocks = p.parse();
    let doc = T2tDoc {
        blocks,
        title,
        author,
        date,
        span: Span::new(0, input.len()),
    };
    (doc, p.diagnostics)
}

/// Parse a txt2tags string — infallible convenience wrapper.
pub fn parse_str(input: &str) -> T2tDoc {
    parse(input).0
}

// ── Parser ────────────────────────────────────────────────────────────────────

pub(crate) struct Parser<'a> {
    pub(crate) lines: Vec<&'a str>,
    pub(crate) pos: usize,
    /// Byte offset of each line start in the original input.
    pub(crate) line_offsets: Vec<usize>,
    pub(crate) diagnostics: Vec<Diagnostic>,
}

impl<'a> Parser<'a> {
    pub(crate) fn new(input: &'a str) -> Self {
        let mut line_offsets = Vec::new();
        let mut offset = 0;
        let lines: Vec<&'a str> = input
            .split('\n')
            .inspect(|line| {
                line_offsets.push(offset);
                // +1 for the '\n' separator (last line may have no trailing newline)
                offset += line.len() + 1;
            })
            .collect();
        Self {
            lines,
            pos: 0,
            line_offsets,
            diagnostics: Vec::new(),
        }
    }

    fn line_start(&self, line_idx: usize) -> usize {
        self.line_offsets.get(line_idx).copied().unwrap_or(0)
    }

    fn line_end(&self, line_idx: usize) -> usize {
        let start = self.line_start(line_idx);
        let len = self.lines.get(line_idx).map(|l| l.len()).unwrap_or(0);
        start + len
    }

    /// Try to parse the txt2tags header (first 3 lines: title, author, date).
    /// The header is only recognized if the first line is non-empty and non-blank.
    /// All three lines must be present (an empty line represents an empty field).
    pub(crate) fn try_parse_header(&mut self) -> (Option<String>, Option<String>, Option<String>) {
        // Header requires at least 3 lines and the first must be non-empty/non-comment
        if self.lines.len() < 3 {
            return (None, None, None);
        }
        let first = self.lines[0].trim();
        // Header is only present if line 1 is non-empty and not a comment or block marker
        if first.is_empty()
            || first.starts_with('%')
            || first == "```"
            || first == "\"\"\""
            || first.starts_with("- ")
            || first.starts_with("+ ")
            || first.starts_with('|')
            || first.starts_with('\t')
            || first.starts_with(": ")
            || is_horizontal_rule(self.lines[0])
        {
            return (None, None, None);
        }
        // Check if line 1 looks like a heading (= ... = or + ... +)
        if self.try_parse_heading(self.lines[0]).is_some() {
            return (None, None, None);
        }

        let title = {
            let t = self.lines[0].trim();
            if t.is_empty() { None } else { Some(t.to_string()) }
        };
        let author = {
            let t = self.lines[1].trim();
            if t.is_empty() { None } else { Some(t.to_string()) }
        };
        let date = {
            let t = self.lines[2].trim();
            if t.is_empty() { None } else { Some(t.to_string()) }
        };

        // Only consume header if at least title is present
        if title.is_some() {
            self.pos = 3;
        }
        (title, author, date)
    }

    pub(crate) fn parse(&mut self) -> Vec<Block> {
        let mut blocks = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            // Skip empty lines
            if line.trim().is_empty() {
                self.pos += 1;
                continue;
            }

            // Comment (% at start of line)
            if line.starts_with('%') {
                self.pos += 1;
                continue;
            }

            // Verbatim block (```)
            if line.trim() == "```" {
                blocks.push(self.parse_verbatim_block());
                continue;
            }

            // Raw block (""")
            if line.trim() == "\"\"\"" {
                blocks.push(self.parse_raw_block());
                continue;
            }

            // Heading = Title = or + Title +
            if let Some(block) = self.try_parse_heading(line) {
                blocks.push(block);
                self.pos += 1;
                continue;
            }

            // Horizontal rule (20+ dashes, equals, or underscores)
            if is_horizontal_rule(line) {
                let span = Span::new(self.line_start(self.pos), self.line_end(self.pos));
                blocks.push(Block::HorizontalRule { span });
                self.pos += 1;
                continue;
            }

            // Quote (lines starting with TAB)
            if line.starts_with('\t') {
                blocks.push(self.parse_quote());
                continue;
            }

            // Unordered list (- item)
            if line.trim_start().starts_with("- ") {
                blocks.push(self.parse_list(false));
                continue;
            }

            // Ordered list (+ item)
            if line.trim_start().starts_with("+ ") {
                blocks.push(self.parse_list(true));
                continue;
            }

            // Table (| cell |)
            if line.trim_start().starts_with('|') {
                blocks.push(self.parse_table());
                continue;
            }

            // Definition list (: Term)
            if line.starts_with(": ") {
                blocks.push(self.parse_definition_list());
                continue;
            }

            // Regular paragraph
            blocks.push(self.parse_paragraph());
        }

        blocks
    }

    pub(crate) fn try_parse_heading(&self, line: &str) -> Option<Block> {
        let trimmed = line.trim();

        // Check for = or + delimited headings
        for (marker, numbered) in [('=', false), ('+', true)] {
            let level = trimmed.chars().take_while(|&c| c == marker).count();
            if level > 0 && level <= 5 {
                let end_marker_count =
                    trimmed.chars().rev().take_while(|&c| c == marker).count();
                if end_marker_count >= level {
                    // Extract content between markers
                    let content_start = level;
                    let content_end = trimmed.len() - end_marker_count;
                    if content_start < content_end {
                        let content = trimmed[content_start..content_end].trim();
                        // Check for label [label-name]
                        let (text, _label) = if let Some(bracket_pos) = content.rfind('[') {
                            if content.ends_with(']') {
                                (
                                    content[..bracket_pos].trim(),
                                    Some(&content[bracket_pos + 1..content.len() - 1]),
                                )
                            } else {
                                (content, None)
                            }
                        } else {
                            (content, None)
                        };

                        let span = Span::new(
                            self.line_start(self.pos),
                            self.line_end(self.pos),
                        );
                        let inlines = parse_inline(text, span.start);
                        return Some(Block::Heading {
                            level: level as u8,
                            numbered,
                            inlines,
                            span,
                        });
                    }
                }
            }
        }
        None
    }

    fn parse_verbatim_block(&mut self) -> Block {
        let block_start = self.line_start(self.pos);
        let mut content = String::new();
        self.pos += 1; // Skip opening ```

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            if line.trim() == "```" {
                self.pos += 1;
                break;
            }
            if !content.is_empty() {
                content.push('\n');
            }
            content.push_str(line);
            self.pos += 1;
        }

        let block_end = self.line_end(self.pos.saturating_sub(1));
        Block::CodeBlock {
            content,
            span: Span::new(block_start, block_end),
        }
    }

    fn parse_raw_block(&mut self) -> Block {
        let block_start = self.line_start(self.pos);
        let mut content = String::new();
        self.pos += 1; // Skip opening """

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            if line.trim() == "\"\"\"" {
                self.pos += 1;
                break;
            }
            if !content.is_empty() {
                content.push('\n');
            }
            content.push_str(line);
            self.pos += 1;
        }

        let block_end = self.line_end(self.pos.saturating_sub(1));
        Block::RawBlock {
            content,
            span: Span::new(block_start, block_end),
        }
    }

    fn parse_quote(&mut self) -> Block {
        let block_start = self.line_start(self.pos);
        let mut content = String::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            if !line.starts_with('\t') {
                break;
            }
            if !content.is_empty() {
                content.push(' ');
            }
            content.push_str(line[1..].trim());
            self.pos += 1;
        }

        let block_end = self.line_end(self.pos.saturating_sub(1));
        let span = Span::new(block_start, block_end);
        let inlines = parse_inline(&content, span.start);
        Block::Blockquote {
            children: vec![Block::Paragraph { inlines, span }],
            span,
        }
    }

    fn parse_list(&mut self, ordered: bool) -> Block {
        let block_start = self.line_start(self.pos);
        let mut items = Vec::new();
        let marker = if ordered { "+ " } else { "- " };

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            let trimmed = line.trim_start();

            if !trimmed.starts_with(marker) {
                break;
            }

            let item_start = self.line_start(self.pos);
            let content = &trimmed[2..];
            let inlines = parse_inline(content, item_start + (line.len() - trimmed.len()) + 2);
            let item_end = self.line_end(self.pos);
            let item_span = Span::new(item_start, item_end);
            let item = vec![Block::Paragraph {
                inlines,
                span: item_span,
            }];
            items.push(item);
            self.pos += 1;
        }

        let block_end = self.line_end(self.pos.saturating_sub(1));
        Block::List {
            ordered,
            items,
            span: Span::new(block_start, block_end),
        }
    }

    fn parse_table(&mut self) -> Block {
        let block_start = self.line_start(self.pos);
        let mut rows = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            let trimmed = line.trim();

            if !trimmed.starts_with('|') {
                break;
            }

            let row_start = self.line_start(self.pos);
            let is_header = trimmed.starts_with("||");
            let row_content = if is_header {
                &trimmed[2..]
            } else {
                &trimmed[1..]
            };

            let mut cells = Vec::new();
            for cell_text in row_content.split('|') {
                let cell_text = cell_text.trim();
                if cell_text.is_empty() && cells.is_empty() {
                    continue; // Skip empty leading cell
                }
                if cell_text.is_empty() {
                    continue; // Skip empty cells
                }
                let inlines = parse_inline(cell_text, row_start);
                cells.push(inlines);
            }

            if !cells.is_empty() {
                let row_end = self.line_end(self.pos);
                rows.push(crate::ast::TableRow {
                    cells,
                    is_header,
                    span: Span::new(row_start, row_end),
                });
            }
            self.pos += 1;
        }

        let block_end = self.line_end(self.pos.saturating_sub(1));
        Block::Table {
            rows,
            span: Span::new(block_start, block_end),
        }
    }

    fn parse_definition_list(&mut self) -> Block {
        let block_start = self.line_start(self.pos);
        let mut items: Vec<(Vec<Inline>, Vec<Block>)> = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            if !line.starts_with(": ") {
                break;
            }

            let term_text = &line[2..].trim();
            let term_start = self.line_start(self.pos);
            let term_inlines = parse_inline(term_text, term_start + 2);
            self.pos += 1;

            // Collect description lines (non-empty lines that don't start with `: `)
            let mut desc_text = String::new();
            while self.pos < self.lines.len() {
                let dline = self.lines[self.pos];
                if dline.trim().is_empty() || dline.starts_with(": ") {
                    break;
                }
                if !desc_text.is_empty() {
                    desc_text.push(' ');
                }
                desc_text.push_str(dline.trim());
                self.pos += 1;
            }

            let desc_blocks = if desc_text.is_empty() {
                Vec::new()
            } else {
                let desc_span = Span::new(term_start, self.line_end(self.pos.saturating_sub(1)));
                let desc_inlines = parse_inline(&desc_text, term_start);
                vec![Block::Paragraph {
                    inlines: desc_inlines,
                    span: desc_span,
                }]
            };

            items.push((term_inlines, desc_blocks));

            // Skip blank lines between items
            while self.pos < self.lines.len() && self.lines[self.pos].trim().is_empty() {
                self.pos += 1;
            }
        }

        let block_end = self.line_end(self.pos.saturating_sub(1));
        Block::DefinitionList {
            items,
            span: Span::new(block_start, block_end),
        }
    }

    fn parse_paragraph(&mut self) -> Block {
        let block_start = self.line_start(self.pos);
        let mut text = String::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            // End paragraph at empty line or block element
            if line.trim().is_empty()
                || line.starts_with('%')
                || line.trim() == "```"
                || line.trim() == "\"\"\""
                || self.try_parse_heading(line).is_some()
                || is_horizontal_rule(line)
                || line.starts_with('\t')
                || line.trim_start().starts_with("- ")
                || line.trim_start().starts_with("+ ")
                || line.trim_start().starts_with('|')
                || line.starts_with(": ")
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
        let span = Span::new(block_start, block_end);
        let inlines = parse_inline(&text, span.start);
        Block::Paragraph { inlines, span }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

pub(crate) fn is_horizontal_rule(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.len() >= 20 {
        let first_char = trimmed.chars().next().unwrap_or(' ');
        if first_char == '-' || first_char == '=' || first_char == '_' {
            return trimmed.chars().all(|c| c == first_char);
        }
    }
    false
}

pub(crate) fn parse_inline(text: &str, _base_offset: usize) -> Vec<Inline> {
    let mut nodes = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Bold **text**
        if chars[i] == '*'
            && i + 1 < chars.len()
            && chars[i + 1] == '*'
            && let Some((end, content)) = find_double_closing(&chars, i + 2, '*')
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            let inner = parse_inline(&content, _base_offset);
            nodes.push(Inline::Bold(inner, Span::NONE));
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
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            let inner = parse_inline(&content, _base_offset);
            nodes.push(Inline::Italic(inner, Span::NONE));
            i = end + 2;
            continue;
        }

        // Underline __text__
        if chars[i] == '_'
            && i + 1 < chars.len()
            && chars[i + 1] == '_'
            && let Some((end, content)) = find_double_closing(&chars, i + 2, '_')
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            let inner = parse_inline(&content, _base_offset);
            nodes.push(Inline::Underline(inner, Span::NONE));
            i = end + 2;
            continue;
        }

        // Strikethrough --text--
        if chars[i] == '-'
            && i + 1 < chars.len()
            && chars[i + 1] == '-'
            && let Some((end, content)) = find_double_closing(&chars, i + 2, '-')
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            let inner = parse_inline(&content, _base_offset);
            nodes.push(Inline::Strikethrough(inner, Span::NONE));
            i = end + 2;
            continue;
        }

        // Verbatim ""text""
        if chars[i] == '"'
            && i + 1 < chars.len()
            && chars[i + 1] == '"'
            && let Some((end, content)) = find_double_closing(&chars, i + 2, '"')
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            nodes.push(Inline::Verbatim(content, Span::NONE));
            i = end + 2;
            continue;
        }

        // Tagged ''text''
        if chars[i] == '\''
            && i + 1 < chars.len()
            && chars[i + 1] == '\''
            && let Some((end, content)) = find_double_closing(&chars, i + 2, '\'')
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            nodes.push(Inline::Tagged(content, Span::NONE));
            i = end + 2;
            continue;
        }

        // Monospace ``text``
        if chars[i] == '`'
            && i + 1 < chars.len()
            && chars[i + 1] == '`'
            && let Some((end, content)) = find_double_closing(&chars, i + 2, '`')
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            nodes.push(Inline::Code(content, Span::NONE));
            i = end + 2;
            continue;
        }

        // Link [label url] or image [filename.ext]
        if chars[i] == '['
            && let Some((end, label, url)) = parse_link_or_image(&chars, i)
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            if is_image_url(&url) {
                nodes.push(Inline::Image {
                    url,
                    span: Span::NONE,
                });
            } else {
                let text_node = Inline::Text(label, Span::NONE);
                nodes.push(Inline::Link {
                    url,
                    children: vec![text_node],
                    span: Span::NONE,
                });
            }
            i = end;
            continue;
        }

        // Auto-detect URLs
        if (chars[i] == 'h' || chars[i] == 'H')
            && let Some((end, url)) = try_parse_url(&chars, i)
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            let text_node = Inline::Text(url.clone(), Span::NONE);
            nodes.push(Inline::Link {
                url,
                children: vec![text_node],
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

fn parse_link_or_image(chars: &[char], start: usize) -> Option<(usize, String, String)> {
    // [label url] or [filename.ext]
    if start >= chars.len() || chars[start] != '[' {
        return None;
    }

    let mut i = start + 1;
    let mut content = String::new();

    while i < chars.len() && chars[i] != ']' {
        content.push(chars[i]);
        i += 1;
    }

    if i >= chars.len() {
        return None;
    }

    let content = content.trim();

    // Check if it's [label url] format
    if let Some(space_pos) = content.rfind(' ') {
        let label = content[..space_pos].trim();
        let url = content[space_pos + 1..].trim();
        // Ensure URL looks like a URL
        if url.contains('.') || url.starts_with('#') || url.starts_with("http") {
            return Some((i + 1, label.to_string(), url.to_string()));
        }
    }

    // Single item - could be URL or image
    if content.contains('.') {
        return Some((i + 1, content.to_string(), content.to_string()));
    }

    None
}

fn is_image_url(url: &str) -> bool {
    let lower = url.to_lowercase();
    lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".gif")
        || lower.ends_with(".svg")
        || lower.ends_with(".webp")
}

fn try_parse_url(chars: &[char], start: usize) -> Option<(usize, String)> {
    let rest: String = chars[start..].iter().collect();
    if rest.starts_with("http://")
        || rest.starts_with("https://")
        || rest.starts_with("HTTP://")
        || rest.starts_with("HTTPS://")
    {
        let mut end = start;
        while end < chars.len() && !chars[end].is_whitespace() {
            end += 1;
        }
        let url: String = chars[start..end].iter().collect();
        return Some((end, url));
    }
    None
}
