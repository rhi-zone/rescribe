//! Jira wiki markup parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-jira` and `rescribe-write-jira` as thin adapter layers.

pub mod ast;
pub mod batch;
pub mod emit;
pub mod events;
pub mod parse;
pub mod writer;

pub use ast::{
    Block, Diagnostic, Inline, JiraDoc, ListItem, ListItemContent, Severity, Span, TableCell,
    TableRow,
};
pub use batch::{BatchParser, BatchSink, Handler, StreamingParser};
pub use emit::{build, collect_inline_text};
pub use events::{Event, OwnedEvent};
pub use parse::parse;
pub use writer::Writer;

/// Parse `input` and return a streaming iterator of [`OwnedEvent`] items.
pub fn events(input: &str) -> events::EagerEventIter {
    events::events(input)
}

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
    fn test_parse_noformat() {
        let (doc, _) = parse("{noformat}\nraw text\n{noformat}");
        assert!(matches!(doc.blocks[0], Block::Noformat { .. }));
        if let Block::Noformat { content, .. } = &doc.blocks[0] {
            assert_eq!(content, "raw text");
        }
    }

    #[test]
    fn test_parse_color_span() {
        let (doc, _) = parse("{color:red}warning{color}");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines
            .iter()
            .any(|i| matches!(i, Inline::ColorSpan { color, .. } if color == "red")));
    }

    #[test]
    fn test_parse_mention() {
        let (doc, _) = parse("Hello @alice!");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines
            .iter()
            .any(|i| matches!(i, Inline::Mention(name, _) if name == "alice")));
    }

    #[test]
    fn test_parse_panel_with_title() {
        let (doc, _) = parse("{panel:title=Note}\nContent\n{panel}");
        assert!(matches!(doc.blocks[0], Block::Panel { .. }));
        if let Block::Panel { title, .. } = &doc.blocks[0] {
            assert_eq!(title.as_deref(), Some("Note"));
        }
    }

    #[test]
    fn test_parse_nested_list() {
        let (doc, _) = parse("* Item 1\n** Sub item\n* Item 2");
        let Block::List { items, .. } = &doc.blocks[0] else {
            panic!("expected list");
        };
        assert_eq!(items.len(), 2);
        // First item should have nested content
        assert!(items[0].children.len() >= 2);
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

    #[test]
    fn test_parse_sample_no_panic() {
        // Adversarial: arbitrary inputs must not panic
        let samples = [
            "",
            "   ",
            "\n\n\n",
            "*",
            "**",
            "***",
            "{code}",
            "{code:}",
            "{panel",
            "{panel:title=}",
            "{quote}",
            "{noformat}",
            "----",
            "|| ||",
            "| |",
            "[",
            "[|",
            "!",
            "{{",
            "}}",
            "{color:red}unclosed",
            "@",
            "h1.",
            "h7. not a heading",
            "* ",
            "# ",
            "** nested without parent",
            "* item\n** sub\n*** subsub\n**** deep\n***** deeper",
            "||h1||h2||\n|missing closing",
            "{code}\nunclosed code block",
            "{panel}\nunclosed panel",
            "{quote}\nunclosed quote",
        ];
        for s in &samples {
            let _ = parse(s);
        }
    }
}
