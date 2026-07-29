//! Event-driven XLSX writer.
//!
//! [`SmlWriter`] accepts [`SmlEvent`] items via [`SmlWriter::write_event`]
//! and produces a complete `.xlsx` file on [`SmlWriter::finish`].
//!
//! # Memory model
//!
//! Each row/cell event is serialised to XML bytes and pushed straight into the
//! open `xl/worksheets/sheetN.xml` ZIP entry as it arrives — there is no event
//! buffer and no [`WorkbookBuilder`](crate::writer::WorkbookBuilder) or other
//! full-sheet AST behind this writer. Peak memory is
//!
//! ```text
//! O(nesting depth) + O(largest single cell) + O(distinct shared strings) + O(sheet count)
//! ```
//!
//! and is independent of the number of rows or cells written.
//!
//! ## What is deferred, and why
//!
//! * **`xl/workbook.xml` and its `.rels`** — the sheet list is only complete once
//!   every `StartWorksheet` event has been seen, so these parts are written at
//!   [`finish`](SmlWriter::finish). Cost: O(sheet count) — sheet *names*, not
//!   sheet contents.
//! * **`xl/sharedStrings.xml`** — cell string *values* are interned into a
//!   dedup table incrementally as cells are written (a new string gets the next
//!   index the moment it is first seen; that index is final and is written into
//!   the sheet XML immediately — no renumbering happens later). Only the
//!   `xl/sharedStrings.xml` part itself, which lists every distinct string, is
//!   written at `finish()`. Cost: O(distinct strings across the whole workbook),
//!   not O(cells) — this is the same bound ECMA-376 SST deduplication always
//!   costs, streaming or not: the value of a shared-string table is deduplication,
//!   which is inherently a "have I seen this exact string before" set, not a
//!   per-cell cost.
//! * **`[Content_Types].xml`** — written last by [`PackageWriter::finish`],
//!   costing O(number of parts).
//! * **The ZIP central directory** — written by `ZipWriter` at finish, as the
//!   format requires. This is the container's own structure, not buffered
//!   content.
//!
//! What is intentionally *not* written, because it is optional per ECMA-376 and
//! would otherwise force buffering proportional to sheet size:
//!
//! * **`<dimension>`** (§18.3.1.35) — the sheet's used-range hint. ECMA-376
//!   states spreadsheet applications are not required to read or write it;
//!   Excel recomputes it from content. Writing an accurate value would require
//!   knowing every cell reference before the first `<row>` is emitted — i.e.
//!   buffering the whole sheet. Omitted.
//! * **`<row spans="...">`** — a span-list optimization hint for readers.
//!   Optional; omitted for the same reason (would need every cell in the row
//!   before the row's own opening tag).
//!
//! Everything else in the `SmlEvent` surface — worksheets, rows, cells (with
//! their reference, style index, type, value, and formula), and inline-string
//! fragments — is written straight through.
//!
//! Every event this writer accepts already streams directly; there is no
//! construct in `SmlEvent` that structurally requires buffering more than a
//! single row/cell at a time (styles, charts, comments, pivot tables, and
//! merged cells are [`WorkbookBuilder`](crate::writer::WorkbookBuilder)-only
//! features with no `SmlEvent` representation, so they are out of scope for
//! this writer, not deferred by it).
//!
//! # Errors
//!
//! [`write_event`](SmlWriter::write_event) is infallible so that it can be used
//! as a `Handler`-style sink. The first I/O or serialisation error is latched and
//! returned from [`finish`](SmlWriter::finish); subsequent events are dropped.
//!
//! # Example
//!
//! ```ignore
//! use std::io::BufWriter;
//! use std::fs::File;
//! use ooxml_sml::{SmlWriter, SmlEvent};
//!
//! let sink = BufWriter::new(File::create("output.xlsx")?);
//! let mut writer = SmlWriter::new(sink);
//! writer.write_event(SmlEvent::StartWorkbook);
//! writer.write_event(SmlEvent::StartWorksheet);
//! writer.write_event(SmlEvent::StartSheetData);
//! writer.write_event(SmlEvent::StartRow { props: Box::default() });
//! writer.write_event(SmlEvent::StartCell { props: Box::new({
//!     let mut c = ooxml_sml::types::Cell::default();
//!     c.reference = Some("A1".to_string());
//!     c
//! })});
//! writer.write_event(SmlEvent::CellValue("42".into()));
//! writer.write_event(SmlEvent::EndCell);
//! writer.write_event(SmlEvent::EndRow);
//! writer.write_event(SmlEvent::EndSheetData);
//! writer.write_event(SmlEvent::EndWorksheet);
//! writer.write_event(SmlEvent::EndWorkbook);
//! writer.finish()?;
//! # Ok::<(), ooxml_sml::Error>(())
//! ```

