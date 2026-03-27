//! Streaming event iterator over a Djot document.
//!
//! `EventIter<'a>` is the concrete struct (defined in `parse.rs`) that
//! implements `Iterator<Item = Event<'a>>` directly. Events are yielded
//! lazily via `push_next_block_frames()` which pushes frames onto a stack
//! without intermediate Block allocation for compound blocks. Compound blocks
//! (blockquote, list items, div) use `Frame::SubParser` to lazily parse inner
//! content. `collect_doc_from_iter()` provides a stack-based tree builder for
//! callers that want to reconstruct a `DjotDoc` from an `EventIter`.

use crate::ast::*;
use std::borrow::Cow;

// ── Public event types ────────────────────────────────────────────────────────

/// A streaming event from a Djot document.
///
/// Raw text content fields use `Cow<'a, str>` so that future optimisations can
/// yield borrowed slices of the input without changing the public API.
/// For the common case of fully-owned events (e.g. batch mode) use the
/// [`OwnedEvent`] type alias.
#[derive(Debug)]
pub enum Event<'a> {
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
    CodeBlockContent(Cow<'a, str>),
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
    Text(Cow<'a, str>),
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
    Verbatim { content: Cow<'a, str>, id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)> },
    MathInline(Cow<'a, str>),
    MathDisplay(Cow<'a, str>),
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

/// Backwards-compatible alias for batch mode (all text is owned).
pub type OwnedEvent = Event<'static>;

impl<'a> Event<'a> {
    /// Convert to an owned event (all `Cow::Borrowed` text fields become `Cow::Owned`).
    pub fn into_owned(self) -> OwnedEvent {
        match self {
            Event::Text(cow) => Event::Text(Cow::Owned(cow.into_owned())),
            Event::CodeBlockContent(cow) => Event::CodeBlockContent(Cow::Owned(cow.into_owned())),
            Event::Verbatim { content, id, classes, kv } => Event::Verbatim {
                content: Cow::Owned(content.into_owned()),
                id,
                classes,
                kv,
            },
            Event::MathInline(cow) => Event::MathInline(Cow::Owned(cow.into_owned())),
            Event::MathDisplay(cow) => Event::MathDisplay(Cow::Owned(cow.into_owned())),
            // Safety: all other variants contain only String/'static fields.
            other => unsafe { std::mem::transmute::<Event<'a>, Event<'static>>(other) },
        }
    }
}

// ── True pull iterator ────────────────────────────────────────────────────────

pub use crate::parse::EventIter;

// ── Tree builder: reconstruct DjotDoc from EventIter ─────────────────────────

/// Reconstruct a `Vec<Block>` from any `OwnedEvent` iterator.
///
/// Used by the direct parse path to drain a `SubParser` into AST nodes for
/// compound block content (blockquote, list items, div, definition descs).
pub(crate) fn collect_blocks_from_event_iter<I: Iterator<Item = OwnedEvent>>(iter: I) -> Vec<Block> {
    let mut block_stack: Vec<BlockFrame> = vec![BlockFrame::Document {
        blocks: Vec::new(),
        footnotes: Vec::new(),
    }];
    let mut inline_stack: Vec<InlineFrame> = Vec::new();

    for event in iter {
        handle_event(event, &mut block_stack, &mut inline_stack);
    }

    match block_stack.pop() {
        Some(BlockFrame::Document { blocks, .. }) => blocks,
        _ => Vec::new(),
    }
}

/// Reconstruct a `DjotDoc` by consuming an `EventIter`.
///
/// Useful for callers that drive `EventIter` as an iterator and want to
/// reconstruct a `DjotDoc` from the event stream. Not used by `parse()`,
/// which takes the direct recursive descent path.
#[allow(dead_code)]
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

fn handle_event(event: Event<'_>, block_stack: &mut Vec<BlockFrame>, inline_stack: &mut Vec<InlineFrame>) {
    match event {
        // ── Block start events ─────────────────────────────────────────────
        Event::StartParagraph { id, classes, kv } => {
            block_stack.push(BlockFrame::Paragraph { inlines: Vec::new(), attr: make_attr(id, classes, kv) });
        }
        Event::StartHeading { level, id, classes, kv } => {
            block_stack.push(BlockFrame::Heading { level, inlines: Vec::new(), attr: make_attr(id, classes, kv) });
        }
        Event::StartBlockquote { id, classes, kv } => {
            block_stack.push(BlockFrame::Blockquote { blocks: Vec::new(), attr: make_attr(id, classes, kv) });
        }
        Event::StartList { kind, tight, id, classes, kv } => {
            block_stack.push(BlockFrame::List { kind, tight, items: Vec::new(), attr: make_attr(id, classes, kv) });
        }
        Event::StartListItem { checked } => {
            block_stack.push(BlockFrame::ListItem { blocks: Vec::new(), checked });
        }
        Event::StartCodeBlock { language, id, classes, kv } => {
            block_stack.push(BlockFrame::CodeBlock { language, content: String::new(), attr: make_attr(id, classes, kv) });
        }
        Event::CodeBlockContent(cow) => {
            if let Some(BlockFrame::CodeBlock { content: c, .. }) = block_stack.last_mut() {
                *c = cow.into_owned();
            }
        }
        Event::StartDiv { class, id, classes, kv } => {
            block_stack.push(BlockFrame::Div { class, blocks: Vec::new(), attr: make_attr(id, classes, kv) });
        }
        Event::StartTable => {
            block_stack.push(BlockFrame::Table { rows: Vec::new() });
        }
        Event::StartTableRow { is_header } => {
            block_stack.push(BlockFrame::TableRow { is_header, cells: Vec::new() });
        }
        Event::StartTableCell { alignment } => {
            block_stack.push(BlockFrame::TableCell { inlines: Vec::new(), alignment });
        }
        Event::StartDefinitionList { id, classes, kv } => {
            block_stack.push(BlockFrame::DefinitionList { items: Vec::new(), attr: make_attr(id, classes, kv) });
        }
        Event::StartDefinitionTerm => {
            block_stack.push(BlockFrame::DefinitionTerm { inlines: Vec::new() });
        }
        Event::StartDefinitionDesc => {
            block_stack.push(BlockFrame::DefinitionDesc { blocks: Vec::new() });
        }
        Event::StartFootnoteDef { label } => {
            block_stack.push(BlockFrame::FootnoteDef { label, blocks: Vec::new() });
        }

        // ── Block end events ───────────────────────────────────────────────
        Event::EndParagraph => {
            if let Some(BlockFrame::Paragraph { inlines, attr }) = block_stack.pop() {
                push_block_to_ctx(Block::Paragraph { inlines, attr, span: Span::NONE }, block_stack);
            }
        }
        Event::EndHeading => {
            if let Some(BlockFrame::Heading { level, inlines, attr }) = block_stack.pop() {
                push_block_to_ctx(Block::Heading { level, inlines, attr, span: Span::NONE }, block_stack);
            }
        }
        Event::EndBlockquote => {
            if let Some(BlockFrame::Blockquote { blocks, attr }) = block_stack.pop() {
                push_block_to_ctx(Block::Blockquote { blocks, attr, span: Span::NONE }, block_stack);
            }
        }
        Event::EndList => {
            if let Some(BlockFrame::List { kind, tight, items, attr }) = block_stack.pop() {
                push_block_to_ctx(Block::List { kind, tight, items, attr, span: Span::NONE }, block_stack);
            }
        }
        Event::EndListItem => {
            if let Some(BlockFrame::ListItem { blocks, checked }) = block_stack.pop()
                && let Some(BlockFrame::List { items, .. }) = block_stack.last_mut() {
                items.push(ListItem { blocks, checked, span: Span::NONE });
            }
        }
        Event::EndCodeBlock => {
            if let Some(BlockFrame::CodeBlock { language, content, attr }) = block_stack.pop() {
                push_block_to_ctx(Block::CodeBlock { language, content, attr, span: Span::NONE }, block_stack);
            }
        }
        Event::RawBlock { format, content } => {
            push_block_to_ctx(Block::RawBlock { format, content, attr: Attr::default(), span: Span::NONE }, block_stack);
        }
        Event::EndDiv => {
            if let Some(BlockFrame::Div { class, blocks, attr }) = block_stack.pop() {
                push_block_to_ctx(Block::Div { class, blocks, attr, span: Span::NONE }, block_stack);
            }
        }
        Event::EndTable => {
            if let Some(BlockFrame::Table { rows }) = block_stack.pop() {
                push_block_to_ctx(Block::Table { caption: None, rows, span: Span::NONE }, block_stack);
            }
        }
        Event::EndTableRow => {
            if let Some(BlockFrame::TableRow { is_header, cells }) = block_stack.pop()
                && let Some(BlockFrame::Table { rows }) = block_stack.last_mut() {
                rows.push(TableRow { cells, is_header, span: Span::NONE });
            }
        }
        Event::EndTableCell => {
            if let Some(BlockFrame::TableCell { inlines, alignment }) = block_stack.pop()
                && let Some(BlockFrame::TableRow { cells, .. }) = block_stack.last_mut() {
                cells.push(TableCell { inlines, alignment, span: Span::NONE });
            }
        }
        Event::EndDefinitionList => {
            if let Some(BlockFrame::DefinitionList { items, attr }) = block_stack.pop() {
                push_block_to_ctx(Block::DefinitionList { items, attr, span: Span::NONE }, block_stack);
            }
        }
        Event::EndDefinitionTerm => {
            if let Some(BlockFrame::DefinitionTerm { inlines }) = block_stack.pop()
                && let Some(BlockFrame::DefinitionList { items, .. }) = block_stack.last_mut() {
                items.push(DefItem { term: inlines, definitions: Vec::new(), span: Span::NONE });
            }
        }
        Event::EndDefinitionDesc => {
            if let Some(BlockFrame::DefinitionDesc { blocks }) = block_stack.pop()
                && let Some(BlockFrame::DefinitionList { items, .. }) = block_stack.last_mut()
                && let Some(last) = items.last_mut() {
                last.definitions = blocks;
            }
        }
        Event::EndFootnoteDef => {
            if let Some(BlockFrame::FootnoteDef { label, blocks }) = block_stack.pop()
                && let Some(BlockFrame::Document { footnotes, .. }) = block_stack.last_mut() {
                footnotes.push(FootnoteDef { label, blocks, span: Span::NONE });
            }
        }
        Event::ThematicBreak { id, classes, kv } => {
            push_block_to_ctx(Block::ThematicBreak { attr: make_attr(id, classes, kv), span: Span::NONE }, block_stack);
        }

        // ── Inline leaf events ─────────────────────────────────────────────
        Event::Text(cow) => {
            push_inline_to_ctx(Inline::Text { content: cow.into_owned(), span: Span::NONE }, block_stack, inline_stack);
        }
        Event::SoftBreak => {
            push_inline_to_ctx(Inline::SoftBreak { span: Span::NONE }, block_stack, inline_stack);
        }
        Event::HardBreak => {
            push_inline_to_ctx(Inline::HardBreak { span: Span::NONE }, block_stack, inline_stack);
        }
        Event::Verbatim { content, id, classes, kv } => {
            push_inline_to_ctx(Inline::Verbatim { content: content.into_owned(), attr: make_attr(id, classes, kv), span: Span::NONE }, block_stack, inline_stack);
        }
        Event::MathInline(cow) => {
            push_inline_to_ctx(Inline::MathInline { content: cow.into_owned(), span: Span::NONE }, block_stack, inline_stack);
        }
        Event::MathDisplay(cow) => {
            push_inline_to_ctx(Inline::MathDisplay { content: cow.into_owned(), span: Span::NONE }, block_stack, inline_stack);
        }
        Event::RawInline { format, content } => {
            push_inline_to_ctx(Inline::RawInline { format, content, span: Span::NONE }, block_stack, inline_stack);
        }
        Event::FootnoteRef(label) => {
            push_inline_to_ctx(Inline::FootnoteRef { label, span: Span::NONE }, block_stack, inline_stack);
        }
        Event::Symbol(name) => {
            push_inline_to_ctx(Inline::Symbol { name, span: Span::NONE }, block_stack, inline_stack);
        }
        Event::Autolink { url, is_email } => {
            push_inline_to_ctx(Inline::Autolink { url, is_email, span: Span::NONE }, block_stack, inline_stack);
        }

        // ── Inline container start events ──────────────────────────────────
        Event::StartEmphasis { id, classes, kv } => {
            inline_stack.push(InlineFrame::Emphasis { inlines: Vec::new(), attr: make_attr(id, classes, kv) });
        }
        Event::StartStrong { id, classes, kv } => {
            inline_stack.push(InlineFrame::Strong { inlines: Vec::new(), attr: make_attr(id, classes, kv) });
        }
        Event::StartDelete { id, classes, kv } => {
            inline_stack.push(InlineFrame::Delete { inlines: Vec::new(), attr: make_attr(id, classes, kv) });
        }
        Event::StartInsert { id, classes, kv } => {
            inline_stack.push(InlineFrame::Insert { inlines: Vec::new(), attr: make_attr(id, classes, kv) });
        }
        Event::StartHighlight { id, classes, kv } => {
            inline_stack.push(InlineFrame::Highlight { inlines: Vec::new(), attr: make_attr(id, classes, kv) });
        }
        Event::StartSubscript { id, classes, kv } => {
            inline_stack.push(InlineFrame::Subscript { inlines: Vec::new(), attr: make_attr(id, classes, kv) });
        }
        Event::StartSuperscript { id, classes, kv } => {
            inline_stack.push(InlineFrame::Superscript { inlines: Vec::new(), attr: make_attr(id, classes, kv) });
        }
        Event::StartLink { url, title, id, classes, kv } => {
            inline_stack.push(InlineFrame::Link { url, title, inlines: Vec::new(), attr: make_attr(id, classes, kv) });
        }
        Event::StartImage { url, title, id, classes, kv } => {
            inline_stack.push(InlineFrame::Image { url, title, inlines: Vec::new(), attr: make_attr(id, classes, kv) });
        }
        Event::StartSpan { id, classes, kv } => {
            inline_stack.push(InlineFrame::Span { inlines: Vec::new(), attr: make_attr(id, classes, kv) });
        }

        // ── Inline container end events ────────────────────────────────────
        Event::EndEmphasis => {
            if let Some(InlineFrame::Emphasis { inlines, attr }) = inline_stack.pop() {
                push_inline_to_ctx(Inline::Emphasis { inlines, attr, span: Span::NONE }, block_stack, inline_stack);
            }
        }
        Event::EndStrong => {
            if let Some(InlineFrame::Strong { inlines, attr }) = inline_stack.pop() {
                push_inline_to_ctx(Inline::Strong { inlines, attr, span: Span::NONE }, block_stack, inline_stack);
            }
        }
        Event::EndDelete => {
            if let Some(InlineFrame::Delete { inlines, attr }) = inline_stack.pop() {
                push_inline_to_ctx(Inline::Delete { inlines, attr, span: Span::NONE }, block_stack, inline_stack);
            }
        }
        Event::EndInsert => {
            if let Some(InlineFrame::Insert { inlines, attr }) = inline_stack.pop() {
                push_inline_to_ctx(Inline::Insert { inlines, attr, span: Span::NONE }, block_stack, inline_stack);
            }
        }
        Event::EndHighlight => {
            if let Some(InlineFrame::Highlight { inlines, attr }) = inline_stack.pop() {
                push_inline_to_ctx(Inline::Highlight { inlines, attr, span: Span::NONE }, block_stack, inline_stack);
            }
        }
        Event::EndSubscript => {
            if let Some(InlineFrame::Subscript { inlines, attr }) = inline_stack.pop() {
                push_inline_to_ctx(Inline::Subscript { inlines, attr, span: Span::NONE }, block_stack, inline_stack);
            }
        }
        Event::EndSuperscript => {
            if let Some(InlineFrame::Superscript { inlines, attr }) = inline_stack.pop() {
                push_inline_to_ctx(Inline::Superscript { inlines, attr, span: Span::NONE }, block_stack, inline_stack);
            }
        }
        Event::EndLink => {
            if let Some(InlineFrame::Link { url, title, inlines, attr }) = inline_stack.pop() {
                push_inline_to_ctx(Inline::Link { url, title, inlines, attr, span: Span::NONE }, block_stack, inline_stack);
            }
        }
        Event::EndImage => {
            if let Some(InlineFrame::Image { url, title, inlines, attr }) = inline_stack.pop() {
                push_inline_to_ctx(Inline::Image { url, title, inlines, attr, span: Span::NONE }, block_stack, inline_stack);
            }
        }
        Event::EndSpan => {
            if let Some(InlineFrame::Span { inlines, attr }) = inline_stack.pop() {
                push_inline_to_ctx(Inline::Span { inlines, attr, span: Span::NONE }, block_stack, inline_stack);
            }
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
