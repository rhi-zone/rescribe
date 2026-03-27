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
//! pub fn parse_str(input: &str) -> (RtfDoc, Vec<Diagnostic>);              // convenience
//!
//! // Streaming reader — semantic document-level events
//! pub fn events(input: &[u8]) -> SemanticEventIter;
//! pub fn events_str(input: &str) -> SemanticEventIter;                     // convenience
//!
//! // Streaming reader — low-level RTF token events
//! pub fn token_events(input: &[u8]) -> impl Iterator<Item = TokenEvent> + '_;
//! pub fn token_events_str(input: &str) -> impl Iterator<Item = TokenEvent> + '_;  // convenience
//!
//! // Batch reader — chunk-driven
//! pub struct batch::BatchParser { .. }   // feed/finish → (RtfDoc, Vec<Diagnostic>)
//! pub struct batch::BatchSink<F> { .. }  // feed/finish → delivers TokenEvent tokens via callback
//!
//! // Streaming writer — RTF token serializer
//! pub struct writer::Writer<W: Write> { .. } // write_event(TokenEvent) / finish() → W
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
mod sem_events;
pub mod batch;
pub mod writer;

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use ast::{Align, Block, Diagnostic, Inline, RtfDoc, Severity, Span, TableRow};
pub use emit::emit;
// Semantic event API (document-level)
pub use sem_events::{Event, OwnedEvent, SemanticEventIter, events, events_str};
// Token-level event API (raw RTF tokens)
pub use events::{TokenEvent, token_events, token_events_str};
pub use parse::{parse, parse_str};
