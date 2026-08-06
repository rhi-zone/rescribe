//! `events()` — pull iterator over an EPUB's parts.
//!
//! # Design decision: a genuine lazy wrapper over `zip-fmt::events()`, not a walk over `parse()`'s AST
//!
//! Unlike `zip-fmt`'s own `events()` (see that crate's `events.rs` module
//! docs — ZIP is a flat entry list with no natural tree to duplicate-walk),
//! EPUB *does* have a real per-entry classification dependency: every
//! entry other than `mimetype`/`container.xml`/the OPF itself can only be
//! classified (nav vs. NCX vs. content document vs. opaque resource) once
//! the OPF's manifest has been read. `zip-fmt::events()` yields entries in
//! archival order, which is not guaranteed to put the OPF first (though it
//! conventionally appears early). This iterator therefore:
//!
//! 1. Pulls entries lazily from the underlying `zip_fmt::EventIter`.
//! 2. Buffers (already-decompressed, since `zip-fmt`'s `events()` yields
//!    fully-decompressed `OwnedEvent`s per entry — see that crate's
//!    `events.rs`) any entry seen before the OPF is found — bounded by
//!    "total bytes of entries preceding the OPF in the archive", not the
//!    full archive.
//! 3. Once the OPF is found and parsed, classifies and drains the buffer
//!    (oldest first) before continuing to classify subsequent entries as
//!    they arrive directly, one per `next()` call.
//!
//! This is a genuinely incremental pull iterator distinct from `parse()`
//! (which decompresses every manifest-referenced entry unconditionally
//! before returning), not `parse()`'s output walked after the fact: a
//! caller that stops iterating early (e.g. "just find the nav document")
//! never pays to decompress entries after it stops, and entries after the
//! OPF are decompressed exactly once, on demand.

use std::collections::VecDeque;

use crate::ast::{CONTAINER_PATH, ENCRYPTION_PATH, MIMETYPE_ENTRY};
use crate::classify::{Classified, classify_entry};

pub use crate::ast::{ContentDocument, Navigation, Ncx, Package, ResourceEntry};
use rescribe_format_api::Diagnostic;

/// One classified EPUB part, delivered as its containing archive entry is
/// reached (see module docs for the OPF-ordering caveat).
#[derive(Debug, PartialEq)]
pub enum Event {
    Container(crate::ast::Container),
    Package(Box<Package>),
    Nav(Navigation),
    Ncx(Ncx),
    ContentDocument(ContentDocument),
    Resource(ResourceEntry),
    EncryptionXml(Vec<u8>),
    /// An archive entry that is neither `mimetype`, `container.xml`, the
    /// OPF, `encryption.xml`, nor referenced by the manifest.
    Unclassified {
        path: String,
        content: Vec<u8>,
    },
}

pub type OwnedEvent = Event;

pub struct EventIter<'a> {
    inner: zip_fmt::EventIter<'a>,
    package: Option<Package>,
    base_dir: String,
    opf_path: Option<String>,
    /// Entries seen before the OPF was found/parsed, held until it is.
    buffered: VecDeque<zip_fmt::OwnedEvent>,
    /// Classified events ready to yield, drained before pulling more from
    /// `inner`.
    pending: VecDeque<Event>,
    diagnostics: Vec<Diagnostic>,
    done: bool,
}

impl<'a> EventIter<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        EventIter {
            inner: zip_fmt::EventIter::new(input),
            package: None,
            base_dir: String::new(),
            opf_path: None,
            buffered: VecDeque::new(),
            pending: VecDeque::new(),
            diagnostics: Vec::new(),
            done: false,
        }
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    fn classify_and_queue(&mut self, name: String, content: Vec<u8>) {
        let Some(package) = &self.package else {
            unreachable!("classify_and_queue called before package is known");
        };
        if name == ENCRYPTION_PATH {
            self.pending.push_back(Event::EncryptionXml(content));
            return;
        }
        match classify_entry(
            package,
            &self.base_dir,
            &name,
            &content,
            &mut self.diagnostics,
        ) {
            Some(Classified::Nav(n)) => self.pending.push_back(Event::Nav(n)),
            Some(Classified::Ncx(n)) => self.pending.push_back(Event::Ncx(n)),
            Some(Classified::ContentDocument(d)) => {
                self.pending.push_back(Event::ContentDocument(d))
            }
            Some(Classified::Resource(r)) => self.pending.push_back(Event::Resource(r)),
            None => self.pending.push_back(Event::Unclassified {
                path: name,
                content,
            }),
        }
    }
}

