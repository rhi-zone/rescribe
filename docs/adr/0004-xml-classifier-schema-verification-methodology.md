# 4. Schema-verification methodology for block/inline element classifiers

## Status

Accepted.

## Context

`rescribe-read-docbook`, `rescribe-read-jats`, and `rescribe-read-tei` each carry an
`is_block_element(tag)` classifier that decides, for any element the reader doesn't have a
dedicated mapping for, whether to raw-preserve it as a block-shaped `generic_div` or an
inline-shaped `generic_span`. Getting this wrong doesn't lose content (raw-preservation still
captures everything verbatim either way) but does produce the wrong wrapper shape on
round-trip, which is a real, if subtler, fidelity bug.

These classifiers were originally written by typical-usage judgment — "this element usually
reads like a block in practice" — not by checking each format's authoritative reference.
DocBook's own commit that introduced the classifier flagged this gap explicitly: several
fields (`author`/`date`/`copyright`/`pubdate`/`releaseinfo` classified inline;
`authorgroup`/`legalnotice`/`revhistory`/`revision` classified block) were unverified guesses.

A first verification attempt checked every element **already present** in `is_block_element`
against each format's reference to see if it was classified correctly, but did not check
whether the format defines block elements **absent from the list entirely**. That is a
structural gap, not a matter of care: a presence-checking pass over an incomplete list can
find "have but shouldn't" but cannot find "should have but don't," no matter how carefully
each listed entry is re-verified. Running that check against DocBook found 4 misclassified
entries; separately extracting DocBook 5.2's full ~392-element index and diffing it against
every element `is_block_element` and `convert_element` actually handle — rather than only
re-checking what was already listed — found 17 *additional* genuine block-shaped elements
missing from the list, using the same element-family knowledge and no new information other
than checking for absence instead of just wrong presence.

## Decision

For each XML-based format vertical, `is_block_element` gets a two-part verification pass:

1. **Absence check (must run first).** Extract the format's *full* element index from its own
   authoritative reference (docbook.org for DocBook, jats.nlm.nih.gov's JATS 1.3 Tag Library,
   TEI P5 Guidelines) — not just the elements already in `is_block_element` — and diff it
   against every element the reader already handles (`is_block_element` and any dedicated
   match arms elsewhere in `convert_element` or equivalent) to surface candidates absent from
   both. Triage each candidate against the format's actual content model before adding it:
   most misses are legitimately phrase-level or handled by a separate dedicated path (e.g.
   DocBook's bibliographic citation fields via `convert_biblio_field`) and should not be added
   just for appearing on the diff.
2. **Entry check (second pass, not a substitute for the first).** Every entry already in
   `is_block_element` is checked against the format's own authoritative reference, using each
   element's actual expanded content model and "may be contained in" list as ground truth, not
   memory or typical-usage inference. Corrections are recorded with an explicit citation trail
   (which page, what the content model said) in the doc comment above the classifier.

Where the format's own reference declines to commit either way — e.g. JATS's Tag Library
states `<alternatives>` "is neither inherently block nor inherently inline in nature...
determined by context and usage" — the classifier leaves that element unclassified
(defaulting to inline) rather than guessing a side the spec itself won't take.

Doing only the entry check (skipping the absence check) is not an acceptable partial
application of this methodology: it structurally cannot find missing elements regardless of
how carefully it's performed, so a report of "N misclassifications, zero missing" produced by
an entry-check-only pass is not a completeness claim and must not be read as one.

## Consequences

- DocBook has run the full two-part methodology (commits `abd6dd447d`, `be578fb98c`): 4
  misclassifications from the entry check, 17 additional missing block elements from the
  absence check.
- JATS (commit `20c27d032e`) and TEI (commit `3e3d84bcef`) have so far only run an
  entry-check-only pass: JATS found one misclassification (`related-article`, wrongly block)
  and four missing block elements found incidentally rather than by systematic absence-check;
  TEI found zero misclassifications and three missing block elements, also incidental. Neither
  result is a completeness claim under this ADR — both need the absence check run against
  their full element indexes before `is_block_element` can be called verified for those
  formats. This is open, tracked work; see `TODO.md`.
- The pattern is reusable for any future format vertical with a similar catch-all
  classifier — the methodology (extract the full reference index, diff for absence first,
  then re-check entries already listed, cite the source in a doc comment, leave genuinely
  undecidable cases unclassified rather than guessed) generalizes beyond XML formats to any
  format with an ambiguous-construct fallback path.
- Cost: requires network access to the format's live reference (or an explicit, disclosed
  fallback to documented knowledge with a stated confidence level if fetching isn't available)
  — a deliberate tradeoff of thoroughness over speed, consistent with CLAUDE.md's rule against
  guessing when a lookup is possible.

## Alternatives considered

- **Trust the original typical-usage classification and move on**: rejected — CLAUDE.md's
  disposition rules treat "something unexpected is a signal" and forbid guessing when
  verification is possible; an already-flagged unverified guess is exactly the kind of thing
  that must be checked, not carried forward.
- **Entry-check only (verify every already-listed element against the reference, skip
  extracting the full index)**: rejected — this was the first approach tried, and it is
  structurally incapable of finding elements missing from the list entirely, regardless of
  how carefully each listed entry is re-verified. DocBook's own corrected re-run found 17
  additional misclassifications this method could not have found no matter how many times it
  was repeated.
