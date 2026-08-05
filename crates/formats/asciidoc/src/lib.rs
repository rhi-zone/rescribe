//! AsciiDoc parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-asciidoc` and `rescribe-write-asciidoc` as thin adapter layers.
//!
//! # API layers
//!
//! `AsciiDoc` implements four of the five shared `rescribe-format-api`
//! traits — `Emit`, `Events`, `StreamingParse`, `StreamingWrite`:
//!
//! ```text
//! use rescribe_format_api::{Emit, Events, StreamingParse, StreamingWrite};
//!
//! // Streaming reader
//! let it: EventIter = AsciiDoc::events(input); // input: &[u8]
//!
//! // Batch reader — chunk-driven
//! let mut p = AsciiDoc::streaming_parser(|ev| ...);
//! p.feed(chunk); // repeat
//! p.finish();
//!
//! // Builder writer
//! let bytes: Vec<u8> = doc.emit();
//!
//! // Streaming writer
//! let mut w = AsciiDoc::writer(sink);
//! w.write_event(event); // repeat
//! w.finish();
//! ```
//!
//! `Parse` (`AsciiDoc::parse(&[u8])`) is also implemented, but its native
//! parser (`parse::parse_str`) takes `&str`, not `&[u8]` — like
//! `commonmark-fmt`, the trait impl UTF-8-decodes internally (producing a
//! `Warning` diagnostic and an empty document on invalid UTF-8) and
//! delegates to it. `parse_str`/`events_str` remain public, documented
//! entry points alongside the trait (str input, no UTF-8 check needed) —
//! they are not redundant duplicates of the trait methods, which take
//! `&[u8]`.
//!
//! # Why `parse()` doesn't test what `rst-fmt`'s equivalent does
//!
//! `parse_str` and `events_str` share one parser (`EventIter::try_parse_block`)
//! under the hood — `parse_str` drives it directly, `events_str`/`events()`
//! layer AST-projection on top. See
//! `crates/rescribe-fixtures/tests/streaming_apis.rs`'s
//! `asciidoc_events_check` module docs for exactly what that means for
//! cross-checking the two APIs.

#![allow(clippy::collapsible_if)]

mod ast;
pub mod batch;
mod emit;
mod events;
mod parse;
pub mod writer;

// Re-exports
pub use ast::{
    AsciiDoc, AsciiDocError, Block, DefinitionItem, Diagnostic, ImageData, Inline, QuoteType,
    Severity, Span, TableRow,
};
pub use batch::{BatchParser, BatchSink, StreamingParser};
pub use events::{Event, EventIter, OwnedEvent};
pub use parse::{parse_inline_content, parse_str};
pub use rescribe_format_api::{Emit, Events, Handler, Parse, StreamingParse, StreamingWrite};
pub use writer::Writer;

/// Return a streaming event iterator over the AsciiDoc source.
///
/// Documented separate entry point alongside
/// [`Events::events`](rescribe_format_api::Events::events): takes `&str`
/// (no UTF-8 check), where the trait method takes `&[u8]`.
pub fn events_str(input: &str) -> EventIter<'_> {
    parse::EventIter::new(input)
}

// ── Trait implementations ───────────────────────────────────────────────────
//
// `AsciiDoc` implements the shared API-mode traits directly — no parallel
// free functions (`asciidoc::parse(&[u8])`, `asciidoc::emit(..)`, ...) exist
// alongside them. `parse_str`/`events_str` above stay public because they
// have a materially different contract (str vs bytes), matching
// `commonmark-fmt`'s precedent.

impl Parse for AsciiDoc {
    fn parse(input: &[u8]) -> (Self, Vec<Diagnostic>) {
        parse::parse(input)
    }
}

impl Emit for AsciiDoc {
    fn emit(&self) -> Vec<u8> {
        emit::build(self).into_bytes()
    }
}

impl Events for AsciiDoc {
    type Event<'a> = Event<'a>;
    type EventIter<'a> = EventIter<'a>;

    /// Invalid UTF-8 input yields an iterator over an empty document
    /// (`StartDocument`/`EndDocument` only), mirroring `commonmark-fmt`'s
    /// handling of the same case — the trait's `events()` is infallible by
    /// contract. Callers that need to distinguish "empty document" from
    /// "invalid UTF-8" should use [`events_str`] directly.
    fn events(input: &[u8]) -> EventIter<'_> {
        match std::str::from_utf8(input) {
            Ok(s) => EventIter::new(s),
            Err(_) => EventIter::new(""),
        }
    }
}

impl StreamingParse for AsciiDoc {
    type Event = OwnedEvent;
    type Parser<H: Handler<OwnedEvent>> = StreamingParser<H>;

    fn streaming_parser<H: Handler<OwnedEvent>>(handler: H) -> StreamingParser<H> {
        StreamingParser::new(handler)
    }
}

