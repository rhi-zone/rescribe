//! Creole wiki parser, AST, and builder.
//!
//! Standalone crate with **no rescribe dependency by default**. The optional
//! `rescribe` feature (default off) adds `crate::rescribe::{parse, emit}`, a
//! thin adapter that translates `creole::CreoleDoc` to and from rescribe's
//! `Document` IR.

pub mod ast;
pub mod emit;
pub mod parse;

#[cfg(feature = "reader-streaming")]
pub mod events;

#[cfg(feature = "reader-batch")]
pub mod batch;

#[cfg(feature = "rescribe")]
pub mod rescribe;

#[cfg(feature = "writer-streaming")]
pub mod writer;

pub use ast::{
    Block, CreoleDoc, DefinitionItem, Diagnostic, Inline, Severity, Span, TableCell, TableRow,
};
pub use emit::collect_inline_text;
pub use parse::parse;
pub use rescribe_format_api::{Emit, Parse};

#[cfg(feature = "reader-streaming")]
pub use events::{Event, EventIter, OwnedEvent};
#[cfg(feature = "reader-streaming")]
pub use rescribe_format_api::Events;

#[cfg(feature = "reader-batch")]
pub use batch::{BatchParser, BatchSink, Handler, StreamingParser};
#[cfg(feature = "reader-batch")]
pub use rescribe_format_api::StreamingParse;

#[cfg(feature = "writer-streaming")]
pub use rescribe_format_api::StreamingWrite;
#[cfg(feature = "writer-streaming")]
pub use writer::Writer;

/// Parse `input` and return a streaming iterator of events.
#[cfg(feature = "reader-streaming")]
pub fn events(input: &str) -> events::EventIter {
    events::events(input)
}

// ── Trait implementations ───────────────────────────────────────────────────
//
// `parse::parse`/`events()` (both `&str`-input) stay as public, documented
// non-trait entry points — materially different contract from the shared
// `&[u8]`-input trait methods (they skip the UTF-8 validity check), mirroring
// commonmark-fmt's `parse_str`/`events_str`. `CreoleDoc::parse`/
// `CreoleDoc::events` bridge from `&[u8]` via `std::str::from_utf8`: invalid
// UTF-8 produces a `Warning` diagnostic for `Parse::parse` (which has a
// diagnostic channel) and an empty-document iterator for `Events::events`
// (which, being infallible-by-contract, does not) — the same split
// commonmark-fmt's `Events` impl documents.

impl Parse for CreoleDoc {
    fn parse(input: &[u8]) -> (Self, Vec<Diagnostic>) {
        match std::str::from_utf8(input) {
            Ok(s) => parse::parse(s),
            Err(_) => (
                CreoleDoc::default(),
                vec![
                    Diagnostic::new(Severity::Warning, "input is not valid UTF-8")
                        .with_code("creole::invalid-utf8"),
                ],
            ),
        }
    }
}

impl Emit for CreoleDoc {
    fn emit(&self) -> Vec<u8> {
        emit::build(self).into_bytes()
    }
}

#[cfg(feature = "reader-streaming")]
impl Events for CreoleDoc {
    type Event<'a> = OwnedEvent;
    type EventIter<'a> = EventIter;

    fn events(input: &[u8]) -> EventIter {
        match std::str::from_utf8(input) {
            Ok(s) => events::EventIter::new(s),
            Err(_) => events::EventIter::new(""),
        }
    }
}

#[cfg(feature = "reader-batch")]
impl StreamingParse for CreoleDoc {
    type Event = OwnedEvent;
    type Parser<H: Handler<OwnedEvent>> = StreamingParser<H>;

    fn streaming_parser<H: Handler<OwnedEvent>>(handler: H) -> StreamingParser<H> {
        StreamingParser::new(handler)
    }
}

#[cfg(feature = "writer-streaming")]
impl StreamingWrite for CreoleDoc {
    type Writer<W: std::io::Write> = Writer<W>;

