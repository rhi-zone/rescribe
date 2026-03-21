# MultiMarkdown Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

MultiMarkdown (MMD) extends Markdown with tables, footnotes, definition lists,
math, metadata, cross-references, glossaries, abbreviations, and more.

## Block constructs (Markdown baseline)
- [x] paragraph — `paragraph`
- [x] heading — `heading`
- [ ] heading levels h2–h6 individually — (missing)
- [ ] setext heading — (missing)
- [x] fenced code block — `code-block`
- [ ] indented code block — (missing)
- [x] blockquote — `blockquote`
- [x] unordered list — `list-unordered`
- [x] ordered list — `list-ordered`
- [x] horizontal rule — `horizontal-rule`
- [x] raw HTML block — `raw-html-block`
- [ ] link reference definition — (missing)

## Block constructs (MMD extensions)
- [x] table (with alignment) — `table`
- [x] footnote definition — `footnote`
- [x] definition list — `definition-list`
- [ ] metadata block (MMD front matter, key: value) — (missing)
- [ ] table of contents placeholder (`{{TOC}}`) — (missing)
- [ ] abbreviation definition — (missing)
- [ ] glossary term definition — (missing)
- [ ] file transclusion (`{{file}}`) — (missing)
- [ ] comment block (`<!--` ... `-->`) — (missing)

## Inline constructs (Markdown baseline)
- [x] emphasis (italic) — `emphasis`
- [x] strong (bold) — `strong`
- [x] strikethrough — `strikeout`
- [x] inline code — `code-inline`
- [x] link (inline) — `link`
- [x] image — `image`
- [x] raw HTML inline — `raw-html-inline`
- [x] hard line break — `line-break`
- [x] soft line break — `soft-break`
- [ ] autolink — (missing)
- [ ] backslash escape — (missing)
- [ ] entity reference — (missing)

## Inline constructs (MMD extensions)
- [x] footnote reference — `footnote`
- [x] subscript — `subscript`
- [x] superscript — `superscript`
- [x] inline math — `math-inline`
- [x] display math — `math-display`
- [ ] cross-reference (heading anchor link) — (missing)
- [ ] image with dimensions (`![alt][ref]{width=...}`) — (missing)
- [ ] critic markup (addition/deletion/substitution) — (missing)
- [ ] abbreviation inline — (missing)
- [ ] glossary reference — (missing)

## Properties
- [ ] fenced code block language — (missing; `code-block` present but lang not separately tested)
- [ ] table column alignment — (missing)
- [ ] ordered list start number — (missing)
- [ ] link title — (missing)
- [ ] image alt text — `image`
- [ ] image title — (missing)
- [ ] image dimensions — (missing)
- [ ] heading level — `heading`
- [ ] metadata title — (missing)
- [ ] metadata author — (missing)
- [ ] metadata date — (missing)
- [ ] footnote reference label — `footnote`

## Composition (integration)
- [ ] emphasis inside table cell — (missing)
- [ ] footnote reference inside list item — (missing)
- [ ] math inside blockquote — (missing)
- [ ] definition list inside blockquote — (missing)
- [ ] nested list with footnotes — (missing)
- [ ] table with formatted cells — (missing)
- [ ] cross-reference to heading — (missing)

## End-to-end
- [ ] realistic academic document (metadata, footnotes, math, table) — (missing)

## Rare
- [ ] setext heading — (missing)
- [ ] indented code block — (missing)
- [ ] table with colspan (MMD extension) — (missing)
- [ ] multiline table cell — (missing)
- [ ] footnote with multiple paragraphs — (missing)
- [ ] nested footnote references — (missing)
- [ ] abbreviation definition and inline use — (missing)
- [ ] file transclusion — (missing)
- [ ] TOC placeholder — (missing)

## Adversarial
- [ ] empty document — (missing)
- [ ] whitespace-only document — (missing)
- [ ] unclosed fenced code block — (missing)
- [ ] unclosed emphasis — (missing)
- [ ] broken link — (missing)
- [ ] footnote reference with no definition — (missing)
- [ ] malformed math (unclosed `$`) — (missing)
- [ ] malformed table — (missing)

## Pathological
- [ ] 1000-item list — (missing)
- [ ] deeply nested blockquotes — (missing)
- [ ] very long paragraph (>64 KB) — (missing)
- [ ] large table (many rows/columns) — (missing)
- [ ] many footnotes — (missing)
- [ ] large math block — (missing)
