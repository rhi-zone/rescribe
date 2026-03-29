//! Streaming Textile writer — converts a stream of [`TextileEvent`]s to Textile text.
//!
//! This implementation buffers all events, reconstructs the AST, then emits.
//!
//! # Example
//! ```no_run
//! use textile_fmt::writer::Writer;
//! use textile_fmt::events::TextileEvent;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! w.write_event(TextileEvent::StartHeading { level: 1, attrs: Default::default() });
//! w.write_event(TextileEvent::Text("Hello".to_string()));
//! w.write_event(TextileEvent::EndHeading);
//! let bytes = w.finish();
//! ```

use std::io::Write;

use crate::ast::{Block, BlockAttrs, Inline, TableCell, TableRow, TextileDoc};
use crate::events::TextileEvent;

/// Streaming Textile writer.
///
/// Feed events with [`write_event`](Writer::write_event), then call
/// [`finish`](Writer::finish) to flush Textile text to the underlying sink and
/// recover it.
pub struct Writer<W: Write> {
    sink: W,
    events: Vec<TextileEvent>,
}

impl<W: Write> Writer<W> {
    pub fn new(sink: W) -> Self {
        Writer { sink, events: Vec::new() }
    }

    /// Feed one event to the writer.
    pub fn write_event(&mut self, event: TextileEvent) {
        self.events.push(event);
    }

    /// Flush all buffered events as Textile text and return the sink.
    pub fn finish(mut self) -> W {
        let doc = events_to_doc(std::mem::take(&mut self.events));
        let text = crate::emit::emit(&doc);
        let _ = self.sink.write_all(text.as_bytes());
        self.sink
    }
}

// ── Event → AST reconstruction ────────────────────────────────────────────────

fn events_to_doc(events: Vec<TextileEvent>) -> TextileDoc {
    let mut builder = DocBuilder::new();
    for event in events {
        builder.process(event);
    }
    builder.finish()
}

// ── Frame stack ───────────────────────────────────────────────────────────────

enum Frame {
    Document {
        blocks: Vec<Block>,
    },
    Paragraph {
        align: Option<String>,
        attrs: BlockAttrs,
        inlines: Vec<Inline>,
    },
    Heading {
        level: u8,
        attrs: BlockAttrs,
        inlines: Vec<Inline>,
    },
    Blockquote {
        attrs: BlockAttrs,
        blocks: Vec<Block>,
    },
    List {
        ordered: bool,
        items: Vec<Vec<Block>>,
        current_item: Option<Vec<Block>>,
    },
    ListItem {
        blocks: Vec<Block>,
        /// Accumulate inline content until a non-inline block forces a flush.
        inline_buf: Vec<Inline>,
    },
    Table {
        rows: Vec<TableRow>,
    },
    TableRow {
        attrs: BlockAttrs,
        cells: Vec<TableCell>,
    },
    TableCell {
        is_header: bool,
        align: Option<String>,
        inlines: Vec<Inline>,
    },
    FootnoteDef {
        label: String,
        inlines: Vec<Inline>,
    },
    DefinitionList {
        items: Vec<(Vec<Inline>, Vec<Inline>)>,
        current_term: Option<Vec<Inline>>,
    },
    DefinitionTerm {
        inlines: Vec<Inline>,
    },
    DefinitionDesc {
        inlines: Vec<Inline>,
    },
    // Inline spans
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
        title: Option<String>,
        inlines: Vec<Inline>,
    },
    Citation {
        inlines: Vec<Inline>,
    },
    GenericSpan {
        attrs: BlockAttrs,
        inlines: Vec<Inline>,
    },
}

struct DocBuilder {
    stack: Vec<Frame>,
}

impl DocBuilder {
    fn new() -> Self {
        DocBuilder { stack: vec![Frame::Document { blocks: vec![] }] }
    }

