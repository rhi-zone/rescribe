//! Pandoc JSON reader and writer for rescribe.
//!
//! Parses and emits Pandoc's JSON AST format against rescribe's document IR.
//! This enables interoperability with Pandoc's extensive format support.
//! This wraps `serde_json` directly against the Pandoc JSON AST schema —
//! there is no separate general-purpose Pandoc-JSON parsing library to
//! delegate to.

mod read;
mod write;

pub use read::{parse, parse_with_options};
pub use write::{emit, emit_with_options};
