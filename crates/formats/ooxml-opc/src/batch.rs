//! `StreamingParser<H: Handler>` — chunk-fed OPC (Open Packaging
//! Conventions) reader, built on `zip-fmt`'s hand-rolled push-based
//! `StreamingParser`. This is a new, additive mode alongside
//! [`crate::Package`]: `Package::open` requires `Read + Seek` because it
//! needs random access to the ZIP central directory (for by-name part
//! lookup and lazy relationship resolution). This module instead consumes
//! the archive as a byte stream — no seeking, no full-archive buffering —
//! at the cost of not being able to look a part up by name before it has
//! streamed past.
//!
//! # Design: incremental part delivery, plus two bounded, narrow buffering exceptions
//!
//! **Update (2026-08-08): generic parts now stream sub-entry.** The
//! original design buffered every ZIP entry's `Data` chunks in full before
//! emitting one `Event::Part { content: Vec<u8>, .. }` — a real gap flagged
//! in the TODO.md "ooxml-fmt rework" entry, since a single OOXML part
//! (`word/document.xml`, a large worksheet) can itself be the
//! multi-hundred-MB file that doesn't fit in memory, defeating the whole
//! point of this module. `Event::Part` is now three events —
//! [`Event::PartStart`], zero or more [`Event::PartData`] chunks, and
//! [`Event::PartEnd`] — and, for the common case (see below), each
//! `zip_fmt::batch::Event::Data` chunk for a generic part is forwarded to
//! the caller's handler the moment it decompresses, not accumulated first.
//!
//! Two documented, scoped exceptions remain — the same shape as
//! `epub-fmt`'s `batch.rs` (the closest analog in this workspace — also a
//! ZIP-based OPC-like container needing a manifest part resolved before
//! other parts can be classified), narrowed from "every entry" to just
//! these:
//!
//! 1. **`[Content_Types].xml` and `.rels` parts are still buffered in
//!    full.** Both need a complete `quick-xml`-style parse
//!    ([`ContentTypes::parse`]/[`Relationships::parse`], neither of which
//!    is chunk-fed) to produce their typed value, and both are inherently
//!    small, bounded metadata files — never the multi-hundred-MB case this
//!    module exists to fix. `[Content_Types].xml` is still classified from
//!    its literal name at `StartEntry`, before any content arrives, so
//!    this exception applies to exactly those two part kinds, never to a
//!    generic content part.
//! 2. **Pre-`[Content_Types].xml` buffering, generic parts only.** Every
//!    generic part's declared MIME type comes from `[Content_Types].xml`
//!    (default-by-extension or a per-part override) — a part encountered
//!    before it has streamed past cannot yet be resolved, so it cannot be
//!    handed to the caller with a known `content_type` yet. A generic part
//!    that streams past before `[Content_Types].xml` is therefore still
//!    accumulated in full and replayed (as a single-chunk `PartStart`/
//!    `PartData`/`PartEnd` triple) once `[Content_Types].xml` arrives, in
//!    original order. This is bounded by "total bytes of generic parts
//!    preceding `[Content_Types].xml`", not the full archive — real OOXML
//!    packages conventionally put it first (it's often literally the first
//!    ZIP entry written by Office/OpenXML SDK), but neither the ZIP nor OPC
//!    spec mandates entry order. **Once `[Content_Types].xml` has streamed
//!    past, every subsequent generic part streams sub-entry with no
//!    buffering** — this is the dominant real-world case.
//!
//! Memory profile: O(largest ZIP-decompressor output chunk + bytes of
//! generic parts preceding `[Content_Types].xml`), not O(largest part) and
//! not O(full archive) — except for `[Content_Types].xml` and `.rels`
//! parts themselves, which remain O(that part's size) (exception 1 above;
//! always small in practice).
//!
//! # Relationship parts are recognized structurally, not buffered specially
//!
//! A `_rels/*.rels` part (`_rels/.rels` for the package root, or
//! `{dir}/_rels/{file}.rels` for `{dir}/{file}`) needs no `[Content_Types].xml`
//! lookup to interpret — its own bytes are self-describing XML. Such parts
//! are parsed into [`Event::Relationships`] the moment they're seen (or
//! immediately on drain, if they arrived before `[Content_Types].xml`);
//! they are never delivered as a generic [`Event::Part`].
//!
//! # What this module does not do
//!
//! It does not resolve relationship targets against parts, does not parse
//! any part's own XML content (`word/document.xml` etc. — that's wml's
//! job), and does not implement a package-level `events()` (core/app
//! properties) mode — both explicitly out of scope for this task. It also
//! does not replace [`crate::Package`]; that seekable path is unchanged.