    fn writer<W: std::io::Write>(sink: W) -> Writer<W> {
        Writer::new(sink)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use emit::build;

    #[test]
    fn test_parse_heading() {
        let (doc, _) = parse("= Title\n");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Heading { .. }));
        if let Block::Heading { level, .. } = &doc.blocks[0] {
            assert_eq!(*level, 1);
        }
    }

    #[test]
    fn test_parse_heading_levels() {
        let (doc, _) = parse("== Level 2\n=== Level 3\n");
        assert_eq!(doc.blocks.len(), 2);
        if let Block::Heading { level, .. } = &doc.blocks[0] {
            assert_eq!(*level, 2);
        }
        if let Block::Heading { level, .. } = &doc.blocks[1] {
            assert_eq!(*level, 3);
        }
    }

    #[test]
    fn test_parse_paragraph() {
        let (doc, _) = parse("Hello world\n");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Paragraph { .. }));
    }

    #[test]
    fn test_parse_bold() {
        let (doc, _) = parse("**bold**\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert_eq!(inlines.len(), 1);
        assert!(matches!(inlines[0], Inline::Bold(_, _)));
    }

    #[test]
    fn test_parse_italic() {
        let (doc, _) = parse("//italic//\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert_eq!(inlines.len(), 1);
        assert!(matches!(inlines[0], Inline::Italic(_, _)));
    }

    #[test]
    fn test_parse_link() {
        let (doc, _) = parse("[[https://example.com|Example]]\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        let link = &inlines[0];
        assert!(matches!(link, Inline::Link { .. }));
        if let Inline::Link { url, .. } = link {
            assert_eq!(url, "https://example.com");
        }
    }

    #[test]
    fn test_parse_list() {
        let (doc, _) = parse("* item1\n* item2\n");
        assert_eq!(doc.blocks.len(), 1);
        let Block::List { items, .. } = &doc.blocks[0] else {
            panic!("expected list");
        };
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_parse_nowiki() {
        let (doc, _) = parse("{{{code}}}\n");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::CodeBlock { .. }));
    }

    #[test]
    fn test_parse_definition_list() {
        let (doc, _) = parse("; Term\n: Definition\n");
        assert_eq!(doc.blocks.len(), 1);
        let Block::DefinitionList { items, .. } = &doc.blocks[0] else {
            panic!("expected definition list");
        };
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn test_parse_blockquote() {
        let (doc, _) = parse("> quoted text\n");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Blockquote { .. }));
    }

    #[test]
    fn test_parse_escape() {
        let (doc, _) = parse("~**not bold~**\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        // Should have a single text node with literal **not bold**
        assert_eq!(inlines.len(), 1);
        if let Inline::Text(s, _) = &inlines[0] {
            assert_eq!(s, "**not bold**");
        } else {
            panic!("expected text, got {:?}", inlines[0]);
        }
    }

    #[test]
    fn test_build_heading() {
        let doc = CreoleDoc {
            blocks: vec![Block::Heading {
                level: 1,
                inlines: vec![Inline::Text("Title".into(), Span::NONE)],
                span: Span::NONE,
            }],
        };
        let output = build(&doc);
        assert!(output.contains("= Title ="));
    }

    #[test]
    fn test_build_paragraph() {
        let doc = CreoleDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Hello, world!".into(), Span::NONE)],
                span: Span::NONE,
            }],
        };
        let output = build(&doc);
        assert!(output.contains("Hello, world!"));
    }

    #[test]
    fn test_build_bold() {
        let doc = CreoleDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Bold(
                    vec![Inline::Text("bold".into(), Span::NONE)],
                    Span::NONE,
                )],
                span: Span::NONE,
            }],
        };
        let output = build(&doc);
        assert!(output.contains("**bold**"));
    }

    #[test]
    fn test_build_italic() {
        let doc = CreoleDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Italic(
                    vec![Inline::Text("italic".into(), Span::NONE)],
                    Span::NONE,
                )],
                span: Span::NONE,
            }],
        };
        let output = build(&doc);
        assert!(output.contains("//italic//"));
    }

    #[test]
    fn test_build_code() {
        let doc = CreoleDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Code("code".into(), Span::NONE)],
                span: Span::NONE,
            }],
        };
        let output = build(&doc);
        assert!(output.contains("{{{code}}}"));
    }

    #[test]
    fn test_build_link() {
        let doc = CreoleDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "https://example.com".into(),
                    children: vec![Inline::Text("click".into(), Span::NONE)],
                    span: Span::NONE,
                }],
                span: Span::NONE,
            }],
        };
        let output = build(&doc);
        assert!(output.contains("[[https://example.com|click]]"));
    }

    #[test]
    fn test_build_list() {
        let doc = CreoleDoc {
            blocks: vec![Block::List {
                ordered: false,
                items: vec![
                    vec![Block::Paragraph {
                        inlines: vec![Inline::Text("one".into(), Span::NONE)],
                        span: Span::NONE,
                    }],
                    vec![Block::Paragraph {
                        inlines: vec![Inline::Text("two".into(), Span::NONE)],
                        span: Span::NONE,
                    }],
                ],
                span: Span::NONE,
            }],
        };
        let output = build(&doc);
        assert!(output.contains("* one"));
        assert!(output.contains("* two"));
    }

    #[test]
    fn test_build_ordered_list() {
        let doc = CreoleDoc {
            blocks: vec![Block::List {
                ordered: true,
                items: vec![
                    vec![Block::Paragraph {
                        inlines: vec![Inline::Text("first".into(), Span::NONE)],
                        span: Span::NONE,
                    }],
                    vec![Block::Paragraph {
                        inlines: vec![Inline::Text("second".into(), Span::NONE)],
                        span: Span::NONE,
                    }],
                ],
                span: Span::NONE,
            }],
        };
        let output = build(&doc);
        assert!(output.contains("# first"));
        assert!(output.contains("# second"));
    }

    #[test]
    fn test_build_code_block() {
        let doc = CreoleDoc {
            blocks: vec![Block::CodeBlock {
                content: "print('hi')".into(),
                span: Span::NONE,
            }],
        };
        let output = build(&doc);
        assert!(output.contains("{{{\n"));
        assert!(output.contains("print('hi')"));
        assert!(output.contains("}}}\n"));
    }

    #[test]
    fn test_build_definition_list() {
        let doc = CreoleDoc {
            blocks: vec![Block::DefinitionList {
                items: vec![DefinitionItem {
                    term: vec![Inline::Text("Term".into(), Span::NONE)],
                    desc: vec![Inline::Text("Definition".into(), Span::NONE)],
                }],
                span: Span::NONE,
            }],
        };
        let output = build(&doc);
        assert!(output.contains("; Term"));
        assert!(output.contains(": Definition"));
    }
}