    #[allow(clippy::too_many_lines)]
    fn process(&mut self, event: TextileEvent) {
        match event {
            // ── Block open ──────────────────────────────────────────────────
            TextileEvent::StartParagraph { align, attrs } => {
                self.stack.push(Frame::Paragraph { align, attrs, inlines: vec![] });
            }
            TextileEvent::EndParagraph => {
                if let Some(Frame::Paragraph { align, attrs, inlines }) = self.stack.pop() {
                    self.push_block(Block::Paragraph {
                        inlines,
                        align,
                        attrs,
                        span: crate::ast::Span::dummy(),
                    });
                }
            }

            TextileEvent::StartHeading { level, attrs } => {
                self.stack.push(Frame::Heading { level, attrs, inlines: vec![] });
            }
            TextileEvent::EndHeading => {
                if let Some(Frame::Heading { level, attrs, inlines }) = self.stack.pop() {
                    self.push_block(Block::Heading {
                        level,
                        inlines,
                        attrs,
                        span: crate::ast::Span::dummy(),
                    });
                }
            }

            TextileEvent::CodeBlock { content, language } => {
                self.push_block(Block::CodeBlock {
                    content,
                    language,
                    span: crate::ast::Span::dummy(),
                });
            }

            TextileEvent::StartBlockquote { attrs } => {
                self.stack.push(Frame::Blockquote { attrs, blocks: vec![] });
            }
            TextileEvent::EndBlockquote => {
                if let Some(Frame::Blockquote { attrs, blocks }) = self.stack.pop() {
                    self.push_block(Block::Blockquote {
                        blocks,
                        attrs,
                        span: crate::ast::Span::dummy(),
                    });
                }
            }

            TextileEvent::StartList { ordered } => {
                self.stack.push(Frame::List { ordered, items: vec![], current_item: None });
            }
            TextileEvent::EndList => {
                if let Some(Frame::List { ordered, mut items, current_item }) = self.stack.pop() {
                    // Flush any dangling current item (shouldn't happen with well-formed events)
                    if let Some(item) = current_item {
                        items.push(item);
                    }
                    self.push_block(Block::List {
                        ordered,
                        items,
                        span: crate::ast::Span::dummy(),
                    });
                }
            }
            TextileEvent::StartListItem => {
                self.stack.push(Frame::ListItem { blocks: vec![], inline_buf: vec![] });
            }
            TextileEvent::EndListItem => {
                if let Some(Frame::ListItem { mut blocks, inline_buf }) = self.stack.pop() {
                    // Flush any trailing inline content as a paragraph
                    if !inline_buf.is_empty() {
                        blocks.push(Block::Paragraph {
                            inlines: inline_buf,
                            align: None,
                            attrs: BlockAttrs::default(),
                            span: crate::ast::Span::dummy(),
                        });
                    }
                    if let Some(Frame::List { items, .. }) = self.stack.last_mut() {
                        items.push(blocks);
                    }
                }
            }

            TextileEvent::StartTable => {
                self.stack.push(Frame::Table { rows: vec![] });
            }
            TextileEvent::EndTable => {
                if let Some(Frame::Table { rows }) = self.stack.pop() {
                    self.push_block(Block::Table {
                        rows,
                        span: crate::ast::Span::dummy(),
                    });
                }
            }
            TextileEvent::StartTableRow { attrs } => {
                self.stack.push(Frame::TableRow { attrs, cells: vec![] });
            }
            TextileEvent::EndTableRow => {
                if let Some(Frame::TableRow { attrs, cells }) = self.stack.pop()
                    && let Some(Frame::Table { rows }) = self.stack.last_mut()
                {
                    rows.push(TableRow { attrs, cells, span: crate::ast::Span::dummy() });
                }
            }
            TextileEvent::StartTableCell { is_header, align } => {
                self.stack.push(Frame::TableCell { is_header, align, inlines: vec![] });
            }
            TextileEvent::EndTableCell => {
                if let Some(Frame::TableCell { is_header, align, inlines }) = self.stack.pop()
                    && let Some(Frame::TableRow { cells, .. }) = self.stack.last_mut()
                {
                    cells.push(TableCell {
                        is_header,
                        align,
                        inlines,
                        span: crate::ast::Span::dummy(),
                    });
                }
            }

            TextileEvent::HorizontalRule => {
                self.push_block(Block::HorizontalRule { span: crate::ast::Span::dummy() });
            }

            TextileEvent::StartFootnoteDef { label } => {
                self.stack.push(Frame::FootnoteDef { label, inlines: vec![] });
            }
            TextileEvent::EndFootnoteDef => {
                if let Some(Frame::FootnoteDef { label, inlines }) = self.stack.pop() {
                    self.push_block(Block::FootnoteDef {
                        label,
                        inlines,
                        span: crate::ast::Span::dummy(),
                    });
                }
            }

            TextileEvent::StartDefinitionList => {
                self.stack.push(Frame::DefinitionList { items: vec![], current_term: None });
            }
            TextileEvent::EndDefinitionList => {
                if let Some(Frame::DefinitionList { items, .. }) = self.stack.pop() {
                    self.push_block(Block::DefinitionList {
                        items,
                        span: crate::ast::Span::dummy(),
                    });
                }
            }
            TextileEvent::StartDefinitionTerm => {
                self.stack.push(Frame::DefinitionTerm { inlines: vec![] });
            }
            TextileEvent::EndDefinitionTerm => {
                if let Some(Frame::DefinitionTerm { inlines }) = self.stack.pop()
                    && let Some(Frame::DefinitionList { current_term, .. }) =
                        self.stack.last_mut()
                {
                    *current_term = Some(inlines);
                }
            }
            TextileEvent::StartDefinitionDesc => {
                self.stack.push(Frame::DefinitionDesc { inlines: vec![] });
            }
            TextileEvent::EndDefinitionDesc => {
                if let Some(Frame::DefinitionDesc { inlines }) = self.stack.pop()
                    && let Some(Frame::DefinitionList { items, current_term }) =
                        self.stack.last_mut()
                {
                    let term = current_term.take().unwrap_or_default();
                    items.push((term, inlines));
                }
            }

            TextileEvent::RawBlock { content } => {
                self.push_block(Block::Raw { content, span: crate::ast::Span::dummy() });
            }

            // ── Inline span open/close ──────────────────────────────────────
            TextileEvent::StartBold => {
                self.stack.push(Frame::Bold { inlines: vec![] });
            }
            TextileEvent::EndBold => {
                if let Some(Frame::Bold { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Bold(inlines, crate::ast::Span::dummy()));
                }
            }

            TextileEvent::StartItalic => {
                self.stack.push(Frame::Italic { inlines: vec![] });
            }
            TextileEvent::EndItalic => {
                if let Some(Frame::Italic { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Italic(inlines, crate::ast::Span::dummy()));
                }
            }

            TextileEvent::StartUnderline => {
                self.stack.push(Frame::Underline { inlines: vec![] });
            }
            TextileEvent::EndUnderline => {
                if let Some(Frame::Underline { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Underline(inlines, crate::ast::Span::dummy()));
                }
            }

            TextileEvent::StartStrikethrough => {
                self.stack.push(Frame::Strikethrough { inlines: vec![] });
            }
            TextileEvent::EndStrikethrough => {
                if let Some(Frame::Strikethrough { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Strikethrough(inlines, crate::ast::Span::dummy()));
                }
            }

            TextileEvent::StartSuperscript => {
                self.stack.push(Frame::Superscript { inlines: vec![] });
            }
            TextileEvent::EndSuperscript => {
                if let Some(Frame::Superscript { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Superscript(inlines, crate::ast::Span::dummy()));
                }
            }

            TextileEvent::StartSubscript => {
                self.stack.push(Frame::Subscript { inlines: vec![] });
            }
            TextileEvent::EndSubscript => {
                if let Some(Frame::Subscript { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Subscript(inlines, crate::ast::Span::dummy()));
                }
            }

            TextileEvent::StartLink { url, title } => {
                self.stack.push(Frame::Link { url, title, inlines: vec![] });
            }
            TextileEvent::EndLink => {
                if let Some(Frame::Link { url, title, inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Link {
                        url,
                        title,
                        children: inlines,
                        span: crate::ast::Span::dummy(),
                    });
                }
            }

            TextileEvent::StartCitation => {
                self.stack.push(Frame::Citation { inlines: vec![] });
            }
            TextileEvent::EndCitation => {
                if let Some(Frame::Citation { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Citation(inlines, crate::ast::Span::dummy()));
                }
            }

            TextileEvent::StartGenericSpan { attrs } => {
                self.stack.push(Frame::GenericSpan { attrs, inlines: vec![] });
            }
            TextileEvent::EndGenericSpan => {
                if let Some(Frame::GenericSpan { attrs, inlines }) = self.stack.pop() {
                    self.push_inline(Inline::GenericSpan {
                        attrs,
                        children: inlines,
                        span: crate::ast::Span::dummy(),
                    });
                }
            }

            // ── Leaf inlines ────────────────────────────────────────────────
            TextileEvent::Text(s) => {
                self.push_inline(Inline::Text(s, crate::ast::Span::dummy()));
            }
            TextileEvent::InlineCode(s) => {
                self.push_inline(Inline::Code(s, crate::ast::Span::dummy()));
            }
            TextileEvent::InlineImage { url, alt } => {
                self.push_inline(Inline::Image { url, alt, span: crate::ast::Span::dummy() });
            }
            TextileEvent::FootnoteRef { label } => {
                self.push_inline(Inline::FootnoteRef { label, span: crate::ast::Span::dummy() });
            }
            TextileEvent::LineBreak => {
                self.push_inline(Inline::LineBreak(crate::ast::Span::dummy()));
            }
            TextileEvent::RawInline { content } => {
                self.push_inline(Inline::Raw(content, crate::ast::Span::dummy()));
            }
            TextileEvent::Acronym { text, title } => {
                self.push_inline(Inline::Acronym {
                    text,
                    title,
                    span: crate::ast::Span::dummy(),
                });
            }
        }
    }

