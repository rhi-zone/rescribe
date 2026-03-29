//! Streaming event iterator over a parsed `JiraDoc`.

use std::borrow::Cow;

use crate::ast::*;

/// A streaming event from a Jira wiki markup document.
#[derive(Debug, PartialEq)]
pub enum Event<'a> {
    // ── Block events ──────────────────────────────────────────────────────────
    StartParagraph,
    EndParagraph,
    StartHeading { level: u8 },
    EndHeading,
    StartBlockquote,
    EndBlockquote,
    StartPanel { title: Option<String> },
    EndPanel,
    StartList { ordered: bool },
    EndList,
    StartListItem,
    EndListItem,
    /// Leaf: a fenced code block.
    CodeBlock {
        language: Option<String>,
        content: Cow<'a, str>,
    },
    /// Leaf: a noformat/preformatted block.
    Noformat {
        content: Cow<'a, str>,
    },
    /// Leaf: a horizontal rule (`----`).
    HorizontalRule,
    StartTable,
    EndTable,
    StartTableRow,
    EndTableRow,
    StartTableCell { is_header: bool },
    EndTableCell,

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
    /// Leaf: inline code span.
    InlineCode(Cow<'a, str>),
    StartLink { url: String },
    EndLink,
    /// Leaf: standalone image.
    InlineImage { url: String, alt: Option<String> },
    StartColorSpan { color: String },
    EndColorSpan,
    /// Leaf: @mention.
    Mention(Cow<'a, str>),
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
            Event::Noformat { content } => Event::Noformat {
                content: Cow::Owned(content.into_owned()),
            },
            Event::Mention(cow) => Event::Mention(Cow::Owned(cow.into_owned())),
            // All other variants contain only owned fields or no data.
            Event::StartParagraph => Event::StartParagraph,
            Event::EndParagraph => Event::EndParagraph,
            Event::StartHeading { level } => Event::StartHeading { level },
            Event::EndHeading => Event::EndHeading,
            Event::StartBlockquote => Event::StartBlockquote,
            Event::EndBlockquote => Event::EndBlockquote,
            Event::StartPanel { title } => Event::StartPanel { title },
            Event::EndPanel => Event::EndPanel,
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
            Event::InlineImage { url, alt } => Event::InlineImage { url, alt },
            Event::StartColorSpan { color } => Event::StartColorSpan { color },
            Event::EndColorSpan => Event::EndColorSpan,
        }
    }
}

/// Parse `input` and return a streaming iterator of events.
pub fn events(input: &str) -> EagerEventIter {
    let (doc, _) = crate::parse::parse(input);
    let mut evs = Vec::new();
    emit_doc_events(&doc, &mut evs);
    EagerEventIter { events: evs, pos: 0 }
}

/// Eager event iterator (events pre-computed from AST).
pub struct EagerEventIter {
    events: Vec<OwnedEvent>,
    pos: usize,
}

impl Iterator for EagerEventIter {
    type Item = OwnedEvent;

    fn next(&mut self) -> Option<OwnedEvent> {
        if self.pos < self.events.len() {
            let ev = std::mem::replace(
                &mut self.events[self.pos],
                Event::StartParagraph, // placeholder
            );
            self.pos += 1;
            Some(ev)
        } else {
            None
        }
    }
}

fn emit_doc_events(doc: &JiraDoc, out: &mut Vec<OwnedEvent>) {
    for block in &doc.blocks {
        emit_block_events(block, out);
    }
}

