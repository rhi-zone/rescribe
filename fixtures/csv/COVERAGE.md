# CSV Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

CSV (Comma-Separated Values) is defined by RFC 4180. The rescribe reader maps CSV to a
`table` with `table_header` cells for the first row and `table_cell` cells for data rows.

## Table structure

- [x] basic table (header + data rows) — `basic`
- [x] single-column table — `single-column`
- [x] three-column table — `three-columns`
- [ ] table with a single data row — (missing)
- [ ] table with many columns — (missing)

## Header handling

- [x] first row as header (table_header cells) — `basic`
- [x] header-only (no data rows) — `adv-header-only`
- [ ] no header (all rows as data) — (missing; format-reader option or convention)

## Field/cell types

- [x] plain unquoted text — `basic`
- [x] quoted field (field containing comma) — `quoted-field`
- [x] quoted field containing embedded quotes (`""`) — (missing)
- [x] empty field — `rare-empty-field`
- [ ] field containing newline — (missing)
- [ ] field containing only whitespace — (missing)
- [ ] numeric field (stored as text) — `basic` (age column is numeric string)
- [ ] field with leading/trailing whitespace — (missing)

## Delimiter / encoding

- [ ] CRLF line endings — (missing)
- [ ] LF-only line endings — `basic`
- [ ] file with BOM — (missing)
- [ ] UTF-8 encoded field values — (missing)

## Quoted field variants

- [x] field with comma inside quotes — `quoted-field`
- [x] rare: comma in field (alias for quoted-field) — `rare-comma-in-field`
- [ ] field with newline inside quotes — (missing)
- [ ] field with double-quote escaped as `""` — (missing)
- [ ] field with double-quote at start — (missing)

## Edge cases

- [x] empty file — `adv-empty`
- [x] empty field value — `rare-empty-field`
- [ ] trailing comma (empty last field) — (missing)
- [ ] rows with different column counts — (missing)
- [ ] file with only a newline — (missing)

## Composition (integration)

- [ ] table with quoted fields in header and data — (missing)
- [ ] table with empty cells interspersed — (missing)

## Adversarial

- [x] empty file — `adv-empty`
- [x] header row with no data rows — `adv-header-only`
- [ ] unclosed quote in field — (missing)
- [ ] row with more fields than header — (missing)
- [ ] row with fewer fields than header — (missing)
- [ ] non-UTF-8 bytes in field — (missing)

## Pathological

- [ ] table with 10,000 rows — (missing)
- [ ] table with 1,000 columns — (missing)
- [ ] single field containing 1 MB of text — (missing)
- [ ] deeply quoted nested commas — (missing)
