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
- [ ] date — (missing; `<date>` in `<title-info>` or `<publish-info>`)
- [ ] series / sequence — (missing; `<sequence name="…" number="…">`)
- [ ] cover image — (missing; `<coverpage>` with `<image>` in `<title-info>`)
- [ ] publisher info — (missing; `<publish-info>` with `<publisher>`, `<year>`, `<isbn>`)
- [ ] document info — (missing; `<document-info>` with `<author>`, `<date>`, `<id>`, `<version>`)
- [ ] custom info — (missing; `<custom-info info-type="…">`)
- [ ] keywords — (missing; `<keywords>` in `<title-info>`)
- [ ] translator — (missing; `<translator>` in `<title-info>`)
- [ ] src-lang — (missing; `<src-lang>` in `<title-info>`)

## Binary resources

- [ ] binary image embedding — (missing; `<binary id="…" content-type="image/png">base64…</binary>`)
- [ ] multiple binaries — (missing; document with several embedded image resources)

## Properties

- [x] section id attribute — `internal-link` (`id` on `<section>`)
- [ ] image alt text — (missing; `alt` attribute on `<image>`)
- [ ] link title — (missing; `type` or title attributes on `<a>`)
- [ ] table alignment (align attribute) — (missing)
- [ ] xml:lang on body — (missing)

## Composition (integration)

- [ ] footnotes (note body + inline ref) — (missing; full footnote roundtrip)
- [ ] poem with epigraph — (missing)
- [ ] section with epigraph and body — covered in `epigraph` fixture
- [ ] inline image inside paragraph — (missing; `<image>` as inline child of `<p>`)
- [ ] multiple bodies (notes body) — (missing; separate `<body name="notes">`)
- [ ] nested list — (missing; FB2 has no native list; typically done with `<p>` + custom prefix, but `<ul>`/`<ol>` extensions exist in some FB2 dialects)

## Adversarial

- [x] empty document — `adv-empty`
- [x] malformed XML (broken opening tag) — `adv-malformed`
- [x] entity references (&amp;, &lt;, etc.) — `adv-entity-refs`
- [x] empty section (no title, no content) — `adv-empty-section`
- [ ] missing xmlns declaration — (missing; should parse fine since reader uses local names)
- [ ] binary with invalid base64 — (missing)
- [ ] broken internal image ref — (missing; `<image l:href="#nonexistent">`)
- [ ] broken internal footnote ref — (missing)
- [ ] numeric character references — (missing)

## Pathological

- [ ] deeply nested sections — (missing; 5+ levels of `<section>`)
- [ ] large binary image — (missing)
- [ ] many paragraphs — (missing)
- [ ] table with many cells — (missing)
