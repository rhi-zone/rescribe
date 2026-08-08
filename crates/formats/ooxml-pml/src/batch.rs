//! Chunk-driven readers for PPTX presentations: [`BatchParser`] (buffer-all,
//! parse-on-finish) and [`StreamingParser`] (chunk-fed, `O(nesting depth +
//! largest token)`).
//!
//! # `BatchParser`: buffer-all
//!
//! Accepts arbitrary-sized chunks via [`BatchParser::feed`] and parses the
//! complete presentation on [`BatchParser::finish`]. All chunks are buffered
//! until `finish()` is called.
//!
//! ```ignore
//! use ooxml_pml::BatchParser;
//!
//! let mut parser = BatchParser::new();
//! for chunk in file_stream {
//!     parser.feed(&chunk);
//! }
//! let presentation = parser.finish()?;
//! ```
//!
//! # `StreamingParser<H>`: genuinely incremental
//!
//! Driven by [`ooxml_opc::StreamingParser`] for container-level (ZIP/OPC)
//! entry delivery, translating each `ppt/slides/slideN.xml` part into the
//! same [`crate::PmlEvent`] stream [`crate::events::events`] already
//! produces for a single slide's bytes — the two APIs share that per-part
//! SAX state machine as a plain function call, not by one being derived
//! from the other (see the crate root's / `CLAUDE.md`'s "three independent
//! implementations" rule).
//!
//! ```ignore
//! use ooxml_pml::batch::StreamingParser;
//!
//! let mut events = Vec::new();
//! let mut p = StreamingParser::new(|ev| events.push(ev));
//! for chunk in pptx_bytes.chunks(4096) {
//!     p.feed(chunk);
//! }
//! let diagnostics = p.finish();
//! ```
//!
//! ## Slide ordering
//!
//! PPTX slide display order is defined by `ppt/presentation.xml`'s
//! `<p:sldIdLst>` (a list of `r:id` values), resolved against
//! `ppt/presentation.xml`'s own relationships part — **not** by ZIP entry
//! order or by `slideN.xml` filename order (neither is guaranteed to match
//! display order; see [`crate::presentation::parse_presentation_slides`]).
//! [`StreamingParser`] reuses the exact same resolution [`Presentation::open`]
//! uses (root `.rels` → presentation part → `<p:sldIdLst>` → presentation
//! part's own `.rels` → slide part paths), reimplemented here only in how
//! the inputs arrive (pushed by [`ooxml_opc::StreamingParser`] instead of
//! pulled via seekable `Package` reads).
//!
//! ## Memory model and buffering
//!
//! **Update (2026-08-08):** `ooxml-opc`'s `Event::Part` no longer delivers
//! a part's full decompressed bytes in one shot — it is now
//! `PartStart`/`PartData`/`PartEnd`, with `PartData` chunks forwarded as
//! they decompress once `[Content_Types].xml` has streamed past (see its
//! own module docs). This module takes advantage of that directly for one
//! case: once presentation/slide-path resolution has completed (see
//! exception 1 below), a part that resolution says is neither the
//! presentation part nor a slide part is now dropped **as its `PartData`
//! chunks arrive**, never accumulated even transiently — a real
//! improvement over the pre-2026-08-08 behavior, where `ooxml-opc` fully
//! buffered every part (including ones this module immediately discarded)
//! before this module ever saw it.
//!
//! Two scoped buffering exceptions remain, both orthogonal to the
//! `ooxml-opc` container-layer change above (they exist because of what
//! this module's own resolution/ordering logic needs, not because of how
//! `ooxml-opc` delivers bytes) and both still needing a part's complete
//! bytes at the point they're held — individual slide XML parts are
//! typically small, so this is judged acceptable for `pml` even though it
//! would not be for `wml`/`sml`'s much larger single-part documents:
//!
//! 1. **Pre-resolution part buffering.** Any part whose classification
//!    (presentation part / slide part / irrelevant) isn't yet knowable —
//!    because the root relationships, the presentation part's own
//!    relationships, or `<p:sldIdLst>` haven't streamed past yet — has its
//!    `PartData` chunks accumulated and held in `pending_parts` until
//!    classification becomes possible, then reclassified (kept if it's the
//!    presentation part or a slide part, dropped otherwise). Bounded by
//!    total bytes of parts preceding full resolution, not the full
//!    archive; real PPTX packages conventionally put `_rels/.rels`,
//!    `ppt/presentation.xml`, and its `.rels` early.
//! 2. **Out-of-slide-order buffering.** Once slide paths are known in
//!    display order, a slide part that streams past before the slide(s)
//!    ahead of it in display order has its `PartData` chunks accumulated
//!    and held in `slide_content` until its turn. Bounded by total bytes
//!    of slides that arrive before the currently-awaited one — worst case
//!    the whole deck, if ZIP entry order happens to be the reverse of
//!    display order, but real PPTX packages conventionally write slide
//!    parts in display order.
//!
//! Neither exception applies to non-presentation, non-slide parts (styles,
//! media, masters, layouts, notes, etc.) once resolution has completed —
//! those are classified from `PartStart` alone and their `PartData`
//! dropped immediately, never buffered at all.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::Result;
use crate::presentation::Presentation;
use std::io::Cursor;

