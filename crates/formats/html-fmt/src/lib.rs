//! HTML5 parser, AST, and emitter.
//!
//! A standalone crate wrapping html5ever with **no rescribe dependency** —
//! usable as a general Rust HTML5 library. The `rescribe-read-html` and
//! `rescribe-write-html` crates are thin adapter layers on top.
//!
//! # API layers
//!
//! `HtmlDoc` implements the five shared `rescribe-format-api` traits:
//!
//! ```text
//! use rescribe_format_api::{Emit, Events, Parse, StreamingParse, StreamingWrite};
//!
//! // AST reader
//! let (doc, diags): (HtmlDoc, Vec<Diagnostic>) = HtmlDoc::parse(input);
//!
//! // Streaming reader — iterator over owned events
//! let it: EventIter = HtmlDoc::events(input);
//!
//! // Batch reader — chunk-driven
//! let mut p = BatchParser::new();
//! p.feed(chunk); // repeat
//! let (doc, diags) = p.finish();
//!
//! // Builder writer — emit from AST
//! let bytes: Vec<u8> = doc.emit();
//!
//! // Streaming writer — emit from events
//! let mut w = HtmlDoc::writer(sink);
//! w.write_event(event); // repeat
//! w.finish(); // flushes to sink
//! ```
//!
//! # Streaming limitation
//!
//! The HTML5 parsing algorithm requires tree construction for correctness
//! (foster parenting, implied elements, adoption agency). All three reader
//! APIs build the full parse tree internally. `events()` and
//! `StreamingParser` walk the tree to produce events after construction.
//! This is a fundamental limitation of the HTML5 spec, not a library choice.

pub mod ast;
pub mod batch;
pub mod emit;
pub mod events;
pub mod parse;
pub mod writer;

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use ast::{Diagnostic, HtmlDoc, Node, Severity, Span, is_void_element};
pub use batch::{BatchParser, Handler, StreamingParser};
pub use emit::{EmitOptions, emit_fragment, emit_fragment_with_options, emit_with_options};
pub use events::{Event, EventIter, OwnedEvent};
pub use rescribe_format_api::{Emit, Events, Parse, StreamingParse, StreamingWrite};
pub use writer::Writer;

// ── Trait implementations ───────────────────────────────────────────────────
//
// `HtmlDoc` implements the five shared API-mode traits. `events()` is
// necessarily *not* a genuinely independent streaming implementation here
// (see the module doc "Streaming limitation" above) — it builds the full
// parse tree and walks it, same as `parse()` — but that is an existing,
// documented, already-accepted crate-level limitation (HTML5's tree
// construction algorithm requires it), not something this migration
// introduces or needs to fix. The fixture harness already marks html-fmt's
// reader-side cross-API equivalence check `NotApplicable` for the same
// documented reason.

impl Parse for HtmlDoc {
    fn parse(input: &[u8]) -> (Self, Vec<Diagnostic>) {
        parse::parse(input)
    }
}

impl Emit for HtmlDoc {
    fn emit(&self) -> Vec<u8> {
        emit::emit(self)
    }
}

impl Events for HtmlDoc {
    type Event<'a> = OwnedEvent;
    type EventIter<'a> = EventIter;

    fn events(input: &[u8]) -> EventIter {
        let (doc, _) = parse::parse(input);
        events::events_from_doc(&doc)
    }
}

impl StreamingParse for HtmlDoc {
    type Event = OwnedEvent;
    type Parser<H: Handler<OwnedEvent>> = StreamingParser<H>;

    fn streaming_parser<H: Handler<OwnedEvent>>(handler: H) -> StreamingParser<H> {
        StreamingParser::new(handler)
    }
}

