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
/// RTF parsing is always infallible — malformed constructs are silently
/// tolerated and produce diagnostics instead of hard errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub span: Span,
    pub severity: Severity,
    pub message: String,
    pub code: &'static str,
}

// ── Document ──────────────────────────────────────────────────────────────────

/// A parsed RTF document.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RtfDoc {
    pub blocks: Vec<Block>,
    pub span: Span,
}

impl RtfDoc {
    /// Return a copy of this document with all spans zeroed.
    ///
    /// Useful for round-trip comparisons where re-parsing produces different
    /// byte offsets but identical structure and content.
    pub fn strip_spans(&self) -> Self {
        RtfDoc {
            blocks: self.blocks.iter().map(Block::strip_spans).collect(),
            span: Span::NONE,
        }
    }
}

// ── Block ─────────────────────────────────────────────────────────────────────

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
    pub fn span(&self) -> Span {
        match self {
            Block::Paragraph { span, .. }
            | Block::Heading { span, .. }
            | Block::CodeBlock { span, .. }
            | Block::Blockquote { span, .. }
            | Block::List { span, .. }
            | Block::Table { span, .. }
            | Block::HorizontalRule { span } => *span,
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

// ── TableRow ──────────────────────────────────────────────────────────────────

/// A table row.
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone, PartialEq)]
pub enum Inline {
    Text {
        text: String,
        span: Span,
    },
    Bold {
        children: Vec<Inline>,
        span: Span,
    },
    Italic {
        children: Vec<Inline>,
        span: Span,
    },
    Underline {
        children: Vec<Inline>,
        span: Span,
    },
    Strikethrough {
        children: Vec<Inline>,
        span: Span,
    },
    Code {
        text: String,
        span: Span,
    },
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
    LineBreak {
        span: Span,
    },
    SoftBreak {
        span: Span,
    },
    Superscript {
        children: Vec<Inline>,
        span: Span,
    },
    Subscript {
        children: Vec<Inline>,
        span: Span,
    },
}

impl Inline {
    pub fn span(&self) -> Span {
        match self {
            Inline::Text { span, .. }
            | Inline::Bold { span, .. }
            | Inline::Italic { span, .. }
            | Inline::Underline { span, .. }
            | Inline::Strikethrough { span, .. }
            | Inline::Code { span, .. }
            | Inline::Link { span, .. }
            | Inline::Image { span, .. }
            | Inline::LineBreak { span }
            | Inline::SoftBreak { span }
            | Inline::Superscript { span, .. }
            | Inline::Subscript { span, .. } => *span,
        }
    }

    pub fn strip_spans(&self) -> Self {
        match self {
            Inline::Text { text, .. } => Inline::Text {
                text: text.clone(),
                span: Span::NONE,
            },
            Inline::Bold { children, .. } => Inline::Bold {
                children: children.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Inline::Italic { children, .. } => Inline::Italic {
                children: children.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Inline::Underline { children, .. } => Inline::Underline {
                children: children.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Inline::Strikethrough { children, .. } => Inline::Strikethrough {
                children: children.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Inline::Code { text, .. } => Inline::Code {
                text: text.clone(),
                span: Span::NONE,
            },
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
            Inline::LineBreak { .. } => Inline::LineBreak { span: Span::NONE },
            Inline::SoftBreak { .. } => Inline::SoftBreak { span: Span::NONE },
            Inline::Superscript { children, .. } => Inline::Superscript {
                children: children.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Inline::Subscript { children, .. } => Inline::Subscript {
                children: children.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
        }
    }
}
