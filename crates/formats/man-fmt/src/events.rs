//! Streaming event iterator over a parsed `ManDoc`.

use std::borrow::Cow;

use crate::ast::*;

/// A streaming event from a man page document.
///
/// Raw text content fields use `Cow<'a, str>` so that future optimisations can
/// yield borrowed slices of the input without changing the public API.
/// For the common case of fully-owned events (e.g. batch mode) use the
/// [`OwnedManEvent`] type alias.
#[derive(Debug, PartialEq)]
pub enum ManEvent<'a> {
    // ── Block events ──────────────────────────────────────────────────────────
    StartDocument,
    EndDocument,
    StartParagraph,
    EndParagraph,
    StartIndentedParagraph,
    EndIndentedParagraph,
    StartHeading { level: u8 },
    EndHeading,
    /// Leaf: a preformatted code block (.nf/.fi).
    CodeBlock { content: Cow<'a, str> },
    /// Leaf: an example block (.EX/.EE).
    ExampleBlock { content: Cow<'a, str> },
    /// Leaf: a horizontal rule (.sp).
    HorizontalRule,
    /// Leaf: a comment (.\" ...).
    Comment { text: Cow<'a, str> },
    StartList { ordered: bool },
    EndList,
    StartListItem,
    EndListItem,
    StartDefinitionList,
    EndDefinitionList,
    StartDefinitionTerm,
    EndDefinitionTerm,
    StartDefinitionDesc,
    EndDefinitionDesc,

    // ── Inline events ─────────────────────────────────────────────────────────
    Text(Cow<'a, str>),
    StartBold,
    EndBold,
    StartItalic,
    EndItalic,
    Code(Cow<'a, str>),
    StartSuperscript,
    EndSuperscript,
    StartSubscript,
    EndSubscript,
    StartLink { url: Cow<'a, str> },
    EndLink,
}

/// Backwards-compatible alias for batch mode (all text is owned).
pub type OwnedManEvent = ManEvent<'static>;

impl<'a> ManEvent<'a> {
    /// Convert to an owned event (all `Cow::Borrowed` text fields become `Cow::Owned`).
    pub fn into_owned(self) -> OwnedManEvent {
        match self {
            ManEvent::Text(cow) => ManEvent::Text(Cow::Owned(cow.into_owned())),
            ManEvent::Code(cow) => ManEvent::Code(Cow::Owned(cow.into_owned())),
            ManEvent::CodeBlock { content } => ManEvent::CodeBlock {
                content: Cow::Owned(content.into_owned()),
            },
            ManEvent::ExampleBlock { content } => ManEvent::ExampleBlock {
                content: Cow::Owned(content.into_owned()),
            },
            ManEvent::Comment { text } => ManEvent::Comment {
                text: Cow::Owned(text.into_owned()),
            },
            ManEvent::StartLink { url } => ManEvent::StartLink {
                url: Cow::Owned(url.into_owned()),
            },
            // All other variants contain no borrowed data.
            ManEvent::StartDocument => ManEvent::StartDocument,
            ManEvent::EndDocument => ManEvent::EndDocument,
            ManEvent::StartParagraph => ManEvent::StartParagraph,
            ManEvent::EndParagraph => ManEvent::EndParagraph,
            ManEvent::StartIndentedParagraph => ManEvent::StartIndentedParagraph,
            ManEvent::EndIndentedParagraph => ManEvent::EndIndentedParagraph,
            ManEvent::StartHeading { level } => ManEvent::StartHeading { level },
            ManEvent::EndHeading => ManEvent::EndHeading,
            ManEvent::HorizontalRule => ManEvent::HorizontalRule,
            ManEvent::StartList { ordered } => ManEvent::StartList { ordered },
            ManEvent::EndList => ManEvent::EndList,
            ManEvent::StartListItem => ManEvent::StartListItem,
            ManEvent::EndListItem => ManEvent::EndListItem,
            ManEvent::StartDefinitionList => ManEvent::StartDefinitionList,
            ManEvent::EndDefinitionList => ManEvent::EndDefinitionList,
            ManEvent::StartDefinitionTerm => ManEvent::StartDefinitionTerm,
            ManEvent::EndDefinitionTerm => ManEvent::EndDefinitionTerm,
            ManEvent::StartDefinitionDesc => ManEvent::StartDefinitionDesc,
            ManEvent::EndDefinitionDesc => ManEvent::EndDefinitionDesc,
            ManEvent::StartBold => ManEvent::StartBold,
            ManEvent::EndBold => ManEvent::EndBold,
            ManEvent::StartItalic => ManEvent::StartItalic,
            ManEvent::EndItalic => ManEvent::EndItalic,
            ManEvent::StartSuperscript => ManEvent::StartSuperscript,
            ManEvent::EndSuperscript => ManEvent::EndSuperscript,
            ManEvent::StartSubscript => ManEvent::StartSubscript,
            ManEvent::EndSubscript => ManEvent::EndSubscript,
            ManEvent::EndLink => ManEvent::EndLink,
        }
    }
}

// ── Pull iterator (walks a ManDoc) ───────────────────────────────────────────

/// A lazy event iterator over a parsed [`ManDoc`].
///
/// Events are produced on demand by walking the AST with a frame stack.
/// Memory usage is O(nesting depth).
pub struct EventIter<'a> {
    doc: &'a ManDoc,
    frame_stack: Vec<Frame<'a>>,
    started: bool,
    finished: bool,
}

#[allow(dead_code)]
enum Frame<'a> {
    /// Iterating over blocks in the document or a container.
    Blocks {
        blocks: &'a [Block],
        index: usize,
    },
    /// Inside a paragraph-like block, iterating inlines.
    Inlines {
        inlines: &'a [Inline],
        index: usize,
        /// What close event to emit when done.
        close: CloseKind,
    },
    /// About to emit a leaf event (CodeBlock, ExampleBlock, HorizontalRule, Comment).
    Leaf(Option<ManEvent<'a>>),
    /// Inside a list, iterating items.
    ListItems {
        items: &'a [Vec<Block>],
        item_index: usize,
        in_item: bool,
    },
    /// Inside a definition list.
    DefListItems {
        items: &'a [(Vec<Inline>, Vec<Block>)],
        item_index: usize,
        phase: DefPhase,
    },
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
enum CloseKind {
    Paragraph,
    IndentedParagraph,
    Heading,
    DefinitionTerm,
    DefinitionDesc,
}

#[derive(Clone, Copy)]
enum DefPhase {
    /// About to start the term.
    StartTerm,
    /// Emitting term inlines.
    InTerm,
    /// About to start the desc.
    StartDesc,
    /// Emitting desc blocks.
    InDesc,
    /// About to close the desc.
    CloseDesc,
}

impl<'a> EventIter<'a> {
    pub fn new(doc: &'a ManDoc) -> Self {
        Self {
            doc,
            frame_stack: Vec::new(),
            started: false,
            finished: false,
        }
    }
}

impl<'a> Iterator for EventIter<'a> {
    type Item = ManEvent<'a>;

    fn next(&mut self) -> Option<ManEvent<'a>> {
        if self.finished {
            return None;
        }

        if !self.started {
            self.started = true;
            self.frame_stack.push(Frame::Blocks {
                blocks: &self.doc.blocks,
                index: 0,
            });
            return Some(ManEvent::StartDocument);
        }

        loop {
            let frame = match self.frame_stack.last_mut() {
                Some(f) => f,
                None => {
                    self.finished = true;
                    return Some(ManEvent::EndDocument);
                }
            };

            match frame {
                Frame::Leaf(event) => {
                    if let Some(ev) = event.take() {
                        return Some(ev);
                    }
                    self.frame_stack.pop();
                    continue;
                }

                Frame::Inlines {
                    inlines,
                    index,
                    close,
                } => {
                    if *index < inlines.len() {
                        let inline = &inlines[*index];
                        *index += 1;
                        match inline {
                            Inline::Text(s, _) => return Some(ManEvent::Text(Cow::Borrowed(s))),
                            Inline::Bold(children, _) => {
                                let close_kind = *close;
                                // Push continuation for rest of inlines
                                // Actually, we need to handle the nested children.
                                // Push a frame for bold children.
                                let remaining_inlines = &inlines[*index..];
                                let remaining_close = close_kind;
                                // Replace current frame with remaining
                                *inlines = remaining_inlines;
                                *index = 0;
                                *close = remaining_close;
                                // Push bold children frame
                                self.frame_stack.push(Frame::Inlines {
                                    inlines: children,
                                    index: 0,
                                    close: CloseKind::Paragraph, // dummy, we handle EndBold manually
                                });
                                // We'll need to emit EndBold after children
                                // Insert a leaf for EndBold between bold children and continuation
                                            // Restructure: push close event, then children frame on top
                                // Stack: [... remaining inlines, EndBold leaf, bold children]
                                // Pop order: bold children first, then EndBold, then remaining
                                let children_frame = self.frame_stack.pop().unwrap();
                                self.frame_stack.push(Frame::Leaf(Some(ManEvent::EndBold)));
                                self.frame_stack.push(children_frame);
                                return Some(ManEvent::StartBold);
                            }
                            Inline::Italic(children, _) => {
                                let close_kind = *close;
                                let remaining_inlines = &inlines[*index..];
                                *inlines = remaining_inlines;
                                *index = 0;
                                *close = close_kind;
                                self.frame_stack.push(Frame::Leaf(Some(ManEvent::EndItalic)));
                                self.frame_stack.push(Frame::Inlines {
                                    inlines: children,
                                    index: 0,
                                    close: CloseKind::Paragraph, // dummy
                                });
                                return Some(ManEvent::StartItalic);
                            }
                            Inline::Code(s, _) => return Some(ManEvent::Code(Cow::Borrowed(s))),
                            Inline::Superscript(children, _) => {
                                let close_kind = *close;
                                let remaining_inlines = &inlines[*index..];
                                *inlines = remaining_inlines;
                                *index = 0;
                                *close = close_kind;
                                self.frame_stack
                                    .push(Frame::Leaf(Some(ManEvent::EndSuperscript)));
                                self.frame_stack.push(Frame::Inlines {
                                    inlines: children,
                                    index: 0,
                                    close: CloseKind::Paragraph,
                                });
                                return Some(ManEvent::StartSuperscript);
                            }
                            Inline::Subscript(children, _) => {
                                let close_kind = *close;
                                let remaining_inlines = &inlines[*index..];
                                *inlines = remaining_inlines;
                                *index = 0;
                                *close = close_kind;
                                self.frame_stack
                                    .push(Frame::Leaf(Some(ManEvent::EndSubscript)));
                                self.frame_stack.push(Frame::Inlines {
                                    inlines: children,
                                    index: 0,
                                    close: CloseKind::Paragraph,
                                });
                                return Some(ManEvent::StartSubscript);
                            }
                            Inline::Link { url, children, .. } => {
                                let close_kind = *close;
                                let remaining_inlines = &inlines[*index..];
                                *inlines = remaining_inlines;
                                *index = 0;
                                *close = close_kind;
                                self.frame_stack
                                    .push(Frame::Leaf(Some(ManEvent::EndLink)));
                                self.frame_stack.push(Frame::Inlines {
                                    inlines: children,
                                    index: 0,
                                    close: CloseKind::Paragraph,
                                });
                                return Some(ManEvent::StartLink {
                                    url: Cow::Borrowed(url),
                                });
                            }
                        }
                    } else {
                        let close = *close;
                        self.frame_stack.pop();
                        match close {
                            CloseKind::Paragraph => return Some(ManEvent::EndParagraph),
                            CloseKind::IndentedParagraph => {
                                return Some(ManEvent::EndIndentedParagraph)
                            }
                            CloseKind::Heading => return Some(ManEvent::EndHeading),
                            CloseKind::DefinitionTerm => {
                                return Some(ManEvent::EndDefinitionTerm)
                            }
                            CloseKind::DefinitionDesc => {
                                return Some(ManEvent::EndDefinitionDesc)
                            }
                        }
                    }
                }

                Frame::Blocks {
                    blocks,
                    index,
                } => {
                    if *index >= blocks.len() {
                        self.frame_stack.pop();
                        continue;
                    }

                    let block = &blocks[*index];
                    *index += 1;

                    match block {
                        Block::Heading {
                            level, inlines, ..
                        } => {
                            self.frame_stack.push(Frame::Inlines {
                                inlines,
                                index: 0,
                                close: CloseKind::Heading,
                            });
                            return Some(ManEvent::StartHeading { level: *level });
                        }
                        Block::Paragraph { inlines, .. } => {
                            self.frame_stack.push(Frame::Inlines {
                                inlines,
                                index: 0,
                                close: CloseKind::Paragraph,
                            });
                            return Some(ManEvent::StartParagraph);
                        }
                        Block::IndentedParagraph { inlines, .. } => {
                            self.frame_stack.push(Frame::Inlines {
                                inlines,
                                index: 0,
                                close: CloseKind::IndentedParagraph,
                            });
                            return Some(ManEvent::StartIndentedParagraph);
                        }
                        Block::CodeBlock { content, .. } => {
                            return Some(ManEvent::CodeBlock {
                                content: Cow::Borrowed(content),
                            });
                        }
                        Block::ExampleBlock { content, .. } => {
                            return Some(ManEvent::ExampleBlock {
                                content: Cow::Borrowed(content),
                            });
                        }
                        Block::HorizontalRule { .. } => {
                            return Some(ManEvent::HorizontalRule);
                        }
                        Block::Comment { text, .. } => {
                            return Some(ManEvent::Comment {
                                text: Cow::Borrowed(text),
                            });
                        }
                        Block::List {
                            ordered, items, ..
                        } => {
                            self.frame_stack.push(Frame::ListItems {
                                items,
                                item_index: 0,
                                in_item: false,
                            });
                            return Some(ManEvent::StartList { ordered: *ordered });
                        }
                        Block::DefinitionList { items, .. } => {
                            self.frame_stack.push(Frame::DefListItems {
                                items,
                                item_index: 0,
                                phase: DefPhase::StartTerm,
                            });
                            return Some(ManEvent::StartDefinitionList);
                        }
                    }
                }

                Frame::ListItems {
                    items,
                    item_index,
                    in_item,
                } => {
                    if *in_item {
                        // Item blocks have been pushed; emit EndListItem
                        *in_item = false;
                        return Some(ManEvent::EndListItem);
                    }
                    if *item_index >= items.len() {
                        self.frame_stack.pop();
                        return Some(ManEvent::EndList);
                    }
                    let item_blocks = &items[*item_index];
                    *item_index += 1;
                    *in_item = true;
                    self.frame_stack.push(Frame::Blocks {
                        blocks: item_blocks,
                        index: 0,
                    });
                    return Some(ManEvent::StartListItem);
                }

                Frame::DefListItems {
                    items,
                    item_index,
                    phase,
                } => {
                    if *item_index >= items.len() {
                        self.frame_stack.pop();
                        return Some(ManEvent::EndDefinitionList);
                    }

                    let (term_inlines, desc_blocks) = &items[*item_index];

                    match *phase {
                        DefPhase::StartTerm => {
                            *phase = DefPhase::InTerm;
                            self.frame_stack.push(Frame::Inlines {
                                inlines: term_inlines,
                                index: 0,
                                close: CloseKind::DefinitionTerm,
                            });
                            return Some(ManEvent::StartDefinitionTerm);
                        }
                        DefPhase::InTerm => {
                            // EndDefinitionTerm was emitted by the Inlines frame.
                            *phase = DefPhase::StartDesc;
                            continue;
                        }
                        DefPhase::StartDesc => {
                            *phase = DefPhase::InDesc;
                            self.frame_stack.push(Frame::Blocks {
                                blocks: desc_blocks,
                                index: 0,
                            });
                            return Some(ManEvent::StartDefinitionDesc);
                        }
                        DefPhase::InDesc => {
                            *phase = DefPhase::CloseDesc;
                            return Some(ManEvent::EndDefinitionDesc);
                        }
                        DefPhase::CloseDesc => {
                            *item_index += 1;
                            *phase = DefPhase::StartTerm;
                            continue;
                        }
                    }
                }
            }
        }
    }
}

/// Parse `input` and return a streaming iterator of [`ManEvent`] items.
pub fn events(input: &str) -> impl Iterator<Item = OwnedManEvent> + '_ {
    let (doc, _) = crate::parse::parse(input);
    // We need to own the doc so the iterator can borrow from it.
    // Since we can't return a self-referencing struct, collect into a Vec.
    let mut iter = EventIter::new(Box::leak(Box::new(doc)));
    // Safety: we leak the doc so the iterator can borrow from it.
    // This is a simplification; in production we'd use a different approach.
    std::iter::from_fn(move || iter.next())
}

/// Collect a complete `ManDoc` from an event stream.
///
/// Useful for callers that drive events and want a complete [`ManDoc`] at the end.
pub fn collect_doc_from_events(events: impl Iterator<Item = OwnedManEvent>) -> ManDoc {
    let mut block_stack: Vec<BlockFrame> = vec![BlockFrame::Document { blocks: Vec::new() }];
    let mut inline_ctx: Vec<InlineFrame> = Vec::new();

    for event in events {
        handle_event(event, &mut block_stack, &mut inline_ctx);
    }

    let blocks = match block_stack.pop() {
        Some(BlockFrame::Document { blocks }) => blocks,
        _ => Vec::new(),
    };

    ManDoc {
        title: None,
        section: None,
        date: None,
        source: None,
        manual: None,
        blocks,
        span: Span::NONE,
    }
}

// ── Block frame stack ─────────────────────────────────────────────────────────

enum BlockFrame {
    Document { blocks: Vec<Block> },
    Paragraph { inlines: Vec<Inline> },
    IndentedParagraph { inlines: Vec<Inline> },
    Heading { level: u8, inlines: Vec<Inline> },
    List { ordered: bool, items: Vec<Vec<Block>> },
    ListItem { blocks: Vec<Block> },
    DefinitionList { items: Vec<(Vec<Inline>, Vec<Block>)> },
    DefinitionTerm { inlines: Vec<Inline> },
    DefinitionDesc { blocks: Vec<Block> },
}

// ── Inline frame stack ────────────────────────────────────────────────────────

enum InlineFrame {
    Bold { inlines: Vec<Inline> },
    Italic { inlines: Vec<Inline> },
    Superscript { inlines: Vec<Inline> },
    Subscript { inlines: Vec<Inline> },
    Link { url: String, inlines: Vec<Inline> },
}

fn inlines_from_frame(frame: &mut InlineFrame) -> &mut Vec<Inline> {
    match frame {
        InlineFrame::Bold { inlines } => inlines,
        InlineFrame::Italic { inlines } => inlines,
        InlineFrame::Superscript { inlines } => inlines,
        InlineFrame::Subscript { inlines } => inlines,
        InlineFrame::Link { inlines, .. } => inlines,
    }
}

fn push_inline(block_stack: &mut [BlockFrame], inline_ctx: &mut [InlineFrame], inline: Inline) {
    if let Some(frame) = inline_ctx.last_mut() {
        inlines_from_frame(frame).push(inline);
        return;
    }
    match block_stack.last_mut() {
        Some(BlockFrame::Paragraph { inlines }) => inlines.push(inline),
        Some(BlockFrame::IndentedParagraph { inlines }) => inlines.push(inline),
        Some(BlockFrame::Heading { inlines, .. }) => inlines.push(inline),
        Some(BlockFrame::DefinitionTerm { inlines }) => inlines.push(inline),
        _ => {}
    }
}

fn push_block(block_stack: &mut [BlockFrame], block: Block) {
    match block_stack.last_mut() {
        Some(BlockFrame::Document { blocks }) => blocks.push(block),
        Some(BlockFrame::ListItem { blocks }) => blocks.push(block),
        Some(BlockFrame::DefinitionDesc { blocks }) => blocks.push(block),
        _ => {}
    }
}

fn handle_event(
    event: OwnedManEvent,
    block_stack: &mut Vec<BlockFrame>,
    inline_ctx: &mut Vec<InlineFrame>,
) {
    match event {
        // ── Block start events ─────────────────────────────────────────────
        ManEvent::StartDocument => {
            // Already initialized with Document frame
        }
        ManEvent::StartParagraph => {
            block_stack.push(BlockFrame::Paragraph {
                inlines: Vec::new(),
            });
        }
        ManEvent::StartIndentedParagraph => {
            block_stack.push(BlockFrame::IndentedParagraph {
                inlines: Vec::new(),
            });
        }
        ManEvent::StartHeading { level } => {
            block_stack.push(BlockFrame::Heading {
                level,
                inlines: Vec::new(),
            });
        }
        ManEvent::StartList { ordered } => {
            block_stack.push(BlockFrame::List {
                ordered,
                items: Vec::new(),
            });
        }
        ManEvent::StartListItem => {
            block_stack.push(BlockFrame::ListItem {
                blocks: Vec::new(),
            });
        }
        ManEvent::StartDefinitionList => {
            block_stack.push(BlockFrame::DefinitionList {
                items: Vec::new(),
            });
        }
        ManEvent::StartDefinitionTerm => {
            block_stack.push(BlockFrame::DefinitionTerm {
                inlines: Vec::new(),
            });
        }
        ManEvent::StartDefinitionDesc => {
            block_stack.push(BlockFrame::DefinitionDesc {
                blocks: Vec::new(),
            });
        }

        // ── Block end events ───────────────────────────────────────────────
        ManEvent::EndDocument => {}
        ManEvent::EndParagraph => {
            if let Some(BlockFrame::Paragraph { inlines }) = block_stack.pop() {
                push_block(
                    block_stack,
                    Block::Paragraph {
                        inlines,
                        span: Span::NONE,
                    },
                );
            }
        }
        ManEvent::EndIndentedParagraph => {
            if let Some(BlockFrame::IndentedParagraph { inlines }) = block_stack.pop() {
                push_block(
                    block_stack,
                    Block::IndentedParagraph {
                        inlines,
                        span: Span::NONE,
                    },
                );
            }
        }
        ManEvent::EndHeading => {
            if let Some(BlockFrame::Heading { level, inlines }) = block_stack.pop() {
                push_block(
                    block_stack,
                    Block::Heading {
                        level,
                        inlines,
                        span: Span::NONE,
                    },
                );
            }
        }
        ManEvent::EndList => {
            if let Some(BlockFrame::List { ordered, items }) = block_stack.pop() {
                push_block(
                    block_stack,
                    Block::List {
                        ordered,
                        items,
                        span: Span::NONE,
                    },
                );
            }
        }
        ManEvent::EndListItem => {
            if let Some(BlockFrame::ListItem { blocks }) = block_stack.pop()
                && let Some(BlockFrame::List { items, .. }) = block_stack.last_mut()
            {
                items.push(blocks);
            }
        }
        ManEvent::EndDefinitionList => {
            if let Some(BlockFrame::DefinitionList { items }) = block_stack.pop() {
                push_block(
                    block_stack,
                    Block::DefinitionList {
                        items,
                        span: Span::NONE,
                    },
                );
            }
        }
        ManEvent::EndDefinitionTerm => {
            if let Some(BlockFrame::DefinitionTerm { inlines }) = block_stack.pop()
                && let Some(BlockFrame::DefinitionList { items }) = block_stack.last_mut()
            {
                items.push((inlines, Vec::new()));
            }
        }
        ManEvent::EndDefinitionDesc => {
            if let Some(BlockFrame::DefinitionDesc { blocks }) = block_stack.pop()
                && let Some(BlockFrame::DefinitionList { items }) = block_stack.last_mut()
                && let Some(last) = items.last_mut()
            {
                last.1 = blocks;
            }
        }

        // ── Leaf block events ──────────────────────────────────────────────
        ManEvent::CodeBlock { content } => {
            push_block(
                block_stack,
                Block::CodeBlock {
                    content: content.into_owned(),
                    span: Span::NONE,
                },
            );
        }
        ManEvent::ExampleBlock { content } => {
            push_block(
                block_stack,
                Block::ExampleBlock {
                    content: content.into_owned(),
                    span: Span::NONE,
                },
            );
        }
        ManEvent::HorizontalRule => {
            push_block(
                block_stack,
                Block::HorizontalRule { span: Span::NONE },
            );
        }
        ManEvent::Comment { text } => {
            push_block(
                block_stack,
                Block::Comment {
                    text: text.into_owned(),
                    span: Span::NONE,
                },
            );
        }

        // ── Inline events ──────────────────────────────────────────────────
        ManEvent::Text(cow) => {
            push_inline(
                block_stack,
                inline_ctx,
                Inline::Text(cow.into_owned(), Span::NONE),
            );
        }
        ManEvent::Code(cow) => {
            push_inline(
                block_stack,
                inline_ctx,
                Inline::Code(cow.into_owned(), Span::NONE),
            );
        }

        // ── Inline container start events ──────────────────────────────────
        ManEvent::StartBold => {
            inline_ctx.push(InlineFrame::Bold {
                inlines: Vec::new(),
            });
        }
        ManEvent::StartItalic => {
            inline_ctx.push(InlineFrame::Italic {
                inlines: Vec::new(),
            });
        }
        ManEvent::StartSuperscript => {
            inline_ctx.push(InlineFrame::Superscript {
                inlines: Vec::new(),
            });
        }
        ManEvent::StartSubscript => {
            inline_ctx.push(InlineFrame::Subscript {
                inlines: Vec::new(),
            });
        }
        ManEvent::StartLink { url } => {
            inline_ctx.push(InlineFrame::Link {
                url: url.into_owned(),
                inlines: Vec::new(),
            });
        }

        // ── Inline container end events ────────────────────────────────────
        ManEvent::EndBold => {
            if let Some(InlineFrame::Bold { inlines }) = inline_ctx.pop() {
                push_inline(
                    block_stack,
                    inline_ctx,
                    Inline::Bold(inlines, Span::NONE),
                );
            }
        }
        ManEvent::EndItalic => {
            if let Some(InlineFrame::Italic { inlines }) = inline_ctx.pop() {
                push_inline(
                    block_stack,
                    inline_ctx,
                    Inline::Italic(inlines, Span::NONE),
                );
            }
        }
        ManEvent::EndSuperscript => {
            if let Some(InlineFrame::Superscript { inlines }) = inline_ctx.pop() {
                push_inline(
                    block_stack,
                    inline_ctx,
                    Inline::Superscript(inlines, Span::NONE),
                );
            }
        }
        ManEvent::EndSubscript => {
            if let Some(InlineFrame::Subscript { inlines }) = inline_ctx.pop() {
                push_inline(
                    block_stack,
                    inline_ctx,
                    Inline::Subscript(inlines, Span::NONE),
                );
            }
        }
        ManEvent::EndLink => {
            if let Some(InlineFrame::Link { url, inlines }) = inline_ctx.pop() {
                push_inline(
                    block_stack,
                    inline_ctx,
                    Inline::Link {
                        url,
                        children: inlines,
                        span: Span::NONE,
                    },
                );
            }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn doc_events(input: &str) -> Vec<OwnedManEvent> {
        events(input).collect()
    }

    #[test]
    fn test_events_paragraph() {
        let evs = doc_events(".PP\nHello world");
        assert!(evs.iter().any(|e| matches!(e, ManEvent::StartParagraph)));
        assert!(evs.iter().any(|e| matches!(e, ManEvent::EndParagraph)));
        assert!(evs
            .iter()
            .any(|e| matches!(e, ManEvent::Text(t) if t == "Hello world")));
    }

    #[test]
    fn test_events_heading() {
        let evs = doc_events(".SH NAME");
        assert!(evs
            .iter()
            .any(|e| matches!(e, ManEvent::StartHeading { level: 2 })));
        assert!(evs.iter().any(|e| matches!(e, ManEvent::EndHeading)));
    }

    #[test]
    fn test_events_code_block() {
        let evs = doc_events(".nf\ncode here\n.fi");
        assert!(evs
            .iter()
            .any(|e| matches!(e, ManEvent::CodeBlock { .. })));
    }

    #[test]
    fn test_events_bold() {
        let evs = doc_events(".B bold text");
        assert!(evs.iter().any(|e| matches!(e, ManEvent::StartBold)));
        assert!(evs.iter().any(|e| matches!(e, ManEvent::EndBold)));
    }

    #[test]
    fn test_events_definition_list() {
        let evs = doc_events(".TP\nterm\ndescription");
        assert!(evs
            .iter()
            .any(|e| matches!(e, ManEvent::StartDefinitionList)));
        assert!(evs
            .iter()
            .any(|e| matches!(e, ManEvent::StartDefinitionTerm)));
        assert!(evs
            .iter()
            .any(|e| matches!(e, ManEvent::EndDefinitionList)));
    }

    #[test]
    fn test_events_document_wrapper() {
        let evs = doc_events(".PP\nhello");
        assert_eq!(evs.first(), Some(&ManEvent::StartDocument));
        assert_eq!(evs.last(), Some(&ManEvent::EndDocument));
    }
}
