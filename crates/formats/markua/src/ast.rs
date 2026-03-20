//! Markua AST types: nodes, spans, diagnostics.

// ── Span / Diagnostic ─────────────────────────────────────────────────────────

/// Byte range in the original source input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    /// A zero-width span at the origin.  Used for programmatically constructed
    /// nodes (e.g. from the rescribe writer) that have no source position.
    pub const NONE: Self = Self { start: 0, end: 0 };

    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

/// Severity of a [`Diagnostic`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Warning,
    Info,
}

/// A diagnostic message produced during parsing.
///
/// Markua parsing is always infallible — malformed constructs produce
/// diagnostics instead of hard errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub span: Span,
    pub severity: Severity,
    pub message: String,
    pub code: &'static str,
}

// ── Document ──────────────────────────────────────────────────────────────────

/// A parsed Markua document.
#[derive(Debug, Clone, Default)]
pub struct MarkuaDoc {
    pub blocks: Vec<Block>,
    pub span: Span,
}

impl MarkuaDoc {
    /// Return a copy of this document with all spans zeroed.
    pub fn strip_spans(&self) -> Self {
        MarkuaDoc {
            blocks: self.blocks.iter().map(Block::strip_spans).collect(),
            span: Span::NONE,
        }
    }
}

// ── Block ─────────────────────────────────────────────────────────────────────

/// Block-level element.
#[derive(Debug, Clone)]
pub enum Block {
    Paragraph {
        inlines: Vec<Inline>,
        span: Span,
    },
    Heading {
        level: u8,
        inlines: Vec<Inline>,
        span: Span,
    },
    CodeBlock {
        content: String,
        language: Option<String>,
        span: Span,
    },
    Blockquote {
        children: Vec<Block>,
        span: Span,
    },
    List {
        ordered: bool,
        items: Vec<Vec<Block>>,
        span: Span,
    },
    Table {
        rows: Vec<TableRow>,
        span: Span,
    },
    HorizontalRule {
        span: Span,
    },
    SpecialBlock {
        block_type: String,
        inlines: Vec<Inline>,
        span: Span,
    },
}

impl Block {
    pub fn span(&self) -> Span {
        match self {
            Block::Paragraph { span, .. }
            | Block::Heading { span, .. }
            | Block::CodeBlock { span, .. }
            | Block::Blockquote { span, .. }
            | Block::List { span, .. }
            | Block::Table { span, .. }
            | Block::HorizontalRule { span }
            | Block::SpecialBlock { span, .. } => *span,
        }
    }

    pub fn strip_spans(&self) -> Self {
        match self {
            Block::Paragraph { inlines, .. } => Block::Paragraph {
                inlines: inlines.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::Heading { level, inlines, .. } => Block::Heading {
                level: *level,
                inlines: inlines.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::CodeBlock {
                content, language, ..
            } => Block::CodeBlock {
                content: content.clone(),
                language: language.clone(),
                span: Span::NONE,
            },
            Block::Blockquote { children, .. } => Block::Blockquote {
                children: children.iter().map(Block::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::List {
                ordered, items, ..
            } => Block::List {
                ordered: *ordered,
                items: items
                    .iter()
                    .map(|item| item.iter().map(Block::strip_spans).collect())
                    .collect(),
                span: Span::NONE,
            },
            Block::Table { rows, .. } => Block::Table {
                rows: rows.iter().map(TableRow::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::HorizontalRule { .. } => Block::HorizontalRule { span: Span::NONE },
            Block::SpecialBlock {
                block_type,
                inlines,
                ..
            } => Block::SpecialBlock {
                block_type: block_type.clone(),
                inlines: inlines.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
        }
    }
}

// ── Table ─────────────────────────────────────────────────────────────────────

/// A table row.
#[derive(Debug, Clone)]
pub struct TableRow {
    pub cells: Vec<Vec<Inline>>,
    pub span: Span,
}

impl TableRow {
    pub fn strip_spans(&self) -> Self {
        TableRow {
            cells: self
                .cells
                .iter()
                .map(|cell| cell.iter().map(Inline::strip_spans).collect())
                .collect(),
            span: Span::NONE,
        }
    }
}

// ── Inline ────────────────────────────────────────────────────────────────────

/// Inline element.
#[derive(Debug, Clone)]
pub enum Inline {
    Text(String, Span),
    Strong(Vec<Inline>, Span),
    Emphasis(Vec<Inline>, Span),
    Strikethrough(Vec<Inline>, Span),
    Code(String, Span),
    Link {
        url: String,
        children: Vec<Inline>,
        span: Span,
    },
    Image {
        url: String,
        alt: String,
        span: Span,
    },
    LineBreak(Span),
    SoftBreak(Span),
}

impl Inline {
    pub fn span(&self) -> Span {
        match self {
            Inline::Text(_, s)
            | Inline::Strong(_, s)
            | Inline::Emphasis(_, s)
            | Inline::Strikethrough(_, s)
            | Inline::Code(_, s)
            | Inline::LineBreak(s)
            | Inline::SoftBreak(s) => *s,
            Inline::Link { span, .. } | Inline::Image { span, .. } => *span,
        }
    }

    pub fn strip_spans(&self) -> Self {
        match self {
            Inline::Text(s, _) => Inline::Text(s.clone(), Span::NONE),
            Inline::Strong(ch, _) => {
                Inline::Strong(ch.iter().map(Inline::strip_spans).collect(), Span::NONE)
            }
            Inline::Emphasis(ch, _) => {
                Inline::Emphasis(ch.iter().map(Inline::strip_spans).collect(), Span::NONE)
            }
            Inline::Strikethrough(ch, _) => {
                Inline::Strikethrough(ch.iter().map(Inline::strip_spans).collect(), Span::NONE)
            }
            Inline::Code(s, _) => Inline::Code(s.clone(), Span::NONE),
            Inline::Link { url, children, .. } => Inline::Link {
                url: url.clone(),
                children: children.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Inline::Image { url, alt, .. } => Inline::Image {
                url: url.clone(),
                alt: alt.clone(),
                span: Span::NONE,
            },
            Inline::LineBreak(_) => Inline::LineBreak(Span::NONE),
            Inline::SoftBreak(_) => Inline::SoftBreak(Span::NONE),
        }
    }
}
