//! Streaming Markua writer — converts a stream of events to Markua text.
//!
//! This implementation buffers all events, reconstructs the AST, then emits.
//!
//! # Example
//! ```no_run
//! use markua::writer::Writer;
//! use markua::OwnedMarkuaEvent;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! w.write_event(OwnedMarkuaEvent::StartHeading { level: 1 });
//! w.write_event(OwnedMarkuaEvent::Text("Hello".to_string().into()));
//! w.write_event(OwnedMarkuaEvent::EndHeading);
//! let bytes = w.finish();
//! ```

use crate::ast::*;
use crate::events::OwnedMarkuaEvent;
use std::io::Write;

/// Streaming Markua writer.
///
/// Feed events with [`write_event`](Writer::write_event), then call
/// [`finish`](Writer::finish) to flush Markua text to the underlying sink and
/// recover it.
pub struct Writer<W: Write> {
    sink: W,
    events: Vec<OwnedMarkuaEvent>,
}

impl<W: Write> Writer<W> {
    pub fn new(sink: W) -> Self {
        Writer { sink, events: Vec::new() }
    }

    /// Feed one event to the writer.
    pub fn write_event(&mut self, event: OwnedMarkuaEvent) {
        self.events.push(event);
    }

    /// Flush all buffered events as Markua text and return the sink.
    pub fn finish(mut self) -> W {
        let doc = events_to_doc(std::mem::take(&mut self.events));
        let text = crate::emit::emit(&doc);
        let _ = self.sink.write_all(text.as_bytes());
        self.sink
    }
}

// ── Event -> AST reconstruction ──────────────────────────────────────────────

fn events_to_doc(events: Vec<OwnedMarkuaEvent>) -> MarkuaDoc {
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
    List { ordered: bool, items: Vec<Vec<Block>> },
    ListItem { blocks: Vec<Block> },
    Table { rows: Vec<TableRow> },
    TableRow { cells: Vec<Vec<Inline>> },
    TableCell { inlines: Vec<Inline> },
    SpecialBlock { kind: String, blocks: Vec<Block> },
    DefinitionList { items: Vec<(Vec<Inline>, Vec<Block>)> },
    DefinitionTerm { inlines: Vec<Inline> },
    DefinitionDesc { blocks: Vec<Block> },
    Figure { blocks: Vec<Block> },
    Caption { inlines: Vec<Inline> },
    // Inline spans
    Strong { inlines: Vec<Inline> },
    Emphasis { inlines: Vec<Inline> },
    Strikethrough { inlines: Vec<Inline> },
    Subscript { inlines: Vec<Inline> },
    Superscript { inlines: Vec<Inline> },
    Underline { inlines: Vec<Inline> },
    SmallCaps { inlines: Vec<Inline> },
    FootnoteRef { inlines: Vec<Inline> },
    Link { url: String, inlines: Vec<Inline> },
}

struct DocBuilder {
    stack: Vec<Frame>,
}

impl DocBuilder {
    fn new() -> Self {
        DocBuilder { stack: vec![Frame::Document { blocks: vec![] }] }
    }

