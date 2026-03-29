//! Streaming event iterator over a parsed `MediawikiDoc`.

use std::borrow::Cow;

use crate::ast::*;

/// A streaming event from a MediaWiki document.
#[derive(Debug, PartialEq)]
pub enum Event<'a> {
    // -- Block events ---------------------------------------------------------
    StartDocument,
    EndDocument,
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
    StartDefinitionList,
    EndDefinitionList,
    StartDefinitionTerm,
    EndDefinitionTerm,
    StartDefinitionDesc,
    EndDefinitionDesc,
    /// Leaf: a code block (indented or syntaxhighlight).
    CodeBlock {
        language: Option<String>,
        content: Cow<'a, str>,
    },
    /// Leaf: a preformatted block (`<pre>`).
    PreBlock {
        content: Cow<'a, str>,
    },
    /// Leaf: a raw block (preserved verbatim).
    RawBlock {
        content: Cow<'a, str>,
    },
    /// Leaf: horizontal rule.
    HorizontalRule,
    StartTable {
        caption: Option<Vec<Inline>>,
    },
    EndTable,
    StartTableRow,
    EndTableRow,
    StartTableCell { is_header: bool },
    EndTableCell,

    // -- Inline events --------------------------------------------------------
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
    /// Leaf: inline code span.
    InlineCode(Cow<'a, str>),
    StartLink { url: String },
    EndLink,
    /// Leaf: image.
    InlineImage { url: String, alt: String },
    /// Leaf: footnote reference.
    FootnoteRef { label: String, content: Option<String> },
    /// Leaf: inline math.
    MathInline { source: String },
    /// Leaf: template transclusion (preserved raw).
    Template { content: String },
    /// Leaf: nowiki span (preserved raw).
    Nowiki { content: String },
}

/// Backwards-compatible alias for batch mode (all text is owned).
pub type OwnedEvent = Event<'static>;

impl<'a> Event<'a> {
    /// Convert to an owned event.
    pub fn into_owned(self) -> OwnedEvent {
        match self {
            Event::Text(cow) => Event::Text(Cow::Owned(cow.into_owned())),
            Event::InlineCode(cow) => Event::InlineCode(Cow::Owned(cow.into_owned())),
            Event::CodeBlock { language, content } => Event::CodeBlock {
                language,
                content: Cow::Owned(content.into_owned()),
            },
            Event::PreBlock { content } => {
                Event::PreBlock { content: Cow::Owned(content.into_owned()) }
            }
            Event::RawBlock { content } => {
                Event::RawBlock { content: Cow::Owned(content.into_owned()) }
            }
            // Safety: all other variants contain only String/'static fields.
            other => unsafe { std::mem::transmute::<Event<'_>, OwnedEvent>(other) },
        }
    }
}

// -- Event iterator -----------------------------------------------------------

/// Pull-based event iterator over a MediaWiki document.
pub struct EventIter {
    events: Vec<OwnedEvent>,
    pos: usize,
}

impl EventIter {
    /// Create a new event iterator from input text.
    pub fn new(input: &str) -> Self {
        let (doc, _) = crate::parse::parse(input);
        let mut events = Vec::new();
        emit_doc_events(&doc, &mut events);
        EventIter { events, pos: 0 }
    }
}

impl Iterator for EventIter {
    type Item = OwnedEvent;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos < self.events.len() {
            let event = std::mem::replace(
                &mut self.events[self.pos],
                Event::EndDocument, // placeholder
            );
            self.pos += 1;
            Some(event)
        } else {
            None
        }
    }
}

fn emit_doc_events(doc: &MediawikiDoc, out: &mut Vec<OwnedEvent>) {
    for block in &doc.blocks {
        emit_block_events(block, out);
    }
}

