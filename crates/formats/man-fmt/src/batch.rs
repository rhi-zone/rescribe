//! Chunk-driven (batch) man page parser.
//!
//! Feed input in arbitrarily-sized chunks with [`StreamingParser::feed`], then
//! call [`StreamingParser::finish`] to deliver all events to the handler.
//!
//! # Memory model
//!
//! [`StreamingParser`] processes input in logical blocks (content between blank
//! lines, or delimited by macro markers). Memory usage is O(largest block in
//! the document). The full document is never held in memory simultaneously.
//!
//! [`BatchParser`] buffers all input until `finish()` and is O(full input).
//! Use it when you need the complete AST.
//!
//! # Example — AST style
//! ```no_run
//! use man_fmt::batch::BatchParser;
//!
//! let mut p = BatchParser::new();
//! p.feed(b".TH TEST 1\n");
//! p.feed(b".SH NAME\n");
//! p.feed(b"test - a test program\n");
//! let (doc, diags) = p.finish();
//! ```
//!
//! # Example — event callback style
//! ```no_run
//! use man_fmt::batch::{StreamingParser, Handler};
//! use man_fmt::OwnedManEvent;
//!
//! let mut events = Vec::new();
//! let mut p = StreamingParser::new(|ev: OwnedManEvent| events.push(ev));
//! p.feed(b".SH NAME\n");
//! p.feed(b"test - a test\n");
//! p.finish();
//! ```

use crate::ast::{Diagnostic, ManDoc};
use crate::events::OwnedManEvent;

/// Chunk-driven man page parser that returns the full AST on finish.
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
    pub fn finish(self) -> (ManDoc, Vec<Diagnostic>) {
        let s = String::from_utf8_lossy(&self.buf);
        crate::parse::parse(&s)
    }
}

/// Handler trait for streaming man page events.
///
/// Implemented automatically for any `FnMut(OwnedManEvent)`.
pub trait Handler {
    fn handle(&mut self, event: OwnedManEvent);
}

impl<F: FnMut(OwnedManEvent)> Handler for F {
    fn handle(&mut self, event: OwnedManEvent) {
        self(event);
    }
}

/// Block accumulation state for the streaming parser.
enum BlockState {
    /// Between blocks — waiting for the first non-blank line.
    Between,
    /// Accumulating normal content.
    Accumulating,
    /// Inside a preformatted block (.nf ... .fi or .EX ... .EE).
    InPreformatted { end_macro: &'static str },
}

/// Chunked streaming man page parser that delivers events to a [`Handler`].
///
/// Memory: O(largest block). See the [module-level docs](self) for details.
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
        // ── Inside a preformatted block ──────────────────────────────────────
        if let BlockState::InPreformatted { end_macro } = &self.state {
            let end = *end_macro;
            self.block_lines.push(line.clone());
            if line.trim() == end {
                self.emit_block();
                self.state = BlockState::Between;
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

        // ── Preformatted block start ─────────────────────────────────────────
        let trimmed = line.trim();
        if trimmed == ".nf" {
            if !self.block_lines.is_empty() {
                self.emit_block();
            }
            self.state = BlockState::InPreformatted { end_macro: ".fi" };
            self.block_lines.push(line);
            return;
        }
        if trimmed == ".EX" {
            if !self.block_lines.is_empty() {
                self.emit_block();
            }
            self.state = BlockState::InPreformatted { end_macro: ".EE" };
            self.block_lines.push(line);
            return;
        }

        // ── Macro line that starts a new block ───────────────────────────────
        if line.starts_with('.') || line.starts_with("'\\\"") {
            // If we have accumulated content, flush it first
            if !self.block_lines.is_empty() && matches!(self.state, BlockState::Accumulating) {
                // Check if the accumulated content is plain text (not a macro)
                // and the new line is a macro — that means the paragraph ended.
                self.emit_block();
            }
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

/// Chunk-driven man page parser that delivers events to a callback on finish.
///
/// Prefer [`StreamingParser`] for new code; `BatchSink` is kept for
/// backwards compatibility.
pub struct BatchSink<F: FnMut(OwnedManEvent)> {
    buf: Vec<u8>,
    callback: F,
}

impl<F: FnMut(OwnedManEvent)> BatchSink<F> {
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
    use crate::events::OwnedManEvent;

    #[test]
    fn test_batch_parser_basic() {
        let mut p = BatchParser::new();
        p.feed(b".SH NAME\n");
        p.feed(b"test - a test program\n");
        let (doc, diags) = p.finish();
        assert!(diags.is_empty(), "unexpected diagnostics: {:?}", diags);
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_batch_parser_split_chunks() {
        let mut p = BatchParser::new();
        for b in b".SH NAME\ntest\n" {
            p.feed(std::slice::from_ref(b));
        }
        let (doc, _) = p.finish();
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_streaming_parser_events() {
        let mut evs = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        p.feed(b".SH NAME\n\n");
        p.feed(b"test program\n");
        p.finish();
        assert!(evs
            .iter()
            .any(|e| matches!(e, OwnedManEvent::StartHeading { level: 2 })));
        assert!(evs
            .iter()
            .any(|e| matches!(e, OwnedManEvent::StartParagraph)));
    }

    #[test]
    fn test_batch_sink_events() {
        let mut events = Vec::new();
        let mut sink = BatchSink::new(|ev| events.push(ev));
        sink.feed(b".SH NAME\n");
        sink.feed(b"test program\n");
        sink.finish();
        assert!(events
            .iter()
            .any(|e| matches!(e, OwnedManEvent::StartHeading { .. })));
        assert!(events
            .iter()
            .any(|e| matches!(e, OwnedManEvent::StartParagraph)));
    }
}
