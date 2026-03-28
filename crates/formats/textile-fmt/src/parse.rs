//! Textile parser — infallible, returns (TextileDoc, Vec<Diagnostic>).

use crate::ast::{Block, BlockAttrs, Diagnostic, Inline, Span, TableCell, TableRow, TextileDoc};

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
            if line.starts_with("bc") && has_block_prefix_after(line, 2) {
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

            // Notextile block: notextile. content
            if let Some(rest) = line.strip_prefix("notextile.") {
                let ls = self.line_start(self.pos);
                let le = self.line_end(self.pos);
                let content = rest.trim().to_string();
                self.pos += 1;
                nodes.push(Block::Raw { content, span: Span::new(ls, le) });
                continue;
            }

            // Heading h1. to h6.
            if let Some(node) = self.try_parse_heading(line) {
                nodes.push(node);
                self.pos += 1;
                continue;
            }

            // Table: starts with '|' or with row attributes prefix (e.g. `{style}. |`)
            if line.trim_start().starts_with('|') || is_table_row_with_attrs(line.trim_start()) {
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

            // Horizontal rule ---
            if line.trim() == "---" {
                let ls = self.line_start(self.pos);
                let le = self.line_end(self.pos);
                self.pos += 1;
                nodes.push(Block::HorizontalRule { span: Span::new(ls, le) });
                continue;
            }

            // Footnote definition fn1. fn2. etc.
            if let Some(node) = self.try_parse_footnote_def(line) {
                nodes.push(node);
                self.pos += 1;
                continue;
            }

            // Definition list — starts with ; (term) or : (definition)
            if line.starts_with(';') || line.starts_with(':') {
                nodes.push(self.parse_definition_list());
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
            let bare = format!("h{}", level);
            if !line.starts_with(&bare) {
                continue;
            }
            let rest = &line[bare.len()..];
            let (attrs, dot_offset) = parse_block_attrs(rest)?;
            let content = rest[dot_offset..].trim();
            let inline_nodes = parse_inline(content, line_start + bare.len() + dot_offset);
            return Some(Block::Heading {
                level,
                inlines: inline_nodes,
                attrs,
                span: Span::new(line_start, self.line_end(self.pos)),
            });
        }
        None
    }

    fn parse_code_block(&mut self) -> Block {
        let block_start = self.line_start(self.pos);
        let first_line = self.lines[self.pos];

        // Parse optional language: bc(lang). or bc(lang)..
        let (language, content_start, extended) = parse_code_block_prefix(first_line);

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
            language,
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
            language: None,
            span: Span::new(block_start, block_end),
        }
    }

    fn parse_blockquote(&mut self) -> Block {
        let block_start = self.line_start(self.pos);
        let first_line = self.lines[self.pos];
        let extended = first_line.starts_with("bq..");

        // `bq. content` → content at offset 3; `bq.. content` → offset 4.
        let content_start = if extended { 4 } else { 3 };
        let attrs = BlockAttrs::default();

        let mut blocks = Vec::new();

        let first_content = first_line[content_start..].trim();
        if !first_content.is_empty() {
            let ls = self.line_start(self.pos);
            let inline_nodes = parse_inline(first_content, ls + content_start);
            blocks.push(Block::Paragraph {
                inlines: inline_nodes,
                align: None,
                attrs: BlockAttrs::default(),
                span: Span::new(ls + content_start, self.line_end(self.pos)),
            });
        }
        self.pos += 1;

        if extended {
            // Collect subsequent paragraphs separated by blank lines, until an explicit
            // new block type is encountered or end of input.
            let mut para_lines: Vec<&str> = Vec::new();
            while self.pos < self.lines.len() {
                let line = self.lines[self.pos];
                if line.trim().is_empty() {
                    // Flush accumulated lines as a paragraph
                    if !para_lines.is_empty() {
                        let text = para_lines.join("\n");
                        let ls = self.line_start(self.pos - para_lines.len());
                        let inline_nodes = parse_inline(&text, ls);
                        blocks.push(Block::Paragraph {
                            inlines: inline_nodes,
                            align: None,
                            attrs: BlockAttrs::default(),
                            span: Span::new(ls, self.line_end(self.pos.saturating_sub(1))),
                        });
                        para_lines.clear();
                    }
                    self.pos += 1;
                    continue;
                }
                // Stop at an explicit block type change
                if is_explicit_block_start(line) {
                    break;
                }
                para_lines.push(line.trim());
                self.pos += 1;
            }
            // Flush any remaining lines
            if !para_lines.is_empty() {
                let text = para_lines.join("\n");
                let ls = self.line_start(self.pos - para_lines.len());
                let inline_nodes = parse_inline(&text, ls);
                blocks.push(Block::Paragraph {
                    inlines: inline_nodes,
                    align: None,
                    attrs: BlockAttrs::default(),
                    span: Span::new(ls, self.line_end(self.pos.saturating_sub(1))),
                });
            }
        }

        let block_end = self.line_end(self.pos.saturating_sub(1));
        Block::Blockquote {
            blocks,
            attrs,
            span: Span::new(block_start, block_end),
        }
    }

    fn parse_table(&mut self) -> Block {
        let block_start = self.line_start(self.pos);
        let mut rows = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            let t = line.trim_start();
            if !t.starts_with('|') && !is_table_row_with_attrs(t) {
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

        // Detect row-level attributes: `{style}. |cell|` or `(class). |cell|`
        let (row_attrs, cell_str) = parse_row_attrs(trimmed);

        let inner = cell_str.trim_start_matches('|').trim_end_matches('|');
        let parts: Vec<&str> = inner.split('|').collect();

        let mut offset = line_start + (trimmed.len() - inner.len());
        for part in parts {
            let part_trimmed = part.trim();
            // Detect header marker: `_.` (plain header) or combined `_<.`, `_>.`, `_=.`, `_<>.`
            let (is_header, after_header) = if let Some(rest) = part_trimmed.strip_prefix('_') {
                if let Some(after_dot) = rest.strip_prefix('.') {
                    (true, after_dot.trim_start())
                } else if rest.starts_with("<>.") || rest.starts_with(">.") || rest.starts_with("<.") || rest.starts_with("=.") {
                    (true, rest) // leave alignment chars for parse_cell_align below
                } else {
                    (false, part_trimmed)
                }
            } else {
                (false, part_trimmed)
            };
            // Parse cell alignment: <. >. =. <>.
            let (cell_content, align) = parse_cell_align(after_header);

            let cell_start = offset;
            let cell_end = cell_start + part.len();
            let inline_nodes = parse_inline(cell_content, cell_start);
            cells.push(TableCell {
                is_header,
                align,
                inlines: inline_nodes,
                span: Span::new(cell_start, cell_end),
            });
            offset += part.len() + 1; // +1 for the '|' separator
        }

        TableRow {
            attrs: row_attrs,
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
                let mut content = trimmed[marker_count + 1..].trim().to_string();
                let line_start = self.line_start(self.pos);

                self.pos += 1;

                // Collect continuation lines: non-blank lines that are not list items
                // or explicit block starts belong to this item.
                while self.pos < self.lines.len() {
                    let cont = self.lines[self.pos].trim_start();
                    if cont.is_empty() || is_explicit_block_start(cont) {
                        break;
                    }
                    let cont_marker_count = cont.chars().take_while(|&c| c == marker).count();
                    let other_marker = if ordered { '*' } else { '#' };
                    let cont_other_count =
                        cont.chars().take_while(|&c| c == other_marker).count();
                    if cont_marker_count > 0 || cont_other_count > 0 {
                        break;
                    }
                    content.push(' ');
                    content.push_str(cont);
                    self.pos += 1;
                }

                let inline_nodes = parse_inline(&content, line_start + marker_count + 1);
                let para = Block::Paragraph {
                    inlines: inline_nodes,
                    align: None,
                    attrs: BlockAttrs::default(),
                    span: Span::new(line_start, self.line_end(self.pos.saturating_sub(1))),
                };
                let mut item_children = vec![para];

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

        // Parse block attributes: p(class){style}[lang]<. or p<>. or p.
        let (first_content, attrs) = parse_paragraph_prefix(first_line);
        let align = attrs.align.clone();
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
                || line.starts_with("bc")
                || line.starts_with("bq.")
                || line.starts_with("pre.")
                || line.starts_with("p.")
                || line.starts_with("p<")
                || line.starts_with("p>")
                || line.starts_with("p=")
                || line.starts_with("notextile.")
                || line.trim_start().starts_with('|')
                || line.trim_start().starts_with("* ")
                || line.trim_start().starts_with("# ")
            {
                break;
            }

            // Use '\n' as separator so the inline parser can emit LineBreak nodes.
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(line.trim());
            self.pos += 1;
        }

        let block_end = self.line_end(self.pos.saturating_sub(1));
        let inline_nodes = parse_inline(&text, block_start);
        Block::Paragraph {
            inlines: inline_nodes,
            align,
            attrs,
            span: Span::new(block_start, block_end),
        }
    }

    /// Try to parse a footnote definition line: `fn1. content text`.
    fn try_parse_footnote_def(&self, line: &str) -> Option<Block> {
        // Match "fn" followed by digits and "."
        if !line.starts_with("fn") {
            return None;
        }
        let rest = &line[2..];
        let dot_pos = rest.find('.')?;
        let label = &rest[..dot_pos];
        if label.is_empty() || !label.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let content = rest[dot_pos + 1..].trim();
        let line_start = self.line_start(self.pos);
        let inlines = parse_inline(content, line_start + 2 + dot_pos + 1);
        Some(Block::FootnoteDef {
            label: label.to_string(),
            inlines,
            span: Span::new(line_start, self.line_end(self.pos)),
        })
    }

    /// Parse a definition list: consecutive lines starting with `;` (term) or `:` (definition).
    fn parse_definition_list(&mut self) -> Block {
        let block_start = self.line_start(self.pos);
        let mut items: Vec<(Vec<Inline>, Vec<Inline>)> = Vec::new();
        let mut current_term: Option<Vec<Inline>> = None;

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            if let Some(term_rest) = line.strip_prefix(';') {
                let term_text = term_rest.trim();
                let ls = self.line_start(self.pos);
                current_term = Some(parse_inline(term_text, ls + 1));
                self.pos += 1;
            } else if let Some(def_rest) = line.strip_prefix(':') {
                let def_text = def_rest.trim();
                let ls = self.line_start(self.pos);
                let def_inlines = parse_inline(def_text, ls + 1);
                let term_inlines = current_term.take().unwrap_or_default();
                items.push((term_inlines, def_inlines));
                self.pos += 1;
            } else {
                break;
            }
        }

        // Any orphan term with no definition becomes an empty-def item
        if let Some(term) = current_term {
            items.push((term, vec![]));
        }

        let block_end = self.line_end(self.pos.saturating_sub(1));
        Block::DefinitionList {
            items,
            span: Span::new(block_start, block_end),
        }
    }
}

