//! Jira wiki markup parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-jira` and `rescribe-write-jira` as thin adapter layers.

pub mod ast;
pub mod emit;
pub mod parse;

pub use ast::{Block, Diagnostic, Inline, JiraDoc, Severity, Span, TableCell, TableRow};
pub use emit::{build, collect_inline_text};
pub use parse::parse;

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_text() {
        let (doc, _) = parse("Hello world");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Paragraph { .. }));
    }

    #[test]
    fn test_parse_heading() {
        let (doc, _) = parse("h1. Title");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Heading { level: 1, .. }));
    }

    #[test]
    fn test_parse_bold() {
        let (doc, _) = parse("This is *bold* text.");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Bold(_, _))));
    }

    #[test]
    fn test_parse_italic() {
        let (doc, _) = parse("This is _italic_ text.");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Italic(_, _))));
    }

    #[test]
    fn test_parse_code() {
        let (doc, _) = parse("Use {{code}} here.");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Code(_, _))));
    }

    #[test]
    fn test_parse_link() {
        let (doc, _) = parse("Click [here|https://example.com].");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Link { .. })));
    }

    #[test]
    fn test_parse_list() {
        let (doc, _) = parse("* Item 1\n* Item 2");
        let block = &doc.blocks[0];
        assert!(matches!(block, Block::List { ordered: false, .. }));
    }

    #[test]
    fn test_parse_code_block() {
        let (doc, _) = parse("{code:java}\npublic class Test {}\n{code}");
        let code = &doc.blocks[0];
        assert!(matches!(code, Block::CodeBlock { .. }));
        if let Block::CodeBlock { language, .. } = code {
            assert_eq!(language.as_deref(), Some("java"));
        }
    }

    #[test]
    fn test_build_paragraph() {
        let doc = JiraDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Hello".into(), Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let output = build(&doc);
        assert!(output.contains("Hello"));
    }

    #[test]
    fn test_build_bold() {
        let doc = JiraDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Bold(
                    vec![Inline::Text("bold".into(), Span::NONE)],
                    Span::NONE,
                )],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let output = build(&doc);
        assert!(output.contains("*bold*"));
    }

    #[test]
    fn test_build_heading() {
        let doc = JiraDoc {
            blocks: vec![Block::Heading {
                level: 1,
                inlines: vec![Inline::Text("Title".into(), Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let output = build(&doc);
        assert!(output.contains("h1. Title"));
    }

    #[test]
    fn test_build_code_block() {
        let doc = JiraDoc {
            blocks: vec![Block::CodeBlock {
                content: "print('hi')".into(),
                language: Some("python".into()),
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let output = build(&doc);
        assert!(output.contains("{code:python}"));
        assert!(output.contains("print('hi')"));
        assert!(output.contains("{code}"));
    }

    #[test]
    fn test_roundtrip_heading() {
        let original = "h1. Title";
        let (doc, _) = parse(original);
        let rebuilt = build(&doc);
        assert!(rebuilt.contains("h1. Title"));
    }

    #[test]
    fn test_roundtrip_bold() {
        let original = "This is *bold* text.";
        let (doc, _) = parse(original);
        let rebuilt = build(&doc);
        assert!(rebuilt.contains("*bold*"));
    }
}
