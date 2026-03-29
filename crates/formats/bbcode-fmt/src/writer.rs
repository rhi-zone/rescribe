//! Streaming BBCode writer — converts a stream of events to BBCode text.
//!
//! This implementation buffers all events, reconstructs the AST, then emits.
//!
//! # Example
//! ```no_run
//! use bbcode_fmt::writer::Writer;
//! use bbcode_fmt::OwnedEvent;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! w.write_event(OwnedEvent::StartParagraph);
//! w.write_event(OwnedEvent::StartBold);
//! w.write_event(OwnedEvent::Text("Hello".to_string().into()));
//! w.write_event(OwnedEvent::EndBold);
//! w.write_event(OwnedEvent::EndParagraph);
//! let bytes = w.finish();
//! ```

use crate::ast::*;
use crate::events::OwnedEvent;
use std::io::Write;

/// Streaming BBCode writer.
///
/// Feed events with [`write_event`](Writer::write_event), then call
/// [`finish`](Writer::finish) to flush BBCode text to the underlying sink and
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

    /// Flush all buffered events as BBCode text and return the sink.
    pub fn finish(mut self) -> W {
        let doc = events_to_doc(std::mem::take(&mut self.events));
        let text = crate::emit::emit(&doc);
        let _ = self.sink.write_all(text.as_bytes());
        self.sink
    }
}

// ── Event → AST reconstruction ────────────────────────────────────────────────

fn events_to_doc(events: Vec<OwnedEvent>) -> BbcodeDoc {
    let mut builder = DocBuilder::new();
    for event in events {
        builder.process(event);
    }
    builder.finish()
}

enum Frame {
    Document {
        blocks: Vec<Block>,
    },
    Paragraph {
        inlines: Vec<Inline>,
    },
    Blockquote {
        author: Option<String>,
        blocks: Vec<Block>,
    },
    List {
        ordered: bool,
        items: Vec<Vec<Inline>>,
    },
    ListItem {
        inlines: Vec<Inline>,
    },
    Table {
        rows: Vec<TableRow>,
    },
    TableRow {
        cells: Vec<(bool, Vec<Inline>)>,
    },
    TableCell {
        is_header: bool,
        inlines: Vec<Inline>,
    },
    Heading {
        level: u8,
        inlines: Vec<Inline>,
    },
    Alignment {
        kind: AlignKind,
        blocks: Vec<Block>,
    },
    Spoiler {
        blocks: Vec<Block>,
    },
    Indent {
        blocks: Vec<Block>,
    },
    // Inline frames
    Bold {
        inlines: Vec<Inline>,
    },
    Italic {
        inlines: Vec<Inline>,
    },
    Underline {
        inlines: Vec<Inline>,
    },
    Strikethrough {
        inlines: Vec<Inline>,
    },
    Subscript {
        inlines: Vec<Inline>,
    },
    Superscript {
        inlines: Vec<Inline>,
    },
    Link {
        url: String,
        inlines: Vec<Inline>,
    },
    Color {
        value: String,
        inlines: Vec<Inline>,
    },
    Size {
        value: String,
        inlines: Vec<Inline>,
    },
    Font {
        name: String,
        inlines: Vec<Inline>,
    },
    Email {
        addr: String,
        inlines: Vec<Inline>,
    },
    Span {
        attr: String,
        value: String,
        inlines: Vec<Inline>,
    },
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

    fn push_inline(&mut self, inline: Inline) {
        match self.stack.last_mut() {
            Some(Frame::Paragraph { inlines })
            | Some(Frame::ListItem { inlines })
            | Some(Frame::Heading { inlines, .. })
            | Some(Frame::Bold { inlines })
            | Some(Frame::Italic { inlines })
            | Some(Frame::Underline { inlines })
            | Some(Frame::Strikethrough { inlines })
            | Some(Frame::Subscript { inlines })
            | Some(Frame::Superscript { inlines })
            | Some(Frame::Link { inlines, .. })
            | Some(Frame::Color { inlines, .. })
            | Some(Frame::Size { inlines, .. })
            | Some(Frame::Font { inlines, .. })
            | Some(Frame::Email { inlines, .. })
            | Some(Frame::Span { inlines, .. })
            | Some(Frame::TableCell { inlines, .. }) => {
                inlines.push(inline);
            }
            _ => {}
        }
    }

