//! Streaming event iterator over a parsed `DokuwikiDoc`.

use std::borrow::Cow;

use crate::ast::*;

/// A streaming event from a DokuWiki document.
#[derive(Debug, PartialEq)]
pub enum Event<'a> {
    // -- Block events --------------------------------------------------------
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
    FileBlock {
        language: Option<String>,
        filename: Option<String>,
        content: Cow<'a, str>,
    },
    HorizontalRule,
    StartTable,
    EndTable,
    StartTableRow { is_header: bool },
    EndTableRow,
    StartTableCell,
    EndTableCell,
    StartDefinitionList,
    EndDefinitionList,
    StartDefinitionTerm,
    EndDefinitionTerm,
    StartDefinitionDesc,
    EndDefinitionDesc,
    RawBlock { format: String, content: String },
    Macro { name: String },

    // -- Inline events -------------------------------------------------------
    Text(Cow<'a, str>),
    SoftBreak,
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
    InlineImage { url: String, alt: Option<String> },
    FootnoteRef { content: String },
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
            Event::FileBlock {
                language,
                filename,
                content,
            } => Event::FileBlock {
                language,
                filename,
                content: Cow::Owned(content.into_owned()),
            },
            // All other variants contain only String/'static fields.
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
            Event::StartDefinitionList => Event::StartDefinitionList,
            Event::EndDefinitionList => Event::EndDefinitionList,
            Event::StartDefinitionTerm => Event::StartDefinitionTerm,
            Event::EndDefinitionTerm => Event::EndDefinitionTerm,
            Event::StartDefinitionDesc => Event::StartDefinitionDesc,
            Event::EndDefinitionDesc => Event::EndDefinitionDesc,
            Event::RawBlock { format, content } => Event::RawBlock { format, content },
            Event::Macro { name } => Event::Macro { name },
            Event::SoftBreak => Event::SoftBreak,
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
            Event::InlineImage { url, alt } => Event::InlineImage { url, alt },
            Event::FootnoteRef { content } => Event::FootnoteRef { content },
        }
    }
}

// -- EventIter (true pull iterator) ----------------------------------------

/// Frame for the event iterator's lazy stack.
enum Frame<'a> {
    // Block frames
    BlockList {
        blocks: &'a [Block],
        index: usize,
    },
    HeadingEnd,
    ParagraphBody {
        inlines: &'a [Inline],
        index: usize,
    },
    ParagraphEnd,
    BlockquoteBody {
        children: &'a [Block],
        index: usize,
    },
    BlockquoteEnd,
    ListBody {
        items: &'a [ListItem],
        index: usize,
    },
    ListEnd,
    ListItemBody {
        inlines: &'a [Inline],
        inline_index: usize,
        children: &'a [Block],
        child_index: usize,
    },
    ListItemEnd,
    TableBody {
        rows: &'a [TableRow],
        index: usize,
    },
    TableEnd,
    TableRowBody {
        cells: &'a [TableCell],
        index: usize,
    },
    TableRowEnd,
    TableCellBody {
        inlines: &'a [Inline],
        index: usize,
    },
    TableCellEnd,
    DefinitionListBody {
        items: &'a [DefinitionItem],
        index: usize,
        phase: DefPhase,
    },
    DefinitionListEnd,
    DefinitionTermBody {
        inlines: &'a [Inline],
        index: usize,
    },
    DefinitionTermEnd,
    DefinitionDescBody {
        inlines: &'a [Inline],
        index: usize,
    },
    DefinitionDescEnd,
    HeadingBody {
        inlines: &'a [Inline],
        index: usize,
    },
    // Inline frames
    InlineList {
        inlines: &'a [Inline],
        index: usize,
    },
    BoldEnd,
    ItalicEnd,
    UnderlineEnd,
    StrikethroughEnd,
    SuperscriptEnd,
    SubscriptEnd,
    LinkEnd,
}

#[derive(Clone, Copy)]
enum DefPhase {
    Term,
    Desc,
    Next,
}

