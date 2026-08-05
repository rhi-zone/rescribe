//! ZimWiki parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-zimwiki` and `rescribe-write-zimwiki` as thin adapter layers.
//!
//! # API layers
//!
//! `ZimwikiDoc` implements all five shared `rescribe-format-api` traits —
//! `Parse`, `Emit`, `Events`, `StreamingParse`, `StreamingWrite`. This
//! crate's native `parse::parse`/`events::events` take `&str`, not `&[u8]`;
//! the `Parse`/`Events` impls bridge via `std::str::from_utf8` (mirroring
//! `commonmark-fmt`'s `Parse`/`Events` impls), producing an empty
//! document/iterator plus a `Warning` diagnostic on invalid UTF-8.
//! `events::EventIter` has no lifetime parameter — unlike `opml-fmt`'s
//! zero-copy lazy pull iterator, it eagerly parses and materializes a
//! `VecDeque<OwnedEvent>` up front (see `events.rs`'s module doc) — so its
//! `Events::Event<'a>` associated type is `OwnedEvent` regardless of `'a`.
//! `parse::parse(&str)` and `events::events(&str)` remain public,
//! module-qualified entry points for callers that already have a `&str` and
//! want to skip the UTF-8 re-check — a materially different contract from
//! the trait methods, same as `commonmark-fmt`'s kept `parse_str`.

pub mod ast;
pub mod batch;
pub mod emit;
pub mod events;
pub mod parse;
pub mod writer;

pub use ast::{Block, Diagnostic, Inline, ListItem, Severity, Span, TableRow, ZimwikiDoc};
pub use batch::{BatchParser, BatchSink, Handler, StreamingParser};
pub use events::{Event, EventIter, OwnedEvent};
pub use writer::Writer;

// ── Trait implementations ───────────────────────────────────────────────────

impl rescribe_format_api::Parse for ZimwikiDoc {
    /// Non-UTF-8 input yields an empty document and a single `Warning`
    /// diagnostic (mirrors `commonmark-fmt`'s `Parse` impl) — this crate's
    /// native `parse::parse` takes `&str`, not `&[u8]`.
    fn parse(input: &[u8]) -> (Self, Vec<Diagnostic>) {
        match std::str::from_utf8(input) {
            Ok(s) => parse::parse(s),
            Err(_) => (
                ZimwikiDoc::default(),
                vec![Diagnostic {
                    span: Span::NONE,
                    severity: Severity::Warning,
                    message: "input is not valid UTF-8".to_string(),
                    code: "zimwiki::invalid-utf8",
                }],
            ),
        }
    }
}

impl rescribe_format_api::Emit for ZimwikiDoc {
    fn emit(&self) -> Vec<u8> {
        emit::build(self).into_bytes()
    }
}

impl rescribe_format_api::Events for ZimwikiDoc {
    type Event<'a> = OwnedEvent;
    type EventIter<'a> = EventIter;

    /// Invalid UTF-8 input yields an iterator over an empty document
    /// (mirrors `commonmark-fmt`'s `Events` impl).
    fn events(input: &[u8]) -> EventIter {
        match std::str::from_utf8(input) {
            Ok(s) => events::events(s),
            Err(_) => events::events(""),
        }
    }
}

impl rescribe_format_api::StreamingParse for ZimwikiDoc {
    type Event = OwnedEvent;
    type Parser<H: Handler<OwnedEvent>> = StreamingParser<H>;

    fn streaming_parser<H: Handler<OwnedEvent>>(handler: H) -> StreamingParser<H> {
        StreamingParser::new(handler)
    }
}

impl rescribe_format_api::StreamingWrite for ZimwikiDoc {
    type Writer<W: std::io::Write> = Writer<W>;

