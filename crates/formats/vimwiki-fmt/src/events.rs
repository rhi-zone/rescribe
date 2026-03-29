//! Streaming event iterator over a parsed `VimwikiDoc`.

use std::borrow::Cow;

use crate::ast::*;

/// A streaming event from a VimWiki document.
#[derive(Debug, PartialEq)]
pub enum Event<'a> {
    // -- Block events ---------------------------------------------------------
    StartParagraph,
    EndParagraph,
    StartHeading { level: usize },
    EndHeading,
    StartBlockquote,
    EndBlockquote,
    StartList { ordered: bool },
    EndList,
    StartListItem { checked: Option<bool> },
    EndListItem,
    CodeBlock {
        language: Option<String>,
        content: Cow<'a, str>,
    },
    HorizontalRule,
    StartTable,
    EndTable,
    StartTableRow,
    EndTableRow,
    StartTableCell,
    EndTableCell,
    StartDefinitionList,
    EndDefinitionList,
    StartDefinitionTerm,
    EndDefinitionTerm,
    StartDefinitionDesc,
    EndDefinitionDesc,

    // -- Inline events --------------------------------------------------------
    Text(Cow<'a, str>),
    StartBold,
    EndBold,
    StartItalic,
    EndItalic,
    StartStrikethrough,
    EndStrikethrough,
    StartSuperscript,
    EndSuperscript,
    StartSubscript,
    EndSubscript,
    InlineCode(Cow<'a, str>),
    StartLink { url: String },
    EndLink,
    InlineImage {
        url: String,
        alt: Option<String>,
        style: Option<String>,
    },
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
            Event::StartParagraph => Event::StartParagraph,
            Event::EndParagraph => Event::EndParagraph,
            Event::StartHeading { level } => Event::StartHeading { level },
            Event::EndHeading => Event::EndHeading,
            Event::StartBlockquote => Event::StartBlockquote,
            Event::EndBlockquote => Event::EndBlockquote,
            Event::StartList { ordered } => Event::StartList { ordered },
            Event::EndList => Event::EndList,
            Event::StartListItem { checked } => Event::StartListItem { checked },
            Event::EndListItem => Event::EndListItem,
            Event::HorizontalRule => Event::HorizontalRule,
            Event::StartTable => Event::StartTable,
            Event::EndTable => Event::EndTable,
            Event::StartTableRow => Event::StartTableRow,
            Event::EndTableRow => Event::EndTableRow,
            Event::StartTableCell => Event::StartTableCell,
            Event::EndTableCell => Event::EndTableCell,
            Event::StartDefinitionList => Event::StartDefinitionList,
            Event::EndDefinitionList => Event::EndDefinitionList,
            Event::StartDefinitionTerm => Event::StartDefinitionTerm,
            Event::EndDefinitionTerm => Event::EndDefinitionTerm,
            Event::StartDefinitionDesc => Event::StartDefinitionDesc,
            Event::EndDefinitionDesc => Event::EndDefinitionDesc,
            Event::StartBold => Event::StartBold,
            Event::EndBold => Event::EndBold,
            Event::StartItalic => Event::StartItalic,
            Event::EndItalic => Event::EndItalic,
            Event::StartStrikethrough => Event::StartStrikethrough,
            Event::EndStrikethrough => Event::EndStrikethrough,
            Event::StartSuperscript => Event::StartSuperscript,
            Event::EndSuperscript => Event::EndSuperscript,
            Event::StartSubscript => Event::StartSubscript,
            Event::EndSubscript => Event::EndSubscript,
            Event::StartLink { url } => Event::StartLink { url },
            Event::EndLink => Event::EndLink,
            Event::InlineImage { url, alt, style } => Event::InlineImage { url, alt, style },
        }
    }
}

// -- True pull iterator -------------------------------------------------------

/// Pull-based event iterator over a VimWiki document.
///
/// Created by [`events()`]. Implements `Iterator<Item = Event<'_>>`.
pub struct EventIter<'a> {
    #[allow(dead_code)]
    doc: VimwikiDoc,
    queue: Vec<Event<'a>>,
    pos: usize,
}

