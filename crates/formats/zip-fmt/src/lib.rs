//! ZIP container reader/writer, with a domain-typed `Archive`/`Entry` AST
//! and a genuinely incremental push-streaming reader — **no rescribe
//! dependency**. Usable as a general Rust ZIP-container library by
//! anything that needs to read or write the ZIP envelope: an EPUB reader
//! (`epub-fmt`, planned), an OOXML (`.docx`/`.xlsx`/`.pptx`) package
//! layer, a JAR/APK inspector, a generic archive utility.
//!
//! `zip-fmt` operates at **raw bytes per entry** — it does not interpret
//! entry content as any document format. A caller that opens an EPUB gets
//! back `("content.opf", bytes)`, `("OEBPS/chapter1.xhtml", bytes)`, ...
//! and is responsible for parsing those itself. This mirrors the existing
//! `ooxml-opc` (packaging) / `ooxml-wml`+`-sml`+`-pml` (content) split in
//! this workspace.
//!
//! # Why this crate exists: the push-streaming gap
//!
//! [`zip`](https://docs.rs/zip) (already a workspace dependency, used by
//! `ooxml-opc`/`odf-fmt`/`rescribe-read-odt`/`rescribe-read-pptx`) is a
//! solid, actively maintained ZIP library — but it has a real, verified
//! gap on the **push-streaming read side** that this crate exists to
//! close (see `batch.rs`'s module docs for the full source citation):
//! `zip::read::stream::ZipStreamReader`/`read_zipfile_from_stream` are
//! blocking/pull-based (they own a `Read` and call it synchronously, not
//! resumable across externally-driven `feed(chunk)` calls) *and* — per
//! `ZipFileData::from_local_block` — they outright reject any entry using
//! a trailing data descriptor (general-purpose flag bit 3), which is
//! exactly the shape a non-seekable writer (including this crate's own
//! [`Writer`]) produces for every entry.
//!
//! `epub-fmt` (not yet started) and a future `ooxml-fmt` streaming rework
//! both need a true `feed()`-shaped ZIP reader for exactly this reason —
//! see `CLAUDE.md`'s "Streaming IR" and "`ooxml-fmt` and the streaming
//! imperative" sections.
//!
//! # Why this crate wraps `zip` for four of its five APIs, and not the fifth
//!
//! `zip::ZipArchive<R: Read + Seek>`/`zip::ZipWriter<W: Write + Seek>` work
//! fine over an in-memory buffer — a `&[u8]`/`Vec<u8>` is trivially
//! `Seek`-able via `std::io::Cursor`, and the ZIP format's own layout is
//! inherently seek-oriented (the authoritative entry list lives in the
//! central directory at the *end* of the file). There is no push-streaming
//! *gap* for `reader-ast` (`parse.rs`), `reader-streaming` (`events.rs`),
//! `writer-builder` (`emit.rs`), or `writer-streaming` (`writer.rs`,
//! adapting `zip::write::ZipWriter::new_stream` — see that module's docs
//! for why this is "adapting", not "hand-rolling an encoder", unlike the
//! reader side). This is the same shape as the `commonmark-fmt` exception
//! documented in `CLAUDE.md`/`docs/format-library-design.md`: wrap the
//! library where it already does the job well, hand-roll only the part it
//! genuinely cannot do (`reader-batch`'s [`StreamingParser`], `batch.rs`).
//! Unlike `commonmark-fmt`, that hand-rolled part is not a documented
//! *permanent* limitation of the underlying library's design (pulldown-cmark
//! requires a full `&str` by its own architecture) — it is a gap in `zip`'s
//! current public API that a future `zip` release could close; this crate
//! fills it in the meantime with its own local header parser + resumable
//! `flate2::Decompress`-based inflate loop.
//!
//! # API layers
//!
//! `Archive` implements the five shared `rescribe-format-api` traits — no
//! parallel free functions exist alongside them:
//!
//! ```text
//! use rescribe_format_api::{Emit, Events, Parse, StreamingParse, StreamingWrite};
//!
//! // AST reader (wraps zip::ZipArchive over Cursor<&[u8]>)
//! let (archive, diags): (Archive, Vec<Diagnostic>) = Archive::parse(input);
//!
//! // Streaming reader — lazy pull iterator, one entry decompressed per next()
//! let it: EventIter = Archive::events(input);
//!
//! // Batch reader — the hand-rolled, genuinely chunk-fed reader (see batch.rs)
//! let mut p = Archive::streaming_parser(|ev| ...);
//! p.feed(chunk); // repeat, any chunk size, any number of times
//! let diagnostics = p.finish();
//!
//! // Builder writer — wraps zip::ZipWriter over Cursor<Vec<u8>>
//! let bytes: Vec<u8> = archive.emit();
//!
//! // Streaming writer — adapts zip::write::ZipWriter::new_stream (see writer.rs)
//! let mut w = Archive::writer(sink);
//! w.write_entry(&entry); // repeat
//! let sink = w.finish()?;
//! ```
//!
//! # Compression coverage
//!
//! `zip`'s default features (all enabled here) decode Store, Deflate,
//! Deflate64, Bzip2, LZMA, XZ, PPMd, Zstd, and AES-encrypted entries;
//! legacy Shrink/Reduce/Implode require its `legacy-zip` feature, not
//! enabled here (rare in modern archives). `zip-fmt` itself only
//! **decompresses/recompresses** Store and Deflate on every API — the
//! two methods that cover virtually all real EPUB/OOXML/ODF files. Any
//! other declared method's entry content is preserved as raw
//! (still-compressed) bytes rather than dropped (see `parse.rs`'s
//! `decompress_raw` and `emit.rs`'s `to_zip_method` for exactly how), but
//! this crate cannot re-encode into those methods, and demotes them to
//! `Store` on write (content bytes preserved; the method-code round-trip
//! for that one entry is not — documented, not silent).
//!
//! # No multi-disk / split-archive support
//!
//! Inherited from `zip`, which does not support multi-volume archives.
//! Out of scope for this crate; not a gap `zip-fmt` introduces.
//!
//! # Fidelity limits on the write side
//!
//! See `emit.rs`'s and `writer.rs`'s doc comments: `zip::write::FileOptions`
//! has no setter for the raw general-purpose flags word, `version_made_by`,
//! `internal_attrs`, or the DOS-attribute half of `external_attrs`, so
//! those `Entry` fields do not round-trip byte-for-byte through
//! `emit()`/`Writer`. Extra-field bytes and content do.

