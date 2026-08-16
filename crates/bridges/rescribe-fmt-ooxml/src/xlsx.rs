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

use crate::chart::{build_minimal_chart_xml, convert_chart};
use ooxml_sml::{
    CellValue, ConditionalRuleType, NumberFormatKind, RowExt, StylesheetExt, Workbook,
    ext::{CellExt, ResolvedSheet},
    types,
    writer::{
        CfColor, CfValue, ColorScaleRule, ConditionalFormat, ConditionalFormatRule, DataBarRule,
        IconSetRule,
    },
};
use rescribe_core::{
    ConversionResult, Document, EmitError, EmitOptions, FidelityWarning, Node, ParseError,
    PropValue, Properties, Severity, SourceInfo, WarningKind,
};
use rescribe_std::{node, prop};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek};
use std::path::Path;

/// Raw-preservation property (CLAUDE.md's "Raw preservation" convention,
/// `{format}:{name}` namespacing): a cell's verbatim OOXML number-format
/// code string (e.g. `"$#,##0.00"`, `"m/d/yyyy"`), set whenever the cell's
/// resolved `numFmtId` isn't the default ("General", id 0). Paired with the
/// semantic `value:type` projection (see [`ooxml_sml::classify_format_code`])
/// the same way ADR 0015 pairs EMU coordinates with a raw fallback property
/// — so the writer can re-emit the exact original display format rather
/// than a canonical substitute, on any cell that carries one, not only the
/// four number-format-derived `value:type`s.
const NUMBER_FORMAT_PROP: &str = "xlsx:number_format";

// ── Conditional formatting (`cfRule`) node kinds ────────────────────────────
//
// Namespaced under `xlsx:` (CLAUDE.md's `{format}:{name}` node-kind
// convention, e.g. `html:div`) rather than added to rescribe-std's shared
// vocabulary: ODF's only conditional-formatting representation
// (`calcext:conditional-formats`) is an unstable LibreOffice extension, not
// part of the stable OASIS ODF spec (verified this session) — so there is
// no second format's real native data model to shape a cross-format
// `rescribe-std` vocabulary against yet. Per this repo's own precedent for
// adding shared IR vocabulary (ADR 0005's bibliography kinds, ADR 0015's
// sheet/cell kinds), that shape is validated against *multiple* formats'
// actual data models before being committed — not derived from one format
// alone. This is scoped, OOXML-specific raw-ish modeling instead, matching
// CLAUDE.md's "Raw preservation" pattern; a future cross-format ADR can
// still promote it to shared vocabulary once a second format's real shape
// is in hand.
//
// A `xlsx:conditional_format` node (child of `sheet`) holds the range
// (`xlsx:range`, OOXML's native `sqref` syntax — e.g. `"A1:C10"` or a
// space-separated multi-range — reused verbatim rather than inventing a
// generic cross-format cell-range concept) and one `xlsx:conditional_format_rule`
// child per `cfRule`. Each rule node carries every `cfRule` field that
// real OOXML files populate (ECMA-376 Part 1, §18.3.1.10, `CT_CfRule`,
// cross-checked against `ooxml-sml`'s generated `ConditionalRule`/
// `ColorScale`/`DataBar`/`IconSet` types this session) as properties;
// `colorScale`/`dataBar`/`iconSet`'s structured sub-elements (`cfvo`/
// `color` lists) use `PropValue::Map`/`PropValue::List` rather than a
// flattened string encoding, so they stay individually inspectable.
const CONDITIONAL_FORMAT: &str = "xlsx:conditional_format";
const CONDITIONAL_FORMAT_RULE: &str = "xlsx:conditional_format_rule";
const CF_RANGE: &str = "xlsx:range";
const CF_TYPE: &str = "xlsx:cf_type";
const CF_PRIORITY: &str = "xlsx:priority";
const CF_DXF_ID: &str = "xlsx:dxf_id";
const CF_OPERATOR: &str = "xlsx:operator";
const CF_FORMULA: &str = "xlsx:formula";
const CF_TEXT: &str = "xlsx:text";
const CF_STOP_IF_TRUE: &str = "xlsx:stop_if_true";
const CF_ABOVE_AVERAGE: &str = "xlsx:above_average";
const CF_PERCENT: &str = "xlsx:percent";
const CF_BOTTOM: &str = "xlsx:bottom";
const CF_EQUAL_AVERAGE: &str = "xlsx:equal_average";
const CF_TIME_PERIOD: &str = "xlsx:time_period";
const CF_RANK: &str = "xlsx:rank";
const CF_STD_DEV: &str = "xlsx:std_dev";
const CF_COLOR_SCALE: &str = "xlsx:color_scale";
const CF_DATA_BAR: &str = "xlsx:data_bar";
const CF_ICON_SET: &str = "xlsx:icon_set";

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
    /// The workbook's styles (`xl/styles.xml`), cloned up front so cell
    /// number-format lookups (`style_index` -> `numFmtId` -> format code ->
    /// [`NumberFormatKind`]) don't need to hold a borrow of `Workbook`
    /// alongside the per-sheet `&mut` calls that resolve sheets.
    stylesheet: Option<ooxml_sml::types::Stylesheet>,
}

