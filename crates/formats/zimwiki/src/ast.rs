//! ZimWiki AST types with span information.

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

/// A parsed ZimWiki document.
#[derive(Debug, Clone, Default)]
pub struct ZimwikiDoc {
    pub blocks: Vec<Block>,
    pub span: Span,
}

impl ZimwikiDoc {
    pub fn strip_spans(self) -> Self {
        ZimwikiDoc {
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
    CodeBlock { content: String, span: Span },
    Blockquote { children: Vec<Block>, span: Span },
    List { ordered: bool, items: Vec<ListItem>, span: Span },
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
            Block::CodeBlock { content, .. } => Block::CodeBlock { content, span: Span::NONE },
            Block::Blockquote { children, .. } => Block::Blockquote {
                children: children.into_iter().map(Block::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::List { ordered, items, .. } => Block::List {
                ordered,
                items: items.into_iter().map(ListItem::strip_spans).collect(),
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

/// A list item.
#[derive(Debug, Clone)]
pub struct ListItem {
    pub checked: Option<bool>,
    pub children: Vec<Block>,
    pub span: Span,
}

impl ListItem {
    pub fn strip_spans(self) -> Self {
        ListItem {
            checked: self.checked,
            children: self.children.into_iter().map(Block::strip_spans).collect(),
            span: Span::NONE,
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
    Subscript(Vec<Inline>, Span),
    Superscript(Vec<Inline>, Span),
    Code(String, Span),
    Link { url: String, children: Vec<Inline>, span: Span },
    Image { url: String, span: Span },
    LineBreak { span: Span },
    SoftBreak { span: Span },
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
            Inline::Subscript(c, _) => {
                Inline::Subscript(c.into_iter().map(Inline::strip_spans).collect(), Span::NONE)
            }
            Inline::Superscript(c, _) => {
                Inline::Superscript(c.into_iter().map(Inline::strip_spans).collect(), Span::NONE)
            }
            Inline::Code(s, _) => Inline::Code(s, Span::NONE),
            Inline::Link { url, children, .. } => Inline::Link {
                url,
                children: children.into_iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Inline::Image { url, .. } => Inline::Image { url, span: Span::NONE },
            Inline::LineBreak { .. } => Inline::LineBreak { span: Span::NONE },
            Inline::SoftBreak { .. } => Inline::SoftBreak { span: Span::NONE },
        }
    }
}
