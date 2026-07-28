# BibTeX Fixture Coverage

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

Entries parse into `bibliography`/`bibliography_entry`/`bibliography_field` IR nodes (see
ADR 0005 and `rescribe_std::node`), not the legacy flat `definition_list` shape. Each field
node carries both `field:role` (the semantic vocabulary) and `bibtex:field` (the exact
BibTeX field name, for round-trip fidelity when a field has no `field:role` of its own, e.g.
`school`/`institution`/`note`/`chapter` all become `misc` fields distinguished by
`bibtex:field`).

## Entry types

BibTeX defines 14 standard entry types. Non-standard types are common in practice.

### Standard entry types
- [x] article — `article`
- [x] book — `book`
- [ ] booklet — (missing)
- [x] inbook — `inbook`
- [x] incollection — `incollection`
- [x] inproceedings — `inproceedings`
- [ ] conference — (missing; alias for inproceedings)
- [ ] manual — (missing)
- [x] mastersthesis — `mastersthesis`
- [ ] misc — `misc`
- [x] phdthesis — `phdthesis`
- [ ] proceedings — (missing)
- [x] techreport — `techreport`
- [ ] unpublished — (missing)

### Non-standard but common
- [ ] online / electronic / www — (missing)
- [ ] software — (missing)
- [ ] dataset — (missing)

## Fields

### Required / commonly required fields
- [x] author (single) — `article`
- [x] author (multiple, `and`-separated) — `two-authors`
- [x] title — `article`
- [x] year — `article`
- [x] journal — `article`
- [x] publisher — `book`

### Author/editor fields
- [x] author — `article`
- [x] editor — `incollection`
- [ ] organization — (missing)

### Title and container fields
- [x] title — `article`
- [x] booktitle — `inbook`, `incollection`, `inproceedings`
- [ ] series — (missing)
- [ ] chapter — `inbook`

### Date / identifier fields
- [x] year — `article`
- [ ] month — (missing)
- [x] doi — `rare-with-doi`
- [ ] url — (missing)
- [ ] isbn — (missing)
- [ ] issn — (missing)

### Publication fields
- [x] publisher — `book`
- [x] institution — `mastersthesis`, `phdthesis`, `techreport`
- [ ] school — (missing; BibTeX alias for institution in thesis types)
- [ ] address — (missing)
- [ ] edition — (missing)
- [x] volume — `article`
- [ ] number — (missing)
- [x] pages — `article`, `inbook`
- [ ] type — `techreport`
- [ ] howpublished — (missing)

### Annotation / note fields
- [ ] abstract — (missing)
- [ ] annote — (missing)
- [ ] note — `misc`
- [ ] key — (missing)

### Cross-reference fields
- [ ] crossref — (missing)

## Special syntax

- [ ] @string definitions — (missing)
- [ ] @preamble — (missing)
- [ ] @comment — (missing)
- [ ] string concatenation with `#` — (missing)
- [ ] `{...}` literal protection — (missing)
- [ ] `"..."` field delimiters — (missing)
- [ ] `{...}` field delimiters — `article` (uses braces)
- [ ] multiple entries in one file — (missing)
- [ ] name list with `and others` — (missing)

## Composition (integration)

- [ ] article with all standard fields — (missing)
- [ ] book with editor field — (missing)
- [ ] multiple entries in one file — (missing)
- [ ] @string used in field value — (missing)

## Adversarial

- [x] empty file — `adv-empty`
- [ ] missing required field — (missing)
- [ ] malformed entry (unclosed brace) — (missing)
- [ ] unknown entry type — (missing)
- [ ] unknown field name — (missing)
- [ ] entry with no key — (missing)
- [ ] duplicate entry keys — (missing)
- [ ] field value with unmatched braces — (missing)

## Pathological

- [ ] file with 1000 entries — (missing)
- [ ] entry with very long field value — (missing)
- [ ] deeply nested braces in field value — (missing)
- [ ] all fields present on a single entry — (missing)