/// Pull-based event iterator over a DokuWiki document.
pub struct EventIter<'a> {
    stack: Vec<Frame<'a>>,
}

impl<'a> EventIter<'a> {
    pub fn new(doc: &'a DokuwikiDoc) -> Self {
        EventIter {
            stack: vec![Frame::BlockList {
                blocks: &doc.blocks,
                index: 0,
            }],
        }
    }
}

impl<'a> Iterator for EventIter<'a> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Event<'a>> {
        loop {
            let frame = self.stack.last_mut()?;

            match frame {
                Frame::BlockList { blocks, index } => {
                    if *index >= blocks.len() {
                        self.stack.pop();
                        continue;
                    }
                    let block = &blocks[*index];
                    *index += 1;
                    match block {
                        Block::Paragraph { inlines, .. } => {
                            self.stack.push(Frame::ParagraphEnd);
                            self.stack.push(Frame::ParagraphBody {
                                inlines,
                                index: 0,
                            });
                            return Some(Event::StartParagraph);
                        }
                        Block::Heading { level, inlines, .. } => {
                            self.stack.push(Frame::HeadingEnd);
                            self.stack.push(Frame::HeadingBody {
                                inlines,
                                index: 0,
                            });
                            return Some(Event::StartHeading { level: *level });
                        }
                        Block::CodeBlock {
                            language, content, ..
                        } => {
                            return Some(Event::CodeBlock {
                                language: language.clone(),
                                content: Cow::Borrowed(content),
                            });
                        }
                        Block::FileBlock {
                            language,
                            filename,
                            content,
                            ..
                        } => {
                            return Some(Event::FileBlock {
                                language: language.clone(),
                                filename: filename.clone(),
                                content: Cow::Borrowed(content),
                            });
                        }
                        Block::Blockquote { children, .. } => {
                            self.stack.push(Frame::BlockquoteEnd);
                            self.stack.push(Frame::BlockquoteBody {
                                children,
                                index: 0,
                            });
                            return Some(Event::StartBlockquote);
                        }
                        Block::List {
                            ordered, items, ..
                        } => {
                            self.stack.push(Frame::ListEnd);
                            self.stack.push(Frame::ListBody { items, index: 0 });
                            return Some(Event::StartList { ordered: *ordered });
                        }
                        Block::Table { rows, .. } => {
                            self.stack.push(Frame::TableEnd);
                            self.stack.push(Frame::TableBody { rows, index: 0 });
                            return Some(Event::StartTable);
                        }
                        Block::DefinitionList { items, .. } => {
                            self.stack.push(Frame::DefinitionListEnd);
                            self.stack.push(Frame::DefinitionListBody {
                                items,
                                index: 0,
                                phase: DefPhase::Term,
                            });
                            return Some(Event::StartDefinitionList);
                        }
                        Block::HorizontalRule(_) => {
                            return Some(Event::HorizontalRule);
                        }
                        Block::RawBlock {
                            format, content, ..
                        } => {
                            return Some(Event::RawBlock {
                                format: format.clone(),
                                content: content.clone(),
                            });
                        }
                        Block::Macro { name, .. } => {
                            return Some(Event::Macro { name: name.clone() });
                        }
                    }
                }

                Frame::ParagraphBody { inlines, index } => {
                    if *index >= inlines.len() {
                        self.stack.pop();
                        continue;
                    }
                    let inline = &inlines[*index];
                    *index += 1;
                    return Some(emit_inline(inline, &mut self.stack));
                }
                Frame::ParagraphEnd => {
                    self.stack.pop();
                    return Some(Event::EndParagraph);
                }

                Frame::HeadingBody { inlines, index } => {
                    if *index >= inlines.len() {
                        self.stack.pop();
                        continue;
                    }
                    let inline = &inlines[*index];
                    *index += 1;
                    return Some(emit_inline(inline, &mut self.stack));
                }
                Frame::HeadingEnd => {
                    self.stack.pop();
                    return Some(Event::EndHeading);
                }

                Frame::BlockquoteBody { children, index } => {
                    if *index >= children.len() {
                        self.stack.pop();
                        continue;
                    }
                    // Re-use BlockList for children
                    let remaining = &children[*index..];
                    *index = children.len();
                    self.stack.push(Frame::BlockList {
                        blocks: remaining,
                        index: 0,
                    });
                    continue;
                }
                Frame::BlockquoteEnd => {
                    self.stack.pop();
                    return Some(Event::EndBlockquote);
                }

                Frame::ListBody { items, index } => {
                    if *index >= items.len() {
                        self.stack.pop();
                        continue;
                    }
                    let item = &items[*index];
                    *index += 1;
                    self.stack.push(Frame::ListItemEnd);
                    self.stack.push(Frame::ListItemBody {
                        inlines: &item.inlines,
                        inline_index: 0,
                        children: &item.children,
                        child_index: 0,
                    });
                    return Some(Event::StartListItem);
                }
                Frame::ListEnd => {
                    self.stack.pop();
                    return Some(Event::EndList);
                }

                Frame::ListItemBody {
                    inlines,
                    inline_index,
                    children,
                    child_index,
                } => {
                    if *inline_index < inlines.len() {
                        let inline = &inlines[*inline_index];
                        *inline_index += 1;
                        return Some(emit_inline(inline, &mut self.stack));
                    }
                    if *child_index < children.len() {
                        let remaining = &children[*child_index..];
                        *child_index = children.len();
                        self.stack.push(Frame::BlockList {
                            blocks: remaining,
                            index: 0,
                        });
                        continue;
                    }
                    self.stack.pop();
                    continue;
                }
                Frame::ListItemEnd => {
                    self.stack.pop();
                    return Some(Event::EndListItem);
                }

                Frame::TableBody { rows, index } => {
                    if *index >= rows.len() {
                        self.stack.pop();
                        continue;
                    }
                    let row = &rows[*index];
                    *index += 1;
                    self.stack.push(Frame::TableRowEnd);
                    self.stack.push(Frame::TableRowBody {
                        cells: &row.cells,
                        index: 0,
                    });
                    return Some(Event::StartTableRow {
                        is_header: row.is_header,
                    });
                }
                Frame::TableEnd => {
                    self.stack.pop();
                    return Some(Event::EndTable);
                }

                Frame::TableRowBody { cells, index } => {
                    if *index >= cells.len() {
                        self.stack.pop();
                        continue;
                    }
                    let cell = &cells[*index];
                    *index += 1;
                    self.stack.push(Frame::TableCellEnd);
                    self.stack.push(Frame::TableCellBody {
                        inlines: &cell.inlines,
                        index: 0,
                    });
                    return Some(Event::StartTableCell);
                }
                Frame::TableRowEnd => {
                    self.stack.pop();
                    return Some(Event::EndTableRow);
                }

                Frame::TableCellBody { inlines, index } => {
                    if *index >= inlines.len() {
                        self.stack.pop();
                        continue;
                    }
                    let inline = &inlines[*index];
                    *index += 1;
                    return Some(emit_inline(inline, &mut self.stack));
                }
                Frame::TableCellEnd => {
                    self.stack.pop();
                    return Some(Event::EndTableCell);
                }

                Frame::DefinitionListBody {
                    items,
                    index,
                    phase,
                } => {
                    if *index >= items.len() {
                        self.stack.pop();
                        continue;
                    }
                    let item = &items[*index];
                    match phase {
                        DefPhase::Term => {
                            *phase = DefPhase::Desc;
                            self.stack.push(Frame::DefinitionTermEnd);
                            self.stack.push(Frame::DefinitionTermBody {
                                inlines: &item.term,
                                index: 0,
                            });
                            return Some(Event::StartDefinitionTerm);
                        }
                        DefPhase::Desc => {
                            *phase = DefPhase::Next;
                            self.stack.push(Frame::DefinitionDescEnd);
                            self.stack.push(Frame::DefinitionDescBody {
                                inlines: &item.desc,
                                index: 0,
                            });
                            return Some(Event::StartDefinitionDesc);
                        }
                        DefPhase::Next => {
                            *index += 1;
                            *phase = DefPhase::Term;
                            continue;
                        }
                    }
                }
                Frame::DefinitionListEnd => {
                    self.stack.pop();
                    return Some(Event::EndDefinitionList);
                }

                Frame::DefinitionTermBody { inlines, index } => {
                    if *index >= inlines.len() {
                        self.stack.pop();
                        continue;
                    }
                    let inline = &inlines[*index];
                    *index += 1;
                    return Some(emit_inline(inline, &mut self.stack));
                }
                Frame::DefinitionTermEnd => {
                    self.stack.pop();
                    return Some(Event::EndDefinitionTerm);
                }

                Frame::DefinitionDescBody { inlines, index } => {
                    if *index >= inlines.len() {
                        self.stack.pop();
                        continue;
                    }
                    let inline = &inlines[*index];
                    *index += 1;
                    return Some(emit_inline(inline, &mut self.stack));
                }
                Frame::DefinitionDescEnd => {
                    self.stack.pop();
                    return Some(Event::EndDefinitionDesc);
                }

                Frame::InlineList { inlines, index } => {
                    if *index >= inlines.len() {
                        self.stack.pop();
                        continue;
                    }
                    let inline = &inlines[*index];
                    *index += 1;
                    return Some(emit_inline(inline, &mut self.stack));
                }

                Frame::BoldEnd => {
                    self.stack.pop();
                    return Some(Event::EndBold);
                }
                Frame::ItalicEnd => {
                    self.stack.pop();
                    return Some(Event::EndItalic);
                }
                Frame::UnderlineEnd => {
                    self.stack.pop();
                    return Some(Event::EndUnderline);
                }
                Frame::StrikethroughEnd => {
                    self.stack.pop();
                    return Some(Event::EndStrikethrough);
                }
                Frame::SuperscriptEnd => {
                    self.stack.pop();
                    return Some(Event::EndSuperscript);
                }
                Frame::SubscriptEnd => {
                    self.stack.pop();
                    return Some(Event::EndSubscript);
                }
                Frame::LinkEnd => {
                    self.stack.pop();
                    return Some(Event::EndLink);
                }
            }
        }
    }
}

