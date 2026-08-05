//! `events()` — pull iterator over ZIP entries.
//!
//! # Design decision: wraps `zip::ZipArchive` directly, not a walk over `parse()`'s `Archive`
//!
//! ZIP is not a nested/hierarchical format the way XML or CommonMark are —
//! an archive is a flat list of entries, each independently addressable via
//! the central directory. There is no equivalent of OPML's "the parser IS
//! the iterator because walking a tree after the fact would be pointless
//! duplication of a *recursive-descent* parser" argument, because there is
//! no recursive descent here: `parse()` and `events()` would both do the
//! exact same thing — look up the central directory once, then iterate
//! entries by index — the only question is whether entry *content* is
//! decompressed eagerly (all `zip.len()` entries, before the caller sees
//! any of them, as `parse()` does to build a fully materialized `Archive`)
//! or lazily (one entry's content per `next()` call, as `EventIter` does
//! here).
//!
//! That laziness is the genuine, real benefit `events()` provides over
//! `parse()` for this format: a caller that only wants entry *names* (e.g.
//! "does this archive contain `mimetype`?") or wants to stop after the
//! first match never pays to decompress entries it doesn't look at.
//! `EventIter` therefore wraps `zip::ZipArchive<Cursor<&[u8]>>` directly
//! and decompresses one entry per `next()` call — a true pull iterator
//! implementation, not `parse()`'s output walked after the fact.
//!
//! This does mean `events()` and `parse()` share the same underlying
//! `zip::ZipArchive` machinery (both are "library-backed" per the
//! crate-level exception rationale — see `lib.rs`) rather than being two
//! independently-hand-rolled implementations the way a hand-rolled-parser
//! format's `parse()`/`events()` are. That is inherent to wrapping `zip`
//! for the in-memory, already-`Seek`-able case; the genuinely independent,
//! hand-rolled implementation in this crate is `reader-batch`'s
//! `StreamingParser` (`batch.rs`), which cannot assume `Seek` and must
//! parse local headers directly. See `lib.rs` for the full API-layer
//! rationale.
//!
//! Unlike a hierarchical format's event stream, there is no `Start*`/`End*`
//! pairing here: each entry is delivered as a single, complete `Event::Entry`
//! (content and all), because nothing about reading from an already-fully-
//! buffered, already-central-directory-indexed archive is naturally
//! chunked — the atomic streaming unit for this format is "one entry", not
//! "one chunk of one entry's content". Contrast `batch.rs`'s `Event`, whose
//! shape (`Start`/`Data`/`End`) is forced by having to emit *something*
//! before an entry's content or final metadata may be fully known.

use std::borrow::Cow;
use std::io::{Cursor, Read};

use crate::ast::{CompressionMethod, Diagnostic, DosTimestamp, Span};
use crate::parse::zip_method_to_code;

/// One ZIP archive-level event: either the archive comment (delivered
/// first, if present) or one fully-read entry.
#[derive(Debug, PartialEq)]
pub enum Event<'a> {
    /// The end-of-central-directory comment, delivered once before any
    /// `Entry` events, if the archive had a non-empty comment.
    ArchiveComment(Cow<'a, [u8]>),
    /// One complete entry: metadata plus decompressed content.
    Entry {
        name: Cow<'a, str>,
        is_utf8_name: bool,
        compression: CompressionMethod,
        uncompressed_size: u64,
        compressed_size: u64,
        crc32: u32,
        modified: DosTimestamp,
        external_attrs: u32,
        comment: Cow<'a, str>,
        extra_field: Cow<'a, [u8]>,
        content: Cow<'a, [u8]>,
    },
}

pub type OwnedEvent = Event<'static>;

