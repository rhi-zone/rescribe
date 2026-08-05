//! Djot tokenizer, AST, and builder.
//!
//! A standalone crate with **no rescribe dependency** — usable as a general
//! Rust Djot library. The `rescribe-read-djot` and `rescribe-write-djot` crates
//! are thin adapter layers on top.
//!
//! # API layers
//!
//! `DjotDoc` implements four of the five shared `rescribe-format-api`
//! traits — `Emit`, `Events`, `StreamingParse`, `StreamingWrite`:
//!
//! ```text
//! use rescribe_format_api::{Emit, Events, StreamingParse, StreamingWrite};
//!
//! // Streaming reader
//! let it: EventIter = DjotDoc::events(input); // input: &[u8]
//!
//! // Batch reader — chunk-driven
//! let mut p = DjotDoc::streaming_parser(|ev| ...);
//! p.feed(chunk); // repeat
//! p.finish();
//!
//! // Builder writer
//! let bytes: Vec<u8> = doc.emit();
//!
//! // Streaming writer
//! let mut w = DjotDoc::writer(sink);
//! w.write_event(event); // repeat
//! w.finish();
//! ```
//!
//! `Parse` (`DjotDoc::parse(&[u8])`) is also implemented, but its native
//! parser (`parse::parse_str`) takes `&str`, not `&[u8]` — like
//! `commonmark-fmt`, the trait impl UTF-8-decodes internally (producing a
//! `Warning` diagnostic and an empty document on invalid UTF-8) and
//! delegates to it. `parse_str`/`events_str` remain public, documented
//! entry points alongside the trait (str input, no UTF-8 check needed) —
//! they are not redundant duplicates of the trait methods, which take
//! `&[u8]`.
//!
//! # Round-trip
//!
//! For well-formed documents: `parse_str(emit(parse_str(input).0)).0.strip_spans()`
//! should equal `parse_str(input).0.strip_spans()`. Verified by the roundtrip
//! fuzz harness.

mod ast;
pub mod batch;
mod emit;
mod events;
mod parse;
pub mod writer;

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use ast::{
    Alignment, Attr, Block, BulletStyle, DefItem, Diagnostic, DjotDoc, FootnoteDef, Inline,
    LinkDef, ListItem, ListKind, OrderedDelimiter, OrderedStyle, Span, TableCell, TableRow,
};
pub use batch::{BatchParser, BatchSink, StreamingParser};
pub use events::{Event, EventIter, EventOwned, OwnedEvent};
pub use parse::parse_str;
pub use rescribe_format_api::{Emit, Events, Handler, Parse, StreamingParse, StreamingWrite};
pub use writer::Writer;

/// Return a streaming event iterator over the parsed document.
///
/// Parses the input first, then walks the AST yielding owned events.
/// For details on the event types, see [`EventOwned`]. Documented separate
/// entry point alongside [`Events::events`](rescribe_format_api::Events::events):
/// takes `&str` (no UTF-8 check), where the trait method takes `&[u8]`.
pub fn events_str(input: &str) -> EventIter<'_> {
    EventIter::new(input)
}

// ── Trait implementations ───────────────────────────────────────────────────
//
// `DjotDoc` implements the shared API-mode traits directly — no parallel
// free functions (`djot_fmt::parse(&[u8])`, `djot_fmt::emit(..)`, ...) exist
// alongside them. `parse_str`/`events_str` above stay public because they
// have a materially different contract (str vs bytes), matching
// `commonmark-fmt`'s precedent.

impl Parse for DjotDoc {
    fn parse(input: &[u8]) -> (Self, Vec<Diagnostic>) {
        parse::parse(input)
    }
}

impl Emit for DjotDoc {
    fn emit(&self) -> Vec<u8> {
        emit::emit(self).into_bytes()
    }
}

impl Events for DjotDoc {
    type Event<'a> = Event<'a>;
    type EventIter<'a> = EventIter<'a>;

    /// Invalid UTF-8 input yields an iterator over an empty document
    /// (`StartDocument`/`EndDocument` only), mirroring `commonmark-fmt`'s
    /// handling of the same case — the trait's `events()` is infallible by
    /// contract. Callers that need to distinguish "empty document" from
    /// "invalid UTF-8" should use [`events_str`] directly.
    fn events(input: &[u8]) -> EventIter<'_> {
        match std::str::from_utf8(input) {
            Ok(s) => EventIter::new(s),
            Err(_) => EventIter::new(""),
        }
    }
}

impl StreamingParse for DjotDoc {
    type Event = OwnedEvent;
    type Parser<H: Handler<OwnedEvent>> = StreamingParser<H>;

    fn streaming_parser<H: Handler<OwnedEvent>>(handler: H) -> StreamingParser<H> {
        StreamingParser::new(handler)
    }
}

impl StreamingWrite for DjotDoc {
    type Writer<W: std::io::Write> = Writer<W>;

    fn writer<W: std::io::Write>(sink: W) -> Writer<W> {
        Writer::new(sink)
    }
}

#[cfg(test)]
mod smoke {
    use super::*;
    #[test]
    fn smoke_roundtrip() {
        let input = "# Hello *World*\n\nHere's a paragraph with _emphasis_, *strong*, and `code`.\n\n- Item one\n- Item two\n\n1. First\n2. Second\n\n> A blockquote\n\n```rust\nfn main() {}\n```\n\nHere is a footnote.[^fn1]\n\n[^fn1]: The footnote content.\n\n[link]: https://example.com\n";
        let (doc, diags) = parse::parse_str(input);
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert!(
            doc.blocks.len() >= 6,
            "expected >=6 blocks, got {}",
            doc.blocks.len()
        );
        assert_eq!(doc.footnotes.len(), 1);
        assert_eq!(doc.link_defs.len(), 1);
        let emitted = emit::emit(&doc);
        let (doc2, _) = parse::parse_str(&emitted);
        assert_eq!(doc.strip_spans(), doc2.strip_spans(), "roundtrip mismatch");
    }
    #[test]
    fn smoke_events() {
        let input = "# Hello\n\nA paragraph.\n";
        let evts: Vec<_> = events_str(input).collect();
        assert!(!evts.is_empty());
    }
}

#[cfg(test)]
mod debug_e2e {
    use super::*;
    #[test]
    fn debug_e2e_rich() {
        let input = "# Rich Document\n\n::: note\nThis div contains a [styled]{.highlight} span and inline math $`x^2`$.\n:::\n\n## References\n\nSee [example site](https://example.com) for more.\n\nDefinition list:\n\n: Term One\n  :: First definition with _emphasis_.\n\n: Term Two\n  :: Second definition.\n\nFootnote reference.[^ref]\n\n[^ref]: The reference content.\n";
        let (doc, diags) = parse::parse_str(input);
        for (i, b) in doc.blocks.iter().enumerate() {
            eprintln!("{i}: {:?}", std::mem::discriminant(b));
        }
        eprintln!("footnotes: {}", doc.footnotes.len());
        eprintln!("diags: {diags:?}");
    }
}
