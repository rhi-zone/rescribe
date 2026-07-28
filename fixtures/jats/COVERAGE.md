# JATS Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

**Coverage-completeness caveat (2026-07-28):** the checklist below is a hand-curated list of
constructs, not yet verified against a spec-derived, machine-readable construct index. An
audit of `fixtures/docbook/COVERAGE.md` and `fixtures/jats/COVERAGE.md` against authoritative
element indexes found hundreds of element names enumerated nowhere, moving denominators
mid-session purely from incidentally-noticed gaps -- a ratio over a hand-written list like this
one is not a coverage measurement. See `docs/format-audit.md`'s "Construct Coverage (CC)"
section for the full evidence; this format's `CC` status there is `U` (unverified) until a
construct registry (in design, see `docs/adr/`) checks this list against the format's own
spec.

JATS reference: JATS 1.3 (NISO Z39.96-2019), Journal Archiving and Interchange Tag Set.
https://jats.nlm.nih.gov/archiving/tag-library/1.3/

Archiving (not Publishing or Authoring) is the deliberate reference tag set — it is the
element superset of the three journal-article tag sets (Publishing and Authoring are
validity-constrained subsets of the same vocabulary, not divergent element sets), so a
fixture suite and adapter element-mapping table built against Archiving already covers
Publishing/Authoring documents. `jats-fmt` itself parses any well-formed XML with no
DTD/schema validation, so this choice affects fixture/mapping scope only, not parser
behavior. See `docs/adr/0012-jats-archiving-tag-set-scope.md` for the full rationale,
including why BITS (book content) and the `ooxml-wml`/`ooxml-sml`/`ooxml-pml` precedent
do not apply here.

## Block constructs

- [x] paragraph — `paragraph` (`<p>`)
- [x] section with heading — `heading` (`<sec>` with `<title>`)
- [x] blockquote — `blockquote` (`<disp-quote>`)
- [x] code block — `code-block` (`<preformat>`)
- [x] ordered list — `list-ordered` (`<list list-type="order">`)
- [x] unordered list — `list-unordered` (`<list list-type="bullet">`)
- [x] definition list — `definition-list` (`<def-list>` / `<def-item>`)
- [x] table — `table` (`<table-wrap>` / `<table>`)
- [x] table with thead/tbody/tfoot — `table-sections`
- [x] figure — `figure` (`<fig>` with `<caption>` and `<graphic>`)
- [x] display math — `math-display` (`<disp-formula>` / `<tex-math>`)
- [x] footnote — `footnote-def` (`<fn>`)
- [x] nested section — `nested-section`
- [x] abstract — `abstract`
- [x] structured abstract — `structured-abstract`
- [x] code block (`<code>`) — `code-block-vs-preformat`
- [x] verse-group — `verse-group`
- [x] speech — `speech`
- [x] statement (theorem, proof, etc.) — `adv-unknown-block-element` (unrecognized
  block-level element, raw-preserved as a tagged `div` rather than silently dropped)
- [x] boxed-text — `boxed-text`
- [x] supplementary-material — `supplementary-material`
- [x] caption as standalone block — `caption-standalone`
- [x] list with `list-type="alpha-lower"` / `"alpha-upper"` / `"roman-lower"` — `list-type-alpha`
- [x] table-wrap-group — `table-wrap-group`
- [x] alternatives — `math-display-mathml-alternatives`, `math-inline-mathml-alternatives`
  (`<mml:math>`/`<tex-math>` wrapped in `<alternatives>` inside `<disp-formula>`/
  `<inline-formula>` — fixes a real corruption bug where the MathML and TeX text got
  silently concatenated into one `math:source` string; MathML is now raw-preserved and
  the sibling `<tex-math>` round-trips via `jats:alternatives-raw`), `figure-alternatives-
  graphics` (the general non-math case — `<alternatives>` anywhere else transparently
  passes through to its already-existing per-child conversion, keeping *every* alternative
  as a real structured node rather than picking one and raw-preserving the rest, since
  each alternative type here already has its own dedicated IR mapping; see TODO.md for
  why "pick richest, raw-preserve the rest" was considered and rejected for this case).
  No block-vs-inline classification is ever needed: `<alternatives>` itself never becomes
  an IR node — either it's elided entirely (math case) or its children convert normally
  in whatever shape they'd have had without the wrapper (general case).
