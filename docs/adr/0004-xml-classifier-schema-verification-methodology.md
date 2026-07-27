# 4. Schema-verification methodology for block/inline element classifiers

## Status

Accepted (docbook: commit `abd6dd447d`, 2026-07-27; tei: commit `3e3d84bcef`; jats: commit
`20c27d032e`, 2026-07-27).

## Context

`rescribe-read-docbook`, `rescribe-read-jats`, and `rescribe-read-tei` each carry an
`is_block_element(tag)` classifier that decides, for any element the reader doesn't have a
dedicated mapping for, whether to raw-preserve it as a block-shaped `generic_div` or an
inline-shaped `generic_span`. Getting this wrong doesn't lose content (raw-preservation still
captures everything verbatim either way) but does produce the wrong wrapper shape on
round-trip, which is a real, if subtler, fidelity bug.

These classifiers were originally written by typical-usage judgment — "this element usually
reads like a block in practice" — not by checking each format's authoritative reference. That
gap was flagged explicitly rather than left implicit: docbook's own commit that introduced the
classifier called out that several fields (`author`/`date`/`copyright`/`pubdate`/`releaseinfo`
classified inline; `authorgroup`/`legalnotice`/`revhistory`/`revision` classified block) were
unverified guesses, and a follow-up task was dispatched specifically to check them against
DocBook's real content model rather than trust the guess.

## Decision

For each XML-based format vertical, `is_block_element` gets a dedicated verification pass:
every entry (not just the ones already flagged as suspect) is checked against that format's own
authoritative reference — the live schema/Tag Library/Guidelines pages fetched directly (e.g.
docbook.org for DocBook, jats.nlm.nih.gov's JATS 1.3 Tag Library, TEI P5 Guidelines) — using
each element's actual expanded content model and "may be contained in" list as ground truth,
not memory or typical-usage inference. Corrections and additions are recorded with an explicit
citation trail (which page, what the content model said) in the doc comment above the
classifier, not just in the commit message.

Where the format's own reference declines to commit either way — e.g. JATS's Tag Library states
`<alternatives>` "is neither inherently block nor inherently inline in nature... determined by
context and usage" — the classifier leaves that element unclassified (defaulting to inline)
rather than guessing a side the spec itself won't take.

## Consequences

- Applied uniformly across all three XML verticals in the same session arc: docbook found
  three misclassifications; tei found zero misclassifications but three missing block elements;
  jats found one misclassification (`related-article`, wrongly block) and four missing block
  elements (`speech`/`speaker`/`supplementary-material`/`block-alternatives`).
- The pattern is reusable for any future format vertical with a similar catch-all classifier —
  the methodology (fetch the live reference, check every entry's content model and containment
  list, cite the source in a doc comment, leave genuinely undecidable cases unclassified rather
  than guessed) generalizes beyond XML formats to any format with an ambiguous-construct
  fallback path.
- Cost: requires network access to the format's live reference (or an explicit, disclosed
  fallback to documented knowledge with a stated confidence level if fetching isn't available)
  — this is a deliberate tradeoff of thoroughness over speed, consistent with CLAUDE.md's rule
  against guessing when a lookup is possible.

## Alternatives considered

- **Trust the original typical-usage classification and move on**: rejected — CLAUDE.md's
  disposition rules treat "something unexpected is a signal" and forbid guessing when
  verification is possible; an already-flagged unverified guess is exactly the kind of thing
  that must be checked, not carried forward.
