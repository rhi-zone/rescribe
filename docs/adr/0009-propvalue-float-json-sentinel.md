# 9. `PropValue::Float` non-finite values serialize as a string sentinel

## Status

Accepted (commit `23b73cd4c8`, 2026-07-28).

## Context

`rescribe-core`'s `serde` feature (wiring up the previously-declared-but-dead `serde`
Cargo feature, done as part of adding `rescribe query`) needs a `Serialize` impl for
`PropValue`, which includes `Float(f64)`. `f64::NAN`, `f64::INFINITY`, and
`f64::NEG_INFINITY` have no JSON representation — the JSON spec's `number` production
only covers finite decimal values. `serde_json`'s default float serialization errors (or,
depending on version/features, silently emits `null`) for non-finite values.

This is a human decision, not a technical fact: there is no single "correct" JSON
representation of a non-finite float, and CLAUDE.md's losslessness stance ("silent drops
are failures") rules out the silent-`null` option without at least a caveat.

Three options were on the table:
1. **Reject with an error** naming the offending property path — keeps the JSON output
   strictly conformant, but makes `query` (and any future JSON export) unusable for any
   document containing a stray `NaN`/`Infinity` property, which is a disproportionate
   failure mode for what is usually a display/debugging concern in the source format.
2. **Coerce to `null` with a fidelity warning** — matches how `ConversionResult` already
   surfaces lossy conversions elsewhere in the codebase, but a reader consuming the JSON
   has no way to recover "this was `null` because non-finite" versus "this was genuinely
   `null`" without out-of-band warning plumbing that `serde_json::to_value` has no channel
   for.
3. **String sentinel** (`"NaN"`, `"Infinity"`, `"-Infinity"`) — the value survives in the
   JSON output and is inspectable/greppable, at the cost of being indistinguishable from a
   genuine string property with that exact text.

## Decision

Non-finite `PropValue::Float` values serialize as the string sentinels `"NaN"`,
`"Infinity"`, and `"-Infinity"` (see `serialize` impl in
`crates/rescribe-core/src/properties.rs`). Finite floats serialize as ordinary JSON
numbers.

This is an **acknowledged compromise, not a lossless representation**. A `PropValue::Float(f64::NAN)`
and a `PropValue::String("NaN".to_string())` are indistinguishable once serialized to
JSON — a consumer of `query` output (or any future JSON export built on this `Serialize`
impl) cannot tell them apart without an out-of-band convention. This is consistent with
the broader decision (recorded in the `rescribe-core` serde commit) to implement
`Serialize` only, not `Deserialize`, for the IR: the JSON mapping is deliberately
one-directional and not intended to be a lossless wire format.

## Consequences

- `rescribe query` and any other JSON export built on `rescribe-core`'s `serde` feature
  can serialize every document without erroring, including documents with non-finite
  float properties (rare in practice — most sources of `PropValue::Float` are unit
  conversions or parsed layout values that don't produce non-finite results — but the
  type permits it, so the impl must handle it).
- A jq filter like `.props.size == "NaN"` will incorrectly match a genuine string
  property with that text, and `(.props.size | type) == "number"` will be `false` for a
  non-finite float even though the source property is typed `Float`. Callers that need to
  distinguish these cases cannot do so from `query` output alone today.
- **Reopening condition**: if round-trip fidelity through query output (or any JSON
  export path) becomes a requirement — e.g. a future `rescribe import-json` that
  reconstructs a `Document` from this JSON shape — this decision needs revisiting, most
  likely toward a tagged representation (e.g. `{"$float": "NaN"}`) that isn't ambiguous
  with a real string, at the cost of being uglier for the common finite-number case and
  for jq filters that don't care about the distinction.

## Alternatives considered

- **Reject with an error**: rejected — disproportionate failure mode; would make `query`
  fail closed on documents where the non-finite value is incidental to what the caller
  is actually querying for.
- **Coerce to `null` with a fidelity warning**: rejected — `serde_json::to_value` (the
  mechanism `rescribe query` and any generic JSON export use) has no return channel for
  warnings alongside the `Value`, so the warning would need a separate side-channel API
  that doesn't exist yet; the sentinel string is strictly more informative than `null`
  with no side-channel, at equivalent implementation cost.
