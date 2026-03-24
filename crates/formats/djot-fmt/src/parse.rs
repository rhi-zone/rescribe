//! Block and inline parser for Djot.

#![allow(
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::if_same_then_else,
    clippy::manual_pattern_char_comparison,
    clippy::manual_repeat_n,
    clippy::manual_strip,
    clippy::manual_str_repeat,
    clippy::needless_return,
    clippy::type_complexity
)]

use crate::ast::*;
use crate::events::{collect_block_events, Event};
use std::collections::VecDeque;

/// Parse a Djot string into a `DjotDoc` and diagnostics. Infallible.
pub fn parse(input: &str) -> (DjotDoc, Vec<Diagnostic>) {
    crate::events::collect_doc_from_iter(input)
}

/// Phase tracker for `EventIter`'s `Iterator` implementation.
#[derive(PartialEq)]
pub(crate) enum Phase {
    /// Parsing top-level blocks.
    Blocks,
    /// Emitting footnote-def events (index into parser.footnote_defs).
    Footnotes(usize),
    /// All done.
    Done,
}

pub struct EventIter<'a> {
    input: &'a str,
    lines: Vec<&'a str>,
    /// Byte offset of the start of each line.
    line_offsets: Vec<usize>,
    pos: usize, // current line index
    pub(crate) diagnostics: Vec<Diagnostic>,
    pub(crate) link_defs: Vec<LinkDef>,
    pub(crate) footnote_defs: Vec<FootnoteDef>,
    /// Pending block attribute from `{...}` line before a block.
    pending_attr: Option<Attr>,
    // ── Iterator state ────────────────────────────────────────────────────
    event_buf: VecDeque<Event<'static>>,
    phase: Phase,
}

