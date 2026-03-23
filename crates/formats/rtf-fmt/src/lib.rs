//! RTF (Rich Text Format) tokenizer, AST, and builder.
//!
//! A standalone crate with **no rescribe dependency** — usable as a general
//! Rust RTF library.  The `rescribe-read-rtf` and `rescribe-write-rtf` crates
//! are thin adapter layers on top.
//!
//! # API layers
//!
//! ```text
//! // AST reader
//! pub fn parse(input: &[u8]) -> (RtfDoc, Vec<Diagnostic>);
//! pub fn parse_str(input: &str) -> (RtfDoc, Vec<Diagnostic>);          // convenience
//!
//! // Streaming reader — low-level RTF token events
//! pub fn events(input: &[u8]) -> impl Iterator<Item = Event> + '_;
//! pub fn events_str(input: &str) -> impl Iterator<Item = Event> + '_;  // convenience
//!
//! // Batch reader — chunk-driven
//! pub struct batch::BatchParser { .. }  // feed/finish → (RtfDoc, Vec<Diagnostic>)
//! pub struct batch::BatchSink<F> { .. } // feed/finish → delivers Event tokens via callback
//!
//! // Streaming writer — RTF token serializer
//! pub struct writer::Writer<W: Write> { .. } // write_event(Event) / finish() → W
//!
//! // Builder writer
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
pub mod batch;
pub mod writer;

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use ast::{Align, Block, Diagnostic, Inline, RtfDoc, Severity, Span, TableRow};
pub use emit::emit;
pub use events::{Event, events, events_str};
pub use parse::{parse, parse_str};
