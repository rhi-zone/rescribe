//! Streaming-API cross-checks for ooxml_sml. Split out of the former monolithic
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
// ooxml-sml (xlsx) events(): real, passing check
// ---------------------------------------------------------------------------

const SML_SIMPLE_SHEET: &[u8] = br#"<?xml version="1.0"?>
<worksheet><sheetData>
<row r="1"><c r="A1" t="inlineStr"><is><t>hi</t></is></c></row>
</sheetData></worksheet>"#;

#[test]
fn sml_events_reaches_cell_text() {
    let events: Vec<_> = ooxml_sml::events::events(SML_SIMPLE_SHEET).collect();
    let has_text = events
        .iter()
        .any(|e| matches!(e, ooxml_sml::SmlEvent::StringFragment(t) if t.contains("hi")));
    assert!(
        has_text,
        "expected a StringFragment(\"hi\") event from the inline string cell; got {events:?}"
    );
}

// ---------------------------------------------------------------------------
// ooxml-sml (xlsx) streaming writer: real, passing fidelity check
// ---------------------------------------------------------------------------
//
// This mirrors the fix already pinned by
// crates/formats/ooxml-sml/tests/streaming_writer.rs
// (`row_and_cell_attributes_pass_through`) — reproduced here, independently,
// as part of the fixture harness so the property is checked from this
// suite's vantage point too, not only the crate's own test file.

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

/// `SmlWriter::finish()` produces a complete XLSX zip package (content
/// types, rels, workbook part, worksheet part — not a bare XML fragment),
/// so unlike rst's `Writer` this cannot be compared byte-for-byte against a
/// builder; instead this extracts `xl/worksheets/sheet1.xml` and checks the
/// row/cell attributes survived.
#[test]
fn sml_streaming_writer_preserves_row_and_cell_attributes() {
    use ooxml_sml::generated::{Cell, CellType, Row};
    use ooxml_sml::{SmlEvent, SmlWriter};
    use std::io::Read;

    let row = Row {
        reference: Some(7),
        height: Some(30.0),
        ..Default::default()
    };
    let cell = Cell {
        reference: Some("A7".to_string()),
        cell_type: Some(CellType::String),
        style_index: Some(3),
        ..Default::default()
    };

    let buf = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
    let mut writer = SmlWriter::new(SharedSink(buf.clone(), 0));
    for e in [
        SmlEvent::StartWorkbook,
        SmlEvent::StartWorksheet,
        SmlEvent::StartSheetData,
        SmlEvent::StartRow {
            props: Box::new(row),
        },
        SmlEvent::StartCell {
            props: Box::new(cell),
        },
        SmlEvent::CellValue("hello".into()),
        SmlEvent::EndCell,
        SmlEvent::EndRow,
        SmlEvent::EndSheetData,
        SmlEvent::EndWorksheet,
        SmlEvent::EndWorkbook,
    ] {
        writer.write_event(e);
    }
    writer.finish().expect("SmlWriter::finish");

    let xlsx = buf.borrow().clone();
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(xlsx)).expect("valid zip package");
    let mut sheet_xml = String::new();
    zip.by_name("xl/worksheets/sheet1.xml")
        .expect("worksheet part present")
        .read_to_string(&mut sheet_xml)
        .expect("read worksheet part");

    assert!(
        sheet_xml.contains(r#"r="7""#),
        "streaming writer dropped row number: {sheet_xml}"
    );
    assert!(
        sheet_xml.contains(r#"s="3""#),
        "streaming writer dropped cell style_index: {sheet_xml}"
    );
}
