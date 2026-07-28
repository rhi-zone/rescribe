# 13. Per-format construct registries: a spec-derived, machine-readable denominator

## Status

Accepted. Piloted end-to-end on JATS (`crates/formats/jats-fmt`). Rollout to DocBook, TEI, and
migrating `ooxml-*` onto this design is planned but not done — see `TODO.md`'s registry
rollout entries.

## Context

Every format vertical's completion claim rests on a ratio: "101/117 constructs covered." The
numerator is real work. The denominator was, until this design, a hand-written list in
`fixtures/{format}/COVERAGE.md` — and a hand-written list cannot supply a trustworthy
denominator, because it only ever grows when somebody happens to notice something missing.

The 2026-07-28 COVERAGE.md audit (see `TODO.md`) measured exactly how badly this fails.
DocBook's denominator had drifted 94 → 105 → 117 and JATS's 106 → 109 → 133 across a single
session, each bump prompted by an incidental discovery during unrelated bug-fixing. Diffing
both checklists against the formats' authoritative element indexes found 265 DocBook element
names and 216 JATS element names enumerated nowhere at all. So "101/105 covered" was reporting
96% of a list someone wrote, not 96% of DocBook.

`docs/adr/0004-xml-classifier-schema-verification-methodology.md` names the same failure in
its own domain: a presence-checking pass over an incomplete list can find "have but shouldn't"
but cannot structurally find "should have but don't," no matter how carefully each listed entry
is re-verified. This ADR is the mechanization of that fix: rather than each verification pass
re-fetching and re-deriving the index by hand — a fallible manual step, performed differently
each time — the index becomes a committed, machine-readable artifact with recorded provenance.

Existing prior art in this repo is `spec/ooxml-features.yaml` (3849 lines) plus
`spec/ooxml-names.yaml` and `spec/ooxml-events-*.yaml`, consumed by
`crates/formats/ooxml-*/build.rs` through `crates/tools/ooxml-codegen`. It establishes several
patterns worth keeping (a committed derived artifact; an escape hatch for "everything under
here"; regeneration gated behind an env var so contributors need not hold the spec) and several
worth not repeating: a tag vocabulary that is a header comment rather than data and has already
drifted from the data it describes (the comment documents `revisions`, 0 uses; the data uses
`track-changes`, 70 uses), an advisory-only completeness lint (`analysis.rs`, gated behind
`OOXML_ANALYZE`, whose `has_unmapped()` is dead code), and zero provenance metadata of any kind
— no spec edition, no checksum, no derivation date. `spec/ooxml-features.yaml`'s tags are also
this project's own hand-chosen functional groupings (`core`, `styling`, `structure`, `charts`,
etc., documented only in that header comment) — they are not derived from, and do not
correspond to, ECMA-376's own modularization (21 namespace schemas, ~59 part entry-points).
That distinction — a format's own published modularization vs. a partition this project
invents for its own purposes — is load-bearing for this design (see "Two kinds of slice"
below). Get this from `spec/ooxml-features.yaml` itself, not from a description of it.

## Decision

Each `-fmt` crate carries a **construct registry**: a committed, machine-readable catalog of
every construct its format defines, derived from the format's own published schema (or, where
none exists, from a reproducible extraction — see "Source kinds" below).

### 1. Per-crate, not centralized

