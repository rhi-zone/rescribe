# FictionBook 2 Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

FictionBook 2 schema reference: http://www.fictionbook.org/index.php/Eng:FictionBook_2.1_Description

## Block constructs

- [x] paragraph — `paragraph` (`<p>`)
- [x] section with heading — `section-heading` (`<section>` with `<title>`)
- [x] nested section — `nested-section` (`<section>` inside `<section>`)
- [x] blockquote / cite — `blockquote` (`<cite>`)
- [x] table — `table` (`<table>` with `<tr>` / `<td>`)
- [x] table header cell — `table-header` (`<th>`)
- [x] verse / poem — `verse-line-break` (`<poem>` / `<stanza>` / `<v>`)
- [x] epigraph — `epigraph` (`<epigraph>` with `<text-author>`)
- [x] empty line — `empty-line` (`<empty-line/>`)
- [x] subtitle — `subtitle` (`<subtitle>`)
- [ ] code block — (missing; there is no dedicated preformatted block in FB2 core; typically `<code>` inline or publisher extension)
- [ ] annotation — (missing; `<annotation>` inside `<description>` / `<title-info>`)
- [ ] text-author (standalone) — `text-author` covered in `epigraph` fixture

## Inline constructs

- [x] emphasis (italic) — `emphasis` (`<emphasis>`)
- [x] strong (bold) — `strong` (`<strong>`)
- [x] strikeout — `strikeout` (`<strikethrough>`)
- [x] subscript — `subscript` (`<sub>`)
- [x] superscript — `superscript` (`<sup>`)
- [x] code (inline) — `code-inline` (`<code>`)
- [x] link — `link` (`<a l:href="…">` external)
- [x] internal link (anchor) — `internal-link` (`<a l:href="#…">` with section `id`)
- [x] image (inline / block) — `image` (`<image l:href="#…">`)
- [ ] footnote reference — (missing; `<a l:href="#fn1" type="note">`)
- [ ] footnote body (note section) — (missing; `<body name="notes"><section id="fn1">`)

## Metadata

- [x] book title — `title-metadata` (`<book-title>` in `<title-info>`)
- [x] author — `author-metadata` (`<author>` with `<first-name>`, `<last-name>` in `<title-info>`)
- [x] genre — `genre-metadata` (`<genre>` in `<title-info>`)
- [x] language — `lang-metadata` (`<lang>` in `<title-info>`)
- [x] date — `date` (`<date>` in `<title-info>` or `<publish-info>`)
- [x] series / sequence — `series-sequence` (`<sequence name="…" number="…">`)
- [x] cover image — `cover-image` (`<coverpage>` with `<image>` in `<title-info>`)
- [x] publisher info — `publisher-info` (`<publish-info>` with `<publisher>`, `<year>`, `<isbn>`)
- [x] document info — `document-info` (`<document-info>` with `<author>`, `<date>`, `<id>`, `<version>`)
- [x] custom info — `custom-info` (`<custom-info info-type="…">`)
- [x] keywords — `keywords` (`<keywords>` in `<title-info>`)
- [x] translator — `translator` (`<translator>` in `<title-info>`)
- [x] src-lang — `src-lang` (`<src-lang>` in `<title-info>`)

## Binary resources

- [ ] binary image embedding — (missing; `<binary id="…" content-type="image/png">base64…</binary>`)
- [ ] multiple binaries — (missing; document with several embedded image resources)

## Properties

- [x] section id attribute — `internal-link` (`id` on `<section>`)
- [x] image alt text — `image-alt-text` (`alt` attribute on `<image>`; parsed by fb2-fmt, url prop set on image node)
- [ ] link title — (missing; `type` or title attributes on `<a>`)
- [ ] table alignment (align attribute) — (missing)
- [x] xml:lang on body — `xml-lang-body` (`xml:lang` on `<body>`; body parsed as div node)

## Composition (integration)

- [ ] footnotes (note body + inline ref) — (missing; full footnote roundtrip)
- [x] poem with epigraph — `poem-epigraph` (`<poem>` with `<epigraph>` before stanzas)
- [ ] section with epigraph and body — covered in `epigraph` fixture
- [x] inline image inside paragraph — `inline-image` (`<image>` as inline child of `<p>`)
- [ ] multiple bodies (notes body) — (missing; separate `<body name="notes">`)
- [ ] nested list — (missing; FB2 has no native list; typically done with `<p>` + custom prefix, but `<ul>`/`<ol>` extensions exist in some FB2 dialects)

## Adversarial

- [x] empty document — `adv-empty`
- [x] malformed XML (broken opening tag) — `adv-malformed`
- [x] entity references (&amp;, &lt;, etc.) — `adv-entity-refs`
- [x] empty section (no title, no content) — `adv-empty-section`
- [x] missing xmlns declaration — `adv-missing-xmlns` (should parse fine since reader uses local names)
- [ ] binary with invalid base64 — (missing)
- [x] broken internal image ref — `adv-broken-image-ref` (`<image l:href="#nonexistent">`)
- [ ] broken internal footnote ref — (missing)
- [x] numeric character references — `adv-numeric-charref` (decimal `&#65;` and hex `&#x41;` references)

## Pathological

- [x] deeply nested sections — `deeply-nested-sections` (6 levels of `<section>`; heading levels clamped to 6)
- [ ] large binary image — (missing)
- [x] many paragraphs — `many-paragraphs` (50 paragraphs in a single section)
- [x] table with many cells — `table-many-cells` (6 rows × 5 columns)
