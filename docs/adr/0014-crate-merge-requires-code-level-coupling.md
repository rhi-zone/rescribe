# 14. Merging crates requires genuine code-level coupling, not topical similarity

## Status

Accepted.

## Context

TODO.md's "ooxml-fmt rework" plan (`### ooxml-fmt rework`, "Consolidation" section)
includes a checklist item to merge `ooxml-wml`, `ooxml-sml`, `ooxml-pml`, `ooxml-dml`,
`ooxml-omml`, `ooxml-opc`, `ooxml-xml` — seven crates — into a single `ooxml-fmt` crate
with feature flags.

This session investigated (read-only, no code changes) where that specific plan came
from, since it was about to be treated as settled groundwork for further work. It has
never had any stated rationale anywhere in this repo's docs or git history. It was
asserted cold in the commit that introduced it, `53c21c77d6` ("docs(todo): note
ooxml-fmt consolidation and deprecation plan", 2026-03-24), whose message gives no
reasoning — and it has been restated as settled fact in every later mention since, with
zero derivation at any point.

The one real, documented rationale that exists near this topic is a different decision
entirely. `c2fea87315` ("chore: merge ooxml workspace into rescribe monorepo") states
"there's no principled reason to keep it external — all format libraries live here" —
but that sentence justifies bringing the ooxml crates into the same git
repo/workspace, not collapsing them into one Cargo crate. That commit's own message
lists "consolidate into ooxml-fmt with feature flags" as a separate "next step" it
explicitly does not itself justify.

So the seven-way merge has been carried forward for months as apparently-settled
groundwork with no actual argument behind it. This ADR does the derivation that was
skipped, and states the general principle so future crate-organization questions in
this repo don't repeat the gap.

### The test

Whether to merge two or more crates should be decided by one question: do they have a
genuine, code-level, load-bearing dependency that forces them to change in lockstep to
remain correct — or is the relationship merely topical/categorical ("both are part of
the same broader standard," "both are document formats") with no such forcing
function? Only the former is a legitimate reason to merge.

### Applying the test

**Real edge, correctly merged (already done this session, retroactively validated by
this test).** Per-format `rescribe-read-{format}`/`rescribe-write-{format}` adapter
crates were folded into each format crate's own `rescribe` feature module. Read and
write of the *same* format share the same AST type and the same
property-modeling/naming decisions; a change to how one property is modeled on the
read side must be mirrored on the write side or round-tripping
(`parse(emit(doc)) == doc`) breaks. That's a real, load-bearing coupling forcing
lockstep changes — a legitimate merge.

**No real edge: `ooxml-wml`/`ooxml-sml`/`ooxml-pml` do not depend on each other's code
at all.** They are siblings that each depend on shared foundational crates
(`ooxml-opc`, `ooxml-dml`, `ooxml-omml`, `ooxml-xml`), but a bug fix or breaking change
in `ooxml-wml` (DOCX-specific) has zero reason to ever touch `ooxml-pml`
(PPTX-specific) code, or vice versa. Merging siblings with no mutual dependency into
one crate costs real things with no offsetting benefit:

- **Compile-unit granularity.** Editing one sub-component forces full-crate
  recompilation of everything else bundled into the merged crate — directly
  undermining the kind of parallel multi-agent crate-level work this session relied on
  heavily, where many agents each edit a different format crate concurrently.
- **Feature-unification leakage.** rescribe is a large multi-crate Cargo workspace.
  Cargo's additive feature unification means one workspace member enabling a feature
  can drag it into the build for another member that never asked for it. Merging
  wml/sml/pml into one crate with feature flags reintroduces exactly the "pay for only
  what you use" failure that per-construct feature gating (ADR 0011) exists to avoid —
  at the crate-boundary level instead of the construct level.
- **Forced lockstep versioning.** One crate means one version number. A breaking
  change to any bundled sub-component — even one a given consumer never uses — forces
  a major-version bump felt by every consumer of the merged crate.

**Not yet shown to be a real edge: `ooxml-opc`/`ooxml-dml`/`ooxml-omml`/`ooxml-xml`,
the shared foundation used across wml/sml/pml.** This session confirmed that
`ooxml-wml` and `ooxml-pml` both import and use `ooxml-dml`'s shape types directly.
That fact does not clear the bar this ADR sets, though — "multiple crates import and
use this shared crate" is an ordinary library-consumer relationship, the same shape as
"every crate in this workspace depends on `serde`." It is symmetric in a way that can
be used to justify grouping almost any set of crates around whichever shared
dependency you pick, and it says nothing about whether a change to `ooxml-dml` forces
a correctness-preserving change in `ooxml-wml` or vice versa — the actual test. This
is the same shape of argument as an earlier, separately-rejected claim in this
session's discussion ("shared dependency implies version-compat risk," rejected as
unsupported for the same reason), so it does not get to stand as a "plausible" case
here either. Without actual evidence of forced-lockstep coupling — a change on one
side breaking correctness unless mirrored on the other, the same kind of evidence that
supports the read/write-adapter case — the opc/dml/omml/xml merge is exactly as
unverified as the wml/sml/pml case, not closer to justified. It is left open on the
same footing.

**Reductio check.** Applying the same test to the hypothetical "merge all ~50+ format
crates in this repo (`rst-fmt`, `org-fmt`, `commonmark-fmt`, etc.) into one giant
rescribe crate with per-format features" gives the same answer as wml/sml/pml, for the
same reason: no code-level dependency between them, purely topical "both are document
formats" relationship. The scale makes the cost more visible, not different in kind —
this session's own large parallel migration (dozens of concurrent agents each
independently editing a different format crate) depended structurally on format
crates being separate compilation units. A mega-crate would make that workflow
impossible or catastrophically slow by forcing a full-codebase rebuild on every edit
anywhere in it.

