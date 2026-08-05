//! `StreamingParser<H: Handler>` — genuinely incremental, chunk-fed ZIP
//! reader. This is the crate's real hand-rolled contribution (see
//! `lib.rs`): `reader-ast`/`reader-streaming`/`writer-builder`/
//! `writer-streaming` all wrap the `zip` crate over an in-memory
//! `Seek`able buffer, which is the right tool for those cases (see each
//! module's doc comment) — but `zip::read::stream::ZipStreamReader` is
//! blocking/pull-based and, per `ZipFileData::from_local_block`
//! (`zip-7.2.0/src/types.rs`), **rejects any entry using a trailing data
//! descriptor** (general-purpose flag bit 3) outright:
//!
//! ```text
//! let using_data_descriptor: bool = flags & (1 << 3) == 1 << 3;
//! if using_data_descriptor {
//!     return Err(ZipError::UnsupportedArchive(
//!         "The file length is not available in the local header",
//!     ));
//! }
//! ```
//!
//! Data-descriptor entries are exactly what a non-seekable writer (e.g.
//! this crate's own [`crate::Writer`], which wraps
//! `zip::write::ZipWriter::new_stream`) produces for *every* entry, so a
//! reader that cannot handle them cannot round-trip this crate's own
//! streaming writer output. This module parses local headers directly and
//! drives decompression itself, so it has no such restriction.
//!
//! # Entry-metadata timing: the data-descriptor problem
//!
//! For a normal entry (bit 3 clear), the local header carries the final
//! CRC-32 and both sizes *before* any content bytes — a consumer can
//! announce complete metadata the moment the header is parsed.
//!
//! For a data-descriptor entry (bit 3 set), the local header's CRC-32 and
//! size fields are zeroed placeholders — the real values are only known
//! from the 12-or-16-byte descriptor written *after* the compressed data.
//! Naively scanning for the descriptor's `0x08074b50` signature to find
//! that boundary is unsound: that exact byte sequence can legally occur
//! *inside* compressed data. This module instead drives decompression to
//! its logical end (`flate2::Decompress` reports [`flate2::Status::StreamEnd`]
//! for Deflate), which tells us unambiguously where the compressed stream
//! stops — the descriptor starts at the very next byte. See
//! [`EntryState::content`] and `read_descriptor` below.
//!
//! **API consequence (the design choice this crate settled on for
//! consistency — see `CLAUDE.md`'s "no guessing" rule, flagged in this
//! crate's top-level report as a documented call, not an unquestionable
//! one):** rather than giving data-descriptor entries a different event
//! shape than normal entries, **every** entry uses the same three-event
//! shape — [`Event::StartEntry`] (whatever is known before content:
//! name, compression, flags, mtime, extra field, and the header-declared
//! sizes/CRC, which are `0` placeholders for a data-descriptor entry),
//! zero or more [`Event::Data`] chunks, and [`Event::EndEntry`] (always
//! the authoritative final size/CRC, regardless of where they came from).
//! A handler that only trusts `EndEntry` is correct for every entry
//! unconditionally; one that wants to act early on `StartEntry`'s sizes
//! must be prepared for a data-descriptor entry to report `0`/`0`/`0`
//! there. The alternative considered — two different event variants
//! (`StartEntryKnownSize` / `StartEntryDeferredSize`) — was rejected as
//! adding a second decision axis (data-descriptor or not) to every
//! consumer's match arm for no benefit, since `EndEntry` must exist
//! either way (a data-descriptor entry has no other point at which the
//! final CRC becomes available even to announce as an update).
//!
//! # Memory profile
//!
//! O(largest token + nesting depth) for every entry **except** one
//! documented, narrow case: a `Stored` (uncompressed) entry that also uses
//! a data descriptor. `Store` has no logical end-of-stream signal the way
//! Deflate's inflate state machine does — the only way to find the
//! boundary is to locate a signature+CRC-validated candidate. This module
//! does so by buffering that one entry's content (bounded by that entry's
//! own size, not the archive's) and validating the CRC-32 of each
//! candidate cut point against the descriptor's declared value before
//! committing to it — a spurious signature match inside genuine content
//! would additionally need to produce a matching CRC-32 by coincidence
//! (≈1-in-4-billion per candidate). This is a real, scoped exception to
//! the O(bounded) memory claim, not a silent one: `zip-fmt`'s own
//! [`crate::Writer`] defaults to Deflate, so this path is not the common
//! case for round-tripping this crate's own output, but a `Stored`+
//! data-descriptor archive from some other producer will hit it.

