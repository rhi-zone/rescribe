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
pub use rescribe_format_api::Span;

/// Severity of a [`Diagnostic`].
///
/// A diagnostic message produced during parsing.
///
/// RTF parsing is always infallible — malformed constructs are silently
/// tolerated and produce diagnostics instead of hard errors.
pub use rescribe_format_api::{Diagnostic, Severity};

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
    /// Inline span with raw RTF character-layout control words preserved verbatim.
    ///
    /// Captures words like `\dn3`, `\up2`, `\shad`, `\expnd10`, `\charscalex90`
    /// that have no cross-format semantic equivalent.  The `char_props` string
    /// contains the raw control words (e.g. `\\dn3\\shad`) exactly as they
    /// should be re-emitted.
    CharSpan {
        /// Raw RTF character-layout control words (e.g. `\dn3\shad`).
        char_props: String,
        children: Vec<Inline>,
        span: Span,
    },
    /// Inline span with an explicit font face from the font table.
    Font {
        /// Font name (e.g. `"Arial"`, `"Times New Roman"`).
        name: String,
        children: Vec<Inline>,
        span: Span,
    },
    /// Inline span with an explicit background (highlight) color.
    BgColor {
        r: u8,
        g: u8,
        b: u8,
        children: Vec<Inline>,
        span: Span,
    },

    /// Inline span with a language tag (from `\lang<N>`).
    ///
    /// `lcid` is the Windows LCID (e.g. 1033 = en-US, 1031 = de-DE).
    Lang {
        lcid: u16,
        children: Vec<Inline>,
        span: Span,
    },

    /// Footnote (or endnote) embedded at its reference position.
    ///
    /// In RTF the content appears inline as `{\footnote ...}` at the point
    /// in the body where the footnote marker is.  `content` holds the parsed
    /// blocks that make up the footnote body.
    Footnote {
        content: Vec<Block>,
        span: Span,
    },

    /// A floating/anchored drawing shape (RTF `\shp` destination group —
    /// the Word 97+ "Office Art" shape format).
    ///
    /// Modeled as `Inline` (not `Block`) because real-world RTF anchors the
    /// `{\shp ...}` group at a point inside a paragraph's inline content
    /// stream (confirmed from real Word-generated files: pandoc's test
    /// corpus and the `bitfocus/rtf2text` sample fixture both place `\shp`
    /// between `\pard` and `\par`), the same position `Inline::Footnote`
    /// occupies for its own out-of-flow block content — this variant
    /// follows that existing precedent rather than introducing a new
    /// block-level anchor mechanism.
    ///
    /// Coordinate semantics (`x`/`y`/`width`/`height` as twips) were
    /// determined by directly reading two independent real-world RTF files
    /// (not simulated/guessed): `shpleft`/`shptop`/`shpright`/`shpbottom`
    /// are absolute corner coordinates (`width = shpright - shpleft`,
    /// `height = shpbottom - shptop`), consistent with their names. The
    /// *unit* (twips) is RTF's universal measurement unit elsewhere in the
    /// spec and is corroborated by real corpus values that are only
    /// plausible as twips (e.g. a ~6.7in-wide shape on a Letter page), plus
    /// an independent implementation (ReactOS's `riched20` RTF reader,
    /// `rtf.h`) defining the sibling legacy `\dpx`/`\dpy`/`\dpxsize`/
    /// `\dpysize` words under its general `rtfTpi = 1440` twips-per-inch
    /// convention — but no primary-source spec text naming twips for
    /// `\shpleft` *specifically* was found (see rescribe.rs / TODO.md for
    /// the full sourcing trail). Treat "twips" here as strongly
    /// corroborated, not spec-confirmed.
    Shape {
        /// Left edge (`\shpleft`), in twips.
        x: i64,
        /// Top edge (`\shptop`), in twips.
        y: i64,
        /// `\shpright - \shpleft`, in twips.
        width: i64,
        /// `\shpbottom - \shptop`, in twips.
        height: i64,
        /// Explicit stacking order (`\shpz`).
        z_order: i64,
        /// Raw `\shpinst` control words not otherwise modeled above (wrap
        /// type, anchor-relative-to flags, `\shplid`, etc.), captured
        /// verbatim for lossless re-emission — same convention as
        /// `Block::Paragraph::para_props`/`Inline::CharSpan::char_props`.
        shape_props: String,
        /// Named shape properties (`{\sp{\sn name}{\sv value}}` groups),
        /// captured in document order. Values are raw RTF source text
        /// (never interpreted — a `pib` property's value, for example, is
        /// itself a nested `{\pict ...}` group) captured losslessly as text
        /// via the same "RTF source is pure ASCII, lossy UTF-8 conversion
        /// is safe" assumption already used for `\colortbl` parsing.
        named_props: Vec<(String, String)>,
        /// Parsed content of the shape's `\shptxt` group (the shape's
        /// actual text), if present. Empty if the shape has no `\shptxt`.
        text: Vec<Block>,
        /// Verbatim raw source text of the `\shprslt{...}` old-reader
        /// fallback group, if present (empty string otherwise). This is
        /// typically a legacy `\do` drawing object duplicating the same
        /// shape for readers that don't understand `\shp`; captured whole
        /// rather than modeled into its own IR shape (see TODO.md /
        /// rescribe.rs for why: no real-world evidence of a standalone
        /// top-level `\do`, and the legacy `\dodhgt` word's exact semantics
        /// — is it truly a z-order equivalent? — could not be confirmed).
        fallback_raw: String,
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
            Inline::CharSpan {
                char_props,
                children,
                span,
            } => Inline::CharSpan {
                char_props: char_props.clone(),
                children: merge_text_inlines(children.iter().map(Inline::normalize).collect()),
                span: *span,
            },
            Inline::Font {
                name,
                children,
                span,
            } => Inline::Font {
                name: name.clone(),
                children: merge_text_inlines(children.iter().map(Inline::normalize).collect()),
                span: *span,
            },
            Inline::BgColor {
                r,
                g,
                b,
                children,
                span,
            } => Inline::BgColor {
                r: *r,
                g: *g,
                b: *b,
                children: merge_text_inlines(children.iter().map(Inline::normalize).collect()),
                span: *span,
            },
            Inline::Lang {
                lcid,
                children,
                span,
            } => Inline::Lang {
                lcid: *lcid,
                children: merge_text_inlines(children.iter().map(Inline::normalize).collect()),
                span: *span,
            },
            Inline::Footnote { content, span } => Inline::Footnote {
                content: content.iter().map(Block::normalize).collect(),
                span: *span,
            },
            Inline::Shape {
                x,
                y,
                width,
                height,
                z_order,
                shape_props,
                named_props,
                text,
                fallback_raw,
                span,
            } => Inline::Shape {
                x: *x,
                y: *y,
                width: *width,
                height: *height,
                z_order: *z_order,
                shape_props: shape_props.clone(),
                named_props: named_props.clone(),
                text: text.iter().map(Block::normalize).collect(),
                fallback_raw: fallback_raw.clone(),
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
            | Inline::Hidden { span, .. }
            | Inline::CharSpan { span, .. }
            | Inline::Font { span, .. }
            | Inline::BgColor { span, .. }
            | Inline::Lang { span, .. }
            | Inline::Footnote { span, .. }
            | Inline::Shape { span, .. } => *span,
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
            Inline::CharSpan {
                char_props,
                children,
                ..
            } => Inline::CharSpan {
                char_props: char_props.clone(),
                children: children.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Inline::Font { name, children, .. } => Inline::Font {
                name: name.clone(),
                children: children.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Inline::BgColor {
                r, g, b, children, ..
            } => Inline::BgColor {
                r: *r,
                g: *g,
                b: *b,
                children: children.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Inline::Lang { lcid, children, .. } => Inline::Lang {
                lcid: *lcid,
                children: children.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Inline::Footnote { content, .. } => Inline::Footnote {
                content: content.iter().map(Block::strip_spans).collect(),
                span: Span::NONE,
            },
            Inline::Shape {
                x,
                y,
                width,
                height,
                z_order,
                shape_props,
                named_props,
                text,
                fallback_raw,
                ..
            } => Inline::Shape {
                x: *x,
                y: *y,
                width: *width,
                height: *height,
                z_order: *z_order,
                shape_props: shape_props.clone(),
                named_props: named_props.clone(),
                text: text.iter().map(Block::strip_spans).collect(),
                fallback_raw: fallback_raw.clone(),
                span: Span::NONE,
            },
        }
    }
}
