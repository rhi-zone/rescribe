//! Streaming Texinfo writer — converts a stream of events to Texinfo text.
//!
//! This implementation buffers all events, reconstructs the AST, then emits.
//!
//! # Example
//! ```no_run
//! use texinfo::writer::Writer;
//! use texinfo::OwnedEvent;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! w.write_event(OwnedEvent::StartHeading { level: 1, kind: texinfo::HeadingKind::Numbered });
//! w.write_event(OwnedEvent::Text("Hello".to_string().into()));
//! w.write_event(OwnedEvent::EndHeading);
//! let bytes = w.finish();
//! ```

use crate::ast::*;
use crate::events::OwnedEvent;
use std::io::Write;

/// Streaming Texinfo writer.
///
/// Feed events with [`write_event`](Writer::write_event), then call
/// [`finish`](Writer::finish) to flush Texinfo text to the underlying sink and
/// recover it.
pub struct Writer<W: Write> {
    sink: W,
    events: Vec<OwnedEvent>,
}

impl<W: Write> Writer<W> {
    pub fn new(sink: W) -> Self {
        Writer {
            sink,
            events: Vec::new(),
        }
    }

    /// Feed one event to the writer.
    pub fn write_event(&mut self, event: OwnedEvent) {
        self.events.push(event);
    }

    /// Flush all buffered events as Texinfo text and return the sink.
    pub fn finish(mut self) -> W {
        let doc = events_to_doc(std::mem::take(&mut self.events));
        let text = crate::emit::emit(&doc);
        let _ = self.sink.write_all(text.as_bytes());
        self.sink
    }
}

// ── Event -> AST reconstruction ──────────────────────────────────────────────

