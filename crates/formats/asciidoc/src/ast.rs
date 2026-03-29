//! AsciiDoc AST types.

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
/// AsciiDoc parsing is always infallible — malformed constructs are silently
/// tolerated and produce diagnostics instead of hard errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub span: Span,
    pub severity: Severity,
    pub message: String,
    pub code: &'static str,
}

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct AsciiDocError(pub String);

impl std::fmt::Display for AsciiDocError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AsciiDoc error: {}", self.0)
    }
}

impl std::error::Error for AsciiDocError {}

// ── Document ──────────────────────────────────────────────────────────────────

/// A parsed AsciiDoc document.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AsciiDoc {
    pub blocks: Vec<Block>,
    pub attributes: std::collections::HashMap<String, String>,
    pub span: Span,
}

impl AsciiDoc {
    /// Return a copy of this document with all spans zeroed.
    pub fn strip_spans(&self) -> Self {
        AsciiDoc {
            blocks: self.blocks.iter().map(Block::strip_spans).collect(),
            attributes: self.attributes.clone(),
            span: Span::NONE,
        }
    }
}

// ── Block ─────────────────────────────────────────────────────────────────────

/// Block-level element.
#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Paragraph {
        inlines: Vec<Inline>,
        /// Block id from `[#id]` attribute list.
        id: Option<String>,
        /// Block role/CSS class from `[.role]` attribute list.
        role: Option<String>,
        /// Checklist item state: `Some(true)` = checked, `Some(false)` = unchecked, `None` = not a checklist item.
        checked: Option<bool>,
        span: Span,
    },
    Heading {
        level: usize,
        inlines: Vec<Inline>,
        /// Block id from `[#id]` attribute list.
        id: Option<String>,
        /// Block role from `[.role]` attribute list.
        role: Option<String>,
        span: Span,
    },
    CodeBlock {
        content: String,
        language: Option<String>,
        span: Span,
    },
    Blockquote {
        children: Vec<Block>,
        attribution: Option<String>,
        span: Span,
    },
    List {
        ordered: bool,
        items: Vec<Vec<Block>>,
        /// List marker style from `[loweralpha]`, `[upperroman]`, etc.
        style: Option<String>,
        span: Span,
    },
    DefinitionList {
        items: Vec<DefinitionItem>,
        span: Span,
    },
    HorizontalRule {
        span: Span,
    },
    PageBreak {
        span: Span,
    },
    Figure {
        image: ImageData,
        span: Span,
    },
    /// A generic div block with an optional CSS class and optional block title.
    Div {
        class: Option<String>,
        /// Optional block title from `.Title` preceding the block.
        title: Option<String>,
        children: Vec<Block>,
        span: Span,
    },
    RawBlock {
        format: String,
        content: String,
        span: Span,
    },
    MathBlock {
        content: String,
        /// The macro flavor: `"stem"`, `"latexmath"`, `"asciimath"`, or `None`.
        flavor: Option<String>,
        span: Span,
    },
    Table {
        rows: Vec<TableRow>,
        span: Span,
    },
}

