//! XLSX (Excel) reader + writer for rescribe.
//!
//! Translates between Excel spreadsheets (.xlsx) and rescribe's document IR
//! using the `ooxml-sml` crate. Each sheet becomes a section with a heading
//! and table; tables in the document become sheets in the workbook on
//! emit.
//!
//! # Example
//!
//! ```ignore
//! use rescribe_fmt_ooxml::xlsx::parse_file;
//!
//! let result = parse_file("spreadsheet.xlsx")?;
//! let doc = result.value;
//! // Process the document...
//! ```

use ooxml_sml::{
    CellValue, RowExt, Workbook,
    ext::{CellExt, ResolvedSheet},
};
use rescribe_core::{
    ConversionResult, Document, EmitError, EmitOptions, FidelityWarning, Node, ParseError,
    Properties, Severity, SourceInfo, WarningKind,
};
use rescribe_std::{node, prop};
use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek};
use std::path::Path;

// ── Reader ───────────────────────────────────────────────────────────────────

/// Parse an XLSX file from a path.
pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<ConversionResult<Document>, ParseError> {
    let file = File::open(path).map_err(|e| {
        ParseError::Io(std::io::Error::other(format!("Failed to open XLSX: {}", e)))
    })?;
    parse(BufReader::new(file))
}

/// Parse XLSX from a reader that implements Read + Seek.
pub fn parse<R: Read + Seek>(reader: R) -> Result<ConversionResult<Document>, ParseError> {
    let mut workbook = Workbook::from_reader(reader)
        .map_err(|e| ParseError::Invalid(format!("Failed to parse XLSX: {}", e)))?;

    let mut converter = Converter::new();
    let children = converter.convert_workbook(&mut workbook)?;

    let metadata = extract_metadata(&workbook);

    let document = Document {
        content: Node::new(node::DOCUMENT).children(children),
        resources: Default::default(),
        metadata,
        source: Some(SourceInfo {
            format: "xlsx".to_string(),
            metadata: Properties::new(),
        }),
    };

    Ok(ConversionResult::with_warnings(
        document,
        converter.warnings,
    ))
}

/// Parse XLSX from bytes.
pub fn parse_bytes(bytes: &[u8]) -> Result<ConversionResult<Document>, ParseError> {
    let cursor = std::io::Cursor::new(bytes);
    parse(cursor)
}

struct Converter {
    warnings: Vec<FidelityWarning>,
}

impl Converter {
    fn new() -> Self {
        Self {
            warnings: Vec::new(),
        }
    }

    fn warn(&mut self, message: impl Into<String>) {
        self.warnings.push(FidelityWarning::new(
            Severity::Minor,
            WarningKind::FeatureLost("xlsx".to_string()),
            message,
        ));
    }

    fn convert_workbook<R: Read + Seek>(
        &mut self,
        workbook: &mut Workbook<R>,
    ) -> Result<Vec<Node>, ParseError> {
        let mut children = Vec::new();
        let sheet_names = workbook
            .sheet_names()
            .into_iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();

        // Warn once if there are any defined names (named ranges).
        if !workbook.defined_names().is_empty() {
            self.warn(format!(
                "Defined names (named ranges) detected ({} entries); not represented in IR",
                workbook.defined_names().len()
            ));
        }

        for (i, name) in sheet_names.iter().enumerate() {
            let sheet = workbook.resolved_sheet(i).map_err(|e| {
                ParseError::Invalid(format!("Failed to load sheet '{}': {}", name, e))
            })?;

            // Add heading for sheet name (if multiple sheets)
            if sheet_names.len() > 1 {
                let heading = Node::new(node::HEADING)
                    .prop(prop::LEVEL, 2i64)
                    .child(Node::new(node::TEXT).prop(prop::CONTENT, sheet.name().to_string()));
                children.push(heading);
            }

            // Convert sheet to table
            if let Some(table) = self.convert_sheet(&sheet)? {
                children.push(table);
            }
        }

        Ok(children)
    }