use std::collections::HashMap;
use std::io::{Seek, Write};

use ooxml_opc::{PackageWriter, Relationship, Relationships, content_type};
use quick_xml::Writer as XmlWriter;
use quick_xml::events::{BytesStart, Event};

use crate::generated::CellType;
use crate::generated_events::SmlEvent;
use crate::generated_serializers::ToXml;
use crate::types;
use crate::writer::{
    CT_SHARED_STRINGS, CT_WORKBOOK, CT_WORKSHEET, NS_SPREADSHEET, REL_OFFICE_DOCUMENT,
    REL_SHARED_STRINGS, REL_WORKSHEET, build_minimal_workbook, serialize_shared_strings_list,
    serialize_workbook,
};
use crate::{Error, Result};

/// XML declaration written at the head of every part.
const XML_DECL: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\r\n";

/// How much emitted XML to accumulate before pushing it into the ZIP entry.
///
/// See `ooxml-wml`'s `streaming.rs` for the throughput rationale: handing
/// single tags to the deflate encoder one at a time made an earlier version of
/// that writer ~8x slower than its builder path. A fixed window is O(1), not
/// O(document).
const OUT_BUF_CAPACITY: usize = 64 * 1024;

/// Event-driven XLSX writer.
///
/// Feed [`SmlEvent`] items one at a time, then call [`finish`](SmlWriter::finish)
/// to produce a complete XLSX workbook. See the [module docs](self) for the
/// memory model and what is deferred.
pub struct SmlWriter<W: Write + Seek> {
    pkg: PackageWriter<W>,
    /// Fixed-capacity window of emitted XML, flushed into the open ZIP entry
    /// whenever it fills. O(1) — see [`OUT_BUF_CAPACITY`].
    out: Vec<u8>,
    /// Sheet names, in order. O(sheet count) — see module docs.
    sheet_names: Vec<String>,
    /// True while a `xl/worksheets/sheetN.xml` part is the open ZIP entry.
    sheet_open: bool,
    /// Shared-string table used to resolve incoming `CellType::SharedString`
    /// cells (their `<v>` holds an index into *this* table, supplied by the
    /// caller via [`set_shared_strings`](Self::set_shared_strings)). This is
    /// the *input*-side table, distinct from `string_index`/`out_strings`
    /// below, which is the table this writer builds for its *output*.
    input_shared_strings: Vec<String>,
    /// Output-side string dedup: text -> final index. O(distinct strings).
    string_index: HashMap<String, u32>,
    /// Output-side string table in index order, mirroring `string_index`.
    /// O(distinct strings) — see module docs on shared-string deferral.
    out_strings: Vec<String>,
    /// Cell props captured at `StartCell`, mutated in place and flushed at
    /// `EndCell`. `None` outside a cell.
    current_cell: Option<Box<types::Cell>>,
    /// Accumulated cell text; reset at `StartCell`, consumed at `EndCell`.
    cell_value: String,
    has_formula: bool,
    cell_formula: String,
    /// True once the package prologue (`_rels/.rels`, default content types)
    /// has been written.
    started: bool,
    /// True once `finish()` has closed the package.
    ended: bool,
    /// First error seen; surfaced by `finish()`.
    error: Option<Error>,
}

impl<W: Write + Seek> SmlWriter<W> {
    /// Create a new writer targeting `sink`.
    pub fn new(sink: W) -> Self {
        SmlWriter {
            pkg: PackageWriter::new(sink),
            out: Vec::with_capacity(OUT_BUF_CAPACITY + 4096),
            sheet_names: Vec::new(),
            sheet_open: false,
            input_shared_strings: Vec::new(),
            string_index: HashMap::new(),
            out_strings: Vec::new(),
            current_cell: None,
            cell_value: String::new(),
            has_formula: false,
            cell_formula: String::new(),
            started: false,
            ended: false,
            error: None,
        }
    }

    /// Set the shared string table used to resolve [`CellType::SharedString`] cells.
    ///
    /// Call this before feeding any events so that shared-string cell values are
    /// resolved to their actual strings rather than falling back to empty. This
    /// is the *input*-side table (indices as they appear in the source data);
    /// this writer builds its own deduplicated *output* table independently as
    /// distinct strings are encountered.
    pub fn set_shared_strings(&mut self, strings: Vec<String>) {
        self.input_shared_strings = strings;
    }

    /// Record the first error and ignore later ones.
    fn latch(&mut self, r: Result<()>) {
        if let Err(e) = r
            && self.error.is_none()
        {
            self.error = Some(e);
        }
    }

