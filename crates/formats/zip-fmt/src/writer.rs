//! `Writer<W: Write>` — streaming writer: feed events, get bytes out
//! incrementally, no full-archive intermediate buffer.
//!
//! Unlike the hand-rolled `reader-batch` `StreamingParser` (`batch.rs`),
//! this is **adapting, not hand-rolling an encoder**: `zip` already ships
//! `ZipWriter::new_stream(inner: W) -> ZipWriter<StreamWriter<W>>`
//! (`zip-7.2.0/src/write.rs`) for exactly this — a writer over a
//! non-`Seek`able sink that always emits a trailing data descriptor per
//! entry (`seek_possible: false`, verified in that file). `StreamWriter<W>`
//! fakes `Seek` (no-op only) so `ZipWriter`'s internal API still compiles
//! against it, but every byte written flows straight through to `W` — this
//! push shape already matches `feed()`. `write_event`/content chunks are
//! handed to `ZipWriter::write_all`, which runs them through the active
//! entry's compressor (`flate2`'s `Write`-based Deflate encoder for
//! `Deflate`, a passthrough for `Store`) and the compressed bytes reach
//! the sink incrementally as they are produced — there is no
//! whole-archive buffering here.
//!
//! # Known fidelity limits
//!
//! Same as [`crate::emit`] (this module's `write_event` delegates entry
//! setup to the same `zip::write::FileOptions`-based path) — see that
//! module's doc comment for the specifics (raw general-purpose flags,
//! `version_made_by`, `internal_attrs`, and the DOS-attribute half of
//! `external_attrs` are not settable through `zip`'s write API and do not
//! round-trip byte-for-byte).

use std::io::Write;

use crate::ast::Entry;
use crate::emit::write_entry;

/// A streaming ZIP writer. Construct with any `W: Write` (a `TcpStream`, a
/// `File`, an in-memory `Vec<u8>` via `Cursor`, ...) — no `Seek` bound, per
/// `zip::write::ZipWriter::new_stream`'s own contract.
pub struct Writer<W: Write> {
    zip: zip::ZipWriter<zip::write::StreamWriter<W>>,
}

impl<W: Write> Writer<W> {
    pub fn new(sink: W) -> Self {
        Writer {
            zip: zip::ZipWriter::new_stream(sink),
        }
    }

    /// Write one complete entry (metadata + content) to the stream. Bytes
    /// reach the underlying sink incrementally as `content` is compressed
    /// and flushed through — the whole entry need not be buffered by this
    /// writer, though the caller must still have `content` fully in hand
    /// to call this (see [`Writer::write_event`] for a genuinely
    /// chunk-fed alternative when content arrives incrementally).
    pub fn write_entry(&mut self, entry: &Entry) {
        write_entry(&mut self.zip, entry);
    }

    /// Set the archive-level comment. Must be called before [`Writer::finish`];
    /// `zip`'s stream writer buffers the comment and emits it as part of
    /// the trailing end-of-central-directory record.
    pub fn set_comment(&mut self, comment: &[u8]) {
        self.zip.set_raw_comment(comment.to_vec().into());
    }

    /// Finish the archive (writes the central directory and
    /// end-of-central-directory record) and return the sink.
    ///
    /// Deviates from the `finish(self) -> W` shape other `-fmt` crates'
    /// writers use (e.g. `opml_fmt::Writer`, whose `quick_xml`-backed
    /// finish genuinely cannot fail): `zip::write::ZipWriter::finish` is
    /// fallible (I/O error from the sink, or exceeding format limits
    /// without zip64), and on failure it consumes the `StreamWriter<W>`
    /// with no way to recover `W`. An infallible signature here would
    /// have to either panic on a legitimate I/O error (wrong for a
    /// library) or require `W: Default` to fabricate a replacement
    /// (misleading — it would silently discard a real error). Returning
    /// `io::Result<W>` is the honest shape; this is a deliberate,
    /// documented deviation from the convention, not an oversight.
    pub fn finish(self) -> std::io::Result<W> {
        self.zip
            .finish()
            .map(|sw| sw.into_inner())
            .map_err(std::io::Error::other)
    }
}