fn emit_block_events(block: &Block, out: &mut Vec<OwnedEvent>) {
    match block {
        Block::Paragraph { inlines, .. } => {
            out.push(Event::StartParagraph);
            emit_inlines_events(inlines, out);
            out.push(Event::EndParagraph);
        }
        Block::Heading { level, inlines, .. } => {
            out.push(Event::StartHeading { level: *level });
            emit_inlines_events(inlines, out);
            out.push(Event::EndHeading);
        }
        Block::CodeBlock { content, language, .. } => {
            out.push(Event::CodeBlock {
                language: language.clone(),
                content: Cow::Owned(content.clone()),
            });
        }
        Block::Noformat { content, .. } => {
            out.push(Event::Noformat {
                content: Cow::Owned(content.clone()),
            });
        }
        Block::Blockquote { children, .. } => {
            out.push(Event::StartBlockquote);
            for child in children {
                emit_block_events(child, out);
            }
            out.push(Event::EndBlockquote);
        }
        Block::Panel { title, children, .. } => {
            out.push(Event::StartPanel { title: title.clone() });
            for child in children {
                emit_block_events(child, out);
            }
            out.push(Event::EndPanel);
        }
        Block::List { ordered, items, .. } => {
            out.push(Event::StartList { ordered: *ordered });
            for item in items {
                out.push(Event::StartListItem);
                for content in &item.children {
                    match content {
                        ListItemContent::Inline(inlines) => {
                            out.push(Event::StartParagraph);
                            emit_inlines_events(inlines, out);
                            out.push(Event::EndParagraph);
                        }
                        ListItemContent::NestedList(block) => {
                            emit_block_events(block, out);
                        }
                    }
                }
                out.push(Event::EndListItem);
            }
            out.push(Event::EndList);
        }
        Block::Table { rows, .. } => {
            out.push(Event::StartTable);
            for row in rows {
                out.push(Event::StartTableRow);
                for cell in &row.cells {
                    out.push(Event::StartTableCell { is_header: cell.is_header });
                    emit_inlines_events(&cell.inlines, out);
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

fn emit_inlines_events(inlines: &[Inline], out: &mut Vec<OwnedEvent>) {
    for inline in inlines {
        emit_inline_events(inline, out);
    }
}

fn emit_inline_events(inline: &Inline, out: &mut Vec<OwnedEvent>) {
    match inline {
        Inline::Text(s, _) => {
            out.push(Event::Text(Cow::Owned(s.clone())));
        }
        Inline::Bold(children, _) => {
            out.push(Event::StartBold);
            emit_inlines_events(children, out);
            out.push(Event::EndBold);
        }
        Inline::Italic(children, _) => {
            out.push(Event::StartItalic);
            emit_inlines_events(children, out);
            out.push(Event::EndItalic);
        }
        Inline::Underline(children, _) => {
            out.push(Event::StartUnderline);
            emit_inlines_events(children, out);
            out.push(Event::EndUnderline);
        }
        Inline::Strikethrough(children, _) => {
            out.push(Event::StartStrikethrough);
            emit_inlines_events(children, out);
            out.push(Event::EndStrikethrough);
        }
        Inline::Code(s, _) => {
            out.push(Event::InlineCode(Cow::Owned(s.clone())));
        }
        Inline::Link { url, children, .. } => {
            out.push(Event::StartLink { url: url.clone() });
            emit_inlines_events(children, out);
            out.push(Event::EndLink);
        }
        Inline::Image { url, alt, .. } => {
            out.push(Event::InlineImage { url: url.clone(), alt: alt.clone() });
        }
        Inline::Superscript(children, _) => {
            out.push(Event::StartSuperscript);
            emit_inlines_events(children, out);
            out.push(Event::EndSuperscript);
        }
        Inline::Subscript(children, _) => {
            out.push(Event::StartSubscript);
            emit_inlines_events(children, out);
            out.push(Event::EndSubscript);
        }
        Inline::ColorSpan { color, children, .. } => {
            out.push(Event::StartColorSpan { color: color.clone() });
            emit_inlines_events(children, out);
            out.push(Event::EndColorSpan);
        }
        Inline::Mention(name, _) => {
            out.push(Event::Mention(Cow::Owned(name.clone())));
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_events_heading() {
        let evs: Vec<_> = events("h1. Hello").collect();
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
        let evs: Vec<_> = events("{code:java}\npublic class Test {}\n{code}").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::CodeBlock { language: Some(l), .. } if l == "java")));
    }

    #[test]
    fn test_events_bold() {
        let evs: Vec<_> = events("*bold*").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartBold)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndBold)));
    }

    #[test]
    fn test_events_link() {
        let evs: Vec<_> = events("[click|https://example.com]").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartLink { url } if url == "https://example.com")));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndLink)));
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
        let evs: Vec<_> = events("||H1||H2||\n|a|b|").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartTable)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartTableCell { is_header: true })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartTableCell { is_header: false })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndTable)));
    }

    #[test]
    fn test_events_horizontal_rule() {
        let evs: Vec<_> = events("----").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::HorizontalRule)));
    }
}