fn events_to_doc(events: Vec<OwnedEvent>) -> TexinfoDoc {
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
        kind: HeadingKind,
        inlines: Vec<Inline>,
    },
    Blockquote {
        blocks: Vec<Block>,
    },
    List {
        ordered: bool,
        items: Vec<Vec<Inline>>,
    },
    ListItem {
        inlines: Vec<Inline>,
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
        is_header: bool,
        cells: Vec<Vec<Inline>>,
    },
    TableCell {
        inlines: Vec<Inline>,
    },
    Menu {
        entries: Vec<MenuEntry>,
    },
    Float {
        float_type: Option<String>,
        label: Option<String>,
        children: Vec<Block>,
    },
    // Inline spans
    Strong {
        inlines: Vec<Inline>,
    },
    Emphasis {
        inlines: Vec<Inline>,
    },
    Var {
        inlines: Vec<Inline>,
    },
    Dfn {
        inlines: Vec<Inline>,
    },
    DirectItalic {
        inlines: Vec<Inline>,
    },
    DirectBold {
        inlines: Vec<Inline>,
    },
    Link {
        url: String,
        inlines: Vec<Inline>,
    },
    Superscript {
        inlines: Vec<Inline>,
    },
    Subscript {
        inlines: Vec<Inline>,
    },
    FootnoteDef {
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

    #[allow(clippy::too_many_lines, clippy::collapsible_if)]
    fn process(&mut self, event: OwnedEvent) {
        match event {
            // ── Block open/close ──────────────────────────────────────────────
            OwnedEvent::StartParagraph => {
                self.stack.push(Frame::Paragraph { inlines: vec![] });
            }
            OwnedEvent::EndParagraph => {
                if let Some(Frame::Paragraph { inlines }) = self.stack.pop() {
                    self.push_block(Block::Paragraph {
                        inlines,
                        span: Span::NONE,
                    });
                }
            }
            OwnedEvent::StartHeading { level, kind } => {
                self.stack.push(Frame::Heading {
                    level,
                    kind,
                    inlines: vec![],
                });
            }
            OwnedEvent::EndHeading => {
                if let Some(Frame::Heading { level, kind, inlines }) = self.stack.pop() {
                    self.push_block(Block::Heading {
                        level,
                        kind,
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
                    self.push_block(Block::Blockquote {
                        children: blocks,
                        span: Span::NONE,
                    });
                }
            }
            OwnedEvent::StartList { ordered } => {
                self.stack.push(Frame::List {
                    ordered,
                    items: vec![],
                });
            }
            OwnedEvent::EndList => {
                if let Some(Frame::List { ordered, items }) = self.stack.pop() {
                    self.push_block(Block::List {
                        ordered,
                        items,
                        span: Span::NONE,
                    });
                }
            }
            OwnedEvent::StartListItem => {
                self.stack.push(Frame::ListItem { inlines: vec![] });
            }
            OwnedEvent::EndListItem => {
                if let Some(Frame::ListItem { inlines }) = self.stack.pop() {
                    if let Some(Frame::List { items, .. }) = self.stack.last_mut() {
                        items.push(inlines);
                    }
                }
            }
            OwnedEvent::CodeBlock { variant, content } => {
                self.push_block(Block::CodeBlock {
                    variant,
                    content: content.into_owned(),
                    span: Span::NONE,
                });
            }
            OwnedEvent::StartDefinitionList => {
                self.stack
                    .push(Frame::DefinitionList { items: vec![] });
            }
            OwnedEvent::EndDefinitionList => {
                if let Some(Frame::DefinitionList { items }) = self.stack.pop() {
                    self.push_block(Block::DefinitionList {
                        items,
                        span: Span::NONE,
                    });
                }
            }
            OwnedEvent::StartDefinitionTerm => {
                self.stack
                    .push(Frame::DefinitionTerm { inlines: vec![] });
            }
            OwnedEvent::EndDefinitionTerm => {
                if let Some(Frame::DefinitionTerm { inlines }) = self.stack.pop() {
                    if let Some(Frame::DefinitionList { items }) = self.stack.last_mut() {
                        items.push((inlines, vec![]));
                    }
                }
            }
            OwnedEvent::StartDefinitionDesc => {
                self.stack
                    .push(Frame::DefinitionDesc { blocks: vec![] });
            }
            OwnedEvent::EndDefinitionDesc => {
                if let Some(Frame::DefinitionDesc { blocks }) = self.stack.pop() {
                    if let Some(Frame::DefinitionList { items }) = self.stack.last_mut() {
                        if let Some(last) = items.last_mut() {
                            last.1 = blocks;
                        }
                    }
                }
            }
            OwnedEvent::StartTable => {
                self.stack.push(Frame::Table { rows: vec![] });
            }
            OwnedEvent::EndTable => {
                if let Some(Frame::Table { rows }) = self.stack.pop() {
                    self.push_block(Block::Table {
                        rows,
                        span: Span::NONE,
                    });
                }
            }
            OwnedEvent::StartTableRow { is_header } => {
                self.stack.push(Frame::TableRow {
                    is_header,
                    cells: vec![],
                });
            }
            OwnedEvent::EndTableRow => {
                if let Some(Frame::TableRow { is_header, cells }) = self.stack.pop() {
                    if let Some(Frame::Table { rows }) = self.stack.last_mut() {
                        rows.push(TableRow { is_header, cells });
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
            OwnedEvent::StartMenu => {
                self.stack.push(Frame::Menu { entries: vec![] });
            }
            OwnedEvent::EndMenu => {
                if let Some(Frame::Menu { entries }) = self.stack.pop() {
                    self.push_block(Block::Menu {
                        entries,
                        span: Span::NONE,
                    });
                }
            }
            OwnedEvent::MenuEntry { node, description } => {
                if let Some(Frame::Menu { entries }) = self.stack.last_mut() {
                    entries.push(MenuEntry { node, description });
                }
            }
            OwnedEvent::HorizontalRule => {
                self.push_block(Block::HorizontalRule { span: Span::NONE });
            }
            OwnedEvent::RawBlock { environment, content } => {
                self.push_block(Block::RawBlock {
                    environment,
                    content,
                    span: Span::NONE,
                });
            }
            OwnedEvent::StartFloat { float_type, label } => {
                self.stack.push(Frame::Float {
                    float_type,
                    label,
                    children: vec![],
                });
            }
            OwnedEvent::EndFloat => {
                if let Some(Frame::Float {
                    float_type,
                    label,
                    children,
                }) = self.stack.pop()
                {
                    self.push_block(Block::Float {
                        float_type,
                        label,
                        children,
                        span: Span::NONE,
                    });
                }
            }
            OwnedEvent::NoIndent => {
                self.push_block(Block::NoIndent { span: Span::NONE });
            }

            // ── Inline events ────────────────────────────────────────────────
            OwnedEvent::Text(cow) => {
                self.push_inline(Inline::Text(cow.into_owned(), Span::NONE));
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
            OwnedEvent::InlineCode(cow) => {
                self.push_inline(Inline::Code(cow.into_owned(), Span::NONE));
            }
            OwnedEvent::StartVar => {
                self.stack.push(Frame::Var { inlines: vec![] });
            }
            OwnedEvent::EndVar => {
                if let Some(Frame::Var { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Var(inlines, Span::NONE));
                }
            }
            OwnedEvent::File(cow) => {
                self.push_inline(Inline::File(cow.into_owned(), Span::NONE));
            }
            OwnedEvent::Command(cow) => {
                self.push_inline(Inline::Command(cow.into_owned(), Span::NONE));
            }
            OwnedEvent::Option(cow) => {
                self.push_inline(Inline::Option(cow.into_owned(), Span::NONE));
            }
            OwnedEvent::Env(cow) => {
                self.push_inline(Inline::Env(cow.into_owned(), Span::NONE));
            }
            OwnedEvent::Samp(cow) => {
                self.push_inline(Inline::Samp(cow.into_owned(), Span::NONE));
            }
            OwnedEvent::Kbd(cow) => {
                self.push_inline(Inline::Kbd(cow.into_owned(), Span::NONE));
            }
            OwnedEvent::Key(cow) => {
                self.push_inline(Inline::Key(cow.into_owned(), Span::NONE));
            }
            OwnedEvent::StartDfn => {
                self.stack.push(Frame::Dfn { inlines: vec![] });
            }
            OwnedEvent::EndDfn => {
                if let Some(Frame::Dfn { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Dfn(inlines, Span::NONE));
                }
            }
            OwnedEvent::Cite(cow) => {
                self.push_inline(Inline::Cite(cow.into_owned(), Span::NONE));
            }
            OwnedEvent::Acronym { abbrev, expansion } => {
                self.push_inline(Inline::Acronym {
                    abbrev,
                    expansion,
                    span: Span::NONE,
                });
            }
            OwnedEvent::Abbr { abbrev, expansion } => {
                self.push_inline(Inline::Abbr {
                    abbrev,
                    expansion,
                    span: Span::NONE,
                });
            }
            OwnedEvent::Roman(cow) => {
                self.push_inline(Inline::Roman(cow.into_owned(), Span::NONE));
            }
            OwnedEvent::SmallCaps(cow) => {
                self.push_inline(Inline::SmallCaps(cow.into_owned(), Span::NONE));
            }
            OwnedEvent::StartDirectItalic => {
                self.stack
                    .push(Frame::DirectItalic { inlines: vec![] });
            }
            OwnedEvent::EndDirectItalic => {
                if let Some(Frame::DirectItalic { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::DirectItalic(inlines, Span::NONE));
                }
            }
            OwnedEvent::StartDirectBold => {
                self.stack.push(Frame::DirectBold { inlines: vec![] });
            }
            OwnedEvent::EndDirectBold => {
                if let Some(Frame::DirectBold { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::DirectBold(inlines, Span::NONE));
                }
            }
            OwnedEvent::DirectTypewriter(cow) => {
                self.push_inline(Inline::DirectTypewriter(cow.into_owned(), Span::NONE));
            }
            OwnedEvent::StartLink { url } => {
                self.stack.push(Frame::Link {
                    url,
                    inlines: vec![],
                });
            }
            OwnedEvent::EndLink => {
                if let Some(Frame::Link { url, inlines }) = self.stack.pop() {
                    self.push_inline(Inline::Link {
                        url,
                        children: inlines,
                        span: Span::NONE,
                    });
                }
            }
            OwnedEvent::Image { file, width, height, alt, extension } => {
                self.push_inline(Inline::Image {
                    file,
                    width,
                    height,
                    alt,
                    extension,
                    span: Span::NONE,
                });
            }
            OwnedEvent::StartSuperscript => {
                self.stack
                    .push(Frame::Superscript { inlines: vec![] });
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
            OwnedEvent::StartFootnoteDef => {
                self.stack
                    .push(Frame::FootnoteDef { inlines: vec![] });
            }
            OwnedEvent::EndFootnoteDef => {
                if let Some(Frame::FootnoteDef { inlines }) = self.stack.pop() {
                    self.push_inline(Inline::FootnoteDef {
                        content: inlines,
                        span: Span::NONE,
                    });
                }
            }
            OwnedEvent::CrossRef { kind, node, text } => {
                self.push_inline(Inline::CrossRef {
                    kind,
                    node,
                    text,
                    span: Span::NONE,
                });
            }
            OwnedEvent::Anchor { name } => {
                self.push_inline(Inline::Anchor {
                    name,
                    span: Span::NONE,
                });
            }
            OwnedEvent::NoBreak(cow) => {
                self.push_inline(Inline::NoBreak(cow.into_owned(), Span::NONE));
            }
            OwnedEvent::Email { address, text } => {
                self.push_inline(Inline::Email {
                    address,
                    text,
                    span: Span::NONE,
                });
            }
            OwnedEvent::Symbol(kind) => {
                self.push_inline(Inline::Symbol(kind, Span::NONE));
            }
        }
    }

    fn push_block(&mut self, block: Block) {
        match self.stack.last_mut() {
            Some(Frame::Document { blocks }) => blocks.push(block),
            Some(Frame::Blockquote { blocks }) => blocks.push(block),
            Some(Frame::DefinitionDesc { blocks }) => blocks.push(block),
            Some(Frame::Float { children, .. }) => children.push(block),
            _ => {} // unexpected context, discard
        }
    }

    fn push_inline(&mut self, inline: Inline) {
        match self.stack.last_mut() {
            Some(Frame::Paragraph { inlines }) => inlines.push(inline),
            Some(Frame::Heading { inlines, .. }) => inlines.push(inline),
            Some(Frame::ListItem { inlines }) => inlines.push(inline),
            Some(Frame::DefinitionTerm { inlines }) => inlines.push(inline),
            Some(Frame::TableCell { inlines }) => inlines.push(inline),
            Some(Frame::Strong { inlines }) => inlines.push(inline),
            Some(Frame::Emphasis { inlines }) => inlines.push(inline),
            Some(Frame::Var { inlines }) => inlines.push(inline),
            Some(Frame::Dfn { inlines }) => inlines.push(inline),
            Some(Frame::DirectItalic { inlines }) => inlines.push(inline),
            Some(Frame::DirectBold { inlines }) => inlines.push(inline),
            Some(Frame::Link { inlines, .. }) => inlines.push(inline),
            Some(Frame::Superscript { inlines }) => inlines.push(inline),
            Some(Frame::Subscript { inlines }) => inlines.push(inline),
            Some(Frame::FootnoteDef { inlines }) => inlines.push(inline),
            _ => {} // unexpected context, discard
        }
    }

    fn finish(mut self) -> TexinfoDoc {
        let blocks = match self.stack.pop() {
            Some(Frame::Document { blocks }) => blocks,
            _ => vec![],
        };
        TexinfoDoc {
            title: None,
            blocks,
            span: Span::NONE,
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
        w.write_event(OwnedEvent::StartHeading {
            level: 1,
            kind: HeadingKind::Numbered,
        });
        w.write_event(OwnedEvent::Text(Cow::Owned("Hello".to_string())));
        w.write_event(OwnedEvent::EndHeading);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("@chapter Hello"), "got: {s:?}");
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
        let input = "@chapter Hello\n\nA paragraph with @strong{bold} text.\n\n@itemize\n@item one\n@item two\n@end itemize\n";
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
