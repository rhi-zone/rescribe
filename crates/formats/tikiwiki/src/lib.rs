//! TikiWiki parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-tikiwiki` and `rescribe-write-tikiwiki` as thin adapter layers.
//!
//! This crate denies `unsafe` by default (production code must never use it); a prior version of `events::EventIter::new`
//! used `unsafe { transmute }` to tie already-owned `Event<'static>` data to
//! the input lifetime `'a`. The transmute was not exploitable UB (widening
//! `'static` to `'a` is sound), but it was unnecessary lifetime laundering —
//! `emit_block`/`emit_inlines` now build `Vec<Event<'a>>` directly since
//! every event they push owns its data. See `src/events.rs`.

#![deny(unsafe_code)]

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
pub use emit::collect_inline_text;
pub use events::{Event, EventIter, OwnedEvent};
pub use parse::parse;
pub use rescribe_format_api::{Emit, Events, Parse, StreamingParse, StreamingWrite};
pub use writer::Writer;

/// Parse `input` and return a streaming iterator of [`OwnedEvent`] items.
pub fn tikiwiki_events(input: &str) -> events::EventIter<'_> {
    events::events(input)
}

// ── Trait implementations ───────────────────────────────────────────────────
//
// `TikiwikiDoc` implements all five shared API-mode traits. Unlike
// `rst-fmt`'s `RstDoc<'a>`, `TikiwikiDoc` owns its data (no lifetime
// parameter) and `parse::parse`/`events::events` already take `&str`, not a
// `RstDoc`-style borrowing type — so `Parse`/`Events` bridge cleanly via the
// same `&[u8]` → `&str` pattern `commonmark-fmt` uses (`std::str::from_utf8`,
// with an `invalid-utf8` `Diagnostic` on failure), rather than being skipped
// the way `rst-fmt` skips them.
//
// `parse::parse(&str)` and `events::events(&str)`/`tikiwiki_events` remain
// public, non-trait entry points: they have a materially different contract
// from the trait methods (`&str` input, skipping the UTF-8 check), mirroring
// `commonmark-fmt`'s kept `parse_str`/`events_str`.

impl Parse for TikiwikiDoc {
    fn parse(input: &[u8]) -> (Self, Vec<Diagnostic>) {
        match std::str::from_utf8(input) {
            Ok(s) => parse::parse(s),
            Err(_) => (
                TikiwikiDoc::default(),
                vec![
                    Diagnostic::new(Severity::Warning, "input is not valid UTF-8")
                        .with_code("tikiwiki::invalid-utf8"),
                ],
            ),
        }
    }
}

impl Emit for TikiwikiDoc {
    fn emit(&self) -> Vec<u8> {
        emit::build(self).into_bytes()
    }
}

impl Events for TikiwikiDoc {
    type Event<'a> = Event<'a>;
    type EventIter<'a> = EventIter<'a>;

    /// Invalid UTF-8 input yields an iterator over an empty document rather
    /// than panicking or returning `Option` — the trait's `events()` is
    /// infallible by contract and has no diagnostic channel (unlike
    /// `Parse::parse`). Callers that need to *distinguish* "empty document"
    /// from "invalid UTF-8" should use [`tikiwiki_events`]/[`events::events`]
    /// directly on an already-validated `&str`.
    fn events(input: &[u8]) -> EventIter<'_> {
        match std::str::from_utf8(input) {
            Ok(s) => events::EventIter::new(s),
            Err(_) => events::EventIter::new(""),
        }
    }
}

impl StreamingParse for TikiwikiDoc {
    type Event = OwnedEvent;
    type Parser<H: Handler<OwnedEvent>> = StreamingParser<H>;

    fn streaming_parser<H: Handler<OwnedEvent>>(handler: H) -> StreamingParser<H> {
        StreamingParser::new(handler)
    }
}

impl StreamingWrite for TikiwikiDoc {
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
        let out = emit::build(&doc);
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
        let out = emit::build(&doc);
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
        let out = emit::build(&doc);
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
        let out = emit::build(&doc);
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
        let out = emit::build(&doc);
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
        let output = emit::build(&doc);
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
