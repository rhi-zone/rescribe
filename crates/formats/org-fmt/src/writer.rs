#![allow(clippy::collapsible_if)]
//! Streaming Org-mode writer — converts a stream of events to Org text.
//!
//! This implementation buffers all events, reconstructs the AST, then emits.
//!
//! # Example
//! ```no_run
//! use org_fmt::writer::Writer;
//! use org_fmt::OwnedEvent;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! w.write_event(OwnedEvent::StartHeading { level: 1, todo: None, priority: None, tags: vec![], properties: vec![], scheduled: None, deadline: None });
//! w.write_event(OwnedEvent::Text("Hello".to_string().into()));
//! w.write_event(OwnedEvent::EndHeading);
//! let bytes = w.finish();
//! ```

use crate::ast::*;
use crate::events::OwnedEvent;
use std::io::Write;

/// Streaming Org-mode writer.
///
/// Feed events with [`write_event`](Writer::write_event), then call
/// [`finish`](Writer::finish) to flush Org text to the underlying sink and
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

    /// Flush all buffered events as Org text and return the sink.
    pub fn finish(mut self) -> W {
        let doc = events_to_doc(std::mem::take(&mut self.events));
        let text = crate::emit::build(&doc);
        let _ = self.sink.write_all(text.as_bytes());
        self.sink
    }
}

// ── Event → AST reconstruction ────────────────────────────────────────────────

fn events_to_doc(events: Vec<OwnedEvent>) -> OrgDoc {
    let mut builder = DocBuilder::new();
    for event in events {
        builder.process(event);
    }
    builder.finish()
}

enum Frame {
    Document { blocks: Vec<Block> },
    Paragraph { inlines: Vec<Inline> },
    Heading { level: usize, todo: Option<String>, priority: Option<String>, tags: Vec<String>, properties: Vec<(String, String)>, scheduled: Option<String>, deadline: Option<String>, inlines: Vec<Inline> },
    Blockquote { blocks: Vec<Block> },
    List { ordered: bool, start: Option<u64>, items: Vec<ListItem> },
    ListItem { checkbox: Option<CheckboxState>, children: Vec<ListItemContent>, current_inlines: Option<Vec<Inline>> },
    Table { rows: Vec<TableRow> },
    TableRow { cells: Vec<Vec<Inline>>, is_header: bool },
    TableCell { inlines: Vec<Inline> },
    DefinitionList { items: Vec<DefinitionItem> },
    DefinitionTerm { inlines: Vec<Inline> },
    DefinitionDesc { inlines: Vec<Inline> },
    Div { inlines: Vec<Inline> },
    Figure { name: Option<String>, blocks: Vec<Block> },
    Caption { inlines: Vec<Inline> },
    FootnoteDefinition { label: String, inlines: Vec<Inline> },
    BlockFootnoteDef { label: String, inlines: Vec<Inline> },
    // Inline spans
    Bold { inlines: Vec<Inline> },
    Italic { inlines: Vec<Inline> },
    Underline { inlines: Vec<Inline> },
    Strikethrough { inlines: Vec<Inline> },
    Superscript { inlines: Vec<Inline> },
    Subscript { inlines: Vec<Inline> },
    Link { url: String, inlines: Vec<Inline> },
}

struct DocBuilder {
    stack: Vec<Frame>,
}

impl DocBuilder {
    fn new() -> Self {
        DocBuilder { stack: vec![Frame::Document { blocks: vec![] }] }
    }

