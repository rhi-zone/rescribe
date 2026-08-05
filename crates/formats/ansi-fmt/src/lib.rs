//! ANSI terminal escape sequence parser, emitter, and AST.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-ansi` and `rescribe-write-ansi` as thin adapter layers.
//!
//! # API
//!
//! ## AST (direct parse)
//!
//! ```rust
//! use ansi_fmt::AnsiDoc;
//! use rescribe_format_api::{Emit, Parse};
//!
//! let (doc, diagnostics) = AnsiDoc::parse(b"\x1b[1mHello\x1b[0m world");
//! let output = doc.emit();
//! ```
//!
//! ## Streaming (pull iterator)
//!
//! ```rust
//! use ansi_fmt::AnsiDoc;
//! use rescribe_format_api::Events;
//!
//! for event in AnsiDoc::events(b"\x1b[1mHello\x1b[0m") {
//!     // process event
//! }
//! ```
//!
//! ## Batch (chunk-driven)
//!
//! ```rust
//! use ansi_fmt::batch::{StreamingParser, Handler};
//! use ansi_fmt::OwnedEvent;
//!
//! let mut evs = Vec::new();
//! let mut p = StreamingParser::new(|ev: OwnedEvent| evs.push(ev));
//! p.feed(b"\x1b[1m");
//! p.feed(b"Hello\x1b[0m");
//! p.finish();
//! ```
//!
//! ## Writer (event-driven output)
//!
//! ```rust
//! use ansi_fmt::writer::Writer;
//! use ansi_fmt::OwnedEvent;
//! use ansi_fmt::ast::Style;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! let mut s = Style::default();
//! s.bold = true;
//! w.write_event(OwnedEvent::Text { text: "Hello".to_string().into(), style: s });
//! w.write_event(OwnedEvent::ResetStyle);
//! let bytes = w.finish();
//! ```

pub mod ast;
pub mod batch;
pub mod emit;
pub mod events;
pub mod parse;
pub mod writer;

// Re-export the most-used types at the crate root.
pub use ast::{
    AnsiDoc, AnsiNode, Color, CursorDirection, Diagnostic, EraseMode, Severity, Span, Style,
};
pub use batch::{BatchParser, BatchSink, Handler, StreamingParser};
pub use emit::collect_text;
pub use events::{Event, EventIter, OwnedEvent};
pub use parse::{parse_str, strip_ansi};
pub use rescribe_format_api::{Emit, Events, Parse, StreamingParse, StreamingWrite};
pub use writer::Writer;

// ── Trait implementations ───────────────────────────────────────────────────
//
// `AnsiDoc`'s original `parse`/`emit`/`events` free functions all already
// took `&[u8]`/produced the exact shapes the shared traits require, so
// there is no materially-different-contract case to preserve for them (per
// commonmark-fmt's/rst-fmt's documented exception) — they're demoted to
// `pub(crate)` and reached only through the trait impls below. `parse_str`
// (`&str`-input convenience) and `strip_ansi` (a distinct utility, not an
// alias) keep their own public, non-trait entry points.

impl Parse for AnsiDoc {
    fn parse(input: &[u8]) -> (Self, Vec<Diagnostic>) {
        parse::parse(input)
    }
}

impl Emit for AnsiDoc {
    fn emit(&self) -> Vec<u8> {
        emit::emit(self).into_bytes()
    }
}

impl Events for AnsiDoc {
    type Event<'a> = Event<'a>;
    type EventIter<'a> = EventIter<'a>;

    fn events(input: &[u8]) -> EventIter<'_> {
        events::events(input)
    }
}

impl StreamingParse for AnsiDoc {
    type Event = OwnedEvent;
    type Parser<H: Handler<OwnedEvent>> = StreamingParser<H>;

    fn streaming_parser<H: Handler<OwnedEvent>>(handler: H) -> StreamingParser<H> {
        StreamingParser::new(handler)
    }
}

impl StreamingWrite for AnsiDoc {
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
    fn test_parse_plain_text() {
        let (doc, _) = AnsiDoc::parse(b"Hello world");
        assert!(!doc.nodes.is_empty());
    }

    #[test]
    fn test_parse_bold() {
        let (doc, _) = AnsiDoc::parse(b"\x1b[1mBold text\x1b[0m");
        let texts: Vec<_> = doc
            .nodes
            .iter()
            .filter_map(|n| match n {
                AnsiNode::Text { text, style, .. } => Some((text.as_str(), style.clone())),
                _ => None,
            })
            .collect();
        assert!(texts.iter().any(|(t, s)| *t == "Bold text" && s.bold));
    }

