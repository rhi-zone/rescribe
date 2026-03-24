//! CommonMark parser and emitter wrapping pulldown-cmark.
//!
//! A standalone crate with **no rescribe dependency** — usable as a general
//! Rust CommonMark library. The `rescribe-read-commonmark` and
//! `rescribe-write-commonmark` crates are thin adapter layers on top.
//!
//! # API layers (planned)
//!
//! ```text
//! // AST reader
//! pub fn parse(input: &[u8]) -> (CmDoc, Vec<Diagnostic>);
//!
//! // Streaming reader — iterator over owned events  [future: events.rs]
//! pub fn events(input: &[u8]) -> impl Iterator<Item = Event> + '_;
//!
//! // Batch reader — chunk-driven  [future: batch.rs]
//! let mut p = BatchParser::new();
//! p.feed(chunk); // repeat
//! let (doc, diags) = p.finish();
//!
//! // Builder writer — emit from AST  [future: emit.rs]
//! pub fn emit(doc: &CmDoc) -> String;
//!
//! // Streaming writer — emit from events  [future: writer.rs]
//! let mut w = Writer::new(sink);
//! w.write_event(event); // repeat
//! w.finish(); // flushes to sink
//! ```
//!
//! # Limitations
//!
//! The future [`StreamingParser`] API will buffer all input before parsing
//! because pulldown-cmark requires the full input as a `&str`. For true
//! chunked streaming, use pulldown-cmark directly. This limitation does not
//! affect [`parse`], which operates at maximum performance.
//!
//! Superseding pulldown-cmark (77M+ weekly downloads) is a non-goal.

#[cfg(feature = "ast")]
pub mod ast;
#[cfg(feature = "ast")]
pub mod emit;
#[cfg(feature = "ast")]
pub mod parse;

#[cfg(feature = "streaming")]
pub mod events;

#[cfg(feature = "batch")]
pub mod batch;

#[cfg(feature = "ast")]
pub use ast::{
    Block, CmDoc, Diagnostic, Inline, LinkDef, ListItem, ListKind, OrderedMarker, Severity, Span,
};
#[cfg(feature = "ast")]
pub use emit::emit;
#[cfg(feature = "ast")]
pub use parse::parse;

#[cfg(feature = "streaming")]
pub use events::{Event, EventIter, OwnedEvent, events, events_str};

#[cfg(feature = "batch")]
pub use batch::{Handler, StreamingParser};
