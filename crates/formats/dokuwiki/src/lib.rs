//! DokuWiki parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency by default. The optional
//! `rescribe` feature (default off) adds `crate::rescribe::{parse, emit}`,
//! a thin adapter that translates `dokuwiki::DokuwikiDoc` to and from
//! rescribe's `Document` IR.
//!
//! This crate denies `unsafe` by default (production code must never use it); a prior version of `events::InputEventIter`
//! used `unsafe { transmute }` to build a self-referential struct around a
//! locally-parsed `DokuwikiDoc`, and was unsound (the doc was moved into the
//! struct after the reference was taken). See `src/events.rs` for the sound,
//! eager-collection replacement and TODO.md for the tracked architecture gap.

#![deny(unsafe_code)]

pub mod ast;
pub mod batch;
pub mod emit;
pub mod events;
pub mod parse;
#[cfg(feature = "rescribe")]
pub mod rescribe;
pub mod writer;

// Re-export everything callers need.
pub use ast::{
    Block, DefinitionItem, Diagnostic, DokuwikiDoc, Inline, ListItem, Severity, Span, TableCell,
    TableRow,
};
pub use batch::{BatchParser, BatchSink, Handler, StreamingParser};
pub use emit::collect_inline_text;
pub use events::{Event, EventIter, InputEventIter, OwnedEvent};
pub use parse::parse;
pub use rescribe_format_api::{Emit, Events, Parse, StreamingParse, StreamingWrite};
pub use writer::Writer;

/// Parse `input` and return a streaming iterator of [`OwnedEvent`] items.
pub fn events(input: &str) -> events::InputEventIter<'_> {
    events::events(input)
}

// ── Trait implementations ───────────────────────────────────────────────────
//
// `parse::parse`/`events()` (both `&str`-input) stay as public, documented
// non-trait entry points — materially different contract from the shared
// `&[u8]`-input trait methods (they skip the UTF-8 validity check), mirroring
// commonmark-fmt's `parse_str`/`events_str`. `DokuwikiDoc::parse`/
// `DokuwikiDoc::events` bridge from `&[u8]` via `std::str::from_utf8`:
// invalid UTF-8 produces a `Warning` diagnostic for `Parse::parse` (which has
// a diagnostic channel) and an empty-document iterator for `Events::events`
// (which, being infallible-by-contract, does not) — the same split
// commonmark-fmt's `Events` impl documents.

impl Parse for DokuwikiDoc {
    fn parse(input: &[u8]) -> (Self, Vec<Diagnostic>) {
        match std::str::from_utf8(input) {
            Ok(s) => parse::parse(s),
            Err(_) => (
                DokuwikiDoc::default(),
                vec![
                    Diagnostic::new(Severity::Warning, "input is not valid UTF-8")
                        .with_code("dokuwiki::invalid-utf8"),
                ],
            ),
        }
    }
}

impl Emit for DokuwikiDoc {
    fn emit(&self) -> Vec<u8> {
        emit::build(self).into_bytes()
    }
}

impl Events for DokuwikiDoc {
    type Event<'a> = OwnedEvent;
    type EventIter<'a> = InputEventIter<'a>;

    fn events(input: &[u8]) -> InputEventIter<'_> {
        match std::str::from_utf8(input) {
            Ok(s) => events::events(s),
            Err(_) => events::events(""),
        }
    }
}

impl StreamingParse for DokuwikiDoc {
    type Event = OwnedEvent;
    type Parser<H: Handler<OwnedEvent>> = StreamingParser<H>;

    fn streaming_parser<H: Handler<OwnedEvent>>(handler: H) -> StreamingParser<H> {
        StreamingParser::new(handler)
    }
}

impl StreamingWrite for DokuwikiDoc {
    type Writer<W: std::io::Write> = Writer<W>;

