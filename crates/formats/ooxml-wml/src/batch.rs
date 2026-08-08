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
//! `word/document.xml`'s own `PartData` chunks are fed to
//! [`crate::chunked_events::ChunkedWmlReader`] as they arrive (see "Memory
//! model" below) — a genuinely independent, chunk-resumable
//! re-implementation of [`crate::events::WmlEventIter`]'s state machine
//! (not a wrapper around `events()` — see that module's docs for the full
//! technique and CLAUDE.md's "three independent implementations" rule).
//! Each [`OwnedWmlEvent`] it resolves is forwarded to the caller's
//! [`Handler`] as soon as it is provably complete, not collected into a
//! `Vec` first.
//!
//! ## Memory model: O(nesting depth + largest token/props-element/lookahead-chain)
//!
//! As of 2026-08-08, `ooxml_opc::StreamingParser` delivers a generic part's
//! content sub-entry (`PartData` chunks as they decompress, not one final
//! `Vec<u8>` — see its own module docs), so the OPC container layer was
//! already not the bottleneck. This module's own XML layer was: it used to
//! accumulate `word/document.xml`'s `PartData` chunks into one buffer
//! before calling [`crate::events::events`] on the complete result, making
//! peak memory O(main part size). That is now closed:
//! [`crate::chunked_events::ChunkedWmlReader`] feeds `PartData` chunks
//! directly into a chunk-resumable `quick_xml::Reader`-based tokenizer (the
//! same technique `docbook_fmt::batch::StreamingParser` uses, extended to
//! WML's props lookahead — see `chunked_events.rs`'s module docs for the
//! full argument), so `word/document.xml` itself is never buffered as a
//! whole. Peak memory is now O(nesting depth + largest still-in-progress
//! XML token, props element, or props-lookahead-recursion chain) — matching
//! [`crate::events::WmlEventIter`]'s own bound, just achieved
//! chunk-resumably instead of over one fully-buffered part.
//!
//! **One documented exception, inherited from the same technique
//! `docbook_fmt` uses:** a genuine XML syntax error (as opposed to a merely
//! truncated buffer that will resolve once more input arrives) cannot be
//! told apart from "needs more bytes" using quick-xml's return value alone.
//! `ChunkedWmlReader` therefore treats every such error as "wait for more
//! input" until `finish()`, which means a genuinely malformed (not just
//! not-yet-fully-arrived) construct forces buffering of the rest of that
//! malformed run until the part ends. This only affects malformed input;
//! well-formed `word/document.xml` — the case this module exists to
//! handle efficiently — never hits it.
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
use std::cell::RefCell;
use std::io::Cursor;
use std::rc::Rc;

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

use rescribe_format_api::Severity;
/// Handler trait for streaming WML events — the shared
/// [`rescribe_format_api::Handler`], not a locally declared trait.
/// Implemented automatically for any `FnMut(OwnedWmlEvent)`.
pub use rescribe_format_api::{Diagnostic, Handler};

use crate::chunked_events::ChunkedWmlReader;
use crate::generated_events::OwnedWmlEvent;

/// The conventional OPC part path for a DOCX's main document body. See the
/// [module docs](self) for why this module matches on the literal path
/// rather than resolving it via `_rels/.rels`.
const MAIN_PART_PATH: &str = "word/document.xml";

/// State shared between [`InnerHandler`] (owned by the inner
/// `ooxml_opc::StreamingParser`, consumed by its `finish()`) and the outer
/// [`StreamingParser`] (which needs to read the accumulated diagnostics
/// back out after that consuming `finish()` call returns) — the same
/// `Rc<RefCell<Shared<H>>>` shape `ooxml_opc::batch`'s own `StreamingParser`
/// uses for the identical reason (see that module's `Shared`).
struct Shared<H: Handler<OwnedWmlEvent>> {
    handler: H,
    diagnostics: Vec<Diagnostic>,
}

/// Adapts `ooxml_opc::batch::Event` to `OwnedWmlEvent`. Feeds
/// `word/document.xml`'s `PartData` chunks directly into a
/// [`ChunkedWmlReader`] as they arrive (see the module docs for the memory
/// model) instead of accumulating them; every other part's `PartData`
/// chunks are dropped as they arrive, never accumulated.
struct InnerHandler<H: Handler<OwnedWmlEvent>> {
    shared: Rc<RefCell<Shared<H>>>,
    /// Whether the part currently open (since the last `PartStart`) is
    /// `word/document.xml`.
    in_target: bool,
    reader: ChunkedWmlReader,
}

impl<H: Handler<OwnedWmlEvent>> Handler<ooxml_opc::StreamingEvent> for InnerHandler<H> {
    fn handle(&mut self, event: ooxml_opc::StreamingEvent) {
        match event {
            ooxml_opc::StreamingEvent::PartStart { path, .. } => {
                self.in_target = path == MAIN_PART_PATH;
                if self.in_target {
                    self.reader = ChunkedWmlReader::new();
                }
            }
            ooxml_opc::StreamingEvent::PartData(chunk) if self.in_target => {
                let mut shared = self.shared.borrow_mut();
                self.reader
                    .feed(&chunk, &mut |ev| shared.handler.handle(ev));
            }
            ooxml_opc::StreamingEvent::PartEnd if self.in_target => {
                let mut shared = self.shared.borrow_mut();
                if let Some(message) = self.reader.finish(&mut |ev| shared.handler.handle(ev)) {
                    shared
                        .diagnostics
                        .push(Diagnostic::new(Severity::Warning, message));
                }
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
    shared: Rc<RefCell<Shared<H>>>,
}

impl<H: Handler<OwnedWmlEvent>> StreamingParser<H> {
    /// Create a new `StreamingParser` that delivers events to `handler`.
    pub fn new(handler: H) -> Self {
        let shared = Rc::new(RefCell::new(Shared {
            handler,
            diagnostics: Vec::new(),
        }));
        let inner_handler = InnerHandler {
            shared: shared.clone(),
            in_target: false,
            reader: ChunkedWmlReader::new(),
        };
        StreamingParser {
            inner: ooxml_opc::StreamingParser::new(inner_handler),
            shared,
        }
    }

    /// Feed the next chunk of `.docx` archive bytes. May be called with
    /// chunks of any size, including 1 byte, and any number of times.
    pub fn feed(&mut self, chunk: &[u8]) {
        self.inner.feed(chunk);
    }

    /// Signal end of input. Returns diagnostics accumulated by the
    /// underlying OPC/ZIP layers (malformed `[Content_Types].xml`,
    /// unparseable `.rels` parts, truncated archive, etc.) plus any
    /// truncated/malformed trailing content in `word/document.xml` itself
    /// (see [`ChunkedWmlReader::finish`]).
    pub fn finish(self) -> Vec<Diagnostic> {
        let opc_diagnostics = self.inner.finish();
        let mut shared = Rc::try_unwrap(self.shared)
            .unwrap_or_else(|_| {
                panic!("ooxml-wml StreamingParser: internal state still referenced after finish")
            })
            .into_inner();
        shared.diagnostics.extend(opc_diagnostics);
        shared.diagnostics
    }
}