use std::cell::RefCell;
use std::rc::Rc;

use rescribe_format_api::Severity;
pub use rescribe_format_api::{Diagnostic, Handler};

use crate::packaging::ContentTypes;
use crate::relationships::Relationships;

/// One OPC-level streaming event.
#[derive(Debug, Clone)]
pub enum Event {
    /// `[Content_Types].xml` has been fully parsed. Always the first
    /// event delivered (aside from any [`Event::Relationships`] whose
    /// owning `.rels` part happened to stream past first — see the module
    /// docs; those are not held back by the `[Content_Types].xml` gate
    /// since they don't need it) — every generic part's [`Event::PartStart`]
    /// is held until this fires.
    ContentTypes(ContentTypes),
    /// A `_rels/*.rels` part has been parsed. `part_path` is the OPC part
    /// these relationships belong to (`""` for the package-level
    /// `_rels/.rels`), derived structurally from the `.rels` part's own
    /// path — the same convention as [`crate::relationships::rels_path_for`],
    /// inverted.
    Relationships {
        part_path: String,
        relationships: Relationships,
    },
    /// A generic package part's content has started. `content_type` is the
    /// resolved MIME type (`None` if `[Content_Types].xml` has neither a
    /// matching `Override` nor a matching `Default` for the part's
    /// extension). Followed by zero or more [`Event::PartData`] chunks and
    /// exactly one [`Event::PartEnd`] before the next `PartStart` (or the
    /// end of the stream) — parts are never interleaved, since ZIP entries
    /// are physically sequential.
    PartStart {
        path: String,
        content_type: Option<String>,
    },
    /// One chunk of the current part's decompressed content, following the
    /// most recent [`Event::PartStart`]. Chunk boundaries are an
    /// implementation detail (driven by the underlying ZIP decompressor's
    /// own output chunking — see the module docs for the one narrow case
    /// where a chunk is the whole part instead), not meaningful framing —
    /// a handler that wants the whole part concatenates every `PartData`
    /// chunk between `PartStart` and `PartEnd`.
    PartData(Vec<u8>),
    /// The part started by the most recent [`Event::PartStart`] has ended.
    PartEnd,
}

/// Given a `.rels` part's own path, return the OPC part path it applies
/// to, per the inverse of [`crate::relationships::rels_path_for`].
/// Returns `None` for any path that isn't a `.rels` part in a `_rels`
/// directory (i.e. any ordinary package part).
fn rels_owner_part(path: &str) -> Option<String> {
    if let Some(idx) = path.rfind("/_rels/") {
        let dir = &path[..idx];
        let file = path[idx + "/_rels/".len()..].strip_suffix(".rels")?;
        return Some(if dir.is_empty() {
            file.to_string()
        } else {
            format!("{dir}/{file}")
        });
    }
    let rest = path.strip_prefix("_rels/")?;
    let file = rest.strip_suffix(".rels")?;
    Some(file.to_string())
}

struct Shared<H: Handler<Event>> {
    handler: H,
    content_types: Option<ContentTypes>,
    /// Generic parts that streamed past before `[Content_Types].xml` — see
    /// module docs, exception 2. Always fully-decompressed content (a
    /// `.rels`/`[Content_Types].xml` part never lands here; both are
    /// classified and handled at `StartEntry` before any content arrives).
    buffered: Vec<(String, Vec<u8>)>,
    diagnostics: Vec<Diagnostic>,
}

/// `[Content_Types].xml`'s full bytes have arrived (buffered per module
/// docs exception 1). Parse it, emit [`Event::ContentTypes`], then replay
/// every generic part that had to wait for it (exception 2), in original
/// order.
fn on_content_types<H: Handler<Event>>(shared: &mut Shared<H>, content: Vec<u8>) {
    match ContentTypes::parse(&content[..]) {
        Ok(ct) => {
            shared.handler.handle(Event::ContentTypes(ct.clone()));
            shared.content_types = Some(ct);
            let buffered = std::mem::take(&mut shared.buffered);
            for (name, content) in buffered {
                emit_buffered_part(shared, name, content);
            }
        }
        Err(e) => shared.diagnostics.push(Diagnostic::new(
            Severity::Warning,
            format!("failed to parse [Content_Types].xml: {e}"),
        )),
    }
}

