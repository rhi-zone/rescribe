//! Streaming event iterator over a parsed `ZimwikiDoc`.

use std::borrow::Cow;
use std::collections::VecDeque;

use crate::ast::*;

/// A streaming event from a ZimWiki document.
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
    StartListItem { checked: Option<bool> },
    EndListItem,
    /// Leaf: a verbatim/code block.
    CodeBlock { content: Cow<'a, str> },
    /// Leaf: a horizontal rule (`----`).
    HorizontalRule,
    StartTable,
    EndTable,
    StartTableRow,
    EndTableRow,
    StartTableCell,
    EndTableCell,

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
    StartStrikethrough,
    EndStrikethrough,
    StartSuperscript,
    EndSuperscript,
    StartSubscript,
    EndSubscript,
    /// Leaf: inline verbatim/code span.
    InlineCode(Cow<'a, str>),
    StartLink { url: String },
    EndLink,
    /// Leaf: inline image.
    InlineImage { url: String },
}

/// Backwards-compatible alias for batch mode (all text is owned).
pub type OwnedEvent = Event<'static>;

impl<'a> Event<'a> {
    /// Convert to an owned event.
    pub fn into_owned(self) -> OwnedEvent {
        match self {
            Event::Text(cow) => Event::Text(Cow::Owned(cow.into_owned())),
            Event::InlineCode(cow) => Event::InlineCode(Cow::Owned(cow.into_owned())),
            Event::CodeBlock { content } => Event::CodeBlock {
                content: Cow::Owned(content.into_owned()),
            },
            // All other variants contain only String/'static fields.
            other => unsafe { std::mem::transmute::<Event<'_>, OwnedEvent>(other) },
        }
    }
}

// ── Pull iterator ─────────────────────────────────────────────────────────────

/// Pull-style event iterator over a ZimWiki document.
pub struct EventIter {
    events: VecDeque<OwnedEvent>,
}

impl EventIter {
    pub fn new(input: &str) -> Self {
        let (doc, _) = crate::parse::parse(input);
        let mut events = Vec::new();
        emit_blocks(&doc.blocks, &mut events);
        EventIter {
            events: events.into_iter().map(|e| e.into_owned()).collect(),
        }
    }
}

impl Iterator for EventIter {
    type Item = OwnedEvent;

    fn next(&mut self) -> Option<Self::Item> {
        self.events.pop_front()
    }
}

// ── Tree builder (inverse) ────────────────────────────────────────────────────

/// Collect a complete `ZimwikiDoc` from a sequence of events.
pub fn collect_doc_from_events(events: impl IntoIterator<Item = OwnedEvent>) -> ZimwikiDoc {
    let mut block_stack: Vec<BlockFrame> = vec![BlockFrame::Document { blocks: Vec::new() }];
    let mut inline_ctx: Vec<InlineFrame> = Vec::new();

    for event in events {
        handle_event(event, &mut block_stack, &mut inline_ctx);
    }

    let blocks = match block_stack.pop() {
        Some(BlockFrame::Document { blocks }) => blocks,
        _ => Vec::new(),
    };

    ZimwikiDoc { blocks, span: Span::NONE }
}

enum BlockFrame {
    Document { blocks: Vec<Block> },
    Paragraph { inlines: Vec<Inline> },
    Heading { level: u8, inlines: Vec<Inline> },
    Blockquote { children: Vec<Block> },
    List { ordered: bool, items: Vec<ListItem> },
    ListItem { checked: Option<bool>, children: Vec<Block> },
    Table { rows: Vec<TableRow> },
    TableRow { cells: Vec<Vec<Inline>> },
    TableCell { inlines: Vec<Inline> },
}

enum InlineFrame {
    Bold { inlines: Vec<Inline> },
    Italic { inlines: Vec<Inline> },
    Underline { inlines: Vec<Inline> },
    Strikethrough { inlines: Vec<Inline> },
    Superscript { inlines: Vec<Inline> },
    Subscript { inlines: Vec<Inline> },
    Link { url: String, inlines: Vec<Inline> },
}

fn inlines_from_frame(frame: &mut InlineFrame) -> &mut Vec<Inline> {
    match frame {
        InlineFrame::Bold { inlines }
        | InlineFrame::Italic { inlines }
        | InlineFrame::Underline { inlines }
        | InlineFrame::Strikethrough { inlines }
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
        | Some(BlockFrame::TableCell { inlines }) => inlines.push(inline),
        _ => {}
    }
}

fn push_block(block_stack: &mut [BlockFrame], block: Block) {
    match block_stack.last_mut() {
        Some(BlockFrame::Document { blocks })
        | Some(BlockFrame::Blockquote { children: blocks })
        | Some(BlockFrame::ListItem { children: blocks, .. }) => blocks.push(block),
        _ => {}
    }
}

fn handle_event(
    event: OwnedEvent,
    block_stack: &mut Vec<BlockFrame>,
    inline_ctx: &mut Vec<InlineFrame>,
) {
    match event {
        // ── Block open ────────────────────────────────────────────────────────
        Event::StartParagraph => {
            block_stack.push(BlockFrame::Paragraph { inlines: Vec::new() });
        }
        Event::StartHeading { level } => {
            block_stack.push(BlockFrame::Heading { level, inlines: Vec::new() });
        }
        Event::StartBlockquote => {
            block_stack.push(BlockFrame::Blockquote { children: Vec::new() });
        }
        Event::StartList { ordered } => {
            block_stack.push(BlockFrame::List { ordered, items: Vec::new() });
        }
        Event::StartListItem { checked } => {
            block_stack.push(BlockFrame::ListItem { checked, children: Vec::new() });
        }
        Event::StartTable => {
            block_stack.push(BlockFrame::Table { rows: Vec::new() });
        }
        Event::StartTableRow => {
            block_stack.push(BlockFrame::TableRow { cells: Vec::new() });
        }
        Event::StartTableCell => {
            block_stack.push(BlockFrame::TableCell { inlines: Vec::new() });
        }

        // ── Block close ───────────────────────────────────────────────────────
        Event::EndParagraph => {
            if let Some(BlockFrame::Paragraph { inlines }) = block_stack.pop() {
                push_block(block_stack, Block::Paragraph { inlines, span: Span::NONE });
            }
        }
        Event::EndHeading => {
            if let Some(BlockFrame::Heading { level, inlines }) = block_stack.pop() {
                push_block(block_stack, Block::Heading { level, inlines, span: Span::NONE });
            }
        }
        Event::EndBlockquote => {
            if let Some(BlockFrame::Blockquote { children }) = block_stack.pop() {
                push_block(block_stack, Block::Blockquote { children, span: Span::NONE });
            }
        }
        Event::EndList => {
            if let Some(BlockFrame::List { ordered, items }) = block_stack.pop() {
                push_block(block_stack, Block::List { ordered, items, span: Span::NONE });
            }
        }
        Event::EndListItem => {
            if let Some(BlockFrame::ListItem { checked, children }) = block_stack.pop()
                && let Some(BlockFrame::List { items, .. }) = block_stack.last_mut()
            {
                items.push(ListItem { checked, children, span: Span::NONE });
            }
        }
        Event::EndTable => {
            if let Some(BlockFrame::Table { rows }) = block_stack.pop() {
                push_block(block_stack, Block::Table { rows, span: Span::NONE });
            }
        }
        Event::EndTableRow => {
            if let Some(BlockFrame::TableRow { cells }) = block_stack.pop()
                && let Some(BlockFrame::Table { rows }) = block_stack.last_mut()
            {
                rows.push(TableRow { cells, span: Span::NONE });
            }
        }
        Event::EndTableCell => {
            if let Some(BlockFrame::TableCell { inlines }) = block_stack.pop()
                && let Some(BlockFrame::TableRow { cells, .. }) = block_stack.last_mut()
            {
                cells.push(inlines);
            }
        }

        // ── Leaf block events ─────────────────────────────────────────────────
        Event::CodeBlock { content } => {
            push_block(
                block_stack,
                Block::CodeBlock { content: content.into_owned(), span: Span::NONE },
            );
        }
        Event::HorizontalRule => {
            push_block(block_stack, Block::HorizontalRule { span: Span::NONE });
        }

        // ── Inline events ─────────────────────────────────────────────────────
        Event::Text(cow) => {
            push_inline(block_stack, inline_ctx, Inline::Text(cow.into_owned(), Span::NONE));
        }
        Event::SoftBreak => {
            push_inline(block_stack, inline_ctx, Inline::SoftBreak { span: Span::NONE });
        }
        Event::LineBreak => {
            push_inline(block_stack, inline_ctx, Inline::LineBreak { span: Span::NONE });
        }
        Event::InlineCode(cow) => {
            push_inline(block_stack, inline_ctx, Inline::Code(cow.into_owned(), Span::NONE));
        }
        Event::InlineImage { url } => {
            push_inline(block_stack, inline_ctx, Inline::Image { url, span: Span::NONE });
        }

        // ── Inline container open ─────────────────────────────────────────────
        Event::StartBold => {
            inline_ctx.push(InlineFrame::Bold { inlines: Vec::new() });
        }
        Event::StartItalic => {
            inline_ctx.push(InlineFrame::Italic { inlines: Vec::new() });
        }
        Event::StartUnderline => {
            inline_ctx.push(InlineFrame::Underline { inlines: Vec::new() });
        }
        Event::StartStrikethrough => {
            inline_ctx.push(InlineFrame::Strikethrough { inlines: Vec::new() });
        }
        Event::StartSuperscript => {
            inline_ctx.push(InlineFrame::Superscript { inlines: Vec::new() });
        }
        Event::StartSubscript => {
            inline_ctx.push(InlineFrame::Subscript { inlines: Vec::new() });
        }
        Event::StartLink { url } => {
            inline_ctx.push(InlineFrame::Link { url, inlines: Vec::new() });
        }

        // ── Inline container close ────────────────────────────────────────────
        Event::EndBold => {
            if let Some(InlineFrame::Bold { inlines }) = inline_ctx.pop() {
                push_inline(block_stack, inline_ctx, Inline::Bold(inlines, Span::NONE));
            }
        }
        Event::EndItalic => {
            if let Some(InlineFrame::Italic { inlines }) = inline_ctx.pop() {
                push_inline(block_stack, inline_ctx, Inline::Italic(inlines, Span::NONE));
            }
        }
        Event::EndUnderline => {
            if let Some(InlineFrame::Underline { inlines }) = inline_ctx.pop() {
                push_inline(block_stack, inline_ctx, Inline::Underline(inlines, Span::NONE));
            }
        }
        Event::EndStrikethrough => {
            if let Some(InlineFrame::Strikethrough { inlines }) = inline_ctx.pop() {
                push_inline(
                    block_stack,
                    inline_ctx,
                    Inline::Strikethrough(inlines, Span::NONE),
                );
            }
        }
        Event::EndSuperscript => {
            if let Some(InlineFrame::Superscript { inlines }) = inline_ctx.pop() {
                push_inline(block_stack, inline_ctx, Inline::Superscript(inlines, Span::NONE));
            }
        }
        Event::EndSubscript => {
            if let Some(InlineFrame::Subscript { inlines }) = inline_ctx.pop() {
                push_inline(block_stack, inline_ctx, Inline::Subscript(inlines, Span::NONE));
            }
        }
        Event::EndLink => {
            if let Some(InlineFrame::Link { url, inlines }) = inline_ctx.pop() {
                push_inline(
                    block_stack,
                    inline_ctx,
                    Inline::Link { url, children: inlines, span: Span::NONE },
                );
            }
        }
    }
}

// ── AST → Event emission ─────────────────────────────────────────────────────

fn emit_blocks<'a>(blocks: &[Block], out: &mut Vec<Event<'a>>) {
    for block in blocks {
        emit_block(block, out);
    }
}

fn emit_block<'a>(block: &Block, out: &mut Vec<Event<'a>>) {
    match block {
        Block::Paragraph { inlines, .. } => {
            out.push(Event::StartParagraph);
            emit_inlines(inlines, out);
            out.push(Event::EndParagraph);
        }
        Block::Heading { level, inlines, .. } => {
            out.push(Event::StartHeading { level: *level });
            emit_inlines(inlines, out);
            out.push(Event::EndHeading);
        }
        Block::CodeBlock { content, .. } => {
            out.push(Event::CodeBlock {
                content: Cow::Owned(content.clone()),
            });
        }
        Block::Blockquote { children, .. } => {
            out.push(Event::StartBlockquote);
            emit_blocks(children, out);
            out.push(Event::EndBlockquote);
        }
        Block::List { ordered, items, .. } => {
            out.push(Event::StartList { ordered: *ordered });
            for item in items {
                out.push(Event::StartListItem { checked: item.checked });
                emit_blocks(&item.children, out);
                out.push(Event::EndListItem);
            }
            out.push(Event::EndList);
        }
        Block::Table { rows, .. } => {
            out.push(Event::StartTable);
            for row in rows {
                out.push(Event::StartTableRow);
                for cell in &row.cells {
                    out.push(Event::StartTableCell);
                    emit_inlines(cell, out);
                    out.push(Event::EndTableCell);
                }
                out.push(Event::EndTableRow);
            }
            out.push(Event::EndTable);
        }
        Block::HorizontalRule { .. } => {
            out.push(Event::HorizontalRule);
        }
    }
}

