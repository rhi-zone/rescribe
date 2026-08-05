//! Shared trait contract for the rescribe `-fmt` standalone format crates.
//!
//! This crate has **no rescribe dependency** — it is pure plumbing shared
//! across `opml-fmt`, `zip-fmt`, `commonmark-fmt`, `rst-fmt`, and the rest
//! of `crates/formats/*`, unifying:
//!
//! - Five capability traits — [`Parse`], [`Events`], [`StreamingParse`],
//!   [`Emit`], [`StreamingWrite`] — one per API mode described in
//!   `docs/format-library-design.md`. A crate implements whichever apply;
//!   flat tabular/citation formats with no meaningful event stream (CSV,
//!   TSV, RIS) are not required to implement all five.
//! - [`Handler<E>`] — **one** shared handler trait that every crate's
//!   chunk-driven `StreamingParser<H>` bounds `H` by, instead of each crate
//!   declaring its own concrete `Handler` trait. This is not cosmetic: a
//!   feasibility spike (`crates/formats/format-crate-trait`, see its
//!   `step3_streaming_wall.rs`) proved that bridging N independently
//!   declared `Handler` traits behind one generic trait via a blanket impl
//!   hits E0210 (orphan rule) — the only real fix is for every crate to
//!   depend on and implement *this* trait directly, so there is only ever
//!   one `Handler` trait in existence.
//! - [`Diagnostic`] / [`Severity`] / [`Span`] — one shared shape, replacing
//!   the ~37 divergent per-crate definitions found in the pre-migration
//!   survey (some missing `severity`, one using `level` instead of
//!   `severity`, some missing `span` or `code`). All fields the union of
//!   crates needed are present; a crate that never populates `code` uses
//!   `""`, a crate that never computes byte spans uses `Span::NONE` — no
//!   crate loses information it previously captured.
//!
//! # What this crate deliberately does not unify
//!
//! `StreamingParser<H>`/`Writer<W>` themselves stay concrete, per-crate
//! types (`opml_fmt::batch::StreamingParser<H>`, ...) — only the *bound*
//! (`Handler<E>`) is shared. This preserves full monomorphization; nothing
//! here introduces `dyn` dispatch or a common runtime primitive that the
//! three reader APIs' independent-implementation requirement forbids.

use std::io::Write as IoWrite;

// ── Diagnostics ─────────────────────────────────────────────────────────────

/// Byte-offset span into a format crate's source input.
///
/// Shared because every crate's `Diagnostic` embeds one; the field shape
/// (`start`/`end`, half-open `[start, end)`) was already identical across
/// every crate surveyed except for name (`Span` everywhere).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    /// Placeholder for a diagnostic that has no meaningful byte span (a few
    /// crates — e.g. `fb2-fmt`, `odf-fmt` pre-migration — never computed
    /// one). Using this instead of dropping the field preserves the
    /// "has a span" contract in `docs/format-library-design.md` for every
    /// diagnostic while costing nothing for crates that never had span
    /// data to begin with.
    pub const NONE: Span = Span { start: 0, end: 0 };

    pub const fn new(start: usize, end: usize) -> Self {
        Span { start, end }
    }
}

/// Severity level for a [`Diagnostic`].
///
/// `Error` is included alongside `docs/format-library-design.md`'s
/// documented `Warning`/`Info` because `fb2-fmt` and `odf-fmt` (pre-
/// migration) genuinely distinguished a third, more severe level; folding
/// it into `Warning` would have dropped real information those crates
/// captured. Most crates only ever construct `Warning`/`Info`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub enum Severity {
    Info,
    Warning,
    Error,
}

/// A diagnostic message produced while parsing or emitting.
///
/// Union of every field any surveyed crate's local `Diagnostic` type
/// carried: `span` + `severity` + `message` + `code`. A crate that doesn't
/// track one of these uses the documented default (`Span::NONE`, `""`)
/// rather than omitting the field — omitting it would mean re-diverging
/// the shape per crate again, defeating the point of unifying it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Diagnostic {
    pub span: Span,
    pub severity: Severity,
    pub message: String,
    /// Machine-readable code, e.g. `"rst::ambiguous-underline"`. `""` when
    /// the producing crate doesn't assign codes.
    pub code: &'static str,
}