impl<'a> EventIter<'a> {
    /// Create a new event iterator by parsing `input`.
    pub fn new(input: &'a str) -> Self {
        let (doc, _) = crate::parse::parse(input);
        let mut queue = Vec::new();
        emit_doc_events(&doc, &mut queue);
        EventIter { doc, queue, pos: 0 }
    }
}

impl<'a> Iterator for EventIter<'a> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos < self.queue.len() {
            let event = std::mem::replace(
                &mut self.queue[self.pos],
                Event::StartParagraph, // placeholder
            );
            self.pos += 1;
            Some(event)
        } else {
            None
        }
    }
}

fn emit_doc_events(doc: &VimwikiDoc, out: &mut Vec<Event<'_>>) {
    for block in &doc.blocks {
        emit_block_events(block, out);
    }
}

fn emit_block_events(block: &Block, out: &mut Vec<Event<'_>>) {
    match block {
        Block::Paragraph { inlines, .. } => {
            out.push(Event::StartParagraph);
            emit_inline_events(inlines, out);
            out.push(Event::EndParagraph);
        }
        Block::Heading { level, inlines, .. } => {
            out.push(Event::StartHeading { level: *level });
            emit_inline_events(inlines, out);
            out.push(Event::EndHeading);
        }
        Block::CodeBlock { language, content, .. } => {
            out.push(Event::CodeBlock {
                language: language.clone(),
                content: Cow::Owned(content.clone()),
            });
        }
        Block::Blockquote { inlines, .. } => {
            out.push(Event::StartBlockquote);
            out.push(Event::StartParagraph);
            emit_inline_events(inlines, out);
            out.push(Event::EndParagraph);
            out.push(Event::EndBlockquote);
        }
        Block::List { ordered, items, .. } => {
            out.push(Event::StartList { ordered: *ordered });
            for item in items {
                out.push(Event::StartListItem { checked: item.checked });
                emit_inline_events(&item.inlines, out);
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
                    emit_inline_events(cell, out);
                    out.push(Event::EndTableCell);
                }
                out.push(Event::EndTableRow);
            }
            out.push(Event::EndTable);
        }
        Block::HorizontalRule { .. } => {
            out.push(Event::HorizontalRule);
        }
        Block::DefinitionList { items, .. } => {
            out.push(Event::StartDefinitionList);
            for item in items {
                out.push(Event::StartDefinitionTerm);
                emit_inline_events(&item.term, out);
                out.push(Event::EndDefinitionTerm);
                out.push(Event::StartDefinitionDesc);
                emit_inline_events(&item.desc, out);
                out.push(Event::EndDefinitionDesc);
            }
            out.push(Event::EndDefinitionList);
        }
    }
}

fn emit_inline_events<'a>(inlines: &[Inline], out: &mut Vec<Event<'a>>) {
    for inline in inlines {
        match inline {
            Inline::Text(s, _) => {
                out.push(Event::Text(Cow::Owned(s.clone())));
            }
            Inline::Bold(children, _) => {
                out.push(Event::StartBold);
                emit_inline_events(children, out);
                out.push(Event::EndBold);
            }
            Inline::Italic(children, _) => {
                out.push(Event::StartItalic);
                emit_inline_events(children, out);
                out.push(Event::EndItalic);
            }
            Inline::Strikethrough(children, _) => {
                out.push(Event::StartStrikethrough);
                emit_inline_events(children, out);
                out.push(Event::EndStrikethrough);
            }
            Inline::Superscript(children, _) => {
                out.push(Event::StartSuperscript);
                emit_inline_events(children, out);
                out.push(Event::EndSuperscript);
            }
            Inline::Subscript(children, _) => {
                out.push(Event::StartSubscript);
                emit_inline_events(children, out);
                out.push(Event::EndSubscript);
            }
            Inline::Code(s, _) => {
                out.push(Event::InlineCode(Cow::Owned(s.clone())));
            }
            Inline::Link { url, label, .. } => {
                out.push(Event::StartLink { url: url.clone() });
                out.push(Event::Text(Cow::Owned(label.clone())));
                out.push(Event::EndLink);
            }
            Inline::Image { url, alt, style, .. } => {
                out.push(Event::InlineImage {
                    url: url.clone(),
                    alt: alt.clone(),
                    style: style.clone(),
                });
            }
        }
    }
}

