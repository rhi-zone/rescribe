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
            let emitted = emit(&doc);
            let emitted_str = std::str::from_utf8(&emitted).unwrap();
            let (doc2, diags) = parse(emitted_str);
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