fn emit_block_events(block: &Block, out: &mut Vec<OwnedEvent>) {
    match block {
        Block::Paragraph { inlines, .. } => {
            out.push(Event::StartParagraph);
            emit_inline_events(inlines, out);
            out.push(Event::EndParagraph);
        }
        Block::Heading { level, inlines, .. } => {
            out.push(Event::StartHeading { level: *level });
            emit_inline_events(inlines, out);
            out.push(Event::EndHeading);
        }
        Block::CodeBlock { language, content, .. } => {
            out.push(Event::CodeBlock {
                language: language.clone(),
                content: Cow::Owned(content.clone()),
            });
        }
        Block::List { ordered, items, .. } => {
            out.push(Event::StartList { ordered: *ordered });
            for item_blocks in items {
                out.push(Event::StartListItem);
                for b in item_blocks {
                    emit_block_events(b, out);
                }
                out.push(Event::EndListItem);
            }
            out.push(Event::EndList);
        }
        Block::DefinitionList { items, .. } => {
            out.push(Event::StartDefinitionList);
            for item in items {
                out.push(Event::StartDefinitionTerm);
                emit_inline_events(&item.term, out);
                out.push(Event::EndDefinitionTerm);
                out.push(Event::StartDefinitionDesc);
                emit_inline_events(&item.desc, out);
                out.push(Event::EndDefinitionDesc);
            }
            out.push(Event::EndDefinitionList);
        }
        Block::HorizontalRule => {
            out.push(Event::HorizontalRule);
        }
        Block::Table { rows, caption, .. } => {
            out.push(Event::StartTable { caption: caption.clone() });
            for row in rows {
                out.push(Event::StartTableRow);
                for cell in &row.cells {
                    out.push(Event::StartTableCell { is_header: cell.is_header });
                    emit_inline_events(&cell.inlines, out);
                    out.push(Event::EndTableCell);
                }
                out.push(Event::EndTableRow);
            }
            out.push(Event::EndTable);
        }
        Block::Blockquote { children, .. } => {
            out.push(Event::StartBlockquote);
            for child in children {
                emit_block_events(child, out);
            }
            out.push(Event::EndBlockquote);
        }
        Block::PreBlock { content, .. } => {
            out.push(Event::PreBlock { content: Cow::Owned(content.clone()) });
        }
        Block::RawBlock { content, .. } => {
            out.push(Event::RawBlock { content: Cow::Owned(content.clone()) });
        }
    }
}

fn emit_inline_events(inlines: &[Inline], out: &mut Vec<OwnedEvent>) {
    for inline in inlines {
        match inline {
            Inline::Text(s) => {
                out.push(Event::Text(Cow::Owned(s.clone())));
            }
            Inline::Bold(children) => {
                out.push(Event::StartBold);
                emit_inline_events(children, out);
                out.push(Event::EndBold);
            }
            Inline::Italic(children) => {
                out.push(Event::StartItalic);
                emit_inline_events(children, out);
                out.push(Event::EndItalic);
            }
            Inline::Code(s) => {
                out.push(Event::InlineCode(Cow::Owned(s.clone())));
            }
            Inline::Link { url, text } => {
                out.push(Event::StartLink { url: url.clone() });
                out.push(Event::Text(Cow::Owned(text.clone())));
                out.push(Event::EndLink);
            }
            Inline::Image { url, alt } => {
                out.push(Event::InlineImage { url: url.clone(), alt: alt.clone() });
            }
            Inline::LineBreak => {
                out.push(Event::LineBreak);
            }
            Inline::Strikeout(children) => {
                out.push(Event::StartStrikethrough);
                emit_inline_events(children, out);
                out.push(Event::EndStrikethrough);
            }
            Inline::Underline(children) => {
                out.push(Event::StartUnderline);
                emit_inline_events(children, out);
                out.push(Event::EndUnderline);
            }
            Inline::Subscript(children) => {
                out.push(Event::StartSubscript);
                emit_inline_events(children, out);
                out.push(Event::EndSubscript);
            }
            Inline::Superscript(children) => {
                out.push(Event::StartSuperscript);
                emit_inline_events(children, out);
                out.push(Event::EndSuperscript);
            }
            Inline::FootnoteRef { label, content } => {
                out.push(Event::FootnoteRef {
                    label: label.clone(),
                    content: content.clone(),
                });
            }
            Inline::MathInline { source } => {
                out.push(Event::MathInline { source: source.clone() });
            }
            Inline::Template { content } => {
                out.push(Event::Template { content: content.clone() });
            }
            Inline::Nowiki { content } => {
                out.push(Event::Nowiki { content: content.clone() });
            }
        }
    }
}

