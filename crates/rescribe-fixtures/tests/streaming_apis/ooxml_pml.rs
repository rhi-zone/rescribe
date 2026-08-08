//! Streaming-API cross-checks for ooxml_pml. Split out of the former monolithic
//! `streaming_apis.rs` (see `crates/rescribe-fixtures/tests/streaming_apis.rs`
//! for the harness overview and `common.rs` for shared helpers) so concurrent
//! per-format edits stop colliding on one file.

#[allow(unused_imports)]
use crate::common::{assert_streaming_parser_is_incremental, find_input, fixtures_root};
#[allow(unused_imports)]
use rescribe_fixtures::streaming_harness::{
    CAPABILITIES, NOT_YET_AUDITED, ObservableSink, adversarial_chunkings, assert_or_known_failure,
};
#[allow(unused_imports)]
use rescribe_format_api::{Emit, Events, Handler, Parse, StreamingParse, StreamingWrite};
#[allow(unused_imports)]
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// ooxml-pml (pptx) events(): txBody unreachable — fixed.
//
// Two independent gaps compounded to drop all slide text: (1)
// `dispatch_start()` had no entry for `<p:txBody>`, `<p:sld>`, `<p:cSld>`, or
// `<p:spTree>` — none of which have a dedicated `PmlEvent` of their own, they
// just wrap tracked content — so they fell into `skip_element()` and the
// whole subtree (every shape on the slide) was silently dropped; and (2)
// `read_props`/`read_shape_transform`'s own scan-ahead loops shared
// ooxml-wml's queue()-overwrite / frame-push-order bugs (see the docx test
// above), which would have re-broken text/nesting even once txBody became
// reachable. Fixed by adding an `is_transparent_wrapper` descend-without-
// emitting path (mirroring ooxml-wml's) for the four wrapper elements, and
// porting the same push-before-build / prepend-not-overwrite fixes to
// `ooxml-pml/src/events.rs`. `read_shape_transform` (which scans a `<p:sp>`
// for `<p:spPr>`) is included: schema-valid content can omit `<p:spPr>`
// entirely, and the fixture below intentionally does, so a shape with no
// `<p:spPr>` must not have its `<p:txBody>` skipped by the shape-transform
// scan either.
// ---------------------------------------------------------------------------

const PML_SIMPLE_SLIDE: &[u8] = br#"<?xml version="1.0"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
<p:cSld><p:spTree>
<p:sp><p:txBody><a:p><a:r><a:t>Hello world</a:t></a:r></a:p></p:txBody></p:sp>
</p:spTree></p:cSld>
</p:sld>"#;

#[test]
fn pml_events_reaches_slide_text() {
    let events: Vec<_> = ooxml_pml::events::events(PML_SIMPLE_SLIDE).collect();
    let has_text = events
        .iter()
        .any(|e| format!("{e:?}").contains("Hello world"));

    let well_nested = {
        // EndRun before EndParagraph before EndShape: each closes the
        // container opened most recently, innermost first.
        let idx = |pred: fn(&ooxml_pml::PmlEvent) -> bool| events.iter().position(|e| pred(e));
        let end_run = idx(|e| matches!(e, ooxml_pml::PmlEvent::EndRun));
        let end_para = idx(|e| matches!(e, ooxml_pml::PmlEvent::EndParagraph));
        let end_shape = idx(|e| matches!(e, ooxml_pml::PmlEvent::EndShape));
        matches!((end_run, end_para, end_shape), (Some(r), Some(p), Some(s)) if r < p && p < s)
    };

    let result = if has_text && well_nested {
        Ok(())
    } else {
        Err(format!(
            "expected an event carrying \"Hello world\" text from the slide's txBody, and \
             EndRun before EndParagraph before EndShape; got {} events: {events:?}",
            events.len()
        ))
    };
    assert_or_known_failure("pptx", "events", result);
}

/// Schema-typical shape: `<p:spPr>` (with `<a:xfrm>` position/size and
/// `<a:prstGeom>` outline) present *before* `<p:txBody>`, matching what real
/// PowerPoint output actually emits (verified against
/// `fixtures/pptx/slide/input.pptx`'s `ppt/slides/slide1.xml` — byte for
/// byte no whitespace between sibling tags, which is why the `<p:sp>`
/// subtree here is kept on one line too: whitespace text nodes between
/// siblings hit a separate, pre-existing `read_props`/`read_shape_transform`
/// fragility — an early return on any text node, even whitespace-only, once
/// it's scanning ahead for a props element — that real PowerPoint/Word
/// output never triggers and is out of scope for the txBody-reachability fix
/// this fixture targets).  Exercises the `read_shape_transform` →
/// `<p:spPr>`-found path specifically, as opposed to `PML_SIMPLE_SLIDE`
/// above which exercises the no-`<p:spPr>` path.
const PML_SLIDE_WITH_SPPR: &[u8] = br#"<?xml version="1.0"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
<p:cSld><p:spTree><p:sp><p:nvSpPr><p:cNvPr id="2" name="Title"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="457200" y="274638"/><a:ext cx="8229600" cy="1143000"/></a:xfrm><a:prstGeom prst="rect"/></p:spPr><p:txBody><a:bodyPr/><a:p><a:r><a:rPr lang="en-US" sz="4400"/><a:t>Slide Title</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld>
</p:sld>"#;