The registry lives in the `-fmt` crate (`crates/formats/jats-fmt/registry/`), not in
workspace-level `spec/`. The `-fmt` crates are first-class ecosystem libraries; rescribe is one
consumer among many. Someone depending on `jats-fmt` alone — for a search indexer, a linter, a
validator — has the same need to ask "what does JATS define" as rescribe's coverage tooling
does, and a catalog in rescribe's workspace root is invisible to them. This also avoids
ooxml's central-YAML wart: a closed set of module names mirrored by hard-coded `match module {
"sml" => …, "wml" => … }` dispatch in `ooxml-codegen`, whose fallback arm silently routes
unknown modules to `sml`. Per-crate files make each registry its own document, with no module
dimension to keep in sync.

### 2. Opt-in Cargo feature

`registry` is off by default. A consumer that only wants to parse XML should not compile, or
pay for, the catalog. The derivation tool sits behind a further `registry-derive` feature,
since it additionally needs the source schema present.

### 3. Two kinds of slice: normative and pragmatic

Every construct may be annotated with slices from two independent, separately-populated
fields:

- **`normative_slices`** — a partition the format *itself* publishes: JATS's 29 DTD Suite
  modules, TEI's 22 `<moduleSpec>` modules, OOXML's 21 namespace schemas / ~59 part
  entry-points (if a future derivation targets that decomposition). Each entry is a `Slice`
  with the module's own declared name, its source file, and a resolvable URL. **May
  legitimately be empty** for a format whose normative schema publishes no modularization —
  DocBook 5.2 is exactly this case (its normative RELAX NG is a flattened monolith, 414
  elements in 420 anonymous `div {}` blocks with no module identity). When empty, the registry
  records why, as a short reason string, rather than leaving an unexplained gap.
- **`pragmatic_slices`** — a partition curated by whoever maintains the registry, for whatever
  purpose is useful (feature gating, a reading order, a "start here" grouping). Always
  permitted, unconditionally — it never needs the format to publish a modularization and never
  needs a licensing question resolved, because it makes no claim to reflect the format's own
  structure. It must be explicitly marked non-normative wherever it's surfaced. ooxml's
  existing ~20 functional tags (`core`/`styling`/`charts`/etc.) are exactly this kind of
  slice, once `ooxml-*` migrates onto this registry design (not done yet — see `TODO.md`); they
  were never the format's own modularization and should stop being described as such.

A construct may appear in either, both, or neither list. `primary_normative_slice()` and
`primary_pragmatic_slice()` are each `Option<&str>`, since either list may be empty for a given
construct or for a whole format.

### 4. Runtime-queryable, and zero-cost at runtime

The registry is a typed Rust API (`jats_fmt::registry`). `registry()` returns a `&'static
Registry` compiled directly into the binary — no parsing, no `OnceLock`, no allocation at call
time (`crates/formats/jats-fmt/src/registry_generated.rs`, committed, matching `ooxml-wml`'s
committed `generated.rs` precedent). `registry().elements()`, `.in_slice(id)`,
`.contains_element(name)`, `.not_handled(kind, handled)` are ordinary calls over static data.
`Construct`/`Registry`/`ContentModel` implement `Serialize`, so the whole document composes
with the `rescribe query`/jaq pattern without the registry crate depending on the query crate.

### 5. Support status is not in the registry

The registry is spec-pure: it records what the format defines, never what any crate supports.
Mixing the two would make it churn on every implementation commit and recreate a
hand-maintained claim in a new file. "Do we support X" is a **join**, performed by the
consumer: `Registry::not_handled(kind, what_i_handle)`.

### 6. Citations are external references, never file+line into a vendored schema

`spec/*` is gitignored and absent from most checkouts and all of CI, so a citation must resolve
without it: a spec identifier (`ANSI/NISO Z39.96-2021`), a canonical base URL per source file,
and a per-construct URL template (`…/tag-library/1.3/element/{name}.html`).

### 7. Content models: flattened, deduplicated

Each element construct carries an optional `ContentModel`: `children: [{name, repeatable}]`
(every element name permitted as a direct child, tagged repeatable/not), `attributes:
[{name, required}]`, and `mixed: bool` (character data permitted alongside children). This
deliberately drops relative order, choice exclusivity, and group/interleave co-occurrence — the
full pattern structure RELAX NG can express — because "can `sec` contain `fig`" is a
set-membership question this flattened form answers directly, while "is *this* `<sec>` valid"
is a validation question a RELAX NG validator already answers correctly; recording the full
grammar would substantially complicate the derivation tool to answer a question this registry
isn't for. Distinct content-model shapes are deduplicated and emitted once each as a named
static (`CM_0`, `CM_1`, …), referenced by pointer from every construct that shares the shape,
pinned by a pointer-identity test (`crate::registry::tests::content_models_are_deduplicated`).
Measured on JATS: 486 elements carry a content model; only 270 distinct shapes exist (44.4%
would otherwise be duplicate data).

Whether a second, full-grammar representation is worth adding alongside the flattened one is
open — no consumer has asked for validation-grade output yet, so it isn't built speculatively.

### 8. Source kinds: schema forms, or a reproducible extraction

`SourceKind` names the form the construct list was derived from: `Relaxng`, `Rnc`, `Dtd`,
`Xsd`, `Odd` for the schema forms this repo has encountered, or `ScriptedExtraction` for a
format with no machine-readable schema at all. The property that matters is not "came from a
schema," it's "reproducibly derived": a format with no schema is pointed at its own published
prose (an HTML element index, a printed reference table) and required to extract that list
*mechanically* — a script, committed alongside the registry, that fetches the published
artifact and parses it, the same way `derive-registry`'s RELAX NG walk does for JATS. A
scripted extraction is re-runnable, diffable against a fresh fetch, and auditable by reading the
script; a hand-typed list is none of those. No format encountered so far (JATS, DocBook, TEI,
OOXML, ODF) lacks some published, script-extractable artifact of its own element vocabulary,
so no hand-curated, unreproducible source kind is offered as an escape hatch.

### 9. The registry is a committed derived artifact; provenance is load-bearing

Neither CI nor any downstream consumer will have the schema, so the registry cannot be
build-time codegen from a local input — it's generated once, committed, and read as data.
Provenance is the only way a reader can judge staleness, so it's recorded in full: spec
identifier, schema form, driver file, canonical base URL, source license and whether it permits
redistribution, whether this repo vendors it, derivation date, deriving tool and version, and a
SHA-256 plus byte count for every source file consumed.

### 10. Verification: two independent drift checks

A developer who fetches the schema can re-derive and diff against it:
`cargo run -p jats-fmt --features registry-derive --bin derive-registry -- --schema-dir
spec/jats-1.3-archiving-rng --check`. This compares parsed documents, not bytes, so
reformatting doesn't read as spec drift.

A second, independent check needs no schema at all:
`registry_derive::drift_tests::generated_rust_matches_committed_source` regenerates
`registry_generated.rs` from the committed JSON source and diffs it against the committed file
— this runs as an ordinary `cargo test`, in CI, with nothing fetched. A third test,
`committed_source_round_trips`, confirms the JSON model survives a serialize/deserialize round
trip.

### 11. Human-readable committed source: JSON, not YAML

The registry's runtime representation is committed generated Rust (decision 4); the
human-readable committed source that `derive-registry` reads and writes is JSON
(`registry/jats-1.3-archiving.json`), parsed only by the offline `registry-derive` tool via
`serde_json` — never at runtime, and never by a normal `registry`-feature build. `serde_yaml`
has been removed from `jats-fmt` entirely, in every feature; no YAML parser of any kind remains
in its dependency graph (verified via `cargo tree -p jats-fmt --features registry-derive -e
normal`).

An earlier draft of this design chose YAML, copied from `spec/ooxml-features.yaml`'s own format
without checking what role that file actually plays: it's ooxml's hand-curated *input*, read by
a build-time codegen step that still needs a human-editable format. This registry's committed
Rust statics are generated *output*, analogous to `ooxml-wml`'s committed `generated.rs`, not to
ooxml's YAML — the human-editable layer here is the offline derivation tool's source file,
which nobody hand-edits directly (it's produced by `derive-registry` from the schema and
regenerated, not typed). Once the runtime path parses nothing, the choice of source-format
serialization is unconstrained by runtime concerns, and JSON was picked over YAML because it
parses more uniformly across language ecosystems for what's meant to be a language-agnostic
artifact (the same reasoning `fixtures/` follows), and because it removes any reason to keep a
`serde_yaml` dependency around for a file nothing runtime-critical reads. `serde_json` was
already a workspace dependency (used by `rescribe query`).

### 12. Cargo-feature collapse for multi-slice constructs: OR-of-all

Where a construct's `pragmatic_slices` membership must collapse into a single `#[cfg]`
predicate for a Cargo feature gate (as `ooxml-codegen` does today, ahead of ooxml's own
migration onto this registry design), the rule is **OR-of-all**: a construct tagged with
several slices — e.g. `Worksheet.drawingHF: [drawings, layout]` — compiles in when *any one* of
its tagged features is enabled (`#[cfg(any(feature = "sml-drawings", feature =
"sml-layout"))]`), via `FeatureMappings::feature_gates` (returns every tag, not just the first)
and a shared `cfg_predicate` helper used identically by `codegen.rs`, `parser_gen.rs`, and
`serializer_gen.rs` so struct fields and the parser/serializer code that reads them can never
disagree. This was chosen over intersection (silently excludes a construct unless *every* tagged
feature is enabled — more surprising than the bug it replaces) and over requiring an explicit
single gate per multi-tagged construct (more principled in the abstract, but demands ~76
individual human judgment calls with no request driving that curation). The rejected prior
behavior — keeping only a construct's first tag, with the rest silently inert and no
diagnostic anywhere — is exactly the failure mode this rule exists to close; regression tests
in `ooxml-codegen` (`codegen.rs`'s unit tests plus the integration test
`ooxml-codegen/tests/multi_tag_feature_gates.rs`) pin the real `Worksheet.drawingHF` construct's
generated predicate so a future edit reverting to `tags.first()` fails rather than silently
ships.

## Consequences

- **The pilot found mechanically what the audit found by hand.** `jats-fmt`'s registry derives
  305 JATS-native elements — plus 181 embedded MathML elements (486 total) — against the ~306
  the Tag Library's own alphabetical element index lists for JATS-native vocabulary alone,
  i.e. the derivation is essentially exact. Of the 305 native elements, 176 are never mentioned
  anywhere in `rescribe-read-jats`'s source; every element the 2026-07-28 hand audit called out
  (`hr`, `sub-article`, `response`, the `ruby`/`rb`/`rt`/`rp` family, `chem-struct`, `array`,
  `index-term`, `media`, `alt-text`) appears in that gap list without anyone having to notice
  it, pinned by a regression test.
- **A derivation boundary was found and fixed rather than accepted.** The first derivation
  produced no `<table>`/`<tr>`/`<td>` at all, because JATS embeds XHTML tables and MathML by
  reference through an include chain the derivation tool's first pass didn't walk
  transitively. Both the derivation tool and `scripts/jats/download-spec.sh` now resolve the
  include graph transitively. A hand-written checklist has no equivalent of this failure being
  detectable.
- **Redistributability is not uniform, and the design must not assume it is.** Verified per
  format:

  | Format | Schema | License | Vendorable? | Normative modularization |
  |---|---|---|---|---|
  | JATS 1.3 Archiving | DTD (canonical) + RNG + XSD | Public domain, per module headers | Yes, verbatim | **Yes** — 29 DTD-suite modules |
  | TEI P5 | RELAX NG + DTD + XSD + ODD | CC BY 3.0 or BSD-2-Clause | Yes | **Yes** — 22 `<moduleSpec>` |
  | DocBook 5.2 | RELAX NG (RNC + XML) | Permissive perpetual grant in schema header | Yes | **No** — normative artifact is a flattened monolith |
  | OOXML (ECMA-376) | RNC + XSD, Strict + Transitional | Unresolved — no copyright/license statement found in schemas or Parts 1/2 | Assume not | Yes — 21 namespace schemas, ~59 part entry-points (not yet derived) |

  DocBook's registry, when rolled out, ships `normative_slices: []` with the recorded reason
  above, and may separately carry a `pragmatic_slices` grouping (invented or borrowed as an
  idea, not a redistribution, from the non-normative Codeberg TC source's ~35-module shape —
  its license remains unverified and unresolved, but doesn't block a pragmatic slice, since
  nothing would be redistributed verbatim).
- **Why JATS was piloted rather than DocBook.** DocBook has the larger known-failure dataset,
  but it has no normative modularization to source `normative_slices` from, while JATS has one,
  a public-domain license, a stable per-element citation URL, and an RNG (XML, parsed by
  `jats_fmt`'s own parser with no dependency on `ooxml-codegen`'s RNC subset) — it exercises
  every part of the design at once.
- **Measured runtime cost.** rlib size with the `registry` feature: 2,275,366 bytes (down from
  4,244,748 bytes under an earlier YAML/`serde_yaml`-parsed design, ~52% smaller); without the
  feature, unchanged at 440,032 bytes. The remaining growth over baseline is the construct data
  itself (734 constructs, ~12,900 permitted-child/attribute entries even after content-model
  dedup) compiled into rodata, not parser machinery.
- **MathML sharing across formats was assessed and deliberately deferred, not built.** JATS,
  DocBook, TEI, and BITS all embed the same MathML vocabulary (181 of JATS's 486 registry
  elements are MathML). Building a shared crate now would be speculative — no second format has
  a registry yet to validate the sharing shape against. If picked up later, the shape is a
  small crate exporting the same `&'static`-statics shape, consumed as an ordinary dependency by
  each format's own generated registry. The cost of deferring is committed-file duplication
  only (each format would embed its own copy of the MathML block); zero runtime cost either
  way, since both shapes are statics.
- **Costs.** Two artifacts must be kept honest: the committed JSON and the schema it came from
  — `--check` closes that loop, but only for a developer who has fetched the schema, so
  drift is detectable on demand, not prevented at commit time for that half. The
  source→generated half (JSON vs. `registry_generated.rs`) *is* prevented at commit time,
  since it runs as an ordinary test with no schema needed.

## Alternatives considered

- **Keep hand-written COVERAGE.md, apply ADR 0004's absence-check methodology more
  diligently.** Rejected: the methodology is right, but performing it by hand is a fallible
  manual step repeated per verification pass, with the fetched index discarded each time —
  precisely how the denominator drifted while every individual pass was conscientious.
- **One central registry for all formats,** extending `spec/ooxml-*.yaml`. Rejected: it serves
  rescribe and nobody else, and the ecosystem consumers of the `-fmt` crates are the point
  (CLAUDE.md's priority hierarchy puts them above rescribe's own needs).
- **Derive at build time from the schema.** Rejected: neither CI nor downstream consumers have
  the schema, and for OOXML they may not legally be given it.
- **Put support status in the registry** (a `supported: bool` per construct). Rejected: it
  would churn the registry on every implementation commit and reintroduce a hand-maintained
  claim; the join is cheap and cannot go stale.
- **Ship the committed artifact as YAML instead of JSON, or as parsed-at-runtime data instead
  of generated Rust statics.** Rejected on both axes: runtime YAML parsing was measured to cost
  ~52% more compiled size than committed Rust statics for no behavioral benefit, since registry
  queries are audit tooling, never a hot path where avoiding codegen would matter; and once the
  runtime path is generated Rust rather than parsed data, the committed human-readable source
  format is free to be whatever parses most uniformly across language ecosystems, which JSON
  does at least as well as YAML.
- **Reuse `ooxml-codegen`'s `parse_rnc` for the pilot.** Not applicable to JATS (its schema is
  RNG, i.e. XML) and not currently viable in general: that parser handles no `grammar`,
  `include`, or `|=`/`&=` combine, and discards RNC annotations before lexing. It's the right
  foundation for a DocBook registry, but needs that work first.

## Open questions

1. **DocBook's `pragmatic_slices`.** Should DocBook's rollout populate `pragmatic_slices` at
   all, and if so, invent a fresh grouping or borrow the shape of the non-normative Codeberg TC
   source's ~35 modules (as an idea, not a redistribution)? Separately: should the Codeberg
   source ever be treated as authoritative enough to populate `normative_slices` instead of
   staying empty — a citation claim that would need its license resolved first, unlike a
   pragmatic slice?
2. **OOXML's license.** No copyright or license statement was found in the ECMA-376 schema
   files or in Parts 1 and 2. The conservative reading — treat as non-redistributable — is what
   the design assumes; confirming it needs someone who can ask Ecma. This only gates a future
   `normative_slices` derivation from the namespace/part schemas, not `pragmatic_slices`.
3. **A full, validation-capable content-model representation**, alongside the flattened one, if
   a consumer ever asks for validation-grade output rather than set-membership queries. Not
   built speculatively.
4. **MathML sharing across JATS/DocBook/TEI/BITS registries**, once a second format has a
   registry to validate the sharing shape against.
