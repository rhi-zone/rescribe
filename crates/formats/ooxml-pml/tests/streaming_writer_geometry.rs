//! Tests for shape geometry fidelity across the true `events()` SAX reader
//! and the event-driven `PmlWriter`.
//!
//! Before this fix, `PmlEvent::StartShape` only carried `ShapeTransform`
//! (bounding box position/size). It did not carry the shape's actual
//! outline (`<a:prstGeom>` preset + adjustment values, or `<a:custGeom>`
//! custom path), so:
//!
//! - The true SAX reader (`ooxml_pml::events`) never read `<a:prstGeom>`/
//!   `<a:custGeom>` at all — it stopped at `<a:xfrm>`.
//! - `PmlWriter` always emitted a plain `Rect` for every shape, regardless
//!   of what the source XML actually contained.
//!
//! Together this meant every shape read via `events()` and re-emitted by
//! `PmlWriter` silently became a plain rectangle — a semantic drop.
//!
//! Two things are tested separately:
//! 1. The reader (`events.rs`) now populates `ShapeGeometry` on
//!    `StartShape` from real slide XML.
//! 2. The writer (`streaming.rs`'s `PmlWriter`) now emits that geometry
//!    instead of hardcoding `Rect`, verified by reading the emitted `.pptx`
//!    back through the AST reader.
//!
//! These are split rather than driven end-to-end through `pml_events()` on
//! a full `<p:sp>` because the true SAX reader has a separate, pre-existing
//! gap unrelated to geometry: `dispatch_start` does not recognize
//! `<p:txBody>` as a transparent container, so `read_xml_info`'s fallback
//! (`skip_element` for any unrecognized `Start`) skips a shape's entire text
//! body — paragraphs and runs inside `<p:txBody>` are never reached. That
//! gap is documented in TODO.md rather than fixed here (fixing it is a
//! general `events.rs` rework, out of scope for the geometry fix).

use ooxml_dml::generated::EGGeometry;
use ooxml_pml::{PmlEvent, PmlWriter, Presentation, ShapeGeometry, ShapeTransform, pml_events};
use std::io::Cursor;

/// A single `<p:sp>` shape element with the given `<p:spPr>` inner XML
/// (xfrm + geometry), in the shape of the shape-tree content the true SAX
/// `events()` reader walks (`dispatch_start` recognizes `sp` as a
/// container and descends into it directly).
fn shape_xml(sp_pr_inner: &str) -> Vec<u8> {
    format!(
        r#"<p:sp xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
              xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:nvSpPr>
    <p:cNvPr id="2" name="Shape 1"/>
    <p:cNvSpPr/>
    <p:nvPr/>
  </p:nvSpPr>
  <p:spPr>
    {sp_pr_inner}
  </p:spPr>
</p:sp>"#
    )
    .into_bytes()
}

/// Run `xml` through the true SAX `events()` reader and return the
/// `(transform, geometry)` carried by its `StartShape` event.
fn read_shape_geometry(xml: &[u8]) -> (Option<ShapeTransform>, Option<ShapeGeometry>) {
    for event in pml_events(xml) {
        if let PmlEvent::StartShape {
            transform,
            geometry,
        } = event
        {
            return (transform, geometry);
        }
    }
    panic!("no StartShape event produced");
}

// ---------------------------------------------------------------------------
// 1. Reader: events.rs populates ShapeGeometry from real slide XML.
// ---------------------------------------------------------------------------

#[test]
fn events_reader_extracts_ellipse_preset() {
    let xml = shape_xml(
        r#"<a:xfrm><a:off x="914400" y="914400"/><a:ext cx="1828800" cy="1828800"/></a:xfrm>
           <a:prstGeom prst="ellipse"><a:avLst/></a:prstGeom>"#,
    );
    let (transform, geometry) = read_shape_geometry(&xml);

    let t = transform.expect("transform should be extracted");
    assert_eq!((t.x, t.y, t.cx, t.cy), (914400, 914400, 1828800, 1828800));

    match geometry {
        Some(ShapeGeometry::Preset {
            preset,
            adjustments,
        }) => {
            assert_eq!(preset, "ellipse");
            assert!(adjustments.is_empty());
        }
        other => panic!("expected Preset(ellipse), got {other:?}"),
    }
}