impl Converter {
    fn new() -> Self {
        Self {
            warnings: Vec::new(),
            stylesheet: None,
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
        self.stylesheet = workbook.stylesheet().cloned();

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
            // `chart` nodes are block-level (ADR 0016) and are not nested
            // inside the `sheet` node they're attached to — they're appended
            // as document-level siblings immediately after their sheet,
            // mirroring how `sheet` nodes themselves are flat siblings under
            // `document` rather than living inside a `workbook` container
            // (ADR 0015 leaves that container undecided). This keeps a
            // `sheet` node's children strictly `sheet_row`s.
            //
            // Each chart is wrapped in a `positioned_container` (ADR 0015)
            // carrying its resolved anchor position (ADR 0015 applied to
            // ADR 0016 chart placement, XLSX side: `ooxml-sml`'s `anchor`
            // module resolves `xdr:twoCellAnchor`/`xdr:oneCellAnchor` cell
            // + offset coordinates to absolute EMU using the sheet's
            // column-width/row-height state). Position is `None` only when
            // the chart's anchor couldn't be correlated with its drawing
            // relationship at all (a malformed source file, not a
            // resolution-precision gap — see `ooxml_sml::ext::Chart::x`) —
            // in that case the chart is left unwrapped and a fidelity
            // warning is emitted rather than silently dropping position.
            for chart in sheet.charts() {
                let chart_node = convert_chart(chart);
                if let (Some(x), Some(y), Some(width), Some(height)) =
                    (chart.x, chart.y, chart.width, chart.height)
                {
                    let mut container = Node::new(node::POSITIONED_CONTAINER)
                        .prop(prop::POSITION_X, x)
                        .prop(prop::POSITION_Y, y)
                        .prop(prop::POSITION_WIDTH, width)
                        .prop(prop::POSITION_HEIGHT, height)
                        .prop(prop::POSITION_Z_ORDER, chart.z_order as i64);
                    if let Some(rot) = chart.rotation {
                        container = container.prop(prop::POSITION_ROTATION, rot as i64);
                    }
                    children.push(container.child(chart_node));
                } else {
                    self.warn(format!(
                        "Chart in sheet \"{}\" has no resolvable anchor position (malformed drawing relationship); position not represented in IR",
                        sheet.name()
                    ));
                    children.push(chart_node);
                }
            }
        }

        Ok(children)
    }

