//! Streaming event iterator over a parsed [`RstDoc`].

use std::collections::VecDeque;

use crate::{Block, DefinitionItem, Inline, TableRow};

/// An owned event from an RST document.
#[derive(Debug, Clone)]
pub enum OwnedEvent {
    // Block events
    StartParagraph,
    EndParagraph,
    StartHeading { level: i64 },
    EndHeading,
    StartBlockquote,
    EndBlockquote,
    StartList { ordered: bool },
    EndList,
    StartListItem,
    EndListItem,
    StartCodeBlock { language: Option<String> },
    EndCodeBlock,
    CodeBlockContent(String),
    RawBlock { format: String, content: String },
    StartDiv { class: Option<String>, directive: Option<String> },
    EndDiv,
    HorizontalRule,
    StartTable,
    EndTable,
    StartTableRow { is_header: bool },
    EndTableRow,
    StartTableCell,
    EndTableCell,
    StartDefinitionList,
    EndDefinitionList,
    StartDefinitionTerm,
    EndDefinitionTerm,
    StartDefinitionDesc,
    EndDefinitionDesc,
    StartFootnoteDef { label: String },
    EndFootnoteDef,
    MathDisplay { source: String },
    StartAdmonition { admonition_type: String },
    EndAdmonition,
    StartFigure { url: String, alt: Option<String> },
    EndFigure,
    /// Image block (standalone, no caption)
    ImageBlock { url: String, alt: Option<String>, title: Option<String> },
    StartLineBlock,
    EndLineBlock,
    StartLineBlockLine,
    EndLineBlockLine,
    // Inline events
    Text(String),
    SoftBreak,
    LineBreak,
    StartEmphasis,
    EndEmphasis,
    StartStrong,
    EndStrong,
    StartStrikeout,
    EndStrikeout,
    StartUnderline,
    EndUnderline,
    StartSubscript,
    EndSubscript,
    StartSuperscript,
    EndSuperscript,
    StartSmallCaps,
    EndSmallCaps,
    Code(String),
    StartLink { url: String },
    EndLink,
    InlineImage { url: String, alt: String },
    FootnoteRef { label: String },
    StartFootnoteDefInline { label: String },
    EndFootnoteDefInline,
    StartQuoted { quote_type: String },
    EndQuoted,
    MathInline { source: String },
    StartRstSpan { role: String },
    EndRstSpan,
}

pub use crate::EventIter;


/// Collect all blocks from an [`EventIter`] into a `Vec<Block>`.
///
/// Called by `parse()` to reconstruct the AST from the lazy event stream.
pub(crate) fn collect_doc_from_iter(iter: &mut EventIter<'_>) -> Vec<Block> {
    let mut block_stack: Vec<BlockFrame> = vec![BlockFrame::Document { blocks: Vec::new() }];
    let mut inline_ctx: Vec<InlineFrame> = Vec::new();

    for event in iter.by_ref() {
        handle_event(event, &mut block_stack, &mut inline_ctx);
    }

    match block_stack.pop() {
        Some(BlockFrame::Document { blocks }) => blocks,
        _ => Vec::new(),
    }
}

// ── Block frame stack ─────────────────────────────────────────────────────────

enum BlockFrame {
    Document { blocks: Vec<Block> },
    Paragraph { inlines: Vec<Inline> },
    Heading { level: i64, inlines: Vec<Inline> },
    CodeBlock { language: Option<String>, content: String },
    Blockquote { children: Vec<Block> },
    List { ordered: bool, items: Vec<Vec<Block>> },
    ListItem { blocks: Vec<Block> },
    DefinitionList { items: Vec<DefinitionItem> },
    DefinitionTerm { inlines: Vec<Inline> },
    DefinitionDesc { inlines: Vec<Inline> },
    Div { class: Option<String>, directive: Option<String>, children: Vec<Block> },
    Table { rows: Vec<TableRow> },
    TableRow { is_header: bool, cells: Vec<Vec<Inline>> },
    TableCell { inlines: Vec<Inline> },
    FootnoteDef { label: String, inlines: Vec<Inline> },
    Admonition { admonition_type: String, children: Vec<Block> },
    Figure { url: String, alt: Option<String>, caption_inlines: Vec<Inline> },
    LineBlock { lines: Vec<Vec<Inline>> },
    LineBlockLine { inlines: Vec<Inline> },
}

