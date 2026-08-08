# 15. Spreadsheet/presentation IR shape: `sheet`/`sheet_row`/`sheet_cell`, `positioned_container`, EMU coordinates

## Status

Accepted (design only — see "Consequences" for what remains unimplemented).

## Context

`odf-fmt` needs spreadsheet (`.ods`) and presentation (`.odp`) IR translation for the
first time — currently only `.odt`/text is supported. Direct code read this session
confirmed `crates/formats/odf-fmt/src/rescribe/{read,write}.rs`: the `Spreadsheet`/
`Presentation` body variants return `ParseError::Invalid` on read, and the writer never
produces them.

This is the first time this repo has modeled spreadsheet or presentation content in the
`Document` IR, with one partial exception: `crates/bridges/rescribe-fmt-ooxml/src/xlsx.rs`
already reuses `table`/`table_row`/`table_header`/`table_cell` for real XLSX spreadsheet
content today. That is an existing, already-shipped precedent, but an imperfect one — cell
type/formula are tagged as `xlsx:cell-type`/`xlsx:formula` properties on the *paragraph
nested inside the cell*, not the cell itself, because content is forced through a
paragraph→text→string pipeline; multi-sheet structure is flat `heading`/`table` sibling
pairs with no workbook container; merged cells, conditional formatting, and charts are
dropped with fidelity warnings rather than modeled. This ADR treats that implementation as
prior art to learn from, not as a settled shape to preserve unchanged (see Decision 2).

Four sub-decisions were needed: where new vocabulary lives, whether spreadsheet cells reuse
the existing `table_cell` kind, how absolute positioning (needed for presentation shapes,
and also DOCX/RTF text-boxes) is represented, and what unit system backs positioning
coordinates.

## Decision

### 1. Vocabulary placement: `rescribe-std`, not `odf:`-namespaced

New sheet/cell/slide/shape node kinds go into `rescribe-std`'s shared cross-format
vocabulary (`crates/nodes/rescribe-std/src/lib.rs`), following the precedent set by
`bibliography`/`bibliography_entry`/`bibliography_field` (ADR 0005, commit `4e15c9966e`).
That precedent was schema-verified against multiple formats' actual data models (DocBook,
JATS, TEI, OOXML) *before* being added — validated by cross-format research, not by having
2+ already-implemented consumers. The same method applies here: shape the new kinds against
both ODF's and OOXML SpreadsheetML/PresentationML's (`ooxml-sml`/`ooxml-pml`) native data
models, both already in-repo though neither yet exposes a `rescribe` module, rather than
deriving the shape from ODF alone and hoping it fits OOXML later.

This is scoped narrowly to the IR-adapter layer. CLAUDE.md's priority hierarchy ranks
rescribe integration (level 4) below Rust-ecosystem format-crate work (levels 2-3); this
decision does not claim spreadsheet/presentation IR work should be prioritized generally,
only records the shape it should take whenever it is done.

### 2. Distinct `sheet`/`sheet_row`/`sheet_cell` kinds, not reused `table`/`table_cell`

A spreadsheet cell's primary content — a typed scalar or formula — is a fundamentally
different content contract than a prose table cell's (block/inline content). This is not
"the same concept with extra properties layered on." This is the same test ADR 0014 applies
to reject crate merges (genuine shared contract vs. topical/container-name similarity only),
and it resolves the same way here: distinct kinds, not a shared one.

New node kinds:

- `sheet` — one worksheet; children are `sheet_row` nodes. A future `workbook` container
  (multi-sheet grouping) is left for the implementer, not decided here.
- `sheet_row` — one row; children are `sheet_cell` nodes.
- `sheet_cell` — one cell. Value is modeled as a typed scalar directly on the cell node
  (via a `value:type`-tagged property plus the value itself), not nested in a child
  paragraph.

Cell value types are informed by both formats' native models, checked this session:
`ooxml-sml`'s `CellValue` enum (`crates/formats/ooxml-sml/src/ext.rs:39-50`) is narrower —
`Empty`/`String`/`Number`/`Boolean`/`Error` only, with `Date`/`Currency` resolved indirectly
via a number-format string — while ODF's `office:value-type` distinguishes string, number,
currency, percentage, date, time, and boolean as first-class types. The IR takes the union:
string, number, currency, percentage, date, time, boolean, and formula-result, so that
readers from either format can populate a real type rather than downgrading to string
(OOXML) or losing the extra distinctions (ODF-to-OOXML-shaped IR). Formula source text is a
separate property from the computed result value, so both survive round-trip.

This means `rescribe-fmt-ooxml/src/xlsx.rs`'s current `table`/`table_cell`-based
implementation is superseded by this shape and should migrate. That migration is a
follow-up item (see Consequences), not performed by this ADR.

### 3. New `positioned_container` node kind for absolute positioning

Nothing in the current flow-document IR expresses absolute (x/y/width/height/rotation/
z-order) positioning — needed for presentation shapes/slides, and also DOCX text-boxes,
PPTX shapes, and RTF `\shp`/`\do` groups.

