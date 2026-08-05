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
pub use rescribe_format_api::{Events, StreamingWrite};

// ── Trait implementations ───────────────────────────────────────────────────
//
// Only `Events` and `StreamingWrite` are implemented here. `Parse` and
// `Emit` are deliberately NOT implemented: this crate's `parse(&[u8]) ->
// Result<ParseResult<OdfDocument>, Error>` and `emit(&OdfDocument) ->
// Result<Vec<u8>, Error>` are hard-`Result`-returning (a corrupt ZIP or a
// failed write is a real `Error`, not a diagnosable-and-continue case —
// a deliberately different philosophy from zip-fmt's own `Parse` impl,
// which treats archive corruption as diagnosable). Forcing these into the
// trait's infallible `(Self, Vec<Diagnostic>)` / `Vec<u8>` shapes would
// mean silently swallowing the `Err` case or synthesizing a placeholder
// document/byte vector on failure — a real behavior change, not a
// mechanical rename, and not something to guess at (same category as
// rst-fmt's `Parse`/`Events` gap, documented in that crate's `lib.rs`).
//
// `StreamingParse` is also NOT implemented: unlike zip-fmt (which hand-
// rolled a genuinely incremental, `Handler`-dispatching chunk parser to
// work around ZIP's end-of-file central directory), odf-fmt's own
// `batch::BatchParser` is pull-style (`feed()` buffers, `finish()` parses
// the whole buffer and returns the AST) — there is no `Handler<E>`-based
// push-dispatch construct anywhere in this crate to hang a `StreamingParse`
// impl on. Building one would be new-feature work (a real, pre-existing
// gap against this crate's own documented "ODF is ZIP-based, true
// incremental parsing is not possible" limitation — see `batch.rs`'s
// module docs), not a trait migration.

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
