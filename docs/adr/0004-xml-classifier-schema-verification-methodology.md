# 4. Schema-verification methodology for block/inline element classifiers

## Status

Accepted, methodology corrected (2026-07-28) after the originally-accepted version proved
insufficient. Original passes: docbook: commit `abd6dd447d`, 2026-07-27; tei: commit
`3e3d84bcef`; jats: commit `20c27d032e`, 2026-07-27. Corrected-methodology re-run: docbook
only, commit `be578fb98c`, 2026-07-28 (found 17 additional misclassifications beyond the
original pass's 4). **JATS and TEI have not yet been re-run with the corrected methodology
— see `TODO.md`'s "Status reset" entry for the pending work.**

## Amendment (2026-07-28): the originally-accepted method was structurally insufficient

The methodology this ADR originally recorded (see "Decision" below, describing the
2026-07-27 passes) checked every element **already present** in `is_block_element` against
each format's reference to see if it was classified correctly. It did not check whether the
format defines block elements **absent from the list entirely**. This is a real methodology
gap, not a one-off oversight: a presence-checking pass over an incomplete list can find
"have but shouldn't" but cannot structurally find "should have but don't," no matter how
carefully each listed entry is re-verified.

This was discovered when a follow-up DocBook pass (commit `be578fb98c`) extracted DocBook
5.2's full ~392-element index and diffed it against every element `is_block_element` and
`convert_element`'s match arms *actually handle*, rather than re-checking only what was
already listed. That pass found 17 additional genuine block-shaped elements missing from
the list, on top of the 4 the original 2026-07-27 pass had named — using the *same*
element-family knowledge, the only change was checking for absence, not just for wrong
presence.

**The JATS (`20c27d032e`) and TEI (`3e3d84bcef`) passes used the original, insufficient
method** (per this ADR's own "Decision" text as originally written: "every entry... is
checked against that format's own authoritative reference" — entry-checking, not
gap-finding). Their "zero misclassifications" (TEI) and "one misclassification, four
missing elements found incidentally" (JATS) results are therefore not reliable indicators
of completeness — the four missing elements JATS did find were presumably found by the same
kind of incidental noticing that inflated the DocBook COVERAGE.md denominator (see
`docs/format-audit.md`'s Construct Coverage section), not by a systematic absence-check.

**Corrected methodology, going forward:** for each format, extract the *full* element index
from the format's own authoritative reference (not just the elements already in
`is_block_element`), and diff it against every element the reader already handles (both
`is_block_element` and any dedicated match arms elsewhere in `convert_element` or
equivalent) to find candidates absent from either. Triage each candidate against the
format's actual content model before adding it — most misses are legitimately phrase-level
or handled by a separate dedicated path (e.g. DocBook's bibliographic citation fields via
`convert_biblio_field`) and should not be added just for appearing on the diff. Only after
this absence-check is complete does re-verifying entries *already listed* (the original
method) add further value, as a second pass, not a substitute for the first.

This does not change the original ADR's decision to require the checks in the first place,
or its record of *which* misclassifications the original 2026-07-27 passes correctly caught
— it corrects what "verified" is allowed to mean going forward. An ADR that documents a
flawed method without this amendment is worse than no ADR, because it would read as
license to repeat the same insufficient check.

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

**Superseded by the 2026-07-28 amendment above — retained for the historical record of what
was originally decided, not as current guidance.** For each XML-based format vertical,
`is_block_element` gets a dedicated verification pass: every entry (not just the ones already
flagged as suspect) is checked against that format's own authoritative reference — the live
schema/Tag Library/Guidelines pages fetched directly (e.g. docbook.org for DocBook,
jats.nlm.nih.gov's JATS 1.3 Tag Library, TEI P5 Guidelines) — using each element's actual
expanded content model and "may be contained in" list as ground truth, not memory or
typical-usage inference. Corrections and additions are recorded with an explicit citation
trail (which page, what the content model said) in the doc comment above the classifier, not
just in the commit message.

This entry-checking step is still valid as a *second* pass — see the amendment's "corrected
methodology" for what must happen first.

Where the format's own reference declines to commit either way — e.g. JATS's Tag Library states
`<alternatives>` "is neither inherently block nor inherently inline in nature... determined by
context and usage" — the classifier leaves that element unclassified (defaulting to inline)
rather than guessing a side the spec itself won't take.

## Consequences

- Applied uniformly across all three XML verticals in the same session arc: docbook found
  three misclassifications; tei found zero misclassifications but three missing block elements;
  jats found one misclassification (`related-article`, wrongly block) and four missing block
  elements (`speech`/`speaker`/`supplementary-material`/`block-alternatives`). **Per the
  2026-07-28 amendment, these tei/jats numbers are not reliable completeness results** — they
  used the entry-checking-only method, and DocBook's own corrected re-run found 17 additional
  misclassifications the entry-checking pass had missed. Re-verification of tei/jats with the
  corrected methodology is pending (`TODO.md`).
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
