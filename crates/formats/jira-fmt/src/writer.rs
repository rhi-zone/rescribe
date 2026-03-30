//! Streaming Jira wiki markup writer — converts a stream of events to Jira text.
//!
//! This implementation buffers all events, reconstructs the AST, then emits.
//!
//! # Example
//! ```no_run
//! use jira_fmt::writer::Writer;
//! use jira_fmt::OwnedEvent;
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

/// Streaming Jira wiki markup writer.
///
/// Feed events with [`write_event`](Writer::write_event), then call
/// [`finish`](Writer::finish) to flush Jira text to the underlying sink and
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

    /// Flush all buffered events as Jira text and return the sink.
    pub fn finish(mut self) -> W {
        let doc = events_to_doc(std::mem::take(&mut self.events));
        let text = crate::emit::build(&doc);
        let _ = self.sink.write_all(text.as_bytes());
        self.sink
    }
}

// ── Event → AST reconstruction ────────────────────────────────────────────────

fn events_to_doc(events: Vec<OwnedEvent>) -> JiraDoc {
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
    Blockquote { blocks: Vec<Block> },
    Panel { title: Option<String>, blocks: Vec<Block> },
    List { ordered: bool, items: Vec<ListItem> },
    ListItem { children: Vec<ListItemContent>, current_inlines: Option<Vec<Inline>> },
    Table { rows: Vec<TableRow> },
    TableRow { cells: Vec<TableCell> },
    TableCell { is_header: bool, inlines: Vec<Inline> },
    // Inline spans
    Bold { inlines: Vec<Inline> },
    Italic { inlines: Vec<Inline> },
    Underline { inlines: Vec<Inline> },
    Strikethrough { inlines: Vec<Inline> },
    Superscript { inlines: Vec<Inline> },
    Subscript { inlines: Vec<Inline> },
    Link { url: String, inlines: Vec<Inline> },
    ColorSpan { color: String, inlines: Vec<Inline> },
}

struct DocBuilder {
    stack: Vec<Frame>,
}

impl DocBuilder {
    fn new() -> Self {
        DocBuilder { stack: vec![Frame::Document { blocks: vec![] }] }
    }

