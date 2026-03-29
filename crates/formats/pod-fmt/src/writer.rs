//! Streaming POD writer — converts a stream of events to POD text.

use crate::ast::*;
use crate::events::OwnedEvent;
use std::io::Write;

/// Streaming POD writer.
///
/// Feed events with [`write_event`](Writer::write_event), then call
/// [`finish`](Writer::finish) to flush POD text to the underlying sink.
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

    /// Flush all buffered events as POD text and return the sink.
    pub fn finish(mut self) -> W {
        let doc = events_to_doc(std::mem::take(&mut self.events));
        let text = crate::emit::build(&doc);
        let _ = self.sink.write_all(text.as_bytes());
        self.sink
    }
}

// ── Event -> AST reconstruction ──────────────────────────────────────────────

fn events_to_doc(events: Vec<OwnedEvent>) -> PodDoc {
    let mut builder = DocBuilder::new();
    for event in events {
        builder.process(event);
    }
    builder.finish()
}

enum Frame {
    Document { blocks: Vec<Block> },
    Paragraph { inlines: Vec<Inline> },
    Heading { level: u32, inlines: Vec<Inline> },
    List { ordered: bool, items: Vec<Vec<Block>> },
    ListItem { blocks: Vec<Block> },
    DefinitionList { items: Vec<DefinitionItem> },
    DefinitionTerm { inlines: Vec<Inline> },
    DefinitionDesc { blocks: Vec<Block> },
    // Inline spans
    Bold { inlines: Vec<Inline> },
    Italic { inlines: Vec<Inline> },
    Underline { inlines: Vec<Inline> },
    Filename { inlines: Vec<Inline> },
    NonBreaking { inlines: Vec<Inline> },
    Link { url: String, label: String, inlines: Vec<Inline> },
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
            OwnedEvent::StartList { ordered } => {
                self.stack.push(Frame::List { ordered, items: vec![] });
            }
            OwnedEvent::EndList => {
                if let Some(Frame::List { ordered, items }) = self.stack.pop() {
                    self.push_block(Block::List { ordered, items, span: Span::NONE });
                }
            }
            OwnedEvent::StartListItem => {
                self.stack.push(Frame::ListItem { blocks: vec![] });
            }
            OwnedEvent::EndListItem => {
                if let Some(Frame::ListItem { blocks }) = self.stack.pop()
                    && let Some(Frame::List { items, .. }) = self.stack.last_mut()
                {
                    items.push(blocks);
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
                if let Some(Frame::DefinitionTerm { inlines }) = self.stack.pop()
                    && let Some(Frame::DefinitionList { items }) = self.stack.last_mut()
                {
                    items.push(DefinitionItem { term: inlines, desc: vec![] });
                }
            }
            OwnedEvent::StartDefinitionDesc => {
                self.stack.push(Frame::DefinitionDesc { blocks: vec![] });
            }
            OwnedEvent::EndDefinitionDesc => {
                if let Some(Frame::DefinitionDesc { blocks }) = self.stack.pop()
                    && let Some(Frame::DefinitionList { items }) = self.stack.last_mut()
                    && let Some(last) = items.last_mut()
                {
                    last.desc = blocks;
                }
            }
            OwnedEvent::CodeBlock { content } => {
                self.push_block(Block::CodeBlock {
                    content: content.into_owned(),
                    span: Span::NONE,
                });
            }
            OwnedEvent::RawBlock { format, content } => {
                self.push_block(Block::RawBlock { format, content, span: Span::NONE });
            }
            OwnedEvent::ForBlock { format, content } => {
                self.push_block(Block::ForBlock { format, content, span: Span::NONE });
            }
            OwnedEvent::Encoding { encoding } => {
                self.push_block(Block::Encoding { encoding, span: Span::NONE });
            }

            // Inline events
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
            OwnedEvent::StartFilename => {
                self.stack.push(Frame::Filename { inlines: vec![] });
            }
            OwnedEvent::EndFilename => {
                if let Some(Frame::Filename { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Filename(inlines, Span::NONE));
                }
            }
            OwnedEvent::StartNonBreaking => {
                self.stack.push(Frame::NonBreaking { inlines: vec![] });
            }
            OwnedEvent::EndNonBreaking => {
                if let Some(Frame::NonBreaking { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::NonBreaking(inlines, Span::NONE));
                }
            }
            OwnedEvent::InlineCode(cow) => {
                self.push_inline(Inline::Code(cow.into_owned(), Span::NONE));
            }
            OwnedEvent::StartLink { url, label } => {
                self.stack.push(Frame::Link { url, label, inlines: vec![] });
            }
            OwnedEvent::EndLink => {
                if let Some(Frame::Link { url, label, .. }) = self.stack.pop() {
                    self.push_inline(Inline::Link { url, label, span: Span::NONE });
                }
            }
            OwnedEvent::IndexEntry(s) => {
                self.push_inline(Inline::IndexEntry(s, Span::NONE));
            }
            OwnedEvent::Null => {
                self.push_inline(Inline::Null(Span::NONE));
            }
            OwnedEvent::Entity(s) => {
                self.push_inline(Inline::Entity(s, Span::NONE));
            }
        }
    }

    fn push_block(&mut self, block: Block) {
        match self.stack.last_mut() {
            Some(Frame::Document { blocks }) => blocks.push(block),
            Some(Frame::ListItem { blocks }) => blocks.push(block),
            Some(Frame::DefinitionDesc { blocks }) => blocks.push(block),
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
            Some(Frame::Filename { inlines }) => inlines.push(inline),
            Some(Frame::NonBreaking { inlines }) => inlines.push(inline),
            Some(Frame::Link { inlines, .. }) => inlines.push(inline),
            Some(Frame::DefinitionTerm { inlines }) => inlines.push(inline),
            _ => {}
        }
    }

    fn finish(mut self) -> PodDoc {
        let blocks = match self.stack.pop() {
            Some(Frame::Document { blocks }) => blocks,
            _ => vec![],
        };
        PodDoc { blocks, span: Span::NONE }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;

    #[test]
    fn test_writer_heading() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedEvent::StartHeading { level: 1 });
        w.write_event(OwnedEvent::Text(Cow::Owned("Hello".to_string())));
        w.write_event(OwnedEvent::EndHeading);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("=head1 Hello"), "got: {s:?}");
    }

    #[test]
    fn test_writer_paragraph() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedEvent::StartParagraph);
        w.write_event(OwnedEvent::Text(Cow::Owned("World".to_string())));
        w.write_event(OwnedEvent::EndParagraph);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("World"), "got: {s:?}");
    }

    #[test]
    fn test_writer_roundtrip_via_events() {
        let input = "=head1 Hello\n\nA paragraph with B<bold> text.\n\n=over 4\n\n=item * one\n\n=item * two\n\n=back\n";
        let (doc, _) = crate::parse::parse(input);
        let evts: Vec<_> = crate::events::EventIter::new(&doc).collect();
        let mut w = Writer::new(Vec::<u8>::new());
        for e in evts {
            w.write_event(e.into_owned());
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