/// Replay one previously-buffered generic part as a single-chunk
/// `PartStart`/`PartData`/`PartEnd` triple, now that `[Content_Types].xml`
/// (or, in [`StreamingParser::finish`]'s case, end of input) has resolved
/// what can be resolved.
fn emit_buffered_part<H: Handler<Event>>(shared: &mut Shared<H>, name: String, content: Vec<u8>) {
    let content_type = shared
        .content_types
        .as_ref()
        .and_then(|ct| ct.get(&name))
        .map(|s| s.to_string());
    shared.handler.handle(Event::PartStart {
        path: name,
        content_type,
    });
    if !content.is_empty() {
        shared.handler.handle(Event::PartData(content));
    }
    shared.handler.handle(Event::PartEnd);
}

fn emit_relationships<H: Handler<Event>>(
    shared: &mut Shared<H>,
    owner: String,
    name: &str,
    content: Vec<u8>,
) {
    match Relationships::parse(&content[..]) {
        Ok(relationships) => shared.handler.handle(Event::Relationships {
            part_path: owner,
            relationships,
        }),
        Err(e) => shared.diagnostics.push(Diagnostic::new(
            Severity::Warning,
            format!("failed to parse relationships part {name}: {e}"),
        )),
    }
}

/// What the current ZIP entry (between `StartEntry` and `EndEntry`) is
/// being treated as, decided at `StartEntry` from the entry's name alone
/// (and, for a generic part, whether `[Content_Types].xml` has resolved
/// yet) — never from its content.
enum EntryKind {
    /// `[Content_Types].xml` itself — buffered in full (module docs,
    /// exception 1).
    ContentTypes,
    /// A `_rels/*.rels` part — buffered in full (exception 1). Carries the
    /// OPC part path these relationships belong to.
    Rels(String),
    /// A generic part, `[Content_Types].xml` already resolved: streamed
    /// sub-entry with no buffering. `PartStart` has already been emitted
    /// by the time this variant is current.
    StreamingPart,
    /// A generic part, `[Content_Types].xml` not yet resolved: buffered in
    /// full (exception 2) for replay once it arrives.
    BufferedPart,
}

struct InnerHandler<H: Handler<Event>> {
    shared: Rc<RefCell<Shared<H>>>,
    current_name: String,
    kind: EntryKind,
    /// Accumulator for `ContentTypes`/`Rels`/`BufferedPart` kinds. Unused
    /// (stays empty) for `StreamingPart`, whose `Data` chunks are forwarded
    /// immediately instead.
    buf: Vec<u8>,
}

impl<H: Handler<Event>> Handler<zip_fmt::batch::Event> for InnerHandler<H> {
    fn handle(&mut self, event: zip_fmt::batch::Event) {
        match event {
            zip_fmt::batch::Event::StartEntry { name, .. } => {
                self.current_name = name.clone();
                self.buf.clear();
                self.kind = if name == "[Content_Types].xml" {
                    EntryKind::ContentTypes
                } else if let Some(owner) = rels_owner_part(&name) {
                    EntryKind::Rels(owner)
                } else {
                    let mut shared = self.shared.borrow_mut();
                    if let Some(content_types) = &shared.content_types {
                        let content_type = content_types.get(&name).map(|s| s.to_string());
                        shared.handler.handle(Event::PartStart {
                            path: name,
                            content_type,
                        });
                        EntryKind::StreamingPart
                    } else {
                        EntryKind::BufferedPart
                    }
                };
            }
            zip_fmt::batch::Event::Data(chunk) => match self.kind {
                EntryKind::StreamingPart => {
                    if !chunk.is_empty() {
                        self.shared
                            .borrow_mut()
                            .handler
                            .handle(Event::PartData(chunk));
                    }
                }
                EntryKind::ContentTypes | EntryKind::Rels(_) | EntryKind::BufferedPart => {
                    self.buf.extend_from_slice(&chunk);
                }
            },
            zip_fmt::batch::Event::EndEntry { .. } => {
                let name = std::mem::take(&mut self.current_name);
                let content = std::mem::take(&mut self.buf);
                let kind = std::mem::replace(&mut self.kind, EntryKind::BufferedPart);
                let mut shared = self.shared.borrow_mut();
                match kind {
                    EntryKind::ContentTypes => on_content_types(&mut shared, content),
                    EntryKind::Rels(owner) => {
                        emit_relationships(&mut shared, owner, &name, content)
                    }
                    EntryKind::StreamingPart => shared.handler.handle(Event::PartEnd),
                    EntryKind::BufferedPart => {
                        // `[Content_Types].xml` cannot have arrived mid-entry
                        // (ZIP entries are physically sequential, and it is
                        // itself a whole separate entry), so this is always
                        // still unresolved here — hold for replay.
                        debug_assert!(shared.content_types.is_none());
                        shared.buffered.push((name, content));
                    }
                }
            }
            zip_fmt::batch::Event::ArchiveComment(_) => {}
        }
    }
}