#[test]
fn events_reader_extracts_round_rect_adjustment_values() {
    let xml = shape_xml(
        r#"<a:xfrm><a:off x="0" y="0"/><a:ext cx="1000000" cy="500000"/></a:xfrm>
           <a:prstGeom prst="roundRect"><a:avLst><a:gd name="adj" fmla="val 8333"/></a:avLst></a:prstGeom>"#,
    );
    let (_, geometry) = read_shape_geometry(&xml);

    match geometry {
        Some(ShapeGeometry::Preset {
            preset,
            adjustments,
        }) => {
            assert_eq!(preset, "roundRect");
            assert_eq!(
                adjustments,
                vec![("adj".to_string(), "val 8333".to_string())],
                "adjustment values must be extracted, not dropped"
            );
        }
        other => panic!("expected Preset(roundRect) with adjustments, got {other:?}"),
    }
}

#[test]
fn events_reader_captures_cust_geom_verbatim() {
    let xml = shape_xml(
        r#"<a:xfrm><a:off x="0" y="0"/><a:ext cx="1000000" cy="1000000"/></a:xfrm>
           <a:custGeom>
             <a:avLst/>
             <a:gdLst/>
             <a:ahLst/>
             <a:cxnLst/>
             <a:rect l="0" t="0" r="0" b="0"/>
             <a:pathLst>
               <a:path w="1000000" h="1000000">
                 <a:moveTo><a:pt x="0" y="0"/></a:moveTo>
                 <a:lnTo><a:pt x="1000000" y="0"/></a:lnTo>
                 <a:lnTo><a:pt x="500000" y="1000000"/></a:lnTo>
                 <a:close/>
               </a:path>
             </a:pathLst>
           </a:custGeom>"#,
    );
    let (_, geometry) = read_shape_geometry(&xml);

    match geometry {
        Some(ShapeGeometry::Custom(raw)) => {
            assert_eq!(raw.name, "a:custGeom");
            let path_lst = raw
                .children
                .iter()
                .find_map(|c| match c {
                    ooxml_xml::RawXmlNode::Element(e) if e.name == "a:pathLst" => Some(e),
                    _ => None,
                })
                .expect("pathLst must be preserved, not dropped/replaced with Rect");
            let path = path_lst
                .children
                .iter()
                .find_map(|c| match c {
                    ooxml_xml::RawXmlNode::Element(e) if e.name == "a:path" => Some(e),
                    _ => None,
                })
                .expect("path must be preserved");
            let move_to_count = path
                .children
                .iter()
                .filter(|c| matches!(c, ooxml_xml::RawXmlNode::Element(e) if e.name == "a:moveTo"))
                .count();
            let ln_to_count = path
                .children
                .iter()
                .filter(|c| matches!(c, ooxml_xml::RawXmlNode::Element(e) if e.name == "a:lnTo"))
                .count();
            assert_eq!(move_to_count, 1);
            assert_eq!(ln_to_count, 2);
        }
        other => panic!(
            "expected Custom(custGeom) captured verbatim, got {other:?} -- \
             this is exactly the silent 'becomes a plain rectangle' bug"
        ),
    }
}

// ---------------------------------------------------------------------------
// 2. Writer: PmlWriter emits ShapeGeometry instead of hardcoding Rect.
// ---------------------------------------------------------------------------