// ── Block prefix helpers ───────────────────────────────────────────────────────

/// Check if after "bc" at position `offset` in `line` there is a valid prefix
/// (either `(lang).`, `(lang)..`, `..`, or `.`).
fn has_block_prefix_after(line: &str, offset: usize) -> bool {
    let rest = &line[offset..];
    rest.starts_with('.') || rest.starts_with("..") || rest.starts_with('(')
}

/// Parse the code block prefix, returning (language, content_byte_offset, is_extended).
fn parse_code_block_prefix(line: &str) -> (Option<String>, usize, bool) {
    // bc(lang).. or bc(lang).
    if let Some(inner) = line.strip_prefix("bc(")
        && let Some(rel_paren) = inner.find(')')
    {
        let paren_end = 3 + rel_paren; // index of ')' in original line
        let lang = inner[..rel_paren].to_string();
        let after = &line[paren_end + 1..]; // after ')'
        if after.starts_with("..") {
            return (Some(lang), paren_end + 3, true); // skip ").."+space
        } else if after.starts_with('.') {
            return (Some(lang), paren_end + 2, false); // skip "."
        }
    }
    // Standard bc.. or bc.
    if line.starts_with("bc..") {
        (None, 4, true)
    } else {
        (None, 3, false)
    }
}

/// Parse a paragraph prefix, returning (trimmed_content, BlockAttrs).
fn parse_paragraph_prefix(line: &str) -> (&str, BlockAttrs) {
    // Strip "p" prefix if present, then parse block attrs
    if let Some(rest) = line.strip_prefix('p') {
        // Must be followed by attrs+'.', otherwise it's not a paragraph prefix
        if let Some((attrs, dot_offset)) = parse_block_attrs(rest) {
            let content = rest[dot_offset..].trim();
            return (content, attrs);
        }
    }
    (line.trim(), BlockAttrs::default())
}

