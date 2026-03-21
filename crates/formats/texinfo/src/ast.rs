//! Texinfo format AST types.

// ── Span & Diagnostics ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const NONE: Self = Self { start: 0, end: 0 };
}

#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub message: String,
    pub severity: Severity,
    pub span: Span,
}

// ── AST ───────────────────────────────────────────────────────────────────────

/// A parsed Texinfo document.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TexinfoDoc {
    pub title: Option<String>,
    pub blocks: Vec<Block>,
    pub span: Span,
}

impl TexinfoDoc {
    /// Recursively reset all spans to [`Span::NONE`].
    pub fn strip_spans(&self) -> Self {
        Self {
            title: self.title.clone(),
            blocks: self.blocks.iter().map(Block::strip_spans).collect(),
            span: Span::NONE,
        }
    }
}

impl Default for Span {
    fn default() -> Self {
        Self::NONE
    }
}

/// Block-level element.
#[derive(Debug, Clone, PartialEq)]
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
        items: Vec<Vec<Inline>>,
        span: Span,
    },
    DefinitionList {
        items: Vec<(Vec<Inline>, Vec<Block>)>,
        span: Span,
    },
    HorizontalRule {
        span: Span,
    },
}

impl Block {
    /// Return a copy of this block with all spans set to [`Span::NONE`].
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
            Block::CodeBlock { content, .. } => Block::CodeBlock {
                content: content.clone(),
                span: Span::NONE,
            },
            Block::Blockquote { children, .. } => Block::Blockquote {
                children: children.iter().map(Block::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::List { ordered, items, .. } => Block::List {
                ordered: *ordered,
                items: items
                    .iter()
                    .map(|item| item.iter().map(Inline::strip_spans).collect())
                    .collect(),
                span: Span::NONE,
            },
            Block::DefinitionList { items, .. } => Block::DefinitionList {
                items: items
                    .iter()
                    .map(|(term, desc)| {
                        (
                            term.iter().map(Inline::strip_spans).collect(),
                            desc.iter().map(Block::strip_spans).collect(),
                        )
                    })
                    .collect(),
                span: Span::NONE,
            },
            Block::HorizontalRule { .. } => Block::HorizontalRule { span: Span::NONE },
        }
    }
}

/// Inline element.
#[derive(Debug, Clone, PartialEq)]
pub enum Inline {
    Text(String, Span),
    Strong(Vec<Inline>, Span),
    Emphasis(Vec<Inline>, Span),
    Code(String, Span),
    Link {
        url: String,
        children: Vec<Inline>,
        span: Span,
    },
    Superscript(Vec<Inline>, Span),
    Subscript(Vec<Inline>, Span),
    LineBreak {
        span: Span,
    },
    SoftBreak {
        span: Span,
    },
    FootnoteDef {
        content: Vec<Inline>,
        span: Span,
    },
}

impl Inline {
    /// Return a copy of this inline with all spans set to [`Span::NONE`].
    pub fn strip_spans(&self) -> Self {
        match self {
            Inline::Text(s, _) => Inline::Text(s.clone(), Span::NONE),
            Inline::Strong(children, _) => {
                Inline::Strong(children.iter().map(Inline::strip_spans).collect(), Span::NONE)
            }
            Inline::Emphasis(children, _) => {
                Inline::Emphasis(children.iter().map(Inline::strip_spans).collect(), Span::NONE)
            }
            Inline::Code(s, _) => Inline::Code(s.clone(), Span::NONE),
            Inline::Link { url, children, .. } => Inline::Link {
                url: url.clone(),
                children: children.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Inline::Superscript(children, _) => {
                Inline::Superscript(children.iter().map(Inline::strip_spans).collect(), Span::NONE)
            }
            Inline::Subscript(children, _) => {
                Inline::Subscript(children.iter().map(Inline::strip_spans).collect(), Span::NONE)
            }
            Inline::LineBreak { .. } => Inline::LineBreak { span: Span::NONE },
            Inline::SoftBreak { .. } => Inline::SoftBreak { span: Span::NONE },
            Inline::FootnoteDef { content, .. } => Inline::FootnoteDef {
                content: content.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
        }
    }
}
