//! Org-mode parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-org` and `rescribe-write-org` as thin adapter layers.

pub mod ast;
pub mod batch;
pub mod emit;
pub mod events;
pub mod parse;
pub mod writer;

// Re-export everything callers need.
pub use ast::{
    Block, CheckboxState, DefinitionItem, Diagnostic, Inline, ListItem, ListItemContent, OrgDoc,
    OrgError, Severity, Span, TableRow, merge_text_inlines,
};
pub use batch::{BatchParser, BatchSink, Handler, StreamingParser};
pub use emit::build;
pub use events::{Event, EventIter, OwnedEvent};
pub use parse::parse;
pub use writer::Writer;

/// Parse `input` and return a streaming iterator of [`OwnedEvent`] items.
pub fn events(input: &str) -> events::EventIter<'_> {
    events::events(input)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(input: &str) -> OrgDoc {
        let (doc, _diags) = parse(input);
        doc
    }

    // ── Parser tests ──────────────────────────────────────────────────────────

    #[test]
    fn test_parse_heading() {
        let doc = parse_ok("* Hello World\n** Subheading");
        assert_eq!(doc.blocks.len(), 2);
        assert!(matches!(doc.blocks[0], Block::Heading { level: 1, .. }));
        assert!(matches!(doc.blocks[1], Block::Heading { level: 2, .. }));
    }

    #[test]
    fn test_parse_paragraph() {
        let doc = parse_ok("This is a paragraph.\n\nThis is another.");
        assert_eq!(doc.blocks.len(), 2);
        assert!(matches!(doc.blocks[0], Block::Paragraph { .. }));
        assert!(matches!(doc.blocks[1], Block::Paragraph { .. }));
    }

    #[test]
    fn test_parse_emphasis() {
        let doc = parse_ok("/italic/ and *bold*");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Italic(..))));
        assert!(inlines.iter().any(|i| matches!(i, Inline::Bold(..))));
    }

    #[test]
    fn test_parse_list() {
        let doc = parse_ok("- First item\n- Second item");
        assert!(!doc.blocks.is_empty());
        let Block::List { ordered, items, .. } = &doc.blocks[0] else {
            panic!("expected list");
        };
        assert!(!ordered);
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_parse_ordered_list() {
        let doc = parse_ok("1. First\n2. Second");
        let Block::List { ordered, .. } = &doc.blocks[0] else {
            panic!("expected list");
        };
        assert!(ordered);
    }

    #[test]
    fn test_parse_code_block() {
        let doc = parse_ok("#+BEGIN_SRC rust\nfn main() {}\n#+END_SRC");
        let Block::CodeBlock { language, content, .. } = &doc.blocks[0] else {
            panic!("expected code block");
        };
        assert_eq!(language.as_deref(), Some("rust"));
        assert_eq!(content, "fn main() {}");
    }

    #[test]
    fn test_parse_metadata() {
        let doc = parse_ok("#+TITLE: My Document\n#+AUTHOR: Jane Doe\n\nContent here.");
        assert!(
            doc.metadata
                .iter()
                .any(|(k, v)| k == "title" && v == "My Document")
        );
        assert!(
            doc.metadata
                .iter()
                .any(|(k, v)| k == "author" && v == "Jane Doe")
        );
    }

    #[test]
    fn test_parse_blockquote() {
        let doc = parse_ok("#+BEGIN_QUOTE\nSome quoted text\n#+END_QUOTE");
        assert!(matches!(doc.blocks[0], Block::Blockquote { .. }));
    }

    #[test]
    fn test_parse_horizontal_rule() {
        let doc = parse_ok("-----");
        assert!(matches!(doc.blocks[0], Block::HorizontalRule { .. }));
    }

    #[test]
    fn test_parse_link() {
        let doc = parse_ok("[[https://example.com][click here]]");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        let Inline::Link { url, .. } = &inlines[0] else {
            panic!("expected link");
        };
        assert_eq!(url, "https://example.com");
    }

    #[test]
    fn test_parse_code_inline() {
        let doc = parse_ok("Some =verbatim= text");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Code(..))));
    }

    // ── Builder tests ─────────────────────────────────────────────────────────

    fn build_str(doc: &OrgDoc) -> String {
        build(doc)
    }

    fn simple_doc(block: Block) -> OrgDoc {
        OrgDoc {
            blocks: vec![block],
            metadata: vec![],
        }
    }

    #[test]
    fn test_build_paragraph() {
        let doc = simple_doc(Block::Paragraph {
            inlines: vec![Inline::Text {
                text: "Hello, world!".into(),
                span: Span::NONE,
            }],
            span: Span::NONE,
        });
        assert!(build_str(&doc).contains("Hello, world!"));
    }

    #[test]
    fn test_build_heading() {
        let doc = simple_doc(Block::Heading {
            level: 1,
            todo: None,
            priority: None,
            tags: vec![],
            properties: vec![],
            scheduled: None,
            deadline: None,
            inlines: vec![Inline::Text {
                text: "Main Title".into(),
                span: Span::NONE,
            }],
            span: Span::NONE,
        });
        assert!(build_str(&doc).contains("* Main Title"));
    }

    #[test]
    fn test_build_heading_levels() {
        let doc = OrgDoc {
            blocks: vec![
                Block::Heading {
                    level: 1,
                    todo: None,
                    priority: None,
                    tags: vec![],
                    properties: vec![],
                    scheduled: None,
                    deadline: None,
                    inlines: vec![Inline::Text {
                        text: "Level 1".into(),
                        span: Span::NONE,
                    }],
                    span: Span::NONE,
                },
                Block::Heading {
                    level: 2,
                    todo: None,
                    priority: None,
                    tags: vec![],
                    properties: vec![],
                    scheduled: None,
                    deadline: None,
                    inlines: vec![Inline::Text {
                        text: "Level 2".into(),
                        span: Span::NONE,
                    }],
                    span: Span::NONE,
                },
                Block::Heading {
                    level: 3,
                    todo: None,
                    priority: None,
                    tags: vec![],
                    properties: vec![],
                    scheduled: None,
                    deadline: None,
                    inlines: vec![Inline::Text {
                        text: "Level 3".into(),
                        span: Span::NONE,
                    }],
                    span: Span::NONE,
                },
            ],
            metadata: vec![],
        };
        let out = build_str(&doc);
        assert!(out.contains("* Level 1"));
        assert!(out.contains("** Level 2"));
        assert!(out.contains("*** Level 3"));
    }

    #[test]
    fn test_build_emphasis() {
        let doc = simple_doc(Block::Paragraph {
            inlines: vec![Inline::Italic(
                vec![Inline::Text {
                    text: "italic".into(),
                    span: Span::NONE,
                }],
                Span::NONE,
            )],
            span: Span::NONE,
        });
        assert!(build_str(&doc).contains("/italic/"));
    }

    #[test]
    fn test_build_strong() {
        let doc = simple_doc(Block::Paragraph {
            inlines: vec![Inline::Bold(
                vec![Inline::Text {
                    text: "bold".into(),
                    span: Span::NONE,
                }],
                Span::NONE,
            )],
            span: Span::NONE,
        });
        assert!(build_str(&doc).contains("*bold*"));
    }

    #[test]
    fn test_build_link() {
        let doc = simple_doc(Block::Paragraph {
            inlines: vec![Inline::Link {
                url: "https://example.com".into(),
                children: vec![Inline::Text {
                    text: "click".into(),
                    span: Span::NONE,
                }],
                span: Span::NONE,
            }],
            span: Span::NONE,
        });
        assert!(build_str(&doc).contains("[[https://example.com][click]]"));
    }

    #[test]
    fn test_build_unordered_list() {
        let doc = simple_doc(Block::List {
            ordered: false,
            start: None,
            items: vec![
                ListItem {
                    children: vec![ListItemContent::Inline(vec![Inline::Text {
                        text: "item 1".into(),
                        span: Span::NONE,
                    }])],
                    checkbox: None,
                },
                ListItem {
                    children: vec![ListItemContent::Inline(vec![Inline::Text {
                        text: "item 2".into(),
                        span: Span::NONE,
                    }])],
                    checkbox: None,
                },
            ],
            span: Span::NONE,
        });
        let out = build_str(&doc);
        assert!(out.contains("- item 1"));
        assert!(out.contains("- item 2"));
    }

    #[test]
    fn test_build_ordered_list() {
        let doc = simple_doc(Block::List {
            ordered: true,
            start: None,
            items: vec![
                ListItem {
                    children: vec![ListItemContent::Inline(vec![Inline::Text {
                        text: "first".into(),
                        span: Span::NONE,
                    }])],
                    checkbox: None,
                },
                ListItem {
                    children: vec![ListItemContent::Inline(vec![Inline::Text {
                        text: "second".into(),
                        span: Span::NONE,
                    }])],
                    checkbox: None,
                },
            ],
            span: Span::NONE,
        });
        let out = build_str(&doc);
        assert!(out.contains("1. first"));
        assert!(out.contains("2. second"));
    }

    #[test]
    fn test_build_code_block() {
        let doc = simple_doc(Block::CodeBlock {
            language: Some("rust".into()),
            header_args: None,
            name: None,
            content: "fn main() {}".into(),
            span: Span::NONE,
        });
        let out = build_str(&doc);
        assert!(out.contains("#+BEGIN_SRC rust"));
        assert!(out.contains("fn main() {}"));
        assert!(out.contains("#+END_SRC"));
    }

    #[test]
    fn test_build_blockquote() {
        let doc = simple_doc(Block::Blockquote {
            children: vec![Block::Paragraph {
                inlines: vec![Inline::Text {
                    text: "A quote".into(),
                    span: Span::NONE,
                }],
                span: Span::NONE,
            }],
            span: Span::NONE,
        });
        let out = build_str(&doc);
        assert!(out.contains("#+BEGIN_QUOTE"));
        assert!(out.contains("A quote"));
        assert!(out.contains("#+END_QUOTE"));
    }

    #[test]
    fn test_parse_superscript() {
        let doc = parse_ok("H^{2}O");
        if let Block::Paragraph { ref inlines, .. } = doc.blocks[0] {
            assert!(inlines.iter().any(|n| matches!(n, Inline::Superscript(..))));
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    fn test_parse_subscript() {
        let doc = parse_ok("H_{2}O");
        if let Block::Paragraph { ref inlines, .. } = doc.blocks[0] {
            assert!(inlines.iter().any(|n| matches!(n, Inline::Subscript(..))));
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    fn test_parse_table() {
        let doc = parse_ok("| Name | Age |\n|------+-----|\n| Alice | 30 |");
        assert!(matches!(doc.blocks[0], Block::Table { .. }));
        if let Block::Table { ref rows, .. } = doc.blocks[0] {
            assert_eq!(rows.len(), 2);
            assert!(rows[0].is_header);
            assert!(!rows[1].is_header);
            assert_eq!(rows[0].cells.len(), 2);
        }
    }

    #[test]
    fn test_parse_definition_list() {
        let doc = parse_ok("- Term :: Description");
        assert!(matches!(doc.blocks[0], Block::DefinitionList { .. }));
        if let Block::DefinitionList { ref items, .. } = doc.blocks[0] {
            assert_eq!(items.len(), 1);
        }
    }

    #[test]
    fn test_parse_footnote_ref() {
        let doc = parse_ok("See note [fn:1].");
        if let Block::Paragraph { ref inlines, .. } = doc.blocks[0] {
            assert!(inlines
                .iter()
                .any(|n| matches!(n, Inline::FootnoteRef { label, .. } if label == "1")));
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    fn test_parse_math_inline() {
        let doc = parse_ok("Solve $x^2 + y^2 = r^2$.");
        if let Block::Paragraph { ref inlines, .. } = doc.blocks[0] {
            assert!(inlines.iter().any(|n| matches!(n, Inline::MathInline { .. })));
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    fn test_parse_block_footnote_def() {
        let doc = parse_ok("[fn:1] This is the footnote text.");
        assert_eq!(doc.blocks.len(), 1);
        if let Block::FootnoteDef { ref label, ref content, .. } = doc.blocks[0] {
            assert_eq!(label, "1");
            assert!(content.iter().any(|n| matches!(n, Inline::Text { text, .. } if text.contains("footnote text"))));
        } else {
            panic!("expected FootnoteDef block, got {:?}", doc.blocks[0]);
        }
    }

    #[test]
    fn test_build_block_footnote_def() {
        let doc = simple_doc(Block::FootnoteDef {
            label: "1".into(),
            content: vec![Inline::Text {
                text: "Footnote content here.".into(),
                span: Span::NONE,
            }],
            span: Span::NONE,
        });
        let out = build_str(&doc);
        assert!(out.contains("[fn:1]"), "expected [fn:1] in: {out}");
        assert!(out.contains("Footnote content here."), "expected content in: {out}");
    }

    #[test]
    fn test_roundtrip_block_footnote_def() {
        let input = "[fn:note] Some footnote content.";
        let (doc, _) = parse(input);
        let emitted = build_str(&doc);
        let (doc2, _) = parse(&emitted);
        assert_eq!(doc.blocks.len(), 1);
        assert_eq!(doc2.blocks.len(), 1);
        // strip_spans equality
        assert!(matches!(doc.blocks[0].strip_spans(), Block::FootnoteDef { .. }));
        assert!(matches!(doc2.blocks[0].strip_spans(), Block::FootnoteDef { .. }));
        if let (Block::FootnoteDef { label: l1, .. }, Block::FootnoteDef { label: l2, .. }) =
            (&doc.blocks[0], &doc2.blocks[0])
        {
            assert_eq!(l1, l2);
        }
    }
}
