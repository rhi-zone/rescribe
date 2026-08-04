//! Chunk-driven (batch) Typst parser.
//!
//! Feed input in arbitrarily-sized chunks with [`StreamingParser::feed`],
//! then call [`StreamingParser::finish`] to deliver all events to the
//! handler; [`BatchParser`] does the same but returns the full
//! [`crate::ast::TypstDoc`] instead of dispatching events.
//!
//! # Limitation — sanctioned exemption
//!
//! This implementation buffers all input until `finish()` is called, then
//! parses the complete buffer with `typst_syntax::parse`. True incremental
//! chunked streaming is not possible: `typst-syntax`'s only from-scratch
//! parse entry points (`parse`, `parse_code`, `parse_math`) require the
//! full input as a `&str` — there is no chunk-fed parse API upstream.
//! (`typst_syntax::reparse`/`Source::edit` are edit-based *re*parsing of an
//! already-built tree for editor-style incremental updates, not applicable
//! to parsing from nothing.) This mirrors `commonmark-fmt`'s identical
//! exemption for pulldown-cmark (see its `batch.rs` module docs) — see
//! CLAUDE.md's "-fmt crates are not rescribe internals" section for the
//! two sanctioned "buffer all input" exceptions and why each is allowed.
//! For large-file use cases where the whole document must not be resident
//! at once, prefer loading the full input and using
//! [`crate::events::events`] directly, or the upstream `typst-syntax`
//! crate's own edit-based incremental reparsing for the editor use case.
//!
//! # Example
//!
//! ```
//! use typst_fmt::batch::StreamingParser;
//! use typst_fmt::Event;
//!
//! let mut collected = Vec::new();
//! let mut p = StreamingParser::new(|ev: Event| collected.push(ev));
//! p.feed(b"= Hello\n\n");
//! p.feed(b"A paragraph.\n");
//! p.finish();
//! assert!(collected.contains(&Event::StartHeading { level: 1 }));
//! ```

use crate::ast::{Diagnostic, TypstDoc};
use crate::events::{Event, EventIter};

/// Chunk-driven Typst parser that returns the full AST on `finish()`.
#[derive(Default)]
pub struct BatchParser {
    buf: Vec<u8>,
}

impl BatchParser {
    pub fn new() -> Self {
        BatchParser { buf: Vec::new() }
    }

    pub fn feed(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);
    }

    /// Parse all buffered input. If the buffer is not valid UTF-8, returns
    /// an empty document with a single diagnostic (Typst source is always
    /// text).
    pub fn finish(self) -> (TypstDoc, Vec<Diagnostic>) {
        match std::str::from_utf8(&self.buf) {
            Ok(s) => crate::parse::parse(s),
            Err(e) => (
                TypstDoc::default(),
                vec![Diagnostic {
                    message: format!("input is not valid UTF-8: {e}"),
                    span: crate::ast::Span::NONE,
                }],
            ),
        }
    }
}

/// Handler closure or type for batch-mode Typst events. Implemented
/// automatically for any `FnMut(Event)`.
pub trait Handler {
    fn handle(&mut self, event: Event);
}

impl<F: FnMut(Event)> Handler for F {
    fn handle(&mut self, event: Event) {
        self(event);
    }
}

/// Chunked streaming parser for Typst. See module docs for the buffering
/// limitation this crate exempts (mirroring `commonmark-fmt`/pulldown-cmark).
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

    /// Append a chunk of bytes to the internal buffer.
    pub fn feed(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);
    }

    /// Parse all buffered input and deliver events to the handler.
    pub fn finish(mut self) -> Vec<Diagnostic> {
        match std::str::from_utf8(&self.buf) {
            Ok(s) => {
                let (doc, diags) = crate::parse::parse(s);
                for ev in EventIter::from_doc(doc) {
                    self.handler.handle(ev);
                }
                diags
            }
            Err(e) => vec![Diagnostic {
                message: format!("input is not valid UTF-8: {e}"),
                span: crate::ast::Span::NONE,
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "= Title\n\nHello *world*.\n\n- one\n- two\n";

    #[test]
    fn batch_parser_matches_direct_parse() {
        let mut p = BatchParser::new();
        let mid = SAMPLE.len() / 2;
        p.feed(&SAMPLE.as_bytes()[..mid]);
        p.feed(&SAMPLE.as_bytes()[mid..]);
        let (doc, diags) = p.finish();
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        let (expected, _) = crate::parse::parse(SAMPLE);
        assert_eq!(doc.strip_spans(), expected.strip_spans());
    }

    #[test]
    fn streaming_parser_matches_events() {
        let mut evs = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        let mid = SAMPLE.len() / 2;
        p.feed(&SAMPLE.as_bytes()[..mid]);
        p.feed(&SAMPLE.as_bytes()[mid..]);
        let diags = p.finish();
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        let expected: Vec<_> = crate::events::events(SAMPLE).collect();
        assert_eq!(evs, expected);
    }
}