fn write_shape_events(geometry: Option<ShapeGeometry>) -> Vec<u8> {
    let mut buf = Cursor::new(Vec::new());
    let mut writer = PmlWriter::new(&mut buf);
    writer.write_event(PmlEvent::StartPresentation);
    writer.write_event(PmlEvent::StartShape {
        transform: Some(ShapeTransform {
            x: 914400,
            y: 914400,
            cx: 1828800,
            cy: 1828800,
        }),
        geometry,
    });
    writer.write_event(PmlEvent::StartParagraph {
        props: Box::default(),
    });
    writer.write_event(PmlEvent::StartRun {
        props: Box::default(),
    });
    writer.write_event(PmlEvent::Text("hello".into()));
    writer.write_event(PmlEvent::EndRun);
    writer.write_event(PmlEvent::EndParagraph);
    writer.write_event(PmlEvent::EndShape);
    writer.write_event(PmlEvent::EndPresentation);
    writer.finish().expect("PmlWriter::finish should succeed");
    buf.into_inner()
}

fn first_shape_geometry(pptx_bytes: Vec<u8>) -> EGGeometry {
    let cursor = Cursor::new(pptx_bytes);
    let mut pres = Presentation::from_reader(cursor).expect("read back should succeed");
    let slide = pres.slide(0).expect("slide 0 should exist");
    let shapes = slide.shapes();
    assert_eq!(shapes.len(), 1, "expected exactly one shape");
    *shapes[0]
        .shape_properties
        .geometry
        .clone()
        .expect("shape should have geometry")
}

#[test]
fn pml_writer_emits_no_geometry_as_default_rect() {
    // Baseline: with no geometry on the event, the writer's documented
    // default (Rect) is still what comes out.
    let pptx = write_shape_events(None);
    match first_shape_geometry(pptx) {
        EGGeometry::PrstGeom(p) => {
            assert_eq!(p.preset, ooxml_dml::generated::STShapeType::Rect);
        }
        other => panic!("expected default Rect, got {other:?}"),
    }
}

#[test]
fn pml_writer_emits_ellipse_preset_not_rect() {
    let pptx = write_shape_events(Some(ShapeGeometry::Preset {
        preset: "ellipse".to_string(),
        adjustments: Vec::new(),
    }));
    match first_shape_geometry(pptx) {
        EGGeometry::PrstGeom(p) => {
            assert_eq!(
                p.preset,
                ooxml_dml::generated::STShapeType::Ellipse,
                "PmlWriter must emit the shape's actual geometry, not hardcode Rect"
            );
        }
        other => panic!("expected PrstGeom(ellipse), got {other:?}"),
    }
}

#[test]
fn pml_writer_emits_round_rect_adjustment_values() {
    let pptx = write_shape_events(Some(ShapeGeometry::Preset {
        preset: "roundRect".to_string(),
        adjustments: vec![("adj".to_string(), "val 8333".to_string())],
    }));
    match first_shape_geometry(pptx) {
        EGGeometry::PrstGeom(p) => {
            assert_eq!(p.preset, ooxml_dml::generated::STShapeType::RoundRect);
            let av_lst = p
                .av_lst
                .as_ref()
                .expect("adjustment values must be emitted, not dropped");
            assert_eq!(av_lst.gd.len(), 1);
            assert_eq!(av_lst.gd[0].name, "adj");
            assert_eq!(av_lst.gd[0].fmla, "val 8333");
        }
        other => panic!("expected PrstGeom(roundRect) with avLst, got {other:?}"),
    }
}

