//! Jira wiki markup parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency by default. The optional
//! `rescribe` feature (default off) adds `crate::rescribe::{parse, emit}`,
//! a thin adapter that translates `JiraDoc` to and from rescribe's
//! `Document` IR.
//!
//! # API layers
//!
//! `JiraDoc` implements the five shared `rescribe-format-api` traits — no
//! parallel free functions exist alongside them:
//!
//! ```text
//! use rescribe_format_api::{Emit, Events, Parse, StreamingParse, StreamingWrite};
//!
//! // AST reader
//! let (doc, diags): (JiraDoc, Vec<Diagnostic>) = JiraDoc::parse(input);
//!
//! // Streaming reader — iterator over owned events, pre-computed from the AST
//! let it: EagerEventIter = JiraDoc::events(input);
//!
//! // Batch reader — chunk-driven, dispatches events as soon as provably complete
//! let mut p = JiraDoc::streaming_parser(|ev| ...);
//! p.feed(chunk); // repeat
//! p.finish();
//!
//! // Builder writer — emit from AST
//! let bytes: Vec<u8> = doc.emit();
//!
//! // Streaming writer — emit from events
//! let mut w = JiraDoc::writer(sink);
//! w.write_event(event); // repeat
//! w.finish(); // flushes to sink
//! ```
//!
//! [`parse_str`](parse::parse_str) and [`events_str`](events::events_str)
//! (take `&str`, skip the UTF-8 check the trait methods perform on `&[u8]`)
//! remain as separate, non-trait entry points — a materially different
//! contract from the trait methods, not a redundant duplicate, mirroring
//! `commonmark-fmt`'s `parse_str`/`events_str` split (see that crate's
//! module docs for the full reasoning).

pub mod ast;
pub mod batch;
pub mod emit;
pub mod events;
pub mod parse;
#[cfg(feature = "rescribe")]
pub mod rescribe;
pub mod writer;

pub use ast::{
    Block, Diagnostic, Inline, JiraDoc, ListItem, ListItemContent, Severity, Span, TableCell,
    TableRow,
};
pub use batch::{BatchParser, BatchSink, Handler, StreamingParser};
pub use emit::collect_inline_text;
pub use events::{Event, OwnedEvent, events_str};
pub use parse::parse_str;
pub use rescribe_format_api::{Emit, Events, Parse, StreamingParse, StreamingWrite};
pub use writer::Writer;

// ── Trait implementations ───────────────────────────────────────────────────
//
// `JiraDoc` implements the five shared API-mode traits directly — there are
// no parallel free functions (`jira_fmt::parse(..)`, `jira_fmt::build(..)`)
// alongside these; callers `use rescribe_format_api::Parse;` (etc.) and call
// `JiraDoc::parse(bytes)` / `doc.emit()` / `JiraDoc::events(bytes)`.

impl Parse for JiraDoc {
    fn parse(input: &[u8]) -> (Self, Vec<Diagnostic>) {
        parse::parse(input)
    }
}

impl Emit for JiraDoc {
    fn emit(&self) -> Vec<u8> {
        emit::build(self).into_bytes()
    }
}

impl Events for JiraDoc {
    // jira-fmt's `EagerEventIter` always yields `OwnedEvent` regardless of
    // the input lifetime (it pre-computes events from the parsed AST
    // rather than borrowing from the input) — the GAT tolerates an impl
    // that simply doesn't use its own lifetime parameter, per the shared
    // trait's docs (and `zip-fmt`'s identical situation).
    type Event<'a> = OwnedEvent;
    type EventIter<'a> = events::EagerEventIter;

    fn events(input: &[u8]) -> events::EagerEventIter {
        match std::str::from_utf8(input) {
            Ok(s) => events::events_str(s),
            Err(_) => events::events_str(""),
        }
    }
}

impl StreamingParse for JiraDoc {
    type Event = OwnedEvent;
    type Parser<H: Handler<OwnedEvent>> = StreamingParser<H>;