/// Parse `input` and return a streaming iterator of events.
pub fn events(input: &str) -> EventIter<'_> {
    EventIter::new(input)
}

// -- Tree builder (inverse of emit) -------------------------------------------

/// Collect a complete `VimwikiDoc` from events.
pub fn collect_doc_from_events(events_list: Vec<OwnedEvent>) -> VimwikiDoc {
    let mut block_stack: Vec<BlockFrame> = vec![BlockFrame::Document { blocks: Vec::new() }];
    let mut inline_ctx: Vec<InlineFrame> = Vec::new();

    for event in events_list {
        handle_event(event, &mut block_stack, &mut inline_ctx);
    }

    let blocks = match block_stack.pop() {
        Some(BlockFrame::Document { blocks }) => blocks,
        _ => Vec::new(),
    };

    VimwikiDoc { blocks, span: Span::NONE }
}

enum BlockFrame {
    Document { blocks: Vec<Block> },
    Paragraph { inlines: Vec<Inline> },
    Heading { level: usize, inlines: Vec<Inline> },
    Blockquote { blocks: Vec<Block> },
    List { ordered: bool, items: Vec<ListItem> },
    ListItem { checked: Option<bool>, inlines: Vec<Inline> },
    Table { rows: Vec<TableRow> },
    TableRow { cells: Vec<Vec<Inline>> },
    TableCell { inlines: Vec<Inline> },
    DefinitionList { items: Vec<DefinitionItem> },
    DefinitionTerm { inlines: Vec<Inline> },
    DefinitionDesc { inlines: Vec<Inline> },
}

enum InlineFrame {
    Bold { inlines: Vec<Inline> },
    Italic { inlines: Vec<Inline> },
    Strikethrough { inlines: Vec<Inline> },
    Superscript { inlines: Vec<Inline> },
    Subscript { inlines: Vec<Inline> },
    Link { url: String, inlines: Vec<Inline> },
}

fn inlines_from_frame(frame: &mut InlineFrame) -> &mut Vec<Inline> {
    match frame {
        InlineFrame::Bold { inlines } => inlines,
        InlineFrame::Italic { inlines } => inlines,
        InlineFrame::Strikethrough { inlines } => inlines,
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
        Some(BlockFrame::Heading { inlines, .. }) => inlines.push(inline),
        Some(BlockFrame::TableCell { inlines }) => inlines.push(inline),
        Some(BlockFrame::DefinitionTerm { inlines }) => inlines.push(inline),
        Some(BlockFrame::DefinitionDesc { inlines }) => inlines.push(inline),
        Some(BlockFrame::ListItem { inlines, .. }) => inlines.push(inline),
        _ => {}
    }
}

fn push_block(block_stack: &mut [BlockFrame], block: Block) {
    match block_stack.last_mut() {
        Some(BlockFrame::Document { blocks }) => blocks.push(block),
        Some(BlockFrame::Blockquote { blocks }) => blocks.push(block),
        _ => {}
    }
}

