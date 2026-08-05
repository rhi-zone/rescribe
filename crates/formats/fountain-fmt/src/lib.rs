//! Fountain screenplay format parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-fountain` and `rescribe-write-fountain` as thin adapter layers.

pub mod ast;
pub mod batch;
pub mod emit;
pub mod events;
pub mod parse;
pub mod writer;

// Re-export the most-used types for convenience.
pub use ast::{Block, Diagnostic, FountainDoc, Severity, Span};
pub use batch::{BatchParser, BatchSink, Handler, StreamingParser};
pub use events::{Event, OwnedEvent};
pub use rescribe_format_api::{Emit, Events, Parse, StreamingParse, StreamingWrite};
pub use writer::Writer;

/// Parse a Fountain string into a [`FountainDoc`].
///
/// Parsing is infallible — all input is accepted.  Diagnostics are returned
/// alongside the document for any construct that could not be interpreted.
pub fn parse(input: &str) -> (FountainDoc, Vec<Diagnostic>) {
    parse::parse(input)
}

/// Build a Fountain string from a [`FountainDoc`].
pub fn build(doc: &FountainDoc) -> String {
    emit::emit(doc)
}

/// Parse `input` and return a streaming iterator of [`OwnedEvent`] items.
pub fn events(input: &str) -> events::OwnedEventIter {
    events::events(input)
}

// ── rescribe-format-api trait implementations ───────────────────────────────
//
// All five traits fit `FountainDoc` cleanly: it's not lifetime-generic (owned
// `String` fields throughout), `parse()` already returns the
// `(Self, Vec<Diagnostic>)` shape `Parse` wants, and both `OwnedEventIter`
// (the crate's actual public `events()` return type) and `Event`/`OwnedEvent`
// never borrow from the input they were built from — `events()` parses to an
// owned `FountainDoc` first and wraps it, so nothing escapes a locally-owned
// `Cow::Owned` buffer. `Parse`/`Events` bridge `&[u8]` input via
// `String::from_utf8_lossy`, matching what `batch::BatchParser::finish` and
// `batch::StreamingParser::emit_block` already do internally.

impl Parse for FountainDoc {
    fn parse(input: &[u8]) -> (Self, Vec<Diagnostic>) {
        let s = String::from_utf8_lossy(input);
        parse::parse(&s)
    }
}

impl Emit for FountainDoc {
    fn emit(&self) -> Vec<u8> {
        build(self).into_bytes()
    }
}

impl Events for FountainDoc {
    type Event<'a> = OwnedEvent;
    type EventIter<'a> = events::OwnedEventIter;

    fn events(input: &[u8]) -> Self::EventIter<'_> {
        let s = String::from_utf8_lossy(input);
        events::events(&s)
    }
}

impl StreamingParse for FountainDoc {
    type Event = OwnedEvent;
    type Parser<H: Handler<OwnedEvent>> = StreamingParser<H>;

    fn streaming_parser<H: Handler<OwnedEvent>>(handler: H) -> StreamingParser<H> {
        StreamingParser::new(handler)
    }
}

impl StreamingWrite for FountainDoc {
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
    fn test_parse_title_page() {
        let input = "Title: My Screenplay\nAuthor: John Doe\n\nINT. HOUSE - DAY";
        let (doc, _diags) = parse(input);
        assert_eq!(
            doc.metadata.get("title").map(|s| s.as_str()),
            Some("My Screenplay")
        );
        assert_eq!(
            doc.metadata.get("author").map(|s| s.as_str()),
            Some("John Doe")
        );
    }

