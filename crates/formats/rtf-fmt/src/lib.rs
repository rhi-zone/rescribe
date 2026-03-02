//! RTF (Rich Text Format) tokenizer, AST, and builder.
//!
//! A standalone crate with **no rescribe dependency** — usable as a general
//! Rust RTF library.  The `rescribe-read-rtf` and `rescribe-write-rtf` crates
//! are thin adapter layers on top.
//!
//! # API layers
//!
//! ```text
//! // Low-level: pull tokenizer — zero allocation, streaming
//! pub fn events(input: &str) -> impl Iterator<Item = Event> + '_;
//!
//! // High-level: owned AST
//! pub fn parse(input: &str) -> (RtfDoc, Vec<Diagnostic>);
//! pub fn emit(doc: &RtfDoc) -> String;
//! ```
//!
//! # Round-trip guarantee
//!
//! For any document `doc` in canonical form,
//! `parse(emit(doc)).strip_spans() == doc.strip_spans()`.
//! Use `RtfDoc::normalize()` to put a programmatically-built document into
//! canonical form before round-tripping.  Verified by the fuzz round-trip
//! harness (`fuzz_rtf_roundtrip`).

mod ast;
mod emit;
mod events;
mod parse;

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use ast::{Block, Diagnostic, Inline, RtfDoc, Severity, Span, TableRow};
pub use emit::emit;
pub use events::{Event, events};
pub use parse::parse;