/// Parse block-level attributes from the string after the block type indicator.
/// Returns `Some((attrs, bytes_consumed_including_dot))` on success.
/// The format is: `[(<class>)][{<style>}][[<lang>]][<align>][<indent>].`
///
/// Examples: `(myclass).`, `{color:red}.`, `[en].`, `<.`, `>.`, `=.`, `<>.`, `.`
fn parse_block_attrs(s: &str) -> Option<(BlockAttrs, usize)> {
    let mut attrs = BlockAttrs::default();
    let bytes = s.as_bytes();
    let mut i = 0;

    // Consume attribute tokens in any order until we hit '.'
    loop {
        if i >= bytes.len() {
            return None;
        }
        match bytes[i] {
            b'(' => {
                // Could be (class#id) or indent (
                // If next char is also '(' or '.', treat as indent
                let inner_start = i + 1;
                // Check if this is indentation: a bare '(' not followed by class chars
                // Simple heuristic: if immediately '.' or ')' without text, it's indent
                if let Some(close) = s[inner_start..].find(')') {
                    let class_id = &s[inner_start..inner_start + close];
                    // class_id could be "name" or "name#id" or "#id"
                    if class_id.is_empty() {
                        // empty () = indent
                        attrs.indent_left += 1;
                    } else if let Some(hash) = class_id.find('#') {
                        attrs.class = Some(class_id[..hash].to_string()).filter(|s| !s.is_empty());
                        attrs.id = Some(class_id[hash + 1..].to_string()).filter(|s| !s.is_empty());
                    } else {
                        attrs.class = Some(class_id.to_string());
                    }
                    i = inner_start + close + 1;
                } else {
                    // No closing ), treat as indent
                    attrs.indent_left += 1;
                    i += 1;
                }
            }
            b')' => {
                attrs.indent_right += 1;
                i += 1;
            }
            b'{' => {
                if let Some(close) = s[i + 1..].find('}') {
                    attrs.style = Some(s[i + 1..i + 1 + close].to_string());
                    i = i + 1 + close + 1;
                } else {
                    return None;
                }
            }
            b'[' => {
                if let Some(close) = s[i + 1..].find(']') {
                    attrs.lang = Some(s[i + 1..i + 1 + close].to_string());
                    i = i + 1 + close + 1;
                } else {
                    return None;
                }
            }
            b'<' if i + 1 < bytes.len() && bytes[i + 1] == b'>' => {
                // <> = justify
                attrs.align = Some("justify".to_string());
                i += 2;
            }
            b'<' => {
                attrs.align = Some("left".to_string());
                i += 1;
            }
            b'>' => {
                attrs.align = Some("right".to_string());
                i += 1;
            }
            b'=' => {
                attrs.align = Some("center".to_string());
                i += 1;
            }
            b'.' => {
                // End of attributes
                return Some((attrs, i + 1));
            }
            _ => return None,
        }
    }
}

