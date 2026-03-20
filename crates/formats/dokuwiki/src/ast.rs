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

// ── AST ───────────────────────────────────────────────────────────────────────

/// A parsed DokuWiki document.
#[derive(Debug, Clone, Default)]
pub struct DokuwikiDoc {
    pub blocks: Vec<Block>,
}

impl DokuwikiDoc {
    pub fn strip_spans(self) -> Self {
        DokuwikiDoc {
            blocks: self.blocks.into_iter().map(|b| b.strip_spans()).collect(),
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
    HorizontalRule(Span),
}

impl Block {
    pub fn strip_spans(self) -> Self {
        match self {
            Block::Paragraph { inlines, .. } => Block::Paragraph {
                inlines: inlines.into_iter().map(|i| i.strip_spans()).collect(),
                span: Span::NONE,
            },
            Block::Heading { level, inlines, .. } => Block::Heading {
                level,
                inlines: inlines.into_iter().map(|i| i.strip_spans()).collect(),
                span: Span::NONE,
            },
            Block::CodeBlock { language, content, .. } => Block::CodeBlock {
                language,
                content,
                span: Span::NONE,
            },
            Block::Blockquote { children, .. } => Block::Blockquote {
                children: children.into_iter().map(|b| b.strip_spans()).collect(),
                span: Span::NONE,
            },
            Block::List { ordered, items, .. } => Block::List {
                ordered,
                items: items
                    .into_iter()
                    .map(|item| item.into_iter().map(|b| b.strip_spans()).collect())
                    .collect(),
                span: Span::NONE,
            },
            Block::HorizontalRule(_) => Block::HorizontalRule(Span::NONE),
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
    Code(String, Span),
    Link { url: String, children: Vec<Inline>, span: Span },
    Image { url: String, alt: Option<String>, span: Span },
    LineBreak(Span),
    SoftBreak(Span),
}

impl Inline {
    pub fn strip_spans(self) -> Self {
        match self {
            Inline::Text(s, _) => Inline::Text(s, Span::NONE),
            Inline::Bold(children, _) => {
                Inline::Bold(children.into_iter().map(|i| i.strip_spans()).collect(), Span::NONE)
            }
            Inline::Italic(children, _) => {
                Inline::Italic(children.into_iter().map(|i| i.strip_spans()).collect(), Span::NONE)
            }
            Inline::Underline(children, _) => Inline::Underline(
                children.into_iter().map(|i| i.strip_spans()).collect(),
                Span::NONE,
            ),
            Inline::Code(s, _) => Inline::Code(s, Span::NONE),
            Inline::Link { url, children, .. } => Inline::Link {
                url,
                children: children.into_iter().map(|i| i.strip_spans()).collect(),
                span: Span::NONE,
            },
            Inline::Image { url, alt, .. } => Inline::Image { url, alt, span: Span::NONE },
            Inline::LineBreak(_) => Inline::LineBreak(Span::NONE),
            Inline::SoftBreak(_) => Inline::SoftBreak(Span::NONE),
        }
    }
}
