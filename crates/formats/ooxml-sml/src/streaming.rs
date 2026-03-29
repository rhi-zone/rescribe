//! Event-driven XLSX writer.
//!
//! [`SmlWriter`] accepts [`SmlEvent`] items via [`SmlWriter::write_event`]
//! and produces a complete `.xlsx` file on [`SmlWriter::finish`].
//!
//! # Memory model
//!
//! Cell state is accumulated incrementally as events arrive. No full event
//! buffer is required; only the current cell's value and metadata are kept
//! until [`finish`](SmlWriter::finish) writes the package.
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

use std::io::{Seek, Write};

use crate::writer::{WorkbookBuilder, WriteCellValue};
use crate::generated::CellType;
use crate::generated_events::SmlEvent;
use crate::Result;

/// Event-driven XLSX writer.
///
/// Feed [`SmlEvent`] items one at a time, then call [`finish`](SmlWriter::finish)
/// to produce a complete XLSX workbook.
pub struct SmlWriter<W: Write + Seek> {
    sink: W,
    builder: WorkbookBuilder,
    sheet_count: usize,
    // Accumulated state for the current cell
    current_cell_ref: Option<String>,
    current_cell_type: Option<CellType>,
    cell_value: String,
    has_formula: bool,
    cell_formula: String,
}

impl<W: Write + Seek> SmlWriter<W> {
    /// Create a new writer targeting `sink`.
    pub fn new(sink: W) -> Self {
        SmlWriter {
            sink,
            builder: WorkbookBuilder::new(),
            sheet_count: 0,
            current_cell_ref: None,
            current_cell_type: None,
            cell_value: String::new(),
            has_formula: false,
            cell_formula: String::new(),
        }
    }

    /// Process one event, updating internal state.
    pub fn write_event(&mut self, event: SmlEvent<'_>) {
        match event.into_owned() {
            // Workbook / sheet structure
            crate::generated_events::SmlEvent::StartWorkbook
            | crate::generated_events::SmlEvent::EndWorkbook => {}

            crate::generated_events::SmlEvent::StartWorksheet => {
                self.sheet_count += 1;
                self.builder.add_sheet(format!("Sheet{}", self.sheet_count));
            }

            crate::generated_events::SmlEvent::EndWorksheet
            | crate::generated_events::SmlEvent::StartSheetData
            | crate::generated_events::SmlEvent::EndSheetData
            | crate::generated_events::SmlEvent::StartRow { .. }
            | crate::generated_events::SmlEvent::EndRow
            | crate::generated_events::SmlEvent::StartInlineString
            | crate::generated_events::SmlEvent::EndInlineString => {}

            // Cell start: capture reference and type, reset accumulators
            crate::generated_events::SmlEvent::StartCell { props } => {
                self.current_cell_ref = props.reference;
                self.current_cell_type = props.cell_type;
                self.cell_value.clear();
                self.has_formula = false;
                self.cell_formula.clear();
            }

            // Cell end: write the accumulated value
            crate::generated_events::SmlEvent::EndCell => {
                if let Some(cell_ref) = self.current_cell_ref.take() {
                    let sheet_idx = self.sheet_count.saturating_sub(1);
                    if let Some(sheet) = self.builder.sheet_mut(sheet_idx) {
                        let value = build_cell_value(
                            self.has_formula,
                            &self.cell_formula,
                            self.current_cell_type,
                            &self.cell_value,
                        );
                        sheet.set_cell(&cell_ref, value);
                    }
                }
                self.current_cell_type = None;
                self.cell_value.clear();
                self.has_formula = false;
                self.cell_formula.clear();
            }

            // Value accumulation
            crate::generated_events::SmlEvent::CellValue(text) => {
                self.cell_value = text.into_owned();
            }

            crate::generated_events::SmlEvent::StringFragment(text) => {
                self.cell_value.push_str(&text);
            }

            crate::generated_events::SmlEvent::Formula(text) => {
                self.has_formula = true;
                self.cell_formula = text.into_owned();
            }
        }
    }

    /// Write the workbook to the underlying sink.
    pub fn finish(self) -> Result<()> {
        self.builder.write(self.sink)
    }
}

fn build_cell_value(
    has_formula: bool,
    formula: &str,
    cell_type: Option<CellType>,
    raw: &str,
) -> WriteCellValue {
    if has_formula {
        return WriteCellValue::Formula(formula.to_owned());
    }
    match cell_type {
        Some(CellType::Boolean) => WriteCellValue::Boolean(raw == "1"),
        Some(CellType::InlineString) | Some(CellType::String) | Some(CellType::Error) => {
            WriteCellValue::String(raw.to_owned())
        }
        // SharedString: value is an index — we lack the shared-string table here,
        // so write the raw index as a number for now.
        Some(CellType::SharedString) | Some(CellType::Number) | None => {
            if raw.is_empty() {
                WriteCellValue::Empty
            } else if let Ok(n) = raw.parse::<f64>() {
                WriteCellValue::Number(n)
            } else {
                WriteCellValue::String(raw.to_owned())
            }
        }
    }
}
