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
    pub fn finish(self) -> Result<RstDoc, RstError> {
        let s = String::from_utf8_lossy(&self.buf);
        crate::parse(&s)
    }
}

/// Handler trait for streaming RST events.
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
pub struct StreamingParser<H: Handler> {
    handler: H,
    line_buf: Vec<u8>,
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

        if line.trim().is_empty() {
            if !self.block_lines.is_empty() {
                self.emit_block();
            }
            self.state = BlockState::Between;
            return;
        }

        let trimmed = line.trim();
        // Lines that introduce an indented body (directives, literal blocks)
        let introduces_body = trimmed.ends_with("::")
            || trimmed.starts_with(".. ");
        self.state = if introduces_body { BlockState::InDirectiveBody } else { BlockState::Accumulating };
        self.block_lines.push(line);
    }

    fn emit_block(&mut self) {
        if self.block_lines.is_empty() {
            return;
        }
        let text = self.block_lines.join("\n");
        self.block_lines.clear();
        for event in crate::events(&text) {
            self.handler.handle(event.into_owned());
        }
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
        BatchSink { buf: Vec::new(), callback }
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
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartHeading { level: 1 })));
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
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartHeading { .. })));
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
        assert!(events.iter().any(|e| matches!(e, OwnedEvent::StartHeading { level: 1 })));
        assert!(events.iter().any(|e| matches!(e, OwnedEvent::StartParagraph)));
    }
}