/// Parse cell-level alignment prefix (`<.`, `>.`, `=.`, `<>.`).
fn parse_cell_align(content: &str) -> (&str, Option<String>) {
    if let Some(rest) = content.strip_prefix("<>.") {
        return (rest.trim(), Some("justify".to_string()));
    }
    if let Some(rest) = content.strip_prefix("<.") {
        return (rest.trim(), Some("left".to_string()));
    }
    if let Some(rest) = content.strip_prefix(">.") {
        return (rest.trim(), Some("right".to_string()));
    }
    if let Some(rest) = content.strip_prefix("=.") {
        return (rest.trim(), Some("center".to_string()));
    }
    (content, None)
}

/// Parse inline span attributes (`{style}(class)[lang]`) from a string prefix.
/// Unlike `parse_block_attrs`, no trailing `.` is required.
/// Returns `(attrs, bytes_consumed)`.
fn parse_inline_span_attrs(s: &str) -> (BlockAttrs, usize) {
    let mut attrs = BlockAttrs::default();
    let bytes = s.as_bytes();
    let mut i = 0;

    loop {
        if i >= bytes.len() {
            break;
        }
        match bytes[i] {
            b'{' => {
                if let Some(close) = s[i + 1..].find('}') {
                    attrs.style = Some(s[i + 1..i + 1 + close].to_string());
                    i += close + 2;
                } else {
                    break;
                }
            }
            b'(' => {
                if let Some(close) = s[i + 1..].find(')') {
                    let inner = &s[i + 1..i + 1 + close];
                    if let Some(hash) = inner.find('#') {
                        attrs.class =
                            Some(inner[..hash].to_string()).filter(|s| !s.is_empty());
                        attrs.id =
                            Some(inner[hash + 1..].to_string()).filter(|s| !s.is_empty());
                    } else {
                        attrs.class = Some(inner.to_string()).filter(|s| !s.is_empty());
                    }
                    i += close + 2;
                } else {
                    break;
                }
            }
            b'[' => {
                if let Some(close) = s[i + 1..].find(']') {
                    attrs.lang = Some(s[i + 1..i + 1 + close].to_string());
                    i += close + 2;
                } else {
                    break;
                }
            }
            _ => break,
        }
    }

    (attrs, i)
}