impl Iterator for EventIter<'_> {
    type Item = Event;

    fn next(&mut self) -> Option<Event> {
        loop {
            if let Some(ev) = self.pending.pop_front() {
                return Some(ev);
            }
            if self.done {
                return None;
            }

            let Some(zev) = self.inner.next() else {
                // Underlying archive exhausted. Any still-buffered entries
                // never got classified (OPF was never found) — surface
                // them as unclassified rather than silently dropping.
                self.done = true;
                while let Some(zip_fmt::events::Event::Entry { name, content, .. }) =
                    self.buffered.pop_front()
                {
                    self.pending.push_back(Event::Unclassified {
                        path: name.into_owned(),
                        content: content.into_owned(),
                    });
                }
                continue;
            };

            let zip_fmt::events::Event::Entry { name, content, .. } = zev else {
                continue; // ArchiveComment: not a classifiable EPUB part.
            };
            let name = name.into_owned();
            let content = content.into_owned();

            if name == MIMETYPE_ENTRY {
                continue;
            }
            if name == CONTAINER_PATH {
                match crate::container::parse_container(&content) {
                    Ok(c) => {
                        self.opf_path = c.rootfiles.first().map(|r| r.full_path.clone());
                        return Some(Event::Container(c));
                    }
                    Err(msg) => {
                        self.diagnostics.push(crate::parse::warn(format!(
                            "failed to parse container.xml: {msg}"
                        )));
                        continue;
                    }
                }
            }

            if self.package.is_none() {
                if self.opf_path.as_deref() == Some(name.as_str()) {
                    match crate::opf::parse_package(&content) {
                        Ok(p) => {
                            self.base_dir = crate::pathutil::dir_of(&name);
                            self.package = Some(p.clone());
                            // Drain everything buffered before the OPF appeared.
                            while let Some(zip_fmt::events::Event::Entry {
                                name, content, ..
                            }) = self.buffered.pop_front()
                            {
                                self.classify_and_queue(name.into_owned(), content.into_owned());
                            }
                            return Some(Event::Package(Box::new(p)));
                        }
                        Err(msg) => {
                            self.diagnostics.push(crate::parse::warn(format!(
                                "failed to parse OPF package document: {msg}"
                            )));
                            continue;
                        }
                    }
                } else {
                    // OPF not found yet (or its path is not yet known
                    // because container.xml hasn't been seen either) —
                    // hold this entry for later classification.
                    self.buffered.push_back(zip_fmt::events::Event::Entry {
                        name: std::borrow::Cow::Owned(name),
                        is_utf8_name: true,
                        compression: zip_fmt::CompressionMethod::Store,
                        uncompressed_size: content.len() as u64,
                        compressed_size: content.len() as u64,
                        crc32: 0,
                        modified: Default::default(),
                        external_attrs: 0,
                        comment: std::borrow::Cow::Borrowed(""),
                        extra_field: std::borrow::Cow::Borrowed(&[]),
                        content: std::borrow::Cow::Owned(content),
                    });
                    continue;
                }
            } else {
                self.classify_and_queue(name, content);
                continue;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EpubDoc;
    use rescribe_format_api::Parse as _;

    fn sample_epub() -> Vec<u8> {
        crate::testutil::sample_epub()
    }

    #[test]
    fn events_match_parse_classification() {
        let bytes = sample_epub();
        let (parsed, _): (EpubDoc, _) = crate::EpubDoc::parse(&bytes);
        let events: Vec<_> = EventIter::new(&bytes).collect();

        let content_doc_count = events
            .iter()
            .filter(|e| matches!(e, Event::ContentDocument(_)))
            .count();
        assert_eq!(content_doc_count, parsed.content_documents.len());
        assert!(events.iter().any(|e| matches!(e, Event::Package(_))));
        assert!(events.iter().any(|e| matches!(e, Event::Container(_))));
    }
}
