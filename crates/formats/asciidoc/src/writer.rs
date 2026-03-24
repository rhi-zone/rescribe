#![allow(clippy::collapsible_if)]
//! Streaming AsciiDoc writer — converts a stream of events to AsciiDoc text.
//!
//! This implementation buffers all events, reconstructs the AST, then emits.
//! Future versions may emit bytes incrementally at block boundaries.
//!
//! # Example
//! ```no_run
//! use asciidoc::writer::Writer;
//! use asciidoc::OwnedEvent;
//! use std::borrow::Cow;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! w.write_event(OwnedEvent::StartHeading { level: 1, id: None, role: None });
//! w.write_event(OwnedEvent::Text(Cow::Owned("Hello".to_string())));
//! w.write_event(OwnedEvent::EndHeading);
//! let bytes = w.finish();
//! ```

use crate::ast::*;
use crate::events::{Event, OwnedEvent};
use std::io::Write;

/// Streaming AsciiDoc writer.
///
/// Feed events with [`write_event`](Writer::write_event), then call
/// [`finish`](Writer::finish) to flush AsciiDoc text to the underlying sink
/// and recover it.
pub struct Writer<W: Write> {
    sink: W,
    events: Vec<OwnedEvent>,
}

impl<W: Write> Writer<W> {
    pub fn new(sink: W) -> Self {
        Writer { sink, events: Vec::new() }
    }

    /// Feed one event to the writer.
    pub fn write_event(&mut self, event: Event<'_>) {
        self.events.push(event.into_owned());
    }

    /// Flush all buffered events as AsciiDoc text and return the sink.
    pub fn finish(mut self) -> W {
        let doc = events_to_doc(std::mem::take(&mut self.events));
        let text = crate::emit::build(&doc);
        let _ = self.sink.write_all(text.as_bytes());
        self.sink
    }
}

// ── Event → AST reconstruction ────────────────────────────────────────────────

fn events_to_doc(events: Vec<OwnedEvent>) -> AsciiDoc {
    let mut builder = DocBuilder::new();
    for event in events {
        builder.process(event);
    }
    builder.finish()
}

enum Frame {
    Document { blocks: Vec<Block> },
    Paragraph { inlines: Vec<Inline>, id: Option<String>, role: Option<String>, checked: Option<bool> },
    Heading { level: usize, inlines: Vec<Inline>, id: Option<String>, role: Option<String> },
    CodeBlock { language: Option<String>, content: String },
    Blockquote { children: Vec<Block>, attribution: Option<String> },
    List { ordered: bool, items: Vec<Vec<Block>>, style: Option<String> },
    ListItem { blocks: Vec<Block> },
    DefinitionList { items: Vec<DefinitionItem> },
    DefinitionTerm { inlines: Vec<Inline> },
    DefinitionDesc { inlines: Vec<Inline> },
    Div { class: Option<String>, title: Option<String>, children: Vec<Block> },
    Table { rows: Vec<TableRow> },
    TableRow { cells: Vec<Vec<Inline>>, is_header: bool },
    TableCell { inlines: Vec<Inline> },
    // Inline spans
    Strong { inlines: Vec<Inline> },
    Emphasis { inlines: Vec<Inline> },
    Superscript { inlines: Vec<Inline> },
    Subscript { inlines: Vec<Inline> },
    Highlight { inlines: Vec<Inline> },
    Strikeout { inlines: Vec<Inline> },
    Underline { inlines: Vec<Inline> },
    SmallCaps { inlines: Vec<Inline> },
    Quoted { quote_type: QuoteType, inlines: Vec<Inline> },
    Link { url: String, inlines: Vec<Inline>, target: Option<String> },
    FootnoteDef { label: String, inlines: Vec<Inline> },
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
            // ── Document boundary ─────────────────────────────────────────────
            OwnedEvent::StartDocument | OwnedEvent::EndDocument => {
                // StartDocument is a no-op (Document frame already on stack).
                // EndDocument is a no-op (finish() pops it).
            }