use crate::ast::{CompressionMethod, DosTimestamp, Span};

/// One streaming ZIP event. See the module docs for the entry-metadata
/// timing contract shared by every entry.
///
/// Owned rather than `Cow`-borrowed (unlike `events.rs`'s pull-iterator
/// `Event<'a>`): a `Handler`-based blanket impl over a lifetime-generic
/// event type (`impl<F: FnMut(Event<'_>)> Handler for F`) runs into a
/// well-known Rust HRTB closure-inference limitation — the compiler
/// cannot always unify a plain closure literal against a higher-ranked
/// `for<'a> FnMut(Event<'a>)` bound without an explicit type annotation on
/// every call site, which would be a poor ergonomic default for this
/// crate's primary hand-rolled API. `opml-fmt`'s `batch.rs` hits the same
/// constraint and resolves it the same way (`OwnedEvent`-only `Handler`).
/// This does cost an allocation per `Data` chunk that could, in the
/// `Store` (non-data-descriptor) case, have been a genuine zero-copy
/// borrow of the internal buffer — a real, small, documented cost, traded
/// for a `Handler` trait ergonomic enough to use with a plain closure.
#[derive(Debug, PartialEq)]
pub enum Event {
    /// Archive comment, if the archive is fed with a trailing comment and
    /// [`StreamingParser::finish`] is reached with one buffered. Rare in
    /// practice (most ZIP writers, including this crate's own, omit it);
    /// `zip-fmt`'s `StreamingParser` does not scan for it mid-stream since
    /// nothing before the end-of-central-directory record identifies it.
    ArchiveComment(Vec<u8>),
    /// A new entry's local header has been fully parsed. `declared_*`
    /// fields are the header's own values — `0` for all three on a
    /// data-descriptor entry (see module docs); trust [`Event::EndEntry`]
    /// for the authoritative final values in every case.
    StartEntry {
        name: String,
        is_utf8_name: bool,
        compression: CompressionMethod,
        flags: u16,
        /// The local header's `version needed to extract` field, verbatim.
        version_needed: u16,
        modified: DosTimestamp,
        extra_field: Vec<u8>,
        declared_uncompressed_size: u64,
        declared_compressed_size: u64,
        declared_crc32: u32,
    },
    /// One chunk of decompressed content for the entry currently open.
    /// Chunk boundaries are an implementation detail (driven by `feed()`
    /// call boundaries and the decompressor's internal buffer), not
    /// meaningful framing — a handler that wants the whole entry
    /// concatenates every `Data` chunk between `StartEntry` and
    /// `EndEntry`.
    Data(Vec<u8>),
    /// The entry currently open has ended. Always carries the
    /// authoritative final metadata regardless of entry kind.
    EndEntry {
        uncompressed_size: u64,
        compressed_size: u64,
        crc32: u32,
    },
}

/// Diagnostic emitted by the streaming parser. Re-exported from
/// `rescribe-format-api` — shared with `crate::ast::Diagnostic` (they used
/// to be two distinct locally-declared types with the same shape; now
/// there is exactly one `Diagnostic` type in the crate).
pub use rescribe_format_api::Diagnostic;

/// Callback sink for [`StreamingParser`] — the shared
/// [`rescribe_format_api::Handler`], not a locally declared trait. See
/// that crate's docs for why every format crate's `StreamingParser<H>`
/// must bound `H` by one shared trait for a common `StreamingParse` trait
/// to exist at all. `handle()` is stack-scoped: the borrowed `Event<'_>`
/// (its `Data` chunk in particular) is only valid for the duration of the
/// call, allowing the parser's internal buffer to compact afterward.
pub use rescribe_format_api::Handler;

const LOCAL_FILE_HEADER_SIG: u32 = 0x0403_4b50;
const DATA_DESCRIPTOR_SIG: u32 = 0x0807_4b50;
const CENTRAL_DIRECTORY_SIG: u32 = 0x0201_4b50;
/// Any of these mark "no more local file entries" once seen where a local
/// file header signature would otherwise be expected.
const END_OF_ENTRIES_SIGS: [u32; 3] = [CENTRAL_DIRECTORY_SIG, 0x0605_4b50, 0x0606_4b50];