    #[allow(clippy::too_many_lines, clippy::collapsible_if)]
    fn process(&mut self, event: OwnedMarkuaEvent) {
        match event {
            // ── Block open/close ──────────────────────────────────────────────
            OwnedMarkuaEvent::StartParagraph => {
                self.stack.push(Frame::Paragraph { inlines: vec![] });
            }
            OwnedMarkuaEvent::EndParagraph => {
                if let Some(Frame::Paragraph { inlines }) = self.stack.pop() {
                    self.push_block(Block::Paragraph { inlines, span: Span::NONE });
                }
            }
            OwnedMarkuaEvent::StartHeading { level } => {
                self.stack.push(Frame::Heading { level, inlines: vec![] });
            }
            OwnedMarkuaEvent::EndHeading => {
                if let Some(Frame::Heading { level, inlines }) = self.stack.pop() {
                    self.push_block(Block::Heading { level, inlines, span: Span::NONE });
                }
            }
            OwnedMarkuaEvent::StartBlockquote => {
                self.stack.push(Frame::Blockquote { blocks: vec![] });
            }
            OwnedMarkuaEvent::EndBlockquote => {
                if let Some(Frame::Blockquote { blocks }) = self.stack.pop() {
                    self.push_block(Block::Blockquote { children: blocks, span: Span::NONE });
                }
            }
            OwnedMarkuaEvent::StartList { ordered } => {
                self.stack.push(Frame::List { ordered, items: vec![] });
            }
            OwnedMarkuaEvent::EndList => {
                if let Some(Frame::List { ordered, items }) = self.stack.pop() {
                    self.push_block(Block::List { ordered, items, span: Span::NONE });
                }
            }
            OwnedMarkuaEvent::StartListItem => {
                self.stack.push(Frame::ListItem { blocks: vec![] });
            }
            OwnedMarkuaEvent::EndListItem => {
                if let Some(Frame::ListItem { blocks }) = self.stack.pop()
                    && let Some(Frame::List { items, .. }) = self.stack.last_mut()
                {
                    items.push(blocks);
                }
            }
            OwnedMarkuaEvent::CodeBlock { language, content } => {
                self.push_block(Block::CodeBlock {
                    language,
                    content: content.into_owned(),
                    span: Span::NONE,
                });
            }
            OwnedMarkuaEvent::HorizontalRule => {
                self.push_block(Block::HorizontalRule { span: Span::NONE });
            }
            OwnedMarkuaEvent::PageBreak => {
                self.push_block(Block::PageBreak { span: Span::NONE });
            }
            OwnedMarkuaEvent::StartTable => {
                self.stack.push(Frame::Table { rows: vec![] });
            }
            OwnedMarkuaEvent::EndTable => {
                if let Some(Frame::Table { rows }) = self.stack.pop() {
                    self.push_block(Block::Table { rows, span: Span::NONE });
                }
            }
            OwnedMarkuaEvent::StartTableRow => {
                self.stack.push(Frame::TableRow { cells: vec![] });
            }
            OwnedMarkuaEvent::EndTableRow => {
                if let Some(Frame::TableRow { cells }) = self.stack.pop()
                    && let Some(Frame::Table { rows }) = self.stack.last_mut()
                {
                    rows.push(TableRow { cells, span: Span::NONE });
                }
            }
            OwnedMarkuaEvent::StartTableCell => {
                self.stack.push(Frame::TableCell { inlines: vec![] });
            }
            OwnedMarkuaEvent::EndTableCell => {
                if let Some(Frame::TableCell { inlines }) = self.stack.pop() {
                    if let Some(Frame::TableRow { cells, .. }) = self.stack.last_mut() {
                        cells.push(inlines);
                    }
                }
            }
            OwnedMarkuaEvent::StartSpecialBlock { kind } => {
                self.stack.push(Frame::SpecialBlock { kind, blocks: vec![] });
            }
            OwnedMarkuaEvent::EndSpecialBlock => {
                if let Some(Frame::SpecialBlock { kind, blocks }) = self.stack.pop() {
                    self.push_block(Block::SpecialBlock { block_type: kind, children: blocks, span: Span::NONE });
                }
            }
            OwnedMarkuaEvent::StartDefinitionList => {
                self.stack.push(Frame::DefinitionList { items: vec![] });
            }
            OwnedMarkuaEvent::EndDefinitionList => {
                if let Some(Frame::DefinitionList { items }) = self.stack.pop() {
                    self.push_block(Block::DefinitionList { items, span: Span::NONE });
                }
            }
            OwnedMarkuaEvent::StartDefinitionTerm => {
                self.stack.push(Frame::DefinitionTerm { inlines: vec![] });
            }
            OwnedMarkuaEvent::EndDefinitionTerm => {
                if let Some(Frame::DefinitionTerm { inlines }) = self.stack.pop() {
                    if let Some(Frame::DefinitionList { items }) = self.stack.last_mut() {
                        items.push((inlines, vec![]));
                    }
                }
            }
            OwnedMarkuaEvent::StartDefinitionDesc => {
                self.stack.push(Frame::DefinitionDesc { blocks: vec![] });
            }
            OwnedMarkuaEvent::EndDefinitionDesc => {
                if let Some(Frame::DefinitionDesc { blocks }) = self.stack.pop() {
                    if let Some(Frame::DefinitionList { items }) = self.stack.last_mut() {
                        if let Some(last) = items.last_mut() {
                            last.1 = blocks;
                        }
                    }
                }
            }
            OwnedMarkuaEvent::StartFigure => {
                self.stack.push(Frame::Figure { blocks: vec![] });
            }
            OwnedMarkuaEvent::EndFigure => {
                if let Some(Frame::Figure { blocks }) = self.stack.pop() {
                    let body = blocks.into_iter().next().unwrap_or(Block::Paragraph { inlines: vec![], span: Span::NONE });
                    self.push_block(Block::Figure { caption: vec![], body: Box::new(body), span: Span::NONE });
                }
            }
            OwnedMarkuaEvent::StartCaption => {
                self.stack.push(Frame::Caption { inlines: vec![] });
            }
            OwnedMarkuaEvent::EndCaption => {
                if let Some(Frame::Caption { inlines }) = self.stack.pop() {
                    // Try to attach caption to a pending Figure
                    if let Some(Frame::Figure { .. }) = self.stack.last() {
                        // Caption was inside figure; store as paragraph for now
                        if let Some(Frame::Figure { blocks }) = self.stack.last_mut() {
                            blocks.push(Block::Paragraph { inlines, span: Span::NONE });
                        }
                    }
                }
            }

            // ── Inline events ────────────────────────────────────────────────
            OwnedMarkuaEvent::Text(cow) => {
                self.push_inline(Inline::Text(cow.into_owned(), Span::NONE));
            }
            OwnedMarkuaEvent::SoftBreak => {
                self.push_inline(Inline::SoftBreak(Span::NONE));
            }
            OwnedMarkuaEvent::LineBreak => {
                self.push_inline(Inline::LineBreak(Span::NONE));
            }
            OwnedMarkuaEvent::StartStrong => {
                self.stack.push(Frame::Strong { inlines: vec![] });
            }
            OwnedMarkuaEvent::EndStrong => {
                if let Some(Frame::Strong { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Strong(inlines, Span::NONE));
                }
            }
            OwnedMarkuaEvent::StartEmphasis => {
                self.stack.push(Frame::Emphasis { inlines: vec![] });
            }
            OwnedMarkuaEvent::EndEmphasis => {
                if let Some(Frame::Emphasis { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Emphasis(inlines, Span::NONE));
                }
            }
            OwnedMarkuaEvent::StartStrikethrough => {
                self.stack.push(Frame::Strikethrough { inlines: vec![] });
            }
            OwnedMarkuaEvent::EndStrikethrough => {
                if let Some(Frame::Strikethrough { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Strikethrough(inlines, Span::NONE));
                }
            }
            OwnedMarkuaEvent::StartSubscript => {
                self.stack.push(Frame::Subscript { inlines: vec![] });
            }
            OwnedMarkuaEvent::EndSubscript => {
                if let Some(Frame::Subscript { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Subscript(inlines, Span::NONE));
                }
            }
            OwnedMarkuaEvent::StartSuperscript => {
                self.stack.push(Frame::Superscript { inlines: vec![] });
            }
            OwnedMarkuaEvent::EndSuperscript => {
                if let Some(Frame::Superscript { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Superscript(inlines, Span::NONE));
                }
            }
            OwnedMarkuaEvent::StartUnderline => {
                self.stack.push(Frame::Underline { inlines: vec![] });
            }
            OwnedMarkuaEvent::EndUnderline => {
                if let Some(Frame::Underline { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Underline(inlines, Span::NONE));
                }
            }
            OwnedMarkuaEvent::StartSmallCaps => {
                self.stack.push(Frame::SmallCaps { inlines: vec![] });
            }
            OwnedMarkuaEvent::EndSmallCaps => {
                if let Some(Frame::SmallCaps { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::SmallCaps(inlines, Span::NONE));
                }
            }
            OwnedMarkuaEvent::StartFootnoteRef => {
                self.stack.push(Frame::FootnoteRef { inlines: vec![] });
            }
            OwnedMarkuaEvent::EndFootnoteRef => {
                if let Some(Frame::FootnoteRef { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::FootnoteRef { content: inlines, span: Span::NONE });
                }
            }
            OwnedMarkuaEvent::InlineCode(cow) => {
                self.push_inline(Inline::Code(cow.into_owned(), Span::NONE));
            }
            OwnedMarkuaEvent::StartLink { url } => {
                self.stack.push(Frame::Link { url, inlines: vec![] });
            }
            OwnedMarkuaEvent::EndLink => {
                if let Some(Frame::Link { url, inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Link { url, children: inlines, span: Span::NONE });
                }
            }
            OwnedMarkuaEvent::Image { url, alt } => {
                self.push_inline(Inline::Image { url, alt, span: Span::NONE });
            }
            OwnedMarkuaEvent::IndexTerm { term } => {
                self.push_inline(Inline::IndexTerm { term, span: Span::NONE });
            }
            OwnedMarkuaEvent::MathInline { content } => {
                self.push_inline(Inline::MathInline { content, span: Span::NONE });
            }
        }
    }

