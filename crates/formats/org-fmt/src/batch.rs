//! Chunk-driven (batch) Org-mode parser.
//!
//! Feed input in arbitrarily-sized chunks with [`StreamingParser::feed`], then
//! call [`StreamingParser::finish`] to deliver all events to the handler.
//!
//! # Memory model
//!
//! [`StreamingParser`] processes input in logical blocks (content between blank
//! lines, or delimited by `#+BEGIN_*`…`#+END_*` markers). Memory usage is
//! O(largest block in the document), which for typical documents means
//! O(longest paragraph or code block). The full document is never held in
//! memory simultaneously.
//!
//! [`BatchParser`] buffers all input until `finish()` and is O(full input).
//! Use it when you need the complete AST.
//!
//! # Known streaming limitations
//!
//! - **Loose lists** (list items separated by blank lines) are emitted as
//!   separate single-item lists rather than one multi-item list.
//! - **Drawers containing blank lines** may be split and emitted incorrectly.
//!   Standard `:PROPERTIES:` drawers do not contain blank lines and are fine.
//! - Split tokens at chunk boundaries (e.g. a keyword split across two
//!   `feed()` calls) are handled correctly; the partial bytes are buffered
//!   until the line is complete.
//!
//! # Example — AST style
//! ```no_run
//! use org_fmt::batch::BatchParser;
//!
//! let mut p = BatchParser::new();
//! p.feed(b"* Hello\n\n");
//! p.feed(b"A paragraph.\n");
//! let (doc, diags) = p.finish();
//! ```
//!
//! # Example — event callback style
//! ```no_run
//! use org_fmt::batch::{StreamingParser, Handler};
//! use org_fmt::OwnedEvent;
//!
//! let mut events = Vec::new();
//! let mut p = StreamingParser::new(|ev: OwnedEvent| events.push(ev));
//! p.feed(b"* Hello\n\n");
//! p.feed(b"A paragraph.\n");
//! p.finish();
//! ```

use crate::ast::{Diagnostic, OrgDoc};
use crate::events::OwnedEvent;

/// Chunk-driven Org-mode parser that returns the full AST on finish.
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
    pub fn finish(self) -> (OrgDoc, Vec<Diagnostic>) {
        let s = String::from_utf8_lossy(&self.buf);
        crate::parse::parse(&s)
    }
}

/// Handler trait for streaming Org-mode events.
///
/// Implemented automatically for any `FnMut(OwnedEvent)`.
pub trait Handler {
    fn handle(&mut self, event: OwnedEvent);
}

impl<F: FnMut(OwnedEvent)> Handler for F {
    fn handle(&mut self, event: OwnedEvent) {
        self(event);
    }
}

/// Block accumulation state for the streaming parser.
enum BlockState {
    /// Between blocks — waiting for the first non-blank line.
    Between,
    /// Accumulating normal content (paragraph, heading, list, table, etc.).
    /// Ends on the next blank line or special-block start.
    Accumulating,
    /// Inside a `#+BEGIN_xxx`…`#+END_xxx` special block.
    /// `begin_marker`/`end_marker` are the uppercase `#+BEGIN_XXX`/`#+END_XXX`
    /// prefixes for this block's keyword; `depth` counts still-open *nested*
    /// same-keyword blocks (e.g. a `#+BEGIN_QUOTE` inside a `#+BEGIN_QUOTE`),
    /// mirroring the nesting counter `parse::EventIter::parse_block` tracks
    /// (parse.rs:521) so a nested block's `#+END_` doesn't close the outer
    /// block early.
    InSpecialBlock {
        begin_marker: String,
        end_marker: String,
        depth: usize,
    },
}

/// Chunked streaming Org-mode parser that delivers events to a [`Handler`].
///
/// Memory: O(largest block). See the [module-level docs](self) for details
/// and known limitations.
pub struct StreamingParser<H: Handler> {
    handler: H,
    /// Bytes of the current incomplete line (not yet terminated by `\n`).
    line_buf: Vec<u8>,
    /// Complete lines of the block currently being accumulated.
    block_lines: Vec<String>,
    state: BlockState,
}

