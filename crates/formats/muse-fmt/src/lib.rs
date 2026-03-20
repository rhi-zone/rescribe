//! Muse markup parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-muse` and `rescribe-write-muse` as thin adapter layers.

pub mod ast;
pub mod emit;
pub mod parse;

// Re-export primary types for convenience.
pub use ast::{Block, Diagnostic, Inline, MuseDoc, Severity, Span};
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
        };
        let out = build(&doc);
        assert!(out.contains("<example>"));
        assert!(out.contains("print hi"));
        assert!(out.contains("</example>"));
    }
}
