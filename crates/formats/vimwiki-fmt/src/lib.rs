//! VimWiki parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-vimwiki` and `rescribe-write-vimwiki` as thin adapter layers.

pub mod ast;
pub mod batch;
pub mod emit;
pub mod events;
pub mod parse;
pub mod writer;

pub use ast::{
    Block, DefinitionItem, Diagnostic, Inline, ListItem, Severity, Span, TableRow, VimwikiDoc,
};
pub use batch::{BatchParser, BatchSink, Handler, StreamingParser};
pub use emit::{build, collect_inline_text};
pub use events::{Event, EventIter, OwnedEvent};
pub use parse::parse;
pub use writer::Writer;

/// Parse `input` and return a streaming iterator of events.
pub fn events(input: &str) -> events::EventIter<'_> {
    events::events(input)
}

// -- Tests --------------------------------------------------------------------

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
    fn test_parse_heading_level3() {
        let (doc, _) = parse("=== H3 ===\n");
        assert!(matches!(doc.blocks[0], Block::Heading { level: 3, .. }));
    }

    #[test]
    fn test_parse_heading_level4() {
        let (doc, _) = parse("==== H4 ====\n");
        assert!(matches!(doc.blocks[0], Block::Heading { level: 4, .. }));
    }

    #[test]
    fn test_parse_heading_level5() {
        let (doc, _) = parse("===== H5 =====\n");
        assert!(matches!(doc.blocks[0], Block::Heading { level: 5, .. }));
    }

    #[test]
    fn test_parse_heading_level6() {
        let (doc, _) = parse("====== H6 ======\n");
        assert!(matches!(doc.blocks[0], Block::Heading { level: 6, .. }));
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
    fn test_parse_superscript() {
        let (doc, _) = parse("^super^\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines
            .iter()
            .any(|i| matches!(i, Inline::Superscript(_, _))));
    }

    #[test]
    fn test_parse_subscript() {
        let (doc, _) = parse(",,sub,,\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines
            .iter()
            .any(|i| matches!(i, Inline::Subscript(_, _))));
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
    fn test_parse_hash_ordered_list() {
        let (doc, _) = parse("# first\n# second\n");
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
    fn test_parse_definition_list() {
        let (doc, _) = parse("; Term\n: Definition\n");
        assert!(matches!(doc.blocks[0], Block::DefinitionList { .. }));
        let Block::DefinitionList { items, .. } = &doc.blocks[0] else {
            panic!("expected definition list");
        };
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn test_parse_image_with_style() {
        let (doc, _) = parse("{{img.png|alt|width:100px}}\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        let img = inlines
            .iter()
            .find(|i| matches!(i, Inline::Image { .. }))
            .unwrap();
        assert!(
            matches!(img, Inline::Image { url, alt: Some(a), style: Some(s), .. } if url == "img.png" && a == "alt" && s == "width:100px")
        );
    }

    #[test]
    fn test_parse_comment_skipped() {
        let (doc, _) = parse("%% This is a comment\nHello\n");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Paragraph { .. }));
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
    fn test_build_superscript() {
        let doc = VimwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Superscript(
                    vec![Inline::Text("sup".to_string(), Span::NONE)],
                    Span::NONE,
                )],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let output = build(&doc);
        assert!(output.contains("^sup^"));
    }

    #[test]
    fn test_build_subscript() {
        let doc = VimwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Subscript(
                    vec![Inline::Text("sub".to_string(), Span::NONE)],
                    Span::NONE,
                )],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let output = build(&doc);
        assert!(output.contains(",,sub,,"));
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

    #[test]
    fn test_build_definition_list() {
        let doc = VimwikiDoc {
            blocks: vec![Block::DefinitionList {
                items: vec![DefinitionItem {
                    term: vec![Inline::Text("Term".to_string(), Span::NONE)],
                    desc: vec![Inline::Text("Definition".to_string(), Span::NONE)],
                    span: Span::NONE,
                }],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let output = build(&doc);
        assert!(output.contains("; Term"));
        assert!(output.contains(": Definition"));
    }

    #[test]
    fn test_roundtrip_basic() {
        let input = "= Hello =\n\nA paragraph with *bold* and _italic_ text.\n\n* item one\n* item two\n";
        let (doc, _) = parse(input);
        let output = build(&doc);
        let (doc2, _) = parse(&output);
        assert_eq!(doc.blocks.len(), doc2.blocks.len());
    }

    #[test]
    fn test_inline_preformatted() {
        let (doc, _) = parse("Use {{{inline code}}} here.\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Code(s, _) if s == "inline code")));
    }
}
