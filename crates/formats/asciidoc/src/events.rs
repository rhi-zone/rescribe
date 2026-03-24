//! Streaming event iterator over a parsed `AsciiDoc`.

use crate::ast::*;
use std::collections::VecDeque;

/// An owned event from an AsciiDoc document.
#[derive(Debug)]
pub enum OwnedEvent {
    // ── Document ──────────────────────────────────────────────────────────────
    StartDocument,
    EndDocument,

    // ── Block events ──────────────────────────────────────────────────────────
    StartParagraph { id: Option<String>, role: Option<String>, checked: Option<bool> },
    EndParagraph,
    StartHeading { level: usize, id: Option<String>, role: Option<String> },
    EndHeading,
    StartCodeBlock { language: Option<String> },
    EndCodeBlock,
    CodeBlockContent(String),
    StartBlockquote { attribution: Option<String> },
    EndBlockquote,
    StartList { ordered: bool, style: Option<String> },
    EndList,
    StartListItem,
    EndListItem,
    StartDefinitionList,
    EndDefinitionList,
    StartDefinitionTerm,
    EndDefinitionTerm,
    StartDefinitionDesc,
    EndDefinitionDesc,
    HorizontalRule,
    PageBreak,
    Figure { url: String, alt: Option<String>, title: Option<String> },
    StartDiv { class: Option<String>, title: Option<String> },
    EndDiv,
    RawBlock { format: String, content: String },
    StartTable,
    EndTable,
    StartTableRow { is_header: bool },
    EndTableRow,
    StartTableCell,
    EndTableCell,

    // ── Inline events ─────────────────────────────────────────────────────────
    Text(String),
    SoftBreak,
    LineBreak,
    StartStrong,
    EndStrong,
    StartEmphasis,
    EndEmphasis,
    Code(String),
    StartSuperscript,
    EndSuperscript,
    StartSubscript,
    EndSubscript,
    StartHighlight,
    EndHighlight,
    StartStrikeout,
    EndStrikeout,
    StartUnderline,
    EndUnderline,
    StartSmallCaps,
    EndSmallCaps,
    StartQuoted { quote_type: String },
    EndQuoted,
    StartLink { url: String, target: Option<String> },
    EndLink,
    InlineImage { url: String, alt: Option<String>, title: Option<String> },
    FootnoteRef { label: String },
    StartFootnoteDef { label: String },
    EndFootnoteDef,
    MathInline { source: String },
    MathDisplay { source: String },
    RawInline { format: String, content: String },
    Anchor { id: String },
}

/// Public iterator that yields `OwnedEvent` items lazily — one block at a time.
pub struct EventIter<'a> {
    parser: crate::parse::Parser<'a>,
    event_buf: VecDeque<OwnedEvent>,
    started: bool,
    done: bool,
}

impl<'a> EventIter<'a> {
    pub(crate) fn new(input: &'a str) -> Self {
        EventIter {
            parser: crate::parse::Parser::new(input),
            event_buf: VecDeque::new(),
            started: false,
            done: false,
        }
    }
}

impl Iterator for EventIter<'_> {
    type Item = OwnedEvent;

    fn next(&mut self) -> Option<OwnedEvent> {
        if let Some(ev) = self.event_buf.pop_front() {
            return Some(ev);
        }
        if self.done {
            return None;
        }
        if !self.started {
            self.started = true;
            return Some(OwnedEvent::StartDocument);
        }
        loop {
            self.parser.skip_blank_lines();
            if self.parser.is_eof() {
                break;
            }
            if let Some(block) = self.parser.try_parse_block() {
                collect_block_events(&block, &mut self.event_buf);
                if let Some(ev) = self.event_buf.pop_front() {
                    return Some(ev);
                }
                // No events produced (shouldn't happen) — keep looping
            } else {
                self.parser.advance_line();
            }
        }
        self.done = true;
        Some(OwnedEvent::EndDocument)
    }
}

