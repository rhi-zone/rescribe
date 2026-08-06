//! AST<->rescribe::Document translation for the POD format.
//!
//! Thin adapter layer that translates between [`crate::PodDoc`] and
//! rescribe's `Document` IR. This module only exists when the `rescribe`
//! feature is enabled; the rest of the crate has no rescribe dependency.

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
mod read;
#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
mod write;

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
pub use read::{parse, parse_with_options};
#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
pub use write::{emit, emit_with_options};