fn emit_inlines<'a>(inlines: &[Inline], out: &mut Vec<Event<'a>>) {
    for inline in inlines {
        emit_inline(inline, out);
    }
}

fn emit_inline<'a>(inline: &Inline, out: &mut Vec<Event<'a>>) {
    match inline {
        Inline::Text(s, _) => out.push(Event::Text(Cow::Owned(s.clone()))),
        Inline::Bold(children, _) => {
            out.push(Event::StartBold);
            emit_inlines(children, out);
            out.push(Event::EndBold);
        }
        Inline::Italic(children, _) => {
            out.push(Event::StartItalic);
            emit_inlines(children, out);
            out.push(Event::EndItalic);
        }
        Inline::Underline(children, _) => {
            out.push(Event::StartUnderline);
            emit_inlines(children, out);
            out.push(Event::EndUnderline);
        }
        Inline::Strikethrough(children, _) => {
            out.push(Event::StartStrikethrough);
            emit_inlines(children, out);
            out.push(Event::EndStrikethrough);
        }
        Inline::Subscript(children, _) => {
            out.push(Event::StartSubscript);
            emit_inlines(children, out);
            out.push(Event::EndSubscript);
        }
        Inline::Superscript(children, _) => {
            out.push(Event::StartSuperscript);
            emit_inlines(children, out);
            out.push(Event::EndSuperscript);
        }
        Inline::Code(s, _) => out.push(Event::InlineCode(Cow::Owned(s.clone()))),
        Inline::Link { url, children, .. } => {
            out.push(Event::StartLink { url: url.clone() });
            emit_inlines(children, out);
            out.push(Event::EndLink);
        }
        Inline::Image { url, .. } => out.push(Event::InlineImage { url: url.clone() }),
        Inline::LineBreak { .. } => out.push(Event::LineBreak),
        Inline::SoftBreak { .. } => out.push(Event::SoftBreak),
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Parse `input` and return an event iterator.
pub fn events(input: &str) -> EventIter {
    EventIter::new(input)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_events_heading() {
        let evs: Vec<_> = events("====== Hello ======").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartHeading { level: 1 })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndHeading)));
    }

    #[test]
    fn test_events_paragraph() {
        let evs: Vec<_> = events("Hello world").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartParagraph)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndParagraph)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::Text(t) if t == "Hello world")));
    }

    #[test]
    fn test_events_bold_italic() {
        let evs: Vec<_> = events("**bold** //italic//").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartBold)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndBold)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartItalic)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndItalic)));
    }

    #[test]
    fn test_events_link() {
        let evs: Vec<_> = events("[[https://example.com|click here]]").collect();
        assert!(evs
            .iter()
            .any(|e| matches!(e, OwnedEvent::StartLink { url } if url == "https://example.com")));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndLink)));
    }

    #[test]
    fn test_events_blockquote() {
        let evs: Vec<_> = events("> quoted text").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartBlockquote)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndBlockquote)));
    }

    #[test]
    fn test_events_roundtrip() {
        let input = "====== Title ======\n\nA paragraph with **bold** text.\n\n* item one\n* item two\n";
        let evs: Vec<_> = events(input).collect();
        let doc = collect_doc_from_events(evs);
        assert_eq!(doc.blocks.len(), 3);
    }
}
