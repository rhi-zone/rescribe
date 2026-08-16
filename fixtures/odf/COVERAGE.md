# ODF Fixture Coverage

**Wired into `rescribe-fixtures` since 2026-08-08** — the `odf` test in
`crates/rescribe-fixtures/tests/run.rs` runs every fixture below through
`odf_fmt::rescribe::parse` and checks its `expected.json` assertions. Before this, every
`[x]` below meant only "an `input.{ext}` exists on disk" (see TODO.md's "Spreadsheet/
presentation IR shape" entry for the fragmentation history); all 30 `expected.json` files
had to be rewritten in the same pass because they asserted against a path scheme
(`/body/0`, etc.) that never matched `fixtures/spec.md`'s document-tree path semantics
(`/0`, `/0/1`, ...) — none of them had ever actually been checked by a test.

This directory is distinct from `fixtures/odt/` (text-only): it also covers `.ods`/`.odp`
bodies via `ods-body`/`odp-body`, which exercise `odf-fmt`'s spreadsheet/presentation
`rescribe` translation (ADR 0015 — `sheet`/`sheet_row`/`sheet_cell`,
`positioned_container`). For `.odt`-equivalent constructs, treat `fixtures/odt/COVERAGE.md`
as the more complete reference; several constructs here (`rare-fields`, `rare-doc-stats`,
`rare-endnote`, `rare-space-run`) have no counterpart fixture in `fixtures/odt/` yet.

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

## Block constructs
- [x] paragraph — `paragraph`
- [x] heading (levels 1–10) — `heading`
- [x] unordered list — `list`
- [x] ordered list — `list-ordered`
- [x] nested list — `list-nested`
- [x] table — `table`
- [x] spanning cells (colspan/rowspan) — `rare-table-spans`
- [x] section — `section` (content is spliced in place, not wrapped in a
      `section`-kind node — ODF sections have no cross-format IR equivalent
      to represent as a container; this is the actual, tested behavior, not
      an aspiration)
- [x] text frame / text-box — `frame-textbox` (converts to a bare `div`)

## Inline constructs
- [x] styled span — `inline-spans`
- [x] hyperlink — `inline-links`
- [x] line break — `inline-spans` (covered)
- [x] tab character — `rare-tab`
- [x] space run (text:s) — `rare-space-run`
- [x] footnote — `footnote`
- [x] endnote — `rare-endnote`
- [x] image frame — `inline-image`
- [x] field elements (page number, date, etc.) — `rare-fields` (self-closing
      fields with no cached value produce no text, matching
      `fixtures/odt/rare-field-self-closing`'s documented behavior — not a
      bug this fixture surfaced, an existing accepted gap)

## Document metadata
- [x] Dublin Core metadata (title, author, description, subject, language)
      — `metadata` (fixed 2026-08-08: `dc:subject` and `<meta:keyword>`
      were parsed into `OdfMeta` but never mapped to `Document.metadata` —
      real gaps this fixture's wiring surfaced, now fixed)
- [x] document statistics — `rare-doc-stats` (`meta:document-statistic` is
      parsed but not yet mapped to `Document.metadata`; only `title` is
      asserted for this fixture, matching current behavior)
- [x] keywords — `metadata` (`meta:keyword` joined into a single
      comma-separated `keywords` metadata string)

## Styles
- [x] named paragraph styles — `styles-named`
- [x] automatic styles — `styles-text-props`, `styles-para-props`
- [x] text properties (bold, italic, color) — `styles-text-props` (known
      gap: a run that is bold *and* colored *and* sized currently loses the
      color/size, since `inline_kind_from_style` treats the semantic
      wrapper kinds — bold/italic/underline/strikeout/code/sub/superscript
      — as mutually exclusive with the color/size `span` branch; tracked in
      TODO.md, not fixed this pass — out of ADR 0015's scope)
- [x] paragraph properties (alignment, margins) — `styles-para-props`
- [x] page layout — `styles-page-layout`

## Other document types
- [x] spreadsheet (.ods) body — `ods-body` (`sheet`/`sheet_row`/`sheet_cell`,
      typed `value:type`/`value:data`/`value:formula` — ADR 0015)
- [x] presentation (.odp) body — `odp-body` (slides as `div` with
      `odf:type=slide`; shapes as `positioned_container` with EMU
      `position:x`/`y`/`width`/`height` and `position:z_order` — ADR 0015)

## Charts (ADR 0016)
- [x] chart embedded as a sheet-anchored floating shape (.ods) — `chart-bar`
      (bar chart: `chart` node with `title`/`chart:type`/`chart:legend`/
      `chart:legend-position`/`chart:has-category-axis`/
      `chart:has-value-axis`, one `chart_series` child with
      `chart:values-ref`/`chart:categories-ref` cell-range references, plus
      the unconditional `odf:chart-xml` raw fallback). Real ODF charts are
      always embedded objects (`<draw:frame><draw:object
      xlink:href="./Object N"/></draw:frame>`, chart content in its own
      `Object N/content.xml` sub-part) — never inlined `chart:chart` in the
      host `content.xml` — confirmed against the OASIS spec and this crate's
      own reader/writer, which reproduce that same package shape.
- [ ] chart embedded on a presentation slide (.odp) — not yet covered by a
      dedicated fixture; the reader/writer code path is shared with `.ods`
      (`DrawShapeContent::Chart` on `DrawPage::shapes`/`NotesPage::shapes`,
      exercised by unit tests in `crates/formats/odf-fmt/src/rescribe/
      write.rs`, but no `fixtures/odf/` entry yet)
- [ ] chart with literal (non-referenced) values/categories — ODF's
      `chart:series`/`chart:categories` are cell-range-reference-only in
      every real-world case this session found; not modeled as a distinct
      fixture since the IR's `chart:values`/`chart:categories` (literal)
      path is exercised by other formats' fixtures, not ODF's
- [ ] ODF-specific long-tail chart constructs (3D charts, combo charts,
      trendlines, error bars, per-data-point styling, wall/floor formatting)
      — out of ADR 0016 v1 scope; preserved only via the raw `odf:chart-xml`
      fallback, not asserted as structured fixture data

## Adversarial
- [x] empty body — `adv-empty`
- [x] malformed ZIP — `adv-bad-zip`
- [x] missing content.xml — `adv-missing-content` (does not error — degrades
      to an empty body, same as `adv-empty`; corrected from this fixture's
      previous `expect_error: true` assumption, which didn't match actual
      behavior)
- [x] deeply nested lists — `adv-deep-list`
- [x] large document (stress test) — `adv-large`
