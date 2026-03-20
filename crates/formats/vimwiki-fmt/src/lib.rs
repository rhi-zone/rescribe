//! VimWiki parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-vimwiki` and `rescribe-write-vimwiki` as thin adapter layers.

pub mod ast;
pub mod emit;
pub mod parse;

pub use ast::{Block, Diagnostic, Inline, ListItem, Severity, Span, TableRow, VimwikiDoc};
pub use emit::{build, collect_inline_text};
pub use parse::parse;

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_heading() {
        let (doc, _) = parse("= Title =\n");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Heading { level: 1, .. }));
    }

    #[test]
    fn test_parse_heading_level2() {
        let (doc, _) = parse("== Subtitle ==\n");
        assert!(matches!(doc.blocks[0], Block::Heading { level: 2, .. }));
    }

    #[test]
    fn test_parse_paragraph() {
        let (doc, _) = parse("Hello world\n");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Paragraph { .. }));
    }

    #[test]
    fn test_parse_bold() {
        let (doc, _) = parse("*bold*\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Bold(_, _))));
    }

    #[test]
    fn test_parse_italic() {
        let (doc, _) = parse("_italic_\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Italic(_, _))));
    }

    #[test]
    fn test_parse_code() {
        let (doc, _) = parse("`code`\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Code(_, _))));
    }

    #[test]
    fn test_parse_wiki_link() {
        let (doc, _) = parse("[[MyPage]]\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Link { .. })));
    }

    #[test]
    fn test_parse_wiki_link_with_description() {
        let (doc, _) = parse("[[MyPage|click here]]\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        let link = inlines
            .iter()
            .find(|i| matches!(i, Inline::Link { .. }))
            .unwrap();
        assert!(
            matches!(link, Inline::Link { url, label, .. } if url == "MyPage" && label == "click here")
        );
    }

    #[test]
    fn test_parse_unordered_list() {
        let (doc, _) = parse("* item1\n* item2\n");
        assert_eq!(doc.blocks.len(), 1);
        let Block::List { ordered, items, .. } = &doc.blocks[0] else {
            panic!("expected list");
        };
        assert!(!ordered);
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_parse_ordered_list() {
        let (doc, _) = parse("1. first\n2. second\n");
        let Block::List { ordered, items, .. } = &doc.blocks[0] else {
            panic!("expected list");
        };
        assert!(*ordered);
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_parse_preformatted() {
        let (doc, _) = parse("{{{\ncode here\n}}}\n");
        assert!(matches!(doc.blocks[0], Block::CodeBlock { .. }));
    }

    #[test]
    fn test_parse_checkbox() {
        let (doc, _) = parse("* [ ] unchecked\n* [X] checked\n");
        let Block::List { items, .. } = &doc.blocks[0] else {
            panic!("expected list");
        };
        assert_eq!(items[0].checked, Some(false));
        assert_eq!(items[1].checked, Some(true));
    }

    #[test]
    fn test_build_heading() {
        let doc = VimwikiDoc {
            blocks: vec![Block::Heading {
                level: 1,
                inlines: vec![Inline::Text("Title".to_string(), Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let output = build(&doc);
        assert!(output.contains("= Title ="));
    }

    #[test]
    fn test_build_paragraph() {
        let doc = VimwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Hello, world!".to_string(), Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let output = build(&doc);
        assert!(output.contains("Hello, world!"));
    }

    #[test]
    fn test_build_bold() {
        let doc = VimwikiDoc {
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
        assert!(output.contains("*bold*"));
    }

    #[test]
    fn test_build_italic() {
        let doc = VimwikiDoc {
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
        assert!(output.contains("_italic_"));
    }

    #[test]
    fn test_build_code() {
        let doc = VimwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Code("code".to_string(), Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let output = build(&doc);
        assert!(output.contains("`code`"));
    }

    #[test]
    fn test_build_link() {
        let doc = VimwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "MyPage".to_string(),
                    label: "click".to_string(),
                    span: Span::NONE,
                }],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let output = build(&doc);
        assert!(output.contains("[[MyPage|click]]"));
    }

    #[test]
    fn test_build_unordered_list() {
        let doc = VimwikiDoc {
            blocks: vec![Block::List {
                ordered: false,
                items: vec![
                    ListItem {
                        checked: None,
                        inlines: vec![Inline::Text("one".to_string(), Span::NONE)],
                        span: Span::NONE,
                    },
                    ListItem {
                        checked: None,
                        inlines: vec![Inline::Text("two".to_string(), Span::NONE)],
                        span: Span::NONE,
                    },
                ],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let output = build(&doc);
        assert!(output.contains("* one"));
        assert!(output.contains("* two"));
    }

    #[test]
    fn test_build_ordered_list() {
        let doc = VimwikiDoc {
            blocks: vec![Block::List {
                ordered: true,
                items: vec![
                    ListItem {
                        checked: None,
                        inlines: vec![Inline::Text("first".to_string(), Span::NONE)],
                        span: Span::NONE,
                    },
                    ListItem {
                        checked: None,
                        inlines: vec![Inline::Text("second".to_string(), Span::NONE)],
                        span: Span::NONE,
                    },
                ],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let output = build(&doc);
        assert!(output.contains("1. first"));
        assert!(output.contains("2. second"));
    }

    #[test]
    fn test_build_code_block() {
        let doc = VimwikiDoc {
            blocks: vec![Block::CodeBlock {
                language: None,
                content: "print hi".to_string(),
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let output = build(&doc);
        assert!(output.contains("{{{"));
        assert!(output.contains("print hi"));
        assert!(output.contains("}}}"));
    }

    #[test]
    fn test_build_horizontal_rule() {
        let doc = VimwikiDoc {
            blocks: vec![Block::HorizontalRule { span: Span::NONE }],
            span: Span::NONE,
        };
        let output = build(&doc);
        assert!(output.contains("----"));
    }
}
