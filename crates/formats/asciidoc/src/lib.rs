//! AsciiDoc parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-asciidoc` and `rescribe-write-asciidoc` as thin adapter layers.

#![allow(clippy::collapsible_if)]

mod ast;
pub mod batch;
mod emit;
mod events;
mod parse;
pub mod writer;

// Re-exports
pub use ast::{
    AsciiDoc, AsciiDocError, Block, DefinitionItem, Diagnostic, ImageData, Inline, QuoteType,
    Severity, Span, TableRow,
};
pub use emit::build;
pub use batch::{BatchParser, BatchSink};
pub use events::{Event, EventIter, OwnedEvent};
pub use writer::Writer;
pub use parse::{parse, parse_inline_content};

/// Return a streaming event iterator over the AsciiDoc source.
pub fn events(input: &str) -> EventIter<'_> {
    parse::EventIter::new(input)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_heading() {
        let (doc, _) = parse("== Hello World\n\nSome text.");
        assert_eq!(doc.blocks.len(), 2);
        assert!(matches!(&doc.blocks[0], Block::Heading { level: 2, .. }));
    }

    #[test]
    fn test_parse_paragraph() {
        let (doc, _) = parse("This is a paragraph.\n\nThis is another.");
        assert_eq!(doc.blocks.len(), 2);
        assert!(matches!(&doc.blocks[0], Block::Paragraph { .. }));
        assert!(matches!(&doc.blocks[1], Block::Paragraph { .. }));
    }

    #[test]
    fn test_parse_strong() {
        let (doc, _) = parse("This is *strong* text.");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Strong(..))));
    }

    #[test]
    fn test_parse_emphasis() {
        let (doc, _) = parse("This is _emphasized_ text.");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Emphasis(..))));
    }

    #[test]
    fn test_parse_bullet_list() {
        let (doc, _) = parse("* First item\n* Second item\n* Third item");
        assert_eq!(doc.blocks.len(), 1);
        let Block::List { ordered, items, .. } = &doc.blocks[0] else {
            panic!("expected list");
        };
        assert!(!ordered);
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn test_parse_numbered_list() {
        let (doc, _) = parse(". First item\n. Second item\n. Third item");
        assert_eq!(doc.blocks.len(), 1);
        let Block::List { ordered, items, .. } = &doc.blocks[0] else {
            panic!("expected list");
        };
        assert!(ordered);
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn test_parse_code_block() {
        let (doc, _) = parse("[source,python]\n----\nprint('hello')\n----");
        assert_eq!(doc.blocks.len(), 1);
        let Block::CodeBlock { language, content, .. } = &doc.blocks[0] else {
            panic!("expected code block");
        };
        assert_eq!(language.as_deref(), Some("python"));
        assert!(content.contains("print('hello')"));
    }

    #[test]
    fn test_parse_inline_code() {
        let (doc, _) = parse("Use `code here` in text.");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Code(..))));
    }

    #[test]
    fn test_parse_link() {
        let (doc, _) = parse("Visit https://example.com[Example Site] for more.");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        let link = inlines.iter().find(|i| matches!(i, Inline::Link { .. }));
        assert!(link.is_some());
        if let Some(Inline::Link { url, .. }) = link {
            assert_eq!(url, "https://example.com");
        }
    }

    #[test]
    fn test_parse_block_image() {
        let (doc, _) = parse("image::path/to/image.png[Alt text]");
        assert_eq!(doc.blocks.len(), 1);
        let Block::Figure { image, .. } = &doc.blocks[0] else {
            panic!("expected figure");
        };
        assert_eq!(image.url, "path/to/image.png");
        assert_eq!(image.alt.as_deref(), Some("Alt text"));
    }

    #[test]
    fn test_build_paragraph() {
        let doc = AsciiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text {
                    text: "Hello, world!".into(),
                    span: Span::NONE,
                }],
                id: None,
                role: None,
                checked: None,
                span: Span::NONE,
            }],
            attributes: Default::default(),
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("Hello, world!"));
    }

    #[test]
    fn test_build_heading() {
        let doc = AsciiDoc {
            blocks: vec![Block::Heading {
                level: 1,
                inlines: vec![Inline::Text {
                    text: "Title".into(),
                    span: Span::NONE,
                }],
                id: None,
                role: None,
                span: Span::NONE,
            }],
            attributes: Default::default(),
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("== Title"));
    }

    #[test]
    fn test_build_code_block() {
        let doc = AsciiDoc {
            blocks: vec![Block::CodeBlock {
                content: "print('hi')".into(),
                language: Some("python".into()),
                span: Span::NONE,
            }],
            attributes: Default::default(),
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("[source,python]"));
        assert!(out.contains("----"));
        assert!(out.contains("print('hi')"));
    }

    #[test]
    fn test_build_list() {
        let doc = AsciiDoc {
            blocks: vec![Block::List {
                ordered: false,
                items: vec![
                    vec![Block::Paragraph {
                        inlines: vec![Inline::Text {
                            text: "one".into(),
                            span: Span::NONE,
                        }],
                        id: None,
                        role: None,
                        checked: None,
                        span: Span::NONE,
                    }],
                    vec![Block::Paragraph {
                        inlines: vec![Inline::Text {
                            text: "two".into(),
                            span: Span::NONE,
                        }],
                        id: None,
                        role: None,
                        checked: None,
                        span: Span::NONE,
                    }],
                ],
                style: None,
                span: Span::NONE,
            }],
            attributes: Default::default(),
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("* one"));
        assert!(out.contains("* two"));
    }

    #[test]
    fn test_build_strong() {
        let doc = AsciiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Strong(
                    vec![Inline::Text {
                        text: "bold".into(),
                        span: Span::NONE,
                    }],
                    Span::NONE,
                )],
                id: None,
                role: None,
                checked: None,
                span: Span::NONE,
            }],
            attributes: Default::default(),
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("*bold*"));
    }

    #[test]
    fn test_build_emphasis() {
        let doc = AsciiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Emphasis(
                    vec![Inline::Text {
                        text: "italic".into(),
                        span: Span::NONE,
                    }],
                    Span::NONE,
                )],
                id: None,
                role: None,
                checked: None,
                span: Span::NONE,
            }],
            attributes: Default::default(),
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("_italic_"));
    }

    #[test]
    fn test_build_inline_code() {
        let doc = AsciiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Code("code".into(), Span::NONE)],
                id: None,
                role: None,
                checked: None,
                span: Span::NONE,
            }],
            attributes: Default::default(),
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("`code`"));
    }

    #[test]
    fn test_build_link() {
        let doc = AsciiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "https://example.com".into(),
                    children: vec![Inline::Text {
                        text: "click".into(),
                        span: Span::NONE,
                    }],
                    target: None,
                    span: Span::NONE,
                }],
                id: None,
                role: None,
                checked: None,
                span: Span::NONE,
            }],
            attributes: Default::default(),
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("https://example.com[click]"));
    }
}