impl Event<'_> {
    pub fn into_owned(self) -> OwnedEvent {
        match self {
            Event::ArchiveComment(c) => Event::ArchiveComment(Cow::Owned(c.into_owned())),
            Event::Entry {
                name,
                is_utf8_name,
                compression,
                uncompressed_size,
                compressed_size,
                crc32,
                modified,
                external_attrs,
                comment,
                extra_field,
                content,
            } => Event::Entry {
                name: Cow::Owned(name.into_owned()),
                is_utf8_name,
                compression,
                uncompressed_size,
                compressed_size,
                crc32,
                modified,
                external_attrs,
                comment: Cow::Owned(comment.into_owned()),
                extra_field: Cow::Owned(extra_field.into_owned()),
                content: Cow::Owned(content.into_owned()),
            },
        }
    }
}

/// A pull iterator over one archive's entries. Decompresses each entry's
/// content lazily, one per `next()` call — see the module docs for why
/// this is a real independent benefit over `parse()` for this format
/// rather than a stylistic variant.
pub struct EventIter<'a> {
    zip: Option<zip::ZipArchive<Cursor<&'a [u8]>>>,
    index: usize,
    len: usize,
    comment_emitted: bool,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> EventIter<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        match zip::ZipArchive::new(Cursor::new(input)) {
            Ok(zip) => {
                let len = zip.len();
                EventIter {
                    zip: Some(zip),
                    index: 0,
                    len,
                    comment_emitted: false,
                    diagnostics: Vec::new(),
                }
            }
            Err(e) => {
                let mut diagnostics = Vec::new();
                diagnostics.push(Diagnostic {
                    message: format!("failed to open ZIP archive: {e}"),
                    span: Span::NONE,
                });
                EventIter {
                    zip: None,
                    index: 0,
                    len: 0,
                    comment_emitted: true,
                    diagnostics,
                }
            }
        }
    }

    /// Diagnostics accumulated so far (archive-open failure, or one entry
    /// this iterator could not read). Meaningful once iteration is
    /// complete; may grow between `next()` calls before that.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

impl<'a> Iterator for EventIter<'a> {
    type Item = OwnedEvent;

    fn next(&mut self) -> Option<OwnedEvent> {
        let zip = self.zip.as_mut()?;

        if !self.comment_emitted {
            self.comment_emitted = true;
            let comment = zip.comment();
            if !comment.is_empty() {
                return Some(Event::ArchiveComment(Cow::Owned(comment.to_vec())));
            }
        }

        while self.index < self.len {
            let i = self.index;
            self.index += 1;
            let mut file = match zip.by_index_raw(i) {
                Ok(f) => f,
                Err(e) => {
                    self.diagnostics.push(Diagnostic {
                        message: format!("entry {i}: failed to read header: {e}"),
                        span: Span::NONE,
                    });
                    continue;
                }
            };

            let name = file.name().to_string();
            let is_utf8_name = file.name_raw() == name.as_bytes();
            let compression = zip_method_to_code(file.compression());
            let uncompressed_size = file.size();
            let modified = file.last_modified().unwrap_or(zip::DateTime::DEFAULT);
            let external_attrs = file.unix_mode().map(|m| m << 16).unwrap_or(0);
            let comment_entry = file.comment().to_string();
            let extra_field = file.extra_data().unwrap_or_default().to_vec();
            let compressed_size = file.compressed_size();
            let crc32 = file.crc32();

            let mut raw_bytes = Vec::with_capacity(compressed_size as usize);
            if let Err(e) = file.read_to_end(&mut raw_bytes) {
                self.diagnostics.push(Diagnostic {
                    message: format!("entry {name}: failed to read raw content: {e}"),
                    span: Span::NONE,
                });
            }
            let content = crate::parse::decompress_raw(
                compression,
                &raw_bytes,
                uncompressed_size,
                &mut self.diagnostics,
                &name,
            );

            return Some(Event::Entry {
                name: Cow::Owned(name),
                is_utf8_name,
                compression,
                uncompressed_size,
                compressed_size,
                crc32,
                modified: DosTimestamp {
                    date: modified.datepart(),
                    time: modified.timepart(),
                },
                external_attrs,
                comment: Cow::Owned(comment_entry),
                extra_field: Cow::Owned(extra_field),
                content: Cow::Owned(content),
            });
        }
        None
    }
}
