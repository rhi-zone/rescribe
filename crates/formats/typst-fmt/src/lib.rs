//! Typst markup parser, AST, and emitter.
//!
//! A standalone crate wrapping `typst-syntax` with **no rescribe dependency
//! by default** — usable as a general Rust Typst library by anything that
//! needs to read or write Typst markup: a static-site generator, a linter,
//! a document-diffing tool, a search indexer. The optional `rescribe`
//! feature (default off) adds `crate::rescribe::{parse, emit}`, a thin
//! adapter that translates `typst_fmt::TypstDoc` to and from rescribe's
//! `Document` IR; the further optional `eval` feature (implies `rescribe`
//! and `reader-ast`) adds `crate::rescribe::parse_evaluated`, which drives
//! the full `typst` compiler (resolving `#let`/`#for`/`#if`/show rules,
//! etc.) instead of the syntax-only parse.
//!
//! # API layers
//!
//! `TypstDoc` implements all five shared `rescribe-format-api` traits:
//!
//! ```text
//! use rescribe_format_api::{Emit, Events, Parse, StreamingParse, StreamingWrite};
//!
//! // AST reader
//! let (doc, diags): (TypstDoc, Vec<Diagnostic>) = TypstDoc::parse(input);
//!
//! // Streaming reader — cursor-based walk over the parsed tree
//! let it: EventIter = TypstDoc::events(input);
//!
//! // Batch reader — chunk-driven, buffers to finish() (see batch.rs docs
//! // and "Limitations" below: no chunk-fed parse API exists upstream)
//! let mut p = TypstDoc::streaming_parser(|ev| ...);
//! p.feed(chunk); // repeat
//! p.finish();
//!
//! // Builder writer — emit from AST
//! let bytes: Vec<u8> = doc.emit();
//!
//! // Streaming writer — emit from events, incrementally
//! let mut w = TypstDoc::writer(sink);
//! w.write_event(event); // repeat
//! w.finish();
//! ```
//!
//! `parse_str`/`events_str` (take `&str`, skip the UTF-8 check that the
//! `&[u8]`-taking trait methods perform) remain as separate, non-trait
//! entry points: they have a materially different contract from the trait
//! methods, not a redundant duplicate of them.
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
//! `events.rs`'s module docs.
//!
//! # Limitations
//!
//! [`batch::StreamingParser`] (the `StreamingParse::Parser` behind
//! [`TypstDoc::streaming_parser`](rescribe_format_api::StreamingParse))
//! buffers all input until `finish()` — `typst-syntax`'s only from-scratch
//! parse entry points (`parse`, `parse_code`, `parse_math`) require the
//! full input as a `&str`; there is no chunk-fed parse API upstream
//! (`reparse`/`Source::edit` are edit-based *re*parsing of an
//! already-built tree for editor-style incremental updates, not applicable
//! to parsing from nothing). This is a second sanctioned "buffer all
//! input" exemption alongside `commonmark-fmt`'s pulldown-cmark, see
//! `batch.rs`'s module docs and CLAUDE.md's "-fmt crates are not rescribe
//! internals" section. The `StreamingParse` trait itself is implemented
//! (the trait only names the constructor/`Handler` shape, which
//! `batch::StreamingParser<H: Handler<Event>>` already satisfies) — the
//! buffering behavior is preserved, not "fixed", by that impl.
//!
//! Superseding `typst-syntax` is a non-goal.

pub mod ast;
pub mod batch;
pub mod emit;
pub mod events;
pub mod parse;
#[cfg(feature = "rescribe")]
pub mod rescribe;
pub mod writer;

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use ast::{Block, Diagnostic, Inline, Severity, Span, TypstDoc};
pub use batch::{BatchParser, Handler, StreamingParser};
pub use events::{Event, EventIter, collect_doc, events_from_doc, events_str};
pub use parse::parse_str;
pub use writer::Writer;

// ── Trait implementations ───────────────────────────────────────────────────
//
// `TypstDoc` implements all five shared API-mode traits directly — the old
// crate-root `parse`/`emit`/`events` free functions (thin wrappers
// duplicating what are now trait methods) are gone. `parse_str`/
// `events_str` (`&str`, skipping the UTF-8 check) stay as documented crate
// API: they have a materially different contract from the trait methods
// (str input vs. the trait's `&[u8]`), not redundant duplicates.
//
// `StreamingParse` wraps `batch::StreamingParser` as-is — see this file's
// module-level "Limitations" section for the buffering exemption that
// implementation carries (mirrors `commonmark-fmt`'s identical exemption
// for pulldown-cmark). Implementing the trait does not change that
// behavior; it only exposes the existing constructor generically.

