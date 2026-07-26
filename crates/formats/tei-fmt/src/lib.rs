//! TEI (Text Encoding Initiative) P5 and generic XML parser, AST, and
//! emitter.
//!
//! A standalone crate wrapping `quick-xml` with **no rescribe dependency**
//! — usable as a general Rust TEI/XML library. `rescribe-read-tei`
//! and `rescribe-write-tei` are thin adapter layers on top that
//! translate `tei_fmt::Node` to and from rescribe's IR; TEI-specific
//! *meaning* (which element names — `<div>`, `<hi rend="…">`, `<ref
//! target="…">`, `<teiHeader>`, etc. — map to which document semantics)
//! lives in those adapters, not here — this crate only knows XML structure.
//!
//! # API layers
//!
//! ```text
//! // AST reader
//! pub fn parse(input: &[u8]) -> (TeiDoc, Vec<Diagnostic>);
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
//! pub fn emit(doc: &TeiDoc) -> Vec<u8>;
//!
//! // Streaming writer — emit from events
//! let mut w = Writer::new(sink);
//! w.write_event(event); // repeat
//! w.finish(); // flushes to sink
//! ```
//!
//! # Why XML can stream where HTML can't
//!
//! `html-fmt`'s three reader APIs all build the full DOM internally because
//! HTML5 tree construction is allowed to rearrange previously-seen nodes
//! (foster parenting, adoption agency). XML has no such rule — it is
//! well-nested by construction — so `events()` and `StreamingParser` here
//! are genuinely independent, incremental implementations that never
//! materialize a `TeiDoc`. See `events.rs` and `batch.rs` module docs
//! for the chunk-boundary handling this requires (mainly: plain text is the
//! one token quick-xml doesn't distinguish from "still coming" vs "done").
//!
//! This crate mirrors the structure of `docbook-fmt` and `jats-fmt`, the
//! two other well-nested-XML format crates in this workspace — TEI, like
//! JATS and DocBook, is plain XML with no HTML5-style tree-construction
//! quirks, so the same generic-XML AST/event model applies verbatim.

pub mod ast;
pub mod batch;
pub mod emit;
pub mod events;
pub mod parse;
pub mod writer;

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use ast::{Diagnostic, Node, Span, TeiDoc, XmlDecl};
pub use batch::{BatchParser, Handler, StreamingParser};
pub use emit::{emit, emit_fragment};
pub use events::{Event, EventIter, OwnedEvent};
pub use parse::parse;
pub use writer::Writer;

/// Return a true streaming event iterator over the document, without
/// building a `TeiDoc`.
pub fn events(input: &[u8]) -> EventIter<'_> {
    EventIter::new(input)
}

#[cfg(test)]
mod smoke {
    use super::*;

    #[test]
    fn smoke_parse_document() {
        let input =
            br#"<?xml version="1.0"?><TEI><teiHeader/><text><body><p>Body</p></body></text></TEI>"#;
        let (doc, diags) = parse(input);
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert!(doc.root().is_some());
    }

    #[test]
    fn smoke_roundtrip() {
        let input =
            br#"<?xml version="1.0"?><TEI><text><body><p>A &amp; B</p></body></text></TEI>"#;
        let (doc1, _) = parse(input);
        let emitted = emit(&doc1);
        let (doc2, diags) = parse(&emitted);
        assert!(diags.is_empty());
        assert_eq!(doc1.strip_spans(), doc2.strip_spans(), "roundtrip mismatch");
    }

    #[test]
    fn smoke_events() {
        let input = b"<p>Hello <hi rend=\"italic\">world</hi></p>";
        let evts: Vec<_> = events(input).collect();
        assert!(!evts.is_empty());
        assert!(
            evts.iter()
                .any(|e| matches!(e, Event::StartElement { name, .. } if name == "p"))
        );
    }

    #[test]
    fn smoke_event_roundtrip() {
        let input = br#"<?xml version="1.0"?><TEI><text><body><p>Body</p></body></text></TEI>"#;
        let (doc1, _) = parse(input);
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
        p.feed(b"<TEI><text><body><p>Hello");
        p.feed(b"</p></body></text></TEI>");
        let (doc, diags) = p.finish();
        assert!(diags.is_empty());
        assert!(doc.root().is_some());
    }

    #[test]
    fn smoke_streaming_parser() {
        let mut evts = Vec::new();
        let mut p = StreamingParser::new(|ev| evts.push(ev));
        p.feed(b"<TEI><text><body><p>Hello");
        p.feed(b"</p></body></text></TEI>");
        let diags = p.finish();
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert!(
            evts.iter()
                .any(|e| matches!(e, Event::StartElement { name, .. } if name == "body"))
        );
        assert!(
            evts.iter()
                .any(|e| matches!(e, Event::Text(t) if t == "Hello"))
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
        p.feed(b"<h");
        p.feed(b"i rend=\"italic\">hi</hi>");
        let diags = p.finish();
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert!(
            evts.iter()
                .any(|e| matches!(e, Event::StartElement { name, .. } if name == "hi"))
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
        let (doc, _) = parse(input);
        let emitted = emit(&doc);
        let xml = String::from_utf8(emitted).unwrap();
        assert!(xml.contains("&amp;"));
        assert!(xml.contains("&lt;"));
        assert!(xml.contains("&gt;"));
    }

    #[test]
    fn smoke_self_closing() {
        let input = b"<p>Hello<lb/>World</p>";
        let (doc, _) = parse(input);
        let emitted = emit(&doc);
        let xml = String::from_utf8(emitted).unwrap();
        assert!(xml.contains("<lb/>"));
    }
}
