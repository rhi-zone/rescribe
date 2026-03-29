//! Fountain AST types.

use std::collections::BTreeMap;

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
/// Fountain parsing is always infallible — malformed constructs produce
/// diagnostics instead of hard errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub span: Span,
    pub severity: Severity,
    pub message: String,
    pub code: &'static str,
}

// ── Document ──────────────────────────────────────────────────────────────────

/// A parsed Fountain document.
#[derive(Debug, Clone, Default)]
pub struct FountainDoc {
    pub metadata: BTreeMap<String, String>,
    pub blocks: Vec<Block>,
    pub span: Span,
}

impl FountainDoc {
    /// Return a copy of this document with all spans zeroed.
    pub fn strip_spans(&self) -> Self {
        FountainDoc {
            metadata: self.metadata.clone(),
            blocks: self.blocks.iter().map(Block::strip_spans).collect(),
            span: Span::NONE,
        }
    }
}

// ── Block ─────────────────────────────────────────────────────────────────────

/// Block-level element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    /// Scene heading (INT./EXT./EST.)
    SceneHeading { text: String, span: Span },
    /// Action/narrative text
    Action { text: String, span: Span },
    /// Character name (possibly with dual dialogue marker)
    Character { name: String, dual: bool, span: Span },
    /// Dialogue line
    Dialogue { text: String, span: Span },
    /// Parenthetical direction
    Parenthetical { text: String, span: Span },
    /// Transition (CUT TO:, FADE OUT, etc.)
    Transition { text: String, span: Span },
    /// Centered text
    Centered { text: String, span: Span },
    /// Lyric (singing, musical notation)
    Lyric { text: String, span: Span },
    /// Note/comment [[text]]
    Note { text: String, span: Span },
    /// Synopsis =text
    Synopsis { text: String, span: Span },
    /// Section heading (#, ##, etc.)
    Section { level: usize, text: String, span: Span },
    /// Page break (===)
    PageBreak { span: Span },
    /// Boneyard / block comment (/* ... */)
    Boneyard { text: String, span: Span },
}

impl Block {
    /// Return a copy of this block with all spans zeroed.
    pub fn strip_spans(&self) -> Self {
        match self {
            Block::SceneHeading { text, .. } => Block::SceneHeading {
                text: text.clone(),
                span: Span::NONE,
            },
            Block::Action { text, .. } => Block::Action {
                text: text.clone(),
                span: Span::NONE,
            },
            Block::Character { name, dual, .. } => Block::Character {
                name: name.clone(),
                dual: *dual,
                span: Span::NONE,
            },
            Block::Dialogue { text, .. } => Block::Dialogue {
                text: text.clone(),
                span: Span::NONE,
            },
            Block::Parenthetical { text, .. } => Block::Parenthetical {
                text: text.clone(),
                span: Span::NONE,
            },
            Block::Transition { text, .. } => Block::Transition {
                text: text.clone(),
                span: Span::NONE,
            },
            Block::Centered { text, .. } => Block::Centered {
                text: text.clone(),
                span: Span::NONE,
            },
            Block::Lyric { text, .. } => Block::Lyric {
                text: text.clone(),
                span: Span::NONE,
            },
            Block::Note { text, .. } => Block::Note {
                text: text.clone(),
                span: Span::NONE,
            },
            Block::Synopsis { text, .. } => Block::Synopsis {
                text: text.clone(),
                span: Span::NONE,
            },
            Block::Section { level, text, .. } => Block::Section {
                level: *level,
                text: text.clone(),
                span: Span::NONE,
            },
            Block::PageBreak { .. } => Block::PageBreak { span: Span::NONE },
            Block::Boneyard { text, .. } => Block::Boneyard {
                text: text.clone(),
                span: Span::NONE,
            },
        }
    }
}