impl rescribe_format_api::Parse for TypstDoc {
    /// Invalid UTF-8 input produces an empty document with a single
    /// `Warning` diagnostic (Typst source is always text) rather than
    /// panicking — mirrors `commonmark-fmt`'s `Parse::parse` behavior.
    fn parse(input: &[u8]) -> (Self, Vec<Diagnostic>) {
        match std::str::from_utf8(input) {
            Ok(s) => parse::parse_str(s),
            Err(e) => (
                TypstDoc::default(),
                vec![Diagnostic {
                    span: Span::NONE,
                    severity: Severity::Warning,
                    message: format!("input is not valid UTF-8: {e}"),
                    code: "",
                }],
            ),
        }
    }
}

impl rescribe_format_api::Emit for TypstDoc {
    fn emit(&self) -> Vec<u8> {
        emit::emit(self)
    }
}

impl rescribe_format_api::Events for TypstDoc {
    type Event<'a> = Event;
    type EventIter<'a> = EventIter;

    /// Invalid UTF-8 input yields an iterator over an empty document
    /// (`StartDocument`/`EndDocument` only) rather than panicking or
    /// returning `Option` — the trait's `events()` is infallible by
    /// contract and has no diagnostic channel (unlike `Parse::parse`).
    /// Callers that need to *distinguish* "empty document" from "invalid
    /// UTF-8" should use [`events::events_str`] directly on pre-validated
    /// `&str` input.
    fn events(input: &[u8]) -> EventIter {
        match std::str::from_utf8(input) {
            Ok(s) => events::events_str(s),
            Err(_) => events::events_str(""),
        }
    }
}

impl rescribe_format_api::StreamingParse for TypstDoc {
    type Event = Event;
    type Parser<H: Handler<Event>> = StreamingParser<H>;

    fn streaming_parser<H: Handler<Event>>(handler: H) -> StreamingParser<H> {
        StreamingParser::new(handler)
    }
}

impl rescribe_format_api::StreamingWrite for TypstDoc {
    type Writer<W: std::io::Write> = Writer<W>;

    fn writer<W: std::io::Write>(sink: W) -> Writer<W> {
        Writer::new(sink)
    }
}

#[cfg(test)]
mod smoke {
    use super::*;
    use rescribe_format_api::{Emit, Events, Parse};

    const SAMPLE: &str = "= Title\n\nHello *bold* and _em_ world.\n\n- one\n- two\n";

    #[test]
    fn smoke_parse_document() {
        let (doc, diags) = TypstDoc::parse(SAMPLE.as_bytes());
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert_eq!(doc.blocks.len(), 3);
        assert!(matches!(doc.blocks[0], Block::Heading { level: 1, .. }));
        assert!(matches!(doc.blocks[1], Block::Paragraph(_)));
        assert!(matches!(doc.blocks[2], Block::List { ordered: false, .. }));
    }

    #[test]
    fn smoke_roundtrip() {
        let (doc1, diags1) = TypstDoc::parse(SAMPLE.as_bytes());
        assert!(diags1.is_empty());
        let emitted = doc1.emit();
        let emitted_str = std::str::from_utf8(&emitted).unwrap();
        let (doc2, diags2) = TypstDoc::parse(emitted_str.as_bytes());
        assert!(diags2.is_empty(), "diagnostics: {diags2:?}");
        assert_eq!(doc1.strip_spans(), doc2.strip_spans(), "roundtrip mismatch");
    }

    #[test]
    fn smoke_events() {
        let evts: Vec<_> = TypstDoc::events(SAMPLE.as_bytes()).collect();
        assert!(!evts.is_empty());
        assert!(evts.contains(&Event::StartHeading { level: 1 }));
        assert!(evts.contains(&Event::StartList { ordered: false }));
    }

