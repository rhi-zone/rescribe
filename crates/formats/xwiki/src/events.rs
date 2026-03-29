//! Streaming event iterator over a parsed `XwikiDoc`.

use std::borrow::Cow;

use crate::ast::*;

/// A streaming event from an XWiki document.
#[derive(Debug, PartialEq)]
pub enum Event<'a> {
    // ── Block events ──────────────────────────────────────────────────────────
    StartParagraph,
    EndParagraph,
    StartHeading { level: u8 },
    EndHeading,
    StartBlockquote,
    EndBlockquote,
    StartList { ordered: bool },
    EndList,
    StartListItem,
    EndListItem,
    CodeBlock { language: Option<String>, content: Cow<'a, str> },
    HorizontalRule,
    StartTable,
    EndTable,
    StartTableRow,
    EndTableRow,
    StartTableCell { is_header: bool },
    EndTableCell,
    MacroBlock { name: String, params: String, content: String },
    MacroInline { name: String, params: String },

    // ── Inline events ─────────────────────────────────────────────────────────
    Text(Cow<'a, str>),
    SoftBreak,
    LineBreak,
    StartBold,
    EndBold,
    StartItalic,
    EndItalic,
    StartUnderline,
    EndUnderline,
    StartStrikeout,
    EndStrikeout,
    StartSuperscript,
    EndSuperscript,
    StartSubscript,
    EndSubscript,
    InlineCode(Cow<'a, str>),
    StartLink { url: String },
    EndLink,
    InlineImage { url: String, alt: Option<String>, params: Vec<(String, String)> },
}

/// Backwards-compatible alias for batch mode (all text is owned).
pub type OwnedEvent = Event<'static>;

impl<'a> Event<'a> {
    /// Convert to an owned event.
    pub fn into_owned(self) -> OwnedEvent {
        match self {
            Event::Text(cow) => Event::Text(Cow::Owned(cow.into_owned())),
            Event::InlineCode(cow) => Event::InlineCode(Cow::Owned(cow.into_owned())),
            Event::CodeBlock { language, content } => Event::CodeBlock {
                language,
                content: Cow::Owned(content.into_owned()),
            },
            // Safety: all other variants contain only String/'static fields.
            other => unsafe { std::mem::transmute::<Event<'_>, OwnedEvent>(other) },
        }
    }
}

// ── True pull iterator ────────────────────────────────────────────────────────

/// A streaming event iterator over an XWiki document.
///
/// Lazily walks the AST, emitting start/end events for each node.
pub struct EventIter<'a> {
    stack: Vec<Frame<'a>>,
    pending: Option<Event<'a>>,
}

enum Frame<'a> {
    Blocks { blocks: &'a [Block], idx: usize },
    Inlines { inlines: &'a [Inline], idx: usize },
    EndBlock(EndTag),
    EndInline(EndTag),
    TableRows { rows: &'a [TableRow], idx: usize },
    TableCells { cells: &'a [TableCell], idx: usize },
    ListItems { items: &'a [Vec<Block>], idx: usize },
}

#[derive(Clone, Copy)]
enum EndTag {
    Paragraph,
    Heading,
    Blockquote,
    List,
    ListItem,
    Table,
    TableRow,
    TableCell,
    Bold,
    Italic,
    Underline,
    Strikeout,
    Superscript,
    Subscript,
    Link,
}

impl EndTag {
    fn to_event(self) -> OwnedEvent {
        match self {
            EndTag::Paragraph => Event::EndParagraph,
            EndTag::Heading => Event::EndHeading,
            EndTag::Blockquote => Event::EndBlockquote,
            EndTag::List => Event::EndList,
            EndTag::ListItem => Event::EndListItem,
            EndTag::Table => Event::EndTable,
            EndTag::TableRow => Event::EndTableRow,
            EndTag::TableCell => Event::EndTableCell,
            EndTag::Bold => Event::EndBold,
            EndTag::Italic => Event::EndItalic,
            EndTag::Underline => Event::EndUnderline,
            EndTag::Strikeout => Event::EndStrikeout,
            EndTag::Superscript => Event::EndSuperscript,
            EndTag::Subscript => Event::EndSubscript,
            EndTag::Link => Event::EndLink,
        }
    }
}

impl<'a> EventIter<'a> {
    pub fn new(doc: &'a XwikiDoc) -> Self {
        EventIter {
            stack: vec![Frame::Blocks { blocks: &doc.blocks, idx: 0 }],
            pending: None,
        }
    }
}

impl<'a> Iterator for EventIter<'a> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Event<'a>> {
        if let Some(ev) = self.pending.take() {
            return Some(ev);
        }
        loop {
            let frame = self.stack.last_mut()?;
            match frame {
                Frame::EndBlock(tag) => {
                    let ev = tag.to_event();
                    self.stack.pop();
                    return Some(ev);
                }
                Frame::EndInline(tag) => {
                    let ev = tag.to_event();
                    self.stack.pop();
                    return Some(ev);
                }
                Frame::Blocks { blocks, idx } => {
                    if *idx >= blocks.len() {
                        self.stack.pop();
                        continue;
                    }
                    let block = &blocks[*idx];
                    *idx += 1;
                    match block {
                        Block::Paragraph { inlines, .. } => {
                            self.stack.push(Frame::EndBlock(EndTag::Paragraph));
                            self.stack.push(Frame::Inlines { inlines, idx: 0 });
                            return Some(Event::StartParagraph);
                        }
                        Block::Heading { level, inlines, .. } => {
                            self.stack.push(Frame::EndBlock(EndTag::Heading));
                            self.stack.push(Frame::Inlines { inlines, idx: 0 });
                            return Some(Event::StartHeading { level: *level });
                        }
                        Block::CodeBlock { language, content, .. } => {
                            return Some(Event::CodeBlock {
                                language: language.clone(),
                                content: Cow::Borrowed(content),
                            });
                        }
                        Block::HorizontalRule { .. } => {
                            return Some(Event::HorizontalRule);
                        }
                        Block::Table { rows, .. } => {
                            self.stack.push(Frame::EndBlock(EndTag::Table));
                            self.stack.push(Frame::TableRows { rows, idx: 0 });
                            return Some(Event::StartTable);
                        }
                        Block::List { ordered, items, .. } => {
                            self.stack.push(Frame::EndBlock(EndTag::List));
                            self.stack.push(Frame::ListItems { items, idx: 0 });
                            return Some(Event::StartList { ordered: *ordered });
                        }
                        Block::Blockquote { children, .. } => {
                            self.stack.push(Frame::EndBlock(EndTag::Blockquote));
                            self.stack.push(Frame::Blocks { blocks: children, idx: 0 });
                            return Some(Event::StartBlockquote);
                        }
                        Block::MacroBlock { name, params, content, .. } => {
                            return Some(Event::MacroBlock {
                                name: name.clone(),
                                params: params.clone(),
                                content: content.clone(),
                            });
                        }
                        Block::MacroInline { name, params, .. } => {
                            return Some(Event::MacroInline {
                                name: name.clone(),
                                params: params.clone(),
                            });
                        }
                    }
                }
                Frame::TableRows { rows, idx } => {
                    if *idx >= rows.len() {
                        self.stack.pop();
                        continue;
                    }
                    let row = &rows[*idx];
                    *idx += 1;
                    self.stack.push(Frame::EndBlock(EndTag::TableRow));
                    self.stack.push(Frame::TableCells { cells: &row.cells, idx: 0 });
                    return Some(Event::StartTableRow);
                }
                Frame::TableCells { cells, idx } => {
                    if *idx >= cells.len() {
                        self.stack.pop();
                        continue;
                    }
                    let cell = &cells[*idx];
                    *idx += 1;
                    self.stack.push(Frame::EndBlock(EndTag::TableCell));
                    self.stack.push(Frame::Inlines { inlines: &cell.inlines, idx: 0 });
                    return Some(Event::StartTableCell { is_header: cell.is_header });
                }
                Frame::ListItems { items, idx } => {
                    if *idx >= items.len() {
                        self.stack.pop();
                        continue;
                    }
                    let item_blocks = &items[*idx];
                    *idx += 1;
                    self.stack.push(Frame::EndBlock(EndTag::ListItem));
                    self.stack.push(Frame::Blocks { blocks: item_blocks, idx: 0 });
                    return Some(Event::StartListItem);
                }
                Frame::Inlines { inlines, idx } => {
                    if *idx >= inlines.len() {
                        self.stack.pop();
                        continue;
                    }
                    let inline = &inlines[*idx];
                    *idx += 1;
                    match inline {
                        Inline::Text(s, _) => {
                            return Some(Event::Text(Cow::Borrowed(s)));
                        }
                        Inline::Bold(children, _) => {
                            self.stack.push(Frame::EndInline(EndTag::Bold));
                            self.stack.push(Frame::Inlines { inlines: children, idx: 0 });
                            return Some(Event::StartBold);
                        }
                        Inline::Italic(children, _) => {
                            self.stack.push(Frame::EndInline(EndTag::Italic));
                            self.stack.push(Frame::Inlines { inlines: children, idx: 0 });
                            return Some(Event::StartItalic);
                        }
                        Inline::Underline(children, _) => {
                            self.stack.push(Frame::EndInline(EndTag::Underline));
                            self.stack.push(Frame::Inlines { inlines: children, idx: 0 });
                            return Some(Event::StartUnderline);
                        }
                        Inline::Strikeout(children, _) => {
                            self.stack.push(Frame::EndInline(EndTag::Strikeout));
                            self.stack.push(Frame::Inlines { inlines: children, idx: 0 });
                            return Some(Event::StartStrikeout);
                        }
                        Inline::Superscript(children, _) => {
                            self.stack.push(Frame::EndInline(EndTag::Superscript));
                            self.stack.push(Frame::Inlines { inlines: children, idx: 0 });
                            return Some(Event::StartSuperscript);
                        }
                        Inline::Subscript(children, _) => {
                            self.stack.push(Frame::EndInline(EndTag::Subscript));
                            self.stack.push(Frame::Inlines { inlines: children, idx: 0 });
                            return Some(Event::StartSubscript);
                        }
                        Inline::Code(s, _) => {
                            return Some(Event::InlineCode(Cow::Borrowed(s)));
                        }
                        Inline::Link { url, label, .. } => {
                            // Emit StartLink now; queue Text(label) + EndLink.
                            // We can only queue one pending event, so push EndLink
                            // on the stack and set pending to Text(label).
                            self.stack.push(Frame::EndInline(EndTag::Link));
                            self.pending =
                                Some(Event::Text(Cow::Owned(label.clone())));
                            return Some(Event::StartLink { url: url.clone() });
                        }
                        Inline::Image { url, alt, params, .. } => {
                            return Some(Event::InlineImage {
                                url: url.clone(),
                                alt: alt.clone(),
                                params: params.clone(),
                            });
                        }
                        Inline::LineBreak { .. } => return Some(Event::LineBreak),
                        Inline::SoftBreak { .. } => return Some(Event::SoftBreak),
                    }
                }
            }
        }
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Walk `doc` and return a streaming iterator of events.
pub fn events(doc: &XwikiDoc) -> EventIter<'_> {
    EventIter::new(doc)
}

// ── Tree builder ──────────────────────────────────────────────────────────────

/// Collect a complete `XwikiDoc` from events.
pub fn collect_doc_from_events(events: impl IntoIterator<Item = OwnedEvent>) -> XwikiDoc {
    let mut block_stack: Vec<BlockFrame> = vec![BlockFrame::Document { blocks: Vec::new() }];
    let mut inline_ctx: Vec<InlineFrame> = Vec::new();
    for event in events {
        handle_event(event, &mut block_stack, &mut inline_ctx);
    }

    let blocks = match block_stack.pop() {
        Some(BlockFrame::Document { blocks }) => blocks,
        _ => Vec::new(),
    };

    XwikiDoc { blocks, span: Span::NONE }
}

enum BlockFrame {
    Document { blocks: Vec<Block> },
    Paragraph { inlines: Vec<Inline> },
    Heading { level: u8, inlines: Vec<Inline> },
    Blockquote { children: Vec<Block> },
    List { ordered: bool, items: Vec<Vec<Block>> },
    ListItem { blocks: Vec<Block> },
    Table { rows: Vec<TableRow> },
    TableRow { cells: Vec<TableCell> },
    TableCell { is_header: bool, inlines: Vec<Inline> },
}

enum InlineFrame {
    Bold { inlines: Vec<Inline> },
    Italic { inlines: Vec<Inline> },
    Underline { inlines: Vec<Inline> },
    Strikeout { inlines: Vec<Inline> },
    Superscript { inlines: Vec<Inline> },
    Subscript { inlines: Vec<Inline> },
    Link { url: String, inlines: Vec<Inline> },
}

fn inlines_from_frame(frame: &mut InlineFrame) -> &mut Vec<Inline> {
    match frame {
        InlineFrame::Bold { inlines }
        | InlineFrame::Italic { inlines }
        | InlineFrame::Underline { inlines }
        | InlineFrame::Strikeout { inlines }
        | InlineFrame::Superscript { inlines }
        | InlineFrame::Subscript { inlines }
        | InlineFrame::Link { inlines, .. } => inlines,
    }
}

fn push_inline(block_stack: &mut [BlockFrame], inline_ctx: &mut [InlineFrame], inline: Inline) {
    if let Some(frame) = inline_ctx.last_mut() {
        inlines_from_frame(frame).push(inline);
        return;
    }
    match block_stack.last_mut() {
        Some(BlockFrame::Paragraph { inlines })
        | Some(BlockFrame::Heading { inlines, .. })
        | Some(BlockFrame::TableCell { inlines, .. }) => inlines.push(inline),
        _ => {}
    }
}

fn push_block(block_stack: &mut [BlockFrame], block: Block) {
    match block_stack.last_mut() {
        Some(BlockFrame::Document { blocks }) => blocks.push(block),
        Some(BlockFrame::Blockquote { children }) => children.push(block),
        Some(BlockFrame::ListItem { blocks }) => blocks.push(block),
        _ => {}
    }
}

fn handle_event(
    event: OwnedEvent,
    block_stack: &mut Vec<BlockFrame>,
    inline_ctx: &mut Vec<InlineFrame>,
) {
    match event {
        Event::StartParagraph => {
            block_stack.push(BlockFrame::Paragraph { inlines: Vec::new() });
        }
        Event::EndParagraph => {
            if let Some(BlockFrame::Paragraph { inlines }) = block_stack.pop() {
                push_block(block_stack, Block::Paragraph { inlines, span: Span::NONE });
            }
        }
        Event::StartHeading { level } => {
            block_stack.push(BlockFrame::Heading { level, inlines: Vec::new() });
        }
        Event::EndHeading => {
            if let Some(BlockFrame::Heading { level, inlines }) = block_stack.pop() {
                push_block(
                    block_stack,
                    Block::Heading { level, inlines, span: Span::NONE },
                );
            }
        }
        Event::StartBlockquote => {
            block_stack.push(BlockFrame::Blockquote { children: Vec::new() });
        }
        Event::EndBlockquote => {
            if let Some(BlockFrame::Blockquote { children }) = block_stack.pop() {
                push_block(
                    block_stack,
                    Block::Blockquote { children, span: Span::NONE },
                );
            }
        }
        Event::StartList { ordered } => {
            block_stack.push(BlockFrame::List { ordered, items: Vec::new() });
        }
        Event::EndList => {
            if let Some(BlockFrame::List { ordered, items }) = block_stack.pop() {
                push_block(
                    block_stack,
                    Block::List { ordered, items, span: Span::NONE },
                );
            }
        }
        Event::StartListItem => {
            block_stack.push(BlockFrame::ListItem { blocks: Vec::new() });
        }
        Event::EndListItem => {
            if let Some(BlockFrame::ListItem { blocks }) = block_stack.pop()
                && let Some(BlockFrame::List { items, .. }) = block_stack.last_mut()
            {
                items.push(blocks);
            }
        }
        Event::CodeBlock { language, content } => {
            push_block(
                block_stack,
                Block::CodeBlock {
                    content: content.into_owned(),
                    language,
                    span: Span::NONE,
                },
            );
        }
        Event::HorizontalRule => {
            push_block(block_stack, Block::HorizontalRule { span: Span::NONE });
        }
        Event::StartTable => {
            block_stack.push(BlockFrame::Table { rows: Vec::new() });
        }
        Event::EndTable => {
            if let Some(BlockFrame::Table { rows }) = block_stack.pop() {
                push_block(block_stack, Block::Table { rows, span: Span::NONE });
            }
        }
        Event::StartTableRow => {
            block_stack.push(BlockFrame::TableRow { cells: Vec::new() });
        }
        Event::EndTableRow => {
            if let Some(BlockFrame::TableRow { cells }) = block_stack.pop()
                && let Some(BlockFrame::Table { rows }) = block_stack.last_mut()
            {
                rows.push(TableRow { cells, span: Span::NONE });
            }
        }
        Event::StartTableCell { is_header } => {
            block_stack.push(BlockFrame::TableCell { is_header, inlines: Vec::new() });
        }
        Event::EndTableCell => {
            if let Some(BlockFrame::TableCell { is_header, inlines }) = block_stack.pop()
                && let Some(BlockFrame::TableRow { cells }) = block_stack.last_mut()
            {
                cells.push(TableCell { is_header, inlines, span: Span::NONE });
            }
        }
        Event::MacroBlock { name, params, content } => {
            push_block(
                block_stack,
                Block::MacroBlock { name, params, content, span: Span::NONE },
            );
        }
        Event::MacroInline { name, params } => {
            push_block(
                block_stack,
                Block::MacroInline { name, params, span: Span::NONE },
            );
        }

        // ── Inline events ──────────────────────────────────────────────────
        Event::Text(cow) => {
            push_inline(
                block_stack,
                inline_ctx,
                Inline::Text(cow.into_owned(), Span::NONE),
            );
        }
        Event::SoftBreak => {
            push_inline(block_stack, inline_ctx, Inline::SoftBreak { span: Span::NONE });
        }
        Event::LineBreak => {
            push_inline(block_stack, inline_ctx, Inline::LineBreak { span: Span::NONE });
        }
        Event::InlineCode(cow) => {
            push_inline(
                block_stack,
                inline_ctx,
                Inline::Code(cow.into_owned(), Span::NONE),
            );
        }
        Event::StartBold => {
            inline_ctx.push(InlineFrame::Bold { inlines: Vec::new() });
        }
        Event::EndBold => {
            if let Some(InlineFrame::Bold { inlines }) = inline_ctx.pop() {
                push_inline(block_stack, inline_ctx, Inline::Bold(inlines, Span::NONE));
            }
        }
        Event::StartItalic => {
            inline_ctx.push(InlineFrame::Italic { inlines: Vec::new() });
        }
        Event::EndItalic => {
            if let Some(InlineFrame::Italic { inlines }) = inline_ctx.pop() {
                push_inline(block_stack, inline_ctx, Inline::Italic(inlines, Span::NONE));
            }
        }
        Event::StartUnderline => {
            inline_ctx.push(InlineFrame::Underline { inlines: Vec::new() });
        }
        Event::EndUnderline => {
            if let Some(InlineFrame::Underline { inlines }) = inline_ctx.pop() {
                push_inline(
                    block_stack,
                    inline_ctx,
                    Inline::Underline(inlines, Span::NONE),
                );
            }
        }
        Event::StartStrikeout => {
            inline_ctx.push(InlineFrame::Strikeout { inlines: Vec::new() });
        }
        Event::EndStrikeout => {
            if let Some(InlineFrame::Strikeout { inlines }) = inline_ctx.pop() {
                push_inline(
                    block_stack,
                    inline_ctx,
                    Inline::Strikeout(inlines, Span::NONE),
                );
            }
        }
        Event::StartSuperscript => {
            inline_ctx.push(InlineFrame::Superscript { inlines: Vec::new() });
        }
        Event::EndSuperscript => {
            if let Some(InlineFrame::Superscript { inlines }) = inline_ctx.pop() {
                push_inline(
                    block_stack,
                    inline_ctx,
                    Inline::Superscript(inlines, Span::NONE),
                );
            }
        }
        Event::StartSubscript => {
            inline_ctx.push(InlineFrame::Subscript { inlines: Vec::new() });
        }
        Event::EndSubscript => {
            if let Some(InlineFrame::Subscript { inlines }) = inline_ctx.pop() {
                push_inline(
                    block_stack,
                    inline_ctx,
                    Inline::Subscript(inlines, Span::NONE),
                );
            }
        }
        Event::StartLink { url } => {
            inline_ctx.push(InlineFrame::Link { url, inlines: Vec::new() });
        }
        Event::EndLink => {
            if let Some(InlineFrame::Link { url, inlines }) = inline_ctx.pop() {
                let label = crate::emit::collect_inline_text(&inlines);
                push_inline(
                    block_stack,
                    inline_ctx,
                    Inline::Link { url, label, span: Span::NONE },
                );
            }
        }
        Event::InlineImage { url, alt, params } => {
            push_inline(
                block_stack,
                inline_ctx,
                Inline::Image { url, alt, params, span: Span::NONE },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_events_heading() {
        let (doc, _) = crate::parse::parse("= Hello =");
        let evs: Vec<_> = events(&doc).collect();
        assert!(evs.iter().any(|e| matches!(e, Event::StartHeading { level: 1 })));
        assert!(evs.iter().any(|e| matches!(e, Event::EndHeading)));
    }

    #[test]
    fn test_events_paragraph() {
        let (doc, _) = crate::parse::parse("Hello world");
        let evs: Vec<_> = events(&doc).collect();
        assert!(evs.iter().any(|e| matches!(e, Event::StartParagraph)));
        assert!(evs.iter().any(|e| matches!(e, Event::EndParagraph)));
        assert!(evs.iter().any(|e| matches!(e, Event::Text(t) if t == "Hello world")));
    }

    #[test]
    fn test_events_bold() {
        let (doc, _) = crate::parse::parse("**bold**");
        let evs: Vec<_> = events(&doc).collect();
        assert!(evs.iter().any(|e| matches!(e, Event::StartBold)));
        assert!(evs.iter().any(|e| matches!(e, Event::EndBold)));
    }

    #[test]
    fn test_events_code_block() {
        let (doc, _) = crate::parse::parse("{{code language=\"rust\"}}\nfn main() {}\n{{/code}}");
        let evs: Vec<_> = events(&doc).collect();
        assert!(evs.iter().any(|e| matches!(e, Event::CodeBlock { .. })));
    }

    #[test]
    fn test_events_table() {
        let (doc, _) = crate::parse::parse("|=Name|=Age|\n|Alice|30|");
        let evs: Vec<_> = events(&doc).collect();
        assert!(evs.iter().any(|e| matches!(e, Event::StartTable)));
        assert!(evs.iter().any(|e| matches!(e, Event::StartTableCell { is_header: true })));
        assert!(evs.iter().any(|e| matches!(e, Event::EndTable)));
    }

    #[test]
    fn test_events_list() {
        let (doc, _) = crate::parse::parse("* Item 1\n* Item 2");
        let evs: Vec<_> = events(&doc).collect();
        assert!(evs.iter().any(|e| matches!(e, Event::StartList { ordered: false })));
        assert_eq!(
            evs.iter().filter(|e| matches!(e, Event::StartListItem)).count(),
            2
        );
        assert!(evs.iter().any(|e| matches!(e, Event::EndList)));
    }

    #[test]
    fn test_events_horizontal_rule() {
        let (doc, _) = crate::parse::parse("----");
        let evs: Vec<_> = events(&doc).collect();
        assert!(evs.iter().any(|e| matches!(e, Event::HorizontalRule)));
    }
}
