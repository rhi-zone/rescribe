//! Streaming event iterator over a parsed `T2tDoc`.

use std::borrow::Cow;

use crate::ast::*;

/// A streaming event from a txt2tags document.
#[derive(Debug, PartialEq)]
pub enum Event<'a> {
    // ── Block events ──────────────────────────────────────────────────────────
    StartDocument,
    EndDocument,
    StartParagraph,
    EndParagraph,
    StartHeading { level: u8, numbered: bool },
    EndHeading,
    CodeBlock { content: Cow<'a, str> },
    RawBlock { content: Cow<'a, str> },
    StartBlockquote,
    EndBlockquote,
    StartTable,
    EndTable,
    StartTableRow { header: bool },
    EndTableRow,
    StartTableCell,
    EndTableCell,
    HorizontalRule,
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
    Code(Cow<'a, str>),
    Verbatim(Cow<'a, str>),
    Tagged(Cow<'a, str>),
    StartLink { url: Cow<'a, str> },
    EndLink,
    Image { src: Cow<'a, str> },
    LineBreak,
    SoftBreak,
}

/// Backwards-compatible alias for batch mode (all text is owned).
pub type OwnedEvent = Event<'static>;

impl<'a> Event<'a> {
    /// Convert to an owned event.
    pub fn into_owned(self) -> OwnedEvent {
        match self {
            Event::Text(cow) => Event::Text(Cow::Owned(cow.into_owned())),
            Event::Code(cow) => Event::Code(Cow::Owned(cow.into_owned())),
            Event::Verbatim(cow) => Event::Verbatim(Cow::Owned(cow.into_owned())),
            Event::Tagged(cow) => Event::Tagged(Cow::Owned(cow.into_owned())),
            Event::CodeBlock { content } => Event::CodeBlock {
                content: Cow::Owned(content.into_owned()),
            },
            Event::RawBlock { content } => Event::RawBlock {
                content: Cow::Owned(content.into_owned()),
            },
            Event::StartLink { url } => Event::StartLink {
                url: Cow::Owned(url.into_owned()),
            },
            Event::Image { src } => Event::Image {
                src: Cow::Owned(src.into_owned()),
            },
            // All other variants contain no borrowed data.
            Event::StartDocument => Event::StartDocument,
            Event::EndDocument => Event::EndDocument,
            Event::StartParagraph => Event::StartParagraph,
            Event::EndParagraph => Event::EndParagraph,
            Event::StartHeading { level, numbered } => Event::StartHeading { level, numbered },
            Event::EndHeading => Event::EndHeading,
            Event::StartBlockquote => Event::StartBlockquote,
            Event::EndBlockquote => Event::EndBlockquote,
            Event::StartTable => Event::StartTable,
            Event::EndTable => Event::EndTable,
            Event::StartTableRow { header } => Event::StartTableRow { header },
            Event::EndTableRow => Event::EndTableRow,
            Event::StartTableCell => Event::StartTableCell,
            Event::EndTableCell => Event::EndTableCell,
            Event::HorizontalRule => Event::HorizontalRule,
            Event::StartList { ordered } => Event::StartList { ordered },
            Event::EndList => Event::EndList,
            Event::StartListItem => Event::StartListItem,
            Event::EndListItem => Event::EndListItem,
            Event::StartDefinitionList => Event::StartDefinitionList,
            Event::EndDefinitionList => Event::EndDefinitionList,
            Event::StartDefinitionTerm => Event::StartDefinitionTerm,
            Event::EndDefinitionTerm => Event::EndDefinitionTerm,
            Event::StartDefinitionDesc => Event::StartDefinitionDesc,
            Event::EndDefinitionDesc => Event::EndDefinitionDesc,
            Event::StartBold => Event::StartBold,
            Event::EndBold => Event::EndBold,
            Event::StartItalic => Event::StartItalic,
            Event::EndItalic => Event::EndItalic,
            Event::StartUnderline => Event::StartUnderline,
            Event::EndUnderline => Event::EndUnderline,
            Event::StartStrikethrough => Event::StartStrikethrough,
            Event::EndStrikethrough => Event::EndStrikethrough,
            Event::EndLink => Event::EndLink,
            Event::LineBreak => Event::LineBreak,
            Event::SoftBreak => Event::SoftBreak,
        }
    }
}

// ── True pull iterator ────────────────────────────────────────────────────────

/// Lazy frame stack for the event iterator.
enum Frame {
    /// Document-level: iterating over blocks.
    Document { blocks: Vec<Block>, idx: usize },
    /// Paragraph: iterating over inlines.
    Paragraph { inlines: Vec<Inline>, idx: usize },
    /// Heading: iterating over inlines.
    Heading { inlines: Vec<Inline>, idx: usize },
    /// Blockquote: iterating over child blocks.
    Blockquote { children: Vec<Block>, idx: usize },
    /// List: iterating over items.
    List { items: Vec<Vec<Block>>, idx: usize },
    /// ListItem: iterating over blocks within an item.
    ListItem { blocks: Vec<Block>, idx: usize },
    /// Table: iterating over rows.
    Table { rows: Vec<TableRow>, idx: usize },
    /// TableRow: iterating over cells.
    TableRow { cells: Vec<Vec<Inline>>, idx: usize },
    /// TableCell: iterating over inlines.
    TableCell { inlines: Vec<Inline>, idx: usize },
    /// DefinitionList: iterating over items.
    DefinitionList {
        items: Vec<(Vec<Inline>, Vec<Block>)>,
        idx: usize,
    },
    /// DefinitionTerm: iterating over inlines.
    DefinitionTerm { inlines: Vec<Inline>, idx: usize },
    /// DefinitionDesc: iterating over blocks.
    DefinitionDesc {
        blocks: Vec<Block>,
        idx: usize,
        started: bool,
    },
    /// Inline container: Bold, Italic, Underline, Strikethrough, Link.
    InlineContainer {
        kind: InlineContainerKind,
        inlines: Vec<Inline>,
        idx: usize,
    },
}

#[derive(Debug, Clone, Copy)]
enum InlineContainerKind {
    Bold,
    Italic,
    Underline,
    Strikethrough,
    Link,
}

/// Pull iterator over events from a `T2tDoc`.
pub struct EventIter {
    frame_stack: Vec<Frame>,
    done: bool,
    /// True if we need to emit StartDocument on the first call.
    emit_start: bool,
}

impl EventIter {
    pub fn new(doc: T2tDoc) -> Self {
        EventIter {
            frame_stack: vec![Frame::Document {
                blocks: doc.blocks,
                idx: 0,
            }],
            done: false,
            emit_start: true,
        }
    }
}

impl Iterator for EventIter {
    type Item = OwnedEvent;

    fn next(&mut self) -> Option<OwnedEvent> {
        if self.done {
            return None;
        }

        if self.emit_start {
            self.emit_start = false;
            return Some(Event::StartDocument);
        }

        loop {
            let frame = self.frame_stack.last_mut()?;

            match frame {
                Frame::Document { blocks, idx } => {
                    if *idx < blocks.len() {
                        let block = blocks[*idx].clone();
                        *idx += 1;
                        return Some(push_block(&mut self.frame_stack, block));
                    } else {
                        self.frame_stack.pop();
                        self.done = true;
                        return Some(Event::EndDocument);
                    }
                }

                Frame::Paragraph { inlines, idx } => {
                    if *idx < inlines.len() {
                        let inline = inlines[*idx].clone();
                        *idx += 1;
                        return Some(push_inline(&mut self.frame_stack, inline));
                    } else {
                        self.frame_stack.pop();
                        return Some(Event::EndParagraph);
                    }
                }

                Frame::Heading { inlines, idx } => {
                    if *idx < inlines.len() {
                        let inline = inlines[*idx].clone();
                        *idx += 1;
                        return Some(push_inline(&mut self.frame_stack, inline));
                    } else {
                        self.frame_stack.pop();
                        return Some(Event::EndHeading);
                    }
                }

                Frame::Blockquote { children, idx } => {
                    if *idx < children.len() {
                        let block = children[*idx].clone();
                        *idx += 1;
                        return Some(push_block(&mut self.frame_stack, block));
                    } else {
                        self.frame_stack.pop();
                        return Some(Event::EndBlockquote);
                    }
                }

                Frame::List { items, idx } => {
                    if *idx < items.len() {
                        let item_blocks = items[*idx].clone();
                        *idx += 1;
                        self.frame_stack.push(Frame::ListItem {
                            blocks: item_blocks,
                            idx: 0,
                        });
                        return Some(Event::StartListItem);
                    } else {
                        self.frame_stack.pop();
                        return Some(Event::EndList);
                    }
                }

                Frame::ListItem { blocks, idx } => {
                    if *idx < blocks.len() {
                        let block = blocks[*idx].clone();
                        *idx += 1;
                        return Some(push_block(&mut self.frame_stack, block));
                    } else {
                        self.frame_stack.pop();
                        return Some(Event::EndListItem);
                    }
                }

                Frame::Table { rows, idx } => {
                    if *idx < rows.len() {
                        let row = rows[*idx].clone();
                        *idx += 1;
                        let header = row.is_header;
                        self.frame_stack.push(Frame::TableRow {
                            cells: row.cells,
                            idx: 0,
                        });
                        return Some(Event::StartTableRow { header });
                    } else {
                        self.frame_stack.pop();
                        return Some(Event::EndTable);
                    }
                }

                Frame::TableRow { cells, idx } => {
                    if *idx < cells.len() {
                        let cell_inlines = cells[*idx].clone();
                        *idx += 1;
                        self.frame_stack.push(Frame::TableCell {
                            inlines: cell_inlines,
                            idx: 0,
                        });
                        return Some(Event::StartTableCell);
                    } else {
                        self.frame_stack.pop();
                        return Some(Event::EndTableRow);
                    }
                }

                Frame::TableCell { inlines, idx } => {
                    if *idx < inlines.len() {
                        let inline = inlines[*idx].clone();
                        *idx += 1;
                        return Some(push_inline(&mut self.frame_stack, inline));
                    } else {
                        self.frame_stack.pop();
                        return Some(Event::EndTableCell);
                    }
                }

                Frame::DefinitionList { items, idx } => {
                    if *idx < items.len() {
                        let (term, desc) = items[*idx].clone();
                        *idx += 1;
                        // Push desc frame first (will be processed after term)
                        // We need term first, so push desc, then term on top.
                        // Actually, we process term first by pushing desc then term:
                        // term frame goes on top, gets processed first.
                        self.frame_stack.push(Frame::DefinitionDesc {
                            blocks: desc,
                            idx: 0,
                            started: false,
                        });
                        self.frame_stack.push(Frame::DefinitionTerm {
                            inlines: term,
                            idx: 0,
                        });
                        return Some(Event::StartDefinitionTerm);
                    } else {
                        self.frame_stack.pop();
                        return Some(Event::EndDefinitionList);
                    }
                }

                Frame::DefinitionTerm { inlines, idx } => {
                    if *idx < inlines.len() {
                        let inline = inlines[*idx].clone();
                        *idx += 1;
                        return Some(push_inline(&mut self.frame_stack, inline));
                    } else {
                        self.frame_stack.pop();
                        // Now the DefinitionDesc frame is on top; emit StartDefinitionDesc
                        // But first, emit EndDefinitionTerm
                        return Some(Event::EndDefinitionTerm);
                    }
                }

                Frame::DefinitionDesc { blocks, idx, started } => {
                    if !*started {
                        *started = true;
                        return Some(Event::StartDefinitionDesc);
                    }
                    if *idx < blocks.len() {
                        let block = blocks[*idx].clone();
                        *idx += 1;
                        return Some(push_block(&mut self.frame_stack, block));
                    } else {
                        self.frame_stack.pop();
                        return Some(Event::EndDefinitionDesc);
                    }
                }

                Frame::InlineContainer { kind, inlines, idx } => {
                    if *idx < inlines.len() {
                        let inline = inlines[*idx].clone();
                        *idx += 1;
                        return Some(push_inline(&mut self.frame_stack, inline));
                    } else {
                        let end_event = match kind {
                            InlineContainerKind::Bold => Event::EndBold,
                            InlineContainerKind::Italic => Event::EndItalic,
                            InlineContainerKind::Underline => Event::EndUnderline,
                            InlineContainerKind::Strikethrough => Event::EndStrikethrough,
                            InlineContainerKind::Link => Event::EndLink,
                        };
                        self.frame_stack.pop();
                        return Some(end_event);
                    }
                }
            }
        }
    }
}

/// Push a block frame and return its start event.
fn push_block(stack: &mut Vec<Frame>, block: Block) -> OwnedEvent {
    match block {
        Block::Paragraph { inlines, .. } => {
            stack.push(Frame::Paragraph { inlines, idx: 0 });
            Event::StartParagraph
        }
        Block::Heading {
            level,
            numbered,
            inlines,
            ..
        } => {
            stack.push(Frame::Heading { inlines, idx: 0 });
            Event::StartHeading { level, numbered }
        }
        Block::CodeBlock { content, .. } => Event::CodeBlock {
            content: Cow::Owned(content),
        },
        Block::RawBlock { content, .. } => Event::RawBlock {
            content: Cow::Owned(content),
        },
        Block::Blockquote { children, .. } => {
            stack.push(Frame::Blockquote {
                children,
                idx: 0,
            });
            Event::StartBlockquote
        }
        Block::List { ordered, items, .. } => {
            stack.push(Frame::List { items, idx: 0 });
            Event::StartList { ordered }
        }
        Block::Table { rows, .. } => {
            stack.push(Frame::Table { rows, idx: 0 });
            Event::StartTable
        }
        Block::HorizontalRule { .. } => Event::HorizontalRule,
        Block::DefinitionList { items, .. } => {
            stack.push(Frame::DefinitionList { items, idx: 0 });
            Event::StartDefinitionList
        }
    }
}

/// Push an inline frame (if container) and return its start/leaf event.
fn push_inline(stack: &mut Vec<Frame>, inline: Inline) -> OwnedEvent {
    match inline {
        Inline::Text(s, _) => Event::Text(Cow::Owned(s)),
        Inline::Bold(children, _) => {
            stack.push(Frame::InlineContainer {
                kind: InlineContainerKind::Bold,
                inlines: children,
                idx: 0,
            });
            Event::StartBold
        }
        Inline::Italic(children, _) => {
            stack.push(Frame::InlineContainer {
                kind: InlineContainerKind::Italic,
                inlines: children,
                idx: 0,
            });
            Event::StartItalic
        }
        Inline::Underline(children, _) => {
            stack.push(Frame::InlineContainer {
                kind: InlineContainerKind::Underline,
                inlines: children,
                idx: 0,
            });
            Event::StartUnderline
        }
        Inline::Strikethrough(children, _) => {
            stack.push(Frame::InlineContainer {
                kind: InlineContainerKind::Strikethrough,
                inlines: children,
                idx: 0,
            });
            Event::StartStrikethrough
        }
        Inline::Code(s, _) => Event::Code(Cow::Owned(s)),
        Inline::Verbatim(s, _) => Event::Verbatim(Cow::Owned(s)),
        Inline::Tagged(s, _) => Event::Tagged(Cow::Owned(s)),
        Inline::Link { url, children, .. } => {
            stack.push(Frame::InlineContainer {
                kind: InlineContainerKind::Link,
                inlines: children,
                idx: 0,
            });
            Event::StartLink {
                url: Cow::Owned(url),
            }
        }
        Inline::Image { url, .. } => Event::Image {
            src: Cow::Owned(url),
        },
        Inline::LineBreak(_) => Event::LineBreak,
        Inline::SoftBreak(_) => Event::SoftBreak,
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Parse `input` and return a streaming iterator of events.
pub fn events(input: &str) -> EventIter {
    let (doc, _) = crate::parse::parse(input);
    EventIter::new(doc)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_events_heading() {
        let evs: Vec<_> = events("= Hello =\n").collect();
        assert!(evs
            .iter()
            .any(|e| matches!(e, Event::StartHeading { level: 1, numbered: false })));
        assert!(evs.iter().any(|e| matches!(e, Event::EndHeading)));
    }

    #[test]
    fn test_events_paragraph() {
        let evs: Vec<_> = events("Hello world\n").collect();
        assert!(evs.iter().any(|e| matches!(e, Event::StartParagraph)));
        assert!(evs.iter().any(|e| matches!(e, Event::EndParagraph)));
        assert!(evs
            .iter()
            .any(|e| matches!(e, Event::Text(t) if t == "Hello world")));
    }

    #[test]
    fn test_events_code_block() {
        let evs: Vec<_> = events("```\nfn main() {}\n```\n").collect();
        assert!(evs
            .iter()
            .any(|e| matches!(e, Event::CodeBlock { .. })));
    }

    #[test]
    fn test_events_list() {
        let evs: Vec<_> = events("- item 1\n- item 2\n").collect();
        assert!(evs
            .iter()
            .any(|e| matches!(e, Event::StartList { ordered: false })));
        assert_eq!(
            evs.iter()
                .filter(|e| matches!(e, Event::StartListItem))
                .count(),
            2
        );
        assert!(evs.iter().any(|e| matches!(e, Event::EndList)));
    }

    #[test]
    fn test_events_table() {
        let evs: Vec<_> = events("|| Name | Age |\n| Alice | 30 |\n").collect();
        assert!(evs.iter().any(|e| matches!(e, Event::StartTable)));
        assert!(evs
            .iter()
            .any(|e| matches!(e, Event::StartTableRow { header: true })));
        assert!(evs.iter().any(|e| matches!(e, Event::EndTable)));
    }

    #[test]
    fn test_events_horizontal_rule() {
        let evs: Vec<_> = events("--------------------\n").collect();
        assert!(evs.iter().any(|e| matches!(e, Event::HorizontalRule)));
    }

    #[test]
    fn test_events_bold_italic() {
        let evs: Vec<_> = events("**bold** //italic//\n").collect();
        assert!(evs.iter().any(|e| matches!(e, Event::StartBold)));
        assert!(evs.iter().any(|e| matches!(e, Event::EndBold)));
        assert!(evs.iter().any(|e| matches!(e, Event::StartItalic)));
        assert!(evs.iter().any(|e| matches!(e, Event::EndItalic)));
    }

    #[test]
    fn test_events_link() {
        let evs: Vec<_> = events("[click http://example.com]\n").collect();
        assert!(evs
            .iter()
            .any(|e| matches!(e, Event::StartLink { url } if url == "http://example.com")));
        assert!(evs.iter().any(|e| matches!(e, Event::EndLink)));
    }

    #[test]
    fn test_events_definition_list() {
        let evs: Vec<_> = events(": Term\nDefinition text\n").collect();
        assert!(evs
            .iter()
            .any(|e| matches!(e, Event::StartDefinitionList)));
        assert!(evs
            .iter()
            .any(|e| matches!(e, Event::StartDefinitionTerm)));
        assert!(evs
            .iter()
            .any(|e| matches!(e, Event::EndDefinitionList)));
    }

    #[test]
    fn test_events_verbatim() {
        let evs: Vec<_> = events("\"\"raw text\"\"\n").collect();
        assert!(evs
            .iter()
            .any(|e| matches!(e, Event::Verbatim(t) if t == "raw text")));
    }

    #[test]
    fn test_events_tagged() {
        let evs: Vec<_> = events("''tagged text''\n").collect();
        assert!(evs
            .iter()
            .any(|e| matches!(e, Event::Tagged(t) if t == "tagged text")));
    }

    #[test]
    fn test_events_document_boundary() {
        let evs: Vec<_> = events("Hello\n").collect();
        assert_eq!(evs.first(), Some(&Event::StartDocument));
        assert_eq!(evs.last(), Some(&Event::EndDocument));
    }
}
