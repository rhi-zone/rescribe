#![allow(clippy::collapsible_if)]
//! Streaming Djot writer — converts a stream of events to Djot text.
//!
//! This implementation buffers all events, reconstructs the AST, then emits.
//! Future versions may emit bytes incrementally at block boundaries.
//!
//! # Example
//! ```no_run
//! use djot_fmt::writer::Writer;
//! use djot_fmt::OwnedEvent;
//! use std::borrow::Cow;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! w.write_event(OwnedEvent::StartHeading { level: 1, id: None, classes: vec![], kv: vec![] });
//! w.write_event(OwnedEvent::Text(Cow::Owned("Hello".to_string())));
//! w.write_event(OwnedEvent::EndHeading);
//! let bytes = w.finish();
//! ```

use crate::ast::*;
use crate::events::{Event, OwnedEvent};
use std::io::Write;

/// Streaming Djot writer.
///
/// Feed events with [`write_event`](Writer::write_event), then call
/// [`finish`](Writer::finish) to flush Djot text to the underlying sink and
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
    pub fn write_event(&mut self, event: Event<'_>) {
        self.events.push(event.into_owned());
    }

    /// Flush all buffered events as Djot text and return the sink.
    pub fn finish(mut self) -> W {
        let doc = events_to_doc(std::mem::take(&mut self.events));
        let text = crate::emit::emit(&doc);
        let _ = self.sink.write_all(text.as_bytes());
        self.sink
    }
}

// ── Event → AST reconstruction ────────────────────────────────────────────────

fn events_to_doc(events: Vec<OwnedEvent>) -> DjotDoc {
    let mut builder = DocBuilder::new();
    for event in events {
        builder.process(event);
    }
    builder.finish()
}

enum Frame {
    Document { blocks: Vec<Block> },
    Paragraph { inlines: Vec<Inline>, attr: Attr },
    Heading { level: u8, inlines: Vec<Inline>, attr: Attr },
    Blockquote { blocks: Vec<Block>, attr: Attr },
    List { kind: ListKind, items: Vec<ListItem>, tight: bool, attr: Attr },
    ListItem { blocks: Vec<Block>, checked: Option<bool> },
    CodeBlock { language: Option<String>, content: String, attr: Attr },
    Div { class: Option<String>, blocks: Vec<Block>, attr: Attr },
    Table { caption: Option<Vec<Inline>>, rows: Vec<TableRow> },
    TablePendingCaption(Vec<Inline>),
    TableRow { cells: Vec<TableCell>, is_header: bool },
    TableCell { inlines: Vec<Inline>, alignment: Alignment },
    DefinitionList { items: Vec<DefItem>, attr: Attr },
    DefinitionTerm { inlines: Vec<Inline> },
    DefinitionDesc { blocks: Vec<Block> },
    FootnoteDef { label: String, blocks: Vec<Block> },
    // Inline spans
    Emphasis { inlines: Vec<Inline>, attr: Attr },
    Strong { inlines: Vec<Inline>, attr: Attr },
    Delete { inlines: Vec<Inline>, attr: Attr },
    Insert { inlines: Vec<Inline>, attr: Attr },
    Highlight { inlines: Vec<Inline>, attr: Attr },
    Subscript { inlines: Vec<Inline>, attr: Attr },
    Superscript { inlines: Vec<Inline>, attr: Attr },
    Link { inlines: Vec<Inline>, url: String, title: Option<String>, attr: Attr },
    Image { inlines: Vec<Inline>, url: String, title: Option<String>, attr: Attr },
    Span { inlines: Vec<Inline>, attr: Attr },
}

struct DocBuilder {
    stack: Vec<Frame>,
    footnotes: Vec<FootnoteDef>,
    link_defs: Vec<LinkDef>,
}

impl DocBuilder {
    fn new() -> Self {
        DocBuilder {
            stack: vec![Frame::Document { blocks: vec![] }],
            footnotes: vec![],
            link_defs: vec![],
        }
    }