    #[test]
    fn smoke_event_roundtrip() {
        let (doc1, _) = TypstDoc::parse(SAMPLE.as_bytes());
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
        let (doc, _diags) = TypstDoc::parse(SAMPLE.as_bytes());
        let expected = events_from_doc(&doc);
        let actual: Vec<_> = TypstDoc::events(SAMPLE.as_bytes()).collect();
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

    /// Directly exercises the same roundtrip property the crate's
    /// `typst_fmt_roundtrip` fuzz target checks
    /// (`parse(emit(arbitrary_ast)).strip_spans() == arbitrary_ast`), once
    /// per construct, as a fast in-tree sanity check that does not depend
    /// on `cargo-fuzz` being installed.
    #[test]
    fn roundtrip_covers_every_construct() {
        let cases: Vec<TypstDoc> = vec![
            TypstDoc {
                blocks: vec![Block::Heading {
                    level: 3,
                    body: vec![Inline::Text("Hi".into())],
                }],
                span: Span::NONE,
            },
            TypstDoc {
                blocks: vec![Block::CodeBlock {
                    lang: Some("rust".into()),
                    content: "fn main() {}".into(),
                }],
                span: Span::NONE,
            },
            TypstDoc {
                blocks: vec![Block::List {
                    ordered: true,
                    items: vec![
                        vec![Block::Paragraph(vec![Inline::Text("a".into())])],
                        vec![Block::Paragraph(vec![Inline::Text("b".into())])],
                    ],
                }],
                span: Span::NONE,
            },
            TypstDoc {
                blocks: vec![Block::DefinitionList(vec![(
                    vec![Inline::Text("term".into())],
                    vec![Inline::Text("desc".into())],
                )])],
                span: Span::NONE,
            },
            TypstDoc {
                blocks: vec![Block::Quote(vec![Block::Paragraph(vec![Inline::Text(
                    "quoted".into(),
                )])])],
                span: Span::NONE,
            },
            TypstDoc {
                blocks: vec![Block::Table {
                    columns: 2,
                    rows: vec![vec![
                        vec![Inline::Text("a".into())],
                        vec![Inline::Text("b".into())],
                    ]],
                }],
                span: Span::NONE,
            },
            TypstDoc {
                blocks: vec![Block::Figure {
                    body: Some(Box::new(Block::Image {
                        url: "cat.png".into(),
                    })),
                    caption: Some(vec![Inline::Text("A cat".into())]),
                }],
                span: Span::NONE,
            },
            // Block::HorizontalRule is intentionally excluded here: parse()
            // has no construct-recognition path that produces it (an
            // unrecognized `#line(...)` function call currently falls
            // through the generic "unknown function" raw-block capture,
            // which does not reliably preserve a horizontal-rule-shaped
            // Raw payload) — a real, already-documented gap (see
            // `fixtures/typst/COVERAGE.md`'s "horizontal line" row and
            // `ast.rs`'s module docs), not something to paper over here.
            // Block::MathDisplay is likewise excluded: a standalone
            // block-level `$ ... $` is parsed as a paragraph containing an
            // *inline*-positioned `Inline::MathDisplay` (see that variant's
            // doc comment — it preserves the pre-existing adapter's
            // "don't split the paragraph around a block equation"
            // behavior), never as its own top-level `Block::MathDisplay`.
            // That variant exists for the writer side only today.
            TypstDoc {
                blocks: vec![Block::Paragraph(vec![
                    Inline::Strong(vec![Inline::Text("b".into())]),
                    Inline::Emph(vec![Inline::Text("i".into())]),
                    Inline::Underline(vec![Inline::Text("u".into())]),
                    Inline::Strike(vec![Inline::Text("s".into())]),
                    Inline::Subscript(vec![Inline::Text("sub".into())]),
                    Inline::Superscript(vec![Inline::Text("sup".into())]),
                    Inline::Code("code".into()),
                    Inline::Link {
                        url: "https://example.com".into(),
                        body: vec![Inline::Text("link".into())],
                    },
                    // A `\` linebreak in Typst source must be followed by
                    // whitespace to be recognized at all, and that
                    // whitespace is *itself* re-tokenized as a separate
                    // space on reparse — an inherent Typst grammar
                    // property, not a bug here. The explicit `Text(" ")`
                    // below is what a roundtrip actually produces; leaving
                    // it implicit would make this test assert something
                    // Typst's own syntax cannot roundtrip.
                    Inline::LineBreak,
                    Inline::Text(" ".into()),
                    Inline::MathInline("y".into()),
                    Inline::Footnote(vec![Inline::Text("note".into())]),
                    Inline::SmallCaps(vec![Inline::Text("caps".into())]),
                    // Inline::Quoted is intentionally not exercised here —
                    // see its doc comment in ast.rs: it is a writer-only
                    // construct with no reader-side production path, so it
                    // fails the "parse(emit(x)) == x" property by
                    // construction, not by bug. `rescribe-write-typst`'s
                    // own tests cover its emit output directly instead.
                ])],
                span: Span::NONE,
            },
        ];

        for doc in cases {
            let emitted = doc.emit();
            let emitted_str = std::str::from_utf8(&emitted).unwrap();
            let (doc2, diags) = TypstDoc::parse(emitted_str.as_bytes());
            assert!(
                diags.is_empty(),
                "diagnostics for {emitted_str:?}: {diags:?}"
            );
            assert_eq!(
                doc.strip_spans(),
                doc2.strip_spans(),
                "roundtrip mismatch for {emitted_str:?}"
            );
        }
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