    fn writer<W: std::io::Write>(sink: W) -> Writer<W> {
        Writer::new(sink)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::emit::build;
    use crate::parse::parse;

    #[test]
    fn test_parse_heading_level1() {
        let (doc, _) = parse("====== Title ======\n");
        assert_eq!(doc.blocks.len(), 1);
        match &doc.blocks[0] {
            Block::Heading { level, .. } => assert_eq!(*level, 1),
            _ => panic!("expected heading"),
        }
    }

    #[test]
    fn test_parse_heading_level2() {
        let (doc, _) = parse("===== Subtitle =====\n");
        match &doc.blocks[0] {
            Block::Heading { level, .. } => assert_eq!(*level, 2),
            _ => panic!("expected heading"),
        }
    }

    #[test]
    fn test_parse_heading_level3() {
        let (doc, _) = parse("==== Level 3 ====\n");
        match &doc.blocks[0] {
            Block::Heading { level, .. } => assert_eq!(*level, 3),
            _ => panic!("expected heading"),
        }
    }

    #[test]
    fn test_parse_heading_level4() {
        let (doc, _) = parse("=== Level 4 ===\n");
        match &doc.blocks[0] {
            Block::Heading { level, .. } => assert_eq!(*level, 4),
            _ => panic!("expected heading"),
        }
    }

    #[test]
    fn test_parse_heading_level5() {
        let (doc, _) = parse("== Level 5 ==\n");
        match &doc.blocks[0] {
            Block::Heading { level, .. } => assert_eq!(*level, 5),
            _ => panic!("expected heading"),
        }
    }

    #[test]
    fn test_parse_paragraph() {
        let (doc, _) = parse("Hello world\n");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Paragraph { .. }));
    }

    #[test]
    fn test_parse_bold() {
        let (doc, _) = parse("**bold**\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Bold(_, _))));
    }

    #[test]
    fn test_parse_italic() {
        let (doc, _) = parse("//italic//\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Italic(_, _))));
    }

    #[test]
    fn test_parse_underline() {
        let (doc, _) = parse("__underlined__\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(
            inlines.iter().any(|i| matches!(i, Inline::Underline(_, _))),
            "expected underline, got: {inlines:?}"
        );
    }

    #[test]
    fn test_parse_strikethrough() {
        let (doc, _) = parse("~~strike~~\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(
            inlines
                .iter()
                .any(|i| matches!(i, Inline::Strikethrough(_, _)))
        );
    }

    #[test]
    fn test_parse_code() {
        let (doc, _) = parse("''code''\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Code(_, _))));
    }

    #[test]
    fn test_parse_link() {
        let (doc, _) = parse("[[MyPage]]\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        let link = inlines.iter().find(|i| matches!(i, Inline::Link { .. }));
        assert!(link.is_some());
    }

    #[test]
    fn test_parse_link_with_label() {
        let (doc, _) = parse("[[MyPage|click here]]\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        if let Some(Inline::Link { url, .. }) =
            inlines.iter().find(|i| matches!(i, Inline::Link { .. }))
        {
            assert_eq!(url, "MyPage");
        } else {
            panic!("expected link");
        }
    }

    #[test]
    fn test_parse_unordered_list() {
        let (doc, _) = parse("* item1\n* item2\n");
        let Block::List { items, .. } = &doc.blocks[0] else {
            panic!("expected list");
        };
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_parse_checkbox_list() {
        let (doc, _) = parse("[ ] unchecked\n[*] checked\n");
        let Block::List { items, .. } = &doc.blocks[0] else {
            panic!("expected list");
        };
        assert_eq!(items[0].checked, Some(false));
        assert_eq!(items[1].checked, Some(true));
    }

    #[test]
    fn test_parse_verbatim() {
        let (doc, _) = parse("'''\ncode here\n'''\n");
        assert!(matches!(doc.blocks[0], Block::CodeBlock { .. }));
    }

    #[test]
    fn test_parse_blockquote() {
        let (doc, _) = parse("> quoted text\n");
        assert!(matches!(doc.blocks[0], Block::Blockquote { .. }));
        if let Block::Blockquote { children, .. } = &doc.blocks[0] {
            assert!(!children.is_empty());
        }
    }

    #[test]
    fn test_build_heading_level1() {
        let doc = ZimwikiDoc {
            blocks: vec![Block::Heading {
                level: 1,
                inlines: vec![Inline::Text("Title".into(), Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("====== Title ======"));
    }

    #[test]
    fn test_build_heading_level2() {
        let doc = ZimwikiDoc {
            blocks: vec![Block::Heading {
                level: 2,
                inlines: vec![Inline::Text("Subtitle".into(), Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("===== Subtitle ====="));
    }

    #[test]
    fn test_build_paragraph() {
        let doc = ZimwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Hello, world!".into(), Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("Hello, world!"));
    }

    #[test]
    fn test_build_bold() {
        let doc = ZimwikiDoc {
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
        assert!(out.contains("**bold**"));
    }

    #[test]
    fn test_build_italic() {
        let doc = ZimwikiDoc {
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
        assert!(out.contains("//italic//"));
    }

    #[test]
    fn test_build_strikethrough() {
        let doc = ZimwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Strikethrough(
                    vec![Inline::Text("deleted".into(), Span::NONE)],
                    Span::NONE,
                )],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("~~deleted~~"));
    }

    #[test]
    fn test_build_code() {
        let doc = ZimwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Code("code".into(), Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("''code''"));
    }

    #[test]
    fn test_build_link() {
        let doc = ZimwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "MyPage".into(),
                    children: vec![Inline::Text("click".into(), Span::NONE)],
                    span: Span::NONE,
                }],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("[[MyPage|click]]"));
    }

    #[test]
    fn test_build_unordered_list() {
        let doc = ZimwikiDoc {
            blocks: vec![Block::List {
                ordered: false,
                items: vec![
                    ListItem {
                        checked: None,
                        children: vec![Block::Paragraph {
                            inlines: vec![Inline::Text("one".into(), Span::NONE)],
                            span: Span::NONE,
                        }],
                        span: Span::NONE,
                    },
                    ListItem {
                        checked: None,
                        children: vec![Block::Paragraph {
                            inlines: vec![Inline::Text("two".into(), Span::NONE)],
                            span: Span::NONE,
                        }],
                        span: Span::NONE,
                    },
                ],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("* one"));
        assert!(out.contains("* two"));
    }

    #[test]
    fn test_build_code_block() {
        let doc = ZimwikiDoc {
            blocks: vec![Block::CodeBlock {
                content: "print hi".into(),
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("'''"));
        assert!(out.contains("print hi"));
    }

    #[test]
    fn test_build_horizontal_rule() {
        let doc = ZimwikiDoc {
            blocks: vec![Block::HorizontalRule { span: Span::NONE }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("----"));
    }

    #[test]
    fn test_build_image() {
        let doc = ZimwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Image {
                    url: "image.png".into(),
                    span: Span::NONE,
                }],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("{{image.png}}"));
    }

    #[test]
    fn test_build_underline() {
        let doc = ZimwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Underline(
                    vec![Inline::Text("underlined".into(), Span::NONE)],
                    Span::NONE,
                )],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("__underlined__"), "got: {out:?}");
    }
}