    /// Write the package prologue: default content types and root relationships.
    fn start(&mut self) -> Result<()> {
        self.pkg
            .add_default_content_type("rels", content_type::RELATIONSHIPS);
        self.pkg.add_default_content_type("xml", content_type::XML);

        let mut pkg_rels = Relationships::new();
        pkg_rels.add(Relationship::new(
            "rId1",
            REL_OFFICE_DOCUMENT,
            "xl/workbook.xml",
        ));
        self.pkg.add_part(
            "_rels/.rels",
            content_type::RELATIONSHIPS,
            pkg_rels.serialize().as_bytes(),
        )?;

        self.started = true;
        Ok(())
    }

    fn ensure_started(&mut self) {
        if !self.started && self.error.is_none() {
            let r = self.start();
            self.latch(r);
        }
    }

    /// Push the accumulated window into the open ZIP entry and reset it.
    fn flush_out(&mut self) -> Result<()> {
        if !self.out.is_empty() {
            self.pkg.write_part_data(&self.out)?;
            self.out.clear();
        }
        Ok(())
    }

    /// Flush if the window has grown past its target size.
    fn flush_if_full(&mut self) -> Result<()> {
        if self.out.len() >= OUT_BUF_CAPACITY {
            self.flush_out()?;
        }
        Ok(())
    }

    fn raw(&mut self, bytes: &[u8]) -> Result<()> {
        self.out.extend_from_slice(bytes);
        self.flush_if_full()
    }

    /// Write a container's opening tag with attributes (but not its content or
    /// closing tag) into the output window, using the type's own `ToXml`
    /// attribute-writing logic.
    fn open_tag<T: ToXml>(&mut self, tag: &str, value: &T) -> Result<()> {
        let start = BytesStart::new(tag);
        let start = value.write_attrs(start);
        let mut xw = XmlWriter::new(&mut self.out);
        xw.write_event(Event::Start(start))?;
        self.flush_if_full()
    }

    /// Serialise one complete `ToXml` element (open tag, children, close tag)
    /// into the output window.
    fn emit<T: ToXml>(&mut self, value: &T, tag: &str) -> Result<()> {
        let mut xw = XmlWriter::new(&mut self.out);
        value.write_element(tag, &mut xw)?;
        self.flush_if_full()
    }

    /// Intern a string into the output-side shared-string table, returning its
    /// final index. O(1) amortized; the table itself is O(distinct strings).
    fn intern(&mut self, s: String) -> u32 {
        if let Some(&idx) = self.string_index.get(&s) {
            return idx;
        }
        let idx = self.out_strings.len() as u32;
        self.string_index.insert(s.clone(), idx);
        self.out_strings.push(s);
        idx
    }

    /// Close the currently open worksheet part, if any.
    fn close_sheet(&mut self) -> Result<()> {
        if self.sheet_open {
            self.raw(b"</sheetData></worksheet>")?;
            self.flush_out()?;
            self.sheet_open = false;
        }
        Ok(())
    }

    /// Process one event, writing it into the package immediately.
    ///
    /// Infallible by design; the first error is latched and returned from
    /// [`finish`](Self::finish).
    pub fn write_event(&mut self, event: SmlEvent<'_>) {
        if self.error.is_some() || self.ended {
            return;
        }
        self.ensure_started();
        if self.error.is_some() {
            return;
        }
        let r = self.write_event_inner(event);
        self.latch(r);
    }

    fn write_event_inner(&mut self, event: SmlEvent<'_>) -> Result<()> {
        match event {
            SmlEvent::StartWorkbook | SmlEvent::EndWorkbook => {}

            SmlEvent::StartWorksheet => {
                // Defensively close a sheet the caller forgot to end, mirroring
                // `finish()`'s dangling-state handling.
                self.close_sheet()?;
                let index = self.sheet_names.len() + 1;
                self.sheet_names.push(format!("Sheet{}", index));
                let part = format!("xl/worksheets/sheet{}.xml", index);
                self.pkg.start_part(&part, CT_WORKSHEET)?;
                self.out.extend_from_slice(XML_DECL);
                self.out.extend_from_slice(b"<worksheet xmlns=\"");
                self.out.extend_from_slice(NS_SPREADSHEET.as_bytes());
                self.out.extend_from_slice(b"\"><sheetData>");
                self.sheet_open = true;
            }
            SmlEvent::EndWorksheet => self.close_sheet()?,

            SmlEvent::StartSheetData | SmlEvent::EndSheetData => {}

            SmlEvent::StartRow { props } => self.open_tag("row", &props)?,
            SmlEvent::EndRow => self.raw(b"</row>")?,

            SmlEvent::StartCell { props } => {
                self.current_cell = Some(props);
                self.cell_value.clear();
                self.has_formula = false;
                self.cell_formula.clear();
            }
            SmlEvent::EndCell => {
                if let Some(mut cell) = self.current_cell.take() {
                    self.resolve_cell_value(&mut cell);
                    self.emit(&cell, "c")?;
                }
                self.cell_value.clear();
                self.has_formula = false;
                self.cell_formula.clear();
            }

            SmlEvent::StartInlineString | SmlEvent::EndInlineString => {}

            SmlEvent::CellValue(text) => {
                self.cell_value = text.into_owned();
            }
            SmlEvent::StringFragment(text) => {
                self.cell_value.push_str(&text);
            }
            SmlEvent::Formula(text) => {
                self.has_formula = true;
                self.cell_formula = text.into_owned();
            }
        }
        Ok(())
    }