pub mod ast;
pub mod batch;
pub mod emit;
pub mod events;
pub mod parse;
pub mod writer;

pub use ast::{Archive, CompressionMethod, Diagnostic, DosTimestamp, Entry, Severity, Span};
pub use batch::{Handler, StreamingParser};
pub use events::{EventIter, OwnedEvent};
pub use rescribe_format_api::{Emit, Events, Parse, StreamingParse, StreamingWrite};
pub use writer::Writer;

// ── Trait implementations ───────────────────────────────────────────────────
//
// `Archive` implements the five shared API-mode traits directly — no
// parallel free functions (`zip_fmt::parse(..)`, `zip_fmt::emit(..)`, ...)
// exist alongside them.

impl Parse for Archive {
    fn parse(input: &[u8]) -> (Self, Vec<Diagnostic>) {
        parse::parse(input)
    }
}

impl Emit for Archive {
    fn emit(&self) -> Vec<u8> {
        emit::emit(self)
    }
}

impl Events for Archive {
    // zip-fmt's EventIter always yields OwnedEvent regardless of the
    // input lifetime (decompression can't borrow) — the GAT tolerates an
    // impl that simply doesn't use its own lifetime parameter, per the
    // feasibility spike this crate's migration is based on.
    type Event<'a> = OwnedEvent;
    type EventIter<'a> = EventIter<'a>;

    fn events(input: &[u8]) -> EventIter<'_> {
        EventIter::new(input)
    }
}

impl StreamingParse for Archive {
    type Event = batch::Event;
    type Parser<H: Handler<batch::Event>> = StreamingParser<H>;

    fn streaming_parser<H: Handler<batch::Event>>(handler: H) -> StreamingParser<H> {
        StreamingParser::new(handler)
    }
}

impl StreamingWrite for Archive {
    type Writer<W: std::io::Write> = Writer<W>;

    fn writer<W: std::io::Write>(sink: W) -> Writer<W> {
        Writer::new(sink)
    }
}

#[cfg(test)]
mod smoke {
    use super::*;
    use std::io::Cursor;

    fn sample_zip() -> Vec<u8> {
        let cursor = Cursor::new(Vec::new());
        let mut zip = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("hello.txt", options).unwrap();
        std::io::Write::write_all(&mut zip, b"hello world, hello world, hello world!").unwrap();
        let options2 = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip.start_file("stored.bin", options2).unwrap();
        std::io::Write::write_all(&mut zip, b"\x00\x01\x02raw bytes").unwrap();
        zip.finish().unwrap().into_inner()
    }

