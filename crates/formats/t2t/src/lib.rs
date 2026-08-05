#![allow(clippy::collapsible_if, clippy::never_loop)]
//! txt2tags (t2t) parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-t2t` and `rescribe-write-t2t` as thin adapter layers.

pub mod ast;
pub mod batch;
pub mod emit;
pub mod events;
pub mod parse;
pub mod writer;

// Re-export the primary public API for convenience.
pub use ast::{Block, Diagnostic, Inline, Severity, Span, T2tDoc, TableRow};
pub use batch::{BatchParser, BatchSink, Handler, StreamingParser};
pub use emit::emit;
pub use events::{Event, EventIter, OwnedEvent};
pub use parse::{parse, parse_str};
pub use rescribe_format_api::{Emit, Events, Parse, StreamingParse, StreamingWrite};
pub use writer::Writer;

/// Parse `input` and return a streaming iterator of events.
pub fn events(input: &str) -> events::EventIter {
    events::events(input)
}

// ── Trait implementations ───────────────────────────────────────────────────
//
// `T2tDoc` implements all five shared API-mode traits. Unlike `rst-fmt`
// (whose `RstDoc<'a>` borrows from the input and whose `parse` returns a
// hard `Result<_, RstError>`), `T2tDoc` is a plain owned type and
// `parse::parse`/`parse::parse_str` are already infallible
// `(T2tDoc, Vec<Diagnostic>)` — the only mismatch versus the shared trait
// signatures is `&str` input vs. `&[u8]`, bridged the same way
// `BatchParser::finish` already does: a lossy UTF-8 decode. `parse`/
// `parse_str`/`events` (the `&str`-taking crate-root functions) stay public:
// they have a materially different contract (no lossy-decode step; callers
// that already have a `&str` skip it) and are real external-consumer entry
// points (used throughout this crate's own tests, `rescribe-read-t2t`, and
// the cross-API fixture harness), not redundant duplicates of the trait
// methods.

impl rescribe_format_api::Parse for T2tDoc {
    fn parse(input: &[u8]) -> (Self, Vec<Diagnostic>) {
        let s = String::from_utf8_lossy(input);
        parse::parse(&s)
    }
}

impl rescribe_format_api::Emit for T2tDoc {
    fn emit(&self) -> Vec<u8> {
        emit::emit(self).into_bytes()
    }
}

impl rescribe_format_api::Events for T2tDoc {
    // `EventIter` isn't lifetime-generic — it re-parses into an owned
    // `T2tDoc` and always yields `OwnedEvent` (`Event<'static>`) regardless
    // of `'a`, the same "GAT tolerates an impl that doesn't use its lifetime
    // parameter" case `zip-fmt` hits (decompression can't borrow either).
    type Event<'a> = OwnedEvent;
    type EventIter<'a> = events::EventIter;

    /// Invalid UTF-8 input yields an iterator over an empty document
    /// (`StartDocument`/`EndDocument` only) rather than panicking — the
    /// trait's `events()` is infallible by contract. Callers that need to
    /// distinguish "empty document" from "invalid UTF-8" should use
    /// [`events()`](crate::events) directly on a `&str` they've already
    /// validated.
    fn events(input: &[u8]) -> events::EventIter {
        let s = String::from_utf8_lossy(input);
        events::events(&s)
    }
}

impl rescribe_format_api::StreamingParse for T2tDoc {
    type Event = OwnedEvent;
    type Parser<H: Handler<OwnedEvent>> = StreamingParser<H>;

    fn streaming_parser<H: Handler<OwnedEvent>>(handler: H) -> StreamingParser<H> {
        StreamingParser::new(handler)
    }
}

impl rescribe_format_api::StreamingWrite for T2tDoc {
    type Writer<W: std::io::Write> = Writer<W>;

    fn writer<W: std::io::Write>(sink: W) -> Writer<W> {
        Writer::new(sink)
    }
}