#[derive(Debug)]
struct LocalHeaderFixed {
    version_needed: u16,
    flags: u16,
    method: u16,
    mod_time: u16,
    mod_date: u16,
    crc32: u32,
    compressed_size: u32,
    uncompressed_size: u32,
    name_len: u16,
    extra_len: u16,
}

impl LocalHeaderFixed {
    const LEN: usize = 26;

    fn parse(b: &[u8]) -> LocalHeaderFixed {
        let u16le = |o: usize| u16::from_le_bytes([b[o], b[o + 1]]);
        let u32le = |o: usize| u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
        LocalHeaderFixed {
            version_needed: u16le(0),
            flags: u16le(2),
            method: u16le(4),
            mod_time: u16le(6),
            mod_date: u16le(8),
            crc32: u32le(10),
            compressed_size: u32le(14),
            uncompressed_size: u32le(18),
            name_len: u16le(22),
            extra_len: u16le(24),
        }
    }
}

/// Scan an entry's raw extra-field bytes for a zip64 (`0x0001`) record and
/// return `(uncompressed_size, compressed_size)` overrides in the order
/// the zip64 record defines them (only present for fields that were the
/// `0xFFFFFFFF` sentinel in the fixed header — per spec, only those
/// sentinel fields' real values appear in the zip64 record, in
/// uncompressed-then-compressed order).
fn zip64_overrides(
    extra: &[u8],
    need_uncompressed: bool,
    need_compressed: bool,
) -> (Option<u64>, Option<u64>) {
    let mut i = 0usize;
    while i + 4 <= extra.len() {
        let id = u16::from_le_bytes([extra[i], extra[i + 1]]);
        let len = u16::from_le_bytes([extra[i + 2], extra[i + 3]]) as usize;
        let start = i + 4;
        let end = start + len;
        if end > extra.len() {
            break;
        }
        if id == 0x0001 {
            let mut off = start;
            let mut uncompressed = None;
            let mut compressed = None;
            if need_uncompressed && off + 8 <= end {
                uncompressed = Some(u64::from_le_bytes(extra[off..off + 8].try_into().unwrap()));
                off += 8;
            }
            if need_compressed && off + 8 <= end {
                compressed = Some(u64::from_le_bytes(extra[off..off + 8].try_into().unwrap()));
            }
            return (uncompressed, compressed);
        }
        i = end;
    }
    (None, None)
}

enum ContentMode {
    Store,
    Deflate(Box<flate2::Decompress>),
}

/// State while streaming one entry's compressed data.
struct EntryContent {
    mode: ContentMode,
    /// `Some(n)` = exactly `n` more compressed bytes remain (normal
    /// entry, size known from the header). `None` = data-descriptor
    /// entry; end is detected by decompressor end-of-stream (Deflate) or
    /// by the buffered signature+CRC scan (Store — see module docs).
    remaining_compressed: Option<u64>,
    uses_zip64_descriptor: bool,
    crc: flate2::Crc,
    /// Only populated for the `Store` + data-descriptor combination (the
    /// one documented O(entry size) exception — see module docs).
    store_dd_accum: Option<Vec<u8>>,
    /// Header-declared final sizes/CRC for a *non*-data-descriptor entry
    /// (meaningless — never read — for a data-descriptor entry, whose
    /// final values instead come from `drive_descriptor`).
    declared_uncompressed_size: u64,
    declared_compressed_size: u64,
    declared_crc32: u32,
}

enum Phase {
    /// Waiting for a 4-byte signature: either a new local file header, or
    /// the central directory (meaning entries are done).
    ScanSignature,
    /// Have a confirmed local-file-header signature; need the 26
    /// remaining fixed-size header bytes.
    LocalFixed,
    /// Fixed header parsed; need `name_len` bytes.
    Name(LocalHeaderFixed),
    /// Name parsed; need `extra_len` bytes.
    Extra(LocalHeaderFixed, String),
    /// Streaming one entry's compressed content.
    Content(EntryContent),
    /// Reading a trailing data descriptor (data-descriptor entries only).
    /// `bool` = whether the 4-byte signature was already consumed.
    Descriptor { zip64: bool, sig_consumed: bool },
    /// No more entries; ignore all further input (central directory /
    /// EOCD onward — this crate does not need it, since every entry's
    /// metadata was already captured from its local header/descriptor).
    Done,
}

