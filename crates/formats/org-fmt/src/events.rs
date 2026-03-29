//! Streaming event iterator over a parsed `OrgDoc`.

use std::borrow::Cow;

use crate::ast::*;

/// A streaming event from an Org-mode document.
///
/// Raw text content fields use `Cow<'a, str>` so that future optimisations can
/// yield borrowed slices of the input without changing the public API.
/// For the common case of fully-owned events (e.g. batch mode) use the
/// [`OwnedEvent`] type alias.
#[derive(Debug, PartialEq)]
pub enum Event<'a> {
    // ── Block events ──────────────────────────────────────────────────────────
    StartParagraph,
    EndParagraph,
    StartHeading {
        level: usize,
        todo: Option<String>,
        priority: Option<String>,
        tags: Vec<String>,
        properties: Vec<(String, String)>,
        scheduled: Option<String>,
        deadline: Option<String>,
    },
    EndHeading,
    StartBlockquote,
    EndBlockquote,
    StartList {
        ordered: bool,
        start: Option<u64>,
    },
    EndList,
    StartListItem {
        checkbox: Option<CheckboxState>,
    },
    EndListItem,
    /// Leaf: a fenced code block.
    CodeBlock {
        language: Option<String>,
        header_args: Option<String>,
        name: Option<String>,
        content: Cow<'a, str>,
    },
    /// Leaf: a raw export block.
    RawBlock {
        format: String,
        content: String,
    },
    /// Leaf: a horizontal rule (`-----`).
    HorizontalRule,
    StartTable,
    EndTable,
    StartTableRow {
        is_header: bool,
    },
    EndTableRow,
    StartTableCell,
    EndTableCell,
    StartDefinitionList,
    EndDefinitionList,
    StartDefinitionTerm,
    EndDefinitionTerm,
    StartDefinitionDesc,
    EndDefinitionDesc,
    StartDiv,
    EndDiv,
    StartFigure {
        name: Option<String>,
    },
    EndFigure,
    StartCaption,
    EndCaption,
    /// Block-level footnote definition `[fn:label] content`.
    StartBlockFootnoteDef {
        label: String,
    },
    EndBlockFootnoteDef,
    /// Unknown block kind (preserved for diagnostics).
    UnknownBlock {
        kind: String,
    },

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
    StartLink {
        url: String,
    },
    EndLink,
    /// Leaf: standalone image link (no children).
    InlineImage {
        url: String,
    },
    /// Leaf: footnote reference.
    FootnoteRef {
        label: String,
    },
    StartFootnoteDefinition {
        label: String,
    },
    EndFootnoteDefinition,
    /// Leaf: inline math `$...$`.
    MathInline {
        source: String,
    },
    /// Leaf: Org timestamp `<...>` or `[...]`.
    Timestamp {
        active: bool,
        value: String,
    },
    /// Leaf: export snippet `@@backend:value@@`.
    ExportSnippet {
        backend: String,
        value: String,
    },
}

/// Backwards-compatible alias for batch mode (all text is owned).
pub type OwnedEvent = Event<'static>;

