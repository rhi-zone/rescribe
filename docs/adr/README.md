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

## Index

| ADR | Title | Status |
|-----|-------|--------|
| [0001](0001-footnote-ref-def-separate-node-kinds.md) | `footnote_ref`/`footnote_def` as separate node kinds | Accepted |
| [0002](0002-span-semantics-include-delimiters.md) | Span semantics: full syntactic construct including delimiters | Accepted |
| [0003](0003-streaming-events-not-derived-from-parse.md) | `events()` is a true pull iterator; `parse()` is implemented as `events().collect()`, never the reverse | Accepted |
| [0004](0004-xml-classifier-schema-verification-methodology.md) | Schema-verification methodology for block/inline element classifiers | Accepted |
| [0005](0005-citation-bibliography-ir-shape.md) | Citation/bibliography IR shape: `bibliography`/`bibliography_entry`/`bibliography_field` | Accepted |
| [0006](0006-content-model-decides-properties-vs-child-nodes.md) | Properties-vs-child-nodes is decided by content-model markup permission, not precedent | Accepted |
| [0007](0007-dtd-entity-resolution-build-vs-buy.md) | DTD-aware entity resolution: buy the entity table, build the DTD subset parser | Accepted (design only; not yet implemented) |
| [0008](0008-ris-sn-tag-not-disambiguated.md) | RIS `SN` tag stays `field:scheme = "sn"`, not resolved to `isbn`/`issn` via `TY` | Accepted |

## Numbering and format

Sequential, never reused. Each ADR is a standalone file: title, status, context, decision,
consequences, and alternatives considered (where applicable). Filenames: `NNNN-kebab-title.md`.