// ── Inline frame stack ────────────────────────────────────────────────────────

enum InlineFrame {
    Emphasis { inlines: Vec<Inline> },
    Strong { inlines: Vec<Inline> },
    Strikeout { inlines: Vec<Inline> },
    Underline { inlines: Vec<Inline> },
    Subscript { inlines: Vec<Inline> },
    Superscript { inlines: Vec<Inline> },
    SmallCaps { inlines: Vec<Inline> },
    Link { url: String, children: Vec<Inline> },
    FootnoteDefInline { label: String, children: Vec<Inline> },
    Quoted { quote_type: String, children: Vec<Inline> },
    RstSpan { role: String, children: Vec<Inline> },
}

fn push_inline(inline: Inline, block_stack: &mut [BlockFrame], inline_ctx: &mut [InlineFrame]) {
    if let Some(frame) = inline_ctx.last_mut() {
        match frame {
            InlineFrame::Emphasis { inlines }
            | InlineFrame::Strong { inlines }
            | InlineFrame::Strikeout { inlines }
            | InlineFrame::Underline { inlines }
            | InlineFrame::Subscript { inlines }
            | InlineFrame::Superscript { inlines }
            | InlineFrame::SmallCaps { inlines }
            | InlineFrame::Link { children: inlines, .. }
            | InlineFrame::FootnoteDefInline { children: inlines, .. }
            | InlineFrame::Quoted { children: inlines, .. }
            | InlineFrame::RstSpan { children: inlines, .. } => {
                inlines.push(inline);
            }
        }
        return;
    }
    // No inline frame — push to current block's inline list.
    match block_stack.last_mut() {
        Some(BlockFrame::Paragraph { inlines })
        | Some(BlockFrame::Heading { inlines, .. })
        | Some(BlockFrame::FootnoteDef { inlines, .. })
        | Some(BlockFrame::TableCell { inlines })
        | Some(BlockFrame::DefinitionTerm { inlines })
        | Some(BlockFrame::DefinitionDesc { inlines })
        | Some(BlockFrame::Figure { caption_inlines: inlines, .. })
        | Some(BlockFrame::LineBlockLine { inlines }) => {
            inlines.push(inline);
        }
        _ => {} // unexpected — drop
    }
}

fn push_block(block: Block, block_stack: &mut [BlockFrame]) {
    match block_stack.last_mut() {
        Some(BlockFrame::Document { blocks })
        | Some(BlockFrame::Blockquote { children: blocks })
        | Some(BlockFrame::ListItem { blocks })
        | Some(BlockFrame::Div { children: blocks, .. })
        | Some(BlockFrame::Admonition { children: blocks, .. }) => {
            blocks.push(block);
        }
        _ => {} // unexpected — drop
    }
}

