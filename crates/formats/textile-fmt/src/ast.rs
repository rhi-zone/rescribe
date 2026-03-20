//! Textile AST types.

// ── Span & Diagnostic ─────────────────────────────────────────────────────────

/// Byte-offset span into the source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// A zero-width span at position 0 — used when no source position is tracked.
    pub fn dummy() -> Self {
        Self { start: 0, end: 0 }
    }
}

/// Diagnostic severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Warning,
    Error,
}

/// A parse diagnostic (non-fatal).
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub span: Span,
}

impl Diagnostic {
    pub fn warning(message: impl Into<String>, span: Span) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
            span,
        }
    }
}

// ── AST ───────────────────────────────────────────────────────────────────────

/// A parsed Textile document.
#[derive(Debug, Clone, Default)]
pub struct TextileDoc {
    pub blocks: Vec<Block>,
    pub span: Span,
}

impl TextileDoc {
    pub fn strip_spans(self) -> Self {
        Self {
            blocks: self.blocks.into_iter().map(Block::strip_spans).collect(),
            span: Span::dummy(),
        }
    }
}

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
        span: Span,
    },
    Blockquote {
        inlines: Vec<Inline>,
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
}

impl Block {
    pub fn span(&self) -> Span {
        match self {
            Block::Paragraph { span, .. } => *span,
            Block::Heading { span, .. } => *span,
            Block::CodeBlock { span, .. } => *span,
            Block::Blockquote { span, .. } => *span,
            Block::List { span, .. } => *span,
            Block::Table { span, .. } => *span,
        }
    }

    pub fn strip_spans(self) -> Self {
        let dummy = Span::dummy();
        match self {
            Block::Paragraph { inlines, .. } => Block::Paragraph {
                inlines: inlines.into_iter().map(Inline::strip_spans).collect(),
                span: dummy,
            },
            Block::Heading { level, inlines, .. } => Block::Heading {
                level,
                inlines: inlines.into_iter().map(Inline::strip_spans).collect(),
                span: dummy,
            },
            Block::CodeBlock { content, .. } => Block::CodeBlock {
                content,
                span: dummy,
            },
            Block::Blockquote { inlines, .. } => Block::Blockquote {
                inlines: inlines.into_iter().map(Inline::strip_spans).collect(),
                span: dummy,
            },
            Block::List { ordered, items, .. } => Block::List {
                ordered,
                items: items
                    .into_iter()
                    .map(|item| item.into_iter().map(Block::strip_spans).collect())
                    .collect(),
                span: dummy,
            },
            Block::Table { rows, .. } => Block::Table {
                rows: rows.into_iter().map(TableRow::strip_spans).collect(),
                span: dummy,
            },
        }
    }
}

/// A table row.
#[derive(Debug, Clone)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
    pub span: Span,
}

impl TableRow {
    pub fn strip_spans(self) -> Self {
        Self {
            cells: self.cells.into_iter().map(TableCell::strip_spans).collect(),
            span: Span::dummy(),
        }
    }
}

/// A table cell.
#[derive(Debug, Clone)]
pub struct TableCell {
    pub is_header: bool,
    pub inlines: Vec<Inline>,
    pub span: Span,
}

impl TableCell {
    pub fn strip_spans(self) -> Self {
        Self {
            is_header: self.is_header,
            inlines: self.inlines.into_iter().map(Inline::strip_spans).collect(),
            span: Span::dummy(),
        }
    }
}

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
        alt: Option<String>,
        span: Span,
    },
    Superscript(Vec<Inline>, Span),
    Subscript(Vec<Inline>, Span),
}

impl Inline {
    pub fn span(&self) -> Span {
        match self {
            Inline::Text(_, s) => *s,
            Inline::Bold(_, s) => *s,
            Inline::Italic(_, s) => *s,
            Inline::Underline(_, s) => *s,
            Inline::Strikethrough(_, s) => *s,
            Inline::Code(_, s) => *s,
            Inline::Link { span, .. } => *span,
            Inline::Image { span, .. } => *span,
            Inline::Superscript(_, s) => *s,
            Inline::Subscript(_, s) => *s,
        }
    }

    pub fn strip_spans(self) -> Self {
        let dummy = Span::dummy();
        match self {
            Inline::Text(s, _) => Inline::Text(s, dummy),
            Inline::Bold(children, _) => {
                Inline::Bold(children.into_iter().map(Inline::strip_spans).collect(), dummy)
            }
            Inline::Italic(children, _) => {
                Inline::Italic(children.into_iter().map(Inline::strip_spans).collect(), dummy)
            }
            Inline::Underline(children, _) => {
                Inline::Underline(children.into_iter().map(Inline::strip_spans).collect(), dummy)
            }
            Inline::Strikethrough(children, _) => Inline::Strikethrough(
                children.into_iter().map(Inline::strip_spans).collect(),
                dummy,
            ),
            Inline::Code(s, _) => Inline::Code(s, dummy),
            Inline::Link { url, children, .. } => Inline::Link {
                url,
                children: children.into_iter().map(Inline::strip_spans).collect(),
                span: dummy,
            },
            Inline::Image { url, alt, .. } => Inline::Image {
                url,
                alt,
                span: dummy,
            },
            Inline::Superscript(children, _) => Inline::Superscript(
                children.into_iter().map(Inline::strip_spans).collect(),
                dummy,
            ),
            Inline::Subscript(children, _) => Inline::Subscript(
                children.into_iter().map(Inline::strip_spans).collect(),
                dummy,
            ),
        }
    }
}
