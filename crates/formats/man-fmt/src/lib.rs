//! Man page (roff/troff) parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency by default. The optional
//! `rescribe` feature (default off) adds `crate::rescribe::{parse, emit}`, a
//! thin adapter that translates [`ManDoc`] to and from rescribe's `Document`
//! IR.
//!
//! This crate denies `unsafe` by default (production code must never use it); a prior version of `events::events` used
//! `Box::leak` to manufacture a `'static` reference to the parsed `ManDoc`,
//! leaking it on every call. See `src/events.rs` for the sound, non-leaking
//! replacement and TODO.md for the tracked architecture gap.
//!
//! # API layers
//!
//! `ManDoc` implements the five shared `rescribe-format-api` traits:
//!
//! ```text
//! use rescribe_format_api::{Emit, Events, Parse, StreamingParse, StreamingWrite};
//!
//! // AST reader
//! let (doc, diags): (ManDoc, Vec<Diagnostic>) = ManDoc::parse(input);
//!
//! // Streaming reader — owned events, collected via the AST-projection walk
//! let it = ManDoc::events(input);
//!
//! // Batch reader — chunk-driven, genuinely incremental (see `batch.rs`)
//! let mut p = ManDoc::streaming_parser(|ev| ...);
//! p.feed(chunk); // repeat
//! p.finish();
//!
//! // Builder writer — emit from AST
//! let bytes: Vec<u8> = doc.emit();
//!
//! // Streaming writer — emit from events, genuinely incremental (see `writer.rs`)
//! let mut w = ManDoc::writer(sink);
//! w.write_event(event); // repeat
//! w.finish(); // flushes to sink
//! ```
//!
//! `parse()` (`parse::parse`, `&str` input) and `man_events()` (`&str` input)
//! remain as separate, non-trait entry points, the same reasoning
//! `commonmark-fmt` documents for `parse_str`/`events_str`: man-fmt's native
//! representation is `&str`, not bytes — `ManDoc::parse`/`ManDoc::events`
//! (the trait methods, bound to `&[u8]` by `rescribe-format-api`) bridge via
//! `String::from_utf8_lossy`, matching the lossy bytes→`&str` convention this
//! crate's own `BatchParser`/`BatchSink` already use elsewhere in this file.
//! Callers that already hold a valid `&str` (e.g. `rescribe-read-man`, which
//! receives one from its own caller) use `parse()`/`man_events()` directly
//! and skip both the lossy re-decode and the allocation it would otherwise
//! force on every call.

#![deny(unsafe_code)]

pub mod ast;
pub mod batch;
pub mod emit;
pub mod events;
pub mod parse;
#[cfg(feature = "rescribe")]
pub mod rescribe;
#[cfg(test)]
mod test_alloc;
pub mod writer;

// Re-export key types for convenience.
pub use ast::{Block, Diagnostic, Inline, ManDoc, Severity, Span};
pub use batch::{BatchParser, BatchSink, Handler, StreamingParser};
#[cfg(test)]
pub(crate) use emit::build;
pub use events::{EventIter, ManEvent, OwnedManEvent};
pub use parse::parse;
pub use rescribe_format_api::{Emit, Events, Parse, StreamingParse, StreamingWrite};
pub use writer::Writer;

/// Parse `input` and return a streaming iterator of [`OwnedManEvent`] items.
///
/// Kept as a documented, non-trait entry point — see this module's doc
/// comment ("API layers") for why.
pub fn man_events(input: &str) -> impl Iterator<Item = OwnedManEvent> + '_ {
    events::events(input)
}

// ── Trait implementations ───────────────────────────────────────────────────
//
// `ManDoc` implements all five shared API-mode traits directly. Unlike
// rst-fmt, nothing here is structurally blocked: `ManDoc` is a plain owned
// type (no lifetime parameter), and `parse::parse`/`events::events` already
// return the exact `(Self, Vec<Diagnostic>)` / owned-event shapes the traits
// expect — the only bridging needed is `&[u8]` → `&str` (`from_utf8_lossy`,
// see the module doc above), not a fundamental signature mismatch.

impl Parse for ManDoc {
    fn parse(input: &[u8]) -> (Self, Vec<Diagnostic>) {
        let s = String::from_utf8_lossy(input);
        parse::parse(&s)
    }
}