#[allow(clippy::too_many_lines)]
fn handle_event(event: OwnedEvent, block_stack: &mut Vec<BlockFrame>, inline_ctx: &mut Vec<InlineFrame>) {
    match event {
        // ── Block openers ─────────────────────────────────────────────────────
        OwnedEvent::StartParagraph => {
            block_stack.push(BlockFrame::Paragraph { inlines: Vec::new() });
        }
        OwnedEvent::EndParagraph => {
            if let Some(BlockFrame::Paragraph { inlines }) = block_stack.pop() {
                push_block(Block::Paragraph { inlines }, block_stack);
            }
        }
        OwnedEvent::StartHeading { level } => {
            block_stack.push(BlockFrame::Heading { level, inlines: Vec::new() });
        }
        OwnedEvent::EndHeading => {
            if let Some(BlockFrame::Heading { level, inlines }) = block_stack.pop() {
                push_block(Block::Heading { level, inlines }, block_stack);
            }
        }
        OwnedEvent::StartCodeBlock { language } => {
            block_stack.push(BlockFrame::CodeBlock { language, content: String::new() });
        }
        OwnedEvent::CodeBlockContent(c) => {
            if let Some(BlockFrame::CodeBlock { content, .. }) = block_stack.last_mut() {
                *content = c;
            }
        }
        OwnedEvent::EndCodeBlock => {
            if let Some(BlockFrame::CodeBlock { language, content }) = block_stack.pop() {
                push_block(Block::CodeBlock { language, content }, block_stack);
            }
        }
        OwnedEvent::StartBlockquote => {
            block_stack.push(BlockFrame::Blockquote { children: Vec::new() });
        }
        OwnedEvent::EndBlockquote => {
            if let Some(BlockFrame::Blockquote { children }) = block_stack.pop() {
                push_block(Block::Blockquote { children }, block_stack);
            }
        }
        OwnedEvent::StartList { ordered } => {
            block_stack.push(BlockFrame::List { ordered, items: Vec::new() });
        }
        OwnedEvent::EndList => {
            if let Some(BlockFrame::List { ordered, items }) = block_stack.pop() {
                push_block(Block::List { ordered, items }, block_stack);
            }
        }
        OwnedEvent::StartListItem => {
            block_stack.push(BlockFrame::ListItem { blocks: Vec::new() });
        }
        OwnedEvent::EndListItem => {
            if let Some(BlockFrame::ListItem { blocks }) = block_stack.pop() {
                if let Some(BlockFrame::List { items, .. }) = block_stack.last_mut() {
                    items.push(blocks);
                }
            }
        }
        OwnedEvent::StartDefinitionList => {
            block_stack.push(BlockFrame::DefinitionList { items: Vec::new() });
        }
        OwnedEvent::EndDefinitionList => {
            if let Some(BlockFrame::DefinitionList { items }) = block_stack.pop() {
                push_block(Block::DefinitionList { items }, block_stack);
            }
        }
        OwnedEvent::StartDefinitionTerm => {
            block_stack.push(BlockFrame::DefinitionTerm { inlines: Vec::new() });
        }
        OwnedEvent::EndDefinitionTerm => {
            if let Some(BlockFrame::DefinitionTerm { inlines }) = block_stack.pop() {
                // Push a partial DefinitionItem with term filled, desc empty.
                // The EndDefinitionDesc will complete it.
                if let Some(BlockFrame::DefinitionList { items }) = block_stack.last_mut() {
                    items.push(DefinitionItem { term: inlines, desc: Vec::new() });
                }
            }
        }
        OwnedEvent::StartDefinitionDesc => {
            block_stack.push(BlockFrame::DefinitionDesc { inlines: Vec::new() });
        }
        OwnedEvent::EndDefinitionDesc => {
            if let Some(BlockFrame::DefinitionDesc { inlines }) = block_stack.pop() {
                if let Some(BlockFrame::DefinitionList { items }) = block_stack.last_mut() {
                    if let Some(last_item) = items.last_mut() {
                        last_item.desc = inlines;
                    }
                }
            }
        }
        OwnedEvent::StartDiv { class, directive } => {
            block_stack.push(BlockFrame::Div { class, directive, children: Vec::new() });
        }
        OwnedEvent::EndDiv => {
            if let Some(BlockFrame::Div { class, directive, children }) = block_stack.pop() {
                push_block(Block::Div { class, directive, children }, block_stack);
            }
        }
        OwnedEvent::HorizontalRule => {
            push_block(Block::HorizontalRule, block_stack);
        }
        OwnedEvent::StartTable => {
            block_stack.push(BlockFrame::Table { rows: Vec::new() });
        }
        OwnedEvent::EndTable => {
            if let Some(BlockFrame::Table { rows }) = block_stack.pop() {
                push_block(Block::Table { rows }, block_stack);
            }
        }
        OwnedEvent::StartTableRow { is_header } => {
            block_stack.push(BlockFrame::TableRow { is_header, cells: Vec::new() });
        }
        OwnedEvent::EndTableRow => {
            if let Some(BlockFrame::TableRow { is_header, cells }) = block_stack.pop() {
                if let Some(BlockFrame::Table { rows }) = block_stack.last_mut() {
                    rows.push(TableRow { cells, is_header });
                }
            }
        }
        OwnedEvent::StartTableCell => {
            block_stack.push(BlockFrame::TableCell { inlines: Vec::new() });
        }
        OwnedEvent::EndTableCell => {
            if let Some(BlockFrame::TableCell { inlines }) = block_stack.pop() {
                if let Some(BlockFrame::TableRow { cells, .. }) = block_stack.last_mut() {
                    cells.push(inlines);
                }
            }
        }
        OwnedEvent::StartFootnoteDef { label } => {
            block_stack.push(BlockFrame::FootnoteDef { label, inlines: Vec::new() });
        }
        OwnedEvent::EndFootnoteDef => {
            if let Some(BlockFrame::FootnoteDef { label, inlines }) = block_stack.pop() {
                push_block(Block::FootnoteDef { label, inlines }, block_stack);
            }
        }
        OwnedEvent::MathDisplay { source } => {
            push_block(Block::MathDisplay { source }, block_stack);
        }
        OwnedEvent::StartAdmonition { admonition_type } => {
            block_stack.push(BlockFrame::Admonition { admonition_type, children: Vec::new() });
        }
        OwnedEvent::EndAdmonition => {
            if let Some(BlockFrame::Admonition { admonition_type, children }) = block_stack.pop() {
                push_block(Block::Admonition { admonition_type, children }, block_stack);
            }
        }
        OwnedEvent::StartFigure { url, alt } => {
            block_stack.push(BlockFrame::Figure { url, alt, caption_inlines: Vec::new() });
        }
        OwnedEvent::EndFigure => {
            if let Some(BlockFrame::Figure { url, alt, caption_inlines }) = block_stack.pop() {
                let caption = if caption_inlines.is_empty() { None } else { Some(caption_inlines) };
                push_block(Block::Figure { url, alt, caption }, block_stack);
            }
        }
        OwnedEvent::ImageBlock { url, alt, title } => {
            push_block(Block::Image { url, alt, title }, block_stack);
        }
        OwnedEvent::RawBlock { format, content } => {
            push_block(Block::RawBlock { format, content }, block_stack);
        }
        OwnedEvent::StartLineBlock => {
            block_stack.push(BlockFrame::LineBlock { lines: Vec::new() });
        }
        OwnedEvent::EndLineBlock => {
            if let Some(BlockFrame::LineBlock { lines }) = block_stack.pop() {
                push_block(Block::LineBlock { lines }, block_stack);
            }
        }
        OwnedEvent::StartLineBlockLine => {
            block_stack.push(BlockFrame::LineBlockLine { inlines: Vec::new() });
        }
        OwnedEvent::EndLineBlockLine => {
            if let Some(BlockFrame::LineBlockLine { inlines }) = block_stack.pop() {
                if let Some(BlockFrame::LineBlock { lines }) = block_stack.last_mut() {
                    lines.push(inlines);
                }
            }
        }

        // ── Inline events ─────────────────────────────────────────────────────
        OwnedEvent::Text(s) => push_inline(Inline::Text(s), block_stack, inline_ctx),
        OwnedEvent::SoftBreak => push_inline(Inline::SoftBreak, block_stack, inline_ctx),
        OwnedEvent::LineBreak => push_inline(Inline::LineBreak, block_stack, inline_ctx),
        OwnedEvent::Code(s) => push_inline(Inline::Code(s), block_stack, inline_ctx),
        OwnedEvent::FootnoteRef { label } => {
            push_inline(Inline::FootnoteRef { label }, block_stack, inline_ctx);
        }
        OwnedEvent::InlineImage { url, alt } => {
            push_inline(Inline::Image { url, alt }, block_stack, inline_ctx);
        }
        OwnedEvent::MathInline { source } => {
            push_inline(Inline::MathInline { source }, block_stack, inline_ctx);
        }
        OwnedEvent::StartEmphasis => inline_ctx.push(InlineFrame::Emphasis { inlines: Vec::new() }),
        OwnedEvent::EndEmphasis => {
            if let Some(InlineFrame::Emphasis { inlines }) = inline_ctx.pop() {
                push_inline(Inline::Emphasis(inlines), block_stack, inline_ctx);
            }
        }
        OwnedEvent::StartStrong => inline_ctx.push(InlineFrame::Strong { inlines: Vec::new() }),
        OwnedEvent::EndStrong => {
            if let Some(InlineFrame::Strong { inlines }) = inline_ctx.pop() {
                push_inline(Inline::Strong(inlines), block_stack, inline_ctx);
            }
        }
        OwnedEvent::StartStrikeout => inline_ctx.push(InlineFrame::Strikeout { inlines: Vec::new() }),
        OwnedEvent::EndStrikeout => {
            if let Some(InlineFrame::Strikeout { inlines }) = inline_ctx.pop() {
                push_inline(Inline::Strikeout(inlines), block_stack, inline_ctx);
            }
        }
        OwnedEvent::StartUnderline => inline_ctx.push(InlineFrame::Underline { inlines: Vec::new() }),
        OwnedEvent::EndUnderline => {
            if let Some(InlineFrame::Underline { inlines }) = inline_ctx.pop() {
                push_inline(Inline::Underline(inlines), block_stack, inline_ctx);
            }
        }
        OwnedEvent::StartSubscript => inline_ctx.push(InlineFrame::Subscript { inlines: Vec::new() }),
        OwnedEvent::EndSubscript => {
            if let Some(InlineFrame::Subscript { inlines }) = inline_ctx.pop() {
                push_inline(Inline::Subscript(inlines), block_stack, inline_ctx);
            }
        }
        OwnedEvent::StartSuperscript => inline_ctx.push(InlineFrame::Superscript { inlines: Vec::new() }),
        OwnedEvent::EndSuperscript => {
            if let Some(InlineFrame::Superscript { inlines }) = inline_ctx.pop() {
                push_inline(Inline::Superscript(inlines), block_stack, inline_ctx);
            }
        }
        OwnedEvent::StartSmallCaps => inline_ctx.push(InlineFrame::SmallCaps { inlines: Vec::new() }),
        OwnedEvent::EndSmallCaps => {
            if let Some(InlineFrame::SmallCaps { inlines }) = inline_ctx.pop() {
                push_inline(Inline::SmallCaps(inlines), block_stack, inline_ctx);
            }
        }
        OwnedEvent::StartLink { url } => {
            inline_ctx.push(InlineFrame::Link { url, children: Vec::new() });
        }
        OwnedEvent::EndLink => {
            if let Some(InlineFrame::Link { url, children }) = inline_ctx.pop() {
                push_inline(Inline::Link { url, children }, block_stack, inline_ctx);
            }
        }
        OwnedEvent::StartFootnoteDefInline { label } => {
            inline_ctx.push(InlineFrame::FootnoteDefInline { label, children: Vec::new() });
        }
        OwnedEvent::EndFootnoteDefInline => {
            if let Some(InlineFrame::FootnoteDefInline { label, children }) = inline_ctx.pop() {
                push_inline(Inline::FootnoteDef { label, children }, block_stack, inline_ctx);
            }
        }
        OwnedEvent::StartQuoted { quote_type } => {
            inline_ctx.push(InlineFrame::Quoted { quote_type, children: Vec::new() });
        }
        OwnedEvent::EndQuoted => {
            if let Some(InlineFrame::Quoted { quote_type, children }) = inline_ctx.pop() {
                push_inline(Inline::Quoted { quote_type, children }, block_stack, inline_ctx);
            }
        }
        OwnedEvent::StartRstSpan { role } => {
            inline_ctx.push(InlineFrame::RstSpan { role, children: Vec::new() });
        }
        OwnedEvent::EndRstSpan => {
            if let Some(InlineFrame::RstSpan { role, children }) = inline_ctx.pop() {
                push_inline(Inline::RstSpan { role, children }, block_stack, inline_ctx);
            }
        }
    }
}