impl<'a> EventIter<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut lines = Vec::new();
        let mut offsets = Vec::new();
        let mut offset = 0;
        for line in input.split('\n') {
            offsets.push(offset);
            lines.push(line);
            offset += line.len() + 1; // +1 for the '\n'
        }
        // Remove trailing empty line produced by a terminal '\n' — it's not a real blank line.
        if lines.last().map(|l| l.is_empty()).unwrap_or(false) {
            lines.pop();
            offsets.pop();
        }
        let mut parser = EventIter {
            input,
            lines,
            line_offsets: offsets,
            pos: 0,
            diagnostics: Vec::new(),
            link_defs: Vec::new(),
            footnote_defs: Vec::new(),
            pending_attr: None,
            event_buf: VecDeque::new(),
            phase: Phase::Blocks,
        };
        parser.pre_scan();
        parser
    }

    fn line_start(&self, idx: usize) -> usize {
        self.line_offsets.get(idx).copied().unwrap_or(self.input.len())
    }

    fn current_line(&self) -> Option<&'a str> {
        self.lines.get(self.pos).copied()
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    pub(crate) fn at_end(&self) -> bool {
        self.pos >= self.lines.len()
    }

    /// Skip blank lines; return true if any were skipped.
    fn skip_blank_lines(&mut self) -> bool {
        let start = self.pos;
        while let Some(line) = self.current_line() {
            if line.trim().is_empty() {
                self.advance();
            } else {
                break;
            }
        }
        self.pos > start
    }

    /// Pre-scan to collect link definitions and footnote definitions.
    fn pre_scan(&mut self) {
        let n = self.lines.len();
        let mut i = 0;
        while i < n {
            let line = self.lines[i];
            let trimmed = line.trim_start();

            // Reference link definition: `[label]: url` (not footnote)
            if trimmed.starts_with('[') && !trimmed.starts_with("[^") {
                if let Some(ld) = parse_link_def(trimmed) {
                    self.link_defs.push(ld);
                    i += 1;
                    continue;
                }
            }

            // Footnote definition: `[^label]: content`
            if trimmed.starts_with("[^") {
                if let Some((label, first_content)) = parse_footnote_def_start(trimmed) {
                    let start_offset = self.line_offsets[i];
                    i += 1;
                    // Collect continuation lines (indented by at least 1 space or tab)
                    let mut content_lines = Vec::new();
                    if !first_content.is_empty() {
                        content_lines.push(first_content.to_string());
                    }
                    while i < n {
                        let next = self.lines[i];
                        if next.starts_with(' ') || next.starts_with('\t') {
                            // Strip one leading space/tab
                            let stripped = if next.starts_with('\t') {
                                &next[1..]
                            } else {
                                &next[1..]
                            };
                            content_lines.push(stripped.to_string());
                            i += 1;
                        } else {
                            break;
                        }
                    }
                    let combined = content_lines.join("\n");
                    let (inner_doc, _) = parse(&combined);
                    self.footnote_defs.push(FootnoteDef {
                        label: label.to_string(),
                        blocks: inner_doc.blocks,
                        span: Span {
                            start: start_offset,
                            end: self.line_offsets.get(i).copied().unwrap_or(self.input.len()),
                        },
                    });
                    continue;
                }
            }

            i += 1;
        }
    }

    fn parse_blocks(&mut self, end_line: usize) -> Vec<Block> {
        let mut blocks = Vec::new();
        while let Some(block) = self.parse_one_block_until(end_line) {
            blocks.push(block);
        }
        blocks
    }

    /// Parse the next single top-level block, up to `end_line`.
    /// Returns `None` when there are no more blocks to parse.
    pub(crate) fn parse_one_block(&mut self) -> Option<Block> {
        self.parse_one_block_until(self.lines.len())
    }

    fn parse_one_block_until(&mut self, end_line: usize) -> Option<Block> {
        loop {
            if self.pos >= end_line || self.at_end() {
                return None;
            }
            self.skip_blank_lines();
            if self.pos >= end_line || self.at_end() {
                return None;
            }
            let line = self.current_line()?;
            let trimmed = line.trim_start();

            // Skip link defs and footnote defs (already pre-scanned)
            if trimmed.starts_with('[') && !trimmed.starts_with("[^") {
                if parse_link_def(trimmed).is_some() {
                    self.advance();
                    continue;
                }
            }
            if trimmed.starts_with("[^") {
                if parse_footnote_def_start(trimmed).is_some() {
                    // Skip this and continuation lines
                    self.advance();
                    while let Some(next) = self.current_line() {
                        if next.starts_with(' ') || next.starts_with('\t') {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    continue;
                }
            }

            // Block attribute line: `{...}` on its own line
            if trimmed.starts_with('{') && looks_like_attr_line(trimmed) {
                if let Some(attr) = parse_attr(trimmed) {
                    self.pending_attr = Some(attr);
                    self.advance();
                    continue;
                }
            }

            // Heading
            if trimmed.starts_with('#') {
                if let Some(b) = self.parse_heading() {
                    return Some(b);
                }
            }

            // Thematic break: 3+ `*` or `-` on a line (with optional spaces)
            if is_thematic_break(trimmed) {
                let start = self.line_start(self.pos);
                self.advance();
                let attr = self.pending_attr.take().unwrap_or_default();
                return Some(Block::ThematicBreak {
                    attr,
                    span: Span { start, end: self.line_start(self.pos) },
                });
            }

            // Fenced code/raw block
            if trimmed.starts_with("```") {
                if let Some(b) = self.parse_fenced_code() {
                    return Some(b);
                }
            }

            // Div block :::
            if trimmed.starts_with(":::") {
                if let Some(b) = self.parse_div() {
                    return Some(b);
                }
            }

            // Blockquote
            if trimmed.starts_with("> ") || trimmed == ">" {
                if let Some(b) = self.parse_blockquote() {
                    return Some(b);
                }
            }

            // Table
            if trimmed.starts_with('|') {
                if let Some(b) = self.parse_table() {
                    return Some(b);
                }
            }

            // List items
            if let Some(list_marker) = detect_list_marker(trimmed) {
                if let Some(b) = self.parse_list(list_marker) {
                    return Some(b);
                }
            }

            // Definition list (`: ` marker)
            if trimmed.starts_with(": ") || trimmed == ":" {
                if let Some(b) = self.parse_definition_list() {
                    return Some(b);
                }
            }

            // Paragraph (fallback)
            if let Some(b) = self.parse_paragraph() {
                return Some(b);
            }

            // Nothing parsed — advance to avoid infinite loop
            self.advance();
        }
    }

    fn take_pending_attr(&mut self) -> Attr {
        self.pending_attr.take().unwrap_or_default()
    }

    fn parse_heading(&mut self) -> Option<Block> {
        let line = self.current_line()?;
        let trimmed = line.trim_start();
        let level = trimmed.chars().take_while(|&c| c == '#').count() as u8;
        if level == 0 || level > 6 {
            return None;
        }
        let after = &trimmed[level as usize..];
        // Must be followed by a space (or end of line for empty heading)
        if !after.is_empty() && !after.starts_with(' ') {
            return None;
        }
        let content = after.trim_start();
        let start = self.line_start(self.pos);
        self.advance();
        let end = self.line_start(self.pos);
        let attr = self.take_pending_attr();
        let inlines = parse_inlines(content, start + level as usize + 1, &self.link_defs);
        Some(Block::Heading { level, inlines, attr, span: Span { start, end } })
    }

    fn parse_fenced_code(&mut self) -> Option<Block> {
        let line = self.current_line()?;
        let indent = count_leading_spaces(line);
        let trimmed = &line[indent..];
        if !trimmed.starts_with("```") {
            return None;
        }
        let fence_char = '`';
        let fence_len = trimmed.chars().take_while(|&c| c == fence_char).count();
        let info = trimmed[fence_len..].trim();
        let start = self.line_start(self.pos);
        self.advance();

        let mut content_lines = Vec::new();
        loop {
            match self.current_line() {
                None => break,
                Some(l) => {
                    let l_trimmed = l.trim_start();
                    if l_trimmed.starts_with(&"`".repeat(fence_len)) {
                        let close_extra = l_trimmed[fence_len..].trim();
                        if close_extra.is_empty() {
                            self.advance();
                            break;
                        }
                    }
                    // Strip indent
                    let stripped = if indent > 0 {
                        l.get(indent..)
                            .filter(|_| l.len() >= indent && l[..indent].chars().all(|c| c == ' '))
                            .unwrap_or(l)
                    } else {
                        l
                    };
                    content_lines.push(stripped);
                    self.advance();
                }
            }
        }
        let end = self.line_start(self.pos);
        let attr = self.take_pending_attr();
        let raw_content = content_lines.join("\n");

        if let Some(fmt) = info.strip_prefix('=') {
            // Raw blocks: no trailing newline (content passed verbatim)
            Some(Block::RawBlock {
                format: fmt.to_string(),
                content: raw_content,
                attr,
                span: Span { start, end },
            })
        } else {
            // Code blocks: trailing newline (Pandoc convention)
            let content = if raw_content.is_empty() {
                String::new()
            } else {
                format!("{raw_content}\n")
            };
            let language = if info.is_empty() { None } else { Some(info.to_string()) };
            Some(Block::CodeBlock {
                language,
                content,
                attr,
                span: Span { start, end },
            })
        }
    }

    fn parse_div(&mut self) -> Option<Block> {
        let line = self.current_line()?;
        let trimmed = line.trim_start();
        if !trimmed.starts_with(":::") {
            return None;
        }
        let after = trimmed[3..].trim();
        let class = if after.is_empty() { None } else { Some(after.to_string()) };
        let start = self.line_start(self.pos);
        self.advance();

        let inner_blocks = self.parse_blocks_until_div_end();
        let end = self.line_start(self.pos);
        let attr = self.take_pending_attr();
        Some(Block::Div {
            class,
            blocks: inner_blocks,
            attr,
            span: Span { start, end },
        })
    }

    fn parse_blocks_until_div_end(&mut self) -> Vec<Block> {
        // Find the matching closing `:::` line, respecting nesting.
        // We scan forward to find it so that parse_blocks knows where to stop,
        // preventing the closing marker from being consumed as a new div.
        let close_line = self.find_div_close(self.pos);
        let blocks = self.parse_blocks(close_line);
        // Consume the closing `:::` line if present.
        if self.pos < self.lines.len() {
            let trimmed = self.lines[self.pos].trim_start();
            if trimmed.starts_with(":::") && trimmed[3..].trim().is_empty() {
                self.advance();
            }
        }
        blocks
    }

    /// Scan forward from `start` to find the line index of the matching `:::`
    /// closer, respecting nested divs. Returns `self.lines.len()` if not found.
    fn find_div_close(&self, start: usize) -> usize {
        let mut depth = 0usize;
        for i in start..self.lines.len() {
            let t = self.lines[i].trim_start();
            if t.starts_with(":::") {
                let rest = t[3..].trim();
                if rest.is_empty() {
                    // Potential closer
                    if depth == 0 {
                        return i;
                    }
                    depth -= 1;
                } else {
                    // Opener of nested div
                    depth += 1;
                }
            }
        }
        self.lines.len()
    }

    fn parse_blockquote(&mut self) -> Option<Block> {
        let start = self.line_start(self.pos);
        let mut inner_lines = Vec::new();

        while let Some(line) = self.current_line() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("> ") {
                inner_lines.push(&trimmed[2..]);
                self.advance();
            } else if trimmed == ">" {
                inner_lines.push("");
                self.advance();
            } else {
                break;
            }
        }

        if inner_lines.is_empty() {
            return None;
        }

        let inner_str = inner_lines.join("\n");
        let (inner_doc, _) = parse(&inner_str);
        let end = self.line_start(self.pos);
        let attr = self.take_pending_attr();
        Some(Block::Blockquote {
            blocks: inner_doc.blocks,
            attr,
            span: Span { start, end },
        })
    }

    fn parse_table(&mut self) -> Option<Block> {
        let start = self.line_start(self.pos);
        let mut raw_rows: Vec<(String, usize)> = Vec::new(); // (line, line_idx)

        // Gather caption line if it starts with `^`
        let mut caption: Option<Vec<Inline>> = None;

        while let Some(line) = self.current_line() {
            let trimmed = line.trim_start();
            if trimmed.starts_with('^') && !trimmed.starts_with('|') {
                // Caption line
                let cap_content = trimmed[1..].trim();
                caption =
                    Some(parse_inlines(cap_content, self.line_start(self.pos), &self.link_defs));
                self.advance();
                continue;
            }
            if trimmed.starts_with('|') {
                raw_rows.push((trimmed.to_string(), self.pos));
                self.advance();
            } else {
                break;
            }
        }

        if raw_rows.is_empty() {
            return None;
        }

        // Find separator rows (rows where all cells match `[-:]+`)
        let mut separator_positions: std::collections::HashSet<usize> =
            std::collections::HashSet::new();
        let mut alignment_map: Vec<Option<Alignment>> = Vec::new();

        for (idx, (row_str, _)) in raw_rows.iter().enumerate() {
            if is_separator_row(row_str) {
                separator_positions.insert(idx);
                if alignment_map.is_empty() {
                    alignment_map = parse_separator_alignments(row_str);
                }
            }
        }

        let mut rows: Vec<TableRow> = Vec::new();
        for (idx, (row_str, line_idx)) in raw_rows.iter().enumerate() {
            if separator_positions.contains(&idx) {
                // If this separator follows a row, make that row a header
                if let Some(prev) = rows.last_mut() {
                    prev.is_header = true;
                }
                continue;
            }

            let is_header = separator_positions.contains(&(idx + 1));
            let cells = parse_table_row(
                row_str,
                self.line_start(*line_idx),
                &alignment_map,
                &self.link_defs,
            );
            rows.push(TableRow {
                cells,
                is_header,
                span: Span {
                    start: self.line_start(*line_idx),
                    end: self.line_start(*line_idx + 1),
                },
            });
        }

        let end = self.line_start(self.pos);
        Some(Block::Table { caption, rows, span: Span { start, end } })
    }

    fn parse_list(&mut self, marker: ListMarker) -> Option<Block> {
        let start = self.line_start(self.pos);
        let attr = self.take_pending_attr();
        let kind = marker.to_list_kind();
        let mut items: Vec<ListItem> = Vec::new();
        let mut tight = true;

        // Extract style hint for disambiguation (roman vs alpha single chars)
        let style_hint = if let ListMarker::Ordered { ref style, .. } = marker {
            Some(style.clone())
        } else {
            None
        };

        while let Some(line) = self.current_line() {
            let trimmed = line.trim_start();
            // Re-detect with style hint so ambiguous tokens (e.g. `c` in roman/alpha)
            // are resolved in favor of the established list style.
            let m = if let Some(ref hint) = style_hint {
                // Try hint-aware ordered detection first
                if let Some(om) = detect_ordered_marker_with_hint(trimmed, Some(hint)) {
                    om
                } else if let Some(bm) = detect_list_marker(trimmed) {
                    bm
                } else {
                    break;
                }
            } else if let Some(bm) = detect_list_marker(trimmed) {
                bm
            } else {
                break;
            };
            if !m.compatible_with(&marker) {
                break;
            }
            let marker_str = m.marker_str();
            let item_start = self.line_start(self.pos);
            let skip = marker_str.len().min(trimmed.len());
            let content_after_marker = trimmed[skip..].trim_start();
            let checked = if m.is_task() {
                parse_task_marker(content_after_marker)
            } else {
                None
            };
            let first_line = if checked.is_some() {
                // strip the `[ ]` or `[x]` prefix
                skip_task_marker(content_after_marker)
            } else {
                content_after_marker
            };

            self.advance();

            // Collect continuation lines (indented)
            let mut item_lines = vec![first_line.to_string()];
            while let Some(next) = self.current_line() {
                if next.trim().is_empty() {
                    let blank_pos = self.pos;
                    self.advance();
                    // Continue collecting only if next non-blank line is indented
                    if let Some(after_blank) = self.current_line() {
                        if after_blank.starts_with("  ") || after_blank.starts_with('\t') {
                            // Blank line between indented blocks = loose
                            tight = false;
                            item_lines.push(String::new());
                            // will be picked up next iteration
                        } else {
                            // Blank line ends this item; restore so outer blank check can see it
                            self.pos = blank_pos;
                            break;
                        }
                    } else {
                        // Blank at end-of-input; item ends here
                        self.pos = blank_pos;
                        break;
                    }
                } else if next.starts_with("  ") || next.starts_with('\t') {
                    let stripped = if next.starts_with('\t') { &next[1..] } else { &next[2..] };
                    item_lines.push(stripped.to_string());
                    self.advance();
                } else {
                    break;
                }
            }

            let item_content = item_lines.join("\n");
            let (inner_doc, _) = parse(&item_content);
            let item_end = self.line_start(self.pos);
            items.push(ListItem {
                blocks: inner_doc.blocks,
                checked,
                span: Span { start: item_start, end: item_end },
            });

            // Check for blank line between items.
            // Only mark loose if the blank line is followed by another list item
            // (not the end of the list or a non-item continuation).
            if let Some(next) = self.current_line() {
                if next.trim().is_empty() {
                    let saved_pos = self.pos;
                    self.skip_blank_lines();
                    // Peek at what follows
                    if let Some(after) = self.current_line() {
                        let after_trimmed = after.trim_start();
                        let hint_ref = style_hint.as_ref();
                        let is_next_item = detect_list_marker(after_trimmed).is_some()
                            || detect_ordered_marker_with_hint(after_trimmed, hint_ref).is_some();
                        if is_next_item {
                            tight = false;
                        } else {
                            // Blank line ends the list — restore position so outer parser
                            // sees the blank line and the following content.
                            self.pos = saved_pos;
                        }
                    }
                    // If nothing follows, the blank line is trailing — ignore, list stays tight.
                }
            }
        }

        if items.is_empty() {
            return None;
        }

        let end = self.line_start(self.pos);
        Some(Block::List { kind, items, tight, attr, span: Span { start, end } })
    }

    fn parse_definition_list(&mut self) -> Option<Block> {
        let start = self.line_start(self.pos);
        let attr = self.take_pending_attr();
        let mut items: Vec<DefItem> = Vec::new();

        while let Some(line) = self.current_line() {
            let trimmed = line.trim_start();
            if !trimmed.starts_with(": ") && trimmed != ":" {
                break;
            }
            let term_content = if trimmed.starts_with(": ") { &trimmed[2..] } else { "" };
            let item_start = self.line_start(self.pos);
            self.advance();

            let mut def_lines: Vec<String> = Vec::new();
            while let Some(next) = self.current_line() {
                if next.starts_with("  ") || next.starts_with('\t') {
                    let stripped = if next.starts_with('\t') { &next[1..] } else { &next[2..] };
                    def_lines.push(stripped.to_string());
                    self.advance();
                } else if next.trim().is_empty() {
                    def_lines.push(String::new());
                    self.advance();
                } else {
                    break;
                }
            }

            let term =
                parse_inlines(term_content, item_start, &self.link_defs);
            let def_content = def_lines.join("\n");
            let (inner_doc, _) = parse(&def_content);
            let item_end = self.line_start(self.pos);
            items.push(DefItem {
                term,
                definitions: inner_doc.blocks,
                span: Span { start: item_start, end: item_end },
            });
        }

        if items.is_empty() {
            return None;
        }

        let end = self.line_start(self.pos);
        Some(Block::DefinitionList { items, attr, span: Span { start, end } })
    }

    fn parse_paragraph(&mut self) -> Option<Block> {
        let start = self.line_start(self.pos);
        let mut lines: Vec<&str> = Vec::new();

        while let Some(line) = self.current_line() {
            if line.trim().is_empty() {
                break;
            }
            let trimmed = line.trim_start();
            // Stop paragraph on block-level constructs
            if trimmed.starts_with('#')
                || trimmed.starts_with("```")
                || trimmed.starts_with(":::")
                || is_thematic_break(trimmed)
                || trimmed.starts_with("> ")
                || trimmed == ">"
                || trimmed.starts_with('|')
                || detect_list_marker(trimmed).is_some()
                || (trimmed.starts_with('{') && looks_like_attr_line(trimmed))
            {
                break;
            }
            lines.push(line.trim_end());
            self.advance();
        }

        if lines.is_empty() {
            return None;
        }

        let attr = self.take_pending_attr();
        let content = lines.join("\n");
        let inlines = parse_inlines(&content, start, &self.link_defs);
        let end = self.line_start(self.pos);
        Some(Block::Paragraph { inlines, attr, span: Span { start, end } })
    }
}