    fn push_block(&mut self, block: Block) {
        match self.stack.last_mut() {
            Some(Frame::Document { blocks }) => blocks.push(block),
            Some(Frame::Blockquote { blocks }) => blocks.push(block),
            Some(Frame::ListItem { blocks }) => blocks.push(block),
            Some(Frame::SpecialBlock { blocks, .. }) => blocks.push(block),
            Some(Frame::Figure { blocks }) => blocks.push(block),
            Some(Frame::DefinitionDesc { blocks }) => blocks.push(block),
            _ => {} // unexpected context
        }
    }

    fn push_inline(&mut self, inline: Inline) {
        match self.stack.last_mut() {
            Some(Frame::Paragraph { inlines }) => inlines.push(inline),
            Some(Frame::Heading { inlines, .. }) => inlines.push(inline),
            Some(Frame::Strong { inlines }) => inlines.push(inline),
            Some(Frame::Emphasis { inlines }) => inlines.push(inline),
            Some(Frame::Strikethrough { inlines }) => inlines.push(inline),
            Some(Frame::Subscript { inlines }) => inlines.push(inline),
            Some(Frame::Superscript { inlines }) => inlines.push(inline),
            Some(Frame::Underline { inlines }) => inlines.push(inline),
            Some(Frame::SmallCaps { inlines }) => inlines.push(inline),
            Some(Frame::FootnoteRef { inlines }) => inlines.push(inline),
            Some(Frame::Link { inlines, .. }) => inlines.push(inline),
            Some(Frame::TableCell { inlines }) => inlines.push(inline),
            Some(Frame::DefinitionTerm { inlines }) => inlines.push(inline),
            Some(Frame::Caption { inlines }) => inlines.push(inline),
            _ => {} // unexpected context
        }
    }