// ── Legacy compatibility (removed) ────────────────────────────────────────────
//
// The old `T2tError` type and `build()` / `parse() -> Result<>` signatures have
// been replaced by infallible `parse() -> (T2tDoc, Vec<Diagnostic>)` and
// `emit(doc) -> String`. Update callers accordingly.

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use crate::ast::*;
    use crate::emit::emit;
    use crate::parse::parse_str;

    #[test]
    fn test_parse_heading() {
        let doc = parse_str("= Title =\n");
        assert_eq!(doc.blocks.len(), 1);
        match &doc.blocks[0] {
            Block::Heading {
                level, numbered, ..
            } => {
                assert_eq!(*level, 1);
                assert!(!*numbered);
            }
            _ => panic!("expected heading"),
        }
    }

    #[test]
    fn test_parse_heading_level2() {
        let doc = parse_str("== Subtitle ==\n");
        match &doc.blocks[0] {
            Block::Heading { level, .. } => {
                assert_eq!(*level, 2);
            }
            _ => panic!("expected heading"),
        }
    }

    #[test]
    fn test_parse_numbered_heading() {
        let doc = parse_str("+ Numbered +\n");
        match &doc.blocks[0] {
            Block::Heading { numbered, .. } => {
                assert!(*numbered);
            }
            _ => panic!("expected heading"),
        }
    }

    #[test]
    fn test_parse_paragraph() {
        let doc = parse_str("Hello world\n");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Paragraph { .. }));
    }

    #[test]
    fn test_parse_bold() {
        let doc = parse_str("**bold**\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(matches!(inlines[0], Inline::Bold(..)));
    }

    #[test]
    fn test_parse_italic() {
        let doc = parse_str("//italic//\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(matches!(inlines[0], Inline::Italic(..)));
    }

    #[test]
    fn test_parse_underline() {
        let doc = parse_str("__underline__\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(matches!(inlines[0], Inline::Underline(..)));
    }

    #[test]
    fn test_parse_strikethrough() {
        let doc = parse_str("--strike--\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(matches!(inlines[0], Inline::Strikethrough(..)));
    }

    #[test]
    fn test_parse_monospace() {
        let doc = parse_str("``code``\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(matches!(inlines[0], Inline::Code(..)));
    }

    #[test]
    fn test_parse_unordered_list() {
        let doc = parse_str("- item1\n- item2\n");
        assert_eq!(doc.blocks.len(), 1);
        let Block::List { ordered, items, .. } = &doc.blocks[0] else {
            panic!("expected list");
        };
        assert!(!*ordered);
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_parse_ordered_list() {
        let doc = parse_str("+ first\n+ second\n");
        let Block::List { ordered, .. } = &doc.blocks[0] else {
            panic!("expected list");
        };
        assert!(*ordered);
    }

    #[test]
    fn test_parse_verbatim_block() {
        let doc = parse_str("```\ncode here\n```\n");
        assert!(matches!(doc.blocks[0], Block::CodeBlock { .. }));
        let Block::CodeBlock { content, .. } = &doc.blocks[0] else {
            panic!("expected code block");
        };
        assert_eq!(content, "code here");
    }

    #[test]
    fn test_parse_link() {
        let doc = parse_str("[click here http://example.com]\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        let link = &inlines[0];
        assert!(matches!(link, Inline::Link { .. }));
        if let Inline::Link { url, .. } = link {
            assert_eq!(url, "http://example.com");
        }
    }

    #[test]
    fn test_parse_quote() {
        let doc = parse_str("\tquoted text\n");
        assert!(matches!(doc.blocks[0], Block::Blockquote { .. }));
    }

    #[test]
    fn test_skip_comments() {
        let doc = parse_str("% comment\ntext\n");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Paragraph { .. }));
    }

    #[test]
    fn test_build_heading() {
        let doc = T2tDoc {
            blocks: vec![Block::Heading {
                level: 1,
                numbered: false,
                inlines: vec![Inline::Text("Title".into(), Span::NONE)],
                span: Span::NONE,
            }],
            ..Default::default()
        };
        let output = emit(&doc);
        assert!(output.contains("= Title ="));
    }

    #[test]
    fn test_build_paragraph() {
        let doc = T2tDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Hello, world!".into(), Span::NONE)],
                span: Span::NONE,
            }],
            ..Default::default()
        };
        let output = emit(&doc);
        assert!(output.contains("Hello, world!"));
    }

    #[test]
    fn test_build_bold() {
        let doc = T2tDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Bold(
                    vec![Inline::Text("bold".into(), Span::NONE)],
                    Span::NONE,
                )],
                span: Span::NONE,
            }],
            ..Default::default()
        };
        let output = emit(&doc);
        assert!(output.contains("**bold**"));
    }

    #[test]
    fn test_build_italic() {
        let doc = T2tDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Italic(
                    vec![Inline::Text("italic".into(), Span::NONE)],
                    Span::NONE,
                )],
                span: Span::NONE,
            }],
            ..Default::default()
        };
        let output = emit(&doc);
        assert!(output.contains("//italic//"));
    }

    #[test]
    fn test_build_code_block() {
        let doc = T2tDoc {
            blocks: vec![Block::CodeBlock {
                content: "print hi".into(),
                span: Span::NONE,
            }],
            ..Default::default()
        };
        let output = emit(&doc);
        assert!(output.contains("```"));
        assert!(output.contains("print hi"));
    }

    #[test]
    fn test_strip_spans() {
        let doc = T2tDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("hello".into(), Span::new(0, 5))],
                span: Span::new(0, 5),
            }],
            span: Span::new(0, 5),
            ..Default::default()
        };
        let stripped = doc.strip_spans();
        assert_eq!(stripped.span, Span::NONE);
        if let Block::Paragraph { span, .. } = &stripped.blocks[0] {
            assert_eq!(*span, Span::NONE);
        }
    }
}
