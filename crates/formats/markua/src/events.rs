//! Streaming event iterator over a parsed `MarkuaDoc`.

use std::borrow::Cow;

/// A streaming event from a Markua document.
///
/// Raw text content fields use `Cow<'a, str>` so that future optimisations can
/// yield borrowed slices of the input without changing the public API.
#[derive(Debug, PartialEq)]
pub enum MarkuaEvent<'a> {
    // ── Block events ──────────────────────────────────────────────────────────
    StartParagraph,
    EndParagraph,
    StartHeading { level: u8 },
    EndHeading,
    StartBlockquote,
    EndBlockquote,
    StartList { ordered: bool },
    EndList,
    StartListItem,
    EndListItem,
    /// Leaf: a fenced code block.
    CodeBlock {
        language: Option<String>,
        content: Cow<'a, str>,
    },
    /// Leaf: a horizontal rule.
    HorizontalRule,
    /// Leaf: a page break.
    PageBreak,
    StartTable,
    EndTable,
    StartTableRow,
    EndTableRow,
    StartTableCell,
    EndTableCell,
    StartSpecialBlock { kind: String },
    EndSpecialBlock,
    StartDefinitionList,
    EndDefinitionList,
    StartDefinitionTerm,
    EndDefinitionTerm,
    StartDefinitionDesc,
    EndDefinitionDesc,
    StartFigure,
    EndFigure,
    StartCaption,
    EndCaption,

    // ── Inline events ─────────────────────────────────────────────────────────
    Text(Cow<'a, str>),
    SoftBreak,
    LineBreak,
    StartStrong,
    EndStrong,
    StartEmphasis,
    EndEmphasis,
    StartStrikethrough,
    EndStrikethrough,
    StartSubscript,
    EndSubscript,
    StartSuperscript,
    EndSuperscript,
    StartUnderline,
    EndUnderline,
    StartSmallCaps,
    EndSmallCaps,
    StartFootnoteRef,
    EndFootnoteRef,
    StartLink { url: String },
    EndLink,
    /// Leaf: inline verbatim/code span.
    InlineCode(Cow<'a, str>),
    /// Leaf: inline image.
    Image { url: String, alt: String },
    /// Leaf: index term.
    IndexTerm { term: String },
    /// Leaf: inline math.
    MathInline { content: String },
}

/// Owned event type (all text is owned).
pub type OwnedMarkuaEvent = MarkuaEvent<'static>;

impl<'a> MarkuaEvent<'a> {
    /// Convert to an owned event.
    pub fn into_owned(self) -> OwnedMarkuaEvent {
        match self {
            MarkuaEvent::Text(cow) => MarkuaEvent::Text(Cow::Owned(cow.into_owned())),
            MarkuaEvent::InlineCode(cow) => MarkuaEvent::InlineCode(Cow::Owned(cow.into_owned())),
            MarkuaEvent::CodeBlock { language, content } => MarkuaEvent::CodeBlock {
                language,
                content: Cow::Owned(content.into_owned()),
            },
            // Safety: all other variants contain only String/'static fields.
            other => unsafe { std::mem::transmute::<MarkuaEvent<'_>, OwnedMarkuaEvent>(other) },
        }
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

pub use crate::parse::EventIter;

/// Parse `input` and return a streaming iterator of [`MarkuaEvent`] items.
pub fn events(input: &str) -> EventIter<'_> {
    EventIter::new(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_events_heading() {
        let evs: Vec<_> = events("# Hello").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedMarkuaEvent::StartHeading { level: 1 })));
        assert!(evs.iter().any(|e| matches!(e, OwnedMarkuaEvent::EndHeading)));
    }

    #[test]
    fn test_events_paragraph() {
        let evs: Vec<_> = events("Hello world").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedMarkuaEvent::StartParagraph)));
        assert!(evs.iter().any(|e| matches!(e, OwnedMarkuaEvent::EndParagraph)));
        assert!(evs.iter().any(|e| matches!(e, OwnedMarkuaEvent::Text(t) if t == "Hello world")));
    }

    #[test]
    fn test_events_code_block() {
        let evs: Vec<_> = events("```rust\nfn main() {}\n```").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedMarkuaEvent::CodeBlock { language: Some(l), .. } if l == "rust")));
    }

    #[test]
    fn test_events_list() {
        let evs: Vec<_> = events("- item 1\n- item 2").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedMarkuaEvent::StartList { ordered: false })));
        assert_eq!(evs.iter().filter(|e| matches!(e, OwnedMarkuaEvent::StartListItem)).count(), 2);
        assert!(evs.iter().any(|e| matches!(e, OwnedMarkuaEvent::EndList)));
    }

    #[test]
    fn test_events_special_block() {
        let evs: Vec<_> = events("W> Be careful!").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedMarkuaEvent::StartSpecialBlock { kind } if kind == "warning")));
        assert!(evs.iter().any(|e| matches!(e, OwnedMarkuaEvent::EndSpecialBlock)));
    }

    #[test]
    fn test_events_horizontal_rule() {
        let evs: Vec<_> = events("* * *").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedMarkuaEvent::HorizontalRule)));
    }

    #[test]
    fn test_events_bold_italic() {
        let evs: Vec<_> = events("**bold** *italic*").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedMarkuaEvent::StartStrong)));
        assert!(evs.iter().any(|e| matches!(e, OwnedMarkuaEvent::EndStrong)));
        assert!(evs.iter().any(|e| matches!(e, OwnedMarkuaEvent::StartEmphasis)));
        assert!(evs.iter().any(|e| matches!(e, OwnedMarkuaEvent::EndEmphasis)));
    }

    #[test]
    fn test_events_link() {
        let evs: Vec<_> = events("[click here](https://example.com)").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedMarkuaEvent::StartLink { url } if url == "https://example.com")));
        assert!(evs.iter().any(|e| matches!(e, OwnedMarkuaEvent::EndLink)));
    }

    #[test]
    fn test_events_inline_code() {
        let evs: Vec<_> = events("Some `verbatim` text").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedMarkuaEvent::InlineCode(s) if s == "verbatim")));
    }

    #[test]
    fn test_events_page_break() {
        let evs: Vec<_> = events("{pagebreak}").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedMarkuaEvent::PageBreak)));
    }

    #[test]
    fn test_events_math_inline() {
        let evs: Vec<_> = events("Solve $x^2 + 1 = 0$.").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedMarkuaEvent::MathInline { .. })));
    }

    #[test]
    fn test_events_footnote_ref() {
        let evs: Vec<_> = events("See ^[this note].").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedMarkuaEvent::StartFootnoteRef)));
        assert!(evs.iter().any(|e| matches!(e, OwnedMarkuaEvent::EndFootnoteRef)));
    }

    #[test]
    fn test_events_subscript_superscript() {
        let evs: Vec<_> = events("H~2~O and x^2^").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedMarkuaEvent::StartSubscript)));
        assert!(evs.iter().any(|e| matches!(e, OwnedMarkuaEvent::StartSuperscript)));
    }

    #[test]
    fn test_events_index_term() {
        let evs: Vec<_> = events("See i[Markua] for details.").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedMarkuaEvent::IndexTerm { term } if term == "Markua")));
    }
}