    fn finish(mut self) -> MarkuaDoc {
        let blocks = match self.stack.pop() {
            Some(Frame::Document { blocks }) => blocks,
            _ => vec![],
        };
        MarkuaDoc {
            blocks,
            span: Span::NONE,
            title: None,
            author: None,
            description: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_writer_heading() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedMarkuaEvent::StartHeading { level: 1 });
        w.write_event(OwnedMarkuaEvent::Text(std::borrow::Cow::Owned("Hello".to_string())));
        w.write_event(OwnedMarkuaEvent::EndHeading);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("# Hello"), "got: {s:?}");
    }

    #[test]
    fn test_writer_paragraph() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedMarkuaEvent::StartParagraph);
        w.write_event(OwnedMarkuaEvent::Text(std::borrow::Cow::Owned("World".to_string())));
        w.write_event(OwnedMarkuaEvent::EndParagraph);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("World"), "got: {s:?}");
    }

    #[test]
    fn test_writer_roundtrip_via_events() {
        let input = "# Hello\n\nA paragraph with **bold** text.\n\n- item one\n- item two\n";
        let evts: Vec<_> = crate::events::events(input).collect();
        let mut w = Writer::new(Vec::<u8>::new());
        for e in evts {
            w.write_event(e);
        }
        let bytes = w.finish();
        let emitted_text = String::from_utf8(bytes).unwrap();
        let (doc_orig, _) = crate::parse::parse(input);
        let (doc_emit, _) = crate::parse::parse(&emitted_text);
        assert_eq!(doc_orig.blocks.len(), doc_emit.blocks.len(), "writer roundtrip block count mismatch");
    }
}
