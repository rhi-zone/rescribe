//! Chunk-driven (batch) RTF parser.
//!
//! Feed input in arbitrarily-sized chunks with [`BatchParser::feed`], then
//! call [`BatchParser::finish`] to obtain the parsed AST.
//!
//! For event-callback style with low-level token events use [`BatchSink`].
//! For event-callback style with semantic document events use [`StreamingParser`].
//!
//! # Memory note — current implementation vs. what the RTF grammar allows
//!
//! RTF requires font tables and color tables (declared in the document header)
//! to be fully parsed before any body content can be semantically interpreted:
//! `\f<N>`/`\cf<N>` body control words are indices into those tables. But this
//! is a *bounded* requirement, not an unbounded one — the RTF 1.9.1 grammar
//! (`<file> ::= '{' <header> <document> '}'`) places the header strictly
//! before the document body, and the spec text is explicit that the header
//! groups "must precede the first plain-text character in the document." So
//! by the time real body content starts, the full font/color tables are
//! already guaranteed present — a correct incremental parser only needs to
//! buffer up through the end of the header group (`O(header size)`, not
//! `O(full input)`), then can stream the body incrementally with no further
//! buffering need beyond ordinary bounded per-construct lookahead (the same
//! kind rst-fmt/djot-fmt/etc. already do).
//!
//! **This implementation does not do that.** [`StreamingParser::feed`] below
//! simply appends every chunk to an internal buffer and does all parsing in
//! `finish()` — a real `O(full input)` buffer-then-finish stub, not a
//! consequence of anything the RTF grammar itself requires. Building a
//! genuine incremental reader (buffer only the header group, then stream) is
//! real implementation work, tracked as still open — see
//! `streaming_harness::KNOWN_FAILURES`'s `rtf`/`streaming_parser` entry and
//! `TODO.md` for the full investigation this doc comment reflects.
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
        BatchSink {
            buf: Vec::new(),
            callback,
        }
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
/// This implementation buffers the **full document** before delivering any
/// events — `feed()` only appends to an internal buffer, and all parsing
/// happens in `finish()`. Memory is `O(full input)`. RTF's font/color-table-
/// before-body requirement only justifies buffering `O(header size)`, not
/// the whole document — this is a real, fixable implementation gap, not an
/// inherent format constraint. See the [module-level docs](self) for the
/// full explanation.
pub struct StreamingParser<H: Handler> {
    buf: Vec<u8>,
    handler: H,
}

impl<H: Handler> StreamingParser<H> {
    /// Create a new `StreamingParser` that delivers events to `handler`.
    pub fn new(handler: H) -> Self {
        StreamingParser {
            buf: Vec::new(),
            handler,
        }
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
        assert!(
            events
                .iter()
                .any(|e| matches!(e, TokenEvent::GroupStart { .. }))
        );
    }
}