/// Detect and strip row-level attributes from a table row line.
/// Format: `{style}. |cells|` or `(class). |cells|` or `[lang]. |cells|`
/// Returns `(attrs, remaining_line_starting_with_|)`.
fn parse_row_attrs(line: &str) -> (BlockAttrs, &str) {
    let first = line.as_bytes().first().copied();
    if !matches!(first, Some(b'{') | Some(b'(') | Some(b'[')) {
        return (BlockAttrs::default(), line);
    }
    let (attrs, consumed) = parse_inline_span_attrs(line);
    let rest = &line[consumed..];
    // Must be followed by ". " or ".|" (dot then space or pipe)
    if let Some(after_dot) = rest.strip_prefix(". ") {
        return (attrs, after_dot.trim_start());
    }
    if let Some(after_dot) = rest.strip_prefix(".|") {
        return (attrs, after_dot.trim_start_matches('|').trim_end_matches('|'));
    }
    (BlockAttrs::default(), line)
}

/// Returns true if `line` looks like a table row with row-level attributes prefix.
/// Pattern: `{...}. |` or `(...). |` or `[...]. |`
fn is_table_row_with_attrs(line: &str) -> bool {
    let first = line.as_bytes().first().copied();
    if !matches!(first, Some(b'{') | Some(b'(') | Some(b'[')) {
        return false;
    }
    let (_, consumed) = parse_inline_span_attrs(line);
    if consumed == 0 {
        return false;
    }
    let rest = &line[consumed..];
    (rest.starts_with(". |") || rest.starts_with(".|")) && rest.contains('|')
}