impl<H: Handler> StreamingParser<H> {
    /// Create a new `StreamingParser` that delivers events to `handler`.
    pub fn new(handler: H) -> Self {
        StreamingParser {
            handler,
            line_buf: Vec::new(),
            block_lines: Vec::new(),
            state: BlockState::Between,
        }
    }

    /// Feed a chunk of bytes.  May call `handler.handle()` zero or more times.
    pub fn feed(&mut self, chunk: &[u8]) {
        for &byte in chunk {
            if byte == b'\n' {
                // Strip trailing \r (Windows line endings)
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
        // ── Inside a special block ──────────────────────────────────────────
        // Compute is_begin/is_end first (borrows self.state), then drop the
        // borrow before mutating self.block_lines / self.state. Matching is
        // starts_with on the *untrimmed*, uppercased line — mirroring
        // parse::EventIter::parse_block (parse.rs:528-539), which never trims
        // either — so nesting depth tracking stays consistent with the
        // BEGIN_/END_ detection below.
        let special: Option<(bool, bool, usize)> = if let BlockState::InSpecialBlock {
            ref begin_marker,
            ref end_marker,
            depth,
        } = self.state
        {
            let line_upper = line.to_uppercase();
            let is_begin = line_upper.starts_with(begin_marker.as_str());
            let is_end = line_upper.starts_with(end_marker.as_str());
            Some((is_begin, is_end, depth))
        } else {
            None
        };

        if let Some((is_begin, is_end, depth)) = special {
            self.block_lines.push(line);
            if is_end && depth == 0 {
                self.emit_block();
                self.state = BlockState::Between;
            } else if let BlockState::InSpecialBlock { depth, .. } = &mut self.state {
                // Track nesting depth of same-keyword BEGIN/END pairs so a
                // nested block's END doesn't close the outer block early.
                if is_begin {
                    *depth += 1;
                } else if is_end {
                    *depth = depth.saturating_sub(1);
                }
            }
            return;
        }

        // ── Blank line: end of current block ───────────────────────────────
        if line.trim().is_empty() {
            if !self.block_lines.is_empty() {
                self.emit_block();
            }
            self.state = BlockState::Between;
            return;
        }

        // ── Special block start ─────────────────────────────────────────────
        // Match on the untrimmed line, like parse::EventIter::parse_next_block
        // (parse.rs:252: `line.to_uppercase().starts_with("#+BEGIN_")`, no
        // trim). An indented `#+BEGIN_` (e.g. inside a list item) is therefore
        // *not* treated as a block start here either — it falls through to
        // the "Regular line" branch below and stays attached to whatever
        // block is accumulating, matching parse.rs's own documented
        // limitation (fixtures/org/integration-list-code: the code fence
        // inside a list item is continuation text, not a nested code block).
        let line_upper = line.to_uppercase();
        if line_upper.starts_with("#+BEGIN_") {
            // A run of affiliated-keyword lines (e.g. `#+NAME:`) immediately
            // preceding this BEGIN_ must stay attached to the same block so a
            // single events() call on the combined text can thread
            // `pending_name` to the block the way
            // parse::EventIter::parse_next_block does (parse.rs:208-220,
            // 262-274) — flushing them first would re-parse `#+NAME:` alone,
            // dropping the name.
            let all_affiliated = !self.block_lines.is_empty()
                && self.block_lines.iter().all(|l| {
                    let u = l.to_uppercase();
                    u.starts_with("#+") && !u.starts_with("#+BEGIN")
                });
            if !self.block_lines.is_empty() && !all_affiliated {
                self.emit_block();
            }
            let rest = line_upper.strip_prefix("#+BEGIN_").unwrap_or("");
            let keyword = rest.split_whitespace().next().unwrap_or("");
            let begin_marker = format!("#+BEGIN_{}", keyword);
            let end_marker = format!("#+END_{}", keyword);
            self.state = BlockState::InSpecialBlock {
                begin_marker,
                end_marker,
                depth: 0,
            };
            self.block_lines.push(line);
            return;
        }

        // ── Regular line ─────────────────────────────────────────────────────
        self.state = BlockState::Accumulating;
        self.block_lines.push(line);
    }

    /// Parse the accumulated block lines and deliver events to the handler.
    fn emit_block(&mut self) {
        if self.block_lines.is_empty() {
            return;
        }
        let text = self.block_lines.join("\n");
        self.block_lines.clear();
        // EventIter borrows from `text`; into_owned() converts each event to
        // owned before the borrow ends.  `text` is dropped at end of function.
        for event in crate::events::events(&text) {
            self.handler.handle(event.into_owned());
        }
    }

    /// Flush any remaining input and deliver final events.
    pub fn finish(mut self) {
        // Flush any bytes that did not end with \n
        if !self.line_buf.is_empty() {
            if self.line_buf.last() == Some(&b'\r') {
                self.line_buf.pop();
            }
            let line = String::from_utf8_lossy(&self.line_buf).into_owned();
            self.feed_line(line);
        }
        // Flush any pending block
        self.emit_block();
    }
}

/// Chunk-driven Org-mode parser that delivers events to a callback on finish.
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
        for event in crate::events::events(&s) {
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
        p.feed(b"* Hello\n\n");
        p.feed(b"A paragraph.\n");
        let (doc, diags) = p.finish();
        assert!(diags.is_empty());
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_batch_parser_split_chunks() {
        let mut p = BatchParser::new();
        for b in b"* Title\n\nContent here.\n" {
            p.feed(std::slice::from_ref(b));
        }
        let (doc, _) = p.finish();
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_streaming_parser_events() {
        let mut evs = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        p.feed(b"* Hello\n\n");
        p.feed(b"A paragraph.\n");
        p.finish();
        assert!(
            evs.iter()
                .any(|e| matches!(e, OwnedEvent::StartHeading { level: 1, .. }))
        );
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartParagraph)));
    }

