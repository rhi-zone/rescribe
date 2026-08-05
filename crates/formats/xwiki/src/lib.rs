//! XWiki 2.0 format parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-xwiki` and `rescribe-write-xwiki` as thin adapter layers.
//!
//! # API layers
//!
//! `XwikiDoc` implements four of the five shared `rescribe-format-api`
//! traits — `Parse`, `Emit`, `StreamingParse`, `StreamingWrite`. `Events` is
//! deliberately NOT implemented: this crate's `events::events` has signature
//! `fn events(doc: &XwikiDoc) -> EventIter<'_>` — it walks an
//! *already-parsed* `XwikiDoc`, not raw input, unlike the `Events` trait's
//! `fn events(input: &[u8]) -> Self::EventIter<'_>` contract (an independent,
//! from-scratch streaming parse, per CLAUDE.md's "three APIs are independent
//! implementations" principle). Implementing `Events` for `XwikiDoc` would
//! require calling `parse()` internally first — exactly the "fake
//! streaming" anti-pattern CLAUDE.md rejects ("a fake streaming API ... is a
//! broken API"). `batch::StreamingParser` doesn't hit this problem despite
//! also calling `parse()`+`events()` per completed block internally: its own
//! `feed`/`finish` contract (matching `StreamingParse`) is genuinely
//! incremental — memory is `O(largest top-level block)`, confirmed by this
//! crate's own peak-memory-bounded test — the AST-input mismatch is
//! `events()`'s problem specifically, not `StreamingParser`'s. `events::events`
//! stays a public, directly-callable function (not folded into a trait) since
//! it's the only way to get an event stream from an already-parsed `XwikiDoc`.

pub mod ast;
pub mod batch;
pub mod emit;
pub mod events;
pub mod parse;
pub mod writer;

pub use ast::{Block, Diagnostic, Inline, Severity, Span, TableCell, TableRow, XwikiDoc};
pub use batch::{BatchParser, BatchSink, Handler, StreamingParser};
pub use events::{Event, EventIter, OwnedEvent};
pub use writer::Writer;

// ── Trait implementations ───────────────────────────────────────────────────

impl rescribe_format_api::Parse for XwikiDoc {
    /// Non-UTF-8 input yields an empty document and a single `Warning`
    /// diagnostic (mirrors `commonmark-fmt`'s `Parse` impl) — this crate's
    /// native `parse::parse` takes `&str`, not `&[u8]`.
    fn parse(input: &[u8]) -> (Self, Vec<Diagnostic>) {
        match std::str::from_utf8(input) {
            Ok(s) => parse::parse(s),
            Err(_) => (
                XwikiDoc::default(),
                vec![Diagnostic {
                    span: Span::NONE,
                    severity: Severity::Warning,
                    message: "input is not valid UTF-8".to_string(),
                    code: "xwiki::invalid-utf8",
                }],
            ),
        }
    }
}

impl rescribe_format_api::Emit for XwikiDoc {
    fn emit(&self) -> Vec<u8> {
        emit::build(self).into_bytes()
    }
}

impl rescribe_format_api::StreamingParse for XwikiDoc {
    type Event = OwnedEvent;
    type Parser<H: Handler<OwnedEvent>> = StreamingParser<H>;

    fn streaming_parser<H: Handler<OwnedEvent>>(handler: H) -> StreamingParser<H> {
        StreamingParser::new(handler)
    }
}

impl rescribe_format_api::StreamingWrite for XwikiDoc {
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
    fn test_parse_heading() {
        let (result, _) = parse("= Heading 1 =\n== Heading 2 ==");
        assert_eq!(result.blocks.len(), 2);
    }

    #[test]
    fn test_parse_bold() {
        let (result, _) = parse("This is **bold** text");
        assert!(!result.blocks.is_empty());
    }

    #[test]
    fn test_parse_italic() {
        let (result, _) = parse("This is //italic// text");
        assert!(!result.blocks.is_empty());
    }

    #[test]
    fn test_parse_link() {
        let (result, _) = parse("[[Example>>http://example.com]]");
        assert!(!result.blocks.is_empty());
    }

    #[test]
    fn test_parse_list() {
        let (result, _) = parse("* Item 1\n* Item 2");
        assert_eq!(result.blocks.len(), 1);
    }

