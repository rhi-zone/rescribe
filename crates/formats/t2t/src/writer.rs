//! Streaming txt2tags writer — converts a stream of events to t2t text.
//!
//! This implementation buffers all events, reconstructs the AST, then emits.
//!
//! # Example
//! ```no_run
//! use t2t::writer::Writer;
//! use t2t::OwnedEvent;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! w.write_event(OwnedEvent::StartHeading { level: 1, numbered: false });
//! w.write_event(OwnedEvent::Text("Hello".to_string().into()));
//! w.write_event(OwnedEvent::EndHeading);
//! let bytes = w.finish();
//! ```

use crate::ast::*;
use crate::events::OwnedEvent;
use std::io::Write;

/// Streaming txt2tags writer.
///
/// Feed events with [`write_event`](Writer::write_event), then call
/// [`finish`](Writer::finish) to flush t2t text to the underlying sink and
/// recover it.
pub struct Writer<W: Write> {
    sink: W,
    events: Vec<OwnedEvent>,
}

impl<W: Write> Writer<W> {
    pub fn new(sink: W) -> Self {
        Writer {
            sink,
            events: Vec::new(),
        }
    }

    /// Feed one event to the writer.
    pub fn write_event(&mut self, event: OwnedEvent) {
        self.events.push(event);
    }

    /// Flush all buffered events as t2t text and return the sink.
    pub fn finish(mut self) -> W {
        let doc = events_to_doc(std::mem::take(&mut self.events));
        let text = crate::emit::emit(&doc);
        let _ = self.sink.write_all(text.as_bytes());
        self.sink
    }
}

// ── Event -> AST reconstruction ──────────────────────────────────────────────

fn events_to_doc(events: Vec<OwnedEvent>) -> T2tDoc {
    let mut builder = DocBuilder::new();
    for event in events {
        builder.process(event);
    }
    builder.finish()
}

enum Frame {
    Document { blocks: Vec<Block> },
    Paragraph { inlines: Vec<Inline> },
    Heading { level: u8, numbered: bool, inlines: Vec<Inline> },
    Blockquote { blocks: Vec<Block> },
    List { ordered: bool, items: Vec<Vec<Block>> },
    ListItem { blocks: Vec<Block> },
    Table { rows: Vec<TableRow> },
    TableRow { cells: Vec<Vec<Inline>>, header: bool },
    TableCell { inlines: Vec<Inline> },
    DefinitionList { items: Vec<(Vec<Inline>, Vec<Block>)> },
    DefinitionTerm { inlines: Vec<Inline> },
    DefinitionDesc { blocks: Vec<Block> },
    // Inline spans
    Bold { inlines: Vec<Inline> },
    Italic { inlines: Vec<Inline> },
    Underline { inlines: Vec<Inline> },
    Strikethrough { inlines: Vec<Inline> },
    Link { url: String, inlines: Vec<Inline> },
}

struct DocBuilder {
    stack: Vec<Frame>,
}

impl DocBuilder {
    fn new() -> Self {
        DocBuilder {
            stack: vec![Frame::Document { blocks: vec![] }],
        }
    }

