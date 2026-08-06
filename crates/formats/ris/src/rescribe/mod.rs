//! AST<->rescribe::Document translation for the RIS format.
//!
//! Thin adapter layer that translates between `crate::ast` types and
//! rescribe's `Document` IR. This module only exists when the `rescribe`
//! feature is enabled; the rest of the crate has no rescribe dependency.

mod read;
mod write;

pub use read::{parse, parse_with_options};
pub use write::{emit, emit_with_options};
