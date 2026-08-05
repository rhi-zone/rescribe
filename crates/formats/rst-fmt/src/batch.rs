//! Chunk-driven (batch) RST parser.
//!
//! # Memory model
//!
//! [`StreamingParser`] processes input in logical blocks (content between blank
//! lines, or directive bodies). Memory usage is O(largest block), not O(full
//! input). [`BatchParser`] buffers all input and is O(full input).
//!
//! # Example — AST style
//! ```no_run
//! use rst_fmt::batch::BatchParser;
//!
//! let mut p = BatchParser::new();
//! p.feed(b"Section\n=======\n\n");
//! p.feed(b"A paragraph.\n");
//! let doc = p.finish().unwrap();
//! ```
//!
//! # Example — event callback style
//! ```no_run
//! use rst_fmt::batch::{StreamingParser, Handler};
//! use rst_fmt::OwnedEvent;
//!
//! let mut events = Vec::new();
//! let mut p = StreamingParser::new(|ev: OwnedEvent| events.push(ev));
//! p.feed(b"Section\n=======\n\n");
//! p.feed(b"A paragraph.\n");
//! p.finish();
//! ```

use crate::events::OwnedEvent;
use crate::{RstDoc, RstError};

/// Chunk-driven RST parser that returns the full AST on finish.
#[derive(Default)]
pub struct BatchParser {
    buf: Vec<u8>,
}

impl BatchParser {
    pub fn new() -> Self {
        BatchParser { buf: Vec::new() }
    }

    /// Feed a chunk of input bytes.
    pub fn feed(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);
    }

    /// Finish parsing and return the AST.
    ///
    /// The AST is `'static`: chunked input lives in the parser's own buffer,
    /// which is dropped here, so the tree is converted with
    /// [`RstDoc::into_owned`]. Callers holding the whole input in memory
    /// already should call [`crate::parse`] directly and keep the borrows.
    pub fn finish(self) -> Result<RstDoc<'static>, RstError> {
        let s = String::from_utf8_lossy(&self.buf);
        crate::parse(&s).map(RstDoc::into_owned)
    }
}

/// Handler trait for streaming RST events — the shared
/// [`rescribe_format_api::Handler`], not a locally declared trait.
/// Implemented automatically for any `FnMut(OwnedEvent)`.
pub use rescribe_format_api::Handler;

/// Block accumulation state for the streaming parser.
enum BlockState {
    Between,
    Accumulating,
    /// Inside a directive body (indented lines after `::`).
    /// Ends when a non-indented non-blank line arrives.
    InDirectiveBody,
}

/// Chunked streaming RST parser that delivers events to a [`Handler`].
///
/// Memory: O(largest block). Split tokens at chunk boundaries are handled
/// correctly — partial lines are buffered until the next `\n`.
///
/// Known limitations in streaming mode:
/// - RST link targets defined later in the document are not resolved.
///   Use [`BatchParser`] + `parse()` for full resolution.
pub struct StreamingParser<H: Handler<OwnedEvent>> {
    handler: H,
    line_buf: Vec<u8>,
    block_lines: Vec<String>,
    state: BlockState,
    /// Set when a blank line arrives while the accumulated block's last
    /// content line is an indented definition body. The flush decision is
    /// deferred until the next line arrives, since — per `parse_definition_list`
    /// in `lib.rs`, which skips blank lines rather than stopping at them — a
    /// blank line between one definition-list item's body and the next
    /// item's term does not end the list.
    pending_blank: bool,
    /// A non-indented, non-directive line seen immediately after a
    /// `pending_blank` is held here, undecided, until the line after it
    /// arrives: indented confirms it's another definition-list term
    /// (`parse_definition_list`'s own one-line `peek_line()` lookahead),
    /// anything else means the block genuinely ended at the blank line and
    /// this line starts a new one.
    pending_term: Option<String>,
    /// Set alongside `pending_blank` when the accumulated block's last
    /// content line is a bullet/numbered list item (`last_line_is_list_item`)
    /// rather than an indented definition body. `parse_bullet_list` /
    /// `parse_numbered_list` (lib.rs) both keep parsing past a blank line
    /// when the line that follows still belongs to the list — another item
    /// at any indentation, or a nested sub-list — so the flush decision is
    /// deferred the same one-line-lookahead way `pending_blank` already
    /// defers it for definition-list bodies, just with a different
    /// confirmation test (see the `pending_blank` handling in `feed_line`).
    pending_list_blank: bool,
    /// Underline-character-to-heading-level assignment, carried forward
    /// across `emit_block()` calls so a later block's heading numbering
    /// stays consistent with an earlier block's — each `emit_block()` call
    /// re-parses its block text in isolation via a fresh `EventIter`, which
    /// would otherwise treat every block's first heading as level 1. See
    /// `EventIter::with_heading_levels`.
    heading_levels: Vec<char>,
}

