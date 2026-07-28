# 7. DTD-aware entity resolution: a standalone crate, layered over a bought standard table

## Status

Accepted; implemented. `xml-entities` (`crates/formats/xml-entities`) is a standalone crate
(commit `37253485d0`), depended on by `docbook-fmt`, `jats-fmt`, and `tei-fmt` (each crate's
`Cargo.toml`, `parse.rs`/`events.rs`/`batch.rs`).

## Context

`docbook-fmt`, `jats-fmt`, and `tei-fmt` raw-preserve any named XML entity beyond the 5
XML-predefined ones (`lt`/`gt`/`amp`/`apos`/`quot`) and numeric character references losslessly
as `Node::EntityRef` / IR `raw_inline` with a format-namespaced property
(`docbook:entity`/`jats:entity`/`tei:entity`) — never silently dropped, per CLAUDE.md's
raw-preservation rule. But without resolution, that's all a consumer gets: DocBook/JATS/TEI
documents can declare custom entities in a DTD internal subset (`<!ENTITY name
"replacement">`) and/or reference standard ISO 8879 / ISO/IEC 9573 entity sets (ISOlat1,
ISOnum, ISOpub, ISOtech, ISOgrk, mmlalias, etc.), and a consumer that wants the actual
replacement text/character has no path to it from raw preservation alone.

A research pass checked crates.io, docs.rs, and GitHub issue trackers for an existing
production-grade crate covering this before building one:

- **quick-xml**, the parser these three format crates already use for the rest of their XML
  handling, does not parse `<!ENTITY>` declarations in the internal/external subset at all.
  GitHub issue #258 ("Declared entity is not recognized") has been open since Feb 2021, tagged
  `enhancement`/`help wanted`, with no maintainer commitment.
- **`dtd`** (0.1.0, last published 2019) and **`dtd-parser`** (0.1.0-alpha3, last published
  2021) both exist but are early-stage/unmaintained, parse DTD grammar into a data structure at
  best, and provide no entity-resolution layer, no bundled ISO/HTML5 tables, and no adoption
  evidence.
- **`entities`** (~6.9M downloads, maintained) bundles the WHATWG HTML5 named-character-
  reference table, a close superset of the ISO 8879/9573 sets DocBook/JATS/TEI lean on — but it
  has no notion of a document's own DTD-declared entities, which the ISO/HTML5 table cannot
  cover by definition (they're format- and document-specific, not standardized).
- No crate packages the ISO 8879/9573 `.ent` files directly as a Rust table (`entity_table` on
  crates.io is an unrelated ECS crate, a false-positive name collision).

**Note on `xml-rs` and `roxmltree`, checked again while writing this revision of the ADR**:
both actually parse internal-subset `<!ENTITY>` declarations, contrary to what an earlier
research pass concluded. `xml-rs`'s own README states "DTD validation is not supported (but
entities defined in the internal subset are supported)," and its parser
(`src/reader/parser/inside_doctype.rs`, `inside_reference.rs`) genuinely discovers and
substitutes internal-subset entity declarations. `roxmltree`'s tokenizer
(`src/tokenizer.rs:406-511`) natively emits a `Token::EntityDeclaration` for internal
declarations; its `EntityResolver` callback is invoked only to fetch *external*
(`SYSTEM`/`PUBLIC`) entity bodies, not to discover internal ones. So switching the underlying
XML parser (away from quick-xml, to `xml-rs` or `roxmltree`) was a real alternative to writing
a standalone entity crate, and it was never actually weighed — the research that produced the
original build decision incorrectly ruled it out. That alternative is not adjudicated here: by
the time this was caught, `xml-entities` already existed and was already wired into all three
readers, and replacing quick-xml as the underlying parser for three format crates is a much
larger, separate architectural decision than entity resolution alone. It is recorded as an
open question below rather than decided unilaterally.

## Decision

**Build** a narrow, standalone `xml-entities` crate that discovers `<!ENTITY name
"replacement">` declarations in a DTD internal subset (general and parameter entities,
`SYSTEM`/`PUBLIC` external entities recorded but never fetched, numeric character references
expanded at declaration time) and **layers them over a bought standard table** — the
`entities` crate's WHATWG HTML5 named-character-reference table — as the fallback/default
entity set. Document-declared entities take precedence over the standard table; resolution is
recursive with cycle/depth guards; unknown or external names resolve to an explicit
non-error variant so callers can raw-preserve rather than drop them.

`xml-entities` has no rescribe dependency: it's a first-class standalone library per
CLAUDE.md's `-fmt`-crate philosophy, usable by any Rust project doing XML/SGML-ish entity
work, not just rescribe's three XML verticals.

## Consequences

- Closes the ceiling flagged across all three XML verticals: named entities now resolve to
  real text/characters, not just raw-preserve.
- Added one new small crate to the workspace (`xml-entities`, dependency: `entities` only),
  wired into `docbook-fmt`, `jats-fmt`, `tei-fmt`.
- The underlying XML parser for these three crates remains quick-xml. Switching to `xml-rs` or
  `roxmltree` (both of which handle internal-subset entity declarations natively, which would
  have removed the need for a standalone crate for that half of the problem) was never
  evaluated on its own merits, because the research that motivated building `xml-entities`
  incorrectly believed no existing parser handled internal entities at all. Revisiting that
  comparison — `xml-entities` as a thin, parser-agnostic add-on vs. a parser swap that folds
  entity discovery into the base XML parsing step — is open work, not resolved by this ADR.

## Alternatives considered

- **Depend on one of the unmaintained DTD crates (`dtd`/`dtd-parser`) instead of building**:
  rejected — both are alpha-stage, last published years ago, with no adoption evidence; using
  them would trade a small build effort for an unmaintained dependency doing the same narrow
  job worse.
- **Skip DTD-declared custom entities, resolve only the standard ISO/HTML5 table**: rejected —
  would leave format-specific custom entity declarations (which DocBook/TEI/JATS documents
  routinely declare) unresolved; only raw-preservation (the pre-existing behavior) would remain
  for those, not actual resolution.
- **Wait for quick-xml issue #258 to be implemented upstream**: rejected as the primary path —
  open since Feb 2021 with no maintainer commitment; nothing prevents contributing upstream
  later, but it isn't a plan to depend on now.
- **Switch the underlying XML parser to `xml-rs` or `roxmltree`** to get internal-entity
  discovery for free: not properly considered at decision time (see Context) and not decided
  here — recorded as open work rather than adjudicated in this revision.
