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
use crate::events::{Event, OwnedEvent};
use std::borrow::Cow;

/// Parse a Djot string into a `DjotDoc` and diagnostics. Infallible.
///
/// Direct recursive descent — builds AST nodes without going through event
/// dispatch. Creates an `EventIter` to reuse `pre_scan` (link defs, footnote
/// defs), then calls the block-parsing helpers directly in a loop.
pub fn parse(input: &str) -> (DjotDoc, Vec<Diagnostic>) {
    let mut iter = EventIter::new(input);
    let blocks = parse_blocks_direct(&mut iter);
    let footnotes = std::mem::take(&mut iter.footnote_defs);
    let link_defs = std::mem::take(&mut iter.link_defs);
    let diagnostics = std::mem::take(&mut iter.diagnostics);
    (DjotDoc { blocks, footnotes, link_defs }, diagnostics)
}

/// Parse all top-level blocks from an `EventIter`, returning them as a `Vec<Block>`.
///
/// Calls the same block-parsing helpers as `push_next_block_frames` but
/// constructs `Block` values directly rather than routing through event
/// dispatch.
fn parse_blocks_direct(iter: &mut EventIter<'_>) -> Vec<Block> {
    let mut blocks = Vec::new();
    while let Some(block) = parse_next_block_direct(iter) {
        blocks.push(block);
    }
    blocks
}

/// Parse the next top-level block from `iter`, returning `None` when exhausted.
///
/// Mirrors `push_next_block_frames` but returns `Option<Block>` directly.
fn parse_next_block_direct(iter: &mut EventIter<'_>) -> Option<Block> {
    loop {
        iter.skip_blank_lines();
        if iter.at_end() {
            return None;
        }
        let line: String = iter.current_line()?.to_string();
        let trimmed = line.trim_start();

        // Skip link defs (pre-scanned in EventIter::new)
        if trimmed.starts_with('[') && !trimmed.starts_with("[^") {
            if parse_link_def(trimmed).is_some() {
                iter.advance();
                continue;
            }
        }
        // Skip footnote defs (pre-scanned in EventIter::new)
        if trimmed.starts_with("[^") {
            if parse_footnote_def_start(trimmed).is_some() {
                iter.advance();
                loop {
                    let starts_with_indent = iter.current_line()
                        .map(|l| l.starts_with(' ') || l.starts_with('\t'))
                        .unwrap_or(false);
                    if starts_with_indent { iter.advance(); } else { break; }
                }
                continue;
            }
        }

        // Block attribute line
        if trimmed.starts_with('{') && looks_like_attr_line(trimmed) {
            if let Some(attr) = parse_attr(trimmed) {
                iter.advance();
                iter.set_pending_attr(attr);
                continue;
            }
        }

        // Heading
        if trimmed.starts_with('#') {
            if let Some(b) = parse_heading_direct(iter) { return Some(b); }
        }

        // Thematic break
        if is_thematic_break(trimmed) {
            return Some(parse_thematic_break_direct(iter));
        }

        // Fenced code / raw block
        if trimmed.starts_with("```") {
            if let Some(b) = parse_fenced_code_direct(iter) { return Some(b); }
        }

        // Div block
        if trimmed.starts_with(":::") {
            if let Some(b) = parse_div_direct(iter) { return Some(b); }
        }

        // Blockquote
        if trimmed.starts_with("> ") || trimmed == ">" {
            if let Some(b) = parse_blockquote_direct(iter) { return Some(b); }
        }

        // Table (may start with caption line `^` or pipe row `|`)
        if trimmed.starts_with('|') || trimmed.starts_with('^') {
            if let Some(b) = parse_table_direct(iter) { return Some(b); }
        }

        // List
        if let Some(list_marker) = detect_list_marker(trimmed) {
            if let Some(b) = parse_list_direct(iter, list_marker) { return Some(b); }
        }

        // Definition list
        if trimmed.starts_with(": ") || trimmed == ":" {
            if let Some(b) = parse_definition_list_direct(iter) { return Some(b); }
        }

        // Paragraph (fallback)
        if let Some(b) = parse_paragraph_direct(iter) { return Some(b); }

        // Nothing parsed — advance to avoid infinite loop
        iter.advance();
    }
}

// ── Direct block constructors ─────────────────────────────────────────────────

fn parse_heading_direct(iter: &mut EventIter<'_>) -> Option<Block> {
    let heading_line = iter.pos();
    let line: String = iter.current_line()?.to_string();
    let trimmed = line.trim_start();
    let level = trimmed.chars().take_while(|&c| c == '#').count() as u8;
    if level == 0 || level > 6 { return None; }
    let after = &trimmed[level as usize..];
    if !after.is_empty() && !after.starts_with(' ') { return None; }
    let content = after.trim_start().to_string();
    iter.advance();
    let attr = iter.take_pending_attr();
    let link_defs: Vec<LinkDef> = iter.link_defs().to_vec();
    // base_offset: byte position of `content` within `self.input`.
    // `content` is the suffix of `line` after stripping leading indent, `#`s, and space.
    // `line.len() - content.len()` gives the prefix byte count.
    let base_offset = iter.line_offsets.get(heading_line).copied().unwrap_or(0)
        + line.len().saturating_sub(content.len());
    let inlines = parse_inlines(&content, base_offset, &link_defs);
    Some(Block::Heading { level, inlines, attr, span: Span::NONE })
}

fn parse_thematic_break_direct(iter: &mut EventIter<'_>) -> Block {
    iter.advance();
    let attr = iter.take_pending_attr();
    Block::ThematicBreak { attr, span: Span::NONE }
}

fn parse_fenced_code_direct(iter: &mut EventIter<'_>) -> Option<Block> {
    let line: String = iter.current_line()?.to_string();
    let indent = count_leading_spaces(&line);
    let trimmed_line = &line[indent..];
    if !trimmed_line.starts_with("```") { return None; }
    let fence_len = trimmed_line.chars().take_while(|&c| c == '`').count();
    let info = trimmed_line[fence_len..].trim().to_string();
    iter.advance();

    let mut content_lines: Vec<String> = Vec::new();
    loop {
        let l_owned: Option<String> = iter.current_line().map(|l| l.to_string());
        match l_owned {
            None => break,
            Some(l) => {
                let l_trimmed = l.trim_start();
                if l_trimmed.starts_with(&"`".repeat(fence_len)) {
                    let close_extra = l_trimmed[fence_len..].trim();
                    if close_extra.is_empty() {
                        iter.advance();
                        break;
                    }
                }
                let stripped = if indent > 0 {
                    // l.get(..indent) is None if indent falls inside a multi-byte char.
                    if let Some(prefix) = l.get(..indent) {
                        if prefix.chars().all(|c| c == ' ') {
                            l[indent..].to_string()
                        } else {
                            l.to_string()
                        }
                    } else {
                        l.to_string()
                    }
                } else {
                    l.to_string()
                };
                content_lines.push(stripped);
                iter.advance();
            }
        }
    }
    let attr = iter.take_pending_attr();
    let raw_content = content_lines.join("\n");

    if let Some(fmt) = info.strip_prefix('=') {
        Some(Block::RawBlock {
            format: fmt.trim().to_string(),
            content: raw_content,
            attr: Attr::default(),
            span: Span::NONE,
        })
    } else {
        let content = if raw_content.is_empty() { String::new() } else { format!("{raw_content}\n") };
        let language = if info.is_empty() { None } else { Some(info) };
        Some(Block::CodeBlock { language, content, attr, span: Span::NONE })
    }
}

fn parse_div_direct(iter: &mut EventIter<'_>) -> Option<Block> {
    let (class, valid) = {
        let line: String = iter.current_line()?.to_string();
        let trimmed = line.trim_start();
        if !trimmed.starts_with(":::") { return None; }
        let after = trimmed[3..].trim().to_string();
        let class = if after.is_empty() { None } else { Some(after) };
        (class, true)
    };
    let _ = valid;
    iter.advance();

    let close_line = find_div_close_generic(iter, iter.pos());
    let mut inner_lines: Vec<String> = Vec::new();
    while iter.pos() < close_line && !iter.at_end() {
        inner_lines.push(iter.line_at(iter.pos()).to_string());
        iter.advance();
    }
    {
        let should_advance = iter.current_line()
            .map(|t| {
                let t = t.trim_start();
                t.starts_with(":::") && t[3..].trim().is_empty()
            })
            .unwrap_or(false);
        if should_advance { iter.advance(); }
    }

    let attr = iter.take_pending_attr();
    let link_defs: Vec<LinkDef> = iter.link_defs().to_vec();
    let sub = SubParser::new(inner_lines, link_defs);
    let blocks = collect_blocks_from_sub(sub);
    Some(Block::Div { class, blocks, attr, span: Span::NONE })
}

