//! Streaming event iterator over a parsed `AsciiDoc`.

use crate::ast::*;
use std::borrow::Cow;
use std::collections::VecDeque;

pub use crate::parse::EventIter;

/// A streaming event from an AsciiDoc document.
///
/// Raw text content fields use `Cow<'a, str>` so that future optimisations can
/// yield borrowed slices of the input without changing the public API.
/// For the common case of fully-owned events (e.g. batch mode) use the
/// [`OwnedEvent`] type alias.
#[derive(Debug)]
pub enum Event<'a> {
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
    CodeBlockContent(Cow<'a, str>),
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
    Text(Cow<'a, str>),
    SoftBreak,
    LineBreak,
    StartStrong,
    EndStrong,
    StartEmphasis,
    EndEmphasis,
    Code(Cow<'a, str>),
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

/// Backwards-compatible alias for batch mode (all text is owned).
pub type OwnedEvent = Event<'static>;

impl<'a> Event<'a> {
    /// Convert to an owned event (all `Cow::Borrowed` text fields become `Cow::Owned`).
    pub fn into_owned(self) -> OwnedEvent {
        match self {
            Event::Text(cow) => Event::Text(Cow::Owned(cow.into_owned())),
            Event::Code(cow) => Event::Code(Cow::Owned(cow.into_owned())),
            Event::CodeBlockContent(cow) => Event::CodeBlockContent(Cow::Owned(cow.into_owned())),
            // Safety: all other variants contain only String fields ('static), so
            // transmuting the lifetime from 'a to 'static is sound.
            other => unsafe { std::mem::transmute::<Event<'a>, Event<'static>>(other) },
        }
    }
}

/// Collect a complete AsciiDoc from an [`EventIter`] used as an event iterator.
/// Called by `parse::parse()` to reconstruct the AST from events.
pub(crate) fn collect_doc_from_iter(
    iter: &mut EventIter<'_>,
) -> (Vec<Block>, std::collections::HashMap<String, String>, Vec<Diagnostic>) {
    let mut block_stack: Vec<BlockFrame> = vec![BlockFrame::Document { blocks: Vec::new() }];
    let mut inline_ctx: Vec<InlineFrame> = Vec::new();

    for event in iter.by_ref() {
        handle_event(event, &mut block_stack, &mut inline_ctx);
    }

    let attributes = iter.take_attributes();
    let diagnostics = std::mem::take(&mut iter.diagnostics);

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

fn handle_event(event: Event<'_>, block_stack: &mut Vec<BlockFrame>, inline_ctx: &mut Vec<InlineFrame>) {
    match event {
        Event::StartDocument | Event::EndDocument => {
            // Document frame already managed by collect_doc_from_iter
        }

        // ── Block start events ─────────────────────────────────────────────
        Event::StartParagraph { id, role, checked } => {
            block_stack.push(BlockFrame::Paragraph { id, role, checked, inlines: Vec::new() });
        }
        Event::StartHeading { level, id, role } => {
            block_stack.push(BlockFrame::Heading { level, id, role, inlines: Vec::new() });
        }
        Event::StartCodeBlock { language } => {
            block_stack.push(BlockFrame::CodeBlock { language, content: String::new() });
        }
        Event::CodeBlockContent(cow) => {
            if let Some(BlockFrame::CodeBlock { content: c, .. }) = block_stack.last_mut() {
                *c = cow.into_owned();
            }
        }
        Event::StartBlockquote { attribution } => {
            block_stack.push(BlockFrame::Blockquote { attribution, children: Vec::new() });
        }
        Event::StartList { ordered, style } => {
            block_stack.push(BlockFrame::List { ordered, style, items: Vec::new() });
        }
        Event::StartListItem => {
            block_stack.push(BlockFrame::ListItem { blocks: Vec::new() });
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
        Event::StartDiv { class, title } => {
            block_stack.push(BlockFrame::Div { class, title, children: Vec::new() });
        }
        Event::StartTable => {
            block_stack.push(BlockFrame::Table { rows: Vec::new() });
        }
        Event::StartTableRow { is_header } => {
            block_stack.push(BlockFrame::TableRow { is_header, cells: Vec::new() });
        }
        Event::StartTableCell => {
            block_stack.push(BlockFrame::TableCell { inlines: Vec::new() });
        }
        Event::StartFootnoteDef { label } => {
            // FootnoteDef is an inline-level frame (appears inside paragraph)
            inline_ctx.push(InlineFrame::FootnoteDef { label, inlines: Vec::new() });
        }

        // ── Block end events ───────────────────────────────────────────────
        Event::EndParagraph => {
            if let Some(BlockFrame::Paragraph { id, role, checked, inlines }) = block_stack.pop() {
                push_block(block_stack, Block::Paragraph { inlines, id, role, checked, span: Span::NONE });
            }
        }
        Event::EndHeading => {
            if let Some(BlockFrame::Heading { level, id, role, inlines }) = block_stack.pop() {
                push_block(block_stack, Block::Heading { level, inlines, id, role, span: Span::NONE });
            }
        }
        Event::EndCodeBlock => {
            if let Some(BlockFrame::CodeBlock { language, content }) = block_stack.pop() {
                push_block(block_stack, Block::CodeBlock { content, language, span: Span::NONE });
            }
        }
        Event::EndBlockquote => {
            if let Some(BlockFrame::Blockquote { attribution, children }) = block_stack.pop() {
                push_block(block_stack, Block::Blockquote { children, attribution, span: Span::NONE });
            }
        }
        Event::EndList => {
            if let Some(BlockFrame::List { ordered, style, items }) = block_stack.pop() {
                push_block(block_stack, Block::List { ordered, items, style, span: Span::NONE });
            }
        }
        Event::EndListItem => {
            if let Some(BlockFrame::ListItem { blocks }) = block_stack.pop() {
                if let Some(BlockFrame::List { items, .. }) = block_stack.last_mut() {
                    items.push(blocks);
                }
            }
        }
        Event::EndDefinitionList => {
            if let Some(BlockFrame::DefinitionList { items }) = block_stack.pop() {
                push_block(block_stack, Block::DefinitionList { items, span: Span::NONE });
            }
        }
        Event::EndDefinitionTerm => {
            if let Some(BlockFrame::DefinitionTerm { inlines }) = block_stack.pop() {
                // Push a pending term into the definition list — we need to hold it
                // until EndDefinitionDesc pairs it. Use a temporary DefinitionItem
                // with empty desc, then complete it on EndDefinitionDesc.
                if let Some(BlockFrame::DefinitionList { items }) = block_stack.last_mut() {
                    items.push(DefinitionItem { term: inlines, desc: Vec::new() });
                }
            }
        }
        Event::EndDefinitionDesc => {
            if let Some(BlockFrame::DefinitionDesc { inlines }) = block_stack.pop() {
                if let Some(BlockFrame::DefinitionList { items }) = block_stack.last_mut() {
                    if let Some(last) = items.last_mut() {
                        last.desc = inlines;
                    }
                }
            }
        }
        Event::EndDiv => {
            if let Some(BlockFrame::Div { class, title, children }) = block_stack.pop() {
                push_block(block_stack, Block::Div { class, title, children, span: Span::NONE });
            }
        }
        Event::EndTable => {
            if let Some(BlockFrame::Table { rows }) = block_stack.pop() {
                push_block(block_stack, Block::Table { rows, span: Span::NONE });
            }
        }
        Event::EndTableRow => {
            if let Some(BlockFrame::TableRow { is_header, cells }) = block_stack.pop() {
                if let Some(BlockFrame::Table { rows }) = block_stack.last_mut() {
                    rows.push(TableRow { cells, is_header });
                }
            }
        }
        Event::EndTableCell => {
            if let Some(BlockFrame::TableCell { inlines }) = block_stack.pop() {
                if let Some(BlockFrame::TableRow { cells, .. }) = block_stack.last_mut() {
                    cells.push(inlines);
                }
            }
        }
        Event::EndFootnoteDef => {
            if let Some(InlineFrame::FootnoteDef { label, inlines }) = inline_ctx.pop() {
                let footnote = Inline::FootnoteDef { label, children: inlines, span: Span::NONE };
                push_inline(block_stack, inline_ctx, footnote);
            }
        }

        // ── Leaf block events ──────────────────────────────────────────────
        Event::HorizontalRule => {
            push_block(block_stack, Block::HorizontalRule { span: Span::NONE });
        }
        Event::PageBreak => {
            push_block(block_stack, Block::PageBreak { span: Span::NONE });
        }
        Event::Figure { url, alt, title } => {
            push_block(block_stack, Block::Figure {
                image: ImageData { url, alt, width: None, height: title },
                span: Span::NONE,
            });
        }
        Event::RawBlock { format, content } => {
            push_block(block_stack, Block::RawBlock { format, content, span: Span::NONE });
        }

        // ── Inline events ──────────────────────────────────────────────────
        Event::Text(cow) => {
            push_inline(block_stack, inline_ctx, Inline::Text { text: cow.into_owned(), span: Span::NONE });
        }
        Event::SoftBreak => {
            push_inline(block_stack, inline_ctx, Inline::SoftBreak { span: Span::NONE });
        }
        Event::LineBreak => {
            push_inline(block_stack, inline_ctx, Inline::LineBreak { span: Span::NONE });
        }
        Event::Code(cow) => {
            push_inline(block_stack, inline_ctx, Inline::Code(cow.into_owned(), Span::NONE));
        }
        Event::InlineImage { url, alt, title } => {
            push_inline(block_stack, inline_ctx, Inline::Image(
                ImageData { url, alt, width: None, height: title },
                Span::NONE,
            ));
        }
        Event::FootnoteRef { label } => {
            push_inline(block_stack, inline_ctx, Inline::FootnoteRef { label, span: Span::NONE });
        }
        Event::MathInline { source } => {
            push_inline(block_stack, inline_ctx, Inline::MathInline { source, span: Span::NONE });
        }
        Event::MathDisplay { source } => {
            push_inline(block_stack, inline_ctx, Inline::MathDisplay { source, span: Span::NONE });
        }
        Event::RawInline { format, content } => {
            push_inline(block_stack, inline_ctx, Inline::RawInline { format, content, span: Span::NONE });
        }
        Event::Anchor { id } => {
            push_inline(block_stack, inline_ctx, Inline::Anchor { id, span: Span::NONE });
        }

        // ── Inline container start events ──────────────────────────────────
        Event::StartStrong => {
            inline_ctx.push(InlineFrame::Strong { inlines: Vec::new() });
        }
        Event::StartEmphasis => {
            inline_ctx.push(InlineFrame::Emphasis { inlines: Vec::new() });
        }
        Event::StartSuperscript => {
            inline_ctx.push(InlineFrame::Superscript { inlines: Vec::new() });
        }
        Event::StartSubscript => {
            inline_ctx.push(InlineFrame::Subscript { inlines: Vec::new() });
        }
        Event::StartHighlight => {
            inline_ctx.push(InlineFrame::Highlight { inlines: Vec::new() });
        }
        Event::StartStrikeout => {
            inline_ctx.push(InlineFrame::Strikeout { inlines: Vec::new() });
        }
        Event::StartUnderline => {
            inline_ctx.push(InlineFrame::Underline { inlines: Vec::new() });
        }
        Event::StartSmallCaps => {
            inline_ctx.push(InlineFrame::SmallCaps { inlines: Vec::new() });
        }
        Event::StartQuoted { quote_type } => {
            let qt = if quote_type == "single" { QuoteType::Single } else { QuoteType::Double };
            inline_ctx.push(InlineFrame::Quoted { quote_type: qt, inlines: Vec::new() });
        }
        Event::StartLink { url, target } => {
            inline_ctx.push(InlineFrame::Link { url, target, inlines: Vec::new() });
        }

        // ── Inline container end events ────────────────────────────────────
        Event::EndStrong => {
            if let Some(InlineFrame::Strong { inlines }) = inline_ctx.pop() {
                push_inline(block_stack, inline_ctx, Inline::Strong(inlines, Span::NONE));
            }
        }
        Event::EndEmphasis => {
            if let Some(InlineFrame::Emphasis { inlines }) = inline_ctx.pop() {
                push_inline(block_stack, inline_ctx, Inline::Emphasis(inlines, Span::NONE));
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
        Event::EndHighlight => {
            if let Some(InlineFrame::Highlight { inlines }) = inline_ctx.pop() {
                push_inline(block_stack, inline_ctx, Inline::Highlight(inlines, Span::NONE));
            }
        }
        Event::EndStrikeout => {
            if let Some(InlineFrame::Strikeout { inlines }) = inline_ctx.pop() {
                push_inline(block_stack, inline_ctx, Inline::Strikeout(inlines, Span::NONE));
            }
        }
        Event::EndUnderline => {
            if let Some(InlineFrame::Underline { inlines }) = inline_ctx.pop() {
                push_inline(block_stack, inline_ctx, Inline::Underline(inlines, Span::NONE));
            }
        }
        Event::EndSmallCaps => {
            if let Some(InlineFrame::SmallCaps { inlines }) = inline_ctx.pop() {
                push_inline(block_stack, inline_ctx, Inline::SmallCaps(inlines, Span::NONE));
            }
        }
        Event::EndQuoted => {
            if let Some(InlineFrame::Quoted { quote_type, inlines }) = inline_ctx.pop() {
                push_inline(block_stack, inline_ctx, Inline::Quoted { quote_type, children: inlines, span: Span::NONE });
            }
        }
        Event::EndLink => {
            if let Some(InlineFrame::Link { url, target, inlines }) = inline_ctx.pop() {
                push_inline(block_stack, inline_ctx, Inline::Link { url, children: inlines, target, span: Span::NONE });
            }
        }
    }
}

pub(crate) fn collect_block_events(block: &Block, queue: &mut VecDeque<Event<'static>>) {
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
            queue.push_back(Event::CodeBlockContent(Cow::Owned(content.clone())));
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

fn collect_blocks_events(blocks: &[Block], queue: &mut VecDeque<Event<'static>>) {
    for block in blocks {
        collect_block_events(block, queue);
    }
}

fn collect_inlines_events(inlines: &[Inline], queue: &mut VecDeque<Event<'static>>) {
    for inline in inlines {
        collect_inline_events(inline, queue);
    }
}

fn collect_inline_events(inline: &Inline, queue: &mut VecDeque<Event<'static>>) {
    match inline {
        Inline::Text { text, .. } => {
            queue.push_back(Event::Text(Cow::Owned(text.clone())));
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
            queue.push_back(Event::Code(Cow::Owned(content.clone())));
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