#[test]
fn pml_events_reaches_slide_text_with_sppr_present() {
    let events: Vec<_> = ooxml_pml::events::events(PML_SLIDE_WITH_SPPR).collect();
    let has_text = events
        .iter()
        .any(|e| format!("{e:?}").contains("Slide Title"));
    let has_transform = events.iter().any(
        |e| matches!(e, ooxml_pml::PmlEvent::StartShape { transform: Some(t), .. } if t.x == 457200 && t.cx == 8229600),
    );
    assert!(
        has_text && has_transform,
        "expected \"Slide Title\" text and a StartShape transform with x=457200, cx=8229600; \
         got {events:?}"
    );
}

// ---------------------------------------------------------------------------
// ooxml-pml (pptx) streaming writer: real, passing fidelity check
// ---------------------------------------------------------------------------
//
// `PmlWriter::finish()` produces a complete PPTX zip package (content types,
// rels, presentation part, slide masters/layouts, slide parts — not a bare
// XML fragment), so like `SmlWriter` (see `ooxml_sml.rs`'s
// `sml_streaming_writer_preserves_row_and_cell_attributes`) this cannot be
// compared byte-for-byte against a builder; instead this extracts each
// `ppt/slides/slideN.xml` part and checks the text made it through, on both
// the single-slide default path and multi-slide `new_slide()` boundaries —
// the two writer capabilities TODO.md's pml-writer checklist marks done
// ("multi-slide support via `new_slide()`") independent of the still-open
// shape-geometry item (untouched here; these events carry no transform/
// geometry, exercising the `None`/`None` default-layout path only).
struct SharedSink(std::rc::Rc<std::cell::RefCell<Vec<u8>>>, u64);

impl std::io::Write for SharedSink {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        let mut v = self.0.borrow_mut();
        let pos = self.1 as usize;
        if v.len() < pos + data.len() {
            v.resize(pos + data.len(), 0);
        }
        v[pos..pos + data.len()].copy_from_slice(data);
        self.1 += data.len() as u64;
        Ok(data.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl std::io::Seek for SharedSink {
    fn seek(&mut self, from: std::io::SeekFrom) -> std::io::Result<u64> {
        let len = self.0.borrow().len() as u64;
        self.1 = match from {
            std::io::SeekFrom::Start(n) => n,
            std::io::SeekFrom::End(n) => (len as i64 + n) as u64,
            std::io::SeekFrom::Current(n) => (self.1 as i64 + n) as u64,
        };
        Ok(self.1)
    }
}

#[test]
fn pml_streaming_writer_preserves_multi_slide_text() {
    use ooxml_pml::{PmlEvent, PmlWriter};
    use std::io::{Cursor, Read};

    let buf = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
    let mut writer = PmlWriter::new(SharedSink(buf.clone(), 0));

    let write_text_shape = |writer: &mut PmlWriter<SharedSink>, text: &str| {
        writer.write_event(PmlEvent::StartShape {
            transform: None,
            geometry: None,
        });
        writer.write_event(PmlEvent::StartParagraph {
            props: Box::default(),
        });
        writer.write_event(PmlEvent::StartRun {
            props: Box::default(),
        });
        writer.write_event(PmlEvent::Text(text.to_string().into()));
        writer.write_event(PmlEvent::EndRun);
        writer.write_event(PmlEvent::EndParagraph);
        writer.write_event(PmlEvent::EndShape);
    };

    writer.write_event(PmlEvent::StartPresentation);
    write_text_shape(&mut writer, "First slide text");
    writer.new_slide();
    write_text_shape(&mut writer, "Second slide text");
    writer.write_event(PmlEvent::EndPresentation);

    writer.finish().expect("PmlWriter::finish");
    let pptx = buf.borrow().clone();

    let mut zip = zip::ZipArchive::new(Cursor::new(pptx)).expect("valid zip package");
    let mut read_part = |name: &str| -> String {
        let mut s = String::new();
        zip.by_name(name)
            .unwrap_or_else(|e| panic!("part {name} present: {e}"))
            .read_to_string(&mut s)
            .unwrap_or_else(|e| panic!("read part {name}: {e}"));
        s
    };

    let slide1 = read_part("ppt/slides/slide1.xml");
    let slide2 = read_part("ppt/slides/slide2.xml");

    assert!(
        slide1.contains("First slide text"),
        "slide1.xml missing its shape's text: {slide1}"
    );
    assert!(
        !slide1.contains("Second slide text"),
        "slide1.xml unexpectedly contains slide 2's text: {slide1}"
    );
    assert!(
        slide2.contains("Second slide text"),
        "new_slide() boundary lost — slide2.xml missing its shape's text: {slide2}"
    );
    assert!(
        !slide2.contains("First slide text"),
        "slide2.xml unexpectedly contains slide 1's text: {slide2}"
    );
}
