# 10. `Resource::data` serializes as base64 in JSON, by default and unconditionally

## Status

Accepted (commit `23b73cd4c8`, 2026-07-28).

## Context

`rescribe-core`'s `serde` feature needs a `Serialize` impl for `Resource`, which holds
raw embedded-resource bytes in `data: Vec<u8>` (images, fonts, and other binary content
embedded in DOCX/EPUB/ODT/PPTX documents). JSON has no native byte-string type. A naive
`#[derive(Serialize)]` on `Vec<u8>` serializes it as a JSON array of small integers — one
array element per byte, each several characters of `NNN,` text — which is technically
valid JSON but is a 5-10x size blowup over the binary and forces any JSON
consumer (including `jaq`, which `rescribe query` runs against this output) to walk
millions of array elements for a multi-megabyte image.

Three shapes were considered for what `Resource::data` becomes in JSON:
1. **Omit by default, metadata placeholder** (e.g. `{"omitted": true, "len": 82301}`) —
   keeps output terminal-friendly and avoids the encode cost entirely when a query never
   touches resource bytes, at the cost of `query` being unable to actually extract
   resource content (e.g. `.resources[] | .data` to pull an embedded image out) without an
   opt-in flag.
2. **Base64-encode, always** — full fidelity; any jq filter that wants the actual bytes
   can get them, and the value is still a valid (if verbose) JSON string. Cost: every
   `query` invocation against a document with embedded resources pays the base64 encode
   for every resource's `data`, even when the filter expression never references
   `.resources` at all — a real performance and memory cost for a multi-megabyte image in
   a DOCX/EPUB, since `Document::Serialize` walks the whole tree unconditionally.
3. **Hash only** (e.g. sha256) — useful for diffing/dedup queries without ever holding
   binary-adjacent data in the JSON output, but discards the ability to extract resource
   content via query, same limitation as (1).

## Decision

`Resource::data` serializes as a base64 string (`base64::engine::general_purpose::STANDARD`,
see `serialize_data_base64` in `crates/rescribe-core/src/resource.rs`), unconditionally —
there is no `--resources=omit|hash|base64` mode yet; every `Document::Serialize` call
encodes every embedded resource's bytes.

This is an **acknowledged performance/memory compromise, not a free choice**: a `query`
invocation against a document with several multi-megabyte embedded images pays the full
base64-encode cost for all of them on every run, regardless of whether the jq filter ever
reads `.resources`. This was accepted as the simplest correct default for the initial
`rescribe query` vertical — full fidelity beats silent data loss, and a lazy/opt-in
alternative was out of scope for this pass (see reopening condition).

## Consequences

- `rescribe query` and any other JSON export built on this `Serialize` impl can always
  extract resource bytes losslessly via a filter like `.resources[] | .data`, with no
  special mode required.
- Every serialization of a `Document` with embedded resources pays the base64 encode cost
  for all resources, unconditionally, even for queries that only touch `.metadata` or
  `.content`. For a document with many/large embedded resources (a DOCX with dozens of
  images, an EPUB with embedded fonts), this is a real per-query cost.
- **Reopening condition**: if this cost proves material in practice (e.g. large-corpus
  batch querying, as flagged as a future direction for `rescribe query`/`CompiledQuery`),
  revisit toward either (a) lazy/on-demand encoding — only base64-encode a resource's
  `data` if the filter actually dereferences it, which would require restructuring the
  `Document → Val` conversion to not eagerly materialize the full JSON tree, or (b) a
  `--resources=omit|hash|base64` flag exposed on `rescribe query` (and a corresponding
  parameter on the library `query`/`CompiledQuery` API) defaulting to a cheaper mode with
  base64 as explicit opt-in. Neither was implemented in this pass; this ADR exists so the
  tradeoff is visible before someone hits it on a large corpus rather than being
  rediscovered from a profiler.

## Alternatives considered

- **Omit by default with a metadata placeholder**: rejected as the *default* — silently
  dropping resource bytes from `query` output contradicts the "no silent drops" stance
  this codebase holds for format conversion, and there's no reason `query`'s JSON export
  path should hold itself to a lower fidelity bar than the format readers/writers do,
  even though the performance argument for doing so is real (see reopening condition
  above — this remains the most likely shape for an opt-in cheaper mode later).
- **Hash-only**: rejected as the default for the same reason; still useful as a future
  opt-in mode for dedup/diff-style queries, not implemented here.
