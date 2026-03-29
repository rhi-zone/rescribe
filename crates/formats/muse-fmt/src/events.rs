//! Streaming event types and iterator for Muse documents.
//!
//! The [`MuseEvent`] enum represents the full set of block and inline
//! open/close pairs that correspond one-to-one with the AST types in
//! [`crate::ast`].
//!
//! [`EventIter`] walks a parsed [`MuseDoc`] and yields events in
//! document order.
//!
//! # Example
//! ```
//! let (doc, _) = muse_fmt::parse("* Hello\n\nA paragraph.\n");
//! let events: Vec<_> = muse_fmt::events::events(&doc).collect();
//! assert!(events.len() > 0);
//! ```

use std::borrow::Cow;
use std::collections::VecDeque;

use crate::ast::{Block, Inline, MuseDoc};

/// A streaming event from a Muse document.
///
/// Text content fields use `Cow<'a, str>` so future optimisations can yield
/// borrowed slices without changing the public API.
#[derive(Debug, PartialEq)]
pub enum MuseEvent<'a> {
    // ── Block events ──────────────────────────────────────────────────────────
    StartDocument,
    EndDocument,
    StartParagraph,
    EndParagraph,
    StartHeading {
        level: u8,
    },
    EndHeading,
    StartBlockquote,
    EndBlockquote,
    StartVerse,
    EndVerse,
    StartCenteredBlock,
    EndCenteredBlock,
    StartRightBlock,
    EndRightBlock,
    LiteralBlock {
        content: Cow<'a, str>,
    },
    SrcBlock {
        lang: Option<Cow<'a, str>>,
        content: Cow<'a, str>,
    },
    CodeBlock {
        content: Cow<'a, str>,
    },
    Comment {
        content: Cow<'a, str>,
    },
    StartTable,
    EndTable,
    StartTableRow {
        header: bool,
    },
    EndTableRow,
    StartTableCell,
    EndTableCell,
    StartList {
        ordered: bool,
    },
    EndList,
    StartListItem,
    EndListItem,
    StartDefinitionList,
    EndDefinitionList,
    StartDefinitionTerm,
    EndDefinitionTerm,
    StartDefinitionDesc,
    EndDefinitionDesc,
    HorizontalRule,
    StartFootnoteDef {
        label: Cow<'a, str>,
    },
    EndFootnoteDef,

    // ── Inline events ─────────────────────────────────────────────────────────
    Text(Cow<'a, str>),
    StartBold,
    EndBold,
    StartItalic,
    EndItalic,
    StartUnderline,
    EndUnderline,
    StartStrikethrough,
    EndStrikethrough,
    StartSuperscript,
    EndSuperscript,
    StartSubscript,
    EndSubscript,
    Code(Cow<'a, str>),
    FootnoteRef {
        label: Cow<'a, str>,
    },
    LineBreak,
    Anchor {
        name: Cow<'a, str>,
    },
    StartLink {
        url: Cow<'a, str>,
    },
    EndLink,
    Image {
        src: Cow<'a, str>,
        alt: Option<Cow<'a, str>>,
    },
}

/// Backwards-compatible alias for batch mode (all text is owned).
pub type OwnedMuseEvent = MuseEvent<'static>;

impl<'a> MuseEvent<'a> {
    /// Convert to an owned event (all `Cow::Borrowed` fields become `Cow::Owned`).
    pub fn into_owned(self) -> OwnedMuseEvent {
        match self {
            MuseEvent::StartDocument => MuseEvent::StartDocument,
            MuseEvent::EndDocument => MuseEvent::EndDocument,
            MuseEvent::StartParagraph => MuseEvent::StartParagraph,
            MuseEvent::EndParagraph => MuseEvent::EndParagraph,
            MuseEvent::StartHeading { level } => MuseEvent::StartHeading { level },
            MuseEvent::EndHeading => MuseEvent::EndHeading,
            MuseEvent::StartBlockquote => MuseEvent::StartBlockquote,
            MuseEvent::EndBlockquote => MuseEvent::EndBlockquote,
            MuseEvent::StartVerse => MuseEvent::StartVerse,
            MuseEvent::EndVerse => MuseEvent::EndVerse,
            MuseEvent::StartCenteredBlock => MuseEvent::StartCenteredBlock,
            MuseEvent::EndCenteredBlock => MuseEvent::EndCenteredBlock,
            MuseEvent::StartRightBlock => MuseEvent::StartRightBlock,
            MuseEvent::EndRightBlock => MuseEvent::EndRightBlock,
            MuseEvent::LiteralBlock { content } => MuseEvent::LiteralBlock {
                content: Cow::Owned(content.into_owned()),
            },
            MuseEvent::SrcBlock { lang, content } => MuseEvent::SrcBlock {
                lang: lang.map(|l| Cow::Owned(l.into_owned())),
                content: Cow::Owned(content.into_owned()),
            },
            MuseEvent::CodeBlock { content } => MuseEvent::CodeBlock {
                content: Cow::Owned(content.into_owned()),
            },
            MuseEvent::Comment { content } => MuseEvent::Comment {
                content: Cow::Owned(content.into_owned()),
            },
            MuseEvent::StartTable => MuseEvent::StartTable,
            MuseEvent::EndTable => MuseEvent::EndTable,
            MuseEvent::StartTableRow { header } => MuseEvent::StartTableRow { header },
            MuseEvent::EndTableRow => MuseEvent::EndTableRow,
            MuseEvent::StartTableCell => MuseEvent::StartTableCell,
            MuseEvent::EndTableCell => MuseEvent::EndTableCell,
            MuseEvent::StartList { ordered } => MuseEvent::StartList { ordered },
            MuseEvent::EndList => MuseEvent::EndList,
            MuseEvent::StartListItem => MuseEvent::StartListItem,
            MuseEvent::EndListItem => MuseEvent::EndListItem,
            MuseEvent::StartDefinitionList => MuseEvent::StartDefinitionList,
            MuseEvent::EndDefinitionList => MuseEvent::EndDefinitionList,
            MuseEvent::StartDefinitionTerm => MuseEvent::StartDefinitionTerm,
            MuseEvent::EndDefinitionTerm => MuseEvent::EndDefinitionTerm,
            MuseEvent::StartDefinitionDesc => MuseEvent::StartDefinitionDesc,
            MuseEvent::EndDefinitionDesc => MuseEvent::EndDefinitionDesc,
            MuseEvent::HorizontalRule => MuseEvent::HorizontalRule,
            MuseEvent::StartFootnoteDef { label } => MuseEvent::StartFootnoteDef {
                label: Cow::Owned(label.into_owned()),
            },
            MuseEvent::EndFootnoteDef => MuseEvent::EndFootnoteDef,
            MuseEvent::Text(cow) => MuseEvent::Text(Cow::Owned(cow.into_owned())),
            MuseEvent::StartBold => MuseEvent::StartBold,
            MuseEvent::EndBold => MuseEvent::EndBold,
            MuseEvent::StartItalic => MuseEvent::StartItalic,
            MuseEvent::EndItalic => MuseEvent::EndItalic,
            MuseEvent::StartUnderline => MuseEvent::StartUnderline,
            MuseEvent::EndUnderline => MuseEvent::EndUnderline,
            MuseEvent::StartStrikethrough => MuseEvent::StartStrikethrough,
            MuseEvent::EndStrikethrough => MuseEvent::EndStrikethrough,
            MuseEvent::StartSuperscript => MuseEvent::StartSuperscript,
            MuseEvent::EndSuperscript => MuseEvent::EndSuperscript,
            MuseEvent::StartSubscript => MuseEvent::StartSubscript,
            MuseEvent::EndSubscript => MuseEvent::EndSubscript,
            MuseEvent::Code(cow) => MuseEvent::Code(Cow::Owned(cow.into_owned())),
            MuseEvent::FootnoteRef { label } => MuseEvent::FootnoteRef {
                label: Cow::Owned(label.into_owned()),
            },
            MuseEvent::LineBreak => MuseEvent::LineBreak,
            MuseEvent::Anchor { name } => MuseEvent::Anchor {
                name: Cow::Owned(name.into_owned()),
            },
            MuseEvent::StartLink { url } => MuseEvent::StartLink {
                url: Cow::Owned(url.into_owned()),
            },
            MuseEvent::EndLink => MuseEvent::EndLink,
            MuseEvent::Image { src, alt } => MuseEvent::Image {
                src: Cow::Owned(src.into_owned()),
                alt: alt.map(|a| Cow::Owned(a.into_owned())),
            },
        }
    }
}

// ── EventIter ────────────────────────────────────────────────────────────────

/// Iterator that walks a [`MuseDoc`] and yields [`MuseEvent`] items.
pub struct EventIter<'a> {
    queue: VecDeque<MuseEvent<'a>>,
}