/// Chunk-driven PPTX parser: buffers all input, parses on `finish()`.
#[derive(Default)]
pub struct BatchParser {
    buf: Vec<u8>,
}

impl BatchParser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn feed(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);
    }

    pub fn finish(self) -> Result<Presentation<Cursor<Vec<u8>>>> {
        Presentation::from_reader(Cursor::new(self.buf))
    }
}

// ---------------------------------------------------------------------------
// StreamingParser<H>
// ---------------------------------------------------------------------------

use crate::events;
use crate::generated_events::OwnedPmlEvent;
use crate::presentation::{
    REL_OFFICE_DOCUMENT, REL_SLIDE, parse_presentation_slides, resolve_path,
};
use ooxml_opc::Relationships;
pub use rescribe_format_api::Handler;
use rescribe_format_api::{Diagnostic, Severity};

struct Shared<H: Handler<OwnedPmlEvent>> {
    handler: H,
    diagnostics: Vec<Diagnostic>,

    root_rels: Option<Relationships>,
    presentation_path: Option<String>,
    /// All `.rels` parts seen so far, keyed by the OPC part they apply to
    /// (`ooxml_opc::StreamingEvent::Relationships`'s own `part_path`).
    rels_by_owner: HashMap<String, Relationships>,
    /// `<p:sldIdLst>`'s `r:id` values, in display order — `None` until the
    /// presentation part's own bytes have streamed past.
    slide_order: Option<Vec<String>>,
    /// Slide part paths in display order, resolved once `presentation_path`,
    /// `rels_by_owner[presentation_path]`, and `slide_order` are all known.
    slide_paths: Option<Vec<String>>,

    /// Parts whose classification isn't yet possible — see module docs,
    /// buffering exception 1.
    pending_parts: Vec<(String, Vec<u8>)>,
    /// Slide parts already classified but not yet at the front of display
    /// order — see module docs, buffering exception 2.
    slide_content: HashMap<String, Vec<u8>>,
    /// Index into `slide_paths` of the next slide to emit.
    next_slide_idx: usize,
}

impl<H: Handler<OwnedPmlEvent>> Shared<H> {
    /// Emit one slide's events: run the same per-part SAX iterator
    /// [`crate::events::events`] uses for a single slide, converting each
    /// borrowed event to owned before handing it to the handler (mirrors
    /// every other migrated crate's `StreamingParser<H: Handler<OwnedEvent>>`
    /// contract).
    fn emit_slide(&mut self, bytes: &[u8]) {
        for event in events::events(bytes) {
            self.handler.handle(event.into_owned());
        }
    }

    /// Drain `slide_content` in display order for as long as the
    /// currently-awaited slide is available.
    fn emit_ready_slides(&mut self) {
        let Some(paths) = self.slide_paths.clone() else {
            return;
        };
        while self.next_slide_idx < paths.len() {
            let path = &paths[self.next_slide_idx];
            let Some(content) = self.slide_content.remove(path) else {
                break;
            };
            self.emit_slide(&content);
            self.next_slide_idx += 1;
        }
    }

