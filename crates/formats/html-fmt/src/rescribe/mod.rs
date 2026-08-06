//! AST↔`rescribe::Document` translation for HTML.
//!
//! This module only translates between [`HtmlDoc`](crate::HtmlDoc)/
//! [`Node`](crate::Node) and rescribe's `Document` IR — no HTML
//! tokenizing/parsing/emitting happens here (that all lives in the rest of
//! this crate; see `crate::parse` and `crate::emit`). Enabled by the
//! `rescribe` feature; each direction is additionally gated on the
//! reader/writer mode feature it depends on, so enabling `rescribe` alone
//! (with no mode feature) compiles nothing. Split into `read`/`write`
//! submodules (rather than one flat file, as `opml-fmt`'s reference
//! migration uses) because HTML's element vocabulary is large enough that
//! a single file would be unwieldy — following `docbook-fmt`'s precedent.

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
mod read;
#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
mod write;

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
pub use read::{parse, parse_with_options};
#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
pub use write::{emit, emit_full_document, emit_full_document_with_options, emit_with_options};