fn parse_blockquote_direct(iter: &mut EventIter<'_>) -> Option<Block> {
    let mut inner_lines: Vec<String> = Vec::new();
    loop {
        let action = iter.current_line().map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with("> ") {
                Some(trimmed[2..].to_string())
            } else if trimmed == ">" {
                Some(String::new())
            } else {
                None
            }
        });
        match action {
            Some(Some(content)) => { inner_lines.push(content); iter.advance(); }
            Some(None) | None => break,
        }
    }
    if inner_lines.is_empty() { return None; }
    let attr = iter.take_pending_attr();
    let link_defs: Vec<LinkDef> = iter.link_defs().to_vec();
    let sub = SubParser::new(inner_lines, link_defs);
    let blocks = collect_blocks_from_sub(sub);
    Some(Block::Blockquote { blocks, attr, span: Span::NONE })
}

fn parse_table_direct(iter: &mut EventIter<'_>) -> Option<Block> {
    let saved_pos = iter.pos();
    let mut raw_rows: Vec<(String, usize)> = Vec::new();
    let mut caption: Option<Vec<Inline>> = None;
    let link_defs: Vec<LinkDef> = iter.link_defs().to_vec();

    loop {
        enum TableLineAction { Caption(String), Row(String), Stop }
        let action = iter.current_line().map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with('^') && !trimmed.starts_with('|') {
                TableLineAction::Caption(trimmed[1..].trim().to_string())
            } else if trimmed.starts_with('|') {
                TableLineAction::Row(trimmed.to_string())
            } else {
                TableLineAction::Stop
            }
        });
        match action {
            None | Some(TableLineAction::Stop) => break,
            Some(TableLineAction::Caption(cap_content)) => {
                caption = Some(parse_inlines(&cap_content, 0, &link_defs));
                iter.advance();
            }
            Some(TableLineAction::Row(row_str)) => {
                let row_pos = iter.pos();
                raw_rows.push((row_str, row_pos));
                iter.advance();
            }
        }
    }
    if raw_rows.is_empty() {
        iter.set_pos(saved_pos);
        return None;
    }

    let mut separator_positions: std::collections::HashSet<usize> = std::collections::HashSet::new();
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
    for (idx, (row_str, _line_idx)) in raw_rows.iter().enumerate() {
        if separator_positions.contains(&idx) {
            if let Some(prev) = rows.last_mut() {
                prev.is_header = true;
            }
            continue;
        }
        let is_header = separator_positions.contains(&(idx + 1));
        let cells = parse_table_row(row_str, 0, &alignment_map, &link_defs);
        rows.push(TableRow { cells, is_header, span: Span::NONE });
    }

    Some(Block::Table { caption, rows, span: Span::NONE })
}

fn parse_list_direct(iter: &mut EventIter<'_>, marker: ListMarker) -> Option<Block> {
    let attr = iter.take_pending_attr();
    let kind = marker.to_list_kind();
    let mut items: Vec<(Vec<String>, Option<bool>)> = Vec::new();
    let mut tight = true;

    let style_hint = if let ListMarker::Ordered { ref style, .. } = marker {
        Some(style.clone())
    } else {
        None
    };

    let link_defs: Vec<LinkDef> = iter.link_defs().to_vec();

    while let Some(l) = iter.current_line() {
        let line_owned: String = l.to_string();
        let trimmed_owned = line_owned.trim_start().to_string();
        let m = if let Some(ref hint) = style_hint {
            if let Some(om) = detect_ordered_marker_with_hint(&trimmed_owned, Some(hint)) {
                om
            } else if let Some(bm) = detect_list_marker(&trimmed_owned) {
                bm
            } else {
                break;
            }
        } else if let Some(bm) = detect_list_marker(&trimmed_owned) {
            bm
        } else {
            break;
        };
        if !m.compatible_with(&marker) { break; }

        let marker_str = m.marker_str();
        let skip = marker_str.len().min(trimmed_owned.len());
        let content_after_marker = trimmed_owned[skip..].trim_start().to_string();
        let checked = if m.is_task() { parse_task_marker(&content_after_marker) } else { None };
        let first_line = if checked.is_some() {
            skip_task_marker(&content_after_marker).to_string()
        } else {
            content_after_marker.clone()
        };

        iter.advance();

        let mut item_lines = vec![first_line];
        loop {
            let next_owned: Option<String> = iter.current_line().map(|l| l.to_string());
            let next = match next_owned { Some(ref s) => s.as_str(), None => break };
            if next.trim().is_empty() {
                let blank_pos = iter.pos();
                iter.advance();
                let after_blank_owned: Option<String> = iter.current_line().map(|l| l.to_string());
                if let Some(ref after_blank) = after_blank_owned {
                    if after_blank.starts_with("  ") || after_blank.starts_with('\t') {
                        tight = false;
                        item_lines.push(String::new());
                    } else {
                        iter.set_pos(blank_pos);
                        break;
                    }
                } else {
                    iter.set_pos(blank_pos);
                    break;
                }
            } else if next.starts_with("  ") || next.starts_with('\t') {
                let stripped = if next.starts_with('\t') { next[1..].to_string() } else { next[2..].to_string() };
                item_lines.push(stripped);
                iter.advance();
            } else {
                break;
            }
        }

        items.push((item_lines, checked));

        {
            let next_is_blank = iter.current_line().map(|l| l.trim().is_empty()).unwrap_or(false);
            if next_is_blank {
                let saved_pos = iter.pos();
                iter.skip_blank_lines();
                let after_owned: Option<String> = iter.current_line().map(|l| l.to_string());
                if let Some(ref after) = after_owned {
                    let after_trimmed = after.trim_start();
                    let hint_ref = style_hint.as_ref();
                    let is_next_item = detect_list_marker(after_trimmed).is_some()
                        || detect_ordered_marker_with_hint(after_trimmed, hint_ref).is_some();
                    if is_next_item {
                        tight = false;
                    } else {
                        iter.set_pos(saved_pos);
                    }
                }
            }
        }
    }

    if items.is_empty() { return None; }

    let list_items: Vec<ListItem> = items
        .into_iter()
        .map(|(item_lines, checked)| {
            let sub = SubParser::new(item_lines, link_defs.clone());
            let blocks = collect_blocks_from_sub(sub);
            ListItem { blocks, checked, span: Span::NONE }
        })
        .collect();

    Some(Block::List { kind, tight, items: list_items, attr, span: Span::NONE })
}

fn parse_definition_list_direct(iter: &mut EventIter<'_>) -> Option<Block> {
    let attr = iter.take_pending_attr();
    let mut def_items: Vec<(Vec<Inline>, Vec<String>)> = Vec::new();
    let link_defs: Vec<LinkDef> = iter.link_defs().to_vec();

    loop {
        let (term_content, valid) = {
            let line_owned: String = match iter.current_line() { Some(l) => l.to_string(), None => break };
            let trimmed = line_owned.trim_start().to_string();
            if !trimmed.starts_with(": ") && trimmed != ":" { break; }
            let tc = if trimmed.starts_with(": ") { trimmed[2..].to_string() } else { String::new() };
            (tc, true)
        };
        let _ = valid;
        iter.advance();

        let mut def_lines: Vec<String> = Vec::new();
        loop {
            let next_owned: Option<String> = iter.current_line().map(|l| l.to_string());
            let next = match next_owned { Some(ref s) => s.as_str(), None => break };
            if next.starts_with("  ") || next.starts_with('\t') {
                let stripped = if next.starts_with('\t') { next[1..].to_string() } else { next[2..].to_string() };
                def_lines.push(stripped);
                iter.advance();
            } else if next.trim().is_empty() {
                def_lines.push(String::new());
                iter.advance();
            } else {
                break;
            }
        }

        let term = parse_inlines(&term_content, 0, &link_defs);
        def_items.push((term, def_lines));
    }

    if def_items.is_empty() { return None; }

    let items: Vec<DefItem> = def_items
        .into_iter()
        .map(|(term, def_lines)| {
            let sub = SubParser::new(def_lines, link_defs.clone());
            let definitions = collect_blocks_from_sub(sub);
            DefItem { term, definitions, span: Span::NONE }
        })
        .collect();

    Some(Block::DefinitionList { items, attr, span: Span::NONE })
}