    /// Resolve `presentation_path` → `slide_order` → `pres_rels` →
    /// `slide_paths`, as far as currently possible, then reclassify any
    /// parts still in `pending_parts` and emit any now-ready slides.
    fn advance(&mut self) {
        if self.presentation_path.is_none()
            && let Some(root_rels) = &self.root_rels
            && let Some(rel) = root_rels.get_by_type(REL_OFFICE_DOCUMENT)
        {
            // Root `.rels` targets are relative to the package root already
            // (OPC spec) — no `resolve_path` needed, matching
            // `Presentation::from_reader`'s identical, unresolved use of
            // `pres_rel.target.clone()`.
            self.presentation_path = Some(rel.target.clone());
        }

        if let Some(presentation_path) = self.presentation_path.clone() {
            if self.slide_order.is_none()
                && let Some(idx) = self
                    .pending_parts
                    .iter()
                    .position(|(p, _)| p == &presentation_path)
            {
                let (_, content) = self.pending_parts.remove(idx);
                match parse_presentation_slides(&content) {
                    Ok(order) => self.slide_order = Some(order),
                    Err(e) => self.diagnostics.push(Diagnostic::new(
                        Severity::Warning,
                        format!("failed to parse <p:sldIdLst> from {presentation_path}: {e}"),
                    )),
                }
            }

            if let (Some(slide_order), Some(pres_rels)) = (
                self.slide_order.clone(),
                self.rels_by_owner.get(&presentation_path).cloned(),
            ) && self.slide_paths.is_none()
            {
                let slide_paths: Vec<String> = slide_order
                    .iter()
                    .filter_map(|rid| {
                        let rel = pres_rels.get(rid)?;
                        if rel.relationship_type != REL_SLIDE {
                            return None;
                        }
                        Some(resolve_path(&presentation_path, &rel.target))
                    })
                    .collect();
                self.slide_paths = Some(slide_paths);
            }
        }

        if let Some(slide_paths) = self.slide_paths.clone() {
            let pending = std::mem::take(&mut self.pending_parts);
            for (path, content) in pending {
                if slide_paths.contains(&path) {
                    self.slide_content.insert(path, content);
                }
                // Else: not the presentation part (already drained above if
                // it matched), not a slide part — irrelevant, dropped.
            }
            self.emit_ready_slides();
        }
    }

    fn on_part(&mut self, path: String, content: Vec<u8>) {
        if let Some(slide_paths) = &self.slide_paths {
            if slide_paths.contains(&path) {
                self.slide_content.insert(path, content);
                self.emit_ready_slides();
                return;
            }
            if self.presentation_path.as_deref() == Some(path.as_str()) {
                // Already resolved via an earlier pending entry; a second
                // delivery of the same part shouldn't occur, but drop
                // rather than reprocess if it does.
                return;
            }
            // Irrelevant part (styles, media, masters, layouts, notes, ...).
            return;
        }

        if self.presentation_path.as_deref() == Some(path.as_str()) {
            match parse_presentation_slides(&content) {
                Ok(order) => self.slide_order = Some(order),
                Err(e) => self.diagnostics.push(Diagnostic::new(
                    Severity::Warning,
                    format!("failed to parse <p:sldIdLst> from {path}: {e}"),
                )),
            }
            self.advance();
            return;
        }

        // Classification not yet possible — hold for reclassification once
        // more relationship/order information arrives.
        self.pending_parts.push((path, content));
    }

    fn on_relationships(&mut self, part_path: String, relationships: Relationships) {
        if part_path.is_empty() {
            self.root_rels = Some(relationships.clone());
        }
        self.rels_by_owner.insert(part_path, relationships);
        self.advance();
    }
}

/// Adapts `ooxml_opc::batch::Event`'s incremental part delivery to
/// [`Shared::on_part`]'s whole-part contract. Accumulates `PartData`
/// chunks into `buf` — *unless* `drop_immediately` says resolution has
/// already determined this part is neither the presentation part nor a
/// slide part, in which case chunks are discarded as they arrive without
/// ever touching `buf`. See the module docs' "Memory model and buffering"
/// section for why the other two buffering exceptions (pre-resolution,
/// out-of-order slides) still need a part's complete bytes once accepted.
struct InnerHandler<H: Handler<OwnedPmlEvent>> {
    shared: Rc<RefCell<Shared<H>>>,
    current_path: String,
    buf: Vec<u8>,
    drop_immediately: bool,
}

