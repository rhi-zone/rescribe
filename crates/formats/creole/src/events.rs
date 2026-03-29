//! Streaming event iterator over a parsed `CreoleDoc`.

use std::borrow::Cow;

use crate::ast::*;

/// A streaming event from a Creole document.
#[derive(Debug, PartialEq, Clone)]
pub enum Event<'a> {
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
    /// Leaf: a preformatted/code block.
    CodeBlock { content: Cow<'a, str> },
    /// Leaf: a horizontal rule (`----`).
    HorizontalRule,
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

    // ── Inline events ─────────────────────────────────────────────────────────
    Text(Cow<'a, str>),
    LineBreak,
    StartBold,
    EndBold,
    StartItalic,
    EndItalic,
    /// Leaf: inline code span.
    InlineCode(Cow<'a, str>),
    StartLink { url: String },
    EndLink,
    /// Leaf: inline image.
    InlineImage { url: String, alt: Option<String> },
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
            Event::HorizontalRule => Event::HorizontalRule,
            Event::StartTable => Event::StartTable,
            Event::EndTable => Event::EndTable,
            Event::StartTableRow => Event::StartTableRow,
            Event::EndTableRow => Event::EndTableRow,
            Event::StartTableCell { is_header } => Event::StartTableCell { is_header },
            Event::EndTableCell => Event::EndTableCell,
            Event::StartDefinitionList => Event::StartDefinitionList,
            Event::EndDefinitionList => Event::EndDefinitionList,
            Event::StartDefinitionTerm => Event::StartDefinitionTerm,
            Event::EndDefinitionTerm => Event::EndDefinitionTerm,
            Event::StartDefinitionDesc => Event::StartDefinitionDesc,
            Event::EndDefinitionDesc => Event::EndDefinitionDesc,
            Event::LineBreak => Event::LineBreak,
            Event::StartBold => Event::StartBold,
            Event::EndBold => Event::EndBold,
            Event::StartItalic => Event::StartItalic,
            Event::EndItalic => Event::EndItalic,
            Event::StartLink { url } => Event::StartLink { url },
            Event::EndLink => Event::EndLink,
            Event::InlineImage { url, alt } => Event::InlineImage { url, alt },
        }
    }
}

// ── Event iterator ───────────────────────────────────────────────────────────

/// Pull-based event iterator. Parses the input once and yields events one at a time.
pub struct EventIter {
    events: Vec<OwnedEvent>,
    pos: usize,
}

impl EventIter {
    /// Create a new event iterator from input text.
    pub fn new(input: &str) -> Self {
        let (doc, _) = crate::parse::parse(input);
        let events = collect_events(&doc);
        EventIter { events, pos: 0 }
    }

    /// Create from a pre-parsed document.
    pub fn from_doc(doc: &CreoleDoc) -> Self {
        let events = collect_events(doc);
        EventIter { events, pos: 0 }
    }
}

impl Iterator for EventIter {
    type Item = OwnedEvent;

    fn next(&mut self) -> Option<OwnedEvent> {
        if self.pos >= self.events.len() {
            return None;
        }
        let idx = self.pos;
        self.pos += 1;
        Some(self.events[idx].clone())
    }
}

/// Collect all events from a document into a Vec of owned events.
fn collect_events(doc: &CreoleDoc) -> Vec<OwnedEvent> {
    let mut events = Vec::new();
    for block in &doc.blocks {
        collect_block_events(block, &mut events);
    }
    events
}

