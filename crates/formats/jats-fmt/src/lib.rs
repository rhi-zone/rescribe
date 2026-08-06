//! JATS (Journal Article Tag Suite) and generic XML parser, AST, and emitter.
//!
//! A standalone crate wrapping `quick-xml` with **no rescribe dependency by
//! default** — usable as a general Rust JATS/XML library. The optional
//! `rescribe` feature (default off) adds `crate::rescribe::{parse, emit}`, a
//! thin adapter layer that translates `jats_fmt::Node` to and from
//! rescribe's IR; JATS-specific *meaning* (which element names —
//! `<article>`, `<sec>`, `<xref>`, `<fig>`, etc. — map to which document
//! semantics) lives in that adapter, not here — this crate only knows XML
//! structure.
//!
//! # Tag-set scope
//!
//! JATS (NISO Z39.96) is published as three sibling tag sets — Archiving and
//! Interchange, Journal Publishing, and Article Authoring — that share the
//! same element vocabulary with progressively tighter content-model
//! constraints (Publishing/Authoring are validity *subsets* of Archiving,
//! not divergent vocabularies), plus BITS, a genuinely separate book-content
//! extension. This crate does not parse against any DTD/schema at all — it
//! accepts any well-formed XML regardless of tag set, so "which tag set" has
//! no effect on this crate's behavior; it only affects which elements
//! `rescribe-read-jats`/`rescribe-write-jats` and `fixtures/jats/` choose to
//! model explicitly. See `docs/adr/0012-jats-archiving-tag-set-scope.md` for
//! the rationale (Archiving chosen as the fixture/mapping reference because
//! it's the element superset).
//!
//! # API layers
//!
//! `JatsDoc` implements the five shared `rescribe-format-api` traits — no
//! parallel free functions exist alongside them:
//!
//! ```text
//! use rescribe_format_api::{Emit, Events, Parse, StreamingParse, StreamingWrite};
//!
//! // AST reader
//! let (doc, diags): (JatsDoc, Vec<Diagnostic>) = JatsDoc::parse(input);
//!
//! // Streaming reader — true SAX-style pull iterator, no tree built
//! let it: EventIter = JatsDoc::events(input);
//!
//! // Batch reader — chunk-driven, dispatches events as soon as provably complete
//! let mut p = JatsDoc::streaming_parser(|ev| ...);
//! p.feed(chunk); // repeat
//! p.finish();
//!
//! // Builder writer — emit from AST
//! let bytes: Vec<u8> = doc.emit();
//!
//! // Streaming writer — emit from events
//! let mut w = JatsDoc::writer(sink);
//! w.write_event(event); // repeat
//! w.finish(); // flushes to sink
//! ```
//!
//! `emit_fragment` (emits a `Node` subtree rather than a whole `JatsDoc`)
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
//! materialize a `JatsDoc`. See `events.rs` and `batch.rs` module docs
//! for the chunk-boundary handling this requires (mainly: plain text is the
//! one token quick-xml doesn't distinguish from "still coming" vs "done").

pub mod ast;
pub mod batch;
pub mod emit;
pub mod events;
pub mod parse;
#[cfg(feature = "registry")]
pub mod registry;
#[cfg(feature = "registry-derive")]
pub mod registry_derive;
#[cfg(feature = "rescribe")]
pub mod rescribe;
pub mod writer;

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use ast::{Diagnostic, JatsDoc, Node, Severity, Span, XmlDecl};
pub use batch::{BatchParser, StreamingParser};
pub use emit::emit_fragment;
pub use events::{Event, EventIter, OwnedEvent};
pub use rescribe_format_api::{Emit, Events, Handler, Parse, StreamingParse, StreamingWrite};
pub use writer::Writer;

// ── Trait implementations ───────────────────────────────────────────────────
//
// `JatsDoc` implements the shared API-mode traits directly — no parallel
// free functions (`jats_fmt::parse(..)`, `jats_fmt::emit(..)`,
// `jats_fmt::events(..)`, ...) exist alongside them. `emit_fragment` stays
// public (materially different contract: `&[Node]`, not `&JatsDoc`).