impl<H: Handler<OwnedPmlEvent>> Handler<ooxml_opc::StreamingEvent> for InnerHandler<H> {
    fn handle(&mut self, event: ooxml_opc::StreamingEvent) {
        match event {
            ooxml_opc::StreamingEvent::ContentTypes(_) => {}
            ooxml_opc::StreamingEvent::Relationships {
                part_path,
                relationships,
            } => self
                .shared
                .borrow_mut()
                .on_relationships(part_path, relationships),
            ooxml_opc::StreamingEvent::PartStart { path, .. } => {
                self.buf.clear();
                let shared = self.shared.borrow();
                self.drop_immediately = match &shared.slide_paths {
                    Some(slide_paths) => {
                        !slide_paths.contains(&path)
                            && shared.presentation_path.as_deref() != Some(path.as_str())
                    }
                    // Resolution not yet complete — classification isn't
                    // knowable yet, so this part must still be accumulated
                    // (module docs, exception 1).
                    None => false,
                };
                self.current_path = path;
            }
            ooxml_opc::StreamingEvent::PartData(chunk) => {
                if !self.drop_immediately {
                    self.buf.extend_from_slice(&chunk);
                }
            }
            ooxml_opc::StreamingEvent::PartEnd => {
                if self.drop_immediately {
                    self.buf.clear();
                    return;
                }
                let path = std::mem::take(&mut self.current_path);
                let content = std::mem::take(&mut self.buf);
                self.shared.borrow_mut().on_part(path, content);
            }
        }
    }
}

/// Genuinely incremental, chunk-fed PPTX reader. See the module docs for the
/// slide-ordering rule and the buffering contract. Additive alongside
/// [`BatchParser`] and [`Presentation`] — does not replace either.
pub struct StreamingParser<H: Handler<OwnedPmlEvent>> {
    inner: ooxml_opc::StreamingParser<InnerHandler<H>>,
    shared: Rc<RefCell<Shared<H>>>,
}

impl<H: Handler<OwnedPmlEvent>> StreamingParser<H> {
    pub fn new(handler: H) -> Self {
        let shared = Rc::new(RefCell::new(Shared {
            handler,
            diagnostics: Vec::new(),
            root_rels: None,
            presentation_path: None,
            rels_by_owner: HashMap::new(),
            slide_order: None,
            slide_paths: None,
            pending_parts: Vec::new(),
            slide_content: HashMap::new(),
            next_slide_idx: 0,
        }));
        let inner_handler = InnerHandler {
            shared: shared.clone(),
            current_path: String::new(),
            buf: Vec::new(),
            drop_immediately: false,
        };
        StreamingParser {
            inner: ooxml_opc::StreamingParser::new(inner_handler),
            shared,
        }
    }

    /// Feed the next chunk of archive bytes. May be called with chunks of
    /// any size, including 1 byte, and any number of times.
    pub fn feed(&mut self, chunk: &[u8]) {
        self.inner.feed(chunk);
    }