fn parse_paragraph_direct(iter: &mut EventIter<'_>) -> Option<Block> {
    let first_line = iter.pos();
    let mut lines: Vec<String> = Vec::new();
    loop {
        let line_owned: Option<String> = iter.current_line().map(|l| l.to_string());
        let line = match line_owned { Some(ref s) => s.as_str(), None => break };
        if line.trim().is_empty() { break; }
        let trimmed = line.trim_start();
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
        lines.push(line.trim_end().to_string());
        iter.advance();
    }
    if lines.is_empty() { return None; }
    let attr = iter.take_pending_attr();
    // base_offset: byte position of the paragraph's first character in `self.input`.
    // Inline spans are relative to the joined content; adding base_offset maps them to
    // absolute positions so EventIter::next can yield Cow::Borrowed for plain text runs.
    // The safety check in EventIter::next (content == &input[span]) catches any mismatch
    // caused by trim_end or non-Unix line endings and falls back to Cow::Owned.
    let base_offset = iter.line_offsets.get(first_line).copied().unwrap_or(0);
    let content = lines.join("\n");
    let link_defs: Vec<LinkDef> = iter.link_defs().to_vec();
    let inlines = parse_inlines(&content, base_offset, &link_defs);
    Some(Block::Paragraph { inlines, attr, span: Span::NONE })
}

/// Collect all blocks from a `SubParser` by draining its event stream into
/// AST nodes. Used by the direct block constructors for compound blocks
/// (blockquote, list items, div, definition descriptions).
fn collect_blocks_from_sub(sub: SubParser) -> Vec<Block> {
    use crate::events::collect_blocks_from_event_iter;
    collect_blocks_from_event_iter(sub)
}

/// Phase tracker for `EventIter`'s `Iterator` implementation.
#[derive(PartialEq)]
pub(crate) enum Phase {
    /// Parsing top-level blocks.
    Blocks,
    /// All done (top-level blocks + footnote defs all consumed).
    Done,
}

/// Lazy-traversal frame for the event-granular iterator.
///
/// Each frame represents a pending tree walk step. Frames are pushed onto
/// `frame_stack` in reverse-emission order, so the top of the stack is always
/// the next thing to emit.
///
/// Memory is O(nesting depth): only one path from the root to the current node
/// is live at any time — no subtree buffering.
enum Frame {
    /// Yield this single event immediately and pop.
    Event(OwnedEvent),
    /// A text inline whose content may borrow from the original input.
    ///
    /// `span` holds the byte range in the document input (set when base_offset
    /// is known); `content` is the owned fallback for cases where the span
    /// doesn't match (e.g. smart-punctuation substitution) or is unavailable.
    /// `EventIter::next` will try `Cow::Borrowed` first; `SubParser::next`
    /// always falls back to `Cow::Owned`.
    InlineText { span: Span, content: String },
    /// Pop the next Block from the iterator, expand it, and put the remainder back.
    Blocks(std::vec::IntoIter<Block>),
    /// Pop the next Inline, expand it, and put the remainder back.
    Inlines(std::vec::IntoIter<Inline>),
    /// Pop the next ListItem (Start + blocks + End), put the remainder back.
    ListItems(std::vec::IntoIter<ListItem>),
    /// Pop the next TableRow (Start + cells + End), put the remainder back.
    TableRows(std::vec::IntoIter<TableRow>),
    /// Pop the next TableCell (Start + inlines + End), put the remainder back.
    TableCells(std::vec::IntoIter<TableCell>),
    /// Pop the next DefItem (term + desc), put the remainder back.
    DefItems(std::vec::IntoIter<DefItem>),
    /// A sub-parser for compound block content (blockquote, list item, div).
    /// No intermediate Block allocation: the SubParser lazily parses owned lines.
    SubParser(Box<SubParser>),
}

// ── ParseContext trait ─────────────────────────────────────────────────────────

/// Shared interface for EventIter and SubParser so that block-pushing logic
/// can be written once as standalone generic functions.
trait ParseContext {
    fn num_lines(&self) -> usize;
    fn line_at(&self, idx: usize) -> &str;
    fn pos(&self) -> usize;
    fn set_pos(&mut self, pos: usize);
    fn take_pending_attr(&mut self) -> Attr;
    fn set_pending_attr(&mut self, attr: Attr);
    fn link_defs(&self) -> &[LinkDef];
    fn push_frame(&mut self, frame: Frame);

    /// Return the byte offset of `line_idx` within the original document input.
    ///
    /// Returns 0 by default (SubParser has no original-input reference).
    /// EventIter overrides this to return real offsets so that inline spans
    /// produced by `parse_inlines` can be used for `Cow::Borrowed` slices.
    fn line_offset_at(&self, _line_idx: usize) -> usize { 0 }

    fn current_line(&self) -> Option<&str> {
        if self.pos() < self.num_lines() { Some(self.line_at(self.pos())) } else { None }
    }
    fn advance(&mut self) {
        let p = self.pos();
        self.set_pos(p + 1);
    }
    fn at_end(&self) -> bool {
        self.pos() >= self.num_lines()
    }
    fn skip_blank_lines(&mut self) {
        while let Some(line) = self.current_line() {
            if line.trim().is_empty() { self.advance(); } else { break; }
        }
    }
}

// ── SubParser ─────────────────────────────────────────────────────────────────

/// A self-contained event-iterator over owned line content.
///
/// Used for compound blocks (blockquote, list items, div) whose inner content
/// is derived from the parent (e.g., ">" stripped from blockquote lines).
///
/// SubParser does NOT run pre_scan — link_defs are passed in from the parent.
/// Footnote defs are top-level only; SubParser never emits StartFootnoteDef.
///
/// Byte spans in SubParser events are 0 (no mapping to original input bytes).
/// This is a known limitation documented here.
pub(crate) struct SubParser {
    lines: Vec<String>,
    pos: usize,
    link_defs: Vec<LinkDef>,
    frame_stack: Vec<Frame>,
    phase: Phase,
    pending_attr: Option<Attr>,
}

impl SubParser {
    pub(crate) fn new(lines: Vec<String>, link_defs: Vec<LinkDef>) -> Self {
        SubParser {
            lines,
            pos: 0,
            link_defs,
            frame_stack: Vec::new(),
            phase: Phase::Blocks,
            pending_attr: None,
        }
    }
}

impl ParseContext for SubParser {
    fn num_lines(&self) -> usize { self.lines.len() }
    fn line_at(&self, idx: usize) -> &str { &self.lines[idx] }
    fn pos(&self) -> usize { self.pos }
    fn set_pos(&mut self, pos: usize) { self.pos = pos; }
    fn take_pending_attr(&mut self) -> Attr { self.pending_attr.take().unwrap_or_default() }
    fn set_pending_attr(&mut self, attr: Attr) { self.pending_attr = Some(attr); }
    fn link_defs(&self) -> &[LinkDef] { &self.link_defs }
    fn push_frame(&mut self, frame: Frame) { self.frame_stack.push(frame); }
}

impl Iterator for SubParser {
    type Item = OwnedEvent;

