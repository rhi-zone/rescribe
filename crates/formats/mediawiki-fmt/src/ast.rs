//! MediaWiki AST types, Span, and Diagnostic.

// -- Span / Diagnostic -------------------------------------------------------
//
// `Span`/`Diagnostic`/`Severity` are the shared `rescribe-format-api` types,
// not local definitions -- see that crate's module docs for the rationale.
// mediawiki-fmt never constructed a `Diagnostic` (the parser always returns
// an empty diagnostics vec today), so there were no local `warning()`/
// `error()` convenience-constructor call sites to migrate.

pub use rescribe_format_api::{Diagnostic, Severity, Span};

// -- AST ----------------------------------------------------------------------

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
        language: Option<String>,
        content: String,
        span: Span,
    },
    List {
        ordered: bool,
        items: Vec<Vec<Block>>,
        span: Span,
    },
    DefinitionList {
        items: Vec<DefinitionItem>,
        span: Span,
    },
    HorizontalRule,
    Table {
        rows: Vec<TableRow>,
        caption: Option<Vec<Inline>>,
        span: Span,
    },
    Blockquote {
        children: Vec<Block>,
        span: Span,
    },
    PreBlock {
        content: String,
        span: Span,
    },
    RawBlock {
        content: String,
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
            Block::CodeBlock {
                language, content, ..
            } => Block::CodeBlock {
                language,
                content,
                span: Span::NONE,
            },
            Block::List { ordered, items, .. } => Block::List {
                ordered,
                items: items
                    .into_iter()
                    .map(|item| item.into_iter().map(Block::strip_spans).collect())
                    .collect(),
                span: Span::NONE,
            },
            Block::DefinitionList { items, .. } => Block::DefinitionList {
                items: items.into_iter().map(DefinitionItem::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::HorizontalRule => Block::HorizontalRule,
            Block::Table { rows, caption, .. } => Block::Table {
                rows: rows.into_iter().map(TableRow::strip_spans).collect(),
                caption: caption.map(|c| c.into_iter().map(Inline::strip_spans).collect()),
                span: Span::NONE,
            },
            Block::Blockquote { children, .. } => Block::Blockquote {
                children: children.into_iter().map(Block::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::PreBlock { content, .. } => Block::PreBlock {
                content,
                span: Span::NONE,
            },
            Block::RawBlock { content, .. } => Block::RawBlock {
                content,
                span: Span::NONE,
            },
        }
    }
}

/// A definition list item.
#[derive(Debug, Clone, PartialEq)]
pub struct DefinitionItem {
    pub term: Vec<Inline>,
    pub desc: Vec<Inline>,
}

impl DefinitionItem {
    pub fn strip_spans(self) -> Self {
        DefinitionItem {
            term: self.term.into_iter().map(Inline::strip_spans).collect(),
            desc: self.desc.into_iter().map(Inline::strip_spans).collect(),
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
    Link {
        url: String,
        text: String,
    },
    Image {
        url: String,
        alt: String,
    },
    LineBreak,
    Strikeout(Vec<Inline>),
    Underline(Vec<Inline>),
    Subscript(Vec<Inline>),
    Superscript(Vec<Inline>),
    FootnoteRef {
        label: String,
        content: Option<String>,
    },
    MathInline {
        source: String,
    },
    Template {
        content: String,
    },
    Nowiki {
        content: String,
    },
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
