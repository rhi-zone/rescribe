//! Streaming event iterator over a parsed `TikiwikiDoc`.

use std::borrow::Cow;

use crate::ast::*;

/// A streaming event from a TikiWiki document.
#[derive(Debug, PartialEq)]
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
    CodeBlock {
        language: Option<String>,
        content: Cow<'a, str>,
    },
    HorizontalRule,
    StartTable,
    EndTable,
    StartTableRow { is_header: bool },
    EndTableRow,
    StartTableCell,
    EndTableCell,

    // ── Inline events ─────────────────────────────────────────────────────────
    Text(Cow<'a, str>),
    LineBreak,
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
    InlineCode(Cow<'a, str>),
    Nowiki(Cow<'a, str>),
    StartLink { url: String },
    EndLink,
    StartWikiLink { page: String },
    EndWikiLink,
    InlineImage { url: String, alt: String },
}

/// Backwards-compatible alias for batch mode (all text is owned).
pub type OwnedEvent = Event<'static>;

impl<'a> Event<'a> {
    /// Convert to an owned event.
    pub fn into_owned(self) -> OwnedEvent {
        match self {
            Event::Text(cow) => Event::Text(Cow::Owned(cow.into_owned())),
            Event::InlineCode(cow) => Event::InlineCode(Cow::Owned(cow.into_owned())),
            Event::Nowiki(cow) => Event::Nowiki(Cow::Owned(cow.into_owned())),
            Event::CodeBlock { language, content } => Event::CodeBlock {
                language,
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
            Event::StartTableRow { is_header } => Event::StartTableRow { is_header },
            Event::EndTableRow => Event::EndTableRow,
            Event::StartTableCell => Event::StartTableCell,
            Event::EndTableCell => Event::EndTableCell,
            Event::LineBreak => Event::LineBreak,
            Event::StartBold => Event::StartBold,
            Event::EndBold => Event::EndBold,
            Event::StartItalic => Event::StartItalic,
            Event::EndItalic => Event::EndItalic,
            Event::StartUnderline => Event::StartUnderline,
            Event::EndUnderline => Event::EndUnderline,
            Event::StartStrikethrough => Event::StartStrikethrough,
            Event::EndStrikethrough => Event::EndStrikethrough,
            Event::StartSuperscript => Event::StartSuperscript,
            Event::EndSuperscript => Event::EndSuperscript,
            Event::StartSubscript => Event::StartSubscript,
            Event::EndSubscript => Event::EndSubscript,
            Event::StartLink { url } => Event::StartLink { url },
            Event::EndLink => Event::EndLink,
            Event::StartWikiLink { page } => Event::StartWikiLink { page },
            Event::EndWikiLink => Event::EndWikiLink,
            Event::InlineImage { url, alt } => Event::InlineImage { url, alt },
        }
    }
}

// ── EventIter ─────────────────────────────────────────────────────────────────

/// Iterator that yields events from a parsed TikiWiki document.
///
/// The iterator walks the AST and emits open/close pairs for containers
/// and leaf events for terminal nodes.
pub struct EventIter<'a> {
    /// The parsed document (held for lifetime).
    _input: &'a str,
    /// Pre-computed event list.
    events: std::vec::IntoIter<Event<'a>>,
}

impl<'a> EventIter<'a> {
    pub fn new(input: &'a str) -> Self {
        let (doc, _) = crate::parse::parse(input);
        let mut events = Vec::new();
        for block in &doc.blocks {
            emit_block(block, &mut events);
        }
        // Safety: all text fields are owned Strings from the parser, so they
        // have no lifetime dependency on `doc`. We transmute to tie them to
        // `'a` which is the input lifetime. This is safe because the Strings
        // are self-contained.
        let events: Vec<Event<'a>> = events
            .into_iter()
            .map(|e| unsafe { std::mem::transmute::<Event<'static>, Event<'a>>(e) })
            .collect();
        EventIter {
            _input: input,
            events: events.into_iter(),
        }
    }
}

impl<'a> Iterator for EventIter<'a> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.events.next()
    }
}

