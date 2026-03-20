/// Source span (byte offsets into the original input).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const NONE: Span = Span { start: 0, end: 0 };
}

/// Severity level for diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    Warning,
    Error,
}

/// A parse-time diagnostic message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub span: Span,
}

impl Diagnostic {
    pub fn warning(message: impl Into<String>, span: Span) -> Self {
        Diagnostic { severity: Severity::Warning, message: message.into(), span }
    }
    pub fn error(message: impl Into<String>, span: Span) -> Self {
        Diagnostic { severity: Severity::Error, message: message.into(), span }
    }
}

/// A parsed POD document.
#[derive(Debug, Clone, PartialEq)]
pub struct PodDoc {
    pub blocks: Vec<Block>,
    pub span: Span,
}

impl PodDoc {
    pub fn strip_spans(self) -> Self {
        PodDoc {
            blocks: self.blocks.into_iter().map(Block::strip_spans).collect(),
            span: Span::NONE,
        }
    }
}

/// Block-level element.
#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Heading { level: u32, inlines: Vec<Inline>, span: Span },
    Paragraph { inlines: Vec<Inline>, span: Span },
    CodeBlock { content: String, span: Span },
    List { ordered: bool, items: Vec<Vec<Block>>, span: Span },
}

impl Block {
    pub fn strip_spans(self) -> Self {
        match self {
            Block::Heading { level, inlines, .. } => Block::Heading {
                level,
                inlines: inlines.into_iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::Paragraph { inlines, .. } => Block::Paragraph {
                inlines: inlines.into_iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::CodeBlock { content, .. } => Block::CodeBlock { content, span: Span::NONE },
            Block::List { ordered, items, .. } => Block::List {
                ordered,
                items: items
                    .into_iter()
                    .map(|item| item.into_iter().map(Block::strip_spans).collect())
                    .collect(),
                span: Span::NONE,
            },
        }
    }
}

/// Inline element.
#[derive(Debug, Clone, PartialEq)]
pub enum Inline {
    Text(String, Span),
    Bold(Vec<Inline>, Span),
    Italic(Vec<Inline>, Span),
    Underline(Vec<Inline>, Span),
    Code(String, Span),
    Link { url: String, label: String, span: Span },
}

impl Inline {
    pub fn strip_spans(self) -> Self {
        match self {
            Inline::Text(s, _) => Inline::Text(s, Span::NONE),
            Inline::Bold(ch, _) => {
                Inline::Bold(ch.into_iter().map(Inline::strip_spans).collect(), Span::NONE)
            }
            Inline::Italic(ch, _) => {
                Inline::Italic(ch.into_iter().map(Inline::strip_spans).collect(), Span::NONE)
            }
            Inline::Underline(ch, _) => {
                Inline::Underline(ch.into_iter().map(Inline::strip_spans).collect(), Span::NONE)
            }
            Inline::Code(s, _) => Inline::Code(s, Span::NONE),
            Inline::Link { url, label, .. } => Inline::Link { url, label, span: Span::NONE },
        }
    }
}