fn emit_inline<'a>(inline: &'a Inline, stack: &mut Vec<Frame<'a>>) -> Event<'a> {
    match inline {
        Inline::Text(s, _) => Event::Text(Cow::Borrowed(s)),
        Inline::Bold(children, _) => {
            stack.push(Frame::BoldEnd);
            stack.push(Frame::InlineList {
                inlines: children,
                index: 0,
            });
            Event::StartBold
        }
        Inline::Italic(children, _) => {
            stack.push(Frame::ItalicEnd);
            stack.push(Frame::InlineList {
                inlines: children,
                index: 0,
            });
            Event::StartItalic
        }
        Inline::Underline(children, _) => {
            stack.push(Frame::UnderlineEnd);
            stack.push(Frame::InlineList {
                inlines: children,
                index: 0,
            });
            Event::StartUnderline
        }
        Inline::Strikethrough(children, _) => {
            stack.push(Frame::StrikethroughEnd);
            stack.push(Frame::InlineList {
                inlines: children,
                index: 0,
            });
            Event::StartStrikethrough
        }
        Inline::Superscript(children, _) => {
            stack.push(Frame::SuperscriptEnd);
            stack.push(Frame::InlineList {
                inlines: children,
                index: 0,
            });
            Event::StartSuperscript
        }
        Inline::Subscript(children, _) => {
            stack.push(Frame::SubscriptEnd);
            stack.push(Frame::InlineList {
                inlines: children,
                index: 0,
            });
            Event::StartSubscript
        }
        Inline::Code(s, _) => Event::InlineCode(Cow::Borrowed(s)),
        Inline::Nowiki(s, _) => Event::Nowiki(Cow::Borrowed(s)),
        Inline::Link { url, children, .. } => {
            stack.push(Frame::LinkEnd);
            stack.push(Frame::InlineList {
                inlines: children,
                index: 0,
            });
            Event::StartLink { url: url.clone() }
        }
        Inline::Image { url, alt, .. } => Event::InlineImage {
            url: url.clone(),
            alt: alt.clone(),
        },
        Inline::FootnoteRef { content, .. } => Event::FootnoteRef {
            content: content.clone(),
        },
        Inline::LineBreak(_) => Event::LineBreak,
        Inline::SoftBreak(_) => Event::SoftBreak,
    }
}

