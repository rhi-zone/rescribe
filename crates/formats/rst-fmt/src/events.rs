//! Streaming events for RST documents.
//!
//! [`Event`] is produced by [`crate::events`] (the [`crate::EventIter`] pull
//! iterator defined in `lib.rs`, next to the `Parser` it wraps) and consumed
//! by [`crate::batch::StreamingParser`] and [`crate::writer::Writer`].

use std::borrow::Cow;

/// A streaming event from an RST document.
///
/// Every text field is a `Cow<'a, str>` borrowed from the parsed input
/// wherever the span is a contiguous slice of it that needs no transformation
/// — which is the common case. The payload is owned only where the format
/// forces it: text runs containing backslash escapes (which must be resolved),
/// content joined from non-contiguous source lines (a multi-line paragraph,
/// list item, block quote or definition body), and synthesised `:ref:`/`:doc:`
/// URLs. For events that must outlive the input, use [`Event::into_owned`] /
/// the [`OwnedEvent`] alias.
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
        language: Option<Cow<'a, str>>,
    },
    EndCodeBlock,
    CodeBlockContent(Cow<'a, str>),
    RawBlock {
        format: Cow<'a, str>,
        content: Cow<'a, str>,
    },
    StartDiv {
        class: Option<Cow<'a, str>>,
        directive: Option<Cow<'a, str>>,
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
        label: Cow<'a, str>,
    },
    EndFootnoteDef,
    MathDisplay {
        source: Cow<'a, str>,
    },
    StartAdmonition {
        admonition_type: Cow<'a, str>,
    },
    EndAdmonition,
    StartFigure {
        url: Cow<'a, str>,
        alt: Option<Cow<'a, str>>,
    },
    EndFigure,
    /// Image block (standalone, no caption).
    ImageBlock {
        url: Cow<'a, str>,
        alt: Option<Cow<'a, str>>,
        title: Option<Cow<'a, str>>,
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
        url: Cow<'a, str>,
    },
    EndLink,
    InlineImage {
        url: Cow<'a, str>,
        alt: Cow<'a, str>,
    },
    FootnoteRef {
        label: Cow<'a, str>,
    },
    StartFootnoteDefInline {
        label: Cow<'a, str>,
    },
    EndFootnoteDefInline,
    StartQuoted {
        quote_type: Cow<'a, str>,
    },
    EndQuoted,
    MathInline {
        source: Cow<'a, str>,
    },
    StartRstSpan {
        role: Cow<'a, str>,
    },
    EndRstSpan,
}

/// Backwards/forwards-compatible alias for fully-owned events (batch mode,
/// streaming callback mode).
pub type OwnedEvent = Event<'static>;

