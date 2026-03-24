//! AST types for CommonMark documents.

/// Byte-offset span into the source input.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const NONE: Span = Span { start: 0, end: 0 };
}

impl Default for Span {
    fn default() -> Self {
        Span::NONE
    }
}

/// A parsed CommonMark document.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct CmDoc {
    pub blocks: Vec<Block>,
    /// Reference-style link definitions collected during parsing.
    pub link_defs: Vec<LinkDef>,
}

impl CmDoc {
    /// Return a copy of this document with all spans set to [`Span::NONE`].
    /// Used for roundtrip equality tests where span values are not under test.
    pub fn strip_spans(&self) -> CmDoc {
        CmDoc {
            blocks: self.blocks.iter().map(|b| b.strip_spans()).collect(),
            link_defs: self.link_defs.clone(),
        }
    }
}

/// A block-level node.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
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
    HtmlBlock {
        content: String,
        span: Span,
    },
    Blockquote {
        blocks: Vec<Block>,
        span: Span,
    },
    List {
        kind: ListKind,
        items: Vec<ListItem>,
        tight: bool,
        span: Span,
    },
    ThematicBreak {
        span: Span,
    },
}

impl Block {
    /// Return a copy with all spans set to [`Span::NONE`].
    pub fn strip_spans(&self) -> Block {
        match self {
            Block::Paragraph { inlines, .. } => Block::Paragraph {
                inlines: inlines.iter().map(|i| i.strip_spans()).collect(),
                span: Span::NONE,
            },
            Block::Heading { level, inlines, .. } => Block::Heading {
                level: *level,
                inlines: inlines.iter().map(|i| i.strip_spans()).collect(),
                span: Span::NONE,
            },
            Block::CodeBlock { language, content, .. } => Block::CodeBlock {
                language: language.clone(),
                content: content.clone(),
                span: Span::NONE,
            },
            Block::HtmlBlock { content, .. } => Block::HtmlBlock {
                content: content.clone(),
                span: Span::NONE,
            },
            Block::Blockquote { blocks, .. } => Block::Blockquote {
                blocks: blocks.iter().map(|b| b.strip_spans()).collect(),
                span: Span::NONE,
            },
            Block::List { kind, items, tight, .. } => Block::List {
                kind: kind.clone(),
                items: items.iter().map(|item| item.strip_spans()).collect(),
                tight: *tight,
                span: Span::NONE,
            },
            Block::ThematicBreak { .. } => Block::ThematicBreak { span: Span::NONE },
        }
    }
}

/// Distinguishes ordered from unordered lists.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub enum ListKind {
    Unordered {
        /// The marker character used: `-`, `*`, or `+`.
        /// pulldown-cmark does not expose the marker, so this defaults to `-`.
        marker: char,
    },
    Ordered {
        /// The number of the first list item.
        start: u64,
        /// Whether items end with `.` or `)`.
        marker: OrderedMarker,
    },
}

/// The punctuation that follows an ordered-list number.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub enum OrderedMarker {
    /// Items like `1.`
    Period,
    /// Items like `1)`
    Paren,
}

/// A single item in a list.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct ListItem {
    pub blocks: Vec<Block>,
    pub span: Span,
}

impl ListItem {
    /// Return a copy with all spans set to [`Span::NONE`].
    pub fn strip_spans(&self) -> ListItem {
        ListItem {
            blocks: self.blocks.iter().map(|b| b.strip_spans()).collect(),
            span: Span::NONE,
        }
    }
}

/// An inline node.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub enum Inline {
    Text {
        content: String,
        span: Span,
    },
    SoftBreak {
        span: Span,
    },
    HardBreak {
        span: Span,
    },
    Emphasis {
        inlines: Vec<Inline>,
        span: Span,
    },
    Strong {
        inlines: Vec<Inline>,
        span: Span,
    },
    /// GFM strikethrough extension (`~~text~~`).
    Strikethrough {
        inlines: Vec<Inline>,
        span: Span,
    },
    Code {
        content: String,
        span: Span,
    },
    HtmlInline {
        content: String,
        span: Span,
    },
    Link {
        inlines: Vec<Inline>,
        url: String,
        title: Option<String>,
        span: Span,
    },
    /// Images have a plain-text alt string rather than `Vec<Inline>` because
    /// pulldown-cmark flattens the alt-text content to a single `Text` event.
    Image {
        alt: String,
        url: String,
        title: Option<String>,
        span: Span,
    },
}

impl Inline {
    /// Return a copy with all spans set to [`Span::NONE`].
    pub fn strip_spans(&self) -> Inline {
        match self {
            Inline::Text { content, .. } => Inline::Text {
                content: content.clone(),
                span: Span::NONE,
            },
            Inline::SoftBreak { .. } => Inline::SoftBreak { span: Span::NONE },
            Inline::HardBreak { .. } => Inline::HardBreak { span: Span::NONE },
            Inline::Emphasis { inlines, .. } => Inline::Emphasis {
                inlines: inlines.iter().map(|i| i.strip_spans()).collect(),
                span: Span::NONE,
            },
            Inline::Strong { inlines, .. } => Inline::Strong {
                inlines: inlines.iter().map(|i| i.strip_spans()).collect(),
                span: Span::NONE,
            },
            Inline::Strikethrough { inlines, .. } => Inline::Strikethrough {
                inlines: inlines.iter().map(|i| i.strip_spans()).collect(),
                span: Span::NONE,
            },
            Inline::Code { content, .. } => Inline::Code {
                content: content.clone(),
                span: Span::NONE,
            },
            Inline::HtmlInline { content, .. } => Inline::HtmlInline {
                content: content.clone(),
                span: Span::NONE,
            },
            Inline::Link { inlines, url, title, .. } => Inline::Link {
                inlines: inlines.iter().map(|i| i.strip_spans()).collect(),
                url: url.clone(),
                title: title.clone(),
                span: Span::NONE,
            },
            Inline::Image { alt, url, title, .. } => Inline::Image {
                alt: alt.clone(),
                url: url.clone(),
                title: title.clone(),
                span: Span::NONE,
            },
        }
    }
}

/// A reference-style link definition (`[label]: url "title"`).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct LinkDef {
    pub label: String,
    pub url: String,
    pub title: Option<String>,
}

/// A diagnostic produced during parsing.
#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    pub span: Span,
    pub severity: Severity,
    pub message: String,
    pub code: &'static str,
}

/// Severity level for a [`Diagnostic`].
#[derive(Clone, Debug, PartialEq)]
pub enum Severity {
    Warning,
    Info,
}