fn handle_event(
    event: Event<'_>,
    block_stack: &mut Vec<BlockFrame>,
    inline_ctx: &mut Vec<InlineFrame>,
) {
    match event {
        // -- Block start events -----------------------------------------------
        Event::StartParagraph => {
            block_stack.push(BlockFrame::Paragraph { inlines: Vec::new() });
        }
        Event::StartHeading { level } => {
            block_stack.push(BlockFrame::Heading { level, inlines: Vec::new() });
        }
        Event::StartBlockquote => {
            block_stack.push(BlockFrame::Blockquote { blocks: Vec::new() });
        }
        Event::StartList { ordered } => {
            block_stack.push(BlockFrame::List { ordered, items: Vec::new() });
        }
        Event::StartListItem { checked } => {
            block_stack.push(BlockFrame::ListItem { checked, inlines: Vec::new() });
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
        Event::StartDefinitionList => {
            block_stack.push(BlockFrame::DefinitionList { items: Vec::new() });
        }
        Event::StartDefinitionTerm => {
            block_stack.push(BlockFrame::DefinitionTerm { inlines: Vec::new() });
        }
        Event::StartDefinitionDesc => {
            block_stack.push(BlockFrame::DefinitionDesc { inlines: Vec::new() });
        }

        // -- Block end events -------------------------------------------------
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
            if let Some(BlockFrame::Blockquote { blocks }) = block_stack.pop() {
                // Flatten: blockquote in vimwiki is inline-only.
                // Merge all paragraph inlines into a single blockquote.
                let mut all_inlines = Vec::new();
                for block in blocks {
                    if let Block::Paragraph { inlines, .. } = block {
                        all_inlines.extend(inlines);
                    }
                }
                push_block(
                    block_stack,
                    Block::Blockquote { inlines: all_inlines, span: Span::NONE },
                );
            }
        }
        Event::EndList => {
            if let Some(BlockFrame::List { ordered, items }) = block_stack.pop() {
                push_block(block_stack, Block::List { ordered, items, span: Span::NONE });
            }
        }
        Event::EndListItem => {
            if let Some(BlockFrame::ListItem { checked, inlines }) = block_stack.pop()
                && let Some(BlockFrame::List { items, .. }) = block_stack.last_mut()
            {
                items.push(ListItem { checked, inlines, span: Span::NONE });
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
        Event::EndDefinitionList => {
            if let Some(BlockFrame::DefinitionList { items }) = block_stack.pop() {
                push_block(
                    block_stack,
                    Block::DefinitionList { items, span: Span::NONE },
                );
            }
        }
        Event::EndDefinitionTerm => {
            if let Some(BlockFrame::DefinitionTerm { inlines }) = block_stack.pop()
                && let Some(BlockFrame::DefinitionList { items }) = block_stack.last_mut()
            {
                items.push(DefinitionItem {
                    term: inlines,
                    desc: Vec::new(),
                    span: Span::NONE,
                });
            }
        }
        Event::EndDefinitionDesc => {
            if let Some(BlockFrame::DefinitionDesc { inlines }) = block_stack.pop()
                && let Some(BlockFrame::DefinitionList { items }) = block_stack.last_mut()
                && let Some(last) = items.last_mut()
            {
                last.desc = inlines;
            }
        }

        // -- Leaf block events ------------------------------------------------
        Event::CodeBlock { language, content } => {
            push_block(
                block_stack,
                Block::CodeBlock {
                    language,
                    content: content.into_owned(),
                    span: Span::NONE,
                },
            );
        }
        Event::HorizontalRule => {
            push_block(block_stack, Block::HorizontalRule { span: Span::NONE });
        }

        // -- Inline events ----------------------------------------------------
        Event::Text(cow) => {
            push_inline(
                block_stack,
                inline_ctx,
                Inline::Text(cow.into_owned(), Span::NONE),
            );
        }
        Event::InlineCode(cow) => {
            push_inline(
                block_stack,
                inline_ctx,
                Inline::Code(cow.into_owned(), Span::NONE),
            );
        }
        Event::InlineImage { url, alt, style } => {
            push_inline(
                block_stack,
                inline_ctx,
                Inline::Image { url, alt, style, span: Span::NONE },
            );
        }

        // -- Inline container start events ------------------------------------
        Event::StartBold => {
            inline_ctx.push(InlineFrame::Bold { inlines: Vec::new() });
        }
        Event::StartItalic => {
            inline_ctx.push(InlineFrame::Italic { inlines: Vec::new() });
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

        // -- Inline container end events --------------------------------------
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
                push_inline(
                    block_stack,
                    inline_ctx,
                    Inline::Superscript(inlines, Span::NONE),
                );
            }
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
        Event::EndLink => {
            if let Some(InlineFrame::Link { url, inlines }) = inline_ctx.pop() {
                // Extract label text from inlines
                let label = crate::emit::collect_inline_text(&inlines);
                push_inline(
                    block_stack,
                    inline_ctx,
                    Inline::Link { url, label, span: Span::NONE },
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_events_heading() {
        let evs: Vec<_> = events("= Hello =").collect();
        assert!(evs.iter().any(|e| matches!(e, Event::StartHeading { level: 1 })));
        assert!(evs.iter().any(|e| matches!(e, Event::EndHeading)));
    }

    #[test]
    fn test_events_paragraph() {
        let evs: Vec<_> = events("Hello world").collect();
        assert!(evs.iter().any(|e| matches!(e, Event::StartParagraph)));
        assert!(evs.iter().any(|e| matches!(e, Event::EndParagraph)));
        assert!(evs.iter().any(|e| matches!(e, Event::Text(t) if t == "Hello world")));
    }

    #[test]
    fn test_events_code_block() {
        let evs: Vec<_> = events("{{{\nfn main() {}\n}}}").collect();
        assert!(evs.iter().any(|e| matches!(e, Event::CodeBlock { .. })));
    }

    #[test]
    fn test_events_list() {
        let evs: Vec<_> = events("* item 1\n* item 2").collect();
        assert!(evs.iter().any(|e| matches!(e, Event::StartList { ordered: false })));
        assert_eq!(
            evs.iter()
                .filter(|e| matches!(e, Event::StartListItem { .. }))
                .count(),
            2
        );
        assert!(evs.iter().any(|e| matches!(e, Event::EndList)));
    }

    #[test]
    fn test_events_table() {
        let evs: Vec<_> = events("| Name | Age |\n| Alice | 30 |").collect();
        assert!(evs.iter().any(|e| matches!(e, Event::StartTable)));
        assert!(evs.iter().any(|e| matches!(e, Event::EndTable)));
    }

    #[test]
    fn test_events_horizontal_rule() {
        let evs: Vec<_> = events("----").collect();
        assert!(evs.iter().any(|e| matches!(e, Event::HorizontalRule)));
    }

    #[test]
    fn test_events_bold_italic() {
        let evs: Vec<_> = events("*bold* _italic_").collect();
        assert!(evs.iter().any(|e| matches!(e, Event::StartBold)));
        assert!(evs.iter().any(|e| matches!(e, Event::EndBold)));
        assert!(evs.iter().any(|e| matches!(e, Event::StartItalic)));
        assert!(evs.iter().any(|e| matches!(e, Event::EndItalic)));
    }

    #[test]
    fn test_events_link() {
        let evs: Vec<_> = events("[[https://example.com|click here]]").collect();
        assert!(evs
            .iter()
            .any(|e| matches!(e, Event::StartLink { url } if url == "https://example.com")));
        assert!(evs.iter().any(|e| matches!(e, Event::EndLink)));
    }

    #[test]
    fn test_events_inline_code() {
        let evs: Vec<_> = events("`verbatim`").collect();
        assert!(evs
            .iter()
            .any(|e| matches!(e, Event::InlineCode(s) if s == "verbatim")));
    }

    #[test]
    fn test_events_definition_list() {
        let evs: Vec<_> = events("; Term\n: Definition").collect();
        assert!(evs.iter().any(|e| matches!(e, Event::StartDefinitionList)));
        assert!(evs.iter().any(|e| matches!(e, Event::StartDefinitionTerm)));
        assert!(evs.iter().any(|e| matches!(e, Event::StartDefinitionDesc)));
        assert!(evs.iter().any(|e| matches!(e, Event::EndDefinitionList)));
    }

    #[test]
    fn test_events_superscript() {
        let evs: Vec<_> = events("^super^").collect();
        assert!(evs.iter().any(|e| matches!(e, Event::StartSuperscript)));
        assert!(evs.iter().any(|e| matches!(e, Event::EndSuperscript)));
    }

    #[test]
    fn test_events_subscript() {
        let evs: Vec<_> = events(",,sub,,").collect();
        assert!(evs.iter().any(|e| matches!(e, Event::StartSubscript)));
        assert!(evs.iter().any(|e| matches!(e, Event::EndSubscript)));
    }
}
