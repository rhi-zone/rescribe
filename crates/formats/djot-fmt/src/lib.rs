//! Djot tokenizer, AST, and builder.
//!
//! A standalone crate with **no rescribe dependency** — usable as a general
//! Rust Djot library. The `rescribe-read-djot` and `rescribe-write-djot` crates
//! are thin adapter layers on top.
//!
//! # API layers
//!
//! ```text
//! // AST reader
//! pub fn parse(input: &str) -> (DjotDoc, Vec<Diagnostic>);
//!
//! // Streaming reader — iterator over owned events
//! pub fn events(input: &str) -> impl Iterator<Item = EventOwned> + '_;
//!
//! // Batch reader — chunk-driven
//! let mut p = BatchParser::new();
//! p.feed(chunk); // repeat
//! let (doc, diags) = p.finish();
//!
//! // Builder writer — emit from AST
//! pub fn emit(doc: &DjotDoc) -> String;
//!
//! // Streaming writer — emit from events
//! let mut w = Writer::new(sink);
//! w.write_event(event); // repeat
//! w.finish(); // flushes to sink
//! ```
//!
//! # Round-trip
//!
//! For well-formed documents: `parse(emit(parse(input).0)).0.strip_spans()` should
//! equal `parse(input).0.strip_spans()`. Verified by the roundtrip fuzz harness.

mod ast;
mod emit;
mod events;
mod parse;
pub mod batch;
pub mod writer;

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use ast::{
    Alignment, Attr, Block, BulletStyle, DefItem, Diagnostic, DjotDoc, FootnoteDef, Inline,
    LinkDef, ListItem, ListKind, OrderedDelimiter, OrderedStyle, Span, TableCell, TableRow,
};
pub use emit::emit;
pub use events::{Event, EventIter, EventOwned, OwnedEvent};
pub use parse::parse;
pub use writer::Writer;
pub use batch::{BatchParser, BatchSink};

/// Return a streaming event iterator over the parsed document.
///
/// Parses the input first, then walks the AST yielding owned events.
/// For details on the event types, see [`EventOwned`].
pub fn events(input: &str) -> EventIter<'_> {
    EventIter::new(input)
}

#[cfg(test)]
mod smoke {
    use super::*;
    #[test]
    fn smoke_roundtrip() {
        let input = "# Hello *World*\n\nHere's a paragraph with _emphasis_, *strong*, and `code`.\n\n- Item one\n- Item two\n\n1. First\n2. Second\n\n> A blockquote\n\n```rust\nfn main() {}\n```\n\nHere is a footnote.[^fn1]\n\n[^fn1]: The footnote content.\n\n[link]: https://example.com\n";
        let (doc, diags) = parse(input);
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert!(doc.blocks.len() >= 6, "expected >=6 blocks, got {}", doc.blocks.len());
        assert_eq!(doc.footnotes.len(), 1);
        assert_eq!(doc.link_defs.len(), 1);
        let emitted = emit(&doc);
        let (doc2, _) = parse(&emitted);
        assert_eq!(doc.strip_spans(), doc2.strip_spans(), "roundtrip mismatch");
    }
    #[test]
    fn smoke_events() {
        let input = "# Hello\n\nA paragraph.\n";
        let evts: Vec<_> = events(input).collect();
        assert!(!evts.is_empty());
    }
}


#[cfg(test)]
mod debug_e2e {
    use super::*;
    #[test]
    fn debug_e2e_rich() {
        let input = "# Rich Document\n\n::: note\nThis div contains a [styled]{.highlight} span and inline math $`x^2`$.\n:::\n\n## References\n\nSee [example site](https://example.com) for more.\n\nDefinition list:\n\n: Term One\n  :: First definition with _emphasis_.\n\n: Term Two\n  :: Second definition.\n\nFootnote reference.[^ref]\n\n[^ref]: The reference content.\n";
        let (doc, diags) = parse(input);
        for (i, b) in doc.blocks.iter().enumerate() {
            eprintln!("{i}: {:?}", std::mem::discriminant(b));
        }
        eprintln!("footnotes: {}", doc.footnotes.len());
        eprintln!("diags: {diags:?}");
    }
}
