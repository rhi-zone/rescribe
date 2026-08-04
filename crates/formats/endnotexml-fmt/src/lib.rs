//! EndNote XML bibliography parser, AST, and emitter.
//!
//! A standalone crate wrapping `quick-xml` with **no rescribe dependency**
//! — usable as a general Rust EndNote XML library by anything that needs to
//! read or write EndNote's export format: a reference-manager import tool,
//! a citation indexer, a bibliography deduplication utility, a metadata
//! extraction pipeline. `rescribe-read-endnotexml` and
//! `rescribe-write-endnotexml` are thin adapter layers on top that
//! translate `endnotexml_fmt::EndNoteDoc` to and from rescribe's
//! `bibliography`/`bibliography_entry`/`bibliography_field` IR (ADR 0005).
//!
//! # API layers
//!
//! ```text
//! // AST reader
//! pub fn parse(input: &[u8]) -> (EndNoteDoc, Vec<Diagnostic>);
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
//! pub fn emit(doc: &EndNoteDoc) -> Vec<u8>;
//!
//! // Streaming writer — emit from events
//! let mut w = Writer::new(sink);
//! w.write_event(event); // repeat
//! w.finish(); // flushes to sink
//! ```
//!
//! # Design
//!
//! EndNote's schema is exporter-dependent (`custom1`..`custom7`,
//! `research-notes`, `work-type`, ...) on top of a documented core
//! vocabulary. That core (ref-type, contributors, titles, periodical,
//! pages, dates, urls, foreign-keys, keywords) is modeled directly as a
//! typed AST (`Record` with named fields), the same rationale `opml-fmt`
//! uses for OPML's small fixed grammar — see `ast.rs`'s module docs for the
//! full design, including how the genuinely open-ended parts (unknown
//! elements, `<style>` markup, incidental nested elements in field content)
//! are captured losslessly rather than dropped.
//!
//! Like `opml-fmt`, EndNote XML is well-nested XML by construction, so
//! `events()` and `StreamingParser` here are genuinely independent,
//! incremental implementations that never materialize an `EndNoteDoc` — see
//! `events.rs` and `batch.rs` module docs.

pub mod ast;
pub mod batch;
pub mod emit;
pub mod events;
pub mod parse;
pub mod writer;

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use ast::{
    AuthorRole, Contributors, Dates, Diagnostic, Element, EndNoteDoc, ForeignKey, ForeignKeys,
    Inline, Periodical, Record, RefType, Span, Titles, UrlRole, Urls, XmlDecl,
};
pub use batch::{BatchParser, Handler, StreamingParser};
pub use emit::emit;
pub use events::{Event, EventIter, OwnedEvent};
pub use parse::parse;
pub use writer::Writer;

/// Return a true streaming event iterator over the document, without
/// building an `EndNoteDoc`.
pub fn events(input: &[u8]) -> EventIter<'_> {
    EventIter::new(input)
}

#[cfg(test)]
mod smoke {
    use super::*;

    const SAMPLE: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<xml><records><record>
  <ref-type name="Journal Article">17</ref-type>
  <contributors><authors><author>Smith, John</author><author>Doe, Jane</author></authors></contributors>
  <titles><title>A Great Paper</title><secondary-title>Nature</secondary-title></titles>
  <dates><year>2020</year></dates>
  <volume>12</volume>
  <pages>100-110</pages>
  <urls><related-urls><url>https://example.com</url></related-urls></urls>
</record></records></xml>"#;

    #[test]
    fn smoke_parse_document() {
        let (doc, diags) = parse(SAMPLE);
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert_eq!(doc.records.len(), 1);
        let r = &doc.records[0];
        assert_eq!(r.ref_type.code, "17");
        assert_eq!(r.contributors.as_ref().unwrap().authors.len(), 2);
        assert_eq!(
            parse::flatten_inline_text(r.titles.as_ref().unwrap().title.as_ref().unwrap()),
            "A Great Paper"
        );
        assert_eq!(
            r.urls.as_ref().unwrap().related_urls[0],
            "https://example.com"
        );
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
        assert!(evts.iter().any(|e| matches!(e, Event::StartRecord)));
        assert!(
            evts.iter()
                .any(|e| matches!(e, Event::StartAuthorRole(AuthorRole::Authors)))
        );
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
    fn smoke_events_equals_ast_projection() {
        let (doc, _diags) = parse(SAMPLE);
        let expected = events::events_from_doc(&doc);
        let actual: Vec<_> = events(SAMPLE).map(|e| e.into_owned()).collect();
        assert_eq!(expected, actual);
    }

    #[test]
    fn smoke_batch_parser() {
        let mut p = BatchParser::new();
        let mid = SAMPLE.len() / 2;
        p.feed(&SAMPLE[..mid]);
        p.feed(&SAMPLE[mid..]);
        let (doc, diags) = p.finish();
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert_eq!(doc.records.len(), 1);
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
        assert!(evts.contains(&OwnedEvent::StartRecord));
    }

    #[test]
    fn smoke_writer() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(Event::StartDocument);
        w.write_event(Event::StartRecord);
        w.write_event(Event::StartElement {
            name: "ref-type".into(),
            attrs: vec![],
        });
        w.write_event(Event::Text("17".into()));
        w.write_event(Event::EndElement);
        w.write_event(Event::EndRecord);
        w.write_event(Event::EndDocument);
        let bytes = w.finish();
        let xml = String::from_utf8(bytes).unwrap();
        assert!(xml.contains("<ref-type>17</ref-type>"));
    }

    #[test]
    fn smoke_style_markup_roundtrips() {
        let input = br#"<xml><records><record>
  <ref-type>17</ref-type>
  <titles><title><style face="normal">A </style><style face="italic">Great</style><style face="normal"> Paper</style></title></titles>
</record></records></xml>"#;
        let (doc, diags) = parse(input);
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        let emitted = emit(&doc);
        let (doc2, diags2) = parse(&emitted);
        assert!(diags2.is_empty());
        assert_eq!(doc.strip_spans(), doc2.strip_spans());
    }

    #[test]
    fn smoke_unknown_elements_round_trip() {
        let input = br#"<xml><records><record>
  <ref-type>13</ref-type>
  <custom1>foo</custom1>
  <foreign-keys><key app="EN" db-id="abc">42</key></foreign-keys>
</record></records></xml>"#;
        let (doc, diags) = parse(input);
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert_eq!(doc.records[0].extra.len(), 1);
        assert_eq!(doc.records[0].extra[0].name, "custom1");
        let emitted = emit(&doc);
        let (doc2, diags2) = parse(&emitted);
        assert!(diags2.is_empty());
        assert_eq!(doc.strip_spans(), doc2.strip_spans());
    }
}
