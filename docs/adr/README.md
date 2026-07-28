# Architecture Decision Records

This log records IR-shape and format-library design decisions for rescribe: what was decided,
what tradeoff prompted it, and what alternatives were rejected and why. It exists so a future
session (or an external contributor) doesn't have to re-derive a decision that was already made
— or, worse, silently re-litigate it differently in one format vertical but not another.

**Scope**: decisions about the IR (`Document`/`Node`/`NodeKind`/`Properties`/`PropValue`), the
three-reader/two-writer API contract format crates must satisfy, and cross-cutting methodology
(e.g. how a classifier gets schema-verified). Not in scope: routine bugfixes, fixture additions,
or per-format construct mappings that don't reflect a genuine design fork — those belong in
`TODO.md` and `docs/format-audit.md`.

**Status key**: `Accepted` means the decision is settled and reflected in code (or, where noted,
settled as a plan with implementation still pending — the ADR says which). rescribe has no
`Proposed`/`Superseded` entries yet; add that status vocabulary if/when a decision here is
later revisited.

**Amendments are for genuine supersession only** — a decision that was correct when made and
was later legitimately changed by new context (a spec clarification, a new requirement). An
amendment is the wrong tool for correcting a claim that was false at the time the ADR was
written; that gets rewritten in place instead, with no struck-through history preserved,
because the false claim was never legitimately "current guidance" to begin with. (A 2026-07-28
audit found several ADRs in this log had used "Amendment" sections to patch factual errors
rather than to record genuine supersession; those ADRs — 0001, 0002, 0004, 0007, 0011, 0013 —
were rewritten in place under this convention rather than left with amendment scaffolding on
top of a wrong original. Same file, same number, on the reasoning that nothing referencing the
old text was relying on a correct decision — see each ADR's git history for the prior,
incorrect text if needed.)

## Index

| ADR | Title | Status |
|-----|-------|--------|
| [0001](0001-footnote-ref-def-separate-node-kinds.md) | `footnote_ref`/`footnote_def` as separate node kinds | Accepted |
| [0002](0002-span-semantics-include-delimiters.md) | Span semantics: full syntactic construct including delimiters | Accepted |
| [0003](0003-streaming-events-not-derived-from-parse.md) | `events()` is a true pull iterator; `parse()` is implemented as `events().collect()`, never the reverse | Accepted |
| [0004](0004-xml-classifier-schema-verification-methodology.md) | Schema-verification methodology for block/inline element classifiers: absence-check first, then entry-check | Accepted |
| [0005](0005-citation-bibliography-ir-shape.md) | Citation/bibliography IR shape: `bibliography`/`bibliography_entry`/`bibliography_field` | Accepted |
| [0006](0006-content-model-decides-properties-vs-child-nodes.md) | Properties-vs-child-nodes is decided by content-model markup permission, not precedent | Accepted |
| [0007](0007-dtd-entity-resolution-build-vs-buy.md) | DTD-aware entity resolution: standalone `xml-entities` crate, layered over a bought standard table | Accepted; implemented |
| [0008](0008-ris-sn-tag-not-disambiguated.md) | RIS `SN` tag stays `field:scheme = "sn"`, not resolved to `isbn`/`issn` via `TY` | Accepted |
| [0009](0009-propvalue-float-json-sentinel.md) | `PropValue::Float` non-finite values serialize as a string sentinel in JSON | Accepted |
| [0010](0010-resource-data-base64-json.md) | `Resource::data` serializes as base64 in JSON, unconditionally | Accepted |
| [0011](0011-commonmark-extension-feature-gating.md) | `commonmark-fmt` construct extensions are opt-in Cargo features, not default-on | Accepted |
| [0012](0012-jats-archiving-tag-set-scope.md) | `jats-fmt`/`jats` fixtures target the Archiving and Interchange Tag Set; no per-tag-set crates or validation modes | Accepted |
| [0013](0013-per-format-construct-registry.md) | Per-format construct registries: a spec-derived, machine-readable denominator, committed as generated Rust statics | Accepted |

## Numbering and format

Sequential, never reused. Each ADR is a standalone file: title, status, context, decision,
consequences, and alternatives considered (where applicable). Filenames: `NNNN-kebab-title.md`.
