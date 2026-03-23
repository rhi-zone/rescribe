//! Djot tokenizer, AST, and builder.
//!
//! A standalone crate with **no rescribe dependency** — usable as a general
//! Rust Djot library. The `rescribe-read-djot` and `rescribe-write-djot` crates
//! are thin adapter layers on top.
//!
//! # API layers
//!
//! ```text
//! // Low-level: streaming events (parse then iterate)
//! pub fn events(input: &str) -> impl Iterator<Item = EventOwned> + '_;
//!
//! // High-level: owned AST
//! pub fn parse(input: &str) -> (DjotDoc, Vec<Diagnostic>);
//! pub fn emit(doc: &DjotDoc) -> String;
//! ```
//!
//! # Round-trip
//!
//! For well-formed documents: `parse(emit(parse(input).0)).0.strip_spans()` should
//! equal `parse(input).0.strip_spans()`. Verified by the roundtrip fuzz harness.

mod ast;
mod emit;
mod events;
mod parse;

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use ast::{
    Alignment, Attr, Block, BulletStyle, DefItem, Diagnostic, DjotDoc, FootnoteDef, Inline,
    LinkDef, ListItem, ListKind, OrderedDelimiter, OrderedStyle, Span, TableCell, TableRow,
};
pub use emit::emit;
pub use events::{EventIter, EventOwned};
pub use parse::parse;

/// Return a streaming event iterator over the parsed document.
///
/// Parses the input first, then walks the AST yielding owned events.
/// For details on the event types, see [`EventOwned`].
pub fn events(input: &str) -> EventIter {
    EventIter::new(input)
}