impl Block {
    pub fn strip_spans(&self) -> Self {
        match self {
            Block::Paragraph { inlines, id, role, checked, .. } => Block::Paragraph {
                inlines: inlines.iter().map(Inline::strip_spans).collect(),
                id: id.clone(),
                role: role.clone(),
                checked: *checked,
                span: Span::NONE,
            },
            Block::Heading { level, inlines, id, role, .. } => Block::Heading {
                level: *level,
                inlines: inlines.iter().map(Inline::strip_spans).collect(),
                id: id.clone(),
                role: role.clone(),
                span: Span::NONE,
            },
            Block::CodeBlock {
                content, language, ..
            } => Block::CodeBlock {
                content: content.clone(),
                language: language.clone(),
                span: Span::NONE,
            },
            Block::Blockquote {
                children,
                attribution,
                ..
            } => Block::Blockquote {
                children: children.iter().map(Block::strip_spans).collect(),
                attribution: attribution.clone(),
                span: Span::NONE,
            },
            Block::List {
                ordered, items, style, ..
            } => Block::List {
                ordered: *ordered,
                items: items
                    .iter()
                    .map(|item| item.iter().map(Block::strip_spans).collect())
                    .collect(),
                style: style.clone(),
                span: Span::NONE,
            },
            Block::DefinitionList { items, .. } => Block::DefinitionList {
                items: items.iter().map(DefinitionItem::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::HorizontalRule { .. } => Block::HorizontalRule { span: Span::NONE },
            Block::PageBreak { .. } => Block::PageBreak { span: Span::NONE },
            Block::Figure { image, .. } => Block::Figure {
                image: image.clone(),
                span: Span::NONE,
            },
            Block::Div {
                class, title, children, ..
            } => Block::Div {
                class: class.clone(),
                title: title.clone(),
                children: children.iter().map(Block::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::RawBlock {
                format, content, ..
            } => Block::RawBlock {
                format: format.clone(),
                content: content.clone(),
                span: Span::NONE,
            },
            Block::MathBlock { content, flavor, .. } => Block::MathBlock {
                content: content.clone(),
                flavor: flavor.clone(),
                span: Span::NONE,
            },
            Block::Table { rows, .. } => Block::Table {
                rows: rows.iter().map(TableRow::strip_spans).collect(),
                span: Span::NONE,
            },
        }
    }
}

// ── Inline ────────────────────────────────────────────────────────────────────

/// Inline element.
#[derive(Debug, Clone, PartialEq)]
pub enum Inline {
    Text {
        text: String,
        span: Span,
    },
    Strong(Vec<Inline>, Span),
    Emphasis(Vec<Inline>, Span),
    Code(String, Span),
    Superscript(Vec<Inline>, Span),
    Subscript(Vec<Inline>, Span),
    Highlight(Vec<Inline>, Span),
    Strikeout(Vec<Inline>, Span),
    Underline(Vec<Inline>, Span),
    SmallCaps(Vec<Inline>, Span),
    Quoted {
        quote_type: QuoteType,
        children: Vec<Inline>,
        span: Span,
    },
    Link {
        url: String,
        children: Vec<Inline>,
        /// Link window target (e.g. `_blank`) from `window=_blank` in the attr list.
        target: Option<String>,
        span: Span,
    },
    Image(ImageData, Span),
    LineBreak {
        span: Span,
    },
    SoftBreak {
        span: Span,
    },
    FootnoteRef {
        label: String,
        span: Span,
    },
    FootnoteDef {
        label: String,
        children: Vec<Inline>,
        span: Span,
    },
    MathInline {
        content: String,
        /// The macro flavor: `"stem"`, `"latexmath"`, `"asciimath"`, or `None`.
        flavor: Option<String>,
        span: Span,
    },
    RawInline {
        format: String,
        content: String,
        span: Span,
    },
    /// Inline anchor `[[id]]` — an in-document target with no display text.
    Anchor {
        id: String,
        span: Span,
    },
}

impl Inline {
    pub fn strip_spans(&self) -> Self {
        match self {
            Inline::Text { text, .. } => Inline::Text {
                text: text.clone(),
                span: Span::NONE,
            },
            Inline::Strong(children, _) => {
                Inline::Strong(children.iter().map(Inline::strip_spans).collect(), Span::NONE)
            }
            Inline::Emphasis(children, _) => {
                Inline::Emphasis(children.iter().map(Inline::strip_spans).collect(), Span::NONE)
            }
            Inline::Code(s, _) => Inline::Code(s.clone(), Span::NONE),
            Inline::Superscript(children, _) => Inline::Superscript(
                children.iter().map(Inline::strip_spans).collect(),
                Span::NONE,
            ),
            Inline::Subscript(children, _) => Inline::Subscript(
                children.iter().map(Inline::strip_spans).collect(),
                Span::NONE,
            ),
            Inline::Highlight(children, _) => Inline::Highlight(
                children.iter().map(Inline::strip_spans).collect(),
                Span::NONE,
            ),
            Inline::Strikeout(children, _) => Inline::Strikeout(
                children.iter().map(Inline::strip_spans).collect(),
                Span::NONE,
            ),
            Inline::Underline(children, _) => Inline::Underline(
                children.iter().map(Inline::strip_spans).collect(),
                Span::NONE,
            ),
            Inline::SmallCaps(children, _) => Inline::SmallCaps(
                children.iter().map(Inline::strip_spans).collect(),
                Span::NONE,
            ),
            Inline::Quoted {
                quote_type,
                children,
                ..
            } => Inline::Quoted {
                quote_type: quote_type.clone(),
                children: children.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Inline::Link {
                url, children, target, ..
            } => Inline::Link {
                url: url.clone(),
                children: children.iter().map(Inline::strip_spans).collect(),
                target: target.clone(),
                span: Span::NONE,
            },
            Inline::Image(img, _) => Inline::Image(img.clone(), Span::NONE),
            Inline::LineBreak { .. } => Inline::LineBreak { span: Span::NONE },
            Inline::SoftBreak { .. } => Inline::SoftBreak { span: Span::NONE },
            Inline::FootnoteRef { label, .. } => Inline::FootnoteRef {
                label: label.clone(),
                span: Span::NONE,
            },
            Inline::FootnoteDef {
                label, children, ..
            } => Inline::FootnoteDef {
                label: label.clone(),
                children: children.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Inline::MathInline { content, flavor, .. } => Inline::MathInline {
                content: content.clone(),
                flavor: flavor.clone(),
                span: Span::NONE,
            },
            Inline::RawInline {
                format, content, ..
            } => Inline::RawInline {
                format: format.clone(),
                content: content.clone(),
                span: Span::NONE,
            },
            Inline::Anchor { id, .. } => Inline::Anchor {
                id: id.clone(),
                span: Span::NONE,
            },
        }
    }
}

// ── Supporting types ──────────────────────────────────────────────────────────

/// An image (URL + optional alt, width, height).
#[derive(Debug, Clone, PartialEq)]
pub struct ImageData {
    pub url: String,
    pub alt: Option<String>,
    pub width: Option<String>,
    pub height: Option<String>,
}

/// A definition list item (term + description).
#[derive(Debug, Clone, PartialEq)]
pub struct DefinitionItem {
    pub term: Vec<Inline>,
    pub desc: Vec<Inline>,
}

impl DefinitionItem {
    pub fn strip_spans(&self) -> Self {
        DefinitionItem {
            term: self.term.iter().map(Inline::strip_spans).collect(),
            desc: self.desc.iter().map(Inline::strip_spans).collect(),
        }
    }
}

/// A table row.
#[derive(Debug, Clone, PartialEq)]
pub struct TableRow {
    pub cells: Vec<Vec<Inline>>,
    pub is_header: bool,
}

impl TableRow {
    pub fn strip_spans(&self) -> Self {
        TableRow {
            cells: self
                .cells
                .iter()
                .map(|cell| cell.iter().map(Inline::strip_spans).collect())
                .collect(),
            is_header: self.is_header,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum QuoteType {
    Single,
    Double,
}