    #[allow(clippy::too_many_lines)]
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
            OwnedEvent::StartHeading { level, todo, priority, tags, properties, scheduled, deadline } => {
                self.stack.push(Frame::Heading {
                    level,
                    todo,
                    priority,
                    tags,
                    properties,
                    scheduled,
                    deadline,
                    inlines: vec![],
                });
            }
            OwnedEvent::EndHeading => {
                if let Some(Frame::Heading { level, todo, priority, tags, properties, scheduled, deadline, inlines }) = self.stack.pop() {
                    self.push_block(Block::Heading {
                        level,
                        todo,
                        priority,
                        tags,
                        properties,
                        scheduled,
                        deadline,
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
                    self.push_block(Block::Blockquote { children: blocks, span: Span::NONE });
                }
            }
            OwnedEvent::StartList { ordered, start } => {
                self.stack.push(Frame::List { ordered, start, items: vec![] });
            }
            OwnedEvent::EndList => {
                if let Some(Frame::List { ordered, start, items }) = self.stack.pop() {
                    self.push_block(Block::List { ordered, start, items, span: Span::NONE });
                }
            }
            OwnedEvent::StartListItem { checkbox } => {
                self.stack.push(Frame::ListItem { checkbox, children: vec![], current_inlines: Some(vec![]) });
            }
            OwnedEvent::EndListItem => {
                if let Some(Frame::ListItem { checkbox, mut children, current_inlines }) = self.stack.pop() {
                    // Flush any pending inlines
                    if let Some(inlines) = current_inlines {
                        if !inlines.is_empty() {
                            children.push(ListItemContent::Inline(inlines));
                        }
                    }
                    if let Some(Frame::List { items, .. }) = self.stack.last_mut() {
                        items.push(ListItem { children, checkbox });
                    }
                }
            }
            OwnedEvent::CodeBlock { language, header_args, name, content } => {
                self.push_block(Block::CodeBlock {
                    language,
                    header_args,
                    name,
                    content: content.into_owned(),
                    span: Span::NONE,
                });
            }
            OwnedEvent::RawBlock { format, content } => {
                self.push_block(Block::RawBlock { format, content, span: Span::NONE });
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
            OwnedEvent::StartTableRow { is_header } => {
                self.stack.push(Frame::TableRow { cells: vec![], is_header });
            }
            OwnedEvent::EndTableRow => {
                if let Some(Frame::TableRow { cells, is_header }) = self.stack.pop() {
                    if let Some(Frame::Table { rows }) = self.stack.last_mut() {
                        rows.push(TableRow { cells, is_header });
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
                self.stack.push(Frame::DefinitionList { items: vec![] });
            }
            OwnedEvent::EndDefinitionList => {
                if let Some(Frame::DefinitionList { items }) = self.stack.pop() {
                    self.push_block(Block::DefinitionList { items, span: Span::NONE });
                }
            }
            OwnedEvent::StartDefinitionTerm => {
                self.stack.push(Frame::DefinitionTerm { inlines: vec![] });
            }
            OwnedEvent::EndDefinitionTerm => {
                if let Some(Frame::DefinitionTerm { inlines }) = self.stack.pop() {
                    if let Some(Frame::DefinitionList { items, .. }) = self.stack.last_mut() {
                        // Push an incomplete item; desc will be filled in by EndDefinitionDesc
                        items.push(DefinitionItem { term: inlines, desc: vec![] });
                    }
                }
            }
            OwnedEvent::StartDefinitionDesc => {
                self.stack.push(Frame::DefinitionDesc { inlines: vec![] });
            }
            OwnedEvent::EndDefinitionDesc => {
                if let Some(Frame::DefinitionDesc { inlines }) = self.stack.pop() {
                    if let Some(Frame::DefinitionList { items, .. }) = self.stack.last_mut() {
                        if let Some(last) = items.last_mut() {
                            last.desc = inlines;
                        }
                    }
                }
            }
            OwnedEvent::StartDiv => {
                self.stack.push(Frame::Div { inlines: vec![] });
            }
            OwnedEvent::EndDiv => {
                if let Some(Frame::Div { inlines }) = self.stack.pop() {
                    self.push_block(Block::Div { inlines, span: Span::NONE });
                }
            }
            OwnedEvent::StartFigure { name } => {
                self.stack.push(Frame::Figure { name, blocks: vec![] });
            }
            OwnedEvent::EndFigure => {
                if let Some(Frame::Figure { name, blocks }) = self.stack.pop() {
                    self.push_block(Block::Figure { name, children: blocks, span: Span::NONE });
                }
            }
            OwnedEvent::StartCaption => {
                self.stack.push(Frame::Caption { inlines: vec![] });
            }
            OwnedEvent::EndCaption => {
                if let Some(Frame::Caption { inlines }) = self.stack.pop() {
                    self.push_block(Block::Caption { inlines, span: Span::NONE });
                }
            }
            OwnedEvent::UnknownBlock { kind } => {
                self.push_block(Block::Unknown { kind, span: Span::NONE });
            }

            // ── Inline events ──────────────────────────────────────────────────
            OwnedEvent::Text(cow) => {
                self.push_inline(Inline::Text { text: cow.into_owned(), span: Span::NONE });
            }
            OwnedEvent::SoftBreak => {
                self.push_inline(Inline::SoftBreak { span: Span::NONE });
            }
            OwnedEvent::LineBreak => {
                self.push_inline(Inline::LineBreak { span: Span::NONE });
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
            OwnedEvent::InlineImage { url } => {
                self.push_inline(Inline::Image { url, span: Span::NONE });
            }
            OwnedEvent::FootnoteRef { label } => {
                self.push_inline(Inline::FootnoteRef { label, span: Span::NONE });
            }
            OwnedEvent::StartFootnoteDefinition { label } => {
                self.stack.push(Frame::FootnoteDefinition { label, inlines: vec![] });
            }
            OwnedEvent::EndFootnoteDefinition => {
                if let Some(Frame::FootnoteDefinition { label, inlines }) = self.stack.pop() {
                    self.push_inline(Inline::FootnoteDefinition { label, children: inlines, span: Span::NONE });
                }
            }
            OwnedEvent::StartBlockFootnoteDef { label } => {
                self.stack.push(Frame::BlockFootnoteDef { label, inlines: vec![] });
            }
            OwnedEvent::EndBlockFootnoteDef => {
                if let Some(Frame::BlockFootnoteDef { label, inlines }) = self.stack.pop() {
                    self.push_block(Block::FootnoteDef { label, content: inlines, span: Span::NONE });
                }
            }
            OwnedEvent::MathInline { source } => {
                self.push_inline(Inline::MathInline { source, span: Span::NONE });
            }
            OwnedEvent::Timestamp { active, value } => {
                self.push_inline(Inline::Timestamp { active, value, span: Span::NONE });
            }
            OwnedEvent::ExportSnippet { backend, value } => {
                self.push_inline(Inline::ExportSnippet { backend, value, span: Span::NONE });
            }
        }
    }

    fn push_block(&mut self, block: Block) {
        match self.stack.last_mut() {
            Some(Frame::Document { blocks }) => blocks.push(block),
            Some(Frame::Blockquote { blocks }) => blocks.push(block),
            Some(Frame::Figure { blocks, .. }) => blocks.push(block),
            Some(Frame::ListItem { children, current_inlines, .. }) => {
                // Flush any pending inlines before pushing a block
                if let Some(inlines) = current_inlines.take() {
                    if !inlines.is_empty() {
                        children.push(ListItemContent::Inline(inlines));
                    }
                }
                children.push(ListItemContent::Block(block));
            }
            _ => {} // unexpected context, discard
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
            Some(Frame::TableCell { inlines }) => inlines.push(inline),
            Some(Frame::DefinitionTerm { inlines }) => inlines.push(inline),
            Some(Frame::DefinitionDesc { inlines }) => inlines.push(inline),
            Some(Frame::Div { inlines }) => inlines.push(inline),
            Some(Frame::Caption { inlines }) => inlines.push(inline),
            Some(Frame::FootnoteDefinition { inlines, .. }) => inlines.push(inline),
            Some(Frame::BlockFootnoteDef { inlines, .. }) => inlines.push(inline),
            Some(Frame::ListItem { current_inlines: Some(inlines), .. }) => inlines.push(inline),
            _ => {} // unexpected context, discard
        }
    }

    fn finish(mut self) -> OrgDoc {
        let blocks = match self.stack.pop() {
            Some(Frame::Document { blocks }) => blocks,
            _ => vec![],
        };
        OrgDoc { blocks, metadata: vec![] }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_writer_heading() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedEvent::StartHeading {
            level: 1,
            todo: None,
            priority: None,
            tags: vec![],
            properties: vec![],
            scheduled: None,
            deadline: None,
        });
        w.write_event(OwnedEvent::Text(std::borrow::Cow::Owned("Hello".to_string())));
        w.write_event(OwnedEvent::EndHeading);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("* Hello"), "got: {s:?}");
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
        let input = "* Hello\n\nA paragraph with *bold* text.\n\n- item one\n- item two\n";
        let evts: Vec<_> = crate::events(input).collect();
        let mut w = Writer::new(Vec::<u8>::new());
        for e in evts {
            w.write_event(e);
        }
        let bytes = w.finish();
        let emitted_text = String::from_utf8(bytes).unwrap();
        // Verify the emitted text re-parses cleanly (no panic, same block count)
        let (doc_orig, _) = crate::parse::parse(input);
        let (doc_emit, _) = crate::parse::parse(&emitted_text);
        assert_eq!(doc_orig.blocks.len(), doc_emit.blocks.len(), "writer roundtrip block count mismatch");
    }
}