/// Returns true if `line` starts an explicit new block type (used to stop extended blocks).
fn is_explicit_block_start(line: &str) -> bool {
    let t = line.trim_start();
    // heading h1–h9
    if let Some(h_rest) = t.strip_prefix('h') {
        let rest = h_rest.trim_start_matches(|c: char| c.is_ascii_digit());
        if rest.starts_with('.') {
            return true;
        }
    }
    // common block prefixes
    for prefix in &["p.", "p(", "p{", "p[", "bq.", "bc.", "pre.", "notextile.", "fn", "---"] {
        if t.starts_with(prefix) {
            return true;
        }
    }
    false
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
        // Hard line break: '\n' within paragraph text
        if chars[i] == '\n' {
            if !current.is_empty() {
                nodes.push(Inline::Text(
                    current.clone(),
                    Span::new(text_start, char_abs(i)),
                ));
                current.clear();
            }
            let lb_span = Span::new(char_abs(i), char_abs(i + 1));
            nodes.push(Inline::LineBreak(lb_span));
            i += 1;
            text_start = char_abs(i);
            continue;
        }

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

        // Notextile inline ==raw==
        if chars[i] == '=' && i + 1 < chars.len() && chars[i + 1] == '=' {
            if !current.is_empty() {
                nodes.push(Inline::Text(
                    current.clone(),
                    Span::new(text_start, char_abs(i)),
                ));
                current.clear();
            }
            let raw_start = char_abs(i);
            i += 2; // skip ==
            let mut raw_content = String::new();
            while i + 1 < chars.len() && !(chars[i] == '=' && chars[i + 1] == '=') {
                raw_content.push(chars[i]);
                i += 1;
            }
            if i + 1 < chars.len() && chars[i] == '=' && chars[i + 1] == '=' {
                i += 2; // skip closing ==
            }
            let raw_end = char_abs(i);
            nodes.push(Inline::Raw(raw_content, Span::new(raw_start, raw_end)));
            text_start = char_abs(i);
            continue;
        }

        // Citation ??text??
        if chars[i] == '?' && i + 1 < chars.len() && chars[i + 1] == '?' {
            if !current.is_empty() {
                nodes.push(Inline::Text(
                    current.clone(),
                    Span::new(text_start, char_abs(i)),
                ));
                current.clear();
            }
            let cite_start = char_abs(i);
            i += 2; // skip ??
            let mut cite_text = String::new();
            while i + 1 < chars.len() && !(chars[i] == '?' && chars[i + 1] == '?') {
                cite_text.push(chars[i]);
                i += 1;
            }
            // consume closing ??
            if i + 1 < chars.len() && chars[i] == '?' && chars[i + 1] == '?' {
                i += 2;
            }
            let cite_end = char_abs(i);
            let cite_children = parse_inline(&cite_text, cite_start + 2);
            nodes.push(Inline::Citation(cite_children, Span::new(cite_start, cite_end)));
            text_start = char_abs(i);
            continue;
        }

        // Try to parse formatting markers (includes % for GenericSpan)
        if let Some((new_i, node)) = try_parse_formatting(&chars, i, &mut current, &mut nodes, &char_offsets, base_offset) {
            text_start = char_abs(new_i);
            i = new_i;
            nodes.push(node);
            continue;
        }

        // Link "text":url or "text(title)":url
        if chars[i] == '"'
            && let Some((link_end, link_text, title, url)) = parse_textile_link(&chars, i)
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
            // Parse the link text for inline formatting
            let link_children = parse_inline(&link_text, link_start + 1);
            nodes.push(Inline::Link {
                url,
                title,
                children: link_children,
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

        // Footnote reference [1], [2], etc.
        if chars[i] == '['
            && let Some((ref_end, label)) = parse_footnote_ref(&chars, i)
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(
                    current.clone(),
                    Span::new(text_start, char_abs(i)),
                ));
                current.clear();
            }
            let ref_span = Span::new(char_abs(i), char_abs(ref_end));
            nodes.push(Inline::FootnoteRef { label, span: ref_span });
            i = ref_end;
            text_start = char_abs(i);
            continue;
        }

        // Acronym ABC(title): look back in `current` for trailing uppercase sequence
        if chars[i] == '('
            && !current.is_empty()
            && let Some((abbr_byte_start, abbr_text, title, end_pos)) =
                try_parse_acronym(&current, &chars, i, &char_offsets, base_offset)
        {
            // Emit accumulated text before the abbreviation
            let pre_text = current[..abbr_byte_start].to_string();
            if !pre_text.is_empty() {
                let pre_end = text_start + pre_text.len();
                nodes.push(Inline::Text(pre_text, Span::new(text_start, pre_end)));
            }
            current.clear();
            // Approximate span: from where abbr starts to end of title+)
            let abbr_abs_start = char_abs(i).saturating_sub(abbr_text.len());
            let abbr_abs_end = char_abs(end_pos);
            nodes.push(Inline::Acronym {
                text: abbr_text,
                title,
                span: Span::new(abbr_abs_start, abbr_abs_end),
            });
            i = end_pos;
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

/// Try to parse an acronym: if `current` ends with ≥2 uppercase letters and `chars[i] == '('`,
/// attempt to read until `)`. Returns `(abbr_byte_start_in_current, abbr_text, title, end_char_pos)`.
fn try_parse_acronym(
    current: &str,
    chars: &[char],
    i: usize,
    _char_offsets: &[usize],
    _base_offset: usize,
) -> Option<(usize, String, String, usize)> {
    // Find trailing uppercase sequence in current
    let bytes = current.as_bytes();
    let mut abbr_byte_start = bytes.len();
    // Walk backwards through the string
    let current_chars: Vec<char> = current.chars().collect();
    let mut ci = current_chars.len();
    while ci > 0 {
        let c = current_chars[ci - 1];
        if c.is_uppercase() {
            ci -= 1;
            abbr_byte_start -= c.len_utf8();
        } else {
            break;
        }
    }
    let abbr = &current[abbr_byte_start..];
    if abbr.chars().count() < 2 {
        return None;
    }

    // Parse the title content: (title text)
    let mut j = i + 1;
    let mut title = String::new();
    while j < chars.len() && chars[j] != ')' {
        title.push(chars[j]);
        j += 1;
    }
    if j >= chars.len() || chars[j] != ')' || title.is_empty() {
        return None;
    }

    Some((abbr_byte_start, abbr.to_string(), title, j + 1))
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

    // (opening_marker, doubled_skip_char, check_prev_alphanumeric)
    // doubled_skip_char: if non-space and next char equals it, skip this position
    let markers: &[(char, char, bool)] = &[
        ('*', '*', true),
        ('_', '_', true),
        ('-', '-', true),
        ('+', '+', true),
        ('^', ' ', false),
        ('~', ' ', false),
        ('%', ' ', false),  // GenericSpan
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
                '%' => {
                    // Strip inline span attrs from the start of content
                    let (span_attrs, attr_len) = parse_inline_span_attrs(&content);
                    let children = if attr_len > 0 {
                        parse_inline(&content[attr_len..], fmt_start + 1 + attr_len)
                    } else {
                        inner
                    };
                    Inline::GenericSpan {
                        attrs: span_attrs,
                        children,
                        span: Span::new(fmt_start, fmt_end),
                    }
                }
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

/// Parse a link: `"text":url` or `"text(title)":url`.
/// Returns (end_pos, text, title, url).
fn parse_textile_link(chars: &[char], start: usize) -> Option<(usize, String, Option<String>, String)> {
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

    // Extract optional title from link text: "text(title)" → text="text", title="title"
    let (clean_text, title) = extract_link_title(&link_text);

    Some((i, clean_text, title, url))
}

/// Extract optional title from Textile link text: `text(title)` → ("text", Some("title")).
fn extract_link_title(text: &str) -> (String, Option<String>) {
    if text.ends_with(')')
        && let Some(paren_start) = text.rfind('(')
    {
        let title = text[paren_start + 1..text.len() - 1].to_string();
        let clean = text[..paren_start].trim_end().to_string();
        return (clean, Some(title));
    }
    (text.to_string(), None)
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

/// Try to parse a footnote reference `[digits]`. Returns (end_pos, label).
fn parse_footnote_ref(chars: &[char], start: usize) -> Option<(usize, String)> {
    if chars[start] != '[' {
        return None;
    }
    let mut i = start + 1;
    let mut label = String::new();
    while i < chars.len() && chars[i].is_ascii_digit() {
        label.push(chars[i]);
        i += 1;
    }
    if label.is_empty() || i >= chars.len() || chars[i] != ']' {
        return None;
    }
    Some((i + 1, label))
}