/// Genuinely incremental, chunk-fed ZIP reader. See the module docs for
/// the full rationale and the entry-metadata timing contract.
pub struct StreamingParser<H: Handler<Event>> {
    handler: H,
    phase: Phase,
    buf: Vec<u8>,
    diagnostics: Vec<Diagnostic>,
    bytes_seen: usize,
}

impl<H: Handler<Event>> StreamingParser<H> {
    pub fn new(handler: H) -> Self {
        StreamingParser {
            handler,
            phase: Phase::ScanSignature,
            buf: Vec::new(),
            diagnostics: Vec::new(),
            bytes_seen: 0,
        }
    }

    /// Feed the next chunk of archive bytes. May be called with chunks of
    /// any size, including 1 byte, and any number of times.
    pub fn feed(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);
        self.bytes_seen += chunk.len();
        self.drive();
    }

    /// Signal end of input. Returns accumulated diagnostics.
    pub fn finish(mut self) -> Vec<Diagnostic> {
        if !matches!(self.phase, Phase::Done | Phase::ScanSignature) {
            self.diagnostics.push(Diagnostic {
                message: "input ended mid-entry (truncated archive)".to_string(),
                span: Span::NONE,
                severity: rescribe_format_api::Severity::Warning,
                code: "",
            });
        }
        self.diagnostics
    }

    /// Drain as much of `self.buf` as currently possible given the
    /// buffered bytes, advancing `self.phase` as entries complete.
    fn drive(&mut self) {
        loop {
            match &mut self.phase {
                Phase::Done => {
                    self.buf.clear();
                    return;
                }
                Phase::ScanSignature => {
                    if self.buf.len() < 4 {
                        return;
                    }
                    let sig =
                        u32::from_le_bytes([self.buf[0], self.buf[1], self.buf[2], self.buf[3]]);
                    if sig == LOCAL_FILE_HEADER_SIG {
                        self.buf.drain(0..4);
                        self.phase = Phase::LocalFixed;
                    } else if END_OF_ENTRIES_SIGS.contains(&sig) {
                        self.phase = Phase::Done;
                    } else {
                        self.diagnostics.push(Diagnostic {
                            message: format!(
                                "unrecognized signature 0x{sig:08x}; stopping entry scan"
                            ),
                            span: Span::NONE,
                            severity: rescribe_format_api::Severity::Warning,
                            code: "",
                        });
                        self.phase = Phase::Done;
                    }
                }
                Phase::LocalFixed => {
                    if self.buf.len() < LocalHeaderFixed::LEN {
                        return;
                    }
                    let fixed = LocalHeaderFixed::parse(&self.buf[..LocalHeaderFixed::LEN]);
                    self.buf.drain(0..LocalHeaderFixed::LEN);
                    self.phase = Phase::Name(fixed);
                }
                Phase::Name(_) => {
                    let Phase::Name(fixed) = std::mem::replace(&mut self.phase, Phase::Done) else {
                        unreachable!()
                    };
                    let need = fixed.name_len as usize;
                    if self.buf.len() < need {
                        self.phase = Phase::Name(fixed);
                        return;
                    }
                    let is_utf8 = fixed.flags & (1 << 11) != 0;
                    let name = crate::ast::decode_name(&self.buf[..need], is_utf8).into_owned();
                    self.buf.drain(0..need);
                    self.phase = Phase::Extra(fixed, name);
                }
                Phase::Extra(..) => {
                    let Phase::Extra(fixed, name) = std::mem::replace(&mut self.phase, Phase::Done)
                    else {
                        unreachable!()
                    };
                    let need = fixed.extra_len as usize;
                    if self.buf.len() < need {
                        self.phase = Phase::Extra(fixed, name);
                        return;
                    }
                    let extra: Vec<u8> = self.buf[..need].to_vec();
                    self.buf.drain(0..need);
                    self.start_entry(fixed, name, extra);
                }
                Phase::Content(_) => {
                    if !self.drive_content() {
                        return;
                    }
                }
                Phase::Descriptor { .. } => {
                    if !self.drive_descriptor() {
                        return;
                    }
                }
            }
        }
    }

    fn start_entry(&mut self, fixed: LocalHeaderFixed, name: String, extra: Vec<u8>) {
        let is_utf8_name = fixed.flags & (1 << 11) != 0;
        let has_dd = fixed.flags & (1 << 3) != 0;
        let compression = CompressionMethod::from_code(fixed.method);

        let uncompressed_sentinel = fixed.uncompressed_size == 0xFFFF_FFFF;
        let compressed_sentinel = fixed.compressed_size == 0xFFFF_FFFF;
        let (z64_uncompressed, z64_compressed) =
            zip64_overrides(&extra, uncompressed_sentinel, compressed_sentinel);
        let uncompressed_size = z64_uncompressed.unwrap_or(fixed.uncompressed_size as u64);
        let compressed_size = z64_compressed.unwrap_or(fixed.compressed_size as u64);
        let uses_zip64 = uncompressed_sentinel || compressed_sentinel;

        self.handler.handle(Event::StartEntry {
            name,
            is_utf8_name,
            compression,
            flags: fixed.flags,
            version_needed: fixed.version_needed,
            modified: DosTimestamp {
                date: fixed.mod_date,
                time: fixed.mod_time,
            },
            extra_field: extra,
            declared_uncompressed_size: if has_dd { 0 } else { uncompressed_size },
            declared_compressed_size: if has_dd { 0 } else { compressed_size },
            declared_crc32: if has_dd { 0 } else { fixed.crc32 },
        });

        let mode = match compression {
            CompressionMethod::Store => ContentMode::Store,
            CompressionMethod::Deflate => {
                ContentMode::Deflate(Box::new(flate2::Decompress::new(false)))
            }
            other => {
                self.diagnostics.push(Diagnostic {
                    message: format!(
                        "entry uses compression method {other:?}, which zip-fmt's StreamingParser \
                         does not decode; treating content as opaque Stored bytes"
                    ),
                    span: Span::NONE,
                    severity: rescribe_format_api::Severity::Warning,
                    code: "",
                });
                ContentMode::Store
            }
        };

        let store_dd_accum = if has_dd && matches!(mode, ContentMode::Store) {
            Some(Vec::new())
        } else {
            None
        };

        self.phase = Phase::Content(EntryContent {
            mode,
            remaining_compressed: if has_dd { None } else { Some(compressed_size) },
            uses_zip64_descriptor: uses_zip64,
            crc: flate2::Crc::new(),
            store_dd_accum,
            declared_uncompressed_size: uncompressed_size,
            declared_compressed_size: compressed_size,
            declared_crc32: fixed.crc32,
        });
    }

    /// Advance `Phase::Content` using currently-buffered bytes. Returns
    /// `false` when it needs more input to make further progress.
    fn drive_content(&mut self) -> bool {
        let Phase::Content(entry) = &mut self.phase else {
            unreachable!()
        };

        match entry.remaining_compressed {
            Some(0) => {
                self.finish_entry_known();
                true
            }
            Some(remaining) => {
                if self.buf.is_empty() {
                    return false;
                }
                let take = (remaining as usize).min(self.buf.len());
                let chunk: Vec<u8> = self.buf.drain(0..take).collect();
                let decompressed = decompress_chunk(&mut entry.mode, &chunk, &mut self.diagnostics);
                entry.crc.update(&decompressed);
                if !decompressed.is_empty() {
                    self.handler.handle(Event::Data(decompressed));
                }
                entry.remaining_compressed = Some(remaining - take as u64);
                true
            }
            None => self.drive_data_descriptor_content(),
        }
    }

    /// Content whose length is only known via the trailing data
    /// descriptor (general-purpose flag bit 3).
    fn drive_data_descriptor_content(&mut self) -> bool {
        let Phase::Content(entry) = &mut self.phase else {
            unreachable!()
        };
        match &mut entry.mode {
            ContentMode::Deflate(inflate) => {
                if self.buf.is_empty() {
                    return false;
                }
                let mut out = vec![0u8; (self.buf.len() * 4).max(4096)];
                let before_in = inflate.total_in();
                let before_out = inflate.total_out();
                let status = inflate.decompress(&self.buf, &mut out, flate2::FlushDecompress::None);
                let status = match status {
                    Ok(s) => s,
                    Err(e) => {
                        self.diagnostics.push(Diagnostic {
                            message: format!("deflate decode error in data-descriptor entry: {e}"),
                            span: Span::NONE,
                            severity: rescribe_format_api::Severity::Warning,
                            code: "",
                        });
                        flate2::Status::StreamEnd
                    }
                };
                let consumed_in = (inflate.total_in() - before_in) as usize;
                let produced_out = (inflate.total_out() - before_out) as usize;
                if produced_out > 0 {
                    let chunk = out[..produced_out].to_vec();
                    entry.crc.update(&chunk);
                    self.handler.handle(Event::Data(chunk));
                }
                self.buf.drain(0..consumed_in);
                if status == flate2::Status::StreamEnd {
                    self.phase = Phase::Descriptor {
                        zip64: matches!(&self.phase, Phase::Content(e) if e.uses_zip64_descriptor),
                        sig_consumed: false,
                    };
                    true
                } else {
                    consumed_in > 0
                }
            }
            ContentMode::Store => {
                // See module docs: Store + data descriptor has no
                // logical end-of-stream signal, so this entry's content
                // is buffered (bounded by this entry's own size) while
                // scanning for a CRC-validated descriptor boundary.
                let accum = entry.store_dd_accum.get_or_insert_with(Vec::new);
                accum.append(&mut self.buf);
                if let Some((cut, zip64)) = find_validated_store_descriptor(accum) {
                    let content = accum[..cut].to_vec();
                    // Skip past the 4-byte signature only; the remaining
                    // CRC-32/size fields are left in `self.buf` for
                    // `drive_descriptor` to parse normally.
                    self.buf = accum[cut + 4..].to_vec();
                    entry.store_dd_accum = None;
                    if !content.is_empty() {
                        self.handler.handle(Event::Data(content));
                    }
                    self.phase = Phase::Descriptor {
                        zip64,
                        sig_consumed: true,
                    };
                    true
                } else {
                    false
                }
            }
        }
    }

    /// A non-data-descriptor entry's declared compressed size has been
    /// fully consumed. The header's own sizes/CRC are authoritative (this
    /// entry never had a data descriptor to override them), so `EndEntry`
    /// reiterates them — per the module docs, `EndEntry` is
    /// unconditionally the source of truth even when, as here, it agrees
    /// with what `StartEntry` already announced. The recomputed content
    /// CRC-32 is not currently cross-checked against the header's
    /// declared value (a mismatch would indicate a corrupt archive, not a
    /// data-descriptor case); doing so is a reasonable future validation
    /// addition, not required for correct event emission.
    fn finish_entry_known(&mut self) {
        let Phase::Content(entry) = std::mem::replace(&mut self.phase, Phase::ScanSignature) else {
            unreachable!()
        };
        self.handler.handle(Event::EndEntry {
            uncompressed_size: entry.declared_uncompressed_size,
            compressed_size: entry.declared_compressed_size,
            crc32: entry.declared_crc32,
        });
    }
}