/// Collect a complete AsciiDoc from an EventIter.
/// Called by `parse::parse()` to reconstruct the AST from events.
pub(crate) fn collect_doc_from_iter(
    iter: &mut EventIter<'_>,
) -> (Vec<Block>, std::collections::HashMap<String, String>, Vec<Diagnostic>) {
    let mut block_stack: Vec<BlockFrame> = vec![BlockFrame::Document { blocks: Vec::new() }];
    let mut inline_ctx: Vec<InlineFrame> = Vec::new();

    for event in iter.by_ref() {
        handle_event(event, &mut block_stack, &mut inline_ctx);
    }

    let attributes = iter.parser.take_attributes();
    let diagnostics = std::mem::take(&mut iter.parser.diagnostics);

    let blocks = match block_stack.pop() {
        Some(BlockFrame::Document { blocks }) => blocks,
        _ => Vec::new(),
    };

    (blocks, attributes, diagnostics)
}

// ── Block frame stack ─────────────────────────────────────────────────────────

enum BlockFrame {
    Document { blocks: Vec<Block> },
    Paragraph { id: Option<String>, role: Option<String>, checked: Option<bool>, inlines: Vec<Inline> },
    Heading { level: usize, id: Option<String>, role: Option<String>, inlines: Vec<Inline> },
    CodeBlock { language: Option<String>, content: String },
    Blockquote { attribution: Option<String>, children: Vec<Block> },
    List { ordered: bool, style: Option<String>, items: Vec<Vec<Block>> },
    ListItem { blocks: Vec<Block> },
    DefinitionList { items: Vec<DefinitionItem> },
    DefinitionTerm { inlines: Vec<Inline> },
    DefinitionDesc { inlines: Vec<Inline> },
    Div { class: Option<String>, title: Option<String>, children: Vec<Block> },
    Table { rows: Vec<TableRow> },
    TableRow { is_header: bool, cells: Vec<Vec<Inline>> },
    TableCell { inlines: Vec<Inline> },
}

// ── Inline frame stack ────────────────────────────────────────────────────────

enum InlineFrame {
    Strong { inlines: Vec<Inline> },
    Emphasis { inlines: Vec<Inline> },
    Superscript { inlines: Vec<Inline> },
    Subscript { inlines: Vec<Inline> },
    Highlight { inlines: Vec<Inline> },
    Strikeout { inlines: Vec<Inline> },
    Underline { inlines: Vec<Inline> },
    SmallCaps { inlines: Vec<Inline> },
    Quoted { quote_type: QuoteType, inlines: Vec<Inline> },
    Link { url: String, target: Option<String>, inlines: Vec<Inline> },
    FootnoteDef { label: String, inlines: Vec<Inline> },
}

fn inlines_from_frame(frame: &mut InlineFrame) -> &mut Vec<Inline> {
    match frame {
        InlineFrame::Strong { inlines } => inlines,
        InlineFrame::Emphasis { inlines } => inlines,
        InlineFrame::Superscript { inlines } => inlines,
        InlineFrame::Subscript { inlines } => inlines,
        InlineFrame::Highlight { inlines } => inlines,
        InlineFrame::Strikeout { inlines } => inlines,
        InlineFrame::Underline { inlines } => inlines,
        InlineFrame::SmallCaps { inlines } => inlines,
        InlineFrame::Quoted { inlines, .. } => inlines,
        InlineFrame::Link { inlines, .. } => inlines,
        InlineFrame::FootnoteDef { inlines, .. } => inlines,
    }
}

fn push_inline(block_stack: &mut [BlockFrame], inline_ctx: &mut [InlineFrame], inline: Inline) {
    if let Some(frame) = inline_ctx.last_mut() {
        inlines_from_frame(frame).push(inline);
        return;
    }
    match block_stack.last_mut() {
        Some(BlockFrame::Paragraph { inlines, .. }) => inlines.push(inline),
        Some(BlockFrame::Heading { inlines, .. }) => inlines.push(inline),
        Some(BlockFrame::DefinitionTerm { inlines }) => inlines.push(inline),
        Some(BlockFrame::DefinitionDesc { inlines }) => inlines.push(inline),
        Some(BlockFrame::TableCell { inlines }) => inlines.push(inline),
        _ => {}
    }
}

fn push_block(block_stack: &mut [BlockFrame], block: Block) {
    match block_stack.last_mut() {
        Some(BlockFrame::Document { blocks }) => blocks.push(block),
        Some(BlockFrame::Blockquote { children, .. }) => children.push(block),
        Some(BlockFrame::ListItem { blocks }) => blocks.push(block),
        Some(BlockFrame::Div { children, .. }) => children.push(block),
        _ => {}
    }
}

