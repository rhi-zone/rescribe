//! Streaming Haddock writer — converts a stream of events to Haddock text.
//!
//! This implementation buffers all events, reconstructs the AST, then emits.
//!
//! # Example
//! ```no_run
//! use haddock_fmt::writer::Writer;
//! use haddock_fmt::OwnedEvent;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! w.write_event(OwnedEvent::StartHeading { level: 1 });
//! w.write_event(OwnedEvent::Text("Hello".to_string().into()));
//! w.write_event(OwnedEvent::EndHeading);
//! let bytes = w.finish();
//! ```

use crate::ast::*;
use crate::events::OwnedEvent;
use std::io::Write;

/// Streaming Haddock writer.
///
/// Feed events with [`write_event`](Writer::write_event), then call
/// [`finish`](Writer::finish) to flush Haddock text to the underlying sink and
/// recover it.
pub struct Writer<W: Write> {
    sink: W,
    events: Vec<OwnedEvent>,
}

impl<W: Write> Writer<W> {
    pub fn new(sink: W) -> Self {
        Writer { sink, events: Vec::new() }
    }

    /// Feed one event to the writer.
    pub fn write_event(&mut self, event: OwnedEvent) {
        self.events.push(event);
    }

    /// Flush all buffered events as Haddock text and return the sink.
    pub fn finish(mut self) -> W {
        let doc = events_to_doc(std::mem::take(&mut self.events));
        let text = crate::emit::build(&doc);
        let _ = self.sink.write_all(text.as_bytes());
        self.sink
    }
}

// ── Event -> AST reconstruction ──────────────────────────────────────────────

fn events_to_doc(events: Vec<OwnedEvent>) -> HaddockDoc {
    let mut builder = DocBuilder::new();
    for event in events {
        builder.process(event);
    }
    builder.finish()
}

enum Frame {
    Document { blocks: Vec<Block> },
    Paragraph { inlines: Vec<Inline> },
    Heading { level: u8, inlines: Vec<Inline> },
    UnorderedList { items: Vec<Vec<Inline>> },
    OrderedList { items: Vec<Vec<Inline>> },
    ListItem { inlines: Vec<Inline> },
    DefinitionList { items: Vec<(Vec<Inline>, Vec<Inline>)> },
    DefinitionTerm { inlines: Vec<Inline> },
    DefinitionDesc { inlines: Vec<Inline> },
    Blockquote { inlines: Vec<Inline> },
    Property { key: String, name: Option<String>, inlines: Vec<Inline> },
    // Inline spans
    Strong { inlines: Vec<Inline> },
    Emphasis { inlines: Vec<Inline> },
    Link { url: String, text: String, inlines: Vec<Inline> },
}

struct DocBuilder {
    stack: Vec<Frame>,
}

impl DocBuilder {
    fn new() -> Self {
        DocBuilder {
            stack: vec![Frame::Document { blocks: Vec::new() }],
        }
    }

    fn push_inline(&mut self, inline: Inline) {
        for frame in self.stack.iter_mut().rev() {
            match frame {
                Frame::Paragraph { inlines }
                | Frame::Heading { inlines, .. }
                | Frame::ListItem { inlines }
                | Frame::DefinitionTerm { inlines }
                | Frame::DefinitionDesc { inlines }
                | Frame::Blockquote { inlines }
                | Frame::Property { inlines, .. }
                | Frame::Strong { inlines }
                | Frame::Emphasis { inlines }
                | Frame::Link { inlines, .. } => {
                    inlines.push(inline);
                    return;
                }
                _ => {}
            }
        }
    }

    fn push_block(&mut self, block: Block) {
        for frame in self.stack.iter_mut().rev() {
            if let Frame::Document { blocks } = frame {
                blocks.push(block);
                return;
            }
        }
    }