fn own(c: Cow<'_, str>) -> Cow<'static, str> {
    Cow::Owned(c.into_owned())
}

fn own_opt(c: Option<Cow<'_, str>>) -> Option<Cow<'static, str>> {
    c.map(own)
}

impl Event<'_> {
    /// Convert to an owned event (all `Cow::Borrowed` text fields become `Cow::Owned`).
    pub fn into_owned(self) -> OwnedEvent {
        match self {
            Event::StartParagraph => Event::StartParagraph,
            Event::EndParagraph => Event::EndParagraph,
            Event::StartHeading { level } => Event::StartHeading { level },
            Event::EndHeading => Event::EndHeading,
            Event::StartBlockquote => Event::StartBlockquote,
            Event::EndBlockquote => Event::EndBlockquote,
            Event::StartList { ordered } => Event::StartList { ordered },
            Event::EndList => Event::EndList,
            Event::StartListItem => Event::StartListItem,
            Event::EndListItem => Event::EndListItem,
            Event::StartCodeBlock { language } => Event::StartCodeBlock {
                language: own_opt(language),
            },
            Event::EndCodeBlock => Event::EndCodeBlock,
            Event::CodeBlockContent(c) => Event::CodeBlockContent(own(c)),
            Event::RawBlock { format, content } => Event::RawBlock {
                format: own(format),
                content: own(content),
            },
            Event::StartDiv { class, directive } => Event::StartDiv {
                class: own_opt(class),
                directive: own_opt(directive),
            },
            Event::EndDiv => Event::EndDiv,
            Event::HorizontalRule => Event::HorizontalRule,
            Event::StartTable => Event::StartTable,
            Event::EndTable => Event::EndTable,
            Event::StartTableRow { is_header } => Event::StartTableRow { is_header },
            Event::EndTableRow => Event::EndTableRow,
            Event::StartTableCell => Event::StartTableCell,
            Event::EndTableCell => Event::EndTableCell,
            Event::StartDefinitionList => Event::StartDefinitionList,
            Event::EndDefinitionList => Event::EndDefinitionList,
            Event::StartDefinitionTerm => Event::StartDefinitionTerm,
            Event::EndDefinitionTerm => Event::EndDefinitionTerm,
            Event::StartDefinitionDesc => Event::StartDefinitionDesc,
            Event::EndDefinitionDesc => Event::EndDefinitionDesc,
            Event::StartFootnoteDef { label } => Event::StartFootnoteDef { label: own(label) },
            Event::EndFootnoteDef => Event::EndFootnoteDef,
            Event::MathDisplay { source } => Event::MathDisplay {
                source: own(source),
            },
            Event::StartAdmonition { admonition_type } => Event::StartAdmonition {
                admonition_type: own(admonition_type),
            },
            Event::EndAdmonition => Event::EndAdmonition,
            Event::StartFigure { url, alt } => Event::StartFigure {
                url: own(url),
                alt: own_opt(alt),
            },
            Event::EndFigure => Event::EndFigure,
            Event::ImageBlock { url, alt, title } => Event::ImageBlock {
                url: own(url),
                alt: own_opt(alt),
                title: own_opt(title),
            },
            Event::Text(c) => Event::Text(own(c)),
            Event::SoftBreak => Event::SoftBreak,
            Event::LineBreak => Event::LineBreak,
            Event::StartEmphasis => Event::StartEmphasis,
            Event::EndEmphasis => Event::EndEmphasis,
            Event::StartStrong => Event::StartStrong,
            Event::EndStrong => Event::EndStrong,
            Event::StartStrikeout => Event::StartStrikeout,
            Event::EndStrikeout => Event::EndStrikeout,
            Event::StartUnderline => Event::StartUnderline,
            Event::EndUnderline => Event::EndUnderline,
            Event::StartSubscript => Event::StartSubscript,
            Event::EndSubscript => Event::EndSubscript,
            Event::StartSuperscript => Event::StartSuperscript,
            Event::EndSuperscript => Event::EndSuperscript,
            Event::StartSmallCaps => Event::StartSmallCaps,
            Event::EndSmallCaps => Event::EndSmallCaps,
            Event::Code(c) => Event::Code(own(c)),
            Event::StartLink { url } => Event::StartLink { url: own(url) },
            Event::EndLink => Event::EndLink,
            Event::InlineImage { url, alt } => Event::InlineImage {
                url: own(url),
                alt: own(alt),
            },
            Event::FootnoteRef { label } => Event::FootnoteRef { label: own(label) },
            Event::StartFootnoteDefInline { label } => {
                Event::StartFootnoteDefInline { label: own(label) }
            }
            Event::EndFootnoteDefInline => Event::EndFootnoteDefInline,
            Event::StartQuoted { quote_type } => Event::StartQuoted {
                quote_type: own(quote_type),
            },
            Event::EndQuoted => Event::EndQuoted,
            Event::MathInline { source } => Event::MathInline {
                source: own(source),
            },
            Event::StartRstSpan { role } => Event::StartRstSpan { role: own(role) },
            Event::EndRstSpan => Event::EndRstSpan,
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

    /// Every text payload an event can carry, as a `Cow`, for borrowedness
    /// assertions. Returns `None` for variants with no text.
    fn text_payloads<'a>(ev: &'a Event<'a>) -> Vec<&'a Cow<'a, str>> {
        match ev {
            Event::Text(c) | Event::Code(c) | Event::CodeBlockContent(c) => vec![c],
            Event::StartLink { url } => vec![url],
            Event::StartFootnoteDef { label }
            | Event::FootnoteRef { label }
            | Event::StartFootnoteDefInline { label } => vec![label],
            Event::StartRstSpan { role } => vec![role],
            Event::MathInline { source } | Event::MathDisplay { source } => vec![source],
            Event::StartAdmonition { admonition_type } => vec![admonition_type],
            Event::StartQuoted { quote_type } => vec![quote_type],
            Event::InlineImage { url, alt } => vec![url, alt],
            Event::StartCodeBlock { language } => language.iter().collect(),
            Event::StartDiv { class, directive } => class.iter().chain(directive.iter()).collect(),
            Event::StartFigure { url, alt } => std::iter::once(url).chain(alt.iter()).collect(),
            Event::ImageBlock { url, alt, title } => std::iter::once(url)
                .chain(alt.iter())
                .chain(title.iter())
                .collect(),
            Event::RawBlock { format, content } => vec![format, content],
            _ => vec![],
        }
    }

    /// `events()` must actually yield `Cow::Borrowed` slices of the input for
    /// every construct whose source span is contiguous and needs no
    /// transformation. This asserts on the `Cow` *variant*, not the string
    /// value — a correctness-only comparison would pass unchanged against a
    /// fully-owned implementation, so only the variant check catches a
    /// regression back to allocating every span.
    #[test]
    fn test_events_are_zero_copy_where_the_format_allows() {
        // Deliberately every construct that should borrow: single-line
        // paragraphs, headings, inline markup, links (inline, named-target and
        // simple-reference), inline literals, roles, footnotes, code blocks,
        // line blocks, directives, tables.
        let input = "\
.. _target link: https://example.com/target

Section Title
=============

A paragraph with **strong**, *emphasis*, ``literal`` and :sup:`sup`.

A `named link <https://example.com/named>`_ and a `target link`_ here.

A :custom:`role span` and some :math:`x^2` maths.

See [1]_ for details.

.. [1] The footnote body.

.. code-block:: rust

   let x = 1;

| a line block line
| another line

===  ===
A    B
===  ===

.. note::

   A note body.
";
        let mut borrowed = 0usize;
        let mut owned: Vec<String> = Vec::new();
        for ev in crate::events(input) {
            for cow in text_payloads(&ev) {
                match cow {
                    Cow::Borrowed(_) => borrowed += 1,
                    Cow::Owned(s) => owned.push(s.clone()),
                }
            }
        }
        assert!(borrowed > 0, "no borrowed payloads at all");
        // The one owned payload in this corpus is the `code-block` body: the
        // directive content collector keeps the trailing blank line inside the
        // body, so the content is joined from two collected lines and is not a
        // contiguous slice of the input. Listing it explicitly (rather than
        // asserting a ratio) keeps any *new* owned payload a test failure.
        assert_eq!(
            owned,
            vec!["let x = 1;\n".to_string()],
            "unexpected owned payloads; {borrowed} borrowed"
        );
    }

    /// The two cases where the format itself forbids borrowing, pinned so the
    /// exception stays narrow and visible: a text run containing a backslash
    /// escape (whose emitted text is not a slice of the source) and a
    /// paragraph joined from non-contiguous source lines.
    #[test]
    fn test_events_own_only_where_the_format_forces_it() {
        let escaped: Vec<_> = crate::events("A \\*literal\\* star.\n")
            .filter_map(|e| match e {
                Event::Text(c) => Some(c),
                _ => None,
            })
            .collect();
        assert!(
            escaped.iter().any(|c| matches!(c, Cow::Owned(_))),
            "escape resolution must produce an owned run: {escaped:?}"
        );

        let multiline: Vec<_> = crate::events("first source line\nsecond source line\n")
            .filter_map(|e| match e {
                Event::Text(c) => Some(c),
                _ => None,
            })
            .collect();
        assert!(
            multiline.iter().any(|c| matches!(c, Cow::Owned(_))),
            "a paragraph joined from two source lines cannot borrow: {multiline:?}"
        );

        // …but a single-line paragraph with no escapes must still borrow.
        let single: Vec<_> = crate::events("just one source line\n")
            .filter_map(|e| match e {
                Event::Text(c) => Some(c),
                _ => None,
            })
            .collect();
        assert!(
            single.iter().all(|c| matches!(c, Cow::Borrowed(_))),
            "single-line paragraph must borrow: {single:?}"
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
