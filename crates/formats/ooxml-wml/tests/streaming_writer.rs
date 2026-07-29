//! Tests for the event-driven DOCX writer (`WmlWriter`).
//!
//! These cover the round trip `events() -> WmlWriter -> events()` and the
//! straight-through emission of each construct, including the ones the previous
//! AST-reconstructing implementation dropped (footnote/endnote references).

use std::borrow::Cow;
use std::io::{Cursor, Read};

use ooxml_wml::events::events;
use ooxml_wml::{WmlEvent, WmlWriter};

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

/// A `Write + Seek` sink whose bytes stay reachable after the writer consumes it.
pub struct SharedSink {
    buf: std::rc::Rc<std::cell::RefCell<Vec<u8>>>,
    pos: u64,
}

impl std::io::Write for SharedSink {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        let mut v = self.buf.borrow_mut();
        let pos = self.pos as usize;
        if v.len() < pos + data.len() {
            v.resize(pos + data.len(), 0);
        }
        v[pos..pos + data.len()].copy_from_slice(data);
        self.pos += data.len() as u64;
        Ok(data.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl std::io::Seek for SharedSink {
    fn seek(&mut self, from: std::io::SeekFrom) -> std::io::Result<u64> {
        let len = self.buf.borrow().len() as u64;
        self.pos = match from {
            std::io::SeekFrom::Start(n) => n,
            std::io::SeekFrom::End(n) => (len as i64 + n) as u64,
            std::io::SeekFrom::Current(n) => (self.pos as i64 + n) as u64,
        };
        Ok(self.pos)
    }
}

/// Run the writer over `evs` and return the produced DOCX bytes.
fn emit(evs: Vec<WmlEvent<'static>>) -> Vec<u8> {
    emit_with(evs, |_| {})
}

/// `emit`, with a hook to configure the writer before events are fed.
fn emit_with(
    evs: Vec<WmlEvent<'static>>,
    configure: impl FnOnce(&mut WmlWriter<SharedSink>),
) -> Vec<u8> {
    let shared = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut w = WmlWriter::new(SharedSink {
        buf: shared.clone(),
        pos: 0,
    });
    configure(&mut w);
    for e in evs {
        w.write_event(e);
    }
    w.finish().expect("finish");
    shared.borrow().clone()
}

/// Extract a part from a DOCX byte image.
fn part(docx: &[u8], name: &str) -> Vec<u8> {
    let mut zip = zip::ZipArchive::new(Cursor::new(docx.to_vec())).expect("valid zip");
    let mut f = zip.by_name(name).unwrap_or_else(|_| panic!("part {name}"));
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).expect("read part");
    buf
}

fn part_text(docx: &[u8], name: &str) -> String {
    String::from_utf8(part(docx, name)).expect("utf-8 part")
}

fn part_names(docx: &[u8]) -> Vec<String> {
    let zip = zip::ZipArchive::new(Cursor::new(docx.to_vec())).expect("valid zip");
    zip.file_names().map(|s| s.to_string()).collect()
}

/// Collapse an event stream to comparable tags, ignoring props payloads.
fn tags(evs: &[WmlEvent<'_>]) -> Vec<String> {
    evs.iter()
        .map(|e| match e {
            WmlEvent::StartDocument => "StartDocument".into(),
            WmlEvent::EndDocument => "EndDocument".into(),
            WmlEvent::StartParagraph { .. } => "StartParagraph".into(),
            WmlEvent::EndParagraph => "EndParagraph".into(),
            WmlEvent::StartRun { .. } => "StartRun".into(),
            WmlEvent::EndRun => "EndRun".into(),
            WmlEvent::StartTable { .. } => "StartTable".into(),
            WmlEvent::EndTable => "EndTable".into(),
            WmlEvent::StartTableRow { .. } => "StartTableRow".into(),
            WmlEvent::EndTableRow => "EndTableRow".into(),
            WmlEvent::StartTableCell { .. } => "StartTableCell".into(),
            WmlEvent::EndTableCell => "EndTableCell".into(),
            WmlEvent::StartHyperlink { rel_id, anchor } => {
                format!("StartHyperlink({rel_id:?},{anchor:?})")
            }
            WmlEvent::EndHyperlink => "EndHyperlink".into(),
            WmlEvent::Text(t) => format!("Text({t})"),
            WmlEvent::LineBreak => "LineBreak".into(),
            WmlEvent::FootnoteRef { id } => format!("FootnoteRef({id})"),
            WmlEvent::EndnoteRef { id } => format!("EndnoteRef({id})"),
            WmlEvent::Image { rel_id } => format!("Image({rel_id})"),
        })
        .collect()
}

fn para(text: &'static str) -> Vec<WmlEvent<'static>> {
    vec![
        WmlEvent::StartParagraph {
            props: Box::default(),
        },
        WmlEvent::StartRun {
            props: Box::default(),
        },
        WmlEvent::Text(Cow::Borrowed(text)),
        WmlEvent::EndRun,
        WmlEvent::EndParagraph,
    ]
}

fn sample_events() -> Vec<WmlEvent<'static>> {
    let mut evs = vec![
        WmlEvent::StartDocument,
        WmlEvent::StartParagraph {
            props: Box::default(),
        },
        WmlEvent::StartRun {
            props: Box::default(),
        },
        WmlEvent::Text(Cow::Borrowed("Hello, world!")),
        WmlEvent::LineBreak,
        WmlEvent::Text(Cow::Borrowed("second line")),
        WmlEvent::EndRun,
        WmlEvent::EndParagraph,
        WmlEvent::StartTable {
            props: Box::default(),
        },
        WmlEvent::StartTableRow {
            props: Box::default(),
        },
        WmlEvent::StartTableCell {
            props: Box::default(),
        },
    ];
    evs.extend(para("cell"));
    evs.extend([
        WmlEvent::EndTableCell,
        WmlEvent::EndTableRow,
        WmlEvent::EndTable,
        WmlEvent::EndDocument,
    ]);
    evs
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn roundtrip_events_through_streaming_writer() {
    let input = sample_events();
    let expected = tags(&input);

    let docx = emit(input);
    let doc_xml = part(&docx, "word/document.xml");
    let reparsed: Vec<WmlEvent<'_>> = events(&doc_xml).collect();

    assert_eq!(tags(&reparsed), expected);
}

#[test]
fn package_has_the_required_parts() {
    let docx = emit(sample_events());
    let names = part_names(&docx);
    for required in [
        "[Content_Types].xml",
        "_rels/.rels",
        "word/document.xml",
        "word/_rels/document.xml.rels",
    ] {
        assert!(names.iter().any(|n| n == required), "missing {required}");
    }
}

#[test]
fn document_opens_as_a_docx() {
    let docx = emit(sample_events());
    let doc = ooxml_wml::Document::from_reader(Cursor::new(docx)).expect("opens as DOCX");
    let text = doc.text();
    assert!(text.contains("Hello, world!"), "text was: {text:?}");
    assert!(text.contains("cell"), "text was: {text:?}");
}

#[test]
fn hyperlinks_emit_attributes_and_relationships() {
    let evs = vec![
        WmlEvent::StartDocument,
        WmlEvent::StartParagraph {
            props: Box::default(),
        },
        WmlEvent::StartHyperlink {
            rel_id: Some(Cow::Borrowed("rIdLink")),
            anchor: Some(Cow::Borrowed("top")),
        },
        WmlEvent::StartRun {
            props: Box::default(),
        },
        WmlEvent::Text(Cow::Borrowed("click")),
        WmlEvent::EndRun,
        WmlEvent::EndHyperlink,
        WmlEvent::EndParagraph,
        WmlEvent::EndDocument,
    ];
    let docx = emit_with(evs, |w| {
        w.register_hyperlink("rIdLink", "https://example.com/a&b")
    });

    let doc_xml = part_text(&docx, "word/document.xml");
    assert!(doc_xml.contains("r:id=\"rIdLink\""), "{doc_xml}");
    assert!(doc_xml.contains("w:anchor=\"top\""), "{doc_xml}");

    let rels = part_text(&docx, "word/_rels/document.xml.rels");
    assert!(rels.contains("rIdLink"), "{rels}");
    assert!(rels.contains("example.com"), "{rels}");
}

#[test]
fn footnote_and_endnote_references_survive() {
    // The previous AST-reconstructing writer dropped these on the floor.
    let evs = vec![
        WmlEvent::StartDocument,
        WmlEvent::StartParagraph {
            props: Box::default(),
        },
        WmlEvent::StartRun {
            props: Box::default(),
        },
        WmlEvent::FootnoteRef { id: 3 },
        WmlEvent::EndnoteRef { id: 4 },
        WmlEvent::EndRun,
        WmlEvent::EndParagraph,
        WmlEvent::EndDocument,
    ];
    let doc_xml = part_text(&emit(evs), "word/document.xml");
    assert!(
        doc_xml.contains("w:footnoteReference") && doc_xml.contains("w:id=\"3\""),
        "{doc_xml}"
    );
    assert!(
        doc_xml.contains("w:endnoteReference") && doc_xml.contains("w:id=\"4\""),
        "{doc_xml}"
    );
}

#[test]
fn registered_images_become_parts_and_relationships() {
    let png = b"\x89PNG\r\n\x1a\n-not-a-real-png".to_vec();
    let evs = vec![
        WmlEvent::StartDocument,
        WmlEvent::StartParagraph {
            props: Box::default(),
        },
        WmlEvent::StartRun {
            props: Box::default(),
        },
        WmlEvent::Image {
            rel_id: Cow::Borrowed("img1"),
        },
        WmlEvent::EndRun,
        WmlEvent::EndParagraph,
        WmlEvent::EndDocument,
    ];
    let docx = emit_with(evs, move |w| w.register_image("img1", png, "image/png"));

    let names = part_names(&docx);
    assert!(
        names.iter().any(|n| n.starts_with("word/media/image")),
        "{names:?}"
    );
    assert!(part_text(&docx, "word/document.xml").contains("w:drawing"));
    assert!(part_text(&docx, "word/_rels/document.xml.rels").contains("media/image"));
}

#[test]
fn images_registered_after_the_document_part_is_open_still_land() {
    let png = b"\x89PNG\r\n\x1a\ndeferred".to_vec();
    let shared = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut w = WmlWriter::new(SharedSink {
        buf: shared.clone(),
        pos: 0,
    });
    w.write_event(WmlEvent::StartDocument);
    w.write_event(WmlEvent::StartParagraph {
        props: Box::default(),
    });
    w.write_event(WmlEvent::StartRun {
        props: Box::default(),
    });
    // Registration happens with `word/document.xml` already open.
    w.register_image("late", png, "image/png");
    w.write_event(WmlEvent::Image {
        rel_id: Cow::Borrowed("late"),
    });
    w.write_event(WmlEvent::EndRun);
    w.write_event(WmlEvent::EndParagraph);
    w.write_event(WmlEvent::EndDocument);
    w.finish().expect("finish");
    let docx = shared.borrow().clone();

    assert!(
        part_names(&docx)
            .iter()
            .any(|n| n.starts_with("word/media/image")),
        "deferred image part missing"
    );
    assert!(part_text(&docx, "word/document.xml").contains("w:drawing"));
}

#[test]
fn unclosed_containers_are_closed_at_finish() {
    // A caller that stops mid-document must still get well-formed XML.
    let evs = vec![
        WmlEvent::StartDocument,
        WmlEvent::StartTable {
            props: Box::default(),
        },
        WmlEvent::StartTableRow {
            props: Box::default(),
        },
        WmlEvent::StartTableCell {
            props: Box::default(),
        },
        WmlEvent::StartParagraph {
            props: Box::default(),
        },
        WmlEvent::StartRun {
            props: Box::default(),
        },
        WmlEvent::Text(Cow::Borrowed("dangling")),
    ];
    let doc_xml = part_text(&emit(evs), "word/document.xml");
    assert!(
        doc_xml.ends_with("</w:r></w:p></w:tc></w:tr></w:tbl></w:body></w:document>"),
        "{doc_xml}"
    );
    let reparsed: Vec<WmlEvent<'_>> = events(doc_xml.as_bytes()).collect();
    assert!(tags(&reparsed).contains(&"Text(dangling)".to_string()));
}

#[test]
fn text_is_xml_escaped() {
    let evs = vec![
        WmlEvent::StartDocument,
        WmlEvent::StartParagraph {
            props: Box::default(),
        },
        WmlEvent::StartRun {
            props: Box::default(),
        },
        WmlEvent::Text(Cow::Borrowed("a < b & c > d")),
        WmlEvent::EndRun,
        WmlEvent::EndParagraph,
        WmlEvent::EndDocument,
    ];
    let docx = emit(evs);
    let doc_xml = part(&docx, "word/document.xml");
    let reparsed: Vec<WmlEvent<'_>> = events(&doc_xml).collect();
    assert!(tags(&reparsed).contains(&"Text(a < b & c > d)".to_string()));
}