    fn convert_sheet(&mut self, sheet: &ResolvedSheet) -> Result<Option<Node>, ParseError> {
        if sheet.row_count() == 0 {
            return Ok(None);
        }

        // Emit fidelity warnings for features we detect but don't fully model.
        if sheet.has_merged_cells() {
            self.warn("Merged cells detected; merge ranges not represented in IR");
        }
        if sheet.has_conditional_formatting() {
            self.warn("Conditional formatting detected; not represented in IR");
        }
        // Warn if any cell carries non-default style (fonts, colors, fills, borders, alignment).
        if sheet.rows().any(|row| {
            row.cells_iter()
                .any(|cell| cell.style_index.is_some_and(|s| s > 0))
        }) {
            self.warn(format!(
                "Cell formatting (fonts, colors, fills, borders, alignment) detected in sheet \"{}\"; style details not represented in IR",
                sheet.name()
            ));
        }
        // Warn if the sheet contains embedded charts.
        if !sheet.charts().is_empty() {
            self.warn(format!(
                "Chart(s) detected in sheet \"{}\" ({} chart(s)); chart data not represented in IR",
                sheet.name(),
                sheet.charts().len()
            ));
        }

        // Determine dimensions
        let (min_row, min_col, max_row, max_col) = match sheet.dimensions() {
            Some(dims) => dims,
            None => return Ok(None),
        };

        let mut table_rows = Vec::new();
        let mut first_row = true;

        for row_num in min_row..=max_row {
            let mut cells = Vec::new();

            for col_num in min_col..=max_col {
                // Use table_header for first row, table_cell for others
                let cell_kind = if first_row {
                    node::TABLE_HEADER
                } else {
                    node::TABLE_CELL
                };

                let mut cell_node = Node::new(cell_kind);

                if let Some(row) = sheet.row(row_num) {
                    if let Some(cell) = row.cell_at_column(col_num) {
                        let val = sheet.cell_value(cell);
                        let text_str = self.convert_cell_value(&val);
                        let text_node = Node::new(node::TEXT).prop(prop::CONTENT, text_str);
                        let mut para = Node::new(node::PARAGRAPH).child(text_node);
                        // Tag the resolved cell type so the writer can round-trip faithfully
                        // without guessing from string content (e.g. "007" must not become 7).
                        let cell_type_tag = match &val {
                            CellValue::Number(_) => Some("n"),
                            CellValue::String(_) => Some("s"),
                            CellValue::Boolean(_) => Some("b"),
                            CellValue::Error(_) => Some("e"),
                            CellValue::Empty => None,
                        };
                        if let Some(ct) = cell_type_tag {
                            para = para.prop("xlsx:cell-type", ct);
                        }
                        // Preserve raw formula for round-trip fidelity.
                        if let Some(formula) = cell.formula_text() {
                            para = para.prop("xlsx:formula", formula.to_string());
                        }
                        cell_node = cell_node.child(para);
                    } else {
                        // Empty cell — add empty paragraph for structural completeness.
                        let text_node = Node::new(node::TEXT).prop(prop::CONTENT, String::new());
                        cell_node = cell_node.child(Node::new(node::PARAGRAPH).child(text_node));
                    }
                } else {
                    let text_node = Node::new(node::TEXT).prop(prop::CONTENT, String::new());
                    cell_node = cell_node.child(Node::new(node::PARAGRAPH).child(text_node));
                }

                cells.push(cell_node);
            }

            table_rows.push(Node::new(node::TABLE_ROW).children(cells));
            first_row = false;
        }

        Ok(Some(Node::new(node::TABLE).children(table_rows)))
    }

    fn convert_cell_value(&mut self, value: &CellValue) -> String {
        match value {
            CellValue::Empty => String::new(),
            CellValue::String(s) => s.clone(),
            CellValue::Number(n) => {
                // Format numbers nicely (avoid trailing .0 for integers)
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    (*n as i64).to_string()
                } else {
                    n.to_string()
                }
            }
            CellValue::Boolean(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
            CellValue::Error(e) => {
                self.warn(format!("Cell contains error: {}", e));
                e.clone()
            }
        }
    }
}

fn extract_metadata<R: Read + Seek>(_workbook: &Workbook<R>) -> Properties {
    let mut metadata = Properties::new();
    // TODO: Extract properties from XLSX if ooxml-sml exposes them
    metadata.set("format", "xlsx");
    metadata
}

// ── Writer ───────────────────────────────────────────────────────────────────

/// Emit a document as XLSX.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    emit_with_options(doc, &EmitOptions::default())
}

/// Emit a document as XLSX with options.
pub fn emit_with_options(
    doc: &Document,
    _options: &EmitOptions,
) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let mut ctx = EmitContext::new();
    ctx.convert_document(doc)?;

    let warnings = std::mem::take(&mut ctx.warnings);
    let bytes = ctx.finish()?;
    Ok(ConversionResult::with_warnings(bytes, warnings))
}

