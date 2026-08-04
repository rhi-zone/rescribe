//! AST types for MultiMarkdown documents.
//!
//! `MmdBlock`/`MmdInline` deliberately mirror `commonmark_fmt::{Block,
//! Inline}` variant-for-variant (same shapes, same feature gates) — every
//! construct commonmark-fmt already models is represented here by wrapping
//! its recursive children in `MmdBlock`/`MmdInline` instead of `Block`/
//! `Inline`, so a citation or cross-reference can appear anywhere plain
//! CommonMark text can (inside a blockquote, a table cell, a footnote, list
//! item, ...), not just at the top level. `commonmark_fmt::Span`,
//! `ListKind`, `OrderedMarker`, and (when the `tables` feature is on)
//! `ColumnAlignment` are reused directly — there is nothing MMD-specific
//! about them.
//!
//! Two variants have no commonmark-fmt equivalent — the genuinely
//! MMD-unique constructs this crate exists to add:
//! - [`MmdInline::Citation`] — `[locator][#refname]` / `[][#refname]`
//! - [`MmdInline::CrossReference`] — `[HeaderAnchor]` / `[Header Text][]`
//!
//! and one block variant, [`MmdBlock::CitationDefinition`], for
//! `[#refname]: citation text` definition lines (the citation analogue of
//! commonmark-fmt's `Block::FootnoteDefinition`).
//!
//! See `crate::transform` for the two directions of conversion between this
//! type and `commonmark_fmt::CmDoc`.

pub use commonmark_fmt::ColumnAlignment;
pub use commonmark_fmt::{LinkDef, ListKind, OrderedMarker, Span};

/// A parsed MultiMarkdown document.
#[derive(Clone, Debug, PartialEq)]
pub struct MmdDoc {
    /// Document metadata (`Key: value` lines at the top of the document).
    pub metadata: Vec<MetadataEntry>,
    /// How the metadata block, if any, was delimited in the source — needed
    /// so the writer reproduces the same style rather than always picking
    /// one.
    pub metadata_style: MetadataStyle,
    pub blocks: Vec<MmdBlock>,
    pub link_defs: Vec<LinkDef>,
}

impl MmdDoc {
    pub fn strip_spans(&self) -> MmdDoc {
        MmdDoc {
            metadata: self.metadata.clone(),
            metadata_style: self.metadata_style,
            blocks: self.blocks.iter().map(|b| b.strip_spans()).collect(),
            link_defs: self.link_defs.clone(),
        }
    }
}

/// How a document's metadata block was delimited.
///
/// MultiMarkdown's metadata grammar is distinct from CommonMark/YAML
/// frontmatter: the block never *requires* `---`/`...` delimiters (a bare
/// `Key: value` run at the very top of the document, ended by a blank
/// line, is the default form) — delimiters are an optional, purely
/// cosmetic addition for YAML-tooling compatibility. See
/// `crate::metadata`'s module doc for the full grammar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MetadataStyle {
    /// No metadata block present.
    #[default]
    None,
    /// `Key: value` lines with no delimiters.
    Bare,
    /// `---` ... `Key: value` ... `---` (or `...`).
    Delimited,
}

/// One `Key: value` metadata entry.
#[derive(Clone, Debug, PartialEq)]
pub struct MetadataEntry {
    pub key: String,
    pub value: String,
}

/// A block-level node.
#[derive(Clone, Debug, PartialEq)]
pub enum MmdBlock {
    Paragraph {
        inlines: Vec<MmdInline>,
        span: Span,
    },
    Heading {
        level: u8,
        inlines: Vec<MmdInline>,
        /// `### Overview [MultiMarkdownOverview] ##` — an explicit
        /// cross-reference anchor label trailing the heading text
        /// (MMD-unique). `None` when the heading has no explicit anchor
        /// (it may still be addressable by its exact text, per MMD's
        /// "reference unlabeled headers by their text" fallback — that
        /// resolution is a caller concern, not modeled here).
        anchor: Option<String>,
        span: Span,
    },
    CodeBlock {
        language: Option<String>,
        content: String,
        span: Span,
    },
    HtmlBlock {
        content: String,
        span: Span,
    },
    Blockquote {
        blocks: Vec<MmdBlock>,
        span: Span,
    },
    List {
        kind: ListKind,
        items: Vec<MmdListItem>,
        tight: bool,
        span: Span,
    },
    ThematicBreak {
        span: Span,
    },
    Table {
        alignments: Vec<ColumnAlignment>,
        head: MmdTableRow,
        rows: Vec<MmdTableRow>,
        span: Span,
    },
    FootnoteDefinition {
        label: String,
        blocks: Vec<MmdBlock>,
        span: Span,
    },
    DefinitionList {
        items: Vec<MmdDefinitionListItem>,
        tight: bool,
        span: Span,
    },
    /// `[#refname]: citation text` — a citation definition (MMD-unique; the
    /// citation counterpart of `FootnoteDefinition`). Only recognized when
    /// it is a paragraph's entire leading text — see `crate::citation`.
    CitationDefinition {
        label: String,
        content: Vec<MmdInline>,
        span: Span,
    },
}

