//! MediaWiki markup parser, AST, and emitter.
//!
//! A standalone crate with **no rescribe dependency by default**. The
//! optional `rescribe` feature (default off) adds `crate::rescribe::{parse,
//! emit}`, a thin adapter that translates `mediawiki_fmt::MediawikiDoc` to
//! and from rescribe's `Document` IR.
//!
//! # API layers
//!
//! `MediawikiDoc` implements the five shared `rescribe-format-api` traits:
//!
//! ```text
//! use rescribe_format_api::{Emit, Events, Parse, StreamingParse, StreamingWrite};
//!
//! // AST reader
//! let (doc, diags): (MediawikiDoc, Vec<Diagnostic>) = MediawikiDoc::parse(input);
//!
//! // Streaming reader -- iterator over owned events
//! let it: EventIter = MediawikiDoc::events(input);
//!
//! // Batch reader -- chunk-driven
//! let mut p = MediawikiDoc::streaming_parser(|ev| ...);
//! p.feed(chunk); // repeat
//! p.finish();
//!
//! // Builder writer -- emit from AST
//! let bytes: Vec<u8> = doc.emit();
//!
//! // Streaming writer -- emit from events
//! let mut w = MediawikiDoc::writer(sink);
//! w.write_event(event); // repeat
//! w.finish(); // flushes to sink
//! ```
//!
//! `parse::parse_str`/`events::events` (take `&str`, skip the UTF-8 check)
//! remain as separate, non-trait entry points: they have a materially
//! different contract from the trait methods (str input, no UTF-8
//! validation) and are real external-consumer entry points --
//! this crate's `rescribe` feature module and this workspace's fixture harness
//! (`crates/rescribe-fixtures/tests/streaming_apis.rs`) call `parse_str`/
//! `events` directly -- not redundant duplicates of the trait methods.

pub mod ast;
pub mod batch;
pub mod emit;
pub mod events;
pub mod parse;
#[cfg(feature = "rescribe")]
pub mod rescribe;
pub mod writer;

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use ast::{
    Block, DefinitionItem, Diagnostic, Inline, MediawikiDoc, Severity, Span, TableCell, TableRow,
};
pub use batch::{BatchParser, BatchSink, Handler, StreamingParser};
pub use events::{Event, EventIter, OwnedEvent, events};
pub use parse::parse_str;
pub use rescribe_format_api::{Emit, Events, Parse, StreamingParse, StreamingWrite};
pub use writer::Writer;

// ── Trait implementations ───────────────────────────────────────────────────
//
// `MediawikiDoc` implements the five shared API-mode traits directly -- the
// old crate-root `parse`/`emit` free functions (thin wrappers duplicating
// what is now the trait method) are gone. `parse_str`/`events` (str input)
// stay as documented crate API -- see the module doc above.

impl Parse for MediawikiDoc {
    fn parse(input: &[u8]) -> (Self, Vec<Diagnostic>) {
        parse::parse(input)
    }
}

impl Emit for MediawikiDoc {
    fn emit(&self) -> Vec<u8> {
        emit::emit(self).into_bytes()
    }
}

impl Events for MediawikiDoc {
    // `events::EventIter` always yields `OwnedEvent` and carries no
    // borrowed data of its own regardless of the input lifetime -- it
    // eagerly builds a `Vec<OwnedEvent>` in `EventIter::new` (see
    // `events.rs`'s doc comment on `EventIter` and the fixture harness's
    // `mediawiki_streaming_writer_matches_builder_over_all_fixtures` test,
    // which documents this as a known non-incremental gap, not something
    // this migration changes). The GAT tolerates an impl that doesn't use
    // its own lifetime parameter, same as `zip-fmt`'s `Archive`.
    type Event<'a> = OwnedEvent;
    type EventIter<'a> = EventIter;

    /// Invalid UTF-8 input yields an iterator over an empty document
    /// rather than panicking -- the trait's `events()` is infallible by
    /// contract and has no diagnostic channel (unlike `Parse::parse`).
    fn events(input: &[u8]) -> EventIter {
        match std::str::from_utf8(input) {
            Ok(s) => events::events(s),
            Err(_) => events::events(""),
        }
    }
}

impl StreamingParse for MediawikiDoc {
    type Event = OwnedEvent;
    type Parser<H: Handler<OwnedEvent>> = StreamingParser<H>;

    fn streaming_parser<H: Handler<OwnedEvent>>(handler: H) -> StreamingParser<H> {
        StreamingParser::new(handler)
    }
}

impl StreamingWrite for MediawikiDoc {
    type Writer<W: std::io::Write> = Writer<W>;

    fn writer<W: std::io::Write>(sink: W) -> Writer<W> {
        Writer::new(sink)
    }
}