struct EmitContext {
    workbook: ooxml_sml::WorkbookBuilder,
    warnings: Vec<FidelityWarning>,
    sheet_count: usize,
}

impl EmitContext {
    fn new() -> Self {
        Self {
            workbook: ooxml_sml::WorkbookBuilder::new(),
            warnings: Vec::new(),
            sheet_count: 0,
        }
    }

    fn convert_document(&mut self, doc: &Document) -> Result<(), EmitError> {
        self.convert_nodes(&doc.content.children)
    }

    fn convert_nodes(&mut self, nodes: &[Node]) -> Result<(), EmitError> {
        let mut current_sheet_name: Option<String> = None;
        let mut pending_table: Option<&Node> = None;

        for node in nodes {
            match node.kind.as_str() {
                "document" => {
                    self.convert_nodes(&node.children)?;
                }
                "heading" => {
                    // Flush any pending table first
                    if let Some(table) = pending_table.take() {
                        let name = current_sheet_name.take().unwrap_or_else(|| {
                            self.sheet_count += 1;
                            format!("Sheet{}", self.sheet_count)
                        });
                        self.convert_table(table, &name)?;
                    }
                    // Extract heading text as next sheet name
                    current_sheet_name = Some(extract_text(node));
                }
                "table" => {
                    // If we have a pending sheet name, use it; otherwise generate one
                    let name = current_sheet_name.take().unwrap_or_else(|| {
                        self.sheet_count += 1;
                        format!("Sheet{}", self.sheet_count)
                    });
                    self.convert_table(node, &name)?;
                }
                "definition_list" => {
                    // Definition lists from bibliography formats - convert to a sheet
                    self.sheet_count += 1;
                    let name = format!("Sheet{}", self.sheet_count);
                    self.convert_definition_list(node, &name)?;
                }
                _ => {
                    // Recurse into other containers
                    if !node.children.is_empty() {
                        self.convert_nodes(&node.children)?;
                    }
                }
            }
        }

        // Flush any remaining pending table
        if let Some(table) = pending_table {
            let name = current_sheet_name.unwrap_or_else(|| {
                self.sheet_count += 1;
                format!("Sheet{}", self.sheet_count)
            });
            self.convert_table(table, &name)?;
        }

        // If no sheets were added, create an empty sheet
        if self.workbook.sheet_count() == 0 {
            self.workbook.add_sheet("Sheet1");
        }

        Ok(())
    }