/// Parse `input` and return a streaming iterator of events.
pub fn events(input: &str) -> EventIter {
    EventIter::new(input)
}

// -- Tests --------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_events_paragraph() {
        let evs: Vec<_> = events("Hello world").collect();
        assert!(evs.iter().any(|e| matches!(e, Event::StartParagraph)));
        assert!(evs.iter().any(|e| matches!(e, Event::EndParagraph)));
        assert!(evs.iter().any(|e| matches!(e, Event::Text(t) if t == "Hello world")));
    }

    #[test]
    fn test_events_heading() {
        let evs: Vec<_> = events("== Title ==").collect();
        assert!(evs.iter().any(|e| matches!(e, Event::StartHeading { level: 2 })));
        assert!(evs.iter().any(|e| matches!(e, Event::EndHeading)));
    }

    #[test]
    fn test_events_bold() {
        let evs: Vec<_> = events("'''bold'''").collect();
        assert!(evs.iter().any(|e| matches!(e, Event::StartBold)));
        assert!(evs.iter().any(|e| matches!(e, Event::EndBold)));
    }

    #[test]
    fn test_events_list() {
        let evs: Vec<_> = events("* item 1\n* item 2").collect();
        assert!(evs.iter().any(|e| matches!(e, Event::StartList { ordered: false })));
        assert_eq!(
            evs.iter().filter(|e| matches!(e, Event::StartListItem)).count(),
            2
        );
        assert!(evs.iter().any(|e| matches!(e, Event::EndList)));
    }

    #[test]
    fn test_events_table() {
        let evs: Vec<_> = events("{|\n! Name\n|-\n| Alice\n|}").collect();
        assert!(evs.iter().any(|e| matches!(e, Event::StartTable { .. })));
        assert!(evs.iter().any(|e| matches!(e, Event::StartTableCell { is_header: true })));
        assert!(evs.iter().any(|e| matches!(e, Event::StartTableCell { is_header: false })));
        assert!(evs.iter().any(|e| matches!(e, Event::EndTable)));
    }

    #[test]
    fn test_events_horizontal_rule() {
        let evs: Vec<_> = events("----").collect();
        assert!(evs.iter().any(|e| matches!(e, Event::HorizontalRule)));
    }

    #[test]
    fn test_events_link() {
        let evs: Vec<_> = events("[[Page|text]]").collect();
        assert!(evs.iter().any(|e| matches!(e, Event::StartLink { url } if url == "Page")));
        assert!(evs.iter().any(|e| matches!(e, Event::EndLink)));
    }

    #[test]
    fn test_events_blockquote() {
        let evs: Vec<_> = events("<blockquote>\nquoted\n</blockquote>").collect();
        assert!(evs.iter().any(|e| matches!(e, Event::StartBlockquote)));
        assert!(evs.iter().any(|e| matches!(e, Event::EndBlockquote)));
    }

    #[test]
    fn test_events_footnote() {
        let evs: Vec<_> = events("Text<ref>note</ref>.").collect();
        assert!(evs
            .iter()
            .any(|e| matches!(e, Event::FootnoteRef { content: Some(c), .. } if c == "note")));
    }

    #[test]
    fn test_events_math() {
        let evs: Vec<_> = events("Solve <math>x^2</math>.").collect();
        assert!(evs
            .iter()
            .any(|e| matches!(e, Event::MathInline { source } if source == "x^2")));
    }
}