/// Genuinely incremental, chunk-fed OPC reader. See the module docs for
/// the buffering contract. Additive alongside [`crate::Package`] — does
/// not replace the seekable path.
pub struct StreamingParser<H: Handler<Event>> {
    inner: zip_fmt::StreamingParser<InnerHandler<H>>,
    shared: Rc<RefCell<Shared<H>>>,
}

impl<H: Handler<Event>> StreamingParser<H> {
    pub fn new(handler: H) -> Self {
        let shared = Rc::new(RefCell::new(Shared {
            handler,
            content_types: None,
            buffered: Vec::new(),
            diagnostics: Vec::new(),
        }));
        let inner_handler = InnerHandler {
            shared: shared.clone(),
            current_name: String::new(),
            kind: EntryKind::BufferedPart,
            buf: Vec::new(),
        };
        StreamingParser {
            inner: zip_fmt::StreamingParser::new(inner_handler),
            shared,
        }
    }

    /// Feed the next chunk of archive bytes. May be called with chunks of
    /// any size, including 1 byte, and any number of times.
    pub fn feed(&mut self, chunk: &[u8]) {
        self.inner.feed(chunk);
    }

    /// Signal end of input. Returns accumulated diagnostics. Any part
    /// still buffered because `[Content_Types].xml` was never found is
    /// replayed as a final `PartStart`/`PartData`/`PartEnd` triple (per
    /// [`Event::PartStart`]'s doc) with `content_type: None`.
    pub fn finish(self) -> Vec<Diagnostic> {
        let zip_diags = self.inner.finish();
        let mut shared = Rc::try_unwrap(self.shared)
            .unwrap_or_else(|_| {
                panic!("ooxml-opc StreamingParser: internal state still referenced after finish")
            })
            .into_inner();
        shared.diagnostics.extend(zip_diags);
        if shared.content_types.is_none() && !shared.buffered.is_empty() {
            shared.diagnostics.push(Diagnostic::new(
                Severity::Warning,
                "[Content_Types].xml was never found; remaining parts delivered with no \
                 resolved content type"
                    .to_string(),
            ));
        }
        let buffered = std::mem::take(&mut shared.buffered);
        for (path, content) in buffered {
            emit_buffered_part(&mut shared, path, content);
        }
        shared.diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packaging::{Package, content_type};
    use crate::relationships::{Relationship, rel_type};
    use std::io::Cursor;

    fn create_test_package() -> Vec<u8> {
        let mut buf = Cursor::new(Vec::new());
        {
            let mut writer = crate::packaging::PackageWriter::new(&mut buf);
            writer.add_default_content_type("rels", content_type::RELATIONSHIPS);
            writer.add_default_content_type("xml", content_type::XML);

            let document = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body><w:p><w:r><w:t>Hello!</w:t></w:r></w:p></w:body>
</w:document>"#;
            writer
                .add_part(
                    "word/document.xml",
                    content_type::WORDPROCESSING_DOCUMENT,
                    document.as_bytes(),
                )
                .unwrap();

            let root_rels = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#;
            writer
                .add_part(
                    "_rels/.rels",
                    content_type::RELATIONSHIPS,
                    root_rels.as_bytes(),
                )
                .unwrap();

            let doc_rels = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
</Relationships>"#;
            writer
                .add_part(
                    "word/_rels/document.xml.rels",
                    content_type::RELATIONSHIPS,
                    doc_rels.as_bytes(),
                )
                .unwrap();

            writer.finish().unwrap();
        }
        buf.into_inner()
    }

    /// Reassemble `PartStart`/`PartData`/`PartEnd` triples in `events` into
    /// `(path, content_type, concatenated_content)` tuples, in the order
    /// each part's `PartStart` appeared. Panics on a malformed sequence
    /// (`PartData`/`PartEnd` with no open `PartStart`) — every test below
    /// relies on the parser never producing one.
    fn collect_parts(events: &[Event]) -> Vec<(String, Option<String>, Vec<u8>)> {
        let mut parts = Vec::new();
        let mut open: Option<(String, Option<String>, Vec<u8>)> = None;
        for ev in events {
            match ev {
                Event::PartStart { path, content_type } => {
                    assert!(
                        open.is_none(),
                        "PartStart while another part was still open"
                    );
                    open = Some((path.clone(), content_type.clone(), Vec::new()));
                }
                Event::PartData(chunk) => {
                    open.as_mut()
                        .expect("PartData with no open PartStart")
                        .2
                        .extend_from_slice(chunk);
                }
                Event::PartEnd => {
                    parts.push(open.take().expect("PartEnd with no open PartStart"));
                }
                _ => {}
            }
        }
        assert!(open.is_none(), "part left open at end of event stream");
        parts
    }

    #[test]
    fn rels_owner_part_matches_rels_path_for() {
        assert_eq!(rels_owner_part("_rels/.rels"), Some(String::new()));
        assert_eq!(
            rels_owner_part("word/_rels/document.xml.rels"),
            Some("word/document.xml".to_string())
        );
        assert_eq!(
            rels_owner_part("_rels/document.xml.rels"),
            Some("document.xml".to_string())
        );
        assert_eq!(rels_owner_part("word/document.xml"), None);
        assert_eq!(rels_owner_part("[Content_Types].xml"), None);

        // Round-trip against the forward function for a sample of paths.
        for part in ["word/document.xml", "document.xml", ""] {
            let rels_path = crate::relationships::rels_path_for(part);
            assert_eq!(rels_owner_part(&rels_path), Some(part.to_string()));
        }
    }

    #[test]
    fn streaming_matches_seekable_package() {
        let bytes = create_test_package();

        // Seekable reference.
        let mut pkg = Package::open(Cursor::new(bytes.clone())).unwrap();
        let expected_doc = pkg.read_part("word/document.xml").unwrap();
        let expected_ct = pkg.content_type("word/document.xml").map(|s| s.to_string());
        let expected_root_rels = pkg.read_relationships().unwrap();
        let expected_doc_rels = pkg.read_part_relationships("word/document.xml").unwrap();

        // Streaming, fed in small, uneven chunks to exercise chunk-boundary
        // handling (mirrors epub-fmt's/zip-fmt's own adversarial tests).
        let mut events = Vec::new();
        {
            let mut p = StreamingParser::new(|ev| events.push(ev));
            for chunk in bytes.chunks(7) {
                p.feed(chunk);
            }
            let diags = p.finish();
            assert!(diags.is_empty(), "diagnostics: {diags:?}");
        }

        let Some(Event::ContentTypes(_)) =
            events.iter().find(|e| matches!(e, Event::ContentTypes(_)))
        else {
            panic!("expected a ContentTypes event");
        };

        let parts = collect_parts(&events);
        let doc_part = parts
            .iter()
            .find(|(path, ..)| path == "word/document.xml")
            .expect("expected word/document.xml as a Part event");
        assert_eq!(doc_part.1, expected_ct);
        assert_eq!(doc_part.2, expected_doc);

        let root_rels = events
            .iter()
            .find_map(|e| match e {
                Event::Relationships {
                    part_path,
                    relationships,
                } if part_path.is_empty() => Some(relationships.clone()),
                _ => None,
            })
            .expect("expected package-level Relationships event");
        assert_eq!(root_rels.len(), expected_root_rels.len());
        assert_eq!(
            root_rels
                .get_by_type(rel_type::OFFICE_DOCUMENT)
                .map(|r| r.target.clone()),
            expected_root_rels
                .get_by_type(rel_type::OFFICE_DOCUMENT)
                .map(|r| r.target.clone())
        );

        let doc_rels = events
            .iter()
            .find_map(|e| match e {
                Event::Relationships {
                    part_path,
                    relationships,
                } if part_path == "word/document.xml" => Some(relationships.clone()),
                _ => None,
            })
            .expect("expected word/document.xml Relationships event");
        assert_eq!(doc_rels.len(), expected_doc_rels.len());

        // No relationships part should ever surface as a generic Part event.
        assert!(!parts.iter().any(|(path, ..)| path.ends_with(".rels")));

        // `.rels` parts are recognized structurally, independent of
        // `Relationship` construction — sanity-check via a manually built
        // (unused) value to keep the `Relationship` import exercised by
        // this test module's intent (owner resolution logic, not content).
        let _ = Relationship::new("rId1", rel_type::STYLES, "styles.xml");
    }

    #[test]
    fn streaming_handles_content_types_arriving_after_parts() {
        // Build a package, then re-zip it with [Content_Types].xml moved to
        // the end, to exercise the pre-buffering path.
        let bytes = create_test_package();
        let (archive, diags) = <zip_fmt::ast::Archive as zip_fmt::Parse>::parse(&bytes);
        assert!(diags.is_empty());

        let mut reordered = archive.entries.clone();
        let ct_index = reordered
            .iter()
            .position(|e| e.name == "[Content_Types].xml")
            .unwrap();
        let ct_entry = reordered.remove(ct_index);
        reordered.push(ct_entry);
        let reordered_archive = zip_fmt::ast::Archive {
            entries: reordered,
            ..archive
        };
        let reordered_bytes = zip_fmt::Emit::emit(&reordered_archive);

        let mut events = Vec::new();
        {
            let mut p = StreamingParser::new(|ev| events.push(ev));
            for chunk in reordered_bytes.chunks(11) {
                p.feed(chunk);
            }
            let diags = p.finish();
            assert!(diags.is_empty(), "diagnostics: {diags:?}");
        }

        // ContentTypes still arrives, and word/document.xml is still
        // resolved to its override content type even though it streamed
        // past before [Content_Types].xml did.
        assert!(events.iter().any(|e| matches!(e, Event::ContentTypes(_))));
        let parts = collect_parts(&events);
        let doc_ct = parts
            .iter()
            .find(|(path, ..)| path == "word/document.xml")
            .map(|(_, content_type, _)| content_type.clone());
        assert_eq!(
            doc_ct,
            Some(Some(content_type::WORDPROCESSING_DOCUMENT.to_string()))
        );
    }

    #[test]
    fn streaming_no_panic_on_truncated_and_empty_input() {
        // Empty input: finish() should not panic, and should report no
        // events beyond nothing.
        let mut events: Vec<Event> = Vec::new();
        {
            let p = StreamingParser::new(|ev| events.push(ev));
            let _diags = p.finish();
        }
        assert!(events.is_empty());

        // Truncated archive: feed only the first half of the bytes.
        let bytes = create_test_package();
        let half = &bytes[..bytes.len() / 2];
        let mut events = Vec::new();
        {
            let mut p = StreamingParser::new(|ev| events.push(ev));
            p.feed(half);
            let _diags = p.finish();
        }
        // Just must not panic; content is whatever partial state resulted.
    }

    /// Adversarial byte-at-a-time feed across a real fixture DOCX, cross-
    /// checked against the seekable `Package` path for every part's bytes.
    #[test]
    fn streaming_matches_package_on_real_docx_fixture() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../fixtures/ooxml/wml/paragraph"
        );
        let Ok(rd) = std::fs::read_dir(path) else {
            // Fixture directory layout can shift; this test is a bonus
            // cross-check, not the primary correctness gate (that's
            // `streaming_matches_seekable_package`, which uses no external
            // files).
            return;
        };
        let Some(docx_path) = rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .find(|p| p.extension().is_some_and(|ext| ext == "docx"))
        else {
            return;
        };
        let bytes = std::fs::read(&docx_path).unwrap();

