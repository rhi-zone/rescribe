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
    DefinitionList { items: Vec<DefinitionItem>, span: Span },
    /// `=begin FORMAT` ... `=end FORMAT` raw region.
    RawBlock { format: String, content: String, span: Span },
    /// `=for FORMAT text` single-paragraph format-specific content.
    ForBlock { format: String, content: String, span: Span },
    /// `=encoding ENC` declaration (preserved for round-trip).
    Encoding { encoding: String, span: Span },
}

/// A term/description pair in a definition list.
#[derive(Debug, Clone, PartialEq)]
pub struct DefinitionItem {
    pub term: Vec<Inline>,
    pub desc: Vec<Block>,
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
            Block::DefinitionList { items, .. } => Block::DefinitionList {
                items: items
                    .into_iter()
                    .map(|item| DefinitionItem {
                        term: item.term.into_iter().map(Inline::strip_spans).collect(),
                        desc: item.desc.into_iter().map(Block::strip_spans).collect(),
                    })
                    .collect(),
                span: Span::NONE,
            },
            Block::RawBlock { format, content, .. } => {
                Block::RawBlock { format, content, span: Span::NONE }
            }
            Block::ForBlock { format, content, .. } => {
                Block::ForBlock { format, content, span: Span::NONE }
            }
            Block::Encoding { encoding, .. } => Block::Encoding { encoding, span: Span::NONE },
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
    /// `F<filename>` — filename inline (semantically distinct from italic).
    Filename(Vec<Inline>, Span),
    /// `S<text>` — non-breaking spaces (spaces in content must not wrap).
    NonBreaking(Vec<Inline>, Span),
    /// `X<entry>` — index entry (invisible in output).
    IndexEntry(String, Span),
    /// `Z<>` — zero-width / null element.
    Null(Span),
    /// `E<escape>` — entity/escape that resolved to a character.
    Entity(String, Span),
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
            Inline::Filename(ch, _) => {
                Inline::Filename(ch.into_iter().map(Inline::strip_spans).collect(), Span::NONE)
            }
            Inline::NonBreaking(ch, _) => {
                Inline::NonBreaking(ch.into_iter().map(Inline::strip_spans).collect(), Span::NONE)
            }
            Inline::IndexEntry(s, _) => Inline::IndexEntry(s, Span::NONE),
            Inline::Null(_) => Inline::Null(Span::NONE),
            Inline::Entity(s, _) => Inline::Entity(s, Span::NONE),
        }
    }
}
