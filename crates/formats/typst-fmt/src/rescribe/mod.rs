//! AST↔`rescribe::Document` translation for Typst.
//!
//! This module only translates between [`TypstDoc`](crate::TypstDoc) (and,
//! on the `eval` path, `typst-html`'s `HtmlDocument`) and rescribe's
//! `Document` IR — no Typst markup parsing/emitting happens here (that all
//! lives in the rest of this crate; see `crate::parse` and `crate::emit`).
//! Enabled by the `rescribe` feature; each direction is additionally gated
//! on the reader/writer mode feature it depends on, so enabling `rescribe`
//! alone (with no mode feature) compiles nothing.
//!
//! Split across submodules (this crate's combined translation logic is
//! larger than a single-file layout comfortably holds):
//! - [`read`]: `TypstDoc` → `Document` (syntax-only parse path)
//! - [`write`]: `Document` → `TypstDoc`
//! - [`eval`]: full Typst-compiler evaluation path (`#let`/`#for`/`#if`/show
//!   rules, etc.) straight to `Document`, bypassing `TypstDoc` — gated on
//!   the separate `eval` feature (which also pulls in `reader-ast`, since
//!   it falls back to [`read::parse`] on compile failure)

#[cfg(feature = "reader-ast")]
mod read;
#[cfg(feature = "writer-builder")]
mod write;

#[cfg(feature = "eval")]
mod eval;

#[cfg(feature = "reader-ast")]
pub use read::{parse, parse_with_options};
#[cfg(feature = "writer-builder")]
pub use write::{emit, emit_with_options};

#[cfg(feature = "eval")]
pub use eval::parse_evaluated;
