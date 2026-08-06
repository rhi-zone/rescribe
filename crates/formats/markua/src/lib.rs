//! Markua (Leanpub) parser, emitter, and AST.
//!
//! Standalone crate with **no rescribe dependency by default**. The
//! optional `rescribe` feature (default off) adds `crate::rescribe::{parse,
//! emit}`, a thin adapter that translates `markua::MarkuaDoc` to and from
//! rescribe's `Document` IR.
//!
//! # API layers
//!
//! `MarkuaDoc` implements the five shared `rescribe-format-api` traits — no
//! parallel free functions exist alongside them:
//!
//! ```rust
//! use rescribe_format_api::{Emit, Events, Parse, StreamingParse, StreamingWrite};
//! use markua::MarkuaDoc;
//!
//! // AST reader
//! let (doc, _diagnostics) = MarkuaDoc::parse(b"# Hello\n\nWorld.\n");
//!
//! // Builder writer — emit from AST
//! let output: Vec<u8> = doc.emit();
//! ```
//!
//! `parse_str`/`events_str` (take `&str`, skip the UTF-8 check) remain as
//! separate, non-trait entry points: they have a materially different
//! contract from the trait methods (`&str` input, not `&[u8]`), not a
//! redundant duplicate of them, mirroring `commonmark-fmt`'s
//! `parse_str`/`events_str`.

pub mod ast;
pub mod batch;
pub mod emit;
pub mod events;
pub mod parse;
#[cfg(feature = "rescribe")]
pub mod rescribe;
pub mod writer;

pub use ast::{Block, Diagnostic, Inline, MarkuaDoc, Severity, Span, TableRow};
pub use batch::{BatchParser, BatchSink, Handler, StreamingParser};
pub use emit::collect_inline_text;
pub use events::{EventIter, MarkuaEvent, OwnedMarkuaEvent, events_str};
pub use parse::parse_str;
pub use rescribe_format_api::{Emit, Events, Parse, StreamingParse, StreamingWrite};
pub use writer::Writer;

// ── Trait implementations ───────────────────────────────────────────────────
//
// `MarkuaDoc` implements the five shared API-mode traits directly — there
// are no parallel free functions (`markua::parse(..)`, `markua::emit(..)`,
// `markua::build(..)`) alongside these; callers `use rescribe_format_api::
// Parse;` (etc.) and call `MarkuaDoc::parse(bytes)` / `doc.emit()` /
// `MarkuaDoc::events(bytes)`.

impl Parse for MarkuaDoc {
    fn parse(input: &[u8]) -> (Self, Vec<Diagnostic>) {
        parse::parse(input)
    }
}

impl Emit for MarkuaDoc {
    fn emit(&self) -> Vec<u8> {
        emit::emit(self).into_bytes()
    }
}

impl Events for MarkuaDoc {
    type Event<'a> = MarkuaEvent<'a>;
    type EventIter<'a> = EventIter<'a>;

    /// Invalid UTF-8 input yields an empty iterator rather than panicking
    /// — the trait's `events()` is infallible by contract and has no
    /// diagnostic channel (unlike `Parse::parse`). Callers that need to
    /// distinguish "empty document" from "invalid UTF-8" should use
    /// [`parse::parse`](crate::parse::parse) directly, whose diagnostics
    /// report the encoding problem.
    fn events(input: &[u8]) -> EventIter<'_> {
        match std::str::from_utf8(input) {
            Ok(s) => EventIter::new(s),
            Err(_) => EventIter::new(""),
        }
    }
}

impl StreamingParse for MarkuaDoc {
    type Event = OwnedMarkuaEvent;
    type Parser<H: Handler<OwnedMarkuaEvent>> = StreamingParser<H>;

    fn streaming_parser<H: Handler<OwnedMarkuaEvent>>(handler: H) -> StreamingParser<H> {
        StreamingParser::new(handler)
    }
}

impl StreamingWrite for MarkuaDoc {
    type Writer<W: std::io::Write> = Writer<W>;