    /// Convert one worksheet into a `sheet` node. Always returns a node (even
    /// for an empty sheet) so a sheet's existence in the workbook is never
    /// silently dropped — the old `table`-based shape lost this for empty
    /// sheets in a multi-sheet workbook (a dangling `heading` with nothing
    /// after it, or a sheet dropped outright).
    fn convert_sheet(&mut self, sheet: &ResolvedSheet) -> Result<Node, ParseError> {
        let sheet_node = Node::new(node::SHEET)
            .prop(prop::TITLE, sheet.name().to_string())
            .children(convert_conditional_formats(sheet.conditional_formatting()));

        if sheet.row_count() == 0 {
            return Ok(sheet_node);
        }

        // Emit fidelity warnings for features we detect but don't (yet) fully
        // model. Merged cells are handled below via `rowspan`/`colspan` on
        // the top-left cell, and conditional formatting via
        // `xlsx:conditional_format` child nodes (see the constants above),
        // so neither needs a warning here.
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
                    let nf_kind = self
                        .stylesheet
                        .as_ref()
                        .map(|s| s.number_format_kind(cell.style_index))
                        .unwrap_or(NumberFormatKind::Number);

                    if formula.is_some() || !val.is_empty() {
                        let (value_type, value_data) = self.convert_cell_value(&val, nf_kind);
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
                    if let Some(raw_fmt) = self
                        .stylesheet
                        .as_ref()
                        .and_then(|s| s.cell_number_format(cell.style_index))
                    {
                        cell_node = cell_node.prop(NUMBER_FORMAT_PROP, raw_fmt);
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
    /// union, sourced from ODF's `office:value-type`) are not distinguished
    /// by `ooxml-sml`'s `CellValue` itself — it only carries
    /// `Number`/`String`/`Boolean`/`Error`/`Empty` — but OOXML resolves
    /// those four indirectly via a cell's number-format string (e.g.
    /// `"0.00%"` → percentage, `"$#,##0.00"` → currency, `"m/d/yyyy"` →
    /// date), classified by the caller via `nf_kind`
    /// (`ooxml_sml::classify_format_code`) and passed in here. A format
    /// combining date *and* time tokens (e.g. builtin ID 22, `"m/d/yy
    /// h:mm"`) classifies as `NumberFormatKind::DateTime`; the IR's
    /// `value:type` union (like ODF's own `office:value-type`) has no
    /// separate "datetime" — mapped to `"date"` here, matching ODF's own
    /// convention of using `office:value-type="date"` with a full
    /// date-plus-time `office:date-value` for combined values (see
    /// `odf-fmt/src/rescribe/read.rs`'s `map_odf_value_type`). The
    /// underlying numeric value itself (`value:data`) is unaffected either
    /// way — it stays the raw Excel serial number, not a converted date/time
    /// string, so round-tripping through this crate alone never risks a
    /// precision-losing serial<->calendar conversion.
    fn convert_cell_value(
        &mut self,
        value: &CellValue,
        nf_kind: NumberFormatKind,
    ) -> (&'static str, String) {
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
                let value_type = match nf_kind {
                    NumberFormatKind::Number => "number",
                    NumberFormatKind::Percentage => "percentage",
                    NumberFormatKind::Currency => "currency",
                    NumberFormatKind::Date | NumberFormatKind::DateTime => "date",
                    NumberFormatKind::Time => "time",
                };
                (value_type, s)
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

/// Convert a sheet's `cfRule`s into `xlsx:conditional_format` nodes — see
/// the node-kind constants above for the design rationale.
fn convert_conditional_formats(cfs: &[types::ConditionalFormatting]) -> Vec<Node> {
    cfs.iter()
        .map(|cf| {
            let node = match &cf.square_reference {
                Some(range) => Node::new(CONDITIONAL_FORMAT).prop(CF_RANGE, range.clone()),
                None => Node::new(CONDITIONAL_FORMAT),
            };
            node.children(cf.cf_rule.iter().map(convert_conditional_rule))
        })
        .collect()
}

fn convert_conditional_rule(rule: &types::ConditionalRule) -> Node {
    let mut node = Node::new(CONDITIONAL_FORMAT_RULE).prop(CF_PRIORITY, rule.priority as i64);
    if let Some(t) = &rule.r#type {
        node = node.prop(CF_TYPE, t.to_string());
    }
    if let Some(dxf_id) = rule.dxf_id {
        node = node.prop(CF_DXF_ID, dxf_id as i64);
    }
    if let Some(op) = &rule.operator {
        node = node.prop(CF_OPERATOR, op.to_string());
    }
    if let Some(text) = &rule.text {
        node = node.prop(CF_TEXT, text.clone());
    }
    if let Some(v) = rule.stop_if_true {
        node = node.prop(CF_STOP_IF_TRUE, v);
    }
    if let Some(v) = rule.above_average {
        node = node.prop(CF_ABOVE_AVERAGE, v);
    }
    if let Some(v) = rule.percent {
        node = node.prop(CF_PERCENT, v);
    }
    if let Some(v) = rule.bottom {
        node = node.prop(CF_BOTTOM, v);
    }
    if let Some(v) = rule.equal_average {
        node = node.prop(CF_EQUAL_AVERAGE, v);
    }
    if let Some(tp) = &rule.time_period {
        node = node.prop(CF_TIME_PERIOD, tp.to_string());
    }
    if let Some(v) = rule.rank {
        node = node.prop(CF_RANK, v as i64);
    }
    if let Some(v) = rule.std_dev {
        node = node.prop(CF_STD_DEV, v as i64);
    }
    if !rule.formula.is_empty() {
        node = node.prop(
            CF_FORMULA,
            PropValue::List(
                rule.formula
                    .iter()
                    .map(|f| PropValue::String(f.clone()))
                    .collect(),
            ),
        );
    }
    if let Some(cs) = &rule.color_scale {
        node = node.prop(CF_COLOR_SCALE, color_scale_to_propvalue(cs));
    }
    if let Some(db) = &rule.data_bar {
        node = node.prop(CF_DATA_BAR, data_bar_to_propvalue(db));
    }
    if let Some(is) = &rule.icon_set {
        node = node.prop(CF_ICON_SET, icon_set_to_propvalue(is));
    }
    node
}

fn cfvo_to_propvalue(v: &types::ConditionalFormatValue) -> PropValue {
    let mut m = HashMap::new();
    m.insert("type".to_string(), PropValue::String(v.r#type.to_string()));
    if let Some(val) = &v.value {
        m.insert("value".to_string(), PropValue::String(val.clone()));
    }
    if let Some(gte) = v.gte {
        m.insert("gte".to_string(), PropValue::Bool(gte));
    }
    PropValue::Map(m)
}

fn color_to_propvalue(c: &types::Color) -> PropValue {
    let mut m = HashMap::new();
    if let Some(rgb) = &c.rgb {
        m.insert("rgb".to_string(), PropValue::String(rgb_bytes_to_hex(rgb)));
    }
    if let Some(theme) = c.theme {
        m.insert("theme".to_string(), PropValue::Int(theme as i64));
    }
    if let Some(tint) = c.tint {
        m.insert("tint".to_string(), PropValue::Float(tint));
    }
    if let Some(indexed) = c.indexed {
        m.insert("indexed".to_string(), PropValue::Int(indexed as i64));
    }
    if let Some(auto) = c.auto {
        m.insert("auto".to_string(), PropValue::Bool(auto));
    }
    PropValue::Map(m)
}

fn color_scale_to_propvalue(cs: &types::ColorScale) -> PropValue {
    let mut m = HashMap::new();
    m.insert(
        "cfvo".to_string(),
        PropValue::List(cs.cfvo.iter().map(cfvo_to_propvalue).collect()),
    );
    m.insert(
        "color".to_string(),
        PropValue::List(cs.color.iter().map(color_to_propvalue).collect()),
    );
    PropValue::Map(m)
}

fn data_bar_to_propvalue(db: &types::DataBar) -> PropValue {
    let mut m = HashMap::new();
    if let Some(v) = db.min_length {
        m.insert("min_length".to_string(), PropValue::Int(v as i64));
    }
    if let Some(v) = db.max_length {
        m.insert("max_length".to_string(), PropValue::Int(v as i64));
    }
    if let Some(v) = db.show_value {
        m.insert("show_value".to_string(), PropValue::Bool(v));
    }
    m.insert(
        "cfvo".to_string(),
        PropValue::List(db.cfvo.iter().map(cfvo_to_propvalue).collect()),
    );
    m.insert("color".to_string(), color_to_propvalue(&db.color));
    PropValue::Map(m)
}

fn icon_set_to_propvalue(is: &types::IconSet) -> PropValue {
    let mut m = HashMap::new();
    if let Some(v) = is.icon_set {
        m.insert("icon_set".to_string(), PropValue::String(v.to_string()));
    }
    if let Some(v) = is.show_value {
        m.insert("show_value".to_string(), PropValue::Bool(v));
    }
    if let Some(v) = is.percent {
        m.insert("percent".to_string(), PropValue::Bool(v));
    }
    if let Some(v) = is.reverse {
        m.insert("reverse".to_string(), PropValue::Bool(v));
    }
    m.insert(
        "cfvo".to_string(),
        PropValue::List(is.cfvo.iter().map(cfvo_to_propvalue).collect()),
    );
    PropValue::Map(m)
}

/// Format RGB(A) bytes (as `types::Color.rgb` stores them, e.g. `[0xFF,
/// 0xFF, 0x00, 0x00]` for opaque red) as an uppercase hex string
/// (`"FFFF0000"`) — the inverse of `ooxml_sml::writer`'s internal
/// `hex_color_to_bytes`.
fn rgb_bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X}")).collect()
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

/// Rebuild a [`ConditionalFormatRule`] from an `xlsx:conditional_format_rule`
/// node's props — the inverse of `convert_conditional_rule`. Returns `None`
/// only when the node carries neither a recognizable `xlsx:cf_type` nor a
/// priority, which shouldn't happen for a node this crate itself produced,
/// but a document assembled by hand elsewhere might omit either.
fn conditional_format_rule_from_node(node: &Node) -> Option<ConditionalFormatRule> {
    let rule_type = node
        .props
        .get_str(CF_TYPE)
        .and_then(ConditionalRuleType::parse)?;
    let priority = node.props.get_int(CF_PRIORITY).unwrap_or(1).max(0) as u32;

    let mut rule = ConditionalFormatRule::new(rule_type, priority);
    if let Some(v) = node.props.get_int(CF_DXF_ID) {
        rule = rule.with_dxf_id(v as u32);
    }
    if let Some(v) = node.props.get_str(CF_OPERATOR) {
        rule = rule.with_operator(v);
    }
    if let Some(v) = node.props.get_str(CF_TEXT) {
        rule = rule.with_text(v);
    }
    if let Some(v) = node.props.get_bool(CF_STOP_IF_TRUE) {
        rule = rule.with_stop_if_true(v);
    }
    if let Some(v) = node.props.get_bool(CF_ABOVE_AVERAGE) {
        rule = rule.with_above_average(v);
    }
    if let Some(v) = node.props.get_bool(CF_PERCENT) {
        rule = rule.with_percent(v);
    }
    if let Some(v) = node.props.get_bool(CF_BOTTOM) {
        rule = rule.with_bottom(v);
    }
    if let Some(v) = node.props.get_bool(CF_EQUAL_AVERAGE) {
        rule = rule.with_equal_average(v);
    }
    if let Some(v) = node.props.get_str(CF_TIME_PERIOD) {
        rule = rule.with_time_period(v);
    }
    if let Some(v) = node.props.get_int(CF_RANK) {
        rule = rule.with_rank(v as u32);
    }
    if let Some(v) = node.props.get_int(CF_STD_DEV) {
        rule = rule.with_std_dev(v as i32);
    }
    if let Some(PropValue::List(items)) = node.props.get(CF_FORMULA) {
        let formulas = items
            .iter()
            .filter_map(|v| match v {
                PropValue::String(s) => Some(s.clone()),
                _ => None,
            })
            .collect();
        rule = rule.with_formulas(formulas);
    }
    if let Some(PropValue::Map(m)) = node.props.get(CF_COLOR_SCALE) {
        rule = rule.with_color_scale(color_scale_from_map(m));
    }
    if let Some(PropValue::Map(m)) = node.props.get(CF_DATA_BAR) {
        rule = rule.with_data_bar(data_bar_from_map(m));
    }
    if let Some(PropValue::Map(m)) = node.props.get(CF_ICON_SET) {
        rule = rule.with_icon_set(icon_set_from_map(m));
    }
    Some(rule)
}

fn map_get_str<'a>(m: &'a HashMap<String, PropValue>, key: &str) -> Option<&'a str> {
    match m.get(key) {
        Some(PropValue::String(s)) => Some(s),
        _ => None,
    }
}

fn map_get_bool(m: &HashMap<String, PropValue>, key: &str) -> Option<bool> {
    match m.get(key) {
        Some(PropValue::Bool(b)) => Some(*b),
        _ => None,
    }
}

fn map_get_int(m: &HashMap<String, PropValue>, key: &str) -> Option<i64> {
    match m.get(key) {
        Some(PropValue::Int(i)) => Some(*i),
        _ => None,
    }
}

fn map_get_float(m: &HashMap<String, PropValue>, key: &str) -> Option<f64> {
    match m.get(key) {
        Some(PropValue::Float(f)) => Some(*f),
        _ => None,
    }
}

fn cfvo_from_map(m: &HashMap<String, PropValue>) -> Option<CfValue> {
    let value_type = map_get_str(m, "type").and_then(|s| s.parse().ok())?;
    let mut v = match map_get_str(m, "value") {
        Some(val) => CfValue::new(value_type, val),
        None => CfValue::min_max(value_type),
    };
    if let Some(gte) = map_get_bool(m, "gte") {
        v = v.with_gte(gte);
    }
    Some(v)
}

fn cfvo_list_from_propvalue(v: Option<&PropValue>) -> Vec<CfValue> {
    match v {
        Some(PropValue::List(items)) => items
            .iter()
            .filter_map(|item| match item {
                PropValue::Map(m) => cfvo_from_map(m),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn color_from_map(m: &HashMap<String, PropValue>) -> CfColor {
    CfColor {
        rgb: map_get_str(m, "rgb").map(str::to_string),
        theme: map_get_int(m, "theme").map(|v| v as u32),
        tint: map_get_float(m, "tint"),
        indexed: map_get_int(m, "indexed").map(|v| v as u32),
        auto: map_get_bool(m, "auto"),
    }
}

fn color_scale_from_map(m: &HashMap<String, PropValue>) -> ColorScaleRule {
    ColorScaleRule {
        cfvo: cfvo_list_from_propvalue(m.get("cfvo")),
        colors: match m.get("color") {
            Some(PropValue::List(items)) => items
                .iter()
                .map(|item| match item {
                    PropValue::Map(cm) => color_from_map(cm),
                    _ => CfColor::default(),
                })
                .collect(),
            _ => Vec::new(),
        },
    }
}

fn data_bar_from_map(m: &HashMap<String, PropValue>) -> DataBarRule {
    DataBarRule {
        min_length: map_get_int(m, "min_length").map(|v| v as u32),
        max_length: map_get_int(m, "max_length").map(|v| v as u32),
        show_value: map_get_bool(m, "show_value"),
        cfvo: cfvo_list_from_propvalue(m.get("cfvo")),
        color: match m.get("color") {
            Some(PropValue::Map(cm)) => color_from_map(cm),
            _ => CfColor::default(),
        },
    }
}

fn icon_set_from_map(m: &HashMap<String, PropValue>) -> IconSetRule {
    IconSetRule {
        icon_set: map_get_str(m, "icon_set").and_then(|s| s.parse().ok()),
        show_value: map_get_bool(m, "show_value"),
        percent: map_get_bool(m, "percent"),
        reverse: map_get_bool(m, "reverse"),
        cfvo: cfvo_list_from_propvalue(m.get("cfvo")),
    }
}

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
    /// Index (into `workbook`'s sheets) of the most recently written sheet, so
    /// a `chart` node encountered as a document-level sibling right after a
    /// `sheet` node (the shape the reader produces, see `convert_workbook`
    /// above) can be attached to the sheet it followed.
    current_sheet_index: Option<usize>,
}

impl EmitContext {
    fn new() -> Self {
        Self {
            workbook: ooxml_sml::WorkbookBuilder::new(),
            warnings: Vec::new(),
            sheet_count: 0,
            current_sheet_index: None,
        }
    }

    fn warn(&mut self, message: impl Into<String>) {
        self.warnings.push(FidelityWarning::new(
            Severity::Minor,
            WarningKind::FeatureLost("xlsx".to_string()),
            message,
        ));
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
                    self.current_sheet_index = Some(self.workbook.sheet_count() - 1);
                }
                node::CHART => {
                    self.convert_chart_node(node, None);
                }
                node::POSITIONED_CONTAINER
                    if node.children.len() == 1
                        && node.children[0].kind.as_str() == node::CHART =>
                {
                    self.convert_chart_node(&node.children[0], Some(node));
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

        // A running counter incremented only for `sheet_row` children, not
        // `enumerate()` position: `sheet_node` may also carry
        // `xlsx:conditional_format` siblings (see the constants above),
        // which must not consume a row-number slot.
        let mut row_num = 0u32;
        for row_node in sheet_node.children.iter() {
            if row_node.kind.as_str() != node::SHEET_ROW {
                continue;
            }
            row_num += 1;

            for (col_idx, cell_node) in row_node.children.iter().enumerate() {
                if cell_node.kind.as_str() != node::SHEET_CELL {
                    continue;
                }
                let col_num = col_idx as u32 + 1;
                let cell_ref = format!("{}{}", column_to_letter(col_num), row_num);

                let formula = cell_node.props.get_str(prop::VALUE_FORMULA);
                let value_data = cell_node.props.get_str(prop::VALUE);
                let value_type = cell_node.props.get_str(prop::VALUE_TYPE);
                let number_format = Self::number_format_for_write(cell_node, value_type);

                if let Some(f) = formula {
                    match number_format {
                        Some(fmt) => sheet.set_formula_styled(
                            &cell_ref,
                            f.to_string(),
                            ooxml_sml::CellStyle::new().with_number_format(fmt),
                        ),
                        None => sheet.set_formula(&cell_ref, f.to_string()),
                    }
                } else if let Some(data) = value_data {
                    let write_value: ooxml_sml::WriteCellValue = match value_type {
                        Some("number" | "percentage" | "currency" | "date" | "time") => {
                            match data.parse::<f64>() {
                                Ok(num) => num.into(),
                                Err(_) => data.to_string().into(),
                            }
                        }
                        Some("boolean") => data.eq_ignore_ascii_case("true").into(),
                        // "string", "formula-result" (a plain, non-formula
                        // cell should never carry this, but it's handled the
                        // same as "string" if it does), or any other/absent
                        // type: write the value as a string, preserving it
                        // exactly.
                        _ => data.to_string().into(),
                    };
                    match number_format {
                        Some(fmt) => sheet.set_cell_styled(
                            &cell_ref,
                            write_value,
                            ooxml_sml::CellStyle::new().with_number_format(fmt),
                        ),
                        None => sheet.set_cell(&cell_ref, write_value),
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

        for cf_node in &sheet_node.children {
            if cf_node.kind.as_str() != CONDITIONAL_FORMAT {
                continue;
            }
            let Some(range) = cf_node.props.get_str(CF_RANGE) else {
                continue;
            };
            let mut cf = ConditionalFormat::new(range);
            for rule_node in &cf_node.children {
                if rule_node.kind.as_str() != CONDITIONAL_FORMAT_RULE {
                    continue;
                }
                if let Some(rule) = conditional_format_rule_from_node(rule_node) {
                    cf = cf.add_rule(rule);
                }
            }
            sheet.add_conditional_format(cf);
        }
    }

    /// Write a `chart` node (ADR 0016) to the most recently written sheet.
    ///
    /// Per ADR 0016 Decision 4, an OOXML-sourced `chart` node always carries
    /// its original chart-part XML verbatim in `prop::OOXML_CHART_XML`; the
    /// writer's job for that (overwhelmingly common) case is just to re-emit
    /// those bytes unchanged, not to re-derive XML from the semantic fields.
    /// Only a chart node with no raw XML at all (e.g. constructed
    /// programmatically, or sourced from a non-OOXML format) falls back to
    /// [`build_minimal_chart_xml`], a best-effort generator covering the
    /// same v1 semantic-core fields the reader populates.
    ///
    /// `container` is the wrapping `positioned_container` (ADR 0015), when
    /// the chart carries its real anchor (either read from an XLSX with a
    /// resolvable drawing anchor, or supplied by an IR producer such as a
    /// non-OOXML reader). When `Some`, the chart is written via
    /// `embed_chart_at_emu` at the container's `position:x/y/width/height`
    /// (and `position:rotation`, when present) — `ooxml-sml`'s `anchor`
    /// module resolves those EMU values back to a cell+offset anchor using
    /// the sheet's own column-width/row-height state (whatever this
    /// sheet's cells/columns have already set). `container` is `None` for
    /// a bare `chart` node with no positioning wrapper (a synthetic/non-
    /// OOXML-sourced chart, or one whose original anchor couldn't be
    /// resolved on read — see `convert_workbook`'s fidelity warning for
    /// that case) — that case falls back to the original fixed
    /// cell-unit default anchor (`(0, 0)`, 8 cols x 15 rows), unchanged
    /// from this crate's original behavior.
    fn convert_chart_node(&mut self, node: &Node, container: Option<&Node>) {
        let Some(idx) = self.current_sheet_index else {
            self.warn(
                "Chart node with no preceding sheet to attach to; dropped on write".to_string(),
            );
            return;
        };
        let chart_xml: Vec<u8> = match node.props.get_str(prop::OOXML_CHART_XML) {
            Some(xml) => xml.as_bytes().to_vec(),
            None => build_minimal_chart_xml(node).into_bytes(),
        };
        let Some(sheet) = self.workbook.sheet_mut(idx) else {
            self.warn("Chart node's target sheet no longer exists; dropped on write".to_string());
            return;
        };
        match container {
            Some(c) => {
                // Defaults (when a `positioned_container` is present but
                // missing a field, which shouldn't happen for an
                // IR-conformant producer) mirror what the old fixed
                // cell-unit default `(0, 0, 8, 15)` resolves to under
                // this crate's own default column-width/row-height
                // fallbacks: 8 * 533400 = 4267200 EMU wide,
                // 15 * 190500 = 2857500 EMU tall.
                let x = c.props.get_int(prop::POSITION_X).unwrap_or(0);
                let y = c.props.get_int(prop::POSITION_Y).unwrap_or(0);
                let width = c.props.get_int(prop::POSITION_WIDTH).unwrap_or(4_267_200);
                let height = c.props.get_int(prop::POSITION_HEIGHT).unwrap_or(2_857_500);
                let rot = c.props.get_int(prop::POSITION_ROTATION).map(|r| r as i32);
                sheet.embed_chart_at_emu(&chart_xml, x, y, width, height, rot);
            }
            None => {
                sheet.embed_chart(&chart_xml, 0, 0, 8, 15);
            }
        }
    }

    /// Resolve the number-format code (if any) a `sheet_cell` should be
    /// written with: the verbatim raw format (`xlsx:number_format`, set by
    /// the reader — see [`NUMBER_FORMAT_PROP`]) if present, so a cell
    /// read from an XLSX round-trips its exact display format; otherwise a
    /// canonical projected format for `value:type`s whose semantics can't
    /// be represented at all without *some* number format (`currency`,
    /// `percentage`, `date`, `time` — a plain `"number"`/`"string"`/etc.
    /// cell authored directly in the IR, with no raw format captured, needs
    /// no number format to round-trip correctly, so stays `None`).
    fn number_format_for_write<'a>(
        cell_node: &'a Node,
        value_type: Option<&str>,
    ) -> Option<&'a str> {
        if let Some(raw) = cell_node.props.get_str(NUMBER_FORMAT_PROP) {
            return Some(raw);
        }
        match value_type {
            Some("currency") => Some("$#,##0.00"),
            Some("percentage") => Some("0.00%"),
            Some("date") => Some("yyyy-mm-dd"),
            Some("time") => Some("hh:mm:ss"),
            _ => None,
        }
    }
}

impl EmitContext {
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

    /// Build a one-cell XLSX with the given number format via
    /// `ooxml_sml::WorkbookBuilder`, parse it back through this crate's
    /// reader, and return the resulting `sheet_cell` node's props.
    fn roundtrip_number_format(value: f64, number_format: &str) -> Properties {
        let mut wb = ooxml_sml::WorkbookBuilder::new();
        {
            let sheet = wb.add_sheet("Sheet1");
            sheet.set_cell_styled(
                "A1",
                value,
                ooxml_sml::CellStyle::new().with_number_format(number_format),
            );
        }
        let mut bytes = Cursor::new(Vec::new());
        wb.write(&mut bytes).unwrap();
        let doc = parse_bytes(&bytes.into_inner()).unwrap().value;
        let sheet = &doc.content.children[0];
        assert_eq!(sheet.kind.as_str(), node::SHEET);
        let cell = &sheet.children[0].children[0];
        assert_eq!(cell.kind.as_str(), node::SHEET_CELL);
        cell.props.clone()
    }

    #[test]
    fn test_read_percentage_number_format() {
        let props = roundtrip_number_format(0.5, "0.00%");
        assert_eq!(props.get_str(prop::VALUE_TYPE), Some("percentage"));
        assert_eq!(props.get_str(prop::VALUE), Some("0.5"));
        assert_eq!(props.get_str(NUMBER_FORMAT_PROP), Some("0.00%"));
    }

    #[test]
    fn test_read_currency_number_format() {
        let props = roundtrip_number_format(19.99, "$#,##0.00");
        assert_eq!(props.get_str(prop::VALUE_TYPE), Some("currency"));
        assert_eq!(props.get_str(NUMBER_FORMAT_PROP), Some("$#,##0.00"));
    }

    #[test]
    fn test_read_date_number_format() {
        let props = roundtrip_number_format(45000.0, "yyyy-mm-dd");
        assert_eq!(props.get_str(prop::VALUE_TYPE), Some("date"));
        assert_eq!(props.get_str(NUMBER_FORMAT_PROP), Some("yyyy-mm-dd"));
    }

    #[test]
    fn test_read_time_number_format() {
        let props = roundtrip_number_format(0.5, "hh:mm:ss");
        assert_eq!(props.get_str(prop::VALUE_TYPE), Some("time"));
        assert_eq!(props.get_str(NUMBER_FORMAT_PROP), Some("hh:mm:ss"));
    }

    #[test]
    fn test_read_datetime_number_format_maps_to_date() {
        // A combined date+time format (builtin ID 22) has no distinct
        // `value:type` in the IR's union (matching ODF's own convention of
        // treating a combined date+time value as `office:value-type="date"`
        // with a full date-plus-time value) — see `convert_cell_value`.
        let props = roundtrip_number_format(45000.5, "m/d/yy h:mm");
        assert_eq!(props.get_str(prop::VALUE_TYPE), Some("date"));
        assert_eq!(props.get_str(NUMBER_FORMAT_PROP), Some("m/d/yy h:mm"));
    }

    #[test]
    fn test_plain_number_has_no_number_format_prop() {
        // A cell with no explicit style at all (no `xf` entry, so no
        // numFmtId to resolve) shouldn't carry a raw xlsx:number_format
        // prop. Note this is distinct from explicitly requesting the
        // "General" format code via a style, which registers a real
        // (if semantically default) custom `xf` entry and does round-trip
        // its raw format string, same as any other explicit format.
        let mut wb = ooxml_sml::WorkbookBuilder::new();
        {
            let sheet = wb.add_sheet("Sheet1");
            sheet.set_cell("A1", 42.0);
        }
        let mut bytes = Cursor::new(Vec::new());
        wb.write(&mut bytes).unwrap();
        let doc = parse_bytes(&bytes.into_inner()).unwrap().value;
        let cell = &doc.content.children[0].children[0].children[0];
        assert_eq!(cell.props.get_str(prop::VALUE_TYPE), Some("number"));
        assert_eq!(cell.props.get_str(NUMBER_FORMAT_PROP), None);
    }

    #[test]
    fn test_write_percentage_cell_gets_number_format_when_authored_directly() {
        // A sheet_cell authored directly in the IR (not read from an XLSX,
        // so no raw xlsx:number_format prop) with value:type="percentage"
        // should still round-trip as a percentage: the writer falls back to
        // a canonical projected number format.
        let sheet = Node::new(NodeKind::from(node::SHEET))
            .prop(prop::TITLE, "Sheet1")
            .child(
                Node::new(NodeKind::from(node::SHEET_ROW)).child(
                    Node::new(NodeKind::from(node::SHEET_CELL))
                        .prop(prop::VALUE_TYPE, "percentage")
                        .prop(prop::VALUE, "0.5"),
                ),
            );
        let doc =
            Document::new().with_content(Node::new(NodeKind::from(node::DOCUMENT)).child(sheet));
        let bytes = emit(&doc).unwrap().value;
        let reread = parse_bytes(&bytes).unwrap().value;
        let cell = &reread.content.children[0].children[0].children[0];
        assert_eq!(cell.props.get_str(prop::VALUE_TYPE), Some("percentage"));
        assert_eq!(cell.props.get_str(prop::VALUE), Some("0.5"));
    }

    #[test]
    fn test_read_conditional_formatting() {
        let mut wb = ooxml_sml::WorkbookBuilder::new();
        {
            let sheet = wb.add_sheet("Sheet1");
            sheet.set_cell("A1", 10.0);
            sheet.set_cell("A2", 20.0);
            let cf = ConditionalFormat::new("A1:A2")
                .add_cell_is_rule("greaterThan", "15", 1, Some(0))
                .add_color_scale_rule(
                    ColorScaleRule {
                        cfvo: vec![
                            CfValue::min_max(ooxml_sml::types::ConditionalValueType::Min),
                            CfValue::min_max(ooxml_sml::types::ConditionalValueType::Max),
                        ],
                        colors: vec![CfColor::rgb("FF0000"), CfColor::rgb("00FF00")],
                    },
                    2,
                );
            sheet.add_conditional_format(cf);
        }
        let mut bytes = Cursor::new(Vec::new());
        wb.write(&mut bytes).unwrap();
        let doc = parse_bytes(&bytes.into_inner()).unwrap().value;
        let sheet = &doc.content.children[0];

        let cf_node = sheet
            .children
            .iter()
            .find(|n| n.kind.as_str() == CONDITIONAL_FORMAT)
            .expect("conditional_format node");
        assert_eq!(cf_node.props.get_str(CF_RANGE), Some("A1:A2"));
        assert_eq!(cf_node.children.len(), 2);

        let cell_is_rule = &cf_node.children[0];
        assert_eq!(cell_is_rule.kind.as_str(), CONDITIONAL_FORMAT_RULE);
        assert_eq!(cell_is_rule.props.get_str(CF_TYPE), Some("cellIs"));
        assert_eq!(cell_is_rule.props.get_str(CF_OPERATOR), Some("greaterThan"));
        assert_eq!(cell_is_rule.props.get_int(CF_DXF_ID), Some(0));
        assert_eq!(cell_is_rule.props.get_int(CF_PRIORITY), Some(1));
        match cell_is_rule.props.get(CF_FORMULA) {
            Some(PropValue::List(items)) => {
                assert_eq!(items, &vec![PropValue::String("15".to_string())]);
            }
            other => panic!("expected formula list, got {other:?}"),
        }

        let color_scale_rule = &cf_node.children[1];
        assert_eq!(color_scale_rule.props.get_str(CF_TYPE), Some("colorScale"));
        match color_scale_rule.props.get(CF_COLOR_SCALE) {
            Some(PropValue::Map(m)) => {
                let PropValue::List(colors) = &m["color"] else {
                    panic!("expected color list")
                };
                assert_eq!(colors.len(), 2);
                let PropValue::Map(c0) = &colors[0] else {
                    panic!("expected color map")
                };
                assert_eq!(c0.get("rgb"), Some(&PropValue::String("FFFF0000".into())));
            }
            other => panic!("expected color_scale map, got {other:?}"),
        }

        // Round-trip through the writer: re-emit and re-parse, confirming
        // the rule survives intact.
        let bytes2 = emit(&doc).unwrap().value;
        let doc2 = parse_bytes(&bytes2).unwrap().value;
        let sheet2 = &doc2.content.children[0];
        let cf_node2 = sheet2
            .children
            .iter()
            .find(|n| n.kind.as_str() == CONDITIONAL_FORMAT)
            .expect("conditional_format node survives round-trip");
        assert_eq!(cf_node2.props.get_str(CF_RANGE), Some("A1:A2"));
        assert_eq!(cf_node2.children.len(), 2);
        assert_eq!(cf_node2.children[0].props.get_str(CF_TYPE), Some("cellIs"));
        assert_eq!(
            cf_node2.children[1].props.get_str(CF_TYPE),
            Some("colorScale")
        );
    }

    #[test]
    fn test_conditional_format_does_not_shift_row_numbers() {
        // A `sheet` node with an `xlsx:conditional_format` child appearing
        // before the `sheet_row` children (as the reader now produces, see
        // `convert_sheet`) must not shift row numbers when written back —
        // regression test for the row_idx-vs-row_num bug this feature
        // could have introduced.
        let sheet = Node::new(NodeKind::from(node::SHEET))
            .prop(prop::TITLE, "Sheet1")
            .child(Node::new(NodeKind::from(CONDITIONAL_FORMAT)).prop(CF_RANGE, "A1:A2"))
            .child(
                Node::new(NodeKind::from(node::SHEET_ROW)).child(
                    Node::new(NodeKind::from(node::SHEET_CELL))
                        .prop(prop::VALUE_TYPE, "string")
                        .prop(prop::VALUE, "row1"),
                ),
            )
            .child(
                Node::new(NodeKind::from(node::SHEET_ROW)).child(
                    Node::new(NodeKind::from(node::SHEET_CELL))
                        .prop(prop::VALUE_TYPE, "string")
                        .prop(prop::VALUE, "row2"),
                ),
            );
        let doc =
            Document::new().with_content(Node::new(NodeKind::from(node::DOCUMENT)).child(sheet));
        let bytes = emit(&doc).unwrap().value;
        let reread = parse_bytes(&bytes).unwrap().value;
        let reread_sheet = &reread.content.children[0];
        let rows: Vec<&Node> = reread_sheet
            .children
            .iter()
            .filter(|n| n.kind.as_str() == node::SHEET_ROW)
            .collect();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].children[0].props.get_str(prop::VALUE), Some("row1"));
        assert_eq!(rows[1].children[0].props.get_str(prop::VALUE), Some("row2"));
    }

    #[test]
    fn test_number_format_raw_prop_round_trips_verbatim_through_write() {
        // A cell carrying a raw xlsx:number_format prop (as the reader
        // sets it) must be re-emitted with that exact format code, not a
        // canonical substitute — verified here with a currency format
        // whose literal string differs from the writer's own canonical
        // "$#,##0.00" default.
        let sheet = Node::new(NodeKind::from(node::SHEET))
            .prop(prop::TITLE, "Sheet1")
            .child(
                Node::new(NodeKind::from(node::SHEET_ROW)).child(
                    Node::new(NodeKind::from(node::SHEET_CELL))
                        .prop(prop::VALUE_TYPE, "currency")
                        .prop(prop::VALUE, "19.99")
                        .prop(NUMBER_FORMAT_PROP, "#,##0.00 \u{20ac}"),
                ),
            );
        let doc =
            Document::new().with_content(Node::new(NodeKind::from(node::DOCUMENT)).child(sheet));
        let bytes = emit(&doc).unwrap().value;
        let reread = parse_bytes(&bytes).unwrap().value;
        let cell = &reread.content.children[0].children[0].children[0];
        assert_eq!(cell.props.get_str(prop::VALUE_TYPE), Some("currency"));
        assert_eq!(
            cell.props.get_str(NUMBER_FORMAT_PROP),
            Some("#,##0.00 \u{20ac}")
        );
    }

    /// ADR 0016 Decision 4: the raw `ooxml:chart-xml` fallback must round-trip
    /// byte-exact through write→re-read, since the writer re-emits it
    /// verbatim rather than re-deriving XML from the semantic fields
    /// whenever it's present — confirmed here rather than assumed.
    #[test]
    fn test_chart_raw_xml_round_trips_verbatim_through_write() {
        let raw_chart_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart><c:plotArea><c:layout/><c:barChart><c:ser><c:idx val="0"/><c:order val="0"/><c:cat><c:strRef><c:f>Sheet1!$A$2:$A$3</c:f><c:strCache><c:ptCount val="2"/><c:pt idx="0"><c:v>A</c:v></c:pt><c:pt idx="1"><c:v>B</c:v></c:pt></c:strCache></c:strRef></c:cat><c:val><c:numRef><c:f>Sheet1!$B$2:$B$3</c:f><c:numCache><c:ptCount val="2"/><c:pt idx="0"><c:v>1</c:v></c:pt><c:pt idx="1"><c:v>2</c:v></c:pt></c:numCache></c:numRef></c:val></c:ser></c:barChart></c:plotArea></c:chart></c:chartSpace>"#;

        let sheet = Node::new(NodeKind::from(node::SHEET)).prop(prop::TITLE, "Sheet1");
        let chart = Node::new(NodeKind::from(node::CHART))
            .prop(prop::CHART_TYPE, "bar")
            .prop(prop::CHART_LEGEND, false)
            .prop(prop::CHART_HAS_CATEGORY_AXIS, false)
            .prop(prop::CHART_HAS_VALUE_AXIS, false)
            .prop(prop::OOXML_CHART_XML, raw_chart_xml.to_string());
        let doc = Document::new()
            .with_content(Node::new(NodeKind::from(node::DOCUMENT)).children([sheet, chart]));

        let bytes = emit(&doc).unwrap().value;
        let reread = parse_bytes(&bytes).unwrap().value;

        // The input `chart` node had no `positioned_container` wrapper, so
        // it was written via the fixed cell-unit default anchor
        // (`embed_chart(_, 0, 0, 8, 15)`, see `convert_chart_node`). On
        // read-back, that anchor now resolves to a real EMU position (the
        // resolver always succeeds via documented fallback defaults — see
        // `ooxml_sml::anchor`), so the chart comes back wrapped in a
        // `positioned_container` (ADR 0015) rather than as a bare sibling.
        let container = &reread.content.children[1];
        assert_eq!(container.kind.as_str(), node::POSITIONED_CONTAINER);
        let chart_node = &container.children[0];
        assert_eq!(chart_node.kind.as_str(), node::CHART);
        assert_eq!(
            chart_node.props.get_str(prop::OOXML_CHART_XML),
            Some(raw_chart_xml)
        );
    }
}
