# 13. Per-format construct registries: a spec-derived, machine-readable denominator

## Status

Accepted (2026-07-28); amended the same day to correct a factual error in decision 3 and
replace the single-slice rule with a two-field model (normative vs. pragmatic slices).
Pilot implemented for JATS 1.3 Archiving in `jats-fmt` (`registry` / `registry-derive`
features); the pilot's schema/types are being updated to the two-field model in the same
session this amendment lands in — check `TODO.md` for whether that update is committed yet.
Rollout to DocBook, TEI, and the OOXML migration is planned but not done — see `TODO.md`.

## Amendment (2026-07-28): decision 3 misstated OOXML's prior art, and the rule it produced was wrong

**The factual error.** Decision 3, as originally written, claimed OOXML's slices come from
"namespace/part schemas." They do not. `spec/ooxml-features.yaml` tags every construct with
one of ~20 hand-chosen functional groupings — `core`, `styling`, `structure`, `formulas`,
`charts`, `layout`, `protection`, `filtering`, `validation`, `comments`, `drawings`,
`hyperlinks`, `metadata`, `i18n`, `pivot`, `tables`, `extensions`, `revisions`, `external`,
and more — documented only in a header comment, and that comment has already drifted from
the data it describes (it documents `revisions`, 0 uses; the data uses `track-changes`, 70
uses; see `TODO.md`'s rollout item 3). These tags were invented by this project. They are
not derived from, and do not correspond to, ECMA-376's own modularization, which is real and
is something else entirely: 21 namespace schemas and ~59 part entry-points, correctly
described in this ADR's own Consequences table (the "OOXML (ECMA-376)" row, "Yes — 21
namespace schemas, ~59 part entry-points"). The ADR's Decision section and its own
Consequences table disagreed with each other about what OOXML's slices are, and Decision 3
was the one that had it wrong. This was confirmed against the live `spec/ooxml-features.yaml`
header and body (verified again while writing this amendment).

**The consequence.** Decision 3 forbade "inventing a partition and presenting it as
authoritative" and named that "the one thing decision 3 forbids." Applied literally, that
rule outlaws the ooxml tags that already ship in this repo — they are exactly the invented
partition the rule forbids — while simultaneously leaving DocBook stuck: DocBook's normative
RELAX NG is a flattened monolith with no module identity (open question 1), so under a
single-slice rule its only moves were "ship empty" or "adopt a partition and hope its
provenance passes muster," and the second of those was blocked on an unresolved license
question for a source (the Codeberg TC repo) nobody had asked to be a prerequisite. TODO.md's
rollout plan recorded this exactly as "DocBook is blocked on an open question, not on
effort" — a direct symptom of the single-slice rule being unable to represent something that
is simultaneously legitimate (a useful, honestly-labeled grouping) and non-authoritative (not
sourced from the format's own publication of its structure).

**Root cause.** The word "slice" was doing two jobs the original decision 3 never separated:

- A **normative/spec-published partition** — JATS's 29 DTD Suite modules, TEI's 22
  `<moduleSpec>` modules, (potentially) OOXML's 21 namespace schemas. Authoritative and
  citable: it tells a downstream implementer how the format itself decomposes. It may
  legitimately not exist, as for DocBook.
- A **pragmatic partition** — ooxml's `core`/`styling`/`charts`. Ours, invented, genuinely
  useful for feature-gating and for telling a consumer what they can skip without reading
  everything. Honest exactly so long as it is labeled as ours rather than presented as the
  format's own structure.

Conflating these into one field forced a choice that need not exist: either the invented
partition gets to claim spec authority it doesn't have, or a legitimately useful invented
partition isn't allowed to exist at all. Both horns are wrong, and the fix is not to pick one
— it is to stop forcing the choice.

**The corrected rule: two independent fields, not one.**

Both `Registry` and `Construct` carry two separate slice collections instead of one:

- `normative_slices` — populated *only* from a partition the format itself publishes (a DTD
  Suite module, a `<moduleSpec>`, a namespace/part schema). Each entry is a `Slice` with the
  module's own declared name, its source file, and a resolvable URL, exactly as decision 3
  originally specified — that part of the design was right, it was just mislabeled as
  OOXML's status quo. **May legitimately be empty** for a format whose normative schema
  publishes no modularization (DocBook today). When empty, the registry must record why (a
  short reason string, not silence) rather than leaving an unexplained gap.
- `pragmatic_slices` — a partition curated by whoever maintains the registry, for whatever
  purpose is useful (feature gating, a reading order, a "start here" grouping). **Always
  permitted, unconditionally** — it never needs the format to publish a modularization, and
  it never needs to wait on a licensing question, because it makes no claim to reflect the
  format's own structure. It must be **explicitly marked non-normative** wherever it is
  surfaced (registry documentation, any generated report, any consumer-facing label) — the
  whole point of splitting the field is that a consumer can always tell which kind of
  partition they are looking at. **May also legitimately be empty** — a format doesn't need
  a pragmatic partition just because the field exists; JATS's pilot populates only
  `normative_slices` and leaves `pragmatic_slices` empty, because inventing a grouping nobody
  asked for would recreate exactly the "invented and presented without qualification" problem
  decision 3 was trying to avoid, just moved to the other field.

A construct may appear in either, both, or (transiently, before triage) neither list.
`primary_slice()` is no longer well-defined as a single method — it is now
`primary_normative_slice()` and `primary_pragmatic_slice()`, each `Option<&str>`, since either
list may be empty for a given construct or for a whole format.

**What this unblocks.**

- **OOXML's existing ~20 tags become legitimate as-is**, once `ooxml-*` migrates onto the
  registry design (not done by this amendment — see `TODO.md` rollout item 3): they populate
  `pragmatic_slices`, explicitly marked as ours, and stop being mischaracterized anywhere as
  "namespace/part schemas." The real namespace/part decomposition (21 namespace schemas, ~59
  parts) remains available as a *future* `normative_slices` source if someone does that
  derivation work; nothing here requires it.
- **DocBook can roll out now**, without resolving the Codeberg source's license first: ship
  with `normative_slices` empty and the reason recorded ("the normative OASIS RNG partitions
  414 elements into 420 anonymous `div {}` blocks with no module identity"), and, if a
  maintainer wants one, an invented `pragmatic_slices` grouping — built the same way ooxml's
  tags were, with no license question at all, because a pragmatic partition is our own
  taxonomy, not a redistribution of someone else's text or structure. The Codeberg TC
  source's ~35 named modules and its unverified license remain exactly what they were: a
  possible *future* way to populate `normative_slices`, if someone wants to argue that
  non-normative build source can stand in for a normative citation and gets the license
  question answered first. That question is not a prerequisite for DocBook's rollout anymore
  — only for that one specific way of eventually filling in the normative field.

**Open question 1, restated under the new model.** The old fork ("ship empty, or adopt the
non-normative Codeberg modules with non-normativity stamped in provenance") no longer has the
same shape, because "adopt the Codeberg modules, marked non-normative" is just populating
`pragmatic_slices` — which needs no decision at all, since pragmatic partitions are
unconditionally permitted. What remains genuinely open is narrower: *should DocBook's
`pragmatic_slices` be populated at all for the initial rollout, and if so, from what* — invent
a fresh grouping, or borrow the Codeberg TC's ~35-module shape as a starting point (still
without claiming it as normative, and still without needing its license resolved, since
nothing would be redistributed verbatim — only the *idea* of a grouping, if even that)? This
is a much smaller question than the original one, and it does not block shipping
`normative_slices: []` with a recorded reason, which can happen immediately.

**Open question 2 (hand-curated registries) is not resolved by this amendment, and the two
are separable.** `SourceKind::HandCurated` is about the *construct list itself* — the
denominator: does the registry's list of "every element/attribute this format defines" come
from a machine-readable schema, or was it typed by a person? That is the exact thing this
whole design exists to make auditable, and a hand-curated denominator is exactly as fallible
as the `COVERAGE.md` checklist it replaces, regardless of how its slices are labeled. A
hand-curated **slice** (a partition *over* an already-trustworthy construct list) is a
different and much lower-stakes claim, and this amendment settles it: explicitly marking a
slice `pragmatic` is sufficient honesty, no further permission needed. Whether a
hand-curated **construct list** should be allowed at all remains exactly as open as it was —
this amendment does not touch it, and no connection between the two should be inferred from
one being resolved.

**The OOXML slice/Cargo-feature conflation flagged by the pilot is only partly resolved.**
The pilot's open item was that ooxml's tags currently serve two jobs at once — a slice
(descriptive grouping) and a Cargo feature gate (`#[cfg(feature = "...")]` selection) — and
that `primary_feature` silently keeps only the first of a construct's tags, so
`drawingHF: [drawings, layout]` compiles behind `sml-drawings` while `layout` is inert with no
diagnostic. The two-field model answers *which list* a feature-gate tag belongs in — clearly
`pragmatic_slices`, since a Cargo feature is unambiguously our own build concern, never a
claim about the format's structure. It does **not** answer the second half of the problem: a
Cargo feature is compiled as a single `#[cfg]` predicate per field, so multi-membership in
`pragmatic_slices` (which the data model explicitly allows, same as `normative_slices`) still
has to collapse to one decision — gate on the intersection, gate on the first tag with the
rest silently inert (today's behavior), gate on an OR of all tags, or refuse multi-tagged
constructs a single feature and require the mapping be stated per-construct. That collapse
rule is a real design choice with real tradeoffs (binary size vs. granularity vs. surprise)
and this amendment does not make it — it is restated below as open question 5, for a human
call.

**This amendment changes only the ADR text — see `TODO.md` for whether the corresponding
`jats-fmt` schema/type/YAML update has landed yet, and whether it was committed separately
from this amendment.**

## Context

Every format vertical's completion claim rests on a ratio: "101/117 constructs covered."
The numerator is real work. The denominator was, until now, a hand-written list in
`fixtures/{format}/COVERAGE.md` — and a hand-written list cannot supply a trustworthy
denominator, because it only ever grows when somebody happens to notice something missing.

The 2026-07-28 COVERAGE.md audit (see `TODO.md`) measured exactly how badly this fails.
DocBook's denominator had drifted 94 → 105 → 117 and JATS's 106 → 109 → 133 across a
single session, each bump prompted by an incidental discovery during unrelated bug-fixing.
Diffing both checklists against the formats' authoritative element indexes found **265
DocBook element names and 216 JATS element names enumerated nowhere at all**. So "101/105
covered" was reporting 96% of a list someone wrote, not 96% of DocBook. The ratio was not
merely imprecise; it was measuring the wrong set, and no amount of care in maintaining the
checklist would have revealed that — a list can only be diligently maintained against
itself.

ADR 0004's 2026-07-28 amendment identifies the same failure in its own domain and names
the structural reason: *"a presence-checking pass over an incomplete list can find 'have
but shouldn't' but cannot structurally find 'should have but don't,' no matter how
carefully each listed entry is re-verified."* That amendment prescribes extracting the
format's full element index and diffing. This ADR is the mechanization of that
prescription: rather than each verification pass re-fetching and re-deriving the index by
hand — which is itself a fallible manual step, performed differently each time — the index
becomes a committed, machine-readable artifact with recorded provenance.

Existing prior art in this repo is `spec/ooxml-features.yaml` (3849 lines) plus
`spec/ooxml-names.yaml` and `spec/ooxml-events-*.yaml`, consumed by
`crates/formats/ooxml-*/build.rs` through `crates/tools/ooxml-codegen`. It establishes
several patterns worth keeping (a committed derived artifact; an escape hatch for
"everything under here"; regeneration gated behind an env var so contributors need not
hold the spec) and several worth not repeating: workspace-central placement, a tag
vocabulary that is a header comment rather than data and has already drifted from the data
it describes, an advisory-only completeness lint (`analysis.rs`, gated behind
`OOXML_ANALYZE`, whose `has_unmapped()` is dead code), and **zero provenance metadata of
any kind** — no spec edition, no checksum, no derivation date.

## Decision

**Decision 3 below is superseded by the 2026-07-28 amendment above — retained for the
historical record of what was originally decided (including its factual error about OOXML),
not as current guidance.** See the amendment for the corrected two-field model. Decisions 1,
2, and 4–9 are unaffected by the amendment and remain as originally decided.

Each `-fmt` crate carries a **construct registry**: a committed, machine-readable catalog
of every construct its format defines, derived from the format's own published schema.

### 1. Per-crate, not centralized

The registry lives in the `-fmt` crate (`crates/formats/jats-fmt/registry/`), not in
workspace-level `spec/`. This is a deliberate change from ooxml's current placement.

The `-fmt` crates are first-class ecosystem libraries; rescribe is one consumer among many.
Someone depending on `jats-fmt` alone — for a search indexer, a linter, a validator — has
the same need to ask "what does JATS define" as rescribe's coverage tooling does, and a
catalog in rescribe's workspace root is invisible to them. Placement follows the artifact's
audience, and the audience is the crate's users.

This also deletes a structural wart: ooxml's central YAMLs are keyed by a closed set of
four module names, mirrored by hard-coded `match module { "sml" => …, "wml" => … }` in
`ooxml-codegen`, whose fallback arm silently routes unknown modules to `sml`. Per-crate
files make each registry its own document, and the module dimension disappears.

### 2. Opt-in Cargo feature

`registry` is off by default. A consumer that only wants to parse XML should not compile,
or pay for, the catalog. The derivation tool sits behind a further `registry-derive`
feature, since it additionally needs the source schema present.

### 3. Slices come from the format's own modularization

Every construct is annotated with the **slice**(s) it belongs to. A slice is a partition
the format itself publishes — JATS's DTD Suite modules, TEI's 22 `<moduleSpec>` modules,
OOXML's namespace/part schemas. It is emphatically *not* an implementation concern
("things we've done", "things that are hard"): those churn with our work and tell a
downstream implementer nothing. A spec-published partition tells them how the format
actually decomposes, so "implement the section and para modules first" becomes a statement
the format supports.

Slices are first-class entries with an id, the module's *own* declared name, its source
file, and a resolvable URL — not the bare strings ooxml's feature tags are today. A
construct may belong to several slices; they are listed in the schema's own `<include>`
order, so `slices[0]` is a stable primary.

Where a format publishes no modularization, we say so and leave slices empty rather than
inventing a partition and presenting it as authoritative. **DocBook 5.2 is exactly this
case** — see Consequences.

### 4. Runtime-queryable

The registry is a typed, `serde`-serializable Rust API (`jats_fmt::registry`), not
build-time-only data that evaporates into `#[cfg]` attributes. `registry().elements()`,
`.in_slice(id)`, `.contains_element(name)`, `.not_handled(kind, handled)` are ordinary
runtime calls, and because the types implement `Serialize`, the whole document composes
with the `rescribe query` / jaq pattern without the registry depending on the query crate.

### 5. Support status is not in the registry

The registry is **spec-pure**: it records what the format defines, never what any crate
supports. Mixing the two would make it churn on every implementation commit, and would
recreate the original problem in a new file — a support column is a hand-maintained
claim, and hand-maintained claims are what this design exists to eliminate.

"Do we support X" is a **join**, performed by the consumer:
`Registry::not_handled(kind, what_i_handle)`. The pilot's consumer
(`crates/readers/rescribe-read-jats/tests/registry_coverage.rs`) supplies the numerator by
extracting element names from the reader's own source text and prints the difference.

### 6. Citations are external references, never file+line into a vendored schema

A citation must still resolve when the schema is absent from the checkout — because for
most formats it is. `spec/*` is gitignored: `git ls-files spec/OfficeOpenXML-RELAXNG-Transitional`
returns nothing, and the ECMA-376 schemas exist only in a developer's working tree after
running `scripts/ooxml/download-spec.sh`. A `file:line` citation into a file nobody has is
worse than no citation, because it looks actionable.

So citations are: a spec identifier (`ANSI/NISO Z39.96-2021`), a canonical base URL per
source file, and a per-construct URL template
(`…/tag-library/1.3/element/{name}.html`). These resolve from a bare checkout, from CI,
and from a downstream crate that has never heard of `spec/`.

### 7. The registry is a committed derived artifact, and provenance is load-bearing

Neither CI nor any downstream consumer will have the schema, so the registry cannot be
build-time codegen from a local input. It is generated once, committed, and read as data
— the same shape as ooxml's committed `generated.rs`, for the same reason.

That makes provenance the *only* way a reader can judge staleness, so it is recorded in
full rather than as decoration: spec identifier, schema form, driver file, canonical base
URL, the source license and whether it permits redistribution, whether this repo actually
vendors it, derivation date, deriving tool and version, and a **SHA-256 plus byte count for
every source file consumed**. ooxml's existing YAMLs have none of this; that gap matters
far more once the source input is unavailable to almost everyone who might want to check.

### 8. Verification mode

A developer who *does* fetch the schema can re-derive and diff:

```
scripts/jats/download-spec.sh
cargo run -p jats-fmt --features registry-derive --bin derive-registry -- \
    --schema-dir spec/jats-1.3-archiving-rng --check
```

`--check` exits non-zero on drift and names the constructs added or removed. It compares
*parsed documents*, not bytes, so reformatting the YAML does not read as spec drift. This
is the analogue of ooxml's `analysis.rs` lint, and the reason the "auditable, re-derivable
denominator" claim survives the schema being absent for everyone else.

### 9. YAML as the canonical committed form

The registry document is YAML, parsed at runtime under the `registry` feature. YAML rather
than generated Rust because the artifact is language-agnostic — the same reasoning that
makes `fixtures/` the primary deliverable applies here: a Python or Go JATS
implementation should be able to consume this file directly. Runtime parsing rather than
build-time codegen keeps a single source of truth with no committed-Rust copy to drift from
it; the cost is a few milliseconds on first access, and registry queries are audit tooling,
never a hot path.

## Consequences

- **The pilot found mechanically what the audit found by hand.** `jats-fmt`'s registry
  derives **305 JATS elements** (excluding embedded MathML) from the Archiving driver
  schema — against the ~306 the Tag Library's alpha index lists, i.e. the derivation is
  essentially exact. Of those, 176 are never mentioned anywhere in `rescribe-read-jats`'s
  source. Every element the 2026-07-28 hand audit called out — `hr`, `sub-article`,
  `response`, the `ruby`/`rb`/`rt`/`rp` family, `chem-struct`, `array`, `index-term`,
  `media`, `alt-text` — appears in that gap list without anyone having to notice it, and a
  regression test pins them so a future derivation cannot silently lose them.

- **The gap report is grouped by slice**, which turns a flat list of 176 names into
  workable units: all 18 funding elements, all 8 BITS question-and-answer elements, 40
  article-metadata elements. That is the decomposition argument for spec-sourced slices,
  demonstrated rather than asserted.

- **A derivation boundary was found and fixed rather than accepted.** The first derivation
  produced no `<table>`/`<tr>`/`<td>` at all. The cause: JATS embeds XHTML tables and
  MathML by reference (`JATS-XHTMLtablesetup1-3.ent` includes `xhtml-table-1.mod.rng`), so
  walking only the driver's direct `<include>` list loses them. Both the derivation tool
  and `scripts/jats/download-spec.sh` now resolve the include graph transitively. A
  hand-written checklist has no equivalent of this failure being *detectable*.

- **Redistributability is not uniform, and the design must not assume it is.** Verified
  per format:

  | Format | Schema | License | Vendorable? | Modularization |
  |---|---|---|---|---|
  | JATS 1.3 Archiving | DTD (canonical) + RNG + XSD | Public domain, per module headers; "do not redistribute modified versions" | Yes, verbatim | **Yes** — 29 DTD-suite modules, mirrored 1:1 in RNG |
  | TEI P5 | RELAX NG + DTD + XSD + ODD | CC BY 3.0 **or** BSD-2-Clause, chooser's option | Yes | **Yes** — 22 `<moduleSpec>`; membership is a literal `module=` attribute on each of 614 `<elementSpec>` |
  | DocBook 5.2 | RELAX NG (RNC + XML) | Permissive perpetual grant in the schema header | Yes | **No** — the normative OASIS artifact is a flattened monolith |
  | OOXML (ECMA-376) | RNC + XSD, Strict + Transitional | **Unresolved** — no copyright or license statement found in the schemas or in Parts 1/2 | **Assume not** | Yes — 21 namespace schemas, ~59 part entry-points |

  Formats with vendorable schemas *could* additionally support real file+line citations and
  fully local re-derivation. The design does not require that, precisely so one uniform
  citation form serves every format including OOXML.

- **DocBook cannot source *normative* slices from its normative schema.** Confirmed by
  fetching `docs.oasis-open.org/docbook/docbook/v5.2/os/rng/docbook.rnc`: 414 distinct
  elements in a single file, partitioned into **420 anonymous `div { }` blocks** carrying no
  module identity. The upstream TC source repo *is* modular (~35 named `.rnc` files) but is
  build source, not the normative artifact, and its license was not verified. Under the
  amended two-field model this no longer blocks rollout: DocBook's registry ships with
  `normative_slices: []` and a recorded reason, and may separately carry an invented, always-
  permitted `pragmatic_slices` grouping — see the amendment and open question 1.

- **Why JATS was piloted rather than DocBook.** DocBook has the larger known-failure
  dataset, but piloting it would have forced inventing a slice partition under the
  then-single-slice rule — the one thing decision 3 (as originally written) forbade — so it
  could not have validated the design's central claim. JATS has a normative machine-readable
  modularization, a public-domain license, a stable per-element citation URL, an RNG (XML, so
  `jats_fmt`'s own parser derives it with no new tooling and no dependency on
  `ooxml-codegen`'s RNC subset), *and* known-failure data. It exercises every part of the
  design at once. (This historical reasoning is why JATS went first, not a claim that
  DocBook is still blocked the same way — see the amendment.)

- **`jats-fmt` gained optional `serde`/`serde_yaml`/`sha2` dependencies**, all behind
  `registry`/`registry-derive`. A default build is unchanged. `sha2` is new to the
  workspace.

- **Costs.** Two artifacts must be kept honest: the committed YAML and the schema it came
  from. `--check` closes that loop but only for a developer who has fetched the schema, so
  drift is *detectable on demand*, not *prevented*. And a registry is only as good as its
  derivation: this one trusts that a construct is what the schema declares with
  `<element name="…">`, which is right for RELAX NG but will need a different reader per
  schema form (DTD, XSD, ODD).

## Open questions

Genuine forks, recorded rather than decided unilaterally:

1. **DocBook's `pragmatic_slices`.** *(Narrowed by the 2026-07-28 amendment — was "ship
   empty or adopt Codeberg," now only about the pragmatic field, since `normative_slices: []`
   with a recorded reason can ship unconditionally.)* Should DocBook's rollout populate
   `pragmatic_slices` at all, and if so, invent a fresh grouping or borrow the shape of the
   non-normative Codeberg TC source's ~35 modules (as an idea, not a redistribution — so its
   license, still unverified, does not need resolving for this)? Separately, and not blocking
   rollout: should the Codeberg source ever be used to populate `normative_slices` instead —
   i.e. treated as authoritative enough to cite as the format's own decomposition despite not
   being the normative artifact? That would need its license resolved first, because it would
   be a citation claim, not an invented grouping.

2. **Hand-curated registries.** *(Unaffected by the amendment — see the amendment's own note
   on why the two questions are separable.)* `SourceKind::HandCurated` exists for a format
   with no machine-readable schema, but such a registry does **not** deliver this design's
   guarantee — it is exactly as fallible as the checklist it replaces. Should such registries
   be allowed at all (clearly marked), or should a format with no schema keep a COVERAGE.md
   and not pretend otherwise? This is about the construct *list*, not about slices — a
   hand-curated `pragmatic_slices` grouping is already settled as fine by the amendment,
   independent of how this question resolves.

3. **OOXML's license.** No copyright or license statement was found in the ECMA-376 schema
   files or in Parts 1 and 2. Ecma publishes two boilerplate notices and neither is tied to
   ECMA-376 anywhere we could find. Microsoft's Open Specification Promise is a patent
   non-assert, not a copyright license. The conservative reading — treat as
   non-redistributable — is what the design assumes; confirming it needs someone who can ask
   Ecma. **This only gates a future `normative_slices` derivation from the ECMA-376 namespace/
   part schemas** (a real citation into the spec's own text); it does not gate
   `pragmatic_slices`, which is our own invented grouping and carries no redistribution claim
   either way.

4. **Attributes.** The pilot registers attributes alongside elements (144 of them). Whether
   the *content model* (which children/attributes each element permits) also belongs in the
   registry is unsettled: it is the natural next question a validator or linter consumer
   would ask, and it is a large increase in derivation complexity and document size.

5. **OOXML's slice/Cargo-feature collapse rule.** *(New, from the 2026-07-28 amendment.)*
   Once ooxml's tags become `pragmatic_slices`, a construct can still legitimately belong to
   several of them (`drawingHF: [drawings, layout]`), but a Cargo feature gate is a single
   `#[cfg]` predicate per field. Today `primary_feature` silently keeps only the first tag and
   drops the rest with no diagnostic. The two-field model settles *which list* the tags live
   in but not *how a multi-membership list becomes one gate* — intersection, first-tag
   (today's silent behavior), OR-of-all, or a hard requirement that multi-tagged constructs
   name their gate explicitly. This needs a human call before or during the ooxml migration
   (`TODO.md` rollout item 3); it is not decided here.

## Alternatives considered

- **Keep hand-written COVERAGE.md, apply ADR 0004's absence-check methodology more
  diligently.** Rejected: the methodology is right, but performing it by hand is a fallible
  manual step repeated per verification pass, with the fetched index discarded each time.
  That is precisely how the denominator drifted 94 → 105 → 117 while every individual pass
  was conscientious. Mechanizing the index is what makes the check repeatable.

- **One central registry for all formats,** extending `spec/ooxml-*.yaml`. Rejected: it
  serves rescribe and nobody else, and the ecosystem consumers of the `-fmt` crates are the
  point (CLAUDE.md's priority hierarchy puts them above rescribe's own needs).

- **Derive at build time from the schema.** Rejected: neither CI nor downstream consumers
  have the schema, and for OOXML they may not legally be given it.

- **Put support status in the registry** (a `supported: bool` per construct). Rejected: it
  would churn the registry on every implementation commit and reintroduce a hand-maintained
  claim. The join is cheap and cannot go stale.

- **Generate Rust instead of shipping YAML.** Rejected: it would make the catalog useless to
  non-Rust implementations, against the same reasoning that makes `fixtures/` the primary
  deliverable.

- **Reuse `ooxml-codegen`'s `parse_rnc` for the pilot.** Not applicable to JATS (its schema
  is RNG, i.e. XML) and not currently viable in general: that parser handles no `grammar`,
  `include`, or `|=`/`&=` combine, and discards RNC annotations before lexing. It is the
  right foundation for a DocBook registry, but needs that work first — see `TODO.md`.