The existing `layout:` property namespace
(`crates/nodes/rescribe-std/src/lib.rs:191-198`) was checked and found to be mostly
aspirational: only `LAYOUT_PAGE_BREAK` and `LAYOUT_COLUMN` have real consumers (both
boolean flags on in-flow blocks, docx-only), and `LAYOUT_FLOAT` has zero usages anywhere.
None of the three carry coordinates. RTF's shape control words (`\shpleft`/`\shptop`/etc.,
`crates/formats/rtf-fmt/src/parse.rs:1291-1373`) are currently recognized but silently
dropped — confirmed to be an existing gap/debt, not usable precedent for how to preserve
this content.

DOCX text-boxes, PPTX shapes, ODP shapes, and RTF `\shp`/`\do` all independently need the
identical field set (x, y, width, height, rotation, z-order). Per CLAUDE.md's own
semantic-vs-raw test, that repetition across four independent formats is a genuine
cross-format semantic concept, not format-specific trivia — so it gets a real node kind
with first-class properties (`position:x`, `position:y`, `position:width`,
`position:height`, `position:rotation`, `position:z_order`), not raw-namespaced flags
bolted onto existing block kinds. A `positioned_container`'s children are the shape's actual
content (text, image, or nested blocks), unconstrained by this decision.

### 4. Canonical unit: EMU (`i64`), paired with a raw-format fallback property

Verified this session:

- OOXML DrawingML's `ST_Coordinate` is EMU (914,400/inch, 360,000/cm), typed `xsd:long`
  (64-bit), origin top-left, y-down.
- ODF's `svg:x`/`svg:y`/`svg:width`/`svg:height` (on `draw:frame`, typed
  `text:coordinate`/`text:length`) are arbitrary-precision decimal strings with a mandatory
  unit suffix, also top-left/y-down origin.
- RTF uses twips (1/1440 inch) for `\tposx`/`\tposy` (confirmed by spec quote), and by
  RTF's uniform-twips convention presumably also for `\shpleft`/`\shptop`/`\dpxsize`/
  `\dpysize` — not independently spec-quoted for those specific control words, flagged here
  as inferred, not directly confirmed.

Precision check: the EMU/twip ratio is exactly 914400/1440 = 635, an integer, so twip→EMU
is always exact, and EMU→twip is exact for any EMU value that is a multiple of 635 (true
for anything RTF-native). ODF's arbitrary-precision decimal is the one case with no
theoretical round-trip guarantee — its schema regex permits unbounded decimal digits, which
in principle exceeds EMU's fixed resolution, though EMU's ~27.8nm resolution swallows any
realistic authored value in practice.

