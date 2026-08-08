//! XLSX (Excel) reader + writer for rescribe.
//!
//! Translates between Excel spreadsheets (.xlsx) and rescribe's document IR
//! using the `ooxml-sml` crate. Each worksheet becomes a `sheet` node (ADR
//! 0015: `docs/adr/0015-spreadsheet-presentation-ir-shape.md`) with
//! `sheet_row`/`sheet_cell` children; a cell's value is a typed scalar
//! carried directly on the `sheet_cell` node (`value:type`/`value:data`,
//! plus `value:formula` for formula source text) rather than nested inside a
//! paragraph the way this crate used to do it. A multi-sheet workbook is
//! represented as multiple sibling `sheet` nodes under `document` — ADR 0015
//! leaves a dedicated `workbook` container for a future decision.
//!
//! The writer also accepts plain `table`/`heading` or `definition_list`
//! content (not just native `sheet` nodes) as a generic document-to-
//! spreadsheet export path, for documents produced by other format readers
//! that have no notion of a spreadsheet at all.
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
use std::collections::HashMap;
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

            children.push(self.convert_sheet(&sheet)?);
        }

        Ok(children)
    }

    /// Convert one worksheet into a `sheet` node. Always returns a node (even
    /// for an empty sheet) so a sheet's existence in the workbook is never
    /// silently dropped — the old `table`-based shape lost this for empty
    /// sheets in a multi-sheet workbook (a dangling `heading` with nothing
    /// after it, or a sheet dropped outright).
    fn convert_sheet(&mut self, sheet: &ResolvedSheet) -> Result<Node, ParseError> {
        let sheet_node = Node::new(node::SHEET).prop(prop::TITLE, sheet.name().to_string());

        if sheet.row_count() == 0 {
            return Ok(sheet_node);
        }

        // Emit fidelity warnings for features we detect but don't (yet) fully
        // model. Merged cells are handled below via `rowspan`/`colspan` on
        // the top-left cell, so they no longer need a warning here.
        if sheet.has_conditional_formatting() {
            self.warn(format!(
                "Conditional formatting detected in sheet \"{}\"; not represented in IR (modeling the full cfRule condition space — cellIs/expression/colorScale/dataBar/iconSet — is a larger undertaking left for a future pass)",
                sheet.name()
            ));
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
                "Chart(s) detected in sheet \"{}\" ({} chart(s)); chart data not represented in IR (no chart node kind exists yet)",
                sheet.name(),
                sheet.charts().len()
            ));
        }

        // Determine dimensions
        let (min_row, min_col, max_row, max_col) = match sheet.dimensions() {
            Some(dims) => dims,
            None => return Ok(sheet_node),
        };

        let merge_map = merge_map(sheet);

        let mut sheet_rows = Vec::new();

        for row_num in min_row..=max_row {
            let mut cells = Vec::new();

            for col_num in min_col..=max_col {
                let mut cell_node = Node::new(node::SHEET_CELL);

                if let Some(row) = sheet.row(row_num)
                    && let Some(cell) = row.cell_at_column(col_num)
                {
                    let val = sheet.cell_value(cell);
                    let formula = cell.formula_text();

                    if formula.is_some() || !val.is_empty() {
                        let (value_type, value_data) = self.convert_cell_value(&val);
                        let value_type = if formula.is_some() {
                            "formula-result"
                        } else {
                            value_type
                        };
                        cell_node = cell_node
                            .prop(prop::VALUE_TYPE, value_type)
                            .prop(prop::VALUE, value_data);
                    }
                    if let Some(f) = formula {
                        cell_node = cell_node.prop(prop::VALUE_FORMULA, f.to_string());
                    }

                    if let Some((rowspan, colspan)) = merge_map.get(&(row_num, col_num)) {
                        if *rowspan > 1 {
                            cell_node = cell_node.prop(prop::ROWSPAN, *rowspan as i64);
                        }
                        if *colspan > 1 {
                            cell_node = cell_node.prop(prop::COLSPAN, *colspan as i64);
                        }
                    }
                }

                cells.push(cell_node);
            }

            sheet_rows.push(Node::new(node::SHEET_ROW).children(cells));
        }

        Ok(sheet_node.children(sheet_rows))
    }

    /// Resolve a `CellValue` to its `(value:type, value:data)` pair.
    ///
    /// `Currency`/`Percentage`/`Date`/`Time` (part of ADR 0015's `value:type`
    /// union, sourced from ODF's `office:value-type`) are not produced here:
    /// `ooxml-sml`'s `CellValue` only distinguishes `Number`/`String`/
    /// `Boolean`/`Error`/`Empty` — OOXML resolves those four indirectly via a
    /// cell's number-format string (e.g. `"0.00%"` → percentage, `"$#,##0.00"`
    /// → currency, `"m/d/yyyy"` → date), which `ooxml-sml` does not currently
    /// expose as a classifier. Writing a general Excel number-format-code
    /// classifier is a real undertaking on its own (format codes are an
    /// under-specified mini-language) — left as a follow-up, not attempted in
    /// this pass. All numeric cells map to `"number"` until that lands.
    fn convert_cell_value(&mut self, value: &CellValue) -> (&'static str, String) {
        match value {
            CellValue::Empty => ("string", String::new()),
            CellValue::String(s) => ("string", s.clone()),
            CellValue::Number(n) => {
                // Format numbers nicely (avoid trailing .0 for integers)
                let s = if n.fract() == 0.0 && n.abs() < 1e15 {
                    (*n as i64).to_string()
                } else {
                    n.to_string()
                };
                ("number", s)
            }
            CellValue::Boolean(b) => ("boolean", if *b { "true" } else { "false" }.to_string()),
            CellValue::Error(e) => {
                self.warn(format!("Cell contains error: {}", e));
                // No dedicated `value:type` for spreadsheet errors exists in
                // the ADR 0015 union (neither OOXML nor ODF treat error as a
                // first-class `office:value-type`/`CellValue` peer type) —
                // "string" is the closest fit, matching what the writer can
                // actually round-trip today (`WriteCellValue` has no error
                // variant either).
                ("string", e.clone())
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

/// Map each merge range's top-left `(row, col)` to its `(rowspan, colspan)`.
/// Only ranges spanning more than one row or column are included.
fn merge_map(sheet: &ResolvedSheet) -> HashMap<(u32, u32), (u32, u32)> {
    let mut map = HashMap::new();
    if let Some(mc) = sheet.merged_cells() {
        for merge in &mc.merge_cell {
            if let Some((start, end)) = parse_range(&merge.reference) {
                let rowspan = end.0 - start.0 + 1;
                let colspan = end.1 - start.1 + 1;
                if rowspan > 1 || colspan > 1 {
                    map.insert(start, (rowspan, colspan));
                }
            }
        }
    }
    map
}

/// Parse a `"A1"`-style cell reference into `(row, col)`, both 1-based.
fn parse_cell_ref(s: &str) -> Option<(u32, u32)> {
    let letters_end = s.find(|c: char| c.is_ascii_digit())?;
    let (letters, digits) = s.split_at(letters_end);
    if letters.is_empty() || digits.is_empty() {
        return None;
    }
    Some((digits.parse().ok()?, letter_to_column(letters)))
}

/// Parse a `"A1:C3"`-style range reference into its `(start, end)` cell
/// references. A single-cell reference (no `:`) yields `start == end`.
fn parse_range(s: &str) -> Option<((u32, u32), (u32, u32))> {
    let mut parts = s.split(':');
    let start = parse_cell_ref(parts.next()?)?;
    let end = match parts.next() {
        Some(e) => parse_cell_ref(e)?,
        None => start,
    };
    Some((start, end))
}

/// Convert Excel column letters (A, B, ..., Z, AA, AB, ...) to a 1-based column number.
fn letter_to_column(letters: &str) -> u32 {
    let mut col = 0u32;
    for c in letters.chars() {
        col = col * 26 + (c.to_ascii_uppercase() as u32 - 'A' as u32 + 1);
    }
    col
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

    fn next_sheet_name(&mut self) -> String {
        self.sheet_count += 1;
        format!("Sheet{}", self.sheet_count)
    }

    fn convert_nodes(&mut self, nodes: &[Node]) -> Result<(), EmitError> {
        let mut current_sheet_name: Option<String> = None;
        let mut pending_table: Option<&Node> = None;

        for node in nodes {
            match node.kind.as_str() {
                "document" => {
                    self.convert_nodes(&node.children)?;
                }
                // Native shape (ADR 0015): the sheet's name travels with the
                // node itself, so it needs none of the heading-pairing logic
                // the legacy `table` fallback below still uses.
                "sheet" => {
                    if let Some(table) = pending_table.take() {
                        let name = current_sheet_name
                            .take()
                            .unwrap_or_else(|| self.next_sheet_name());
                        self.convert_table(table, &name)?;
                    }
                    let name = node
                        .props
                        .get_str(prop::TITLE)
                        .map(str::to_string)
                        .unwrap_or_else(|| self.next_sheet_name());
                    self.convert_sheet(node, &name);
                }
                "heading" => {
                    // Flush any pending table first
                    if let Some(table) = pending_table.take() {
                        let name = current_sheet_name
                            .take()
                            .unwrap_or_else(|| self.next_sheet_name());
                        self.convert_table(table, &name)?;
                    }
                    // Extract heading text as next sheet name
                    current_sheet_name = Some(extract_text(node));
                }
                "table" => {
                    // If we have a pending sheet name, use it; otherwise generate one
                    let name = current_sheet_name
                        .take()
                        .unwrap_or_else(|| self.next_sheet_name());
                    self.convert_table(node, &name)?;
                }
                "definition_list" => {
                    // Definition lists from bibliography formats - convert to a sheet
                    let name = self.next_sheet_name();
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
            let name = current_sheet_name.unwrap_or_else(|| self.next_sheet_name());
            self.convert_table(table, &name)?;
        }

        // If no sheets were added, create an empty sheet
        if self.workbook.sheet_count() == 0 {
            self.workbook.add_sheet("Sheet1");
        }

        Ok(())
    }

    /// Convert a native `sheet` node (ADR 0015: `sheet_row`/`sheet_cell`
    /// children, typed value directly on the cell) into a workbook sheet.
    fn convert_sheet(&mut self, sheet_node: &Node, name: &str) {
        let sheet = self.workbook.add_sheet(name);

        for (row_idx, row_node) in sheet_node.children.iter().enumerate() {
            if row_node.kind.as_str() != node::SHEET_ROW {
                continue;
            }
            let row_num = row_idx as u32 + 1;

            for (col_idx, cell_node) in row_node.children.iter().enumerate() {
                if cell_node.kind.as_str() != node::SHEET_CELL {
                    continue;
                }
                let col_num = col_idx as u32 + 1;
                let cell_ref = format!("{}{}", column_to_letter(col_num), row_num);

                let formula = cell_node.props.get_str(prop::VALUE_FORMULA);
                let value_data = cell_node.props.get_str(prop::VALUE);

                if let Some(f) = formula {
                    sheet.set_formula(&cell_ref, f.to_string());
                } else if let Some(data) = value_data {
                    match cell_node.props.get_str(prop::VALUE_TYPE) {
                        Some("number") => {
                            if let Ok(num) = data.parse::<f64>() {
                                sheet.set_cell(&cell_ref, num);
                            } else {
                                sheet.set_cell(&cell_ref, data.to_string());
                            }
                        }
                        Some("boolean") => {
                            sheet.set_cell(&cell_ref, data.eq_ignore_ascii_case("true"));
                        }
                        // "string", "formula-result" (a plain, non-formula
                        // cell should never carry this, but it's handled the
                        // same as "string" if it does), or any other/absent
                        // type: write the value as a string, preserving it
                        // exactly.
                        _ => {
                            sheet.set_cell(&cell_ref, data.to_string());
                        }
                    }
                }

                let rowspan = cell_node.props.get_int(prop::ROWSPAN).unwrap_or(1).max(1);
                let colspan = cell_node.props.get_int(prop::COLSPAN).unwrap_or(1).max(1);
                if rowspan > 1 || colspan > 1 {
                    let end_row = row_num + rowspan as u32 - 1;
                    let end_col = col_num + colspan as u32 - 1;
                    let range = format!(
                        "{}{}:{}{}",
                        column_to_letter(col_num),
                        row_num,
                        column_to_letter(end_col),
                        end_row
                    );
                    sheet.merge_cells(&range);
                }
            }
        }
    }

    /// Generic `table`-node fallback: converts a plain prose `table` (as
    /// produced by any non-spreadsheet format reader — commonmark, RST,
    /// etc.) into a workbook sheet. Kept alongside the native `sheet` path
    /// above so documents with no notion of a spreadsheet can still be
    /// exported to XLSX.
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
                    sheet.set_cell(&cell_ref, cell_text);
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
    fn test_letter_to_column() {
        assert_eq!(letter_to_column("A"), 1);
        assert_eq!(letter_to_column("B"), 2);
        assert_eq!(letter_to_column("Z"), 26);
        assert_eq!(letter_to_column("AA"), 27);
        assert_eq!(letter_to_column("AB"), 28);
        assert_eq!(letter_to_column("BA"), 53);
    }

    #[test]
    fn test_parse_range_single_cell() {
        assert_eq!(parse_range("A1"), Some(((1, 1), (1, 1))));
    }

    #[test]
    fn test_parse_range_multi_cell() {
        assert_eq!(parse_range("A1:C3"), Some(((1, 1), (3, 3))));
        assert_eq!(parse_range("B2:D2"), Some(((2, 2), (2, 4))));
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
    fn test_emit_sheet() {
        let sheet = Node::new(NodeKind::from(node::SHEET))
            .prop(prop::TITLE, "Sheet1")
            .children(vec![
                Node::new(NodeKind::from(node::SHEET_ROW)).children(vec![
                    Node::new(NodeKind::from(node::SHEET_CELL))
                        .prop(prop::VALUE_TYPE, "string")
                        .prop(prop::VALUE, "Name"),
                    Node::new(NodeKind::from(node::SHEET_CELL))
                        .prop(prop::VALUE_TYPE, "string")
                        .prop(prop::VALUE, "Age"),
                ]),
                Node::new(NodeKind::from(node::SHEET_ROW)).children(vec![
                    Node::new(NodeKind::from(node::SHEET_CELL))
                        .prop(prop::VALUE_TYPE, "string")
                        .prop(prop::VALUE, "Alice"),
                    Node::new(NodeKind::from(node::SHEET_CELL))
                        .prop(prop::VALUE_TYPE, "number")
                        .prop(prop::VALUE, "30"),
                ]),
            ]);

        let doc =
            Document::new().with_content(Node::new(NodeKind::from(node::DOCUMENT)).child(sheet));

        let result = emit(&doc).unwrap();
        assert!(!result.value.is_empty());
        // XLSX files start with ZIP magic
        assert_eq!(&result.value[0..4], &[0x50, 0x4b, 0x03, 0x04]);
    }

    #[test]
    fn test_emit_table_fallback() {
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