// -- Tests --------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_text() {
        let (doc, _) = parse_str("Some simple text");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Paragraph { .. }));
    }

    #[test]
    fn test_parse_heading() {
        let (doc, _) = parse_str("== Heading ==");
        assert_eq!(doc.blocks.len(), 1);
        if let Block::Heading { level, .. } = &doc.blocks[0] {
            assert_eq!(*level, 2);
        } else {
            panic!("expected heading");
        }
    }

    #[test]
    fn test_parse_bold() {
        let (doc, _) = parse_str("'''bold'''");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Bold(_))));
    }

    #[test]
    fn test_parse_italic() {
        let (doc, _) = parse_str("''italic''");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Italic(_))));
    }

    #[test]
    fn test_parse_list() {
        let (doc, _) = parse_str("* Item 1\n* Item 2");
        assert_eq!(doc.blocks.len(), 1);
        let Block::List { ordered, items, .. } = &doc.blocks[0] else {
            panic!("expected list");
        };
        assert!(!ordered);
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_parse_internal_link() {
        let (doc, _) = parse_str("[[Title|Link text]]");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        let has_link = inlines.iter().any(|i| {
            if let Inline::Link { url, .. } = i {
                url == "Title"
            } else {
                false
            }
        });
        assert!(has_link);
    }

    #[test]
    fn test_parse_external_link() {
        let (doc, _) = parse_str("[https://example.com Example]");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        let has_link = inlines.iter().any(|i| {
            if let Inline::Link { url, .. } = i {
                url == "https://example.com"
            } else {
                false
            }
        });
        assert!(has_link);
    }

    #[test]
    fn test_parse_horizontal_rule() {
        let (doc, _) = parse_str("----");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::HorizontalRule));
    }

    #[test]
    fn test_parse_code_block() {
        let (doc, _) = parse_str(" code line 1\n code line 2");
        assert_eq!(doc.blocks.len(), 1);
        if let Block::CodeBlock { content, .. } = &doc.blocks[0] {
            assert!(content.contains("code line"));
        } else {
            panic!("expected code block");
        }
    }

    #[test]
    fn test_parse_blockquote() {
        let (doc, _) = parse_str("<blockquote>\nSome quoted text.\n</blockquote>");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Blockquote { .. }));
    }

    #[test]
    fn test_parse_definition_list() {
        let (doc, _) = parse_str("; Term\n: Definition");
        assert_eq!(doc.blocks.len(), 1);
        if let Block::DefinitionList { items, .. } = &doc.blocks[0] {
            assert_eq!(items.len(), 1);
        } else {
            panic!("expected definition list");
        }
    }

    #[test]
    fn test_parse_footnote_ref() {
        let (doc, _) = parse_str("Text<ref>footnote content</ref>.");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(
            i,
            Inline::FootnoteRef {
                content: Some(_),
                ..
            }
        )));
    }

    #[test]
    fn test_parse_math_inline() {
        let (doc, _) = parse_str("Solve <math>x^2</math>.");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(
            inlines
                .iter()
                .any(|i| matches!(i, Inline::MathInline { .. }))
        );
    }

    #[test]
    fn test_parse_nowiki() {
        let (doc, _) = parse_str("Some <nowiki>'''not bold'''</nowiki> text.");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Nowiki { .. })));
    }

    #[test]
    fn test_parse_template() {
        let (doc, _) = parse_str("See {{cite|author=Smith}}.");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Template { .. })));
    }

    #[test]
    fn test_parse_syntaxhighlight() {
        let (doc, _) =
            parse_str("<syntaxhighlight lang=\"python\">\nprint(\"hello\")\n</syntaxhighlight>");
        assert_eq!(doc.blocks.len(), 1);
        if let Block::CodeBlock {
            language, content, ..
        } = &doc.blocks[0]
        {
            assert_eq!(language.as_deref(), Some("python"));
            assert!(content.contains("print"));
        } else {
            panic!("expected code block, got {:?}", doc.blocks[0]);
        }
    }

    #[test]
    fn test_emit_heading() {
        let doc = MediawikiDoc {
            blocks: vec![Block::Heading {
                level: 2,
                inlines: vec![Inline::Text("Title".into())],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = crate::emit::emit(&doc);
        assert!(out.contains("== Title =="));
    }

    #[test]
    fn test_emit_bold() {
        let doc = MediawikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Bold(vec![Inline::Text("bold".into())])],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = crate::emit::emit(&doc);
        assert!(out.contains("'''bold'''"));
    }

    #[test]
    fn test_emit_italic() {
        let doc = MediawikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Italic(vec![Inline::Text("italic".into())])],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = crate::emit::emit(&doc);
        assert!(out.contains("''italic''"));
    }

    #[test]
    fn test_emit_list() {
        let doc = MediawikiDoc {
            blocks: vec![Block::List {
                ordered: false,
                items: vec![
                    vec![Block::Paragraph {
                        inlines: vec![Inline::Text("Item 1".into())],
                        span: Span::NONE,
                    }],
                    vec![Block::Paragraph {
                        inlines: vec![Inline::Text("Item 2".into())],
                        span: Span::NONE,
                    }],
                ],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = crate::emit::emit(&doc);
        assert!(out.contains("* Item 1"));
        assert!(out.contains("* Item 2"));
    }

    #[test]
    fn test_emit_internal_link() {
        let doc = MediawikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "Title".to_string(),
                    text: "Link text".to_string(),
                }],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = crate::emit::emit(&doc);
        assert!(out.contains("[[Title|Link text]]"));
    }

    #[test]
    fn test_emit_external_link() {
        let doc = MediawikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "https://example.com".to_string(),
                    text: "Example".to_string(),
                }],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = crate::emit::emit(&doc);
        assert!(out.contains("[https://example.com Example]"));
    }

    #[test]
    fn test_roundtrip_simple() {
        let input = "== Heading ==\n\nSome '''bold''' text.";
        let (doc, _) = parse_str(input);
        let output = crate::emit::emit(&doc);
        // Output should be parseable again
        let (doc2, _) = parse_str(&output);
        assert!(!doc2.blocks.is_empty());
    }
}
