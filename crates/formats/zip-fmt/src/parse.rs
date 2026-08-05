//! `parse()` — full-archive AST reader.
//!
//! Wraps `zip::ZipArchive<Cursor<&[u8]>>`. A `&[u8]` is trivially `Seek`
//! (via `Cursor`), and the ZIP container format is itself seek-oriented —
//! the authoritative entry list lives in the central directory at the
//! *end* of the file, not discoverable by a single forward pass — so an
//! in-memory random-access reader is the right tool here, not a hand-rolled
//! parser. See the crate-level docs for why this differs from
//! `reader-batch`'s `StreamingParser`, which cannot assume a `Seek`able
//! source.

use std::io::{Cursor, Read};

use crate::ast::{Archive, CompressionMethod, DosTimestamp, Entry, Span};

/// Diagnostic message from parsing.
pub use crate::ast::Diagnostic;

/// Parse a complete ZIP archive already in memory.
///
/// Never panics: a malformed or truncated archive produces an empty
/// `Archive` plus a `Diagnostic` describing the failure, matching the
/// error-tolerant contract other `-fmt` crates in this workspace follow.
pub(crate) fn parse(input: &[u8]) -> (Archive, Vec<Diagnostic>) {
    let mut diags = Vec::new();
    let cursor = Cursor::new(input);
    let mut zip = match zip::ZipArchive::new(cursor) {
        Ok(z) => z,
        Err(e) => {
            diags.push(Diagnostic {
                message: format!("failed to open ZIP archive: {e}"),
                span: Span {
                    start: 0,
                    end: input.len(),
                },
                severity: rescribe_format_api::Severity::Warning,
                code: "",
            });
            return (
                Archive {
                    entries: Vec::new(),
                    comment: Vec::new(),
                    span: Span {
                        start: 0,
                        end: input.len(),
                    },
                },
                diags,
            );
        }
    };

    let comment = zip.comment().to_vec();
    let mut entries = Vec::with_capacity(zip.len());

    for i in 0..zip.len() {
        let mut file = match zip.by_index_raw(i) {
            Ok(f) => f,
            Err(e) => {
                diags.push(Diagnostic {
                    message: format!("entry {i}: failed to read header: {e}"),
                    span: Span::NONE,
                    severity: rescribe_format_api::Severity::Warning,
                    code: "",
                });
                continue;
            }
        };

        let name = file.name().to_string();
        let is_utf8_name = file.name_raw() == name.as_bytes();
        let compression = zip_method_to_code(file.compression());
        let uncompressed_size = file.size();
        let compressed_size = file.compressed_size();
        let crc32 = file.crc32();
        let modified = file.last_modified().unwrap_or(zip::DateTime::DEFAULT);
        // `zip::ZipFile` does not expose the raw general-purpose flags
        // field publicly (its `flags` accessor is crate-private) — only
        // the two bits it separately surfaces: encryption (`encrypted()`)
        // and UTF-8 naming (inferred here from whether `name_raw` matches
        // the decoded UTF-8 `name`). Other flag bits (enhanced-deflate
        // sub-options, strong encryption, ...) are not reconstructable
        // through this API and are best-effort zeroed on this reader path.
        // The `reader-batch` `StreamingParser` parses local headers itself
        // and captures the raw flags field exactly — see `batch.rs`.
        let mut flags: u16 = 0;
        if file.encrypted() {
            flags |= 1;
        }
        if is_utf8_name {
            flags |= 1 << 11;
        }
        let (version_made_by_host, version_made_by_ver) = file.version_made_by();
        let version_made_by = ((version_made_by_host as u16) << 8) | version_made_by_ver as u16;
        // External file attributes: not directly exposed either; `unix_mode()`
        // reconstructs the Unix permission bits (upper 16 bits) when the
        // producing host was Unix, which is the field's dominant real-world
        // use. DOS-host attribute bytes (lower 8 bits on non-Unix archives)
        // are not recoverable through this API and are zeroed here.
        let external_attrs = file.unix_mode().map(|m| m << 16).unwrap_or(0);
        let comment_entry = file.comment().to_string();
        let extra_field = file.extra_data().unwrap_or_default().to_vec();
        let header_start = file.header_start();
        let header_end = file.data_start();

        // Read the *raw* (still-compressed) bytes so entries this crate
        // cannot decompress (Shrink/Reduce/Implode without the `legacy-zip`
        // feature on the `zip` dependency) still round-trip their content
        // bytes verbatim rather than being dropped.
        let mut raw_bytes = Vec::with_capacity(compressed_size as usize);
        if let Err(e) = file.read_to_end(&mut raw_bytes) {
            diags.push(Diagnostic {
                message: format!("entry {name}: failed to read raw content: {e}"),
                span: Span::NONE,
                severity: rescribe_format_api::Severity::Warning,
                code: "",
            });
        }
        let content = decompress_raw(
            compression,
            &raw_bytes,
            uncompressed_size,
            &mut diags,
            &name,
        );

        entries.push(Entry {
            name,
            is_utf8_name,
            compression,
            uncompressed_size,
            compressed_size,
            crc32,
            modified: DosTimestamp {
                date: modified.datepart(),
                time: modified.timepart(),
            },
            flags,
            external_attrs,
            internal_attrs: 0,
            version_made_by,
            version_needed: 0,
            extra_field,
            comment: comment_entry,
            content,
            span: Span {
                start: header_start as usize,
                end: header_end as usize,
            },
        });
    }

    (
        Archive {
            entries,
            comment,
            span: Span {
                start: 0,
                end: input.len(),
            },
        },
        diags,
    )
}

