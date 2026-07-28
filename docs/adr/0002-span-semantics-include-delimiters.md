# 2. Span semantics: full syntactic construct including delimiters

## Status

Accepted.

## Context

Span boundaries are a case where "just don't specify it precisely" is tempting: `strip_spans()`
already exists for structural-only test comparisons, and it's easy to lean on it permanently —
treat span boundaries as an implementation detail no test needs to pin down. That temptation was
rejected: leaving span semantics implementation-defined means any two backends (or a backend
before/after a refactor) are free to disagree on where a node's span starts and ends, with
nothing to say which one is right, and a test suite that always strips spans before comparing
can't catch that disagreement even if it wanted to.

## Decision

A node's span is defined to cover the **full syntactic construct, including delimiters** —
`**bold**`'s `strong` span runs from the opening `**` to the closing `**` inclusive, not just
the inner text. This is stated as IR-level semantics (see `CLAUDE.md`), not an implementation
detail a backend gets to define for itself. When two backends disagree on span boundaries for
equivalent input, that disagreement is a bug in whichever one doesn't match this definition —
not a reason to strip spans and move on.

`strip_spans()` remains valid, but scoped down: it's for structural-only tests where span
correctness genuinely isn't the thing under test, not a general-purpose way to make span
disagreements stop mattering.

Verified against the live implementation: `commonmark-fmt`'s parser produces exactly this
boundary for `**bold**` — `Strong { span: Span { start: 0, end: 8 }, .. }` for input
`"**bold** and *em*"`, spanning both delimiter pairs
(`crates/formats/commonmark-fmt/src/parse.rs`, built on pulldown-cmark's `into_offset_iter()`
ranges).

## Consequences

- Every format backend must conform its span-producing code to this definition.
- Cost: a backend that computes spans loosely (e.g. reusing an inner-content range instead of
  tracking delimiter boundaries) needs rework to track delimiter boundaries precisely.
- **Not yet delivered**: a dedicated span-correctness test tier (spans compared exactly, as
  distinct from structural-equality tests that strip spans) is not currently implemented for
  `commonmark-fmt` or the other format crates. This ADR states the semantics; a test suite that
  actually pins them down for each backend remains open work, not something already in place.

## Alternatives considered

- **Span = inner content only, excluding delimiters**: not chosen; would still need to be
  applied consistently across backends, and offers no particular advantage over the
  full-construct definition — the deciding factor was consistency and testability, not which
  boundary convention is inherently better.
- **Leave span boundaries implementation-defined and always strip spans in comparison tests**:
  rejected — this doesn't fix the underlying potential for inconsistency, it just makes the
  test suite unable to see it.
