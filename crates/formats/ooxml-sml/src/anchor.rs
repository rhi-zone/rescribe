//! Column-width/row-height resolution for `xdr:twoCellAnchor`/
//! `xdr:oneCellAnchor` drawing anchors (ADR 0015 `positioned_container`
//! applied to ADR 0016 chart placement, XLSX side).
//!
//! Unlike DrawingML's PresentationML (`p:xfrm`) and DrawingML-for-shapes
//! coordinates, which are absolute EMU, `xdr:from`/`xdr:to` markers
//! (ECMA-376 Part 1 §20.5.2.16 `CT_Marker`) identify a position as a
//! **cell index + an EMU offset within that cell**. Resolving a marker to
//! an absolute EMU coordinate requires knowing the cumulative width of
//! every preceding column (and height of every preceding row), which is
//! computed here from the worksheet's `<cols>` overrides and
//! `<sheetFormatPr>` defaults.
//!
//! # Column width formula
//!
//! ECMA-376 Part 1 §18.3.1.13 (`col`/`width` remarks) documents the
//! character-width-to-pixel conversion:
//!
//! ```text
//! pixels = floor(((256 * width + floor(128 / MDW)) / 256) * MDW)
//! ```
//!
//! where `MDW` is the "Maximum Digit Width" of the workbook's normal-style
//! font, in pixels. This formula is corroborated by a Microsoft Q&A thread
//! (<https://learn.microsoft.com/en-us/answers/questions/5858112/column-and-character-widths>)
//! and the MS-OI29500 conformance-clause note for the same ECMA-376
//! section. `MDW = 7` for the standard default font (Calibri 11pt @ 96
//! DPI) — cited from the ClosedXML wiki
//! (<https://github.com/ClosedXML/ClosedXML/wiki/Cell-Dimensions>): "Calibri
//! has MDW 7 at 11pts." Pixels convert to EMU at the standard OOXML/
//! DrawingML 96-DPI convention, `EMU = pixels * 9525` (914400 EMU/inch ÷ 96
//! px/inch), confirmed against openpyxl's `units.py`
//! (`pixels_to_EMU(value) = int(value * 9525)`).
//!
//! When `defaultColWidth` is absent, `baseColWidth` (default 8 per schema)
//! is used directly as the `width` input to the same formula — this is the
//! documented relationship (`baseColWidth`'s own spec description points at
//! `defaultColWidth`'s width-to-pixels math).
//!
//! # Row height
//!
//! `ht` (per-`CT_Row`) and `defaultRowHeight` (on `CT_SheetFormatPr`) are
//! already in points, no character-width indirection needed:
//! `EMU = points * 12700` (914400 EMU/inch ÷ 72 pt/inch).
//!
//! # Fallback chains
//!
//! Column width, for 0-based column `c` (see [`col_width_emu`]):
//! 1. An explicit `<cols><col min max width>` entry covering `c` (1-based
//!    `c+1`), if `width` is set.
//! 2. `sheetFormatPr@defaultColWidth`, if `<sheetFormatPr>` is present and
//!    sets it.
//! 3. `sheetFormatPr@baseColWidth` (or literally 8 if `<sheetFormatPr>` is
//!    absent entirely).
//!
//! Row height, for 0-based row `r` (see [`row_height_emu`]):
//! 1. An explicit `<row r="r+1" ht="...">`, if present.
//! 2. `sheetFormatPr@defaultRowHeight`, if `<sheetFormatPr>` is present
//!    (spec-required on the element when it is present at all).
//! 3. Excel's well-known literal default, 15pt, when `<sheetFormatPr>` is
//!    absent entirely.

use crate::types::{Columns, Row, SheetFormat};

/// "Maximum Digit Width" in pixels for the standard default font (Calibri
/// 11pt @ 96 DPI). See module docs for citation.
const MDW: f64 = 7.0;

/// EMU per pixel at 96 DPI (914400 EMU/inch ÷ 96 px/inch).
const EMU_PER_PIXEL: i64 = 9525;

/// EMU per point (914400 EMU/inch ÷ 72 pt/inch).
const EMU_PER_POINT: f64 = 12700.0;

/// ECMA-376's default `baseColWidth` (schema default, `CT_SheetFormatPr`).
const DEFAULT_BASE_COL_WIDTH: u32 = 8;

/// Excel's well-known literal default row height, in points, used only
/// when `<sheetFormatPr>` is entirely absent (when present, its
/// `defaultRowHeight` attribute is spec-required, so a real value is
/// always available in that case).
const DEFAULT_ROW_HEIGHT_POINTS: f64 = 15.0;

/// Hard iteration caps matching Excel's own worksheet limits (16,384
/// columns / 1,048,576 rows), so reverse EMU-to-cell resolution
/// ([`emu_to_col_marker`]/[`emu_to_row_marker`]) always terminates even if
/// a pathological zero-width column/row run were encountered.
const MAX_COLS: u32 = 16_384;
const MAX_ROWS: u32 = 1_048_576;

