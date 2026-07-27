# JATS Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

JATS reference: JATS 1.3 (NISO Z39.96-2019), Journal Archiving and Interchange Tag Set.
https://jats.nlm.nih.gov/archiving/tag-library/1.3/

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
- [ ] alternatives — (missing; `<alternatives>` container for math/graphic variants —
  genuine design fork, see TODO.md: JATS's own Tag Library page says it "is neither
  inherently block nor inherently inline")

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

## Back matter

- [ ] reference list — (missing; `<ref-list>` / `<ref>` / `<mixed-citation>` /
  `<element-citation>` — genuine design fork, see TODO.md: whether a dedicated
  citation/bibliography IR shape should exist, or whether the current thin
  ref-list -> div / ref -> paragraph / \*-citation -> span mapping is the
  intended final answer, is genuinely undecided)
- [ ] element-citation (structured ref) — (missing; same fork)
- [ ] mixed-citation (text ref) — (missing; same fork)
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
- [ ] MathML math — (missing; `<math>` MathML content as alternative to `<tex-math>` —
  genuine design fork, see TODO.md)

## Composition (integration)

- [x] nested sections (2 levels) — `nested-section` (Block constructs dimension)
- [x] inline formatting inside list items — `list-item-inline-formatting`
- [x] table with inline formatting in cells — `table-cell-inline-formatting`
- [x] figure with supplementary content — `figure-with-supplementary-material`
- [x] footnote in table cell — `footnote-in-table-cell`
- [ ] citation + reference list roundtrip — (missing; tied to the citation/
  reference-list IR-shape design fork, see TODO.md)
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
- [x] empty paragraph — `adv-empty-paragraph`

## Pathological

- [x] very large table — `path-large-table` (200 rows x 10 columns)
- [x] deeply nested sections — `path-deeply-nested-sections` (10 levels)
- [ ] many references in ref-list — (missing; tied to the citation/
  reference-list IR-shape design fork, see TODO.md)
- [x] large number of footnotes — `path-many-footnotes` (100 footnotes)
