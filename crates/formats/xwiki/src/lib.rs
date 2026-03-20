//! XWiki 2.0 format parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-xwiki` and `rescribe-write-xwiki` as thin adapter layers.

pub mod ast;
pub mod emit;
pub mod parse;

pub use ast::{Block, Diagnostic, Inline, Severity, Span, TableCell, TableRow, XwikiDoc};
pub use emit::{build, collect_inline_text};
pub use parse::parse;

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
}
