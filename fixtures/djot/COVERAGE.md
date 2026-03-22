# Djot Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

Djot is a light markup language designed by John MacFarlane as a CommonMark successor
with an unambiguous grammar and richer inline/block extensibility.

## Block constructs
- [x] paragraph — `paragraph`
- [x] heading (ATX-style, levels 1–6) — `heading`
- [x] heading h2 — `heading-h2`
- [x] heading h3–h6 individually — `heading-h3`, `heading-h4`, `heading-h5`, `heading-h6`
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
- [x] definition list — `definition-list`
- [x] pipe table (alternative table syntax) — `table` (Djot table syntax is already pipe-table)
- [x] block-level attributes (`{.class #id key=val}`) — `block-attributes`
- [x] thematic break (alternative forms) — `thematic-break-alt`

## Inline constructs
- [x] emphasis (italic, `_..._`) — `italic`
- [x] strong (bold, `*...*`) — `bold`
- [x] strikethrough (`{-...-}`) — `rare-strikeout`
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
- [x] autolink (`<url>`) — `autolink`
- [x] smart punctuation (em-dash, en-dash, ellipsis, smart quotes) — `smart-punctuation`
- [x] insert (`{+...+}`) — `insert`
- [x] delete (`{-...-}`) — `delete`

## Properties
- [x] fenced code block language — `code-block-lang`
- [x] heading level — `heading`, `heading-h2`, `heading-h3`, `heading-h4`, `heading-h5`, `heading-h6`
- [x] link title — `link-title`
- [x] image alt text — `image`
- [x] image title — `image-title`
- [x] task list item checked state — `task-list`
- [x] block-level class attribute — `block-attributes`
- [x] block-level id attribute — `block-attributes`
- [x] span class attribute — `span`
- [x] inline-level id attribute — `link-title`
- [x] div class attribute — `div`
- [x] raw block format — `raw-block`
- [x] raw inline format — `raw-inline`
- [x] footnote reference label — `footnote`
- [x] table cell alignment — `table-alignment`
- [x] ordered list style (decimal, roman, alpha) — `list-alpha`, `list-roman`
- [x] ordered list start number — `list-ordered-start`

## Composition (integration)
- [x] emphasis inside table cell — `emphasis-in-table`
- [x] footnote inside list item — `footnote-in-list`
- [x] math inside blockquote — `math-in-blockquote`
- [x] span with attributes inside heading — `span-in-heading`
- [x] div containing list and table — `div-with-list-and-table`
- [x] nested lists — `nested-lists`
- [x] raw block inside div — `raw-block-in-div`
- [x] strikethrough inside emphasis — `strikethrough-in-emphasis`
- [x] link inside strong — `link-in-strong`

## End-to-end
- [x] realistic multi-section document — `e2e-multi-section`
- [x] document using div, span, attributes, math, footnotes — `e2e-rich`

## Rare
- [x] blockquote — `rare-blockquote`
- [x] strikethrough — `rare-strikeout`
- [x] setext-style thematic break — `rare-thematic-break`
- [x] autolink — `autolink`
- [x] smart punctuation — `smart-punctuation`
- [x] insert / delete inline — `rare-insert-delete`
- [x] definition list — `definition-list`
- [x] ordered list with letter numbering — `list-alpha`
- [x] ordered list with roman numerals — `list-roman`
- [x] nested spans with multiple attributes — `nested-spans`
- [x] block attribute on fenced code block — `block-attr-on-code`
- [x] link reference definition — `link-reference`

## Adversarial
- [x] empty document — `adv-empty`
- [x] whitespace-only document — `adv-whitespace-only`
- [x] unclosed fenced code block — `adv-unclosed-fence`
- [x] unclosed inline markup — `adv-unclosed-inline`
- [x] broken link (no closing paren) — `adv-broken-link`
- [x] malformed attribute block (`{...` unclosed) — `adv-malformed-attr`
- [x] malformed table — `adv-malformed-table`
- [x] footnote reference with no definition — `adv-footnote-no-def`
- [x] deeply nested divs — `adv-nested-divs`

## Pathological
- [x] 1000-item list — `path-large-list`
- [x] deeply nested divs (100 levels) — `path-deep-divs`
- [x] very long paragraph (>64 KB) — `path-long-paragraph`
- [x] large table (many rows/columns) — `path-large-table`
- [x] many footnotes — `path-many-footnotes`
- [x] document with many span attributes — `path-many-spans`
