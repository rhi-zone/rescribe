//! Muse markup parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-muse` and `rescribe-write-muse` as thin adapter layers.

pub mod ast;
pub mod emit;
pub mod parse;

// Re-export primary types for convenience.
pub use ast::{Block, Diagnostic, Inline, MuseDoc, Severity, Span, TableRow};
pub use emit::build;

// ── Public API ────────────────────────────────────────────────────────────────

/// Parse a Muse string into a [`MuseDoc`] and any diagnostics.
///
/// This is the primary entry point. Parsing is infallible — malformed input
/// produces diagnostics rather than errors.
pub fn parse(input: &str) -> (MuseDoc, Vec<Diagnostic>) {
    parse::parse(input)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_heading() {
        let (doc, _) = parse("* Title\n");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Heading { level: 1, .. }));
    }

    #[test]
    fn test_parse_heading_levels() {
        let (doc, _) = parse("** Level 2\n*** Level 3\n");
        assert_eq!(doc.blocks.len(), 2);
        assert!(matches!(doc.blocks[0], Block::Heading { level: 2, .. }));
        assert!(matches!(doc.blocks[1], Block::Heading { level: 3, .. }));
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
    fn test_parse_emphasis() {
        let (doc, _) = parse("text with *emphasis*\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|n| matches!(n, Inline::Italic(_, _))));
    }

    #[test]
    fn test_parse_code() {
        let (doc, _) = parse("=code=\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert_eq!(inlines.len(), 1);
        assert!(matches!(inlines[0], Inline::Code(_, _)));
    }

    #[test]
    fn test_parse_link() {
        let (doc, _) = parse("[[https://example.com][Example]]\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        let link = &inlines[0];
        assert!(matches!(link, Inline::Link { .. }));
    }

    #[test]
    fn test_parse_unordered_list() {
        let (doc, _) = parse(" - item1\n - item2\n");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::List { ordered: false, .. }));
        if let Block::List { items, .. } = &doc.blocks[0] {
            assert_eq!(items.len(), 2);
        }
    }

    #[test]
    fn test_parse_ordered_list() {
        let (doc, _) = parse(" 1. item1\n 2. item2\n");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::List { ordered: true, .. }));
        if let Block::List { items, .. } = &doc.blocks[0] {
            assert_eq!(items.len(), 2);
        }
    }

    #[test]
    fn test_parse_example_block() {
        let (doc, _) = parse("<example>\ncode here\n</example>\n");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::CodeBlock { .. }));
    }

    #[test]
    fn test_build_heading() {
        let doc = MuseDoc {
            blocks: vec![Block::Heading {
                level: 1,
                inlines: vec![Inline::Text("Title".into(), Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
            ..Default::default()
        };
        let out = build(&doc);
        assert!(out.contains("* Title"));
    }

    #[test]
    fn test_build_paragraph() {
        let doc = MuseDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Hello, world!".into(), Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
            ..Default::default()
        };
        let out = build(&doc);
        assert!(out.contains("Hello, world!"));
    }

    #[test]
    fn test_build_bold() {
        let doc = MuseDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Bold(
                    vec![Inline::Text("bold".into(), Span::NONE)],
                    Span::NONE,
                )],
                span: Span::NONE,
            }],
            span: Span::NONE,
            ..Default::default()
        };
        let out = build(&doc);
        assert!(out.contains("**bold**"));
    }

    #[test]
    fn test_build_emphasis() {
        let doc = MuseDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Italic(
                    vec![Inline::Text("emphasis".into(), Span::NONE)],
                    Span::NONE,
                )],
                span: Span::NONE,
            }],
            span: Span::NONE,
            ..Default::default()
        };
        let out = build(&doc);
        assert!(out.contains("*emphasis*"));
    }

    #[test]
    fn test_build_code() {
        let doc = MuseDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Code("code".into(), Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
            ..Default::default()
        };
        let out = build(&doc);
        assert!(out.contains("=code="));
    }

    #[test]
    fn test_build_link() {
        let doc = MuseDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "https://example.com".into(),
                    children: vec![Inline::Text("click".into(), Span::NONE)],
                    span: Span::NONE,
                }],
                span: Span::NONE,
            }],
            span: Span::NONE,
            ..Default::default()
        };
        let out = build(&doc);
        assert!(out.contains("[[https://example.com][click]]"));
    }

    #[test]
    fn test_build_unordered_list() {
        let doc = MuseDoc {
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
            span: Span::NONE,
            ..Default::default()
        };
        let out = build(&doc);
        assert!(out.contains(" - one"));
        assert!(out.contains(" - two"));
    }

    #[test]
    fn test_build_ordered_list() {
        let doc = MuseDoc {
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
            span: Span::NONE,
            ..Default::default()
        };
        let out = build(&doc);
        assert!(out.contains(" 1. first"));
        assert!(out.contains(" 2. second"));
    }

    #[test]
    fn test_build_code_block() {
        let doc = MuseDoc {
            blocks: vec![Block::CodeBlock {
                content: "print hi".into(),
                span: Span::NONE,
            }],
            span: Span::NONE,
            ..Default::default()
        };
        let out = build(&doc);
        assert!(out.contains("<example>"));
        assert!(out.contains("print hi"));
        assert!(out.contains("</example>"));
    }

    // ── New construct tests ─────────────────────────────────────────────

    #[test]
    fn test_parse_heading_h3_h4() {
        let (doc, _) = parse("*** Level 3\n**** Level 4\n");
        assert_eq!(doc.blocks.len(), 2);
        assert!(matches!(doc.blocks[0], Block::Heading { level: 3, .. }));
        assert!(matches!(doc.blocks[1], Block::Heading { level: 4, .. }));
    }

    #[test]
    fn test_parse_verse_block() {
        let (doc, _) = parse("<verse>\nLine one\nLine two\n</verse>\n");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Verse { .. }));
    }

    #[test]
    fn test_parse_center_block() {
        let (doc, _) = parse("<center>\nCentered text\n</center>\n");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::CenteredBlock { .. }));
    }

    #[test]
    fn test_parse_right_block() {
        let (doc, _) = parse("<right>\nRight text\n</right>\n");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::RightBlock { .. }));
    }

    #[test]
    fn test_parse_literal_block() {
        let (doc, _) = parse("<literal>\n<b>HTML</b>\n</literal>\n");
        assert_eq!(doc.blocks.len(), 1);
        if let Block::LiteralBlock { content, .. } = &doc.blocks[0] {
            assert_eq!(content, "<b>HTML</b>");
        } else {
            panic!("expected LiteralBlock");
        }
    }

    #[test]
    fn test_parse_src_block() {
        let (doc, _) = parse("<src lang=\"python\">\ndef f():\n    pass\n</src>\n");
        assert_eq!(doc.blocks.len(), 1);
        if let Block::SrcBlock { lang, content, .. } = &doc.blocks[0] {
            assert_eq!(lang.as_deref(), Some("python"));
            assert!(content.contains("def f()"));
        } else {
            panic!("expected SrcBlock");
        }
    }

    #[test]
    fn test_parse_line_comment() {
        let (doc, _) = parse(";; This is a comment\n");
        assert_eq!(doc.blocks.len(), 1);
        if let Block::Comment { content, .. } = &doc.blocks[0] {
            assert_eq!(content, "This is a comment");
        } else {
            panic!("expected Comment");
        }
    }

    #[test]
    fn test_parse_comment_block() {
        let (doc, _) = parse("<comment>\nBlock comment\n</comment>\n");
        assert_eq!(doc.blocks.len(), 1);
        if let Block::Comment { content, .. } = &doc.blocks[0] {
            assert_eq!(content, "Block comment");
        } else {
            panic!("expected Comment");
        }
    }

    #[test]
    fn test_parse_table() {
        let (doc, _) = parse("|| Name || Age ||\n| Alice | 30 |\n| Bob | 25 |\n");
        assert_eq!(doc.blocks.len(), 1);
        if let Block::Table { rows, .. } = &doc.blocks[0] {
            assert_eq!(rows.len(), 3);
            assert!(rows[0].header);
            assert!(!rows[1].header);
        } else {
            panic!("expected Table");
        }
    }

    #[test]
    fn test_parse_footnote_def() {
        let (doc, _) = parse("[1] This is a footnote.\n");
        assert_eq!(doc.blocks.len(), 1);
        if let Block::FootnoteDef { label, .. } = &doc.blocks[0] {
            assert_eq!(label, "1");
        } else {
            panic!("expected FootnoteDef");
        }
    }

    #[test]
    fn test_parse_underline() {
        let (doc, _) = parse("_underlined_\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(matches!(inlines[0], Inline::Underline(_, _)));
    }

    #[test]
    fn test_parse_strikethrough() {
        let (doc, _) = parse("~~struck~~\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(matches!(inlines[0], Inline::Strikethrough(_, _)));
    }

    #[test]
    fn test_parse_superscript() {
        let (doc, _) = parse("x^2^\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|n| matches!(n, Inline::Superscript(_, _))));
    }

    #[test]
    fn test_parse_subscript() {
        let (doc, _) = parse("H<sub>2</sub>O\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|n| matches!(n, Inline::Subscript(_, _))));
    }

    #[test]
    fn test_parse_footnote_ref() {
        let (doc, _) = parse("See [1] for details.\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|n| matches!(n, Inline::FootnoteRef { .. })));
    }

    #[test]
    fn test_parse_line_break() {
        let (doc, _) = parse("first<br>second\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|n| matches!(n, Inline::LineBreak(_))));
    }

    #[test]
    fn test_parse_anchor() {
        let (doc, _) = parse("<anchor intro>This is the intro.\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|n| matches!(n, Inline::Anchor { .. })));
    }

    #[test]
    fn test_parse_image() {
        let (doc, _) = parse("See [[photo.png]] here.\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|n| matches!(n, Inline::Image { .. })));
    }

    #[test]
    fn test_parse_document_header() {
        let (doc, _) = parse("#title My Doc\n#author Jane\n#date 2024-01-15\n\nBody text.\n");
        assert_eq!(doc.title.as_deref(), Some("My Doc"));
        assert_eq!(doc.author.as_deref(), Some("Jane"));
        assert_eq!(doc.date.as_deref(), Some("2024-01-15"));
        assert_eq!(doc.blocks.len(), 1);
    }

    #[test]
    fn test_roundtrip_new_constructs() {
        // Test that parse(build(doc)) preserves structure for new constructs
        let inputs = [
            "<center>\nCentered\n</center>\n",
            "<right>\nRight\n</right>\n",
            "<literal>\nraw content\n</literal>\n",
            "<src lang=\"rust\">\nfn main() {}\n</src>\n",
            ";; A comment\n",
            "|| A || B ||\n| 1 | 2 |\n",
            "[1] Footnote text.\n",
            "_underlined_ text\n",
            "~~struck~~ text\n",
            "x^2^ formula\n",
            "H<sub>2</sub>O\n",
            "See [1] here.\n",
            "first<br>second\n",
            "[[photo.png]]\n",
        ];
        for input in &inputs {
            let (doc, _) = parse(input);
            let output = build(&doc);
            let (doc2, _) = parse(&output);
            assert_eq!(
                doc.blocks.len(),
                doc2.blocks.len(),
                "Block count mismatch for input: {}",
                input
            );
        }
    }
}
