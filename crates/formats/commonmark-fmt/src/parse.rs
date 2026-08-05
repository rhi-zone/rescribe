//! CommonMark parser — wraps pulldown-cmark's offset iterator into the [`CmDoc`] AST.

#[cfg(feature = "definition-lists")]
use crate::ast::DefinitionListItem;
use crate::ast::{
    Block, CmDoc, Diagnostic, Inline, LinkDef, ListItem, ListKind, OrderedMarker, Severity, Span,
};
#[cfg(feature = "frontmatter")]
use crate::ast::{FrontMatter, FrontMatterKind};
#[cfg(feature = "tables")]
use crate::ast::{TableCell, TableRow};
#[cfg(feature = "tables")]
use crate::options::{ColumnAlignment, pd_alignment_to_ast};
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Parser, Tag, TagEnd};

use crate::options::build_options;

// ── Frame stack ──────────────────────────────────────────────────────────────

/// One level in the tree-builder stack.
enum Frame {
    /// The root document accumulates top-level blocks.
    Doc {
        blocks: Vec<Block>,
    },
    Blockquote {
        blocks: Vec<Block>,
        start: usize,
    },
    List {
        kind: ListKind,
        items: Vec<ListItem>,
        tight: bool,
        start: usize,
    },
    Item {
        blocks: Vec<Block>,
        tight_para: bool,
        tight_inlines: Vec<Inline>,
        #[cfg(feature = "task-lists")]
        checked: Option<bool>,
        start: usize,
    },
    Paragraph {
        inlines: Vec<Inline>,
        start: usize,
    },
    Heading {
        level: u8,
        inlines: Vec<Inline>,
        start: usize,
    },
    Emphasis {
        inlines: Vec<Inline>,
        start: usize,
    },
    Strong {
        inlines: Vec<Inline>,
        start: usize,
    },
    #[cfg(feature = "strikethrough")]
    Strikethrough {
        inlines: Vec<Inline>,
        start: usize,
    },
    Link {
        inlines: Vec<Inline>,
        url: String,
        title: Option<String>,
        start: usize,
    },
    /// Accumulates the alt text from the text events inside an image tag.
    Image {
        alt: String,
        url: String,
        title: Option<String>,
        start: usize,
    },
    /// A buffered HTML block: content is accumulated from consecutive Html events.
    HtmlBlock {
        content: String,
        start: usize,
    },
    /// A code block: accumulates a single Text event as content.
    CodeBlock {
        language: Option<String>,
        content: String,
        start: usize,
    },
    /// Front-matter block: accumulates raw text content.
    #[cfg(feature = "frontmatter")]
    FrontMatter {
        kind: FrontMatterKind,
        content: String,
        start: usize,
    },
    /// GFM table: accumulates the head row and body rows.
    #[cfg(feature = "tables")]
    Table {
        alignments: Vec<ColumnAlignment>,
        head: Option<TableRow>,
        rows: Vec<TableRow>,
        start: usize,
    },
    /// A table row (either the head row or a body row).
    #[cfg(feature = "tables")]
    TableRow {
        cells: Vec<TableCell>,
        start: usize,
    },
    /// A single table cell — accumulates inline content only (GFM table cells
    /// cannot contain block content).
    #[cfg(feature = "tables")]
    TableCell {
        inlines: Vec<Inline>,
        start: usize,
    },
    /// A footnote definition: accumulates block content, exactly like
    /// `Blockquote` (footnote definitions never use the tight-inline
    /// shortcut — pulldown-cmark always wraps their content in real
    /// `Paragraph` tags; see `math.rs`/`footnotes.rs` upstream test suite).
    #[cfg(feature = "footnotes")]
    FootnoteDefinition {
        label: String,
        blocks: Vec<Block>,
        start: usize,
    },
    /// A definition list: accumulates completed `DefinitionListItem`s. Tracks
    /// the in-progress term/definitions group so consecutive
    /// `DefinitionListDefinition`s (there can be more than one per term) are
    /// grouped under the term that precedes them, and whether the list is
    /// tight — same signal `Frame::List`'s `tight` field uses (an explicit
    /// `Paragraph` tag inside a definition means the whole list is loose).
    #[cfg(feature = "definition-lists")]
    DefinitionList {
        items: Vec<DefinitionListItem>,
        pending_term: Option<Vec<Inline>>,
        pending_defs: Vec<Vec<Block>>,
        term_start: usize,
        /// End offset for the still-open pending item's span — updated at
        /// every `DefinitionListTitle`/`DefinitionListDefinition` close so
        /// the pending item always has a valid span even if flushed with
        /// zero definitions.
        last_end: usize,
        tight: bool,
        start: usize,
    },
    /// A definition list term (`dt`): accumulates inline content only —
    /// pulldown-cmark never wraps a title in a nested `Paragraph` tag.
    #[cfg(feature = "definition-lists")]
    DefinitionListTitle {
        inlines: Vec<Inline>,
        start: usize,
    },
    /// A single definition (`dd`) body: accumulates block content, with the
    /// same tight-inline accumulation `Frame::Item` uses (tight definition
    /// lists get bare inline content directly, with no wrapping `Paragraph`
    /// tag from pulldown-cmark).
    #[cfg(feature = "definition-lists")]
    DefinitionListDefinition {
        blocks: Vec<Block>,
        tight_para: bool,
        tight_inlines: Vec<Inline>,
    },
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Parse CommonMark (plus GFM strikethrough) from a byte slice.
///
/// Always succeeds; non-UTF-8 input produces a single `Warning` diagnostic and
/// an empty document. Any unknown pulldown-cmark events are silently skipped —
/// this is a strict superset of the CommonMark spec and diagnostics are only
/// generated for encoding problems.
pub(crate) fn parse(input: &[u8]) -> (CmDoc, Vec<Diagnostic>) {
    let s = match std::str::from_utf8(input) {
        Ok(s) => s,
        Err(_) => {
            return (
                CmDoc {
                    blocks: vec![],
                    link_defs: vec![],
                    #[cfg(feature = "frontmatter")]
                    frontmatter: None,
                },
                vec![Diagnostic {
                    span: Span::NONE,
                    severity: Severity::Warning,
                    message: "input is not valid UTF-8".to_string(),
                    code: "commonmark::invalid-utf8",
                }],
            );
        }
    };
    parse_str(s)
}

/// Parse CommonMark (plus GFM strikethrough) from a `&str`.
pub fn parse_str(input: &str) -> (CmDoc, Vec<Diagnostic>) {
    let opts = build_options();
    let iter = Parser::new_ext(input, opts).into_offset_iter();

    let mut stack: Vec<Frame> = vec![Frame::Doc { blocks: vec![] }];
    let diagnostics: Vec<Diagnostic> = vec![];
    #[cfg(feature = "frontmatter")]
    let mut frontmatter: Option<FrontMatter> = None;

    // pulldown-cmark exposes reference link definitions via a separate API.
    let link_defs = collect_link_defs(input);

    for (event, range) in iter {
        let start = range.start;
        let end = range.end;

        match event {
            // ── Block opens ──────────────────────────────────────────────────
            Event::Start(Tag::Paragraph) => {
                stack.push(Frame::Paragraph {
                    inlines: vec![],
                    start,
                });
            }
            Event::Start(Tag::Heading { level, .. }) => {
                let level_u8 = heading_level_to_u8(level);
                stack.push(Frame::Heading {
                    level: level_u8,
                    inlines: vec![],
                    start,
                });
            }
            Event::Start(Tag::BlockQuote(_)) => {
                stack.push(Frame::Blockquote {
                    blocks: vec![],
                    start,
                });
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                let language = match kind {
                    CodeBlockKind::Fenced(lang) => {
                        let s = lang.trim().to_string();
                        if s.is_empty() { None } else { Some(s) }
                    }
                    CodeBlockKind::Indented => None,
                };
                stack.push(Frame::CodeBlock {
                    language,
                    content: String::new(),
                    start,
                });
            }
            Event::Start(Tag::List(first)) => {
                let kind = match first {
                    None => ListKind::Unordered { marker: '-' },
                    Some(n) => ListKind::Ordered {
                        start: n,
                        marker: OrderedMarker::Period,
                    },
                };
                // tight is determined later by whether Item children contain paragraphs
                stack.push(Frame::List {
                    kind,
                    items: vec![],
                    tight: true,
                    start,
                });
            }
            Event::Start(Tag::Item) => {
                stack.push(Frame::Item {
                    blocks: vec![],
                    tight_para: false,
                    tight_inlines: vec![],
                    #[cfg(feature = "task-lists")]
                    checked: None,
                    start,
                });
            }
            Event::Start(Tag::HtmlBlock) => {
                stack.push(Frame::HtmlBlock {
                    content: String::new(),
                    start,
                });
            }
            #[cfg(feature = "frontmatter")]
            Event::Start(Tag::MetadataBlock(kind)) => {
                let kind = match kind {
                    pulldown_cmark::MetadataBlockKind::YamlStyle => FrontMatterKind::Yaml,
                    pulldown_cmark::MetadataBlockKind::PlusesStyle => FrontMatterKind::Toml,
                };
                stack.push(Frame::FrontMatter {
                    kind,
                    content: String::new(),
                    start,
                });
            }
            #[cfg(feature = "tables")]
            Event::Start(Tag::Table(alignments)) => {
                let alignments = alignments.iter().map(pd_alignment_to_ast).collect();
                stack.push(Frame::Table {
                    alignments,
                    head: None,
                    rows: vec![],
                    start,
                });
            }
            #[cfg(feature = "tables")]
            Event::Start(Tag::TableHead) => {
                stack.push(Frame::TableRow {
                    cells: vec![],
                    start,
                });
            }
            #[cfg(feature = "tables")]
            Event::Start(Tag::TableRow) => {
                stack.push(Frame::TableRow {
                    cells: vec![],
                    start,
                });
            }
            #[cfg(feature = "tables")]
            Event::Start(Tag::TableCell) => {
                stack.push(Frame::TableCell {
                    inlines: vec![],
                    start,
                });
            }
            #[cfg(feature = "footnotes")]
            Event::Start(Tag::FootnoteDefinition(label)) => {
                stack.push(Frame::FootnoteDefinition {
                    label: label.into_string(),
                    blocks: vec![],
                    start,
                });
            }
            #[cfg(feature = "definition-lists")]
            Event::Start(Tag::DefinitionList) => {
                stack.push(Frame::DefinitionList {
                    items: vec![],
                    pending_term: None,
                    pending_defs: vec![],
                    term_start: start,
                    last_end: start,
                    tight: true,
                    start,
                });
            }
            #[cfg(feature = "definition-lists")]
            Event::Start(Tag::DefinitionListTitle) => {
                stack.push(Frame::DefinitionListTitle {
                    inlines: vec![],
                    start,
                });
            }
            #[cfg(feature = "definition-lists")]
            Event::Start(Tag::DefinitionListDefinition) => {
                stack.push(Frame::DefinitionListDefinition {
                    blocks: vec![],
                    tight_para: false,
                    tight_inlines: vec![],
                });
            }

            // ── Inline opens ──────────────────────────────────────────────────
            Event::Start(Tag::Emphasis) => {
                stack.push(Frame::Emphasis {
                    inlines: vec![],
                    start,
                });
            }
            Event::Start(Tag::Strong) => {
                stack.push(Frame::Strong {
                    inlines: vec![],
                    start,
                });
            }
            #[cfg(feature = "strikethrough")]
            Event::Start(Tag::Strikethrough) => {
                stack.push(Frame::Strikethrough {
                    inlines: vec![],
                    start,
                });
            }
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                title,
                ..
            }) => {
                let raw_url = dest_url.into_string();
                let url = if link_type == pulldown_cmark::LinkType::Email
                    && !raw_url.starts_with("mailto:")
                {
                    format!("mailto:{raw_url}")
                } else {
                    raw_url
                };
                let title = if title.is_empty() {
                    None
                } else {
                    Some(title.into_string())
                };
                stack.push(Frame::Link {
                    inlines: vec![],
                    url,
                    title,
                    start,
                });
            }
            Event::Start(Tag::Image {
                dest_url, title, ..
            }) => {
                let url = dest_url.into_string();
                let title = if title.is_empty() {
                    None
                } else {
                    Some(title.into_string())
                };
                stack.push(Frame::Image {
                    alt: String::new(),
                    url,
                    title,
                    start,
                });
            }

            // ── Closes ────────────────────────────────────────────────────────
            Event::End(TagEnd::Paragraph) => {
                let frame = stack.pop();
                if let Some(Frame::Paragraph { inlines, start: s }) = frame {
                    let block = Block::Paragraph {
                        inlines,
                        span: Span { start: s, end },
                    };
                    // If inside an item, mark it as having an explicit paragraph child
                    // (→ loose list).
                    if let Some(Frame::Item {
                        blocks, tight_para, ..
                    }) = stack.last_mut()
                    {
                        *tight_para = true;
                        blocks.push(block);
                    } else if push_tight_paragraph_to_definition(&mut stack, &block) {
                        // handled inside push_tight_paragraph_to_definition
                    } else {
                        push_block(&mut stack, block);
                    }
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                let frame = stack.pop();
                if let Some(Frame::Heading {
                    level,
                    inlines,
                    start: s,
                }) = frame
                {
                    let block = Block::Heading {
                        level,
                        inlines,
                        span: Span { start: s, end },
                    };
                    push_block(&mut stack, block);
                }
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                let frame = stack.pop();
                if let Some(Frame::Blockquote { blocks, start: s }) = frame {
                    let block = Block::Blockquote {
                        blocks,
                        span: Span { start: s, end },
                    };
                    push_block(&mut stack, block);
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                let frame = stack.pop();
                if let Some(Frame::CodeBlock {
                    language,
                    content,
                    start: s,
                }) = frame
                {
                    let block = Block::CodeBlock {
                        language,
                        content,
                        span: Span { start: s, end },
                    };
                    push_block(&mut stack, block);
                }
            }
            Event::End(TagEnd::List(_)) => {
                let frame = stack.pop();
                if let Some(Frame::List {
                    kind,
                    items,
                    tight,
                    start: s,
                }) = frame
                {
                    let block = Block::List {
                        kind,
                        items,
                        tight,
                        span: Span { start: s, end },
                    };
                    push_block(&mut stack, block);
                }
            }
            Event::End(TagEnd::Item) => {
                let frame = stack.pop();
                if let Some(Frame::Item {
                    mut blocks,
                    tight_para,
                    tight_inlines,
                    #[cfg(feature = "task-lists")]
                    checked,
                    start: s,
                }) = frame
                {
                    // Tight list items accumulate inlines directly (no Paragraph wrapper
                    // from pulldown). Wrap them in an implicit paragraph so every item
                    // always has Block children — consistent with loose items. (Any
                    // tight_inlines preceding a nested block were already flushed by
                    // push_block, in source order; this handles inlines that are the
                    // item's only/trailing content.)
                    let mut tight_inlines = tight_inlines;
                    flush_tight_inlines(&mut blocks, &mut tight_inlines);
                    let item = ListItem {
                        blocks,
                        span: Span { start: s, end },
                        #[cfg(feature = "task-lists")]
                        checked,
                    };
                    // If this item had explicit paragraphs, mark the parent list as loose.
                    if tight_para && let Some(Frame::List { tight, .. }) = stack.last_mut() {
                        *tight = false;
                    }
                    if let Some(Frame::List { items, .. }) = stack.last_mut() {
                        items.push(item);
                    }
                }
            }
            Event::End(TagEnd::HtmlBlock) => {
                let frame = stack.pop();
                if let Some(Frame::HtmlBlock { content, start: s }) = frame {
                    let block = Block::HtmlBlock {
                        content,
                        span: Span { start: s, end },
                    };
                    push_block(&mut stack, block);
                }
            }
            Event::End(TagEnd::Emphasis) => {
                let frame = stack.pop();
                if let Some(Frame::Emphasis { inlines, start: s }) = frame {
                    let inline = Inline::Emphasis {
                        inlines,
                        span: Span { start: s, end },
                    };
                    push_inline(&mut stack, inline);
                }
            }
            Event::End(TagEnd::Strong) => {
                let frame = stack.pop();
                if let Some(Frame::Strong { inlines, start: s }) = frame {
                    let inline = Inline::Strong {
                        inlines,
                        span: Span { start: s, end },
                    };
                    push_inline(&mut stack, inline);
                }
            }
            #[cfg(feature = "strikethrough")]
            Event::End(TagEnd::Strikethrough) => {
                let frame = stack.pop();
                if let Some(Frame::Strikethrough { inlines, start: s }) = frame {
                    let inline = Inline::Strikethrough {
                        inlines,
                        span: Span { start: s, end },
                    };
                    push_inline(&mut stack, inline);
                }
            }
            #[cfg(feature = "frontmatter")]
            Event::End(TagEnd::MetadataBlock(_)) => {
                let frame = stack.pop();
                if let Some(Frame::FrontMatter {
                    kind,
                    content,
                    start: s,
                }) = frame
                {
                    // First front-matter block wins, matching pulldown-cmark's own
                    // one-per-document behavior.
                    frontmatter.get_or_insert(FrontMatter {
                        kind,
                        content,
                        span: Span { start: s, end },
                    });
                }
            }
            #[cfg(feature = "tables")]
            Event::End(TagEnd::TableCell) => {
                let frame = stack.pop();
                if let Some(Frame::TableCell { inlines, start: s }) = frame {
                    let cell = TableCell {
                        inlines,
                        span: Span { start: s, end },
                    };
                    if let Some(Frame::TableRow { cells, .. }) = stack.last_mut() {
                        cells.push(cell);
                    }
                }
            }
            #[cfg(feature = "tables")]
            Event::End(TagEnd::TableHead) => {
                let frame = stack.pop();
                if let Some(Frame::TableRow {
                    cells, start: s, ..
                }) = frame
                {
                    let row = TableRow {
                        cells,
                        span: Span { start: s, end },
                    };
                    if let Some(Frame::Table { head, .. }) = stack.last_mut() {
                        *head = Some(row);
                    }
                }
            }
            #[cfg(feature = "tables")]
            Event::End(TagEnd::TableRow) => {
                let frame = stack.pop();
                if let Some(Frame::TableRow {
                    cells, start: s, ..
                }) = frame
                {
                    let row = TableRow {
                        cells,
                        span: Span { start: s, end },
                    };
                    if let Some(Frame::Table { rows, .. }) = stack.last_mut() {
                        rows.push(row);
                    }
                }
            }
            #[cfg(feature = "tables")]
            Event::End(TagEnd::Table) => {
                let frame = stack.pop();
                if let Some(Frame::Table {
                    alignments,
                    head,
                    rows,
                    start: s,
                }) = frame
                {
                    let block = Block::Table {
                        alignments,
                        head: head.unwrap_or(TableRow {
                            cells: vec![],
                            span: Span::NONE,
                        }),
                        rows,
                        span: Span { start: s, end },
                    };
                    push_block(&mut stack, block);
                }
            }
            #[cfg(feature = "footnotes")]
            Event::End(TagEnd::FootnoteDefinition) => {
                let frame = stack.pop();
                if let Some(Frame::FootnoteDefinition {
                    label,
                    blocks,
                    start: s,
                }) = frame
                {
                    let block = Block::FootnoteDefinition {
                        label,
                        blocks,
                        span: Span { start: s, end },
                    };
                    push_block(&mut stack, block);
                }
            }
            #[cfg(feature = "definition-lists")]
            Event::End(TagEnd::DefinitionListTitle) => {
                let frame = stack.pop();
                if let Some(Frame::DefinitionListTitle {
                    inlines,
                    start: title_start,
                }) = frame
                    && let Some(Frame::DefinitionList {
                        items,
                        pending_term,
                        pending_defs,
                        term_start,
                        last_end,
                        ..
                    }) = stack.last_mut()
                {
                    if let Some(term) = pending_term.take() {
                        items.push(DefinitionListItem {
                            term,
                            definitions: std::mem::take(pending_defs),
                            span: Span {
                                start: *term_start,
                                end: *last_end,
                            },
                        });
                    }
                    *pending_term = Some(inlines);
                    *term_start = title_start;
                    *last_end = end;
                }
            }
            #[cfg(feature = "definition-lists")]
            Event::End(TagEnd::DefinitionListDefinition) => {
                let frame = stack.pop();
                if let Some(Frame::DefinitionListDefinition {
                    mut blocks,
                    tight_para,
                    mut tight_inlines,
                    ..
                }) = frame
                {
                    flush_tight_inlines(&mut blocks, &mut tight_inlines);
                    if tight_para
                        && let Some(Frame::DefinitionList { tight, .. }) = stack.last_mut()
                    {
                        *tight = false;
                    }
                    if let Some(Frame::DefinitionList {
                        pending_defs,
                        last_end,
                        ..
                    }) = stack.last_mut()
                    {
                        pending_defs.push(blocks);
                        *last_end = end;
                    }
                }
            }
            #[cfg(feature = "definition-lists")]
            Event::End(TagEnd::DefinitionList) => {
                let frame = stack.pop();
                if let Some(Frame::DefinitionList {
                    mut items,
                    pending_term,
                    pending_defs,
                    term_start,
                    last_end,
                    tight,
                    start: s,
                }) = frame
                {
                    if let Some(term) = pending_term {
                        items.push(DefinitionListItem {
                            term,
                            definitions: pending_defs,
                            span: Span {
                                start: term_start,
                                end: last_end,
                            },
                        });
                    }
                    let block = Block::DefinitionList {
                        items,
                        tight,
                        span: Span { start: s, end },
                    };
                    push_block(&mut stack, block);
                }
            }
            Event::End(TagEnd::Link) => {
                let frame = stack.pop();
                if let Some(Frame::Link {
                    inlines,
                    url,
                    title,
                    start: s,
                }) = frame
                {
                    let inline = Inline::Link {
                        inlines,
                        url,
                        title,
                        span: Span { start: s, end },
                    };
                    push_inline(&mut stack, inline);
                }
            }
            Event::End(TagEnd::Image) => {
                let frame = stack.pop();
                if let Some(Frame::Image {
                    alt,
                    url,
                    title,
                    start: s,
                }) = frame
                {
                    let inline = Inline::Image {
                        alt,
                        url,
                        title,
                        span: Span { start: s, end },
                    };
                    push_inline(&mut stack, inline);
                }
            }

            // ── Leaf events ───────────────────────────────────────────────────
            Event::Text(text) => {
                let s = text.into_string();
                // Text events inside an image frame accumulate the alt text.
                if let Some(Frame::Image { alt, .. }) = stack.last_mut() {
                    alt.push_str(&s);
                } else if let Some(Frame::CodeBlock { content, .. }) = stack.last_mut() {
                    content.push_str(&s);
                } else if let Some(Frame::HtmlBlock { content, .. }) = stack.last_mut() {
                    content.push_str(&s);
                } else if push_frontmatter_text(&mut stack, &s) {
                    // handled inside push_frontmatter_text
                } else {
                    let inline = Inline::Text {
                        content: s,
                        span: Span { start, end },
                    };
                    push_inline(&mut stack, inline);
                }
            }
            Event::Code(text) => {
                let inline = Inline::Code {
                    content: text.into_string(),
                    span: Span { start, end },
                };
                push_inline(&mut stack, inline);
            }
            Event::Html(text) => {
                // Html events are block-level raw HTML; they arrive while HtmlBlock frame is on stack.
                if let Some(Frame::HtmlBlock { content, .. }) = stack.last_mut() {
                    content.push_str(&text);
                } else {
                    // Unexpected Html event outside HtmlBlock frame — treat as HtmlBlock directly.
                    let block = Block::HtmlBlock {
                        content: text.into_string(),
                        span: Span { start, end },
                    };
                    push_block(&mut stack, block);
                }
            }
            Event::InlineHtml(text) => {
                let inline = Inline::HtmlInline {
                    content: text.into_string(),
                    span: Span { start, end },
                };
                push_inline(&mut stack, inline);
            }
            Event::SoftBreak => {
                let inline = Inline::SoftBreak {
                    span: Span { start, end },
                };
                push_inline(&mut stack, inline);
            }
            Event::HardBreak => {
                let inline = Inline::HardBreak {
                    span: Span { start, end },
                };
                push_inline(&mut stack, inline);
            }
            Event::Rule => {
                let block = Block::ThematicBreak {
                    span: Span { start, end },
                };
                push_block(&mut stack, block);
            }
            #[cfg(feature = "task-lists")]
            Event::TaskListMarker(checked) => {
                if let Some(Frame::Item { checked: c, .. }) = stack.last_mut() {
                    *c = Some(checked);
                }
            }
            #[cfg(feature = "footnotes")]
            Event::FootnoteReference(label) => {
                let inline = Inline::FootnoteReference {
                    label: label.into_string(),
                    span: Span { start, end },
                };
                push_inline(&mut stack, inline);
            }
            #[cfg(feature = "math")]
            Event::InlineMath(math) => {
                let inline = Inline::InlineMath {
                    source: math.into_string(),
                    span: Span { start, end },
                };
                push_inline(&mut stack, inline);
            }
            #[cfg(feature = "math")]
            Event::DisplayMath(math) => {
                let inline = Inline::DisplayMath {
                    source: math.into_string(),
                    span: Span { start, end },
                };
                push_inline(&mut stack, inline);
            }

            // ── Ignored events ───────────────────────────────────────────────
            // Reached only when the corresponding construct feature is off —
            // its Options bit is never set, so pulldown-cmark never produces
            // that event for us to see here (e.g. TaskListMarker with
            // `task-lists` off, FootnoteReference with `footnotes` off).
            _ => {}
        }
    }

    // Drain the root Doc frame.
    let blocks = match stack.into_iter().next() {
        Some(Frame::Doc { blocks }) => blocks,
        _ => vec![],
    };

    (
        CmDoc {
            blocks,
            link_defs,
            #[cfg(feature = "frontmatter")]
            frontmatter,
        },
        diagnostics,
    )
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Accumulate text into the top-of-stack front-matter frame, if present.
/// Returns `true` if the text was consumed this way.
#[cfg(feature = "frontmatter")]
fn push_frontmatter_text(stack: &mut [Frame], s: &str) -> bool {
    if let Some(Frame::FrontMatter { content, .. }) = stack.last_mut() {
        content.push_str(s);
        true
    } else {
        false
    }
}
#[cfg(not(feature = "frontmatter"))]
fn push_frontmatter_text(_stack: &mut [Frame], _s: &str) -> bool {
    false
}

/// If the top-of-stack frame is a `DefinitionListDefinition`, record that it
/// had an explicit `Paragraph` child (marking the whole list loose, mirroring
/// `Frame::Item`'s identical signal for `Frame::List`) and push `block` onto
/// it. Returns `true` if handled this way.
#[cfg(feature = "definition-lists")]
fn push_tight_paragraph_to_definition(stack: &mut [Frame], block: &Block) -> bool {
    if let Some(Frame::DefinitionListDefinition {
        blocks, tight_para, ..
    }) = stack.last_mut()
    {
        *tight_para = true;
        blocks.push(block.clone());
        true
    } else {
        false
    }
}
#[cfg(not(feature = "definition-lists"))]
fn push_tight_paragraph_to_definition(_stack: &mut [Frame], _block: &Block) -> bool {
    false
}

/// Push a completed block onto the nearest block-accepting frame.
fn push_block(stack: &mut [Frame], block: Block) {
    for frame in stack.iter_mut().rev() {
        match frame {
            Frame::Doc { blocks } | Frame::Blockquote { blocks, .. } => {
                blocks.push(block);
                return;
            }
            Frame::Item {
                blocks,
                tight_inlines,
                ..
            } => {
                // A tight list item can start with inline content (no
                // Paragraph wrapper from pulldown) followed by a nested
                // block (e.g. a sublist): "- outer\n  - inner\n". The
                // leading inlines accumulate in `tight_inlines` and are
                // normally flushed into an implicit paragraph at End(Item)
                // — but a sibling block like this nested list arrives via
                // push_block *before* End(Item) fires, so without flushing
                // here first, the block would land ahead of text that
                // precedes it in the source. Flush any pending tight_inlines
                // now, before appending, to preserve source order.
                flush_tight_inlines(blocks, tight_inlines);
                blocks.push(block);
                return;
            }
            #[cfg(feature = "footnotes")]
            Frame::FootnoteDefinition { blocks, .. } => {
                blocks.push(block);
                return;
            }
            #[cfg(feature = "definition-lists")]
            Frame::DefinitionListDefinition {
                blocks,
                tight_inlines,
                ..
            } => {
                flush_tight_inlines(blocks, tight_inlines);
                blocks.push(block);
                return;
            }
            _ => {}
        }
    }
}

/// Flush accumulated tight-list-item inline content into an implicit
/// `Block::Paragraph`, using the span bounds of the flushed inlines
/// themselves (not the enclosing item's span).
fn flush_tight_inlines(blocks: &mut Vec<Block>, tight_inlines: &mut Vec<Inline>) {
    if tight_inlines.is_empty() {
        return;
    }
    let inlines = std::mem::take(tight_inlines);
    let span = Span {
        start: inline_span_start(&inlines[0]),
        end: inline_span_end(&inlines[inlines.len() - 1]),
    };
    blocks.push(Block::Paragraph { inlines, span });
}

fn inline_span_start(inline: &Inline) -> usize {
    inline_span(inline).start
}
fn inline_span_end(inline: &Inline) -> usize {
    inline_span(inline).end
}
fn inline_span(inline: &Inline) -> Span {
    match inline {
        Inline::Text { span, .. }
        | Inline::SoftBreak { span }
        | Inline::HardBreak { span }
        | Inline::Emphasis { span, .. }
        | Inline::Strong { span, .. }
        | Inline::Code { span, .. }
        | Inline::HtmlInline { span, .. }
        | Inline::Link { span, .. }
        | Inline::Image { span, .. } => *span,
        #[cfg(feature = "strikethrough")]
        Inline::Strikethrough { span, .. } => *span,
        #[cfg(feature = "footnotes")]
        Inline::FootnoteReference { span, .. } => *span,
        #[cfg(feature = "math")]
        Inline::InlineMath { span, .. } | Inline::DisplayMath { span, .. } => *span,
    }
}

/// Push a completed inline onto the nearest inline-accepting frame.
///
/// For tight list items, pulldown-cmark does not emit `Start/End(Paragraph)` events;
/// inlines arrive with only `Frame::Item` on the stack. We accumulate them in
/// `Frame::Item::tight_inlines` and wrap them in a `Block::Paragraph` at `End(Item)`.
fn push_inline(stack: &mut [Frame], inline: Inline) {
    for frame in stack.iter_mut().rev() {
        let target: &mut Vec<Inline> = match frame {
            Frame::Paragraph { inlines, .. }
            | Frame::Heading { inlines, .. }
            | Frame::Emphasis { inlines, .. }
            | Frame::Strong { inlines, .. }
            | Frame::Link { inlines, .. } => inlines,
            #[cfg(feature = "strikethrough")]
            Frame::Strikethrough { inlines, .. } => inlines,
            #[cfg(feature = "tables")]
            Frame::TableCell { inlines, .. } => inlines,
            #[cfg(feature = "definition-lists")]
            Frame::DefinitionListTitle { inlines, .. } => inlines,
            // Tight list item: accumulate inlines for later wrapping in a paragraph.
            Frame::Item { tight_inlines, .. } => tight_inlines,
            // Tight definition body: same shortcut as tight list items.
            #[cfg(feature = "definition-lists")]
            Frame::DefinitionListDefinition { tight_inlines, .. } => tight_inlines,
            // Image alt text is handled before push_inline is called.
            _ => continue,
        };
        // Merge consecutive Text nodes — pulldown-cmark can split a single logical
        // text run into multiple Text events (e.g. backslash escapes).
        if let (
            Inline::Text {
                content: new_content,
                span: new_span,
            },
            Some(Inline::Text { content, span }),
        ) = (&inline, target.last_mut())
        {
            content.push_str(new_content);
            span.end = new_span.end;
            return;
        }
        target.push(inline);
        return;
    }
}

fn heading_level_to_u8(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// Collect reference link definitions from the input string.
///
/// pulldown-cmark exposes these through [`Parser::reference_definitions`].
/// `pub(crate)` so `events.rs` can reuse the exact same extraction (and
/// deterministic sort order) rather than duplicating it — `events()` needs
/// the same `Vec<LinkDef>` `parse()` builds, since pulldown-cmark's event
/// stream itself never surfaces reference definitions.
pub(crate) fn collect_link_defs(input: &str) -> Vec<LinkDef> {
    let opts = build_options();
    let parser = Parser::new_ext(input, opts);
    let defs = parser.reference_definitions();
    let mut out: Vec<LinkDef> = defs
        .iter()
        .map(|(label, def)| LinkDef {
            label: label.to_string(),
            url: def.dest.to_string(),
            title: def.title.as_ref().map(|t| t.to_string()),
        })
        .collect();
    // Sort for deterministic output.
    out.sort_by(|a, b| a.label.cmp(&b.label));
    out
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paragraph() {
        let (doc, diags) = parse(b"Hello, world!");
        assert!(diags.is_empty());
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(&doc.blocks[0], Block::Paragraph { .. }));
    }

    #[test]
    fn test_heading() {
        let (doc, diags) = parse(b"# Heading 1\n\n## Heading 2\n");
        assert!(diags.is_empty());
        assert_eq!(doc.blocks.len(), 2);
        assert!(matches!(&doc.blocks[0], Block::Heading { level: 1, .. }));
        assert!(matches!(&doc.blocks[1], Block::Heading { level: 2, .. }));
    }

    #[test]
    fn test_emphasis_and_strong() {
        let (doc, diags) = parse(b"*em* and **strong**");
        assert!(diags.is_empty());
        if let Block::Paragraph { inlines, .. } = &doc.blocks[0] {
            assert!(inlines.iter().any(|i| matches!(i, Inline::Emphasis { .. })));
            assert!(inlines.iter().any(|i| matches!(i, Inline::Strong { .. })));
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    fn test_code_block() {
        let (doc, diags) = parse(b"```rust\nfn main() {}\n```\n");
        assert!(diags.is_empty());
        assert!(matches!(
            &doc.blocks[0],
            Block::CodeBlock { language: Some(lang), content, .. }
            if lang == "rust" && content == "fn main() {}\n"
        ));
    }

    #[test]
    fn test_unordered_list() {
        let (doc, diags) = parse(b"- one\n- two\n- three\n");
        assert!(diags.is_empty());
        assert!(matches!(
            &doc.blocks[0],
            Block::List {
                kind: ListKind::Unordered { .. },
                ..
            }
        ));
        if let Block::List { items, .. } = &doc.blocks[0] {
            assert_eq!(items.len(), 3);
        }
    }

    #[test]
    fn test_ordered_list() {
        let (doc, diags) = parse(b"1. first\n2. second\n");
        assert!(diags.is_empty());
        assert!(matches!(
            &doc.blocks[0],
            Block::List {
                kind: ListKind::Ordered { start: 1, .. },
                ..
            }
        ));
    }

    #[test]
    fn test_blockquote() {
        let (doc, diags) = parse(b"> A quoted paragraph.\n");
        assert!(diags.is_empty());
        assert!(matches!(&doc.blocks[0], Block::Blockquote { .. }));
    }

    #[test]
    fn test_thematic_break() {
        let (doc, diags) = parse(b"---\n");
        assert!(diags.is_empty());
        assert!(matches!(&doc.blocks[0], Block::ThematicBreak { .. }));
    }

    #[test]
    fn test_link() {
        let (doc, diags) = parse(b"[text](https://example.com)\n");
        assert!(diags.is_empty());
        if let Block::Paragraph { inlines, .. } = &doc.blocks[0] {
            assert!(inlines.iter().any(|i| matches!(i, Inline::Link { .. })));
        }
    }

    #[test]
    fn test_image() {
        let (doc, diags) = parse(b"![alt text](img.png)\n");
        assert!(diags.is_empty());
        if let Block::Paragraph { inlines, .. } = &doc.blocks[0] {
            assert!(
                inlines
                    .iter()
                    .any(|i| matches!(i, Inline::Image { alt, .. } if alt == "alt text"))
            );
        }
    }

    #[test]
    fn test_html_block() {
        let (doc, diags) = parse(b"<div>\ncontent\n</div>\n");
        assert!(diags.is_empty());
        assert!(matches!(&doc.blocks[0], Block::HtmlBlock { .. }));
    }

    #[test]
    fn test_inline_html() {
        let (doc, diags) = parse(b"text <em>inline</em> html\n");
        assert!(diags.is_empty());
        if let Block::Paragraph { inlines, .. } = &doc.blocks[0] {
            assert!(
                inlines
                    .iter()
                    .any(|i| matches!(i, Inline::HtmlInline { .. }))
            );
        }
    }

    #[test]
    fn test_invalid_utf8() {
        let (doc, diags) = parse(b"\xff\xfe");
        assert_eq!(doc.blocks.len(), 0);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "commonmark::invalid-utf8");
    }

    #[test]
    fn test_link_def() {
        let (doc, diags) = parse(b"[link][ref]\n\n[ref]: https://example.com\n");
        assert!(diags.is_empty());
        assert_eq!(doc.link_defs.len(), 1);
        assert_eq!(doc.link_defs[0].url, "https://example.com");
    }

    #[test]
    fn test_strip_spans() {
        let (doc, _) = parse(b"# Hello\n\nA paragraph.\n");
        let stripped = doc.strip_spans();
        for block in &stripped.blocks {
            match block {
                Block::Heading { span, .. } | Block::Paragraph { span, .. } => {
                    assert_eq!(*span, Span::NONE);
                }
                _ => {}
            }
        }
    }

    #[test]
    fn test_loose_list() {
        // A blank line between items makes a loose list.
        let (doc, _) = parse(b"- item one\n\n- item two\n");
        if let Block::List { tight, .. } = &doc.blocks[0] {
            assert!(!tight, "list with blank-separated items should be loose");
        } else {
            panic!("expected list");
        }
    }

    #[test]
    fn test_tight_list() {
        let (doc, _) = parse(b"- item one\n- item two\n");
        if let Block::List { tight, .. } = &doc.blocks[0] {
            assert!(*tight, "list without blank lines should be tight");
        } else {
            panic!("expected list");
        }
    }

    #[test]
    #[cfg(feature = "strikethrough")]
    fn test_gfm_strikethrough() {
        let (doc, diags) = parse(b"~~deleted~~\n");
        assert!(diags.is_empty());
        if let Block::Paragraph { inlines, .. } = &doc.blocks[0] {
            assert!(
                inlines
                    .iter()
                    .any(|i| matches!(i, Inline::Strikethrough { .. }))
            );
        }
    }

    #[test]
    #[cfg(not(feature = "strikethrough"))]
    fn test_no_strikethrough_by_default() {
        // Without the `strikethrough` feature, `~~text~~` is plain CommonMark
        // (tildes are literal characters), matching the CommonMark spec.
        let (doc, diags) = parse(b"~~deleted~~\n");
        assert!(diags.is_empty());
        if let Block::Paragraph { inlines, .. } = &doc.blocks[0] {
            assert!(
                inlines
                    .iter()
                    .any(|i| matches!(i, Inline::Text { content, .. } if content.contains("~~")))
            );
        }
    }

    #[test]
    #[cfg(feature = "frontmatter")]
    fn test_yaml_frontmatter() {
        let (doc, diags) = parse(b"---\ntitle: X\n---\n\nbody\n");
        assert!(diags.is_empty());
        let fm = doc.frontmatter.as_ref().expect("expected frontmatter");
        assert_eq!(fm.kind, FrontMatterKind::Yaml);
        assert_eq!(fm.content.trim(), "title: X");
        // No bogus ThematicBreak/Heading in the body.
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(&doc.blocks[0], Block::Paragraph { .. }));
    }

    #[test]
    #[cfg(feature = "frontmatter")]
    fn test_toml_frontmatter() {
        let (doc, diags) = parse(b"+++\ntitle = \"X\"\n+++\n\nbody\n");
        assert!(diags.is_empty());
        let fm = doc.frontmatter.as_ref().expect("expected frontmatter");
        assert_eq!(fm.kind, FrontMatterKind::Toml);
        assert_eq!(fm.content.trim(), "title = \"X\"");
        assert_eq!(doc.blocks.len(), 1);
    }

    #[test]
    #[cfg(feature = "tables")]
    fn test_table() {
        let (doc, diags) = parse(b"| a | b |\n|---|---|\n| 1 | 2 |\n");
        assert!(diags.is_empty());
        assert!(matches!(&doc.blocks[0], Block::Table { .. }));
        if let Block::Table {
            head,
            rows,
            alignments,
            ..
        } = &doc.blocks[0]
        {
            assert_eq!(alignments.len(), 2);
            assert_eq!(head.cells.len(), 2);
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].cells.len(), 2);
        }
    }

