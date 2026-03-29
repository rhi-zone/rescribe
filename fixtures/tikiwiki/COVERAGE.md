# TikiWiki Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

TikiWiki markup reference: https://doc.tiki.org/Wiki-Syntax

## Block constructs

- [x] paragraph — `paragraph`
- [x] heading h1 (`!`) — `heading`
- [x] heading h2 (`!!`) — `heading-h2`
- [x] heading h3–h6 (`!!!` through `!!!!!!`) — `heading-h3-h6`
- [x] horizontal rule (`---`) — `horizontal-rule`
- [x] unordered list (`*`) — `list-unordered`
- [x] ordered list (`#`) — `list-ordered`
- [x] nested list (2+ levels) — `nested-list`
- [x] code block (`{CODE}` / `{code}`) — `code-block`
- [x] table (wiki table syntax `||`) — `table`
- [x] table with header row — `table-header`
- [x] blockquote (`{QUOTE()}`) — `blockquote`
- [x] preformatted (`~np~...~/np~`) — `nowiki`

## Inline constructs

- [x] bold (`__text__`) — `bold`
- [x] italic (`''text''`) — `italic`
- [x] underline (`===text===`) — `rare-underline`
- [x] strikethrough (`--text--`) — `strikethrough`
- [x] inline code / monospace (`-+text+-`) — `rare-code-inline`
- [x] subscript (`,,text,,`) — `subscript`
- [x] superscript (`^text^`) — `superscript`
- [x] link (`[url|text]`) — `link`
- [x] external link (`[url]`) — `external-link`
- [x] image (`{img src=...}`) — `image`
- [x] wiki link (`((page))`) — `wikilink`

## Properties

- [x] heading levels h3–h6 — `heading-h3-h6`
- [x] link display text — `link`
- [x] image alt text — `image`
- [x] code block language — `code-block-language`

## Composition (integration)

- [x] nested lists (2+ levels) — `nested-list`
- [x] inline markup inside table cells — `inline-in-table`
- [x] inline markup inside list items — `inline-in-list`

## Adversarial

- [x] empty document — `adv-empty`
- [x] unclosed bold/italic — `adv-unclosed-bold`
- [x] unclosed `{CODE}` block — `adv-unclosed-code`
- [x] table with missing row delimiter — `adv-table-missing-delim`
- [x] adjacent tables — `adv-nested-tables`

## Pathological

- [x] deeply nested lists (5+ levels) — `nested-list-deep`
- [x] very wide table (20+ columns) — `path-wide-table`
- [x] heading containing inline markup — `path-heading-inline`
- [x] paragraph with many consecutive inline spans — `path-many-inline-spans`
