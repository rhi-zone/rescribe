# 5. Citation/bibliography IR shape: `bibliography`/`bibliography_entry`/`bibliography_field`

## Status

Accepted (commit `4e15c9966e`, 2026-07-28).

## Context

Before this decision, rescribe had zero citation/bibliography node kinds. All three XML
verticals (DocBook, JATS, TEI) degraded reference-list constructs (`ref-list`/`ref`,
`biblStruct`, `bibliography`/`biblioentry`, etc.) to generic containers (`div`/`generic_span`)
with no field extraction — not a missing-fixture gap but a genuine IR gap, since there was no
node shape to extract fields *into*.

The open design question was the field content model: should a citation field (title, author,
publisher, ...) be a flat `PropValue::String` on the entry (matching how table cell attributes
like `colspan`/`rowspan` are Properties), or a child node whose own children are inline nodes?

An early draft of this proposal argued Properties on the grounds of "existing convention... table
cells, list attributes, etc. all use Properties, not child nodes, for non-prose structured
data." That claim does not hold up against rescribe's own standard node kinds: `table`,
`table_row`, and `table_cell` **are** node kinds — `table_cell` is a child node whose content is
itself child nodes, not a flat property. Only cell *attributes* (`colspan`/`rowspan`) are
Properties; cell *content* is always nodes, precisely because a cell's content is renderable
prose that can carry its own markup. That precedent, once checked, actually argues for
child-node fields, not against them. **This is a corrected misconception, not a stylistic
preference** — the "matches existing convention" justification in the earlier draft was wrong,
and the shape below was chosen on different (and correct) grounds instead: whether the source
format's content model permits nested markup in that field.

To settle it with evidence rather than a guess, each of the 8 semantic citation fields (author,
title, container_title, publisher, publisher_location, edition, volume/issue, page_range,
identifier) was checked against DocBook 5.2, JATS 1.3, TEI P5, and OOXML's `b:` bibliography
namespace, for whether that field's content model permits nested inline markup (emphasis,
abbreviations, editorial tags) in at least one of the four. Result: **7 of 8 fields** do
(title, container_title, publisher, publisher_location, edition, volume/issue, and identifier
via TEI's `idno`'s narrow glyph/recursive model). `page_range` is markup-permitting in
TEI/DocBook but text-only (`#PCDATA`) in JATS's `fpage`/`lpage` specifically — not a universal
exception. OOXML's `b:` schema is the sole outlier, entirely flat/`xsd:string`-typed across
every field, since it's a Word-internal citation-manager format rather than a rich-markup one —
consistent with being the narrowest format in the survey, not evidence against child nodes for
the other three.

`date` is the one field kept as a flat property rather than a child node: date sub-parts
(year/month/day) are atomic, non-markup-bearing data in every schema surveyed — no format lets
you italicize part of a year — and a structured `PropValue::Map` lets writers reformat per
regional convention without re-parsing an ambiguous flat string, something a child-node or flat
string representation couldn't support unambiguously.

## Decision

Three new node kinds in `rescribe-std`:

- `bibliography` — container; children are `bibliography_entry` nodes.
- `bibliography_entry` — one citation/reference; children are `bibliography_field` nodes, and
  (for structural nesting cases like DocBook's `biblioset` or TEI's `analytic`/`monogr`/`series`
  levels) nested `bibliography_entry` nodes.
- `bibliography_field` — a single tagged field (role given by `field:role` property: `author`,
  `editor`, `title`, `container_title`, `publisher`, `publisher_location`, `edition`, `volume`,
  `issue`, `page_first`, `page_last`, `identifier`, `misc`); children are ordinary inline nodes,
  so markup nested inside a field (an italicized journal title, an abbreviation in a publisher
  name) is preserved rather than flattened to a string. Repeated fields (multiple authors) are
  multiple sibling `bibliography_field` nodes sharing the same `field:role`, in document order.

Plus `field:scheme` (identifier scheme, e.g. `doi`/`isbn`/`issn`/`url`, for fields with
`field:role == "identifier"`) and `date` (structured `PropValue::Map` with `year` and optional
`month`/`day`) as properties on `bibliography_entry`.

## Consequences

- Enables lossless citation round-tripping across DocBook/JATS/TEI once each format's adapter
  is wired to this shape (adapter wiring itself is separate follow-up work — this decision only
  adds the node kinds).
- `date` is the one deliberate asymmetry: every other semantic field is a child node, `date`
  alone is a Property, because it's the only field that is atomic and non-markup-bearing in
  every schema checked. A future format that turns out to allow markup inside a date sub-part
  would be a genuine counter-signal to revisit this, not just an inconvenience to route around.
- Corrects a documented misconception: "Properties matches existing convention" is not a valid
  justification in this codebase — `table_cell` already establishes that renderable/markup-
  bearing content is a child node, not a property, regardless of how "structured" the data
  looks at a glance.

## Alternatives considered

- **Flat `PropValue::String` per field** (the earlier draft's proposal, citing table cells as
  precedent): rejected once the precedent was actually checked — `table_cell` contradicts
  rather than supports it. Would silently drop markup in 7 of 8 fields across at least one of
  the four surveyed formats.
- **One node per entry with all fields as Properties keyed by name** (no `bibliography_field`
  layer): rejected for the same content-model reason — still can't carry nested markup without
  a child-node layer for the markup-permitting fields.