    #[test]
    fn test_parse_code_block() {
        let (result, _) = parse("{{code language=\"rust\"}}\nfn main() {}\n{{/code}}");
        assert_eq!(result.blocks.len(), 1);
    }

    #[test]
    fn test_parse_table() {
        let (result, _) = parse("|=Header|Cell|");
        assert_eq!(result.blocks.len(), 1);
    }

    #[test]
    fn test_parse_superscript() {
        let (result, _) = parse("H^^2^^O");
        let Block::Paragraph { inlines, .. } = &result.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Superscript(..))));
    }

    #[test]
    fn test_parse_subscript() {
        let (result, _) = parse("H~~2~~O");
        let Block::Paragraph { inlines, .. } = &result.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Subscript(..))));
    }

    #[test]
    fn test_parse_line_break() {
        let (result, _) = parse("line one\\\\ line two");
        let Block::Paragraph { inlines, .. } = &result.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(
            inlines
                .iter()
                .any(|i| matches!(i, Inline::LineBreak { .. }))
        );
    }

    #[test]
    fn test_parse_image() {
        let (result, _) = parse("[[image:photo.png]]");
        let Block::Paragraph { inlines, .. } = &result.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(
            inlines
                .iter()
                .any(|i| matches!(i, Inline::Image { url, .. } if url == "photo.png"))
        );
    }

    #[test]
    fn test_parse_image_with_alt() {
        let (result, _) = parse("[[image:photo.png||alt=\"A photo\"]]");
        let Block::Paragraph { inlines, .. } = &result.blocks[0] else {
            panic!("expected paragraph");
        };
        if let Some(Inline::Image { alt, .. }) = inlines.first() {
            assert_eq!(alt.as_deref(), Some("A photo"));
        } else {
            panic!("expected image");
        }
    }

    #[test]
    fn test_parse_blockquote() {
        let (result, _) = parse("{{quote}}\nSome quoted text.\n{{/quote}}");
        assert!(matches!(result.blocks[0], Block::Blockquote { .. }));
    }

    #[test]
    fn test_parse_info_macro() {
        let (result, _) = parse("{{info}}\nSome info.\n{{/info}}");
        assert!(matches!(result.blocks[0], Block::MacroBlock { .. }));
        if let Block::MacroBlock { name, content, .. } = &result.blocks[0] {
            assert_eq!(name, "info");
            assert!(content.contains("Some info."));
        }
    }

    #[test]
    fn test_parse_toc_macro() {
        let (result, _) = parse("{{toc/}}");
        assert!(matches!(result.blocks[0], Block::MacroInline { .. }));
        if let Block::MacroInline { name, .. } = &result.blocks[0] {
            assert_eq!(name, "toc");
        }
    }

    #[test]
    fn test_build_heading() {
        let doc = XwikiDoc {
            blocks: vec![Block::Heading {
                level: 1,
                inlines: vec![Inline::Text("Title".into(), Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("= Title ="));
    }

    #[test]
    fn test_build_bold() {
        let doc = XwikiDoc {
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
    fn test_build_link() {
        let doc = XwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "http://example.com".into(),
                    label: "Example".into(),
                    span: Span::NONE,
                }],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("[[Example>>http://example.com]]"));
    }

    #[test]
    fn test_roundtrip_simple() {
        let input = "= Heading =\n\nSimple paragraph with **bold** text.";
        let (doc, _) = parse(input);
        let output = build(&doc);
        let (doc2, _) = parse(&output);
        assert_eq!(doc.blocks.len(), doc2.blocks.len());
    }

    #[test]
    fn test_parse_sample_no_panic() {
        // Adversarial: should not panic on any input
        let samples = [
            "",
            "= heading =",
            "**unclosed bold",
            "{{code}}\nmissing end",
            "|= header | data",
            "* list\n* items",
            "{{info}}\nmissing close",
            "{{toc/}}",
            "[[image:test.png||alt=\"hello\"]]",
            "^^super^^ and ~~sub~~",
            "\\\\ line break",
            "{{quote}}\nquoted\n{{/quote}}",
            "{{velocity}}\n$var\n{{/velocity}}",
            "= = = = = =",
            "||||||||",
            "{{unknown_macro}}\ncontent\n{{/unknown_macro}}",
        ];
        for sample in &samples {
            let (_, _) = parse(sample);
        }
    }
}
