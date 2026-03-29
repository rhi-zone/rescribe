//! Man page AST types.

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
/// Man page parsing is always infallible — malformed constructs produce
/// diagnostics instead of hard errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub span: Span,
    pub severity: Severity,
    pub message: String,
    pub code: &'static str,
}

// ── Document ──────────────────────────────────────────────────────────────────

/// A parsed man page document.
#[derive(Debug, Clone, Default)]
pub struct ManDoc {
    pub title: Option<String>,
    pub section: Option<String>,
    pub date: Option<String>,
    pub source: Option<String>,
    pub manual: Option<String>,
    pub blocks: Vec<Block>,
    pub span: Span,
}

impl ManDoc {
    /// Return a copy of this document with all spans zeroed.
    pub fn strip_spans(&self) -> Self {
        ManDoc {
            title: self.title.clone(),
            section: self.section.clone(),
            date: self.date.clone(),
            source: self.source.clone(),
            manual: self.manual.clone(),
            blocks: self.blocks.iter().map(Block::strip_spans).collect(),
            span: Span::NONE,
        }
    }
}

// ── Block ─────────────────────────────────────────────────────────────────────

/// Block-level element.
#[derive(Debug, Clone)]
pub enum Block {
    Heading {
        level: u8,
        inlines: Vec<Inline>,
        span: Span,
    },
    Paragraph {
        inlines: Vec<Inline>,
        span: Span,
    },
    IndentedParagraph {
        inlines: Vec<Inline>,
        span: Span,
    },
    CodeBlock {
        content: String,
        span: Span,
    },
    ExampleBlock {
        content: String,
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
    Comment {
        text: String,
        span: Span,
    },
}

impl Block {
    pub fn strip_spans(&self) -> Self {
        match self {
            Block::Heading { level, inlines, .. } => Block::Heading {
                level: *level,
                inlines: inlines.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::Paragraph { inlines, .. } => Block::Paragraph {
                inlines: inlines.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::IndentedParagraph { inlines, .. } => Block::IndentedParagraph {
                inlines: inlines.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::CodeBlock { content, .. } => Block::CodeBlock {
                content: content.clone(),
                span: Span::NONE,
            },
            Block::ExampleBlock { content, .. } => Block::ExampleBlock {
                content: content.clone(),
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
            Block::DefinitionList { items, .. } => Block::DefinitionList {
                items: items
                    .iter()
                    .map(|(term, blocks)| {
                        (
                            term.iter().map(Inline::strip_spans).collect(),
                            blocks.iter().map(Block::strip_spans).collect(),
                        )
                    })
                    .collect(),
                span: Span::NONE,
            },
            Block::HorizontalRule { .. } => Block::HorizontalRule { span: Span::NONE },
            Block::Comment { text, .. } => Block::Comment {
                text: text.clone(),
                span: Span::NONE,
            },
        }
    }
}

// ── Inline ─────────────────────────────────────────────────────────────────────

/// Inline element.
#[derive(Debug, Clone)]
pub enum Inline {
    Text(String, Span),
    Bold(Vec<Inline>, Span),
    Italic(Vec<Inline>, Span),
    Code(String, Span),
    Superscript(Vec<Inline>, Span),
    Subscript(Vec<Inline>, Span),
    Link {
        url: String,
        children: Vec<Inline>,
        span: Span,
    },
}

impl Inline {
    pub fn strip_spans(&self) -> Self {
        match self {
            Inline::Text(s, _) => Inline::Text(s.clone(), Span::NONE),
            Inline::Bold(children, _) => {
                Inline::Bold(children.iter().map(Inline::strip_spans).collect(), Span::NONE)
            }
            Inline::Italic(children, _) => {
                Inline::Italic(children.iter().map(Inline::strip_spans).collect(), Span::NONE)
            }
            Inline::Code(s, _) => Inline::Code(s.clone(), Span::NONE),
            Inline::Superscript(children, _) => {
                Inline::Superscript(children.iter().map(Inline::strip_spans).collect(), Span::NONE)
            }
            Inline::Subscript(children, _) => {
                Inline::Subscript(children.iter().map(Inline::strip_spans).collect(), Span::NONE)
            }
            Inline::Link {
                url,
                children,
                ..
            } => Inline::Link {
                url: url.clone(),
                children: children.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
        }
    }
}
