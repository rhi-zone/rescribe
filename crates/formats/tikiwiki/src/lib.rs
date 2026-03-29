//! TikiWiki parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-tikiwiki` and `rescribe-write-tikiwiki` as thin adapter layers.

pub mod ast;
pub mod batch;
pub mod emit;
pub mod events;
pub mod parse;
pub mod writer;

// Re-export everything callers need.
pub use ast::{
    Block, Diagnostic, Inline, ListItem, Severity, Span, TableCell, TableRow, TikiwikiDoc,
};
pub use batch::{BatchParser, BatchSink, Handler, StreamingParser};
pub use emit::{build, collect_inline_text};
pub use events::{Event, EventIter, OwnedEvent};
pub use parse::parse;
pub use writer::Writer;

/// Parse `input` and return a streaming iterator of [`OwnedEvent`] items.
pub fn tikiwiki_events(input: &str) -> events::EventIter<'_> {
    events::events(input)
}

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
        let (doc, _) = parse("* Item 1\n* Item 2");
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
        assert!(out.contains("! Title"));
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
    fn test_parse_superscript() {
        let (doc, _) = parse("H^2^O");
        if let Block::Paragraph { ref inlines, .. } = doc.blocks[0] {
            assert!(inlines.iter().any(|i| matches!(i, Inline::Superscript(..))));
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    fn test_parse_subscript() {
        let (doc, _) = parse("H,,2,,O");
        if let Block::Paragraph { ref inlines, .. } = doc.blocks[0] {
            assert!(inlines.iter().any(|i| matches!(i, Inline::Subscript(..))));
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    fn test_parse_wikilink() {
        let (doc, _) = parse("See ((WikiWord))");
        if let Block::Paragraph { ref inlines, .. } = doc.blocks[0] {
            assert!(inlines.iter().any(|i| matches!(i, Inline::WikiLink { .. })));
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    fn test_parse_image() {
        let (doc, _) = parse("{img src=image.png}");
        if let Block::Paragraph { ref inlines, .. } = doc.blocks[0] {
            assert!(inlines.iter().any(|i| matches!(i, Inline::Image { .. })));
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    fn test_parse_nowiki() {
        let (doc, _) = parse("~np~raw __text__~/np~");
        if let Block::Paragraph { ref inlines, .. } = doc.blocks[0] {
            assert!(inlines.iter().any(|i| matches!(i, Inline::Nowiki(..))));
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    fn test_parse_blockquote() {
        let (doc, _) = parse("{QUOTE()}\nSome quoted text\n{QUOTE}");
        assert!(matches!(doc.blocks[0], Block::Blockquote { .. }));
    }

    #[test]
    fn test_roundtrip_simple() {
        let input = "! Heading\n\nParagraph text\n\n__bold__";
        let (doc, _) = parse(input);
        let output = build(&doc);
        let (doc2, _) = parse(&output);
        assert_eq!(doc.blocks.len(), doc2.blocks.len());
    }

    #[test]
    fn test_parse_sample_no_panic() {
        // Adversarial: arbitrary bytes must not panic
        let samples = [
            "",
            "!",
            "!!!!!!! too many",
            "__unclosed bold",
            "''unclosed italic",
            "{CODE()\nunclosed code block",
            "||unclosed|table",
            "***deeply nested",
            "[unclosed link",
            "((unclosed wikilink",
            "~np~unclosed nowiki",
            "---",
            "\n\n\n\n",
            "normal text",
            "{img src=}",
            "^unclosed super",
            ",,unclosed sub",
            "--unclosed strike",
        ];
        for sample in &samples {
            let _ = parse(sample);
        }
    }
}
