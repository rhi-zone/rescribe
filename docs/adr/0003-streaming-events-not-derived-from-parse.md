# 3. `events()` is a true pull iterator; `parse()` is implemented as `events().collect()`, never the reverse

## Status

Accepted (session `fe139400`, ~2026-03-24).

## Context

An audit of the existing hand-rolled format crates (`rst-fmt`, `asciidoc`, `djot-fmt`,
`org-fmt`) found that every one of them implemented `events()` by calling `parse()` internally
to build the full AST, then draining a `VecDeque` of pre-computed events. This matches the
`events()` type signature but delivers none of its actual benefit: a caller using `events()`
expecting incremental, lower-memory processing over a large input silently gets full-tree
materialization first regardless. `rtf-fmt` was the one crate in the audit that had it right —
a genuinely lazy `{input, pos}` state machine that advances per `next()` call.

The tempting framing for the fix was "good enough for document conversion" — rescribe's own
call sites always end up wanting the full `Document` tree anyway, so does it matter if
`events()` secretly materializes first? That framing was explicitly rejected: the `-fmt` crates
are general-purpose Rust libraries, not rescribe internals (see `CLAUDE.md`), and a caller that
genuinely needs incremental/low-memory/event-driven processing gets silently broken by a fake
streaming API that happens to pass rescribe's own tests.

## Decision

`events()` must be a true pull iterator: `EventIter` holds parser state, and `next()` advances
the state machine and returns one event, with `Cow::Borrowed` slices from the input where the
format allows zero-copy. `parse()` is **not** implemented as `events().collect()` either,
because that formulation forces materialization through the event-dispatch layer and prevents
direct struct construction — sacrificing performance for code reuse. Instead: `parse()` = direct
recursive descent into the AST (no events, no intermediate representation, fastest path to a
materialized tree), and `events()` = its own independent lazy implementation. Both share
state-transition logic as plain functions, not a shared runtime primitive, so each stays
independently optimal.

(Note: the corrected framing this decision produced — "`parse()` is NOT `events().collect()`"
— is stated directly in `CLAUDE.md`'s "-fmt crates are not rescribe internals" section, which
also documents the third reader API, `StreamingParser<H>`, and its own distinct callback-model
requirements.)

## Consequences

- Every hand-rolled format crate needs three genuinely independent reader implementations
  (`parse()`, `events()`, `StreamingParser<H>`), not one primitive with two derived wrappers —
  more implementation surface per crate, verified by tests that exercise each API's actual
  laziness/streaming behavior (e.g. chunk-boundary-split tests for `StreamingParser`).
- A caller needing true incremental CommonMark/RST/Org/etc. processing gets a real answer from
  `events()` instead of a silent full-materialization trap.
- `commonmark-fmt` is the sole documented exception: it wraps `pulldown-cmark`, which requires
  a full `&str`, so its `StreamingParser` buffers all input before parsing. This is disclosed
  explicitly in the crate rather than silently accepted as the norm for other crates.

## Alternatives considered

- **`events()` implemented via `parse()` + drain** (the pre-existing pattern in
  `rst-fmt`/`asciidoc`/`djot-fmt`/`org-fmt`): rejected — matches the API surface but not its
  contract; explicitly named a "fake streaming API" that "fails silently for any caller that
  needs true incremental processing, low-memory operation, or event-driven pipelines."
- **`parse()` implemented as `events().collect()`**: rejected — technically would deliver a
  correct AST, but funnels every full-tree build through the event-dispatch layer and blocks
  direct struct construction, which is strictly slower for the common case of just wanting the
  tree. The behavior must be equivalent across all three APIs; the implementation must not be.
