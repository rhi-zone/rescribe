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
- [ ] code block (`<code>`) — (missing; JATS `<code>` element distinct from `<preformat>`)
- [ ] verse-group — (missing; `<verse-group>` / `<verse-line>`)
- [ ] speech — (missing; `<speech>` with `<speaker>` and `<p>`)
- [ ] statement (theorem, proof, etc.) — (missing; `<statement>`)
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
- [ ] monospace — (missing; `<monospace>`)
- [ ] overline — (missing; `<overline>`)
- [ ] roman — (missing; `<roman>`)
- [ ] sans-serif — (missing; `<sans-serif>`)
- [ ] code (inline) — (missing; `<code>` as inline)
- [ ] named-content — (missing; `<named-content content-type="…">`)
- [ ] styled-content — (missing; `<styled-content style="…">`)
- [ ] xref (cross-reference) — (missing; `<xref ref-type="…" rid="…">`)
- [ ] internal link — (missing; `<xref ref-type="fig">`, `<xref ref-type="table">`, etc.)
- [ ] citation (inline xref to ref-list) — (missing; `<xref ref-type="bibr">`)
- [ ] footnote reference (xref to fn) — (missing; `<xref ref-type="fn">`)
- [ ] abbrev — (missing; `<abbrev>`)
- [ ] inline-supplementary-material — (missing)
- [ ] milestone-start / milestone-end — (missing)
- [ ] target (anchor) — (missing; `<target id="…">`)

## Metadata (front matter)

- [ ] article-meta / article title — (missing; `<article-meta>` / `<title-group>` / `<article-title>`)
- [ ] subtitle — (missing; `<subtitle>` in `<title-group>`)
- [ ] author / contrib — (missing; `<contrib contrib-type="author">` / `<name>`)
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
- [ ] section id (`id` attribute) — (missing)
- [ ] xml:lang — (missing; language attribute)
- [ ] figure id / label — (missing; `id` and `<label>` on `<fig>`)
- [ ] table caption — (missing; `<caption>` on `<table-wrap>`)
- [ ] table id / label — (missing)
- [ ] list continuation / start value — (missing; `continued-from` attribute)
- [ ] colgroup / colspec in table — (missing; column width/alignment attributes)
- [ ] table cell spanning — (missing; `colspan`, `rowspan`)
- [ ] underline style — (missing; `underline-style` attribute on `<underline>`)
- [ ] ext-link type — (missing; `ext-link-type` attribute variants beyond "uri")
- [ ] MathML math — (missing; `<math>` MathML content as alternative to `<tex-math>`)

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
- [ ] unknown element (extension) — (missing; element not in JATS spec)
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
