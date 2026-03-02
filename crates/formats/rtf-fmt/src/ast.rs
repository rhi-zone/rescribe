/// Merge consecutive `Inline::Text` nodes in a flat list into one.
///
/// This is the canonical normalization the parser always applies; any document
/// produced by `parse()` is already in this form.  Use it to normalize
/// programmatically-constructed documents before round-trip comparisons.
pub(crate) fn merge_text_inlines(inlines: Vec<Inline>) -> Vec<Inline> {
    let mut out: Vec<Inline> = Vec::with_capacity(inlines.len());
    for inline in inlines {
        if let Inline::Text { text: new_text, .. } = &inline
            && let Some(Inline::Text {
                text: prev_text, ..
            }) = out.last_mut()
        {
            prev_text.push_str(new_text);
            continue;
        }
        out.push(inline);
    }
    out
}

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
/// RTF parsing is always infallible — malformed constructs are silently
/// tolerated and produce diagnostics instead of hard errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub span: Span,
    pub severity: Severity,
    pub message: String,
    pub code: &'static str,
}

/// Paragraph text alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    /// No explicit alignment set (RTF default, typically left).
    #[default]
    Default,
    Left,
    Center,
    Right,
    Justify,
}

// ── Document ──────────────────────────────────────────────────────────────────

/// A parsed RTF document.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RtfDoc {
    pub blocks: Vec<Block>,
    /// Colors referenced by `\cf<n>` in this document.
    /// Index 0 is always the auto/default color; indices 1..N are RGB triples.
    pub color_table: Vec<(u8, u8, u8)>,
    pub span: Span,
}

impl RtfDoc {
    /// Return a copy of this document with all spans zeroed.
    ///
    /// Useful for round-trip comparisons where re-parsing produces different
    /// byte offsets but identical structure and content.
    pub fn strip_spans(&self) -> Self {
        RtfDoc {
            blocks: self.blocks.iter().map(Block::strip_spans).collect(),
            color_table: self.color_table.clone(),
            span: Span::NONE,
        }
    }

    /// Return a copy of this document in canonical form.
    ///
    /// "Canonical form" matches the output the parser always produces:
    /// adjacent `Text` siblings are merged into one node, recursively through
    /// all container inlines.  A document that is not in canonical form cannot
    /// roundtrip through `emit → parse` without structural changes.
    pub fn normalize(&self) -> Self {
        RtfDoc {
            blocks: self.blocks.iter().map(Block::normalize).collect(),
            color_table: self.color_table.clone(),
            span: self.span,
        }
    }
}

// ── Block ─────────────────────────────────────────────────────────────────────

/// Block-level element.
#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Paragraph {
        inlines: Vec<Inline>,
        align: Align,
        /// Raw RTF paragraph-layout control words (e.g. `\li720\keep`) captured
        /// verbatim during parsing so the emitter can re-emit them without loss.
        /// Empty string means no paragraph-layout words were present.
        para_props: String,
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
    Table {
        rows: Vec<TableRow>,
        span: Span,
    },
    HorizontalRule {
        span: Span,
    },
}

impl Block {
    pub fn normalize(&self) -> Self {
        match self {
            Block::Paragraph {
                inlines,
                align,
                para_props,
                span,
            } => Block::Paragraph {
                inlines: merge_text_inlines(inlines.iter().map(Inline::normalize).collect()),
                align: *align,
                para_props: para_props.clone(),
                span: *span,
            },
            Block::Heading {
                level,
                inlines,
                span,
            } => Block::Heading {
                level: *level,
                inlines: merge_text_inlines(inlines.iter().map(Inline::normalize).collect()),
                span: *span,
            },
            Block::Blockquote { children, span } => Block::Blockquote {
                children: children.iter().map(Block::normalize).collect(),
                span: *span,
            },
            Block::List {
                ordered,
                items,
                span,
            } => Block::List {
                ordered: *ordered,
                items: items
                    .iter()
                    .map(|item| item.iter().map(Block::normalize).collect())
                    .collect(),
                span: *span,
            },
            Block::Table { rows, span } => Block::Table {
                rows: rows.iter().map(TableRow::normalize).collect(),
                span: *span,
            },
            other => other.clone(),
        }
    }