    fn writer<W: std::io::Write>(sink: W) -> Writer<W> {
        Writer::new(sink)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use emit::build;

    #[test]
    fn test_parse_heading() {
        let (doc, _) = parse("====== Title ======");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Heading { level: 1, .. }));
    }

    #[test]
    fn test_parse_heading_levels() {
        let (doc, _) = parse("====== H1 ======\n===== H2 =====\n==== H3 ====");
        assert_eq!(doc.blocks.len(), 3);
        let Block::Heading { level: l1, .. } = &doc.blocks[0] else {
            panic!("expected heading");
        };
        assert_eq!(*l1, 1);
        let Block::Heading { level: l2, .. } = &doc.blocks[1] else {
            panic!("expected heading");
        };
        assert_eq!(*l2, 2);
        let Block::Heading { level: l3, .. } = &doc.blocks[2] else {
            panic!("expected heading");
        };
        assert_eq!(*l3, 3);
    }

    #[test]
    fn test_parse_paragraph() {
        let (doc, _) = parse("Hello world!");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Paragraph { .. }));
    }

    #[test]
    fn test_parse_bold() {
        let (doc, _) = parse("This is **bold** text.");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Bold(_, _))));
    }

    #[test]
    fn test_parse_italic() {
        let (doc, _) = parse("This is //italic// text.");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Italic(_, _))));
    }

    #[test]
    fn test_parse_code() {
        let (doc, _) = parse("Use ''code'' here.");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Code(_, _))));
    }

    #[test]
    fn test_parse_link() {
        let (doc, _) = parse("Click [[https://example.com|here]].");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(
            inlines
                .iter()
                .any(|i| matches!(i, Inline::Link { url, .. } if url == "https://example.com"))
        );
    }

    #[test]
    fn test_parse_list() {
        let (doc, _) = parse("  * Item 1\n  * Item 2");
        assert_eq!(doc.blocks.len(), 1);
        let Block::List { ordered, items, .. } = &doc.blocks[0] else {
            panic!("expected list");
        };
        assert!(!ordered);
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_parse_code_block() {
        let (doc, _) = parse("<code rust>\nfn main() {}\n</code>");
        assert_eq!(doc.blocks.len(), 1);
        let Block::CodeBlock {
            language, content, ..
        } = &doc.blocks[0]
        else {
            panic!("expected code block");
        };
        assert_eq!(language.as_deref(), Some("rust"));
        assert!(content.contains("fn main()"));
    }

    #[test]
    fn test_parse_strikethrough() {
        let (doc, _) = parse("This is <del>struck</del> text.");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(
            inlines
                .iter()
                .any(|i| matches!(i, Inline::Strikethrough(_, _)))
        );
    }

    #[test]
    fn test_parse_superscript() {
        let (doc, _) = parse("E=mc<sup>2</sup>");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(
            inlines
                .iter()
                .any(|i| matches!(i, Inline::Superscript(_, _)))
        );
    }

    #[test]
    fn test_parse_subscript() {
        let (doc, _) = parse("H<sub>2</sub>O");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Subscript(_, _))));
    }

    #[test]
    fn test_parse_footnote() {
        let (doc, _) = parse("See this((footnote text)).");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(
            inlines
                .iter()
                .any(|i| matches!(i, Inline::FootnoteRef { .. }))
        );
    }

    #[test]
    fn test_parse_nowiki() {
        let (doc, _) = parse("This %%**not bold**%% stays.");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(
            inlines
                .iter()
                .any(|i| matches!(i, Inline::Nowiki(s, _) if s == "**not bold**"))
        );
    }

    #[test]
    fn test_parse_table() {
        let (doc, _) = parse("^ Name ^ Age ^\n| Alice | 30 |");
        assert_eq!(doc.blocks.len(), 1);
        let Block::Table { rows, .. } = &doc.blocks[0] else {
            panic!("expected table");
        };
        assert_eq!(rows.len(), 2);
        assert!(rows[0].is_header);
        assert!(!rows[1].is_header);
    }

    #[test]
    fn test_parse_definition_list() {
        let (doc, _) = parse("; Term\n: Description");
        assert_eq!(doc.blocks.len(), 1);
        let Block::DefinitionList { items, .. } = &doc.blocks[0] else {
            panic!("expected definition list");
        };
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn test_parse_line_break() {
        let (doc, _) = parse("line one\\\\ line two");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::LineBreak(_))));
    }

    #[test]
    fn test_parse_file_block() {
        let (doc, _) = parse("<file>\nsome content\n</file>");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::FileBlock { .. }));
    }

    #[test]
    fn test_parse_macro() {
        let (doc, _) = parse("~~NOTOC~~");
        assert_eq!(doc.blocks.len(), 1);
        let Block::Macro { name, .. } = &doc.blocks[0] else {
            panic!("expected macro");
        };
        assert_eq!(name, "NOTOC");
    }

    #[test]
    fn test_build_paragraph() {
        let doc = DokuwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Hello, world!".into(), Span::NONE)],
                span: Span::NONE,
            }],
        };
        let out = build(&doc);
        assert!(out.contains("Hello, world!"));
    }

    #[test]
    fn test_build_heading() {
        let doc = DokuwikiDoc {
            blocks: vec![Block::Heading {
                level: 1,
                inlines: vec![Inline::Text("Title".into(), Span::NONE)],
                span: Span::NONE,
            }],
        };
        let out = build(&doc);
        assert!(out.contains("====== Title ======"));
    }

    #[test]
    fn test_build_bold() {
        let doc = DokuwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Bold(
                    vec![Inline::Text("bold".into(), Span::NONE)],
                    Span::NONE,
                )],
                span: Span::NONE,
            }],
        };
        let out = build(&doc);
        assert!(out.contains("**bold**"));
    }

    #[test]
    fn test_build_italic() {
        let doc = DokuwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Italic(
                    vec![Inline::Text("italic".into(), Span::NONE)],
                    Span::NONE,
                )],
                span: Span::NONE,
            }],
        };
        let out = build(&doc);
        assert!(out.contains("//italic//"));
    }

    #[test]
    fn test_build_code() {
        let doc = DokuwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Code("code".into(), Span::NONE)],
                span: Span::NONE,
            }],
        };
        let out = build(&doc);
        assert!(out.contains("''code''"));
    }

    #[test]
    fn test_build_link() {
        let doc = DokuwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "https://example.com".into(),
                    children: vec![Inline::Text("click".into(), Span::NONE)],
                    span: Span::NONE,
                }],
                span: Span::NONE,
            }],
        };
        let out = build(&doc);
        assert!(out.contains("[[https://example.com|click]]"));
    }

    #[test]
    fn test_build_code_block() {
        let doc = DokuwikiDoc {
            blocks: vec![Block::CodeBlock {
                language: Some("python".into()),
                content: "print('hi')".into(),
                span: Span::NONE,
            }],
        };
        let out = build(&doc);
        assert!(out.contains("<code python>"));
        assert!(out.contains("print('hi')"));
        assert!(out.contains("</code>"));
    }
}
