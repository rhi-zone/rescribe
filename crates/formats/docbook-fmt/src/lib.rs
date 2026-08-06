//! DocBook (and generic XML) parser, AST, and emitter.
//!
//! A standalone crate wrapping `quick-xml` with **no rescribe dependency by
//! default** — usable as a general Rust DocBook/XML library. The optional
//! `rescribe` feature (default off) adds `crate::rescribe::{parse, emit}`,
//! a thin adapter that translates `docbook_fmt::Node` to and from
//! rescribe's `Document` IR; DocBook-specific *meaning* (which element
//! names map to which document semantics) lives in that adapter, not here
//! — this crate only knows XML structure by default.
//!
//! # API layers
//!
//! `DocBookDoc` implements the five shared `rescribe-format-api` traits —
//! no parallel free functions exist alongside them:
//!
//! ```text
//! use rescribe_format_api::{Emit, Events, Parse, StreamingParse, StreamingWrite};
//!
//! // AST reader
//! let (doc, diags): (DocBookDoc, Vec<Diagnostic>) = DocBookDoc::parse(input);
//!
//! // Streaming reader — true SAX-style pull iterator, no tree built
//! let it: EventIter = DocBookDoc::events(input);
//!
//! // Batch reader — chunk-driven, dispatches events as soon as provably complete
//! let mut p = DocBookDoc::streaming_parser(|ev| ...);
//! p.feed(chunk); // repeat
//! p.finish();
//!
//! // Builder writer — emit from AST
//! let bytes: Vec<u8> = doc.emit();
//!
//! // Streaming writer — emit from events
//! let mut w = DocBookDoc::writer(sink);
//! w.write_event(event); // repeat
//! w.finish(); // flushes to sink
//! ```
//!
//! `emit_fragment` (emits a `Node` subtree rather than a whole `DocBookDoc`)
//! stays as a separate, documented entry point alongside `Emit::emit` — a
//! materially different contract (different input type), not a redundant
//! duplicate.
//!
//! # Why XML can stream where HTML can't
//!
//! `html-fmt`'s three reader APIs all build the full DOM internally because
//! HTML5 tree construction is allowed to rearrange previously-seen nodes
//! (foster parenting, adoption agency). XML has no such rule — it is
//! well-nested by construction — so `events()` and `StreamingParser` here
//! are genuinely independent, incremental implementations that never
//! materialize a `DocBookDoc`. See `events.rs` and `batch.rs` module docs
//! for the chunk-boundary handling this requires (mainly: plain text is the
//! one token quick-xml doesn't distinguish from "still coming" vs "done").

pub mod ast;
pub mod batch;
pub mod emit;
pub mod events;
pub mod parse;
#[cfg(feature = "rescribe")]
pub mod rescribe;
pub mod writer;

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use ast::{Diagnostic, DocBookDoc, Node, Severity, Span, XmlDecl};
pub use batch::{BatchParser, StreamingParser};
pub use emit::emit_fragment;
pub use events::{Event, EventIter, OwnedEvent};
pub use rescribe_format_api::{Emit, Events, Handler, Parse, StreamingParse, StreamingWrite};
pub use writer::Writer;

// ── Trait implementations ───────────────────────────────────────────────────
//
// `DocBookDoc` implements the shared API-mode traits directly — no
// parallel free functions (`docbook_fmt::parse(..)`, `docbook_fmt::emit(..)`,
// `docbook_fmt::events(..)`, ...) exist alongside them. `emit_fragment`
// stays public (materially different contract: `&[Node]`, not
// `&DocBookDoc`).

impl Parse for DocBookDoc {
    fn parse(input: &[u8]) -> (Self, Vec<Diagnostic>) {
        parse::parse(input)
    }
}

impl Emit for DocBookDoc {
    fn emit(&self) -> Vec<u8> {
        emit::emit(self)
    }
}

impl Events for DocBookDoc {
    type Event<'a> = Event<'a>;
    type EventIter<'a> = EventIter<'a>;

    fn events(input: &[u8]) -> EventIter<'_> {
        EventIter::new(input)
    }
}

impl StreamingParse for DocBookDoc {
    type Event = OwnedEvent;
    type Parser<H: Handler<OwnedEvent>> = StreamingParser<H>;

    fn streaming_parser<H: Handler<OwnedEvent>>(handler: H) -> StreamingParser<H> {
        StreamingParser::new(handler)
    }
}

impl StreamingWrite for DocBookDoc {
    type Writer<W: std::io::Write> = Writer<W>;

    fn writer<W: std::io::Write>(sink: W) -> Writer<W> {
        Writer::new(sink)
    }
}

#[cfg(test)]
mod smoke {
    use super::*;

