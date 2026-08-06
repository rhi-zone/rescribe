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
//! - **[`batch::BatchParser`]** — chunk-driven (feed/finish) reader
//! - **[`batch::Writer`]** — event-driven writer (feed events, flush ZIP on finish)
//!
//! # ODF version coverage
//!
//! - `feature = "odf-1-2"` — ISO 26300:2015 (OpenDocument v1.2, widely deployed)
//! - `feature = "odf-1-3"` — OASIS Standard 2021 (OpenDocument v1.3, current)
//!
//! Both are enabled by default. The generated schema types are in the
//! [`generated`] module and provide attribute structs and simple-type enums.

pub mod ast;
pub mod batch;
pub mod error;
pub mod events;
pub mod generated;
pub mod parser;
pub mod writer;

pub use ast::*;
pub use error::{DiagLevel, Diagnostic, Error, ParseResult};
pub use events::{EventIter, OdfEvent};
pub use rescribe_format_api::{Emit, Events, Parse, StreamingWrite};

// ── Trait implementations ───────────────────────────────────────────────────
//
// `Parse` and `Emit` wrap `parser::parse_lenient`/`writer::emit_lenient`,
// diagnostic-and-continue variants added alongside the pre-existing
// `Result`-returning `parser::parse`/`writer::emit` (still exported below,
// still used by `rescribe-read-odt`/`rescribe-write-odt` and the batch
// API, which want the hard-`Result` for real I/O failures). The two
// variants differ only in how a ZIP archive that fails to open at all is
// handled — every other fallible-looking construct in `parser.rs`
// already degrades gracefully (`Err(_) => break`) rather than
// propagating an error, so there was no wider "silently swallow errors"
// behavior change hiding in this split. See `parser::parse_lenient` and
// `writer::emit_lenient` for the exact shape, matching zip-fmt's own
// `Parse`/`Emit` impls (`crates/formats/zip-fmt/src/parse.rs`,
// `emit.rs`).
//
// `StreamingParse` is still NOT implemented: unlike zip-fmt (which hand-
// rolled a genuinely incremental, `Handler`-dispatching chunk parser to
// work around ZIP's end-of-file central directory), odf-fmt's own
// `batch::BatchParser` is pull-style (`feed()` buffers, `finish()` parses
// the whole buffer and returns the AST) — there is no `Handler<E>`-based
// push-dispatch construct anywhere in this crate to hang a `StreamingParse`
// impl on. This is a separate, tracked gap (see TODO.md) from the
// `Parse`/`Emit` gap this commit closes.

impl Parse for OdfDocument {
    fn parse(input: &[u8]) -> (Self, Vec<Diagnostic>) {
        parser::parse_lenient(input)
    }
}

impl Emit for OdfDocument {
    fn emit(&self) -> Vec<u8> {
        writer::emit_lenient(self)
    }
}

impl Events for OdfDocument {
    type Event<'a> = OdfEvent<'static>;
    type EventIter<'a> = EventIter;

    fn events(input: &[u8]) -> EventIter {
        events::events(input)
    }
}

impl StreamingWrite for OdfDocument {
    type Writer<W: std::io::Write> = batch::Writer<W>;

    fn writer<W: std::io::Write>(sink: W) -> batch::Writer<W> {
        batch::Writer::new(sink)
    }
}

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