    #[test]
    fn test_parse_italic() {
        let (doc, _) = AnsiDoc::parse(b"\x1b[3mItalic text\x1b[0m");
        let texts: Vec<_> = doc
            .nodes
            .iter()
            .filter_map(|n| match n {
                AnsiNode::Text { text, style, .. } => Some((text.as_str(), style.clone())),
                _ => None,
            })
            .collect();
        assert!(texts.iter().any(|(t, s)| *t == "Italic text" && s.italic));
    }

    #[test]
    fn test_parse_underline() {
        let (doc, _) = AnsiDoc::parse(b"\x1b[4mUnderlined\x1b[0m");
        let texts: Vec<_> = doc
            .nodes
            .iter()
            .filter_map(|n| match n {
                AnsiNode::Text { text, style, .. } => Some((text.as_str(), style.clone())),
                _ => None,
            })
            .collect();
        assert!(texts.iter().any(|(t, s)| *t == "Underlined" && s.underline));
    }

    #[test]
    fn test_strip_ansi() {
        assert_eq!(strip_ansi("\x1b[1mBold\x1b[0m"), "Bold");
        assert_eq!(strip_ansi("\x1b[31mRed\x1b[0m"), "Red");
        assert_eq!(strip_ansi("Plain text"), "Plain text");
    }

    #[test]
    fn test_combined_styles() {
        let (doc, _) = AnsiDoc::parse(b"\x1b[1;3mBold and italic\x1b[0m");
        let texts: Vec<_> = doc
            .nodes
            .iter()
            .filter_map(|n| match n {
                AnsiNode::Text { text, style, .. } => Some((text.as_str(), style.clone())),
                _ => None,
            })
            .collect();
        assert!(
            texts
                .iter()
                .any(|(t, s)| *t == "Bold and italic" && s.bold && s.italic)
        );
    }

    #[test]
    fn test_emit_simple() {
        let doc = AnsiDoc {
            nodes: vec![AnsiNode::Text {
                text: "Hello world".into(),
                style: Style::default(),
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let output = doc.emit();
        assert!(String::from_utf8(output).unwrap().contains("Hello world"));
    }

    #[test]
    fn test_roundtrip_plain() {
        let input = b"Hello world";
        let (doc, _) = AnsiDoc::parse(input);
        let emitted = doc.emit();
        let (doc2, _) = AnsiDoc::parse(&emitted);
        assert_eq!(collect_text(&doc), collect_text(&doc2));
    }

    #[test]
    fn test_roundtrip_bold() {
        let input = b"\x1b[1mHello\x1b[0m";
        let (doc, _) = AnsiDoc::parse(input);
        let emitted = doc.emit();
        let (doc2, _) = AnsiDoc::parse(&emitted);
        assert_eq!(collect_text(&doc), collect_text(&doc2));
    }

    #[test]
    fn test_roundtrip_colors() {
        let input =
            b"\x1b[31mRed\x1b[0m \x1b[38;5;196mPalette\x1b[0m \x1b[38;2;255;128;0mTruecolor\x1b[0m";
        let (doc, _) = AnsiDoc::parse(input);
        let emitted = doc.emit();
        let (doc2, _) = AnsiDoc::parse(&emitted);
        assert_eq!(collect_text(&doc), collect_text(&doc2));
    }

    #[test]
    fn test_parse_no_panic_sample() {
        // Common SGR sequences — must not panic.
        let sample = b"\x1b[1mbold\x1b[0m \x1b[3mitalic\x1b[0m \x1b[4munderline\x1b[0m \
            \x1b[31mred\x1b[0m \x1b[38;5;196m256-color\x1b[0m \
            \x1b[38;2;255;128;0mtruecolor\x1b[0m \x1b[2mdim\x1b[0m \
            \x1b[5mblink\x1b[0m \x1b[7mreverse\x1b[0m \x1b[9mstrike\x1b[0m";
        let (doc, _) = AnsiDoc::parse(sample);
        assert!(!doc.nodes.is_empty());
    }

    #[test]
    fn test_parse_no_panic_adversarial() {
        // Truncated, malformed, bare ESC — must not panic.
        let inputs: &[&[u8]] = &[
            b"",
            b"\x1b",
            b"\x1b[",
            b"\x1b[1",
            b"\x1b[999999999m",
            b"\x1b[38;5m",
            b"\x1b[38;2m",
            b"\x1b]",
            b"\x1b]8;;\x07",
            b"\x1bX",
            b"\x1b\x1b\x1b",
            b"\x1b[?99z",
        ];
        for input in inputs {
            let _ = AnsiDoc::parse(input); // Must not panic.
        }
    }
}