    fn next(&mut self) -> Option<OwnedEvent> {
        loop {
            match self.frame_stack.pop() {
                Some(Frame::Event(ev)) => return Some(ev),
                // SubParser has no input reference; always fall back to Cow::Owned.
                Some(Frame::InlineText { content, .. }) => {
                    return Some(OwnedEvent::Text(Cow::Owned(content)));
                }
                Some(Frame::Blocks(mut iter)) => {
                    if let Some(block) = iter.next() {
                        self.frame_stack.push(Frame::Blocks(iter));
                        expand_block_frames(self, block);
                    }
                    continue;
                }
                Some(Frame::Inlines(mut iter)) => {
                    if let Some(inline) = iter.next() {
                        self.frame_stack.push(Frame::Inlines(iter));
                        expand_inline_frames(self, inline);
                    }
                    continue;
                }
                Some(Frame::ListItems(mut iter)) => {
                    if let Some(item) = iter.next() {
                        self.frame_stack.push(Frame::ListItems(iter));
                        self.frame_stack.push(Frame::Event(OwnedEvent::EndListItem));
                        if !item.blocks.is_empty() {
                            self.frame_stack.push(Frame::Blocks(item.blocks.into_iter()));
                        }
                        self.frame_stack.push(Frame::Event(OwnedEvent::StartListItem {
                            checked: item.checked,
                        }));
                    }
                    continue;
                }
                Some(Frame::TableRows(mut iter)) => {
                    if let Some(row) = iter.next() {
                        self.frame_stack.push(Frame::TableRows(iter));
                        self.frame_stack.push(Frame::Event(OwnedEvent::EndTableRow));
                        if !row.cells.is_empty() {
                            self.frame_stack.push(Frame::TableCells(row.cells.into_iter()));
                        }
                        self.frame_stack.push(Frame::Event(OwnedEvent::StartTableRow {
                            is_header: row.is_header,
                        }));
                    }
                    continue;
                }
                Some(Frame::TableCells(mut iter)) => {
                    if let Some(cell) = iter.next() {
                        self.frame_stack.push(Frame::TableCells(iter));
                        self.frame_stack.push(Frame::Event(OwnedEvent::EndTableCell));
                        if !cell.inlines.is_empty() {
                            self.frame_stack.push(Frame::Inlines(cell.inlines.into_iter()));
                        }
                        self.frame_stack.push(Frame::Event(OwnedEvent::StartTableCell {
                            alignment: cell.alignment,
                        }));
                    }
                    continue;
                }
                Some(Frame::DefItems(mut iter)) => {
                    if let Some(item) = iter.next() {
                        self.frame_stack.push(Frame::DefItems(iter));
                        self.frame_stack.push(Frame::Event(OwnedEvent::EndDefinitionDesc));
                        if !item.definitions.is_empty() {
                            self.frame_stack.push(Frame::Blocks(item.definitions.into_iter()));
                        }
                        self.frame_stack.push(Frame::Event(OwnedEvent::StartDefinitionDesc));
                        self.frame_stack.push(Frame::Event(OwnedEvent::EndDefinitionTerm));
                        if !item.term.is_empty() {
                            self.frame_stack.push(Frame::Inlines(item.term.into_iter()));
                        }
                        self.frame_stack.push(Frame::Event(OwnedEvent::StartDefinitionTerm));
                    }
                    continue;
                }
                Some(Frame::SubParser(mut sub)) => {
                    if let Some(ev) = sub.next() {
                        self.frame_stack.push(Frame::SubParser(sub));
                        return Some(ev);
                    }
                    continue;
                }
                None => {
                    if self.phase == Phase::Done {
                        return None;
                    }
                    if !push_next_block_frames(self) {
                        self.phase = Phase::Done;
                    }
                    continue;
                }
            }
        }
    }
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
    /// Lazy frame stack — replaces the old `VecDeque<Event>` buffer.
    /// Memory is O(nesting depth), not O(subtree event count).
    frame_stack: Vec<Frame>,
    phase: Phase,
}

impl<'a> ParseContext for EventIter<'a> {
    fn num_lines(&self) -> usize { self.lines.len() }
    fn line_at(&self, idx: usize) -> &str { self.lines[idx] }
    fn pos(&self) -> usize { self.pos }
    fn set_pos(&mut self, pos: usize) { self.pos = pos; }
    fn take_pending_attr(&mut self) -> Attr { self.pending_attr.take().unwrap_or_default() }
    fn set_pending_attr(&mut self, attr: Attr) { self.pending_attr = Some(attr); }
    fn link_defs(&self) -> &[LinkDef] { &self.link_defs }
    fn push_frame(&mut self, frame: Frame) { self.frame_stack.push(frame); }
    fn line_offset_at(&self, line_idx: usize) -> usize {
        self.line_offsets.get(line_idx).copied().unwrap_or(0)
    }
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
            frame_stack: Vec::new(),
            phase: Phase::Blocks,
        };
        parser.pre_scan();
        parser
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
}

// ── Frame-stack expansion helpers (standalone, work with any ParseContext) ────

/// Expand a Block into frames pushed via ctx.push_frame() in reverse-emission
/// order so that frame_stack.pop() yields the start event first.
fn expand_block_frames<C: ParseContext>(ctx: &mut C, block: Block) {
    match block {
        Block::Paragraph { inlines, attr, .. } => {
            ctx.push_frame(Frame::Event(OwnedEvent::EndParagraph));
            if !inlines.is_empty() {
                ctx.push_frame(Frame::Inlines(inlines.into_iter()));
            }
            ctx.push_frame(Frame::Event(OwnedEvent::StartParagraph {
                id: attr.id, classes: attr.classes, kv: attr.kv,
            }));
        }
        Block::Heading { level, inlines, attr, .. } => {
            ctx.push_frame(Frame::Event(OwnedEvent::EndHeading));
            if !inlines.is_empty() {
                ctx.push_frame(Frame::Inlines(inlines.into_iter()));
            }
            ctx.push_frame(Frame::Event(OwnedEvent::StartHeading {
                level, id: attr.id, classes: attr.classes, kv: attr.kv,
            }));
        }
        Block::Blockquote { blocks, attr, .. } => {
            ctx.push_frame(Frame::Event(OwnedEvent::EndBlockquote));
            if !blocks.is_empty() {
                ctx.push_frame(Frame::Blocks(blocks.into_iter()));
            }
            ctx.push_frame(Frame::Event(OwnedEvent::StartBlockquote {
                id: attr.id, classes: attr.classes, kv: attr.kv,
            }));
        }
        Block::List { kind, items, tight, attr, .. } => {
            ctx.push_frame(Frame::Event(OwnedEvent::EndList));
            if !items.is_empty() {
                ctx.push_frame(Frame::ListItems(items.into_iter()));
            }
            ctx.push_frame(Frame::Event(OwnedEvent::StartList {
                kind, tight, id: attr.id, classes: attr.classes, kv: attr.kv,
            }));
        }
        Block::CodeBlock { language, content, attr, .. } => {
            ctx.push_frame(Frame::Event(OwnedEvent::EndCodeBlock));
            ctx.push_frame(Frame::Event(OwnedEvent::CodeBlockContent(Cow::Owned(content))));
            ctx.push_frame(Frame::Event(OwnedEvent::StartCodeBlock {
                language, id: attr.id, classes: attr.classes, kv: attr.kv,
            }));
        }
        Block::RawBlock { format, content, .. } => {
            ctx.push_frame(Frame::Event(OwnedEvent::RawBlock { format, content }));
        }
        Block::Div { class, blocks, attr, .. } => {
            ctx.push_frame(Frame::Event(OwnedEvent::EndDiv));
            if !blocks.is_empty() {
                ctx.push_frame(Frame::Blocks(blocks.into_iter()));
            }
            ctx.push_frame(Frame::Event(OwnedEvent::StartDiv {
                class, id: attr.id, classes: attr.classes, kv: attr.kv,
            }));
        }
        Block::Table { caption, rows, .. } => {
            ctx.push_frame(Frame::Event(OwnedEvent::EndTable));
            if !rows.is_empty() {
                ctx.push_frame(Frame::TableRows(rows.into_iter()));
            }
            ctx.push_frame(Frame::Event(OwnedEvent::StartTable));
            if let Some(cap) = caption {
                ctx.push_frame(Frame::Event(OwnedEvent::TableCaption(cap)));
            }
        }
        Block::ThematicBreak { attr, .. } => {
            ctx.push_frame(Frame::Event(OwnedEvent::ThematicBreak {
                id: attr.id, classes: attr.classes, kv: attr.kv,
            }));
        }
        Block::DefinitionList { items, attr, .. } => {
            ctx.push_frame(Frame::Event(OwnedEvent::EndDefinitionList));
            if !items.is_empty() {
                ctx.push_frame(Frame::DefItems(items.into_iter()));
            }
            ctx.push_frame(Frame::Event(OwnedEvent::StartDefinitionList {
                id: attr.id, classes: attr.classes, kv: attr.kv,
            }));
        }
    }
}

