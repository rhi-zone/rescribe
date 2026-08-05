# `zip-fmt` and the fixture suite: a documented deviation

`zip-fmt` does not have a `fixtures/zip-fmt/{feature}/input.zip` +
`expected.json` suite in the shape `fixtures/spec.md` defines, and this file
records why, rather than silently omitting it.

## Why the existing fixture format doesn't map

`fixtures/spec.md`'s `expected.json` schema asserts against a **rescribe
document tree**: `path`/`kind`/`props` assertions walk `document.content`,
where `kind` is a rescribe `NodeKind` (`paragraph`, `heading`, `text`, ...)
and `props` are rescribe `Properties`. That schema is inherently about
*interpreting content as a document* — every existing fixture suite
(`markdown`, `rst`, `docx`, `epub`, ...) is for a format whose bytes decode
into prose/structure.

`zip-fmt` is not that kind of crate. Per its own crate-level docs (see
`crates/formats/zip-fmt/src/lib.rs`), it is a **container** library: it
reads and writes the ZIP envelope and hands callers `(name, bytes)` pairs
per entry. It has no concept of a paragraph, a heading, or any document
node — those only exist once some other crate (`epub-fmt`, `ooxml-wml`,
...) interprets an entry's bytes as a document. Forcing `zip-fmt`'s own
correctness properties (entry list, per-entry compression method,
sizes/CRC-32, extra-field bytes, data-descriptor handling, ...) through
`expected.json`'s `kind`/`props` node-assertion vocabulary would not
describe what this crate actually does — there is no document tree to
assert paths into.

Compare: `fixtures/epub/` already exists and *does* use the standard
schema — because it is a fixture suite for the future `epub-fmt` →
rescribe-IR adapter (asserting `paragraph`/`heading`/... nodes extracted
from an EPUB's XHTML content), not for `zip-fmt`'s container layer that
EPUB happens to sit on top of. The two are testing different layers of
the same file format by design, and only one of them produces a document
tree.

## What actually exercises `zip-fmt`'s correctness properties instead

- `crates/formats/zip-fmt/src/lib.rs`'s `#[cfg(test)] mod smoke` —
  construct-a-known-archive-and-assert-on-`Entry`/`Event` fields tests for
  each of the five APIs (`parse`, `events`, `StreamingParser`, `emit`,
  `Writer`), including the data-descriptor entry-metadata-timing behavior
  `StreamingParser` exists for.
- `crates/formats/zip-fmt/tests/roundtrip.rs` — a seeded-PRNG property
  test (`emit(archive) → parse → assert equality` and
  `emit(archive) → StreamingParser → assert equality`) run over 260 seeds,
  the same property `fuzz/fuzz_targets/zip_fmt_roundtrip.rs` checks against
  truly arbitrary input.
- `fuzz/fuzz_targets/zip_fmt_reader.rs` (no-panic gate) and
  `fuzz/fuzz_targets/zip_fmt_roundtrip.rs` (arbitrary-`Archive` round-trip
  property, per `CLAUDE.md`'s roundtrip-direction rule).

## If this changes

If the ecosystem grows more binary-container `-fmt` crates (a future
`tar-fmt`, `sevenz-fmt`, ...), a genuinely reusable container-fixture
format — asserting entry lists/metadata/content directly, without a
document-tree detour — would be worth adding as a sibling schema in
`fixtures/spec.md`. That is a real, deliberate spec extension, not
something this task should invent unilaterally for a single crate;
flagged here as a follow-up rather than resolved.
