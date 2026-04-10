//! OpenDocument Format (ODF) text document support.
//!
//! A standalone Rust library for reading and writing ODF documents
//! (`.odt`, `.ods`, `.odp`). No rescribe dependency.
//!
//! # API modes
//!
//! - **`parse()`** (`feature = "reader-ast"`) — full tree, infallible, `Span` on every node
//! - **`events()`** (`feature = "reader-streaming"`) — SAX-style iterator, no full-tree allocation
//! - **`StreamingParser`** (`feature = "reader-batch"`) — chunk-driven, O(working state)
//! - **`emit()`** / **`Writer`** (`feature = "writer-builder"` / `"writer-streaming"`) — write ODF output
//!
//! # ODF version coverage
//!
//! - `feature = "odf-1-2"` — ISO 26300:2015 (OpenDocument v1.2, widely deployed)
//! - `feature = "odf-1-3"` — OASIS Standard 2021 (OpenDocument v1.3, current)
//!
//! Both are enabled by default.

mod generated;

pub use generated::*;
