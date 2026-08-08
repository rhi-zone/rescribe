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
//! `[Content_Types].xml`, `.rels`, or a generic part the moment that entry
//! finishes decompressing — see `ooxml_opc::batch`'s module docs for its
//! own buffering contract. This module only acts on one of those parts:
//! `word/document.xml`, the conventional path Word (and every other real
//! DOCX producer — LibreOffice, python-docx, the OpenXML SDK) writes the
//! main document body to. Its bytes are handed to [`crate::events::events`]
//! — the same hand-rolled `quick_xml::Reader`-based SAX iterator `events()`
//! already uses — and each [`OwnedWmlEvent`] it yields is forwarded to the
//! caller's [`Handler`] as it is produced, not collected into a `Vec`
//! first.
//!
//! ## Memory model: O(part size), not O(full archive) — and not O(nesting
//! ## depth) for `word/document.xml` specifically
//!
//! This is a real, documented improvement over [`BatchParser`] (was
//! O(full DOCX), now O(largest single part + nesting depth)), but **not**
//! the tightest possible bound, and that gap is deliberate, not
//! accidental: `ooxml_opc::StreamingParser`'s current `Event::Part`
//! delivers a part's **full decompressed content as one `Vec<u8>`**, not
//! incremental XML tokens (its own module docs, "Design: buffer-per-ZIP-
//! entry", document this as a scoped exception). That means this crate
//! cannot see `word/document.xml`'s bytes before the whole part has
//! streamed past and been reassembled by the OPC layer — the byte buffer
//! for that one part is unavoidably O(part size) with the current
//! `ooxml-opc` surface. What *is* genuinely incremental on top of that
//! buffer: `events()`'s `quick_xml::Reader` state machine never builds a
//! DOM or a full `WmlEvent` list — it holds only the open-container stack
//! (O(nesting depth)) and, transiently, one props element (`pPr`/`rPr`/…)
//! being parsed for the container that owns it. So peak *additional*
//! memory beyond the one buffered part is O(nesting depth + largest props
//! element), matching `events()`'s own bound exactly.
//!
//! Every other part (media, styles, numbering, etc.) is also fully
//! buffered momentarily by `ooxml_opc::StreamingParser` before its
//! `Event::Part` fires, per that crate's own contract — but this module
//! drops that content immediately without forwarding or retaining it, so
//! it costs only a transient O(that part's size), never accumulated
//! across parts.
//!
//! Tightening this further to true O(largest XML token) for
//! `word/document.xml` itself — feeding XML bytes into `quick_xml::Reader`
//! as they arrive mid-ZIP-entry, before decompression of the whole entry
//! completes — would require `ooxml_opc::StreamingParser` to expose
//! sub-entry chunks for a part instead of one final `Vec<u8>`. That is a
//! real architectural fork in `ooxml-opc`, out of scope for this crate:
//! `ooxml-opc` is a shared foundation crate also relied on by
//! `ooxml-sml`/`ooxml-pml`, so changing its `Event::Part` shape here is
//! not this module's call to make unilaterally. Document parts
//! (`word/document.xml` included) are typically well under the size where
//! this matters in practice — unlike embedded media, which this module
//! never even attempts to hold onto — so O(part size) is treated as an
//! acceptable interim bound, not a permanent one.
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

/// Adapts `ooxml_opc::batch::Event` to `OwnedWmlEvent`, forwarding only
/// `word/document.xml`'s content to `events()` and dropping every other
/// part's buffered bytes immediately.
struct InnerHandler<H: Handler<OwnedWmlEvent>> {
    handler: H,
}

impl<H: Handler<OwnedWmlEvent>> Handler<ooxml_opc::StreamingEvent> for InnerHandler<H> {
    fn handle(&mut self, event: ooxml_opc::StreamingEvent) {
        if let ooxml_opc::StreamingEvent::Part { path, content, .. } = event
            && path == MAIN_PART_PATH
        {
            for wml_event in crate::events::events(&content) {
                self.handler.handle(wml_event.into_owned());
            }
        }
        // `ContentTypes`, `Relationships`, and every other `Part` are
        // intentionally dropped here — see the module docs' "Locating
        // word/document.xml" section. `content` (if any) is freed when
        // this match arm's binding goes out of scope.
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
            inner: ooxml_opc::StreamingParser::new(InnerHandler { handler }),
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
