//! RTF (Rich Text Format) tokenizer, AST, and builder.
//!
//! A standalone crate with **no rescribe dependency by default** — usable as
//! a general Rust RTF library. The optional `rescribe` feature (default off)
//! adds `crate::rescribe::{parse, emit}`, a thin adapter that translates
//! `rtf_fmt::RtfDoc` to and from rescribe's `Document` IR.
//!
//! # API layers
//!
//! `RtfDoc` implements the five shared `rescribe-format-api` traits:
//!
//! ```text
//! use rescribe_format_api::{Emit, Events, Handler, Parse, StreamingParse, StreamingWrite};
//!
//! // AST reader
//! let (doc, diags): (RtfDoc, Vec<Diagnostic>) = RtfDoc::parse(input);
//!
//! // Streaming reader — iterator over owned semantic document-level events
//! let it: SemanticEventIter = RtfDoc::events(input);
//!
//! // Batch reader — chunk-driven
//! let mut p = RtfDoc::streaming_parser(|ev| ...);
//! p.feed(chunk); // repeat
//! p.finish();
//!
//! // Builder writer — emit from AST
//! let bytes: Vec<u8> = doc.emit();
//!
//! // Streaming writer — emit from semantic events
//! let mut w = RtfDoc::writer(sink);
//! w.write_event(event); // repeat
//! w.finish(); // flushes to sink
//! ```
//!
//! `parse_str`/`events_str` (take `&str`, skip the UTF-8-from-bytes step) and
//! the low-level RTF token API (`token_events`/`token_events_str`,
//! [`batch::BatchSink`], [`writer::Writer`]) remain as separate, non-trait
//! entry points: they have a materially different contract from the shared
//! traits (`&str` input; or an entirely different, lower abstraction level —
//! raw RTF tokens rather than document-level semantic events), not redundant
//! duplicates of them.
//!
//! # Round-trip guarantee
//!
//! For any document `doc` in canonical form,
//! `RtfDoc::parse(&doc.emit()).strip_spans() == doc.strip_spans()`.
//! Use `RtfDoc::normalize()` to put a programmatically-built document into
//! canonical form before round-tripping.  Verified by the fuzz round-trip
//! harness (`fuzz_rtf_roundtrip`).

#[cfg(test)]
mod alloc_probe;
mod ast;
pub mod batch;
mod emit;
mod events;
mod incremental;
mod parse;
#[cfg(feature = "rescribe")]
pub mod rescribe;
mod sem_events;
pub mod sem_writer;
mod tables;
pub mod writer;

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use ast::{Align, Block, Diagnostic, Inline, RtfDoc, Severity, Span, TableRow};
// Semantic event API (document-level). `events()` (byte-slice) is exposed
// through the `Events` trait impl below; `events_str` stays a separate public
// entry point (see its doc comment for why).
pub use sem_events::{Event, OwnedEvent, SemanticEventIter, events_str};
// Token-level event API (raw RTF tokens) — not one of the five shared
// `rescribe-format-api` capabilities (no document-level AST/event mapping),
// so it stays as its own free-function API.
pub use events::{TokenEvent, token_events, token_events_str};
// `parse()` (byte-slice) is exposed through the `Parse` trait impl below;
// `parse_str` stays a separate public entry point (see its doc comment for why).
pub use parse::parse_str;
// Shared font/color table computation — used by both `emit()` and
// `sem_events::events()`'s `Event::StartDocument`, and by any caller that
// needs to replicate their exact index assignment.
pub use tables::{build_color_map, build_font_map, collect_used_colors, collect_used_fonts};

// ── Trait implementations ───────────────────────────────────────────────────
//
// `RtfDoc` implements all five shared `rescribe-format-api` capability
// traits. Unlike `rst-fmt`, `RtfDoc` has no lifetime parameter and `parse`/
// `events` already match the trait signatures exactly (`&[u8]` input,
// infallible `(Self, Vec<Diagnostic>)`/owned-event-iterator return shapes),
// so all five traits apply cleanly — no structural mismatch to document here.

impl rescribe_format_api::Parse for RtfDoc {
    fn parse(input: &[u8]) -> (Self, Vec<Diagnostic>) {
        parse::parse(input)
    }
}

impl rescribe_format_api::Emit for RtfDoc {
    fn emit(&self) -> Vec<u8> {
        emit::emit(self).into_bytes()
    }
}

impl rescribe_format_api::Events for RtfDoc {
    type Event<'a> = OwnedEvent;
    type EventIter<'a> = SemanticEventIter;

    fn events(input: &[u8]) -> SemanticEventIter {
        sem_events::events(input)
    }
}

impl rescribe_format_api::StreamingParse for RtfDoc {
    type Event = OwnedEvent;
    type Parser<H: rescribe_format_api::Handler<OwnedEvent>> = batch::StreamingParser<H>;

    fn streaming_parser<H: rescribe_format_api::Handler<OwnedEvent>>(
        handler: H,
    ) -> batch::StreamingParser<H> {
        batch::StreamingParser::new(handler)
    }
}

impl rescribe_format_api::StreamingWrite for RtfDoc {
    type Writer<W: std::io::Write> = sem_writer::Writer<W>;

    fn writer<W: std::io::Write>(sink: W) -> sem_writer::Writer<W> {
        sem_writer::Writer::new(sink)
    }
}
