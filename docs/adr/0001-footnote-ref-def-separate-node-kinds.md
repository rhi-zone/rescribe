# 1. `footnote_ref`/`footnote_def` as separate node kinds

## Status

Accepted.

## Context

A footnote has two distinct sites in a document: the inline marker where it's referenced and
the content of the note itself, which some formats place immediately at the marker (RTF keeps
footnote content inline in the run stream at the reference point) and others place elsewhere
(ODT, DocBook, JATS, TEI can hoist the definition to wherever the source document puts it,
independent of the marker's position).

## Decision

`footnote_ref` (inline node, appears at the marker's position in running text) and
`footnote_def` (block-shaped content) are separate `NodeKind`s. **How a given reader relates
them depends on the source format's own structure, and both of the following are legitimate,
currently-implemented shapes — this is not a single uniform rule:**

- **Embedded**: the note's content is placed directly as children of `footnote_ref`, and no
  `footnote_def` node is created at all. Used where the source format keeps footnote content
  inline at the marker with no separate addressable definition site: `rescribe-read-rtf`
  (`Inline::Footnote { content, .. }` → `Node::new(FOOTNOTE_REF).children(...)`,
  `crates/readers/rescribe-read-rtf/src/lib.rs:193-195`), `rescribe-read-docx`
  (`crates/readers/rescribe-read-docx/src/lib.rs:561-564,573-576`), and JATS's
  `<xref ref-type="fn">` (`crates/readers/rescribe-read-jats/src/lib.rs:809-814`).
- **Linked by label**: `footnote_ref` and `footnote_def` are separate nodes, each carrying the
  same `label` property value, and a consumer resolves the link by matching labels. Used where
  the source format has a genuinely separate, independently-addressable definition site: ODT
  (`crates/readers/rescribe-read-odt/src/lib.rs:444-450`, where `footnote_def` is collected and
  emitted after the paragraph) and DocBook's `<footnote>`/`<footnoteref linkend="…"/>` pair
  (`crates/readers/rescribe-read-docbook/src/lib.rs:1212,1219-1220`).

A reader picks whichever shape matches how the source format itself represents the
marker/content relationship; neither shape is deprecated or preferred over the other by this
ADR.

## Consequences

- A consumer that wants "the note text for this reference" cannot assume one shape: it must
  check whether `footnote_ref` already has children (embedded shape) before falling back to
  looking up a `footnote_def` by matching `label` (linked shape).
- Writers must reproduce whichever shape the reader originally chose (or the shape appropriate
  to the target format), rather than assuming a single universal footnote shape exists in the
  IR.
- `footnote_def` existing as its own `NodeKind` (rather than being folded entirely into
  `footnote_ref`'s children in every case) still matters for the linked shape: it lets a
  definition live at a different point in the tree than its marker, which the embedded shape
  cannot express.

## Open question

**Should the IR converge on one shape (most likely always-linked, since it can represent both
inline-at-marker and hoisted-elsewhere placement, while the embedded shape cannot represent
hoisting), with embedding readers migrated to synthesize a `footnote_def` and a label even when
the source format keeps the content inline?** This was not decided here: it is a real design
fork with a real migration cost across at least three readers, not a false-premise correction,
so it is recorded rather than resolved unilaterally. See `TODO.md` for tracking.

## Alternatives considered

- **One `footnote` node carrying both marker and content** (content as a property or nested
  child, with no separate ref/def kinds at all): rejected because it forces a single position
  in the tree, which doesn't match every format's actual structure, and would require a writer
  to relocate content on emit for formats where hoisting is the natural shape.