impl StreamingWrite for AsciiDoc {
    type Writer<W: std::io::Write> = Writer<W>;

    fn writer<W: std::io::Write>(sink: W) -> Writer<W> {
        Writer::new(sink)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_heading() {
        let (doc, _) = parse::parse_str("== Hello World\n\nSome text.");
        assert_eq!(doc.blocks.len(), 2);
        assert!(matches!(&doc.blocks[0], Block::Heading { level: 2, .. }));
    }

    #[test]
    fn test_parse_paragraph() {
        let (doc, _) = parse::parse_str("This is a paragraph.\n\nThis is another.");
        assert_eq!(doc.blocks.len(), 2);
        assert!(matches!(&doc.blocks[0], Block::Paragraph { .. }));
        assert!(matches!(&doc.blocks[1], Block::Paragraph { .. }));
    }

    #[test]
    fn test_parse_strong() {
        let (doc, _) = parse::parse_str("This is *strong* text.");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Strong(..))));
    }

    #[test]
    fn test_parse_emphasis() {
        let (doc, _) = parse::parse_str("This is _emphasized_ text.");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Emphasis(..))));
    }

    #[test]
    fn test_parse_bullet_list() {
        let (doc, _) = parse::parse_str("* First item\n* Second item\n* Third item");
        assert_eq!(doc.blocks.len(), 1);
        let Block::List { ordered, items, .. } = &doc.blocks[0] else {
            panic!("expected list");
        };
        assert!(!ordered);
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn test_parse_numbered_list() {
        let (doc, _) = parse::parse_str(". First item\n. Second item\n. Third item");
        assert_eq!(doc.blocks.len(), 1);
        let Block::List { ordered, items, .. } = &doc.blocks[0] else {
            panic!("expected list");
        };
        assert!(ordered);
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn test_parse_code_block() {
        let (doc, _) = parse::parse_str("[source,python]\n----\nprint('hello')\n----");
        assert_eq!(doc.blocks.len(), 1);
        let Block::CodeBlock {
            language, content, ..
        } = &doc.blocks[0]
        else {
            panic!("expected code block");
        };
        assert_eq!(language.as_deref(), Some("python"));
        assert!(content.contains("print('hello')"));
    }

    #[test]
    fn test_parse_inline_code() {
        let (doc, _) = parse::parse_str("Use `code here` in text.");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Code(..))));
    }

    #[test]
    fn test_parse_link() {
        let (doc, _) = parse::parse_str("Visit https://example.com[Example Site] for more.");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        let link = inlines.iter().find(|i| matches!(i, Inline::Link { .. }));
        assert!(link.is_some());
        if let Some(Inline::Link { url, .. }) = link {
            assert_eq!(url, "https://example.com");
        }
    }

    #[test]
    fn test_parse_block_image() {
        let (doc, _) = parse::parse_str("image::path/to/image.png[Alt text]");
        assert_eq!(doc.blocks.len(), 1);
        let Block::Figure { image, .. } = &doc.blocks[0] else {
            panic!("expected figure");
        };
        assert_eq!(image.url, "path/to/image.png");
        assert_eq!(image.alt.as_deref(), Some("Alt text"));
    }

    #[test]
    fn test_build_paragraph() {
        let doc = AsciiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text {
                    text: "Hello, world!".into(),
                    span: Span::NONE,
                }],
                id: None,
                role: None,
                checked: None,
                span: Span::NONE,
            }],
            attributes: Default::default(),
            span: Span::NONE,
        };
        let out = emit::build(&doc);
        assert!(out.contains("Hello, world!"));
    }

    #[test]
    fn test_build_heading() {
        let doc = AsciiDoc {
            blocks: vec![Block::Heading {
                level: 1,
                inlines: vec![Inline::Text {
                    text: "Title".into(),
                    span: Span::NONE,
                }],
                id: None,
                role: None,
                span: Span::NONE,
            }],
            attributes: Default::default(),
            span: Span::NONE,
        };
        let out = emit::build(&doc);
        assert!(out.contains("== Title"));
    }

    #[test]
    fn test_build_code_block() {
        let doc = AsciiDoc {
            blocks: vec![Block::CodeBlock {
                content: "print('hi')".into(),
                language: Some("python".into()),
                span: Span::NONE,
            }],
            attributes: Default::default(),
            span: Span::NONE,
        };
        let out = emit::build(&doc);
        assert!(out.contains("[source,python]"));
        assert!(out.contains("----"));
        assert!(out.contains("print('hi')"));
    }

    #[test]
    fn test_build_list() {
        let doc = AsciiDoc {
            blocks: vec![Block::List {
                ordered: false,
                items: vec![
                    vec![Block::Paragraph {
                        inlines: vec![Inline::Text {
                            text: "one".into(),
                            span: Span::NONE,
                        }],
                        id: None,
                        role: None,
                        checked: None,
                        span: Span::NONE,
                    }],
                    vec![Block::Paragraph {
                        inlines: vec![Inline::Text {
                            text: "two".into(),
                            span: Span::NONE,
                        }],
                        id: None,
                        role: None,
                        checked: None,
                        span: Span::NONE,
                    }],
                ],
                style: None,
                span: Span::NONE,
            }],
            attributes: Default::default(),
            span: Span::NONE,
        };
        let out = emit::build(&doc);
        assert!(out.contains("* one"));
        assert!(out.contains("* two"));
    }

    #[test]
    fn test_build_strong() {
        let doc = AsciiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Strong(
                    vec![Inline::Text {
                        text: "bold".into(),
                        span: Span::NONE,
                    }],
                    Span::NONE,
                )],
                id: None,
                role: None,
                checked: None,
                span: Span::NONE,
            }],
            attributes: Default::default(),
            span: Span::NONE,
        };
        let out = emit::build(&doc);
        assert!(out.contains("*bold*"));
    }

    #[test]
    fn test_build_emphasis() {
        let doc = AsciiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Emphasis(
                    vec![Inline::Text {
                        text: "italic".into(),
                        span: Span::NONE,
                    }],
                    Span::NONE,
                )],
                id: None,
                role: None,
                checked: None,
                span: Span::NONE,
            }],
            attributes: Default::default(),
            span: Span::NONE,
        };
        let out = emit::build(&doc);
        assert!(out.contains("_italic_"));
    }

    #[test]
    fn test_build_inline_code() {
        let doc = AsciiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Code("code".into(), Span::NONE)],
                id: None,
                role: None,
                checked: None,
                span: Span::NONE,
            }],
            attributes: Default::default(),
            span: Span::NONE,
        };
        let out = emit::build(&doc);
        assert!(out.contains("`code`"));
    }

    #[test]
    fn test_build_link() {
        let doc = AsciiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "https://example.com".into(),
                    children: vec![Inline::Text {
                        text: "click".into(),
                        span: Span::NONE,
                    }],
                    target: None,
                    span: Span::NONE,
                }],
                id: None,
                role: None,
                checked: None,
                span: Span::NONE,
            }],
            attributes: Default::default(),
            span: Span::NONE,
        };
        let out = emit::build(&doc);
        assert!(out.contains("https://example.com[click]"));
    }

    #[test]
    fn test_parse_table_basic() {
        let (doc, diags) = parse::parse_str("|===\n| Cell A | Cell B\n|===\n");
        assert!(diags.is_empty());
        assert_eq!(doc.blocks.len(), 1);
        let Block::Table { rows, .. } = &doc.blocks[0] else {
            panic!("expected table, got {:?}", doc.blocks[0]);
        };
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].cells.len(), 2);
        assert!(!rows[0].is_header);
    }

    #[test]
    fn test_parse_table_with_header() {
        let input = "|===\n| Col1 | Col2\n\n| Cell1\n| Cell2\n|===\n";
        let (doc, diags) = parse::parse_str(input);
        assert!(diags.is_empty());
        assert_eq!(doc.blocks.len(), 1);
        let Block::Table { rows, .. } = &doc.blocks[0] else {
            panic!("expected table");
        };
        // First group (before blank line) becomes header
        assert_eq!(rows.len(), 2);
        assert!(rows[0].is_header);
        assert!(!rows[1].is_header);
    }

    #[test]
    fn test_table_roundtrip() {
        // Build an arbitrary table AST and verify parse(emit(ast)) == ast
        let ast = AsciiDoc {
            blocks: vec![Block::Table {
                rows: vec![
                    TableRow {
                        cells: vec![
                            vec![Inline::Text {
                                text: "Col1".into(),
                                span: Span::NONE,
                            }],
                            vec![Inline::Text {
                                text: "Col2".into(),
                                span: Span::NONE,
                            }],
                        ],
                        is_header: false,
                    },
                    TableRow {
                        cells: vec![
                            vec![Inline::Text {
                                text: "A".into(),
                                span: Span::NONE,
                            }],
                            vec![Inline::Text {
                                text: "B".into(),
                                span: Span::NONE,
                            }],
                        ],
                        is_header: false,
                    },
                ],
                span: Span::NONE,
            }],
            attributes: Default::default(),
            span: Span::NONE,
        };
        let emitted = emit::build(&ast);
        let (reparsed, diags) = parse::parse_str(&emitted);
        assert!(diags.is_empty(), "unexpected diagnostics: {:?}", diags);
        assert_eq!(reparsed.strip_spans(), ast.strip_spans());
    }

    #[test]
    fn test_table_events() {
        let evs: Vec<_> = events_str("|===\n| A | B\n|===\n").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartTable)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndTable)));
        assert!(
            evs.iter()
                .any(|e| matches!(e, OwnedEvent::StartTableRow { .. }))
        );
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartTableCell)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndTableCell)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndTableRow)));
    }
}
