# 16. Chart IR shape: `chart`/`chart_series` node kinds and cell-range reference properties

## Status

Accepted (design only). Implementation not started — see "Consequences" for the
follow-up items this ADR opens but does not perform, matching the pattern ADR 0015 used
(decided and recorded first, implementation a separate follow-up).

## Context

Charts are currently dropped entirely from rescribe's IR: no `chart` node kind exists
anywhere in `rescribe-std`. Both OOXML backends already detect charts and emit an
explicit fidelity warning rather than silently dropping them, confirmed this session —
`crates/bridges/rescribe-fmt-ooxml/src/xlsx.rs:164-169` ("Chart(s) detected in sheet
\"{}\" ({} chart(s)); chart data not represented in IR (no chart node kind exists yet)")
and `crates/bridges/rescribe-fmt-ooxml/src/pptx/read.rs:62-68` ("Slide {}: {} embedded
chart(s) detected; chart data not represented in IR"). That is the correct interim
behavior per CLAUDE.md's "fidelity warnings are not optional" rule, but the underlying
gap — no way to represent chart content at all — remains.

This session's scoping investigation (read-only, no code changes) found:

- OOXML's full chart schema (ECMA-376 §21.2) is already codegen'd in
  `crates/formats/ooxml-dml/src/generated.rs` (~946 lines, ~130 distinct types), gated
  behind a `dml-charts` Cargo feature (off by default; `ooxml-pml` has a matching
  `pml-charts` feature). It covers bar/line/pie/scatter/radar/stock/bubble/surface/
  doughnut/of-pie charts and their 3D variants, `PlotArea`, `Legend`, category/value/
  date/series axes, `ChartTitle`, `Trendline`, `ErrorBars`, and `DataLabels`/
  `DataPoint`.
- Series data resolves via `AxisDataSource`/`NumericDataSource`, which point to either a
  `NumericReference`/`StringReference` (a cell-range formula string, e.g.
  `Sheet1!$B$2:$B$5`, paired with an optional cached `NumericData`/`StringData`
  snapshot) or literal inline data — both are valid per schema.
- `ooxml-sml/src/workbook.rs` already hand-parses a simplified `Chart{title,
  chart_type}` via its own ad hoc XML walk (not through the generated model);
  `ooxml-pml/src/writer.rs` already writes `ppt/charts/chart1.xml` on the write side.
  Neither goes through rescribe's IR — both are format-crate-internal today.
- ODF has an equivalent, independently structured schema, also already partially
  codegen'd in `odf-fmt/src/generated.rs` (`ChartSeriesAttlist`, `ChartAxisAttlist`,
  `ChartPlotAreaAttlist`, `ChartLegendAttlist`, etc.):
  `office:chart` → `chart:chart` → `chart:plot-area` → `chart:series`/`chart:axis`,
  with data sourced via a `table:cell-range-address` attribute on the series/axis
  element (per the OASIS ODF spec, confirmed this session — this attribute moved from
  `plot-area` to series/axis level in ODF 1.4).
- **Key finding**: in both formats, independently, chart data is fundamentally a
  *reference* into spreadsheet cells — not a coincidence, the same underlying shape.
  `rescribe-std/src/lib.rs` currently has no cell-range-reference concept at all
  (checked this session). This is genuinely new IR ground, not reusable from ADR
  0015's sheet/cell work, though it must interoperate with that shape: a chart series
  needs to be able to point at a `sheet`/`sheet_row`/`sheet_cell` range within the same
  document.
- Complexity split: a minimal viable scope (chart type, title, series with
  embedded-or-referenced values, axis/legend presence) is tractable in one pass. Full
  long-tail coverage — 10+ chart types, 3D variants, combo charts, trendlines, error
  bars, per-point styling across ~130 OOXML types plus ODF's parallel schema — is a
  multi-day vertical, not same-day work.

Four sub-decisions were needed: how a chart series references a spreadsheet cell range,
whether OOXML's cached/literal data snapshot is preserved, whether chart type is an open
string or a closed enum, and where the v1 semantic/raw-preservation boundary sits.

## Decision

### 1. Cell-range reference as a structured property, not a new node kind

A chart series needs to express "these values come from this range of cells," optionally
paired with a cached snapshot. This is genuinely shared cross-format vocabulary —
OOXML's `NumericReference`/`StringReference` and ODF's `table:cell-range-address` are the
same concept (a reference into spreadsheet cell data), independently arrived at, checked
against both formats' native models rather than assumed from one — the same
cross-format-verification bar ADR 0005 and ADR 0015 Decision 1 already apply.