/// A resolved cell-anchor marker (`xdr:from`/`xdr:to`): 0-based column/row
/// plus an EMU offset within that cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellMarker {
    pub col: u32,
    pub col_off: i64,
    pub row: u32,
    pub row_off: i64,
}

/// Convert a column width in character units to pixels.
///
/// ECMA-376 Part 1 §18.3.1.13 (see module docs for the full formula
/// citation).
fn col_width_chars_to_pixels(width_chars: f64) -> i64 {
    let padding = (128.0 / MDW).floor();
    (((256.0 * width_chars + padding) / 256.0) * MDW).floor() as i64
}

/// The default column width, in character units, per the fallback chain
/// (`defaultColWidth` if set, else `baseColWidth`, else the schema default
/// 8).
fn default_col_width_chars(sheet_format: Option<&SheetFormat>) -> f64 {
    match sheet_format {
        Some(sf) => sf
            .default_col_width
            .unwrap_or_else(|| sf.base_col_width.unwrap_or(DEFAULT_BASE_COL_WIDTH) as f64),
        None => DEFAULT_BASE_COL_WIDTH as f64,
    }
}

/// The width, in character units, of 0-based column `col`: an explicit
/// `<cols>` override if one covers it, else the sheet/schema default.
fn col_width_chars(col: u32, cols: &[Columns], sheet_format: Option<&SheetFormat>) -> f64 {
    let col_1based = col + 1;
    for group in cols {
        for c in &group.col {
            if c.start_column <= col_1based
                && col_1based <= c.end_column
                && let Some(w) = c.width
            {
                return w;
            }
        }
    }
    default_col_width_chars(sheet_format)
}

/// The width, in EMU, of 0-based column `col`.
pub fn col_width_emu(col: u32, cols: &[Columns], sheet_format: Option<&SheetFormat>) -> i64 {
    col_width_chars_to_pixels(col_width_chars(col, cols, sheet_format)) * EMU_PER_PIXEL
}

/// The default row height, in points, per the fallback chain
/// (`sheetFormatPr@defaultRowHeight` if `<sheetFormatPr>` is present, else
/// Excel's literal default 15pt).
fn default_row_height_points(sheet_format: Option<&SheetFormat>) -> f64 {
    match sheet_format {
        Some(sf) => sf.default_row_height,
        None => DEFAULT_ROW_HEIGHT_POINTS,
    }
}

/// The height, in points, of 0-based row `row`: an explicit `<row ht=...>`
/// if present, else the sheet/literal default.
fn row_height_points(row: u32, rows: &[Row], sheet_format: Option<&SheetFormat>) -> f64 {
    let row_1based = row + 1;
    for r in rows {
        if r.reference == Some(row_1based)
            && let Some(h) = r.height
        {
            return h;
        }
    }
    default_row_height_points(sheet_format)
}

/// The height, in EMU, of 0-based row `row`.
pub fn row_height_emu(row: u32, rows: &[Row], sheet_format: Option<&SheetFormat>) -> i64 {
    (row_height_points(row, rows, sheet_format) * EMU_PER_POINT).round() as i64
}

/// Resolve a [`CellMarker`] (an `xdr:from`/`xdr:to` element) to an absolute
/// `(x, y)` EMU coordinate: the cumulative width/height of every preceding
/// column/row, plus the marker's own offset.
pub fn marker_to_emu(
    marker: &CellMarker,
    cols: &[Columns],
    sheet_format: Option<&SheetFormat>,
    rows: &[Row],
) -> (i64, i64) {
    let x = (0..marker.col)
        .map(|c| col_width_emu(c, cols, sheet_format))
        .sum::<i64>()
        + marker.col_off;
    let y = (0..marker.row)
        .map(|r| row_height_emu(r, rows, sheet_format))
        .sum::<i64>()
        + marker.row_off;
    (x, y)
}

/// Resolve an absolute horizontal EMU coordinate back to a 0-based column
/// index + EMU offset within that column — the reverse of the column half
/// of [`marker_to_emu`], used when writing a `positioned_container`'s
/// `position:x`/`position:width` back out as a cell-anchored `xdr:from`.
pub fn emu_to_col_marker(
    x_emu: i64,
    cols: &[Columns],
    sheet_format: Option<&SheetFormat>,
) -> (u32, i64) {
    let mut remaining = x_emu.max(0);
    let mut col = 0u32;
    while col < MAX_COLS {
        let w = col_width_emu(col, cols, sheet_format);
        if remaining < w {
            break;
        }
        remaining -= w;
        col += 1;
    }
    (col, remaining)
}

