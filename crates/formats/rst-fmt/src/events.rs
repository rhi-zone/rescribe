//! Streaming events for RST documents.
//!
//! [`Event`] is produced by [`crate::events`] (the [`crate::EventIter`] pull
//! iterator defined in `lib.rs`, next to the `Parser` it wraps) and consumed
//! by [`crate::batch::StreamingParser`] and [`crate::writer::Writer`].

use std::borrow::Cow;

/// A streaming event from an RST document.
///
/// Raw text content fields use `Cow<'a, str>` so that a future zero-copy
/// inline tokenizer can yield borrowed slices of the input without changing
/// the public API. Today every `Cow` is `Cow::Owned` because the shared
/// inline parser (`parse_inline_content`, used by both `parse()` and
/// `events()`) already builds owned `String`s. For the common case of
/// fully-owned events (batch/streaming callback mode) use the [`OwnedEvent`]
/// alias.
#[derive(Debug, Clone, PartialEq)]
pub enum Event<'a> {
    // Block events
    StartParagraph,
    EndParagraph,
    StartHeading {
        level: i64,
    },
    EndHeading,
    StartBlockquote,
    EndBlockquote,
    StartList {
        ordered: bool,
    },
    EndList,
    StartListItem,
    EndListItem,
    StartCodeBlock {
        language: Option<String>,
    },
    EndCodeBlock,
    CodeBlockContent(Cow<'a, str>),
    RawBlock {
        format: String,
        content: String,
    },
    StartDiv {
        class: Option<String>,
        directive: Option<String>,
    },
    EndDiv,
    HorizontalRule,
    StartTable,
    EndTable,
    StartTableRow {
        is_header: bool,
    },
    EndTableRow,
    StartTableCell,
    EndTableCell,
    StartDefinitionList,
    EndDefinitionList,
    StartDefinitionTerm,
    EndDefinitionTerm,
    StartDefinitionDesc,
    EndDefinitionDesc,
    StartFootnoteDef {
        label: String,
    },
    EndFootnoteDef,
    MathDisplay {
        source: String,
    },
    StartAdmonition {
        admonition_type: String,
    },
    EndAdmonition,
    StartFigure {
        url: String,
        alt: Option<String>,
    },
    EndFigure,
    /// Image block (standalone, no caption).
    ImageBlock {
        url: String,
        alt: Option<String>,
        title: Option<String>,
    },
    // Inline events
    Text(Cow<'a, str>),
    SoftBreak,
    LineBreak,
    StartEmphasis,
    EndEmphasis,
    StartStrong,
    EndStrong,
    StartStrikeout,
    EndStrikeout,
    StartUnderline,
    EndUnderline,
    StartSubscript,
    EndSubscript,
    StartSuperscript,
    EndSuperscript,
    StartSmallCaps,
    EndSmallCaps,
    Code(Cow<'a, str>),
    StartLink {
        url: String,
    },
    EndLink,
    InlineImage {
        url: String,
        alt: String,
    },
    FootnoteRef {
        label: String,
    },
    StartFootnoteDefInline {
        label: String,
    },
    EndFootnoteDefInline,
    StartQuoted {
        quote_type: String,
    },
    EndQuoted,
    MathInline {
        source: String,
    },
    StartRstSpan {
        role: String,
    },
    EndRstSpan,
}

/// Backwards/forwards-compatible alias for fully-owned events (batch mode,
/// streaming callback mode).
pub type OwnedEvent = Event<'static>;

impl<'a> Event<'a> {
    /// Convert to an owned event (all `Cow::Borrowed` text fields become `Cow::Owned`).
    pub fn into_owned(self) -> OwnedEvent {
        match self {
            Event::Text(cow) => Event::Text(Cow::Owned(cow.into_owned())),
            Event::Code(cow) => Event::Code(Cow::Owned(cow.into_owned())),
            Event::CodeBlockContent(cow) => Event::CodeBlockContent(Cow::Owned(cow.into_owned())),
            // Safety: every other variant contains only String/'static-safe fields
            // (no borrowed data), so the lifetime-only transmute is sound.
            other => unsafe { std::mem::transmute::<Event<'_>, OwnedEvent>(other) },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_events_heading() {
        let evs: Vec<_> = crate::events("Section\n=======\n").collect();
        assert!(
            evs.iter()
                .any(|e| matches!(e, Event::StartHeading { level: 1 }))
        );
        assert!(evs.iter().any(|e| matches!(e, Event::EndHeading)));
    }

    #[test]
    fn test_events_paragraph() {
        let evs: Vec<_> = crate::events("Hello world").collect();
        assert!(evs.iter().any(|e| matches!(e, Event::StartParagraph)));
        assert!(evs.iter().any(|e| matches!(e, Event::EndParagraph)));
    }

    #[test]
    fn test_events_code_block() {
        let evs: Vec<_> = crate::events(".. code-block:: rust\n\n   let x = 1;\n").collect();
        assert!(
            evs.iter()
                .any(|e| matches!(e, Event::StartCodeBlock { language: Some(l) } if l == "rust"))
        );
        assert!(evs.iter().any(|e| matches!(e, Event::EndCodeBlock)));
    }

    #[test]
    fn test_events_list() {
        let evs: Vec<_> = crate::events("- item one\n- item two\n").collect();
        assert!(
            evs.iter()
                .any(|e| matches!(e, Event::StartList { ordered: false }))
        );
        assert!(evs.iter().any(|e| matches!(e, Event::StartListItem)));
        assert!(evs.iter().any(|e| matches!(e, Event::EndList)));
    }

    #[test]
    fn test_events_table() {
        let input = "===  ===\nA    B\n===  ===\n";
        let evs: Vec<_> = crate::events(input).collect();
        assert!(evs.iter().any(|e| matches!(e, Event::StartTable)));
        assert!(evs.iter().any(|e| matches!(e, Event::StartTableRow { .. })));
        assert!(evs.iter().any(|e| matches!(e, Event::EndTable)));
    }

    #[test]
    fn test_events_footnote() {
        let input = "See [1]_.\n\n.. [1] A footnote body.\n";
        let evs: Vec<_> = crate::events(input).collect();
        assert!(
            evs.iter()
                .any(|e| matches!(e, Event::FootnoteRef { label } if label == "1"))
        );
        assert!(
            evs.iter()
                .any(|e| matches!(e, Event::StartFootnoteDef { label } if label == "1"))
        );
    }

    #[test]
    fn test_events_nonempty_for_all_constructs() {
        let inputs = [
            "Section\n=======\n\nHello world.\n",
            "- item one\n- item two\n",
            ".. code-block:: rust\n\n   let x = 1;\n",
            ".. note::\n\n   Some note.\n",
        ];
        for input in inputs {
            let evs: Vec<_> = crate::events(input).collect();
            assert!(!evs.is_empty(), "no events for input: {input:?}");
            crate::parse(input).expect("parse failed");
        }
    }
}
