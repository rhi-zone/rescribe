//! FictionBook 2 (FB2) tokenizer, AST, and builder.
//!
//! A standalone crate with **no rescribe dependency**.
//! The `rescribe-read-fb2` and `rescribe-write-fb2` crates are thin adapters.
//!
//! # API
//!
//! ```text
//! // AST round-trip
//! pub fn parse(input: &[u8]) -> (FictionBook, Vec<Diagnostic>);
//! pub fn parse_str(input: &str) -> (FictionBook, Vec<Diagnostic>);
//! pub fn emit(fb: &FictionBook) -> Vec<u8>;
//!
//! // Streaming events (walks parsed FictionBook)
//! pub fn events(fb: &FictionBook) -> impl Iterator<Item = Event<'_>>;
//! ```

pub mod ast;
mod emit;
mod events;
mod parse;

pub use ast::*;
pub use emit::emit;
pub use events::{Event, EventIter, events};
pub use parse::{parse, parse_str};