impl Emit for ManDoc {
    fn emit(&self) -> Vec<u8> {
        emit::build(self).into_bytes()
    }
}

impl Events for ManDoc {
    // `events::events` eagerly collects into an owned `Vec` (see its own doc
    // comment on the architecture gap this reflects) — `Self::EventIter<'a>`
    // is therefore `std::vec::IntoIter`, a concrete type that, like
    // `zip-fmt`'s `OwnedEvent`, simply doesn't use the GAT's lifetime.
    type Event<'a> = OwnedManEvent;
    type EventIter<'a> = std::vec::IntoIter<OwnedManEvent>;

    fn events(input: &[u8]) -> Self::EventIter<'_> {
        let s = String::from_utf8_lossy(input);
        events::events(&s).collect::<Vec<_>>().into_iter()
    }
}

impl StreamingParse for ManDoc {
    type Event = OwnedManEvent;
    type Parser<H: Handler<OwnedManEvent>> = StreamingParser<H>;

    fn streaming_parser<H: Handler<OwnedManEvent>>(handler: H) -> StreamingParser<H> {
        StreamingParser::new(handler)
    }
}

impl StreamingWrite for ManDoc {
    type Writer<W: std::io::Write> = Writer<W>;

    fn writer<W: std::io::Write>(sink: W) -> Writer<W> {
        Writer::new(sink)
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(input: &str) -> ManDoc {
        let (doc, _diags) = parse(input);
        doc
    }

    #[test]
    fn test_parse_title() {
        let doc = parse_ok(".TH TEST 1 \"2024-01-01\" \"Version 1.0\"");
        assert_eq!(doc.title, Some("TEST".to_string()));
        assert_eq!(doc.section, Some("1".to_string()));
        assert_eq!(doc.date, Some("2024-01-01".to_string()));
        assert_eq!(doc.source, Some("Version 1.0".to_string()));
    }

    #[test]
    fn test_parse_sections() {
        let doc = parse_ok(".SH NAME\ntest \\- a test program\n.SH SYNOPSIS\ntest [options]");
        assert_eq!(doc.blocks.len(), 4); // 2 headings + 2 paragraphs
    }

    #[test]
    fn test_parse_bold() {
        let doc = parse_ok(".B bold text");
        let block = &doc.blocks[0];
        assert!(matches!(block, Block::Paragraph { .. }));
        if let Block::Paragraph { inlines, .. } = block {
            assert!(inlines.iter().any(|i| matches!(i, Inline::Bold(..))));
        }
    }

    #[test]
    fn test_parse_italic() {
        let doc = parse_ok(".I italic text");
        let block = &doc.blocks[0];
        assert!(matches!(block, Block::Paragraph { .. }));
        if let Block::Paragraph { inlines, .. } = block {
            assert!(inlines.iter().any(|i| matches!(i, Inline::Italic(..))));
        }
    }

    #[test]
    fn test_parse_preformatted() {
        let doc = parse_ok(".nf\ncode line 1\ncode line 2\n.fi");
        let block = &doc.blocks[0];
        assert!(matches!(block, Block::CodeBlock { .. }));
    }

    #[test]
    fn test_parse_inline_font() {
        let doc = parse_ok("This is \\fBbold\\fR text");
        let block = &doc.blocks[0];
        if let Block::Paragraph { inlines, .. } = block {
            // Should have multiple inlines
            assert!(inlines.len() >= 2);
        }
    }

    #[test]
    fn test_parse_example_block() {
        let doc = parse_ok(".EX\nexample code\n.EE");
        let block = &doc.blocks[0];
        assert!(matches!(block, Block::ExampleBlock { .. }));
        if let Block::ExampleBlock { content, .. } = block {
            assert_eq!(content, "example code");
        }
    }

    #[test]
    fn test_parse_inline_code() {
        let doc = parse_ok("Use \\f(CWcommand\\fR here");
        let block = &doc.blocks[0];
        if let Block::Paragraph { inlines, .. } = block {
            assert!(inlines.iter().any(|i| matches!(i, Inline::Code(..))));
        }
    }

    #[test]
    fn test_parse_special_chars() {
        let doc = parse_ok("em dash \\(em and en dash \\(en");
        let block = &doc.blocks[0];
        if let Block::Paragraph { inlines, .. } = block {
            let text = inlines
                .iter()
                .filter_map(|i| match i {
                    Inline::Text(s, _) => Some(s.as_str()),
                    _ => None,
                })
                .collect::<String>();
            assert!(text.contains('\u{2014}'), "expected em dash");
            assert!(text.contains('\u{2013}'), "expected en dash");
        }
    }

    #[test]
    fn test_parse_comment() {
        let doc = parse_ok(".\\\" This is a comment\n.PP\nhello");
        assert!(
            doc.blocks
                .iter()
                .any(|b| matches!(b, Block::Comment { .. }))
        );
    }

    #[test]
    fn test_parse_indented_paragraph() {
        let doc = parse_ok(".IP\nIndented text here");
        assert!(
            doc.blocks
                .iter()
                .any(|b| matches!(b, Block::IndentedParagraph { .. }))
        );
    }

    #[test]
    fn test_parse_th_metadata() {
        let doc = parse_ok(".TH MYAPP 1 \"2024-01-15\" \"MyApp 1.0\" \"User Commands\"");
        assert_eq!(doc.title, Some("MYAPP".to_string()));
        assert_eq!(doc.section, Some("1".to_string()));
        assert_eq!(doc.date, Some("2024-01-15".to_string()));
        assert_eq!(doc.source, Some("MyApp 1.0".to_string()));
        assert_eq!(doc.manual, Some("User Commands".to_string()));
    }

    #[test]
    fn test_build_basic() {
        let doc = ManDoc {
            title: Some("TEST".to_string()),
            section: Some("1".to_string()),
            date: None,
            source: None,
            manual: None,
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Hello, world!".to_string(), Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let output = build(&doc);
        assert!(output.contains(".TH"));
        assert!(output.contains(".PP"));
        assert!(output.contains("Hello, world!"));
    }

    #[test]
    fn test_build_heading() {
        let doc = ManDoc {
            title: None,
            section: None,
            date: None,
            source: None,
            manual: None,
            blocks: vec![Block::Heading {
                level: 2,
                inlines: vec![Inline::Text("Section Title".to_string(), Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let output = build(&doc);
        assert!(output.contains(".SH SECTION TITLE"));
    }

    #[test]
    fn test_build_bold() {
        let doc = ManDoc {
            title: None,
            section: None,
            date: None,
            source: None,
            manual: None,
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
        assert!(output.contains("\\fBbold\\fR"));
    }

    #[test]
    fn test_build_italic() {
        let doc = ManDoc {
            title: None,
            section: None,
            date: None,
            source: None,
            manual: None,
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
        assert!(output.contains("\\fIitalic\\fR"));
    }

    #[test]
    fn test_build_link() {
        let doc = ManDoc {
            title: None,
            section: None,
            date: None,
            source: None,
            manual: None,
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "https://example.com".to_string(),
                    children: vec![Inline::Text("Example".to_string(), Span::NONE)],
                    span: Span::NONE,
                }],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let output = build(&doc);
        assert!(output.contains("Example"));
        assert!(output.contains("https://example.com"));
    }

    #[test]
    fn test_build_inline_code() {
        let doc = ManDoc {
            title: None,
            section: None,
            date: None,
            source: None,
            manual: None,
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Code("command".to_string(), Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let output = build(&doc);
        assert!(output.contains("\\f(CWcommand\\fR"));
    }

    #[test]
    fn test_build_example_block() {
        let doc = ManDoc {
            title: None,
            section: None,
            date: None,
            source: None,
            manual: None,
            blocks: vec![Block::ExampleBlock {
                content: "example code".to_string(),
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let output = build(&doc);
        assert!(output.contains(".EX"));
        assert!(output.contains("example code"));
        assert!(output.contains(".EE"));
    }

    #[test]
    fn test_build_comment() {
        let doc = ManDoc {
            title: None,
            section: None,
            date: None,
            source: None,
            manual: None,
            blocks: vec![Block::Comment {
                text: "This is a comment".to_string(),
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let output = build(&doc);
        assert!(output.contains(".\\\" This is a comment"));
    }
}
