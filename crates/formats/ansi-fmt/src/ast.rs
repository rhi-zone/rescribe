//! ANSI AST types: nodes, spans, diagnostics.

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
/// ANSI parsing is always infallible — malformed constructs produce
/// diagnostics instead of hard errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub span: Span,
    pub severity: Severity,
    pub message: String,
    pub code: &'static str,
}

// ── Document ──────────────────────────────────────────────────────────────────

/// A parsed ANSI document.
#[derive(Debug, Clone, Default)]
pub struct AnsiDoc {
    pub blocks: Vec<Block>,
    pub span: Span,
}

impl AnsiDoc {
    /// Return a copy of this document with all spans zeroed.
    pub fn strip_spans(&self) -> Self {
        AnsiDoc {
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
        language: Option<String>,
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
    ListItem {
        children: Vec<Block>,
        span: Span,
    },
    Table {
        rows: Vec<TableRow>,
        span: Span,
    },
    TableRow {
        cells: Vec<TableCell>,
        span: Span,
    },
    TableCell {
        inlines: Vec<Inline>,
        span: Span,
    },
    TableHeader {
        cells: Vec<TableCell>,
        span: Span,
    },
    TableBody {
        rows: Vec<TableRow>,
        span: Span,
    },
    TableFoot {
        rows: Vec<TableRow>,
        span: Span,
    },
    HorizontalRule {
        span: Span,
    },
    Div {
        children: Vec<Block>,
        span: Span,
    },
    SpanBlock {
        inlines: Vec<Inline>,
        span: Span,
    },
    RawBlock {
        content: String,
        span: Span,
    },
    RawInline {
        content: String,
        span: Span,
    },
    DefinitionList {
        items: Vec<DefinitionItem>,
        span: Span,
    },
    DefinitionTerm {
        inlines: Vec<Inline>,
        span: Span,
    },
    DefinitionDesc {
        children: Vec<Block>,
        span: Span,
    },
    Figure {
        children: Vec<Block>,
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
            | Block::ListItem { span, .. }
            | Block::Table { span, .. }
            | Block::TableRow { span, .. }
            | Block::TableCell { span, .. }
            | Block::TableHeader { span, .. }
            | Block::TableBody { span, .. }
            | Block::TableFoot { span, .. }
            | Block::HorizontalRule { span }
            | Block::Div { span, .. }
            | Block::SpanBlock { span, .. }
            | Block::RawBlock { span, .. }
            | Block::RawInline { span, .. }
            | Block::DefinitionList { span, .. }
            | Block::DefinitionTerm { span, .. }
            | Block::DefinitionDesc { span, .. }
            | Block::Figure { span, .. } => *span,
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
                language, content, ..
            } => Block::CodeBlock {
                language: language.clone(),
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
            Block::ListItem { children, .. } => Block::ListItem {
                children: children.iter().map(Block::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::Table { rows, .. } => Block::Table {
                rows: rows.iter().map(TableRow::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::TableRow { cells, .. } => Block::TableRow {
                cells: cells.iter().map(TableCell::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::TableCell { inlines, .. } => Block::TableCell {
                inlines: inlines.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::TableHeader { cells, .. } => Block::TableHeader {
                cells: cells.iter().map(TableCell::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::TableBody { rows, .. } => Block::TableBody {
                rows: rows.iter().map(TableRow::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::TableFoot { rows, .. } => Block::TableFoot {
                rows: rows.iter().map(TableRow::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::HorizontalRule { .. } => Block::HorizontalRule { span: Span::NONE },
            Block::Div { children, .. } => Block::Div {
                children: children.iter().map(Block::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::SpanBlock { inlines, .. } => Block::SpanBlock {
                inlines: inlines.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::RawBlock { content, .. } => Block::RawBlock {
                content: content.clone(),
                span: Span::NONE,
            },
            Block::RawInline { content, .. } => Block::RawInline {
                content: content.clone(),
                span: Span::NONE,
            },
            Block::DefinitionList { items, .. } => Block::DefinitionList {
                items: items.iter().map(DefinitionItem::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::DefinitionTerm { inlines, .. } => Block::DefinitionTerm {
                inlines: inlines.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::DefinitionDesc { children, .. } => Block::DefinitionDesc {
                children: children.iter().map(Block::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::Figure { children, .. } => Block::Figure {
                children: children.iter().map(Block::strip_spans).collect(),
                span: Span::NONE,
            },
        }
    }
}

// ── Table ─────────────────────────────────────────────────────────────────────

/// A table row.
#[derive(Debug, Clone)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
    pub span: Span,
}

impl TableRow {
    pub fn strip_spans(&self) -> Self {
        TableRow {
            cells: self.cells.iter().map(TableCell::strip_spans).collect(),
            span: Span::NONE,
        }
    }
}

/// A table cell.
#[derive(Debug, Clone)]
pub struct TableCell {
    pub inlines: Vec<Inline>,
    pub span: Span,
}

impl TableCell {
    pub fn strip_spans(&self) -> Self {
        TableCell {
            inlines: self.inlines.iter().map(Inline::strip_spans).collect(),
            span: Span::NONE,
        }
    }
}

// ── DefinitionItem ────────────────────────────────────────────────────────────

/// A definition item (term + description).
#[derive(Debug, Clone)]
pub struct DefinitionItem {
    pub term: Vec<Inline>,
    pub desc: Vec<Block>,
    pub span: Span,
}

impl DefinitionItem {
    pub fn strip_spans(&self) -> Self {
        DefinitionItem {
            term: self.term.iter().map(Inline::strip_spans).collect(),
            desc: self.desc.iter().map(Block::strip_spans).collect(),
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
}

impl Inline {
    pub fn span(&self) -> Span {
        match self {
            Inline::Text(_, s)
            | Inline::Bold(_, s)
            | Inline::Italic(_, s)
            | Inline::Underline(_, s)
            | Inline::Strikethrough(_, s) => *s,
        }
    }

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
        }
    }
}
