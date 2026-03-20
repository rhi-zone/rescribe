//! TWiki format parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-twiki` and `rescribe-write-twiki` as thin adapter layers.

pub mod ast;
pub mod emit;
pub mod parse;

pub use ast::{Block, Diagnostic, Inline, Severity, Span, TableCell, TableRow, TwikiDoc};
pub use emit::{build, collect_inline_text};
pub use parse::parse;

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_heading() {
        let (result, _) = parse("---+ Heading 1\n---++ Heading 2");
        assert_eq!(result.blocks.len(), 2);
        assert!(matches!(result.blocks[0], Block::Heading { .. }));
    }

    #[test]
    fn test_parse_bold() {
        let (result, _) = parse("This is *bold* text");
        assert_eq!(result.blocks.len(), 1);
        assert!(matches!(result.blocks[0], Block::Paragraph { .. }));
    }

    #[test]
    fn test_parse_italic() {
        let (result, _) = parse("This is _italic_ text");
        assert_eq!(result.blocks.len(), 1);
        assert!(matches!(result.blocks[0], Block::Paragraph { .. }));
    }

    #[test]
    fn test_parse_code() {
        let (result, _) = parse("Use =code= here");
        assert_eq!(result.blocks.len(), 1);
        assert!(matches!(result.blocks[0], Block::Paragraph { .. }));
    }

    #[test]
    fn test_parse_link() {
        let (result, _) = parse("Visit [[http://example.com][Example]]");
        assert_eq!(result.blocks.len(), 1);
        assert!(matches!(result.blocks[0], Block::Paragraph { .. }));
    }

    #[test]
    fn test_parse_table() {
        let (result, _) = parse("| A | B |\n| C | D |");
        assert_eq!(result.blocks.len(), 1);
        assert!(matches!(result.blocks[0], Block::Table { .. }));
    }

    #[test]
    fn test_parse_heading_level() {
        let (result, _) = parse("---+ Level 1\n---++ Level 2\n---+++ Level 3");
        assert_eq!(result.blocks.len(), 3);
        if let Block::Heading { level, .. } = &result.blocks[0] {
            assert_eq!(*level, 1);
        } else {
            panic!("expected heading");
        }
        if let Block::Heading { level, .. } = &result.blocks[1] {
            assert_eq!(*level, 2);
        } else {
            panic!("expected heading");
        }
        if let Block::Heading { level, .. } = &result.blocks[2] {
            assert_eq!(*level, 3);
        } else {
            panic!("expected heading");
        }
    }

    #[test]
    fn test_build_heading() {
        let doc = TwikiDoc {
            blocks: vec![Block::Heading {
                level: 1,
                inlines: vec![Inline::Text("Title".into(), Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("---+ Title"));
    }

    #[test]
    fn test_build_bold() {
        let doc = TwikiDoc {
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
        assert!(out.contains("*bold*"));
    }

    #[test]
    fn test_build_italic() {
        let doc = TwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Italic(
                    vec![Inline::Text("italic".into(), Span::NONE)],
                    Span::NONE,
                )],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("_italic_"));
    }

    #[test]
    fn test_build_link() {
        let doc = TwikiDoc {
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
        assert!(out.contains("[[http://example.com][Example]]"));
    }

    #[test]
    fn test_build_list() {
        let doc = TwikiDoc {
            blocks: vec![Block::List {
                ordered: false,
                items: vec![
                    vec![Inline::Text("one".into(), Span::NONE)],
                    vec![Inline::Text("two".into(), Span::NONE)],
                ],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("   * one"));
        assert!(out.contains("   * two"));
    }

    #[test]
    fn test_build_paragraph() {
        let doc = TwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Hello, world!".into(), Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("Hello, world!"));
    }

    #[test]
    fn test_roundtrip_simple() {
        let original = "---+ Hello\n\nThis is a paragraph.\n\n";
        let (doc, _) = parse(original);
        let rebuilt = build(&doc);
        let (doc2, _) = parse(&rebuilt);
        assert_eq!(doc.blocks.len(), doc2.blocks.len());
    }
}
