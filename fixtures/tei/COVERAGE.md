# TEI Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

TEI reference: TEI P5 Guidelines (https://tei-c.org/release/doc/tei-p5-doc/en/html/).
TEI All tagset (tei_all). Primary module focus: tei, core, textstructure, linking.

## Block constructs

- [x] paragraph — `paragraph` (`<p>`)
- [x] heading (div1) — `heading` (`<div1>` with `<head>`)
- [x] heading (div2) — `heading-h2` (`<div2>` with `<head>`)
- [x] blockquote — `blockquote` (`<quote>`)
- [x] code block — `code-block` (`<eg>`)
- [x] ordered list — `list-ordered` (`<list rend="numbered">`)
- [x] unordered list — `list-unordered` (`<list>`)
- [x] definition list — `definition-list` (`<gloss>` with `<term>` and `<def>`)
- [x] table — `table` (`<table>` with `<row>` / `<cell>`)
- [x] table header row — `table-header` (`<cell rend="header">`)
- [x] figure — `figure` (`<figure>` with `<figDesc>` and `<graphic>`)
- [x] footnote — `footnote-def` (`<note>`)
- [x] horizontal rule / page break — `horizontal-rule` (`<pb/>`)
- [x] verse / line group — `verse` (`<lg>` with `<l>`)
- [x] display math — `math-display` (`<formula>`)
- [x] div (unnumbered) — `div-unnumbered` (`<div>` with `<head>`)
- [x] div3 / div4 / div5 / div6 — `div-deep-levels`
- [x] nested div structure — `div-nested`
- [x] sp / said (speech) — `speech` (`<sp>` with `<speaker>` and `<p>`)
- [x] stage direction — `stage-direction`
- [x] epigraph — `epigraph`
- [x] argument — `argument`
- [x] byline — `byline`
- [x] dateline / salute / signed — `letter-elements`
- [x] trailer — `trailer`
- [x] castList — `cast-list`
- [x] cit (quotation with attribution) — `cit`
- [x] ab (anonymous block) — `ab`
- [x] gap / space — `gap-space`
- [x] list with `type` attribute variants — `list-type-variants`
- [x] nested list — `list-nested`
- [x] item with label — `list-item-label`

## Inline constructs

- [x] emphasis (italic) — `emphasis` (`<hi rend="italic">`)
- [x] strong (bold) — `strong` (`<hi rend="bold">`)
- [x] strikeout — `strikeout` (`<hi rend="strike">`)
- [x] underline — `underline` (`<hi rend="underline">`)
- [x] subscript — `subscript` (`<hi rend="sub">`)
- [x] superscript — `superscript` (`<hi rend="sup">`)
- [x] small caps — `small-caps` (`<hi rend="sc">`)
- [x] link — `link` (`<ref target="…">`)
- [x] image (inline graphic) — `image` (`<graphic url="…">`)
- [x] line break — `line-break` (`<lb/>`)
- [x] inline code — `inline-code` (`<code>`)
- [x] foreign language phrase — `foreign` (`<foreign xml:lang="…">`)
- [x] term — `term-inline`
- [x] gloss (inline) — `gloss-inline`
- [x] abbr — `abbr`
- [x] expan (expansion of abbreviation) — `expan`
- [x] choice (abbr/expan pair) — `choice-abbr-expan`
- [x] orig / reg (normalization pair) — `choice-orig-reg`
- [x] sic / corr (correction pair) — `choice-sic-corr`
- [x] add (addition) — `add`
- [x] del (deletion) — `del`
- [x] supplied — `supplied`
- [x] unclear — `unclear`
- [x] persName — `pers-name`
- [x] placeName — `place-name`
- [x] orgName — `org-name`
- [x] date (inline) — `date-inline`
- [x] title (inline) — `title-inline`
- [x] name (generic) — `name-generic`
- [x] num — `num`
- [x] measure — `measure`
- [x] xref / ptr — `ptr`
- [x] anchor — `anchor`
- [x] milestone — `milestone`
- [x] seg — `seg`
- [x] w / pc (token / punctuation) — `word-token`
- [x] inline math — `math-inline` (`<formula type="inline">`)
- [x] note (marginal / endnote) — `note-place`

## TEI Header (metadata)

- [x] teiHeader / fileDesc — `header-file-desc`
- [x] titleStmt — `header-title-stmt`
- [x] publicationStmt — `header-publication-stmt`
- [x] sourceDesc — `header-source-desc`
- [x] profileDesc / langUsage — `header-lang-usage`
- [x] encodingDesc — `header-encoding-desc` (raw-preserved verbatim via
  `encodingDesc_raw` metadata, alongside a flattened `encodingDesc` summary)
- [x] revisionDesc / change — `header-revision-desc`
- [x] abstract (in profileDesc) — `header-abstract`
- [x] keywords (in profileDesc) — `header-keywords`
- [x] msDesc (manuscript description) — `header-ms-desc` (raw-preserved
  verbatim via `msDesc_raw` metadata, alongside a flattened `msDesc` summary)
- [x] any other teiHeader element with no dedicated semantic mapping —
  `header-partic-desc` (`<profileDesc><particDesc>`) exercises the general
  raw-preservation fallback (`{tag}_raw` metadata), not just the two
  historically hardcoded `msDesc`/`encodingDesc` names

## Properties

- [x] heading level (div1 vs div2) — `heading`, `heading-h2`
- [x] list ordered/unordered via `rend` — `list-ordered`, `list-unordered`
- [x] figure description — `figure` (`<figDesc>`)
- [x] table row role ("label") — `table` (`<row role="label">`)
- [x] table cell rend ("header") — `table-header`
- [x] xml:id on div — `prop-xml-id-div`
- [x] xml:lang on element — `prop-xml-lang`
- [x] rend values beyond covered set — `prop-rend-align` (`rend="center"`, `rend="right"`)
- [x] type attribute on note — `prop-note-type`
- [x] n attribute (numbering) — `prop-n-attribute`
- [x] corresp / sameAs (linking attributes) — `prop-corresp-sameas`
- [x] graphic dimensions (width/height) — `prop-graphic-dims`
- [x] table cols/rows — `prop-table-cols-rows`
- [x] list item label — `list-item-label`

## Composition (integration)

- [x] nested divs (3 levels) — `int-nested-divs`
- [x] inline formatting inside list items — `int-list-inline`
- [x] table with inline formatting in cells — `int-table-inline`
- [x] verse with speaker — `int-verse-speaker` (`<sp>` containing `<lg>`)
- [x] footnote with formatted content — `int-footnote-formatted` (`<note>` containing `<hi>`)
- [x] cit with bibl attribution — `int-cit-bibl`
- [x] choice (sic/corr) inside paragraph — `int-choice-in-paragraph`
- [x] full document with teiHeader + text — `e2e-full-document`
- [x] front matter + body + back matter — `e2e-front-body-back` (`<front>`, `<body>`, `<back>`)

## Adversarial

- [x] empty document — `adv-empty`
- [x] malformed XML (unclosed tag) — `adv-malformed-xml`
- [x] missing TEI namespace — `adv-no-namespace`
- [x] unknown element — `adv-unknown-element` (non-block-vocabulary element at
  block-dispatch position, stays a bare span), `adv-unknown-block-element`
  (block-vocabulary element, raw-preserved as a tagged `div` — no `<p>`-wrap
  round-trip drift), `adv-unknown-inline-element` (unrecognized element nested
  inline within running text, stays a tagged span in place)
- [x] entity references — `adv-entity-references`
- [x] numeric character references — `adv-numeric-char-ref`
- [x] entity declared in the document's own DOCTYPE internal subset,
  resolved via the `xml-entities` crate — `dtd-entity-resolution`
- [x] named entity resolved via the standard WHATWG/ISO table with no
  DOCTYPE present — `rare-named-entity-standard-table`
- [x] named entity unresolvable by either layer, still raw-preserved as
  `raw_inline` — `adv-unresolvable-entity`
- [x] empty paragraph — `adv-empty-paragraph` (`<p/>`)
- [x] note with no content — `adv-empty-note` (`<note/>`)
- [x] broken target reference — `adv-broken-ref` (`<ref target="#nonexistent">`)

## Pathological

- [x] deeply nested divs (6+ levels) — `path-deep-nested-divs` (8 levels)
- [x] very large table — `path-large-table` (31 rows × 4 cols)
- [x] long poem with many lines — `path-long-poem` (50 lines)
- [x] document with many footnotes — `path-many-footnotes` (20 footnotes)
- [x] teiHeader with all optional metadata — `path-full-header`
