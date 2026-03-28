//! Textile markup parser, AST, and emitter.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-textile` and `rescribe-write-textile` as thin adapter layers.

pub mod ast;
pub mod emit;
pub mod parse;

// ── Re-exports ────────────────────────────────────────────────────────────────

pub use ast::{Block, BlockAttrs, Diagnostic, Inline, Severity, Span, TableCell, TableRow, TextileDoc};
pub use emit::emit;
pub use parse::parse;

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_doc(input: &str) -> TextileDoc {
        let (doc, _diags) = parse(input);
        doc
    }

    #[test]
    fn test_parse_heading() {
        let doc = parse_doc("h1. Title\n");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Heading { level: 1, .. }));
    }

    #[test]
    fn test_parse_heading_levels() {
        let doc = parse_doc("h2. Level 2\nh3. Level 3\n");
        assert_eq!(doc.blocks.len(), 2);
        assert!(matches!(doc.blocks[0], Block::Heading { level: 2, .. }));
        assert!(matches!(doc.blocks[1], Block::Heading { level: 3, .. }));
    }

    #[test]
    fn test_parse_paragraph() {
        let doc = parse_doc("Hello world\n");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Paragraph { .. }));
    }

    #[test]
    fn test_parse_bold() {
        let doc = parse_doc("*bold*\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert_eq!(inlines.len(), 1);
        assert!(matches!(inlines[0], Inline::Bold(..)));
    }

    #[test]
    fn test_parse_italic() {
        let doc = parse_doc("_italic_\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert_eq!(inlines.len(), 1);
        assert!(matches!(inlines[0], Inline::Italic(..)));
    }

    #[test]
    fn test_parse_code() {
        let doc = parse_doc("@code@\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert_eq!(inlines.len(), 1);
        assert!(matches!(inlines[0], Inline::Code(..)));
    }

    #[test]
    fn test_parse_link() {
        let doc = parse_doc("\"Example\":https://example.com\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        let link = &inlines[0];
        assert!(matches!(link, Inline::Link { .. }));
        if let Inline::Link { url, .. } = link {
            assert_eq!(url, "https://example.com");
        }
    }

    #[test]
    fn test_parse_list() {
        let doc = parse_doc("* item1\n* item2\n");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::List { .. }));
        if let Block::List { items, .. } = &doc.blocks[0] {
            assert_eq!(items.len(), 2);
        }
    }

    #[test]
    fn test_parse_code_block() {
        let doc = parse_doc("bc. code here\n");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::CodeBlock { .. }));
    }

    #[test]
    fn test_emit_heading() {
        let doc = TextileDoc {
            blocks: vec![Block::Heading {
                level: 1,
                inlines: vec![Inline::Text("Title".to_string(), Span::dummy())],
                attrs: BlockAttrs::default(),
                span: Span::dummy(),
            }],
            span: Span::dummy(),
        };
        let output = emit(&doc);
        assert!(output.contains("h1. Title"));
    }

    #[test]
    fn test_emit_paragraph() {
        let doc = TextileDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Hello, world!".to_string(), Span::dummy())],
                align: None,
                attrs: BlockAttrs::default(),
                span: Span::dummy(),
            }],
            span: Span::dummy(),
        };
        let output = emit(&doc);
        assert!(output.contains("Hello, world!"));
    }

    #[test]
    fn test_emit_bold() {
        let doc = TextileDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Bold(
                    vec![Inline::Text("bold".to_string(), Span::dummy())],
                    Span::dummy(),
                )],
                align: None,
                attrs: BlockAttrs::default(),
                span: Span::dummy(),
            }],
            span: Span::dummy(),
        };
        let output = emit(&doc);
        assert!(output.contains("*bold*"));
    }

    #[test]
    fn test_emit_italic() {
        let doc = TextileDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Italic(
                    vec![Inline::Text("italic".to_string(), Span::dummy())],
                    Span::dummy(),
                )],
                align: None,
                attrs: BlockAttrs::default(),
                span: Span::dummy(),
            }],
            span: Span::dummy(),
        };
        let output = emit(&doc);
        assert!(output.contains("_italic_"));
    }

    #[test]
    fn test_emit_code() {
        let doc = TextileDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Code("code".to_string(), Span::dummy())],
                align: None,
                attrs: BlockAttrs::default(),
                span: Span::dummy(),
            }],
            span: Span::dummy(),
        };
        let output = emit(&doc);
        assert!(output.contains("@code@"));
    }

    #[test]
    fn test_emit_link() {
        let doc = TextileDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "https://example.com".to_string(),
                    title: None,
                    children: vec![Inline::Text("click".to_string(), Span::dummy())],
                    span: Span::dummy(),
                }],
                align: None,
                attrs: BlockAttrs::default(),
                span: Span::dummy(),
            }],
            span: Span::dummy(),
        };
        let output = emit(&doc);
        assert!(output.contains("\"click\":https://example.com"));
    }

    #[test]
    fn test_emit_list() {
        let doc = TextileDoc {
            blocks: vec![Block::List {
                ordered: false,
                items: vec![
                    vec![Block::Paragraph {
                        inlines: vec![Inline::Text("one".to_string(), Span::dummy())],
                        align: None,
                        attrs: BlockAttrs::default(),
                        span: Span::dummy(),
                    }],
                    vec![Block::Paragraph {
                        inlines: vec![Inline::Text("two".to_string(), Span::dummy())],
                        align: None,
                        attrs: BlockAttrs::default(),
                        span: Span::dummy(),
                    }],
                ],
                span: Span::dummy(),
            }],
            span: Span::dummy(),
        };
        let output = emit(&doc);
        assert!(output.contains("* one"));
        assert!(output.contains("* two"));
    }

    #[test]
    fn test_emit_ordered_list() {
        let doc = TextileDoc {
            blocks: vec![Block::List {
                ordered: true,
                items: vec![
                    vec![Block::Paragraph {
                        inlines: vec![Inline::Text("first".to_string(), Span::dummy())],
                        align: None,
                        attrs: BlockAttrs::default(),
                        span: Span::dummy(),
                    }],
                    vec![Block::Paragraph {
                        inlines: vec![Inline::Text("second".to_string(), Span::dummy())],
                        align: None,
                        attrs: BlockAttrs::default(),
                        span: Span::dummy(),
                    }],
                ],
                span: Span::dummy(),
            }],
            span: Span::dummy(),
        };
        let output = emit(&doc);
        assert!(output.contains("# first"));
        assert!(output.contains("# second"));
    }

    #[test]
    fn test_emit_code_block() {
        let doc = TextileDoc {
            blocks: vec![Block::CodeBlock {
                content: "print('hi')".to_string(),
                language: None,
                span: Span::dummy(),
            }],
            span: Span::dummy(),
        };
        let output = emit(&doc);
        assert!(output.contains("bc. print('hi')"));
    }

    #[test]
    fn test_parse_image() {
        let doc = parse_doc("!image.png!\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(matches!(inlines[0], Inline::Image { .. }));
    }

    #[test]
    fn test_emit_image() {
        let doc = TextileDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Image {
                    url: "image.png".to_string(),
                    alt: None,
                    span: Span::dummy(),
                }],
                align: None,
                attrs: BlockAttrs::default(),
                span: Span::dummy(),
            }],
            span: Span::dummy(),
        };
        let output = emit(&doc);
        assert!(output.contains("!image.png!"));
    }

    #[test]
    fn test_emit_image_with_alt() {
        let doc = TextileDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Image {
                    url: "image.png".to_string(),
                    alt: Some("alt text".to_string()),
                    span: Span::dummy(),
                }],
                align: None,
                attrs: BlockAttrs::default(),
                span: Span::dummy(),
            }],
            span: Span::dummy(),
        };
        let output = emit(&doc);
        assert!(output.contains("!image.png(alt text)!"));
    }
}
