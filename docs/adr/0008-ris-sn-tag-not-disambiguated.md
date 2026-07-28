# 8. RIS `SN` tag stays `field:scheme = "sn"`, not resolved to `isbn`/`issn`

## Status

Accepted (commit `dbf287d9`, 2026-07-28).

## Context

RIS's `SN` tag holds an ISBN, an ISSN, or a report/document/patent number, with no way to
tell which from the tag alone. `rescribe-read-ris` represents it as a `bibliography_field`
with `field:role = "identifier"` and `field:scheme = "sn"` — naming the scheme after the raw
RIS tag rather than resolving it.

RIS entries also carry a `TY` tag (reference type: `JOUR`, `BOOK`, `CHAP`, `CONF`, `THES`,
...), which rescribe preserves as the `ris:type` property on the `bibliography_entry` node.
Since `TY` is genuinely carried data — not a guess from string shape — a `TY`-based
disambiguation (e.g. `BOOK`/`CHAP` → `isbn`, `JOUR` → `issn`) was considered as a candidate
that would *not* violate CLAUDE.md's "never infer type from string content when the IR
already carries the type" rule.

Before implementing it, the available RIS documentation was checked for an authoritative
`TY`→`SN` rule:

- Wikipedia's "RIS (file format)" tag table — the closest thing to a canonical current RIS
  reference, since Clarivate/EndNote does not publish a maintained standalone RIS spec —
  defines `SN` uniformly as **"ISSN, ISBN, or report/document/patent number"** for every entry
  type, with no per-`TY` breakdown anywhere in the article.
- The `gris` Python RIS library's spec docs likewise define `SN` only as `"ISBN/ISSN"`, with
  no type dependency.
- No current, reachable EndNote/Clarivate official documentation stating a `TY`-based `SN`
  rule was found (the commonly-cited EndNote PDF URL 404s; Clarivate does not appear to
  maintain a current authoritative RIS spec page at all).
- Two real tools *do* apply a `TY`-based heuristic internally, and they disagree with each
  other: Zotero's RIS translator (`RIS.js`) defaults `SN` to ISBN and overrides to ISSN only
  for `journalArticle`/`magazineArticle`/`newspaperArticle` (plus patent/report special
  cases); refbase maps `SN`→ISBN for `BOOK`/`CHAP`/`STD`/`THES` and ISSN otherwise. Both
  authors describe their own mapping as a heuristic/workaround, not a citation of a spec
  (refbase's author: "some kind of content-sniffing mechanism would be even better").

## Decision

Keep `field:scheme = "sn"` (named after the raw RIS tag) rather than resolving it to
`"isbn"` or `"issn"` via `TY`. No authoritative RIS specification defines a `TY`→`SN`
disambiguation — the splits that exist are tool-specific implementation conventions, and the
two found tools don't even agree with each other. Implementing a `TY`-based mapping would
mean adopting one specific tool's convention (most plausibly Zotero's, as the more widely
deployed reference manager) and presenting it as RIS's own semantics — inventing a
convention and recording it as settled fact, which CLAUDE.md prohibits. The ambiguity is
inherent to the RIS format, not an oversight in rescribe's reader.

## Consequences

- Downstream consumers of the IR see `field:scheme = "sn"` rather than a resolved
  `"isbn"`/`"issn"` scheme, and must apply their own heuristic if they need one to be
  resolved further. The data needed to do so (`field:scheme = "sn"` plus `ris:type` on the
  entry) is preserved losslessly, so a consumer can reproduce Zotero's, refbase's, or any
  other heuristic without rescribe having silently picked one for them.
- rescribe stays honest about what the source format actually said, consistent with the
  "never infer type from string content when the IR already carries the type" principle —
  here extended to: never invent a disambiguation the format itself doesn't define, even
  when the disambiguating data (`TY`) is genuinely present.
- **Reopening condition**: if an authoritative RIS specification source is later found that
  *does* define per-`TY` `SN` semantics (e.g. a rediscovered current EndNote/Clarivate
  document, superseding the 404'd PDF), this decision should be revisited against that
  evidence.

## Alternatives considered

- **`TY`-based disambiguation using Zotero's mapping** (default ISBN, override to ISSN for
  journal/magazine/newspaper article types): rejected — Zotero's own translator describes
  this as a heuristic, not a spec rule, and refbase's competing heuristic disagrees with it
  on `BOOK`/`CHAP`. Adopting either would present one tool's convention as RIS semantics.
- **`TY`-based disambiguation using refbase's mapping** (ISBN for `BOOK`/`CHAP`/`STD`/`THES`,
  ISSN otherwise): rejected for the same reason — refbase's own author flags it as a
  workaround, and it disagrees with Zotero's mapping.
- **Leave `SN` unrepresented as an identifier field (raw-preserve only)**: rejected —
  `field:scheme = "sn"` already carries the value losslessly as a real identifier field, so
  discarding the `identifier` role would be a regression, not a neutral choice; the open
  question was only ever about the *scheme* value, not whether to model the field at all.
