# 2. Span semantics: full syntactic construct including delimiters

## Status

Accepted (session `fe139400`, ~2026-03-23).

## Context

`pulldown-cmark` and `tree-sitter-md` backends disagreed on byte-range boundaries for the same
construct — e.g. whether a `strong` node's span for `**bold**` includes the surrounding `**`
delimiters or covers only the inner content `bold`. This surfaced as parity-test failures
between the two CommonMark backends.

The easy way out was already sitting in the codebase: `strip_spans()` existed for structural-only
test comparisons, and the immediate instinct was to lean on it permanently here too — treat span
disagreement as irrelevant since only tree structure is asserted in parity tests. That was
challenged directly: stripping spans to make the disagreement disappear papers over the
inconsistency rather than fixing it, and leaves the IR without a consistent, implementation-
independent definition of what a span *means*. A test suite that can't tell the two backends
apart on span correctness isn't verifying anything about spans at all.

## Decision

A node's span is defined to cover the **full syntactic construct, including delimiters** —
`**bold**`'s `strong` span runs from the opening `**` to the closing `**` inclusive, not just
the inner text. This is now stated as IR-level semantics (see `CLAUDE.md`), not an
implementation detail either backend gets to define for itself. When two backends disagree on
span boundaries, that disagreement is a bug in whichever one doesn't match this definition —
not a reason to strip spans and move on.

`strip_spans()` remains valid, but scoped down: it's for structural-only tests where span
correctness genuinely isn't the thing under test. Span correctness itself gets its own,
separate test coverage, so a backend can't silently regress on span boundaries just because the
structural tests still pass with spans stripped.

## Consequences

- Every format backend must conform its span-producing code to this definition, which surfaced
  and fixed real span-boundary bugs rather than hiding them behind `strip_spans()`.
- Test suites need two tiers: structural-equality tests (spans stripped) and span-correctness
  tests (spans compared exactly) — one no longer substitutes for the other.
- Cost: backends that previously computed spans loosely (e.g. reusing an inner-content range)
  needed rework to track delimiter boundaries precisely.

## Alternatives considered

- **Span = inner content only, excluding delimiters**: not chosen; would still need to be
  applied consistently across backends, and offers no particular advantage over the
  full-construct definition — the deciding factor was consistency and testability, not which
  boundary convention is inherently better.
- **Permanently strip spans for all parity/backend-comparison tests**: rejected — this doesn't
  fix the underlying inconsistency, it just makes the test suite blind to it. Explicitly named
  as "papering over" the problem rather than solving it.
