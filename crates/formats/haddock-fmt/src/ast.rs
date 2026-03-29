/// Source span (byte offsets into the original input).
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
        Diagnostic {
            severity: Severity::Warning,
            message: message.into(),
            span,
        }
    }
    pub fn error(message: impl Into<String>, span: Span) -> Self {
        Diagnostic {
            severity: Severity::Error,
            message: message.into(),
            span,
        }
    }
}

/// A parsed Haddock document.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HaddockDoc {
    pub blocks: Vec<Block>,
    pub span: Span,
}

impl HaddockDoc {
    pub fn strip_spans(self) -> Self {
        HaddockDoc {
            blocks: self.blocks.into_iter().map(Block::strip_spans).collect(),
            span: Span::NONE,
        }
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
    /// `@`-delimited code block (`@ ... @` on separate lines).
    AtCodeBlock {
        content: String,
        span: Span,
    },
    UnorderedList {
        items: Vec<Vec<Inline>>,
        span: Span,
    },
    OrderedList {
        items: Vec<Vec<Inline>>,
        span: Span,
    },
    DefinitionList {
        items: Vec<(Vec<Inline>, Vec<Inline>)>,
        span: Span,
    },
    /// `>>> expression` / `>>> expression\nresult` doc-test example.
    DocTest {
        expression: String,
        result: Option<String>,
        span: Span,
    },
    /// `> quoted text` blockquote (when not a bird-track code block).
    Blockquote {
        inlines: Vec<Inline>,
        span: Span,
    },
    /// Property annotations: `@since`, `@deprecated`, `@param`, `@returns`.
    Property {
        key: String,
        name: Option<String>,
        description: Vec<Inline>,
        span: Span,
    },
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
            Block::CodeBlock { content, .. } => Block::CodeBlock {
                content,
                span: Span::NONE,
            },
            Block::AtCodeBlock { content, .. } => Block::AtCodeBlock {
                content,
                span: Span::NONE,
            },
            Block::UnorderedList { items, .. } => Block::UnorderedList {
                items: items
                    .into_iter()
                    .map(|v| v.into_iter().map(Inline::strip_spans).collect())
                    .collect(),
                span: Span::NONE,
            },
            Block::OrderedList { items, .. } => Block::OrderedList {
                items: items
                    .into_iter()
                    .map(|v| v.into_iter().map(Inline::strip_spans).collect())
                    .collect(),
                span: Span::NONE,
            },
            Block::DefinitionList { items, .. } => Block::DefinitionList {
                items: items
                    .into_iter()
                    .map(|(t, d)| {
                        (
                            t.into_iter().map(Inline::strip_spans).collect(),
                            d.into_iter().map(Inline::strip_spans).collect(),
                        )
                    })
                    .collect(),
                span: Span::NONE,
            },
            Block::DocTest { expression, result, .. } => Block::DocTest {
                expression,
                result,
                span: Span::NONE,
            },
            Block::Blockquote { inlines, .. } => Block::Blockquote {
                inlines: inlines.into_iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::Property { key, name, description, .. } => Block::Property {
                key,
                name,
                description: description.into_iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
        }
    }
}

/// Inline element.
#[derive(Debug, Clone, PartialEq)]
pub enum Inline {
    Text(String, Span),
    Code(String, Span),
    Strong(Vec<Inline>, Span),
    Emphasis(Vec<Inline>, Span),
    Link {
        url: String,
        text: String,
        span: Span,
    },
    /// Module cross-reference: `"Module.Name"`.
    ModuleLink {
        module: String,
        span: Span,
    },
}

impl Inline {
    pub fn strip_spans(self) -> Self {
        match self {
            Inline::Text(s, _) => Inline::Text(s, Span::NONE),
            Inline::Code(s, _) => Inline::Code(s, Span::NONE),
            Inline::Strong(children, _) => Inline::Strong(
                children.into_iter().map(Inline::strip_spans).collect(),
                Span::NONE,
            ),
            Inline::Emphasis(children, _) => Inline::Emphasis(
                children.into_iter().map(Inline::strip_spans).collect(),
                Span::NONE,
            ),
            Inline::Link { url, text, .. } => Inline::Link {
                url,
                text,
                span: Span::NONE,
            },
            Inline::ModuleLink { module, .. } => Inline::ModuleLink {
                module,
                span: Span::NONE,
            },
        }
    }
}
