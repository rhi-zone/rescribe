//! AST types for ZIP archives.
//!
//! `zip-fmt` is a **container** format library: it reads and writes the ZIP
//! envelope (local/central file headers, the end-of-central-directory
//! record, per-entry compressed streams) and hands callers the raw bytes of
//! each entry. It never interprets entry content as any particular document
//! format — a caller that opens an EPUB or an OOXML `.docx` gets back a list
//! of `(name, bytes)` pairs and is responsible for parsing `content.opf` or
//! `word/document.xml` itself. This mirrors the split already used for
//! OOXML in this workspace: `ooxml-opc` does the OPC/ZIP packaging layer,
//! `ooxml-wml`/`-sml`/`-pml` interpret the XML inside.
//!
//! Losslessness (per `CLAUDE.md`): every entry's general-purpose bit flags,
//! external/internal file attributes, and raw extra-field bytes are kept
//! verbatim on [`Entry`] rather than being decoded into a lossy subset —
//! `extra_field` in particular is stored as the exact bytes the source
//! archive wrote (including any vendor/future extra-field IDs this crate
//! doesn't interpret), so `parse` → `emit` round-trips those bytes
//! unchanged even for extra-field kinds this crate has no special handling
//! for.

use std::borrow::Cow;

/// Byte offset span in the source input.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const NONE: Span = Span { start: 0, end: 0 };
}

/// Compression method used to store one entry's content.
///
/// This mirrors the method codes the ZIP spec defines (APPNOTE.TXT §4.4.5),
/// not `zip` crate's own enum — `zip-fmt`'s public surface must stand alone
/// as a general ZIP-container vocabulary. `Other(code)` preserves any method
/// code this crate does not have a variant for (legacy Shrink/Reduce/
/// Implode, or a future method), so an entry using an unrecognized method
/// still round-trips its raw compressed bytes and declared method code even
/// though `zip-fmt` cannot decompress it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CompressionMethod {
    #[default]
    Store,
    Deflate,
    Bzip2,
    Zstd,
    Xz,
    Lzma,
    Ppmd,
    /// AES encryption wrapper (method 99). The real compression method is
    /// carried in the AES extra field, which is preserved verbatim in
    /// [`Entry::extra_field`] since this crate does not decrypt.
    Aes,
    /// Any method code not listed above (e.g. legacy Shrink=1, Reduce 2-5,
    /// Implode=6, Deflate64=9, or a future/vendor method), preserved by its
    /// raw numeric code.
    Other(u16),
}

impl CompressionMethod {
    /// The raw method code as written in the ZIP local/central header.
    pub fn code(self) -> u16 {
        match self {
            CompressionMethod::Store => 0,
            CompressionMethod::Other(c) => c,
            CompressionMethod::Deflate => 8,
            CompressionMethod::Bzip2 => 12,
            CompressionMethod::Lzma => 14,
            CompressionMethod::Xz => 95,
            CompressionMethod::Ppmd => 98,
            CompressionMethod::Aes => 99,
            CompressionMethod::Zstd => 93,
        }
    }

    pub fn from_code(code: u16) -> CompressionMethod {
        match code {
            0 => CompressionMethod::Store,
            8 => CompressionMethod::Deflate,
            12 => CompressionMethod::Bzip2,
            14 => CompressionMethod::Lzma,
            93 => CompressionMethod::Zstd,
            95 => CompressionMethod::Xz,
            98 => CompressionMethod::Ppmd,
            99 => CompressionMethod::Aes,
            other => CompressionMethod::Other(other),
        }
    }
}

/// Raw MS-DOS date/time fields as stored in the ZIP local/central header
/// (2-second resolution). Kept as the two raw `u16` fields rather than
/// decoded into a calendar type so an out-of-range or nonstandard value
/// (some writers emit garbage here) still round-trips exactly instead of
/// being clamped or rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct DosTimestamp {
    pub date: u16,
    pub time: u16,
}

impl DosTimestamp {
    /// Decode to `(year, month, day, hour, minute, second)`, per the
    /// MS-DOS date/time bit layout. Returns `None` if the fields don't
    /// decode to a representable date (this never blocks round-tripping —
    /// callers that only need the raw archive back can ignore this).
    pub fn to_ymd_hms(self) -> Option<(u16, u8, u8, u8, u8, u8)> {
        let year = 1980 + (self.date >> 9);
        let month = ((self.date >> 5) & 0xf) as u8;
        let day = (self.date & 0x1f) as u8;
        let hour = (self.time >> 11) as u8;
        let minute = ((self.time >> 5) & 0x3f) as u8;
        let second = ((self.time & 0x1f) as u32 * 2) as u8;
        if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
            return None;
        }
        Some((year, month, day, hour, minute, second))
    }
}