/// Expand an Inline into frames pushed via ctx.push_frame().
fn expand_inline_frames<C: ParseContext>(ctx: &mut C, inline: Inline) {
    match inline {
        Inline::Text { content, span } => {
            ctx.push_frame(Frame::InlineText { span, content });
        }
        Inline::SoftBreak { .. } => {
            ctx.push_frame(Frame::Event(OwnedEvent::SoftBreak));
        }
        Inline::HardBreak { .. } => {
            ctx.push_frame(Frame::Event(OwnedEvent::HardBreak));
        }
        Inline::Emphasis { inlines, attr, .. } => {
            ctx.push_frame(Frame::Event(OwnedEvent::EndEmphasis));
            if !inlines.is_empty() {
                ctx.push_frame(Frame::Inlines(inlines.into_iter()));
            }
            ctx.push_frame(Frame::Event(OwnedEvent::StartEmphasis {
                id: attr.id, classes: attr.classes, kv: attr.kv,
            }));
        }
        Inline::Strong { inlines, attr, .. } => {
            ctx.push_frame(Frame::Event(OwnedEvent::EndStrong));
            if !inlines.is_empty() {
                ctx.push_frame(Frame::Inlines(inlines.into_iter()));
            }
            ctx.push_frame(Frame::Event(OwnedEvent::StartStrong {
                id: attr.id, classes: attr.classes, kv: attr.kv,
            }));
        }
        Inline::Delete { inlines, attr, .. } => {
            ctx.push_frame(Frame::Event(OwnedEvent::EndDelete));
            if !inlines.is_empty() {
                ctx.push_frame(Frame::Inlines(inlines.into_iter()));
            }
            ctx.push_frame(Frame::Event(OwnedEvent::StartDelete {
                id: attr.id, classes: attr.classes, kv: attr.kv,
            }));
        }
        Inline::Insert { inlines, attr, .. } => {
            ctx.push_frame(Frame::Event(OwnedEvent::EndInsert));
            if !inlines.is_empty() {
                ctx.push_frame(Frame::Inlines(inlines.into_iter()));
            }
            ctx.push_frame(Frame::Event(OwnedEvent::StartInsert {
                id: attr.id, classes: attr.classes, kv: attr.kv,
            }));
        }
        Inline::Highlight { inlines, attr, .. } => {
            ctx.push_frame(Frame::Event(OwnedEvent::EndHighlight));
            if !inlines.is_empty() {
                ctx.push_frame(Frame::Inlines(inlines.into_iter()));
            }
            ctx.push_frame(Frame::Event(OwnedEvent::StartHighlight {
                id: attr.id, classes: attr.classes, kv: attr.kv,
            }));
        }
        Inline::Subscript { inlines, attr, .. } => {
            ctx.push_frame(Frame::Event(OwnedEvent::EndSubscript));
            if !inlines.is_empty() {
                ctx.push_frame(Frame::Inlines(inlines.into_iter()));
            }
            ctx.push_frame(Frame::Event(OwnedEvent::StartSubscript {
                id: attr.id, classes: attr.classes, kv: attr.kv,
            }));
        }
        Inline::Superscript { inlines, attr, .. } => {
            ctx.push_frame(Frame::Event(OwnedEvent::EndSuperscript));
            if !inlines.is_empty() {
                ctx.push_frame(Frame::Inlines(inlines.into_iter()));
            }
            ctx.push_frame(Frame::Event(OwnedEvent::StartSuperscript {
                id: attr.id, classes: attr.classes, kv: attr.kv,
            }));
        }
        Inline::Verbatim { content, attr, .. } => {
            ctx.push_frame(Frame::Event(Event::Verbatim {
                content: Cow::Owned(content),
                id: attr.id, classes: attr.classes, kv: attr.kv,
            }));
        }
        Inline::MathInline { content, .. } => {
            ctx.push_frame(Frame::Event(Event::MathInline(Cow::Owned(content))));
        }
        Inline::MathDisplay { content, .. } => {
            ctx.push_frame(Frame::Event(Event::MathDisplay(Cow::Owned(content))));
        }
        Inline::RawInline { format, content, .. } => {
            ctx.push_frame(Frame::Event(OwnedEvent::RawInline { format, content }));
        }
        Inline::Link { inlines, url, title, attr, .. } => {
            ctx.push_frame(Frame::Event(OwnedEvent::EndLink));
            if !inlines.is_empty() {
                ctx.push_frame(Frame::Inlines(inlines.into_iter()));
            }
            ctx.push_frame(Frame::Event(OwnedEvent::StartLink {
                url, title, id: attr.id, classes: attr.classes, kv: attr.kv,
            }));
        }
        Inline::Image { inlines, url, title, attr, .. } => {
            ctx.push_frame(Frame::Event(OwnedEvent::EndImage));
            if !inlines.is_empty() {
                ctx.push_frame(Frame::Inlines(inlines.into_iter()));
            }
            ctx.push_frame(Frame::Event(OwnedEvent::StartImage {
                url, title, id: attr.id, classes: attr.classes, kv: attr.kv,
            }));
        }
        Inline::Span { inlines, attr, .. } => {
            ctx.push_frame(Frame::Event(OwnedEvent::EndSpan));
            if !inlines.is_empty() {
                ctx.push_frame(Frame::Inlines(inlines.into_iter()));
            }
            ctx.push_frame(Frame::Event(OwnedEvent::StartSpan {
                id: attr.id, classes: attr.classes, kv: attr.kv,
            }));
        }
        Inline::FootnoteRef { label, .. } => {
            ctx.push_frame(Frame::Event(OwnedEvent::FootnoteRef(label)));
        }
        Inline::Symbol { name, .. } => {
            ctx.push_frame(Frame::Event(OwnedEvent::Symbol(name)));
        }
        Inline::Autolink { url, is_email, .. } => {
            ctx.push_frame(Frame::Event(OwnedEvent::Autolink { url, is_email }));
        }
    }
}

/// Push frames for the next top-level block in `ctx`.
///
/// For compound blocks (blockquote, list, div), pushes a `Frame::SubParser`
/// so that inner content is parsed lazily without building intermediate Block
/// structs. Simple blocks (paragraph, heading, code, table, thematic break)
/// are parsed directly.
///
/// Returns `true` if a block was pushed, `false` if the input is exhausted.
fn push_next_block_frames<C: ParseContext>(ctx: &mut C) -> bool {
    loop {
        ctx.skip_blank_lines();
        if ctx.at_end() {
            return false;
        }
        // Clone to owned String to avoid borrow conflict when passing ctx mutably below.
        let line: String = match ctx.current_line() {
            Some(l) => l.to_string(),
            None => return false,
        };
        let trimmed = line.trim_start();

        // Skip link defs (pre-scanned)
        if trimmed.starts_with('[') && !trimmed.starts_with("[^") {
            if parse_link_def(trimmed).is_some() {
                ctx.advance();
                continue;
            }
        }
        // Skip footnote defs (pre-scanned)
        if trimmed.starts_with("[^") {
            if parse_footnote_def_start(trimmed).is_some() {
                ctx.advance();
                loop {
                    let starts_with_indent = ctx.current_line()
                        .map(|l| l.starts_with(' ') || l.starts_with('\t'))
                        .unwrap_or(false);
                    if starts_with_indent { ctx.advance(); } else { break; }
                }
                continue;
            }
        }

        // Block attribute line
        if trimmed.starts_with('{') && looks_like_attr_line(trimmed) {
            if let Some(attr) = parse_attr(trimmed) {
                ctx.advance();
                ctx.set_pending_attr(attr);
                continue;
            }
        }

        // Heading
        if trimmed.starts_with('#') {
            if push_heading_frames(ctx) { return true; }
        }

        // Thematic break
        if is_thematic_break(trimmed) {
            push_thematic_break_frame(ctx);
            return true;
        }

        // Fenced code / raw block
        if trimmed.starts_with("```") {
            if push_fenced_code_frames(ctx) { return true; }
        }

        // Div block
        if trimmed.starts_with(":::") {
            if push_div_frames(ctx) { return true; }
        }

        // Blockquote
        if trimmed.starts_with("> ") || trimmed == ">" {
            if push_blockquote_frames(ctx) { return true; }
        }

        // Table (may start with caption line `^` or pipe row `|`)
        if trimmed.starts_with('|') || trimmed.starts_with('^') {
            if push_table_frames(ctx) { return true; }
        }

        // List
        if let Some(list_marker) = detect_list_marker(trimmed) {
            if push_list_frames(ctx, list_marker) { return true; }
        }

        // Definition list
        if trimmed.starts_with(": ") || trimmed == ":" {
            if push_definition_list_frames(ctx) { return true; }
        }

        // Paragraph (fallback)
        if push_paragraph_frames(ctx) { return true; }

        // Nothing parsed — advance to avoid infinite loop
        ctx.advance();
    }
}

// ── Block-frame push functions ────────────────────────────────────────────────