impl<'a> EventIter<'a> {
    fn new(doc: &'a MuseDoc) -> Self {
        let mut queue = VecDeque::new();
        queue.push_back(MuseEvent::StartDocument);
        for block in &doc.blocks {
            enqueue_block(block, &mut queue);
        }
        queue.push_back(MuseEvent::EndDocument);
        EventIter { queue }
    }
}

impl<'a> Iterator for EventIter<'a> {
    type Item = MuseEvent<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.queue.pop_front()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.queue.len();
        (len, Some(len))
    }
}

impl<'a> ExactSizeIterator for EventIter<'a> {}

// ── Block flattening ─────────────────────────────────────────────────────────

fn enqueue_block<'a>(block: &'a Block, q: &mut VecDeque<MuseEvent<'a>>) {
    match block {
        Block::Paragraph { inlines, .. } => {
            q.push_back(MuseEvent::StartParagraph);
            enqueue_inlines(inlines, q);
            q.push_back(MuseEvent::EndParagraph);
        }
        Block::Heading { level, inlines, .. } => {
            q.push_back(MuseEvent::StartHeading { level: *level });
            enqueue_inlines(inlines, q);
            q.push_back(MuseEvent::EndHeading);
        }
        Block::CodeBlock { content, .. } => {
            q.push_back(MuseEvent::CodeBlock {
                content: Cow::Borrowed(content),
            });
        }
        Block::Blockquote { children, .. } => {
            q.push_back(MuseEvent::StartBlockquote);
            for child in children {
                enqueue_block(child, q);
            }
            q.push_back(MuseEvent::EndBlockquote);
        }
        Block::List { ordered, items, .. } => {
            q.push_back(MuseEvent::StartList { ordered: *ordered });
            for item_blocks in items {
                q.push_back(MuseEvent::StartListItem);
                for b in item_blocks {
                    enqueue_block(b, q);
                }
                q.push_back(MuseEvent::EndListItem);
            }
            q.push_back(MuseEvent::EndList);
        }
        Block::DefinitionList { items, .. } => {
            q.push_back(MuseEvent::StartDefinitionList);
            for (term_inlines, desc_blocks) in items {
                q.push_back(MuseEvent::StartDefinitionTerm);
                enqueue_inlines(term_inlines, q);
                q.push_back(MuseEvent::EndDefinitionTerm);
                q.push_back(MuseEvent::StartDefinitionDesc);
                for b in desc_blocks {
                    enqueue_block(b, q);
                }
                q.push_back(MuseEvent::EndDefinitionDesc);
            }
            q.push_back(MuseEvent::EndDefinitionList);
        }
        Block::HorizontalRule { .. } => {
            q.push_back(MuseEvent::HorizontalRule);
        }
        Block::Verse { children, .. } => {
            q.push_back(MuseEvent::StartVerse);
            for child in children {
                enqueue_block(child, q);
            }
            q.push_back(MuseEvent::EndVerse);
        }
        Block::CenteredBlock { children, .. } => {
            q.push_back(MuseEvent::StartCenteredBlock);
            for child in children {
                enqueue_block(child, q);
            }
            q.push_back(MuseEvent::EndCenteredBlock);
        }
        Block::RightBlock { children, .. } => {
            q.push_back(MuseEvent::StartRightBlock);
            for child in children {
                enqueue_block(child, q);
            }
            q.push_back(MuseEvent::EndRightBlock);
        }
        Block::LiteralBlock { content, .. } => {
            q.push_back(MuseEvent::LiteralBlock {
                content: Cow::Borrowed(content),
            });
        }
        Block::SrcBlock { lang, content, .. } => {
            q.push_back(MuseEvent::SrcBlock {
                lang: lang.as_deref().map(Cow::Borrowed),
                content: Cow::Borrowed(content),
            });
        }
        Block::Comment { content, .. } => {
            q.push_back(MuseEvent::Comment {
                content: Cow::Borrowed(content),
            });
        }
        Block::Table { rows, .. } => {
            q.push_back(MuseEvent::StartTable);
            for row in rows {
                q.push_back(MuseEvent::StartTableRow {
                    header: row.header,
                });
                for cell in &row.cells {
                    q.push_back(MuseEvent::StartTableCell);
                    enqueue_inlines(cell, q);
                    q.push_back(MuseEvent::EndTableCell);
                }
                q.push_back(MuseEvent::EndTableRow);
            }
            q.push_back(MuseEvent::EndTable);
        }
        Block::FootnoteDef {
            label, content, ..
        } => {
            q.push_back(MuseEvent::StartFootnoteDef {
                label: Cow::Borrowed(label),
            });
            enqueue_inlines(content, q);
            q.push_back(MuseEvent::EndFootnoteDef);
        }
    }
}