/// After the compressed stream for a data-descriptor entry has ended,
/// read the descriptor itself (optional 4-byte signature, then CRC-32,
/// then compressed/uncompressed sizes as 4-byte or 8-byte fields
/// depending on whether the entry used zip64).
impl<H: Handler<Event>> StreamingParser<H> {
    fn drive_descriptor(&mut self) -> bool {
        let Phase::Descriptor {
            zip64,
            sig_consumed,
        } = &mut self.phase
        else {
            unreachable!()
        };
        if !*sig_consumed {
            if self.buf.len() < 4 {
                return false;
            }
            let maybe_sig =
                u32::from_le_bytes([self.buf[0], self.buf[1], self.buf[2], self.buf[3]]);
            if maybe_sig == DATA_DESCRIPTOR_SIG {
                self.buf.drain(0..4);
            }
            *sig_consumed = true;
        }
        let size_field_len = if *zip64 { 8 } else { 4 };
        let need = 4 + size_field_len * 2; // crc32 + compressed + uncompressed
        if self.buf.len() < need {
            return false;
        }
        let crc32 = u32::from_le_bytes([self.buf[0], self.buf[1], self.buf[2], self.buf[3]]);
        let (compressed_size, uncompressed_size) = if *zip64 {
            (
                u64::from_le_bytes(self.buf[4..12].try_into().unwrap()),
                u64::from_le_bytes(self.buf[12..20].try_into().unwrap()),
            )
        } else {
            (
                u32::from_le_bytes(self.buf[4..8].try_into().unwrap()) as u64,
                u32::from_le_bytes(self.buf[8..12].try_into().unwrap()) as u64,
            )
        };
        self.buf.drain(0..need);
        self.handler.handle(Event::EndEntry {
            uncompressed_size,
            compressed_size,
            crc32,
        });
        self.phase = Phase::ScanSignature;
        true
    }
}

