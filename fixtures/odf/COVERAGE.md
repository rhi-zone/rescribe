# ODF Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

## Block constructs
- [x] paragraph — `paragraph`
- [x] heading (levels 1–10) — `heading`
- [x] unordered list — `list`
- [x] ordered list — `list-ordered`
- [x] nested list — `list-nested`
- [x] table — `table`
- [x] spanning cells (colspan/rowspan) — `rare-table-spans`
- [x] section — `section`
- [x] text frame / text-box — `frame-textbox`

## Inline constructs
- [x] styled span — `inline-spans`
- [x] hyperlink — `inline-links`
- [x] line break — `inline-spans` (covered)
- [x] tab character — `rare-tab`
- [x] space run (text:s) — `rare-space-run`
- [x] footnote — `footnote`
- [x] endnote — `rare-endnote`
- [x] image frame — `inline-image`
- [x] field elements (page number, date, etc.) — `rare-fields`

## Document metadata
- [x] Dublin Core metadata (title, creator, etc.) — `metadata`
- [x] document statistics — `rare-doc-stats`
- [x] keywords — `metadata` (keywords field covered)

## Styles
- [x] named paragraph styles — `styles-named`
- [x] automatic styles — `styles-text-props`, `styles-para-props`
- [x] text properties (bold, italic, color) — `styles-text-props`
- [x] paragraph properties (alignment, margins) — `styles-para-props`
- [x] page layout — `styles-page-layout`

## Other document types
- [x] spreadsheet (.ods) body — `ods-body`
- [x] presentation (.odp) body — `odp-body`

## Adversarial
- [x] empty body — `adv-empty`
- [x] malformed ZIP — `adv-bad-zip`
- [x] missing content.xml — `adv-missing-content`
- [x] deeply nested lists — `adv-deep-list`
- [x] large document (stress test) — `adv-large`