    fn writer<W: std::io::Write>(sink: W) -> Writer<W> {
        Writer::new(sink)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::emit::emit as build;
    use crate::parse::parse_str as parse;

    #[test]
    fn test_parse_heading() {
        let (doc, _) = parse("# Title\n");
        assert_eq!(doc.blocks.len(), 1);
        match &doc.blocks[0] {
            Block::Heading { level, .. } => assert_eq!(*level, 1),
            _ => panic!("expected heading"),
        }
    }

    #[test]
    fn test_parse_heading_level2() {
        let (doc, _) = parse("## Subtitle\n");
        match &doc.blocks[0] {
            Block::Heading { level, .. } => assert_eq!(*level, 2),
            _ => panic!("expected heading"),
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
        assert!(inlines.iter().any(|i| matches!(i, Inline::Strong(..))));
    }

    #[test]
    fn test_parse_italic() {
        let (doc, _) = parse("*italic*\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Emphasis(..))));
    }

    #[test]
    fn test_parse_code() {
        let (doc, _) = parse("`code`\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Code(..))));
    }

    #[test]
    fn test_parse_link() {
        let (doc, _) = parse("[click here](https://example.com)\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        let link = inlines.iter().find(|i| matches!(i, Inline::Link { .. }));
        assert!(link.is_some());
    }

    #[test]
    fn test_parse_aside() {
        let (doc, _) = parse("A> This is an aside.\n");
        let block = &doc.blocks[0];
        assert!(matches!(block, Block::SpecialBlock { block_type, .. } if block_type == "aside"));
    }

    #[test]
    fn test_parse_warning() {
        let (doc, _) = parse("W> This is a warning.\n");
        let block = &doc.blocks[0];
        assert!(matches!(block, Block::SpecialBlock { block_type, .. } if block_type == "warning"));
    }

    #[test]
    fn test_parse_tip() {
        let (doc, _) = parse("T> This is a tip.\n");
        let block = &doc.blocks[0];
        assert!(matches!(block, Block::SpecialBlock { block_type, .. } if block_type == "tip"));
    }

    #[test]
    fn test_parse_blockquote() {
        let (doc, _) = parse("> Quoted text\n");
        assert!(matches!(doc.blocks[0], Block::Blockquote { .. }));
    }

    #[test]
    fn test_parse_unordered_list() {
        let (doc, _) = parse("- item1\n- item2\n");
        let block = &doc.blocks[0];
        match block {
            Block::List { ordered, items, .. } => {
                assert!(!ordered);
                assert_eq!(items.len(), 2);
            }
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn test_parse_ordered_list() {
        let (doc, _) = parse("1. first\n2. second\n");
        let block = &doc.blocks[0];
        match block {
            Block::List { ordered, .. } => assert!(*ordered),
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn test_parse_code_block() {
        let (doc, _) = parse("```\ncode here\n```\n");
        assert!(matches!(doc.blocks[0], Block::CodeBlock { .. }));
    }

    #[test]
    fn test_parse_code_block_with_language() {
        let (doc, _) = parse("```ruby\nputs 'hello'\n```\n");
        let block = &doc.blocks[0];
        match block {
            Block::CodeBlock { language, .. } => {
                assert_eq!(language.as_deref(), Some("ruby"));
            }
            _ => panic!("expected code block"),
        }
    }

    #[test]
    fn test_parse_scene_break() {
        let (doc, _) = parse("* * *\n");
        assert!(matches!(doc.blocks[0], Block::HorizontalRule { .. }));
    }

    #[test]
    fn test_parse_image() {
        let (doc, _) = parse("![Alt text](image.png)\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        let img = inlines.iter().find(|i| matches!(i, Inline::Image { .. }));
        assert!(img.is_some());
    }

    #[test]
    fn test_parse_page_break() {
        let (doc, _) = parse("{pagebreak}\n");
        assert!(matches!(doc.blocks[0], Block::PageBreak { .. }));
    }

    #[test]
    fn test_parse_page_break_hyphenated() {
        let (doc, _) = parse("{page-break}\n");
        assert!(matches!(doc.blocks[0], Block::PageBreak { .. }));
    }

    #[test]
    fn test_parse_subscript() {
        let (doc, _) = parse("H~2~O\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Subscript(..))));
    }

    #[test]
    fn test_parse_superscript() {
        let (doc, _) = parse("x^2^\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Superscript(..))));
    }

    #[test]
    fn test_parse_footnote_ref() {
        let (doc, _) = parse("text ^[a note] more\n");
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
    fn test_parse_index_term() {
        let (doc, _) = parse("See i[Markua] here.\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(
            inlines
                .iter()
                .any(|i| matches!(i, Inline::IndexTerm { term, .. } if term == "Markua"))
        );
    }

    #[test]
    fn test_parse_math_inline() {
        let (doc, _) = parse("Solve $x^2 + 1 = 0$.\n");
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
    fn test_parse_definition_list() {
        let (doc, _) = parse("Term\n: Definition text\n");
        assert!(matches!(doc.blocks[0], Block::DefinitionList { .. }));
    }

    #[test]
    fn test_parse_table() {
        let (doc, _) = parse("| A | B |\n| --- | --- |\n| 1 | 2 |\n");
        assert!(matches!(doc.blocks[0], Block::Table { .. }));
    }

    #[test]
    fn test_parse_special_block_with_children() {
        let (doc, _) = parse("W> - item 1\nW> - item 2\n");
        match &doc.blocks[0] {
            Block::SpecialBlock {
                block_type,
                children,
                ..
            } => {
                assert_eq!(block_type, "warning");
                assert!(!children.is_empty());
            }
            _ => panic!("expected special block"),
        }
    }

    #[test]
    fn test_build_paragraph() {
        let doc = MarkuaDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Hello, world!".into(), Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
            title: None,
            author: None,
            description: None,
        };
        let out = build(&doc);
        assert!(out.contains("Hello, world!"));
    }

    #[test]
    fn test_build_bold() {
        let doc = MarkuaDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Strong(
                    vec![Inline::Text("bold".into(), Span::NONE)],
                    Span::NONE,
                )],
                span: Span::NONE,
            }],
            span: Span::NONE,
            title: None,
            author: None,
            description: None,
        };
        let out = build(&doc);
        assert!(out.contains("**bold**"));
    }

    #[test]
    fn test_build_heading() {
        let doc = MarkuaDoc {
            blocks: vec![Block::Heading {
                level: 1,
                inlines: vec![Inline::Text("Title".into(), Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
            title: None,
            author: None,
            description: None,
        };
        let out = build(&doc);
        assert!(out.contains("# Title"));
    }

    #[test]
    fn test_build_code_block() {
        let doc = MarkuaDoc {
            blocks: vec![Block::CodeBlock {
                content: "print hi".into(),
                language: None,
                span: Span::NONE,
            }],
            span: Span::NONE,
            title: None,
            author: None,
            description: None,
        };
        let out = build(&doc);
        assert!(out.contains("```"));
        assert!(out.contains("print hi"));
    }

    #[test]
    fn test_roundtrip_heading() {
        let (doc, _) = parse("# Title\n");
        let output = build(&doc);
        assert!(output.contains("# Title"));
    }

    #[test]
    fn test_roundtrip_bold() {
        let (doc, _) = parse("**bold text**\n");
        let output = build(&doc);
        assert!(output.contains("**bold text**"));
    }

    #[test]
    fn test_roundtrip_page_break() {
        let (doc, _) = parse("{pagebreak}\n");
        let output = build(&doc);
        assert!(output.contains("{pagebreak}"));
    }

    #[test]
    fn test_roundtrip_math_inline() {
        let (doc, _) = parse("$x^2 + 1$\n");
        let output = build(&doc);
        assert!(output.contains("$x^2 + 1$"));
    }

    #[test]
    fn test_roundtrip_footnote_ref() {
        let (doc, _) = parse("^[a note]\n");
        let output = build(&doc);
        assert!(output.contains("^[a note]"));
    }
}
