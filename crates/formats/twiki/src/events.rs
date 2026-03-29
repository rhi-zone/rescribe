//! Streaming event iterator over a parsed `TwikiDoc`.

use std::borrow::Cow;

use crate::ast::*;

/// A streaming event from a TWiki document.
#[derive(Debug, PartialEq)]
pub enum Event<'a> {
    // ── Block events ──────────────────────────────────────────────────────────
    StartParagraph,
    EndParagraph,
    StartHeading { level: u8 },
    EndHeading,
    StartList { ordered: bool },
    EndList,
    StartListItem,
    EndListItem,
    StartTable,
    EndTable,
    StartTableRow,
    EndTableRow,
    StartTableCell { is_header: bool },
    EndTableCell,
    StartDefinitionList,
    EndDefinitionList,
    StartDefinitionTerm,
    EndDefinitionTerm,
    StartDefinitionDesc,
    EndDefinitionDesc,
    StartBlockquote,
    EndBlockquote,
    /// Leaf: a verbatim/code block.
    CodeBlock { content: Cow<'a, str> },
    /// Leaf: a horizontal rule.
    HorizontalRule,
    /// Leaf: a raw block (macro, etc.).
    RawBlock { content: Cow<'a, str> },

    // ── Inline events ─────────────────────────────────────────────────────────
    Text(Cow<'a, str>),
    LineBreak,
    StartBold,
    EndBold,
    StartItalic,
    EndItalic,
    StartBoldItalic,
    EndBoldItalic,
    StartStrikethrough,
    EndStrikethrough,
    StartSuperscript,
    EndSuperscript,
    StartSubscript,
    EndSubscript,
    StartUnderline,
    EndUnderline,
    StartBoldCode,
    EndBoldCode,
    StartLink { url: String },
    EndLink,
    /// Leaf: inline code.
    InlineCode(Cow<'a, str>),
    /// Leaf: image reference.
    Image { url: String, alt: String },
    /// Leaf: raw inline (macros, control tags).
    RawInline { content: String },
    /// Leaf: WikiWord auto-link.
    WikiWord { word: String },
}

/// Backwards-compatible alias for batch mode (all text is owned).
pub type OwnedEvent = Event<'static>;

impl<'a> Event<'a> {
    /// Convert to an owned event.
    pub fn into_owned(self) -> OwnedEvent {
        match self {
            Event::Text(cow) => Event::Text(Cow::Owned(cow.into_owned())),
            Event::InlineCode(cow) => Event::InlineCode(Cow::Owned(cow.into_owned())),
            Event::CodeBlock { content } => Event::CodeBlock {
                content: Cow::Owned(content.into_owned()),
            },
            Event::RawBlock { content } => Event::RawBlock {
                content: Cow::Owned(content.into_owned()),
            },
            // All other variants contain only String/'static fields.
            other => unsafe { std::mem::transmute::<Event<'_>, OwnedEvent>(other) },
        }
    }
}

// ── Linearize AST into event Vec ──────────────────────────────────────────────

/// Pull-based event iterator over a `TwikiDoc`.
///
/// Internally linearizes the AST into a Vec of events and iterates over them.
pub struct EventIter<'a> {
    events: std::vec::IntoIter<OwnedEvent>,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a> EventIter<'a> {
    pub fn new(doc: &'a TwikiDoc) -> Self {
        let mut evs = Vec::new();
        for block in &doc.blocks {
            emit_block(&mut evs, block);
        }
        EventIter {
            events: evs.into_iter(),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<'a> Iterator for EventIter<'a> {
    type Item = OwnedEvent;

    fn next(&mut self) -> Option<OwnedEvent> {
        self.events.next()
    }
}

fn emit_block(evs: &mut Vec<OwnedEvent>, block: &Block) {
    match block {
        Block::Paragraph { inlines, .. } => {
            evs.push(Event::StartParagraph);
            emit_inlines(evs, inlines);
            evs.push(Event::EndParagraph);
        }
        Block::Heading { level, inlines, .. } => {
            evs.push(Event::StartHeading { level: *level });
            emit_inlines(evs, inlines);
            evs.push(Event::EndHeading);
        }
        Block::CodeBlock { content, .. } => {
            evs.push(Event::CodeBlock { content: Cow::Owned(content.clone()) });
        }
        Block::List { ordered, items, .. } => {
            evs.push(Event::StartList { ordered: *ordered });
            for item in items {
                evs.push(Event::StartListItem);
                emit_inlines(evs, &item.inlines);
                for child in &item.children {
                    emit_block(evs, child);
                }
                evs.push(Event::EndListItem);
            }
            evs.push(Event::EndList);
        }
        Block::Table { rows, .. } => {
            evs.push(Event::StartTable);
            for row in rows {
                evs.push(Event::StartTableRow);
                for cell in &row.cells {
                    evs.push(Event::StartTableCell { is_header: cell.is_header });
                    emit_inlines(evs, &cell.inlines);
                    evs.push(Event::EndTableCell);
                }
                evs.push(Event::EndTableRow);
            }
            evs.push(Event::EndTable);
        }
        Block::HorizontalRule { .. } => {
            evs.push(Event::HorizontalRule);
        }
        Block::RawBlock { content, .. } => {
            evs.push(Event::RawBlock { content: Cow::Owned(content.clone()) });
        }
        Block::DefinitionList { items, .. } => {
            evs.push(Event::StartDefinitionList);
            for item in items {
                evs.push(Event::StartDefinitionTerm);
                emit_inlines(evs, &item.term);
                evs.push(Event::EndDefinitionTerm);
                evs.push(Event::StartDefinitionDesc);
                emit_inlines(evs, &item.desc);
                evs.push(Event::EndDefinitionDesc);
            }
            evs.push(Event::EndDefinitionList);
        }
        Block::Blockquote { children, .. } => {
            evs.push(Event::StartBlockquote);
            for child in children {
                emit_block(evs, child);
            }
            evs.push(Event::EndBlockquote);
        }
    }
}

fn emit_inlines(evs: &mut Vec<OwnedEvent>, inlines: &[Inline]) {
    for inline in inlines {
        emit_inline(evs, inline);
    }
}

fn emit_inline(evs: &mut Vec<OwnedEvent>, inline: &Inline) {
    match inline {
        Inline::Text(s, _) => {
            evs.push(Event::Text(Cow::Owned(s.clone())));
        }
        Inline::Bold(children, _) => {
            evs.push(Event::StartBold);
            emit_inlines(evs, children);
            evs.push(Event::EndBold);
        }
        Inline::Italic(children, _) => {
            evs.push(Event::StartItalic);
            emit_inlines(evs, children);
            evs.push(Event::EndItalic);
        }
        Inline::BoldItalic(children, _) => {
            evs.push(Event::StartBoldItalic);
            emit_inlines(evs, children);
            evs.push(Event::EndBoldItalic);
        }
        Inline::Code(s, _) => {
            evs.push(Event::InlineCode(Cow::Owned(s.clone())));
        }
        Inline::BoldCode(children, _) => {
            evs.push(Event::StartBoldCode);
            emit_inlines(evs, children);
            evs.push(Event::EndBoldCode);
        }
        Inline::Link { url, label, .. } => {
            evs.push(Event::StartLink { url: url.clone() });
            evs.push(Event::Text(Cow::Owned(label.clone())));
            evs.push(Event::EndLink);
        }
        Inline::LineBreak { .. } => {
            evs.push(Event::LineBreak);
        }
        Inline::Strikethrough(children, _) => {
            evs.push(Event::StartStrikethrough);
            emit_inlines(evs, children);
            evs.push(Event::EndStrikethrough);
        }
        Inline::Superscript(children, _) => {
            evs.push(Event::StartSuperscript);
            emit_inlines(evs, children);
            evs.push(Event::EndSuperscript);
        }
        Inline::Subscript(children, _) => {
            evs.push(Event::StartSubscript);
            emit_inlines(evs, children);
            evs.push(Event::EndSubscript);
        }
        Inline::Underline(children, _) => {
            evs.push(Event::StartUnderline);
            emit_inlines(evs, children);
            evs.push(Event::EndUnderline);
        }
        Inline::Image { url, alt, .. } => {
            evs.push(Event::Image { url: url.clone(), alt: alt.clone() });
        }
        Inline::RawInline { content, .. } => {
            evs.push(Event::RawInline { content: content.clone() });
        }
        Inline::WikiWord { word, .. } => {
            evs.push(Event::WikiWord { word: word.clone() });
        }
    }
}

/// Parse `input` and return a streaming iterator of events.
pub fn events(doc: &TwikiDoc) -> EventIter<'_> {
    EventIter::new(doc)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_events_heading() {
        let (doc, _) = crate::parse::parse("---+ Hello");
        let evs: Vec<_> = events(&doc).collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartHeading { level: 1 })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndHeading)));
    }

    #[test]
    fn test_events_paragraph() {
        let (doc, _) = crate::parse::parse("Hello world");
        let evs: Vec<_> = events(&doc).collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartParagraph)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndParagraph)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::Text(t) if t == "Hello world")));
    }

    #[test]
    fn test_events_bold() {
        let (doc, _) = crate::parse::parse("*bold*");
        let evs: Vec<_> = events(&doc).collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartBold)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndBold)));
    }

    #[test]
    fn test_events_table() {
        let (doc, _) = crate::parse::parse("| A | B |\n| C | D |");
        let evs: Vec<_> = events(&doc).collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartTable)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndTable)));
    }

    #[test]
    fn test_events_list() {
        let (doc, _) = crate::parse::parse("   * one\n   * two");
        let evs: Vec<_> = events(&doc).collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartList { ordered: false })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndList)));
    }
}
