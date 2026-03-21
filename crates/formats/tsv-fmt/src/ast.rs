//! TSV AST types.

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

/// A parsed TSV document.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TsvDoc {
    pub rows: Vec<Row>,
    pub span: Span,
}

impl TsvDoc {
    pub fn strip_spans(self) -> Self {
        TsvDoc {
            rows: self.rows.into_iter().map(Row::strip_spans).collect(),
            span: Span::NONE,
        }
    }
}

/// A single row in a TSV document.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Row {
    pub cells: Vec<Cell>,
    pub span: Span,
}

impl Row {
    pub fn strip_spans(self) -> Self {
        Row {
            cells: self.cells.into_iter().map(Cell::strip_spans).collect(),
            span: Span::NONE,
        }
    }
}

/// A single cell in a TSV row.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Cell {
    pub value: String,
    pub span: Span,
}

impl Cell {
    pub fn strip_spans(self) -> Self {
        Cell { value: self.value, span: Span::NONE }
    }
}
