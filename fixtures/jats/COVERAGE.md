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
- [ ] nested section — (missing; `<sec>` inside `<sec>`, 2+ levels deep)
- [ ] abstract — (missing; `<abstract>`)
- [ ] structured abstract — (missing; `<abstract abstract-type="structured">` with `<sec>`)
- [x] code block (`<code>`) — `code-block-vs-preformat`
- [ ] verse-group — (missing; `<verse-group>` / `<verse-line>`)
- [ ] speech — (missing; `<speech>` with `<speaker>` and `<p>`)
- [x] statement (theorem, proof, etc.) — `adv-unknown-block-element` (unrecognized
  block-level element, raw-preserved as a tagged `div` rather than silently dropped)
- [ ] boxed-text — (missing; `<boxed-text>`)
- [ ] supplementary-material — (missing; `<supplementary-material>`)
- [ ] caption as standalone block — (missing; `<caption>` outside `<fig>`)
- [ ] list with `list-type="alpha-lower"` / `"alpha-upper"` / `"roman-lower"` — (missing; list type variants)
- [ ] table-wrap-group — (missing; `<table-wrap-group>`)
- [ ] alternatives — (missing; `<alternatives>` container for math/graphic variants)

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
- [ ] inline-supplementary-material — (missing)
- [x] milestone-start / milestone-end — `milestone`
- [x] target (anchor) — `target-anchor`

## Metadata (front matter)

- [x] article-meta / article title — `header-contrib-group` (`<article-meta>` /
  `<title-group>` / `<article-title>` extracted into `title` metadata)
- [ ] subtitle — (missing; `<subtitle>` in `<title-group>`)
- [x] author / contrib — `header-contrib-group` (`<contrib-group>` has no dedicated
  semantic mapping; raw-preserved verbatim as `contrib-group_raw` metadata, alongside
  a flattened `contrib-group` name summary — exercises the general `<article-meta>`
  front-matter fallback, not a hardcoded special case)
- [ ] affiliation — (missing; `<aff>`)
- [ ] abstract — (missing; `<abstract>` in `<article-meta>`)
- [ ] keywords — (missing; `<kwd-group>` / `<kwd>`)
- [ ] journal-meta — (missing; `<journal-meta>` with `<journal-title>`, `<issn>`)
- [ ] pub-date — (missing; `<pub-date>` with `<year>`, `<month>`, `<day>`)
- [ ] volume / issue / fpage / lpage — (missing; article pagination metadata)
- [ ] doi / article-id — (missing; `<article-id pub-id-type="doi">`)
- [ ] permissions / license — (missing; `<permissions>` / `<license>`)
- [ ] funding-group — (missing; `<funding-group>` / `<funding-source>`)
- [ ] history (received/accepted dates) — (missing; `<history>` / `<date date-type="received">`)

## Back matter

- [ ] reference list — (missing; `<ref-list>` / `<ref>` / `<mixed-citation>` / `<element-citation>`)
- [ ] element-citation (structured ref) — (missing)
- [ ] mixed-citation (text ref) — (missing)
- [ ] appendix — (missing; `<app>` / `<app-group>`)
- [ ] glossary — (missing; `<glossary>` / `<def-list>`)
- [ ] acknowledgments — (missing; `<ack>`)
- [ ] fn-group (footnote group in back) — (missing; `<fn-group>`)
- [ ] notes (back notes) — (missing; `<notes>`)

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
- [ ] list continuation / start value — (missing; `continued-from` attribute)
- [x] colgroup / colspec in table — `table-colgroup`
- [x] table cell spanning — `table-cell-spanning`
- [ ] underline style — (missing; `underline-style` attribute on `<underline>`)
- [x] ext-link type — `ext-link-type`
- [ ] MathML math — (missing; `<math>` MathML content as alternative to `<tex-math>` —
  genuine design fork, see TODO.md)

## Composition (integration)

- [ ] nested sections (2 levels) — (missing)
- [ ] inline formatting inside list items — (missing)
- [ ] table with inline formatting in cells — (missing)
- [ ] figure with supplementary content — (missing)
- [ ] footnote in table cell — (missing)
- [ ] citation + reference list roundtrip — (missing)
- [ ] display formula with label — (missing; `<label>` on `<disp-formula>`)
- [ ] full article with front/body/back — (missing)

## Adversarial

- [x] empty document — `adv-empty`
- [ ] malformed XML (unclosed tag) — (missing)
- [x] unknown element (extension) — `adv-unknown-block-element` (block-shaped,
  raw-preserved as a tagged `div`), `adv-unknown-inline-element` (inline-shaped,
  raw-preserved as a tagged `span` in place) — neither is silently dropped
- [ ] missing xlink namespace — (missing)
- [ ] broken xref (rid pointing to nonexistent id) — (missing)
- [ ] entity references — (missing)
- [ ] numeric character references — (missing)
- [ ] empty paragraph — (missing; `<p/>`)

## Pathological

- [ ] very large table — (missing)
- [ ] deeply nested sections — (missing; 6+ levels)
- [ ] many references in ref-list — (missing)
- [ ] large number of footnotes — (missing)
