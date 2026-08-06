//! AST<->rescribe::Document translation for ODF/ODT.
//!
//! Thin adapter layer that translates between [`crate::OdfDocument`] and
//! rescribe's `Document` IR by delegating all ZIP unpacking, XML parsing,
//! and XML serialisation to the rest of this crate. This module only
//! exists when the `rescribe` feature is enabled; each direction is
//! additionally gated on the reader/writer mode feature it depends on, so
//! enabling `rescribe` alone (with no mode feature) compiles nothing.

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
mod read;
#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
mod write;

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
pub use read::{parse, parse_with_options};
#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
pub use write::{emit, emit_with_options};
