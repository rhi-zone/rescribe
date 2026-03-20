//! TikiWiki AST types with span information.

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

/// A parsed TikiWiki document.
#[derive(Debug, Clone, Default)]
pub struct TikiwikiDoc {
    pub blocks: Vec<Block>,
    pub span: Span,
}

impl TikiwikiDoc {
    pub fn strip_spans(self) -> Self {
        TikiwikiDoc {
            blocks: self.blocks.into_iter().map(Block::strip_spans).collect(),
            span: Span::NONE,
        }
    }
}

/// Block-level element.
#[derive(Debug, Clone)]
pub enum Block {
    Paragraph { inlines: Vec<Inline>, span: Span },
    Heading { level: u8, inlines: Vec<Inline>, span: Span },
    CodeBlock { content: String, language: Option<String>, span: Span },
    Blockquote { inlines: Vec<Inline>, span: Span },
    List { ordered: bool, items: Vec<Vec<Inline>>, span: Span },
    Table { rows: Vec<TableRow>, span: Span },
    HorizontalRule { span: Span },
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
            Block::CodeBlock { content, language, .. } => {
                Block::CodeBlock { content, language, span: Span::NONE }
            }
            Block::Blockquote { inlines, .. } => Block::Blockquote {
                inlines: inlines.into_iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::List { ordered, items, .. } => Block::List {
                ordered,
                items: items
                    .into_iter()
                    .map(|item| item.into_iter().map(Inline::strip_spans).collect())
                    .collect(),
                span: Span::NONE,
            },
            Block::Table { rows, .. } => Block::Table {
                rows: rows.into_iter().map(TableRow::strip_spans).collect(),
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
    pub span: Span,
}

impl TableRow {
    pub fn strip_spans(self) -> Self {
        TableRow {
            cells: self
                .cells
                .into_iter()
                .map(|c| c.into_iter().map(Inline::strip_spans).collect())
                .collect(),
            span: Span::NONE,
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
    Link { url: String, children: Vec<Inline>, span: Span },
    Image { url: String, alt: String, span: Span },
    LineBreak { span: Span },
}

impl Inline {
    pub fn strip_spans(self) -> Self {
        match self {
            Inline::Text(s, _) => Inline::Text(s, Span::NONE),
            Inline::Bold(c, _) => {
                Inline::Bold(c.into_iter().map(Inline::strip_spans).collect(), Span::NONE)
            }
            Inline::Italic(c, _) => {
                Inline::Italic(c.into_iter().map(Inline::strip_spans).collect(), Span::NONE)
            }
            Inline::Underline(c, _) => {
                Inline::Underline(c.into_iter().map(Inline::strip_spans).collect(), Span::NONE)
            }
            Inline::Strikethrough(c, _) => {
                Inline::Strikethrough(c.into_iter().map(Inline::strip_spans).collect(), Span::NONE)
            }
            Inline::Code(s, _) => Inline::Code(s, Span::NONE),
            Inline::Link { url, children, .. } => Inline::Link {
                url,
                children: children.into_iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Inline::Image { url, alt, .. } => Inline::Image { url, alt, span: Span::NONE },
            Inline::LineBreak { .. } => Inline::LineBreak { span: Span::NONE },
        }
    }
}
