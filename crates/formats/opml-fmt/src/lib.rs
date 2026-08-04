//! OPML (Outline Processor Markup Language) parser, AST, and emitter.
//!
//! A standalone crate wrapping `quick-xml` with **no rescribe dependency**
//! — usable as a general Rust OPML library by anything that needs to read
//! or write outline/subscription-list files: a feed reader importing an
//! OPML export, a podcast client's subscription list, a mind-mapping tool's
//! outline import/export, a static-site generator's blogroll page.
//! `rescribe-read-opml` and `rescribe-write-opml` are thin adapter layers
//! on top that translate `opml_fmt::OpmlDoc` to and from rescribe's IR.
//!
//! # API layers
//!
//! ```text
//! // AST reader
//! pub fn parse(input: &[u8]) -> (OpmlDoc, Vec<Diagnostic>);
//!
//! // Streaming reader — true SAX-style pull iterator, no tree built
//! pub fn events(input: &[u8]) -> EventIter;
//!
//! // Batch reader — chunk-driven, dispatches events as soon as provably complete
//! let mut p = StreamingParser::new(|ev| ...);
//! p.feed(chunk); // repeat
//! p.finish();
//!
//! // Builder writer — emit from AST
//! pub fn emit(doc: &OpmlDoc) -> Vec<u8>;
//!
//! // Streaming writer — emit from events
//! let mut w = Writer::new(sink);
//! w.write_event(event); // repeat
//! w.finish(); // flushes to sink
//! ```
//!
//! # Why XML can stream where HTML can't
//!
//! OPML is well-nested XML by construction (unlike HTML5, which allows tree
//! construction to rearrange already-seen nodes), so `events()` and
//! `StreamingParser` here are genuinely independent, incremental
//! implementations that never materialize an `OpmlDoc`. See `events.rs`
//! and `batch.rs` module docs for the chunk-boundary handling this
//! requires. This crate follows the same structural pattern as
//! `docbook-fmt`/`jats-fmt`/`tei-fmt` (the other well-nested-XML format
//! crates in this workspace) for that plumbing, but — unlike those three,
//! which model "any well-formed XML document" generically because their
//! formats have hundreds of document-specific element names — OPML has a
//! small, fixed grammar, so its AST and event vocabulary are domain-typed
//! (`Head`/`Body`/`Outline`, `StartOutline`/`HeadField`/...) rather than a
//! generic element tree. See `ast.rs`'s module docs for the rationale.

pub mod ast;
pub mod batch;
pub mod emit;
pub mod events;
pub mod parse;
pub mod writer;

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use ast::{Body, Diagnostic, Head, OpmlDoc, Outline, Span, XmlDecl};
pub use batch::{BatchParser, Handler, StreamingParser};
pub use emit::emit;
pub use events::{Event, EventIter, OwnedEvent};
pub use parse::parse;
pub use writer::Writer;

/// Return a true streaming event iterator over the document, without
/// building an `OpmlDoc`.
pub fn events(input: &[u8]) -> EventIter<'_> {
    EventIter::new(input)
}

#[cfg(test)]
mod smoke {
    use super::*;

    const SAMPLE: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<opml version="2.0">
  <head>
    <title>Sample</title>
    <ownerName>Alice</ownerName>
  </head>
  <body>
    <outline text="Parent">
      <outline text="Child" xmlUrl="https://example.com/feed.xml"/>
    </outline>
    <outline text="Sibling" isComment="true"/>
  </body>
</opml>"#;

    #[test]
    fn smoke_parse_document() {
        let (doc, diags) = parse(SAMPLE);
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert_eq!(doc.version, "2.0");
        assert_eq!(doc.head.title.as_deref(), Some("Sample"));
        assert_eq!(doc.body.outlines.len(), 2);
        assert_eq!(doc.body.outlines[0].children.len(), 1);
        assert_eq!(
            doc.body.outlines[0].children[0].xml_url(),
            Some("https://example.com/feed.xml")
        );
        assert_eq!(doc.body.outlines[1].is_comment(), Some(true));
    }

    #[test]
    fn smoke_roundtrip() {
        let (doc1, diags1) = parse(SAMPLE);
        assert!(diags1.is_empty());
        let emitted = emit(&doc1);
        let (doc2, diags2) = parse(&emitted);
        assert!(diags2.is_empty(), "diagnostics: {diags2:?}");
        assert_eq!(doc1.strip_spans(), doc2.strip_spans(), "roundtrip mismatch");
    }

    #[test]
    fn smoke_events() {
        let evts: Vec<_> = events(SAMPLE).collect();
        assert!(!evts.is_empty());
        assert!(evts.iter().any(|e| matches!(e, Event::StartOutline { .. })));
        assert!(evts.iter().any(|e| matches!(e, Event::EmptyOutline { .. })));
    }

    #[test]
    fn smoke_event_roundtrip() {
        let (doc1, _) = parse(SAMPLE);
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
        let mid = SAMPLE.len() / 2;
        p.feed(&SAMPLE[..mid]);
        p.feed(&SAMPLE[mid..]);
        let (doc, diags) = p.finish();
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert_eq!(doc.body.outlines.len(), 2);
    }

    #[test]
    fn smoke_streaming_parser() {
        let mut evts = Vec::new();
        let mut p = StreamingParser::new(|ev| evts.push(ev));
        let mid = SAMPLE.len() / 2;
        p.feed(&SAMPLE[..mid]);
        p.feed(&SAMPLE[mid..]);
        let diags = p.finish();
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert!(
            evts.iter()
                .any(|e| matches!(e, Event::HeadField { name, .. } if name == "title"))
        );
    }

    #[test]
    fn smoke_writer() {
        use std::borrow::Cow;

        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(Event::StartOpml {
            version: Cow::Borrowed("2.0"),
        });
        w.write_event(Event::StartHead);
        w.write_event(Event::EndHead);
        w.write_event(Event::StartBody);
        w.write_event(Event::EmptyOutline {
            attrs: vec![("text".to_string(), Cow::Borrowed("Hi"))],
        });
        w.write_event(Event::EndBody);
        w.write_event(Event::EndOpml);
        let bytes = w.finish();
        let xml = String::from_utf8(bytes).unwrap();
        assert!(xml.contains(r#"<outline text="Hi"/>"#));
    }

    #[test]
    fn smoke_unknown_attributes_and_head_elements_round_trip() {
        let input = br#"<opml version="2.0"><head><title>T</title><futureField>x</futureField></head><body><outline text="A" appSpecific="v"/></body></opml>"#;
        let (doc, diags) = parse(input);
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert_eq!(doc.body.outlines[0].attr("appSpecific"), Some("v"));
        assert_eq!(
            doc.head.extra,
            vec![("futureField".to_string(), "x".to_string())]
        );
        let emitted = emit(&doc);
        let (doc2, diags2) = parse(&emitted);
        assert!(diags2.is_empty());
        assert_eq!(doc.strip_spans(), doc2.strip_spans());
    }
}