fn push_heading_frames<C: ParseContext>(ctx: &mut C) -> bool {
    let heading_line = ctx.pos();
    let line: String = match ctx.current_line() { Some(l) => l.to_string(), None => return false };
    let trimmed = line.trim_start();
    let level = trimmed.chars().take_while(|&c| c == '#').count() as u8;
    if level == 0 || level > 6 { return false; }
    let after = &trimmed[level as usize..];
    if !after.is_empty() && !after.starts_with(' ') { return false; }
    let content = after.trim_start().to_string();
    ctx.advance();
    let attr = ctx.take_pending_attr();
    let link_defs: Vec<LinkDef> = ctx.link_defs().to_vec();
    // base_offset positions `content` within the document input for Cow::Borrowed.
    let base_offset = ctx.line_offset_at(heading_line)
        + line.len().saturating_sub(content.len());
    let inlines = parse_inlines(&content, base_offset, &link_defs);
    ctx.push_frame(Frame::Event(OwnedEvent::EndHeading));
    if !inlines.is_empty() {
        ctx.push_frame(Frame::Inlines(inlines.into_iter()));
    }
    ctx.push_frame(Frame::Event(OwnedEvent::StartHeading {
        level, id: attr.id, classes: attr.classes, kv: attr.kv,
    }));
    true
}

fn push_thematic_break_frame<C: ParseContext>(ctx: &mut C) {
    ctx.advance();
    let attr = ctx.take_pending_attr();
    ctx.push_frame(Frame::Event(OwnedEvent::ThematicBreak {
        id: attr.id, classes: attr.classes, kv: attr.kv,
    }));
}

fn push_fenced_code_frames<C: ParseContext>(ctx: &mut C) -> bool {
    let line: String = match ctx.current_line() { Some(l) => l.to_string(), None => return false };
    let indent = count_leading_spaces(&line);
    let trimmed_line = &line[indent..];
    if !trimmed_line.starts_with("```") { return false; }
    let fence_len = trimmed_line.chars().take_while(|&c| c == '`').count();
    let info = trimmed_line[fence_len..].trim().to_string();
    ctx.advance();

    let mut content_lines: Vec<String> = Vec::new();
    loop {
        // Convert to owned to avoid borrow conflict with ctx.advance()
        let l_owned: Option<String> = ctx.current_line().map(|l| l.to_string());
        match l_owned {
            None => break,
            Some(l) => {
                let l_trimmed = l.trim_start();
                if l_trimmed.starts_with(&"`".repeat(fence_len)) {
                    let close_extra = l_trimmed[fence_len..].trim();
                    if close_extra.is_empty() {
                        ctx.advance();
                        break;
                    }
                }
                let stripped = if indent > 0 {
                    // l.get(..indent) is None if indent falls inside a multi-byte char.
                    if let Some(prefix) = l.get(..indent) {
                        if prefix.chars().all(|c| c == ' ') {
                            l[indent..].to_string()
                        } else {
                            l.to_string()
                        }
                    } else {
                        l.to_string()
                    }
                } else {
                    l.to_string()
                };
                content_lines.push(stripped);
                ctx.advance();
            }
        }
    }
    let attr = ctx.take_pending_attr();
    let raw_content = content_lines.join("\n");

    if let Some(fmt) = info.strip_prefix('=') {
        ctx.push_frame(Frame::Event(OwnedEvent::RawBlock {
            format: fmt.trim().to_string(),
            content: raw_content,
        }));
    } else {
        let content = if raw_content.is_empty() { String::new() } else { format!("{raw_content}\n") };
        let language = if info.is_empty() { None } else { Some(info) };
        ctx.push_frame(Frame::Event(OwnedEvent::EndCodeBlock));
        ctx.push_frame(Frame::Event(OwnedEvent::CodeBlockContent(Cow::Owned(content))));
        ctx.push_frame(Frame::Event(OwnedEvent::StartCodeBlock {
            language, id: attr.id, classes: attr.classes, kv: attr.kv,
        }));
    }
    true
}

fn find_div_close_generic<C: ParseContext>(ctx: &C, start: usize) -> usize {
    let mut depth = 0usize;
    let n = ctx.num_lines();
    for i in start..n {
        let t = ctx.line_at(i).trim_start();
        if t.starts_with(":::") {
            let rest = t[3..].trim();
            if rest.is_empty() {
                if depth == 0 { return i; }
                depth -= 1;
            } else {
                depth += 1;
            }
        }
    }
    n
}

fn push_div_frames<C: ParseContext>(ctx: &mut C) -> bool {
    let (class, valid) = {
        let line: String = match ctx.current_line() { Some(l) => l.to_string(), None => return false };
        let trimmed = line.trim_start();
        if !trimmed.starts_with(":::") { return false; }
        let after = trimmed[3..].trim().to_string();
        let class = if after.is_empty() { None } else { Some(after) };
        (class, true)
    };
    let _ = valid;
    ctx.advance();

    // Collect inner lines up to the matching `:::` closer
    let close_line = find_div_close_generic(ctx, ctx.pos());
    let mut inner_lines: Vec<String> = Vec::new();
    while ctx.pos() < close_line && !ctx.at_end() {
        inner_lines.push(ctx.line_at(ctx.pos()).to_string());
        ctx.advance();
    }
    // Consume closing `:::`
    {
        let should_advance = ctx.current_line()
            .map(|t| {
                let t = t.trim_start();
                t.starts_with(":::") && t[3..].trim().is_empty()
            })
            .unwrap_or(false);
        if should_advance { ctx.advance(); }
    }

    let attr = ctx.take_pending_attr();
    let link_defs: Vec<LinkDef> = ctx.link_defs().to_vec();
    let sub = SubParser::new(inner_lines, link_defs);

    ctx.push_frame(Frame::Event(OwnedEvent::EndDiv));
    ctx.push_frame(Frame::SubParser(Box::new(sub)));
    ctx.push_frame(Frame::Event(OwnedEvent::StartDiv {
        class, id: attr.id, classes: attr.classes, kv: attr.kv,
    }));
    true
}

fn push_blockquote_frames<C: ParseContext>(ctx: &mut C) -> bool {
    let mut inner_lines: Vec<String> = Vec::new();

    loop {
        let action = ctx.current_line().map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with("> ") {
                Some(trimmed[2..].to_string())
            } else if trimmed == ">" {
                Some(String::new())
            } else {
                None
            }
        });
        match action {
            Some(Some(content)) => { inner_lines.push(content); ctx.advance(); }
            Some(None) | None => break,
        }
    }

    if inner_lines.is_empty() { return false; }

    let attr = ctx.take_pending_attr();
    let link_defs: Vec<LinkDef> = ctx.link_defs().to_vec();
    let sub = SubParser::new(inner_lines, link_defs);

    ctx.push_frame(Frame::Event(OwnedEvent::EndBlockquote));
    ctx.push_frame(Frame::SubParser(Box::new(sub)));
    ctx.push_frame(Frame::Event(OwnedEvent::StartBlockquote {
        id: attr.id, classes: attr.classes, kv: attr.kv,
    }));
    true
}

fn push_table_frames<C: ParseContext>(ctx: &mut C) -> bool {
    let saved_pos = ctx.pos();
    let mut raw_rows: Vec<(String, usize)> = Vec::new();
    let mut caption: Option<Vec<Inline>> = None;
    let link_defs: Vec<LinkDef> = ctx.link_defs().to_vec();

    loop {
        enum TableLineAction { Caption(String), Row(String), Stop }
        let action = ctx.current_line().map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with('^') && !trimmed.starts_with('|') {
                TableLineAction::Caption(trimmed[1..].trim().to_string())
            } else if trimmed.starts_with('|') {
                TableLineAction::Row(trimmed.to_string())
            } else {
                TableLineAction::Stop
            }
        });
        match action {
            None | Some(TableLineAction::Stop) => break,
            Some(TableLineAction::Caption(cap_content)) => {
                caption = Some(parse_inlines(&cap_content, 0, &link_defs));
                ctx.advance();
            }
            Some(TableLineAction::Row(row_str)) => {
                let row_pos = ctx.pos();
                raw_rows.push((row_str, row_pos));
                ctx.advance();
            }
        }
    }

    if raw_rows.is_empty() {
        ctx.set_pos(saved_pos);
        return false;
    }

    let mut separator_positions: std::collections::HashSet<usize> = std::collections::HashSet::new();
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
    for (idx, (row_str, _line_idx)) in raw_rows.iter().enumerate() {
        if separator_positions.contains(&idx) {
            if let Some(prev) = rows.last_mut() {
                prev.is_header = true;
            }
            continue;
        }
        let is_header = separator_positions.contains(&(idx + 1));
        let cells = parse_table_row(row_str, 0, &alignment_map, &link_defs);
        rows.push(TableRow { cells, is_header, span: Span::NONE });
    }

    let table = Block::Table { caption, rows, span: Span::NONE };
    expand_block_frames(ctx, table);
    true
}

