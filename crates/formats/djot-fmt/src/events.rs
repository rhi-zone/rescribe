//! Streaming event iterator over a Djot document.
//!
//! `EventIter<'a>` is the concrete struct (defined in `parse.rs`) that
//! implements `Iterator<Item = OwnedEvent>` directly. Events are yielded
//! lazily, one block at a time, via `parse_one_block()`. No full AST is
//! built internally. `parse()` reconstructs a `DjotDoc` from an `EventIter`
//! via a stack-based tree builder in `collect_doc_from_iter()`.

use crate::ast::*;
use std::collections::VecDeque;

// ── Public event types ────────────────────────────────────────────────────────

/// A borrowed streaming event (kept for reference; not used by the iterator).
#[allow(dead_code)]
#[derive(Debug)]
pub enum Event<'a> {
    // Block events
    StartParagraph { attr: &'a Attr },
    EndParagraph,
    StartHeading { level: u8, attr: &'a Attr },
    EndHeading,
    StartBlockquote { attr: &'a Attr },
    EndBlockquote,
    StartList { kind: &'a ListKind, tight: bool, attr: &'a Attr },
    EndList,
    StartListItem { checked: Option<bool> },
    EndListItem,
    StartCodeBlock { language: Option<&'a str>, attr: &'a Attr },
    EndCodeBlock,
    CodeBlockContent(&'a str),
    RawBlock { format: &'a str, content: &'a str },
    StartDiv { class: Option<&'a str>, attr: &'a Attr },
    EndDiv,
    StartTable,
    EndTable,
    StartTableRow { is_header: bool },
    EndTableRow,
    StartTableCell { alignment: Alignment },
    EndTableCell,
    ThematicBreak { attr: &'a Attr },
    StartDefinitionList { attr: &'a Attr },
    EndDefinitionList,
    StartDefinitionTerm,
    EndDefinitionTerm,
    StartDefinitionDesc,
    EndDefinitionDesc,
    StartFootnoteDef { label: &'a str },
    EndFootnoteDef,
    // Inline events
    Text(&'a str),
    SoftBreak,
    HardBreak,
    StartEmphasis { attr: &'a Attr },
    EndEmphasis,
    StartStrong { attr: &'a Attr },
    EndStrong,
    StartDelete { attr: &'a Attr },
    EndDelete,
    StartInsert { attr: &'a Attr },
    EndInsert,
    StartHighlight { attr: &'a Attr },
    EndHighlight,
    StartSubscript { attr: &'a Attr },
    EndSubscript,
    StartSuperscript { attr: &'a Attr },
    EndSuperscript,
    Verbatim { content: &'a str, attr: &'a Attr },
    MathInline(&'a str),
    MathDisplay(&'a str),
    RawInline { format: &'a str, content: &'a str },
    StartLink { url: &'a str, title: Option<&'a str>, attr: &'a Attr },
    EndLink,
    StartImage { url: &'a str, title: Option<&'a str>, attr: &'a Attr },
    EndImage,
    StartSpan { attr: &'a Attr },
    EndSpan,
    FootnoteRef(&'a str),
    Symbol(&'a str),
    Autolink { url: &'a str, is_email: bool },
}

/// Owned event (no lifetime) yielded by `EventIter`.
#[derive(Debug)]
pub enum OwnedEvent {
    StartParagraph { id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)> },
    EndParagraph,
    StartHeading { level: u8, id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)> },
    EndHeading,
    StartBlockquote { id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)> },
    EndBlockquote,
    StartList { kind: ListKind, tight: bool, id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)> },
    EndList,
    StartListItem { checked: Option<bool> },
    EndListItem,
    StartCodeBlock { language: Option<String>, id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)> },
    EndCodeBlock,
    CodeBlockContent(String),
    RawBlock { format: String, content: String },
    StartDiv { class: Option<String>, id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)> },
    EndDiv,
    StartTable,
    EndTable,
    StartTableRow { is_header: bool },
    EndTableRow,
    StartTableCell { alignment: Alignment },
    EndTableCell,
    ThematicBreak { id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)> },
    StartDefinitionList { id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)> },
    EndDefinitionList,
    StartDefinitionTerm,
    EndDefinitionTerm,
    StartDefinitionDesc,
    EndDefinitionDesc,
    StartFootnoteDef { label: String },
    EndFootnoteDef,
    Text(String),
    SoftBreak,
    HardBreak,
    StartEmphasis { id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)> },
    EndEmphasis,
    StartStrong { id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)> },
    EndStrong,
    StartDelete { id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)> },
    EndDelete,
    StartInsert { id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)> },
    EndInsert,
    StartHighlight { id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)> },
    EndHighlight,
    StartSubscript { id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)> },
    EndSubscript,
    StartSuperscript { id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)> },
    EndSuperscript,
    Verbatim { content: String, id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)> },
    MathInline(String),
    MathDisplay(String),
    RawInline { format: String, content: String },
    StartLink { url: String, title: Option<String>, id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)> },
    EndLink,
    StartImage { url: String, title: Option<String>, id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)> },
    EndImage,
    StartSpan { id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)> },
    EndSpan,
    FootnoteRef(String),
    Symbol(String),
    Autolink { url: String, is_email: bool },
}

// ── True pull iterator ────────────────────────────────────────────────────────

pub use crate::parse::EventIter;

// ── Tree builder: reconstruct DjotDoc from EventIter ─────────────────────────

/// Reconstruct a `DjotDoc` by consuming an `EventIter`.
/// Called by `parse::parse()` so that `parse()` = `events().collect()`.
pub(crate) fn collect_doc_from_iter(input: &str) -> (DjotDoc, Vec<Diagnostic>) {
    let mut iter = EventIter::new(input);

    let mut block_stack: Vec<BlockFrame> = vec![BlockFrame::Document {
        blocks: Vec::new(),
        footnotes: Vec::new(),
    }];
    let mut inline_stack: Vec<InlineFrame> = Vec::new();

    for event in iter.by_ref() {
        handle_event(event, &mut block_stack, &mut inline_stack);
    }

    let diagnostics = std::mem::take(&mut iter.diagnostics);
    let link_defs = std::mem::take(&mut iter.link_defs);

    let (blocks, footnotes) = match block_stack.pop() {
        Some(BlockFrame::Document { blocks, footnotes }) => (blocks, footnotes),
        _ => (Vec::new(), Vec::new()),
    };

    let doc = DjotDoc { blocks, footnotes, link_defs };
    (doc, diagnostics)
}

// ── Block frame stack ─────────────────────────────────────────────────────────

enum BlockFrame {
    Document { blocks: Vec<Block>, footnotes: Vec<FootnoteDef> },
    Paragraph { inlines: Vec<Inline>, attr: Attr },
    Heading { level: u8, inlines: Vec<Inline>, attr: Attr },
    Blockquote { blocks: Vec<Block>, attr: Attr },
    List { kind: ListKind, tight: bool, items: Vec<ListItem>, attr: Attr },
    ListItem { blocks: Vec<Block>, checked: Option<bool> },
    CodeBlock { language: Option<String>, content: String, attr: Attr },
    Div { class: Option<String>, blocks: Vec<Block>, attr: Attr },
    Table { rows: Vec<TableRow> },
    TableRow { is_header: bool, cells: Vec<TableCell> },
    TableCell { inlines: Vec<Inline>, alignment: Alignment },
    DefinitionList { items: Vec<DefItem>, attr: Attr },
    DefinitionTerm { inlines: Vec<Inline> },
    DefinitionDesc { blocks: Vec<Block> },
    FootnoteDef { label: String, blocks: Vec<Block> },
}

// ── Inline frame stack ────────────────────────────────────────────────────────

enum InlineFrame {
    Emphasis { inlines: Vec<Inline>, attr: Attr },
    Strong { inlines: Vec<Inline>, attr: Attr },
    Delete { inlines: Vec<Inline>, attr: Attr },
    Insert { inlines: Vec<Inline>, attr: Attr },
    Highlight { inlines: Vec<Inline>, attr: Attr },
    Subscript { inlines: Vec<Inline>, attr: Attr },
    Superscript { inlines: Vec<Inline>, attr: Attr },
    Link { url: String, title: Option<String>, inlines: Vec<Inline>, attr: Attr },
    Image { url: String, title: Option<String>, inlines: Vec<Inline>, attr: Attr },
    Span { inlines: Vec<Inline>, attr: Attr },
}

fn inline_frame_inlines(frame: &mut InlineFrame) -> &mut Vec<Inline> {
    match frame {
        InlineFrame::Emphasis { inlines, .. } => inlines,
        InlineFrame::Strong { inlines, .. } => inlines,
        InlineFrame::Delete { inlines, .. } => inlines,
        InlineFrame::Insert { inlines, .. } => inlines,
        InlineFrame::Highlight { inlines, .. } => inlines,
        InlineFrame::Subscript { inlines, .. } => inlines,
        InlineFrame::Superscript { inlines, .. } => inlines,
        InlineFrame::Link { inlines, .. } => inlines,
        InlineFrame::Image { inlines, .. } => inlines,
        InlineFrame::Span { inlines, .. } => inlines,
    }
}

// ── Event dispatch ────────────────────────────────────────────────────────────

fn make_attr(id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)>) -> Attr {
    Attr { id, classes, kv }
}

fn push_inline_to_ctx(inline: Inline, block_stack: &mut [BlockFrame], inline_stack: &mut [InlineFrame]) {
    if let Some(frame) = inline_stack.last_mut() {
        inline_frame_inlines(frame).push(inline);
        return;
    }
    match block_stack.last_mut() {
        Some(BlockFrame::Paragraph { inlines, .. }) => inlines.push(inline),
        Some(BlockFrame::Heading { inlines, .. }) => inlines.push(inline),
        Some(BlockFrame::DefinitionTerm { inlines }) => inlines.push(inline),
        Some(BlockFrame::TableCell { inlines, .. }) => inlines.push(inline),
        _ => {}
    }
}

fn push_block_to_ctx(block: Block, block_stack: &mut [BlockFrame]) {
    match block_stack.last_mut() {
        Some(BlockFrame::Document { blocks, .. }) => blocks.push(block),
        Some(BlockFrame::Blockquote { blocks, .. }) => blocks.push(block),
        Some(BlockFrame::ListItem { blocks, .. }) => blocks.push(block),
        Some(BlockFrame::Div { blocks, .. }) => blocks.push(block),
        Some(BlockFrame::FootnoteDef { blocks, .. }) => blocks.push(block),
        Some(BlockFrame::DefinitionDesc { blocks }) => blocks.push(block),
        _ => {}
    }
}

fn handle_event(event: OwnedEvent, block_stack: &mut Vec<BlockFrame>, inline_stack: &mut Vec<InlineFrame>) {
    match event {
        // ── Block start events ─────────────────────────────────────────────
        OwnedEvent::StartParagraph { id, classes, kv } => {
            block_stack.push(BlockFrame::Paragraph { inlines: Vec::new(), attr: make_attr(id, classes, kv) });
        }
        OwnedEvent::StartHeading { level, id, classes, kv } => {
            block_stack.push(BlockFrame::Heading { level, inlines: Vec::new(), attr: make_attr(id, classes, kv) });
        }
        OwnedEvent::StartBlockquote { id, classes, kv } => {
            block_stack.push(BlockFrame::Blockquote { blocks: Vec::new(), attr: make_attr(id, classes, kv) });
        }
        OwnedEvent::StartList { kind, tight, id, classes, kv } => {
            block_stack.push(BlockFrame::List { kind, tight, items: Vec::new(), attr: make_attr(id, classes, kv) });
        }
        OwnedEvent::StartListItem { checked } => {
            block_stack.push(BlockFrame::ListItem { blocks: Vec::new(), checked });
        }
        OwnedEvent::StartCodeBlock { language, id, classes, kv } => {
            block_stack.push(BlockFrame::CodeBlock { language, content: String::new(), attr: make_attr(id, classes, kv) });
        }
        OwnedEvent::CodeBlockContent(content) => {
            if let Some(BlockFrame::CodeBlock { content: c, .. }) = block_stack.last_mut() {
                *c = content;
            }
        }
        OwnedEvent::StartDiv { class, id, classes, kv } => {
            block_stack.push(BlockFrame::Div { class, blocks: Vec::new(), attr: make_attr(id, classes, kv) });
        }
        OwnedEvent::StartTable => {
            block_stack.push(BlockFrame::Table { rows: Vec::new() });
        }
        OwnedEvent::StartTableRow { is_header } => {
            block_stack.push(BlockFrame::TableRow { is_header, cells: Vec::new() });
        }
        OwnedEvent::StartTableCell { alignment } => {
            block_stack.push(BlockFrame::TableCell { inlines: Vec::new(), alignment });
        }
        OwnedEvent::StartDefinitionList { id, classes, kv } => {
            block_stack.push(BlockFrame::DefinitionList { items: Vec::new(), attr: make_attr(id, classes, kv) });
        }
        OwnedEvent::StartDefinitionTerm => {
            block_stack.push(BlockFrame::DefinitionTerm { inlines: Vec::new() });
        }
        OwnedEvent::StartDefinitionDesc => {
            block_stack.push(BlockFrame::DefinitionDesc { blocks: Vec::new() });
        }
        OwnedEvent::StartFootnoteDef { label } => {
            block_stack.push(BlockFrame::FootnoteDef { label, blocks: Vec::new() });
        }

        // ── Block end events ───────────────────────────────────────────────
        OwnedEvent::EndParagraph => {
            if let Some(BlockFrame::Paragraph { inlines, attr }) = block_stack.pop() {
                push_block_to_ctx(Block::Paragraph { inlines, attr, span: Span::NONE }, block_stack);
            }
        }
        OwnedEvent::EndHeading => {
            if let Some(BlockFrame::Heading { level, inlines, attr }) = block_stack.pop() {
                push_block_to_ctx(Block::Heading { level, inlines, attr, span: Span::NONE }, block_stack);
            }
        }
        OwnedEvent::EndBlockquote => {
            if let Some(BlockFrame::Blockquote { blocks, attr }) = block_stack.pop() {
                push_block_to_ctx(Block::Blockquote { blocks, attr, span: Span::NONE }, block_stack);
            }
        }
        OwnedEvent::EndList => {
            if let Some(BlockFrame::List { kind, tight, items, attr }) = block_stack.pop() {
                push_block_to_ctx(Block::List { kind, tight, items, attr, span: Span::NONE }, block_stack);
            }
        }
        OwnedEvent::EndListItem => {
            if let Some(BlockFrame::ListItem { blocks, checked }) = block_stack.pop()
                && let Some(BlockFrame::List { items, .. }) = block_stack.last_mut() {
                items.push(ListItem { blocks, checked, span: Span::NONE });
            }
        }
        OwnedEvent::EndCodeBlock => {
            if let Some(BlockFrame::CodeBlock { language, content, attr }) = block_stack.pop() {
                push_block_to_ctx(Block::CodeBlock { language, content, attr, span: Span::NONE }, block_stack);
            }
        }
        OwnedEvent::RawBlock { format, content } => {
            push_block_to_ctx(Block::RawBlock { format, content, attr: Attr::default(), span: Span::NONE }, block_stack);
        }
        OwnedEvent::EndDiv => {
            if let Some(BlockFrame::Div { class, blocks, attr }) = block_stack.pop() {
                push_block_to_ctx(Block::Div { class, blocks, attr, span: Span::NONE }, block_stack);
            }
        }
        OwnedEvent::EndTable => {
            if let Some(BlockFrame::Table { rows }) = block_stack.pop() {
                push_block_to_ctx(Block::Table { caption: None, rows, span: Span::NONE }, block_stack);
            }
        }
        OwnedEvent::EndTableRow => {
            if let Some(BlockFrame::TableRow { is_header, cells }) = block_stack.pop()
                && let Some(BlockFrame::Table { rows }) = block_stack.last_mut() {
                rows.push(TableRow { cells, is_header, span: Span::NONE });
            }
        }
        OwnedEvent::EndTableCell => {
            if let Some(BlockFrame::TableCell { inlines, alignment }) = block_stack.pop()
                && let Some(BlockFrame::TableRow { cells, .. }) = block_stack.last_mut() {
                cells.push(TableCell { inlines, alignment, span: Span::NONE });
            }
        }
        OwnedEvent::EndDefinitionList => {
            if let Some(BlockFrame::DefinitionList { items, attr }) = block_stack.pop() {
                push_block_to_ctx(Block::DefinitionList { items, attr, span: Span::NONE }, block_stack);
            }
        }
        OwnedEvent::EndDefinitionTerm => {
            if let Some(BlockFrame::DefinitionTerm { inlines }) = block_stack.pop()
                && let Some(BlockFrame::DefinitionList { items, .. }) = block_stack.last_mut() {
                items.push(DefItem { term: inlines, definitions: Vec::new(), span: Span::NONE });
            }
        }
        OwnedEvent::EndDefinitionDesc => {
            if let Some(BlockFrame::DefinitionDesc { blocks }) = block_stack.pop()
                && let Some(BlockFrame::DefinitionList { items, .. }) = block_stack.last_mut()
                && let Some(last) = items.last_mut() {
                last.definitions = blocks;
            }
        }
        OwnedEvent::EndFootnoteDef => {
            if let Some(BlockFrame::FootnoteDef { label, blocks }) = block_stack.pop()
                && let Some(BlockFrame::Document { footnotes, .. }) = block_stack.last_mut() {
                footnotes.push(FootnoteDef { label, blocks, span: Span::NONE });
            }
        }
        OwnedEvent::ThematicBreak { id, classes, kv } => {
            push_block_to_ctx(Block::ThematicBreak { attr: make_attr(id, classes, kv), span: Span::NONE }, block_stack);
        }

        // ── Inline leaf events ─────────────────────────────────────────────
        OwnedEvent::Text(content) => {
            push_inline_to_ctx(Inline::Text { content, span: Span::NONE }, block_stack, inline_stack);
        }
        OwnedEvent::SoftBreak => {
            push_inline_to_ctx(Inline::SoftBreak { span: Span::NONE }, block_stack, inline_stack);
        }
        OwnedEvent::HardBreak => {
            push_inline_to_ctx(Inline::HardBreak { span: Span::NONE }, block_stack, inline_stack);
        }
        OwnedEvent::Verbatim { content, id, classes, kv } => {
            push_inline_to_ctx(Inline::Verbatim { content, attr: make_attr(id, classes, kv), span: Span::NONE }, block_stack, inline_stack);
        }
        OwnedEvent::MathInline(content) => {
            push_inline_to_ctx(Inline::MathInline { content, span: Span::NONE }, block_stack, inline_stack);
        }
        OwnedEvent::MathDisplay(content) => {
            push_inline_to_ctx(Inline::MathDisplay { content, span: Span::NONE }, block_stack, inline_stack);
        }
        OwnedEvent::RawInline { format, content } => {
            push_inline_to_ctx(Inline::RawInline { format, content, span: Span::NONE }, block_stack, inline_stack);
        }
        OwnedEvent::FootnoteRef(label) => {
            push_inline_to_ctx(Inline::FootnoteRef { label, span: Span::NONE }, block_stack, inline_stack);
        }
        OwnedEvent::Symbol(name) => {
            push_inline_to_ctx(Inline::Symbol { name, span: Span::NONE }, block_stack, inline_stack);
        }
        OwnedEvent::Autolink { url, is_email } => {
            push_inline_to_ctx(Inline::Autolink { url, is_email, span: Span::NONE }, block_stack, inline_stack);
        }

        // ── Inline container start events ──────────────────────────────────
        OwnedEvent::StartEmphasis { id, classes, kv } => {
            inline_stack.push(InlineFrame::Emphasis { inlines: Vec::new(), attr: make_attr(id, classes, kv) });
        }
        OwnedEvent::StartStrong { id, classes, kv } => {
            inline_stack.push(InlineFrame::Strong { inlines: Vec::new(), attr: make_attr(id, classes, kv) });
        }
        OwnedEvent::StartDelete { id, classes, kv } => {
            inline_stack.push(InlineFrame::Delete { inlines: Vec::new(), attr: make_attr(id, classes, kv) });
        }
        OwnedEvent::StartInsert { id, classes, kv } => {
            inline_stack.push(InlineFrame::Insert { inlines: Vec::new(), attr: make_attr(id, classes, kv) });
        }
        OwnedEvent::StartHighlight { id, classes, kv } => {
            inline_stack.push(InlineFrame::Highlight { inlines: Vec::new(), attr: make_attr(id, classes, kv) });
        }
        OwnedEvent::StartSubscript { id, classes, kv } => {
            inline_stack.push(InlineFrame::Subscript { inlines: Vec::new(), attr: make_attr(id, classes, kv) });
        }
        OwnedEvent::StartSuperscript { id, classes, kv } => {
            inline_stack.push(InlineFrame::Superscript { inlines: Vec::new(), attr: make_attr(id, classes, kv) });
        }
        OwnedEvent::StartLink { url, title, id, classes, kv } => {
            inline_stack.push(InlineFrame::Link { url, title, inlines: Vec::new(), attr: make_attr(id, classes, kv) });
        }
        OwnedEvent::StartImage { url, title, id, classes, kv } => {
            inline_stack.push(InlineFrame::Image { url, title, inlines: Vec::new(), attr: make_attr(id, classes, kv) });
        }
        OwnedEvent::StartSpan { id, classes, kv } => {
            inline_stack.push(InlineFrame::Span { inlines: Vec::new(), attr: make_attr(id, classes, kv) });
        }

        // ── Inline container end events ────────────────────────────────────
        OwnedEvent::EndEmphasis => {
            if let Some(InlineFrame::Emphasis { inlines, attr }) = inline_stack.pop() {
                push_inline_to_ctx(Inline::Emphasis { inlines, attr, span: Span::NONE }, block_stack, inline_stack);
            }
        }
        OwnedEvent::EndStrong => {
            if let Some(InlineFrame::Strong { inlines, attr }) = inline_stack.pop() {
                push_inline_to_ctx(Inline::Strong { inlines, attr, span: Span::NONE }, block_stack, inline_stack);
            }
        }
        OwnedEvent::EndDelete => {
            if let Some(InlineFrame::Delete { inlines, attr }) = inline_stack.pop() {
                push_inline_to_ctx(Inline::Delete { inlines, attr, span: Span::NONE }, block_stack, inline_stack);
            }
        }
        OwnedEvent::EndInsert => {
            if let Some(InlineFrame::Insert { inlines, attr }) = inline_stack.pop() {
                push_inline_to_ctx(Inline::Insert { inlines, attr, span: Span::NONE }, block_stack, inline_stack);
            }
        }
        OwnedEvent::EndHighlight => {
            if let Some(InlineFrame::Highlight { inlines, attr }) = inline_stack.pop() {
                push_inline_to_ctx(Inline::Highlight { inlines, attr, span: Span::NONE }, block_stack, inline_stack);
            }
        }
        OwnedEvent::EndSubscript => {
            if let Some(InlineFrame::Subscript { inlines, attr }) = inline_stack.pop() {
                push_inline_to_ctx(Inline::Subscript { inlines, attr, span: Span::NONE }, block_stack, inline_stack);
            }
        }
        OwnedEvent::EndSuperscript => {
            if let Some(InlineFrame::Superscript { inlines, attr }) = inline_stack.pop() {
                push_inline_to_ctx(Inline::Superscript { inlines, attr, span: Span::NONE }, block_stack, inline_stack);
            }
        }
        OwnedEvent::EndLink => {
            if let Some(InlineFrame::Link { url, title, inlines, attr }) = inline_stack.pop() {
                push_inline_to_ctx(Inline::Link { url, title, inlines, attr, span: Span::NONE }, block_stack, inline_stack);
            }
        }
        OwnedEvent::EndImage => {
            if let Some(InlineFrame::Image { url, title, inlines, attr }) = inline_stack.pop() {
                push_inline_to_ctx(Inline::Image { url, title, inlines, attr, span: Span::NONE }, block_stack, inline_stack);
            }
        }
        OwnedEvent::EndSpan => {
            if let Some(InlineFrame::Span { inlines, attr }) = inline_stack.pop() {
                push_inline_to_ctx(Inline::Span { inlines, attr, span: Span::NONE }, block_stack, inline_stack);
            }
        }
    }
}

// ── Block-to-events serializer ────────────────────────────────────────────────

pub(crate) fn collect_block_events(block: &Block, queue: &mut VecDeque<OwnedEvent>) {
    match block {
        Block::Paragraph { inlines, attr, .. } => {
            queue.push_back(OwnedEvent::StartParagraph {
                id: attr.id.clone(),
                classes: attr.classes.clone(),
                kv: attr.kv.clone(),
            });
            collect_inlines_events(inlines, queue);
            queue.push_back(OwnedEvent::EndParagraph);
        }
        Block::Heading { level, inlines, attr, .. } => {
            queue.push_back(OwnedEvent::StartHeading {
                level: *level,
                id: attr.id.clone(),
                classes: attr.classes.clone(),
                kv: attr.kv.clone(),
            });
            collect_inlines_events(inlines, queue);
            queue.push_back(OwnedEvent::EndHeading);
        }
        Block::Blockquote { blocks, attr, .. } => {
            queue.push_back(OwnedEvent::StartBlockquote {
                id: attr.id.clone(),
                classes: attr.classes.clone(),
                kv: attr.kv.clone(),
            });
            collect_blocks_events(blocks, queue);
            queue.push_back(OwnedEvent::EndBlockquote);
        }
        Block::List { kind, items, tight, attr, .. } => {
            queue.push_back(OwnedEvent::StartList {
                kind: kind.clone(),
                tight: *tight,
                id: attr.id.clone(),
                classes: attr.classes.clone(),
                kv: attr.kv.clone(),
            });
            for item in items {
                queue.push_back(OwnedEvent::StartListItem { checked: item.checked });
                collect_blocks_events(&item.blocks, queue);
                queue.push_back(OwnedEvent::EndListItem);
            }
            queue.push_back(OwnedEvent::EndList);
        }
        Block::CodeBlock { language, content, attr, .. } => {
            queue.push_back(OwnedEvent::StartCodeBlock {
                language: language.clone(),
                id: attr.id.clone(),
                classes: attr.classes.clone(),
                kv: attr.kv.clone(),
            });
            queue.push_back(OwnedEvent::CodeBlockContent(content.clone()));
            queue.push_back(OwnedEvent::EndCodeBlock);
        }
        Block::RawBlock { format, content, .. } => {
            queue.push_back(OwnedEvent::RawBlock {
                format: format.clone(),
                content: content.clone(),
            });
        }
        Block::Div { class, blocks, attr, .. } => {
            queue.push_back(OwnedEvent::StartDiv {
                class: class.clone(),
                id: attr.id.clone(),
                classes: attr.classes.clone(),
                kv: attr.kv.clone(),
            });
            collect_blocks_events(blocks, queue);
            queue.push_back(OwnedEvent::EndDiv);
        }
        Block::Table { caption: _, rows, .. } => {
            queue.push_back(OwnedEvent::StartTable);
            for row in rows {
                queue.push_back(OwnedEvent::StartTableRow { is_header: row.is_header });
                for cell in &row.cells {
                    queue.push_back(OwnedEvent::StartTableCell { alignment: cell.alignment.clone() });
                    collect_inlines_events(&cell.inlines, queue);
                    queue.push_back(OwnedEvent::EndTableCell);
                }
                queue.push_back(OwnedEvent::EndTableRow);
            }
            queue.push_back(OwnedEvent::EndTable);
        }
        Block::ThematicBreak { attr, .. } => {
            queue.push_back(OwnedEvent::ThematicBreak {
                id: attr.id.clone(),
                classes: attr.classes.clone(),
                kv: attr.kv.clone(),
            });
        }
        Block::DefinitionList { items, attr, .. } => {
            queue.push_back(OwnedEvent::StartDefinitionList {
                id: attr.id.clone(),
                classes: attr.classes.clone(),
                kv: attr.kv.clone(),
            });
            for item in items {
                queue.push_back(OwnedEvent::StartDefinitionTerm);
                collect_inlines_events(&item.term, queue);
                queue.push_back(OwnedEvent::EndDefinitionTerm);
                queue.push_back(OwnedEvent::StartDefinitionDesc);
                collect_blocks_events(&item.definitions, queue);
                queue.push_back(OwnedEvent::EndDefinitionDesc);
            }
            queue.push_back(OwnedEvent::EndDefinitionList);
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
        collect_inline_event(inline, queue);
    }
}

fn collect_inline_event(inline: &Inline, queue: &mut VecDeque<OwnedEvent>) {
    match inline {
        Inline::Text { content, .. } => {
            queue.push_back(OwnedEvent::Text(content.clone()));
        }
        Inline::SoftBreak { .. } => {
            queue.push_back(OwnedEvent::SoftBreak);
        }
        Inline::HardBreak { .. } => {
            queue.push_back(OwnedEvent::HardBreak);
        }
        Inline::Emphasis { inlines, attr, .. } => {
            queue.push_back(OwnedEvent::StartEmphasis {
                id: attr.id.clone(),
                classes: attr.classes.clone(),
                kv: attr.kv.clone(),
            });
            collect_inlines_events(inlines, queue);
            queue.push_back(OwnedEvent::EndEmphasis);
        }
        Inline::Strong { inlines, attr, .. } => {
            queue.push_back(OwnedEvent::StartStrong {
                id: attr.id.clone(),
                classes: attr.classes.clone(),
                kv: attr.kv.clone(),
            });
            collect_inlines_events(inlines, queue);
            queue.push_back(OwnedEvent::EndStrong);
        }
        Inline::Delete { inlines, attr, .. } => {
            queue.push_back(OwnedEvent::StartDelete {
                id: attr.id.clone(),
                classes: attr.classes.clone(),
                kv: attr.kv.clone(),
            });
            collect_inlines_events(inlines, queue);
            queue.push_back(OwnedEvent::EndDelete);
        }
        Inline::Insert { inlines, attr, .. } => {
            queue.push_back(OwnedEvent::StartInsert {
                id: attr.id.clone(),
                classes: attr.classes.clone(),
                kv: attr.kv.clone(),
            });
            collect_inlines_events(inlines, queue);
            queue.push_back(OwnedEvent::EndInsert);
        }
        Inline::Highlight { inlines, attr, .. } => {
            queue.push_back(OwnedEvent::StartHighlight {
                id: attr.id.clone(),
                classes: attr.classes.clone(),
                kv: attr.kv.clone(),
            });
            collect_inlines_events(inlines, queue);
            queue.push_back(OwnedEvent::EndHighlight);
        }
        Inline::Subscript { inlines, attr, .. } => {
            queue.push_back(OwnedEvent::StartSubscript {
                id: attr.id.clone(),
                classes: attr.classes.clone(),
                kv: attr.kv.clone(),
            });
            collect_inlines_events(inlines, queue);
            queue.push_back(OwnedEvent::EndSubscript);
        }
        Inline::Superscript { inlines, attr, .. } => {
            queue.push_back(OwnedEvent::StartSuperscript {
                id: attr.id.clone(),
                classes: attr.classes.clone(),
                kv: attr.kv.clone(),
            });
            collect_inlines_events(inlines, queue);
            queue.push_back(OwnedEvent::EndSuperscript);
        }
        Inline::Verbatim { content, attr, .. } => {
            queue.push_back(OwnedEvent::Verbatim {
                content: content.clone(),
                id: attr.id.clone(),
                classes: attr.classes.clone(),
                kv: attr.kv.clone(),
            });
        }
        Inline::MathInline { content, .. } => {
            queue.push_back(OwnedEvent::MathInline(content.clone()));
        }
        Inline::MathDisplay { content, .. } => {
            queue.push_back(OwnedEvent::MathDisplay(content.clone()));
        }
        Inline::RawInline { format, content, .. } => {
            queue.push_back(OwnedEvent::RawInline {
                format: format.clone(),
                content: content.clone(),
            });
        }
        Inline::Link { inlines, url, title, attr, .. } => {
            queue.push_back(OwnedEvent::StartLink {
                url: url.clone(),
                title: title.clone(),
                id: attr.id.clone(),
                classes: attr.classes.clone(),
                kv: attr.kv.clone(),
            });
            collect_inlines_events(inlines, queue);
            queue.push_back(OwnedEvent::EndLink);
        }
        Inline::Image { inlines, url, title, attr, .. } => {
            queue.push_back(OwnedEvent::StartImage {
                url: url.clone(),
                title: title.clone(),
                id: attr.id.clone(),
                classes: attr.classes.clone(),
                kv: attr.kv.clone(),
            });
            collect_inlines_events(inlines, queue);
            queue.push_back(OwnedEvent::EndImage);
        }
        Inline::Span { inlines, attr, .. } => {
            queue.push_back(OwnedEvent::StartSpan {
                id: attr.id.clone(),
                classes: attr.classes.clone(),
                kv: attr.kv.clone(),
            });
            collect_inlines_events(inlines, queue);
            queue.push_back(OwnedEvent::EndSpan);
        }
        Inline::FootnoteRef { label, .. } => {
            queue.push_back(OwnedEvent::FootnoteRef(label.clone()));
        }
        Inline::Symbol { name, .. } => {
            queue.push_back(OwnedEvent::Symbol(name.clone()));
        }
        Inline::Autolink { url, is_email, .. } => {
            queue.push_back(OwnedEvent::Autolink { url: url.clone(), is_email: *is_email });
        }
    }
}

// Re-export OwnedEvent as the public event type.
pub use OwnedEvent as EventOwned;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_events_heading() {
        let evs: Vec<_> = EventIter::new("# Hello").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartHeading { level: 1, .. })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndHeading)));
    }

    #[test]
    fn test_events_paragraph() {
        let evs: Vec<_> = EventIter::new("Hello world").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartParagraph { .. })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndParagraph)));
    }

    #[test]
    fn test_events_code_block() {
        let evs: Vec<_> = EventIter::new("```rust\ncode\n```").collect();
        assert!(evs
            .iter()
            .any(|e| matches!(e, OwnedEvent::StartCodeBlock { language: Some(l), .. } if l == "rust")));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndCodeBlock)));
    }
}
