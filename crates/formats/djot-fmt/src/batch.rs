//! Chunk-driven (batch) Djot parser.
//!
//! Feed input in arbitrarily-sized chunks with [`BatchParser::feed`], then
//! call [`BatchParser::finish`] to obtain the parsed result.
//!
//! For event-callback style, use [`BatchSink`].
//!
//! # Note
//!
//! This initial implementation buffers all input until `finish()`. Future
//! versions will deliver events incrementally at block boundaries, providing
//! O(working state) memory usage for large files.
//!
//! # Example — AST style
//! ```no_run
//! use djot_fmt::batch::BatchParser;
//!
//! let mut p = BatchParser::new();
//! p.feed(b"# Hello\n\n");
//! p.feed(b"A paragraph.\n");
//! let (doc, diags) = p.finish();
//! ```
//!
//! # Example — callback style
//! ```no_run
//! use djot_fmt::batch::BatchSink;
//! use djot_fmt::EventOwned;
//!
//! let mut events = Vec::new();
//! let mut sink = BatchSink::new(|ev: EventOwned| events.push(ev));
//! sink.feed(b"# Hello\n");
//! sink.finish();
//! ```

use crate::ast::{Diagnostic, DjotDoc};
use crate::events::OwnedEvent;

/// Chunk-driven Djot parser that returns the full AST on finish.
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
    pub fn finish(self) -> (DjotDoc, Vec<Diagnostic>) {
        let s = String::from_utf8_lossy(&self.buf);
        crate::parse::parse(&s)
    }
}

/// Chunk-driven Djot parser that delivers events to a callback on finish.
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
            (self.callback)(event);
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
        p.feed(b"# Hello\n\n");
        p.feed(b"A paragraph.\n");
        let (doc, diags) = p.finish();
        assert!(diags.is_empty());
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_batch_parser_split_chunks() {
        let mut p = BatchParser::new();
        // Feed character by character to test chunk handling
        for b in b"# Title\n\nContent here.\n" {
            p.feed(std::slice::from_ref(b));
        }
        let (doc, _) = p.finish();
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_batch_sink_events() {
        let mut events = Vec::new();
        let mut sink = BatchSink::new(|ev| events.push(ev));
        sink.feed(b"# Hello\n\n");
        sink.feed(b"A paragraph.\n");
        sink.finish();
        assert!(events.iter().any(|e| matches!(e, OwnedEvent::StartHeading { level: 1, .. })));
        assert!(events.iter().any(|e| matches!(e, OwnedEvent::StartParagraph { .. })));
    }
}
