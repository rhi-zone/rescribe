//! Chunk-driven (batch) Muse parser.
//!
//! Feed input in arbitrarily-sized chunks with [`StreamingParser::feed`], then
//! call [`StreamingParser::finish`] to deliver all events to the handler.
//!
//! # Memory model
//!
//! [`BatchParser`] buffers all input until `finish()` and is O(full input).
//! Use it when you need the complete AST.
//!
//! [`StreamingParser`] also buffers all input (Muse's block-level structure
//! makes true incremental parsing difficult without a dedicated state machine),
//! then delivers events via the handler on `finish()`.
//!
//! # Example — AST style
//! ```no_run
//! use muse_fmt::batch::BatchParser;
//!
//! let mut p = BatchParser::new();
//! p.feed(b"* Hello\n\n");
//! p.feed(b"A paragraph.\n");
//! let (doc, diags) = p.finish();
//! ```
//!
//! # Example — event callback style
//! ```no_run
//! use muse_fmt::batch::{StreamingParser, Handler};
//! use muse_fmt::OwnedMuseEvent;
//!
//! let mut events = Vec::new();
//! let mut p = StreamingParser::new(|ev: OwnedMuseEvent| events.push(ev));
//! p.feed(b"* Hello\n\n");
//! p.feed(b"A paragraph.\n");
//! p.finish();
//! ```

use crate::ast::{Diagnostic, MuseDoc};
use crate::events::OwnedMuseEvent;

/// Chunk-driven Muse parser that returns the full AST on finish.
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
    pub fn finish(self) -> (MuseDoc, Vec<Diagnostic>) {
        let s = String::from_utf8_lossy(&self.buf);
        crate::parse::parse(&s)
    }
}

/// Handler trait for streaming Muse events.
///
/// Implemented automatically for any `FnMut(OwnedMuseEvent)`.
pub trait Handler {
    fn handle(&mut self, event: OwnedMuseEvent);
}

impl<F: FnMut(OwnedMuseEvent)> Handler for F {
    fn handle(&mut self, event: OwnedMuseEvent) {
        self(event);
    }
}

/// Chunked streaming Muse parser that delivers events to a [`Handler`].
///
/// Buffers all input, then parses and delivers events on `finish()`.
pub struct StreamingParser<H: Handler> {
    handler: H,
    buf: Vec<u8>,
}

impl<H: Handler> StreamingParser<H> {
    /// Create a new `StreamingParser` that delivers events to `handler`.
    pub fn new(handler: H) -> Self {
        StreamingParser {
            handler,
            buf: Vec::new(),
        }
    }

    /// Feed a chunk of bytes.
    pub fn feed(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);
    }

    /// Flush all input, parse, and deliver events to the handler.
    pub fn finish(mut self) {
        let s = String::from_utf8_lossy(&self.buf);
        let (doc, _) = crate::parse::parse(&s);
        for event in crate::events::events(&doc) {
            self.handler.handle(event.into_owned());
        }
    }
}

/// Chunk-driven Muse parser that delivers events to a callback on finish.
///
/// Convenience wrapper around [`StreamingParser`] for closure-based usage.
pub struct BatchSink<F: FnMut(OwnedMuseEvent)> {
    buf: Vec<u8>,
    callback: F,
}

impl<F: FnMut(OwnedMuseEvent)> BatchSink<F> {
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
        let (doc, _) = crate::parse::parse(&s);
        for event in crate::events::events(&doc) {
            (self.callback)(event.into_owned());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::OwnedMuseEvent;

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
    fn test_batch_parser_matches_parse() {
        let input = "* Heading\n\nA paragraph.\n\n - item1\n - item2\n";
        let (doc_direct, _) = crate::parse(input);
        let mut p = BatchParser::new();
        p.feed(input.as_bytes());
        let (doc_batch, _) = p.finish();
        assert_eq!(doc_direct.blocks.len(), doc_batch.blocks.len());
    }

    #[test]
    fn test_streaming_parser_events() {
        let mut evs = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        p.feed(b"* Hello\n\n");
        p.feed(b"A paragraph.\n");
        p.finish();
        assert!(evs
            .iter()
            .any(|e| matches!(e, OwnedMuseEvent::StartHeading { level: 1 })));
        assert!(evs.iter().any(|e| matches!(e, OwnedMuseEvent::StartParagraph)));
    }

    #[test]
    fn test_streaming_parser_split_chunks() {
        let mut evs: Vec<OwnedMuseEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        for b in b"* Title\n\nContent.\n" {
            p.feed(std::slice::from_ref(b));
        }
        p.finish();
        assert!(evs
            .iter()
            .any(|e| matches!(e, OwnedMuseEvent::StartHeading { .. })));
        assert!(evs.iter().any(|e| matches!(e, OwnedMuseEvent::StartParagraph)));
    }

    #[test]
    fn test_streaming_matches_events() {
        let input = b"* Heading\n\nParagraph one.\n\nParagraph two.\n";
        let s = String::from_utf8_lossy(input);
        let (doc, _) = crate::parse(&s);
        let bulk: Vec<OwnedMuseEvent> = crate::events::events(&doc)
            .map(|e| e.into_owned())
            .collect();

        let mut streamed: Vec<OwnedMuseEvent> = Vec::new();
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
        sink.feed(b"* Hello\n\n");
        sink.feed(b"A paragraph.\n");
        sink.finish();
        assert!(events
            .iter()
            .any(|e| matches!(e, OwnedMuseEvent::StartHeading { level: 1 })));
        assert!(events
            .iter()
            .any(|e| matches!(e, OwnedMuseEvent::StartParagraph)));
    }
}