/// Decompress raw entry bytes read via `by_index_raw`, given the entry's
/// declared method. `by_index_raw` deliberately skips `zip`'s own
/// decompression so this crate controls the mapping from *our*
/// `CompressionMethod` back to bytes — the same code path `emit()` inverts.
pub(crate) fn decompress_raw(
    method: CompressionMethod,
    raw: &[u8],
    uncompressed_size: u64,
    diags: &mut Vec<Diagnostic>,
    name: &str,
) -> Vec<u8> {
    match method {
        CompressionMethod::Store => raw.to_vec(),
        CompressionMethod::Deflate => {
            let mut out = Vec::with_capacity(uncompressed_size as usize);
            let mut d = flate2::read::DeflateDecoder::new(raw);
            match d.read_to_end(&mut out) {
                Ok(_) => out,
                Err(e) => {
                    diags.push(Diagnostic {
                        message: format!("entry {name}: deflate decode failed: {e}"),
                        span: Span::NONE,
                        severity: rescribe_format_api::Severity::Warning,
                        code: "",
                    });
                    Vec::new()
                }
            }
        }
        _ => {
            diags.push(Diagnostic {
                message: format!(
                    "entry {name}: compression method {method:?} not decoded by zip-fmt; \
                     content stored as raw (still-compressed) bytes"
                ),
                span: Span::NONE,
                severity: rescribe_format_api::Severity::Warning,
                code: "",
            });
            raw.to_vec()
        }
    }
}

pub(crate) fn zip_method_to_code(m: zip::CompressionMethod) -> CompressionMethod {
    #[allow(deprecated)]
    match m {
        zip::CompressionMethod::Stored => CompressionMethod::Store,
        zip::CompressionMethod::Unsupported(c) => CompressionMethod::from_code(c),
        other if other == zip::CompressionMethod::DEFLATE => CompressionMethod::Deflate,
        other if other == zip::CompressionMethod::BZIP2 => CompressionMethod::Bzip2,
        other if other == zip::CompressionMethod::ZSTD => CompressionMethod::Zstd,
        other if other == zip::CompressionMethod::XZ => CompressionMethod::Xz,
        other if other == zip::CompressionMethod::LZMA => CompressionMethod::Lzma,
        other if other == zip::CompressionMethod::PPMD => CompressionMethod::Ppmd,
        other if other == zip::CompressionMethod::AES => CompressionMethod::Aes,
        other if other == zip::CompressionMethod::DEFLATE64 => CompressionMethod::Other(9),
        _ => CompressionMethod::Other(0xffff),
    }
}