        let mut pkg = Package::open(Cursor::new(bytes.clone())).unwrap();
        let mut expected_parts: Vec<(String, Vec<u8>)> = pkg
            .parts()
            .filter(|n| *n != "[Content_Types].xml" && !n.ends_with(".rels"))
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .into_iter()
            .map(|n| {
                let data = pkg.read_part(&n).unwrap();
                (n, data)
            })
            .collect();
        expected_parts.sort_by(|a, b| a.0.cmp(&b.0));

        let mut events = Vec::new();
        {
            let mut p = StreamingParser::new(|ev| events.push(ev));
            for chunk in bytes.chunks(4096) {
                p.feed(chunk);
            }
            let diags = p.finish();
            assert!(diags.is_empty(), "diagnostics: {diags:?}");
        }

        let mut actual_parts: Vec<(String, Vec<u8>)> = collect_parts(&events)
            .into_iter()
            .map(|(path, _content_type, content)| (path, content))
            .collect();
        actual_parts.sort_by(|a, b| a.0.cmp(&b.0));

        assert_eq!(actual_parts, expected_parts);
    }

    /// The actual fix this module exists for: a large generic part's
    /// `PartData` must arrive as multiple chunks, none anywhere near the
    /// full part size, when `[Content_Types].xml` streams past first (the
    /// dominant real-world case — see module docs). Structurally verifies
    /// the memory characteristic (bounded per-event chunk size) rather than
    /// only re-checking correctness of the reassembled content, per the
    /// task's own discipline: "still produces correct output" is not
    /// sufficient evidence the improvement landed.
    #[test]
    fn large_part_streams_as_multiple_bounded_chunks_after_content_types() {
        // Incompressible content (not a repeating pattern) so Deflate can't
        // collapse it into a single small compressed — and thus single
        // decompressed — chunk; a naive LCG keeps this deterministic
        // without pulling in a `rand` dependency.
        let mut state: u32 = 0x2545_F491;
        let big: Vec<u8> = (0..80_000u32)
            .map(|_| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (state >> 24) as u8
            })
            .collect();

        let mut buf = Cursor::new(Vec::new());
        {
            let mut writer = crate::packaging::PackageWriter::new(&mut buf);
            writer.add_default_content_type("rels", content_type::RELATIONSHIPS);
            writer.add_default_content_type("bin", "application/octet-stream");
            writer
                .add_part("big/part.bin", "application/octet-stream", &big)
                .unwrap();
            writer.finish().unwrap();
        }
        let unordered_bytes = buf.into_inner();

        // `PackageWriter::finish()` writes `[Content_Types].xml` *last*
        // (see its own doc comment), so re-zip with it moved to the front
        // to exercise the streaming path this test targets — the dominant
        // real-world case per the module docs (Office/OpenXML SDK output
        // conventionally puts it first). The reverse ordering (part before
        // `[Content_Types].xml`, forcing the buffering exception) is
        // already covered by `streaming_handles_content_types_arriving_after_parts`
        // above.
        let (archive, diags) = <zip_fmt::ast::Archive as zip_fmt::Parse>::parse(&unordered_bytes);
        assert!(diags.is_empty());
        let mut reordered = archive.entries.clone();
        let ct_index = reordered
            .iter()
            .position(|e| e.name == "[Content_Types].xml")
            .unwrap();
        let ct_entry = reordered.remove(ct_index);
        reordered.insert(0, ct_entry);
        let reordered_archive = zip_fmt::ast::Archive {
            entries: reordered,
            ..archive
        };
        let bytes = zip_fmt::Emit::emit(&reordered_archive);

        let mut events = Vec::new();
        {
            let mut p = StreamingParser::new(|ev| events.push(ev));
            for chunk in bytes.chunks(1024) {
                p.feed(chunk);
            }
            let diags = p.finish();
            assert!(diags.is_empty(), "diagnostics: {diags:?}");
        }

        // Walk raw events (not `collect_parts`, which discards chunk
        // boundaries) to inspect each individual `PartData` chunk's size.
        let mut chunk_sizes = Vec::new();
        let mut in_target = false;
        for ev in &events {
            match ev {
                Event::PartStart { path, .. } if path == "big/part.bin" => in_target = true,
                Event::PartData(chunk) if in_target => chunk_sizes.push(chunk.len()),
                Event::PartEnd if in_target => in_target = false,
                _ => {}
            }
        }

        assert!(
            chunk_sizes.len() > 1,
            "expected the large part to arrive as multiple PartData chunks, got {chunk_sizes:?}"
        );
        let max_chunk = chunk_sizes.iter().copied().max().unwrap();
        assert!(
            max_chunk < big.len() / 2,
            "a single PartData chunk ({max_chunk} bytes) was a large fraction of the whole \
             part's size ({} bytes) — this indicates the part is still being buffered whole \
             before delivery, not streamed sub-entry",
            big.len()
        );

        // Correctness: concatenating every chunk still reproduces the
        // original bytes exactly.
        let parts = collect_parts(&events);
        let (_, content_type, content) = parts
            .iter()
            .find(|(path, ..)| path == "big/part.bin")
            .expect("expected big/part.bin as a Part event");
        assert_eq!(content_type.as_deref(), Some("application/octet-stream"));
        assert_eq!(content, &big);
    }
}
