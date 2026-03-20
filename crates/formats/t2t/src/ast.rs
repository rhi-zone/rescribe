//! txt2tags AST types, Span, and Diagnostic.

// ── Span / Diagnostic ─────────────────────────────────────────────────────────

/// Byte range in the original source input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    /// A zero-width span at the origin. Used for programmatically constructed
    /// nodes that have no source position.
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
/// txt2tags parsing is always infallible — malformed constructs are silently
/// tolerated and produce diagnostics instead of hard errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub span: Span,
    pub severity: Severity,
    pub message: String,
    pub code: &'static str,
}

// ── Document ──────────────────────────────────────────────────────────────────

/// A parsed txt2tags document.
#[derive(Debug, Clone, Default)]
pub struct T2tDoc {
    pub blocks: Vec<Block>,
    pub span: Span,
}

impl T2tDoc {
    /// Return a copy of this document with all spans zeroed.
    pub fn strip_spans(&self) -> Self {
        T2tDoc {
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
        numbered: bool,
        inlines: Vec<Inline>,
        span: Span,
    },
    CodeBlock {
        content: String,
        span: Span,
    },
    RawBlock {
        content: String,
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
}

impl Block {
    /// Return a copy of this block with all spans zeroed.
    pub fn strip_spans(&self) -> Self {
        match self {
            Block::Paragraph { inlines, .. } => Block::Paragraph {
                inlines: inlines.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::Heading {
                level,
                numbered,
                inlines,
                ..
            } => Block::Heading {
                level: *level,
                numbered: *numbered,
                inlines: inlines.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::CodeBlock { content, .. } => Block::CodeBlock {
                content: content.clone(),
                span: Span::NONE,
            },
            Block::RawBlock { content, .. } => Block::RawBlock {
                content: content.clone(),
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
        }
    }
}

/// A table row.
#[derive(Debug, Clone)]
pub struct TableRow {
    pub cells: Vec<Vec<Inline>>,
    pub is_header: bool,
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
            is_header: self.is_header,
            span: Span::NONE,
        }
    }
}

// ── Inline ────────────────────────────────────────────────────────────────────

/// Inline element.
#[derive(Debug, Clone)]
pub enum Inline {
    Text(String, Span),
    Bold(Vec<Inline>, Span),
    Italic(Vec<Inline>, Span),
    Underline(Vec<Inline>, Span),
    Strikethrough(Vec<Inline>, Span),
    Code(String, Span),
    Link {
        url: String,
        children: Vec<Inline>,
        span: Span,
    },
    Image {
        url: String,
        span: Span,
    },
    LineBreak(Span),
    SoftBreak(Span),
}

impl Inline {
    /// Return a copy of this inline with all spans zeroed.
    pub fn strip_spans(&self) -> Self {
        match self {
            Inline::Text(s, _) => Inline::Text(s.clone(), Span::NONE),
            Inline::Bold(children, _) => {
                Inline::Bold(children.iter().map(Inline::strip_spans).collect(), Span::NONE)
            }
            Inline::Italic(children, _) => {
                Inline::Italic(children.iter().map(Inline::strip_spans).collect(), Span::NONE)
            }
            Inline::Underline(children, _) => Inline::Underline(
                children.iter().map(Inline::strip_spans).collect(),
                Span::NONE,
            ),
            Inline::Strikethrough(children, _) => Inline::Strikethrough(
                children.iter().map(Inline::strip_spans).collect(),
                Span::NONE,
            ),
            Inline::Code(s, _) => Inline::Code(s.clone(), Span::NONE),
            Inline::Link { url, children, .. } => Inline::Link {
                url: url.clone(),
                children: children.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Inline::Image { url, .. } => Inline::Image {
                url: url.clone(),
                span: Span::NONE,
            },
            Inline::LineBreak(_) => Inline::LineBreak(Span::NONE),
            Inline::SoftBreak(_) => Inline::SoftBreak(Span::NONE),
        }
    }
}