    fn push_block(&mut self, block: Block) {
        match self.stack.last_mut() {
            Some(Frame::Document { blocks })
            | Some(Frame::Blockquote { blocks, .. })
            | Some(Frame::Alignment { blocks, .. })
            | Some(Frame::Spoiler { blocks })
            | Some(Frame::Indent { blocks }) => {
                blocks.push(block);
            }
            _ => {}
        }
    }

    #[allow(clippy::too_many_lines)]
    fn process(&mut self, event: OwnedEvent) {
        match event {
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
            OwnedEvent::StartBlockquote { author } => {
                self.stack.push(Frame::Blockquote {
                    author,
                    blocks: vec![],
                });
            }
            OwnedEvent::EndBlockquote => {
                if let Some(Frame::Blockquote { author, blocks }) = self.stack.pop() {
                    self.push_block(Block::Blockquote {
                        author,
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
                self.stack.push(Frame::ListItem { inlines: vec![] });
            }
            OwnedEvent::EndListItem => {
                if let Some(Frame::ListItem { inlines }) = self.stack.pop()
                    && let Some(Frame::List { items, .. }) = self.stack.last_mut()
                {
                    items.push(inlines);
                }
            }
            OwnedEvent::CodeBlock { language, content } => {
                self.push_block(Block::CodeBlock {
                    language,
                    content: content.into_owned(),
                    span: Span::NONE,
                });
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
            OwnedEvent::StartTableRow => {
                self.stack.push(Frame::TableRow { cells: vec![] });
            }
            OwnedEvent::EndTableRow => {
                if let Some(Frame::TableRow { cells }) = self.stack.pop()
                    && let Some(Frame::Table { rows }) = self.stack.last_mut()
                {
                    rows.push(TableRow {
                        cells,
                        span: Span::NONE,
                    });
                }
            }
            OwnedEvent::StartTableCell { is_header } => {
                self.stack.push(Frame::TableCell {
                    is_header,
                    inlines: vec![],
                });
            }
            OwnedEvent::EndTableCell => {
                if let Some(Frame::TableCell { is_header, inlines }) = self.stack.pop()
                    && let Some(Frame::TableRow { cells }) = self.stack.last_mut()
                {
                    cells.push((is_header, inlines));
                }
            }
            OwnedEvent::HorizontalRule => {
                self.push_block(Block::HorizontalRule { span: Span::NONE });
            }
            OwnedEvent::StartHeading { level } => {
                self.stack.push(Frame::Heading {
                    level,
                    inlines: vec![],
                });
            }
            OwnedEvent::EndHeading => {
                if let Some(Frame::Heading { level, inlines }) = self.stack.pop() {
                    self.push_block(Block::Heading {
                        level,
                        children: inlines,
                        span: Span::NONE,
                    });
                }
            }
            OwnedEvent::StartAlignment { kind } => {
                self.stack.push(Frame::Alignment {
                    kind,
                    blocks: vec![],
                });
            }
            OwnedEvent::EndAlignment => {
                if let Some(Frame::Alignment { kind, blocks }) = self.stack.pop() {
                    self.push_block(Block::Alignment {
                        kind,
                        children: blocks,
                        span: Span::NONE,
                    });
                }
            }
            OwnedEvent::StartSpoiler => {
                self.stack.push(Frame::Spoiler { blocks: vec![] });
            }
            OwnedEvent::EndSpoiler => {
                if let Some(Frame::Spoiler { blocks }) = self.stack.pop() {
                    self.push_block(Block::Spoiler {
                        children: blocks,
                        span: Span::NONE,
                    });
                }
            }
            OwnedEvent::Preformatted { content } => {
                self.push_block(Block::Preformatted {
                    content: content.into_owned(),
                    span: Span::NONE,
                });
            }
            OwnedEvent::StartIndent => {
                self.stack.push(Frame::Indent { blocks: vec![] });
            }
            OwnedEvent::EndIndent => {
                if let Some(Frame::Indent { blocks }) = self.stack.pop() {
                    self.push_block(Block::Indent {
                        children: blocks,
                        span: Span::NONE,
                    });
                }
            }
            // ── Inline events ───────────────────────────────────────────────
            OwnedEvent::Text(cow) => {
                self.push_inline(Inline::Text(cow.into_owned(), Span::NONE));
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
            OwnedEvent::StartSubscript => {
                self.stack.push(Frame::Subscript { inlines: vec![] });
            }
            OwnedEvent::EndSubscript => {
                if let Some(Frame::Subscript { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Subscript(inlines, Span::NONE));
                }
            }
            OwnedEvent::StartSuperscript => {
                self.stack
                    .push(Frame::Superscript { inlines: vec![] });
            }
            OwnedEvent::EndSuperscript => {
                if let Some(Frame::Superscript { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Superscript(inlines, Span::NONE));
                }
            }
            OwnedEvent::InlineCode(cow) => {
                self.push_inline(Inline::Code(cow.into_owned(), Span::NONE));
            }
            OwnedEvent::StartLink { url } => {
                self.stack.push(Frame::Link {
                    url,
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
            OwnedEvent::InlineImage {
                url,
                width,
                height,
            } => {
                self.push_inline(Inline::Image {
                    url,
                    width,
                    height,
                    span: Span::NONE,
                });
            }
            OwnedEvent::StartColor { value } => {
                self.stack.push(Frame::Color {
                    value,
                    inlines: vec![],
                });
            }
            OwnedEvent::EndColor => {
                if let Some(Frame::Color { value, inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Color {
                        value,
                        children: inlines,
                        span: Span::NONE,
                    });
                }
            }
            OwnedEvent::StartSize { value } => {
                self.stack.push(Frame::Size {
                    value,
                    inlines: vec![],
                });
            }
            OwnedEvent::EndSize => {
                if let Some(Frame::Size { value, inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Size {
                        value,
                        children: inlines,
                        span: Span::NONE,
                    });
                }
            }
            OwnedEvent::StartFont { name } => {
                self.stack.push(Frame::Font {
                    name,
                    inlines: vec![],
                });
            }
            OwnedEvent::EndFont => {
                if let Some(Frame::Font { name, inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Font {
                        name,
                        children: inlines,
                        span: Span::NONE,
                    });
                }
            }
            OwnedEvent::StartEmail { addr } => {
                self.stack.push(Frame::Email {
                    addr,
                    inlines: vec![],
                });
            }
            OwnedEvent::EndEmail => {
                if let Some(Frame::Email { addr, inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Email {
                        addr,
                        children: inlines,
                        span: Span::NONE,
                    });
                }
            }
            OwnedEvent::Noparse(cow) => {
                self.push_inline(Inline::Noparse(cow.into_owned(), Span::NONE));
            }
            OwnedEvent::StartSpan { attr, value } => {
                self.stack.push(Frame::Span {
                    attr,
                    value,
                    inlines: vec![],
                });
            }
            OwnedEvent::EndSpan => {
                if let Some(Frame::Span {
                    attr,
                    value,
                    inlines,
                }) = self.stack.pop()
                {
                    self.push_inline(Inline::Span {
                        attr,
                        value,
                        children: inlines,
                        span: Span::NONE,
                    });
                }
            }
        }
    }

    fn finish(mut self) -> BbcodeDoc {
        let blocks = match self.stack.pop() {
            Some(Frame::Document { blocks }) => blocks,
            _ => Vec::new(),
        };
        BbcodeDoc {
            blocks,
            span: Span::NONE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::OwnedEvent;
    use std::borrow::Cow;

    #[test]
    fn test_writer_roundtrip() {
        let input = "[b]bold[/b]";
        let events: Vec<OwnedEvent> = crate::events::events(input)
            .map(|e| e.into_owned())
            .collect();

        let mut w = Writer::new(Vec::<u8>::new());
        for ev in events {
            w.write_event(ev);
        }
        let bytes = w.finish();
        let output = String::from_utf8(bytes).unwrap();
        assert!(output.contains("[b]bold[/b]"));
    }

    #[test]
    fn test_writer_complex() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedEvent::StartParagraph);
        w.write_event(OwnedEvent::StartBold);
        w.write_event(OwnedEvent::Text(Cow::Owned("hello".to_string())));
        w.write_event(OwnedEvent::EndBold);
        w.write_event(OwnedEvent::EndParagraph);
        let bytes = w.finish();
        let output = String::from_utf8(bytes).unwrap();
        assert!(output.contains("[b]hello[/b]"));
    }
}