    /// Signal end of input. Returns accumulated diagnostics — including a
    /// warning if slide resolution never completed, or if some declared
    /// slides never streamed past (e.g. truncated input).
    pub fn finish(self) -> Vec<Diagnostic> {
        let opc_diags = self.inner.finish();
        let mut shared = Rc::try_unwrap(self.shared)
            .unwrap_or_else(|_| {
                panic!("ooxml-pml StreamingParser: internal state still referenced after finish")
            })
            .into_inner();
        shared.diagnostics.extend(opc_diags);
        shared.advance();

        match &shared.slide_paths {
            None => shared.diagnostics.push(Diagnostic::new(
                Severity::Warning,
                "presentation.xml, its relationships, or <p:sldIdLst> could not be resolved; \
                 no slide events were emitted"
                    .to_string(),
            )),
            Some(paths) if shared.next_slide_idx < paths.len() => {
                shared.diagnostics.push(Diagnostic::new(
                    Severity::Warning,
                    format!(
                        "{} of {} declared slide(s) never streamed past in the archive; their \
                         events were not emitted",
                        paths.len() - shared.next_slide_idx,
                        paths.len()
                    ),
                ));
            }
            Some(_) => {}
        }

        shared.diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated_events::PmlEvent;
    use ooxml_opc::PackageWriter;
    use std::io::Cursor;

    const CT_RELATIONSHIPS: &str = "application/vnd.openxmlformats-package.relationships+xml";
    const CT_XML: &str = "application/xml";
    const CT_PRESENTATION: &str =
        "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml";
    const CT_SLIDE: &str = "application/vnd.openxmlformats-officedocument.presentationml.slide+xml";

    /// Deliberately single-line, no inter-element whitespace — real PPTX
    /// parts are written this way (not pretty-printed), and `events()`'s
    /// top-level SAX loop (unlike its props-lookahead helpers) does not
    /// trim whitespace-only text nodes, so a pretty-printed fixture here
    /// would produce spurious whitespace `Text` events the real format
    /// never does.
    fn slide_xml(text: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/><p:sp><p:nvSpPr><p:cNvPr id="2" name="TextBox"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>{text}</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:sld>"#
        )
    }