    fn process(&mut self, event: OwnedEvent) {
        match event {
            // ── Block open/close ───────────────────────────────────────────────
            OwnedEvent::StartParagraph => {
                self.stack.push(Frame::Paragraph { inlines: vec![] });
            }
            OwnedEvent::EndParagraph => {
                if let Some(Frame::Paragraph { inlines }) = self.stack.pop() {
                    self.push_block(Block::Paragraph { inlines, span: Span::NONE });
                }
            }
            OwnedEvent::StartHeading { level } => {
                self.stack.push(Frame::Heading { level, inlines: vec![] });
            }
            OwnedEvent::EndHeading => {
                if let Some(Frame::Heading { level, inlines }) = self.stack.pop() {
                    self.push_block(Block::Heading { level, inlines, span: Span::NONE });
                }
            }
            OwnedEvent::StartBlockquote => {
                self.stack.push(Frame::Blockquote { blocks: vec![] });
            }
            OwnedEvent::EndBlockquote => {
                if let Some(Frame::Blockquote { blocks }) = self.stack.pop() {
                    self.push_block(Block::Blockquote { children: blocks, span: Span::NONE });
                }
            }
            OwnedEvent::StartPanel { title } => {
                self.stack.push(Frame::Panel { title, blocks: vec![] });
            }
            OwnedEvent::EndPanel => {
                if let Some(Frame::Panel { title, blocks }) = self.stack.pop() {
                    self.push_block(Block::Panel { title, children: blocks, span: Span::NONE });
                }
            }
            OwnedEvent::StartList { ordered } => {
                self.stack.push(Frame::List { ordered, items: vec![] });
            }
            OwnedEvent::EndList => {
                if let Some(Frame::List { ordered, items }) = self.stack.pop() {
                    self.push_block(Block::List { ordered, items, span: Span::NONE });
                }
            }
            OwnedEvent::StartListItem => {
                self.stack.push(Frame::ListItem {
                    children: vec![],
                    current_inlines: None,
                });
            }
            OwnedEvent::EndListItem => {
                if let Some(Frame::ListItem { mut children, current_inlines }) = self.stack.pop() {
                    if let Some(inlines) = current_inlines && !inlines.is_empty() {
                        children.push(ListItemContent::Inline(inlines));
                    }
                    if let Some(Frame::List { items, .. }) = self.stack.last_mut() {
                        items.push(ListItem { children });
                    }
                }
            }
            OwnedEvent::CodeBlock { language, content } => {
                self.push_block(Block::CodeBlock {
                    content: content.into_owned(),
                    language,
                    span: Span::NONE,
                });
            }
            OwnedEvent::Noformat { content } => {
                self.push_block(Block::Noformat {
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
                    self.push_block(Block::Table { rows, span: Span::NONE });
                }
            }
            OwnedEvent::StartTableRow => {
                self.stack.push(Frame::TableRow { cells: vec![] });
            }
            OwnedEvent::EndTableRow => {
                if let Some(Frame::TableRow { cells }) = self.stack.pop()
                    && let Some(Frame::Table { rows }) = self.stack.last_mut()
                {
                    rows.push(TableRow { cells, span: Span::NONE });
                }
            }
            OwnedEvent::StartTableCell { is_header } => {
                self.stack.push(Frame::TableCell { is_header, inlines: vec![] });
            }
            OwnedEvent::EndTableCell => {
                if let Some(Frame::TableCell { is_header, inlines }) = self.stack.pop()
                    && let Some(Frame::TableRow { cells }) = self.stack.last_mut()
                {
                    cells.push(TableCell { is_header, inlines, span: Span::NONE });
                }
            }

            // ── Inline events ──────────────────────────────────────────────────
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
                self.stack.push(Frame::Strikethrough { inlines: vec![] });
            }
            OwnedEvent::EndStrikethrough => {
                if let Some(Frame::Strikethrough { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Strikethrough(inlines, Span::NONE));
                }
            }
            OwnedEvent::StartSuperscript => {
                self.stack.push(Frame::Superscript { inlines: vec![] });
            }
            OwnedEvent::EndSuperscript => {
                if let Some(Frame::Superscript { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Superscript(inlines, Span::NONE));
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
            OwnedEvent::InlineCode(cow) => {
                self.push_inline(Inline::Code(cow.into_owned(), Span::NONE));
            }
            OwnedEvent::StartLink { url } => {
                self.stack.push(Frame::Link { url, inlines: vec![] });
            }
            OwnedEvent::EndLink => {
                if let Some(Frame::Link { url, inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Link { url, children: inlines, span: Span::NONE });
                }
            }
            OwnedEvent::InlineImage { url, alt } => {
                self.push_inline(Inline::Image { url, alt, span: Span::NONE });
            }
            OwnedEvent::StartColorSpan { color } => {
                self.stack.push(Frame::ColorSpan { color, inlines: vec![] });
            }
            OwnedEvent::EndColorSpan => {
                if let Some(Frame::ColorSpan { color, inlines }) = self.stack.pop() {
                    self.push_inline(Inline::ColorSpan {
                        color,
                        children: inlines,
                        span: Span::NONE,
                    });
                }
            }
            OwnedEvent::Mention(cow) => {
                self.push_inline(Inline::Mention(cow.into_owned(), Span::NONE));
            }
        }
    }

    fn push_block(&mut self, block: Block) {
        match self.stack.last_mut() {
            Some(Frame::Document { blocks }) => blocks.push(block),
            Some(Frame::Blockquote { blocks }) => blocks.push(block),
            Some(Frame::Panel { blocks, .. }) => blocks.push(block),
            Some(Frame::ListItem { children, current_inlines, .. }) => {
                if let Some(inlines) = current_inlines.take() && !inlines.is_empty() {
                    children.push(ListItemContent::Inline(inlines));
                }
                // A Paragraph inside a list item is stored as inline content,
                // not as a nested block, to preserve list structure on roundtrip.
                match block {
                    Block::Paragraph { inlines, .. } => {
                        children.push(ListItemContent::Inline(inlines));
                    }
                    other => {
                        children.push(ListItemContent::NestedList(other));
                    }
                }
            }
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
            Some(Frame::Superscript { inlines }) => inlines.push(inline),
            Some(Frame::Subscript { inlines }) => inlines.push(inline),
            Some(Frame::Link { inlines, .. }) => inlines.push(inline),
            Some(Frame::TableCell { inlines, .. }) => inlines.push(inline),
            Some(Frame::ColorSpan { inlines, .. }) => inlines.push(inline),
            Some(Frame::ListItem { current_inlines, .. }) => {
                if current_inlines.is_none() {
                    *current_inlines = Some(vec![]);
                }
                current_inlines.as_mut().unwrap().push(inline);
            }
            _ => {}
        }
    }

    fn finish(mut self) -> JiraDoc {
        let blocks = match self.stack.pop() {
            Some(Frame::Document { blocks }) => blocks,
            _ => vec![],
        };
        JiraDoc { blocks, span: Span::NONE }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_writer_heading() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedEvent::StartHeading { level: 1 });
        w.write_event(OwnedEvent::Text(std::borrow::Cow::Owned("Hello".to_string())));
        w.write_event(OwnedEvent::EndHeading);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("h1. Hello"), "got: {s:?}");
    }

    #[test]
    fn test_writer_paragraph() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedEvent::StartParagraph);
        w.write_event(OwnedEvent::Text(std::borrow::Cow::Owned("World".to_string())));
        w.write_event(OwnedEvent::EndParagraph);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("World"), "got: {s:?}");
    }

    #[test]
    fn test_writer_roundtrip_via_events() {
        let input = "h1. Hello\n\nA paragraph with *bold* text.\n\n* item one\n* item two\n";
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