impl<H: Handler<OwnedEvent>> StreamingParser<H> {
    /// Create a new `StreamingParser` that delivers events to `handler`.
    pub fn new(handler: H) -> Self {
        StreamingParser {
            handler,
            line_buf: Vec::new(),
            block_lines: Vec::new(),
            state: BlockState::Between,
            pending_blank: false,
            pending_term: None,
            pending_list_blank: false,
            heading_levels: Vec::new(),
        }
    }

    /// Feed a chunk of bytes.  May call `handler.handle()` zero or more times.
    pub fn feed(&mut self, chunk: &[u8]) {
        for &byte in chunk {
            if byte == b'\n' {
                if self.line_buf.last() == Some(&b'\r') {
                    self.line_buf.pop();
                }
                let line = String::from_utf8_lossy(&self.line_buf).into_owned();
                self.line_buf.clear();
                self.feed_line(line);
            } else {
                self.line_buf.push(byte);
            }
        }
    }

    fn feed_line(&mut self, line: String) {
        // Inside a directive body: keep accumulating indented/blank lines
        if matches!(self.state, BlockState::InDirectiveBody) {
            let is_blank = line.trim().is_empty();
            let is_indented = line.starts_with(' ') || line.starts_with('\t');
            if is_blank || is_indented {
                self.block_lines.push(line);
                return;
            }
            // Non-indented non-blank: directive body ended
            self.emit_block();
            self.state = BlockState::Between;
            // Fall through to process this line as a new block start
        }

        // A candidate definition-list term is awaiting confirmation from this
        // line: indented confirms the list continues, anything else means the
        // previous block already ended at the blank line before `term`.
        if let Some(term) = self.pending_term.take() {
            let is_blank = line.trim().is_empty();
            let is_indented = (line.starts_with(' ') || line.starts_with('\t')) && !is_blank;
            if is_indented {
                self.block_lines.push(String::new());
                self.block_lines.push(term);
                self.block_lines.push(line);
                self.state = BlockState::Accumulating;
                return;
            }
            self.emit_block();
            self.state = BlockState::Between;
            self.feed_line(term);
            self.feed_line(line);
            return;
        }

        if line.trim().is_empty() {
            if self.pending_blank {
                // Second consecutive blank line: no candidate term ever
                // arrived, so the block genuinely ended at the first blank.
                self.pending_blank = false;
                self.pending_list_blank = false;
                self.emit_block();
                self.state = BlockState::Between;
                return;
            }
            if !self.block_lines.is_empty() && self.last_line_is_list_item() {
                // The block's last content line is a bullet/numbered list
                // item — checked before `last_line_is_indented` below because
                // a nested sub-list item (e.g. "  - Nested item") is *both*
                // indented and a list item, and it's the list-item grammar
                // (any indentation, any marker) that governs continuation
                // here, not the definition-list-body grammar. Defer the
                // flush decision until the next line arrives (see
                // `pending_list_blank`'s doc comment).
                self.pending_blank = true;
                self.pending_list_blank = true;
                return;
            }
            if !self.block_lines.is_empty() && self.last_line_is_indented() {
                // The block's last content line is an indented definition
                // body — defer the flush decision until the next non-blank
                // line arrives (see `pending_blank`'s doc comment).
                self.pending_blank = true;
                return;
            }
            if !self.block_lines.is_empty() {
                self.emit_block();
            }
            self.state = BlockState::Between;
            return;
        }

        if self.pending_blank && self.pending_list_blank {
            self.pending_blank = false;
            self.pending_list_blank = false;
            let trimmed = line.trim();
            let is_indented = line.starts_with(' ') || line.starts_with('\t');
            if is_indented || Self::is_list_item_line(trimmed) {
                // The list continues: a nested sub-list line (indented) or
                // another item (any indentation, any marker — the merged
                // text is re-parsed as a whole by `emit_block`, so the real
                // recursive-descent grammar in lib.rs makes the final call
                // on how many lists/items this actually is).
                self.block_lines.push(String::new());
                self.block_lines.push(line);
                self.state = BlockState::Accumulating;
                return;
            }
            // Plain, non-indented, non-list-item line: the list genuinely
            // ended at the blank line.
            self.emit_block();
            self.state = BlockState::Between;
            // Fall through to process `line` as the start of a new block.
        } else if self.pending_blank {
            self.pending_blank = false;
            let trimmed = line.trim();
            let is_indented = line.starts_with(' ') || line.starts_with('\t');
            if !is_indented && !trimmed.starts_with(".. ") {
                // Candidate term — hold it until the line after it confirms
                // or denies an indented definition body.
                self.pending_term = Some(line);
                return;
            }
            // Indented, or a directive/comment: not a definition-list term
            // candidate, so the block genuinely ended at the blank line.
            self.emit_block();
            self.state = BlockState::Between;
            // Fall through to process `line` as the start of a new block.
        }

        let trimmed = line.trim();
        // Lines that introduce an indented body (directives, literal blocks)
        let introduces_body = trimmed.ends_with("::") || trimmed.starts_with(".. ");
        self.state = if introduces_body {
            BlockState::InDirectiveBody
        } else {
            BlockState::Accumulating
        };
        self.block_lines.push(line);
    }