That clears the bar for real semantic modeling (not raw preservation), but does not by
itself decide *how* it's represented in the IR: as a new node kind, or as a property
value on `chart_series`. `PropValue` already has a `Map(HashMap<String, PropValue>)`
variant (`crates/rescribe-core/src/properties.rs:13-20`) capable of carrying a
reference-plus-cache structure directly. ADR 0015 Decision 2 already established the
precedent for this kind of choice: a `sheet_cell`'s typed value is modeled as properties
directly on the cell node ("not nested in a child paragraph"), rejecting a child-node
shape for what is fundamentally a leaf value attached to its owner. A cell-range
reference is the same shape of thing — a leaf value describing where a series' data
comes from, not itself a piece of document structure with its own children — so the
same resolution applies here: a property, not a node kind.

Decision: `chart_series` carries `chart:values-ref` (`PropValue::String`, the verbatim
range-reference formula string — OOXML's `Sheet1!$B$2:$B$5` syntax or ODF's
`table:cell-range-address` syntax, stored as-is; translating between the two syntaxes is
an implementation detail of each format's `rescribe` module, not decided by this ADR) when
the series is reference-backed, and `chart:categories-ref` analogously for the category/
axis data source. Literal (non-referenced) series carry values directly via
`chart:values`/`chart:categories` (`PropValue::List`) with no `-ref` property set.

### 2. Cached snapshot preserved alongside the reference

OOXML's `numCache`/`StrCache` let a chart carry a snapshot of the referenced values so a
reader without spreadsheet access can still render something meaningful. Per CLAUDE.md's
losslessness principle, dropping this would lose real information present in the source
file — a `parse(emit(parse(input)))` round-trip would produce a chart with no fallback
data even though the original file had one.

Decision: when a series is reference-backed, `chart:values`/`chart:categories`
(`PropValue::List`) hold the cached snapshot alongside `chart:values-ref`/
`chart:categories-ref`. This reuses the same property names as the literal-data case
(Decision 1) — presence of the paired `-ref` property is what distinguishes "these are
the only values" (literal) from "these are a cached snapshot of a live reference"
(reference-backed), avoiding a third property name for what is structurally the same
list-of-values shape either way. ODF's schema has no equivalent cache mechanism (checked
this session) — an ODF-sourced chart's series will have `-ref` set with no cache, which
is a faithful representation of what ODF actually stores, not a gap in the IR.

### 3. Chart type: open string, matching `NodeKind`'s existing convention

