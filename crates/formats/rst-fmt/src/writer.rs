#![allow(clippy::collapsible_if)]
//! Streaming RST writer — converts a stream of events to RST text.
//!
//! This implementation buffers all events, reconstructs the AST, then emits.
//! Future versions may emit bytes incrementally at block boundaries.
//!
//! # Example
//! ```no_run
//! use rst_fmt::writer::Writer;
//! use rst_fmt::OwnedEvent;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! w.write_event(OwnedEvent::StartHeading { level: 1 });
//! w.write_event(OwnedEvent::Text("Hello".to_string()));
//! w.write_event(OwnedEvent::EndHeading);
//! let bytes = w.finish();
//! ```

use crate::events::OwnedEvent;
use crate::{Block, DefinitionItem, Inline, RstDoc, TableRow};
use std::io::Write;

/// Streaming RST writer.
///
/// Feed events with [`write_event`](Writer::write_event), then call
/// [`finish`](Writer::finish) to flush RST text to the underlying sink and
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

    /// Flush all buffered events as RST text and return the sink.
    pub fn finish(mut self) -> W {
        let doc = events_to_doc(std::mem::take(&mut self.events));
        let text = crate::build(&doc);
        let _ = self.sink.write_all(text.as_bytes());
        self.sink
    }
}

// ── Event → AST reconstruction ────────────────────────────────────────────────

fn events_to_doc(events: Vec<OwnedEvent>) -> RstDoc {
    let mut builder = DocBuilder::new();
    for event in events {
        builder.process(event);
    }
    builder.finish()
}

