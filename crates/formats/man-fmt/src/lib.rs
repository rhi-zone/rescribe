//! Man page (roff/troff) parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-man` and `rescribe-write-man` as thin adapter layers.

pub mod ast;
pub mod batch;
pub mod emit;
pub mod events;
pub mod parse;
pub mod writer;

// Re-export key types for convenience.
pub use ast::{Block, Diagnostic, Inline, ManDoc, Severity, Span};
pub use batch::{BatchParser, BatchSink, Handler, StreamingParser};
pub use emit::build;
pub use events::{EventIter, ManEvent, OwnedManEvent};
pub use parse::parse;
pub use writer::Writer;

/// Parse `input` and return a streaming iterator of [`OwnedManEvent`] items.
pub fn man_events(input: &str) -> impl Iterator<Item = OwnedManEvent> + '_ {
    events::events(input)
}

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
        assert_eq!(doc.date, Some("2024-01-01".to_string()));
        assert_eq!(doc.source, Some("Version 1.0".to_string()));
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
    fn test_parse_example_block() {
        let doc = parse_ok(".EX\nexample code\n.EE");
        let block = &doc.blocks[0];
        assert!(matches!(block, Block::ExampleBlock { .. }));
        if let Block::ExampleBlock { content, .. } = block {
            assert_eq!(content, "example code");
        }
    }

    #[test]
    fn test_parse_inline_code() {
        let doc = parse_ok("Use \\f(CWcommand\\fR here");
        let block = &doc.blocks[0];
        if let Block::Paragraph { inlines, .. } = block {
            assert!(inlines.iter().any(|i| matches!(i, Inline::Code(..))));
        }
    }

    #[test]
    fn test_parse_special_chars() {
        let doc = parse_ok("em dash \\(em and en dash \\(en");
        let block = &doc.blocks[0];
        if let Block::Paragraph { inlines, .. } = block {
            let text = inlines
                .iter()
                .filter_map(|i| match i {
                    Inline::Text(s, _) => Some(s.as_str()),
                    _ => None,
                })
                .collect::<String>();
            assert!(text.contains('\u{2014}'), "expected em dash");
            assert!(text.contains('\u{2013}'), "expected en dash");
        }
    }

    #[test]
    fn test_parse_comment() {
        let doc = parse_ok(".\\\" This is a comment\n.PP\nhello");
        assert!(doc.blocks.iter().any(|b| matches!(b, Block::Comment { .. })));
    }

    #[test]
    fn test_parse_indented_paragraph() {
        let doc = parse_ok(".IP\nIndented text here");
        assert!(doc
            .blocks
            .iter()
            .any(|b| matches!(b, Block::IndentedParagraph { .. })));
    }

    #[test]
    fn test_parse_th_metadata() {
        let doc = parse_ok(".TH MYAPP 1 \"2024-01-15\" \"MyApp 1.0\" \"User Commands\"");
        assert_eq!(doc.title, Some("MYAPP".to_string()));
        assert_eq!(doc.section, Some("1".to_string()));
        assert_eq!(doc.date, Some("2024-01-15".to_string()));
        assert_eq!(doc.source, Some("MyApp 1.0".to_string()));
        assert_eq!(doc.manual, Some("User Commands".to_string()));
    }

    #[test]
    fn test_build_basic() {
        let doc = ManDoc {
            title: Some("TEST".to_string()),
            section: Some("1".to_string()),
            date: None,
            source: None,
            manual: None,
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
            date: None,
            source: None,
            manual: None,
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
            date: None,
            source: None,
            manual: None,
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
            date: None,
            source: None,
            manual: None,
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
            date: None,
            source: None,
            manual: None,
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

    #[test]
    fn test_build_inline_code() {
        let doc = ManDoc {
            title: None,
            section: None,
            date: None,
            source: None,
            manual: None,
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Code("command".to_string(), Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let output = build(&doc);
        assert!(output.contains("\\f(CWcommand\\fR"));
    }

    #[test]
    fn test_build_example_block() {
        let doc = ManDoc {
            title: None,
            section: None,
            date: None,
            source: None,
            manual: None,
            blocks: vec![Block::ExampleBlock {
                content: "example code".to_string(),
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let output = build(&doc);
        assert!(output.contains(".EX"));
        assert!(output.contains("example code"));
        assert!(output.contains(".EE"));
    }

    #[test]
    fn test_build_comment() {
        let doc = ManDoc {
            title: None,
            section: None,
            date: None,
            source: None,
            manual: None,
            blocks: vec![Block::Comment {
                text: "This is a comment".to_string(),
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let output = build(&doc);
        assert!(output.contains(".\\\" This is a comment"));
    }
}