impl<'a> Iterator for EventIter<'a> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Event<'a>> {
        // Return buffered events first.
        if let Some(ev) = self.event_buf.pop_front() {
            return Some(ev);
        }

        loop {
            match &self.phase {
                Phase::Done => return None,

                Phase::Blocks => {
                    match self.parse_one_block() {
                        Some(block) => {
                            collect_block_events(&block, &mut self.event_buf);
                            if let Some(ev) = self.event_buf.pop_front() {
                                return Some(ev);
                            }
                            // block produced no events — keep going
                        }
                        None => {
                            // All top-level blocks consumed; switch to footnotes.
                            self.phase = Phase::Footnotes(0);
                        }
                    }
                }

                Phase::Footnotes(idx) => {
                    let i = *idx;
                    if i >= self.footnote_defs.len() {
                        self.phase = Phase::Done;
                        return None;
                    }
                    self.phase = Phase::Footnotes(i + 1);
                    let fn_def = &self.footnote_defs[i];
                    self.event_buf.push_back(Event::StartFootnoteDef { label: fn_def.label.clone() });
                    for block in &fn_def.blocks.clone() {
                        collect_block_events(block, &mut self.event_buf);
                    }
                    self.event_buf.push_back(Event::EndFootnoteDef);
                    if let Some(ev) = self.event_buf.pop_front() {
                        return Some(ev);
                    }
                }
            }
        }
    }
}

// ── Inline parsing ─────────────────────────────────────────────────────────

fn parse_inlines(input: &str, base_offset: usize, link_defs: &[LinkDef]) -> Vec<Inline> {
    let mut parser = InlineParser::new(input, base_offset, link_defs);
    parser.parse()
}

