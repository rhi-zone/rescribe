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
- [ ] div (unnumbered) — (missing; `<div>` with `<head>`, the preferred modern TEI division)
- [ ] div3 / div4 / div5 / div6 — (missing; deeper numbered division levels)
- [ ] nested div structure — (missing; `<div>` inside `<div>` at 3+ levels)
- [ ] sp / said (speech) — (missing; `<sp>` with `<speaker>` and `<p>`)
- [ ] stage direction — (missing; `<stage>`)
- [ ] epigraph — (missing; `<epigraph>` with `<quote>` and `<bibl>`)
- [ ] argument — (missing; `<argument>` prefatory block)
- [ ] byline — (missing; `<byline>`)
- [ ] dateline / salute / signed — (missing; letter/document structure elements)
- [ ] trailer — (missing; `<trailer>`)
- [ ] castList — (missing; `<castList>` with `<castItem>`)
- [ ] cit (quotation with attribution) — (missing; `<cit>` with `<quote>` and `<bibl>`)
- [ ] ab (anonymous block) — (missing; `<ab>`)
- [ ] gap / space — (missing; `<gap>` editorial intervention, `<space>`)
- [ ] list with `type` attribute variants — (missing; `type="bulleted"`, `type="ordered"`, `type="gloss"`, `type="simple"`)
- [ ] nested list — (missing; `<list>` inside `<item>`)
- [ ] item with label — (missing; `<label>` sibling to `<item>`)

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
- [ ] inline code — (missing; `<code>` or `<hi rend="code">`)
- [ ] foreign language phrase — (missing; `<foreign xml:lang="…">`)
- [ ] term — (missing; `<term>` inline)
- [ ] gloss (inline) — (missing; `<gloss>` inline)
- [ ] abbr — (missing; `<abbr>`)
- [ ] expan (expansion of abbreviation) — (missing; `<expan>`)
- [ ] choice (abbr/expan pair) — (missing; `<choice><abbr>…</abbr><expan>…</expan></choice>`)
- [ ] orig / reg (normalization pair) — (missing; `<choice><orig>…</orig><reg>…</reg></choice>`)
- [ ] sic / corr (correction pair) — (missing; `<choice><sic>…</sic><corr>…</corr></choice>`)
- [ ] add (addition) — (missing; `<add>`)
- [ ] del (deletion) — (missing; `<del>`)
- [ ] supplied — (missing; `<supplied>`)
- [ ] unclear — (missing; `<unclear>`)
- [ ] persName — (missing; `<persName>`)
- [ ] placeName — (missing; `<placeName>`)
- [ ] orgName — (missing; `<orgName>`)
- [ ] date (inline) — (missing; `<date when="…">`)
- [ ] title (inline) — (missing; `<title>` as inline bibliographic reference)
- [ ] name (generic) — (missing; `<name>`)
- [ ] num — (missing; `<num>`)
- [ ] measure — (missing; `<measure>`)
- [ ] xref / ptr — (missing; `<ptr target="…">`)
- [ ] anchor — (missing; `<anchor xml:id="…">`)
- [ ] milestone — (missing; `<milestone unit="…">`)
- [ ] seg — (missing; `<seg>` generic inline span)
- [ ] w / pc (token / punctuation) — (missing; corpus / linguistic annotation)
- [ ] inline math — (missing; `<formula type="inline">`)
- [ ] note (marginal / endnote) — (missing; `<note place="margin">`, `<note place="end">`)

## TEI Header (metadata)

- [ ] teiHeader / fileDesc — (missing; `<teiHeader>` with `<fileDesc>`)
- [ ] titleStmt — (missing; `<titleStmt>` with `<title>`, `<author>`, `<editor>`)
- [ ] publicationStmt — (missing; `<publicationStmt>` with `<publisher>`, `<date>`, `<idno>`)
- [ ] sourceDesc — (missing; `<sourceDesc>`)
- [ ] profileDesc / langUsage — (missing; `<langUsage>` with `<language ident="…">`)
- [ ] encodingDesc — (missing; `<encodingDesc>`)
- [ ] revisionDesc / change — (missing; `<revisionDesc>` with `<change>`)
- [ ] abstract (in profileDesc) — (missing; `<abstract>` in `<profileDesc>`)
- [ ] keywords (in profileDesc) — (missing; `<textClass>` / `<keywords>`)
- [ ] msDesc (manuscript description) — (missing; `<msDesc>` in `<sourceDesc>`)

## Properties

- [x] heading level (div1 vs div2) — `heading`, `heading-h2`
- [x] list ordered/unordered via `rend` — `list-ordered`, `list-unordered`
- [x] figure description — `figure` (`<figDesc>`)
- [x] table row role ("label") — `table` (`<row role="label">`)
- [x] table cell rend ("header") — `table-header`
- [ ] xml:id on div — (missing)
- [ ] xml:lang on element — (missing)
- [ ] rend values beyond covered set — (missing; `rend="center"`, `rend="right"`, `rend="it"`, `rend="b"`)
- [ ] type attribute on note — (missing; `<note type="footnote">` vs `type="endnote">`)
- [ ] n attribute (numbering) — (missing; `n` on `<div>`, `<l>`, `<p>`)
- [ ] corresp / sameAs (linking attributes) — (missing)
- [ ] graphic dimensions (width/height) — (missing; `<graphic width="…" height="…">`)
- [ ] table cols/rows — (missing; `cols` and `rows` on `<cell>`)
- [ ] list item label — (missing; `<label>` before `<item>`)

## Composition (integration)

- [ ] nested divs (3 levels) — (missing)
- [ ] inline formatting inside list items — (missing)
- [ ] table with inline formatting in cells — (missing)
- [ ] verse with speaker — (missing; `<sp>` containing `<lg>`)
- [ ] footnote with formatted content — (missing; `<note>` containing `<hi>`)
- [ ] cit with bibl attribution — (missing)
- [ ] choice (sic/corr) inside paragraph — (missing)
- [ ] full document with teiHeader + text — (missing)
- [ ] front matter + body + back matter — (missing; `<front>`, `<body>`, `<back>`)

## Adversarial

- [x] empty document — `adv-empty`
- [ ] malformed XML (unclosed tag) — (missing)
- [ ] missing TEI namespace — (missing)
- [ ] unknown element — (missing; element not in TEI spec)
- [ ] entity references — (missing)
- [ ] numeric character references — (missing)
- [ ] empty paragraph — (missing; `<p/>`)
- [ ] note with no content — (missing; `<note/>`)
- [ ] broken target reference — (missing; `<ref target="#nonexistent">`)

## Pathological

- [ ] deeply nested divs (6+ levels) — (missing)
- [ ] very large table — (missing)
- [ ] long poem with many lines — (missing)
- [ ] document with many footnotes — (missing)
- [ ] teiHeader with all optional metadata — (missing)