// ── Serialize Block → events ───────────────────────────────────────────────────

/// Walk an [`RstDoc`] and collect events into the given queue.
#[cfg(test)]
fn events_from_doc(doc: &crate::RstDoc, queue: &mut VecDeque<OwnedEvent>) {
    collect_blocks_events(&doc.blocks, queue);
}

fn collect_blocks_events(blocks: &[Block], queue: &mut VecDeque<OwnedEvent>) {
    for block in blocks {
        collect_block_events(block, queue);
    }
}

pub(crate) fn collect_block_events(block: &Block, queue: &mut VecDeque<OwnedEvent>) {
    match block {
        Block::Paragraph { inlines } => {
            queue.push_back(OwnedEvent::StartParagraph);
            collect_inlines_events(inlines, queue);
            queue.push_back(OwnedEvent::EndParagraph);
        }
        Block::Heading { level, inlines } => {
            queue.push_back(OwnedEvent::StartHeading { level: *level });
            collect_inlines_events(inlines, queue);
            queue.push_back(OwnedEvent::EndHeading);
        }
        Block::CodeBlock { language, content } => {
            queue.push_back(OwnedEvent::StartCodeBlock { language: language.clone() });
            queue.push_back(OwnedEvent::CodeBlockContent(content.clone()));
            queue.push_back(OwnedEvent::EndCodeBlock);
        }
        Block::Blockquote { children } => {
            queue.push_back(OwnedEvent::StartBlockquote);
            collect_blocks_events(children, queue);
            queue.push_back(OwnedEvent::EndBlockquote);
        }
        Block::List { ordered, items } => {
            queue.push_back(OwnedEvent::StartList { ordered: *ordered });
            for item in items {
                queue.push_back(OwnedEvent::StartListItem);
                collect_blocks_events(item, queue);
                queue.push_back(OwnedEvent::EndListItem);
            }
            queue.push_back(OwnedEvent::EndList);
        }
        Block::DefinitionList { items } => {
            queue.push_back(OwnedEvent::StartDefinitionList);
            for item in items {
                collect_definition_item_events(item, queue);
            }
            queue.push_back(OwnedEvent::EndDefinitionList);
        }
        Block::Figure { url, alt, caption } => {
            queue.push_back(OwnedEvent::StartFigure { url: url.clone(), alt: alt.clone() });
            if let Some(cap) = caption {
                collect_inlines_events(cap, queue);
            }
            queue.push_back(OwnedEvent::EndFigure);
        }
        Block::Image { url, alt, title } => {
            queue.push_back(OwnedEvent::ImageBlock {
                url: url.clone(),
                alt: alt.clone(),
                title: title.clone(),
            });
        }
        Block::RawBlock { format, content } => {
            queue.push_back(OwnedEvent::RawBlock {
                format: format.clone(),
                content: content.clone(),
            });
        }
        Block::Div { class, directive, children } => {
            queue.push_back(OwnedEvent::StartDiv {
                class: class.clone(),
                directive: directive.clone(),
            });
            collect_blocks_events(children, queue);
            queue.push_back(OwnedEvent::EndDiv);
        }
        Block::HorizontalRule => {
            queue.push_back(OwnedEvent::HorizontalRule);
        }
        Block::Table { rows } => {
            queue.push_back(OwnedEvent::StartTable);
            for row in rows {
                collect_table_row_events(row, queue);
            }
            queue.push_back(OwnedEvent::EndTable);
        }
        Block::FootnoteDef { label, inlines } => {
            queue.push_back(OwnedEvent::StartFootnoteDef { label: label.clone() });
            collect_inlines_events(inlines, queue);
            queue.push_back(OwnedEvent::EndFootnoteDef);
        }
        Block::MathDisplay { source } => {
            queue.push_back(OwnedEvent::MathDisplay { source: source.clone() });
        }
        Block::Admonition { admonition_type, children } => {
            queue.push_back(OwnedEvent::StartAdmonition { admonition_type: admonition_type.clone() });
            collect_blocks_events(children, queue);
            queue.push_back(OwnedEvent::EndAdmonition);
        }
        Block::LineBlock { lines } => {
            queue.push_back(OwnedEvent::StartLineBlock);
            for line in lines {
                queue.push_back(OwnedEvent::StartLineBlockLine);
                collect_inlines_events(line, queue);
                queue.push_back(OwnedEvent::EndLineBlockLine);
            }
            queue.push_back(OwnedEvent::EndLineBlock);
        }
    }
}