impl Parse for JatsDoc {
    fn parse(input: &[u8]) -> (Self, Vec<Diagnostic>) {
        parse::parse(input)
    }
}

impl Emit for JatsDoc {
    fn emit(&self) -> Vec<u8> {
        emit::emit(self)
    }
}

impl Events for JatsDoc {
    type Event<'a> = Event<'a>;
    type EventIter<'a> = EventIter<'a>;

    fn events(input: &[u8]) -> EventIter<'_> {
        EventIter::new(input)
    }
}

impl StreamingParse for JatsDoc {
    type Event = OwnedEvent;
    type Parser<H: Handler<OwnedEvent>> = StreamingParser<H>;

    fn streaming_parser<H: Handler<OwnedEvent>>(handler: H) -> StreamingParser<H> {
        StreamingParser::new(handler)
    }
}

impl StreamingWrite for JatsDoc {
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
        let input = br#"<?xml version="1.0"?><article><title>Hi</title><p>Body</p></article>"#;
        let (doc, diags) = JatsDoc::parse(input);
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert!(doc.root().is_some());
    }

    #[test]
    fn smoke_roundtrip() {
        let input = br#"<?xml version="1.0"?><article><title>Hi</title><p>A &amp; B</p></article>"#;
        let (doc1, _) = JatsDoc::parse(input);
        let emitted = doc1.emit();
        let (doc2, diags) = JatsDoc::parse(&emitted);
        assert!(diags.is_empty());
        assert_eq!(doc1.strip_spans(), doc2.strip_spans(), "roundtrip mismatch");
    }

    #[test]
    fn smoke_events() {
        let input = b"<p>Hello <italic>world</italic></p>";
        let evts: Vec<_> = JatsDoc::events(input).collect();
        assert!(!evts.is_empty());
        assert!(
            evts.iter()
                .any(|e| matches!(e, Event::StartElement { name, .. } if name == "p"))
        );
    }

    #[test]
    fn smoke_event_roundtrip() {
        let input = br#"<?xml version="1.0"?><article><title>Hi</title><p>Body</p></article>"#;
        let (doc1, _) = JatsDoc::parse(input);
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
        p.feed(b"<p>World</p></article>");
        let (doc, diags) = p.finish();
        assert!(diags.is_empty());
        assert!(doc.root().is_some());
    }

    #[test]
    fn smoke_streaming_parser() {
        let mut evts = Vec::new();
        let mut p = StreamingParser::new(|ev| evts.push(ev));
        p.feed(b"<article><title>Hello</title>");
        p.feed(b"<p>World</p></article>");
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
        p.feed(b"<p>Hello, wo");
        p.feed(b"rld</p>");
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
        p.feed(b"<ita");
        p.feed(b"lic>hi</italic>");
        let diags = p.finish();
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert!(
            evts.iter()
                .any(|e| matches!(e, Event::StartElement { name, .. } if name == "italic"))
        );
    }

    #[test]
    fn smoke_writer() {
        use std::borrow::Cow;

        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(Event::StartElement {
            name: Cow::Borrowed("p"),
            attrs: vec![],
        });
        w.write_event(Event::Text(Cow::Borrowed("Hello")));
        w.write_event(Event::EndElement {
            name: Cow::Borrowed("p"),
        });
        let bytes = w.finish();
        assert_eq!(String::from_utf8(bytes).unwrap(), "<p>Hello</p>");
    }

    #[test]
    fn smoke_escape() {
        let input = b"<p>&amp; &lt; &gt;</p>";
        let (doc, _) = JatsDoc::parse(input);
        let emitted = doc.emit();
        let xml = String::from_utf8(emitted).unwrap();
        assert!(xml.contains("&amp;"));
        assert!(xml.contains("&lt;"));
        assert!(xml.contains("&gt;"));
    }

    #[test]
    fn smoke_self_closing() {
        let input = b"<p>Hello<break/>World</p>";
        let (doc, _) = JatsDoc::parse(input);
        let emitted = doc.emit();
        let xml = String::from_utf8(emitted).unwrap();
        assert!(xml.contains("<break/>"));
    }
}
