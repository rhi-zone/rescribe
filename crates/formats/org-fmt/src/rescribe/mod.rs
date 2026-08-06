//! AST<->rescribe::Document translation for Org-mode.
//!
//! Thin adapter layer that translates between [`crate::OrgDoc`] and
//! rescribe's `Document` IR. This module only exists when the `rescribe`
//! feature is enabled; each direction is additionally gated on the
//! reader/writer mode feature it depends on, so enabling `rescribe` alone
//! (with no mode feature) compiles nothing.

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
mod read;
#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
mod write;

#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
pub mod builder;

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
pub use read::{parse, parse_with_options};
#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
pub use write::{emit, emit_with_options};
