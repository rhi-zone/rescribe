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

impl Default for Span {
    fn default() -> Self {
        Self::NONE
    }
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

/// Block-level element.
#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Heading {
        level: u8,
        kind: HeadingKind,
        inlines: Vec<Inline>,
        span: Span,
    },
    Paragraph {
        inlines: Vec<Inline>,
        span: Span,
    },
    CodeBlock {
        variant: CodeBlockVariant,
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
    Table {
        rows: Vec<TableRow>,
        span: Span,
    },
    Menu {
        entries: Vec<MenuEntry>,
        span: Span,
    },
    HorizontalRule {
        span: Span,
    },
    /// Raw/conditional block (`@iftex`, `@ifhtml`, etc.) preserved verbatim.
    RawBlock {
        environment: String,
        content: String,
        span: Span,
    },
    /// `@float` environment with optional type and label.
    Float {
        float_type: Option<String>,
        label: Option<String>,
        children: Vec<Block>,
        span: Span,
    },
    /// `@noindent` directive.
    NoIndent {
        span: Span,
    },
}

/// What kind of heading command produced this heading.
#[derive(Debug, Clone, PartialEq)]
pub enum HeadingKind {
    /// `@chapter`, `@section`, `@subsection`, `@subsubsection`
    Numbered,
    /// `@unnumbered`, `@unnumberedsec`, etc.
    Unnumbered,
    /// `@appendix`, `@appendixsec`, etc.
    Appendix,
}

/// Code block variant.
#[derive(Debug, Clone, PartialEq)]
pub enum CodeBlockVariant {
    Example,
    SmallExample,
    Verbatim,
    Lisp,
    Display,
    Format,
}

/// A row in a `@multitable`.
#[derive(Debug, Clone, PartialEq)]
pub struct TableRow {
    pub is_header: bool,
    pub cells: Vec<Vec<Inline>>,
}

/// An entry in a `@menu` block.
#[derive(Debug, Clone, PartialEq)]
pub struct MenuEntry {
    pub node: String,
    pub description: Option<String>,
}

