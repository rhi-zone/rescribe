# Djot Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

Djot is a light markup language designed by John MacFarlane as a CommonMark successor
with an unambiguous grammar and richer inline/block extensibility.

## Block constructs
- [x] paragraph — `paragraph`
- [x] heading (ATX-style, levels 1–6) — `heading`
- [x] heading h2 — `heading-h2`
- [ ] heading h3–h6 individually — (missing)
- [x] fenced code block — `code-block`
- [x] blockquote (rare syntax `>`) — `rare-blockquote`
- [x] unordered list — `list-unordered`
- [x] ordered list — `list-ordered`
- [x] horizontal rule — `horizontal-rule`
- [x] table — `table`
- [x] task list — `task-list`
- [x] footnote definition — `footnote`
- [x] div (fenced with `:::`) — `div`
- [x] raw block — `raw-block`
- [ ] definition list — `definition-list`
- [ ] pipe table (alternative table syntax) — (missing)
- [ ] block-level attributes (`{.class #id key=val}`) — (missing)
- [ ] thematic break (alternative forms) — (missing)

## Inline constructs
- [x] emphasis (italic, `_..._`) — `italic`
- [x] strong (bold, `**...**`) — `bold`
- [x] strikethrough (`{~~...~~}`) — `rare-strikeout`
- [x] underline (`{+...+}`) — `underline`
- [x] mark / highlight (`{=...=}`) — `mark`
- [x] subscript (`~...~`) — `subscript`
- [x] superscript (`^...^`) — `superscript`
- [x] inline code (`` `...` ``) — `code-inline`
- [x] link (`[text](url)`) — `link`
- [x] image (`![alt](url)`) — `image`
- [x] hard line break — `line-break`
- [x] soft line break — `soft-break`
- [x] raw inline — `raw-inline`
- [x] footnote reference — `footnote`
- [x] span (`[text]{attrs}`) — `span`
- [x] math inline — `math`
- [x] math display — `math-display`
- [ ] autolink (`<url>`) — (missing)
- [ ] smart punctuation (em-dash, en-dash, ellipsis, smart quotes) — (missing)
- [ ] insert (`{++...++}`) — (missing)
- [ ] delete (`{--...--}`) — (missing)

## Properties
- [x] fenced code block language — `code-block-lang`
- [ ] heading level — `heading` (h1 and h2 covered; h3–h6 missing)
- [ ] link title — (missing)
- [ ] image alt text — `image`
- [ ] image title — (missing)
- [ ] task list item checked state — `task-list`
- [ ] block-level class attribute — (missing)
- [ ] block-level id attribute — (missing)
- [ ] span class attribute — `span`
- [ ] inline-level id attribute — (missing)
- [ ] div class attribute — `div`
- [ ] raw block format — `raw-block`
- [ ] raw inline format — `raw-inline`
- [ ] footnote reference label — `footnote`
- [ ] table cell alignment — (missing)
- [ ] ordered list style (decimal, roman, alpha) — (missing)
- [ ] ordered list start number — (missing)

## Composition (integration)
- [ ] emphasis inside table cell — (missing)
- [ ] footnote inside list item — (missing)
- [ ] math inside blockquote — (missing)
- [ ] span with attributes inside heading — (missing)
- [ ] div containing list and table — (missing)
- [ ] nested lists — (missing)
- [ ] raw block inside div — (missing)
- [ ] strikethrough inside emphasis — (missing)
- [ ] link inside strong — (missing)

## End-to-end
- [ ] realistic multi-section document — (missing)
- [ ] document using div, span, attributes, math, footnotes — (missing)

## Rare
- [x] blockquote — `rare-blockquote`
- [x] strikethrough — `rare-strikeout`
- [ ] setext-style thematic break — (missing)
- [ ] autolink — (missing)
- [ ] smart punctuation — (missing)
- [ ] insert / delete inline — (missing)
- [ ] definition list — `definition-list`
- [ ] ordered list with letter numbering — (missing)
- [ ] ordered list with roman numerals — (missing)
- [ ] nested spans with multiple attributes — (missing)
- [ ] block attribute on fenced code block — (missing)
- [ ] link reference definition — (missing)

## Adversarial
- [x] empty document — `adv-empty`
- [ ] whitespace-only document — (missing)
- [ ] unclosed fenced code block — (missing)
- [ ] unclosed inline markup — (missing)
- [ ] broken link (no closing paren) — (missing)
- [ ] malformed attribute block (`{...` unclosed) — (missing)
- [ ] malformed table — (missing)
- [ ] footnote reference with no definition — (missing)
- [ ] deeply nested divs — (missing)

## Pathological
- [ ] 1000-item list — (missing)
- [ ] deeply nested divs (100 levels) — (missing)
- [ ] very long paragraph (>64 KB) — (missing)
- [ ] large table (many rows/columns) — (missing)
- [ ] many footnotes — (missing)
- [ ] document with many span attributes — (missing)