    fn process(&mut self, event: OwnedEvent) {
        match event {
            // ── Block open/close ───────────────────────────────────────────────
            OwnedEvent::StartDocument => {}
            OwnedEvent::EndDocument => {}
            OwnedEvent::StartParagraph => {
                self.stack.push(Frame::Paragraph { inlines: vec![] });
            }
            OwnedEvent::EndParagraph => {
                if let Some(Frame::Paragraph { inlines }) = self.stack.pop() {
                    self.push_block(Block::Paragraph {
                        inlines,
                        span: Span::NONE,
                    });
                }
            }
            OwnedEvent::StartHeading { level, numbered } => {
                self.stack.push(Frame::Heading {
                    level,
                    numbered,
                    inlines: vec![],
                });
            }
            OwnedEvent::EndHeading => {
                if let Some(Frame::Heading {
                    level,
                    numbered,
                    inlines,
                }) = self.stack.pop()
                {
                    self.push_block(Block::Heading {
                        level,
                        numbered,
                        inlines,
                        span: Span::NONE,
                    });
                }
            }
            OwnedEvent::StartBlockquote => {
                self.stack.push(Frame::Blockquote { blocks: vec![] });
            }
            OwnedEvent::EndBlockquote => {
                if let Some(Frame::Blockquote { blocks }) = self.stack.pop() {
                    self.push_block(Block::Blockquote {
                        children: blocks,
                        span: Span::NONE,
                    });
                }
            }
            OwnedEvent::StartList { ordered } => {
                self.stack.push(Frame::List {
                    ordered,
                    items: vec![],
                });
            }
            OwnedEvent::EndList => {
                if let Some(Frame::List { ordered, items }) = self.stack.pop() {
                    self.push_block(Block::List {
                        ordered,
                        items,
                        span: Span::NONE,
                    });
                }
            }
            OwnedEvent::StartListItem => {
                self.stack.push(Frame::ListItem { blocks: vec![] });
            }
            OwnedEvent::EndListItem => {
                if let Some(Frame::ListItem { blocks }) = self.stack.pop() {
                    if let Some(Frame::List { items, .. }) = self.stack.last_mut() {
                        items.push(blocks);
                    }
                }
            }
            OwnedEvent::CodeBlock { content } => {
                self.push_block(Block::CodeBlock {
                    content: content.into_owned(),
                    span: Span::NONE,
                });
            }
            OwnedEvent::RawBlock { content } => {
                self.push_block(Block::RawBlock {
                    content: content.into_owned(),
                    span: Span::NONE,
                });
            }
            OwnedEvent::HorizontalRule => {
                self.push_block(Block::HorizontalRule { span: Span::NONE });
            }
            OwnedEvent::StartTable => {
                self.stack.push(Frame::Table { rows: vec![] });
            }
            OwnedEvent::EndTable => {
                if let Some(Frame::Table { rows }) = self.stack.pop() {
                    self.push_block(Block::Table {
                        rows,
                        span: Span::NONE,
                    });
                }
            }
            OwnedEvent::StartTableRow { header } => {
                self.stack.push(Frame::TableRow {
                    cells: vec![],
                    header,
                });
            }
            OwnedEvent::EndTableRow => {
                if let Some(Frame::TableRow { cells, header }) = self.stack.pop() {
                    if let Some(Frame::Table { rows }) = self.stack.last_mut() {
                        rows.push(TableRow {
                            cells,
                            is_header: header,
                            span: Span::NONE,
                        });
                    }
                }
            }
            OwnedEvent::StartTableCell => {
                self.stack.push(Frame::TableCell { inlines: vec![] });
            }
            OwnedEvent::EndTableCell => {
                if let Some(Frame::TableCell { inlines }) = self.stack.pop() {
                    if let Some(Frame::TableRow { cells, .. }) = self.stack.last_mut() {
                        cells.push(inlines);
                    }
                }
            }
            OwnedEvent::StartDefinitionList => {
                self.stack
                    .push(Frame::DefinitionList { items: vec![] });
            }
            OwnedEvent::EndDefinitionList => {
                if let Some(Frame::DefinitionList { items }) = self.stack.pop() {
                    self.push_block(Block::DefinitionList {
                        items,
                        span: Span::NONE,
                    });
                }
            }
            OwnedEvent::StartDefinitionTerm => {
                self.stack
                    .push(Frame::DefinitionTerm { inlines: vec![] });
            }
            OwnedEvent::EndDefinitionTerm => {
                if let Some(Frame::DefinitionTerm { inlines }) = self.stack.pop() {
                    if let Some(Frame::DefinitionList { items }) = self.stack.last_mut() {
                        items.push((inlines, vec![]));
                    }
                }
            }
            OwnedEvent::StartDefinitionDesc => {
                self.stack
                    .push(Frame::DefinitionDesc { blocks: vec![] });
            }
            OwnedEvent::EndDefinitionDesc => {
                if let Some(Frame::DefinitionDesc { blocks }) = self.stack.pop() {
                    if let Some(Frame::DefinitionList { items }) = self.stack.last_mut() {
                        if let Some(last) = items.last_mut() {
                            last.1 = blocks;
                        }
                    }
                }
            }

            // ── Inline events ──────────────────────────────────────────────────
            OwnedEvent::Text(cow) => {
                self.push_inline(Inline::Text(cow.into_owned(), Span::NONE));
            }
            OwnedEvent::LineBreak => {
                self.push_inline(Inline::LineBreak(Span::NONE));
            }
            OwnedEvent::SoftBreak => {
                self.push_inline(Inline::SoftBreak(Span::NONE));
            }
            OwnedEvent::StartBold => {
                self.stack.push(Frame::Bold { inlines: vec![] });
            }
            OwnedEvent::EndBold => {
                if let Some(Frame::Bold { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Bold(inlines, Span::NONE));
                }
            }
            OwnedEvent::StartItalic => {
                self.stack.push(Frame::Italic { inlines: vec![] });
            }
            OwnedEvent::EndItalic => {
                if let Some(Frame::Italic { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Italic(inlines, Span::NONE));
                }
            }
            OwnedEvent::StartUnderline => {
                self.stack.push(Frame::Underline { inlines: vec![] });
            }
            OwnedEvent::EndUnderline => {
                if let Some(Frame::Underline { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Underline(inlines, Span::NONE));
                }
            }
            OwnedEvent::StartStrikethrough => {
                self.stack
                    .push(Frame::Strikethrough { inlines: vec![] });
            }
            OwnedEvent::EndStrikethrough => {
                if let Some(Frame::Strikethrough { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Strikethrough(inlines, Span::NONE));
                }
            }
            OwnedEvent::Code(cow) => {
                self.push_inline(Inline::Code(cow.into_owned(), Span::NONE));
            }
            OwnedEvent::Verbatim(cow) => {
                self.push_inline(Inline::Verbatim(cow.into_owned(), Span::NONE));
            }
            OwnedEvent::Tagged(cow) => {
                self.push_inline(Inline::Tagged(cow.into_owned(), Span::NONE));
            }
            OwnedEvent::StartLink { url } => {
                self.stack.push(Frame::Link {
                    url: url.into_owned(),
                    inlines: vec![],
                });
            }
            OwnedEvent::EndLink => {
                if let Some(Frame::Link { url, inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Link {
                        url,
                        children: inlines,
                        span: Span::NONE,
                    });
                }
            }
            OwnedEvent::Image { src } => {
                self.push_inline(Inline::Image {
                    url: src.into_owned(),
                    span: Span::NONE,
                });
            }
        }
    }