    #[test]
    fn smoke_parse() {
        let bytes = sample_zip();
        let (archive, diags) = Archive::parse(&bytes);
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert_eq!(archive.entries.len(), 2);
        assert_eq!(archive.entries[0].name, "hello.txt");
        assert_eq!(
            archive.entries[0].content,
            b"hello world, hello world, hello world!"
        );
        assert_eq!(archive.entries[0].compression, CompressionMethod::Deflate);
        assert_eq!(archive.entries[1].name, "stored.bin");
        assert_eq!(archive.entries[1].compression, CompressionMethod::Store);
        assert_eq!(archive.entries[1].content, b"\x00\x01\x02raw bytes");
    }

    #[test]
    fn smoke_events() {
        let bytes = sample_zip();
        let evts: Vec<_> = Archive::events(&bytes).collect();
        assert_eq!(evts.len(), 2);
        let events::Event::Entry { name, content, .. } = &evts[0] else {
            panic!("expected Entry event");
        };
        assert_eq!(name, "hello.txt");
        assert_eq!(content.as_ref(), b"hello world, hello world, hello world!");
    }

    #[test]
    fn smoke_emit_roundtrip() {
        let bytes = sample_zip();
        let (archive1, _) = Archive::parse(&bytes);
        let emitted = archive1.emit();
        let (archive2, diags2) = Archive::parse(&emitted);
        assert!(diags2.is_empty(), "diagnostics: {diags2:?}");
        assert_eq!(archive1.entries.len(), archive2.entries.len());
        for (a, b) in archive1.entries.iter().zip(archive2.entries.iter()) {
            assert_eq!(a.name, b.name);
            assert_eq!(a.content, b.content);
            assert_eq!(a.compression, b.compression);
        }
    }

    #[test]
    fn smoke_streaming_parser() {
        let bytes = sample_zip();
        let mut starts = Vec::new();
        let mut data: Vec<u8> = Vec::new();
        let mut ends = 0usize;
        {
            let mut p = StreamingParser::new(|ev| match ev {
                batch::Event::StartEntry { name, .. } => starts.push(name),
                batch::Event::Data(chunk) => data.extend_from_slice(&chunk),
                batch::Event::EndEntry { .. } => ends += 1,
                batch::Event::ArchiveComment(_) => {}
            });
            // Feed one byte at a time to exercise chunk-boundary handling.
            for byte in &bytes {
                p.feed(std::slice::from_ref(byte));
            }
            let diags = p.finish();
            assert!(diags.is_empty(), "diagnostics: {diags:?}");
        }
        assert_eq!(
            starts,
            vec!["hello.txt".to_string(), "stored.bin".to_string()]
        );
        assert_eq!(ends, 2);
        assert!(data.starts_with(b"hello world"));
    }

    #[test]
    fn smoke_writer() {
        let mut w = Writer::new(Vec::<u8>::new());
        let entry = Entry {
            name: "a.txt".to_string(),
            is_utf8_name: true,
            compression: CompressionMethod::Deflate,
            content: b"streamed content".to_vec(),
            ..Entry::default()
        };
        w.write_entry(&entry);
        let bytes = w.finish().unwrap();
        let (archive, diags) = Archive::parse(&bytes);
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert_eq!(archive.entries.len(), 1);
        assert_eq!(archive.entries[0].name, "a.txt");
        assert_eq!(archive.entries[0].content, b"streamed content");
    }

    /// The scenario `batch.rs` exists for: a data-descriptor entry (this
    /// crate's own [`Writer`] always produces one, since it targets a
    /// non-`Seek`able sink) round-tripped through `StreamingParser`.
    #[test]
    fn smoke_streaming_parser_handles_data_descriptor_entries() {
        let mut w = Writer::new(Vec::<u8>::new());
        let entry = Entry {
            name: "dd.txt".to_string(),
            is_utf8_name: true,
            compression: CompressionMethod::Deflate,
            content: b"data-descriptor round trip content, repeated repeated repeated".to_vec(),
            ..Entry::default()
        };
        w.write_entry(&entry);
        let bytes = w.finish().unwrap();

        let mut declared_at_start = None;
        let mut content = Vec::new();
        let mut final_size = None;
        {
            let mut p = StreamingParser::new(|ev| match ev {
                batch::Event::StartEntry {
                    declared_uncompressed_size,
                    ..
                } => declared_at_start = Some(declared_uncompressed_size),
                batch::Event::Data(chunk) => content.extend_from_slice(&chunk),
                batch::Event::EndEntry {
                    uncompressed_size, ..
                } => final_size = Some(uncompressed_size),
                batch::Event::ArchiveComment(_) => {}
            });
            p.feed(&bytes);
            let diags = p.finish();
            assert!(diags.is_empty(), "diagnostics: {diags:?}");
        }
        // A data-descriptor entry's StartEntry cannot know the size yet.
        assert_eq!(declared_at_start, Some(0));
        assert_eq!(content, entry.content);
        assert_eq!(final_size, Some(entry.content.len() as u64));
    }
}