fn handle_event(event: OwnedEvent, block_stack: &mut Vec<BlockFrame>, inline_ctx: &mut Vec<InlineFrame>) {
    match event {
        OwnedEvent::StartDocument | OwnedEvent::EndDocument => {
            // Document frame already managed by collect_doc_from_iter
        }

        // ── Block start events ─────────────────────────────────────────────
        OwnedEvent::StartParagraph { id, role, checked } => {
            block_stack.push(BlockFrame::Paragraph { id, role, checked, inlines: Vec::new() });
        }
        OwnedEvent::StartHeading { level, id, role } => {
            block_stack.push(BlockFrame::Heading { level, id, role, inlines: Vec::new() });
        }
        OwnedEvent::StartCodeBlock { language } => {
            block_stack.push(BlockFrame::CodeBlock { language, content: String::new() });
        }
        OwnedEvent::CodeBlockContent(content) => {
            if let Some(BlockFrame::CodeBlock { content: c, .. }) = block_stack.last_mut() {
                *c = content;
            }
        }
        OwnedEvent::StartBlockquote { attribution } => {
            block_stack.push(BlockFrame::Blockquote { attribution, children: Vec::new() });
        }
        OwnedEvent::StartList { ordered, style } => {
            block_stack.push(BlockFrame::List { ordered, style, items: Vec::new() });
        }
        OwnedEvent::StartListItem => {
            block_stack.push(BlockFrame::ListItem { blocks: Vec::new() });
        }
        OwnedEvent::StartDefinitionList => {
            block_stack.push(BlockFrame::DefinitionList { items: Vec::new() });
        }
        OwnedEvent::StartDefinitionTerm => {
            block_stack.push(BlockFrame::DefinitionTerm { inlines: Vec::new() });
        }
        OwnedEvent::StartDefinitionDesc => {
            block_stack.push(BlockFrame::DefinitionDesc { inlines: Vec::new() });
        }
        OwnedEvent::StartDiv { class, title } => {
            block_stack.push(BlockFrame::Div { class, title, children: Vec::new() });
        }
        OwnedEvent::StartTable => {
            block_stack.push(BlockFrame::Table { rows: Vec::new() });
        }
        OwnedEvent::StartTableRow { is_header } => {
            block_stack.push(BlockFrame::TableRow { is_header, cells: Vec::new() });
        }
        OwnedEvent::StartTableCell => {
            block_stack.push(BlockFrame::TableCell { inlines: Vec::new() });
        }
        OwnedEvent::StartFootnoteDef { label } => {
            // FootnoteDef is an inline-level frame (appears inside paragraph)
            inline_ctx.push(InlineFrame::FootnoteDef { label, inlines: Vec::new() });
        }

        // ── Block end events ───────────────────────────────────────────────
        OwnedEvent::EndParagraph => {
            if let Some(BlockFrame::Paragraph { id, role, checked, inlines }) = block_stack.pop() {
                push_block(block_stack, Block::Paragraph { inlines, id, role, checked, span: Span::NONE });
            }
        }
        OwnedEvent::EndHeading => {
            if let Some(BlockFrame::Heading { level, id, role, inlines }) = block_stack.pop() {
                push_block(block_stack, Block::Heading { level, inlines, id, role, span: Span::NONE });
            }
        }
        OwnedEvent::EndCodeBlock => {
            if let Some(BlockFrame::CodeBlock { language, content }) = block_stack.pop() {
                push_block(block_stack, Block::CodeBlock { content, language, span: Span::NONE });
            }
        }
        OwnedEvent::EndBlockquote => {
            if let Some(BlockFrame::Blockquote { attribution, children }) = block_stack.pop() {
                push_block(block_stack, Block::Blockquote { children, attribution, span: Span::NONE });
            }
        }
        OwnedEvent::EndList => {
            if let Some(BlockFrame::List { ordered, style, items }) = block_stack.pop() {
                push_block(block_stack, Block::List { ordered, items, style, span: Span::NONE });
            }
        }
        OwnedEvent::EndListItem => {
            if let Some(BlockFrame::ListItem { blocks }) = block_stack.pop() {
                if let Some(BlockFrame::List { items, .. }) = block_stack.last_mut() {
                    items.push(blocks);
                }
            }
        }
        OwnedEvent::EndDefinitionList => {
            if let Some(BlockFrame::DefinitionList { items }) = block_stack.pop() {
                push_block(block_stack, Block::DefinitionList { items, span: Span::NONE });
            }
        }
        OwnedEvent::EndDefinitionTerm => {
            if let Some(BlockFrame::DefinitionTerm { inlines }) = block_stack.pop() {
                // Push a pending term into the definition list — we need to hold it
                // until EndDefinitionDesc pairs it. Use a temporary DefinitionItem
                // with empty desc, then complete it on EndDefinitionDesc.
                if let Some(BlockFrame::DefinitionList { items }) = block_stack.last_mut() {
                    items.push(DefinitionItem { term: inlines, desc: Vec::new() });
                }
            }
        }
        OwnedEvent::EndDefinitionDesc => {
            if let Some(BlockFrame::DefinitionDesc { inlines }) = block_stack.pop() {
                if let Some(BlockFrame::DefinitionList { items }) = block_stack.last_mut() {
                    if let Some(last) = items.last_mut() {
                        last.desc = inlines;
                    }
                }
            }
        }
        OwnedEvent::EndDiv => {
            if let Some(BlockFrame::Div { class, title, children }) = block_stack.pop() {
                push_block(block_stack, Block::Div { class, title, children, span: Span::NONE });
            }
        }
        OwnedEvent::EndTable => {
            if let Some(BlockFrame::Table { rows }) = block_stack.pop() {
                push_block(block_stack, Block::Table { rows, span: Span::NONE });
            }
        }
        OwnedEvent::EndTableRow => {
            if let Some(BlockFrame::TableRow { is_header, cells }) = block_stack.pop() {
                if let Some(BlockFrame::Table { rows }) = block_stack.last_mut() {
                    rows.push(TableRow { cells, is_header });
                }
            }
        }
        OwnedEvent::EndTableCell => {
            if let Some(BlockFrame::TableCell { inlines }) = block_stack.pop() {
                if let Some(BlockFrame::TableRow { cells, .. }) = block_stack.last_mut() {
                    cells.push(inlines);
                }
            }
        }
        OwnedEvent::EndFootnoteDef => {
            if let Some(InlineFrame::FootnoteDef { label, inlines }) = inline_ctx.pop() {
                let footnote = Inline::FootnoteDef { label, children: inlines, span: Span::NONE };
                push_inline(block_stack, inline_ctx, footnote);
            }
        }

        // ── Leaf block events ──────────────────────────────────────────────
        OwnedEvent::HorizontalRule => {
            push_block(block_stack, Block::HorizontalRule { span: Span::NONE });
        }
        OwnedEvent::PageBreak => {
            push_block(block_stack, Block::PageBreak { span: Span::NONE });
        }
        OwnedEvent::Figure { url, alt, title } => {
            push_block(block_stack, Block::Figure {
                image: ImageData { url, alt, width: None, height: title },
                span: Span::NONE,
            });
        }
        OwnedEvent::RawBlock { format, content } => {
            push_block(block_stack, Block::RawBlock { format, content, span: Span::NONE });
        }

        // ── Inline events ──────────────────────────────────────────────────
        OwnedEvent::Text(text) => {
            push_inline(block_stack, inline_ctx, Inline::Text { text, span: Span::NONE });
        }
        OwnedEvent::SoftBreak => {
            push_inline(block_stack, inline_ctx, Inline::SoftBreak { span: Span::NONE });
        }
        OwnedEvent::LineBreak => {
            push_inline(block_stack, inline_ctx, Inline::LineBreak { span: Span::NONE });
        }
        OwnedEvent::Code(content) => {
            push_inline(block_stack, inline_ctx, Inline::Code(content, Span::NONE));
        }
        OwnedEvent::InlineImage { url, alt, title } => {
            push_inline(block_stack, inline_ctx, Inline::Image(
                ImageData { url, alt, width: None, height: title },
                Span::NONE,
            ));
        }
        OwnedEvent::FootnoteRef { label } => {
            push_inline(block_stack, inline_ctx, Inline::FootnoteRef { label, span: Span::NONE });
        }
        OwnedEvent::MathInline { source } => {
            push_inline(block_stack, inline_ctx, Inline::MathInline { source, span: Span::NONE });
        }
        OwnedEvent::MathDisplay { source } => {
            push_inline(block_stack, inline_ctx, Inline::MathDisplay { source, span: Span::NONE });
        }
        OwnedEvent::RawInline { format, content } => {
            push_inline(block_stack, inline_ctx, Inline::RawInline { format, content, span: Span::NONE });
        }
        OwnedEvent::Anchor { id } => {
            push_inline(block_stack, inline_ctx, Inline::Anchor { id, span: Span::NONE });
        }

        // ── Inline container start events ──────────────────────────────────
        OwnedEvent::StartStrong => {
            inline_ctx.push(InlineFrame::Strong { inlines: Vec::new() });
        }
        OwnedEvent::StartEmphasis => {
            inline_ctx.push(InlineFrame::Emphasis { inlines: Vec::new() });
        }
        OwnedEvent::StartSuperscript => {
            inline_ctx.push(InlineFrame::Superscript { inlines: Vec::new() });
        }
        OwnedEvent::StartSubscript => {
            inline_ctx.push(InlineFrame::Subscript { inlines: Vec::new() });
        }
        OwnedEvent::StartHighlight => {
            inline_ctx.push(InlineFrame::Highlight { inlines: Vec::new() });
        }
        OwnedEvent::StartStrikeout => {
            inline_ctx.push(InlineFrame::Strikeout { inlines: Vec::new() });
        }
        OwnedEvent::StartUnderline => {
            inline_ctx.push(InlineFrame::Underline { inlines: Vec::new() });
        }
        OwnedEvent::StartSmallCaps => {
            inline_ctx.push(InlineFrame::SmallCaps { inlines: Vec::new() });
        }
        OwnedEvent::StartQuoted { quote_type } => {
            let qt = if quote_type == "single" { QuoteType::Single } else { QuoteType::Double };
            inline_ctx.push(InlineFrame::Quoted { quote_type: qt, inlines: Vec::new() });
        }
        OwnedEvent::StartLink { url, target } => {
            inline_ctx.push(InlineFrame::Link { url, target, inlines: Vec::new() });
        }

        // ── Inline container end events ────────────────────────────────────
        OwnedEvent::EndStrong => {
            if let Some(InlineFrame::Strong { inlines }) = inline_ctx.pop() {
                push_inline(block_stack, inline_ctx, Inline::Strong(inlines, Span::NONE));
            }
        }
        OwnedEvent::EndEmphasis => {
            if let Some(InlineFrame::Emphasis { inlines }) = inline_ctx.pop() {
                push_inline(block_stack, inline_ctx, Inline::Emphasis(inlines, Span::NONE));
            }
        }
        OwnedEvent::EndSuperscript => {
            if let Some(InlineFrame::Superscript { inlines }) = inline_ctx.pop() {
                push_inline(block_stack, inline_ctx, Inline::Superscript(inlines, Span::NONE));
            }
        }
        OwnedEvent::EndSubscript => {
            if let Some(InlineFrame::Subscript { inlines }) = inline_ctx.pop() {
                push_inline(block_stack, inline_ctx, Inline::Subscript(inlines, Span::NONE));
            }
        }
        OwnedEvent::EndHighlight => {
            if let Some(InlineFrame::Highlight { inlines }) = inline_ctx.pop() {
                push_inline(block_stack, inline_ctx, Inline::Highlight(inlines, Span::NONE));
            }
        }
        OwnedEvent::EndStrikeout => {
            if let Some(InlineFrame::Strikeout { inlines }) = inline_ctx.pop() {
                push_inline(block_stack, inline_ctx, Inline::Strikeout(inlines, Span::NONE));
            }
        }
        OwnedEvent::EndUnderline => {
            if let Some(InlineFrame::Underline { inlines }) = inline_ctx.pop() {
                push_inline(block_stack, inline_ctx, Inline::Underline(inlines, Span::NONE));
            }
        }
        OwnedEvent::EndSmallCaps => {
            if let Some(InlineFrame::SmallCaps { inlines }) = inline_ctx.pop() {
                push_inline(block_stack, inline_ctx, Inline::SmallCaps(inlines, Span::NONE));
            }
        }
        OwnedEvent::EndQuoted => {
            if let Some(InlineFrame::Quoted { quote_type, inlines }) = inline_ctx.pop() {
                push_inline(block_stack, inline_ctx, Inline::Quoted { quote_type, children: inlines, span: Span::NONE });
            }
        }
        OwnedEvent::EndLink => {
            if let Some(InlineFrame::Link { url, target, inlines }) = inline_ctx.pop() {
                push_inline(block_stack, inline_ctx, Inline::Link { url, children: inlines, target, span: Span::NONE });
            }
        }
    }
}