    #[test]
    fn test_streaming_parser_split_chunks() {
        let mut evs: Vec<OwnedEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        // Feed byte-by-byte to exercise split-token handling
        for b in b"* Title\n\nContent.\n" {
            p.feed(std::slice::from_ref(b));
        }
        p.finish();
        assert!(
            evs.iter()
                .any(|e| matches!(e, OwnedEvent::StartHeading { .. }))
        );
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartParagraph)));
    }

    #[test]
    fn test_streaming_parser_code_block() {
        let mut evs: Vec<OwnedEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        p.feed(b"#+BEGIN_SRC rust\nlet x = 1;\n#+END_SRC\n");
        p.finish();
        assert!(
            evs.iter()
                .any(|e| matches!(e, OwnedEvent::CodeBlock { .. }))
        );
    }

    #[test]
    fn test_streaming_parser_code_block_with_blank_lines() {
        // Code blocks can contain blank lines; streaming parser must not split
        // them at the blank.
        let mut evs: Vec<OwnedEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        p.feed(b"#+BEGIN_SRC rust\nlet x = 1;\n\nlet y = 2;\n#+END_SRC\n");
        p.finish();
        let code_blocks: Vec<_> = evs
            .iter()
            .filter(|e| matches!(e, OwnedEvent::CodeBlock { .. }))
            .collect();
        assert_eq!(
            code_blocks.len(),
            1,
            "should be exactly one code block event"
        );
        if let OwnedEvent::CodeBlock { content, .. } = &code_blocks[0] {
            assert!(content.contains("let x = 1;"));
            assert!(content.contains("let y = 2;"));
        }
    }

    #[test]
    fn test_streaming_matches_bulk() {
        // StreamingParser must emit the same events as processing all at once.
        let input = b"* Heading\n\nParagraph one.\n\nParagraph two.\n";

        let bulk: Vec<OwnedEvent> = {
            let s = String::from_utf8_lossy(input);
            crate::events::events(&s).map(|e| e.into_owned()).collect()
        };

        let mut streamed: Vec<OwnedEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| streamed.push(ev));
        // Feed in 7-byte chunks to exercise boundary conditions
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
        sink.feed(b"* Hello\n\n");
        sink.feed(b"A paragraph.\n");
        sink.finish();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, OwnedEvent::StartHeading { level: 1, .. }))
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, OwnedEvent::StartParagraph))
        );
    }
}