    fn streaming_parser<H: Handler<OwnedEvent>>(handler: H) -> StreamingParser<H> {
        StreamingParser::new(handler)
    }
}

impl StreamingWrite for JiraDoc {
    type Writer<W: std::io::Write> = Writer<W>;

    fn writer<W: std::io::Write>(sink: W) -> Writer<W> {
        Writer::new(sink)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_text() {
        let (doc, _) = parse_str("Hello world");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Paragraph { .. }));
    }

    #[test]
    fn test_parse_heading() {
        let (doc, _) = parse_str("h1. Title");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Heading { level: 1, .. }));
    }

    #[test]
    fn test_parse_bold() {
        let (doc, _) = parse_str("This is *bold* text.");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Bold(_, _))));
    }

    #[test]
    fn test_parse_italic() {
        let (doc, _) = parse_str("This is _italic_ text.");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Italic(_, _))));
    }

    #[test]
    fn test_parse_code() {
        let (doc, _) = parse_str("Use {{code}} here.");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Code(_, _))));
    }

    #[test]
    fn test_parse_link() {
        let (doc, _) = parse_str("Click [here|https://example.com].");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Link { .. })));
    }

    #[test]
    fn test_parse_list() {
        let (doc, _) = parse_str("* Item 1\n* Item 2");
        let block = &doc.blocks[0];
        assert!(matches!(block, Block::List { ordered: false, .. }));
    }

    #[test]
    fn test_parse_code_block() {
        let (doc, _) = parse_str("{code:java}\npublic class Test {}\n{code}");
        let code = &doc.blocks[0];
        assert!(matches!(code, Block::CodeBlock { .. }));
        if let Block::CodeBlock { language, .. } = code {
            assert_eq!(language.as_deref(), Some("java"));
        }
    }

    #[test]
    fn test_parse_noformat() {
        let (doc, _) = parse_str("{noformat}\nraw text\n{noformat}");
        assert!(matches!(doc.blocks[0], Block::Noformat { .. }));
        if let Block::Noformat { content, .. } = &doc.blocks[0] {
            assert_eq!(content, "raw text");
        }
    }

    #[test]
    fn test_parse_color_span() {
        let (doc, _) = parse_str("{color:red}warning{color}");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(
            inlines
                .iter()
                .any(|i| matches!(i, Inline::ColorSpan { color, .. } if color == "red"))
        );
    }

    #[test]
    fn test_parse_mention() {
        let (doc, _) = parse_str("Hello @alice!");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(
            inlines
                .iter()
                .any(|i| matches!(i, Inline::Mention(name, _) if name == "alice"))
        );
    }

    #[test]
    fn test_parse_panel_with_title() {
        let (doc, _) = parse_str("{panel:title=Note}\nContent\n{panel}");
        assert!(matches!(doc.blocks[0], Block::Panel { .. }));
        if let Block::Panel { title, .. } = &doc.blocks[0] {
            assert_eq!(title.as_deref(), Some("Note"));
        }
    }

    #[test]
    fn test_parse_nested_list() {
        let (doc, _) = parse_str("* Item 1\n** Sub item\n* Item 2");
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
        let output = emit::build(&doc);
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
        let output = emit::build(&doc);
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
        let output = emit::build(&doc);
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
        let output = emit::build(&doc);
        assert!(output.contains("{code:python}"));
        assert!(output.contains("print('hi')"));
        assert!(output.contains("{code}"));
    }

    #[test]
    fn test_roundtrip_heading() {
        let original = "h1. Title";
        let (doc, _) = parse_str(original);
        let rebuilt = emit::build(&doc);
        assert!(rebuilt.contains("h1. Title"));
    }

    #[test]
    fn test_roundtrip_bold() {
        let original = "This is *bold* text.";
        let (doc, _) = parse_str(original);
        let rebuilt = emit::build(&doc);
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
            let _ = parse_str(s);
        }
    }
}
