//! Org-mode AST types.

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
/// Org-mode parsing is always infallible — malformed constructs are silently
/// tolerated and produce diagnostics instead of hard errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub span: Span,
    pub severity: Severity,
    pub message: String,
    pub code: &'static str,
}

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct OrgError(pub String);

impl std::fmt::Display for OrgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Org error: {}", self.0)
    }
}

impl std::error::Error for OrgError {}

// ── Document ──────────────────────────────────────────────────────────────────

/// A parsed Org-mode document.
#[derive(Debug, Clone, Default)]
pub struct OrgDoc {
    pub blocks: Vec<Block>,
    /// Document-level metadata (e.g. title, author from #+TITLE: etc.)
    pub metadata: Vec<(String, String)>,
}

impl OrgDoc {
    /// Return a copy of this document with all spans zeroed.
    pub fn strip_spans(&self) -> Self {
        OrgDoc {
            blocks: self.blocks.iter().map(Block::strip_spans).collect(),
            metadata: self.metadata.clone(),
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
        level: usize,
        /// Optional TODO/DONE keyword (e.g. "TODO", "DONE")
        todo: Option<String>,
        /// Optional priority cookie (e.g. "A", "B", "C")
        priority: Option<String>,
        /// Tags extracted from heading (e.g. ["tag1", "work"])
        tags: Vec<String>,
        inlines: Vec<Inline>,
        span: Span,
    },
    CodeBlock {
        language: Option<String>,
        /// Header arguments after language (e.g. ":results output")
        header_args: Option<String>,
        content: String,
        span: Span,
    },
    Blockquote {
        children: Vec<Block>,
        span: Span,
    },
    List {
        ordered: bool,
        items: Vec<ListItem>,
        span: Span,
    },
    Table {
        rows: Vec<TableRow>,
        span: Span,
    },
    HorizontalRule {
        span: Span,
    },
    DefinitionList {
        items: Vec<DefinitionItem>,
        span: Span,
    },
    Div {
        inlines: Vec<Inline>,
        span: Span,
    },
    /// Raw block (format, content)
    RawBlock {
        format: String,
        content: String,
        span: Span,
    },
    Figure {
        children: Vec<Block>,
        span: Span,
    },
    Caption {
        inlines: Vec<Inline>,
        span: Span,
    },
    /// Unknown block type logged as diagnostic
    Unknown {
        kind: String,
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
            Block::Heading { level, todo, priority, tags, inlines, .. } => Block::Heading {
                level: *level,
                todo: todo.clone(),
                priority: priority.clone(),
                tags: tags.clone(),
                inlines: inlines.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::CodeBlock {
                language, header_args, content, ..
            } => Block::CodeBlock {
                language: language.clone(),
                header_args: header_args.clone(),
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
                items: items.iter().map(ListItem::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::Table { rows, .. } => Block::Table {
                rows: rows.iter().map(TableRow::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::HorizontalRule { .. } => Block::HorizontalRule { span: Span::NONE },
            Block::DefinitionList { items, .. } => Block::DefinitionList {
                items: items.iter().map(DefinitionItem::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::Div { inlines, .. } => Block::Div {
                inlines: inlines.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::RawBlock {
                format, content, ..
            } => Block::RawBlock {
                format: format.clone(),
                content: content.clone(),
                span: Span::NONE,
            },
            Block::Figure { children, .. } => Block::Figure {
                children: children.iter().map(Block::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::Caption { inlines, .. } => Block::Caption {
                inlines: inlines.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::Unknown { kind, .. } => Block::Unknown {
                kind: kind.clone(),
                span: Span::NONE,
            },
        }
    }
}

// ── List / Table ──────────────────────────────────────────────────────────────

/// Checkbox state for a list item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckboxState {
    /// `[ ]` — not yet done.
    Unchecked,
    /// `[X]` — completed.
    Checked,
    /// `[-]` — partially done / in progress.
    Partial,
}

/// A list item (may contain inline or block content).
#[derive(Debug, Clone)]
pub struct ListItem {
    pub children: Vec<ListItemContent>,
    /// Optional checkbox marker (`[ ]`, `[X]`, `[-]`).
    pub checkbox: Option<CheckboxState>,
}

impl ListItem {
    pub fn strip_spans(&self) -> Self {
        ListItem {
            children: self.children.iter().map(ListItemContent::strip_spans).collect(),
            checkbox: self.checkbox,
        }
    }
}

/// Content within a list item.
#[derive(Debug, Clone)]
pub enum ListItemContent {
    Inline(Vec<Inline>),
    Block(Block),
}

impl ListItemContent {
    pub fn strip_spans(&self) -> Self {
        match self {
            ListItemContent::Inline(inlines) => {
                ListItemContent::Inline(inlines.iter().map(Inline::strip_spans).collect())
            }
            ListItemContent::Block(block) => ListItemContent::Block(block.strip_spans()),
        }
    }
}

/// A definition list item with term and description.
#[derive(Debug, Clone)]
pub struct DefinitionItem {
    pub term: Vec<Inline>,
    pub desc: Vec<Inline>,
}

impl DefinitionItem {
    pub fn strip_spans(&self) -> Self {
        DefinitionItem {
            term: self.term.iter().map(Inline::strip_spans).collect(),
            desc: self.desc.iter().map(Inline::strip_spans).collect(),
        }
    }
}

/// A table row.
#[derive(Debug, Clone)]
pub struct TableRow {
    pub cells: Vec<Vec<Inline>>,
    pub is_header: bool,
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
        }
    }
}

// ── Inline ────────────────────────────────────────────────────────────────────

/// Inline element.
#[derive(Debug, Clone)]
pub enum Inline {
    Text {
        text: String,
        span: Span,
    },
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
    LineBreak {
        span: Span,
    },
    SoftBreak {
        span: Span,
    },
    Superscript(Vec<Inline>, Span),
    Subscript(Vec<Inline>, Span),
    FootnoteRef {
        label: String,
        span: Span,
    },
    FootnoteDefinition {
        label: String,
        children: Vec<Inline>,
        span: Span,
    },
    MathInline {
        source: String,
        span: Span,
    },
    /// Org timestamp: `<YYYY-MM-DD Day>` (active) or `[YYYY-MM-DD Day]` (inactive)
    Timestamp {
        active: bool,
        value: String,
        span: Span,
    },
    /// Export snippet: `@@backend:content@@`
    ExportSnippet {
        backend: String,
        value: String,
        span: Span,
    },
}

impl Inline {
    pub fn strip_spans(&self) -> Self {
        match self {
            Inline::Text { text, .. } => Inline::Text {
                text: text.clone(),
                span: Span::NONE,
            },
            Inline::Bold(c, _) => Inline::Bold(c.iter().map(Inline::strip_spans).collect(), Span::NONE),
            Inline::Italic(c, _) => Inline::Italic(c.iter().map(Inline::strip_spans).collect(), Span::NONE),
            Inline::Underline(c, _) => Inline::Underline(c.iter().map(Inline::strip_spans).collect(), Span::NONE),
            Inline::Strikethrough(c, _) => {
                Inline::Strikethrough(c.iter().map(Inline::strip_spans).collect(), Span::NONE)
            }
            Inline::Code(s, _) => Inline::Code(s.clone(), Span::NONE),
            Inline::Link {
                url, children, ..
            } => Inline::Link {
                url: url.clone(),
                children: children.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Inline::Image { url, .. } => Inline::Image {
                url: url.clone(),
                span: Span::NONE,
            },
            Inline::LineBreak { .. } => Inline::LineBreak { span: Span::NONE },
            Inline::SoftBreak { .. } => Inline::SoftBreak { span: Span::NONE },
            Inline::Superscript(c, _) => {
                Inline::Superscript(c.iter().map(Inline::strip_spans).collect(), Span::NONE)
            }
            Inline::Subscript(c, _) => {
                Inline::Subscript(c.iter().map(Inline::strip_spans).collect(), Span::NONE)
            }
            Inline::FootnoteRef { label, .. } => Inline::FootnoteRef {
                label: label.clone(),
                span: Span::NONE,
            },
            Inline::FootnoteDefinition {
                label, children, ..
            } => Inline::FootnoteDefinition {
                label: label.clone(),
                children: children.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Inline::MathInline { source, .. } => Inline::MathInline {
                source: source.clone(),
                span: Span::NONE,
            },
            Inline::Timestamp { active, value, .. } => Inline::Timestamp {
                active: *active,
                value: value.clone(),
                span: Span::NONE,
            },
            Inline::ExportSnippet { backend, value, .. } => Inline::ExportSnippet {
                backend: backend.clone(),
                value: value.clone(),
                span: Span::NONE,
            },
        }
    }
}

/// Merge adjacent `Inline::Text` siblings.
pub fn merge_text_inlines(inlines: Vec<Inline>) -> Vec<Inline> {
    let mut result: Vec<Inline> = Vec::new();
    for inline in inlines {
        match inline {
            Inline::Text { text, span } => {
                if let Some(Inline::Text {
                    text: last_text, ..
                }) = result.last_mut()
                {
                    last_text.push_str(&text);
                    let _ = span; // discard span of merged text
                } else {
                    result.push(Inline::Text { text, span });
                }
            }
            other => result.push(other),
        }
    }
    result
}
