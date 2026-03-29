//! Streaming Muse writer — converts a stream of events to Muse text.
//!
//! This implementation buffers all events, reconstructs the AST, then emits
//! using [`crate::emit::build`].
//!
//! # Example
//! ```no_run
//! use muse_fmt::writer::Writer;
//! use muse_fmt::OwnedMuseEvent;
//! use std::borrow::Cow;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! w.write_event(OwnedMuseEvent::StartHeading { level: 1 });
//! w.write_event(OwnedMuseEvent::Text(Cow::Owned("Hello".to_string())));
//! w.write_event(OwnedMuseEvent::EndHeading);
//! let bytes = w.finish();
//! ```

use crate::ast::*;
use crate::events::OwnedMuseEvent;
use std::io::Write;

/// Streaming Muse writer.
///
/// Feed events with [`write_event`](Writer::write_event), then call
/// [`finish`](Writer::finish) to flush Muse text to the underlying sink and
/// recover it.
pub struct Writer<W: Write> {
    sink: W,
    events: Vec<OwnedMuseEvent>,
}

impl<W: Write> Writer<W> {
    pub fn new(sink: W) -> Self {
        Writer {
            sink,
            events: Vec::new(),
        }
    }

    /// Feed one event to the writer.
    pub fn write_event(&mut self, event: OwnedMuseEvent) {
        self.events.push(event);
    }

    /// Flush all buffered events as Muse text and return the sink.
    pub fn finish(mut self) -> W {
        let doc = events_to_doc(std::mem::take(&mut self.events));
        let text = crate::emit::build(&doc);
        let _ = self.sink.write_all(text.as_bytes());
        self.sink
    }
}

// ── Event -> AST reconstruction ──────────────────────────────────────────────