/// Resolve an absolute vertical EMU coordinate back to a 0-based row index
/// and EMU offset within that row — the reverse of the row half of
/// [`marker_to_emu`].
pub fn emu_to_row_marker(
    y_emu: i64,
    rows: &[Row],
    sheet_format: Option<&SheetFormat>,
) -> (u32, i64) {
    let mut remaining = y_emu.max(0);
    let mut row = 0u32;
    while row < MAX_ROWS {
        let h = row_height_emu(row, rows, sheet_format);
        if remaining < h {
            break;
        }
        remaining -= h;
        row += 1;
    }
    (row, remaining)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn col(min: u32, max: u32, width: f64) -> crate::types::Column {
        crate::types::Column {
            start_column: min,
            end_column: max,
            width: Some(width),
            style: None,
            hidden: None,
            best_fit: None,
            custom_width: Some(true),
            phonetic: None,
            outline_level: None,
            collapsed: None,
            #[cfg(feature = "extra-attrs")]
            extra_attrs: Default::default(),
        }
    }

    fn cols_group(entries: Vec<crate::types::Column>) -> Vec<Columns> {
        vec![Columns {
            col: entries,
            #[cfg(feature = "extra-children")]
            extra_children: Vec::new(),
        }]
    }

    fn sheet_format(base: Option<u32>, default: Option<f64>, row_height: f64) -> SheetFormat {
        SheetFormat {
            base_col_width: base,
            default_col_width: default,
            default_row_height: row_height,
            custom_height: None,
            zero_height: None,
            thick_top: None,
            thick_bottom: None,
            outline_level_row: None,
            outline_level_col: None,
            #[cfg(feature = "extra-attrs")]
            extra_attrs: Default::default(),
        }
    }

    /// Neither `<cols>` nor `<sheetFormatPr>` present: falls all the way
    /// back to the schema's literal `baseColWidth` default (8 characters).
    /// pixels = floor(((256*8 + 18)/256)*7) = floor(56.4921875) = 56;
    /// EMU = 56 * 9525 = 533400.
    #[test]
    fn col_width_no_cols_no_sheet_format() {
        assert_eq!(col_width_emu(0, &[], None), 533_400);
    }

    /// `sheetFormatPr@defaultColWidth` present, no `<cols>` overrides:
    /// width=10 chars -> pixels = floor(((2560+18)/256)*7) =
    /// floor(70.4921875) = 70; EMU = 70 * 9525 = 666750.
    #[test]
    fn col_width_default_col_width_no_cols() {
        let sf = sheet_format(None, Some(10.0), 15.0);
        assert_eq!(col_width_emu(5, &[], Some(&sf)), 666_750);
    }

    /// An explicit `<cols>` entry overrides both `<sheetFormatPr>` and the
    /// schema default for the columns it covers.
    #[test]
    fn col_width_explicit_cols_entry() {
        let cols = cols_group(vec![col(4, 12, 10.0)]); // D..L, 1-based
        let sf = sheet_format(Some(8), None, 15.0);
        // Column D (0-based 3) is covered by the override -> 666750.
        assert_eq!(col_width_emu(3, &cols, Some(&sf)), 666_750);
        // Column A (0-based 0) is not covered -> falls back to baseColWidth=8 -> 533400.
        assert_eq!(col_width_emu(0, &cols, Some(&sf)), 533_400);
    }

    /// A `<cols>` entry with a `min`-`max` range covering the target
    /// column via the inclusive range, not just a single column.
    #[test]
    fn col_width_range_covers_middle_column() {
        let cols = cols_group(vec![col(2, 6, 12.0)]); // B..F, 1-based
        // Column D (0-based 3, 1-based 4) falls inside [2,6].
        // pixels = floor(((3072+18)/256)*7) = floor(84.4921875) = 84;
        // EMU = 84 * 9525 = 800100.
        assert_eq!(col_width_emu(3, &cols, None), 800_100);
    }

    /// Row height: explicit `<row ht>` wins; default falls back to
    /// `sheetFormatPr@defaultRowHeight` when present, else the literal
    /// 15pt.
    #[test]
    fn row_height_fallbacks() {
        let rows = vec![Row {
            reference: Some(3),
            height: Some(30.0),
            ..Default::default()
        }];
        // Row index 2 (0-based) == @r=3 -> explicit height 30pt -> 381000 EMU.
        assert_eq!(row_height_emu(2, &rows, None), 381_000);
        // Row index 0 (0-based) == @r=1, no explicit height, no sheetFormatPr
        // -> literal default 15pt -> 190500 EMU.
        assert_eq!(row_height_emu(0, &rows, None), 190_500);
        let sf = sheet_format(None, None, 20.0);
        // With sheetFormatPr present, its defaultRowHeight (20pt) applies -> 254000 EMU.
        assert_eq!(row_height_emu(0, &rows, Some(&sf)), 254_000);
    }

    /// `marker_to_emu`/`emu_to_col_marker`/`emu_to_row_marker` round-trip
    /// for a marker with a non-zero offset.
    #[test]
    fn marker_round_trips() {
        let cols = cols_group(vec![col(4, 12, 10.0)]);
        let sf = sheet_format(Some(8), None, 15.0);
        let marker = CellMarker {
            col: 5,
            col_off: 1000,
            row: 2,
            row_off: 500,
        };
        let (x, y) = marker_to_emu(&marker, &cols, Some(&sf), &[]);
        let (col, col_off) = emu_to_col_marker(x, &cols, Some(&sf));
        let (row, row_off) = emu_to_row_marker(y, &[], Some(&sf));
        assert_eq!(col, marker.col);
        assert_eq!(col_off, marker.col_off);
        assert_eq!(row, marker.row);
        assert_eq!(row_off, marker.row_off);
    }
}