struct InlineParser<'a> {
    #[allow(dead_code)]
    input: &'a str,
    chars: Vec<char>,
    pos: usize,
    base_offset: usize,
    link_defs: &'a [LinkDef],
}

impl<'a> InlineParser<'a> {
    fn new(input: &'a str, base_offset: usize, link_defs: &'a [LinkDef]) -> Self {
        InlineParser {
            input,
            chars: input.chars().collect(),
            pos: 0,
            base_offset,
            link_defs,
        }
    }

    fn at_end(&self) -> bool {
        self.pos >= self.chars.len()
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek2(&self) -> Option<char> {
        self.chars.get(self.pos + 1).copied()
    }

    fn current_byte_offset(&self) -> usize {
        self.base_offset + self.chars[..self.pos].iter().map(|c| c.len_utf8()).sum::<usize>()
    }

    fn parse(&mut self) -> Vec<Inline> {
        let mut result: Vec<Inline> = Vec::new();
        while !self.at_end() {
            if let Some(inline) = self.parse_one(&mut result) {
                result.push(inline);
            }
        }
        merge_text_nodes(result)
    }

    fn parse_one(&mut self, acc: &mut Vec<Inline>) -> Option<Inline> {
        let start_offset = self.current_byte_offset();
        let ch = self.peek()?;

        // Newline → soft break or hard break
        if ch == '\n' {
            self.pos += 1;
            // Check what came before: if last char was `\`, that's a hard break
            // We handle `\` + newline below; here it's just a soft break
            return Some(Inline::SoftBreak { span: Span { start: start_offset, end: start_offset + 1 } });
        }

        // Backslash escapes / hard break / non-breaking space
        if ch == '\\' {
            self.pos += 1;
            let end_offset = self.current_byte_offset();
            match self.peek() {
                Some('\n') => {
                    // Remove trailing soft break from acc if present
                    if let Some(Inline::SoftBreak { .. }) = acc.last() {
                        acc.pop();
                    }
                    self.pos += 1;
                    return Some(Inline::HardBreak {
                        span: Span { start: start_offset, end: self.current_byte_offset() },
                    });
                }
                Some(' ') => {
                    self.pos += 1;
                    return Some(Inline::Text {
                        content: "\u{00A0}".to_string(),
                        span: Span { start: start_offset, end: self.current_byte_offset() },
                    });
                }
                Some(c) if is_punct(c) => {
                    let escaped = c;
                    self.pos += 1;
                    return Some(Inline::Text {
                        content: escaped.to_string(),
                        span: Span { start: start_offset, end: self.current_byte_offset() },
                    });
                }
                _ => {
                    return Some(Inline::Text {
                        content: "\\".to_string(),
                        span: Span { start: start_offset, end: end_offset },
                    });
                }
            }
        }

        // Smart punctuation
        if ch == '"' || ch == '\'' || ch == '-' || ch == '.' {
            if let Some(s) = self.try_smart_punct() {
                let end_offset = self.current_byte_offset();
                return Some(Inline::Text {
                    content: s,
                    span: Span { start: start_offset, end: end_offset },
                });
            }
        }

        // Code/verbatim span: backtick(s)
        if ch == '`' {
            if let Some(inline) = self.try_verbatim(start_offset) {
                return Some(inline);
            }
        }

        // Math: $ or $$
        if ch == '$' {
            if let Some(inline) = self.try_math(start_offset) {
                return Some(inline);
            }
        }

        // Autolink: `<url>` or `<email>`
        if ch == '<' {
            if let Some(inline) = self.try_autolink(start_offset) {
                return Some(inline);
            }
        }

        // Image: `![alt](url)` or `![alt][label]`
        if ch == '!' && self.peek2() == Some('[') {
            if let Some(inline) = self.try_image(start_offset) {
                return Some(inline);
            }
        }

        // Link / span / footnote ref: `[...](...)`
        if ch == '[' {
            if let Some(inline) = self.try_bracket(start_offset) {
                return Some(inline);
            }
        }

        // Span markers using brace syntax: `{-text-}`, `{+text+}`, `{=text=}`
        if ch == '{' {
            if let Some(inline) = self.try_brace_span(start_offset) {
                return Some(inline);
            }
        }

        // Emphasis: `_text_`
        if ch == '_' {
            if let Some(inline) = self.try_delimited('_', start_offset, |inlines, attr, span| {
                Inline::Emphasis { inlines, attr, span }
            }) {
                return Some(inline);
            }
        }

        // Strong: `*text*`
        if ch == '*' {
            if let Some(inline) = self.try_delimited('*', start_offset, |inlines, attr, span| {
                Inline::Strong { inlines, attr, span }
            }) {
                return Some(inline);
            }
        }

        // Subscript: `~text~`
        if ch == '~' {
            if let Some(inline) = self.try_delimited('~', start_offset, |inlines, attr, span| {
                Inline::Subscript { inlines, attr, span }
            }) {
                return Some(inline);
            }
        }

        // Superscript: `^text^`
        if ch == '^' {
            if let Some(inline) = self.try_delimited('^', start_offset, |inlines, attr, span| {
                Inline::Superscript { inlines, attr, span }
            }) {
                return Some(inline);
            }
        }

        // Symbol: `:name:`
        if ch == ':' {
            if let Some(inline) = self.try_symbol(start_offset) {
                return Some(inline);
            }
        }

        // Plain text
        self.pos += 1;
        let end_offset = self.current_byte_offset();
        Some(Inline::Text {
            content: ch.to_string(),
            span: Span { start: start_offset, end: end_offset },
        })
    }

    fn try_smart_punct(&mut self) -> Option<String> {
        let ch = self.peek()?;
        match ch {
            '"' => {
                self.pos += 1;
                // Simple heuristic: opening if preceded by whitespace/start, closing otherwise
                Some("\u{201C}".to_string())
            }
            '\'' => {
                self.pos += 1;
                Some("\u{2018}".to_string())
            }
            '-' => {
                if self.chars.get(self.pos + 1) == Some(&'-') {
                    if self.chars.get(self.pos + 2) == Some(&'-') {
                        self.pos += 3;
                        Some("\u{2014}".to_string()) // em dash
                    } else {
                        self.pos += 2;
                        Some("\u{2013}".to_string()) // en dash
                    }
                } else {
                    None
                }
            }
            '.' => {
                if self.chars.get(self.pos + 1) == Some(&'.')
                    && self.chars.get(self.pos + 2) == Some(&'.')
                {
                    self.pos += 3;
                    Some("\u{2026}".to_string()) // ellipsis
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn try_verbatim(&mut self, start_offset: usize) -> Option<Inline> {
        // Count backticks
        let tick_start = self.pos;
        let mut n = 0;
        while self.chars.get(self.pos + n) == Some(&'`') {
            n += 1;
        }
        // Find matching closing backticks
        let search_from = tick_start + n;
        let ticks_str: String = std::iter::repeat('`').take(n).collect();
        let remaining: String = self.chars[search_from..].iter().collect();
        if let Some(close_pos) = remaining.find(&ticks_str) {
            let content: String = self.chars[search_from..search_from + close_pos].iter().collect();
            // Trim single space on each side if content is non-empty and starts/ends with space
            let content = if content.starts_with(' ') && content.ends_with(' ') && content.len() > 2
            {
                content[1..content.len() - 1].to_string()
            } else {
                content
            };
            self.pos = search_from + close_pos + n;
            let end_offset = self.current_byte_offset();
            // Check for attribute or raw inline specifier
            let (attr, format) = self.try_post_verbatim_attr();
            if let Some(fmt) = format {
                return Some(Inline::RawInline {
                    format: fmt,
                    content,
                    span: Span { start: start_offset, end: self.current_byte_offset() },
                });
            }
            // Check for math prefix in original token
            // Actually math is `$` followed by verbatim — handled in try_math
            return Some(Inline::Verbatim {
                content,
                attr,
                span: Span { start: start_offset, end: end_offset },
            });
        }
        // No closing found — emit backticks as text
        let backtick_text: String = std::iter::repeat('`').take(n).collect();
        self.pos += n;
        let end_offset = self.current_byte_offset();
        Some(Inline::Text {
            content: backtick_text,
            span: Span { start: start_offset, end: end_offset },
        })
    }

    fn try_post_verbatim_attr(&mut self) -> (Attr, Option<String>) {
        if self.peek() == Some('{') {
            let saved = self.pos;
            self.pos += 1; // consume `{`
            let brace_start = self.pos;
            let mut depth = 1;
            while !self.at_end() && depth > 0 {
                match self.peek() {
                    Some('{') => { depth += 1; self.pos += 1; }
                    Some('}') => { depth -= 1; self.pos += 1; }
                    _ => { self.pos += 1; }
                }
            }
            let inner: String = self.chars[brace_start..self.pos - 1].iter().collect();
            let inner = inner.trim();
            if inner.starts_with('=') {
                let fmt = inner[1..].trim().to_string();
                return (Attr::default(), Some(fmt));
            }
            if let Some(attr) = parse_attr(&format!("{{{inner}}}")) {
                return (attr, None);
            }
            // Not a valid attr — restore position
            self.pos = saved;
        }
        (Attr::default(), None)
    }

    fn try_math(&mut self, start_offset: usize) -> Option<Inline> {
        // `$` or `$$` followed by backtick-verbatim
        let is_display = self.chars.get(self.pos + 1) == Some(&'$');
        let math_len = if is_display { 2 } else { 1 };
        let backtick_pos = self.pos + math_len;
        if self.chars.get(backtick_pos) != Some(&'`') {
            return None;
        }
        self.pos = backtick_pos;
        let saved = self.pos;
        if let Some(verbatim) = self.try_verbatim(start_offset) {
            if let Inline::Verbatim { content, .. } = verbatim {
                let end_offset = self.current_byte_offset();
                if is_display {
                    return Some(Inline::MathDisplay {
                        content,
                        span: Span { start: start_offset, end: end_offset },
                    });
                } else {
                    return Some(Inline::MathInline {
                        content,
                        span: Span { start: start_offset, end: end_offset },
                    });
                }
            }
        }
        self.pos = saved;
        None
    }

    fn try_autolink(&mut self, start_offset: usize) -> Option<Inline> {
        // `<` followed by url/email up to `>`
        let search_start = self.pos + 1;
        let remaining: String = self.chars[search_start..].iter().collect();
        if let Some(close) = remaining.find('>') {
            let inner = &remaining[..close];
            // Must look like a URL or email
            let is_url = inner.starts_with("http://")
                || inner.starts_with("https://")
                || inner.starts_with("ftp://")
                || inner.starts_with("mailto:");
            let is_email = !is_url && inner.contains('@') && !inner.contains(' ');
            if is_url || is_email {
                self.pos = search_start + close + 1; // past `>`
                let end_offset = self.current_byte_offset();
                let url = if is_email && !inner.starts_with("mailto:") {
                    format!("mailto:{inner}")
                } else {
                    inner.to_string()
                };
                return Some(Inline::Autolink {
                    url,
                    is_email,
                    span: Span { start: start_offset, end: end_offset },
                });
            }
        }
        None
    }

    fn try_image(&mut self, start_offset: usize) -> Option<Inline> {
        // `![...](url)` or `![...][label]`
        self.pos += 1; // consume `!`
        if let Some(inline) = self.try_bracket(start_offset) {
            match inline {
                Inline::Link { inlines, url, title, attr, span } => {
                    return Some(Inline::Image { inlines, url, title, attr, span });
                }
                Inline::Span { inlines, attr, span } => {
                    // Image with reference — look up in link_defs
                    // Extract label from span (simplified: use text content)
                    let label = extract_text_content(&inlines);
                    if let Some(ld) = self.link_defs.iter().find(|ld| ld.label == label) {
                        return Some(Inline::Image {
                            inlines,
                            url: ld.url.clone(),
                            title: ld.title.clone(),
                            attr,
                            span,
                        });
                    }
                    return Some(Inline::Image {
                        inlines,
                        url: String::new(),
                        title: None,
                        attr,
                        span,
                    });
                }
                _ => {
                    self.pos -= 1; // restore `!`
                    return None;
                }
            }
        }
        self.pos -= 1;
        None
    }

    fn try_bracket(&mut self, start_offset: usize) -> Option<Inline> {
        // `[` consumed next
        if self.peek() != Some('[') {
            return None;
        }
        self.pos += 1; // consume `[`

        // Check for footnote ref: `[^label]`
        if self.peek() == Some('^') {
            self.pos += 1; // consume `^`
            let label_start = self.pos;
            while let Some(c) = self.peek() {
                if c == ']' {
                    break;
                }
                self.pos += 1;
            }
            let label: String = self.chars[label_start..self.pos].iter().collect();
            if self.peek() == Some(']') {
                self.pos += 1; // consume `]`
                let end_offset = self.current_byte_offset();
                return Some(Inline::FootnoteRef {
                    label,
                    span: Span { start: start_offset, end: end_offset },
                });
            }
            // Not a footnote ref — restore
            self.pos = label_start - 1; // back to `[`
            return None;
        }

        // Find matching `]`
        let content_start = self.pos;
        let mut depth = 1;
        while !self.at_end() && depth > 0 {
            match self.peek() {
                Some('[') => { depth += 1; self.pos += 1; }
                Some(']') => { depth -= 1; if depth > 0 { self.pos += 1; } else { break; } }
                _ => { self.pos += 1; }
            }
        }
        if self.peek() != Some(']') {
            // No closing bracket
            self.pos = content_start - 1;
            return None;
        }
        let content: String = self.chars[content_start..self.pos].iter().collect();
        self.pos += 1; // consume `]`

        let end_of_brackets = self.pos;

        // What follows?
        match self.peek() {
            Some('(') => {
                // Inline link: `[text](url "title")`
                self.pos += 1;
                let url_start = self.pos;
                let mut paren_depth = 1;
                while !self.at_end() && paren_depth > 0 {
                    match self.peek() {
                        Some('(') => { paren_depth += 1; self.pos += 1; }
                        Some(')') => { paren_depth -= 1; if paren_depth > 0 { self.pos += 1; } else { break; } }
                        _ => { self.pos += 1; }
                    }
                }
                let url_content: String = self.chars[url_start..self.pos].iter().collect();
                if self.peek() == Some(')') {
                    self.pos += 1;
                }
                let (url, title) = parse_url_title(&url_content);
                let end_offset = self.current_byte_offset();
                let (attr, _) = self.try_post_verbatim_attr();
                let inlines = parse_inlines(&content, start_offset + 1, self.link_defs);
                return Some(Inline::Link {
                    inlines,
                    url,
                    title,
                    attr,
                    span: Span { start: start_offset, end: end_offset },
                });
            }
            Some('[') => {
                // Reference link: `[text][label]`
                self.pos += 1;
                let label_start = self.pos;
                while let Some(c) = self.peek() {
                    if c == ']' { break; }
                    self.pos += 1;
                }
                let label: String = self.chars[label_start..self.pos].iter().collect();
                if self.peek() == Some(']') {
                    self.pos += 1;
                }
                let end_offset = self.current_byte_offset();
                let url = self
                    .link_defs
                    .iter()
                    .find(|ld| ld.label == label)
                    .map(|ld| ld.url.clone())
                    .unwrap_or_default();
                let title = self
                    .link_defs
                    .iter()
                    .find(|ld| ld.label == label)
                    .and_then(|ld| ld.title.clone());
                let inlines = parse_inlines(&content, start_offset + 1, self.link_defs);
                return Some(Inline::Link {
                    inlines,
                    url,
                    title,
                    attr: Attr::default(),
                    span: Span { start: start_offset, end: end_offset },
                });
            }
            Some('{') => {
                // Span with attributes: `[text]{attrs}`
                let (attr, _) = self.try_post_verbatim_attr();
                let end_offset = self.current_byte_offset();
                let inlines = parse_inlines(&content, start_offset + 1, self.link_defs);
                return Some(Inline::Span {
                    inlines,
                    attr,
                    span: Span { start: start_offset, end: end_offset },
                });
            }
            _ => {
                // Just brackets — treat as span with no attr (or restore)
                // In Djot, `[text]` without following `(`, `[`, or `{` is just text
                self.pos = end_of_brackets;
                let end_offset = self.current_byte_offset();
                let inlines = parse_inlines(&content, start_offset + 1, self.link_defs);
                // Try as reference using content as label
                if let Some(ld) = self.link_defs.iter().find(|ld| ld.label == content) {
                    return Some(Inline::Link {
                        inlines,
                        url: ld.url.clone(),
                        title: ld.title.clone(),
                        attr: Attr::default(),
                        span: Span { start: start_offset, end: end_offset },
                    });
                }
                // Fall back: just return text-like span
                return Some(Inline::Span {
                    inlines,
                    attr: Attr::default(),
                    span: Span { start: start_offset, end: end_offset },
                });
            }
        }
    }

    fn try_brace_span(&mut self, start_offset: usize) -> Option<Inline> {
        // `{-text-}`, `{+text+}`, `{=text=}`
        if self.peek() != Some('{') {
            return None;
        }
        let next = self.chars.get(self.pos + 1).copied()?;
        let (open_marker, close_marker, builder): (
            char,
            char,
            fn(Vec<Inline>, Attr, Span) -> Inline,
        ) = match next {
            '-' => ('-', '-', |i, a, s| Inline::Delete { inlines: i, attr: a, span: s }),
            '+' => ('+', '+', |i, a, s| Inline::Insert { inlines: i, attr: a, span: s }),
            '=' => ('=', '=', |i, a, s| Inline::Highlight { inlines: i, attr: a, span: s }),
            _ => return None,
        };

        self.pos += 2; // consume `{` and marker char
        let content_start = self.pos;

        // Find `marker}`
        let close_pattern: String = format!("{close_marker}}}");
        let remaining: String = self.chars[content_start..].iter().collect();
        if let Some(close_pos) = remaining.find(&close_pattern) {
            let content: String = self.chars[content_start..content_start + close_pos].iter().collect();
            self.pos = content_start + close_pos + 2; // past `marker}`
            let end_offset = self.current_byte_offset();
            let inlines = parse_inlines(&content, start_offset + 2, self.link_defs);
            let _ = open_marker; // suppress unused warning
            return Some(builder(inlines, Attr::default(), Span { start: start_offset, end: end_offset }));
        }

        // No match — restore
        self.pos = content_start - 2;
        None
    }

    fn try_delimited(
        &mut self,
        delim: char,
        start_offset: usize,
        builder: fn(Vec<Inline>, Attr, Span) -> Inline,
    ) -> Option<Inline> {
        if self.peek() != Some(delim) {
            return None;
        }
        self.pos += 1; // consume opening delimiter
        let content_start = self.pos;

        // Find closing delimiter (not escaped)
        let mut i = content_start;
        while i < self.chars.len() {
            if self.chars[i] == '\\' {
                i += 2; // skip escaped char
                continue;
            }
            if self.chars[i] == delim {
                break;
            }
            i += 1;
        }

        if i >= self.chars.len() || self.chars[i] != delim {
            // No closing delimiter — restore
            self.pos = content_start - 1;
            return None;
        }

        let content: String = self.chars[content_start..i].iter().collect();
        self.pos = i + 1; // past closing delimiter

        // Check for attribute
        let (attr, _) = self.try_post_verbatim_attr();
        let end_offset = self.current_byte_offset();
        let inlines = parse_inlines(&content, start_offset + 1, self.link_defs);
        Some(builder(inlines, attr, Span { start: start_offset, end: end_offset }))
    }

    fn try_symbol(&mut self, start_offset: usize) -> Option<Inline> {
        if self.peek() != Some(':') {
            return None;
        }
        let name_start = self.pos + 1;
        let mut i = name_start;
        while i < self.chars.len() {
            let c = self.chars[i];
            if c == ':' {
                break;
            }
            if !c.is_alphanumeric() && c != '_' && c != '-' {
                return None; // Invalid symbol char
            }
            i += 1;
        }
        if i == name_start || i >= self.chars.len() || self.chars[i] != ':' {
            return None;
        }
        let name: String = self.chars[name_start..i].iter().collect();
        self.pos = i + 1; // past closing `:`
        let end_offset = self.current_byte_offset();
        Some(Inline::Symbol {
            name,
            span: Span { start: start_offset, end: end_offset },
        })
    }
}

// ── Helper functions ────────────────────────────────────────────────────────

fn merge_text_nodes(inlines: Vec<Inline>) -> Vec<Inline> {
    let mut result: Vec<Inline> = Vec::new();
    for inline in inlines {
        match inline {
            Inline::Text { content, span } => {
                if let Some(Inline::Text { content: prev_content, span: prev_span }) =
                    result.last_mut()
                {
                    prev_content.push_str(&content);
                    prev_span.end = span.end;
                } else {
                    result.push(Inline::Text { content, span });
                }
            }
            other => result.push(other),
        }
    }
    result
}

fn is_punct(c: char) -> bool {
    matches!(
        c,
        '!' | '"'
            | '#'
            | '$'
            | '%'
            | '&'
            | '\''
            | '('
            | ')'
            | '*'
            | '+'
            | ','
            | '-'
            | '.'
            | '/'
            | ':'
            | ';'
            | '<'
            | '='
            | '>'
            | '?'
            | '@'
            | '['
            | '\\'
            | ']'
            | '^'
            | '_'
            | '`'
            | '{'
            | '|'
            | '}'
            | '~'
    )
}

fn count_leading_spaces(s: &str) -> usize {
    s.chars().take_while(|&c| c == ' ').count()
}

fn is_thematic_break(s: &str) -> bool {
    let s = s.trim();
    if s.len() < 3 {
        return false;
    }
    let first = s.chars().next().unwrap_or(' ');
    if first != '*' && first != '-' {
        return false;
    }
    s.chars().all(|c| c == first || c == ' ')
        && s.chars().filter(|&c| c == first).count() >= 3
}

fn looks_like_attr_line(s: &str) -> bool {
    let s = s.trim();
    if !s.starts_with('{') || !s.ends_with('}') {
        return false;
    }
    // Quick sanity: should contain at least one of `#`, `.`, `=`, or be empty `{}`
    let inner = &s[1..s.len() - 1];
    inner.is_empty()
        || inner.starts_with('#')
        || inner.starts_with('.')
        || inner.contains('=')
        || inner.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

/// Parse `{#id .class key=val}` attribute string.
pub fn parse_attr(s: &str) -> Option<Attr> {
    let s = s.trim();
    if !s.starts_with('{') || !s.ends_with('}') {
        return None;
    }
    let inner = s[1..s.len() - 1].trim();
    let mut attr = Attr::default();
    let mut remaining = inner;
    while !remaining.is_empty() {
        remaining = remaining.trim_start();
        if remaining.is_empty() {
            break;
        }
        if remaining.starts_with('#') {
            // ID
            let end = remaining.find(|c: char| c.is_whitespace()).unwrap_or(remaining.len());
            attr.id = Some(remaining[1..end].to_string());
            remaining = &remaining[end..];
        } else if remaining.starts_with('.') {
            // Class
            let end = remaining.find(|c: char| c.is_whitespace()).unwrap_or(remaining.len());
            attr.classes.push(remaining[1..end].to_string());
            remaining = &remaining[end..];
        } else if remaining.starts_with('%') {
            // Comment — skip to end
            break;
        } else {
            // key=val or key="val"
            let eq_pos = remaining.find('=')?;
            let key = remaining[..eq_pos].trim().to_string();
            remaining = &remaining[eq_pos + 1..];
            let val = if remaining.starts_with('"') {
                let close = remaining[1..].find('"')? + 1;
                let v = remaining[1..close].to_string();
                remaining = &remaining[close + 1..];
                v
            } else {
                let end = remaining.find(|c: char| c.is_whitespace()).unwrap_or(remaining.len());
                let v = remaining[..end].to_string();
                remaining = &remaining[end..];
                v
            };
            attr.kv.push((key, val));
        }
    }
    Some(attr)
}

fn parse_link_def(s: &str) -> Option<LinkDef> {
    // `[label]: url` or `[label]: url "title"`
    if !s.starts_with('[') {
        return None;
    }
    let close = s.find(']')?;
    let label = s[1..close].to_string();
    let after = s[close + 1..].trim_start();
    if !after.starts_with(':') {
        return None;
    }
    let url_and_title = after[1..].trim();
    if url_and_title.is_empty() {
        return None;
    }
    let (url, title) = parse_url_title(url_and_title);
    Some(LinkDef { label, url, title, attr: Attr::default() })
}

fn parse_footnote_def_start(s: &str) -> Option<(&str, &str)> {
    if !s.starts_with("[^") {
        return None;
    }
    let close = s.find(']')?;
    let label = &s[2..close];
    let after = s[close + 1..].trim_start();
    if !after.starts_with(':') {
        return None;
    }
    let content = after[1..].trim_start();
    Some((label, content))
}

fn parse_url_title(s: &str) -> (String, Option<String>) {
    let s = s.trim();
    // Find url (up to whitespace or end)
    let url_end = s
        .find(|c: char| c.is_whitespace())
        .unwrap_or(s.len());
    let url = s[..url_end].to_string();
    let rest = s[url_end..].trim();
    let title = if rest.starts_with('"') && rest.ends_with('"') && rest.len() > 1 {
        Some(rest[1..rest.len() - 1].to_string())
    } else if rest.starts_with('\'') && rest.ends_with('\'') && rest.len() > 1 {
        Some(rest[1..rest.len() - 1].to_string())
    } else if rest.starts_with('(') && rest.ends_with(')') && rest.len() > 1 {
        Some(rest[1..rest.len() - 1].to_string())
    } else {
        None
    };
    (url, title)
}

// ── List marker detection ────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
enum ListMarker {
    Bullet(BulletStyle),
    Ordered {
        style: OrderedStyle,
        delimiter: OrderedDelimiter,
        number: u32,
    },
    Task,
}

impl ListMarker {
    fn to_list_kind(&self) -> ListKind {
        match self {
            ListMarker::Bullet(s) => ListKind::Bullet(s.clone()),
            ListMarker::Ordered { style, delimiter, number } => ListKind::Ordered {
                style: style.clone(),
                delimiter: delimiter.clone(),
                start: *number,
            },
            ListMarker::Task => ListKind::Task,
        }
    }

    fn compatible_with(&self, other: &ListMarker) -> bool {
        match (self, other) {
            (ListMarker::Bullet(a), ListMarker::Bullet(b)) => a == b,
            (ListMarker::Task, ListMarker::Task) => true,
            (
                ListMarker::Ordered { style: s1, delimiter: d1, .. },
                ListMarker::Ordered { style: s2, delimiter: d2, .. },
            ) => s1 == s2 && d1 == d2,
            _ => false,
        }
    }

    fn marker_str(&self) -> String {
        match self {
            ListMarker::Bullet(BulletStyle::Dash) => "- ".to_string(),
            ListMarker::Bullet(BulletStyle::Star) => "* ".to_string(),
            ListMarker::Bullet(BulletStyle::Plus) => "+ ".to_string(),
            ListMarker::Task => "- ".to_string(),
            ListMarker::Ordered { style, delimiter, number } => {
                let num_str = format_ordered_number(*number, style);
                match delimiter {
                    OrderedDelimiter::Period => format!("{num_str}. "),
                    OrderedDelimiter::Paren => format!("{num_str}) "),
                    OrderedDelimiter::Enclosed => format!("({num_str}) "),
                }
            }
        }
    }

    fn is_task(&self) -> bool {
        matches!(self, ListMarker::Task)
    }
}

fn format_ordered_number(n: u32, style: &OrderedStyle) -> String {
    match style {
        OrderedStyle::Decimal => n.to_string(),
        OrderedStyle::LowerAlpha => {
            let n = ((n - 1) % 26) as u8;
            ((b'a' + n) as char).to_string()
        }
        OrderedStyle::UpperAlpha => {
            let n = ((n - 1) % 26) as u8;
            ((b'A' + n) as char).to_string()
        }
        OrderedStyle::LowerRoman => to_roman(n).to_lowercase(),
        OrderedStyle::UpperRoman => to_roman(n),
    }
}

fn to_roman(n: u32) -> String {
    let vals = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut result = String::new();
    let mut n = n;
    for (val, sym) in vals {
        while n >= val {
            result.push_str(sym);
            n -= val;
        }
    }
    result
}

fn detect_list_marker(s: &str) -> Option<ListMarker> {
    if s.is_empty() {
        return None;
    }
    // Task list: `- [ ] ` or `- [x] ` (also `* [ ] `, `+ [ ] `)
    for bullet in &["- ", "* ", "+ "] {
        if let Some(rest) = s.strip_prefix(bullet) {
            if rest.starts_with("[ ] ") || rest.starts_with("[x] ") || rest.starts_with("[X] ") {
                return Some(ListMarker::Task);
            }
        }
    }

    // Bullet: `- `, `* `, `+ `
    if s.starts_with("- ") || s == "-" {
        return Some(ListMarker::Bullet(BulletStyle::Dash));
    }
    if s.starts_with("* ") || s == "*" {
        return Some(ListMarker::Bullet(BulletStyle::Star));
    }
    if s.starts_with("+ ") || s == "+" {
        return Some(ListMarker::Bullet(BulletStyle::Plus));
    }

    // Ordered: various patterns
    // `(1) `, `1. `, `1) `, `(a) `, `a. `, `a) `, `(i) `, etc.
    detect_ordered_marker(s)
}

fn detect_ordered_marker(s: &str) -> Option<ListMarker> {
    detect_ordered_marker_with_hint(s, None)
}

fn detect_ordered_marker_with_hint(s: &str, hint: Option<&OrderedStyle>) -> Option<ListMarker> {
    // Try enclosed: `(N) `
    if s.starts_with('(') {
        let close = s.find(')')?;
        let inner = &s[1..close];
        let after = &s[close + 1..];
        if after.starts_with(' ') || after.is_empty() {
            if let Some((style, number)) = parse_ordered_token_with_hint(inner, hint) {
                return Some(ListMarker::Ordered {
                    style,
                    delimiter: OrderedDelimiter::Enclosed,
                    number,
                });
            }
        }
    }

    // Try `N. ` or `N) `
    let dot_pos = s.find(|c| c == '.' || c == ')');
    if let Some(dp) = dot_pos {
        let token = &s[..dp];
        let delim_char = s.chars().nth(dp).unwrap_or('.');
        let after = &s[dp + 1..];
        if (after.starts_with(' ') || after.is_empty()) && !token.is_empty() {
            if let Some((style, number)) = parse_ordered_token_with_hint(token, hint) {
                let delimiter = if delim_char == '.' {
                    OrderedDelimiter::Period
                } else {
                    OrderedDelimiter::Paren
                };
                return Some(ListMarker::Ordered { style, delimiter, number });
            }
        }
    }

    None
}

#[allow(dead_code)]
fn parse_ordered_token(s: &str) -> Option<(OrderedStyle, u32)> {
    parse_ordered_token_with_hint(s, None)
}

/// Parse an ordered list token with an optional style hint (the established list style).
/// When the token is ambiguous (e.g. `c` is both roman-C=100 and alpha-c=3), the hint
/// resolves the ambiguity: if the list is already LowerAlpha, treat `c` as alpha(3).
fn parse_ordered_token_with_hint(
    s: &str,
    hint: Option<&OrderedStyle>,
) -> Option<(OrderedStyle, u32)> {
    if s.is_empty() {
        return None;
    }
    // Decimal
    if let Ok(n) = s.parse::<u32>() {
        return Some((OrderedStyle::Decimal, n));
    }
    let is_all_roman = s
        .chars()
        .all(|c| matches!(c, 'i' | 'v' | 'x' | 'l' | 'c' | 'd' | 'm' | 'I' | 'V' | 'X' | 'L' | 'C' | 'D' | 'M'));
    let is_single_char = s.len() == 1;

    // If the token is a single char that's a valid roman char, it's ambiguous.
    // Resolve using hint: if list is already alpha, keep it alpha.
    if is_single_char && is_all_roman {
        let c = s.chars().next().unwrap();
        let prefer_alpha = matches!(
            hint,
            Some(OrderedStyle::LowerAlpha) | Some(OrderedStyle::UpperAlpha)
        );
        if prefer_alpha {
            if c.is_ascii_lowercase() {
                return Some((OrderedStyle::LowerAlpha, (c as u32) - ('a' as u32) + 1));
            } else {
                return Some((OrderedStyle::UpperAlpha, (c as u32) - ('A' as u32) + 1));
            }
        }
        // No hint or roman hint: treat as roman (e.g. `i)` starting a roman list)
        if let Some(n) = from_roman(s) {
            if c.is_ascii_lowercase() {
                return Some((OrderedStyle::LowerRoman, n));
            } else {
                return Some((OrderedStyle::UpperRoman, n));
            }
        }
    }

    // Multi-char all-roman token → unambiguously roman
    if !is_single_char && is_all_roman {
        if let Some(n) = from_roman(s) {
            if s.chars().next().map(|c| c.is_ascii_lowercase()).unwrap_or(false) {
                return Some((OrderedStyle::LowerRoman, n));
            } else {
                return Some((OrderedStyle::UpperRoman, n));
            }
        }
    }

    // Single letter (non-roman chars like a, b, e, f, g, h, j, k, n, o, p...)
    if is_single_char {
        let c = s.chars().next().unwrap();
        if c.is_ascii_lowercase() {
            return Some((OrderedStyle::LowerAlpha, (c as u32) - ('a' as u32) + 1));
        }
        if c.is_ascii_uppercase() {
            return Some((OrderedStyle::UpperAlpha, (c as u32) - ('A' as u32) + 1));
        }
    }
    None
}

fn from_roman(s: &str) -> Option<u32> {
    let s = s.to_uppercase();
    let vals = [('I', 1), ('V', 5), ('X', 10), ('L', 50), ('C', 100), ('D', 500), ('M', 1000)];
    let mut result = 0u32;
    let chars: Vec<char> = s.chars().collect();
    if chars.is_empty() {
        return None;
    }
    // Quick check: all chars are roman
    for c in &chars {
        if !matches!(c, 'I' | 'V' | 'X' | 'L' | 'C' | 'D' | 'M') {
            return None;
        }
    }
    let val_of = |c: char| vals.iter().find(|&&(ch, _)| ch == c).map(|&(_, v)| v).unwrap_or(0);
    for i in 0..chars.len() {
        let v = val_of(chars[i]);
        let next_v = if i + 1 < chars.len() { val_of(chars[i + 1]) } else { 0 };
        if v < next_v {
            result = result.saturating_sub(v);
        } else {
            result += v;
        }
    }
    if result == 0 { None } else { Some(result) }
}

fn parse_task_marker(s: &str) -> Option<bool> {
    if s.starts_with("[ ] ") {
        Some(false)
    } else if s.starts_with("[x] ") || s.starts_with("[X] ") {
        Some(true)
    } else {
        None
    }
}

fn skip_task_marker(s: &str) -> &str {
    if s.starts_with("[ ] ") || s.starts_with("[x] ") || s.starts_with("[X] ") {
        &s[4..]
    } else {
        s
    }
}

// ── Table helpers ────────────────────────────────────────────────────────────

fn is_separator_row(s: &str) -> bool {
    let s = s.trim();
    if !s.starts_with('|') {
        return false;
    }
    // All cells must match `[-:]+` pattern
    let cells: Vec<&str> = s.trim_matches('|').split('|').collect();
    if cells.is_empty() {
        return false;
    }
    cells.iter().all(|cell| {
        let t = cell.trim();
        !t.is_empty() && t.chars().all(|c| c == '-' || c == ':' || c == ' ')
    })
}

fn parse_separator_alignments(s: &str) -> Vec<Option<Alignment>> {
    let s = s.trim().trim_matches('|');
    s.split('|')
        .map(|cell| {
            let t = cell.trim();
            let left = t.starts_with(':');
            let right = t.ends_with(':');
            match (left, right) {
                (true, true) => Some(Alignment::Center),
                (true, false) => Some(Alignment::Left),
                (false, true) => Some(Alignment::Right),
                (false, false) => None,
            }
        })
        .collect()
}

fn parse_table_row(
    s: &str,
    base_offset: usize,
    alignments: &[Option<Alignment>],
    link_defs: &[LinkDef],
) -> Vec<TableCell> {
    let s = s.trim().trim_matches('|');
    s.split('|')
        .enumerate()
        .map(|(i, cell)| {
            let content = cell.trim();
            let alignment = alignments
                .get(i)
                .and_then(|a| a.clone())
                .unwrap_or(Alignment::Default);
            TableCell {
                inlines: parse_inlines(content, base_offset, link_defs),
                alignment,
                span: Span::NONE,
            }
        })
        .collect()
}

fn extract_text_content(inlines: &[Inline]) -> String {
    let mut result = String::new();
    for inline in inlines {
        match inline {
            Inline::Text { content, .. } => result.push_str(content),
            Inline::Emphasis { inlines: inner, .. }
            | Inline::Strong { inlines: inner, .. }
            | Inline::Span { inlines: inner, .. } => {
                result.push_str(&extract_text_content(inner));
            }
            _ => {}
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_heading() {
        let (doc, diags) = parse("# Hello");
        assert!(diags.is_empty());
        assert_eq!(doc.blocks.len(), 1);
        match &doc.blocks[0] {
            Block::Heading { level, inlines, .. } => {
                assert_eq!(*level, 1);
                assert!(!inlines.is_empty());
            }
            other => panic!("expected heading, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_paragraph() {
        let (doc, diags) = parse("Hello world");
        assert!(diags.is_empty());
        assert_eq!(doc.blocks.len(), 1);
        match &doc.blocks[0] {
            Block::Paragraph { inlines, .. } => {
                assert!(!inlines.is_empty());
            }
            other => panic!("expected paragraph, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_code_block() {
        let input = "```rust\nfn main() {}\n```";
        let (doc, _) = parse(input);
        assert_eq!(doc.blocks.len(), 1);
        match &doc.blocks[0] {
            Block::CodeBlock { language, content, .. } => {
                assert_eq!(language.as_deref(), Some("rust"));
                assert_eq!(content, "fn main() {}\n");
            }
            other => panic!("expected code block, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_bullet_list() {
        let input = "- one\n- two\n- three";
        let (doc, _) = parse(input);
        assert_eq!(doc.blocks.len(), 1);
        match &doc.blocks[0] {
            Block::List { items, .. } => {
                assert_eq!(items.len(), 3);
            }
            other => panic!("expected list, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_thematic_break() {
        let (doc, _) = parse("---");
        assert!(matches!(doc.blocks[0], Block::ThematicBreak { .. }));
    }

    #[test]
    fn test_smart_punctuation() {
        let (doc, _) = parse("hello---world");
        match &doc.blocks[0] {
            Block::Paragraph { inlines, .. } => {
                let text: String = inlines
                    .iter()
                    .filter_map(|i| {
                        if let Inline::Text { content, .. } = i {
                            Some(content.as_str())
                        } else {
                            None
                        }
                    })
                    .collect();
                assert!(text.contains('\u{2014}'));
            }
            other => panic!("expected paragraph, got {other:?}"),
        }
    }

    #[test]
    fn test_strip_spans() {
        let (doc, _) = parse("# Heading\n\nParagraph text.");
        let stripped = doc.strip_spans();
        for block in &stripped.blocks {
            match block {
                Block::Heading { span, .. } | Block::Paragraph { span, .. } => {
                    assert_eq!(*span, Span::NONE);
                }
                _ => {}
            }
        }
    }
}
