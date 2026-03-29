//! ANSI AST types: styled text spans, escape sequences, diagnostics.
//!
//! ANSI is not a document format — it is a stream of text interspersed with
//! terminal escape sequences.  The AST models this as a flat sequence of
//! [`AnsiNode`] entries, each being either styled text or a control sequence.

// ── Span / Diagnostic ─────────────────────────────────────────────────────────

/// Byte range in the original source input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    /// A zero-width span at the origin.  Used for programmatically constructed
    /// nodes that have no source position.
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
/// ANSI parsing is always infallible — malformed constructs produce
/// diagnostics instead of hard errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub span: Span,
    pub severity: Severity,
    pub message: String,
    pub code: &'static str,
}

// ── Color ─────────────────────────────────────────────────────────────────────

/// A color value used in SGR sequences.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    /// Standard 3-bit color (0–7).
    Standard(u8),
    /// Bright/high-intensity color (0–7, maps to SGR 90–97 / 100–107).
    Bright(u8),
    /// 256-color palette index (0–255).
    Palette(u8),
    /// 24-bit true color.
    Rgb(u8, u8, u8),
    /// Default terminal color (SGR 39 / 49).
    Default,
}

// ── Style ─────────────────────────────────────────────────────────────────────

/// The full SGR style state at a given point in the stream.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Style {
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub double_underline: bool,
    pub blink: bool,
    pub rapid_blink: bool,
    pub reverse: bool,
    pub hidden: bool,
    pub strikethrough: bool,
    pub overline: bool,
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub underline_color: Option<Color>,
}

impl Style {
    /// True when no attributes are set (equivalent to SGR 0).
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

// ── Cursor direction ──────────────────────────────────────────────────────────

/// Direction for cursor movement CSI sequences.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorDirection {
    Up,
    Down,
    Forward,
    Back,
}

// ── Erase mode ────────────────────────────────────────────────────────────────

/// Mode parameter for erase-in-display (CSI J) and erase-in-line (CSI K).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EraseMode {
    /// Erase from cursor to end (parameter 0 or absent).
    ToEnd,
    /// Erase from beginning to cursor (parameter 1).
    ToBeginning,
    /// Erase entire display/line (parameter 2).
    All,
}

// ── AST Node ──────────────────────────────────────────────────────────────────

/// A single element in an ANSI byte stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnsiNode {
    /// A run of plain or styled text.
    Text {
        text: String,
        style: Style,
        span: Span,
    },
    /// A newline character (`\n`).
    Newline { span: Span },
    /// Cursor movement (CSI A/B/C/D).
    CursorMove {
        direction: CursorDirection,
        count: u32,
        span: Span,
    },
    /// Cursor absolute position (CSI H / CSI f).
    CursorPosition { row: u32, col: u32, span: Span },
    /// Erase in display (CSI J).
    EraseDisplay { mode: EraseMode, span: Span },
    /// Erase in line (CSI K).
    EraseLine { mode: EraseMode, span: Span },
    /// Cursor show/hide (CSI ?25h / ?25l).
    CursorVisibility { visible: bool, span: Span },
    /// Save cursor position (CSI s / ESC 7).
    SaveCursor { span: Span },
    /// Restore cursor position (CSI u / ESC 8).
    RestoreCursor { span: Span },
    /// Set scroll region (CSI r).
    ScrollRegion {
        top: u32,
        bottom: u32,
        span: Span,
    },
    /// OSC hyperlink (OSC 8).
    Hyperlink {
        url: String,
        text: String,
        style: Style,
        span: Span,
    },
    /// An unrecognised escape sequence, preserved verbatim.
    RawEscape { content: String, span: Span },
}

impl AnsiNode {
    pub fn span(&self) -> Span {
        match self {
            AnsiNode::Text { span, .. }
            | AnsiNode::Newline { span }
            | AnsiNode::CursorMove { span, .. }
            | AnsiNode::CursorPosition { span, .. }
            | AnsiNode::EraseDisplay { span, .. }
            | AnsiNode::EraseLine { span, .. }
            | AnsiNode::CursorVisibility { span, .. }
            | AnsiNode::SaveCursor { span }
            | AnsiNode::RestoreCursor { span }
            | AnsiNode::ScrollRegion { span, .. }
            | AnsiNode::Hyperlink { span, .. }
            | AnsiNode::RawEscape { span, .. } => *span,
        }
    }

    pub fn strip_spans(&self) -> Self {
        match self {
            AnsiNode::Text { text, style, .. } => AnsiNode::Text {
                text: text.clone(),
                style: style.clone(),
                span: Span::NONE,
            },
            AnsiNode::Newline { .. } => AnsiNode::Newline { span: Span::NONE },
            AnsiNode::CursorMove {
                direction, count, ..
            } => AnsiNode::CursorMove {
                direction: *direction,
                count: *count,
                span: Span::NONE,
            },
            AnsiNode::CursorPosition { row, col, .. } => AnsiNode::CursorPosition {
                row: *row,
                col: *col,
                span: Span::NONE,
            },
            AnsiNode::EraseDisplay { mode, .. } => AnsiNode::EraseDisplay {
                mode: *mode,
                span: Span::NONE,
            },
            AnsiNode::EraseLine { mode, .. } => AnsiNode::EraseLine {
                mode: *mode,
                span: Span::NONE,
            },
            AnsiNode::CursorVisibility { visible, .. } => AnsiNode::CursorVisibility {
                visible: *visible,
                span: Span::NONE,
            },
            AnsiNode::SaveCursor { .. } => AnsiNode::SaveCursor { span: Span::NONE },
            AnsiNode::RestoreCursor { .. } => AnsiNode::RestoreCursor { span: Span::NONE },
            AnsiNode::ScrollRegion { top, bottom, .. } => AnsiNode::ScrollRegion {
                top: *top,
                bottom: *bottom,
                span: Span::NONE,
            },
            AnsiNode::Hyperlink {
                url, text, style, ..
            } => AnsiNode::Hyperlink {
                url: url.clone(),
                text: text.clone(),
                style: style.clone(),
                span: Span::NONE,
            },
            AnsiNode::RawEscape { content, .. } => AnsiNode::RawEscape {
                content: content.clone(),
                span: Span::NONE,
            },
        }
    }
}

// ── Document ──────────────────────────────────────────────────────────────────

/// A parsed ANSI document — a flat sequence of styled text runs and control
/// sequences.
#[derive(Debug, Clone, Default)]
pub struct AnsiDoc {
    pub nodes: Vec<AnsiNode>,
    pub span: Span,
}

impl AnsiDoc {
    /// Return a copy with all spans zeroed.
    pub fn strip_spans(&self) -> Self {
        AnsiDoc {
            nodes: self.nodes.iter().map(AnsiNode::strip_spans).collect(),
            span: Span::NONE,
        }
    }
}