    #[allow(clippy::too_many_lines)]
    fn process(&mut self, event: OwnedEvent) {
        match event {
            // ── Block open/close ───────────────────────────────────────────────
            OwnedEvent::StartParagraph { id, classes, kv } => {
                self.stack.push(Frame::Paragraph { inlines: vec![], attr: Attr { id, classes, kv } });
            }
            OwnedEvent::EndParagraph => {
                if let Some(Frame::Paragraph { inlines, attr }) = self.stack.pop() {
                    self.push_block(Block::Paragraph { inlines, attr, span: Span::NONE });
                }
            }
            OwnedEvent::StartHeading { level, id, classes, kv } => {
                self.stack.push(Frame::Heading { level, inlines: vec![], attr: Attr { id, classes, kv } });
            }
            OwnedEvent::EndHeading => {
                if let Some(Frame::Heading { level, inlines, attr }) = self.stack.pop() {
                    self.push_block(Block::Heading { level, inlines, attr, span: Span::NONE });
                }
            }
            OwnedEvent::StartBlockquote { id, classes, kv } => {
                self.stack.push(Frame::Blockquote { blocks: vec![], attr: Attr { id, classes, kv } });
            }
            OwnedEvent::EndBlockquote => {
                if let Some(Frame::Blockquote { blocks, attr }) = self.stack.pop() {
                    self.push_block(Block::Blockquote { blocks, attr, span: Span::NONE });
                }
            }
            OwnedEvent::StartList { kind, tight, id, classes, kv } => {
                self.stack.push(Frame::List { kind, items: vec![], tight, attr: Attr { id, classes, kv } });
            }
            OwnedEvent::EndList => {
                if let Some(Frame::List { kind, items, tight, attr }) = self.stack.pop() {
                    self.push_block(Block::List { kind, items, tight, attr, span: Span::NONE });
                }
            }
            OwnedEvent::StartListItem { checked } => {
                self.stack.push(Frame::ListItem { blocks: vec![], checked });
            }
            OwnedEvent::EndListItem => {
                if let Some(Frame::ListItem { blocks, checked }) = self.stack.pop() {
                    if let Some(Frame::List { items, .. }) = self.stack.last_mut() {
                        items.push(ListItem { blocks, checked, span: Span::NONE });
                    }
                }
            }
            OwnedEvent::StartCodeBlock { language, id, classes, kv } => {
                self.stack.push(Frame::CodeBlock {
                    language,
                    content: String::new(),
                    attr: Attr { id, classes, kv },
                });
            }
            OwnedEvent::CodeBlockContent(cow) => {
                if let Some(Frame::CodeBlock { content: buf, .. }) = self.stack.last_mut() {
                    buf.push_str(&cow);
                }
            }
            OwnedEvent::EndCodeBlock => {
                if let Some(Frame::CodeBlock { language, content, attr }) = self.stack.pop() {
                    self.push_block(Block::CodeBlock { language, content, attr, span: Span::NONE });
                }
            }
            OwnedEvent::RawBlock { format, content } => {
                self.push_block(Block::RawBlock { format, content, attr: Attr::default(), span: Span::NONE });
            }
            OwnedEvent::StartDiv { class, id, classes, kv } => {
                self.stack.push(Frame::Div { class, blocks: vec![], attr: Attr { id, classes, kv } });
            }
            OwnedEvent::EndDiv => {
                if let Some(Frame::Div { class, blocks, attr }) = self.stack.pop() {
                    self.push_block(Block::Div { class, blocks, attr, span: Span::NONE });
                }
            }
            OwnedEvent::TableCaption(inlines) => {
                self.stack.push(Frame::TablePendingCaption(inlines));
            }
            OwnedEvent::StartTable => {
                let caption = if matches!(self.stack.last(), Some(Frame::TablePendingCaption(_))) {
                    if let Some(Frame::TablePendingCaption(cap)) = self.stack.pop() { Some(cap) } else { None }
                } else {
                    None
                };
                self.stack.push(Frame::Table { caption, rows: vec![] });
            }
            OwnedEvent::EndTable => {
                if let Some(Frame::Table { caption, rows }) = self.stack.pop() {
                    self.push_block(Block::Table { caption, rows, span: Span::NONE });
                }
            }
            OwnedEvent::StartTableRow { is_header } => {
                self.stack.push(Frame::TableRow { cells: vec![], is_header });
            }
            OwnedEvent::EndTableRow => {
                if let Some(Frame::TableRow { cells, is_header }) = self.stack.pop() {
                    if let Some(Frame::Table { rows, .. }) = self.stack.last_mut() {
                        rows.push(TableRow { cells, is_header, span: Span::NONE });
                    }
                }
            }
            OwnedEvent::StartTableCell { alignment } => {
                self.stack.push(Frame::TableCell { inlines: vec![], alignment });
            }
            OwnedEvent::EndTableCell => {
                if let Some(Frame::TableCell { inlines, alignment }) = self.stack.pop() {
                    if let Some(Frame::TableRow { cells, .. }) = self.stack.last_mut() {
                        cells.push(TableCell { inlines, alignment, span: Span::NONE });
                    }
                }
            }
            OwnedEvent::ThematicBreak { id, classes, kv } => {
                self.push_block(Block::ThematicBreak { attr: Attr { id, classes, kv }, span: Span::NONE });
            }
            OwnedEvent::StartDefinitionList { id, classes, kv } => {
                self.stack.push(Frame::DefinitionList { items: vec![], attr: Attr { id, classes, kv } });
            }
            OwnedEvent::EndDefinitionList => {
                if let Some(Frame::DefinitionList { items, attr }) = self.stack.pop() {
                    self.push_block(Block::DefinitionList { items, attr, span: Span::NONE });
                }
            }
            OwnedEvent::StartDefinitionTerm => {
                self.stack.push(Frame::DefinitionTerm { inlines: vec![] });
            }
            OwnedEvent::EndDefinitionTerm => {
                if let Some(Frame::DefinitionTerm { inlines }) = self.stack.pop() {
                    // Append an incomplete DefItem; desc will be filled in by EndDefinitionDesc
                    if let Some(Frame::DefinitionList { items, .. }) = self.stack.last_mut() {
                        items.push(DefItem { term: inlines, definitions: vec![], span: Span::NONE });
                    }
                }
            }
            OwnedEvent::StartDefinitionDesc => {
                self.stack.push(Frame::DefinitionDesc { blocks: vec![] });
            }
            OwnedEvent::EndDefinitionDesc => {
                if let Some(Frame::DefinitionDesc { blocks }) = self.stack.pop() {
                    if let Some(Frame::DefinitionList { items, .. }) = self.stack.last_mut() {
                        if let Some(last) = items.last_mut() {
                            last.definitions = blocks;
                        }
                    }
                }
            }
            OwnedEvent::StartFootnoteDef { label } => {
                self.stack.push(Frame::FootnoteDef { label, blocks: vec![] });
            }
            OwnedEvent::EndFootnoteDef => {
                if let Some(Frame::FootnoteDef { label, blocks }) = self.stack.pop() {
                    self.footnotes.push(FootnoteDef { label, blocks, span: Span::NONE });
                }
            }

            // ── Inline events ──────────────────────────────────────────────────
            OwnedEvent::Text(cow) => {
                self.push_inline(Inline::Text { content: cow.into_owned(), span: Span::NONE });
            }
            OwnedEvent::SoftBreak => {
                self.push_inline(Inline::SoftBreak { span: Span::NONE });
            }
            OwnedEvent::HardBreak => {
                self.push_inline(Inline::HardBreak { span: Span::NONE });
            }
            OwnedEvent::StartEmphasis { id, classes, kv } => {
                self.stack.push(Frame::Emphasis { inlines: vec![], attr: Attr { id, classes, kv } });
            }
            OwnedEvent::EndEmphasis => {
                if let Some(Frame::Emphasis { inlines, attr }) = self.stack.pop() {
                    self.push_inline(Inline::Emphasis { inlines, attr, span: Span::NONE });
                }
            }
            OwnedEvent::StartStrong { id, classes, kv } => {
                self.stack.push(Frame::Strong { inlines: vec![], attr: Attr { id, classes, kv } });
            }
            OwnedEvent::EndStrong => {
                if let Some(Frame::Strong { inlines, attr }) = self.stack.pop() {
                    self.push_inline(Inline::Strong { inlines, attr, span: Span::NONE });
                }
            }
            OwnedEvent::StartDelete { id, classes, kv } => {
                self.stack.push(Frame::Delete { inlines: vec![], attr: Attr { id, classes, kv } });
            }
            OwnedEvent::EndDelete => {
                if let Some(Frame::Delete { inlines, attr }) = self.stack.pop() {
                    self.push_inline(Inline::Delete { inlines, attr, span: Span::NONE });
                }
            }
            OwnedEvent::StartInsert { id, classes, kv } => {
                self.stack.push(Frame::Insert { inlines: vec![], attr: Attr { id, classes, kv } });
            }
            OwnedEvent::EndInsert => {
                if let Some(Frame::Insert { inlines, attr }) = self.stack.pop() {
                    self.push_inline(Inline::Insert { inlines, attr, span: Span::NONE });
                }
            }
            OwnedEvent::StartHighlight { id, classes, kv } => {
                self.stack.push(Frame::Highlight { inlines: vec![], attr: Attr { id, classes, kv } });
            }
            OwnedEvent::EndHighlight => {
                if let Some(Frame::Highlight { inlines, attr }) = self.stack.pop() {
                    self.push_inline(Inline::Highlight { inlines, attr, span: Span::NONE });
                }
            }
            OwnedEvent::StartSubscript { id, classes, kv } => {
                self.stack.push(Frame::Subscript { inlines: vec![], attr: Attr { id, classes, kv } });
            }
            OwnedEvent::EndSubscript => {
                if let Some(Frame::Subscript { inlines, attr }) = self.stack.pop() {
                    self.push_inline(Inline::Subscript { inlines, attr, span: Span::NONE });
                }
            }
            OwnedEvent::StartSuperscript { id, classes, kv } => {
                self.stack.push(Frame::Superscript { inlines: vec![], attr: Attr { id, classes, kv } });
            }
            OwnedEvent::EndSuperscript => {
                if let Some(Frame::Superscript { inlines, attr }) = self.stack.pop() {
                    self.push_inline(Inline::Superscript { inlines, attr, span: Span::NONE });
                }
            }
            OwnedEvent::Verbatim { content, id, classes, kv } => {
                self.push_inline(Inline::Verbatim { content: content.into_owned(), attr: Attr { id, classes, kv }, span: Span::NONE });
            }
            OwnedEvent::MathInline(cow) => {
                self.push_inline(Inline::MathInline { content: cow.into_owned(), span: Span::NONE });
            }
            OwnedEvent::MathDisplay(cow) => {
                self.push_inline(Inline::MathDisplay { content: cow.into_owned(), span: Span::NONE });
            }
            OwnedEvent::RawInline { format, content } => {
                self.push_inline(Inline::RawInline { format, content, span: Span::NONE });
            }
            OwnedEvent::StartLink { url, title, id, classes, kv } => {
                self.stack.push(Frame::Link { inlines: vec![], url, title, attr: Attr { id, classes, kv } });
            }
            OwnedEvent::EndLink => {
                if let Some(Frame::Link { inlines, url, title, attr }) = self.stack.pop() {
                    self.push_inline(Inline::Link { inlines, url, title, attr, span: Span::NONE });
                }
            }
            OwnedEvent::StartImage { url, title, id, classes, kv } => {
                self.stack.push(Frame::Image { inlines: vec![], url, title, attr: Attr { id, classes, kv } });
            }
            OwnedEvent::EndImage => {
                if let Some(Frame::Image { inlines, url, title, attr }) = self.stack.pop() {
                    self.push_inline(Inline::Image { inlines, url, title, attr, span: Span::NONE });
                }
            }
            OwnedEvent::StartSpan { id, classes, kv } => {
                self.stack.push(Frame::Span { inlines: vec![], attr: Attr { id, classes, kv } });
            }
            OwnedEvent::EndSpan => {
                if let Some(Frame::Span { inlines, attr }) = self.stack.pop() {
                    self.push_inline(Inline::Span { inlines, attr, span: Span::NONE });
                }
            }
            OwnedEvent::FootnoteRef(label) => {
                self.push_inline(Inline::FootnoteRef { label, span: Span::NONE });
            }
            OwnedEvent::Symbol(name) => {
                self.push_inline(Inline::Symbol { name, span: Span::NONE });
            }
            OwnedEvent::Autolink { url, is_email } => {
                self.push_inline(Inline::Autolink { url, is_email, span: Span::NONE });
            }
        }
    }

