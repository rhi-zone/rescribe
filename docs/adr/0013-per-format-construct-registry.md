# 13. Per-format construct registries: a spec-derived, machine-readable denominator

## Status

Accepted (2026-07-28). Pilot implemented for JATS 1.3 Archiving in `jats-fmt`
(`registry` / `registry-derive` features). Rollout to DocBook, TEI, and the OOXML
migration is planned but not done — see `TODO.md`.

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

- **DocBook cannot source slices from its normative schema.** Confirmed by fetching
  `docs.oasis-open.org/docbook/docbook/v5.2/os/rng/docbook.rnc`: 414 distinct elements in a
  single file, partitioned into **420 anonymous `div { }` blocks** carrying no module
  identity. The upstream TC source repo *is* modular (~35 named `.rnc` files) but is build
  source, not the normative artifact, and its license was not verified. DocBook's registry
  should therefore ship with empty slices and a recorded reason, or adopt the non-normative
  modularization with that provenance stated explicitly — an open question, below.

- **Why JATS was piloted rather than DocBook.** DocBook has the larger known-failure
  dataset, but piloting it would have forced inventing a slice partition — the one thing
  decision 3 forbids — so it could not have validated the design's central claim. JATS has
  a normative machine-readable modularization, a public-domain license, a stable per-element
  citation URL, an RNG (XML, so `jats_fmt`'s own parser derives it with no new tooling and
  no dependency on `ooxml-codegen`'s RNC subset), *and* known-failure data. It exercises
  every part of the design at once.

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

1. **DocBook slices.** Ship an empty slice set with a recorded "the normative schema
   publishes no modularization" reason, or adopt the non-normative Codeberg TC source's ~35
   modules with that non-normativity stamped in provenance? The first is strictly honest and
   less useful; the second is more useful and cites a non-authoritative source. Its license
   was also not verified.

2. **Hand-curated registries.** `SourceKind::HandCurated` exists for a format with no
   machine-readable schema, but such a registry does **not** deliver this design's guarantee
   — it is exactly as fallible as the checklist it replaces. Should such registries be
   allowed at all (clearly marked), or should a format with no schema keep a COVERAGE.md and
   not pretend otherwise?

3. **OOXML's license.** No copyright or license statement was found in the ECMA-376 schema
   files or in Parts 1 and 2. Ecma publishes two boilerplate notices and neither is tied to
   ECMA-376 anywhere we could find. Microsoft's Open Specification Promise is a patent
   non-assert, not a copyright license. The conservative reading — treat as
   non-redistributable — is what the design assumes; confirming it needs someone who can ask
   Ecma.

4. **Attributes.** The pilot registers attributes alongside elements (144 of them). Whether
   the *content model* (which children/attributes each element permits) also belongs in the
   registry is unsettled: it is the natural next question a validator or linter consumer
   would ask, and it is a large increase in derivation complexity and document size.

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