            // ── Block open/close ──────────────────────────────────────────────
            OwnedEvent::StartParagraph { id, role, checked } => {
                self.stack.push(Frame::Paragraph { inlines: vec![], id, role, checked });
            }
            OwnedEvent::EndParagraph => {
                if let Some(Frame::Paragraph { inlines, id, role, checked }) = self.stack.pop() {
                    self.push_block(Block::Paragraph {
                        inlines,
                        id,
                        role,
                        checked,
                        span: Span::NONE,
                    });
                }
            }
            OwnedEvent::StartHeading { level, id, role } => {
                self.stack.push(Frame::Heading { level, inlines: vec![], id, role });
            }
            OwnedEvent::EndHeading => {
                if let Some(Frame::Heading { level, inlines, id, role }) = self.stack.pop() {
                    self.push_block(Block::Heading {
                        level,
                        inlines,
                        id,
                        role,
                        span: Span::NONE,
                    });
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
                    self.push_block(Block::CodeBlock { language, content, span: Span::NONE });
                }
            }
            OwnedEvent::StartBlockquote { attribution } => {
                self.stack.push(Frame::Blockquote { children: vec![], attribution });
            }
            OwnedEvent::EndBlockquote => {
                if let Some(Frame::Blockquote { children, attribution }) = self.stack.pop() {
                    self.push_block(Block::Blockquote { children, attribution, span: Span::NONE });
                }
            }
            OwnedEvent::StartList { ordered, style } => {
                self.stack.push(Frame::List { ordered, items: vec![], style });
            }
            OwnedEvent::EndList => {
                if let Some(Frame::List { ordered, items, style }) = self.stack.pop() {
                    self.push_block(Block::List { ordered, items, style, span: Span::NONE });
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
                    // Append an incomplete DefinitionItem; desc will be filled in later.
                    if let Some(Frame::DefinitionList { items, .. }) = self.stack.last_mut() {
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
            OwnedEvent::HorizontalRule => {
                self.push_block(Block::HorizontalRule { span: Span::NONE });
            }
            OwnedEvent::PageBreak => {
                self.push_block(Block::PageBreak { span: Span::NONE });
            }
            OwnedEvent::Figure { url, alt, title: _ } => {
                self.push_block(Block::Figure {
                    image: ImageData { url, alt, width: None, height: None },
                    span: Span::NONE,
                });
            }
            OwnedEvent::StartDiv { class, title } => {
                self.stack.push(Frame::Div { class, title, children: vec![] });
            }
            OwnedEvent::EndDiv => {
                if let Some(Frame::Div { class, title, children }) = self.stack.pop() {
                    self.push_block(Block::Div { class, title, children, span: Span::NONE });
                }
            }
            OwnedEvent::RawBlock { format, content } => {
                self.push_block(Block::RawBlock { format, content, span: Span::NONE });
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

            // ── Inline events ─────────────────────────────────────────────────
            OwnedEvent::Text(cow) => {
                self.push_inline(Inline::Text { text: cow.into_owned(), span: Span::NONE });
            }
            OwnedEvent::SoftBreak => {
                self.push_inline(Inline::SoftBreak { span: Span::NONE });
            }
            OwnedEvent::LineBreak => {
                self.push_inline(Inline::LineBreak { span: Span::NONE });
            }
            OwnedEvent::StartStrong => {
                self.stack.push(Frame::Strong { inlines: vec![] });
            }
            OwnedEvent::EndStrong => {
                if let Some(Frame::Strong { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Strong(inlines, Span::NONE));
                }
            }
            OwnedEvent::StartEmphasis => {
                self.stack.push(Frame::Emphasis { inlines: vec![] });
            }
            OwnedEvent::EndEmphasis => {
                if let Some(Frame::Emphasis { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Emphasis(inlines, Span::NONE));
                }
            }
            OwnedEvent::Code(cow) => {
                self.push_inline(Inline::Code(cow.into_owned(), Span::NONE));
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
            OwnedEvent::StartHighlight => {
                self.stack.push(Frame::Highlight { inlines: vec![] });
            }
            OwnedEvent::EndHighlight => {
                if let Some(Frame::Highlight { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Highlight(inlines, Span::NONE));
                }
            }
            OwnedEvent::StartStrikeout => {
                self.stack.push(Frame::Strikeout { inlines: vec![] });
            }
            OwnedEvent::EndStrikeout => {
                if let Some(Frame::Strikeout { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Strikeout(inlines, Span::NONE));
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
            OwnedEvent::StartSmallCaps => {
                self.stack.push(Frame::SmallCaps { inlines: vec![] });
            }
            OwnedEvent::EndSmallCaps => {
                if let Some(Frame::SmallCaps { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::SmallCaps(inlines, Span::NONE));
                }
            }
            OwnedEvent::StartQuoted { quote_type } => {
                let qt = if quote_type == "single" {
                    QuoteType::Single
                } else {
                    QuoteType::Double
                };
                self.stack.push(Frame::Quoted { quote_type: qt, inlines: vec![] });
            }
            OwnedEvent::EndQuoted => {
                if let Some(Frame::Quoted { quote_type, inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Quoted {
                        quote_type,
                        children: inlines,
                        span: Span::NONE,
                    });
                }
            }
            OwnedEvent::StartLink { url, target } => {
                self.stack.push(Frame::Link { url, inlines: vec![], target });
            }
            OwnedEvent::EndLink => {
                if let Some(Frame::Link { url, inlines, target }) = self.stack.pop() {
                    self.push_inline(Inline::Link {
                        url,
                        children: inlines,
                        target,
                        span: Span::NONE,
                    });
                }
            }
            OwnedEvent::InlineImage { url, alt, title: _ } => {
                self.push_inline(Inline::Image(
                    ImageData { url, alt, width: None, height: None },
                    Span::NONE,
                ));
            }
            OwnedEvent::FootnoteRef { label } => {
                self.push_inline(Inline::FootnoteRef { label, span: Span::NONE });
            }
            OwnedEvent::StartFootnoteDef { label } => {
                self.stack.push(Frame::FootnoteDef { label, inlines: vec![] });
            }
            OwnedEvent::EndFootnoteDef => {
                if let Some(Frame::FootnoteDef { label, inlines }) = self.stack.pop() {
                    self.push_inline(Inline::FootnoteDef {
                        label,
                        children: inlines,
                        span: Span::NONE,
                    });
                }
            }
            OwnedEvent::MathInline { source } => {
                self.push_inline(Inline::MathInline { source, span: Span::NONE });
            }
            OwnedEvent::MathDisplay { source } => {
                self.push_inline(Inline::MathDisplay { source, span: Span::NONE });
            }
            OwnedEvent::RawInline { format, content } => {
                self.push_inline(Inline::RawInline { format, content, span: Span::NONE });
            }
            OwnedEvent::Anchor { id } => {
                self.push_inline(Inline::Anchor { id, span: Span::NONE });
            }
        }
    }

    fn push_block(&mut self, block: Block) {
        match self.stack.last_mut() {
            Some(Frame::Document { blocks }) => blocks.push(block),
            Some(Frame::Blockquote { children, .. }) => children.push(block),
            Some(Frame::ListItem { blocks }) => blocks.push(block),
            Some(Frame::Div { children, .. }) => children.push(block),
            _ => {} // unexpected context, discard
        }
    }

    fn push_inline(&mut self, inline: Inline) {
        match self.stack.last_mut() {
            Some(Frame::Paragraph { inlines, .. }) => inlines.push(inline),
            Some(Frame::Heading { inlines, .. }) => inlines.push(inline),
            Some(Frame::Strong { inlines }) => inlines.push(inline),
            Some(Frame::Emphasis { inlines }) => inlines.push(inline),
            Some(Frame::Superscript { inlines }) => inlines.push(inline),
            Some(Frame::Subscript { inlines }) => inlines.push(inline),
            Some(Frame::Highlight { inlines }) => inlines.push(inline),
            Some(Frame::Strikeout { inlines }) => inlines.push(inline),
            Some(Frame::Underline { inlines }) => inlines.push(inline),
            Some(Frame::SmallCaps { inlines }) => inlines.push(inline),
            Some(Frame::Quoted { inlines, .. }) => inlines.push(inline),
            Some(Frame::Link { inlines, .. }) => inlines.push(inline),
            Some(Frame::FootnoteDef { inlines, .. }) => inlines.push(inline),
            Some(Frame::TableCell { inlines }) => inlines.push(inline),
            Some(Frame::DefinitionTerm { inlines }) => inlines.push(inline),
            Some(Frame::DefinitionDesc { inlines }) => inlines.push(inline),
            _ => {} // unexpected context, discard
        }
    }

    fn finish(mut self) -> AsciiDoc {
        let blocks = match self.stack.pop() {
            Some(Frame::Document { blocks }) => blocks,
            _ => vec![],
        };
        AsciiDoc { blocks, attributes: Default::default(), span: Span::NONE }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_writer_heading() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedEvent::StartHeading { level: 1, id: None, role: None });
        w.write_event(OwnedEvent::Text(std::borrow::Cow::Owned("Hello".to_string())));
        w.write_event(OwnedEvent::EndHeading);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("Hello"), "got: {s:?}");
    }

    #[test]
    fn test_writer_paragraph() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedEvent::StartParagraph { id: None, role: None, checked: None });
        w.write_event(OwnedEvent::Text(std::borrow::Cow::Owned("World".to_string())));
        w.write_event(OwnedEvent::EndParagraph);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("World"), "got: {s:?}");
    }

    #[test]
    fn test_writer_roundtrip_via_events() {
        // Use paragraph + list only; heading roundtrip depends on parse/emit level
        // convention which is tested separately.
        let input = "A paragraph with *strong* text.\n\n* item one\n* item two\n";
        let evts: Vec<_> = crate::events(input).collect();
        let mut w = Writer::new(Vec::<u8>::new());
        for e in evts {
            w.write_event(e);
        }
        let bytes = w.finish();
        let emitted_text = String::from_utf8(bytes).unwrap();
        // Re-parse the emitted text and emit again; the two emissions must match.
        let evts2: Vec<_> = crate::events(&emitted_text).collect();
        let mut w2 = Writer::new(Vec::<u8>::new());
        for e in evts2 {
            w2.write_event(e);
        }
        let bytes2 = w2.finish();
        let emitted2 = String::from_utf8(bytes2).unwrap();
        assert_eq!(emitted_text, emitted2, "writer roundtrip mismatch");
    }
}