fn collect_definition_item_events(item: &DefinitionItem, queue: &mut VecDeque<OwnedEvent>) {
    queue.push_back(OwnedEvent::StartDefinitionTerm);
    collect_inlines_events(&item.term, queue);
    queue.push_back(OwnedEvent::EndDefinitionTerm);
    queue.push_back(OwnedEvent::StartDefinitionDesc);
    collect_inlines_events(&item.desc, queue);
    queue.push_back(OwnedEvent::EndDefinitionDesc);
}

fn collect_table_row_events(row: &TableRow, queue: &mut VecDeque<OwnedEvent>) {
    queue.push_back(OwnedEvent::StartTableRow { is_header: row.is_header });
    for cell in &row.cells {
        queue.push_back(OwnedEvent::StartTableCell);
        collect_inlines_events(cell, queue);
        queue.push_back(OwnedEvent::EndTableCell);
    }
    queue.push_back(OwnedEvent::EndTableRow);
}

fn collect_inlines_events(inlines: &[Inline], queue: &mut VecDeque<OwnedEvent>) {
    for inline in inlines {
        collect_inline_events(inline, queue);
    }
}

fn collect_inline_events(inline: &Inline, queue: &mut VecDeque<OwnedEvent>) {
    match inline {
        Inline::Text(s) => {
            queue.push_back(OwnedEvent::Text(s.clone()));
        }
        Inline::SoftBreak => {
            queue.push_back(OwnedEvent::SoftBreak);
        }
        Inline::LineBreak => {
            queue.push_back(OwnedEvent::LineBreak);
        }
        Inline::Emphasis(children) => {
            queue.push_back(OwnedEvent::StartEmphasis);
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndEmphasis);
        }
        Inline::Strong(children) => {
            queue.push_back(OwnedEvent::StartStrong);
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndStrong);
        }
        Inline::Strikeout(children) => {
            queue.push_back(OwnedEvent::StartStrikeout);
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndStrikeout);
        }
        Inline::Underline(children) => {
            queue.push_back(OwnedEvent::StartUnderline);
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndUnderline);
        }
        Inline::Subscript(children) => {
            queue.push_back(OwnedEvent::StartSubscript);
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndSubscript);
        }
        Inline::Superscript(children) => {
            queue.push_back(OwnedEvent::StartSuperscript);
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndSuperscript);
        }
        Inline::SmallCaps(children) => {
            queue.push_back(OwnedEvent::StartSmallCaps);
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndSmallCaps);
        }
        Inline::Code(s) => {
            queue.push_back(OwnedEvent::Code(s.clone()));
        }
        Inline::Link { url, children } => {
            queue.push_back(OwnedEvent::StartLink { url: url.clone() });
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndLink);
        }
        Inline::Image { url, alt } => {
            queue.push_back(OwnedEvent::InlineImage { url: url.clone(), alt: alt.clone() });
        }
        Inline::FootnoteRef { label } => {
            queue.push_back(OwnedEvent::FootnoteRef { label: label.clone() });
        }
        Inline::FootnoteDef { label, children } => {
            queue.push_back(OwnedEvent::StartFootnoteDefInline { label: label.clone() });
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndFootnoteDefInline);
        }
        Inline::Quoted { quote_type, children } => {
            queue.push_back(OwnedEvent::StartQuoted { quote_type: quote_type.clone() });
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndQuoted);
        }
        Inline::MathInline { source } => {
            queue.push_back(OwnedEvent::MathInline { source: source.clone() });
        }
        Inline::RstSpan { role, children } => {
            queue.push_back(OwnedEvent::StartRstSpan { role: role.clone() });
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndRstSpan);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_events_heading() {
        let evs: Vec<_> = crate::events("Section\n=======\n").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartHeading { level: 1 })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndHeading)));
    }

    #[test]
    fn test_events_paragraph() {
        let evs: Vec<_> = crate::events("Hello world").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartParagraph)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndParagraph)));
    }

    #[test]
    fn test_events_code_block() {
        let evs: Vec<_> = crate::events(".. code-block:: rust\n\n   let x = 1;\n").collect();
        assert!(evs
            .iter()
            .any(|e| matches!(e, OwnedEvent::StartCodeBlock { language: Some(l) } if l == "rust")));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndCodeBlock)));
    }

    #[test]
    fn test_events_list() {
        let evs: Vec<_> = crate::events("- item one\n- item two\n").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartList { ordered: false })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartListItem)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndList)));
    }

    #[test]
    fn test_parse_equals_events_collect() {
        // parse() must produce the same result as collecting from events().
        let inputs = [
            "Section\n=======\n\nHello world.\n",
            "- item one\n- item two\n",
            ".. code-block:: rust\n\n   let x = 1;\n",
            ".. note::\n\n   Some note.\n",
        ];
        for input in inputs {
            let via_parse = crate::parse(input).expect("parse failed");
            let via_events: Vec<_> = crate::events(input).collect();
            // Verify parse() and events() agree by re-serializing parse() result to events.
            let mut queue = std::collections::VecDeque::new();
            events_from_doc(&via_parse, &mut queue);
            let via_parse_events: Vec<_> = queue.into_iter().collect();
            assert_eq!(
                via_events.len(),
                via_parse_events.len(),
                "event count mismatch for input: {input:?}"
            );
        }
    }
}