    fn push_block(&mut self, block: Block) {
        match self.stack.last_mut() {
            Some(Frame::Document { blocks }) => blocks.push(block),
            Some(Frame::Blockquote { blocks }) => blocks.push(block),
            Some(Frame::ListItem { blocks }) => blocks.push(block),
            Some(Frame::DefinitionDesc { blocks }) => blocks.push(block),
            _ => {}
        }
    }

    fn push_inline(&mut self, inline: Inline) {
        match self.stack.last_mut() {
            Some(Frame::Paragraph { inlines }) => inlines.push(inline),
            Some(Frame::Heading { inlines, .. }) => inlines.push(inline),
            Some(Frame::Bold { inlines }) => inlines.push(inline),
            Some(Frame::Italic { inlines }) => inlines.push(inline),
            Some(Frame::Underline { inlines }) => inlines.push(inline),
            Some(Frame::Strikethrough { inlines }) => inlines.push(inline),
            Some(Frame::Link { inlines, .. }) => inlines.push(inline),
            Some(Frame::TableCell { inlines }) => inlines.push(inline),
            Some(Frame::DefinitionTerm { inlines }) => inlines.push(inline),
            _ => {}
        }
    }

    fn finish(mut self) -> T2tDoc {
        let blocks = match self.stack.pop() {
            Some(Frame::Document { blocks }) => blocks,
            _ => vec![],
        };
        T2tDoc {
            blocks,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;

    #[test]
    fn test_writer_heading() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedEvent::StartHeading {
            level: 1,
            numbered: false,
        });
        w.write_event(OwnedEvent::Text(Cow::Owned("Hello".to_string())));
        w.write_event(OwnedEvent::EndHeading);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("= Hello ="), "got: {s:?}");
    }

    #[test]
    fn test_writer_paragraph() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedEvent::StartParagraph);
        w.write_event(OwnedEvent::Text(Cow::Owned("World".to_string())));
        w.write_event(OwnedEvent::EndParagraph);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("World"), "got: {s:?}");
    }

    #[test]
    fn test_writer_roundtrip_via_events() {
        let input = "= Hello =\n\nA paragraph with **bold** text.\n\n- item one\n- item two\n";
        let evts: Vec<_> = crate::events::events(input).collect();
        let mut w = Writer::new(Vec::<u8>::new());
        for e in evts {
            w.write_event(e);
        }
        let bytes = w.finish();
        let emitted_text = String::from_utf8(bytes).unwrap();
        let (doc_orig, _) = crate::parse::parse(input);
        let (doc_emit, _) = crate::parse::parse(&emitted_text);
        assert_eq!(
            doc_orig.blocks.len(),
            doc_emit.blocks.len(),
            "writer roundtrip block count mismatch"
        );
    }
}