    fn push_block(&mut self, block: Block) {
        match self.stack.last_mut() {
            Some(Frame::Document { blocks }) => blocks.push(block),
            Some(Frame::Blockquote { blocks, .. }) => blocks.push(block),
            Some(Frame::ListItem { blocks, inline_buf }) => {
                // Flush any pending inline content as a paragraph before the block.
                if !inline_buf.is_empty() {
                    let inlines = std::mem::take(inline_buf);
                    blocks.push(Block::Paragraph {
                        inlines,
                        align: None,
                        attrs: BlockAttrs::default(),
                        span: crate::ast::Span::dummy(),
                    });
                }
                blocks.push(block);
            }
            _ => {} // unexpected context, discard
        }
    }

    fn push_inline(&mut self, inline: Inline) {
        match self.stack.last_mut() {
            Some(Frame::Paragraph { inlines, .. }) => inlines.push(inline),
            Some(Frame::Heading { inlines, .. }) => inlines.push(inline),
            Some(Frame::FootnoteDef { inlines, .. }) => inlines.push(inline),
            Some(Frame::DefinitionTerm { inlines }) => inlines.push(inline),
            Some(Frame::DefinitionDesc { inlines }) => inlines.push(inline),
            Some(Frame::TableCell { inlines, .. }) => inlines.push(inline),
            Some(Frame::Bold { inlines }) => inlines.push(inline),
            Some(Frame::Italic { inlines }) => inlines.push(inline),
            Some(Frame::Underline { inlines }) => inlines.push(inline),
            Some(Frame::Strikethrough { inlines }) => inlines.push(inline),
            Some(Frame::Superscript { inlines }) => inlines.push(inline),
            Some(Frame::Subscript { inlines }) => inlines.push(inline),
            Some(Frame::Link { inlines, .. }) => inlines.push(inline),
            Some(Frame::Citation { inlines }) => inlines.push(inline),
            Some(Frame::GenericSpan { inlines, .. }) => inlines.push(inline),
            Some(Frame::ListItem { inline_buf, .. }) => inline_buf.push(inline),
            _ => {} // unexpected context, discard
        }
    }

