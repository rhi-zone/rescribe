# 12. `jats-fmt`/`jats` fixtures target the Archiving and Interchange Tag Set; no per-tag-set crates or validation modes

## Status

Accepted (2026-07-28).

## Context

JATS (NISO Z39.96) is published as three sibling "journal article" tag sets plus one
book-oriented extension:

- **Archiving and Interchange ("green")** — loosest constraints, designed so libraries
  and archives can ingest content from any source. ~306 elements
  (jats.nlm.nih.gov/archiving/tag-library/1.3/).
- **Journal Publishing ("blue")** — tighter constraints for publication production
  (publishers, hosting platforms, portals). ~298 elements
  (jats.nlm.nih.gov/publishing/tag-library/1.3/). JATS's own documentation describes
  Publishing as having "fewer elements and tagging choices than the JATS Archiving Tag
  Set" — i.e. Archiving is the superset.
- **Article Authoring ("orange")** — most restrictive, for authors submitting new
  content; a further-constrained subset again.
- **BITS (Book Interchange Tag Suite)** — a *separate* extension of JATS for books,
  reusing JATS modules where content overlaps and adding book-specific modules
  (`book-part`, `book-meta`, glossary, index, etc.) that have no JATS journal-article
  equivalent. Unlike Archiving/Publishing/Authoring, BITS is not merely a tighter or
  looser constraint profile of the same vocabulary — it adds real new element names for
  book structure.

Every existing reference in this codebase (`jats-fmt` docs, `rescribe-read-jats`,
`fixtures/jats/COVERAGE.md`) cites the Archiving tag library URL, but no prior commit,
ADR, or TODO entry ever chose Archiving deliberately — it was an inherited default from
whichever page was open when the crate was first written. An audit was requested to
either justify that default or replace it, using `ooxml`'s crate structure
(`ooxml-opc`/`ooxml-xml`/`ooxml-dml`/`ooxml-omml` shared, `ooxml-wml`/`ooxml-sml`/
`ooxml-pml` schema-specific) as prior art for how rescribe handles a format family with
multiple related schemas.

### Does the ooxml precedent apply?

No — the two situations are structurally different, and forcing the ooxml pattern onto
JATS would be wrong:

- WordprocessingML, SpreadsheetML, and PresentationML are **genuinely different
  vocabularies** for **different document types** (word processing, spreadsheets,
  presentations). A `<w:p>` paragraph and a spreadsheet `<row>` share almost no element
  names; each schema has its own root, its own content model, its own semantics. What
  they *do* share — OPC packaging (`ooxml-opc`), DrawingML shapes/charts (`ooxml-dml`),
  OMML math (`ooxml-omml`) — is factored into separate crates precisely because it's
  used unchanged by multiple genuinely-distinct schemas.
- Archiving, Publishing, and Authoring are **the same vocabulary for the same document
  type** (a journal article) with **tightening content-model constraints**: Publishing's
  ~298 elements are (per JATS's own documentation) a subset of Archiving's ~306 with
  stricter rules about what's required/optional/repeatable where; Authoring tightens
  further. There is no analogue of "SpreadsheetML has cells, WordprocessingML doesn't" —
  the element names largely coincide across all three.

So the ooxml split exists because wml/sml/pml are different vocabularies sharing common
infrastructure. Archiving/Publishing/Authoring are not different vocabularies at all —
they're validity *profiles* of one vocabulary. The ooxml precedent's actual lesson
("split when the vocabularies genuinely diverge, share a base when they don't") argues
*against* splitting JATS tag sets, not for it. BITS is the one JATS relative that would
actually fit the ooxml pattern (real additive vocabulary for a different document type)
if book support is ever undertaken — see Consequences.

### Is `jats-fmt` even tag-set-aware?

No. `crates/formats/jats-fmt/src/parse.rs` parses **any well-formed XML** into a
generic `Node::Element`/`Text`/`Cdata`/`Comment`/`ProcessingInstruction`/`Doctype`/
`EntityRef` tree with no schema validation, no DTD/schema fetch, and no tag-set-specific
logic anywhere in the crate (confirmed by reading the full parser and AST). The doc
comment on `JatsDoc` states this explicitly: "JATS-specific meaning... lives in the
rescribe adapter layer, not here." `rescribe-read-jats`/`rescribe-write-jats` map
element names to IR node kinds by string match; any unrecognized element already
raw-preserves as a `jats:tag`-tagged div/span rather than erroring or dropping (per
CLAUDE.md's losslessness rule, and the 2026-07-27 "jats bug fix" TODO entry). A document
using Archiving-only elements, Publishing-only elements, Authoring elements, or entirely
non-JATS XML all parse identically today, and all round-trip through the adapter's
raw-preservation path if the element isn't in the mapping table.

**"Which tag set" is therefore purely a question of which elements the adapter's mapping
table and the fixture suite cover — not a parser-architecture question.** There is
nothing to validate against, and nothing to fork.

## Decision

1. Keep one `jats-fmt` crate and one `rescribe-read-jats`/`rescribe-write-jats` adapter
   pair, permissive as today (no DTD/schema validation of any kind).
2. `fixtures/jats/COVERAGE.md` explicitly targets the **Archiving and Interchange Tag
   Set** as its element inventory, because it's the superset — a fixture suite and
   adapter mapping table built against Archiving already covers every Publishing and
   Authoring element (they're subsets), so no coverage is lost by picking the widest
   tag set as the reference list. This line already existed in COVERAGE.md; it's now
   backed by this ADR instead of being an unexamined inherited default.
3. No validation modes, no per-tag-set feature flags, no separate crates. Adding a
   "reject documents using elements outside tag set X" validation mode is a real,
   separable feature (schema-conformance checking) that nothing today asks for — it is
   not required to make the current permissive-parse-plus-raw-preservation design
   correct or honest about scope, and is left as a possible future addition (see
   Consequences) rather than implemented speculatively.

## Consequences

- No code changes: the crate's existing permissive design was already the right shape
  for this problem, and the existing COVERAGE.md reference URL was already the right
  choice (Archiving as the superset) — this ADR documents and ratifies that inherited
  default rather than changing it, closing the "audit found an undocumented decision"
  gap.
- If BITS support is ever wanted, it is **not** a variant of this decision — BITS adds a
  genuinely separate element vocabulary (book structure) the way DrawingML is genuinely
  separate from WordprocessingML text flow. That would warrant evaluating a structure
  closer to the ooxml one (e.g. a `bits-fmt`-equivalent, or extending `jats-fmt`'s
  mapping table with BITS-only elements raw-preserved/modeled the same permissive way
  already used for unrecognized elements) rather than reopening this ADR. Not undertaken
  here — no current requirement for book-format JATS support.
- If a future caller genuinely needs tag-set conformance checking (e.g. validating that
  a document intended for Authoring submission doesn't use Archiving-only elements),
  that is new scope: a validation pass consuming `jats-fmt`'s `JatsDoc` against a
  per-tag-set element/content-model table, independent of parsing. Not needed by
  rescribe's conversion use case today and not implemented speculatively per CLAUDE.md's
  no-guessing rule; tracked as a possible future TODO item only if requested.