    /// True if the last line in `block_lines` is a non-blank, indented line
    /// (an RST definition body, block quote, or directive-body continuation).
    fn last_line_is_indented(&self) -> bool {
        self.block_lines
            .last()
            .is_some_and(|l| !l.trim().is_empty() && (l.starts_with(' ') || l.starts_with('\t')))
    }

    /// True if the last line in `block_lines` is a bullet or numbered list
    /// item, at any indentation.
    fn last_line_is_list_item(&self) -> bool {
        self.block_lines
            .last()
            .is_some_and(|l| Self::is_list_item_line(l.trim()))
    }

    /// True if `trimmed` (already whitespace-trimmed) is a bullet or
    /// numbered list item marker — mirrors the marker patterns
    /// `parse_bullet_list` / `parse_numbered_list` (lib.rs) match on.
    fn is_list_item_line(trimmed: &str) -> bool {
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ") {
            return true;
        }
        if let Some(idx) = trimmed.find(". ") {
            let prefix = &trimmed[..idx];
            if prefix.chars().all(|c| c.is_ascii_digit()) || prefix == "#" {
                return true;
            }
        }
        false
    }

    fn emit_block(&mut self) {
        if self.block_lines.is_empty() {
            return;
        }
        let text = self.block_lines.join("\n");
        self.block_lines.clear();
        let mut iter =
            crate::EventIter::with_heading_levels(&text, std::mem::take(&mut self.heading_levels));
        for event in iter.by_ref() {
            self.handler.handle(event.into_owned());
        }
        self.heading_levels = iter.heading_levels().to_vec();
    }

    /// Flush any remaining input and deliver final events.
    pub fn finish(mut self) {
        if !self.line_buf.is_empty() {
            if self.line_buf.last() == Some(&b'\r') {
                self.line_buf.pop();
            }
            let line = String::from_utf8_lossy(&self.line_buf).into_owned();
            self.feed_line(line);
        }
        if let Some(term) = self.pending_term.take() {
            // EOF arrived instead of a confirming line — `term` has no
            // indented continuation (matching `parse_definition_list`'s own
            // `peek_line()` returning `None` at EOF), so it does not extend
            // the previous block. Flush that block, then process `term` as
            // the start of its own trailing block.
            self.emit_block();
            self.state = BlockState::Between;
            self.feed_line(term);
        }
        self.pending_blank = false;
        self.pending_list_blank = false;
        self.emit_block();
    }
}

/// Chunk-driven RST parser that delivers events to a callback on finish.
///
/// Prefer [`StreamingParser`] for new code; `BatchSink` is kept for
/// backwards compatibility.
pub struct BatchSink<F: FnMut(OwnedEvent)> {
    buf: Vec<u8>,
    callback: F,
}

impl<F: FnMut(OwnedEvent)> BatchSink<F> {
    pub fn new(callback: F) -> Self {
        BatchSink {
            buf: Vec::new(),
            callback,
        }
    }

