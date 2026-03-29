//! XWiki 2.0 format parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-xwiki` and `rescribe-write-xwiki` as thin adapter layers.

pub mod ast;
pub mod batch;
pub mod emit;
pub mod events;
pub mod parse;
pub mod writer;

pub use ast::{Block, Diagnostic, Inline, Severity, Span, TableCell, TableRow, XwikiDoc};
pub use batch::{BatchParser, BatchSink, Handler, StreamingParser};
pub use emit::{build, collect_inline_text};
pub use events::{Event, EventIter, OwnedEvent};
pub use parse::parse;
pub use writer::Writer;

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_heading() {
        let (result, _) = parse("= Heading 1 =\n== Heading 2 ==");
        assert_eq!(result.blocks.len(), 2);
    }

    #[test]
    fn test_parse_bold() {
        let (result, _) = parse("This is **bold** text");
        assert!(!result.blocks.is_empty());
    }

    #[test]
    fn test_parse_italic() {
        let (result, _) = parse("This is //italic// text");
        assert!(!result.blocks.is_empty());
    }

    #[test]
    fn test_parse_link() {
        let (result, _) = parse("[[Example>>http://example.com]]");
        assert!(!result.blocks.is_empty());
    }

    #[test]
    fn test_parse_list() {
        let (result, _) = parse("* Item 1\n* Item 2");
        assert_eq!(result.blocks.len(), 1);
    }

    #[test]
    fn test_parse_code_block() {
        let (result, _) = parse("{{code language=\"rust\"}}\nfn main() {}\n{{/code}}");
        assert_eq!(result.blocks.len(), 1);
    }

    #[test]
    fn test_parse_table() {
        let (result, _) = parse("|=Header|Cell|");
        assert_eq!(result.blocks.len(), 1);
    }

    #[test]
    fn test_parse_superscript() {
        let (result, _) = parse("H^^2^^O");
        let Block::Paragraph { inlines, .. } = &result.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Superscript(..))));
    }

    #[test]
    fn test_parse_subscript() {
        let (result, _) = parse("H~~2~~O");
        let Block::Paragraph { inlines, .. } = &result.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Subscript(..))));
    }

    #[test]
    fn test_parse_line_break() {
        let (result, _) = parse("line one\\\\ line two");
        let Block::Paragraph { inlines, .. } = &result.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::LineBreak { .. })));
    }

    #[test]
    fn test_parse_image() {
        let (result, _) = parse("[[image:photo.png]]");
        let Block::Paragraph { inlines, .. } = &result.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Image { url, .. } if url == "photo.png")));
    }

    #[test]
    fn test_parse_image_with_alt() {
        let (result, _) = parse("[[image:photo.png||alt=\"A photo\"]]");
        let Block::Paragraph { inlines, .. } = &result.blocks[0] else {
            panic!("expected paragraph");
        };
        if let Some(Inline::Image { alt, .. }) = inlines.first() {
            assert_eq!(alt.as_deref(), Some("A photo"));
        } else {
            panic!("expected image");
        }
    }

    #[test]
    fn test_parse_blockquote() {
        let (result, _) = parse("{{quote}}\nSome quoted text.\n{{/quote}}");
        assert!(matches!(result.blocks[0], Block::Blockquote { .. }));
    }

    #[test]
    fn test_parse_info_macro() {
        let (result, _) = parse("{{info}}\nSome info.\n{{/info}}");
        assert!(matches!(result.blocks[0], Block::MacroBlock { .. }));
        if let Block::MacroBlock { name, content, .. } = &result.blocks[0] {
            assert_eq!(name, "info");
            assert!(content.contains("Some info."));
        }
    }

    #[test]
    fn test_parse_toc_macro() {
        let (result, _) = parse("{{toc/}}");
        assert!(matches!(result.blocks[0], Block::MacroInline { .. }));
        if let Block::MacroInline { name, .. } = &result.blocks[0] {
            assert_eq!(name, "toc");
        }
    }

    #[test]
    fn test_build_heading() {
        let doc = XwikiDoc {
            blocks: vec![Block::Heading {
                level: 1,
                inlines: vec![Inline::Text("Title".into(), Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("= Title ="));
    }

    #[test]
    fn test_build_bold() {
        let doc = XwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Bold(
                    vec![Inline::Text("bold".into(), Span::NONE)],
                    Span::NONE,
                )],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("**bold**"));
    }

    #[test]
    fn test_build_link() {
        let doc = XwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "http://example.com".into(),
                    label: "Example".into(),
                    span: Span::NONE,
                }],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("[[Example>>http://example.com]]"));
    }

    #[test]
    fn test_roundtrip_simple() {
        let input = "= Heading =\n\nSimple paragraph with **bold** text.";
        let (doc, _) = parse(input);
        let output = build(&doc);
        let (doc2, _) = parse(&output);
        assert_eq!(doc.blocks.len(), doc2.blocks.len());
    }

    #[test]
    fn test_parse_sample_no_panic() {
        // Adversarial: should not panic on any input
        let samples = [
            "",
            "= heading =",
            "**unclosed bold",
            "{{code}}\nmissing end",
            "|= header | data",
            "* list\n* items",
            "{{info}}\nmissing close",
            "{{toc/}}",
            "[[image:test.png||alt=\"hello\"]]",
            "^^super^^ and ~~sub~~",
            "\\\\ line break",
            "{{quote}}\nquoted\n{{/quote}}",
            "{{velocity}}\n$var\n{{/velocity}}",
            "= = = = = =",
            "||||||||",
            "{{unknown_macro}}\ncontent\n{{/unknown_macro}}",
        ];
        for sample in &samples {
            let (_, _) = parse(sample);
        }
    }
}