/// Decompress one chunk of a *known-length* entry's compressed bytes
/// (`entry.remaining_compressed == Some(_)`). Loops calling
/// `Decompress::decompress` until the whole `compressed` slice has been
/// consumed — a single call is not guaranteed to consume all input if the
/// output buffer fills first (highly-repetitive content can expand by a
/// large factor under Deflate), so this cannot assume one call suffices.
fn decompress_chunk(
    mode: &mut ContentMode,
    compressed: &[u8],
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<u8> {
    match mode {
        ContentMode::Store => compressed.to_vec(),
        ContentMode::Deflate(inflate) => {
            let mut result = Vec::new();
            let mut input = compressed;
            let mut out = vec![0u8; 64 * 1024];
            loop {
                let before_in = inflate.total_in();
                let before_out = inflate.total_out();
                let status =
                    match inflate.decompress(input, &mut out, flate2::FlushDecompress::None) {
                        Ok(s) => s,
                        Err(e) => {
                            diagnostics.push(Diagnostic {
                                message: format!("deflate decode error: {e}"),
                                span: Span::NONE,
                                severity: rescribe_format_api::Severity::Warning,
                                code: "",
                            });
                            break;
                        }
                    };
                let consumed_in = (inflate.total_in() - before_in) as usize;
                let produced_out = (inflate.total_out() - before_out) as usize;
                result.extend_from_slice(&out[..produced_out]);
                input = &input[consumed_in..];
                if status == flate2::Status::StreamEnd || input.is_empty() {
                    break;
                }
                if consumed_in == 0 && produced_out == 0 {
                    // No progress possible with the current output buffer
                    // size and remaining input — avoid an infinite loop.
                    break;
                }
            }
            result
        }
    }
}

/// Scan `accum` for a data-descriptor whose declared CRC-32 and size
/// match the content preceding it, returning `(cut_index, uses_zip64)`.
/// Tries the non-zip64 (12-byte) form first, then the zip64 (20-byte)
/// form, at every signature occurrence, since both are legal and this
/// module has no other signal for which one a `Store` entry used.
fn find_validated_store_descriptor(accum: &[u8]) -> Option<(usize, bool)> {
    let sig = DATA_DESCRIPTOR_SIG.to_le_bytes();
    let mut search_from = 0;
    while let Some(rel) = accum[search_from..].windows(4).position(|w| w == sig) {
        let p = search_from + rel;
        if let Some(result) = try_validate_store_descriptor(accum, p, false) {
            return Some(result);
        }
        if let Some(result) = try_validate_store_descriptor(accum, p, true) {
            return Some(result);
        }
        search_from = p + 1;
        if search_from >= accum.len() {
            break;
        }
    }
    None
}

fn try_validate_store_descriptor(
    accum: &[u8],
    sig_pos: usize,
    zip64: bool,
) -> Option<(usize, bool)> {
    let after_sig = sig_pos + 4;
    let size_len = if zip64 { 8 } else { 4 };
    let need = 4 + size_len * 2;
    if after_sig + need > accum.len() {
        return None;
    }
    let crc = u32::from_le_bytes(accum[after_sig..after_sig + 4].try_into().unwrap());
    let (compressed, uncompressed) = if zip64 {
        (
            u64::from_le_bytes(accum[after_sig + 4..after_sig + 12].try_into().unwrap()),
            u64::from_le_bytes(accum[after_sig + 12..after_sig + 20].try_into().unwrap()),
        )
    } else {
        (
            u32::from_le_bytes(accum[after_sig + 4..after_sig + 8].try_into().unwrap()) as u64,
            u32::from_le_bytes(accum[after_sig + 8..after_sig + 12].try_into().unwrap()) as u64,
        )
    };
    if compressed as usize != sig_pos || uncompressed as usize != sig_pos {
        return None;
    }
    let mut hasher = flate2::Crc::new();
    hasher.update(&accum[..sig_pos]);
    if hasher.sum() != crc {
        return None;
    }
    Some((sig_pos, zip64))
}
