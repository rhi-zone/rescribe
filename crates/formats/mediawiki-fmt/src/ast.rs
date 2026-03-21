//! MediaWiki AST types, Span, and Diagnostic.

// ── Span / Diagnostic ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const NONE: Span = Span { start: 0, end: 0 };
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub span: Span,
}

impl Diagnostic {
    pub fn warning(message: impl Into<String>, span: Span) -> Self {
        Self { severity: Severity::Warning, message: message.into(), span }
    }

    pub fn error(message: impl Into<String>, span: Span) -> Self {
        Self { severity: Severity::Error, message: message.into(), span }
    }
}

// ── AST ───────────────────────────────────────────────────────────────────────

/// A parsed MediaWiki document.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MediawikiDoc {
    pub blocks: Vec<Block>,
    pub span: Span,
}

impl MediawikiDoc {
    pub fn strip_spans(self) -> Self {
        MediawikiDoc {
            blocks: self.blocks.into_iter().map(Block::strip_spans).collect(),
            span: Span::NONE,
        }
    }
}

/// Block-level element.
#[derive(Debug, Clone, PartialEq)]
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
        span: Span,
    },
    List {
        ordered: bool,
        items: Vec<Vec<Block>>,
        span: Span,
    },
    HorizontalRule,
    Table {
        rows: Vec<TableRow>,
        span: Span,
    },
}

impl Block {
    pub fn strip_spans(self) -> Self {
        match self {
            Block::Paragraph { inlines, .. } => Block::Paragraph {
                inlines: inlines.into_iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::Heading { level, inlines, .. } => Block::Heading {
                level,
                inlines: inlines.into_iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::CodeBlock { content, .. } => Block::CodeBlock { content, span: Span::NONE },
            Block::List { ordered, items, .. } => Block::List {
                ordered,
                items: items
                    .into_iter()
                    .map(|item| item.into_iter().map(Block::strip_spans).collect())
                    .collect(),
                span: Span::NONE,
            },
            Block::HorizontalRule => Block::HorizontalRule,
            Block::Table { rows, .. } => Block::Table {
                rows: rows.into_iter().map(TableRow::strip_spans).collect(),
                span: Span::NONE,
            },
        }
    }
}

/// A table row.
#[derive(Debug, Clone, PartialEq)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
    pub span: Span,
}

impl TableRow {
    pub fn strip_spans(self) -> Self {
        TableRow {
            cells: self.cells.into_iter().map(TableCell::strip_spans).collect(),
            span: Span::NONE,
        }
    }
}

/// A table cell.
#[derive(Debug, Clone, PartialEq)]
pub struct TableCell {
    pub is_header: bool,
    pub inlines: Vec<Inline>,
    pub span: Span,
}

impl TableCell {
    pub fn strip_spans(self) -> Self {
        TableCell {
            is_header: self.is_header,
            inlines: self.inlines.into_iter().map(Inline::strip_spans).collect(),
            span: Span::NONE,
        }
    }
}

/// Inline element.
#[derive(Debug, Clone, PartialEq)]
pub enum Inline {
    Text(String),
    Bold(Vec<Inline>),
    Italic(Vec<Inline>),
    Code(String),
    Link { url: String, text: String },
    Image { url: String, alt: String },
    LineBreak,
    Strikeout(Vec<Inline>),
    Underline(Vec<Inline>),
    Subscript(Vec<Inline>),
    Superscript(Vec<Inline>),
}

impl Inline {
    pub fn strip_spans(self) -> Self {
        match self {
            Inline::Bold(children) => {
                Inline::Bold(children.into_iter().map(Inline::strip_spans).collect())
            }
            Inline::Italic(children) => {
                Inline::Italic(children.into_iter().map(Inline::strip_spans).collect())
            }
            Inline::Strikeout(children) => {
                Inline::Strikeout(children.into_iter().map(Inline::strip_spans).collect())
            }
            Inline::Underline(children) => {
                Inline::Underline(children.into_iter().map(Inline::strip_spans).collect())
            }
            Inline::Subscript(children) => {
                Inline::Subscript(children.into_iter().map(Inline::strip_spans).collect())
            }
            Inline::Superscript(children) => {
                Inline::Superscript(children.into_iter().map(Inline::strip_spans).collect())
            }
            other => other,
        }
    }
}
