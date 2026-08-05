//! `emit()` — builder writer: `Archive` → complete ZIP bytes.
//!
//! Wraps `zip::ZipWriter<Cursor<Vec<u8>>>`, the same rationale as `parse.rs`
//! (an in-memory `Vec<u8>` sink is trivially `Seek`, and the seekable
//! `ZipWriter` is what lets the central directory be written after all
//! entries, per the format's own layout).
//!
//! # Known fidelity limits of the `zip`-crate-backed writer
//!
//! `zip::write::FileOptions` does not expose setters for the raw
//! general-purpose bit-flags word, `version_made_by`, `internal_attrs`, or
//! the DOS-attribute-byte half of `external_attrs` — only a `unix_mode`-
//! style permissions setter and a `compression_method`/`last_modified_time`
//! pair. This is a real, source-verified limitation of the upstream `zip`
//! crate's public write API (see `docs` note in this crate's `lib.rs`), not
//! a `zip-fmt` design choice: an `Entry`'s `flags`/`version_made_by`/
//! `internal_attrs` fields, and the non-permission bits of
//! `external_attrs`, do not round-trip through `emit()`/[`crate::Writer`]
//! byte-for-byte. `extra_field` bytes and content **do** round-trip
//! (extra-field records are re-added verbatim by ID via
//! `FileOptions::add_extra_data`). The hand-rolled `reader-batch`
//! `StreamingParser` (`batch.rs`) captures the raw flags word exactly on
//! *read*, since it parses local headers itself — but no corresponding
//! exact-flags *write* path exists, since the write side deliberately
//! adapts `zip::write::ZipWriter::new_stream` rather than hand-rolling an
//! encoder (see `writer.rs`).

use std::io::{Cursor, Write};

use crate::ast::{Archive, CompressionMethod, Entry};

/// Serialize an `Archive` to a complete ZIP file.
///
/// `zip::ZipWriter::finish` is fallible even over an in-memory `Cursor`
/// (format-limit errors, e.g. more than `u16::MAX` entries without
/// zip64), but `emit()`'s signature (matching every other `-fmt` crate's
/// builder writer) is infallible. On the rare failure path this returns
/// an empty `Vec` rather than a partial/corrupt archive — a caller that
/// needs to distinguish "empty input" from "write failed" should use
/// [`crate::Writer`] instead, whose `finish()` returns `io::Result`.
pub fn emit(ast: &Archive) -> Vec<u8> {
    let cursor = Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(cursor);
    if !ast.comment.is_empty() {
        zip.set_raw_comment(ast.comment.clone().into());
    }
    for entry in &ast.entries {
        write_entry(&mut zip, entry);
    }
    zip.finish()
        .map(|c| c.into_inner())
        .unwrap_or_else(|_| Vec::new())
}

pub(crate) fn write_entry<W: std::io::Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    entry: &Entry,
) {
    let method = to_zip_method(entry.compression);
    let mtime = zip::DateTime::try_from_msdos(entry.modified.date, entry.modified.time)
        .unwrap_or(zip::DateTime::DEFAULT);

    let mut options = zip::write::FullFileOptions::default()
        .compression_method(method)
        .last_modified_time(mtime)
        .unix_permissions((entry.external_attrs >> 16) & 0o777);

    for (id, data) in split_extra_field_records(&entry.extra_field) {
        // Best-effort: a malformed TLV (declared length runs past the end
        // of the buffer) is simply dropped rather than corrupting the
        // whole write — `split_extra_field_records` already only yields
        // well-formed records.
        let _ = options.add_extra_data(id, data, false);
    }

    if zip.start_file(&entry.name, options).is_err() {
        return;
    }
    let _ = zip.write_all(&entry.content);
}

/// Split raw extra-field bytes (as stored verbatim in [`Entry::extra_field`])
/// into `(header_id, data)` TLV records per the ZIP spec's extra-field
/// layout (`u16 id, u16 size, data[size]`, repeated). Malformed trailing
/// bytes (a record whose declared size runs past the buffer) are dropped —
/// this can only lose fidelity on an already-malformed input, never on a
/// well-formed one.
pub(crate) fn split_extra_field_records(raw: &[u8]) -> Vec<(u16, &[u8])> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + 4 <= raw.len() {
        let id = u16::from_le_bytes([raw[i], raw[i + 1]]);
        let len = u16::from_le_bytes([raw[i + 2], raw[i + 3]]) as usize;
        let data_start = i + 4;
        let data_end = data_start + len;
        if data_end > raw.len() {
            break;
        }
        out.push((id, &raw[data_start..data_end]));
        i = data_end;
    }
    out
}

pub(crate) fn to_zip_method(m: CompressionMethod) -> zip::CompressionMethod {
    #[allow(deprecated)]
    match m {
        CompressionMethod::Store => zip::CompressionMethod::Stored,
        CompressionMethod::Deflate => zip::CompressionMethod::DEFLATE,
        CompressionMethod::Bzip2 => zip::CompressionMethod::BZIP2,
        CompressionMethod::Zstd => zip::CompressionMethod::ZSTD,
        CompressionMethod::Xz => zip::CompressionMethod::XZ,
        CompressionMethod::Lzma => zip::CompressionMethod::LZMA,
        CompressionMethod::Ppmd => zip::CompressionMethod::PPMD,
        CompressionMethod::Aes => zip::CompressionMethod::AES,
        // `zip-fmt` cannot re-encode a method it never decoded (see
        // `parse.rs`'s `decompress_raw`) — the entry's `content` for these
        // is already the raw compressed bytes read straight through, so
        // the only correct re-emission would be `Stored` (write the raw
        // bytes back unmodified) *if* the consumer understands they are
        // not actually the declared method's compressed form. Since
        // `zip::ZipWriter` offers no "write these exact raw bytes under
        // method code N" primitive on the builder/streaming write path,
        // `Other` methods are demoted to `Stored` here — a deliberate,
        // documented lossy fallback, not a silent drop (content bytes
        // themselves are preserved).
        CompressionMethod::Other(_) => zip::CompressionMethod::Stored,
    }
}