enum Frame {
    Document { blocks: Vec<Block> },
    Paragraph { inlines: Vec<Inline> },
    Heading { level: i64, inlines: Vec<Inline> },
    Blockquote { blocks: Vec<Block> },
    List { ordered: bool, items: Vec<Vec<Block>> },
    ListItem { blocks: Vec<Block> },
    CodeBlock { language: Option<String>, content: String },
    Div { class: Option<String>, directive: Option<String>, blocks: Vec<Block> },
    Table { rows: Vec<TableRow> },
    TableRow { cells: Vec<Vec<Inline>>, is_header: bool },
    TableCell { inlines: Vec<Inline> },
    DefinitionList { items: Vec<DefinitionItem> },
    DefinitionTerm { inlines: Vec<Inline> },
    DefinitionDesc { inlines: Vec<Inline> },
    FootnoteDef { label: String, inlines: Vec<Inline> },
    Figure { url: String, alt: Option<String>, caption: Vec<Inline> },
    Admonition { admonition_type: String, blocks: Vec<Block> },
    LineBlock { lines: Vec<Vec<Inline>> },
    LineBlockLine { inlines: Vec<Inline> },
    // Inline spans
    Emphasis { inlines: Vec<Inline> },
    Strong { inlines: Vec<Inline> },
    Strikeout { inlines: Vec<Inline> },
    Underline { inlines: Vec<Inline> },
    Subscript { inlines: Vec<Inline> },
    Superscript { inlines: Vec<Inline> },
    SmallCaps { inlines: Vec<Inline> },
    Link { url: String, inlines: Vec<Inline> },
    FootnoteDefInline { label: String, inlines: Vec<Inline> },
    Quoted { quote_type: String, inlines: Vec<Inline> },
    RstSpan { role: String, inlines: Vec<Inline> },
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
                    self.push_block(Block::Paragraph { inlines });
                }
            }
            OwnedEvent::StartHeading { level } => {
                self.stack.push(Frame::Heading { level, inlines: vec![] });
            }
            OwnedEvent::EndHeading => {
                if let Some(Frame::Heading { level, inlines }) = self.stack.pop() {
                    self.push_block(Block::Heading { level, inlines });
                }
            }
            OwnedEvent::StartBlockquote => {
                self.stack.push(Frame::Blockquote { blocks: vec![] });
            }
            OwnedEvent::EndBlockquote => {
                if let Some(Frame::Blockquote { blocks }) = self.stack.pop() {
                    self.push_block(Block::Blockquote { children: blocks });
                }
            }
            OwnedEvent::StartList { ordered } => {
                self.stack.push(Frame::List { ordered, items: vec![] });
            }
            OwnedEvent::EndList => {
                if let Some(Frame::List { ordered, items }) = self.stack.pop() {
                    self.push_block(Block::List { ordered, items });
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
            OwnedEvent::StartCodeBlock { language } => {
                self.stack.push(Frame::CodeBlock { language, content: String::new() });
            }
            OwnedEvent::CodeBlockContent(content) => {
                if let Some(Frame::CodeBlock { content: buf, .. }) = self.stack.last_mut() {
                    buf.push_str(&content);
                }
            }
            OwnedEvent::EndCodeBlock => {
                if let Some(Frame::CodeBlock { language, content }) = self.stack.pop() {
                    self.push_block(Block::CodeBlock { language, content });
                }
            }
            OwnedEvent::RawBlock { format, content } => {
                self.push_block(Block::RawBlock { format, content });
            }
            OwnedEvent::StartDiv { class, directive } => {
                self.stack.push(Frame::Div { class, directive, blocks: vec![] });
            }
            OwnedEvent::EndDiv => {
                if let Some(Frame::Div { class, directive, blocks }) = self.stack.pop() {
                    self.push_block(Block::Div { class, directive, children: blocks });
                }
            }
            OwnedEvent::HorizontalRule => {
                self.push_block(Block::HorizontalRule);
            }
            OwnedEvent::StartTable => {
                self.stack.push(Frame::Table { rows: vec![] });
            }
            OwnedEvent::EndTable => {
                if let Some(Frame::Table { rows }) = self.stack.pop() {
                    self.push_block(Block::Table { rows });
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
                    self.push_block(Block::DefinitionList { items });
                }
            }
            OwnedEvent::StartDefinitionTerm => {
                self.stack.push(Frame::DefinitionTerm { inlines: vec![] });
            }
            OwnedEvent::EndDefinitionTerm => {
                if let Some(Frame::DefinitionTerm { inlines }) = self.stack.pop() {
                    if let Some(Frame::DefinitionList { items, .. }) = self.stack.last_mut() {
                        // Push incomplete item; desc will be filled by EndDefinitionDesc
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
            OwnedEvent::StartFootnoteDef { label } => {
                self.stack.push(Frame::FootnoteDef { label, inlines: vec![] });
            }
            OwnedEvent::EndFootnoteDef => {
                if let Some(Frame::FootnoteDef { label, inlines }) = self.stack.pop() {
                    self.push_block(Block::FootnoteDef { label, inlines });
                }
            }
            OwnedEvent::MathDisplay { source } => {
                self.push_block(Block::MathDisplay { source });
            }
            OwnedEvent::StartAdmonition { admonition_type } => {
                self.stack.push(Frame::Admonition { admonition_type, blocks: vec![] });
            }
            OwnedEvent::EndAdmonition => {
                if let Some(Frame::Admonition { admonition_type, blocks }) = self.stack.pop() {
                    self.push_block(Block::Admonition { admonition_type, children: blocks });
                }
            }
            OwnedEvent::StartFigure { url, alt } => {
                self.stack.push(Frame::Figure { url, alt, caption: vec![] });
            }
            OwnedEvent::EndFigure => {
                if let Some(Frame::Figure { url, alt, caption }) = self.stack.pop() {
                    let caption = if caption.is_empty() { None } else { Some(caption) };
                    self.push_block(Block::Figure { url, alt, caption });
                }
            }
            OwnedEvent::ImageBlock { url, alt, title } => {
                self.push_block(Block::Image { url, alt, title });
            }
            OwnedEvent::StartLineBlock => {
                self.stack.push(Frame::LineBlock { lines: vec![] });
            }
            OwnedEvent::EndLineBlock => {
                if let Some(Frame::LineBlock { lines }) = self.stack.pop() {
                    self.push_block(Block::LineBlock { lines });
                }
            }
            OwnedEvent::StartLineBlockLine => {
                self.stack.push(Frame::LineBlockLine { inlines: vec![] });
            }
            OwnedEvent::EndLineBlockLine => {
                if let Some(Frame::LineBlockLine { inlines }) = self.stack.pop() {
                    if let Some(Frame::LineBlock { lines }) = self.stack.last_mut() {
                        lines.push(inlines);
                    }
                }
            }

            // ── Inline events ──────────────────────────────────────────────────
            OwnedEvent::Text(s) => {
                self.push_inline(Inline::Text(s));
            }
            OwnedEvent::SoftBreak => {
                self.push_inline(Inline::SoftBreak);
            }
            OwnedEvent::LineBreak => {
                self.push_inline(Inline::LineBreak);
            }
            OwnedEvent::StartEmphasis => {
                self.stack.push(Frame::Emphasis { inlines: vec![] });
            }
            OwnedEvent::EndEmphasis => {
                if let Some(Frame::Emphasis { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Emphasis(inlines));
                }
            }
            OwnedEvent::StartStrong => {
                self.stack.push(Frame::Strong { inlines: vec![] });
            }
            OwnedEvent::EndStrong => {
                if let Some(Frame::Strong { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Strong(inlines));
                }
            }
            OwnedEvent::StartStrikeout => {
                self.stack.push(Frame::Strikeout { inlines: vec![] });
            }
            OwnedEvent::EndStrikeout => {
                if let Some(Frame::Strikeout { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Strikeout(inlines));
                }
            }
            OwnedEvent::StartUnderline => {
                self.stack.push(Frame::Underline { inlines: vec![] });
            }
            OwnedEvent::EndUnderline => {
                if let Some(Frame::Underline { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Underline(inlines));
                }
            }
            OwnedEvent::StartSubscript => {
                self.stack.push(Frame::Subscript { inlines: vec![] });
            }
            OwnedEvent::EndSubscript => {
                if let Some(Frame::Subscript { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Subscript(inlines));
                }
            }
            OwnedEvent::StartSuperscript => {
                self.stack.push(Frame::Superscript { inlines: vec![] });
            }
            OwnedEvent::EndSuperscript => {
                if let Some(Frame::Superscript { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Superscript(inlines));
                }
            }
            OwnedEvent::StartSmallCaps => {
                self.stack.push(Frame::SmallCaps { inlines: vec![] });
            }
            OwnedEvent::EndSmallCaps => {
                if let Some(Frame::SmallCaps { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::SmallCaps(inlines));
                }
            }
            OwnedEvent::Code(s) => {
                self.push_inline(Inline::Code(s));
            }
            OwnedEvent::StartLink { url } => {
                self.stack.push(Frame::Link { url, inlines: vec![] });
            }
            OwnedEvent::EndLink => {
                if let Some(Frame::Link { url, inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Link { url, children: inlines });
                }
            }
            OwnedEvent::InlineImage { url, alt } => {
                self.push_inline(Inline::Image { url, alt });
            }
            OwnedEvent::FootnoteRef { label } => {
                self.push_inline(Inline::FootnoteRef { label });
            }
            OwnedEvent::StartFootnoteDefInline { label } => {
                self.stack.push(Frame::FootnoteDefInline { label, inlines: vec![] });
            }
            OwnedEvent::EndFootnoteDefInline => {
                if let Some(Frame::FootnoteDefInline { label, inlines }) = self.stack.pop() {
                    self.push_inline(Inline::FootnoteDef { label, children: inlines });
                }
            }
            OwnedEvent::StartQuoted { quote_type } => {
                self.stack.push(Frame::Quoted { quote_type, inlines: vec![] });
            }
            OwnedEvent::EndQuoted => {
                if let Some(Frame::Quoted { quote_type, inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Quoted { quote_type, children: inlines });
                }
            }
            OwnedEvent::MathInline { source } => {
                self.push_inline(Inline::MathInline { source });
            }
            OwnedEvent::StartRstSpan { role } => {
                self.stack.push(Frame::RstSpan { role, inlines: vec![] });
            }
            OwnedEvent::EndRstSpan => {
                if let Some(Frame::RstSpan { role, inlines }) = self.stack.pop() {
                    self.push_inline(Inline::RstSpan { role, children: inlines });
                }
            }
        }
    }

    fn push_block(&mut self, block: Block) {
        match self.stack.last_mut() {
            Some(Frame::Document { blocks }) => blocks.push(block),
            Some(Frame::Blockquote { blocks }) => blocks.push(block),
            Some(Frame::ListItem { blocks }) => blocks.push(block),
            Some(Frame::Div { blocks, .. }) => blocks.push(block),
            Some(Frame::Admonition { blocks, .. }) => blocks.push(block),
            _ => {} // unexpected context, discard
        }
    }

    fn push_inline(&mut self, inline: Inline) {
        match self.stack.last_mut() {
            Some(Frame::Paragraph { inlines }) => inlines.push(inline),
            Some(Frame::Heading { inlines, .. }) => inlines.push(inline),
            Some(Frame::Emphasis { inlines }) => inlines.push(inline),
            Some(Frame::Strong { inlines }) => inlines.push(inline),
            Some(Frame::Strikeout { inlines }) => inlines.push(inline),
            Some(Frame::Underline { inlines }) => inlines.push(inline),
            Some(Frame::Subscript { inlines }) => inlines.push(inline),
            Some(Frame::Superscript { inlines }) => inlines.push(inline),
            Some(Frame::SmallCaps { inlines }) => inlines.push(inline),
            Some(Frame::Link { inlines, .. }) => inlines.push(inline),
            Some(Frame::FootnoteDefInline { inlines, .. }) => inlines.push(inline),
            Some(Frame::Quoted { inlines, .. }) => inlines.push(inline),
            Some(Frame::RstSpan { inlines, .. }) => inlines.push(inline),
            Some(Frame::TableCell { inlines }) => inlines.push(inline),
            Some(Frame::DefinitionTerm { inlines }) => inlines.push(inline),
            Some(Frame::DefinitionDesc { inlines }) => inlines.push(inline),
            Some(Frame::FootnoteDef { inlines, .. }) => inlines.push(inline),
            Some(Frame::Figure { caption, .. }) => caption.push(inline),
            Some(Frame::LineBlockLine { inlines }) => inlines.push(inline),
            _ => {} // unexpected context, discard
        }
    }

    fn finish(mut self) -> RstDoc {
        let blocks = match self.stack.pop() {
            Some(Frame::Document { blocks }) => blocks,
            _ => vec![],
        };
        RstDoc { blocks }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_writer_heading() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedEvent::StartHeading { level: 1 });
        w.write_event(OwnedEvent::Text("Hello".to_string()));
        w.write_event(OwnedEvent::EndHeading);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("Hello"), "got: {s:?}");
    }

    #[test]
    fn test_writer_paragraph() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedEvent::StartParagraph);
        w.write_event(OwnedEvent::Text("World".to_string()));
        w.write_event(OwnedEvent::EndParagraph);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("World"), "got: {s:?}");
    }

    #[test]
    fn test_writer_roundtrip_via_events() {
        let input = "Section\n=======\n\nA paragraph with *emphasis* text.\n\n- item one\n- item two\n";
        let doc = crate::parse(input).unwrap();
        let evts: Vec<_> = crate::EventIter::new(input).collect();
        let mut w = Writer::new(Vec::<u8>::new());
        for e in evts {
            w.write_event(e);
        }
        let bytes = w.finish();
        let emitted_text = String::from_utf8(bytes).unwrap();
        let doc2 = crate::parse(&emitted_text).unwrap();
        // Compare block counts as a basic sanity check
        assert_eq!(
            doc.blocks.len(),
            doc2.blocks.len(),
            "writer roundtrip block count mismatch"
        );
    }
}