impl MmdBlock {
    pub fn strip_spans(&self) -> MmdBlock {
        match self {
            MmdBlock::Paragraph { inlines, .. } => MmdBlock::Paragraph {
                inlines: inlines.iter().map(|i| i.strip_spans()).collect(),
                span: Span::NONE,
            },
            MmdBlock::Heading {
                level,
                inlines,
                anchor,
                ..
            } => MmdBlock::Heading {
                level: *level,
                inlines: inlines.iter().map(|i| i.strip_spans()).collect(),
                anchor: anchor.clone(),
                span: Span::NONE,
            },
            MmdBlock::CodeBlock {
                language, content, ..
            } => MmdBlock::CodeBlock {
                language: language.clone(),
                content: content.clone(),
                span: Span::NONE,
            },
            MmdBlock::HtmlBlock { content, .. } => MmdBlock::HtmlBlock {
                content: content.clone(),
                span: Span::NONE,
            },
            MmdBlock::Blockquote { blocks, .. } => MmdBlock::Blockquote {
                blocks: blocks.iter().map(|b| b.strip_spans()).collect(),
                span: Span::NONE,
            },
            MmdBlock::List {
                kind, items, tight, ..
            } => MmdBlock::List {
                kind: kind.clone(),
                items: items.iter().map(|i| i.strip_spans()).collect(),
                tight: *tight,
                span: Span::NONE,
            },
            MmdBlock::ThematicBreak { .. } => MmdBlock::ThematicBreak { span: Span::NONE },
            MmdBlock::Table {
                alignments,
                head,
                rows,
                ..
            } => MmdBlock::Table {
                alignments: alignments.clone(),
                head: head.strip_spans(),
                rows: rows.iter().map(|r| r.strip_spans()).collect(),
                span: Span::NONE,
            },
            MmdBlock::FootnoteDefinition { label, blocks, .. } => MmdBlock::FootnoteDefinition {
                label: label.clone(),
                blocks: blocks.iter().map(|b| b.strip_spans()).collect(),
                span: Span::NONE,
            },
            MmdBlock::DefinitionList { items, tight, .. } => MmdBlock::DefinitionList {
                items: items.iter().map(|i| i.strip_spans()).collect(),
                tight: *tight,
                span: Span::NONE,
            },
            MmdBlock::CitationDefinition { label, content, .. } => MmdBlock::CitationDefinition {
                label: label.clone(),
                content: content.iter().map(|i| i.strip_spans()).collect(),
                span: Span::NONE,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MmdTableRow {
    pub cells: Vec<MmdTableCell>,
    pub span: Span,
}

impl MmdTableRow {
    pub fn strip_spans(&self) -> MmdTableRow {
        MmdTableRow {
            cells: self.cells.iter().map(|c| c.strip_spans()).collect(),
            span: Span::NONE,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MmdTableCell {
    pub inlines: Vec<MmdInline>,
    pub span: Span,
}

impl MmdTableCell {
    pub fn strip_spans(&self) -> MmdTableCell {
        MmdTableCell {
            inlines: self.inlines.iter().map(|i| i.strip_spans()).collect(),
            span: Span::NONE,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MmdDefinitionListItem {
    pub term: Vec<MmdInline>,
    pub definitions: Vec<Vec<MmdBlock>>,
    pub span: Span,
}

impl MmdDefinitionListItem {
    pub fn strip_spans(&self) -> MmdDefinitionListItem {
        MmdDefinitionListItem {
            term: self.term.iter().map(|i| i.strip_spans()).collect(),
            definitions: self
                .definitions
                .iter()
                .map(|def| def.iter().map(|b| b.strip_spans()).collect())
                .collect(),
            span: Span::NONE,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MmdListItem {
    pub blocks: Vec<MmdBlock>,
    pub span: Span,
    pub checked: Option<bool>,
}

impl MmdListItem {
    pub fn strip_spans(&self) -> MmdListItem {
        MmdListItem {
            blocks: self.blocks.iter().map(|b| b.strip_spans()).collect(),
            span: Span::NONE,
            checked: self.checked,
        }
    }
}

/// An inline node.
#[derive(Clone, Debug, PartialEq)]
pub enum MmdInline {
    Text {
        content: String,
        span: Span,
    },
    SoftBreak {
        span: Span,
    },
    HardBreak {
        span: Span,
    },
    Emphasis {
        inlines: Vec<MmdInline>,
        span: Span,
    },
    Strong {
        inlines: Vec<MmdInline>,
        span: Span,
    },
    Strikethrough {
        inlines: Vec<MmdInline>,
        span: Span,
    },
    Code {
        content: String,
        span: Span,
    },
    HtmlInline {
        content: String,
        span: Span,
    },
    Link {
        inlines: Vec<MmdInline>,
        url: String,
        title: Option<String>,
        span: Span,
    },
    Image {
        alt: String,
        url: String,
        title: Option<String>,
        span: Span,
    },
    FootnoteReference {
        label: String,
        span: Span,
    },
    InlineMath {
        source: String,
        span: Span,
    },
    DisplayMath {
        source: String,
        span: Span,
    },
    /// `[locator][#refname]` or `[][#refname]` (MMD-unique). `locator` is
    /// `None` for the empty-brackets form.
    Citation {
        locator: Option<String>,
        label: String,
        span: Span,
    },
    /// `[HeaderAnchor]` (shortcut) or `[Header Text][]` (collapsed)
    /// (MMD-unique). `target` is the raw label text, captured verbatim —
    /// resolving it against an actual heading anchor is a caller concern
    /// (e.g. the rescribe IR adapter), not this crate's.
    CrossReference {
        target: String,
        /// `true` for the `[target][]` collapsed form, `false` for the bare
        /// `[target]` shortcut form.
        collapsed: bool,
        span: Span,
    },
}

impl MmdInline {
    pub fn strip_spans(&self) -> MmdInline {
        match self {
            MmdInline::Text { content, .. } => MmdInline::Text {
                content: content.clone(),
                span: Span::NONE,
            },
            MmdInline::SoftBreak { .. } => MmdInline::SoftBreak { span: Span::NONE },
            MmdInline::HardBreak { .. } => MmdInline::HardBreak { span: Span::NONE },
            MmdInline::Emphasis { inlines, .. } => MmdInline::Emphasis {
                inlines: inlines.iter().map(|i| i.strip_spans()).collect(),
                span: Span::NONE,
            },
            MmdInline::Strong { inlines, .. } => MmdInline::Strong {
                inlines: inlines.iter().map(|i| i.strip_spans()).collect(),
                span: Span::NONE,
            },
            MmdInline::Strikethrough { inlines, .. } => MmdInline::Strikethrough {
                inlines: inlines.iter().map(|i| i.strip_spans()).collect(),
                span: Span::NONE,
            },
            MmdInline::Code { content, .. } => MmdInline::Code {
                content: content.clone(),
                span: Span::NONE,
            },
            MmdInline::HtmlInline { content, .. } => MmdInline::HtmlInline {
                content: content.clone(),
                span: Span::NONE,
            },
            MmdInline::Link {
                inlines,
                url,
                title,
                ..
            } => MmdInline::Link {
                inlines: inlines.iter().map(|i| i.strip_spans()).collect(),
                url: url.clone(),
                title: title.clone(),
                span: Span::NONE,
            },
            MmdInline::Image {
                alt, url, title, ..
            } => MmdInline::Image {
                alt: alt.clone(),
                url: url.clone(),
                title: title.clone(),
                span: Span::NONE,
            },
            MmdInline::FootnoteReference { label, .. } => MmdInline::FootnoteReference {
                label: label.clone(),
                span: Span::NONE,
            },
            MmdInline::InlineMath { source, .. } => MmdInline::InlineMath {
                source: source.clone(),
                span: Span::NONE,
            },
            MmdInline::DisplayMath { source, .. } => MmdInline::DisplayMath {
                source: source.clone(),
                span: Span::NONE,
            },
            MmdInline::Citation { locator, label, .. } => MmdInline::Citation {
                locator: locator.clone(),
                label: label.clone(),
                span: Span::NONE,
            },
            MmdInline::CrossReference {
                target, collapsed, ..
            } => MmdInline::CrossReference {
                target: target.clone(),
                collapsed: *collapsed,
                span: Span::NONE,
            },
        }
    }
}

/// A diagnostic produced during parsing.
#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    pub span: Span,
    pub message: String,
    pub code: &'static str,
}
