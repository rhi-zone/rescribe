# 6. Properties-vs-child-nodes is decided by content-model markup permission, not precedent

## Status

Accepted (derived alongside ADR 0005, commit `4e15c9966e`, 2026-07-28).

## Context

ADR 0005 (citation/bibliography IR shape) needed a general rule for when a construct's content
belongs in a `Properties` bag versus as child nodes, because the wrong call silently drops
nested markup (a flat string can't carry an embedded `emphasis` or `link`). The draft proposal
for that decision initially reached for "match existing convention," citing table cells and list
attributes as precedent for using Properties for "non-prose structured data." That precedent
turned out to be wrong on inspection: `table`, `table_row`, and `table_cell` are themselves node
kinds in `rescribe-std`, and `table_cell`'s *content* is child nodes, not a property — only
attributes like `colspan`/`rowspan` are Properties. The convention actually already in the
codebase argues the opposite of what the draft claimed.

This is worth recording as its own principle, separate from the citation decision it was
derived alongside, because it's a general rule for any future node-kind design (equations,
metadata blocks, form fields, whatever comes next) — not something specific to bibliographies.

## Decision

**Whether a piece of content is a Property or a child node is decided by whether the source
format's content model permits nested markup for that field, in at least one format under
consideration — not by convenience, not by how "structured" or "atomic-looking" the data
appears, and not by an appeal to existing precedent without first checking that the precedent
actually says what it's claimed to say.**

Operationally: if any surveyed format's schema allows phrase-level markup (emphasis,
abbreviations, links, editorial tags) inside a given field/element, that field must be
represented as a child node (or a node whose children are inline nodes), because a flat
`PropValue::String` would silently drop that markup for any document that uses it — a direct
violation of CLAUDE.md's losslessness principle. A field stays a flat Property only when it is
atomic and non-markup-bearing across every format actually checked (e.g. `date` sub-parts in
ADR 0005).

## Consequences

- This is now the standard test to apply before adding any new node kind or property to
  `rescribe-std`: check the content model, don't reach for the "this looks like data not prose"
  intuition.
- Requires actually checking the relevant format schemas (or explicitly flagging as unverified
  if a schema can't be reached) rather than asserting the answer from general impression — same
  discipline as ADR 0004's schema-verification methodology, applied to node-shape decisions
  instead of block/inline classification.
- A stated "existing convention" is not itself evidence — this decision exists specifically
  because an existing-convention claim was made, sounded plausible, and was wrong. Any future
  appeal to precedent should be checked against the actual node-kind/property definitions in
  `rescribe-std`'s source, not against a remembered summary of them.

## Alternatives considered

- **Decide flat-vs-node by data "shape" (e.g. dates and identifiers feel like data, titles and
  names feel like prose)**: rejected as too subjective and, per the table_cell precedent check
  above, not actually how the codebase already behaves — it would readmit exactly the kind of
  convenience-based judgment call this principle exists to rule out.
- **Default to Properties and only promote to child nodes when a bug report shows markup was
  dropped**: rejected — this is reactive rather than schema-verified, and CLAUDE.md's
  losslessness rule requires the fidelity check up front, not after a real document already lost
  content silently.