/// Parse `input` and return a streaming iterator of events.
pub fn events(input: &str) -> InputEventIter<'_> {
    InputEventIter::new(input)
}

/// Event iterator that parses from a raw input string (owns the parsed doc).
pub struct InputEventIter<'a> {
    _input: &'a str,
    _doc: DokuwikiDoc,
    inner: Option<EventIter<'static>>,
}

impl<'a> InputEventIter<'a> {
    fn new(input: &'a str) -> Self {
        let (doc, _) = crate::parse::parse(input);
        // Safety: we store the doc and create a self-referential iterator.
        // The doc is owned by this struct and lives as long as the iterator.
        // We use transmute to extend the lifetime.
        let iter = unsafe {
            let doc_ref: &DokuwikiDoc = &doc;
            let iter = EventIter::new(doc_ref);
            std::mem::transmute::<EventIter<'_>, EventIter<'static>>(iter)
        };
        InputEventIter {
            _input: input,
            _doc: doc,
            inner: Some(iter),
        }
    }
}

impl Iterator for InputEventIter<'_> {
    type Item = OwnedEvent;

    fn next(&mut self) -> Option<OwnedEvent> {
        self.inner.as_mut()?.next().map(|e| e.into_owned())
    }
}

// Safety: InputEventIter borrows doc internally. The doc field must not be
// moved or dropped while inner is alive. This is guaranteed by struct layout.
// The inner iterator only borrows from self.doc which is pinned by being in
// the same struct.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_events_heading() {
        let evs: Vec<_> = events("====== Hello ======").collect();
        assert!(evs
            .iter()
            .any(|e| matches!(e, OwnedEvent::StartHeading { level: 1 })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndHeading)));
    }

    #[test]
    fn test_events_paragraph() {
        let evs: Vec<_> = events("Hello world").collect();
        assert!(evs
            .iter()
            .any(|e| matches!(e, OwnedEvent::StartParagraph)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndParagraph)));
        assert!(evs
            .iter()
            .any(|e| matches!(e, OwnedEvent::Text(t) if t == "Hello world")));
    }

    #[test]
    fn test_events_bold() {
        let evs: Vec<_> = events("**bold**").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartBold)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndBold)));
    }

    #[test]
    fn test_events_code_block() {
        let evs: Vec<_> = events("<code rust>\nfn main() {}\n</code>").collect();
        assert!(evs.iter().any(
            |e| matches!(e, OwnedEvent::CodeBlock { language: Some(l), .. } if l == "rust")
        ));
    }

    #[test]
    fn test_events_list() {
        let evs: Vec<_> = events("  * item 1\n  * item 2").collect();
        assert!(evs
            .iter()
            .any(|e| matches!(e, OwnedEvent::StartList { ordered: false })));
        assert_eq!(
            evs.iter()
                .filter(|e| matches!(e, OwnedEvent::StartListItem))
                .count(),
            2
        );
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndList)));
    }

    #[test]
    fn test_events_table() {
        let evs: Vec<_> = events("^ Name ^ Age ^\n| Alice | 30 |").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartTable)));
        assert!(evs
            .iter()
            .any(|e| matches!(e, OwnedEvent::StartTableRow { is_header: true })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndTable)));
    }

    #[test]
    fn test_events_horizontal_rule() {
        let evs: Vec<_> = events("----").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::HorizontalRule)));
    }
}
