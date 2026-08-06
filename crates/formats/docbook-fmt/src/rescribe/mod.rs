//! AST↔`rescribe::Document` translation for DocBook.
//!
//! This module only translates between [`DocBookDoc`](crate::DocBookDoc)/
//! [`Node`](crate::Node) and rescribe's `Document` IR — no XML
//! tokenizing/parsing/emitting happens here (that all lives in the rest of
//! this crate; see `crate::parse` and `crate::emit`). Enabled by the
//! `rescribe` feature; each direction is additionally gated on the
//! reader/writer mode feature it depends on, so enabling `rescribe` alone
//! (with no mode feature) compiles nothing. Split into `read`/`write`
//! submodules (rather than one flat file, as `opml-fmt`'s reference
//! migration uses) because DocBook's element vocabulary is large enough
//! that a single file would be unwieldy.

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
mod read;
#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
mod write;

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
pub use read::parse;
#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
pub use write::emit;