    fn finish(mut self) -> TextileDoc {
        let blocks = match self.stack.pop() {
            Some(Frame::Document { blocks }) => blocks,
            _ => vec![],
        };
        TextileDoc { blocks, span: crate::ast::Span::dummy() }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::BlockAttrs;

    #[test]
    fn test_writer_heading() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(TextileEvent::StartHeading {
            level: 1,
            attrs: BlockAttrs::default(),
        });
        w.write_event(TextileEvent::Text("Hello".to_string()));
        w.write_event(TextileEvent::EndHeading);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("h1. Hello"), "got: {s:?}");
    }

    #[test]
    fn test_writer_paragraph() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(TextileEvent::StartParagraph {
            align: None,
            attrs: BlockAttrs::default(),
        });
        w.write_event(TextileEvent::Text("World".to_string()));
        w.write_event(TextileEvent::EndParagraph);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("World"), "got: {s:?}");
    }

    #[test]
    fn test_writer_bold() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(TextileEvent::StartParagraph {
            align: None,
            attrs: BlockAttrs::default(),
        });
        w.write_event(TextileEvent::StartBold);
        w.write_event(TextileEvent::Text("bold".to_string()));
        w.write_event(TextileEvent::EndBold);
        w.write_event(TextileEvent::EndParagraph);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("*bold*"), "got: {s:?}");
    }

    #[test]
    fn test_writer_code_block() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(TextileEvent::CodeBlock {
            content: "print('hi')".to_string(),
            language: None,
        });
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("bc. print('hi')"), "got: {s:?}");
    }

    #[test]
    fn test_writer_footnote_def() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(TextileEvent::StartFootnoteDef { label: "1".to_string() });
        w.write_event(TextileEvent::Text("Note content".to_string()));
        w.write_event(TextileEvent::EndFootnoteDef);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("fn1. Note content"), "got: {s:?}");
    }

    #[test]
    fn test_writer_definition_list() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(TextileEvent::StartDefinitionList);
        w.write_event(TextileEvent::StartDefinitionTerm);
        w.write_event(TextileEvent::Text("Term".to_string()));
        w.write_event(TextileEvent::EndDefinitionTerm);
        w.write_event(TextileEvent::StartDefinitionDesc);
        w.write_event(TextileEvent::Text("Definition".to_string()));
        w.write_event(TextileEvent::EndDefinitionDesc);
        w.write_event(TextileEvent::EndDefinitionList);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains(";Term"), "got: {s:?}");
        assert!(s.contains(":Definition"), "got: {s:?}");
    }

    #[test]
    fn test_writer_horizontal_rule() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(TextileEvent::HorizontalRule);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("---"), "got: {s:?}");
    }

    #[test]
    fn test_writer_roundtrip_via_events() {
        let input = "h1. Title\n\nA paragraph with *bold* text.\n\n* item one\n* item two\n";
        let evts: Vec<_> = crate::events::events(input).collect();
        let mut w = Writer::new(Vec::<u8>::new());
        for e in evts {
            w.write_event(e);
        }
        let bytes = w.finish();
        let emitted_text = String::from_utf8(bytes).unwrap();
        // Verify the emitted text re-parses cleanly
        let (doc_orig, _) = crate::parse::parse(input);
        let (doc_emit, _) = crate::parse::parse(&emitted_text);
        assert_eq!(
            doc_orig.blocks.len(),
            doc_emit.blocks.len(),
            "writer roundtrip block count mismatch"
        );
    }

    #[test]
    fn test_writer_citation() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(TextileEvent::StartParagraph {
            align: None,
            attrs: BlockAttrs::default(),
        });
        w.write_event(TextileEvent::StartCitation);
        w.write_event(TextileEvent::Text("cited".to_string()));
        w.write_event(TextileEvent::EndCitation);
        w.write_event(TextileEvent::EndParagraph);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("??cited??"), "got: {s:?}");
    }

    #[test]
    fn test_writer_acronym() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(TextileEvent::StartParagraph {
            align: None,
            attrs: BlockAttrs::default(),
        });
        w.write_event(TextileEvent::Acronym {
            text: "HTML".to_string(),
            title: "HyperText Markup Language".to_string(),
        });
        w.write_event(TextileEvent::EndParagraph);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("HTML(HyperText Markup Language)"), "got: {s:?}");
    }

    #[test]
    fn test_writer_raw_block() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(TextileEvent::RawBlock { content: "<b>raw</b>".to_string() });
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("notextile. <b>raw</b>"), "got: {s:?}");
    }

    #[test]
    fn test_writer_raw_inline() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(TextileEvent::StartParagraph {
            align: None,
            attrs: BlockAttrs::default(),
        });
        w.write_event(TextileEvent::RawInline { content: "<em>x</em>".to_string() });
        w.write_event(TextileEvent::EndParagraph);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("==<em>x</em>=="), "got: {s:?}");
    }
}
