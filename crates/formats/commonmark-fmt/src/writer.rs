//! Streaming CommonMark writer — converts a stream of events to CommonMark bytes.
//!
//! This implementation buffers all events, reconstructs the AST via a stack
//! machine, then emits CommonMark text on `finish()`. The public API is
//! streaming (event-at-a-time), but the internal implementation is
//! buffer-then-emit for correctness (reuses the proven `emit()` path).
//!
//! # Example
//! ```
//! use commonmark_fmt::writer::Writer;
//! use commonmark_fmt::events::OwnedEvent;
//! use std::borrow::Cow;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! w.write_event(OwnedEvent::StartHeading { level: 1 });
//! w.write_event(OwnedEvent::Text(Cow::Owned("Hello".to_string())));
//! w.write_event(OwnedEvent::EndHeading { level: 1 });
//! let bytes = w.finish().unwrap();
//! assert!(String::from_utf8(bytes).unwrap().starts_with("# Hello"));
//! ```

use crate::ast::*;
use crate::events::{Event, OwnedEvent};
use std::io::Write;

/// Streaming CommonMark writer.
///
/// Feed events with [`write_event`](Writer::write_event), then call
/// [`finish`](Writer::finish) to flush CommonMark text to the underlying sink
/// and recover it.
pub struct Writer<W: Write> {
    sink: W,
    events: Vec<OwnedEvent>,
}

impl<W: Write> Writer<W> {
    /// Create a new writer that emits to `sink`.
    pub fn new(sink: W) -> Self {
        Writer { sink, events: Vec::new() }
    }

    /// Feed one event to the writer. Events are buffered until `finish()`.
    pub fn write_event(&mut self, event: Event<'_>) {
        self.events.push(event.into_owned());
    }

    /// Flush all buffered events as CommonMark text to the sink and return it.
    pub fn finish(mut self) -> std::io::Result<W> {
        let doc = events_to_doc(std::mem::take(&mut self.events));
        let bytes = crate::emit::emit(&doc);
        self.sink.write_all(&bytes)?;
        Ok(self.sink)
    }
}

// ── Event → AST reconstruction ────────────────────────────────────────────────