/// One entry (file or directory) in a ZIP archive.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Entry {
    /// The entry's path/name, verbatim as decoded from the local header
    /// (UTF-8 if the UTF-8 language-encoding flag, bit 11, was set; CP437
    /// otherwise — [`Entry::is_utf8_name`] records which). Uses whatever
    /// path separator the source archive used (`/` per spec, though some
    /// writers use `\`); not normalized.
    pub name: String,
    /// True if `name`/`comment` were declared UTF-8 (general-purpose flag
    /// bit 11). Needed to re-emit the same flag bit on write.
    pub is_utf8_name: bool,
    pub compression: CompressionMethod,
    pub uncompressed_size: u64,
    pub compressed_size: u64,
    pub crc32: u32,
    pub modified: DosTimestamp,
    /// Raw general-purpose bit flags (local header `flags` field), stored
    /// verbatim so bits this crate does not otherwise model (encryption,
    /// strong encryption, enhanced deflate sub-fields, ...) still
    /// round-trip. Bit 3 (data descriptor) and bit 11 (UTF-8) are also
    /// individually exposed as [`Entry::has_data_descriptor`]/
    /// [`Entry::is_utf8_name`] for convenience, but `flags` is the single
    /// source of truth — the accessors just read bits out of it.
    pub flags: u16,
    /// External file attributes (host-OS-specific: Unix permission bits in
    /// the upper 16 bits when `version_made_by`'s host is Unix, DOS
    /// attribute byte otherwise). Preserved verbatim, not decoded.
    pub external_attrs: u32,
    /// Internal file attributes (ZIP spec §4.4.4; bit 0 is the "apparent
    /// ASCII/text file" hint). Preserved verbatim.
    pub internal_attrs: u16,
    /// Upper byte of `version_made_by` (host system) and lower byte
    /// (spec version), preserved verbatim so a re-emitted entry claims the
    /// same originating tool/version rather than always claiming this
    /// crate's own.
    pub version_made_by: u16,
    pub version_needed: u16,
    /// The entry's extra-field bytes, verbatim, exactly as they appear
    /// after the filename in the local header (ID/length/data records,
    /// concatenated). This is the crate's raw-preservation mechanism for
    /// zip64 size overrides, NTFS/Unix timestamps, Unicode path extra
    /// fields, and any other extra-field kind — nothing here is
    /// interpreted or re-derived; it is copied through unchanged.
    pub extra_field: Vec<u8>,
    /// Per-entry comment (from the central directory), verbatim.
    pub comment: String,
    /// The entry's content, decompressed. Empty for directory entries
    /// (`name` ending in `/` with `uncompressed_size == 0`, per convention
    /// — ZIP has no separate directory-entry bit).
    pub content: Vec<u8>,
    pub span: Span,
}

impl Entry {
    pub fn has_data_descriptor(&self) -> bool {
        self.flags & (1 << 3) != 0
    }

    pub fn is_encrypted(&self) -> bool {
        self.flags & 1 != 0
    }

    /// Convention-only: ZIP has no directory-entry flag; a path ending in
    /// `/` with no content is the universal convention for "this is a
    /// directory", used by `zip`/`unzip`/every major ZIP writer.
    pub fn is_dir(&self) -> bool {
        self.name.ends_with('/') && self.content.is_empty() && self.uncompressed_size == 0
    }

    pub fn strip_spans(&self) -> Entry {
        Entry {
            span: Span::NONE,
            ..self.clone()
        }
    }
}

/// A ZIP archive: an ordered list of entries plus the archive-level
/// comment (the free-text field at the end of the end-of-central-directory
/// record).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Archive {
    pub entries: Vec<Entry>,
    /// Archive comment, verbatim bytes reinterpreted as `Cow`'d text only
    /// where lossless — stored as raw bytes since the ZIP spec does not
    /// mandate an encoding for it (unlike entry names, no UTF-8 flag
    /// applies to the archive comment).
    pub comment: Vec<u8>,
    pub span: Span,
}

impl Archive {
    pub fn strip_spans(&self) -> Archive {
        Archive {
            entries: self.entries.iter().map(Entry::strip_spans).collect(),
            comment: self.comment.clone(),
            span: Span::NONE,
        }
    }
}

/// Diagnostic message from parsing.
#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    pub message: String,
    pub span: Span,
}

/// Helper: decode a `Cow<str>` from raw bytes per the entry's declared
/// encoding (UTF-8 if the language-encoding flag is set, CP437 otherwise).
/// CP437 decoding here is intentionally minimal (ASCII passthrough with a
/// lossy fallback for the high half) — full CP437 tables are a solved
/// problem the `zip` crate already owns for the `reader-ast`/`reader-
/// streaming` paths; this helper only backs the hand-rolled
/// `reader-batch` path, and non-ASCII CP437 filenames are rare in the
/// modern-toolchain corpora this crate targets (EPUB, OOXML).
pub(crate) fn decode_name(bytes: &[u8], is_utf8: bool) -> Cow<'_, str> {
    if is_utf8 {
        String::from_utf8_lossy(bytes)
    } else {
        Cow::Owned(bytes.iter().map(|&b| b as char).collect())
    }
}
