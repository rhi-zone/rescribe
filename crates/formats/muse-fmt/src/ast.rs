//! Muse AST types.

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
/// Muse parsing is always infallible — malformed constructs are silently
/// tolerated and produce diagnostics instead of hard errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub span: Span,
    pub severity: Severity,
    pub message: String,
    pub code: &'static str,
}

// ── Document ──────────────────────────────────────────────────────────────────

/// A parsed Muse document.
#[derive(Debug, Clone, Default)]
pub struct MuseDoc {
    pub blocks: Vec<Block>,
    pub span: Span,
    /// `#title` directive value.
    pub title: Option<String>,
    /// `#author` directive value.
    pub author: Option<String>,
    /// `#date` directive value.
    pub date: Option<String>,
    /// `#desc` directive value.
    pub description: Option<String>,
    /// `#keywords` directive value.
    pub keywords: Option<String>,
}

impl MuseDoc {
    /// Strip all span information (for roundtrip comparison).
    pub fn strip_spans(self) -> Self {
        Self {
            blocks: self.blocks.into_iter().map(Block::strip_spans).collect(),
            span: Span::NONE,
            title: self.title,
            author: self.author,
            date: self.date,
            description: self.description,
            keywords: self.keywords,
        }
    }
}

/// A row in a Muse table.
#[derive(Debug, Clone)]
pub struct TableRow {
    /// Each cell contains inline content.
    pub cells: Vec<Vec<Inline>>,
    /// True if the row uses `||` header delimiters.
    pub header: bool,
}

impl TableRow {
    pub fn strip_spans(self) -> Self {
        Self {
            cells: self
                .cells
                .into_iter()
                .map(|cell| cell.into_iter().map(Inline::strip_spans).collect())
                .collect(),
            header: self.header,
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
        children: Vec<Block>,
        span: Span,
    },
    List {
        ordered: bool,
        items: Vec<Vec<Block>>,
        span: Span,
    },
    DefinitionList {
        items: Vec<(Vec<Inline>, Vec<Block>)>,
        span: Span,
    },
    HorizontalRule {
        span: Span,
    },
    /// `<verse>...</verse>` — preserves line breaks.
    Verse {
        children: Vec<Block>,
        span: Span,
    },
    /// `<center>...</center>` — centered block.
    CenteredBlock {
        children: Vec<Block>,
        span: Span,
    },
    /// `<right>...</right>` — right-aligned block.
    RightBlock {
        children: Vec<Block>,
        span: Span,
    },
    /// `<literal>...</literal>` — literal passthrough (raw HTML, etc.).
    LiteralBlock {
        content: String,
        span: Span,
    },
    /// `<src lang="...">...</src>` — code block with language.
    SrcBlock {
        lang: Option<String>,
        content: String,
        span: Span,
    },
    /// `;; text` or `<comment>...</comment>` — comment (no output).
    Comment {
        content: String,
        span: Span,
    },
    /// Simple table: `| cell | cell |` rows.
    Table {
        rows: Vec<TableRow>,
        span: Span,
    },
    /// Footnote definition: `[N] text`.
    FootnoteDef {
        label: String,
        content: Vec<Inline>,
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
            Block::CodeBlock { content, .. } => Block::CodeBlock {
                content,
                span: Span::NONE,
            },
            Block::Blockquote { children, .. } => Block::Blockquote {
                children: children.into_iter().map(Block::strip_spans).collect(),
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
                items: items
                    .into_iter()
                    .map(|(term, desc)| {
                        (
                            term.into_iter().map(Inline::strip_spans).collect(),
                            desc.into_iter().map(Block::strip_spans).collect(),
                        )
                    })
                    .collect(),
                span: Span::NONE,
            },
            Block::HorizontalRule { .. } => Block::HorizontalRule { span: Span::NONE },
            Block::Verse { children, .. } => Block::Verse {
                children: children.into_iter().map(Block::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::CenteredBlock { children, .. } => Block::CenteredBlock {
                children: children.into_iter().map(Block::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::RightBlock { children, .. } => Block::RightBlock {
                children: children.into_iter().map(Block::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::LiteralBlock { content, .. } => Block::LiteralBlock {
                content,
                span: Span::NONE,
            },
            Block::SrcBlock { lang, content, .. } => Block::SrcBlock {
                lang,
                content,
                span: Span::NONE,
            },
            Block::Comment { content, .. } => Block::Comment {
                content,
                span: Span::NONE,
            },
            Block::Table { rows, .. } => Block::Table {
                rows: rows.into_iter().map(TableRow::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::FootnoteDef {
                label, content, ..
            } => Block::FootnoteDef {
                label,
                content: content.into_iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
        }
    }
}

/// Inline element.
#[derive(Debug, Clone)]
pub enum Inline {
    Text(String, Span),
    Bold(Vec<Inline>, Span),
    Italic(Vec<Inline>, Span),
    Code(String, Span),
    Link {
        url: String,
        children: Vec<Inline>,
        span: Span,
    },
    /// `_underline_`
    Underline(Vec<Inline>, Span),
    /// `~~strikethrough~~`
    Strikethrough(Vec<Inline>, Span),
    /// `^superscript^`
    Superscript(Vec<Inline>, Span),
    /// `<sub>text</sub>`
    Subscript(Vec<Inline>, Span),
    /// `[N]` footnote reference
    FootnoteRef {
        label: String,
        span: Span,
    },
    /// `<br>` hard line break
    LineBreak(Span),
    /// `<anchor name>` named anchor
    Anchor {
        name: String,
        span: Span,
    },
    /// `[[image.png]]` image link
    Image {
        src: String,
        alt: Option<String>,
        span: Span,
    },
}

impl Inline {
    pub fn strip_spans(self) -> Self {
        match self {
            Inline::Text(s, _) => Inline::Text(s, Span::NONE),
            Inline::Bold(children, _) => {
                Inline::Bold(children.into_iter().map(Inline::strip_spans).collect(), Span::NONE)
            }
            Inline::Italic(children, _) => {
                Inline::Italic(children.into_iter().map(Inline::strip_spans).collect(), Span::NONE)
            }
            Inline::Code(s, _) => Inline::Code(s, Span::NONE),
            Inline::Link { url, children, .. } => Inline::Link {
                url,
                children: children.into_iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Inline::Underline(children, _) => Inline::Underline(
                children.into_iter().map(Inline::strip_spans).collect(),
                Span::NONE,
            ),
            Inline::Strikethrough(children, _) => Inline::Strikethrough(
                children.into_iter().map(Inline::strip_spans).collect(),
                Span::NONE,
            ),
            Inline::Superscript(children, _) => Inline::Superscript(
                children.into_iter().map(Inline::strip_spans).collect(),
                Span::NONE,
            ),
            Inline::Subscript(children, _) => Inline::Subscript(
                children.into_iter().map(Inline::strip_spans).collect(),
                Span::NONE,
            ),
            Inline::FootnoteRef { label, .. } => Inline::FootnoteRef {
                label,
                span: Span::NONE,
            },
            Inline::LineBreak(_) => Inline::LineBreak(Span::NONE),
            Inline::Anchor { name, .. } => Inline::Anchor {
                name,
                span: Span::NONE,
            },
            Inline::Image { src, alt, .. } => Inline::Image {
                src,
                alt,
                span: Span::NONE,
            },
        }
    }
}