    /// Fill in a captured `Cell`'s `t`/`v`/`f` fields from the accumulated
    /// value/formula state, interning string values into the output shared-
    /// string table. Mirrors `writer::WorkbookBuilder::build_cell`'s value
    /// mapping so both writers emit the same shape for the same input.
    fn resolve_cell_value(&mut self, cell: &mut types::Cell) {
        let input_type = cell.cell_type;
        cell.value = None;
        cell.formula = None;
        cell.is = None;

        if self.has_formula {
            cell.cell_type = None;
            cell.formula = Some(Box::new(types::CellFormula {
                text: Some(std::mem::take(&mut self.cell_formula)),
                ..Default::default()
            }));
            return;
        }

        match input_type {
            Some(CellType::Boolean) => {
                cell.cell_type = Some(CellType::Boolean);
                cell.value = Some(if self.cell_value == "1" { "1" } else { "0" }.to_string());
            }
            Some(CellType::InlineString) | Some(CellType::String) | Some(CellType::Error) => {
                let s = std::mem::take(&mut self.cell_value);
                let idx = self.intern(s);
                cell.cell_type = Some(CellType::SharedString);
                cell.value = Some(idx.to_string());
            }
            Some(CellType::SharedString) => {
                let resolved = self
                    .cell_value
                    .parse::<usize>()
                    .ok()
                    .and_then(|i| self.input_shared_strings.get(i).cloned());
                match resolved {
                    Some(s) => {
                        let idx = self.intern(s);
                        cell.cell_type = Some(CellType::SharedString);
                        cell.value = Some(idx.to_string());
                    }
                    None => {
                        cell.cell_type = None;
                        cell.value = None;
                    }
                }
            }
            Some(CellType::Number) | None => {
                if self.cell_value.is_empty() {
                    cell.cell_type = None;
                    cell.value = None;
                } else if self.cell_value.parse::<f64>().is_ok() {
                    cell.cell_type = None;
                    cell.value = Some(std::mem::take(&mut self.cell_value));
                } else {
                    let s = std::mem::take(&mut self.cell_value);
                    let idx = self.intern(s);
                    cell.cell_type = Some(CellType::SharedString);
                    cell.value = Some(idx.to_string());
                }
            }
        }
    }

    /// Close the package and write the deferred parts.
    ///
    /// See the [module docs](self) for why `xl/workbook.xml` and
    /// `xl/sharedStrings.xml` can only be written here.
    pub fn finish(mut self) -> Result<()> {
        if let Some(e) = self.error.take() {
            return Err(e);
        }
        self.ensure_started();
        if let Some(e) = self.error.take() {
            return Err(e);
        }

        self.close_sheet()?;
        if let Some(e) = self.error.take() {
            return Err(e);
        }
        self.ended = true;

        let has_strings = !self.out_strings.is_empty();

        let mut wb_rels = Relationships::new();
        for (i, _name) in self.sheet_names.iter().enumerate() {
            wb_rels.add(Relationship::new(
                format!("rId{}", i + 1),
                REL_WORKSHEET,
                format!("worksheets/sheet{}.xml", i + 1),
            ));
        }
        if has_strings {
            wb_rels.add(Relationship::new(
                format!("rId{}", self.sheet_names.len() + 1),
                REL_SHARED_STRINGS,
                "sharedStrings.xml",
            ));
        }
        self.pkg.add_part(
            "xl/_rels/workbook.xml.rels",
            content_type::RELATIONSHIPS,
            wb_rels.serialize().as_bytes(),
        )?;

        let workbook = build_minimal_workbook(&self.sheet_names);
        let workbook_xml = serialize_workbook(&workbook)?;
        self.pkg
            .add_part("xl/workbook.xml", CT_WORKBOOK, &workbook_xml)?;

        if has_strings {
            let sst_xml = serialize_shared_strings_list(&self.out_strings)?;
            self.pkg
                .add_part("xl/sharedStrings.xml", CT_SHARED_STRINGS, &sst_xml)?;
        }

        self.pkg.finish()?;
        Ok(())
    }
}
