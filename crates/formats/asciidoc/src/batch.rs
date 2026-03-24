//! Chunk-driven (batch) AsciiDoc parser.
//!
//! Feed input in arbitrarily-sized chunks with [`StreamingParser::feed`], then
//! call [`StreamingParser::finish`] to deliver all events to the handler.
//!
//! # Note
//!
//! This implementation buffers all input until `finish()`. True incremental
//! chunked streaming will be added in a future version once the parser is
//! restructured as a true state machine. For large-file use cases, prefer
//! loading the full input and using [`events`](crate::events) directly.
//!
//! # Example — AST style
//! ```no_run
//! use asciidoc::batch::BatchParser;
//!
//! let mut p = BatchParser::new();
//! p.feed(b"== Hello\n\n");
//! p.feed(b"A paragraph.\n");
//! let (doc, diags) = p.finish();
//! ```
//!
//! # Example — event callback style
//! ```no_run
//! use asciidoc::batch::{StreamingParser, Handler};
//! use asciidoc::OwnedEvent;
//!
//! let mut events = Vec::new();
//! let mut p = StreamingParser::new(|ev: OwnedEvent| events.push(ev));
//! p.feed(b"== Hello\n\n");
//! p.feed(b"A paragraph.\n");
//! p.finish();
//! ```

use crate::ast::{AsciiDoc, Diagnostic};
use crate::events::OwnedEvent;

/// Chunk-driven AsciiDoc parser that returns the full AST on finish.
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
    pub fn finish(self) -> (AsciiDoc, Vec<Diagnostic>) {
        let s = String::from_utf8_lossy(&self.buf);
        crate::parse::parse(&s)
    }
}

/// Handler trait for streaming AsciiDoc events.
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

/// Chunked streaming AsciiDoc parser that delivers events to a [`Handler`].
///
/// # Note
///
/// This implementation buffers all input until [`finish`](StreamingParser::finish).
/// True incremental chunked streaming will be added in a future version.
pub struct StreamingParser<H: Handler> {
    buf: Vec<u8>,
    handler: H,
}

impl<H: Handler> StreamingParser<H> {
    /// Create a new `StreamingParser` that delivers events to `handler`.
    pub fn new(handler: H) -> Self {
        StreamingParser { buf: Vec::new(), handler }
    }

    /// Append a chunk of bytes to the internal buffer.
    pub fn feed(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);
    }

    /// Parse all buffered input and deliver events to the handler.
    pub fn finish(mut self) {
        let s = String::from_utf8_lossy(&self.buf);
        for event in crate::events(&s) {
            self.handler.handle(event.into_owned());
        }
    }
}

/// Chunk-driven AsciiDoc parser that delivers events to a callback on finish.
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
        for event in crate::events::EventIter::new(&s) {
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
        p.feed(b"== Hello\n\n");
        p.feed(b"A paragraph.\n");
        let (doc, diags) = p.finish();
        assert!(diags.is_empty());
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_batch_parser_split_chunks() {
        let mut p = BatchParser::new();
        for b in b"== Title\n\nContent here.\n" {
            p.feed(std::slice::from_ref(b));
        }
        let (doc, _) = p.finish();
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_streaming_parser_events() {
        let mut evs = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        p.feed(b"== Hello\n\n");
        p.feed(b"A paragraph.\n");
        p.finish();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartHeading { level: 2, .. })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartParagraph { .. })));
    }

    #[test]
    fn test_streaming_parser_split_chunks() {
        let mut evs: Vec<OwnedEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        for b in b"== Title\n\nContent.\n" {
            p.feed(std::slice::from_ref(b));
        }
        p.finish();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartHeading { .. })));
    }

    #[test]
    fn test_batch_sink_events() {
        let mut events = Vec::new();
        let mut sink = BatchSink::new(|ev| events.push(ev));
        sink.feed(b"== Hello\n\n");
        sink.feed(b"A paragraph.\n");
        sink.finish();
        assert!(events.iter().any(|e| matches!(e, OwnedEvent::StartHeading { level: 2, .. })));
        assert!(events.iter().any(|e| matches!(e, OwnedEvent::StartParagraph { .. })));
    }
}
