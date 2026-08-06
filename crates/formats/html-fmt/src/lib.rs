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
//! # Streaming
//!
//! `events()` and `StreamingParser` are genuinely incremental: both are
//! driven by [`sink::IncrementalSink`], a custom `html5ever::TreeSink` that
//! emits events as the tokenizer/tree-builder produce them, not after a
//! full tree walk. HTML5's tree-construction algorithm can still
//! retroactively rearrange already-open nodes (foster parenting, adoption
//! agency) — a verified read of html5ever 0.36.1's `TreeSink` call sites
//! shows this is bounded to nodes still on the stack of open elements, not
//! the whole document — so events carry explicit node/parent ids and
//! structural correction events (`NodeReparented`, `ChildrenReparented`,
//! `NodeDetached`) cover those cases, the same "correction event" pattern
//! `commonmark-fmt` and `rtf-fmt` use for their own retroactive cases. See
//! the [`events`] module docs for the full design and verified rationale.

pub mod ast;
pub mod batch;
pub mod emit;
pub mod events;
mod ids;
pub mod parse;
mod sink;
pub mod writer;

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use ast::{Diagnostic, HtmlDoc, Node, Severity, Span, is_void_element};
pub use batch::{BatchParser, Handler, StreamingParser};
pub use emit::{EmitOptions, emit_fragment, emit_fragment_with_options, emit_with_options};
pub use events::{Event, EventIter, NodeId, OwnedEvent, collect_doc, events_from_doc};
pub use rescribe_format_api::{Emit, Events, Parse, StreamingParse, StreamingWrite};
pub use writer::Writer;

// ── Trait implementations ───────────────────────────────────────────────────

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
        events::events_from_input(input)
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
            node: NodeId(1),
            parent: NodeId::DOCUMENT,
            before_sibling: None,
            tag: Cow::Borrowed("p"),
            attrs: vec![],
            self_closing: false,
        });
        w.write_event(Event::Text {
            node: NodeId(2),
            parent: NodeId(1),
            before_sibling: None,
            content: Cow::Borrowed("Hello"),
        });
        w.write_event(Event::EndElement {
            node: NodeId(1),
            tag: Cow::Borrowed("p"),
        });
        let bytes = w.finish();
        assert_eq!(String::from_utf8(bytes).unwrap(), "<p>Hello</p>");
    }

    #[test]
    fn smoke_event_roundtrip() {
        let input = b"<!DOCTYPE html><html><head></head><body><p>Hello</p></body></html>";
        let (doc1, _) = HtmlDoc::parse(input);
        let evts = events::events_from_doc(&doc1);
        let doc2 = events::collect_doc(evts);
        assert_eq!(
            doc1.strip_spans(),
            doc2.strip_spans(),
            "event roundtrip mismatch"
        );
    }

    #[test]
    fn smoke_events_incremental_matches_parse() {
        let input = b"<!DOCTYPE html><html><head></head><body><p>Hello</p></body></html>";
        let (doc1, _) = HtmlDoc::parse(input);
        let evts: Vec<_> = HtmlDoc::events(input).collect();
        let doc2 = events::collect_doc(evts);
        assert_eq!(
            doc1.strip_spans(),
            doc2.strip_spans(),
            "incremental events() did not reconstruct the same tree as parse()"
        );
    }

    /// Foster parenting: `<div>` is not valid table content, so html5ever
    /// relocates it (and the misplaced text inside it) to just before the
    /// `<table>` rather than as the table's child. Verifies the incremental
    /// sink reproduces `parse()`'s tree exactly for this case.
    ///
    /// By design (see `crate::sink` module docs) this is absorbed entirely
    /// by `emit_first_attach`'s `before_sibling` positioning — html5ever
    /// decides the final location before ever calling `append` for the
    /// `<div>`, so no correction event is needed here. Asserted explicitly
    /// so a future change that starts emitting a spurious correction for
    /// this case is caught.
    #[test]
    fn foster_parenting_incremental_matches_parse_no_corrections() {
        let input = b"<table><div>foo</div></table>";
        let (doc1, _) = HtmlDoc::parse(input);
        let evts: Vec<_> = HtmlDoc::events(input).collect();
        assert!(
            !evts.iter().any(|e| matches!(
                e,
                Event::NodeReparented { .. }
                    | Event::ChildrenReparented { .. }
                    | Event::NodeDetached { .. }
            )),
            "foster parenting should be absorbed by first-attach positioning, not a \
             correction event: {evts:#?}"
        );
        let doc2 = events::collect_doc(evts);
        assert_eq!(
            doc1.strip_spans(),
            doc2.strip_spans(),
            "foster-parented tree mismatch between parse() and incremental events()"
        );
    }

    /// Adoption agency with a genuine "furthest block" (`<div>` sits between
    /// the misnested `<b>` and its out-of-place end tag) — the one case that
    /// actually exercises `NodeDetached`/`NodeReparented`/
    /// `ChildrenReparented`, per the verified html5ever 0.36.1 call-site
    /// trace in `crate::events`'s module docs. `<b><i></b></i>` and
    /// `<b>1<i>2</b>3</i></p>`-style formatting-only misnesting do NOT reach
    /// this path (no "special" category element sits between the two), so a
    /// `<div>` (block, "special") is required to actually observe the
    /// correction events.
    #[test]
    fn adoption_agency_fires_correction_events_and_matches_parse() {
        let input = b"<b>1<div>2</b>3</div>";
        let (doc1, _) = HtmlDoc::parse(input);
        let evts: Vec<_> = HtmlDoc::events(input).collect();
        assert!(
            evts.iter().any(|e| matches!(e, Event::NodeDetached { .. })),
            "expected a NodeDetached correction event: {evts:#?}"
        );
        assert!(
            evts.iter()
                .any(|e| matches!(e, Event::NodeReparented { .. })),
            "expected a NodeReparented correction event: {evts:#?}"
        );
        assert!(
            evts.iter()
                .any(|e| matches!(e, Event::ChildrenReparented { .. })),
            "expected a ChildrenReparented correction event: {evts:#?}"
        );
        let doc2 = events::collect_doc(evts);
        assert_eq!(
            doc1.strip_spans(),
            doc2.strip_spans(),
            "adoption-agency tree mismatch between parse() and incremental events()"
        );
    }

    /// Regression test for the verified `EndElement`-is-not-guaranteed gap
    /// documented on `Event::EndElement`: closing tags processed while
    /// foster-parenting is active route through an html5ever code path
    /// that never calls `TreeSink::pop`, so no `EndElement` fires for
    /// `div` or `table` here even though both are explicitly closed in the
    /// source. The tree is still correct (checked in
    /// `foster_parenting_incremental_matches_parse_no_corrections`) — this
    /// test exists so a future html5ever upgrade that starts calling `pop`
    /// here is noticed (this assertion would then start failing) rather
    /// than the gap silently drifting further from what's documented.
    #[test]
    fn end_element_not_guaranteed_for_fostered_content() {
        let input = b"<table><div>foo</div></table>";
        let evts: Vec<_> = HtmlDoc::events(input).collect();
        let end_tags: Vec<_> = evts
            .iter()
            .filter_map(|e| match e {
                Event::EndElement { tag, .. } => Some(tag.as_ref()),
                _ => None,
            })
            .collect();
        assert_eq!(
            end_tags,
            vec!["head", "body", "html"],
            "expected div/table to have no EndElement (verified html5ever behavior — see \
             Event::EndElement docs); got {end_tags:?}. If this now includes \"div\"/\"table\", \
             html5ever's behavior changed and the doc comment on Event::EndElement should be \
             updated."
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