impl StreamingWrite for HtmlDoc {
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
        let input = b"<!DOCTYPE html><html><head></head><body><p>Hello</p></body></html>";
        let (doc, diags) = HtmlDoc::parse(input);
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        // Should have doctype + html element
        assert!(doc.nodes.len() >= 2);
    }

    #[test]
    fn smoke_parse_fragment() {
        let input = b"<p>Hello <em>world</em></p>";
        let (doc, _) = HtmlDoc::parse(input);
        // html5ever wraps in html/head/body
        assert!(!doc.nodes.is_empty());
    }

    #[test]
    fn smoke_roundtrip() {
        let input = b"<!DOCTYPE html><html><head></head><body><p>Hello</p></body></html>";
        let (doc1, _) = HtmlDoc::parse(input);
        let emitted = doc1.emit();
        let (doc2, _) = HtmlDoc::parse(&emitted);
        assert_eq!(doc1.strip_spans(), doc2.strip_spans(), "roundtrip mismatch");
    }

    #[test]
    fn smoke_events() {
        let input = b"<p>Hello</p>";
        let evts: Vec<_> = HtmlDoc::events(input).collect();
        assert!(!evts.is_empty());
        // Should contain StartElement for p
        assert!(
            evts.iter()
                .any(|e| matches!(e, Event::StartElement { tag, .. } if tag == "p"))
        );
    }

    #[test]
    fn smoke_batch_parser() {
        let mut p = BatchParser::new();
        p.feed(b"<h1>Hello</h1>");
        p.feed(b"<p>World</p>");
        let (doc, _) = p.finish();
        assert!(!doc.nodes.is_empty());
    }

    #[test]
    fn smoke_streaming_parser() {
        let mut evts = Vec::new();
        let mut p = StreamingParser::new(|ev| evts.push(ev));
        p.feed(b"<h1>Hello</h1>");
        p.feed(b"<p>World</p>");
        p.finish();
        assert!(
            evts.iter()
                .any(|e| matches!(e, Event::StartElement { tag, .. } if tag == "h1"))
        );
        assert!(
            evts.iter()
                .any(|e| matches!(e, Event::StartElement { tag, .. } if tag == "p"))
        );
    }

    #[test]
    fn smoke_writer() {
        use std::borrow::Cow;

        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(Event::StartElement {
            tag: Cow::Borrowed("p"),
            attrs: vec![],
            self_closing: false,
        });
        w.write_event(Event::Text(Cow::Borrowed("Hello")));
        w.write_event(Event::EndElement {
            tag: Cow::Borrowed("p"),
        });
        let bytes = w.finish();
        assert_eq!(String::from_utf8(bytes).unwrap(), "<p>Hello</p>");
    }

    #[test]
    fn smoke_event_roundtrip() {
        let input = b"<!DOCTYPE html><html><head></head><body><p>Hello</p></body></html>";
        let (doc1, _) = HtmlDoc::parse(input);
        let evts: Vec<_> = events::events_from_doc(&doc1).collect();
        let doc2 = events::collect_doc(evts);
        assert_eq!(
            doc1.strip_spans(),
            doc2.strip_spans(),
            "event roundtrip mismatch"
        );
    }

    #[test]
    fn smoke_escape() {
        let input = b"<p>&amp; &lt; &gt;</p>";
        let (doc, _) = HtmlDoc::parse(input);
        let emitted = doc.emit();
        let html = String::from_utf8(emitted).unwrap();
        // The text should contain & < > and be re-escaped in output
        assert!(html.contains("&amp;"));
        assert!(html.contains("&lt;"));
        assert!(html.contains("&gt;"));
    }

    #[test]
    fn smoke_void_elements() {
        let input = b"<p>Hello<br>World</p>";
        let (doc, _) = HtmlDoc::parse(input);
        let emitted = doc.emit();
        let html = String::from_utf8(emitted).unwrap();
        // <br> should not have a closing tag
        assert!(html.contains("<br>"));
        assert!(!html.contains("</br>"));
    }

    #[test]
    fn smoke_attributes() {
        let input = b"<a href=\"https://example.com\" class=\"link\">click</a>";
        let (doc, _) = HtmlDoc::parse(input);
        let emitted = doc.emit();
        let html = String::from_utf8(emitted).unwrap();
        assert!(html.contains("href=\"https://example.com\""));
        assert!(html.contains("class=\"link\""));
    }
}
