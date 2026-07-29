//! Tests for the event-driven XLSX writer (`SmlWriter`).
//!
//! These feed `SmlEvent`s through `SmlWriter` and read the result back with
//! `Workbook`/`ResolvedSheet` to check round-trip fidelity of cell values,
//! types, formulas, and row/cell attribute pass-through.

use std::cell::RefCell;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::rc::Rc;

use ooxml_sml::generated::{Cell as GenCell, CellType, Row as GenRow};
use ooxml_sml::{CellExt, CellResolveExt, RowExt, SmlEvent, SmlWriter, Workbook};

/// A `Write + Seek` sink whose bytes stay reachable after the writer consumes it.
struct SharedSink {
    buf: Rc<RefCell<Vec<u8>>>,
    pos: u64,
}

impl Write for SharedSink {
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

impl Seek for SharedSink {
    fn seek(&mut self, from: SeekFrom) -> std::io::Result<u64> {
        let len = self.buf.borrow().len() as u64;
        self.pos = match from {
            SeekFrom::Start(n) => n,
            SeekFrom::End(n) => (len as i64 + n) as u64,
            SeekFrom::Current(n) => (self.pos as i64 + n) as u64,
        };
        Ok(self.pos)
    }
}

fn emit(evs: Vec<SmlEvent<'static>>) -> Vec<u8> {
    emit_with(evs, |_| {})
}

fn emit_with(
    evs: Vec<SmlEvent<'static>>,
    configure: impl FnOnce(&mut SmlWriter<SharedSink>),
) -> Vec<u8> {
    let shared = Rc::new(RefCell::new(Vec::new()));
    let mut w = SmlWriter::new(SharedSink {
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

fn part_names(xlsx: &[u8]) -> Vec<String> {
    let zip = zip::ZipArchive::new(Cursor::new(xlsx.to_vec())).expect("valid zip");
    zip.file_names().map(|s| s.to_string()).collect()
}

fn part_text(xlsx: &[u8], name: &str) -> String {
    let mut zip = zip::ZipArchive::new(Cursor::new(xlsx.to_vec())).expect("valid zip");
    let mut f = zip.by_name(name).unwrap_or_else(|_| panic!("part {name}"));
    let mut buf = String::new();
    f.read_to_string(&mut buf).expect("read part");
    buf
}

fn cell(reference: &str) -> GenCell {
    GenCell {
        reference: Some(reference.to_string()),
        ..Default::default()
    }
}

fn typed_cell(reference: &str, t: CellType) -> GenCell {
    GenCell {
        reference: Some(reference.to_string()),
        cell_type: Some(t),
        ..Default::default()
    }
}

fn row(n: u32) -> GenRow {
    GenRow {
        reference: Some(n),
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Straight-through emission / package shape
// ---------------------------------------------------------------------------

#[test]
fn empty_workbook_is_a_valid_zip_with_one_sheet_when_none_written() {
    let xlsx = emit(vec![SmlEvent::StartWorkbook, SmlEvent::EndWorkbook]);
    let names = part_names(&xlsx);
    assert!(names.contains(&"xl/workbook.xml".to_string()));
    assert!(names.contains(&"[Content_Types].xml".to_string()));
    assert!(names.contains(&"_rels/.rels".to_string()));
}

#[test]
fn multiple_sheets_get_sequential_parts_and_names() {
    let evs = vec![
        SmlEvent::StartWorkbook,
        SmlEvent::StartWorksheet,
        SmlEvent::EndWorksheet,
        SmlEvent::StartWorksheet,
        SmlEvent::EndWorksheet,
        SmlEvent::EndWorkbook,
    ];
    let xlsx = emit(evs);
    let names = part_names(&xlsx);
    assert!(names.contains(&"xl/worksheets/sheet1.xml".to_string()));
    assert!(names.contains(&"xl/worksheets/sheet2.xml".to_string()));

    let wb_xml = part_text(&xlsx, "xl/workbook.xml");
    assert!(wb_xml.contains(r#"name="Sheet1""#));
    assert!(wb_xml.contains(r#"name="Sheet2""#));
}

#[test]
fn dangling_worksheet_is_closed_defensively_at_finish() {
    // Caller forgets EndWorksheet.
    let evs = vec![SmlEvent::StartWorkbook, SmlEvent::StartWorksheet];
    let xlsx = emit(evs);
    let sheet_xml = part_text(&xlsx, "xl/worksheets/sheet1.xml");
    assert!(sheet_xml.trim_end().ends_with("</worksheet>"));
}

// ---------------------------------------------------------------------------
// Round trip through the reader
// ---------------------------------------------------------------------------

fn one_sheet_workbook(rows: Vec<SmlEvent<'static>>) -> Vec<u8> {
    let mut evs = vec![
        SmlEvent::StartWorkbook,
        SmlEvent::StartWorksheet,
        SmlEvent::StartSheetData,
    ];
    evs.extend(rows);
    evs.push(SmlEvent::EndSheetData);
    evs.push(SmlEvent::EndWorksheet);
    evs.push(SmlEvent::EndWorkbook);
    emit(evs)
}

#[test]
fn round_trips_number_string_boolean_and_formula_cells() {
    let evs = vec![
        SmlEvent::StartRow {
            props: Box::new(row(1)),
        },
        SmlEvent::StartCell {
            props: Box::new(cell("A1")),
        },
        SmlEvent::CellValue("42.5".into()),
        SmlEvent::EndCell,
        SmlEvent::StartCell {
            props: Box::new(typed_cell("B1", CellType::String)),
        },
        SmlEvent::CellValue("hello world".into()),
        SmlEvent::EndCell,
        SmlEvent::StartCell {
            props: Box::new(typed_cell("C1", CellType::Boolean)),
        },
        SmlEvent::CellValue("1".into()),
        SmlEvent::EndCell,
        SmlEvent::StartCell {
            props: Box::new(cell("D1")),
        },
        SmlEvent::Formula("SUM(A1:A1)".into()),
        SmlEvent::EndCell,
        SmlEvent::EndRow,
    ];
    let xlsx = one_sheet_workbook(evs);

    let mut wb = Workbook::from_reader(Cursor::new(xlsx)).expect("open");
    let sheet = wb.resolved_sheet(0).expect("sheet");
    let rows: Vec<_> = sheet.rows().collect();
    assert_eq!(rows.len(), 1);
    let cells: Vec<_> = rows[0].cells_iter().collect();

    let a1 = cells
        .iter()
        .find(|c| c.reference_str() == Some("A1"))
        .unwrap();
    assert_eq!(a1.value_as_string(sheet.context()), "42.5");

    let b1 = cells
        .iter()
        .find(|c| c.reference_str() == Some("B1"))
        .unwrap();
    assert_eq!(b1.value_as_string(sheet.context()), "hello world");
    assert!(b1.is_shared_string());

    let c1 = cells
        .iter()
        .find(|c| c.reference_str() == Some("C1"))
        .unwrap();
    assert_eq!(c1.value_as_bool(sheet.context()), Some(true));

    let d1 = cells
        .iter()
        .find(|c| c.reference_str() == Some("D1"))
        .unwrap();
    assert!(d1.has_formula());
    assert_eq!(d1.formula_text(), Some("SUM(A1:A1)"));
}

#[test]
fn repeated_strings_are_deduplicated_in_the_shared_string_table() {
    let evs = vec![
        SmlEvent::StartRow {
            props: Box::new(row(1)),
        },
        SmlEvent::StartCell {
            props: Box::new(typed_cell("A1", CellType::String)),
        },
        SmlEvent::CellValue("repeat-me".into()),
        SmlEvent::EndCell,
        SmlEvent::StartCell {
            props: Box::new(typed_cell("B1", CellType::String)),
        },
        SmlEvent::CellValue("repeat-me".into()),
        SmlEvent::EndCell,
        SmlEvent::StartCell {
            props: Box::new(typed_cell("C1", CellType::String)),
        },
        SmlEvent::CellValue("unique".into()),
        SmlEvent::EndCell,
        SmlEvent::EndRow,
    ];
    let xlsx = one_sheet_workbook(evs);

    let sst_xml = part_text(&xlsx, "xl/sharedStrings.xml");
    // Exactly two distinct strings, so exactly two `<si>` entries — not three.
    assert_eq!(sst_xml.matches("<si>").count(), 2);

    let mut wb = Workbook::from_reader(Cursor::new(xlsx)).expect("open");
    let sheet = wb.resolved_sheet(0).expect("sheet");
    let rows: Vec<_> = sheet.rows().collect();
    let cells: Vec<_> = rows[0].cells_iter().collect();
    let a1 = cells
        .iter()
        .find(|c| c.reference_str() == Some("A1"))
        .unwrap();
    let b1 = cells
        .iter()
        .find(|c| c.reference_str() == Some("B1"))
        .unwrap();
    // Same string content resolves to the same shared-string index.
    assert_eq!(a1.raw_value(), b1.raw_value());
    assert_eq!(a1.value_as_string(sheet.context()), "repeat-me");
}

#[test]
fn shared_string_input_cells_are_resolved_via_set_shared_strings() {
    let input_sst = vec!["from-source".to_string(), "also-from-source".to_string()];
    let evs = vec![
        SmlEvent::StartWorkbook,
        SmlEvent::StartWorksheet,
        SmlEvent::StartSheetData,
        SmlEvent::StartRow {
            props: Box::new(row(1)),
        },
        SmlEvent::StartCell {
            props: Box::new(typed_cell("A1", CellType::SharedString)),
        },
        SmlEvent::CellValue("1".into()), // index into input_sst
        SmlEvent::EndCell,
        SmlEvent::EndRow,
        SmlEvent::EndSheetData,
        SmlEvent::EndWorksheet,
        SmlEvent::EndWorkbook,
    ];
    let xlsx = emit_with(evs, |w| w.set_shared_strings(input_sst));

    let mut wb = Workbook::from_reader(Cursor::new(xlsx)).expect("open");
    let sheet = wb.resolved_sheet(0).expect("sheet");
    let rows: Vec<_> = sheet.rows().collect();
    let cells: Vec<_> = rows[0].cells_iter().collect();
    let a1 = cells
        .iter()
        .find(|c| c.reference_str() == Some("A1"))
        .unwrap();
    assert_eq!(a1.value_as_string(sheet.context()), "also-from-source");
}

#[test]
fn row_and_cell_attributes_pass_through() {
    // Row number and cell style index are carried on the props structs, not
    // reconstructed — this pins the fix for the pre-rework gap where the
    // event-driven writer dropped `StartRow` props (including the row number)
    // and cell `style_index` entirely.
    let evs = vec![
        SmlEvent::StartRow {
            props: Box::new(GenRow {
                reference: Some(7),
                ..Default::default()
            }),
        },
        SmlEvent::StartCell {
            props: Box::new(GenCell {
                reference: Some("A7".to_string()),
                style_index: Some(3),
                ..Default::default()
            }),
        },
        SmlEvent::CellValue("1".into()),
        SmlEvent::EndCell,
        SmlEvent::EndRow,
    ];
    let xlsx = one_sheet_workbook(evs);
    let sheet_xml = part_text(&xlsx, "xl/worksheets/sheet1.xml");
    assert!(sheet_xml.contains(r#"<row r="7">"#), "{sheet_xml}");
    assert!(
        sheet_xml.contains(r#"r="A7" s="3""#),
        "cell style index should pass through: {sheet_xml}"
    );
}

#[test]
fn empty_cell_round_trips_as_empty() {
    let evs = vec![
        SmlEvent::StartRow {
            props: Box::new(row(1)),
        },
        SmlEvent::StartCell {
            props: Box::new(cell("A1")),
        },
        SmlEvent::EndCell,
        SmlEvent::EndRow,
    ];
    let xlsx = one_sheet_workbook(evs);
    let mut wb = Workbook::from_reader(Cursor::new(xlsx)).expect("open");
    let sheet = wb.resolved_sheet(0).expect("sheet");
    let rows: Vec<_> = sheet.rows().collect();
    let cells: Vec<_> = rows[0].cells_iter().collect();
    let a1 = cells
        .iter()
        .find(|c| c.reference_str() == Some("A1"))
        .unwrap();
    assert_eq!(a1.value_as_string(sheet.context()), "");
}

#[test]
fn no_dimension_element_is_written() {
    // <dimension> is optional per ECMA-376 §18.3.1.35 and would require
    // buffering the whole sheet to compute accurately; the streaming writer
    // omits it rather than doing that.
    let evs = vec![
        SmlEvent::StartRow {
            props: Box::new(row(1)),
        },
        SmlEvent::StartCell {
            props: Box::new(cell("A1")),
        },
        SmlEvent::CellValue("1".into()),
        SmlEvent::EndCell,
        SmlEvent::EndRow,
    ];
    let xlsx = one_sheet_workbook(evs);
    let sheet_xml = part_text(&xlsx, "xl/worksheets/sheet1.xml");
    assert!(!sheet_xml.contains("<dimension"));
}
