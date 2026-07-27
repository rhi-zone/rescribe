# 1. `footnote_ref`/`footnote_def` as separate node kinds

## Status

Accepted (in force since early IR scaffolding; reaffirmed as a load-bearing constraint on
format-crate AST design during `rtf-fmt` adapter work, session `112b1bb9`, ~2026-03-02).

## Context

A footnote has two distinct sites in a document: the inline marker where it's referenced
(`footnote_ref`, appearing wherever the author placed the superscript number) and the block
content of the note itself (`footnote_def`, which most formats collect elsewhere — endnotes,
a notes section, or an inline `<footnote>` depending on format). A single combined node kind
carrying both the marker position and the definition content as a property would conflate two
things that live at different points in the tree and round-trip through different mechanisms
per format (RTF keeps footnote content inline at the marker; DocBook/JATS/TEI can place the
definition anywhere; HTML/CommonMark conventionally split marker and endnote section).

This surfaced concretely while building `rtf-fmt`'s adapter: the format crate had to choose
between representing RTF footnotes as a separate `footnotes: Vec<(usize, Vec<Block>)>` side
list on `RtfDoc`, or as an inline `Block::Footnote` sitting in the main block stream at the
reference point. The choice was constrained by an existing rule: rescribe's IR expects
`footnote_ref` and `footnote_def` as separate node kinds that can each appear at the document
level independently of each other's position.

## Decision

`footnote_ref` (inline node, appears at the marker's position in running text) and
`footnote_def` (can appear inline or hoisted to a document-level collection, depending on what
the source format does) are separate `NodeKind`s, linked by an id/label property rather than
one node embedding the other as a child or property.

## Consequences

- Format readers are free to place `footnote_def` wherever the source format naturally puts
  it (inline for RTF, a trailing notes section for CommonMark/HTML-style conventions,
  anywhere in the tree for DocBook/JATS/TEI's own footnote elements) without forcing every
  format into one convention.
- Writers reconstruct the format-appropriate placement from the id link rather than needing
  the reader to have already normalized position.
- Cost: any consumer of the IR that wants "the note text for this reference" must resolve the
  link (look up the matching `footnote_def` by id) rather than reading it directly off the
  `footnote_ref` node — a small extra indirection in exchange for not baking one format's
  placement convention into the IR shape.

## Alternatives considered

- **One `footnote` node carrying both marker and content** (content as a property or nested
  child): rejected because it forces a single position in the tree, which doesn't match every
  format's actual structure (see RTF's inline-at-marker vs. DocBook's hoistable-anywhere
  behavior) and would require the writer to relocate content on emit for formats where that's
  not the natural shape.