    /// Feed a chunk of input bytes.
    pub fn feed(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);
    }

    /// Finish parsing and deliver all events to the callback.
    pub fn finish(mut self) {
        let s = String::from_utf8_lossy(&self.buf);
        for event in crate::EventIter::new(&s) {
            (self.callback)(event.into_owned());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::OwnedEvent;

    #[test]
    fn test_batch_parser_basic() {
        let mut p = BatchParser::new();
        p.feed(b"Section\n=======\n\n");
        p.feed(b"A paragraph.\n");
        let doc = p.finish().unwrap();
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_batch_parser_split_chunks() {
        let mut p = BatchParser::new();
        for b in b"Title\n=====\n\nContent here.\n" {
            p.feed(std::slice::from_ref(b));
        }
        let doc = p.finish().unwrap();
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_streaming_parser_events() {
        let mut evs = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        p.feed(b"Section\n=======\n\n");
        p.feed(b"A paragraph.\n");
        p.finish();
        assert!(
            evs.iter()
                .any(|e| matches!(e, OwnedEvent::StartHeading { level: 1 }))
        );
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartParagraph)));
    }

    #[test]
    fn test_streaming_parser_split_chunks() {
        let mut evs: Vec<OwnedEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        for b in b"Title\n=====\n\nContent.\n" {
            p.feed(std::slice::from_ref(b));
        }
        p.finish();
        assert!(
            evs.iter()
                .any(|e| matches!(e, OwnedEvent::StartHeading { .. }))
        );
    }

    #[test]
    fn test_streaming_matches_bulk() {
        let input = b"Section\n=======\n\nParagraph one.\n\nParagraph two.\n";

        let bulk: Vec<OwnedEvent> = {
            let s = String::from_utf8_lossy(input);
            crate::events(&s).map(|e| e.into_owned()).collect()
        };

        let mut streamed: Vec<OwnedEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| streamed.push(ev));
        for chunk in input.chunks(7) {
            p.feed(chunk);
        }
        p.finish();

        assert_eq!(bulk, streamed);
    }

    #[test]
    fn test_batch_sink_events() {
        let mut events = Vec::new();
        let mut sink = BatchSink::new(|ev| events.push(ev));
        sink.feed(b"Section\n=======\n\n");
        sink.feed(b"A paragraph.\n");
        sink.finish();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, OwnedEvent::StartHeading { level: 1 }))
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, OwnedEvent::StartParagraph))
        );
    }

    /// Feed `input` one byte at a time — the most awkward possible split:
    /// every UTF-8 continuation byte, every keyword, every line ending, and
    /// every blank-line boundary is torn apart across `feed()` calls.
    fn streamed_events(input: &[u8]) -> Vec<OwnedEvent> {
        let mut streamed = Vec::new();
        let mut p = StreamingParser::new(|ev| streamed.push(ev));
        for byte in input {
            p.feed(std::slice::from_ref(byte));
        }
        p.finish();
        streamed
    }

    fn bulk_events(input: &[u8]) -> Vec<OwnedEvent> {
        let s = String::from_utf8_lossy(input);
        crate::events(&s).map(|e| e.into_owned()).collect()
    }

    #[test]
    fn test_streaming_split_mid_directive_keyword() {
        // Split lands inside "code-block" and inside the "::" marker.
        let input = b".. code-block:: rust\n\n   let x = 1;\n   let y = 2;\n";
        assert_eq!(bulk_events(input), streamed_events(input));
    }

    #[test]
    fn test_streaming_split_mid_heading_underline() {
        // One byte at a time tears the underline run apart from the title line.
        let input = b"A Heading\n=========\n\nBody text.\n";
        assert_eq!(bulk_events(input), streamed_events(input));
    }

    #[test]
    fn test_streaming_split_mid_list_and_blank_line() {
        let input = b"- item one\n- item two\n\nParagraph after list.\n";
        assert_eq!(bulk_events(input), streamed_events(input));
    }

    #[test]
    fn test_streaming_split_mid_footnote_continuation() {
        // Splits inside "[1]_", inside the ".. [1]" marker, and inside the
        // indented continuation line of the footnote body.
        let input = b"See [1]_ here.\n\n.. [1] First line\n   second line.\n";
        assert_eq!(bulk_events(input), streamed_events(input));
    }

    #[test]
    fn test_streaming_split_mid_utf8_char() {
        // "café" — the 'é' is 2 UTF-8 bytes, split across separate feed() calls.
        let input = "café society\n\nAnother caf\u{e9} paragraph.\n".as_bytes();
        assert_eq!(bulk_events(input), streamed_events(input));
    }

    #[test]
    fn test_streaming_split_mid_table_border() {
        let input = b"+--------+--------+\n| A      | B      |\n+========+========+\n| Cell 1 | Cell 2 |\n+--------+--------+\n";
        assert_eq!(bulk_events(input), streamed_events(input));
    }
}