    fn convert_table(&mut self, table: &Node, name: &str) -> Result<(), EmitError> {
        let sheet = self.workbook.add_sheet(name);

        for (row_idx, row_node) in table.children.iter().enumerate() {
            if row_node.kind.as_str() != node::TABLE_ROW {
                continue;
            }

            for (col_idx, cell_node) in row_node.children.iter().enumerate() {
                let cell_text = extract_text(cell_node);
                if !cell_text.is_empty() {
                    let col_letter = column_to_letter(col_idx as u32 + 1);
                    let cell_ref = format!("{}{}", col_letter, row_idx + 1);

                    // Find the paragraph node to read xlsx:* props.
                    let para = cell_node
                        .children
                        .iter()
                        .find(|n| n.kind.as_str() == node::PARAGRAPH);

                    let cell_type = para
                        .and_then(|p| p.props.get_str("xlsx:cell-type"))
                        .unwrap_or("");

                    let formula = para.and_then(|p| p.props.get_str("xlsx:formula"));

                    if let Some(f) = formula {
                        // Formula cells: re-emit the formula; cached value is not stored.
                        sheet.set_formula(&cell_ref, f.to_string());
                    } else {
                        // Use the tagged cell type from the reader; fall back to string
                        // when the type is absent (e.g. cells produced outside this reader).
                        match cell_type {
                            "n" => {
                                if let Ok(num) = cell_text.parse::<f64>() {
                                    sheet.set_cell(&cell_ref, num);
                                } else {
                                    sheet.set_cell(&cell_ref, cell_text);
                                }
                            }
                            "b" => {
                                sheet.set_cell(&cell_ref, cell_text.eq_ignore_ascii_case("true"));
                            }
                            _ => {
                                // "s", "e", or absent: write as string, preserving value exactly.
                                sheet.set_cell(&cell_ref, cell_text);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn convert_definition_list(&mut self, list: &Node, name: &str) -> Result<(), EmitError> {
        let sheet = self.workbook.add_sheet(name);

        // Header row
        sheet.set_cell("A1", "Key");
        sheet.set_cell("B1", "Value");

        let mut row = 2u32;
        for entry in &list.children {
            // Look for term and description
            let mut term_text = String::new();
            let mut desc_text = String::new();

            for child in &entry.children {
                match child.kind.as_str() {
                    "definition_term" => {
                        term_text = extract_text(child);
                    }
                    "definition_desc" => {
                        desc_text = extract_text(child);
                    }
                    _ => {}
                }
            }

            if !term_text.is_empty() || !desc_text.is_empty() {
                sheet.set_cell(&format!("A{}", row), term_text);
                sheet.set_cell(&format!("B{}", row), desc_text);
                row += 1;
            }
        }

        Ok(())
    }

    fn finish(self) -> Result<Vec<u8>, EmitError> {
        let mut cursor = Cursor::new(Vec::new());
        self.workbook.write(&mut cursor).map_err(|e| {
            EmitError::Io(std::io::Error::other(format!(
                "Failed to write XLSX: {}",
                e
            )))
        })?;
        Ok(cursor.into_inner())
    }
}

/// Extract all text content from a node recursively.
fn extract_text(node: &Node) -> String {
    let mut text = String::new();

    if let Some(content) = node.props.get_str(prop::CONTENT) {
        text.push_str(content);
    }

    for child in &node.children {
        let child_text = extract_text(child);
        if !child_text.is_empty() {
            if !text.is_empty() && !text.ends_with(' ') {
                text.push(' ');
            }
            text.push_str(&child_text);
        }
    }

    text
}

/// Convert a 1-based column number to Excel column letters (A, B, ..., Z, AA, AB, ...).
fn column_to_letter(mut col: u32) -> String {
    let mut result = String::new();
    while col > 0 {
        col -= 1;
        let c = (b'A' + (col % 26) as u8) as char;
        result.insert(0, c);
        col /= 26;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use rescribe_core::NodeKind;

    #[test]
    fn test_column_to_letter() {
        assert_eq!(column_to_letter(1), "A");
        assert_eq!(column_to_letter(2), "B");
        assert_eq!(column_to_letter(26), "Z");
        assert_eq!(column_to_letter(27), "AA");
        assert_eq!(column_to_letter(28), "AB");
        assert_eq!(column_to_letter(52), "AZ");
        assert_eq!(column_to_letter(53), "BA");
    }

    #[test]
    fn test_emit_empty_document() {
        let doc = Document::new();
        let result = emit(&doc).unwrap();
        // Should produce valid XLSX (ZIP with XML)
        assert!(!result.value.is_empty());
        // XLSX files start with ZIP magic
        assert_eq!(&result.value[0..4], &[0x50, 0x4b, 0x03, 0x04]);
    }

    #[test]
    fn test_emit_table() {
        let table = Node::new(NodeKind::from(node::TABLE)).children(vec![
            Node::new(NodeKind::from(node::TABLE_ROW)).children(vec![
                Node::new(NodeKind::from(node::TABLE_HEADER)).child(
                    Node::new(NodeKind::from(node::PARAGRAPH))
                        .child(Node::new(NodeKind::from(node::TEXT)).prop(prop::CONTENT, "Name")),
                ),
                Node::new(NodeKind::from(node::TABLE_HEADER)).child(
                    Node::new(NodeKind::from(node::PARAGRAPH))
                        .child(Node::new(NodeKind::from(node::TEXT)).prop(prop::CONTENT, "Age")),
                ),
            ]),
            Node::new(NodeKind::from(node::TABLE_ROW)).children(vec![
                Node::new(NodeKind::from(node::TABLE_CELL)).child(
                    Node::new(NodeKind::from(node::PARAGRAPH))
                        .child(Node::new(NodeKind::from(node::TEXT)).prop(prop::CONTENT, "Alice")),
                ),
                Node::new(NodeKind::from(node::TABLE_CELL)).child(
                    Node::new(NodeKind::from(node::PARAGRAPH))
                        .child(Node::new(NodeKind::from(node::TEXT)).prop(prop::CONTENT, "30")),
                ),
            ]),
        ]);

        let doc =
            Document::new().with_content(Node::new(NodeKind::from(node::DOCUMENT)).child(table));

        let result = emit(&doc).unwrap();
        assert!(!result.value.is_empty());
        // XLSX files start with ZIP magic
        assert_eq!(&result.value[0..4], &[0x50, 0x4b, 0x03, 0x04]);
    }
}
