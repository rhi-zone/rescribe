//! CommonMark parser and emitter wrapping pulldown-cmark.
//!
//! A standalone crate with **no rescribe dependency** — usable as a general
//! Rust CommonMark library. The `rescribe-read-commonmark` and
//! `rescribe-write-commonmark` crates are thin adapter layers on top.
//!
//! # API layers
//!
//! `CmDoc` implements the five shared `rescribe-format-api` traits:
//!
//! ```text
//! use rescribe_format_api::{Emit, Events, Parse, StreamingParse, StreamingWrite};
//!
//! // AST reader
//! let (doc, diags): (CmDoc, Vec<Diagnostic>) = CmDoc::parse(input);
//!
//! // Streaming reader — iterator over owned events
//! let it: EventIter = CmDoc::events(input);
//!
//! // Batch reader — chunk-driven
//! let mut p = CmDoc::streaming_parser(|ev| ...);
//! p.feed(chunk); // repeat
//! p.finish();
//!
//! // Builder writer — emit from AST
//! let bytes: Vec<u8> = doc.emit();
//!
//! // Streaming writer — emit from events
//! let mut w = CmDoc::writer(sink);
//! w.write_event(event); // repeat
//! w.finish(); // flushes to sink
//! ```
//!
//! `parse_str`/`events::events_str` (take `&str`, skip the UTF-8 check) and
//! `events::events` (byte slice, `Option<EventIter>` — `None` signals
//! invalid UTF-8) remain as separate, non-trait entry points: they have a
//! materially different contract from the trait methods, not a redundant
//! duplicate of them, and `multimarkdown-fmt` depends on them directly.
//!
//! # Limitations
//!
//! [`StreamingParser`] buffers all input before parsing because
//! pulldown-cmark requires the full input as a `&str`. For true chunked
//! streaming, use pulldown-cmark directly. This limitation does not affect
//! [`Parse::parse`](rescribe_format_api::Parse::parse), which operates at
//! maximum performance.
//!
//! Superseding pulldown-cmark (77M+ weekly downloads) is a non-goal.

mod options;

#[cfg(feature = "reader-ast")]
pub mod ast;
#[cfg(feature = "reader-ast")]
pub mod emit;
#[cfg(feature = "reader-ast")]
pub mod parse;

#[cfg(feature = "reader-streaming")]
pub mod events;

#[cfg(feature = "reader-batch")]
pub mod batch;

#[cfg(feature = "writer-streaming")]
pub mod writer;
#[cfg(feature = "writer-streaming")]
pub use writer::Writer;

#[cfg(all(feature = "reader-ast", feature = "definition-lists"))]
pub use ast::DefinitionListItem;
#[cfg(all(feature = "reader-ast", feature = "frontmatter"))]
pub use ast::FrontMatter;
#[cfg(feature = "reader-ast")]
pub use ast::{
    Block, CmDoc, Diagnostic, Inline, LinkDef, ListItem, ListKind, OrderedMarker, Severity, Span,
};
#[cfg(all(feature = "reader-ast", feature = "tables"))]
pub use ast::{TableCell, TableRow};
#[cfg(feature = "tables")]
pub use options::ColumnAlignment;
#[cfg(feature = "frontmatter")]
pub use options::FrontMatterKind;
#[cfg(feature = "reader-ast")]
pub use parse::parse_str;

#[cfg(feature = "reader-streaming")]
pub use events::{Event, EventIter, OwnedEvent, events, events_str};

#[cfg(feature = "reader-batch")]
pub use batch::{Handler, StreamingParser};

// ── Trait implementations ───────────────────────────────────────────────────
//
// `CmDoc` implements the five shared API-mode traits directly — the old
// crate-root `parse`/`emit` free functions (thin wrappers duplicating what
// is now the trait method) are gone. `parse_str`/`events_str`/`events`
// (byte-slice, `Option`-returning) stay as documented crate API: they have
// a materially different contract from the trait methods (str input
// skipping the UTF-8 check; `Option<EventIter>` signaling invalid UTF-8,
// which the infallible `Events::events` trait method cannot express) and
// are real external-consumer entry points (`multimarkdown-fmt` calls
// `parse_str`/`events_str` directly), not redundant duplicates.
#[cfg(feature = "reader-ast")]
impl rescribe_format_api::Parse for CmDoc {
    fn parse(input: &[u8]) -> (Self, Vec<Diagnostic>) {
        parse::parse(input)
    }
}

#[cfg(feature = "reader-ast")]
impl rescribe_format_api::Emit for CmDoc {
    fn emit(&self) -> Vec<u8> {
        emit::emit(self)
    }
}

#[cfg(feature = "reader-streaming")]
impl rescribe_format_api::Events for CmDoc {
    type Event<'a> = Event<'a>;
    type EventIter<'a> = EventIter<'a>;

    /// Invalid UTF-8 input yields an iterator over an empty document
    /// (`StartDocument`/`EndDocument` only) rather than panicking or
    /// returning `Option` — the trait's `events()` is infallible by
    /// contract and has no diagnostic channel (unlike `Parse::parse`).
    /// Callers that need to *distinguish* "empty document" from "invalid
    /// UTF-8" should use [`events()`](crate::events::events) directly,
    /// which returns `Option<EventIter>` for exactly that reason.
    fn events(input: &[u8]) -> EventIter<'_> {
        match std::str::from_utf8(input) {
            Ok(s) => events::EventIter::new(s),
            Err(_) => events::EventIter::new(""),
        }
    }
}

#[cfg(feature = "reader-batch")]
impl rescribe_format_api::StreamingParse for CmDoc {
    type Event = OwnedEvent;
    type Parser<H: Handler<OwnedEvent>> = StreamingParser<H>;

    fn streaming_parser<H: Handler<OwnedEvent>>(handler: H) -> StreamingParser<H> {
        StreamingParser::new(handler)
    }
}

#[cfg(feature = "writer-streaming")]
impl rescribe_format_api::StreamingWrite for CmDoc {
    type Writer<W: std::io::Write> = Writer<W>;

    fn writer<W: std::io::Write>(sink: W) -> Writer<W> {
        Writer::new(sink)
    }
}