fn events_to_doc(events: Vec<OwnedEvent>) -> CmDoc {
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
    List { ordered: bool, start: u64, tight: bool, items: Vec<ListItem> },
    ListItem { blocks: Vec<Block> },
    // Inline spans
    Emphasis { inlines: Vec<Inline> },
    Strong { inlines: Vec<Inline> },
    Strikethrough { inlines: Vec<Inline> },
    Link { inlines: Vec<Inline>, url: String, title: Option<String> },
    Image { url: String, title: Option<String>, alt: String },
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
            OwnedEvent::StartDocument | OwnedEvent::EndDocument => {
                // No-op: document frame already on stack.
            }

            // ── Block opens ────────────────────────────────────────────────────
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
            OwnedEvent::EndHeading { .. } => {
                if let Some(Frame::Heading { level, inlines }) = self.stack.pop() {
                    self.push_block(Block::Heading { level, inlines, span: Span::NONE });
                }
            }
            OwnedEvent::StartBlockquote => {
                self.stack.push(Frame::Blockquote { blocks: vec![] });
            }
            OwnedEvent::EndBlockquote => {
                if let Some(Frame::Blockquote { blocks }) = self.stack.pop() {
                    self.push_block(Block::Blockquote { blocks, span: Span::NONE });
                }
            }
            OwnedEvent::StartList { ordered, start, tight } => {
                self.stack.push(Frame::List { ordered, start, tight, items: vec![] });
            }
            OwnedEvent::EndList => {
                if let Some(Frame::List { ordered, start, tight, items }) = self.stack.pop() {
                    let kind = if ordered {
                        ListKind::Ordered { start, marker: OrderedMarker::Period }
                    } else {
                        ListKind::Unordered { marker: '-' }
                    };
                    self.push_block(Block::List { kind, items, tight, span: Span::NONE });
                }
            }
            OwnedEvent::StartItem => {
                self.stack.push(Frame::ListItem { blocks: vec![] });
            }
            OwnedEvent::EndItem => {
                if let Some(Frame::ListItem { blocks }) = self.stack.pop()
                    && let Some(Frame::List { items, .. }) = self.stack.last_mut()
                {
                    items.push(ListItem { blocks, span: Span::NONE });
                }
            }

            // ── Leaf block events ──────────────────────────────────────────────
            OwnedEvent::CodeBlock { language, content } => {
                self.push_block(Block::CodeBlock {
                    language: language.map(|c| c.into_owned()),
                    content: content.into_owned(),
                    span: Span::NONE,
                });
            }
            OwnedEvent::HtmlBlock(content) => {
                self.push_block(Block::HtmlBlock {
                    content: content.into_owned(),
                    span: Span::NONE,
                });
            }
            OwnedEvent::ThematicBreak => {
                self.push_block(Block::ThematicBreak { span: Span::NONE });
            }

            // ── Inline opens ───────────────────────────────────────────────────
            OwnedEvent::StartEmphasis => {
                self.stack.push(Frame::Emphasis { inlines: vec![] });
            }
            OwnedEvent::EndEmphasis => {
                if let Some(Frame::Emphasis { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Emphasis { inlines, span: Span::NONE });
                }
            }
            OwnedEvent::StartStrong => {
                self.stack.push(Frame::Strong { inlines: vec![] });
            }
            OwnedEvent::EndStrong => {
                if let Some(Frame::Strong { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Strong { inlines, span: Span::NONE });
                }
            }
            OwnedEvent::StartStrikethrough => {
                self.stack.push(Frame::Strikethrough { inlines: vec![] });
            }
            OwnedEvent::EndStrikethrough => {
                if let Some(Frame::Strikethrough { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Strikethrough { inlines, span: Span::NONE });
                }
            }
            OwnedEvent::StartLink { url, title } => {
                self.stack.push(Frame::Link {
                    inlines: vec![],
                    url: url.into_owned(),
                    title: title.map(|t| t.into_owned()),
                });
            }
            OwnedEvent::EndLink => {
                if let Some(Frame::Link { inlines, url, title }) = self.stack.pop() {
                    self.push_inline(Inline::Link { inlines, url, title, span: Span::NONE });
                }
            }
            OwnedEvent::StartImage { url, title, alt } => {
                self.stack.push(Frame::Image {
                    url: url.into_owned(),
                    title: title.map(|t| t.into_owned()),
                    alt: alt.into_owned(),
                });
            }
            OwnedEvent::EndImage => {
                if let Some(Frame::Image { url, title, alt }) = self.stack.pop() {
                    self.push_inline(Inline::Image { alt, url, title, span: Span::NONE });
                }
            }

            // ── Inline leaf events ─────────────────────────────────────────────
            OwnedEvent::Text(cow) => {
                self.push_inline(Inline::Text { content: cow.into_owned(), span: Span::NONE });
            }
            OwnedEvent::Code(cow) => {
                self.push_inline(Inline::Code { content: cow.into_owned(), span: Span::NONE });
            }
            OwnedEvent::HtmlInline(cow) => {
                self.push_inline(Inline::HtmlInline {
                    content: cow.into_owned(),
                    span: Span::NONE,
                });
            }
            OwnedEvent::SoftBreak => {
                self.push_inline(Inline::SoftBreak { span: Span::NONE });
            }
            OwnedEvent::HardBreak => {
                self.push_inline(Inline::HardBreak { span: Span::NONE });
            }
        }
    }

    fn push_block(&mut self, block: Block) {
        match self.stack.last_mut() {
            Some(Frame::Document { blocks }) => blocks.push(block),
            Some(Frame::Blockquote { blocks }) => blocks.push(block),
            Some(Frame::ListItem { blocks }) => blocks.push(block),
            _ => {} // unexpected context — discard
        }
    }

    fn push_inline(&mut self, inline: Inline) {
        match self.stack.last_mut() {
            Some(Frame::Paragraph { inlines }) => inlines.push(inline),
            Some(Frame::Heading { inlines, .. }) => inlines.push(inline),
            Some(Frame::Emphasis { inlines }) => inlines.push(inline),
            Some(Frame::Strong { inlines }) => inlines.push(inline),
            Some(Frame::Strikethrough { inlines }) => inlines.push(inline),
            Some(Frame::Link { inlines, .. }) => inlines.push(inline),
            // For images we only collect the alt text via StartImage; Text events
            // between StartImage/EndImage are already captured in the alt field
            // of StartImage. We discard them here to avoid double-counting.
            Some(Frame::Image { .. }) => {}
            _ => {} // unexpected context — discard
        }
    }

    fn finish(mut self) -> CmDoc {
        let blocks = match self.stack.pop() {
            Some(Frame::Document { blocks }) => blocks,
            _ => vec![],
        };
        CmDoc { blocks, link_defs: vec![] }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;

    fn run(events: Vec<OwnedEvent>) -> String {
        let mut w = Writer::new(Vec::<u8>::new());
        for e in events {
            w.write_event(e);
        }
        let bytes = w.finish().unwrap();
        String::from_utf8(bytes).unwrap()
    }

    #[test]
    fn test_writer_paragraph() {
        let out = run(vec![
            OwnedEvent::StartDocument,
            OwnedEvent::StartParagraph,
            OwnedEvent::Text(Cow::Borrowed("Hello")),
            OwnedEvent::EndParagraph,
            OwnedEvent::EndDocument,
        ]);
        // emit() produces "Hello\n" for a single paragraph (blank separator is
        // between blocks, not after the last one).
        assert!(out.contains("Hello"), "got: {out:?}");
        assert!(out.ends_with('\n'), "should end with newline, got: {out:?}");
    }

    #[test]
    fn test_writer_heading() {
        let out = run(vec![
            OwnedEvent::StartDocument,
            OwnedEvent::StartHeading { level: 2 },
            OwnedEvent::Text(Cow::Borrowed("My heading")),
            OwnedEvent::EndHeading { level: 2 },
            OwnedEvent::EndDocument,
        ]);
        assert!(out.starts_with("## My heading"), "got: {out:?}");
    }

    #[test]
    fn test_writer_matches_emit() {
        let input = "# Title\n\nA paragraph with *em* and **strong**.\n\n- item one\n- item two\n";
        // Build expected output via emit().
        let (doc, _) = crate::parse::parse(input.as_bytes());
        let expected = String::from_utf8(crate::emit::emit(&doc)).unwrap();

        // Build output via Writer fed from events().
        let evts: Vec<_> =
            crate::events::events_str(input).map(|e| e.into_owned()).collect();
        let out = run(evts);

        // Parse both and compare ASTs (strips span differences).
        let (ast_expected, _) = crate::parse::parse(expected.as_bytes());
        let (ast_out, _) = crate::parse::parse(out.as_bytes());
        assert_eq!(
            ast_expected.strip_spans(),
            ast_out.strip_spans(),
            "writer output doesn't match emit output\nexpected:\n{expected}\ngot:\n{out}"
        );
    }

    #[test]
    fn test_writer_code_block() {
        let out = run(vec![
            OwnedEvent::StartDocument,
            OwnedEvent::CodeBlock {
                language: Some(Cow::Borrowed("rust")),
                content: Cow::Borrowed("fn main() {}\n"),
            },
            OwnedEvent::EndDocument,
        ]);
        assert!(out.contains("```rust"), "got: {out:?}");
        assert!(out.contains("fn main() {}"), "got: {out:?}");
    }

    #[test]
    fn test_writer_blockquote() {
        let out = run(vec![
            OwnedEvent::StartDocument,
            OwnedEvent::StartBlockquote,
            OwnedEvent::StartParagraph,
            OwnedEvent::Text(Cow::Borrowed("quoted")),
            OwnedEvent::EndParagraph,
            OwnedEvent::EndBlockquote,
            OwnedEvent::EndDocument,
        ]);
        assert!(out.contains("> quoted"), "got: {out:?}");
    }

    #[test]
    fn test_writer_thematic_break() {
        let out = run(vec![
            OwnedEvent::StartDocument,
            OwnedEvent::ThematicBreak,
            OwnedEvent::EndDocument,
        ]);
        assert!(out.contains("---"), "got: {out:?}");
    }

    #[test]
    fn test_writer_link() {
        let out = run(vec![
            OwnedEvent::StartDocument,
            OwnedEvent::StartParagraph,
            OwnedEvent::StartLink {
                url: Cow::Borrowed("https://example.com"),
                title: None,
            },
            OwnedEvent::Text(Cow::Borrowed("click")),
            OwnedEvent::EndLink,
            OwnedEvent::EndParagraph,
            OwnedEvent::EndDocument,
        ]);
        assert!(out.contains("[click](https://example.com)"), "got: {out:?}");
    }

    #[test]
    fn test_writer_image() {
        let out = run(vec![
            OwnedEvent::StartDocument,
            OwnedEvent::StartParagraph,
            OwnedEvent::StartImage {
                url: Cow::Borrowed("img.png"),
                title: None,
                alt: Cow::Borrowed("alt text"),
            },
            // Text between StartImage/EndImage — should not be double-counted.
            OwnedEvent::Text(Cow::Borrowed("alt text")),
            OwnedEvent::EndImage,
            OwnedEvent::EndParagraph,
            OwnedEvent::EndDocument,
        ]);
        assert!(out.contains("![alt text](img.png)"), "got: {out:?}");
    }

    #[test]
    fn test_writer_emphasis_strong() {
        let out = run(vec![
            OwnedEvent::StartDocument,
            OwnedEvent::StartParagraph,
            OwnedEvent::StartEmphasis,
            OwnedEvent::Text(Cow::Borrowed("em")),
            OwnedEvent::EndEmphasis,
            OwnedEvent::Text(Cow::Borrowed(" and ")),
            OwnedEvent::StartStrong,
            OwnedEvent::Text(Cow::Borrowed("strong")),
            OwnedEvent::EndStrong,
            OwnedEvent::EndParagraph,
            OwnedEvent::EndDocument,
        ]);
        assert!(out.contains("*em*"), "got: {out:?}");
        assert!(out.contains("**strong**"), "got: {out:?}");
    }
}