Decision: store the semantic value as EMU (`i64`) on the `positioned_container`'s
`position:*` properties, **and** pair it with a raw format-namespaced property (`odf:x`,
`odf:width`, etc. — the verbatim decimal+unit string) so ODF round-trips stay byte-exact
regardless of the EMU projection. This follows this repo's existing raw-preservation
convention (CLAUDE.md's "Raw preservation" section): format-specific properties capture
whatever the semantic model doesn't fully cover.

Origin/axis convention (top-left origin, y increasing downward) is identical across all
three formats checked, so no flip or coordinate-system translation is needed in either
direction. `i64` EMU gives a range of roughly 159 million miles at the extremes, matching
OOXML's own native type's stated range exactly — not a narrowing.

### 5. Rotation: EMU-style pattern extended, ODF projection is lossy in general

Verified in a follow-up session:

- ODF has no dedicated rotation-angle attribute. Rotation is expressed via `svg:transform`,
  using an SVG-style transform-list grammar (`rotate(angle)`, combinable with `translate()`/
  `scale()`/`matrix()`). The angle uses ODF's general `angle` datatype — unbounded-precision
  decimal degrees by default, with optional `deg`/`grad`/`rad` suffixes.
- The rotation pivot (the "origin of the shape's coordinate system" that `rotate()` turns
  around) is a documented, real-world interop ambiguity in the ODF spec itself — not
  consistently agreed on across implementations, per a cited OASIS-spec-quoting source (an
  OpenOffice/ODF-TC contributor draft).
- OOXML's `ST_Angle` is, by contrast, a fixed 60,000ths-of-a-degree integer, positive =
  clockwise, pivoted at the shape's bounding-box center — a well-established, unambiguous
  convention.
- RTF's legacy `\shp` shape properties predate rotation support entirely: no dedicated
  control word exists. Rotation would ride through RTF's generic named-property escape
  hatch if present at all.

Resolution: the same pattern as Decision 4's coordinate/EMU case. `position:rotation`
stores the OOXML-style 60,000ths-of-a-degree integer projection, paired with a raw
`odf:transform` property holding the verbatim `svg:transform` string (preserving pivot
ambiguity and any combined translate/scale) so ODF round-trips stay exact regardless of the
projection. OOXML→ODF is exact (a decimal string can represent the integer precisely).
ODF→OOXML is **not** losslessly convertible in general — confirmed, not assumed — and is
only a clean projection when the source transform is a pure `rotate()` around a
center-equivalent pivot; anything else (a non-center pivot, or a `rotate()` combined with
`translate()`/`scale()`/full `matrix()`) needs a fidelity warning on write, not a silent
projection.

### 6. Z-order: fully compatible across all three formats, no fallback needed

Verified in a follow-up session:

- ODF has an explicit `draw:z-index` (OASIS ODF spec §19.231); values increase back-to-front
  and are independent of document order.
- RTF has an explicit `shpz`.
- OOXML has no explicit field: stacking is purely document order within `spTree`.

Resolution: `position:z_order` stores an explicit integer always — derived from document
order when reading OOXML-sourced content, read directly from ODF's `draw:z-index` or RTF's
`shpz` otherwise. This round-trips losslessly in all directions: to ODF/RTF's explicit field
directly, and to OOXML's implicit order by re-sorting children by the `z_order` value on
write. Unlike rotation, no raw-property fallback is needed here — there is no precision-loss
risk, only an explicit-vs-implicit representational difference that a canonical explicit
field resolves cleanly.

## Consequences

- This ADR records the shape; it does not implement it. Still to be done, tracked
  separately in TODO.md:
  - `odf-fmt`'s spreadsheet/presentation `rescribe` read/write translation, built against
    this shape.
  - The new node kinds (`sheet`, `sheet_row`, `sheet_cell`, `positioned_container`) and
    `position:*`/`value:*` properties actually added to `rescribe-std`.
  - `rescribe-fmt-ooxml/src/xlsx.rs` migrated off `table`/`table_cell` onto `sheet`/
    `sheet_row`/`sheet_cell`, superseding its current paragraph-nested property placement.
  - Rotation/z-order semantics for ODF are now verified (Decisions 5-6); the `odf-fmt`
    read/write translation should implement `position:rotation`/`odf:transform` and
    `position:z_order` per those decisions, including the write-side fidelity warning for
    non-projectable ODF rotations (non-center pivot, or `rotate()` combined with other
    transform-list functions).
- Once implemented, `xlsx.rs` stops being a second, permanently-diverging pattern for
  spreadsheet content — there is one IR shape for spreadsheets, not one per format adapter.
- `positioned_container` becomes available to RTF's currently-dropped `\shp`/`\do` shape
  control words as well, closing a documented existing gap, not just serving ODF/OOXML.
- The EMU-plus-raw-property pattern (semantic projection paired with a raw fallback
  property scoped to the one format whose native precision the projection can't
  theoretically guarantee) is usable as precedent for future format-specific
  arbitrary-precision-vs-fixed-resolution conflicts, not just this one.

## Alternatives considered

- **Reuse `table`/`table_cell` for sheets, with cell-type properties on the cell** (a
  refinement of the current `xlsx.rs` approach rather than a new kind): rejected by the same
  test ADR 0014 uses for crate merges — a spreadsheet cell's content contract (typed scalar/
  formula) and a table cell's (block/inline prose) are not the same concept, and forcing one
  into the other's shape is what produced `xlsx.rs`'s existing paragraph-nesting workaround
  in the first place.
- **`odf:`/`ooxml:`-namespaced sheet/cell kinds per format** instead of shared
  `rescribe-std` kinds: rejected on the same grounds ADR 0005 already settled for
  bibliography — cross-format research showed the underlying concept (typed cell, sheet,
  positioned shape) is genuinely shared, not format-specific, once both formats' native
  models were actually checked rather than assumed from one format alone.
- **`layout:`-namespaced properties on existing block kinds for positioning**, instead of a
  new `positioned_container` kind: rejected because the properties in question (x/y/width/
  height/rotation/z-order) are not decorations on an otherwise-normal in-flow block — the
  defining characteristic of this content is that it is *not* in-flow, which needs a node
  kind boundary, not a flag.
- **Twips or ODF's native decimal string as the canonical unit**, instead of EMU: rejected
  because EMU already has a defined, unambiguous, spec-stated 64-bit range and is exact for
  the twip conversion (the RTF case); ODF's arbitrary-precision decimal has no fixed
  resolution to serve as a canonical wire type, which is precisely why it is preserved
  raw alongside the EMU projection rather than promoted to canonical status itself.
- **Storing ODF's verbatim `svg:transform` string as the sole canonical rotation
  representation**, instead of an OOXML-style integer projection paired with a raw
  fallback: rejected for the same reason twips/ODF-decimal was rejected as the canonical
  coordinate unit — it would make every non-ODF reader/writer parse an SVG transform-list
  grammar just to read a rotation angle, when only ODF has the pivot/combined-transform
  ambiguity in the first place.
- **Silently rounding all ODF rotations to the nearest OOXML 60,000th on write**, instead of
  gating on a fidelity warning: rejected because a non-center pivot or a combined
  `rotate()`+`translate()`/`scale()`/`matrix()` changes the shape's actual rendered
  position, not just its angle — collapsing that to a bare integer without a warning is
  exactly the silent-drop failure mode CLAUDE.md's losslessness section prohibits.
