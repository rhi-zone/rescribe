# ODF Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

## Block constructs
- [x] paragraph — `paragraph`
- [x] heading (levels 1–10) — `heading`
- [x] unordered list — `list`
- [x] ordered list — `list-ordered` (TODO)
- [x] nested list — `list-nested` (TODO)
- [x] table — `table`
- [ ] spanning cells (colspan/rowspan) — `rare-table-spans`
- [ ] section — `section`
- [ ] text frame / text-box — `frame-textbox`

## Inline constructs
- [x] styled span — `inline-spans`
- [x] hyperlink — `inline-links`
- [x] line break — `inline-spans` (covered)
- [ ] tab character — `rare-tab`
- [ ] space run (text:s) — `rare-space-run`
- [ ] footnote — `footnote`
- [ ] endnote — `rare-endnote`
- [ ] image frame — `inline-image`
- [ ] field elements (page number, date, etc.) — `rare-fields`

## Document metadata
- [x] Dublin Core metadata (title, creator, etc.) — `metadata`
- [ ] document statistics — `rare-doc-stats`
- [ ] keywords — `rare-keywords`

## Styles
- [ ] named paragraph styles — `styles-named`
- [ ] automatic styles — `styles-automatic`
- [ ] text properties (bold, italic, color) — `styles-text-props`
- [ ] paragraph properties (alignment, margins) — `styles-para-props`
- [ ] page layout — `styles-page-layout`

## Other document types
- [ ] spreadsheet (.ods) body — `ods-body`
- [ ] presentation (.odp) body — `odp-body`

## Adversarial
- [ ] empty body — `adv-empty`
- [ ] malformed ZIP — `adv-bad-zip`
- [ ] missing content.xml — `adv-missing-content`
- [ ] deeply nested lists — `adv-deep-list`
- [ ] large document (stress test) — `adv-large`
