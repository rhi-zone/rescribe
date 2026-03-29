//! Chunk-driven (batch) Textile parser.
//!
//! Feed input in arbitrarily-sized chunks with [`BatchParser::feed`], then
//! call [`BatchParser::finish`] to get the full AST.
//!
//! For event-driven use, see [`StreamingParser`] and [`Handler`].
//!
//! # Memory model
//!
//! [`BatchParser`] buffers all input until `finish()` and is O(full input).
//! Use it when you need the complete AST.
//!
//! [`StreamingParser`] delivers events to a [`Handler`] on `finish()`.
//! It also buffers all input (Textile parsing requires full input context),
//! so memory is likewise O(full input).
//!
//! # Example — AST style
//! ```no_run
//! use textile_fmt::batch::BatchParser;
//!
//! let mut p = BatchParser::new();
//! p.feed(b"h1. Hello\n\n");
//! p.feed(b"A paragraph.\n");
//! let (doc, diags) = p.finish();
//! ```
//!
//! # Example — event callback style
//! ```no_run
//! use textile_fmt::batch::{StreamingParser, Handler};
//! use textile_fmt::TextileEvent;
//!
//! let mut events = Vec::new();
//! let mut p = StreamingParser::new(|ev: TextileEvent| events.push(ev));
//! p.feed(b"h1. Hello\n\n");
//! p.feed(b"A paragraph.\n");
//! p.finish();
//! ```

use crate::ast::{Diagnostic, TextileDoc};
use crate::events::TextileEvent;

/// Chunk-driven Textile parser that returns the full AST on finish.
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
    pub fn finish(self) -> (TextileDoc, Vec<Diagnostic>) {
        let s = String::from_utf8_lossy(&self.buf);
        crate::parse::parse(&s)
    }
}

/// Handler trait for streaming Textile events.
///
/// Implemented automatically for any `FnMut(TextileEvent)`.
pub trait Handler {
    fn handle(&mut self, event: TextileEvent);
}

impl<F: FnMut(TextileEvent)> Handler for F {
    fn handle(&mut self, event: TextileEvent) {
        self(event);
    }
}

/// Chunk-driven Textile parser that delivers events to a [`Handler`] on finish.
///
/// Buffers all input until `finish()` (Textile parsing requires full input context).
pub struct StreamingParser<H: Handler> {
    buf: Vec<u8>,
    handler: H,
}

impl<H: Handler> StreamingParser<H> {
    /// Create a new `StreamingParser` that delivers events to `handler`.
    pub fn new(handler: H) -> Self {
        StreamingParser { buf: Vec::new(), handler }
    }

    /// Feed a chunk of bytes.
    pub fn feed(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);
    }

    /// Finish parsing and deliver all events to the handler.
    pub fn finish(mut self) {
        let s = String::from_utf8_lossy(&self.buf);
        for event in crate::events::events(&s) {
            self.handler.handle(event);
        }
    }
}

/// Chunk-driven Textile parser that delivers events to a callback on finish.
///
/// Prefer [`StreamingParser`] for new code; `BatchSink` is kept for
/// API symmetry with other format crates.
pub struct BatchSink<F: FnMut(TextileEvent)> {
    buf: Vec<u8>,
    callback: F,
}

impl<F: FnMut(TextileEvent)> BatchSink<F> {
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
        for event in crate::events::events(&s) {
            (self.callback)(event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_parser_basic() {
        let mut p = BatchParser::new();
        p.feed(b"h1. Hello\n\n");
        p.feed(b"A paragraph.\n");
        let (doc, diags) = p.finish();
        assert!(diags.is_empty());
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_batch_parser_split_chunks() {
        let mut p = BatchParser::new();
        for b in b"h1. Title\n\nContent here.\n" {
            p.feed(std::slice::from_ref(b));
        }
        let (doc, _) = p.finish();
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_batch_parser_matches_parse() {
        let input = "h1. Title\n\nA paragraph.\n";
        let (expected_doc, _) = crate::parse::parse(input);

        let mut p = BatchParser::new();
        p.feed(input.as_bytes());
        let (actual_doc, _) = p.finish();

        assert_eq!(expected_doc.blocks.len(), actual_doc.blocks.len());
    }

    #[test]
    fn test_streaming_parser_events() {
        let mut evs = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        p.feed(b"h1. Hello\n\n");
        p.feed(b"A paragraph.\n");
        p.finish();
        assert!(evs.iter().any(|e| matches!(e, TextileEvent::StartHeading { level: 1, .. })));
        assert!(evs.iter().any(|e| matches!(e, TextileEvent::StartParagraph { .. })));
    }

    #[test]
    fn test_streaming_parser_split_chunks() {
        let mut evs: Vec<TextileEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        for b in b"h1. Title\n\nContent.\n" {
            p.feed(std::slice::from_ref(b));
        }
        p.finish();
        assert!(evs.iter().any(|e| matches!(e, TextileEvent::StartHeading { .. })));
        assert!(evs.iter().any(|e| matches!(e, TextileEvent::StartParagraph { .. })));
    }

    #[test]
    fn test_streaming_matches_bulk() {
        let input = b"h1. Heading\n\nParagraph one.\n\nParagraph two.\n";

        let bulk: Vec<TextileEvent> = {
            let s = String::from_utf8_lossy(input);
            crate::events::events(&s).collect()
        };

        let mut streamed: Vec<TextileEvent> = Vec::new();
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
        sink.feed(b"h1. Hello\n\n");
        sink.feed(b"A paragraph.\n");
        sink.finish();
        assert!(events.iter().any(|e| matches!(e, TextileEvent::StartHeading { level: 1, .. })));
        assert!(events.iter().any(|e| matches!(e, TextileEvent::StartParagraph { .. })));
    }
}
