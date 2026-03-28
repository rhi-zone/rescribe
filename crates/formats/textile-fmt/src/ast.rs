//! Textile AST types.

// ── Span & Diagnostic ─────────────────────────────────────────────────────────

/// Byte-offset span into the source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// A zero-width span at position 0 — used when no source position is tracked.
    pub fn dummy() -> Self {
        Self { start: 0, end: 0 }
    }
}

/// Diagnostic severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Warning,
    Error,
}

/// A parse diagnostic (non-fatal).
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub span: Span,
}

impl Diagnostic {
    pub fn warning(message: impl Into<String>, span: Span) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
            span,
        }
    }
}

// ── AST ───────────────────────────────────────────────────────────────────────

/// A parsed Textile document.
#[derive(Debug, Clone, Default)]
pub struct TextileDoc {
    pub blocks: Vec<Block>,
    pub span: Span,
}

impl TextileDoc {
    pub fn strip_spans(self) -> Self {
        Self {
            blocks: self.blocks.into_iter().map(Block::strip_spans).collect(),
            span: Span::dummy(),
        }
    }
}

/// Block-level attributes: class, id, CSS style string, language, alignment, indentation.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BlockAttrs {
    /// CSS class name(s).
    pub class: Option<String>,
    /// Element id.
    pub id: Option<String>,
    /// Inline CSS style.
    pub style: Option<String>,
    /// Language attribute.
    pub lang: Option<String>,
    /// Text alignment: "left", "right", "center", "justify".
    pub align: Option<String>,
    /// Left indentation level (one per `(` after block type).
    pub indent_left: u8,
    /// Right indentation level (one per `)` after block type).
    pub indent_right: u8,
}

impl BlockAttrs {
    pub fn is_empty(&self) -> bool {
        self.class.is_none()
            && self.id.is_none()
            && self.style.is_none()
            && self.lang.is_none()
            && self.align.is_none()
            && self.indent_left == 0
            && self.indent_right == 0
    }
}

/// Block-level element.
#[derive(Debug, Clone)]
pub enum Block {
    Paragraph {
        inlines: Vec<Inline>,
        /// Text alignment: "left", "right", "center", "justify", or None.
        align: Option<String>,
        /// Block-level attributes (class, id, style, lang, indentation).
        attrs: BlockAttrs,
        span: Span,
    },
    Heading {
        level: u8,
        inlines: Vec<Inline>,
        /// Block-level attributes.
        attrs: BlockAttrs,
        span: Span,
    },
    CodeBlock {
        content: String,
        /// Programming language hint (from `bc(lang). ...`).
        language: Option<String>,
        span: Span,
    },
    Blockquote {
        inlines: Vec<Inline>,
        span: Span,
    },
    List {
        ordered: bool,
        items: Vec<Vec<Block>>,
        span: Span,
    },
    Table {
        rows: Vec<TableRow>,
        span: Span,
    },
    /// Horizontal rule (`---` on its own line).
    HorizontalRule { span: Span },
    /// Footnote definition: `fn1. content` at block level.
    FootnoteDef {
        label: String,
        inlines: Vec<Inline>,
        span: Span,
    },
    /// Definition list: pairs of (term inlines, definition inlines).
    DefinitionList {
        items: Vec<(Vec<Inline>, Vec<Inline>)>,
        span: Span,
    },
    /// Raw (notextile) block: content is passed through verbatim.
    Raw { content: String, span: Span },
}

impl Block {
    pub fn span(&self) -> Span {
        match self {
            Block::Paragraph { span, .. } => *span,
            Block::Heading { span, .. } => *span,
            Block::CodeBlock { span, .. } => *span,
            Block::Blockquote { span, .. } => *span,
            Block::List { span, .. } => *span,
            Block::Table { span, .. } => *span,
            Block::HorizontalRule { span } => *span,
            Block::FootnoteDef { span, .. } => *span,
            Block::DefinitionList { span, .. } => *span,
            Block::Raw { span, .. } => *span,
        }
    }

    pub fn strip_spans(self) -> Self {
        let dummy = Span::dummy();
        match self {
            Block::Paragraph { inlines, align, attrs, .. } => Block::Paragraph {
                inlines: inlines.into_iter().map(Inline::strip_spans).collect(),
                align,
                attrs,
                span: dummy,
            },
            Block::Heading { level, inlines, attrs, .. } => Block::Heading {
                level,
                inlines: inlines.into_iter().map(Inline::strip_spans).collect(),
                attrs,
                span: dummy,
            },
            Block::CodeBlock { content, language, .. } => Block::CodeBlock {
                content,
                language,
                span: dummy,
            },
            Block::Blockquote { inlines, .. } => Block::Blockquote {
                inlines: inlines.into_iter().map(Inline::strip_spans).collect(),
                span: dummy,
            },
            Block::List { ordered, items, .. } => Block::List {
                ordered,
                items: items
                    .into_iter()
                    .map(|item| item.into_iter().map(Block::strip_spans).collect())
                    .collect(),
                span: dummy,
            },
            Block::Table { rows, .. } => Block::Table {
                rows: rows.into_iter().map(TableRow::strip_spans).collect(),
                span: dummy,
            },
            Block::HorizontalRule { .. } => Block::HorizontalRule { span: dummy },
            Block::FootnoteDef { label, inlines, .. } => Block::FootnoteDef {
                label,
                inlines: inlines.into_iter().map(Inline::strip_spans).collect(),
                span: dummy,
            },
            Block::DefinitionList { items, .. } => Block::DefinitionList {
                items: items
                    .into_iter()
                    .map(|(term, def)| {
                        (
                            term.into_iter().map(Inline::strip_spans).collect(),
                            def.into_iter().map(Inline::strip_spans).collect(),
                        )
                    })
                    .collect(),
                span: dummy,
            },
            Block::Raw { content, .. } => Block::Raw { content, span: dummy },
        }
    }
}