fn push_list_frames<C: ParseContext>(ctx: &mut C, marker: ListMarker) -> bool {
    let attr = ctx.take_pending_attr();
    let kind = marker.to_list_kind();
    let mut items: Vec<(Vec<String>, Option<bool>)> = Vec::new(); // (lines, checked)
    let mut tight = true;

    let style_hint = if let ListMarker::Ordered { ref style, .. } = marker {
        Some(style.clone())
    } else {
        None
    };

    let link_defs: Vec<LinkDef> = ctx.link_defs().to_vec();

    while let Some(l) = ctx.current_line() {
        // Convert to owned to avoid borrow conflict with ctx.advance()/set_pos().
        let line_owned: String = l.to_string();
        let trimmed_owned = line_owned.trim_start().to_string();
        let m = if let Some(ref hint) = style_hint {
            if let Some(om) = detect_ordered_marker_with_hint(&trimmed_owned, Some(hint)) {
                om
            } else if let Some(bm) = detect_list_marker(&trimmed_owned) {
                bm
            } else {
                break;
            }
        } else if let Some(bm) = detect_list_marker(&trimmed_owned) {
            bm
        } else {
            break;
        };
        if !m.compatible_with(&marker) { break; }

        let marker_str = m.marker_str();
        let skip = marker_str.len().min(trimmed_owned.len());
        let content_after_marker = trimmed_owned[skip..].trim_start().to_string();
        let checked = if m.is_task() { parse_task_marker(&content_after_marker) } else { None };
        let first_line = if checked.is_some() {
            skip_task_marker(&content_after_marker).to_string()
        } else {
            content_after_marker.clone()
        };

        ctx.advance();

        let mut item_lines = vec![first_line];
        loop {
            // Use owned to avoid borrow conflict with advance/set_pos
            let next_owned: Option<String> = ctx.current_line().map(|l| l.to_string());
            let next = match next_owned { Some(ref s) => s.as_str(), None => break };
            if next.trim().is_empty() {
                let blank_pos = ctx.pos();
                ctx.advance();
                let after_blank_owned: Option<String> = ctx.current_line().map(|l| l.to_string());
                if let Some(ref after_blank) = after_blank_owned {
                    if after_blank.starts_with("  ") || after_blank.starts_with('\t') {
                        tight = false;
                        item_lines.push(String::new());
                    } else {
                        ctx.set_pos(blank_pos);
                        break;
                    }
                } else {
                    ctx.set_pos(blank_pos);
                    break;
                }
            } else if next.starts_with("  ") || next.starts_with('\t') {
                let stripped = if next.starts_with('\t') { next[1..].to_string() } else { next[2..].to_string() };
                item_lines.push(stripped);
                ctx.advance();
            } else {
                break;
            }
        }

        items.push((item_lines, checked));

        // Check for blank line between items
        {
            let next_is_blank = ctx.current_line().map(|l| l.trim().is_empty()).unwrap_or(false);
            if next_is_blank {
                let saved_pos = ctx.pos();
                ctx.skip_blank_lines();
                let after_owned: Option<String> = ctx.current_line().map(|l| l.to_string());
                if let Some(ref after) = after_owned {
                    let after_trimmed = after.trim_start();
                    let hint_ref = style_hint.as_ref();
                    let is_next_item = detect_list_marker(after_trimmed).is_some()
                        || detect_ordered_marker_with_hint(after_trimmed, hint_ref).is_some();
                    if is_next_item {
                        tight = false;
                    } else {
                        ctx.set_pos(saved_pos);
                    }
                }
            }
        }
    }

    if items.is_empty() { return false; }

    // Build list items as SubParsers to avoid intermediate Block allocation
    // for item content. Push in reverse so first item is on top of stack.
    ctx.push_frame(Frame::Event(OwnedEvent::EndList));

    // Push items in reverse order
    for (item_lines, checked) in items.into_iter().rev() {
        let sub = SubParser::new(item_lines, link_defs.clone());
        ctx.push_frame(Frame::Event(OwnedEvent::EndListItem));
        ctx.push_frame(Frame::SubParser(Box::new(sub)));
        ctx.push_frame(Frame::Event(OwnedEvent::StartListItem { checked }));
    }

    ctx.push_frame(Frame::Event(OwnedEvent::StartList {
        kind, tight, id: attr.id, classes: attr.classes, kv: attr.kv,
    }));
    true
}

fn push_definition_list_frames<C: ParseContext>(ctx: &mut C) -> bool {
    let attr = ctx.take_pending_attr();
    let mut items: Vec<(Vec<Inline>, Vec<String>)> = Vec::new();
    let link_defs: Vec<LinkDef> = ctx.link_defs().to_vec();

    loop {
        // Convert to owned to avoid borrow conflict with advance()
        let (term_content, valid) = {
            let line_owned: String = match ctx.current_line() { Some(l) => l.to_string(), None => break };
            let trimmed = line_owned.trim_start().to_string();
            if !trimmed.starts_with(": ") && trimmed != ":" { break; }
            let tc = if trimmed.starts_with(": ") { trimmed[2..].to_string() } else { String::new() };
            (tc, true)
        };
        let _ = valid;
        ctx.advance();

        let mut def_lines: Vec<String> = Vec::new();
        loop {
            let next_owned: Option<String> = ctx.current_line().map(|l| l.to_string());
            let next = match next_owned { Some(ref s) => s.as_str(), None => break };
            if next.starts_with("  ") || next.starts_with('\t') {
                let stripped = if next.starts_with('\t') { next[1..].to_string() } else { next[2..].to_string() };
                def_lines.push(stripped);
                ctx.advance();
            } else if next.trim().is_empty() {
                def_lines.push(String::new());
                ctx.advance();
            } else {
                break;
            }
        }

        let term = parse_inlines(&term_content, 0, &link_defs);
        items.push((term, def_lines));
    }

    if items.is_empty() { return false; }

    ctx.push_frame(Frame::Event(OwnedEvent::EndDefinitionList));

    // Push items in reverse order so first is processed first
    for (term, def_lines) in items.into_iter().rev() {
        let sub = SubParser::new(def_lines, link_defs.clone());
        ctx.push_frame(Frame::Event(OwnedEvent::EndDefinitionDesc));
        ctx.push_frame(Frame::SubParser(Box::new(sub)));
        ctx.push_frame(Frame::Event(OwnedEvent::StartDefinitionDesc));
        ctx.push_frame(Frame::Event(OwnedEvent::EndDefinitionTerm));
        if !term.is_empty() {
            ctx.push_frame(Frame::Inlines(term.into_iter()));
        }
        ctx.push_frame(Frame::Event(OwnedEvent::StartDefinitionTerm));
    }

    ctx.push_frame(Frame::Event(OwnedEvent::StartDefinitionList {
        id: attr.id, classes: attr.classes, kv: attr.kv,
    }));
    true
}

fn push_paragraph_frames<C: ParseContext>(ctx: &mut C) -> bool {
    let first_line = ctx.pos();
    let mut lines: Vec<String> = Vec::new();

    loop {
        // Convert to owned to avoid borrow conflict with ctx.advance()
        let line_owned: Option<String> = ctx.current_line().map(|l| l.to_string());
        let line = match line_owned { Some(ref s) => s.as_str(), None => break };
        if line.trim().is_empty() { break; }
        let trimmed = line.trim_start();
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
        lines.push(line.trim_end().to_string());
        ctx.advance();
    }

    if lines.is_empty() { return false; }

    let attr = ctx.take_pending_attr();
    // base_offset: byte position of the paragraph start within the document input.
    // Used by parse_inlines so Text spans are absolute, enabling Cow::Borrowed in EventIter.
    let base_offset = ctx.line_offset_at(first_line);
    let content = lines.join("\n");
    let link_defs: Vec<LinkDef> = ctx.link_defs().to_vec();
    let inlines = parse_inlines(&content, base_offset, &link_defs);

    ctx.push_frame(Frame::Event(OwnedEvent::EndParagraph));
    if !inlines.is_empty() {
        ctx.push_frame(Frame::Inlines(inlines.into_iter()));
    }
    ctx.push_frame(Frame::Event(OwnedEvent::StartParagraph {
        id: attr.id, classes: attr.classes, kv: attr.kv,
    }));
    true
}