fn collect_block_events(block: &Block, queue: &mut VecDeque<OwnedEvent>) {
    match block {
        Block::Paragraph { inlines, id, role, checked, .. } => {
            queue.push_back(OwnedEvent::StartParagraph {
                id: id.clone(),
                role: role.clone(),
                checked: *checked,
            });
            collect_inlines_events(inlines, queue);
            queue.push_back(OwnedEvent::EndParagraph);
        }
        Block::Heading { level, inlines, id, role, .. } => {
            queue.push_back(OwnedEvent::StartHeading {
                level: *level,
                id: id.clone(),
                role: role.clone(),
            });
            collect_inlines_events(inlines, queue);
            queue.push_back(OwnedEvent::EndHeading);
        }
        Block::CodeBlock { content, language, .. } => {
            queue.push_back(OwnedEvent::StartCodeBlock { language: language.clone() });
            queue.push_back(OwnedEvent::CodeBlockContent(content.clone()));
            queue.push_back(OwnedEvent::EndCodeBlock);
        }
        Block::Blockquote { children, attribution, .. } => {
            queue.push_back(OwnedEvent::StartBlockquote { attribution: attribution.clone() });
            collect_blocks_events(children, queue);
            queue.push_back(OwnedEvent::EndBlockquote);
        }
        Block::List { ordered, items, style, .. } => {
            queue.push_back(OwnedEvent::StartList {
                ordered: *ordered,
                style: style.clone(),
            });
            for item in items {
                queue.push_back(OwnedEvent::StartListItem);
                collect_blocks_events(item, queue);
                queue.push_back(OwnedEvent::EndListItem);
            }
            queue.push_back(OwnedEvent::EndList);
        }
        Block::DefinitionList { items, .. } => {
            queue.push_back(OwnedEvent::StartDefinitionList);
            for item in items {
                queue.push_back(OwnedEvent::StartDefinitionTerm);
                collect_inlines_events(&item.term, queue);
                queue.push_back(OwnedEvent::EndDefinitionTerm);
                queue.push_back(OwnedEvent::StartDefinitionDesc);
                collect_inlines_events(&item.desc, queue);
                queue.push_back(OwnedEvent::EndDefinitionDesc);
            }
            queue.push_back(OwnedEvent::EndDefinitionList);
        }
        Block::HorizontalRule { .. } => {
            queue.push_back(OwnedEvent::HorizontalRule);
        }
        Block::PageBreak { .. } => {
            queue.push_back(OwnedEvent::PageBreak);
        }
        Block::Figure { image, .. } => {
            queue.push_back(OwnedEvent::Figure {
                url: image.url.clone(),
                alt: image.alt.clone(),
                title: image.height.clone().or_else(|| image.width.clone()),
            });
        }
        Block::Div { class, title, children, .. } => {
            queue.push_back(OwnedEvent::StartDiv {
                class: class.clone(),
                title: title.clone(),
            });
            collect_blocks_events(children, queue);
            queue.push_back(OwnedEvent::EndDiv);
        }
        Block::RawBlock { format, content, .. } => {
            queue.push_back(OwnedEvent::RawBlock {
                format: format.clone(),
                content: content.clone(),
            });
        }
        Block::Table { rows, .. } => {
            queue.push_back(OwnedEvent::StartTable);
            for row in rows {
                queue.push_back(OwnedEvent::StartTableRow { is_header: row.is_header });
                for cell in &row.cells {
                    queue.push_back(OwnedEvent::StartTableCell);
                    collect_inlines_events(cell, queue);
                    queue.push_back(OwnedEvent::EndTableCell);
                }
                queue.push_back(OwnedEvent::EndTableRow);
            }
            queue.push_back(OwnedEvent::EndTable);
        }
    }
}