/// A table row.
#[derive(Debug, Clone)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
    pub span: Span,
}

impl TableRow {
    pub fn strip_spans(self) -> Self {
        Self {
            cells: self.cells.into_iter().map(TableCell::strip_spans).collect(),
            span: Span::dummy(),
        }
    }
}

/// A table cell.
#[derive(Debug, Clone)]
pub struct TableCell {
    pub is_header: bool,
    /// Cell text alignment: "left", "right", "center", "justify", or None.
    pub align: Option<String>,
    pub inlines: Vec<Inline>,
    pub span: Span,
}

impl TableCell {
    pub fn strip_spans(self) -> Self {
        Self {
            is_header: self.is_header,
            align: self.align,
            inlines: self.inlines.into_iter().map(Inline::strip_spans).collect(),
            span: Span::dummy(),
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
    Strikethrough(Vec<Inline>, Span),
    Code(String, Span),
    Link {
        url: String,
        /// Optional title attribute from `"text(title)":url`.
        title: Option<String>,
        children: Vec<Inline>,
        span: Span,
    },
    Image {
        url: String,
        alt: Option<String>,
        span: Span,
    },
    Superscript(Vec<Inline>, Span),
    Subscript(Vec<Inline>, Span),
    /// Inline footnote reference: `[1]` in Textile.
    FootnoteRef { label: String, span: Span },
    /// Hard line break (newline within a paragraph).
    LineBreak(Span),
    /// Raw (notextile) inline: `==content==` — content passed through verbatim.
    Raw(String, Span),
    /// Citation: `??text??` — maps to `<cite>` semantics.
    Citation(Vec<Inline>, Span),
    /// Generic span: `%text%` — maps to `<span>`.
    GenericSpan(Vec<Inline>, Span),
    /// Acronym: `ABC(meaning)` — all-caps abbreviation with title.
    Acronym { text: String, title: String, span: Span },
}

impl Inline {
    pub fn span(&self) -> Span {
        match self {
            Inline::Text(_, s) => *s,
            Inline::Bold(_, s) => *s,
            Inline::Italic(_, s) => *s,
            Inline::Underline(_, s) => *s,
            Inline::Strikethrough(_, s) => *s,
            Inline::Code(_, s) => *s,
            Inline::Link { span, .. } => *span,
            Inline::Image { span, .. } => *span,
            Inline::Superscript(_, s) => *s,
            Inline::Subscript(_, s) => *s,
            Inline::FootnoteRef { span, .. } => *span,
            Inline::LineBreak(s) => *s,
            Inline::Raw(_, s) => *s,
            Inline::Citation(_, s) => *s,
            Inline::GenericSpan(_, s) => *s,
            Inline::Acronym { span, .. } => *span,
        }
    }

    pub fn strip_spans(self) -> Self {
        let dummy = Span::dummy();
        match self {
            Inline::Text(s, _) => Inline::Text(s, dummy),
            Inline::Bold(children, _) => {
                Inline::Bold(children.into_iter().map(Inline::strip_spans).collect(), dummy)
            }
            Inline::Italic(children, _) => {
                Inline::Italic(children.into_iter().map(Inline::strip_spans).collect(), dummy)
            }
            Inline::Underline(children, _) => {
                Inline::Underline(children.into_iter().map(Inline::strip_spans).collect(), dummy)
            }
            Inline::Strikethrough(children, _) => Inline::Strikethrough(
                children.into_iter().map(Inline::strip_spans).collect(),
                dummy,
            ),
            Inline::Code(s, _) => Inline::Code(s, dummy),
            Inline::Link { url, title, children, .. } => Inline::Link {
                url,
                title,
                children: children.into_iter().map(Inline::strip_spans).collect(),
                span: dummy,
            },
            Inline::Image { url, alt, .. } => Inline::Image {
                url,
                alt,
                span: dummy,
            },
            Inline::Superscript(children, _) => Inline::Superscript(
                children.into_iter().map(Inline::strip_spans).collect(),
                dummy,
            ),
            Inline::Subscript(children, _) => Inline::Subscript(
                children.into_iter().map(Inline::strip_spans).collect(),
                dummy,
            ),
            Inline::FootnoteRef { label, .. } => Inline::FootnoteRef { label, span: dummy },
            Inline::LineBreak(_) => Inline::LineBreak(dummy),
            Inline::Raw(s, _) => Inline::Raw(s, dummy),
            Inline::Citation(children, _) => Inline::Citation(
                children.into_iter().map(Inline::strip_spans).collect(),
                dummy,
            ),
            Inline::GenericSpan(children, _) => Inline::GenericSpan(
                children.into_iter().map(Inline::strip_spans).collect(),
                dummy,
            ),
            Inline::Acronym { text, title, .. } => Inline::Acronym { text, title, span: dummy },
        }
    }
}