// ── Inline flattening ────────────────────────────────────────────────────────

fn enqueue_inlines<'a>(inlines: &'a [Inline], q: &mut VecDeque<MuseEvent<'a>>) {
    for inline in inlines {
        enqueue_inline(inline, q);
    }
}

fn enqueue_inline<'a>(inline: &'a Inline, q: &mut VecDeque<MuseEvent<'a>>) {
    match inline {
        Inline::Text(s, _) => {
            q.push_back(MuseEvent::Text(Cow::Borrowed(s)));
        }
        Inline::Bold(children, _) => {
            q.push_back(MuseEvent::StartBold);
            enqueue_inlines(children, q);
            q.push_back(MuseEvent::EndBold);
        }
        Inline::Italic(children, _) => {
            q.push_back(MuseEvent::StartItalic);
            enqueue_inlines(children, q);
            q.push_back(MuseEvent::EndItalic);
        }
        Inline::Code(s, _) => {
            q.push_back(MuseEvent::Code(Cow::Borrowed(s)));
        }
        Inline::Link { url, children, .. } => {
            q.push_back(MuseEvent::StartLink {
                url: Cow::Borrowed(url),
            });
            enqueue_inlines(children, q);
            q.push_back(MuseEvent::EndLink);
        }
        Inline::Underline(children, _) => {
            q.push_back(MuseEvent::StartUnderline);
            enqueue_inlines(children, q);
            q.push_back(MuseEvent::EndUnderline);
        }
        Inline::Strikethrough(children, _) => {
            q.push_back(MuseEvent::StartStrikethrough);
            enqueue_inlines(children, q);
            q.push_back(MuseEvent::EndStrikethrough);
        }
        Inline::Superscript(children, _) => {
            q.push_back(MuseEvent::StartSuperscript);
            enqueue_inlines(children, q);
            q.push_back(MuseEvent::EndSuperscript);
        }
        Inline::Subscript(children, _) => {
            q.push_back(MuseEvent::StartSubscript);
            enqueue_inlines(children, q);
            q.push_back(MuseEvent::EndSubscript);
        }
        Inline::FootnoteRef { label, .. } => {
            q.push_back(MuseEvent::FootnoteRef {
                label: Cow::Borrowed(label),
            });
        }
        Inline::LineBreak(_) => {
            q.push_back(MuseEvent::LineBreak);
        }
        Inline::Anchor { name, .. } => {
            q.push_back(MuseEvent::Anchor {
                name: Cow::Borrowed(name),
            });
        }
        Inline::Image { src, alt, .. } => {
            q.push_back(MuseEvent::Image {
                src: Cow::Borrowed(src),
                alt: alt.as_deref().map(Cow::Borrowed),
            });
        }
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Walk a [`MuseDoc`] and return a streaming iterator of [`MuseEvent`] items.
pub fn events(doc: &MuseDoc) -> EventIter<'_> {
    EventIter::new(doc)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_events_heading() {
        let (doc, _) = crate::parse("* Hello\n");
        let evs: Vec<_> = events(&doc).collect();
        assert!(evs.iter().any(|e| matches!(e, MuseEvent::StartHeading { level: 1 })));
        assert!(evs.iter().any(|e| matches!(e, MuseEvent::EndHeading)));
        assert!(evs.iter().any(|e| matches!(e, MuseEvent::Text(t) if t == "Hello")));
    }

    #[test]
    fn test_events_paragraph() {
        let (doc, _) = crate::parse("Hello world\n");
        let evs: Vec<_> = events(&doc).collect();
        assert!(evs.iter().any(|e| matches!(e, MuseEvent::StartParagraph)));
        assert!(evs.iter().any(|e| matches!(e, MuseEvent::EndParagraph)));
    }

    #[test]
    fn test_events_bold_italic() {
        let (doc, _) = crate::parse("**bold** and *italic*\n");
        let evs: Vec<_> = events(&doc).collect();
        assert!(evs.iter().any(|e| matches!(e, MuseEvent::StartBold)));
        assert!(evs.iter().any(|e| matches!(e, MuseEvent::EndBold)));
        assert!(evs.iter().any(|e| matches!(e, MuseEvent::StartItalic)));
        assert!(evs.iter().any(|e| matches!(e, MuseEvent::EndItalic)));
    }

    #[test]
    fn test_events_code_block() {
        let (doc, _) = crate::parse("<example>\ncode here\n</example>\n");
        let evs: Vec<_> = events(&doc).collect();
        assert!(evs.iter().any(|e| matches!(e, MuseEvent::CodeBlock { .. })));
    }

    #[test]
    fn test_events_list() {
        let (doc, _) = crate::parse(" - item1\n - item2\n");
        let evs: Vec<_> = events(&doc).collect();
        assert!(evs.iter().any(|e| matches!(e, MuseEvent::StartList { ordered: false })));
        assert_eq!(
            evs.iter().filter(|e| matches!(e, MuseEvent::StartListItem)).count(),
            2
        );
        assert!(evs.iter().any(|e| matches!(e, MuseEvent::EndList)));
    }

    #[test]
    fn test_events_table() {
        let (doc, _) = crate::parse("|| Name || Age ||\n| Alice | 30 |\n");
        let evs: Vec<_> = events(&doc).collect();
        assert!(evs.iter().any(|e| matches!(e, MuseEvent::StartTable)));
        assert!(evs.iter().any(|e| matches!(e, MuseEvent::StartTableRow { header: true })));
        assert!(evs.iter().any(|e| matches!(e, MuseEvent::EndTable)));
    }

    #[test]
    fn test_events_horizontal_rule() {
        let (doc, _) = crate::parse("----\n");
        let evs: Vec<_> = events(&doc).collect();
        assert!(evs.iter().any(|e| matches!(e, MuseEvent::HorizontalRule)));
    }

    #[test]
    fn test_events_link() {
        let (doc, _) = crate::parse("[[https://example.com][click]]\n");
        let evs: Vec<_> = events(&doc).collect();
        assert!(evs.iter().any(|e| matches!(e, MuseEvent::StartLink { url } if url == "https://example.com")));
        assert!(evs.iter().any(|e| matches!(e, MuseEvent::EndLink)));
    }

    #[test]
    fn test_events_document_wrapper() {
        let doc = MuseDoc::default();
        let evs: Vec<_> = events(&doc).collect();
        assert_eq!(evs.len(), 2);
        assert!(matches!(evs[0], MuseEvent::StartDocument));
        assert!(matches!(evs[1], MuseEvent::EndDocument));
    }

    #[test]
    fn test_events_exact_size() {
        let (doc, _) = crate::parse("* Heading\n\nParagraph.\n");
        let iter = events(&doc);
        let hint = iter.size_hint();
        let evs: Vec<_> = events(&doc).collect();
        assert_eq!(hint.0, evs.len());
        assert_eq!(hint.1, Some(evs.len()));
    }

    #[test]
    fn test_events_verse() {
        let (doc, _) = crate::parse("<verse>\nLine one\nLine two\n</verse>\n");
        let evs: Vec<_> = events(&doc).collect();
        assert!(evs.iter().any(|e| matches!(e, MuseEvent::StartVerse)));
        assert!(evs.iter().any(|e| matches!(e, MuseEvent::EndVerse)));
    }

    #[test]
    fn test_events_footnote_def_and_ref() {
        let (doc, _) = crate::parse("[1] A footnote.\n");
        let evs: Vec<_> = events(&doc).collect();
        assert!(evs
            .iter()
            .any(|e| matches!(e, MuseEvent::StartFootnoteDef { label } if label == "1")));
        assert!(evs.iter().any(|e| matches!(e, MuseEvent::EndFootnoteDef)));
    }
}
