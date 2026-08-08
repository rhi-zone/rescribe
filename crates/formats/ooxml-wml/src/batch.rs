//! Chunk-driven readers for DOCX documents: [`BatchParser`] (buffer-then-parse,
//! materialised `Document`) and [`StreamingParser`] (genuinely incremental,
//! `Handler`-driven [`OwnedWmlEvent`] stream).
//!
//! # `BatchParser`: buffer-then-parse
//!
//! [`BatchParser`] accepts arbitrary-sized chunks via [`BatchParser::feed`]
//! and parses the complete document on [`BatchParser::finish`]. All chunks
//! are buffered until `finish()` is called, so peak memory equals the full
//! DOCX size.
//!
//! ```ignore
//! use ooxml_wml::BatchParser;
//!
//! let mut parser = BatchParser::new();
//! for chunk in file_stream {
//!     parser.feed(&chunk);
//! }
//! let doc = parser.finish()?;
//! ```
//!
//! # `StreamingParser<H>`: genuinely incremental
//!
//! [`StreamingParser`] is driven by [`ooxml_opc::StreamingParser`] for
//! container-level (ZIP/OPC) entry delivery: `feed()` pushes raw `.docx`
//! archive bytes into the OPC layer, which classifies each ZIP entry into
//! `[Content_Types].xml`, `.rels`, or a generic part — see `ooxml_opc::batch`'s
//! module docs for its own buffering contract, updated 2026-08-08 so a
//! generic part's content now arrives as `PartStart`/`PartData`/`PartEnd`
//! (`PartData` chunks forwarded as they decompress, not buffered whole) once
//! `[Content_Types].xml` has streamed past. This module only acts on one
//! part: `word/document.xml`, the conventional path Word (and every other
//! real DOCX producer — LibreOffice, python-docx, the OpenXML SDK) writes
//! the main document body to. Every other part's `PartData` chunks are
//! discarded the moment they arrive, without ever being accumulated.
//!
//! `word/document.xml`'s own `PartData` chunks *are* accumulated into one
//! buffer (see "Memory model" below for why) until its `PartEnd`, at which
//! point the complete bytes are handed to [`crate::events::events`] — the
//! same hand-rolled `quick_xml::Reader`-based SAX iterator `events()`
//! already uses — and each [`OwnedWmlEvent`] it yields is forwarded to the
//! caller's [`Handler`] as it is produced, not collected into a `Vec`
//! first.
//!
//! ## Memory model: O(main part size + nesting depth), not O(full archive)
//!
//! This is a real, documented improvement over [`BatchParser`] (was
//! O(full DOCX), now O(largest single part + nesting depth)), but **not**
//! the tightest possible bound, and that gap is deliberate, not
//! accidental — now confined to a different layer than before. As of
//! 2026-08-08, `ooxml_opc::StreamingParser` itself delivers a generic
//! part's content sub-entry (`PartData` chunks as they decompress, not one
//! final `Vec<u8>` — see its own module docs), so the OPC container layer
//! is no longer the bottleneck. What *is* still O(part size) is this
//! crate's own XML layer: [`crate::events::events`] takes `bytes: &[u8]`
//! — a `quick_xml::Reader<&[u8]>` over the complete input — because this
//! crate has no chunk-fed XML tokenizer today. This module therefore still
//! accumulates `word/document.xml`'s `PartData` chunks into one buffer
//! before calling `events()` on the complete result. Reaching true
//! O(largest XML token) for `word/document.xml` would require feeding XML
//! bytes into a `quick_xml::Reader` (or an equivalent incremental
//! tokenizer) as `PartData` chunks arrive, before the whole part has
//! streamed past — a real architectural addition to this crate's XML
//! layer (not `ooxml-opc`'s container layer, which now already supports
//! sub-entry delivery), out of scope for this task and left as a follow-up
//! rather than silently left undocumented. What *is* genuinely incremental
//! on top of the one buffered part: `events()`'s `quick_xml::Reader` state
//! machine never builds a DOM or a full `WmlEvent` list — it holds only
//! the open-container stack (O(nesting depth)) and, transiently, one props
//! element (`pPr`/`rPr`/…) being parsed for the container that owns it. So
//! peak *additional* memory beyond the one buffered part is O(nesting
//! depth + largest props element), matching `events()`'s own bound
//! exactly.
//!
//! Every other part (media, styles, numbering, etc.) is never buffered at
//! all by this module — its `PartData` chunks are dropped as they arrive,
//! each chunk transient and bounded by `ooxml-opc`'s own O(largest
//! decompressor output chunk) delivery, never accumulated across a part or
//! across parts. This is a genuine improvement over the pre-2026-08-08
//! behavior, where `ooxml_opc::StreamingParser` fully buffered every part
//! (including ones this module immediately discarded) before this module
//! ever saw it.
//!
//! ## Locating `word/document.xml`: convention, not relationship resolution
//!
//! [`crate::document::Document::open`] (the AST/seekable path) resolves
//! the main part dynamically via the root `_rels/.rels`'
//! `rel_type::OFFICE_DOCUMENT` relationship, since OPC does not normatively
//! require the main part to live at `word/document.xml`. This module does
//! not do that resolution: `ooxml_opc::StreamingParser` deliberately does
//! not resolve relationship targets against parts (its own module docs
//! say so explicitly), and doing it here would mean buffering every part
//! that arrives before `_rels/.rels` does — on the (unenforced) chance it
//! turns out to be the main part — which would reintroduce exactly the
//! kind of unbounded prefix-buffering this module exists to avoid, for a
//! part identity that in practice is never anything but
//! `word/document.xml`. This module matches on that literal path. A
//! hypothetical package that renamed the main part would silently produce
//! no events here — a real, explicit limitation, not a silent one: it is
//! documented here and callers needing full relationship-based resolution
//! should use [`BatchParser`] or [`crate::document::Document::open`]
//! instead.
//!
//! ```ignore
//! use ooxml_wml::batch::{StreamingParser, Handler};
//! use ooxml_wml::OwnedWmlEvent;
//!
//! let mut events: Vec<OwnedWmlEvent> = Vec::new();
//! let mut p = StreamingParser::new(|ev: OwnedWmlEvent| events.push(ev));
//! for chunk in docx_bytes.chunks(4096) {
//!     p.feed(chunk);
//! }
//! let diagnostics = p.finish();
//! ```