    /// Build a minimal PPTX with `n` slides, each holding one text run
    /// `"slide{i}"`. `write_order` gives the ZIP-entry order to add the
    /// slide parts in (as slide indices `0..n`) — independent of
    /// `sld_id_order`, the `<p:sldIdLst>` order (also slide indices), which
    /// is the actual display order a correct reader must follow.
    fn build_test_pptx(n: usize, sld_id_order: &[usize], write_order: &[usize]) -> Vec<u8> {
        assert_eq!(sld_id_order.len(), n);

        let mut buf = Cursor::new(Vec::new());
        {
            let mut pkg = PackageWriter::new(&mut buf);
            pkg.add_default_content_type("rels", CT_RELATIONSHIPS);
            pkg.add_default_content_type("xml", CT_XML);

            let root_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#;
            pkg.add_part("_rels/.rels", CT_RELATIONSHIPS, root_rels.as_bytes())
                .unwrap();

            // Slide relationship IDs: rId(10 + slide index), arbitrary but
            // distinct from the root rel's rId1.
            let sld_id_list: String = sld_id_order
                .iter()
                .map(|&i| format!(r#"<p:sldId id="{}" r:id="rId{}"/>"#, 256 + i, 10 + i))
                .collect();
            let presentation_xml = format!(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:sldIdLst>{sld_id_list}</p:sldIdLst>
</p:presentation>"#
            );
            pkg.add_part(
                "ppt/presentation.xml",
                CT_PRESENTATION,
                presentation_xml.as_bytes(),
            )
            .unwrap();

            let pres_rels: String = {
                let mut s = String::from(
                    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
"#,
                );
                for i in 0..n {
                    s.push_str(&format!(
                        r#"  <Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide{}.xml"/>
"#,
                        10 + i,
                        i + 1
                    ));
                }
                s.push_str("</Relationships>");
                s
            };
            pkg.add_part(
                "ppt/_rels/presentation.xml.rels",
                CT_RELATIONSHIPS,
                pres_rels.as_bytes(),
            )
            .unwrap();

            for &i in write_order {
                let xml = slide_xml(&format!("slide{i}"));
                pkg.add_part(
                    &format!("ppt/slides/slide{}.xml", i + 1),
                    CT_SLIDE,
                    xml.as_bytes(),
                )
                .unwrap();
            }

            pkg.finish().unwrap();
        }
        buf.into_inner()
    }

    /// Collect the `Text` payloads emitted by a `StreamingParser` run, fed
    /// in small, uneven chunks (mirrors `ooxml-opc`'s own adversarial chunk
    /// tests) — one `Vec<String>` entry per slide's worth of text
    /// (`events()`'s per-slide `StartPresentation..EndPresentation`
    /// bookends demarcate slide boundaries; see module docs).
    fn run_streaming(bytes: &[u8]) -> (Vec<String>, Vec<Diagnostic>) {
        let mut texts = Vec::new();
        let mut p = StreamingParser::new(|ev: PmlEvent<'static>| {
            if let PmlEvent::Text(t) = ev {
                texts.push(t.into_owned());
            }
        });
        for chunk in bytes.chunks(7) {
            p.feed(chunk);
        }
        let diags = p.finish();
        (texts, diags)
    }

    #[test]
    fn streaming_emits_slides_in_write_order_when_already_in_display_order() {
        let bytes = build_test_pptx(3, &[0, 1, 2], &[0, 1, 2]);
        let (texts, diags) = run_streaming(&bytes);
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert_eq!(texts, vec!["slide0", "slide1", "slide2"]);
    }

    /// The core correctness point: slide *display* order comes from
    /// `<p:sldIdLst>`, not ZIP entry order. Slides are written to the
    /// archive in reverse of their `<p:sldIdLst>` order; a reader that
    /// (incorrectly) emitted events in ZIP-arrival order would produce
    /// `["slide2", "slide1", "slide0"]`.
    #[test]
    fn streaming_slide_order_follows_sld_id_lst_not_zip_order() {
        let bytes = build_test_pptx(3, &[0, 1, 2], &[2, 1, 0]);
        let (texts, diags) = run_streaming(&bytes);
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert_eq!(texts, vec!["slide0", "slide1", "slide2"]);
    }

    /// `<p:sldIdLst>` itself may list slides in an order different from
    /// their `slideN.xml` filenames (e.g. after reordering slides in the
    /// authoring tool without renaming parts) — display order must follow
    /// the list, not the filename-implied numeric order.
    #[test]
    fn streaming_slide_order_follows_sld_id_lst_not_filename_order() {
        let bytes = build_test_pptx(3, &[2, 0, 1], &[0, 1, 2]);
        let (texts, diags) = run_streaming(&bytes);
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert_eq!(texts, vec!["slide2", "slide0", "slide1"]);
    }

    #[test]
    fn streaming_matches_ast_reader_content() {
        let bytes = build_test_pptx(2, &[0, 1], &[0, 1]);

        let mut pres =
            crate::presentation::Presentation::from_reader(Cursor::new(bytes.clone())).unwrap();
        let slides = pres.slides().unwrap();
        let expected: Vec<String> = slides.iter().map(|s| s.text().replace('\n', "")).collect();

        let (texts, diags) = run_streaming(&bytes);
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert_eq!(texts, expected);
    }

    #[test]
    fn streaming_no_panic_on_empty_and_truncated_and_garbage_input() {
        // Empty input.
        {
            let p = StreamingParser::new(|_: PmlEvent<'static>| {});
            let _ = p.finish();
        }

        // Truncated real archive, fed byte-at-a-time.
        let bytes = build_test_pptx(3, &[0, 1, 2], &[0, 1, 2]);
        let half = &bytes[..bytes.len() / 2];
        {
            let mut p = StreamingParser::new(|_: PmlEvent<'static>| {});
            for b in half {
                p.feed(std::slice::from_ref(b));
            }
            let _ = p.finish();
        }

        // Arbitrary non-ZIP garbage bytes.
        for seed in 0..8u8 {
            let garbage: Vec<u8> = (0..200u8)
                .map(|i| i.wrapping_mul(seed).wrapping_add(i))
                .collect();
            let mut p = StreamingParser::new(|_: PmlEvent<'static>| {});
            for chunk in garbage.chunks(3) {
                p.feed(chunk);
            }
            let _ = p.finish();
        }
    }

    #[test]
    fn streaming_reports_missing_slide_when_never_streamed() {
        // Presentation declares 2 slides but only 1 is actually in the
        // archive (simulates truncation losing a slide part).
        let bytes = build_test_pptx(2, &[0, 1], &[0]);
        let (_, diags) = run_streaming(&bytes);
        assert!(
            diags.iter().any(|d| d.message.contains("never streamed")),
            "expected a missing-slide diagnostic, got: {diags:?}"
        );
    }
}