fn events_to_doc(events: Vec<OwnedMuseEvent>) -> MuseDoc {
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
    Heading {
        level: u8,
        inlines: Vec<Inline>,
    },
    Blockquote {
        blocks: Vec<Block>,
    },
    Verse {
        blocks: Vec<Block>,
    },
    CenteredBlock {
        blocks: Vec<Block>,
    },
    RightBlock {
        blocks: Vec<Block>,
    },
    List {
        ordered: bool,
        items: Vec<Vec<Block>>,
    },
    ListItem {
        blocks: Vec<Block>,
    },
    DefinitionList {
        items: Vec<(Vec<Inline>, Vec<Block>)>,
    },
    DefinitionTerm {
        inlines: Vec<Inline>,
    },
    DefinitionDesc {
        blocks: Vec<Block>,
    },
    Table {
        rows: Vec<TableRow>,
    },
    TableRow {
        header: bool,
        cells: Vec<Vec<Inline>>,
    },
    TableCell {
        inlines: Vec<Inline>,
    },
    FootnoteDef {
        label: String,
        inlines: Vec<Inline>,
    },
    // Inline span frames
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
    Superscript {
        inlines: Vec<Inline>,
    },
    Subscript {
        inlines: Vec<Inline>,
    },
    Link {
        url: String,
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

    #[allow(clippy::too_many_lines)]
    fn process(&mut self, event: OwnedMuseEvent) {
        match event {
            // ── Document wrapper (ignored for reconstruction) ─────────────
            OwnedMuseEvent::StartDocument | OwnedMuseEvent::EndDocument => {}

            // ── Block open ────────────────────────────────────────────────
            OwnedMuseEvent::StartParagraph => {
                self.stack.push(Frame::Paragraph { inlines: vec![] });
            }
            OwnedMuseEvent::EndParagraph => {
                if let Some(Frame::Paragraph { inlines }) = self.stack.pop() {
                    self.push_block(Block::Paragraph {
                        inlines,
                        span: Span::NONE,
                    });
                }
            }
            OwnedMuseEvent::StartHeading { level } => {
                self.stack.push(Frame::Heading {
                    level,
                    inlines: vec![],
                });
            }
            OwnedMuseEvent::EndHeading => {
                if let Some(Frame::Heading { level, inlines }) = self.stack.pop() {
                    self.push_block(Block::Heading {
                        level,
                        inlines,
                        span: Span::NONE,
                    });
                }
            }
            OwnedMuseEvent::StartBlockquote => {
                self.stack.push(Frame::Blockquote { blocks: vec![] });
            }
            OwnedMuseEvent::EndBlockquote => {
                if let Some(Frame::Blockquote { blocks }) = self.stack.pop() {
                    self.push_block(Block::Blockquote {
                        children: blocks,
                        span: Span::NONE,
                    });
                }
            }
            OwnedMuseEvent::StartVerse => {
                self.stack.push(Frame::Verse { blocks: vec![] });
            }
            OwnedMuseEvent::EndVerse => {
                if let Some(Frame::Verse { blocks }) = self.stack.pop() {
                    self.push_block(Block::Verse {
                        children: blocks,
                        span: Span::NONE,
                    });
                }
            }
            OwnedMuseEvent::StartCenteredBlock => {
                self.stack.push(Frame::CenteredBlock { blocks: vec![] });
            }
            OwnedMuseEvent::EndCenteredBlock => {
                if let Some(Frame::CenteredBlock { blocks }) = self.stack.pop() {
                    self.push_block(Block::CenteredBlock {
                        children: blocks,
                        span: Span::NONE,
                    });
                }
            }
            OwnedMuseEvent::StartRightBlock => {
                self.stack.push(Frame::RightBlock { blocks: vec![] });
            }
            OwnedMuseEvent::EndRightBlock => {
                if let Some(Frame::RightBlock { blocks }) = self.stack.pop() {
                    self.push_block(Block::RightBlock {
                        children: blocks,
                        span: Span::NONE,
                    });
                }
            }
            OwnedMuseEvent::StartList { ordered } => {
                self.stack.push(Frame::List {
                    ordered,
                    items: vec![],
                });
            }
            OwnedMuseEvent::EndList => {
                if let Some(Frame::List { ordered, items }) = self.stack.pop() {
                    self.push_block(Block::List {
                        ordered,
                        items,
                        span: Span::NONE,
                    });
                }
            }
            OwnedMuseEvent::StartListItem => {
                self.stack.push(Frame::ListItem { blocks: vec![] });
            }
            OwnedMuseEvent::EndListItem => {
                if let Some(Frame::ListItem { blocks }) = self.stack.pop()
                    && let Some(Frame::List { items, .. }) = self.stack.last_mut()
                {
                    items.push(blocks);
                }
            }
            OwnedMuseEvent::StartDefinitionList => {
                self.stack
                    .push(Frame::DefinitionList { items: vec![] });
            }
            OwnedMuseEvent::EndDefinitionList => {
                if let Some(Frame::DefinitionList { items }) = self.stack.pop() {
                    self.push_block(Block::DefinitionList {
                        items,
                        span: Span::NONE,
                    });
                }
            }
            OwnedMuseEvent::StartDefinitionTerm => {
                self.stack
                    .push(Frame::DefinitionTerm { inlines: vec![] });
            }
            OwnedMuseEvent::EndDefinitionTerm => {
                if let Some(Frame::DefinitionTerm { inlines }) = self.stack.pop()
                    && let Some(Frame::DefinitionList { items }) = self.stack.last_mut()
                {
                    items.push((inlines, vec![]));
                }
            }
            OwnedMuseEvent::StartDefinitionDesc => {
                self.stack
                    .push(Frame::DefinitionDesc { blocks: vec![] });
            }
            OwnedMuseEvent::EndDefinitionDesc => {
                if let Some(Frame::DefinitionDesc { blocks }) = self.stack.pop()
                    && let Some(Frame::DefinitionList { items }) = self.stack.last_mut()
                    && let Some(last) = items.last_mut()
                {
                    last.1 = blocks;
                }
            }
            OwnedMuseEvent::StartTable => {
                self.stack.push(Frame::Table { rows: vec![] });
            }
            OwnedMuseEvent::EndTable => {
                if let Some(Frame::Table { rows }) = self.stack.pop() {
                    self.push_block(Block::Table {
                        rows,
                        span: Span::NONE,
                    });
                }
            }
            OwnedMuseEvent::StartTableRow { header } => {
                self.stack.push(Frame::TableRow {
                    header,
                    cells: vec![],
                });
            }
            OwnedMuseEvent::EndTableRow => {
                if let Some(Frame::TableRow { header, cells }) = self.stack.pop()
                    && let Some(Frame::Table { rows }) = self.stack.last_mut()
                {
                    rows.push(TableRow { cells, header });
                }
            }
            OwnedMuseEvent::StartTableCell => {
                self.stack.push(Frame::TableCell { inlines: vec![] });
            }
            OwnedMuseEvent::EndTableCell => {
                if let Some(Frame::TableCell { inlines }) = self.stack.pop()
                    && let Some(Frame::TableRow { cells, .. }) = self.stack.last_mut()
                {
                    cells.push(inlines);
                }
            }
            OwnedMuseEvent::StartFootnoteDef { label } => {
                self.stack.push(Frame::FootnoteDef {
                    label: label.into_owned(),
                    inlines: vec![],
                });
            }
            OwnedMuseEvent::EndFootnoteDef => {
                if let Some(Frame::FootnoteDef { label, inlines }) = self.stack.pop() {
                    self.push_block(Block::FootnoteDef {
                        label,
                        content: inlines,
                        span: Span::NONE,
                    });
                }
            }
            OwnedMuseEvent::HorizontalRule => {
                self.push_block(Block::HorizontalRule { span: Span::NONE });
            }

            // ── Leaf block events ─────────────────────────────────────────
            OwnedMuseEvent::LiteralBlock { content } => {
                self.push_block(Block::LiteralBlock {
                    content: content.into_owned(),
                    span: Span::NONE,
                });
            }
            OwnedMuseEvent::SrcBlock { lang, content } => {
                self.push_block(Block::SrcBlock {
                    lang: lang.map(|l| l.into_owned()),
                    content: content.into_owned(),
                    span: Span::NONE,
                });
            }
            OwnedMuseEvent::CodeBlock { content } => {
                self.push_block(Block::CodeBlock {
                    content: content.into_owned(),
                    span: Span::NONE,
                });
            }
            OwnedMuseEvent::Comment { content } => {
                self.push_block(Block::Comment {
                    content: content.into_owned(),
                    span: Span::NONE,
                });
            }

            // ── Inline events ─────────────────────────────────────────────
            OwnedMuseEvent::Text(cow) => {
                self.push_inline(Inline::Text(cow.into_owned(), Span::NONE));
            }
            OwnedMuseEvent::StartBold => {
                self.stack.push(Frame::Bold { inlines: vec![] });
            }
            OwnedMuseEvent::EndBold => {
                if let Some(Frame::Bold { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Bold(inlines, Span::NONE));
                }
            }
            OwnedMuseEvent::StartItalic => {
                self.stack.push(Frame::Italic { inlines: vec![] });
            }
            OwnedMuseEvent::EndItalic => {
                if let Some(Frame::Italic { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Italic(inlines, Span::NONE));
                }
            }
            OwnedMuseEvent::StartUnderline => {
                self.stack.push(Frame::Underline { inlines: vec![] });
            }
            OwnedMuseEvent::EndUnderline => {
                if let Some(Frame::Underline { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Underline(inlines, Span::NONE));
                }
            }
            OwnedMuseEvent::StartStrikethrough => {
                self.stack
                    .push(Frame::Strikethrough { inlines: vec![] });
            }
            OwnedMuseEvent::EndStrikethrough => {
                if let Some(Frame::Strikethrough { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Strikethrough(inlines, Span::NONE));
                }
            }
            OwnedMuseEvent::StartSuperscript => {
                self.stack.push(Frame::Superscript { inlines: vec![] });
            }
            OwnedMuseEvent::EndSuperscript => {
                if let Some(Frame::Superscript { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Superscript(inlines, Span::NONE));
                }
            }
            OwnedMuseEvent::StartSubscript => {
                self.stack.push(Frame::Subscript { inlines: vec![] });
            }
            OwnedMuseEvent::EndSubscript => {
                if let Some(Frame::Subscript { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Subscript(inlines, Span::NONE));
                }
            }
            OwnedMuseEvent::Code(cow) => {
                self.push_inline(Inline::Code(cow.into_owned(), Span::NONE));
            }
            OwnedMuseEvent::StartLink { url } => {
                self.stack.push(Frame::Link {
                    url: url.into_owned(),
                    inlines: vec![],
                });
            }
            OwnedMuseEvent::EndLink => {
                if let Some(Frame::Link { url, inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Link {
                        url,
                        children: inlines,
                        span: Span::NONE,
                    });
                }
            }
            OwnedMuseEvent::FootnoteRef { label } => {
                self.push_inline(Inline::FootnoteRef {
                    label: label.into_owned(),
                    span: Span::NONE,
                });
            }
            OwnedMuseEvent::LineBreak => {
                self.push_inline(Inline::LineBreak(Span::NONE));
            }
            OwnedMuseEvent::Anchor { name } => {
                self.push_inline(Inline::Anchor {
                    name: name.into_owned(),
                    span: Span::NONE,
                });
            }
            OwnedMuseEvent::Image { src, alt } => {
                self.push_inline(Inline::Image {
                    src: src.into_owned(),
                    alt: alt.map(|a| a.into_owned()),
                    span: Span::NONE,
                });
            }
        }
    }

    fn push_block(&mut self, block: Block) {
        match self.stack.last_mut() {
            Some(Frame::Document { blocks }) => blocks.push(block),
            Some(Frame::Blockquote { blocks }) => blocks.push(block),
            Some(Frame::Verse { blocks }) => blocks.push(block),
            Some(Frame::CenteredBlock { blocks }) => blocks.push(block),
            Some(Frame::RightBlock { blocks }) => blocks.push(block),
            Some(Frame::ListItem { blocks }) => blocks.push(block),
            Some(Frame::DefinitionDesc { blocks }) => blocks.push(block),
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
            Some(Frame::FootnoteDef { inlines, .. }) => inlines.push(inline),
            _ => {} // unexpected context, discard
        }
    }

    fn finish(mut self) -> MuseDoc {
        let blocks = match self.stack.pop() {
            Some(Frame::Document { blocks }) => blocks,
            _ => vec![],
        };
        MuseDoc {
            blocks,
            span: Span::NONE,
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
        w.write_event(OwnedMuseEvent::StartHeading { level: 1 });
        w.write_event(OwnedMuseEvent::Text(Cow::Owned("Hello".to_string())));
        w.write_event(OwnedMuseEvent::EndHeading);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("* Hello"), "got: {s:?}");
    }

    #[test]
    fn test_writer_paragraph() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedMuseEvent::StartParagraph);
        w.write_event(OwnedMuseEvent::Text(Cow::Owned("World".to_string())));
        w.write_event(OwnedMuseEvent::EndParagraph);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("World"), "got: {s:?}");
    }

    #[test]
    fn test_writer_bold() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedMuseEvent::StartParagraph);
        w.write_event(OwnedMuseEvent::StartBold);
        w.write_event(OwnedMuseEvent::Text(Cow::Owned("strong".to_string())));
        w.write_event(OwnedMuseEvent::EndBold);
        w.write_event(OwnedMuseEvent::EndParagraph);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("**strong**"), "got: {s:?}");
    }

    #[test]
    fn test_writer_roundtrip_via_events() {
        let input = "* Hello\n\nA paragraph with **bold** text.\n\n - item one\n - item two\n";
        let (doc, _) = crate::parse(input);
        let evts: Vec<_> = crate::events::events(&doc).collect();
        let mut w = Writer::new(Vec::<u8>::new());
        for e in evts {
            w.write_event(e.into_owned());
        }
        let bytes = w.finish();
        let emitted_text = String::from_utf8(bytes).unwrap();
        let (doc_orig, _) = crate::parse(input);
        let (doc_emit, _) = crate::parse(&emitted_text);
        assert_eq!(
            doc_orig.blocks.len(),
            doc_emit.blocks.len(),
            "writer roundtrip block count mismatch"
        );
    }

    #[test]
    fn test_writer_code_block() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedMuseEvent::CodeBlock {
            content: Cow::Owned("fn main() {}".to_string()),
        });
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("<example>"), "got: {s:?}");
        assert!(s.contains("fn main() {}"), "got: {s:?}");
        assert!(s.contains("</example>"), "got: {s:?}");
    }

    #[test]
    fn test_writer_table() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedMuseEvent::StartTable);
        w.write_event(OwnedMuseEvent::StartTableRow { header: true });
        w.write_event(OwnedMuseEvent::StartTableCell);
        w.write_event(OwnedMuseEvent::Text(Cow::Owned("Name".to_string())));
        w.write_event(OwnedMuseEvent::EndTableCell);
        w.write_event(OwnedMuseEvent::EndTableRow);
        w.write_event(OwnedMuseEvent::EndTable);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("||"), "got: {s:?}");
        assert!(s.contains("Name"), "got: {s:?}");
    }
}