impl Block {
    /// Return a copy of this block with all spans set to [`Span::NONE`].
    pub fn strip_spans(&self) -> Self {
        match self {
            Block::Heading { level, kind, inlines, .. } => Block::Heading {
                level: *level,
                kind: kind.clone(),
                inlines: inlines.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::Paragraph { inlines, .. } => Block::Paragraph {
                inlines: inlines.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::CodeBlock { variant, content, .. } => Block::CodeBlock {
                variant: variant.clone(),
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
            Block::Table { rows, .. } => Block::Table {
                rows: rows
                    .iter()
                    .map(|row| TableRow {
                        is_header: row.is_header,
                        cells: row
                            .cells
                            .iter()
                            .map(|cell| cell.iter().map(Inline::strip_spans).collect())
                            .collect(),
                    })
                    .collect(),
                span: Span::NONE,
            },
            Block::Menu { entries, .. } => Block::Menu {
                entries: entries.clone(),
                span: Span::NONE,
            },
            Block::HorizontalRule { .. } => Block::HorizontalRule { span: Span::NONE },
            Block::RawBlock { environment, content, .. } => Block::RawBlock {
                environment: environment.clone(),
                content: content.clone(),
                span: Span::NONE,
            },
            Block::Float { float_type, label, children, .. } => Block::Float {
                float_type: float_type.clone(),
                label: label.clone(),
                children: children.iter().map(Block::strip_spans).collect(),
                span: Span::NONE,
            },
            Block::NoIndent { .. } => Block::NoIndent { span: Span::NONE },
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
    /// `@var{text}` — semantic variable name (rendered in italics by convention).
    Var(Vec<Inline>, Span),
    /// `@file{name}` — file name.
    File(String, Span),
    /// `@command{cmd}` — command name.
    Command(String, Span),
    /// `@option{-o}` — command-line option.
    Option(String, Span),
    /// `@env{VAR}` — environment variable.
    Env(String, Span),
    /// `@samp{sample}` — sample text.
    Samp(String, Span),
    /// `@kbd{C-c}` — keyboard input.
    Kbd(String, Span),
    /// `@key{RET}` — key name.
    Key(String, Span),
    /// `@dfn{term}` — defining occurrence of a term.
    Dfn(Vec<Inline>, Span),
    /// `@cite{reference}` — citation.
    Cite(String, Span),
    /// `@acronym{ACR}` or `@acronym{ACR, expansion}`.
    Acronym {
        abbrev: String,
        expansion: Option<String>,
        span: Span,
    },
    /// `@abbr{abbr}` or `@abbr{abbr, expansion}`.
    Abbr {
        abbrev: String,
        expansion: Option<String>,
        span: Span,
    },
    /// `@r{text}` — Roman font.
    Roman(String, Span),
    /// `@sc{text}` — small caps.
    SmallCaps(String, Span),
    /// `@i{text}` — italic font (direct).
    DirectItalic(Vec<Inline>, Span),
    /// `@b{text}` — bold font (direct).
    DirectBold(Vec<Inline>, Span),
    /// `@t{text}` — typewriter font (direct).
    DirectTypewriter(String, Span),
    Link {
        url: String,
        children: Vec<Inline>,
        span: Span,
    },
    /// `@image{file,width,height,alt,ext}`.
    Image {
        file: String,
        width: Option<String>,
        height: Option<String>,
        alt: Option<String>,
        extension: Option<String>,
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
    /// `@xref{node}`, `@ref{node,text}`, `@pxref{node}`.
    CrossRef {
        kind: CrossRefKind,
        node: String,
        text: Option<String>,
        span: Span,
    },
    /// `@anchor{name}`.
    Anchor {
        name: String,
        span: Span,
    },
    /// `@w{text}` — non-breaking (no-wrap) text.
    NoBreak(String, Span),
    /// `@email{addr}` or `@email{addr, text}`.
    Email {
        address: String,
        text: Option<String>,
        span: Span,
    },
    /// Symbol commands: `@dots{}`, `@copyright{}`, etc.
    Symbol(SymbolKind, Span),
}

/// Cross-reference command kind.
#[derive(Debug, Clone, PartialEq)]
pub enum CrossRefKind {
    /// `@xref{node}` — "See ..."
    Xref,
    /// `@ref{node}` — bare reference
    Ref,
    /// `@pxref{node}` — "see ..."
    Pxref,
}

/// Known symbol commands.
#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    /// `@dots{}` → `...`
    Dots,
    /// `@enddots{}` → `....`
    EndDots,
    /// `@minus{}` → minus sign
    Minus,
    /// `@copyright{}` → (C)
    Copyright,
    /// `@registeredsymbol{}` → (R)
    Registered,
    /// `@LaTeX{}` → `LaTeX`
    LaTeX,
    /// `@TeX{}` → `TeX`
    TeX,
    /// `@tie{}` → non-breaking space
    Tie,
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
            Inline::Var(children, _) => {
                Inline::Var(children.iter().map(Inline::strip_spans).collect(), Span::NONE)
            }
            Inline::File(s, _) => Inline::File(s.clone(), Span::NONE),
            Inline::Command(s, _) => Inline::Command(s.clone(), Span::NONE),
            Inline::Option(s, _) => Inline::Option(s.clone(), Span::NONE),
            Inline::Env(s, _) => Inline::Env(s.clone(), Span::NONE),
            Inline::Samp(s, _) => Inline::Samp(s.clone(), Span::NONE),
            Inline::Kbd(s, _) => Inline::Kbd(s.clone(), Span::NONE),
            Inline::Key(s, _) => Inline::Key(s.clone(), Span::NONE),
            Inline::Dfn(children, _) => {
                Inline::Dfn(children.iter().map(Inline::strip_spans).collect(), Span::NONE)
            }
            Inline::Cite(s, _) => Inline::Cite(s.clone(), Span::NONE),
            Inline::Acronym { abbrev, expansion, .. } => Inline::Acronym {
                abbrev: abbrev.clone(),
                expansion: expansion.clone(),
                span: Span::NONE,
            },
            Inline::Abbr { abbrev, expansion, .. } => Inline::Abbr {
                abbrev: abbrev.clone(),
                expansion: expansion.clone(),
                span: Span::NONE,
            },
            Inline::Roman(s, _) => Inline::Roman(s.clone(), Span::NONE),
            Inline::SmallCaps(s, _) => Inline::SmallCaps(s.clone(), Span::NONE),
            Inline::DirectItalic(children, _) => {
                Inline::DirectItalic(children.iter().map(Inline::strip_spans).collect(), Span::NONE)
            }
            Inline::DirectBold(children, _) => {
                Inline::DirectBold(children.iter().map(Inline::strip_spans).collect(), Span::NONE)
            }
            Inline::DirectTypewriter(s, _) => Inline::DirectTypewriter(s.clone(), Span::NONE),
            Inline::Link { url, children, .. } => Inline::Link {
                url: url.clone(),
                children: children.iter().map(Inline::strip_spans).collect(),
                span: Span::NONE,
            },
            Inline::Image { file, width, height, alt, extension, .. } => Inline::Image {
                file: file.clone(),
                width: width.clone(),
                height: height.clone(),
                alt: alt.clone(),
                extension: extension.clone(),
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
            Inline::CrossRef { kind, node, text, .. } => Inline::CrossRef {
                kind: kind.clone(),
                node: node.clone(),
                text: text.clone(),
                span: Span::NONE,
            },
            Inline::Anchor { name, .. } => Inline::Anchor {
                name: name.clone(),
                span: Span::NONE,
            },
            Inline::NoBreak(s, _) => Inline::NoBreak(s.clone(), Span::NONE),
            Inline::Email { address, text, .. } => Inline::Email {
                address: address.clone(),
                text: text.clone(),
                span: Span::NONE,
            },
            Inline::Symbol(kind, _) => Inline::Symbol(kind.clone(), Span::NONE),
        }
    }
}
