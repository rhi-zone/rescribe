//! OpenDocument Format (ODF) document library.
//!
//! A standalone Rust library for reading and writing ODF documents
//! (`.odt`, `.ods`, `.odp`). No rescribe dependency.
//!
//! # Quick start
//!
//! ```no_run
//! let bytes = std::fs::read("document.odt").unwrap();
//!
//! // Full AST parse
//! let result = odf_fmt::parse(&bytes).unwrap();
//! let doc = result.value;
//! println!("Title: {:?}", doc.meta.title);
//!
//! // SAX-style events
//! for event in odf_fmt::events(&bytes) {
//!     println!("{:?}", event);
//! }
//!
//! // Round-trip: parse then emit
//! let bytes2 = odf_fmt::emit(&doc).unwrap();
//! ```
//!
//! # API modes
//!
//! - **`parse()`** — full [`OdfDocument`] AST
//! - **`events()`** — SAX-style iterator, yields [`OdfEvent`] per element
//! - **`emit()`** — serialise an [`OdfDocument`] back to a ZIP archive
//!
//! # ODF version coverage
//!
//! - `feature = "odf-1-2"` — ISO 26300:2015 (OpenDocument v1.2, widely deployed)
//! - `feature = "odf-1-3"` — OASIS Standard 2021 (OpenDocument v1.3, current)
//!
//! Both are enabled by default. The generated schema types are in the
//! [`generated`] module and provide attribute structs and simple-type enums.

pub mod ast;
pub mod error;
pub mod events;
pub mod generated;
pub mod parser;
pub mod writer;

pub use ast::*;
pub use error::{Diagnostic, DiagLevel, Error, ParseResult};
pub use events::{EventIter, OdfEvent};

/// Parse an ODF ZIP archive from bytes and return a SAX-style event iterator.
///
/// Each call to `next()` on the returned iterator yields one [`OdfEvent`].
///
/// # Example
///
/// ```no_run
/// for event in odf_fmt::events(b"...") { }
/// ```
pub fn events(input: &[u8]) -> EventIter {
    events::events(input)
}

/// Parse an ODF ZIP archive from bytes into a full [`OdfDocument`] AST.
///
/// Returns a [`ParseResult`] containing the document and any non-fatal
/// diagnostics emitted during parsing.
///
/// # Errors
///
/// Returns [`Error`] if the input is not a valid ZIP archive or if the
/// archive cannot be read.
pub fn parse(input: &[u8]) -> Result<ParseResult<OdfDocument>, Error> {
    parser::parse(input)
}

/// Serialise an [`OdfDocument`] to an ODF ZIP archive.
///
/// # Errors
///
/// Returns [`Error`] if writing the ZIP archive fails.
pub fn emit(doc: &OdfDocument) -> Result<Vec<u8>, Error> {
    writer::emit(doc)
}