- [ ] horizontal rule — (missing-and-unhandled; `<hr>` — rescribe-std already
  defines `horizontal_rule` (`crates/nodes/rescribe-std/src/lib.rs:39`) but the
  JATS reader has no arm for it and it is absent from `is_block_element`, so
  it falls through as an inline span rather than a block. Full-schema audit
  vs. jats.nlm.nih.gov's 1.3 archiving alpha-index, see TODO.md)
- [ ] sub-article / response — (missing-and-unhandled; `<sub-article>`/
  `<response>` — whole nested front/body/back substructures; absent from
  `is_block_element`, so they default to an inline `generic_span` wrapping a
  block subtree — the most structurally risky gap found in the audit, needs
  investigation before adding a fixture. Full-schema audit, see TODO.md)
- [ ] ruby annotation — (missing-and-unhandled; `<ruby>`/`<rb>`/`<rt>`/`<rp>` —
  no rescribe-std node kind exists for ruby text; raw preservation via the
  generic catch-all is the only path today. Full-schema audit, see TODO.md)
- [ ] question-and-answer set — (missing-and-unhandled; `<question-wrap-group>`
  / `<question-wrap>` / `<question>` / `<question-preamble>` / `<answer>` /
  `<answer-set>` / `<explanation>`. Full-schema audit, see TODO.md)
- [ ] chemical structure — (missing-and-unhandled; `<chem-struct>` /
  `<chem-struct-wrap>`. Full-schema audit, see TODO.md)
- [ ] array (untagged tabular data) — (missing-and-unhandled; `<array>`.
  Full-schema audit, see TODO.md)
- [ ] index term — (missing-and-unhandled; `<index-term>` /
  `<index-term-range-end>`. Full-schema audit, see TODO.md)
- [ ] media / accessibility description — (missing-and-unhandled; `<media>` /
  `<inline-media>` / `<long-desc>` / `<alt-text>` / `<textual-form>`.
  Full-schema audit, see TODO.md)
- [ ] appendix — (missing-but-handled; `<app-group>`/`<app>`, already
  `is_block_element`-classified and fixture-tested at `fixtures/jats/appendix`
  — bookkeeping gap only, not enumerated by element name. Full-schema audit,
  see TODO.md)
- [ ] signature block — (missing-but-handled; `<sig-block>`/`<sig>`,
  `is_block_element`-classified, no fixture. Full-schema audit, see TODO.md)
- [ ] custom-meta-group / product / kwd-group — (missing-but-handled;
  `is_block_element`-classified, no fixture. Full-schema audit, see TODO.md)

## Inline constructs

- [x] emphasis (italic) — `emphasis` (`<italic>`)
- [x] strong (bold) — `strong` (`<bold>`)
- [x] strikeout — `strikeout` (`<strike>`)
- [x] underline — `underline` (`<underline>`)
- [x] subscript — `subscript` (`<sub>`)
- [x] superscript — `superscript` (`<sup>`)
- [x] small caps — `small-caps` (`<sc>`)
- [x] link (external) — `link` (`<ext-link>`)
- [x] image (inline graphic) — `image` (`<graphic>`)
- [x] line break — `line-break` (`<break>`)
- [x] inline math — `math-inline` (`<inline-formula>` / `<tex-math>`)
- [x] monospace — `monospace`
- [x] overline — `overline`
- [x] roman — `roman`
- [x] sans-serif — `sans-serif`
- [x] code (inline) — `inline-code`
- [x] named-content — `named-content`
- [x] styled-content — `adv-unknown-inline-element` (unrecognized inline element,
  raw-preserved as a tagged `span` in place rather than silently dropped)
- [x] xref (cross-reference) — `xref-internal-link`
- [x] internal link — `xref-internal-link` (`<xref ref-type="fig">`; same mapping covers `table`, `sec`, etc.)
- [x] citation (inline xref to ref-list) — `xref-citation`
- [x] footnote reference (xref to fn) — `xref-footnote`
- [x] abbrev — `abbrev`
- [x] inline-supplementary-material — `inline-supplementary-material`
- [x] milestone-start / milestone-end — `milestone`
- [x] target (anchor) — `target-anchor`
- [ ] open-access license statements — (missing-and-unhandled;
  `<ali:free_to_read>` / `<ali:license_ref>`. Full-schema audit, see TODO.md)
- [ ] private character / glyph — (missing-and-unhandled; `<private-char>` /
  `<glyph-ref>` / `<glyph-data>` / `<fixed-case>`. Full-schema audit, see
  TODO.md)

## Metadata (front matter)

- [x] article-meta / article title — `header-contrib-group` (`<article-meta>` /
  `<title-group>` / `<article-title>` extracted into `title` metadata)
- [x] subtitle — `subtitle`
- [x] author / contrib — `header-contrib-group` (`<contrib-group>` has no dedicated
  semantic mapping; raw-preserved verbatim as `contrib-group_raw` metadata, alongside
  a flattened `contrib-group` name summary — exercises the general `<article-meta>`
  front-matter fallback, not a hardcoded special case)
- [x] affiliation — `affiliation`
- [x] abstract — `abstract`
- [x] keywords — `keywords`
- [x] journal-meta — `journal-meta`
- [x] pub-date — `pub-date`
- [x] volume / issue / fpage / lpage — `pagination`
- [x] doi / article-id — `article-id-doi`
- [x] permissions / license — `permissions-license`
- [x] funding-group — `funding-group`
- [x] history (received/accepted dates) — `history`
- [ ] contrib name subfields — (missing-but-handled; one level inside
  `<name>`/`<contrib>` via the generic raw-preserve fallback: `<given-names>`/
  `<surname>`/`<prefix>`/`<suffix>`/`<degrees>`/`<role>`/`<email>`/`<phone>`/
  `<fax>`/`<bio>`/`<author-notes>`/`<author-comment>`/`<corresp>`/
  `<on-behalf-of>`/`<etal>`/`<anonymous>`/`<aff-alternatives>`/`<contrib-id>` —
  no dedicated fixture. Full-schema audit, see TODO.md)
- [ ] journal identification — (missing-but-handled; `<journal-id>`/
  `<journal-title-group>`/`<journal-title>`/`<journal-subtitle>`/
  `<abbrev-journal-title>`/`<issn>`/`<issn-l>`/`<isbn>`/`<publisher>` —
  raw-preserved via the `<journal-meta>` fallback, no fixture. Full-schema
  audit, see TODO.md)
- [ ] funding/awards detail — (missing-but-handled; `<funding-source>`/
  `<funding-statement>`/`<award-group>`/`<award-id>`/`<award-name>`/
  `<award-desc>`/`<principal-award-recipient>`/`<principal-investigator>` —
  raw-preserved via the `<article-meta>` fallback beyond the already-covered
  `funding-group`, no fixture. Full-schema audit, see TODO.md)
- [ ] conference metadata — (missing-but-handled; `<conference>`/
  `<conf-name>`/`<conf-acronym>`/`<conf-loc>`/`<conf-date>`/`<conf-num>`/
  `<conf-sponsor>`/`<conf-theme>`. Full-schema audit, see TODO.md)
- [ ] keyword structure — (missing-but-handled; `<compound-kwd>`/
  `<compound-kwd-part>`/`<nested-kwd>`/`<unstructured-kwd-group>` — `<kwd>`/
  `<kwd-group>` already covered by `keywords`. Full-schema audit, see
  TODO.md)
- [ ] article/journal counts — (missing-but-handled; `<word-count>`/
  `<fig-count>`/`<table-count>`/`<equation-count>`/`<ref-count>`/
  `<page-count>`, wrapped in `<counts>`. Full-schema audit, see TODO.md)
- [ ] article categorization / related links — (missing-but-handled;
  `<article-categories>`/`<subj-group>`/`<subject>`/`<compound-subject>`/
  `<related-article>`/`<related-object>`/`<self-uri>`/`<product>`/
  `<supplement>`. Full-schema audit, see TODO.md)
- [ ] copyright detail — (missing-but-handled; `<copyright-statement>`/
  `<copyright-year>`/`<copyright-holder>`/`<license-p>` — under the
  already-covered `permissions-license`. Full-schema audit, see TODO.md)
- [ ] custom metadata fields — (missing-but-handled; `<meta-name>`/
  `<meta-value>` inside the already-covered `<custom-meta>`/
  `<custom-meta-group>`. Full-schema audit, see TODO.md)
- [ ] volume/issue detail — (missing-but-handled; `<volume-id>`/
  `<volume-series>`/`<volume-issue-group>`/`<issue-id>`/`<issue-part>`/
  `<issue-sponsor>`/`<issue-subtitle>`/`<issue-title>`/`<issue-title-group>`.
  Full-schema audit, see TODO.md)

## Back matter

- [x] reference list — `<ref-list>` -> `bibliography`, `<ref>` ->
  `bibliography_entry`, using the dedicated citation/bibliography IR shape
  added in `4e15c996` (schema-verified against DocBook 5.2/JATS/TEI/OOXML) —
  `citation-simple-author`
- [x] element-citation (structured ref) — `citation-simple-author`,
  `citation-multi-author`, `citation-date`
- [x] mixed-citation (text ref) — `citation-mixed-citation`
- [x] appendix — `appendix`
- [x] glossary — `glossary`
- [x] acknowledgments — `acknowledgments`
- [x] fn-group (footnote group in back) — `fn-group-back`
- [x] notes (back notes) — `back-notes`

## Properties

- [x] list type (ordered vs unordered) — `list-ordered`, `list-unordered`
- [x] figure caption — `figure`
- [x] table header cells — `table-sections`
- [x] section id (`id` attribute) — `section-id`
- [x] xml:lang — `xml-lang`
- [x] figure id / label — `figure-id-label`
- [x] table caption — `table-caption`
- [x] table id / label — `table-id` (id on `<table-wrap>` covered by
  `table-caption`, id on the inner `<table>` by `table-id`)
- [x] list continuation / start value — `list-continued-from` (idref raw-preserved
  via `jats:continued-from`; not resolved to an actual numeric start value, which
  would need a second document pass this single-pass conversion doesn't do)
- [x] colgroup / colspec in table — `table-colgroup`
- [x] table cell spanning — `table-cell-spanning`
- [x] underline style — `underline-style`
- [x] ext-link type — `ext-link-type`
- [x] MathML math — `math-display-mathml`, `math-inline-mathml` (`<mml:math>` inside
  `<disp-formula>`/`<inline-formula>`, as an alternative to `<tex-math>` per the JATS
  1.3 Tag Library's content model — raw-preserved verbatim as `math:source` with
  `math:format="mathml"`, following the precedent already established for HTML's
  `<math>` handling; previously mis-classified as a genuine design fork — see TODO.md's
  "MathML resolved" entry)
- [ ] table sub-elements — (missing-but-handled; `<col>`/`<colgroup>` covered
  by `table-colgroup`; `<tr>`/`<td>`/`<th>`/`<thead>`/`<tbody>`/`<tfoot>`
  covered by `table-sections`; `<table-wrap-foot>` is `is_block_element`-
  classified but has no fixture. Full-schema audit, see TODO.md)

## Composition (integration)

- [x] nested sections (2 levels) — `nested-section` (Block constructs dimension)
- [x] inline formatting inside list items — `list-item-inline-formatting`
- [x] table with inline formatting in cells — `table-cell-inline-formatting`
- [x] figure with supplementary content — `figure-with-supplementary-material`
- [x] footnote in table cell — `footnote-in-table-cell`
- [x] citation + reference list roundtrip — `citation-markup-in-field`
  (`<italic>`/`<bold>` nested inside an `article-title`/`collab` field,
  proving `bibliography_field`'s children are ordinary markup-capable inline
  nodes, not a flat string)
- [x] display formula with label — `disp-formula-with-label`
- [x] full article with front/body/back — `e2e-full-article`

## Adversarial

- [x] empty document — `adv-empty`
- [x] malformed XML (unclosed tag) — `adv-malformed-xml`
- [x] unknown element (extension) — `adv-unknown-block-element` (block-shaped,
  raw-preserved as a tagged `div`), `adv-unknown-inline-element` (inline-shaped,
  raw-preserved as a tagged `span` in place) — neither is silently dropped
- [x] missing xlink namespace — `adv-missing-xlink-ns`
- [x] broken xref (rid pointing to nonexistent id) — `adv-broken-xref`
- [x] entity references — `adv-entity-references`
- [x] numeric character references — `adv-numeric-char-ref`
- [x] entity declared in the document's own DOCTYPE internal subset,
  resolved via the `xml-entities` crate — `dtd-entity-resolution`
- [x] named entity resolved via the standard WHATWG/ISO table with no
  DOCTYPE present — `rare-named-entity-standard-table`
- [x] named entity unresolvable by either layer, still raw-preserved as
  `raw_inline` — `adv-unresolvable-entity`
- [x] empty paragraph — `adv-empty-paragraph`

## Pathological

- [x] very large table — `path-large-table` (200 rows x 10 columns)
- [x] deeply nested sections — `path-deeply-nested-sections` (10 levels)
- [x] many references in ref-list — `path-many-references` (60 references)
- [x] large number of footnotes — `path-many-footnotes` (100 footnotes)
