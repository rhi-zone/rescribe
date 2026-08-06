//! EPUB2/EPUB3 reader/writer, with a domain-typed package/manifest/spine/
//! navigation AST built on top of two other standalone crates rather than
//! reimplementing either layer:
//!
//! - [`zip_fmt`] for the OCF (ZIP) container — entry listing,
//!   decompression, and (for [`StreamingParser`]) genuine push-streaming
//!   over chunked input.
//! - [`html_fmt`] for every XHTML content document, including the EPUB3
//!   navigation document (`nav.xhtml`) — this crate never tokenizes HTML
//!   itself.
//!
//! What *is* this crate's own logic: `META-INF/container.xml`, the OPF
//! package document, and the EPUB2 NCX are all EPUB-specific XML
//! sub-formats with no independent ecosystem crate to delegate to (see
//! `xml.rs`'s module docs) — parsing/emitting those is genuinely
//! `epub-fmt`'s job, plus the manifest-driven classification that ties an
//! archive entry to "this is the nav document" / "this is a content
//! document" / "this is an opaque resource" (`classify.rs`).
//!
//! # API layers
//!
//! `EpubDoc` implements the five shared `rescribe-format-api` traits:
//!
//! ```text
//! use rescribe_format_api::{Emit, Events, Parse, StreamingParse, StreamingWrite};
//!
//! // AST reader
//! let (doc, diags): (EpubDoc, Vec<Diagnostic>) = EpubDoc::parse(input);
//!
//! // Streaming reader — lazy pull iterator; see events.rs for the
//! // OPF-classification-ordering design this needs that zip-fmt itself
//! // doesn't (ZIP has no cross-entry dependency; EPUB does).
//! let it: EventIter = EpubDoc::events(input);
//!
//! // Batch reader — chunk-driven, built on zip-fmt's own StreamingParser
//! let mut p = EpubDoc::streaming_parser(|ev| ...);
//! p.feed(chunk); // repeat
//! let diagnostics = p.finish();
//!
//! // Builder writer — wraps zip-fmt's builder emit
//! let bytes: Vec<u8> = doc.emit();
//!
//! // Streaming writer — built on zip-fmt's streaming Writer
//! let mut w = EpubDoc::writer(sink);
//! w.write_event(event); // repeat
//! let sink = w.finish()?;
//! ```
//!
//! # Losslessness
//!
//! See `ast.rs`'s module docs. Every OPF/NCX attribute and element this
//! crate does not specifically model is raw-preserved (`extra_attrs`/
//! [`RawXml`]), and every archive entry not resolved by the manifest is
//! kept verbatim ([`EpubDoc::unclassified`]).
//!
//! # Known gaps (documented, not silent)
//!
//! - SMIL media overlays and any other structured-but-unmodeled manifest
//!   item are preserved as opaque bytes in [`EpubDoc::resources`], not
//!   decoded into a domain structure.
//! - `META-INF/encryption.xml` presence and raw bytes are preserved
//!   ([`EpubDoc::encryption_xml`]); this crate never decrypts.
//! - EPUB3 fixed-layout metadata and Calibre/vendor series metadata are
//!   captured generically via [`crate::ast::MetaElement`] (no dedicated
//!   `fixed_layout`/`series` field) — round-trips losslessly, but callers
//!   wanting a typed accessor for these must read `property`/`value`
//!   themselves.
//! - `<collection>`/`<bindings>`/`<tours>` OPF elements are raw-preserved
//!   via [`crate::ast::RawXml`], not structurally modeled.

pub mod ast;
pub mod batch;
mod classify;
pub mod container;
pub mod emit;
pub mod events;
mod nav;
pub mod ncx;
pub mod opf;
pub mod parse;
mod pathutil;
#[cfg(feature = "rescribe")]
pub mod rescribe;
#[cfg(test)]
mod testutil;
pub mod writer;
mod xml;

// ── Public re-exports ─────────────────────────────────────────────────────

pub use ast::{
    Container, ContentDocument, DcElement, Diagnostic, EpubDoc, GuideRef, LinkElement,
    ManifestItem, MetaElement, Metadata, NavList, NavPoint, Navigation, Ncx, Package, RawXml,
    ResourceEntry, RootFile, Severity, Span, Spine, SpineItemRef,
};
pub use batch::{Handler, StreamingParser};
pub use events::{Event, EventIter, OwnedEvent};
pub use rescribe_format_api::{Emit, Events, Parse, StreamingParse, StreamingWrite};
pub use writer::{WriteEvent, Writer};

// ── Trait implementations ───────────────────────────────────────────────────

impl Parse for EpubDoc {
    fn parse(input: &[u8]) -> (Self, Vec<Diagnostic>) {
        parse::parse(input)
    }
}

impl Emit for EpubDoc {
    fn emit(&self) -> Vec<u8> {
        emit::emit(self)
    }
}

impl Events for EpubDoc {
    type Event<'a> = Event;
    type EventIter<'a> = EventIter<'a>;

    fn events(input: &[u8]) -> EventIter<'_> {
        EventIter::new(input)
    }
}

impl StreamingParse for EpubDoc {
    type Event = Event;
    type Parser<H: Handler<Event>> = StreamingParser<H>;

    fn streaming_parser<H: Handler<Event>>(handler: H) -> StreamingParser<H> {
        StreamingParser::new(handler)
    }
}

impl StreamingWrite for EpubDoc {
    type Writer<W: std::io::Write> = Writer<W>;

    fn writer<W: std::io::Write>(sink: W) -> Writer<W> {
        Writer::new(sink)
    }
}

#[cfg(test)]
mod smoke {
    use super::*;

    #[test]
    fn smoke_parse_and_roundtrip() {
        let bytes = testutil::sample_epub();
        let (doc, diags) = EpubDoc::parse(&bytes);
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert_eq!(doc.package.metadata.titles[0].value, "Sample Book");
        assert_eq!(doc.content_documents.len(), 2);
        assert!(doc.nav.is_some());
        assert_eq!(doc.resources.len(), 1);

        let emitted = doc.emit();
        let (doc2, diags2) = EpubDoc::parse(&emitted);
        assert!(diags2.is_empty(), "diagnostics: {diags2:?}");
        assert_eq!(doc.strip_spans(), doc2.strip_spans());
    }

    #[test]
    fn smoke_events() {
        let bytes = testutil::sample_epub();
        let evts: Vec<_> = EpubDoc::events(&bytes).collect();
        assert!(evts.iter().any(|e| matches!(e, Event::Package(_))));
    }

    #[test]
    fn smoke_streaming_parser() {
        let bytes = testutil::sample_epub();
        let mut evts = Vec::new();
        let mut p = StreamingParser::new(|ev| evts.push(ev));
        p.feed(&bytes);
        let diags = p.finish();
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert!(evts.iter().any(|e| matches!(e, Event::ContentDocument(_))));
    }

    #[test]
    fn smoke_streaming_writer() {
        let bytes = testutil::sample_epub();
        let (doc, _) = EpubDoc::parse(&bytes);
        let mut w = EpubDoc::writer(Vec::<u8>::new());
        w.write_event(WriteEvent::Container(doc.container.clone()));
        w.write_event(WriteEvent::Package {
            path: doc.container.rootfiles[0].full_path.clone(),
            package: Box::new(doc.package.clone()),
        });
        let out = w.finish().unwrap();
        assert!(out.starts_with(b"PK"));
    }
}