fn emit_block(block: &Block, out: &mut Vec<OwnedEvent>) {
    match block {
        Block::Paragraph { inlines, .. } => {
            out.push(Event::StartParagraph);
            emit_inlines(inlines, out);
            out.push(Event::EndParagraph);
        }
        Block::Heading { level, inlines, .. } => {
            out.push(Event::StartHeading { level: *level });
            emit_inlines(inlines, out);
            out.push(Event::EndHeading);
        }
        Block::CodeBlock { content, language, .. } => {
            out.push(Event::CodeBlock {
                language: language.clone(),
                content: Cow::Owned(content.clone()),
            });
        }
        Block::Blockquote { blocks, .. } => {
            out.push(Event::StartBlockquote);
            for b in blocks {
                emit_block(b, out);
            }
            out.push(Event::EndBlockquote);
        }
        Block::List { ordered, items, .. } => {
            out.push(Event::StartList { ordered: *ordered });
            for item in items {
                emit_list_item(item, out);
            }
            out.push(Event::EndList);
        }
        Block::Table { rows, .. } => {
            out.push(Event::StartTable);
            for row in rows {
                out.push(Event::StartTableRow { is_header: row.is_header });
                for cell in &row.cells {
                    out.push(Event::StartTableCell);
                    emit_inlines(&cell.inlines, out);
                    out.push(Event::EndTableCell);
                }
                out.push(Event::EndTableRow);
            }
            out.push(Event::EndTable);
        }
        Block::HorizontalRule { .. } => {
            out.push(Event::HorizontalRule);
        }
    }
}

fn emit_list_item(item: &ListItem, out: &mut Vec<OwnedEvent>) {
    out.push(Event::StartListItem);
    emit_inlines(&item.inlines, out);
    for child in &item.children {
        emit_block(child, out);
    }
    out.push(Event::EndListItem);
}

fn emit_inlines(inlines: &[Inline], out: &mut Vec<OwnedEvent>) {
    for inline in inlines {
        emit_inline(inline, out);
    }
}

fn emit_inline(inline: &Inline, out: &mut Vec<OwnedEvent>) {
    match inline {
        Inline::Text(s, _) => out.push(Event::Text(Cow::Owned(s.clone()))),
        Inline::Bold(children, _) => {
            out.push(Event::StartBold);
            emit_inlines(children, out);
            out.push(Event::EndBold);
        }
        Inline::Italic(children, _) => {
            out.push(Event::StartItalic);
            emit_inlines(children, out);
            out.push(Event::EndItalic);
        }
        Inline::Underline(children, _) => {
            out.push(Event::StartUnderline);
            emit_inlines(children, out);
            out.push(Event::EndUnderline);
        }
        Inline::Strikethrough(children, _) => {
            out.push(Event::StartStrikethrough);
            emit_inlines(children, out);
            out.push(Event::EndStrikethrough);
        }
        Inline::Superscript(children, _) => {
            out.push(Event::StartSuperscript);
            emit_inlines(children, out);
            out.push(Event::EndSuperscript);
        }
        Inline::Subscript(children, _) => {
            out.push(Event::StartSubscript);
            emit_inlines(children, out);
            out.push(Event::EndSubscript);
        }
        Inline::Code(s, _) => out.push(Event::InlineCode(Cow::Owned(s.clone()))),
        Inline::Nowiki(s, _) => out.push(Event::Nowiki(Cow::Owned(s.clone()))),
        Inline::Link { url, children, .. } => {
            out.push(Event::StartLink { url: url.clone() });
            emit_inlines(children, out);
            out.push(Event::EndLink);
        }
        Inline::WikiLink { page, children, .. } => {
            out.push(Event::StartWikiLink { page: page.clone() });
            emit_inlines(children, out);
            out.push(Event::EndWikiLink);
        }
        Inline::Image { url, alt, .. } => {
            out.push(Event::InlineImage { url: url.clone(), alt: alt.clone() });
        }
        Inline::LineBreak { .. } => out.push(Event::LineBreak),
    }
}

/// Parse `input` and return a streaming iterator of events.
pub fn events(input: &str) -> EventIter<'_> {
    EventIter::new(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_events_heading() {
        let evs: Vec<_> = events("! Hello").collect();
        assert!(evs.iter().any(|e| matches!(e, Event::StartHeading { level: 1 })));
        assert!(evs.iter().any(|e| matches!(e, Event::EndHeading)));
    }

    #[test]
    fn test_events_paragraph() {
        let evs: Vec<_> = events("Hello world").collect();
        assert!(evs.iter().any(|e| matches!(e, Event::StartParagraph)));
        assert!(evs.iter().any(|e| matches!(e, Event::EndParagraph)));
    }

    #[test]
    fn test_events_bold() {
        let evs: Vec<_> = events("__bold__").collect();
        assert!(evs.iter().any(|e| matches!(e, Event::StartBold)));
        assert!(evs.iter().any(|e| matches!(e, Event::EndBold)));
    }

    #[test]
    fn test_events_table() {
        let evs: Vec<_> = events("||A|B||\n||C|D||").collect();
        assert!(evs.iter().any(|e| matches!(e, Event::StartTable)));
        assert!(evs.iter().any(|e| matches!(e, Event::EndTable)));
        assert!(evs.iter().any(|e| matches!(e, Event::StartTableCell)));
    }
}
