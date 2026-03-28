//! Chunk-driven (batch) RTF parser.
//!
//! Feed input in arbitrarily-sized chunks with [`BatchParser::feed`], then
//! call [`BatchParser::finish`] to obtain the parsed AST.
//!
//! For event-callback style with low-level token events use [`BatchSink`].
//! For event-callback style with semantic document events use [`StreamingParser`].
//!
//! # Memory note — RTF structural constraint
//!
//! RTF requires font tables and color tables (declared in the document header)
//! to be fully parsed before any body content can be semantically interpreted.
//! This means [`StreamingParser`] must buffer the full document before delivering
//! semantic events — it is O(full input), unlike the line-oriented format crates.
//!
//! This is an inherent property of the RTF format, not an implementation
//! limitation. True incremental RTF streaming would require the caller to
//! re-supply the header on every chunk, which is not practical.
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
//! use rtf_fmt::TokenEvent;
//!
//! let mut evs = Vec::new();
//! let mut sink = BatchSink::new(|ev: TokenEvent| evs.push(ev));
//! sink.feed(b"{\\rtf1 Hello}");
//! sink.finish();
//! ```

use crate::ast::{Diagnostic, RtfDoc};
use crate::events::TokenEvent;
use crate::sem_events::OwnedEvent;

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

/// Chunk-driven RTF tokenizer that delivers low-level token events to a callback on finish.
pub struct BatchSink<F: FnMut(TokenEvent)> {
    buf: Vec<u8>,
    callback: F,
}

impl<F: FnMut(TokenEvent)> BatchSink<F> {
    pub fn new(callback: F) -> Self {
        BatchSink { buf: Vec::new(), callback }
    }

    /// Feed a chunk of input bytes.
    pub fn feed(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);
    }

    /// Finish and deliver all RTF token events to the callback.
    pub fn finish(mut self) {
        for event in crate::events::token_events(&self.buf) {
            (self.callback)(event);
        }
    }
}

/// Handler trait for semantic RTF events.
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

/// Chunked streaming RTF parser delivering semantic document events to a [`Handler`].
///
/// # Memory note
///
/// RTF's font and color tables must be parsed before body content can be
/// interpreted, so this implementation buffers the **full document** before
/// delivering any events. Memory is O(full input). See the
/// [module-level docs](self) for details.
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

    /// Parse all buffered input and deliver semantic events to the handler.
    pub fn finish(mut self) {
        for event in crate::sem_events::events(&self.buf) {
            self.handler.handle(event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::TokenEvent;

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
        assert!(events.iter().any(|e| matches!(e, TokenEvent::GroupStart { .. })));
    }
}