fn collect_block_events(block: &Block, events: &mut Vec<OwnedEvent>) {
    match block {
        Block::Paragraph { inlines, .. } => {
            events.push(Event::StartParagraph);
            collect_inline_events(inlines, events);
            events.push(Event::EndParagraph);
        }
        Block::Heading { level, inlines, .. } => {
            events.push(Event::StartHeading { level: *level });
            collect_inline_events(inlines, events);
            events.push(Event::EndHeading);
        }
        Block::CodeBlock { content, .. } => {
            events.push(Event::CodeBlock { content: Cow::Owned(content.clone()) });
        }
        Block::Blockquote { children, .. } => {
            events.push(Event::StartBlockquote);
            for child in children {
                collect_block_events(child, events);
            }
            events.push(Event::EndBlockquote);
        }
        Block::List { ordered, items, .. } => {
            events.push(Event::StartList { ordered: *ordered });
            for item in items {
                events.push(Event::StartListItem);
                for child in item {
                    collect_block_events(child, events);
                }
                events.push(Event::EndListItem);
            }
            events.push(Event::EndList);
        }
        Block::Table { rows, .. } => {
            events.push(Event::StartTable);
            for row in rows {
                events.push(Event::StartTableRow);
                for cell in &row.cells {
                    events.push(Event::StartTableCell { is_header: cell.is_header });
                    collect_inline_events(&cell.inlines, events);
                    events.push(Event::EndTableCell);
                }
                events.push(Event::EndTableRow);
            }
            events.push(Event::EndTable);
        }
        Block::DefinitionList { items, .. } => {
            events.push(Event::StartDefinitionList);
            for item in items {
                events.push(Event::StartDefinitionTerm);
                collect_inline_events(&item.term, events);
                events.push(Event::EndDefinitionTerm);
                events.push(Event::StartDefinitionDesc);
                collect_inline_events(&item.desc, events);
                events.push(Event::EndDefinitionDesc);
            }
            events.push(Event::EndDefinitionList);
        }
        Block::HorizontalRule(_) => {
            events.push(Event::HorizontalRule);
        }
    }
}

fn collect_inline_events(inlines: &[Inline], events: &mut Vec<OwnedEvent>) {
    for inline in inlines {
        match inline {
            Inline::Text(s, _) => {
                events.push(Event::Text(Cow::Owned(s.clone())));
            }
            Inline::LineBreak(_) => {
                events.push(Event::LineBreak);
            }
            Inline::Code(s, _) => {
                events.push(Event::InlineCode(Cow::Owned(s.clone())));
            }
            Inline::Bold(children, _) => {
                events.push(Event::StartBold);
                collect_inline_events(children, events);
                events.push(Event::EndBold);
            }
            Inline::Italic(children, _) => {
                events.push(Event::StartItalic);
                collect_inline_events(children, events);
                events.push(Event::EndItalic);
            }
            Inline::Link { url, children, .. } => {
                events.push(Event::StartLink { url: url.clone() });
                collect_inline_events(children, events);
                events.push(Event::EndLink);
            }
            Inline::Image { url, alt, .. } => {
                events.push(Event::InlineImage { url: url.clone(), alt: alt.clone() });
            }
        }
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Parse `input` and return a streaming iterator of [`OwnedEvent`] items.
pub fn events(input: &str) -> EventIter {
    EventIter::new(input)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_events_heading() {
        let evs: Vec<_> = events("= Hello").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartHeading { level: 1 })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndHeading)));
    }

    #[test]
    fn test_events_paragraph() {
        let evs: Vec<_> = events("Hello world").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartParagraph)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndParagraph)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::Text(t) if t == "Hello world")));
    }

    #[test]
    fn test_events_code_block() {
        let evs: Vec<_> = events("{{{\nfn main() {}\n}}}").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::CodeBlock { .. })));
    }

    #[test]
    fn test_events_list() {
        let evs: Vec<_> = events("* item 1\n* item 2").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartList { ordered: false })));
        assert_eq!(evs.iter().filter(|e| matches!(e, OwnedEvent::StartListItem)).count(), 2);
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndList)));
    }

    #[test]
    fn test_events_table() {
        let evs: Vec<_> = events("|= Name |= Age |\n| Alice | 30 |").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartTable)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartTableCell { is_header: true })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndTable)));
    }

    #[test]
    fn test_events_horizontal_rule() {
        let evs: Vec<_> = events("----").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::HorizontalRule)));
    }

    #[test]
    fn test_events_bold_italic() {
        let evs: Vec<_> = events("**bold** //italic//").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartBold)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndBold)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartItalic)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndItalic)));
    }

    #[test]
    fn test_events_link() {
        let evs: Vec<_> = events("[[https://example.com|click here]]").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartLink { url } if url == "https://example.com")));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndLink)));
    }

    #[test]
    fn test_events_inline_code() {
        let evs: Vec<_> = events("Some {{{verbatim}}} text").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::InlineCode(s) if s == "verbatim")));
    }

    #[test]
    fn test_events_blockquote() {
        let evs: Vec<_> = events("> quoted text").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartBlockquote)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndBlockquote)));
    }

    #[test]
    fn test_events_definition_list() {
        let evs: Vec<_> = events("; Term\n: Description").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartDefinitionList)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartDefinitionTerm)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartDefinitionDesc)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndDefinitionList)));
    }
}