impl<'a> Event<'a> {
    /// Convert to an owned event (all `Cow::Borrowed` text fields become `Cow::Owned`).
    pub fn into_owned(self) -> OwnedEvent {
        match self {
            Event::Text(cow) => Event::Text(Cow::Owned(cow.into_owned())),
            Event::InlineCode(cow) => Event::InlineCode(Cow::Owned(cow.into_owned())),
            Event::CodeBlock { language, header_args, name, content } => Event::CodeBlock {
                language,
                header_args,
                name,
                content: Cow::Owned(content.into_owned()),
            },
            // Safety: all other variants contain only String/'static fields.
            other => unsafe { std::mem::transmute::<Event<'_>, OwnedEvent>(other) },
        }
    }
}

// ── True pull iterator ────────────────────────────────────────────────────────

pub use crate::parse::EventIter;

// ── Tree builder (inverse of the collect_* functions) ────────────────────────

/// Collect a complete `OrgDoc` from an `EventIter`.
///
/// Useful for callers that drive [`EventIter`] as an iterator and want a
/// complete [`crate::ast::OrgDoc`] at the end.  `parse::parse()` no longer
/// calls this; it uses direct recursive descent instead.
pub fn collect_doc_from_iter(
    iter: &mut EventIter<'_>,
) -> (Vec<Block>, Vec<(String, String)>, Vec<Diagnostic>) {
    let mut block_stack: Vec<BlockFrame> = vec![BlockFrame::Document { blocks: Vec::new() }];
    let mut inline_ctx: Vec<InlineFrame> = Vec::new();

    for event in iter.by_ref() {
        handle_event(event, &mut block_stack, &mut inline_ctx);
    }

    let metadata = iter.take_metadata();
    let diagnostics = std::mem::take(&mut iter.diagnostics);

    let blocks = match block_stack.pop() {
        Some(BlockFrame::Document { blocks }) => blocks,
        _ => Vec::new(),
    };

    (blocks, metadata, diagnostics)
}

// ── Block frame stack ─────────────────────────────────────────────────────────

#[allow(dead_code)]
enum BlockFrame {
    Document { blocks: Vec<Block> },
    Paragraph { inlines: Vec<Inline> },
    Heading { level: usize, todo: Option<String>, priority: Option<String>, tags: Vec<String>, properties: Vec<(String, String)>, scheduled: Option<String>, deadline: Option<String>, inlines: Vec<Inline> },
    Blockquote { children: Vec<Block> },
    List { ordered: bool, start: Option<u64>, items: Vec<ListItem> },
    ListItem { checkbox: Option<CheckboxState>, children: Vec<ListItemContent>, inline_buf: Vec<Inline> },
    Table { rows: Vec<TableRow> },
    TableRow { is_header: bool, cells: Vec<Vec<Inline>> },
    TableCell { inlines: Vec<Inline> },
    DefinitionList { items: Vec<DefinitionItem> },
    DefinitionTerm { inlines: Vec<Inline> },
    DefinitionDesc { inlines: Vec<Inline> },
    Div { inlines: Vec<Inline> },
    Figure { name: Option<String>, children: Vec<Block> },
    Caption { inlines: Vec<Inline> },
    BlockFootnoteDef { label: String, inlines: Vec<Inline> },
}

// ── Inline frame stack ────────────────────────────────────────────────────────

#[allow(dead_code)]
enum InlineFrame {
    Bold { inlines: Vec<Inline> },
    Italic { inlines: Vec<Inline> },
    Underline { inlines: Vec<Inline> },
    Strikethrough { inlines: Vec<Inline> },
    Superscript { inlines: Vec<Inline> },
    Subscript { inlines: Vec<Inline> },
    Link { url: String, inlines: Vec<Inline> },
    FootnoteDefinition { label: String, inlines: Vec<Inline> },
}

#[allow(dead_code)]
fn inlines_from_frame(frame: &mut InlineFrame) -> &mut Vec<Inline> {
    match frame {
        InlineFrame::Bold { inlines } => inlines,
        InlineFrame::Italic { inlines } => inlines,
        InlineFrame::Underline { inlines } => inlines,
        InlineFrame::Strikethrough { inlines } => inlines,
        InlineFrame::Superscript { inlines } => inlines,
        InlineFrame::Subscript { inlines } => inlines,
        InlineFrame::Link { inlines, .. } => inlines,
        InlineFrame::FootnoteDefinition { inlines, .. } => inlines,
    }
}

#[allow(dead_code)]
fn push_inline(block_stack: &mut [BlockFrame], inline_ctx: &mut [InlineFrame], inline: Inline) {
    if let Some(frame) = inline_ctx.last_mut() {
        inlines_from_frame(frame).push(inline);
        return;
    }
    match block_stack.last_mut() {
        Some(BlockFrame::Paragraph { inlines }) => inlines.push(inline),
        Some(BlockFrame::Heading { inlines, .. }) => inlines.push(inline),
        Some(BlockFrame::DefinitionTerm { inlines }) => inlines.push(inline),
        Some(BlockFrame::DefinitionDesc { inlines }) => inlines.push(inline),
        Some(BlockFrame::TableCell { inlines }) => inlines.push(inline),
        Some(BlockFrame::Div { inlines }) => inlines.push(inline),
        Some(BlockFrame::Caption { inlines }) => inlines.push(inline),
        Some(BlockFrame::BlockFootnoteDef { inlines, .. }) => inlines.push(inline),
        Some(BlockFrame::ListItem { inline_buf, .. }) => inline_buf.push(inline),
        _ => {}
    }
}

#[allow(dead_code)]
fn push_block(block_stack: &mut [BlockFrame], block: Block) {
    match block_stack.last_mut() {
        Some(BlockFrame::Document { blocks }) => blocks.push(block),
        Some(BlockFrame::Blockquote { children }) => children.push(block),
        Some(BlockFrame::Figure { children, .. }) => children.push(block),
        Some(BlockFrame::ListItem { children, inline_buf, .. }) => {
            // Flush any accumulated inline content before the nested block.
            if !inline_buf.is_empty() {
                let inlines = std::mem::take(inline_buf);
                children.push(ListItemContent::Inline(inlines));
            }
            children.push(ListItemContent::Block(block));
        }
        _ => {}
    }
}

#[allow(dead_code)]
fn handle_event(event: Event<'_>, block_stack: &mut Vec<BlockFrame>, inline_ctx: &mut Vec<InlineFrame>) {
    match event {
        // ── Block start events ─────────────────────────────────────────────
        Event::StartParagraph => {
            block_stack.push(BlockFrame::Paragraph { inlines: Vec::new() });
        }
        Event::StartHeading { level, todo, priority, tags, properties, scheduled, deadline } => {
            block_stack.push(BlockFrame::Heading { level, todo, priority, tags, properties, scheduled, deadline, inlines: Vec::new() });
        }
        Event::StartBlockquote => {
            block_stack.push(BlockFrame::Blockquote { children: Vec::new() });
        }
        Event::StartList { ordered, start } => {
            block_stack.push(BlockFrame::List { ordered, start, items: Vec::new() });
        }
        Event::StartListItem { checkbox } => {
            block_stack.push(BlockFrame::ListItem { checkbox, children: Vec::new(), inline_buf: Vec::new() });
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
        Event::StartDefinitionList => {
            block_stack.push(BlockFrame::DefinitionList { items: Vec::new() });
        }
        Event::StartDefinitionTerm => {
            block_stack.push(BlockFrame::DefinitionTerm { inlines: Vec::new() });
        }
        Event::StartDefinitionDesc => {
            block_stack.push(BlockFrame::DefinitionDesc { inlines: Vec::new() });
        }
        Event::StartDiv => {
            block_stack.push(BlockFrame::Div { inlines: Vec::new() });
        }
        Event::StartFigure { name } => {
            block_stack.push(BlockFrame::Figure { name, children: Vec::new() });
        }
        Event::StartCaption => {
            block_stack.push(BlockFrame::Caption { inlines: Vec::new() });
        }
        Event::StartFootnoteDefinition { label } => {
            inline_ctx.push(InlineFrame::FootnoteDefinition { label, inlines: Vec::new() });
        }
        Event::StartBlockFootnoteDef { label } => {
            block_stack.push(BlockFrame::BlockFootnoteDef { label, inlines: Vec::new() });
        }

        // ── Block end events ───────────────────────────────────────────────
        Event::EndParagraph => {
            if let Some(BlockFrame::Paragraph { inlines }) = block_stack.pop() {
                push_block(block_stack, Block::Paragraph { inlines, span: Span::NONE });
            }
        }
        Event::EndHeading => {
            if let Some(BlockFrame::Heading { level, todo, priority, tags, properties, scheduled, deadline, inlines }) = block_stack.pop() {
                push_block(block_stack, Block::Heading {
                    level, todo, priority, tags, properties, scheduled, deadline,
                    inlines,
                    span: Span::NONE,
                });
            }
        }
        Event::EndBlockquote => {
            if let Some(BlockFrame::Blockquote { children }) = block_stack.pop() {
                push_block(block_stack, Block::Blockquote { children, span: Span::NONE });
            }
        }
        Event::EndList => {
            if let Some(BlockFrame::List { ordered, start, items }) = block_stack.pop() {
                push_block(block_stack, Block::List { ordered, start, items, span: Span::NONE });
            }
        }
        Event::EndListItem => {
            if let Some(BlockFrame::ListItem { checkbox, mut children, inline_buf }) = block_stack.pop() {
                // Flush any trailing inline content.
                if !inline_buf.is_empty() {
                    children.push(ListItemContent::Inline(inline_buf));
                }
                if let Some(BlockFrame::List { items, .. }) = block_stack.last_mut() {
                    items.push(ListItem { children, checkbox });
                }
            }
        }
        Event::EndTable => {
            if let Some(BlockFrame::Table { rows }) = block_stack.pop() {
                push_block(block_stack, Block::Table { rows, span: Span::NONE });
            }
        }
        Event::EndTableRow => {
            if let Some(BlockFrame::TableRow { is_header, cells }) = block_stack.pop()
                && let Some(BlockFrame::Table { rows }) = block_stack.last_mut()
            {
                rows.push(TableRow { cells, is_header });
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
                push_block(block_stack, Block::DefinitionList { items, span: Span::NONE });
            }
        }
        Event::EndDefinitionTerm => {
            // Push a partial item with empty desc; EndDefinitionDesc fills it in.
            if let Some(BlockFrame::DefinitionTerm { inlines }) = block_stack.pop()
                && let Some(BlockFrame::DefinitionList { items }) = block_stack.last_mut()
            {
                items.push(DefinitionItem { term: inlines, desc: Vec::new() });
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
        Event::EndDiv => {
            if let Some(BlockFrame::Div { inlines }) = block_stack.pop() {
                push_block(block_stack, Block::Div { inlines, span: Span::NONE });
            }
        }
        Event::EndFigure => {
            if let Some(BlockFrame::Figure { name, children }) = block_stack.pop() {
                push_block(block_stack, Block::Figure { name, children, span: Span::NONE });
            }
        }
        Event::EndCaption => {
            if let Some(BlockFrame::Caption { inlines }) = block_stack.pop() {
                push_block(block_stack, Block::Caption { inlines, span: Span::NONE });
            }
        }
        Event::EndBlockFootnoteDef => {
            if let Some(BlockFrame::BlockFootnoteDef { label, inlines }) = block_stack.pop() {
                push_block(block_stack, Block::FootnoteDef { label, content: inlines, span: Span::NONE });
            }
        }
        Event::EndFootnoteDefinition => {
            if let Some(InlineFrame::FootnoteDefinition { label, inlines }) = inline_ctx.pop() {
                push_inline(block_stack, inline_ctx, Inline::FootnoteDefinition {
                    label,
                    children: inlines,
                    span: Span::NONE,
                });
            }
        }

        // ── Leaf block events ──────────────────────────────────────────────
        Event::CodeBlock { language, header_args, name, content } => {
            push_block(block_stack, Block::CodeBlock { language, header_args, name, content: content.into_owned(), span: Span::NONE });
        }
        Event::RawBlock { format, content } => {
            push_block(block_stack, Block::RawBlock { format, content, span: Span::NONE });
        }
        Event::HorizontalRule => {
            push_block(block_stack, Block::HorizontalRule { span: Span::NONE });
        }
        Event::UnknownBlock { kind } => {
            push_block(block_stack, Block::Unknown { kind, span: Span::NONE });
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
        Event::InlineCode(cow) => {
            push_inline(block_stack, inline_ctx, Inline::Code(cow.into_owned(), Span::NONE));
        }
        Event::InlineImage { url } => {
            push_inline(block_stack, inline_ctx, Inline::Image { url, span: Span::NONE });
        }
        Event::FootnoteRef { label } => {
            push_inline(block_stack, inline_ctx, Inline::FootnoteRef { label, span: Span::NONE });
        }
        Event::MathInline { source } => {
            push_inline(block_stack, inline_ctx, Inline::MathInline { source, span: Span::NONE });
        }
        Event::Timestamp { active, value } => {
            push_inline(block_stack, inline_ctx, Inline::Timestamp { active, value, span: Span::NONE });
        }
        Event::ExportSnippet { backend, value } => {
            push_inline(block_stack, inline_ctx, Inline::ExportSnippet { backend, value, span: Span::NONE });
        }

        // ── Inline container start events ──────────────────────────────────
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

        // ── Inline container end events ────────────────────────────────────
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
                push_inline(block_stack, inline_ctx, Inline::Strikethrough(inlines, Span::NONE));
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
                push_inline(block_stack, inline_ctx, Inline::Link { url, children: inlines, span: Span::NONE });
            }
        }
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Parse `input` and return a streaming iterator of [`OwnedEvent`] items.
pub fn events(input: &str) -> EventIter<'_> {
    EventIter::new(input)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_events_heading() {
        let evs: Vec<_> = events("* Hello").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartHeading { level: 1, .. })));
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
    fn test_events_code_block() {
        let evs: Vec<_> = events("#+BEGIN_SRC rust\nfn main() {}\n#+END_SRC").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::CodeBlock { language: Some(l), .. } if l == "rust")));
    }

    #[test]
    fn test_events_list() {
        let evs: Vec<_> = events("- item 1\n- item 2").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartList { ordered: false, .. })));
        assert_eq!(evs.iter().filter(|e| matches!(e, OwnedEvent::StartListItem { .. })).count(), 2);
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndList)));
    }

    #[test]
    fn test_events_table() {
        let evs: Vec<_> = events("| Name | Age |\n|------+-----|\n| Alice | 30 |").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartTable)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartTableRow { is_header: true })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndTable)));
    }

    #[test]
    fn test_events_horizontal_rule() {
        let evs: Vec<_> = events("-----").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::HorizontalRule)));
    }

    #[test]
    fn test_events_bold_italic() {
        let evs: Vec<_> = events("*bold* /italic/").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartBold)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndBold)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartItalic)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndItalic)));
    }

    #[test]
    fn test_events_link() {
        let evs: Vec<_> = events("[[https://example.com][click here]]").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartLink { url } if url == "https://example.com")));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndLink)));
    }

    #[test]
    fn test_events_inline_code() {
        let evs: Vec<_> = events("Some =verbatim= text").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::InlineCode(s) if s == "verbatim")));
    }

    #[test]
    fn test_events_footnote_ref() {
        let evs: Vec<_> = events("See [fn:1].").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::FootnoteRef { label } if label == "1")));
    }

    #[test]
    fn test_events_math_inline() {
        let evs: Vec<_> = events("Solve $x^2 + y^2 = r^2$.").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::MathInline { .. })));
    }

    #[test]
    fn test_events_blockquote() {
        let evs: Vec<_> = events("#+BEGIN_QUOTE\nquoted\n#+END_QUOTE").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartBlockquote)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndBlockquote)));
    }

    #[test]
    fn test_events_definition_list() {
        let evs: Vec<_> = events("- Term :: Description").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartDefinitionList)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartDefinitionTerm)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartDefinitionDesc)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndDefinitionList)));
    }
}
