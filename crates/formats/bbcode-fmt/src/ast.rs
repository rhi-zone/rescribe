//! BBCode AST types: nodes, spans, diagnostics.

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
/// BBCode parsing is always infallible — malformed constructs are silently
/// tolerated and produce diagnostics instead of hard errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub span: Span,
    pub severity: Severity,
    pub message: String,
    pub code: &'static str,
}

// ── Document ──────────────────────────────────────────────────────────────────

/// A parsed BBCode document.
#[derive(Debug, Clone, Default)]
pub struct BbcodeDoc {
    pub blocks: Vec<Block>,
    pub span: Span,
}

impl BbcodeDoc {
    /// Return a copy of this document with all spans zeroed.
    pub fn strip_spans(&self) -> Self {
        BbcodeDoc {
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
    CodeBlock {
        content: String,
        span: Span,
    },
    Blockquote {
        children: Vec<Block>,
        span: Span,
    },
    List {
        ordered: bool,
        items: Vec<Vec<Inline>>,
        span: Span,
    },
    Table {
        rows: Vec<TableRow>,
        span: Span,
    },
}

impl Block {
    pub fn strip_spans(&self) -> Self {
        match self {
            Block::Paragraph { inlines, .. } => Block::Paragraph {
                inlines: inlines.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::CodeBlock { content, .. } => Block::CodeBlock {
                content: content.clone(),
                span: Span::NONE,
            },
            Block::Blockquote { children, .. } => Block::Blockquote {
                children: children.iter().map(Block::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::List { ordered, items, .. } => Block::List {
                ordered: *ordered,
                items: items
                    .iter()
                    .map(|row| row.iter().map(Inline::strip_spans).collect())
                    .collect(),
                span: Span::NONE,
            },
            Block::Table { rows, .. } => Block::Table {
                rows: rows.iter().map(TableRow::strip_spans).collect(),
                span: Span::NONE,
            },
        }
    }
}

// ── Table ─────────────────────────────────────────────────────────────────────

/// A table row.
#[derive(Debug, Clone)]
pub struct TableRow {
    pub cells: Vec<(bool, Vec<Inline>)>, // (is_header, inlines)
    pub span: Span,
}

impl TableRow {
    pub fn strip_spans(&self) -> Self {
        TableRow {
            cells: self
                .cells
                .iter()
                .map(|(is_header, inlines)| {
                    (*is_header, inlines.iter().map(Inline::strip_spans).collect())
                })
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
    Subscript(Vec<Inline>, Span),
    Superscript(Vec<Inline>, Span),
    Span {
        attr: String,
        value: String,
        children: Vec<Inline>,
        span: Span,
    },
}

impl Inline {
    pub fn strip_spans(&self) -> Self {
        match self {
            Inline::Text(s, _) => Inline::Text(s.clone(), Span::NONE),
            Inline::Bold(ch, _) => {
                Inline::Bold(ch.iter().map(Inline::strip_spans).collect(), Span::NONE)
            }
            Inline::Italic(ch, _) => {
                Inline::Italic(ch.iter().map(Inline::strip_spans).collect(), Span::NONE)
            }
            Inline::Underline(ch, _) => {
                Inline::Underline(ch.iter().map(Inline::strip_spans).collect(), Span::NONE)
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
            Inline::Image { url, .. } => Inline::Image {
                url: url.clone(),
                span: Span::NONE,
            },
            Inline::Subscript(ch, _) => {
                Inline::Subscript(ch.iter().map(Inline::strip_spans).collect(), Span::NONE)
            }
            Inline::Superscript(ch, _) => {
                Inline::Superscript(ch.iter().map(Inline::strip_spans).collect(), Span::NONE)
            }
            Inline::Span {
                attr,
                value,
                children,
                ..
            } => Inline::Span {
                attr: attr.clone(),
                value: value.clone(),
                children: children.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
        }
    }
}