    fn push_block(&mut self, block: Block) {
        match self.stack.last_mut() {
            Some(Frame::Document { blocks }) => blocks.push(block),
            Some(Frame::Blockquote { blocks, .. }) => blocks.push(block),
            Some(Frame::ListItem { blocks, .. }) => blocks.push(block),
            Some(Frame::Div { blocks, .. }) => blocks.push(block),
            Some(Frame::DefinitionDesc { blocks }) => blocks.push(block),
            Some(Frame::FootnoteDef { blocks, .. }) => blocks.push(block),
            _ => {} // unexpected context, discard
        }
    }

    fn push_inline(&mut self, inline: Inline) {
        match self.stack.last_mut() {
            Some(Frame::Paragraph { inlines, .. }) => inlines.push(inline),
            Some(Frame::Heading { inlines, .. }) => inlines.push(inline),
            Some(Frame::Emphasis { inlines, .. }) => inlines.push(inline),
            Some(Frame::Strong { inlines, .. }) => inlines.push(inline),
            Some(Frame::Delete { inlines, .. }) => inlines.push(inline),
            Some(Frame::Insert { inlines, .. }) => inlines.push(inline),
            Some(Frame::Highlight { inlines, .. }) => inlines.push(inline),
            Some(Frame::Subscript { inlines, .. }) => inlines.push(inline),
            Some(Frame::Superscript { inlines, .. }) => inlines.push(inline),
            Some(Frame::Link { inlines, .. }) => inlines.push(inline),
            Some(Frame::Image { inlines, .. }) => inlines.push(inline),
            Some(Frame::Span { inlines, .. }) => inlines.push(inline),
            Some(Frame::TableCell { inlines, .. }) => inlines.push(inline),
            Some(Frame::DefinitionTerm { inlines }) => inlines.push(inline),
            _ => {} // unexpected context, discard
        }
    }

