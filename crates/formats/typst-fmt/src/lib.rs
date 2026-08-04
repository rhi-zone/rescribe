//! Typst markup parser, AST, and emitter.
//!
//! A standalone crate wrapping `typst-syntax` with **no rescribe
//! dependency** — usable as a general Rust Typst library by anything that
//! needs to read or write Typst markup: a static-site generator, a linter,
//! a document-diffing tool, a search indexer. `rescribe-read-typst` and
//! `rescribe-write-typst` are thin adapter layers on top that translate
//! `typst_fmt::TypstDoc` to and from rescribe's IR.
//!
//! # API layers
//!
//! ```text
//! // AST reader — thin wrap of typst_syntax::parse plus a tree walk
//! pub fn parse(input: &str) -> (TypstDoc, Vec<Diagnostic>);
//!
//! // Streaming reader — cursor-based walk over the parsed tree
//! pub fn events(input: &str) -> EventIter;
//!
//! // Batch reader — chunk-driven, buffers to finish() (see batch.rs docs
//! // for why: no chunk-fed parse API exists upstream)
//! let mut p = StreamingParser::new(|ev| ...);
//! p.feed(chunk); // repeat
//! p.finish();
//!
//! // Builder writer — emit from AST
//! pub fn emit(doc: &TypstDoc) -> Vec<u8>;
//!
//! // Streaming writer — emit from events, incrementally
//! let mut w = Writer::new(sink);
//! w.write_event(event); // repeat
//! w.finish();
//! ```
//!
//! # Design
//!
//! `typst-syntax`'s `SyntaxNode` tree is already a legitimate domain AST for
//! Typst (unlike a generic XML tree). This crate still defines its own
//! [`TypstDoc`]/[`Block`]/[`Inline`] rather than exposing `SyntaxNode`
//! directly: the writer side needs a type it can *construct* from scratch
//! (there is no upstream Typst document builder), and the fuzz roundtrip
//! property this crate is held to
//! (`parse(emit(arbitrary_ast)).strip_spans() == arbitrary_ast`, see
//! CLAUDE.md's "Roundtrip direction matters") needs one shared type on both
//! ends. See `ast.rs`'s module docs for the full construct-coverage
//! rationale.
//!
//! `events()` is a tree-walk-with-cursor over the already-parsed structure,
//! not a from-scratch incremental parse — `typst-syntax` has no native
//! event/SAX mode, so parsing to a tree first is unavoidable; see
//! `events.rs`'s module docs. `StreamingParser` buffers all input until
//! `finish()` for the same underlying reason (no chunk-fed parse API
//! upstream) — a second sanctioned "buffer all input" exemption alongside
//! `commonmark-fmt`'s pulldown-cmark, see `batch.rs`'s module docs and
//! CLAUDE.md's "-fmt crates are not rescribe internals" section.

pub mod ast;
pub mod batch;
pub mod emit;
pub mod events;
pub mod parse;
pub mod writer;

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use ast::{Block, Diagnostic, Inline, Span, TypstDoc};
pub use batch::{BatchParser, Handler, StreamingParser};
pub use emit::emit;
pub use events::{Event, EventIter, collect_doc, events_from_doc};
pub use parse::parse;
pub use writer::Writer;

/// Return a streaming event iterator over `input`'s parsed structure,
/// without the caller needing to hold onto a separately-parsed [`TypstDoc`].
pub fn events(input: &str) -> EventIter {
    events::events(input)
}

#[cfg(test)]
mod smoke {
    use super::*;

    const SAMPLE: &str = "= Title\n\nHello *bold* and _em_ world.\n\n- one\n- two\n";

    #[test]
    fn smoke_parse_document() {
        let (doc, diags) = parse(SAMPLE);
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert_eq!(doc.blocks.len(), 3);
        assert!(matches!(doc.blocks[0], Block::Heading { level: 1, .. }));
        assert!(matches!(doc.blocks[1], Block::Paragraph(_)));
        assert!(matches!(doc.blocks[2], Block::List { ordered: false, .. }));
    }

    #[test]
    fn smoke_roundtrip() {
        let (doc1, diags1) = parse(SAMPLE);
        assert!(diags1.is_empty());
        let emitted = emit(&doc1);
        let emitted_str = std::str::from_utf8(&emitted).unwrap();
        let (doc2, diags2) = parse(emitted_str);
        assert!(diags2.is_empty(), "diagnostics: {diags2:?}");
        assert_eq!(doc1.strip_spans(), doc2.strip_spans(), "roundtrip mismatch");
    }

    #[test]
    fn smoke_events() {
        let evts: Vec<_> = events(SAMPLE).collect();
        assert!(!evts.is_empty());
        assert!(evts.contains(&Event::StartHeading { level: 1 }));
        assert!(evts.contains(&Event::StartList { ordered: false }));
    }

    #[test]
    fn smoke_event_roundtrip() {
        let (doc1, _) = parse(SAMPLE);
        let evts = events_from_doc(&doc1);
        let doc2 = collect_doc(evts);
        assert_eq!(
            doc1.strip_spans(),
            doc2.strip_spans(),
            "event roundtrip mismatch"
        );
    }

    #[test]
    fn smoke_events_equals_ast_projection() {
        let (doc, _diags) = parse(SAMPLE);
        let expected = events_from_doc(&doc);
        let actual: Vec<_> = events(SAMPLE).collect();
        assert_eq!(expected, actual);
    }

    #[test]
    fn smoke_batch_parser() {
        let mut p = BatchParser::new();
        let mid = SAMPLE.len() / 2;
        p.feed(&SAMPLE.as_bytes()[..mid]);
        p.feed(&SAMPLE.as_bytes()[mid..]);
        let (doc, diags) = p.finish();
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert_eq!(doc.blocks.len(), 3);
    }

    #[test]
    fn smoke_streaming_parser() {
        let mut evts = Vec::new();
        let mut p = StreamingParser::new(|ev| evts.push(ev));
        let mid = SAMPLE.len() / 2;
        p.feed(&SAMPLE.as_bytes()[..mid]);
        p.feed(&SAMPLE.as_bytes()[mid..]);
        let diags = p.finish();
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert!(evts.contains(&Event::StartHeading { level: 1 }));
    }

    #[test]
    fn smoke_writer() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(Event::StartDocument);
        w.write_event(Event::StartHeading { level: 1 });
        w.write_event(Event::Text("Hi".into()));
        w.write_event(Event::EndHeading);
        w.write_event(Event::EndDocument);
        let bytes = w.finish();
        let out = String::from_utf8(bytes).unwrap();
        assert!(out.contains("= Hi"));
    }
}
