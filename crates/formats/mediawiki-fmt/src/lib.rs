//! MediaWiki markup parser, AST, and emitter.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-mediawiki` and `rescribe-write-mediawiki` as thin adapter layers.

pub mod ast;
pub mod batch;
pub mod emit;
pub mod events;
pub mod parse;
pub mod writer;

// Re-export everything callers need.
pub use ast::{
    Block, DefinitionItem, Diagnostic, Inline, MediawikiDoc, Severity, Span, TableCell,
    TableRow,
};
pub use batch::{BatchParser, BatchSink, Handler, StreamingParser};
pub use emit::emit;
pub use events::{Event, EventIter, OwnedEvent};
pub use parse::parse;
pub use writer::Writer;

/// Parse `input` and return a streaming iterator of [`OwnedEvent`] items.
pub fn events(input: &str) -> events::EventIter {
    events::events(input)
}

// -- Tests --------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_text() {
        let (doc, _) = parse("Some simple text");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Paragraph { .. }));
    }

    #[test]
    fn test_parse_heading() {
        let (doc, _) = parse("== Heading ==");
        assert_eq!(doc.blocks.len(), 1);
        if let Block::Heading { level, .. } = &doc.blocks[0] {
            assert_eq!(*level, 2);
        } else {
            panic!("expected heading");
        }
    }

    #[test]
    fn test_parse_bold() {
        let (doc, _) = parse("'''bold'''");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Bold(_))));
    }

    #[test]
    fn test_parse_italic() {
        let (doc, _) = parse("''italic''");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Italic(_))));
    }

    #[test]
    fn test_parse_list() {
        let (doc, _) = parse("* Item 1\n* Item 2");
        assert_eq!(doc.blocks.len(), 1);
        let Block::List { ordered, items, .. } = &doc.blocks[0] else {
            panic!("expected list");
        };
        assert!(!ordered);
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_parse_internal_link() {
        let (doc, _) = parse("[[Title|Link text]]");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        let has_link = inlines.iter().any(|i| {
            if let Inline::Link { url, .. } = i { url == "Title" } else { false }
        });
        assert!(has_link);
    }

    #[test]
    fn test_parse_external_link() {
        let (doc, _) = parse("[https://example.com Example]");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        let has_link = inlines.iter().any(|i| {
            if let Inline::Link { url, .. } = i {
                url == "https://example.com"
            } else {
                false
            }
        });
        assert!(has_link);
    }

    #[test]
    fn test_parse_horizontal_rule() {
        let (doc, _) = parse("----");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::HorizontalRule));
    }

    #[test]
    fn test_parse_code_block() {
        let (doc, _) = parse(" code line 1\n code line 2");
        assert_eq!(doc.blocks.len(), 1);
        if let Block::CodeBlock { content, .. } = &doc.blocks[0] {
            assert!(content.contains("code line"));
        } else {
            panic!("expected code block");
        }
    }

    #[test]
    fn test_parse_blockquote() {
        let (doc, _) = parse("<blockquote>\nSome quoted text.\n</blockquote>");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Blockquote { .. }));
    }

    #[test]
    fn test_parse_definition_list() {
        let (doc, _) = parse("; Term\n: Definition");
        assert_eq!(doc.blocks.len(), 1);
        if let Block::DefinitionList { items, .. } = &doc.blocks[0] {
            assert_eq!(items.len(), 1);
        } else {
            panic!("expected definition list");
        }
    }

    #[test]
    fn test_parse_footnote_ref() {
        let (doc, _) = parse("Text<ref>footnote content</ref>.");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(
            i,
            Inline::FootnoteRef { content: Some(_), .. }
        )));
    }

    #[test]
    fn test_parse_math_inline() {
        let (doc, _) = parse("Solve <math>x^2</math>.");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::MathInline { .. })));
    }

    #[test]
    fn test_parse_nowiki() {
        let (doc, _) = parse("Some <nowiki>'''not bold'''</nowiki> text.");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Nowiki { .. })));
    }

    #[test]
    fn test_parse_template() {
        let (doc, _) = parse("See {{cite|author=Smith}}.");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Template { .. })));
    }

    #[test]
    fn test_parse_syntaxhighlight() {
        let (doc, _) =
            parse("<syntaxhighlight lang=\"python\">\nprint(\"hello\")\n</syntaxhighlight>");
        assert_eq!(doc.blocks.len(), 1);
        if let Block::CodeBlock { language, content, .. } = &doc.blocks[0] {
            assert_eq!(language.as_deref(), Some("python"));
            assert!(content.contains("print"));
        } else {
            panic!("expected code block, got {:?}", doc.blocks[0]);
        }
    }

    #[test]
    fn test_emit_heading() {
        let doc = MediawikiDoc {
            blocks: vec![Block::Heading {
                level: 2,
                inlines: vec![Inline::Text("Title".into())],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = emit(&doc);
        assert!(out.contains("== Title =="));
    }

    #[test]
    fn test_emit_bold() {
        let doc = MediawikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Bold(vec![Inline::Text("bold".into())])],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = emit(&doc);
        assert!(out.contains("'''bold'''"));
    }

    #[test]
    fn test_emit_italic() {
        let doc = MediawikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Italic(vec![Inline::Text("italic".into())])],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = emit(&doc);
        assert!(out.contains("''italic''"));
    }

    #[test]
    fn test_emit_list() {
        let doc = MediawikiDoc {
            blocks: vec![Block::List {
                ordered: false,
                items: vec![
                    vec![Block::Paragraph {
                        inlines: vec![Inline::Text("Item 1".into())],
                        span: Span::NONE,
                    }],
                    vec![Block::Paragraph {
                        inlines: vec![Inline::Text("Item 2".into())],
                        span: Span::NONE,
                    }],
                ],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = emit(&doc);
        assert!(out.contains("* Item 1"));
        assert!(out.contains("* Item 2"));
    }

    #[test]
    fn test_emit_internal_link() {
        let doc = MediawikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "Title".to_string(),
                    text: "Link text".to_string(),
                }],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = emit(&doc);
        assert!(out.contains("[[Title|Link text]]"));
    }

    #[test]
    fn test_emit_external_link() {
        let doc = MediawikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "https://example.com".to_string(),
                    text: "Example".to_string(),
                }],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = emit(&doc);
        assert!(out.contains("[https://example.com Example]"));
    }

    #[test]
    fn test_roundtrip_simple() {
        let input = "== Heading ==\n\nSome '''bold''' text.";
        let (doc, _) = parse(input);
        let output = emit(&doc);
        // Output should be parseable again
        let (doc2, _) = parse(&output);
        assert!(!doc2.blocks.is_empty());
    }
}
