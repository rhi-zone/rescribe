//! AST types for Typst markup documents.
//!
//! Typst's own `typst-syntax` crate already gives a materialized, byte-span-
//! carrying parse tree (`SyntaxNode`/the typed `ast::*` cast layer) — a
//! legitimate domain AST for Typst in its own right, unlike a generic XML
//! tree (the situation `docbook-fmt`/`jats-fmt`/`tei-fmt` model with a fully
//! open element tree). We still define our own domain-typed [`TypstDoc`]
//! here rather than exposing `typst_syntax::SyntaxNode` directly as "the"
//! AST, for one concrete reason: the writer side needs a type it can
//! *construct* (there is no upstream "Typst document builder" — `SyntaxNode`
//! is a rowan-style green tree meant to be produced by parsing, not hand
//! assembled) and the fuzz roundtrip property required by this crate's
//! completion checklist (`parse(emit(arbitrary_ast)).strip_spans() ==
//! arbitrary_ast`) needs one shared type on both ends. [`parse`](crate::parse::parse_str)
//! builds a `TypstDoc` by walking `typst_syntax`'s tree; [`emit`](crate::emit::emit)
//! consumes a `TypstDoc` to produce markup bytes.
//!
//! The construct set modeled here matches what the pre-existing
//! `rescribe-read-typst`/`rescribe-write-typst` adapters supported before
//! this crate was extracted (paragraphs, headings, lists, definition lists,
//! code, blockquotes, tables, figures, footnotes, sub/superscript,
//! underline/strike, math, links, images, line breaks) plus a horizontal
//! rule and the writer's own `math_display` case, which the reader side
//! never populated. Constructs `typst-syntax` can parse but this AST has no
//! dedicated variant for (an unrecognized function call, a raw block-level
//! `#{ ... }` code expression, etc.) are preserved verbatim via
//! [`Block::Raw`] / [`Inline::Raw`] rather than dropped — see
//! `fixtures/typst/COVERAGE.md` for the exact boundary of what is
//! semantically modeled vs. raw-preserved today.

/// Byte offset span in the source input — the shared
/// `rescribe_format_api::Span`, not a locally declared type. See that
/// crate for why `Span`/`Diagnostic`/`Severity` are shared across every
/// format crate.
pub use rescribe_format_api::Span;

/// A parsed Typst document: a flat sequence of top-level blocks.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct TypstDoc {
    pub blocks: Vec<Block>,
    pub span: Span,
}

impl TypstDoc {
    pub fn strip_spans(&self) -> TypstDoc {
        TypstDoc {
            blocks: self.blocks.iter().map(Block::strip_spans).collect(),
            span: Span::NONE,
        }
    }
}

/// A block-level construct.
#[derive(Clone, Debug, PartialEq)]
pub enum Block {
    Paragraph(Vec<Inline>),
    Heading {
        level: u8,
        body: Vec<Inline>,
    },
    CodeBlock {
        lang: Option<String>,
        content: String,
    },
    /// A bullet (`-`) or numbered (`+`/`1.`) list. Each item is itself a
    /// list of blocks (matching the pre-existing adapter's "list item wraps
    /// a paragraph" shape, but open-ended for a list item containing more
    /// than one paragraph).
    List {
        ordered: bool,
        items: Vec<Vec<Block>>,
    },
    /// A `/ term: description` definition list. Each entry is
    /// `(term, description)`, both inline-content lists.
    DefinitionList(Vec<(Vec<Inline>, Vec<Inline>)>),
    /// `#quote[...]`.
    Quote(Vec<Block>),
    Table {
        columns: usize,
        /// Row-major flat cell list; each cell is inline content.
        rows: Vec<Vec<Vec<Inline>>>,
    },
    Figure {
        body: Option<Box<Block>>,
        caption: Option<Vec<Inline>>,
    },
    HorizontalRule,
    MathDisplay(String),
    /// A block-level image (`#image("...")` used as a standalone block).
    Image {
        url: String,
    },
    /// A block-level construct this AST has no dedicated variant for
    /// (an unrecognized function call, `#{ ... }` code, etc.), captured as
    /// its exact Typst source text.
    Raw(String),
}

impl Block {
    pub fn strip_spans(&self) -> Block {
        // No block variant currently carries its own Span field (spans live
        // only at the TypstDoc level today); cloning is the strip.
        self.clone()
    }
}

/// An inline-level construct.
#[derive(Clone, Debug, PartialEq)]
pub enum Inline {
    Text(String),
    Strong(Vec<Inline>),
    Emph(Vec<Inline>),
    Underline(Vec<Inline>),
    Strike(Vec<Inline>),
    Subscript(Vec<Inline>),
    Superscript(Vec<Inline>),
    Code(String),
    Link {
        url: String,
        body: Vec<Inline>,
    },
    Image {
        url: String,
    },
    LineBreak,
    MathInline(String),
    /// A block-style equation (`$ ... $`, `eq.block() == true` in
    /// `typst-syntax`) that appears in inline/paragraph position rather
    /// than as its own top-level block — matches the pre-existing
    /// `rescribe-read-typst` behavior of placing such equations inside the
    /// enclosing paragraph rather than splitting the paragraph around them.
    /// Distinct from [`Block::MathDisplay`], which the *writer* side uses
    /// for a standalone block-level `$ ... $` construct (never produced by
    /// [`crate::parse::parse_str`] today — see `fixtures/typst/COVERAGE.md`).
    MathDisplay(String),
    Footnote(Vec<Inline>),
    /// `#smallcaps[...]`.
    SmallCaps(Vec<Inline>),
    /// A quoted span. `double` selects `"..."` vs `'...'` — matches
    /// rescribe's `style:quote_type` property (`"double"`/`"single"`).
    /// Writer-only today: `"..."`/`'...'` in Typst source is
    /// `typst-syntax`'s `SmartQuote` token wrapping plain text, not a
    /// nestable span construct, so [`crate::parse::parse_str`] has no path
    /// that produces a grouped `Quoted` node (it produces standalone quote-
    /// character `Text` nodes instead, matching the pre-existing adapter).
    Quoted {
        double: bool,
        body: Vec<Inline>,
    },
    /// An inline construct this AST has no dedicated variant for, captured
    /// as its exact Typst source text.
    Raw(String),
}

/// Diagnostic message from parsing — the shared
/// `rescribe_format_api::{Diagnostic, Severity}`, not locally declared
/// types.
pub use rescribe_format_api::{Diagnostic, Severity};