use crate::Result;
use crate::document::Document;
use std::io::Cursor;

/// Chunk-driven DOCX parser.
///
/// See the [module documentation](self) for details.
#[derive(Default)]
pub struct BatchParser {
    buf: Vec<u8>,
}

impl BatchParser {
    /// Create a new parser.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a chunk of bytes to the internal buffer.
    pub fn feed(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);
    }

    /// Parse the buffered bytes as a DOCX document.
    pub fn finish(self) -> Result<Document<Cursor<Vec<u8>>>> {
        Document::from_reader(Cursor::new(self.buf))
    }
}

// ---------------------------------------------------------------------------
// StreamingParser<H>: genuinely incremental, Handler-driven WmlEvent stream.
// ---------------------------------------------------------------------------

/// Handler trait for streaming WML events — the shared
/// [`rescribe_format_api::Handler`], not a locally declared trait.
/// Implemented automatically for any `FnMut(OwnedWmlEvent)`.
pub use rescribe_format_api::{Diagnostic, Handler};

use crate::generated_events::OwnedWmlEvent;

/// The conventional OPC part path for a DOCX's main document body. See the
/// [module docs](self) for why this module matches on the literal path
/// rather than resolving it via `_rels/.rels`.
const MAIN_PART_PATH: &str = "word/document.xml";

/// Adapts `ooxml_opc::batch::Event` to `OwnedWmlEvent`. Accumulates
/// `word/document.xml`'s `PartData` chunks (see the module docs for why
/// this crate's own XML layer still needs the complete part) and forwards
/// the result to `events()` on that part's `PartEnd`; every other part's
/// `PartData` chunks are dropped as they arrive, never accumulated.
struct InnerHandler<H: Handler<OwnedWmlEvent>> {
    handler: H,
    /// Whether the part currently open (since the last `PartStart`) is
    /// `word/document.xml`.
    in_target: bool,
    buf: Vec<u8>,
}

impl<H: Handler<OwnedWmlEvent>> Handler<ooxml_opc::StreamingEvent> for InnerHandler<H> {
    fn handle(&mut self, event: ooxml_opc::StreamingEvent) {
        match event {
            ooxml_opc::StreamingEvent::PartStart { path, .. } => {
                self.in_target = path == MAIN_PART_PATH;
                self.buf.clear();
            }
            ooxml_opc::StreamingEvent::PartData(chunk) if self.in_target => {
                self.buf.extend_from_slice(&chunk);
            }
            ooxml_opc::StreamingEvent::PartEnd if self.in_target => {
                for wml_event in crate::events::events(&self.buf) {
                    self.handler.handle(wml_event.into_owned());
                }
                self.buf.clear();
                self.in_target = false;
            }
            // `ContentTypes`, `Relationships`, and every other part's
            // `PartData`/`PartEnd` are intentionally dropped here — see
            // the module docs' "Locating word/document.xml" section.
            _ => {}
        }
    }
}

/// Genuinely incremental, chunk-fed DOCX reader. See the [module
/// docs](self) for the memory model and how `word/document.xml` is
/// located. Additive alongside [`BatchParser`] and [`crate::events::events`]
/// — an independent way to reach the same [`OwnedWmlEvent`] vocabulary for
/// chunked/streamed input, not a wrapper that collects `events()` into a
/// `Vec` first.
pub struct StreamingParser<H: Handler<OwnedWmlEvent>> {
    inner: ooxml_opc::StreamingParser<InnerHandler<H>>,
}

impl<H: Handler<OwnedWmlEvent>> StreamingParser<H> {
    /// Create a new `StreamingParser` that delivers events to `handler`.
    pub fn new(handler: H) -> Self {
        StreamingParser {
            inner: ooxml_opc::StreamingParser::new(InnerHandler {
                handler,
                in_target: false,
                buf: Vec::new(),
            }),
        }
    }

    /// Feed the next chunk of `.docx` archive bytes. May be called with
    /// chunks of any size, including 1 byte, and any number of times.
    pub fn feed(&mut self, chunk: &[u8]) {
        self.inner.feed(chunk);
    }

    /// Signal end of input. Returns diagnostics accumulated by the
    /// underlying OPC/ZIP layers (malformed `[Content_Types].xml`,
    /// unparseable `.rels` parts, truncated archive, etc.).
    pub fn finish(self) -> Vec<Diagnostic> {
        self.inner.finish()
    }
}