    #[test]
    fn smoke_parse_document() {
        let input =
            br#"<?xml version="1.0"?><article><title>Hi</title><para>Body</para></article>"#;
        let (doc, diags) = DocBookDoc::parse(input);
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert!(doc.root().is_some());
    }

    #[test]
    fn smoke_roundtrip() {
        let input =
            br#"<?xml version="1.0"?><article><title>Hi</title><para>A &amp; B</para></article>"#;
        let (doc1, _) = DocBookDoc::parse(input);
        let emitted = doc1.emit();
        let (doc2, diags) = DocBookDoc::parse(&emitted);
        assert!(diags.is_empty());
        assert_eq!(doc1.strip_spans(), doc2.strip_spans(), "roundtrip mismatch");
    }

    #[test]
    fn smoke_events() {
        let input = b"<para>Hello <emphasis>world</emphasis></para>";
        let evts: Vec<_> = DocBookDoc::events(input).collect();
        assert!(!evts.is_empty());
        assert!(
            evts.iter()
                .any(|e| matches!(e, Event::StartElement { name, .. } if name == "para"))
        );
    }

    #[test]
    fn smoke_event_roundtrip() {
        let input =
            br#"<?xml version="1.0"?><article><title>Hi</title><para>Body</para></article>"#;
        let (doc1, _) = DocBookDoc::parse(input);
        let evts = events::events_from_doc(&doc1);
        let doc2 = events::collect_doc(evts);
        assert_eq!(
            doc1.strip_spans(),
            doc2.strip_spans(),
            "event roundtrip mismatch"
        );
    }

    #[test]
    fn smoke_batch_parser() {
        let mut p = BatchParser::new();
        p.feed(b"<article><title>Hello</title>");
        p.feed(b"<para>World</para></article>");
        let (doc, diags) = p.finish();
        assert!(diags.is_empty());
        assert!(doc.root().is_some());
    }

    #[test]
    fn smoke_streaming_parser() {
        let mut evts = Vec::new();
        let mut p = StreamingParser::new(|ev| evts.push(ev));
        p.feed(b"<article><title>Hello</title>");
        p.feed(b"<para>World</para></article>");
        let diags = p.finish();
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert!(
            evts.iter()
                .any(|e| matches!(e, Event::StartElement { name, .. } if name == "title"))
        );
        assert!(
            evts.iter()
                .any(|e| matches!(e, Event::Text(t) if t == "World"))
        );
    }

    #[test]
    fn smoke_streaming_parser_splits_text_across_chunks() {
        // "Hello, world" split mid-word across two feed() calls must arrive
        // as one Text event, not two.
        let mut evts = Vec::new();
        let mut p = StreamingParser::new(|ev| evts.push(ev));
        p.feed(b"<para>Hello, wo");
        p.feed(b"rld</para>");
        let diags = p.finish();
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        let texts: Vec<_> = evts
            .iter()
            .filter_map(|e| match e {
                Event::Text(t) => Some(t.as_ref()),
                _ => None,
            })
            .collect();
        assert_eq!(texts, vec!["Hello, world"]);
    }

    #[test]
    fn smoke_streaming_parser_splits_tag_across_chunks() {
        let mut evts = Vec::new();
        let mut p = StreamingParser::new(|ev| evts.push(ev));
        p.feed(b"<emph");
        p.feed(b"asis>hi</emphasis>");
        let diags = p.finish();
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert!(
            evts.iter()
                .any(|e| matches!(e, Event::StartElement { name, .. } if name == "emphasis"))
        );
    }

    #[test]
    fn smoke_writer() {
        use std::borrow::Cow;

        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(Event::StartElement {
            name: Cow::Borrowed("para"),
            attrs: vec![],
        });
        w.write_event(Event::Text(Cow::Borrowed("Hello")));
        w.write_event(Event::EndElement {
            name: Cow::Borrowed("para"),
        });
        let bytes = w.finish();
        assert_eq!(String::from_utf8(bytes).unwrap(), "<para>Hello</para>");
    }

    #[test]
    fn smoke_escape() {
        let input = b"<para>&amp; &lt; &gt;</para>";
        let (doc, _) = DocBookDoc::parse(input);
        let emitted = doc.emit();
        let xml = String::from_utf8(emitted).unwrap();
        assert!(xml.contains("&amp;"));
        assert!(xml.contains("&lt;"));
        assert!(xml.contains("&gt;"));
    }

    #[test]
    fn smoke_self_closing() {
        let input = b"<para>Hello<sbr/>World</para>";
        let (doc, _) = DocBookDoc::parse(input);
        let emitted = doc.emit();
        let xml = String::from_utf8(emitted).unwrap();
        assert!(xml.contains("<sbr/>"));
    }
}
