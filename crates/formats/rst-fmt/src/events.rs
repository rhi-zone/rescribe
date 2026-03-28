//! Streaming event iterator over a parsed [`RstDoc`].

use std::borrow::Cow;

use crate::{Block, DefinitionItem, Inline, TableRow};

/// A streaming event from an RST document.
///
/// Raw text content fields use `Cow<'a, str>` so that future optimisations can
/// yield borrowed slices of the input without changing the public API.
/// For the common case of fully-owned events (e.g. batch mode) use the
/// [`OwnedEvent`] type alias.
#[derive(Debug, Clone, PartialEq)]
pub enum Event<'a> {
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
    CodeBlockContent(Cow<'a, str>),
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
    Text(Cow<'a, str>),
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
    Code(Cow<'a, str>),
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

/// Backwards-compatible alias for batch mode (all text is owned).
pub type OwnedEvent = Event<'static>;

impl<'a> Event<'a> {
    /// Convert to an owned event (all `Cow::Borrowed` text fields become `Cow::Owned`).
    pub fn into_owned(self) -> OwnedEvent {
        match self {
            Event::Text(cow) => Event::Text(Cow::Owned(cow.into_owned())),
            Event::Code(cow) => Event::Code(Cow::Owned(cow.into_owned())),
            Event::CodeBlockContent(cow) => Event::CodeBlockContent(Cow::Owned(cow.into_owned())),
            // Safety: all other variants contain only String/'static fields.
            other => unsafe { std::mem::transmute::<Event<'_>, OwnedEvent>(other) },
        }
    }
}

pub use crate::EventIter;


/// Collect all blocks from an [`EventIter`] into a `Vec<Block>`.
///
/// Available for callers who drive [`EventIter`] as an iterator and want to
/// reconstruct the AST from the event stream. `parse()` uses direct recursive
/// descent instead.
#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(clippy::too_many_lines, dead_code)]
fn handle_event(event: Event<'_>, block_stack: &mut Vec<BlockFrame>, inline_ctx: &mut Vec<InlineFrame>) {
    match event {
        // ── Block openers ─────────────────────────────────────────────────────
        Event::StartParagraph => {
            block_stack.push(BlockFrame::Paragraph { inlines: Vec::new() });
        }
        Event::EndParagraph => {
            if let Some(BlockFrame::Paragraph { inlines }) = block_stack.pop() {
                push_block(Block::Paragraph { inlines }, block_stack);
            }
        }
        Event::StartHeading { level } => {
            block_stack.push(BlockFrame::Heading { level, inlines: Vec::new() });
        }
        Event::EndHeading => {
            if let Some(BlockFrame::Heading { level, inlines }) = block_stack.pop() {
                push_block(Block::Heading { level, inlines }, block_stack);
            }
        }
        Event::StartCodeBlock { language } => {
            block_stack.push(BlockFrame::CodeBlock { language, content: String::new() });
        }
        Event::CodeBlockContent(cow) => {
            if let Some(BlockFrame::CodeBlock { content, .. }) = block_stack.last_mut() {
                *content = cow.into_owned();
            }
        }
        Event::EndCodeBlock => {
            if let Some(BlockFrame::CodeBlock { language, content }) = block_stack.pop() {
                push_block(Block::CodeBlock { language, content }, block_stack);
            }
        }
        Event::StartBlockquote => {
            block_stack.push(BlockFrame::Blockquote { children: Vec::new() });
        }
        Event::EndBlockquote => {
            if let Some(BlockFrame::Blockquote { children }) = block_stack.pop() {
                push_block(Block::Blockquote { children }, block_stack);
            }
        }
        Event::StartList { ordered } => {
            block_stack.push(BlockFrame::List { ordered, items: Vec::new() });
        }
        Event::EndList => {
            if let Some(BlockFrame::List { ordered, items }) = block_stack.pop() {
                push_block(Block::List { ordered, items }, block_stack);
            }
        }
        Event::StartListItem => {
            block_stack.push(BlockFrame::ListItem { blocks: Vec::new() });
        }
        Event::EndListItem => {
            if let Some(BlockFrame::ListItem { blocks }) = block_stack.pop() {
                if let Some(BlockFrame::List { items, .. }) = block_stack.last_mut() {
                    items.push(blocks);
                }
            }
        }
        Event::StartDefinitionList => {
            block_stack.push(BlockFrame::DefinitionList { items: Vec::new() });
        }
        Event::EndDefinitionList => {
            if let Some(BlockFrame::DefinitionList { items }) = block_stack.pop() {
                push_block(Block::DefinitionList { items }, block_stack);
            }
        }
        Event::StartDefinitionTerm => {
            block_stack.push(BlockFrame::DefinitionTerm { inlines: Vec::new() });
        }
        Event::EndDefinitionTerm => {
            if let Some(BlockFrame::DefinitionTerm { inlines }) = block_stack.pop() {
                // Push a partial DefinitionItem with term filled, desc empty.
                // The EndDefinitionDesc will complete it.
                if let Some(BlockFrame::DefinitionList { items }) = block_stack.last_mut() {
                    items.push(DefinitionItem { term: inlines, desc: Vec::new() });
                }
            }
        }
        Event::StartDefinitionDesc => {
            block_stack.push(BlockFrame::DefinitionDesc { inlines: Vec::new() });
        }
        Event::EndDefinitionDesc => {
            if let Some(BlockFrame::DefinitionDesc { inlines }) = block_stack.pop() {
                if let Some(BlockFrame::DefinitionList { items }) = block_stack.last_mut() {
                    if let Some(last_item) = items.last_mut() {
                        last_item.desc = inlines;
                    }
                }
            }
        }
        Event::StartDiv { class, directive } => {
            block_stack.push(BlockFrame::Div { class, directive, children: Vec::new() });
        }
        Event::EndDiv => {
            if let Some(BlockFrame::Div { class, directive, children }) = block_stack.pop() {
                push_block(Block::Div { class, directive, children }, block_stack);
            }
        }
        Event::HorizontalRule => {
            push_block(Block::HorizontalRule, block_stack);
        }
        Event::StartTable => {
            block_stack.push(BlockFrame::Table { rows: Vec::new() });
        }
        Event::EndTable => {
            if let Some(BlockFrame::Table { rows }) = block_stack.pop() {
                push_block(Block::Table { rows }, block_stack);
            }
        }
        Event::StartTableRow { is_header } => {
            block_stack.push(BlockFrame::TableRow { is_header, cells: Vec::new() });
        }
        Event::EndTableRow => {
            if let Some(BlockFrame::TableRow { is_header, cells }) = block_stack.pop() {
                if let Some(BlockFrame::Table { rows }) = block_stack.last_mut() {
                    rows.push(TableRow { cells, is_header });
                }
            }
        }
        Event::StartTableCell => {
            block_stack.push(BlockFrame::TableCell { inlines: Vec::new() });
        }
        Event::EndTableCell => {
            if let Some(BlockFrame::TableCell { inlines }) = block_stack.pop() {
                if let Some(BlockFrame::TableRow { cells, .. }) = block_stack.last_mut() {
                    cells.push(inlines);
                }
            }
        }
        Event::StartFootnoteDef { label } => {
            block_stack.push(BlockFrame::FootnoteDef { label, inlines: Vec::new() });
        }
        Event::EndFootnoteDef => {
            if let Some(BlockFrame::FootnoteDef { label, inlines }) = block_stack.pop() {
                push_block(Block::FootnoteDef { label, inlines }, block_stack);
            }
        }
        Event::MathDisplay { source } => {
            push_block(Block::MathDisplay { source }, block_stack);
        }
        Event::StartAdmonition { admonition_type } => {
            block_stack.push(BlockFrame::Admonition { admonition_type, children: Vec::new() });
        }
        Event::EndAdmonition => {
            if let Some(BlockFrame::Admonition { admonition_type, children }) = block_stack.pop() {
                push_block(Block::Admonition { admonition_type, children }, block_stack);
            }
        }
        Event::StartFigure { url, alt } => {
            block_stack.push(BlockFrame::Figure { url, alt, caption_inlines: Vec::new() });
        }
        Event::EndFigure => {
            if let Some(BlockFrame::Figure { url, alt, caption_inlines }) = block_stack.pop() {
                let caption = if caption_inlines.is_empty() { None } else { Some(caption_inlines) };
                push_block(Block::Figure { url, alt, caption }, block_stack);
            }
        }
        Event::ImageBlock { url, alt, title } => {
            push_block(Block::Image { url, alt, title }, block_stack);
        }
        Event::RawBlock { format, content } => {
            push_block(Block::RawBlock { format, content }, block_stack);
        }
        Event::StartLineBlock => {
            block_stack.push(BlockFrame::LineBlock { lines: Vec::new() });
        }
        Event::EndLineBlock => {
            if let Some(BlockFrame::LineBlock { lines }) = block_stack.pop() {
                push_block(Block::LineBlock { lines }, block_stack);
            }
        }
        Event::StartLineBlockLine => {
            block_stack.push(BlockFrame::LineBlockLine { inlines: Vec::new() });
        }
        Event::EndLineBlockLine => {
            if let Some(BlockFrame::LineBlockLine { inlines }) = block_stack.pop() {
                if let Some(BlockFrame::LineBlock { lines }) = block_stack.last_mut() {
                    lines.push(inlines);
                }
            }
        }

        // ── Inline events ─────────────────────────────────────────────────────
        Event::Text(cow) => push_inline(Inline::Text(cow.into_owned()), block_stack, inline_ctx),
        Event::SoftBreak => push_inline(Inline::SoftBreak, block_stack, inline_ctx),
        Event::LineBreak => push_inline(Inline::LineBreak, block_stack, inline_ctx),
        Event::Code(cow) => push_inline(Inline::Code(cow.into_owned()), block_stack, inline_ctx),
        Event::FootnoteRef { label } => {
            push_inline(Inline::FootnoteRef { label }, block_stack, inline_ctx);
        }
        Event::InlineImage { url, alt } => {
            push_inline(Inline::Image { url, alt }, block_stack, inline_ctx);
        }
        Event::MathInline { source } => {
            push_inline(Inline::MathInline { source }, block_stack, inline_ctx);
        }
        Event::StartEmphasis => inline_ctx.push(InlineFrame::Emphasis { inlines: Vec::new() }),
        Event::EndEmphasis => {
            if let Some(InlineFrame::Emphasis { inlines }) = inline_ctx.pop() {
                push_inline(Inline::Emphasis(inlines), block_stack, inline_ctx);
            }
        }
        Event::StartStrong => inline_ctx.push(InlineFrame::Strong { inlines: Vec::new() }),
        Event::EndStrong => {
            if let Some(InlineFrame::Strong { inlines }) = inline_ctx.pop() {
                push_inline(Inline::Strong(inlines), block_stack, inline_ctx);
            }
        }
        Event::StartStrikeout => inline_ctx.push(InlineFrame::Strikeout { inlines: Vec::new() }),
        Event::EndStrikeout => {
            if let Some(InlineFrame::Strikeout { inlines }) = inline_ctx.pop() {
                push_inline(Inline::Strikeout(inlines), block_stack, inline_ctx);
            }
        }
        Event::StartUnderline => inline_ctx.push(InlineFrame::Underline { inlines: Vec::new() }),
        Event::EndUnderline => {
            if let Some(InlineFrame::Underline { inlines }) = inline_ctx.pop() {
                push_inline(Inline::Underline(inlines), block_stack, inline_ctx);
            }
        }
        Event::StartSubscript => inline_ctx.push(InlineFrame::Subscript { inlines: Vec::new() }),
        Event::EndSubscript => {
            if let Some(InlineFrame::Subscript { inlines }) = inline_ctx.pop() {
                push_inline(Inline::Subscript(inlines), block_stack, inline_ctx);
            }
        }
        Event::StartSuperscript => inline_ctx.push(InlineFrame::Superscript { inlines: Vec::new() }),
        Event::EndSuperscript => {
            if let Some(InlineFrame::Superscript { inlines }) = inline_ctx.pop() {
                push_inline(Inline::Superscript(inlines), block_stack, inline_ctx);
            }
        }
        Event::StartSmallCaps => inline_ctx.push(InlineFrame::SmallCaps { inlines: Vec::new() }),
        Event::EndSmallCaps => {
            if let Some(InlineFrame::SmallCaps { inlines }) = inline_ctx.pop() {
                push_inline(Inline::SmallCaps(inlines), block_stack, inline_ctx);
            }
        }
        Event::StartLink { url } => {
            inline_ctx.push(InlineFrame::Link { url, children: Vec::new() });
        }
        Event::EndLink => {
            if let Some(InlineFrame::Link { url, children }) = inline_ctx.pop() {
                push_inline(Inline::Link { url, children }, block_stack, inline_ctx);
            }
        }
        Event::StartFootnoteDefInline { label } => {
            inline_ctx.push(InlineFrame::FootnoteDefInline { label, children: Vec::new() });
        }
        Event::EndFootnoteDefInline => {
            if let Some(InlineFrame::FootnoteDefInline { label, children }) = inline_ctx.pop() {
                push_inline(Inline::FootnoteDef { label, children }, block_stack, inline_ctx);
            }
        }
        Event::StartQuoted { quote_type } => {
            inline_ctx.push(InlineFrame::Quoted { quote_type, children: Vec::new() });
        }
        Event::EndQuoted => {
            if let Some(InlineFrame::Quoted { quote_type, children }) = inline_ctx.pop() {
                push_inline(Inline::Quoted { quote_type, children }, block_stack, inline_ctx);
            }
        }
        Event::StartRstSpan { role } => {
            inline_ctx.push(InlineFrame::RstSpan { role, children: Vec::new() });
        }
        Event::EndRstSpan => {
            if let Some(InlineFrame::RstSpan { role, children }) = inline_ctx.pop() {
                push_inline(Inline::RstSpan { role, children }, block_stack, inline_ctx);
            }
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
        // Verify that events() produces well-formed bracketed event sequences.
        let inputs = [
            "Section\n=======\n\nHello world.\n",
            "- item one\n- item two\n",
            ".. code-block:: rust\n\n   let x = 1;\n",
            ".. note::\n\n   Some note.\n",
        ];
        for input in inputs {
            let evs: Vec<_> = crate::events(input).collect();
            // Events must be non-empty for non-empty input.
            assert!(!evs.is_empty(), "no events for input: {input:?}");
            // parse() must succeed.
            crate::parse(input).expect("parse failed");
        }
    }
}