    #[test]
    fn test_parse_scene_heading() {
        let input = "INT. COFFEE SHOP - DAY";
        let (doc, _diags) = parse(input);
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::SceneHeading { .. }));
    }

    #[test]
    fn test_parse_dialogue() {
        let input = "JOHN\nHello, how are you?";
        let (doc, _diags) = parse(input);
        assert!(!doc.blocks.is_empty());
        assert!(matches!(doc.blocks[0], Block::Character { .. }));
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_parse_action() {
        let input = "The door slowly opens. A figure emerges from the shadows.";
        let (doc, _diags) = parse(input);
        assert!(!doc.blocks.is_empty());
        assert!(matches!(doc.blocks[0], Block::Action { .. }));
    }

    #[test]
    fn test_parse_transition() {
        let input = "CUT TO:";
        let (doc, _diags) = parse(input);
        assert!(!doc.blocks.is_empty());
        assert!(matches!(doc.blocks[0], Block::Transition { .. }));
    }

    #[test]
    fn test_build_simple() {
        let mut doc = FountainDoc::default();
        doc.blocks.push(Block::SceneHeading {
            text: "INT. OFFICE - DAY".to_string(),
            span: Span::NONE,
        });
        doc.blocks.push(Block::Action {
            text: "John enters.".to_string(),
            span: Span::NONE,
        });
        let output = build(&doc);
        assert!(output.contains("INT. OFFICE - DAY"));
        assert!(output.contains("John enters."));
    }

    #[test]
    fn test_build_with_metadata() {
        use std::collections::BTreeMap;
        let mut metadata = BTreeMap::new();
        metadata.insert("title".to_string(), "My Script".to_string());
        metadata.insert("author".to_string(), "Jane Doe".to_string());

        let doc = FountainDoc {
            metadata,
            blocks: vec![Block::Action {
                text: "Fade in.".to_string(),
                span: Span::NONE,
            }],
            span: Span::NONE,
        };

        let output = build(&doc);
        assert!(output.contains("Title: My Script"));
        assert!(output.contains("Author: Jane Doe"));
    }

    #[test]
    fn test_parse_section() {
        let input = "# ACT ONE\n\nINT. HOUSE - DAY";
        let (doc, _diags) = parse(input);
        assert!(
            doc.blocks
                .iter()
                .any(|b| matches!(b, Block::Section { .. }))
        );
    }

    #[test]
    fn test_parse_note() {
        let input = "This is action [[with a note]]";
        let (doc, _diags) = parse(input);
        assert!(doc.blocks.iter().any(|b| matches!(b, Block::Note { .. })));
    }

    #[test]
    fn test_parse_centered() {
        let input = ">CENTERED TEXT<";
        let (doc, _diags) = parse(input);
        assert!(
            doc.blocks
                .iter()
                .any(|b| matches!(b, Block::Centered { .. }))
        );
    }

    #[test]
    fn test_parse_lyric() {
        let input = "~And the music plays on...";
        let (doc, _diags) = parse(input);
        assert!(doc.blocks.iter().any(|b| matches!(b, Block::Lyric { .. })));
    }

    #[test]
    fn test_parse_page_break() {
        let input = "Action\n\n===\n\nMore action";
        let (doc, _diags) = parse(input);
        assert!(
            doc.blocks
                .iter()
                .any(|b| matches!(b, Block::PageBreak { .. }))
        );
    }

    #[test]
    fn test_build_transition() {
        let mut doc = FountainDoc::default();
        doc.blocks.push(Block::Transition {
            text: "CUT TO:".to_string(),
            span: Span::NONE,
        });
        let output = build(&doc);
        assert!(output.contains("CUT TO:"));
    }

    #[test]
    fn test_build_character_dual() {
        let mut doc = FountainDoc::default();
        doc.blocks.push(Block::Character {
            name: "JOHN".to_string(),
            dual: true,
            span: Span::NONE,
        });
        let output = build(&doc);
        assert!(output.contains("JOHN ^"));
    }

    #[test]
    fn test_parse_boneyard() {
        let input = "/* This is a boneyard comment */";
        let (doc, _diags) = parse(input);
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Boneyard { .. }));
        if let Block::Boneyard { ref text, .. } = doc.blocks[0] {
            assert_eq!(text, "This is a boneyard comment");
        }
    }

    #[test]
    fn test_parse_boneyard_multiline() {
        let input = "/* Line one\nLine two\nLine three */";
        let (doc, _diags) = parse(input);
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Boneyard { .. }));
    }

    #[test]
    fn test_parse_forced_action() {
        let input = "!INT. OFFICE - DAY";
        let (doc, _diags) = parse(input);
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Action { .. }));
        if let Block::Action { ref text, .. } = doc.blocks[0] {
            assert_eq!(text, "INT. OFFICE - DAY");
        }
    }

    #[test]
    fn test_parse_forced_character() {
        let input = "@McCLANE\nYippee ki-yay.";
        let (doc, _diags) = parse(input);
        assert!(
            doc.blocks
                .iter()
                .any(|b| matches!(b, Block::Character { .. }))
        );
        if let Block::Character { ref name, .. } = doc.blocks[0] {
            assert_eq!(name, "McCLANE");
        }
    }

    #[test]
    fn test_parse_dual_dialogue() {
        let input = "JOHN\nHello!\n\nMARY ^\nHi!";
        let (doc, _diags) = parse(input);
        let dual_chars: Vec<_> = doc
            .blocks
            .iter()
            .filter(|b| matches!(b, Block::Character { dual: true, .. }))
            .collect();
        assert_eq!(dual_chars.len(), 1);
    }

    #[test]
    fn test_strip_spans() {
        let input = "INT. OFFICE - DAY\n\nJohn enters.";
        let (doc, _) = parse(input);
        let stripped = doc.strip_spans();
        for block in &stripped.blocks {
            match block {
                Block::SceneHeading { span, .. } | Block::Action { span, .. } => {
                    assert_eq!(*span, Span::NONE);
                }
                _ => {}
            }
        }
    }
}