Checked this session: every node kind and construct-vocabulary property in
`rescribe-std/src/lib.rs` (`sheet`, `sheet_cell`'s `value:type`, etc.) is an open string
constant — there is no closed Rust enum anywhere in the crate for a similarly finite
category, including cases like `value:type` that already have a fixed, spec-defined set
of legal values (string/number/currency/percentage/date/time/boolean/formula-result,
ADR 0015 Decision 2). Chart type (bar/line/pie/scatter/...) is the same shape of thing:
finite per-spec today, but a closed Rust enum would need a breaking IR change every time
a future format or future OOXML/ODF revision adds a chart type, where an open string
with documented known values does not.

Decision: `chart:type` is `PropValue::String`, following the `NodeKind` open-string
convention. The known-value vocabulary (documented, not enforced by the type system) is
the union of OOXML's and ODF's chart-type sets, established when the implementation
lands — not enumerated in this ADR, to avoid this document going stale as either
schema's type list evolves.

### 4. v1 scope: semantic core plus a full raw-XML fallback, not a semantic-only cutoff

New node kinds: `chart` (block-level, siblings with other block content) with children
`chart_series` (one per data series). `chart` carries `title` (existing `TITLE`
constant, reused — not a new property), `chart:type`, and legend/axis
*presence* only: `chart:legend` (bool) plus `chart:legend-position` (open string,
e.g. `right`/`bottom`/`top`/`left`, when present), `chart:has-category-axis` (bool),
`chart:has-value-axis` (bool). `chart_series` carries `title`, and the values/categories
properties from Decisions 1-2.

Everything beyond that — 3D variants, combo charts, trendlines, error bars, per-point
styling, detailed axis formatting — is out of v1's semantic model. Per CLAUDE.md's
two-tier preservation model (semantic modeling vs. raw preservation) and the precedent
ADR 0015 Decision 5 set for ODF's non-center-pivot rotation case, none of that content is
dropped: the `chart` node also carries the full verbatim chart-part XML as a
format-namespaced raw property (`ooxml:chart-xml` for OOXML's `chart1.xml` part,
`odf:chart-xml` for ODF's `office:chart` subtree) alongside the semantic properties. This
differs from ADR 0015's rotation case in one respect: there, the raw fallback only
activates when the semantic projection is lossy (non-pure transforms). Here, the raw
fallback is unconditional in v1, because the semantic model itself is intentionally a
subset (not a lossy projection of a superset) — the raw property is what makes v1 a
correct partial implementation rather than a lossy one: a v1 reader populates the
semantic fields it knows how to populate *and* the raw blob always, so a v1 writer can
always reconstruct the original chart exactly by re-emitting the raw blob, even for
constructs the semantic model doesn't yet cover. A later pass that extends the semantic
model (trendlines, 3D, etc.) narrows what falls only into the raw blob; it does not
change this ADR's shape.

## Consequences

Not yet implemented — this ADR records the shape only. Follow-up work, in the order
CLAUDE.md's priority hierarchy would suggest (fixture suite / cross-format-verified
shape before single-consumer wiring):

- Add `chart`, `chart_series` node kinds and the `chart:*` properties to
  `crates/nodes/rescribe-std/src/lib.rs`.
- Wire `rescribe-fmt-ooxml/src/xlsx.rs` and `pptx/read.rs` (and their write sides) to
  populate `chart`/`chart_series` instead of only emitting the current fidelity warning,
  using `ooxml-dml`'s `dml-charts`-gated generated types (already codegen'd, per the
  Context section) rather than `ooxml-sml/src/workbook.rs`'s existing ad hoc XML walk —
  that hand-rolled parser should be superseded by the generated model going through this
  IR shape, the same way ADR 0015 superseded `xlsx.rs`'s original `table`/`table_cell`
  workaround.
- Wire `odf-fmt`'s `rescribe` module for `office:chart`/`chart:chart`.
- Fixture suite: `fixtures/xlsx/`, `fixtures/pptx/`, `fixtures/odp/`/`fixtures/ods/`
  chart fixtures, per CLAUDE.md's "no new feature without a fixture" rule.
- This ADR does not decide whether chart support belongs in the OOXML DrawingML crate
  work already tracked for `ooxml-fmt`, or is scoped as its own vertical slice — that is
  a scheduling decision for whoever picks this up, not a design decision this ADR needs
  to make.

## Alternatives considered

- **A new node kind wrapping the cell-range reference** (e.g. a `cell_range_ref` child
  node under `chart_series`) instead of a property: rejected on the same grounds ADR
  0015 Decision 2 rejected nesting a `sheet_cell`'s value in a child paragraph — the
  reference is a leaf value describing the series' data source, not itself structural
  content with children of its own, so it belongs in the property bag `PropValue::Map`
  already supports.
- **Dropping the OOXML cache and re-deriving series values from the live spreadsheet on
  read**: rejected — a chart's source `.xlsx`/`.pptx` is not guaranteed to travel
  alongside the chart when only the chart XML part is being read (e.g. a chart embedded
  in a PPTX with its data source in a since-detached XLSX), and even when it is,
  discarding the cache and always re-deriving is a silent drop of information actually
  present in the source file, which CLAUDE.md's losslessness principle rules out.
- **Closed Rust enum for `chart:type`**: rejected — no other similarly finite,
  spec-bounded vocabulary in `rescribe-std` (including `value:type`, which is at least as
  finite) uses a closed enum; adding one here would be inconsistent with the crate's one
  existing convention and would force a breaking IR change on every future chart-type
  addition.
- **Semantic-only v1 with no raw-XML fallback, deferring unmodeled constructs entirely to
  a later ADR**: rejected — that would make v1 a lossy implementation (a source file with
  a trendline would silently lose the trendline through v1's read/write cycle), which is
  exactly the silent-drop failure mode CLAUDE.md's "fidelity warnings are not optional"
  section prohibits. Pairing the v1 semantic subset with an always-present raw blob keeps
  v1 correct (lossless) while still being a partial implementation (not yet queryable as
  structured data beyond the v1 fields).