## Decision

Merge crates only when they share a genuine code-level dependency that forces
lockstep changes to preserve correctness. Do not merge crates whose relationship is
merely topical or categorical — including "part of the same broader standard/spec
family" — since compile-granularity loss, feature-unification leakage (in this
specific repo's multi-crate-workspace context), and forced lockstep versioning are
real, ongoing costs with no offsetting technical benefit in the topical-only case.

This is the general rule for future crate-organization questions in this repo, not
just a one-off ruling on ooxml.

## Consequences

- TODO.md's ooxml-fmt rework "Consolidation" checklist item (merging all seven ooxml
  crates into one `ooxml-fmt`) does not currently meet this bar for the wml/sml/pml
  portion specifically. It is flagged as blocked pending verification rather than
  dropped or carried forward as-is: either drop that part of the plan (keep
  wml/sml/pml as separate crates, each depending on the shared opc/dml/omml/xml
  foundation), or someone independently verifies real code-level coupling between
  wml/sml/pml *themselves* (not just their shared use of the foundation crates) before
  proceeding with a full seven-way merge.
- The opc/dml/omml/xml foundation-crate merge question is left open, not decided by
  this ADR, and on the same footing as the wml/sml/pml case — not closer to
  justified. "Imported and used by multiple format crates" is not, by itself,
  evidence of forced-lockstep coupling (see "Not yet shown to be a real edge" above);
  it would need actual evidence that a change on one side breaks correctness unless
  mirrored on the other before being treated as a real edge under this test.
- The streaming-rework portion of the same TODO.md item (implementing
  `StreamingParser<H>`, package-level `events()`, etc.) is unaffected by this ADR and
  keeps its own independent, already-stated justification (DOCX/XLSX/PPTX files in
  legal discovery, academic corpora, and enterprise search routinely exceed available
  RAM). This ADR is about the crate-boundary question only, not the streaming
  architecture.

## Alternatives considered

- **Leave the seven-way merge as previously planned, unexamined.** Rejected: the plan
  was never derived from anything, and asserting it again would just be restating an
  ungrounded claim one more time.
- **Drop the merge item outright without recording why.** Rejected: the
  shared-foundation crates (opc/dml/omml/xml) are a genuinely open question — not yet
  shown to be a real edge, but not yet shown to be a non-edge either — and a silent
  drop would erase that open question rather than record it. Future readers need the
  test itself, not just this one ruling, to answer the next crate-organization
  question that comes up.