fn collect_blocks_events(blocks: &[Block], queue: &mut VecDeque<OwnedEvent>) {
    for block in blocks {
        collect_block_events(block, queue);
    }
}

fn collect_inlines_events(inlines: &[Inline], queue: &mut VecDeque<OwnedEvent>) {
    for inline in inlines {
        collect_inline_events(inline, queue);
    }
}

fn collect_inline_events(inline: &Inline, queue: &mut VecDeque<OwnedEvent>) {
    match inline {
        Inline::Text { text, .. } => {
            queue.push_back(OwnedEvent::Text(text.clone()));
        }
        Inline::SoftBreak { .. } => {
            queue.push_back(OwnedEvent::SoftBreak);
        }
        Inline::LineBreak { .. } => {
            queue.push_back(OwnedEvent::LineBreak);
        }
        Inline::Strong(children, _) => {
            queue.push_back(OwnedEvent::StartStrong);
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndStrong);
        }
        Inline::Emphasis(children, _) => {
            queue.push_back(OwnedEvent::StartEmphasis);
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndEmphasis);
        }
        Inline::Code(content, _) => {
            queue.push_back(OwnedEvent::Code(content.clone()));
        }
        Inline::Superscript(children, _) => {
            queue.push_back(OwnedEvent::StartSuperscript);
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndSuperscript);
        }
        Inline::Subscript(children, _) => {
            queue.push_back(OwnedEvent::StartSubscript);
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndSubscript);
        }
        Inline::Highlight(children, _) => {
            queue.push_back(OwnedEvent::StartHighlight);
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndHighlight);
        }
        Inline::Strikeout(children, _) => {
            queue.push_back(OwnedEvent::StartStrikeout);
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndStrikeout);
        }
        Inline::Underline(children, _) => {
            queue.push_back(OwnedEvent::StartUnderline);
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndUnderline);
        }
        Inline::SmallCaps(children, _) => {
            queue.push_back(OwnedEvent::StartSmallCaps);
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndSmallCaps);
        }
        Inline::Quoted { quote_type, children, .. } => {
            let qt = match quote_type {
                QuoteType::Single => "single".to_string(),
                QuoteType::Double => "double".to_string(),
            };
            queue.push_back(OwnedEvent::StartQuoted { quote_type: qt });
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndQuoted);
        }
        Inline::Link { url, children, target, .. } => {
            queue.push_back(OwnedEvent::StartLink {
                url: url.clone(),
                target: target.clone(),
            });
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndLink);
        }
        Inline::Image(img, _) => {
            queue.push_back(OwnedEvent::InlineImage {
                url: img.url.clone(),
                alt: img.alt.clone(),
                title: img.height.clone().or_else(|| img.width.clone()),
            });
        }
        Inline::FootnoteRef { label, .. } => {
            queue.push_back(OwnedEvent::FootnoteRef { label: label.clone() });
        }
        Inline::FootnoteDef { label, children, .. } => {
            queue.push_back(OwnedEvent::StartFootnoteDef { label: label.clone() });
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndFootnoteDef);
        }
        Inline::MathInline { source, .. } => {
            queue.push_back(OwnedEvent::MathInline { source: source.clone() });
        }
        Inline::MathDisplay { source, .. } => {
            queue.push_back(OwnedEvent::MathDisplay { source: source.clone() });
        }
        Inline::RawInline { format, content, .. } => {
            queue.push_back(OwnedEvent::RawInline {
                format: format.clone(),
                content: content.clone(),
            });
        }
        Inline::Anchor { id, .. } => {
            queue.push_back(OwnedEvent::Anchor { id: id.clone() });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_events_heading() {
        let evs: Vec<_> = EventIter::new("== Hello World").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartHeading { level: 2, .. })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndHeading)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::Text(t) if t == "Hello World")));
    }

    #[test]
    fn test_events_paragraph() {
        let evs: Vec<_> = EventIter::new("Hello world").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartParagraph { .. })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndParagraph)));
    }

    #[test]
    fn test_events_code_block() {
        let evs: Vec<_> = EventIter::new("[source,python]\n----\nprint('hello')\n----").collect();
        assert!(evs
            .iter()
            .any(|e| matches!(e, OwnedEvent::StartCodeBlock { language: Some(l) } if l == "python")));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndCodeBlock)));
    }

    #[test]
    fn test_events_strong() {
        let evs: Vec<_> = EventIter::new("This is *bold* text.").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartStrong)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndStrong)));
    }

    #[test]
    fn test_events_list() {
        let evs: Vec<_> = EventIter::new("* item one\n* item two").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartList { ordered: false, .. })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartListItem)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndList)));
    }

    #[test]
    fn test_events_document_wrap() {
        let evs: Vec<_> = EventIter::new("Hello").collect();
        assert!(matches!(evs.first(), Some(OwnedEvent::StartDocument)));
        assert!(matches!(evs.last(), Some(OwnedEvent::EndDocument)));
    }
}