    fn finish(mut self) -> DjotDoc {
        let blocks = match self.stack.pop() {
            Some(Frame::Document { blocks }) => blocks,
            _ => vec![],
        };
        DjotDoc { blocks, footnotes: self.footnotes, link_defs: self.link_defs }
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
            id: None,
            classes: vec![],
            kv: vec![],
        });
        w.write_event(OwnedEvent::Text(std::borrow::Cow::Owned("Hello".to_string())));
        w.write_event(OwnedEvent::EndHeading);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("# Hello"), "got: {s:?}");
    }

    #[test]
    fn test_writer_paragraph() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedEvent::StartParagraph { id: None, classes: vec![], kv: vec![] });
        w.write_event(OwnedEvent::Text(std::borrow::Cow::Owned("World".to_string())));
        w.write_event(OwnedEvent::EndParagraph);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("World"), "got: {s:?}");
    }

    #[test]
    fn test_writer_roundtrip_via_events() {
        let input = "# Hello\n\nA paragraph with *strong* text.\n\n- item one\n- item two\n";
        let (doc, _) = crate::parse::parse(input);
        let evts: Vec<_> = crate::events(input).collect();
        let mut w = Writer::new(Vec::<u8>::new());
        for e in evts {
            w.write_event(e);
        }
        let bytes = w.finish();
        let emitted_text = String::from_utf8(bytes).unwrap();
        let (doc2, _) = crate::parse::parse(&emitted_text);
        assert_eq!(doc.strip_spans(), doc2.strip_spans(), "writer roundtrip mismatch");
    }

    #[test]
    fn test_writer_table_caption_roundtrip() {
        // Verify that a table with a caption survives the full
        // parse → events → Writer → parse roundtrip.
        let input = "^ Caption text\n| A | B |\n|---|---|\n| x | y |\n";
        let (doc, _) = crate::parse::parse(input);

        // Confirm parse captured the caption.
        match &doc.blocks[0] {
            crate::ast::Block::Table { caption: Some(_), .. } => {}
            other => panic!("Expected Table with caption after direct parse, got {:?}", other),
        }

        // Reconstruct via streaming events.
        let evts: Vec<_> = crate::events(input).collect();
        let mut w = Writer::new(Vec::<u8>::new());
        for e in evts {
            w.write_event(e);
        }
        let bytes = w.finish();
        let emitted_text = String::from_utf8(bytes).unwrap();
        let (doc2, _) = crate::parse::parse(&emitted_text);

        assert_eq!(
            doc.strip_spans(),
            doc2.strip_spans(),
            "table caption lost in event path; emitted: {emitted_text:?}"
        );
    }
}
