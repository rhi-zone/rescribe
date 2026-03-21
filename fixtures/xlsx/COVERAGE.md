# XLSX Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

## Workbook structure
- [x] single sheet — `basic`
- [x] multiple sheets — `multi-sheet`
- [ ] sheet with tab color — (missing)
- [ ] hidden sheet — (missing)
- [ ] very hidden sheet (xlVeryHidden) — (missing)
- [ ] sheet ordering vs. tab order — (missing)
- [ ] named ranges / defined names — (missing)

## Cell value types
- [x] number (integer and float) — `basic`
- [x] string — `basic`
- [x] formula — `formula`
- [ ] boolean (TRUE/FALSE) — (missing)
- [ ] error values (#DIV/0!, #N/A, #REF!, #VALUE!, #NAME?, #NULL!, #NUM!) — (missing)
- [ ] empty cell — (missing)
- [ ] inline string (`<is><t>`) vs. shared string — (missing)
- [ ] date (number with date format) — (missing)
- [ ] time (fractional number with time format) — (missing)
- [ ] datetime — (missing)
- [ ] currency — (missing)
- [ ] percentage — (missing)
- [ ] scientific notation — (missing)

## Formulas
- [x] basic formula — `formula`
- [ ] formula with external reference — (missing)
- [ ] array formula — (missing)
- [ ] shared formula — (missing)
- [ ] formula with named range reference — (missing)
- [ ] formula result cache (value without recalculation) — (missing)

## Cell formatting / style
- [ ] number format (format code) — (missing)
- [ ] font name — (missing)
- [ ] font size — (missing)
- [ ] bold — (missing)
- [ ] italic — (missing)
- [ ] underline — (missing)
- [ ] strikeout — (missing)
- [ ] font color — (missing)
- [ ] background (fill) color — (missing)
- [ ] cell border (top/bottom/left/right) — (missing)
- [ ] horizontal alignment (left/center/right/fill/justify) — (missing)
- [ ] vertical alignment (top/middle/bottom) — (missing)
- [ ] text wrap — (missing)
- [ ] indent level — (missing)
- [ ] text rotation — (missing)
- [ ] cell protection (locked/hidden) — (missing)

## Sheet structure
- [ ] column width — (missing)
- [ ] row height — (missing)
- [ ] hidden row — (missing)
- [ ] hidden column — (missing)
- [ ] merged cells (`<mergeCells>`) — (missing)
- [ ] frozen panes (`<sheetView pane>`) — (missing)
- [ ] split panes — (missing)
- [ ] auto-filter — (missing)
- [ ] conditional formatting — (missing)
- [ ] data validation — (missing)
- [ ] sheet protection — (missing)
- [ ] print area — (missing)
- [ ] row/column grouping (outline) — (missing)

## Rich text
- [ ] cell with rich text (multiple runs inside `<is>`) — (missing)
- [ ] rich text: bold run — (missing)
- [ ] rich text: color run — (missing)
- [ ] rich text: font size run — (missing)

## Embedded content
- [ ] chart — (missing)
- [ ] drawing / image — (missing)
- [ ] comment (`<comment>`) — (missing)
- [ ] sparkline — (missing)
- [ ] pivot table — (missing)
- [ ] table object (`<tableParts>`) — (missing)

## Workbook metadata
- [ ] title (core properties) — (missing)
- [ ] author — (missing)
- [ ] creation/modification date — (missing)
- [ ] last modified by — (missing)
- [ ] calculation mode (auto/manual) — (missing)

## Composition (integration)
- [ ] sheet with mixed value types in same column — (missing)
- [ ] formula referencing another sheet — (missing)
- [ ] merged cell spanning multiple rows and columns — (missing)
- [ ] table object with header row — (missing)
- [ ] chart derived from a data range — (missing)

## Adversarial
- [ ] malformed zip archive — (missing)
- [ ] missing xl/workbook.xml — (missing)
- [ ] missing shared strings file — (missing)
- [ ] corrupt styles.xml — (missing)
- [ ] cell with out-of-range style index — (missing)
- [ ] empty workbook (no sheets) — (missing)
- [ ] sheet with no cells — (missing)
- [ ] formula with circular reference — (missing)
- [ ] string with embedded null bytes — (missing)

## Pathological
- [ ] sheet with thousands of rows — (missing)
- [ ] sheet with hundreds of columns — (missing)
- [ ] workbook with hundreds of sheets — (missing)
- [ ] shared strings table with thousands of entries — (missing)
- [ ] cell with very long string value — (missing)