impl Diagnostic {
    pub fn new(severity: Severity, message: impl Into<String>) -> Self {
        Diagnostic {
            span: Span::NONE,
            severity,
            message: message.into(),
            code: "",
        }
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.span = span;
        self
    }

    pub fn with_code(mut self, code: &'static str) -> Self {
        self.code = code;
        self
    }
}

// ── Reader API 1: full-tree parse ───────────────────────────────────────────

/// AST reader: full tree in memory, direct recursive descent.
///
/// The implementing type **is** the AST (`impl Parse for OpmlDoc`), not a
/// separate marker type — `parse()` is an associated function returning
/// `Self`, so there is no pre-existing instance to attach the impl to
/// otherwise.
pub trait Parse: Sized {
    fn parse(input: &[u8]) -> (Self, Vec<Diagnostic>);
}

// ── Writer API 1: full-tree emit ────────────────────────────────────────────

/// Builder writer: serialize an already-parsed AST all at once.
pub trait Emit {
    fn emit(&self) -> Vec<u8>;
}

// ── Reader API 2: streaming pull iterator ───────────────────────────────────

/// Streaming reader: the parser IS the iterator.
///
/// `Event<'a>` is a GAT because it borrows from the `input: &[u8]` passed
/// to `events()`, not from `&self`/`Self` — a plain associated type can't
/// express a lifetime that varies per call. Some crates' events are owned
/// regardless of `'a` (e.g. `zip-fmt`, where decompression can't borrow);
/// the GAT tolerates an impl that simply doesn't use its lifetime
/// parameter — verified in the feasibility spike.
pub trait Events {
    type Event<'a>
    where
        Self: 'a;
    type EventIter<'a>: Iterator<Item = Self::Event<'a>>
    where
        Self: 'a;

    fn events(input: &[u8]) -> Self::EventIter<'_>;
}

// ── Shared Handler<E> ────────────────────────────────────────────────────────

/// Callback trait for chunk-driven ([`StreamingParse`]) parsing.
///
/// **This is the one and only `Handler` trait** every format crate's
/// `StreamingParser<H>` bounds `H` by. Format crates used to each declare
/// their own concrete `Handler` trait (`opml_fmt::batch::Handler`,
/// `zip_fmt::batch::Handler`, ...); bridging those after the fact behind a
/// shared generic trait is impossible without an orphan-rule-violating
/// blanket impl (confirmed: E0210, see `format-crate-trait`'s
/// `step3_streaming_wall.rs`). Depending on and implementing *this* trait
/// directly — rather than declaring a local one — is what makes
/// `StreamingParser<H: Handler<E>>` a shared shape across every crate
/// instead of 44 incompatible ones.
pub trait Handler<E> {
    fn handle(&mut self, event: E);
}

impl<E, F: FnMut(E)> Handler<E> for F {
    fn handle(&mut self, event: E) {
        self(event)
    }
}

// ── Reader API 3: chunk-driven streaming ────────────────────────────────────

/// Chunked streaming reader: `feed()` advances a state machine and calls
/// `handler.handle(event)` for each event provably complete so far.
///
/// `Parser<H>` stays a concrete, crate-defined type (e.g.
/// `opml_fmt::batch::StreamingParser<H>`) — only the bound on `H` is
/// shared, via [`Handler`]. This preserves full monomorphization; nothing
/// here erases to `dyn`.
pub trait StreamingParse {
    /// The (typically owned — see [`Handler`]'s docs) event type delivered
    /// to the handler.
    type Event;
    type Parser<H: Handler<Self::Event>>;

    fn streaming_parser<H: Handler<Self::Event>>(handler: H) -> Self::Parser<H>;
}

// ── Writer API 2: streaming writer ──────────────────────────────────────────

/// Streaming writer: emit bytes as events arrive, no intermediate buffer.
///
/// `Writer<W>` stays a concrete, crate-defined type — this trait only
/// names the capability and its constructor generically. Unlike
/// [`StreamingParse`], there is no orphan-rule wall to work around here:
/// every crate's `Writer<W: std::io::Write>` already bounds `W` by the one
/// trait in `std`, so there was never a divergent per-crate trait to
/// unify on the writer side.
pub trait StreamingWrite {
    type Writer<W: IoWrite>;

    fn writer<W: IoWrite>(sink: W) -> Self::Writer<W>;
}