    #[test]
    #[cfg(feature = "footnotes")]
    fn test_footnote() {
        let (doc, diags) = parse(b"Text.[^1]\n\n[^1]: A note.\n");
        assert!(diags.is_empty());
        if let Block::Paragraph { inlines, .. } = &doc.blocks[0] {
            assert!(
                inlines
                    .iter()
                    .any(|i| matches!(i, Inline::FootnoteReference { label, .. } if label == "1"))
            );
        } else {
            panic!("expected paragraph");
        }
        assert!(matches!(
            &doc.blocks[1],
            Block::FootnoteDefinition { label, .. } if label == "1"
        ));
        if let Block::FootnoteDefinition { blocks, .. } = &doc.blocks[1] {
            assert_eq!(blocks.len(), 1);
            assert!(matches!(&blocks[0], Block::Paragraph { .. }));
        }
    }

    #[test]
    #[cfg(feature = "definition-lists")]
    fn test_definition_list_tight() {
        let (doc, diags) = parse(b"apple\n:   red fruit\n\norange\n:   orange fruit\n");
        assert!(diags.is_empty());
        assert!(matches!(
            &doc.blocks[0],
            Block::DefinitionList { tight: true, .. }
        ));
        if let Block::DefinitionList { items, .. } = &doc.blocks[0] {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].definitions.len(), 1);
            assert_eq!(items[0].definitions[0].len(), 1);
            assert!(matches!(
                &items[0].definitions[0][0],
                Block::Paragraph { .. }
            ));
        }
    }

    #[test]
    #[cfg(feature = "definition-lists")]
    fn test_definition_list_loose() {
        let (doc, diags) = parse(b"apple\n\n:   red fruit\n\norange\n\n:   orange fruit\n");
        assert!(diags.is_empty());
        assert!(matches!(
            &doc.blocks[0],
            Block::DefinitionList { tight: false, .. }
        ));
    }

    #[test]
    #[cfg(feature = "definition-lists")]
    fn test_definition_list_multi_def() {
        let (doc, diags) = parse(b"apple\n:   red fruit\n:   computer company\n");
        assert!(diags.is_empty());
        if let Block::DefinitionList { items, .. } = &doc.blocks[0] {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].definitions.len(), 2);
        } else {
            panic!("expected definition list");
        }
    }

    #[test]
    #[cfg(feature = "math")]
    fn test_inline_math() {
        let (doc, diags) = parse(b"Euler's identity: $e^{i\\pi}+1=0$\n");
        assert!(diags.is_empty());
        if let Block::Paragraph { inlines, .. } = &doc.blocks[0] {
            assert!(inlines.iter().any(
                |i| matches!(i, Inline::InlineMath { source, .. } if source == "e^{i\\pi}+1=0")
            ));
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    #[cfg(feature = "math")]
    fn test_display_math() {
        let (doc, diags) = parse(b"$$a^2+b^2=c^2$$\n");
        assert!(diags.is_empty());
        if let Block::Paragraph { inlines, .. } = &doc.blocks[0] {
            assert!(inlines.iter().any(
                |i| matches!(i, Inline::DisplayMath { source, .. } if source == "a^2+b^2=c^2")
            ));
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    #[cfg(feature = "task-lists")]
    fn test_task_list() {
        let (doc, diags) = parse(b"- [ ] todo\n- [x] done\n");
        assert!(diags.is_empty());
        if let Block::List { items, .. } = &doc.blocks[0] {
            assert_eq!(items[0].checked, Some(false));
            assert_eq!(items[1].checked, Some(true));
        } else {
            panic!("expected list");
        }
    }
}