impl<'a> Iterator for EventIter<'a> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Event<'a>> {
        loop {
            match self.frame_stack.pop() {
                // ── Atomic: yield immediately ─────────────────────────────────
                Some(Frame::Event(ev)) => return Some(ev),

                // ── Text inline: borrow from input when span is valid ─────────
                Some(Frame::InlineText { span, content }) => {
                    let text = if span.start < span.end
                        && span.end <= self.input.len()
                        && self.input.is_char_boundary(span.start)
                        && self.input.is_char_boundary(span.end)
                        && &self.input[span.start..span.end] == content.as_str()
                    {
                        Cow::Borrowed(&self.input[span.start..span.end])
                    } else {
                        Cow::Owned(content)
                    };
                    return Some(Event::Text(text));
                }

                // ── Block sequence (from expand_block_frames on pre-built Blocks) ──
                Some(Frame::Blocks(mut iter)) => {
                    if let Some(block) = iter.next() {
                        self.frame_stack.push(Frame::Blocks(iter));
                        expand_block_frames(self, block);
                    }
                    continue;
                }

                // ── Inline sequence ───────────────────────────────────────────
                Some(Frame::Inlines(mut iter)) => {
                    if let Some(inline) = iter.next() {
                        self.frame_stack.push(Frame::Inlines(iter));
                        expand_inline_frames(self, inline);
                    }
                    continue;
                }

                // ── List items (from pre-built Block::List) ───────────────────
                Some(Frame::ListItems(mut iter)) => {
                    if let Some(item) = iter.next() {
                        self.frame_stack.push(Frame::ListItems(iter));
                        self.frame_stack.push(Frame::Event(OwnedEvent::EndListItem));
                        if !item.blocks.is_empty() {
                            self.frame_stack.push(Frame::Blocks(item.blocks.into_iter()));
                        }
                        self.frame_stack.push(Frame::Event(OwnedEvent::StartListItem {
                            checked: item.checked,
                        }));
                    }
                    continue;
                }

                // ── Table rows ────────────────────────────────────────────────
                Some(Frame::TableRows(mut iter)) => {
                    if let Some(row) = iter.next() {
                        self.frame_stack.push(Frame::TableRows(iter));
                        self.frame_stack.push(Frame::Event(OwnedEvent::EndTableRow));
                        if !row.cells.is_empty() {
                            self.frame_stack.push(Frame::TableCells(row.cells.into_iter()));
                        }
                        self.frame_stack.push(Frame::Event(OwnedEvent::StartTableRow {
                            is_header: row.is_header,
                        }));
                    }
                    continue;
                }

                // ── Table cells ───────────────────────────────────────────────
                Some(Frame::TableCells(mut iter)) => {
                    if let Some(cell) = iter.next() {
                        self.frame_stack.push(Frame::TableCells(iter));
                        self.frame_stack.push(Frame::Event(OwnedEvent::EndTableCell));
                        if !cell.inlines.is_empty() {
                            self.frame_stack.push(Frame::Inlines(cell.inlines.into_iter()));
                        }
                        self.frame_stack.push(Frame::Event(OwnedEvent::StartTableCell {
                            alignment: cell.alignment,
                        }));
                    }
                    continue;
                }

                // ── Definition list items (from pre-built Block) ──────────────
                Some(Frame::DefItems(mut iter)) => {
                    if let Some(item) = iter.next() {
                        self.frame_stack.push(Frame::DefItems(iter));
                        self.frame_stack.push(Frame::Event(OwnedEvent::EndDefinitionDesc));
                        if !item.definitions.is_empty() {
                            self.frame_stack.push(Frame::Blocks(item.definitions.into_iter()));
                        }
                        self.frame_stack.push(Frame::Event(OwnedEvent::StartDefinitionDesc));
                        self.frame_stack.push(Frame::Event(OwnedEvent::EndDefinitionTerm));
                        if !item.term.is_empty() {
                            self.frame_stack.push(Frame::Inlines(item.term.into_iter()));
                        }
                        self.frame_stack.push(Frame::Event(OwnedEvent::StartDefinitionTerm));
                    }
                    continue;
                }

                // ── Sub-parser for compound block content ─────────────────────
                Some(Frame::SubParser(mut sub)) => {
                    if let Some(ev) = sub.next() {
                        self.frame_stack.push(Frame::SubParser(sub));
                        return Some(ev);
                    }
                    continue;
                }

                // ── Frame stack empty: parse next top-level block ─────────────
                None => {
                    if self.phase == Phase::Done {
                        return None;
                    }
                    if !push_next_block_frames(self) {
                        // All top-level blocks consumed; push footnote defs.
                        let fns = std::mem::take(&mut self.footnote_defs);
                        for fn_def in fns.into_iter().rev() {
                            self.frame_stack.push(Frame::Event(OwnedEvent::EndFootnoteDef));
                            if !fn_def.blocks.is_empty() {
                                self.frame_stack.push(Frame::Blocks(fn_def.blocks.into_iter()));
                            }
                            self.frame_stack.push(Frame::Event(OwnedEvent::StartFootnoteDef {
                                label: fn_def.label,
                            }));
                        }
                        self.phase = Phase::Done;
                    }
                    continue;
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
        if let Some(close_pos_bytes) = remaining.find(&ticks_str) {
            // close_pos_bytes is a byte index in `remaining` (a UTF-8 string).
            // Convert to a char count for indexing into self.chars.
            let close_pos = remaining[..close_pos_bytes].chars().count();
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
            if depth > 0 {
                // Unclosed brace — restore and return default (not a valid attr)
                self.pos = saved;
                return (Attr::default(), None);
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
        if let Some(close_bytes) = remaining.find('>') {
            let inner = &remaining[..close_bytes];
            // Must look like a URL or email
            let is_url = inner.starts_with("http://")
                || inner.starts_with("https://")
                || inner.starts_with("ftp://")
                || inner.starts_with("mailto:");
            let is_email = !is_url && inner.contains('@') && !inner.contains(' ');
            if is_url || is_email {
                // close_bytes is a byte index; convert to char count for self.pos.
                let close = remaining[..close_bytes].chars().count();
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
        if let Some(close_pos_bytes) = remaining.find(&close_pattern) {
            // close_pos_bytes is a byte index in `remaining`; convert to char count.
            let close_pos = remaining[..close_pos_bytes].chars().count();
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
    fn test_events_cow_borrowed_plain_text() {
        // Plain text with no escape sequences should yield Cow::Borrowed slices
        // pointing into the original input, not freshly-allocated Cow::Owned strings.
        use crate::events::EventIter;
        use std::borrow::Cow;
        let input = "# Hello\n\nA paragraph.";
        let mut heading_borrowed = false;
        let mut para_borrowed = false;
        for ev in EventIter::new(input) {
            if let crate::events::Event::Text(Cow::Borrowed(s)) = ev {
                // Verify it's actually a slice of the input (same bytes, same address range)
                let input_range = input.as_ptr()..=input.as_ptr().wrapping_add(input.len());
                assert!(input_range.contains(&s.as_ptr()), "borrowed slice not in input");
                if s == "Hello" { heading_borrowed = true; }
                if s == "A paragraph." { para_borrowed = true; }
            }
        }
        assert!(heading_borrowed, "heading text should be Cow::Borrowed");
        assert!(para_borrowed, "paragraph text should be Cow::Borrowed");
    }

    #[test]
    fn test_events_cow_owned_smart_punct() {
        // Smart punctuation substitutions must yield Cow::Owned (content differs from input).
        use crate::events::EventIter;
        use std::borrow::Cow;
        let input = "A -- B"; // em-dash substitution
        let owned_texts: Vec<_> = EventIter::new(input)
            .filter_map(|ev| {
                if let crate::events::Event::Text(Cow::Owned(s)) = ev { Some(s) } else { None }
            })
            .collect();
        // The em-dash text should be owned (can't borrow "—" from "–" input)
        assert!(owned_texts.iter().any(|s| s.contains('\u{2013}') || s.contains('\u{2014}')),
            "smart punctuation should yield Cow::Owned");
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