    fn process(&mut self, event: OwnedEvent) {
        match event {
            OwnedEvent::StartParagraph => {
                self.stack.push(Frame::Paragraph { inlines: Vec::new() });
            }
            OwnedEvent::EndParagraph => {
                if let Some(Frame::Paragraph { inlines }) = self.stack.pop() {
                    self.push_block(Block::Paragraph { inlines, span: Span::NONE });
                }
            }
            OwnedEvent::StartHeading { level } => {
                self.stack.push(Frame::Heading { level, inlines: Vec::new() });
            }
            OwnedEvent::EndHeading => {
                if let Some(Frame::Heading { level, inlines }) = self.stack.pop() {
                    self.push_block(Block::Heading { level, inlines, span: Span::NONE });
                }
            }
            OwnedEvent::CodeBlock { content } => {
                self.push_block(Block::CodeBlock {
                    content: content.into_owned(),
                    span: Span::NONE,
                });
            }
            OwnedEvent::AtCodeBlock { content } => {
                self.push_block(Block::AtCodeBlock {
                    content: content.into_owned(),
                    span: Span::NONE,
                });
            }
            OwnedEvent::StartUnorderedList => {
                self.stack.push(Frame::UnorderedList { items: Vec::new() });
            }
            OwnedEvent::EndUnorderedList => {
                if let Some(Frame::UnorderedList { items }) = self.stack.pop() {
                    self.push_block(Block::UnorderedList { items, span: Span::NONE });
                }
            }
            OwnedEvent::StartOrderedList => {
                self.stack.push(Frame::OrderedList { items: Vec::new() });
            }
            OwnedEvent::EndOrderedList => {
                if let Some(Frame::OrderedList { items }) = self.stack.pop() {
                    self.push_block(Block::OrderedList { items, span: Span::NONE });
                }
            }
            OwnedEvent::StartListItem => {
                self.stack.push(Frame::ListItem { inlines: Vec::new() });
            }
            OwnedEvent::EndListItem => {
                if let Some(Frame::ListItem { inlines }) = self.stack.pop() {
                    // Find the parent list and add the item
                    for frame in self.stack.iter_mut().rev() {
                        match frame {
                            Frame::UnorderedList { items } | Frame::OrderedList { items } => {
                                items.push(inlines);
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }
            OwnedEvent::StartDefinitionList => {
                self.stack.push(Frame::DefinitionList { items: Vec::new() });
            }
            OwnedEvent::EndDefinitionList => {
                if let Some(Frame::DefinitionList { items }) = self.stack.pop() {
                    self.push_block(Block::DefinitionList { items, span: Span::NONE });
                }
            }
            OwnedEvent::StartDefinitionTerm => {
                self.stack.push(Frame::DefinitionTerm { inlines: Vec::new() });
            }
            OwnedEvent::EndDefinitionTerm => {
                // Leave term on a temporary — will be consumed by EndDefinitionDesc
                // Actually, keep it as-is; we'll pair it when we get desc.
            }
            OwnedEvent::StartDefinitionDesc => {
                self.stack.push(Frame::DefinitionDesc { inlines: Vec::new() });
            }
            OwnedEvent::EndDefinitionDesc => {
                let desc = if let Some(Frame::DefinitionDesc { inlines }) = self.stack.pop() {
                    inlines
                } else {
                    Vec::new()
                };
                let term = if let Some(Frame::DefinitionTerm { inlines }) = self.stack.pop() {
                    inlines
                } else {
                    Vec::new()
                };
                for frame in self.stack.iter_mut().rev() {
                    if let Frame::DefinitionList { items } = frame {
                        items.push((term, desc));
                        break;
                    }
                }
            }
            OwnedEvent::DocTest { expression, result } => {
                self.push_block(Block::DocTest {
                    expression: expression.into_owned(),
                    result: result.map(|r| r.into_owned()),
                    span: Span::NONE,
                });
            }
            OwnedEvent::StartBlockquote => {
                self.stack.push(Frame::Blockquote { inlines: Vec::new() });
            }
            OwnedEvent::EndBlockquote => {
                if let Some(Frame::Blockquote { inlines }) = self.stack.pop() {
                    self.push_block(Block::Blockquote { inlines, span: Span::NONE });
                }
            }
            OwnedEvent::Property { key, name } => {
                self.stack.push(Frame::Property {
                    key: key.into_owned(),
                    name: name.map(|n| n.into_owned()),
                    inlines: Vec::new(),
                });
            }
            OwnedEvent::EndProperty => {
                if let Some(Frame::Property { key, name, inlines }) = self.stack.pop() {
                    self.push_block(Block::Property {
                        key,
                        name,
                        description: inlines,
                        span: Span::NONE,
                    });
                }
            }
            OwnedEvent::Text(cow) => {
                self.push_inline(Inline::Text(cow.into_owned(), Span::NONE));
            }
            OwnedEvent::InlineCode(cow) => {
                self.push_inline(Inline::Code(cow.into_owned(), Span::NONE));
            }
            OwnedEvent::StartStrong => {
                self.stack.push(Frame::Strong { inlines: Vec::new() });
            }
            OwnedEvent::EndStrong => {
                if let Some(Frame::Strong { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Strong(inlines, Span::NONE));
                }
            }
            OwnedEvent::StartEmphasis => {
                self.stack.push(Frame::Emphasis { inlines: Vec::new() });
            }
            OwnedEvent::EndEmphasis => {
                if let Some(Frame::Emphasis { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Emphasis(inlines, Span::NONE));
                }
            }
            OwnedEvent::StartLink { url, text } => {
                self.stack.push(Frame::Link { url, text, inlines: Vec::new() });
            }
            OwnedEvent::EndLink => {
                if let Some(Frame::Link { url, text, .. }) = self.stack.pop() {
                    self.push_inline(Inline::Link { url, text, span: Span::NONE });
                }
            }
            OwnedEvent::ModuleLink { module } => {
                self.push_inline(Inline::ModuleLink { module, span: Span::NONE });
            }
        }
    }

    fn finish(mut self) -> HaddockDoc {
        let blocks = match self.stack.pop() {
            Some(Frame::Document { blocks }) => blocks,
            _ => Vec::new(),
        };
        HaddockDoc { blocks, span: Span::NONE }
    }
}
