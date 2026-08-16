# XLSX Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

**Coverage-completeness caveat (2026-07-28):** the checklist below is a hand-curated list of
constructs, not yet verified against a spec-derived, machine-readable construct index. An
audit of `fixtures/docbook/COVERAGE.md` and `fixtures/jats/COVERAGE.md` against authoritative
element indexes found hundreds of element names enumerated nowhere, moving denominators
mid-session purely from incidentally-noticed gaps -- a ratio over a hand-written list like this
one is not a coverage measurement. See `docs/format-audit.md`'s "Construct Coverage (CC)"
section for the full evidence; this format's `CC` status there is `U` (unverified) until a
construct registry (in design, see `docs/adr/`) checks this list against the format's own
spec.

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
- [x] date/time/currency/percentage values — `number-formats` (cell numFmtId classified via `ooxml_sml::classify_format_code` into `value:type`; a combined date+time format maps to `"date"`, matching ODF's own value-type convention for a full date-plus-time value; the underlying value stays the raw Excel serial number, not a converted date/time string)
- [lib] error values — ooxml-sml CellValue::Error emits fidelity warning; hard to construct via builder
- [x] empty cells — `adv-empty-sheet` (sheet with no data)

## Cell properties preserved in IR
- [x] xlsx:cell-type prop — `basic` (string cells get "s", numbers "n", booleans "b")
- [x] xlsx:formula prop — `formula`
- [x] xlsx:number_format prop (raw number-format code, paired with the semantic `value:type` projection) — `number-formats`
- [x] mixed cell types in one sheet — `cell-types-mixed`

## Sheet structure
- [x] header row (first row → table_header) — `basic`
- [x] data rows (table_cell) — `basic`
- [x] merged cells (fidelity warning, content preserved) — `merged-cells`
- [x] conditional formatting: cellIs, colorScale, dataBar, iconSet (modeled as `xlsx:conditional_format`/`xlsx:conditional_format_rule` child nodes on the sheet — OOXML-namespaced, not `rescribe-std` vocabulary, since ODF's only conditional-formatting representation is an unstable LibreOffice extension with no stable spec to validate a cross-format shape against; no fidelity warning needed) — `conditional-formatting`
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
- [x] number format — classified into `value:type` (percentage/currency/date/time/number, `number-formats`) and the verbatim format code preserved raw via `xlsx:number_format` for exact round-trip (font/color/fill/border/alignment styling is still `[lib]`, unaffected)

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
