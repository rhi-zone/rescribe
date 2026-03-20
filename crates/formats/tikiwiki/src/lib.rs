//! TikiWiki parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-tikiwiki` and `rescribe-write-tikiwiki` as thin adapter layers.

pub mod ast;
pub mod emit;
pub mod parse;

pub use ast::{Block, Diagnostic, Inline, Severity, Span, TableRow, TikiwikiDoc};
pub use emit::{build, collect_inline_text};
pub use parse::parse;

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_heading() {
        let (doc, _) = parse("!Heading 1\n!!Heading 2");
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_parse_bold() {
        let (doc, _) = parse("This is __bold__ text");
        assert_eq!(doc.blocks.len(), 1);
    }

    #[test]
    fn test_parse_italic() {
        let (doc, _) = parse("This is ''italic'' text");
        assert_eq!(doc.blocks.len(), 1);
    }

    #[test]
    fn test_parse_link() {
        let (doc, _) = parse("[http://example.com|Example]");
        assert_eq!(doc.blocks.len(), 1);
    }

    #[test]
    fn test_parse_list() {
        let (doc, _) = parse("*Item 1\n*Item 2");
        assert_eq!(doc.blocks.len(), 1);
        if let Block::List { .. } = &doc.blocks[0] {
            // OK
        } else {
            panic!("Expected list block");
        }
    }

    #[test]
    fn test_parse_table() {
        let (doc, _) = parse("||A|B||\n||C|D||");
        assert_eq!(doc.blocks.len(), 1);
        if let Block::Table { rows, .. } = &doc.blocks[0] {
            assert_eq!(rows.len(), 2);
        } else {
            panic!("Expected table block");
        }
    }

    #[test]
    fn test_build_heading() {
        let doc = TikiwikiDoc {
            blocks: vec![Block::Heading {
                level: 1,
                inlines: vec![Inline::Text("Title".into(), Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("!Title"));
    }

    #[test]
    fn test_build_bold() {
        let doc = TikiwikiDoc {
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
        assert!(out.contains("__bold__"));
    }

    #[test]
    fn test_build_italic() {
        let doc = TikiwikiDoc {
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
        assert!(out.contains("''italic''"));
    }

    #[test]
    fn test_build_link() {
        let doc = TikiwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "http://example.com".into(),
                    children: vec![Inline::Text("Example".into(), Span::NONE)],
                    span: Span::NONE,
                }],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("[http://example.com|Example]"));
    }

    #[test]
    fn test_build_code_block() {
        let doc = TikiwikiDoc {
            blocks: vec![Block::CodeBlock {
                content: "let x = 5;".into(),
                language: Some("rust".into()),
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("{CODE(lang=rust)}"));
        assert!(out.contains("let x = 5;"));
    }

    #[test]
    fn test_roundtrip_simple() {
        let input = "!Heading\n\nParagraph text\n\n__bold__";
        let (doc, _) = parse(input);
        let output = build(&doc);
        // Parse again to verify consistency
        let (doc2, _) = parse(&output);
        assert_eq!(doc.blocks.len(), doc2.blocks.len());
    }
}
