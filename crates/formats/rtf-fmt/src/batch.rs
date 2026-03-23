//! Chunk-driven (batch) RTF parser.
//!
//! Feed input in arbitrarily-sized chunks with [`BatchParser::feed`], then
//! call [`BatchParser::finish`] to obtain the parsed AST.
//!
//! For event-callback style (low-level token delivery), use [`BatchSink`].
//!
//! # Note
//!
//! This initial implementation buffers all input until `finish()`. Future
//! versions will deliver events incrementally where the RTF structure allows.
//!
//! # Example — AST style
//! ```no_run
//! use rtf_fmt::batch::BatchParser;
//!
//! let mut p = BatchParser::new();
//! p.feed(b"{\\rtf1\\ansi ");
//! p.feed(b"Hello}");
//! let (doc, diags) = p.finish();
//! ```
//!
//! # Example — callback style
//! ```no_run
//! use rtf_fmt::batch::BatchSink;
//! use rtf_fmt::Event;
//!
//! let mut evs = Vec::new();
//! let mut sink = BatchSink::new(|ev: Event| evs.push(ev));
//! sink.feed(b"{\\rtf1 Hello}");
//! sink.finish();
//! ```

use crate::ast::{Diagnostic, RtfDoc};
use crate::events::Event;

/// Chunk-driven RTF parser that returns the full AST on finish.
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
    pub fn finish(self) -> (RtfDoc, Vec<Diagnostic>) {
        crate::parse::parse(&self.buf)
    }
}

/// Chunk-driven RTF tokenizer that delivers low-level events to a callback on finish.
pub struct BatchSink<F: FnMut(Event)> {
    buf: Vec<u8>,
    callback: F,
}

impl<F: FnMut(Event)> BatchSink<F> {
    pub fn new(callback: F) -> Self {
        BatchSink { buf: Vec::new(), callback }
    }

    /// Feed a chunk of input bytes.
    pub fn feed(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);
    }

    /// Finish and deliver all RTF token events to the callback.
    pub fn finish(mut self) {
        for event in crate::events::events(&self.buf) {
            (self.callback)(event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::Event;

    #[test]
    fn test_batch_parser_basic() {
        let mut p = BatchParser::new();
        p.feed(b"{\\rtf1\\ansi Hello}");
        let (doc, _diags) = p.finish();
        // Should have at least one block
        assert!(!doc.blocks.is_empty());
    }

    #[test]
    fn test_batch_parser_split_chunks() {
        let mut p = BatchParser::new();
        for b in b"{\\rtf1\\ansi Hello}" {
            p.feed(std::slice::from_ref(b));
        }
        let (doc, _) = p.finish();
        assert!(!doc.blocks.is_empty());
    }

    #[test]
    fn test_batch_sink_events() {
        let mut events = Vec::new();
        let mut sink = BatchSink::new(|ev| events.push(ev));
        sink.feed(b"{\\rtf1 Hello}");
        sink.finish();
        assert!(events.iter().any(|e| matches!(e, Event::GroupStart { .. })));
    }
}
