# XLSX Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

Items marked `[lib]` are not exposed by the upstream `ooxml-sml` reader or represent
features that produce fidelity warnings and are not represented in the IR.

## Workbook structure
- [x] single sheet — `basic`
- [x] multiple sheets — `multi-sheet`
- [lib] sheet with tab color — not in WorkbookBuilder API
- [lib] hidden sheet — not in WorkbookBuilder API

## Cell value types
- [x] string values — `basic`, `cell-types-mixed`
- [x] numeric values (integer) — `numbers`
- [x] numeric values (float) — `numbers`
- [x] boolean values — `booleans`
- [x] formula cells — `formula`
- [lib] date/time values — ooxml-sml resolves dates to numbers; no semantic date type in IR
- [lib] error values — ooxml-sml CellValue::Error emits fidelity warning; hard to construct via builder
- [x] empty cells — `adv-empty-sheet` (sheet with no data)

## Cell properties preserved in IR
- [x] xlsx:cell-type prop — `basic` (string cells get "s", numbers "n", booleans "b")
- [x] xlsx:formula prop — `formula`
- [x] mixed cell types in one sheet — `cell-types-mixed`

## Sheet structure
- [x] header row (first row → table_header) — `basic`
- [x] data rows (table_cell) — `basic`
- [x] merged cells (fidelity warning, content preserved) — `merged-cells`
- [x] frozen panes (not in IR, content preserved) — `freeze-pane`
- [x] auto-filter (not in IR, content preserved) — `auto-filter`
- [x] column widths (not in IR, content preserved) — `column-widths`
- [x] row heights (not in IR, content preserved) — `row-heights`
- [lib] hidden rows/columns — not in WorkbookBuilder API

## Cell interactions
- [x] hyperlink (not in IR, cell text preserved) — `hyperlinks`
- [x] comment (not in IR, cell text preserved) — `comments`
- [lib] rich text in cell (multiple runs with different formatting) — ooxml-sml resolves to plain string
- [lib] cell validation — not represented in IR

## Cell formatting (all produce fidelity warnings)
- [lib] bold / italic / underline — style index detected; warning emitted; IR not updated
- [lib] font color / size / name — style index detected; warning emitted
- [lib] fill color — style index detected; warning emitted
- [lib] borders — style index detected; warning emitted
- [lib] alignment — style index detected; warning emitted
- [lib] number format — ooxml-sml resolves to raw number; format string not in IR

## Workbook metadata
- [lib] author / created date — not in WorkbookBuilder API
- [lib] title / subject / description — not in WorkbookBuilder API
- [lib] named ranges (defined names) — fidelity warning emitted; not in IR

## Adversarial
- [x] empty workbook (no sheets) — `adv-empty-workbook`
- [x] sheet with no data — `adv-empty-sheet`
- [x] malformed zip archive — `adv-malformed-zip`
- [x] empty bytes — `adv-empty-bytes`
- [lib] missing xl/workbook.xml — not constructible via WorkbookBuilder
- [lib] corrupt relationship file — not constructible via WorkbookBuilder

## Pathological
- [x] sheet with 50 data rows — `path-many-rows`
- [x] sheet with 10 columns — `path-many-columns`
- [x] workbook with 10 sheets — `path-many-sheets`
- [lib] very large numbers / NaN / Infinity — not constructible via WriteCellValue

## Composition
- [x] multi-sheet with mixed cell types — `mixed-content`
