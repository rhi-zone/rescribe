//! Man page (roff/troff) parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-man` and `rescribe-write-man` as thin adapter layers.

pub mod ast;
pub mod emit;
pub mod parse;

// Re-export key types for convenience.
pub use ast::{Block, Diagnostic, Inline, ManDoc, Severity, Span};
pub use emit::build;
pub use parse::parse;

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(input: &str) -> ManDoc {
        let (doc, _diags) = parse(input);
        doc
    }

    #[test]
    fn test_parse_title() {
        let doc = parse_ok(".TH TEST 1 \"2024-01-01\" \"Version 1.0\"");
        assert_eq!(doc.title, Some("TEST".to_string()));
        assert_eq!(doc.section, Some("1".to_string()));
    }

    #[test]
    fn test_parse_sections() {
        let doc = parse_ok(".SH NAME\ntest \\- a test program\n.SH SYNOPSIS\ntest [options]");
        assert_eq!(doc.blocks.len(), 4); // 2 headings + 2 paragraphs
    }

    #[test]
    fn test_parse_bold() {
        let doc = parse_ok(".B bold text");
        let block = &doc.blocks[0];
        assert!(matches!(block, Block::Paragraph { .. }));
        if let Block::Paragraph { inlines, .. } = block {
            assert!(inlines.iter().any(|i| matches!(i, Inline::Bold(..))));
        }
    }

    #[test]
    fn test_parse_italic() {
        let doc = parse_ok(".I italic text");
        let block = &doc.blocks[0];
        assert!(matches!(block, Block::Paragraph { .. }));
        if let Block::Paragraph { inlines, .. } = block {
            assert!(inlines.iter().any(|i| matches!(i, Inline::Italic(..))));
        }
    }

    #[test]
    fn test_parse_preformatted() {
        let doc = parse_ok(".nf\ncode line 1\ncode line 2\n.fi");
        let block = &doc.blocks[0];
        assert!(matches!(block, Block::CodeBlock { .. }));
    }

    #[test]
    fn test_parse_inline_font() {
        let doc = parse_ok("This is \\fBbold\\fR text");
        let block = &doc.blocks[0];
        if let Block::Paragraph { inlines, .. } = block {
            // Should have multiple inlines
            assert!(inlines.len() >= 2);
        }
    }

    #[test]
    fn test_build_basic() {
        let doc = ManDoc {
            title: Some("TEST".to_string()),
            section: Some("1".to_string()),
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Hello, world!".to_string(), Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let output = build(&doc);
        assert!(output.contains(".TH"));
        assert!(output.contains(".PP"));
        assert!(output.contains("Hello, world!"));
    }

    #[test]
    fn test_build_heading() {
        let doc = ManDoc {
            title: None,
            section: None,
            blocks: vec![Block::Heading {
                level: 2,
                inlines: vec![Inline::Text("Section Title".to_string(), Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let output = build(&doc);
        assert!(output.contains(".SH SECTION TITLE"));
    }

    #[test]
    fn test_build_bold() {
        let doc = ManDoc {
            title: None,
            section: None,
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Bold(
                    vec![Inline::Text("bold".to_string(), Span::NONE)],
                    Span::NONE,
                )],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let output = build(&doc);
        assert!(output.contains("\\fBbold\\fR"));
    }

    #[test]
    fn test_build_italic() {
        let doc = ManDoc {
            title: None,
            section: None,
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Italic(
                    vec![Inline::Text("italic".to_string(), Span::NONE)],
                    Span::NONE,
                )],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let output = build(&doc);
        assert!(output.contains("\\fIitalic\\fR"));
    }

    #[test]
    fn test_build_link() {
        let doc = ManDoc {
            title: None,
            section: None,
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "https://example.com".to_string(),
                    children: vec![Inline::Text("Example".to_string(), Span::NONE)],
                    span: Span::NONE,
                }],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let output = build(&doc);
        assert!(output.contains("Example"));
        assert!(output.contains("https://example.com"));
    }
}
