//! Chunk-driven (batch) Texinfo parser.
//!
//! Feed input in arbitrarily-sized chunks with [`StreamingParser::feed`], then
//! call [`StreamingParser::finish`] to deliver all events to the handler.
//!
//! # Memory model
//!
//! [`StreamingParser`] buffers all input until `finish()` — Texinfo requires
//! the full input for correct parsing (forward references, @set/@value, etc.).
//! Memory usage is O(full input).
//!
//! [`BatchParser`] similarly buffers all input until `finish()` and is O(full input).
//!
//! # Example — AST style
//! ```no_run
//! use texinfo::batch::BatchParser;
//!
//! let mut p = BatchParser::new();
//! p.feed(b"@chapter Hello\n\n");
//! p.feed(b"A paragraph.\n");
//! let (doc, diags) = p.finish();
//! ```
//!
//! # Example — event callback style
//! ```no_run
//! use texinfo::batch::{StreamingParser, Handler};
//! use texinfo::OwnedEvent;
//!
//! let mut events = Vec::new();
//! let mut p = StreamingParser::new(|ev: OwnedEvent| events.push(ev));
//! p.feed(b"@chapter Hello\n\n");
//! p.feed(b"A paragraph.\n");
//! p.finish();
//! ```

use crate::ast::{Diagnostic, TexinfoDoc};
use crate::events::OwnedEvent;

/// Chunk-driven Texinfo parser that returns the full AST on finish.
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
    pub fn finish(self) -> (TexinfoDoc, Vec<Diagnostic>) {
        let s = String::from_utf8_lossy(&self.buf);
        crate::parse::parse(&s)
    }
}

/// Handler trait for streaming Texinfo events.
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

/// Chunked streaming Texinfo parser that delivers events to a [`Handler`].
///
/// Memory: O(full input). Texinfo requires full input for correct parsing.
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

    /// Flush any remaining input and deliver final events.
    pub fn finish(mut self) {
        let s = String::from_utf8_lossy(&self.buf);
        for event in crate::events::events(&s) {
            self.handler.handle(event.into_owned());
        }
    }
}

/// Chunk-driven Texinfo parser that delivers events to a callback on finish.
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
        p.feed(b"@chapter Hello\n\n");
        p.feed(b"A paragraph.\n");
        let (doc, diags) = p.finish();
        assert!(diags.is_empty());
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_batch_parser_split_chunks() {
        let mut p = BatchParser::new();
        for b in b"@chapter Title\n\nContent here.\n" {
            p.feed(std::slice::from_ref(b));
        }
        let (doc, _) = p.finish();
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_streaming_parser_events() {
        let mut evs = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        p.feed(b"@chapter Hello\n\n");
        p.feed(b"A paragraph.\n");
        p.finish();
        assert!(evs
            .iter()
            .any(|e| matches!(e, OwnedEvent::StartHeading { level: 1, .. })));
        assert!(evs
            .iter()
            .any(|e| matches!(e, OwnedEvent::StartParagraph)));
    }

    #[test]
    fn test_streaming_parser_split_chunks() {
        let mut evs: Vec<OwnedEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        for b in b"@chapter Title\n\nContent.\n" {
            p.feed(std::slice::from_ref(b));
        }
        p.finish();
        assert!(evs
            .iter()
            .any(|e| matches!(e, OwnedEvent::StartHeading { .. })));
        assert!(evs
            .iter()
            .any(|e| matches!(e, OwnedEvent::StartParagraph)));
    }

    #[test]
    fn test_streaming_matches_bulk() {
        let input = b"@chapter Heading\n\nParagraph one.\n\nParagraph two.\n";

        let bulk: Vec<OwnedEvent> = {
            let s = String::from_utf8_lossy(input);
            crate::events::events(&s)
                .map(|e| e.into_owned())
                .collect()
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
        sink.feed(b"@chapter Hello\n\n");
        sink.feed(b"A paragraph.\n");
        sink.finish();
        assert!(events
            .iter()
            .any(|e| matches!(e, OwnedEvent::StartHeading { level: 1, .. })));
        assert!(events
            .iter()
            .any(|e| matches!(e, OwnedEvent::StartParagraph)));
    }
}