#[test]
fn pml_writer_emits_cust_geom() {
    // NOTE: `<a:moveTo>`/`<a:lnTo>` here carry `x`/`y` directly rather than
    // real ECMA-376's nested `<a:pt x=".." y=".."/>` child, because
    // ooxml-dml's generated `CTAdjPoint2D::from_xml`/`ToXml` for
    // `Path2DMoveToElement`/`Path2DLineToElement` reads/writes those
    // attributes on the `moveTo`/`lnTo` element itself (see
    // `ooxml_dml::generated_parsers::CTPath2D::from_xml`), not on a nested
    // `<a:pt>`. That is a pre-existing, separate bug in ooxml-dml's codegen
    // output (real PPTX files always nest `<a:pt>`), tracked in TODO.md —
    // out of scope for this fix. This test uses the shape ooxml-dml's own
    // parser/serializer pair actually agree on, so it isolates and proves
    // *this crate's* `PmlEvent`/`PmlWriter` geometry plumbing is correct.
    // `pml_writer_falls_back_gracefully_on_unparseable_cust_geom` below
    // covers the real-shaped-XML case, which currently degrades instead of
    // silently mis-rendering.
    let raw_xml = br#"<a:custGeom><a:avLst/><a:gdLst/><a:ahLst/><a:cxnLst/><a:rect l="0" t="0" r="0" b="0"/><a:pathLst><a:path w="1000000" h="1000000"><a:moveTo x="0" y="0"/><a:lnTo x="1000000" y="0"/><a:lnTo x="500000" y="1000000"/><a:close/></a:path></a:pathLst></a:custGeom>"#;
    let mut reader = quick_xml::Reader::from_reader(raw_xml.as_slice());
    let mut buf = Vec::new();
    let raw = match reader.read_event_into(&mut buf).unwrap() {
        quick_xml::events::Event::Start(e) => {
            ooxml_xml::RawXmlElement::from_reader(&mut reader, &e).unwrap()
        }
        _ => panic!("expected custGeom start"),
    };

    let pptx = write_shape_events(Some(ShapeGeometry::Custom(raw)));
    match first_shape_geometry(pptx) {
        EGGeometry::CustGeom(c) => {
            assert_eq!(
                c.path_lst.path.len(),
                1,
                "custGeom must be emitted verbatim, not replaced with Rect"
            );
            assert_eq!(c.path_lst.path[0].move_to.len(), 1);
            assert_eq!(c.path_lst.path[0].ln_to.len(), 2);
            assert_eq!(c.path_lst.path[0].close.len(), 1);
        }
        other => panic!("expected CustGeom, got {other:?}"),
    }
}

/// Real ECMA-376-shaped `custGeom` (nested `<a:pt>` inside `moveTo`/`lnTo`,
/// as PowerPoint actually writes) currently fails to re-parse into
/// `CTCustomGeometry2D` via `RawXmlElement::parse_as`, due to the
/// pre-existing ooxml-dml bug documented above. Per CLAUDE.md, a construct
/// that cannot be modeled must not be silently dropped without a trace: the
/// writer falls back to the default `Rect` (still keeping the shape's text)
/// rather than emitting corrupt or nonsensical geometry. This test locks in
/// that fallback is graceful (no panic, text preserved) so the gap stays
/// visible instead of silently regressing into a crash or lost text.
#[test]
fn pml_writer_falls_back_gracefully_on_unparseable_cust_geom() {
    let raw_xml = br#"<a:custGeom><a:avLst/><a:gdLst/><a:ahLst/><a:cxnLst/><a:rect l="0" t="0" r="0" b="0"/><a:pathLst><a:path w="1000000" h="1000000"><a:moveTo><a:pt x="0" y="0"/></a:moveTo><a:lnTo><a:pt x="1000000" y="0"/></a:lnTo><a:close/></a:path></a:pathLst></a:custGeom>"#;
    let mut reader = quick_xml::Reader::from_reader(raw_xml.as_slice());
    let mut buf = Vec::new();
    let raw = match reader.read_event_into(&mut buf).unwrap() {
        quick_xml::events::Event::Start(e) => {
            ooxml_xml::RawXmlElement::from_reader(&mut reader, &e).unwrap()
        }
        _ => panic!("expected custGeom start"),
    };

    let pptx = write_shape_events(Some(ShapeGeometry::Custom(raw)));
    let cursor = Cursor::new(pptx);
    let mut pres = Presentation::from_reader(cursor).expect("read back should succeed");
    let slide = pres.slide(0).expect("slide 0 should exist");
    let shapes = slide.shapes();
    assert_eq!(shapes.len(), 1);
    assert_eq!(
        shapes[0].text_body.as_ref().map(|_| ()),
        Some(()),
        "text must be preserved even when geometry falls back"
    );
}