    pub fn span(&self) -> Span {
        match self {
            Block::Paragraph { span, .. }
            | Block::Heading { span, .. }
            | Block::CodeBlock { span, .. }
            | Block::Blockquote { span, .. }
            | Block::List { span, .. }
            | Block::Table { span, .. }
            | Block::HorizontalRule { span } => *span,
        }
    }

    pub fn strip_spans(&self) -> Self {
        match self {
            Block::Paragraph {
                inlines,
                align,
                para_props,
                ..
            } => Block::Paragraph {
                inlines: inlines.iter().map(Inline::strip_spans).collect(),
                align: *align,
                para_props: para_props.clone(),
                span: Span::NONE,
            },
            Block::Heading { level, inlines, .. } => Block::Heading {
                level: *level,
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
                    .map(|item| item.iter().map(Block::strip_spans).collect())
                    .collect(),
                span: Span::NONE,
            },
            Block::Table { rows, .. } => Block::Table {
                rows: rows.iter().map(TableRow::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::HorizontalRule { .. } => Block::HorizontalRule { span: Span::NONE },
        }
    }
}

// ── TableRow ──────────────────────────────────────────────────────────────────

/// A table row.
#[derive(Debug, Clone, PartialEq)]
pub struct TableRow {
    pub cells: Vec<Vec<Inline>>,
    pub span: Span,
}

impl TableRow {
    pub fn normalize(&self) -> Self {
        TableRow {
            cells: self
                .cells
                .iter()
                .map(|cell| merge_text_inlines(cell.iter().map(Inline::normalize).collect()))
                .collect(),
            span: self.span,
        }
    }

    pub fn strip_spans(&self) -> Self {
        TableRow {
            cells: self
                .cells
                .iter()
                .map(|cell| cell.iter().map(Inline::strip_spans).collect())
                .collect(),
            span: Span::NONE,
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
    Bold {
        children: Vec<Inline>,
        span: Span,
    },
    Italic {
        children: Vec<Inline>,
        span: Span,
    },
    Underline {
        children: Vec<Inline>,
        span: Span,
    },
    Strikethrough {
        children: Vec<Inline>,
        span: Span,
    },
    Code {
        text: String,
        span: Span,
    },
    Link {
        url: String,
        children: Vec<Inline>,
        span: Span,
    },
    Image {
        url: String,
        alt: String,
        span: Span,
    },
    LineBreak {
        span: Span,
    },
    SoftBreak {
        span: Span,
    },
    Superscript {
        children: Vec<Inline>,
        span: Span,
    },
    Subscript {
        children: Vec<Inline>,
        span: Span,
    },
    /// Inline span with a specific font size (in half-points, e.g. 24 = 12pt).
    FontSize {
        size: u16,
        children: Vec<Inline>,
        span: Span,
    },
    /// Inline span with explicit text color.
    Color {
        r: u8,
        g: u8,
        b: u8,
        children: Vec<Inline>,
        span: Span,
    },
    /// All-caps rendering (`\caps`): text stored in original case, rendered uppercase.
    AllCaps {
        children: Vec<Inline>,
        span: Span,
    },
    /// Small-caps rendering (`\scaps`): text stored in original case, rendered in small capitals.
    SmallCaps {
        children: Vec<Inline>,
        span: Span,
    },
    /// Hidden text (`\v`, `\webhidden`): content present in the document but not displayed.
    Hidden {
        children: Vec<Inline>,
        span: Span,
    },
}

impl Inline {
    pub fn normalize(&self) -> Self {
        match self {
            Inline::Bold { children, span } => Inline::Bold {
                children: merge_text_inlines(children.iter().map(Inline::normalize).collect()),
                span: *span,
            },
            Inline::Italic { children, span } => Inline::Italic {
                children: merge_text_inlines(children.iter().map(Inline::normalize).collect()),
                span: *span,
            },
            Inline::Underline { children, span } => Inline::Underline {
                children: merge_text_inlines(children.iter().map(Inline::normalize).collect()),
                span: *span,
            },
            Inline::Strikethrough { children, span } => Inline::Strikethrough {
                children: merge_text_inlines(children.iter().map(Inline::normalize).collect()),
                span: *span,
            },
            Inline::Superscript { children, span } => Inline::Superscript {
                children: merge_text_inlines(children.iter().map(Inline::normalize).collect()),
                span: *span,
            },
            Inline::Subscript { children, span } => Inline::Subscript {
                children: merge_text_inlines(children.iter().map(Inline::normalize).collect()),
                span: *span,
            },
            Inline::Link {
                url,
                children,
                span,
            } => Inline::Link {
                url: url.clone(),
                children: merge_text_inlines(children.iter().map(Inline::normalize).collect()),
                span: *span,
            },
            Inline::FontSize {
                size,
                children,
                span,
            } => Inline::FontSize {
                size: *size,
                children: merge_text_inlines(children.iter().map(Inline::normalize).collect()),
                span: *span,
            },
            Inline::Color {
                r,
                g,
                b,
                children,
                span,
            } => Inline::Color {
                r: *r,
                g: *g,
                b: *b,
                children: merge_text_inlines(children.iter().map(Inline::normalize).collect()),
                span: *span,
            },
            Inline::AllCaps { children, span } => Inline::AllCaps {
                children: merge_text_inlines(children.iter().map(Inline::normalize).collect()),
                span: *span,
            },
            Inline::SmallCaps { children, span } => Inline::SmallCaps {
                children: merge_text_inlines(children.iter().map(Inline::normalize).collect()),
                span: *span,
            },
            Inline::Hidden { children, span } => Inline::Hidden {
                children: merge_text_inlines(children.iter().map(Inline::normalize).collect()),
                span: *span,
            },
            other => other.clone(),
        }
    }

    pub fn span(&self) -> Span {
        match self {
            Inline::Text { span, .. }
            | Inline::Bold { span, .. }
            | Inline::Italic { span, .. }
            | Inline::Underline { span, .. }
            | Inline::Strikethrough { span, .. }
            | Inline::Code { span, .. }
            | Inline::Link { span, .. }
            | Inline::Image { span, .. }
            | Inline::LineBreak { span }
            | Inline::SoftBreak { span }
            | Inline::Superscript { span, .. }
            | Inline::Subscript { span, .. }
            | Inline::FontSize { span, .. }
            | Inline::Color { span, .. }
            | Inline::AllCaps { span, .. }
            | Inline::SmallCaps { span, .. }
            | Inline::Hidden { span, .. } => *span,
        }
    }

    pub fn strip_spans(&self) -> Self {
        match self {
            Inline::Text { text, .. } => Inline::Text {
                text: text.clone(),
                span: Span::NONE,
            },
            Inline::Bold { children, .. } => Inline::Bold {
                children: children.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Inline::Italic { children, .. } => Inline::Italic {
                children: children.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Inline::Underline { children, .. } => Inline::Underline {
                children: children.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Inline::Strikethrough { children, .. } => Inline::Strikethrough {
                children: children.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Inline::Code { text, .. } => Inline::Code {
                text: text.clone(),
                span: Span::NONE,
            },
            Inline::Link { url, children, .. } => Inline::Link {
                url: url.clone(),
                children: children.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Inline::Image { url, alt, .. } => Inline::Image {
                url: url.clone(),
                alt: alt.clone(),
                span: Span::NONE,
            },
            Inline::LineBreak { .. } => Inline::LineBreak { span: Span::NONE },
            Inline::SoftBreak { .. } => Inline::SoftBreak { span: Span::NONE },
            Inline::Superscript { children, .. } => Inline::Superscript {
                children: children.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Inline::Subscript { children, .. } => Inline::Subscript {
                children: children.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Inline::FontSize { size, children, .. } => Inline::FontSize {
                size: *size,
                children: children.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Inline::Color {
                r, g, b, children, ..
            } => Inline::Color {
                r: *r,
                g: *g,
                b: *b,
                children: children.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Inline::AllCaps { children, .. } => Inline::AllCaps {
                children: children.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Inline::SmallCaps { children, .. } => Inline::SmallCaps {
                children: children.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Inline::Hidden { children, .. } => Inline::Hidden {
                children: children.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
        }
    }
}
