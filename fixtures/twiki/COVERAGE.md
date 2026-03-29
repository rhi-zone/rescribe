# TWiki Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

TWiki markup reference: https://twiki.org/cgi-bin/view/TWiki/TextFormattingRules

## Block constructs

- [x] paragraph — `paragraph`
- [x] heading h1 (`---+`) — `heading`
- [x] heading h2 (`---++`) — `heading-h2`
- [x] heading h3–h6 (`---+++` through `---++++++`) — `heading-h3-h6`
- [x] horizontal rule (`---`) — `horizontal-rule`
- [x] unordered list (`   *`) — `list-unordered`
- [x] ordered list (`   1.` or `   1`) — `list-ordered`
- [x] nested list (2+ levels) — `nested-list`
- [x] definition list (`   $ term: definition`) — `definition-list`
- [x] code block / verbatim (`<verbatim>`) — `code-block`
- [x] table (pipe-based `|`) — `table`
- [x] table with header row (`*bold cell*` or `|*header*|`) — `table-header`
- [x] blockquote (indented text via `<blockquote>`) — `blockquote`
- [x] `%INCLUDE{}%` macro — `include-macro`
- [x] `%TOC%` macro — `toc-macro`
- [x] HTML passthrough — `html-passthrough`

## Inline constructs

- [x] bold (`*text*`) — `bold`
- [x] italic (`_text_`) — `italic`
- [x] bold+italic (`__text__`) — `rare-bold-italic`
- [x] bold fixed-width (`==text==`) — `rare-bold-fixed`
- [x] inline code / fixed-width (`=text=`) — `code-inline`
- [x] strikethrough (`<del>text</del>`) — `strikethrough`
- [x] underline (`<u>text</u>`) — `underline`
- [x] subscript (`<sub>text</sub>`) — `subscript`
- [x] superscript (`<sup>text</sup>`) — `superscript`
- [x] link (`[[url]]` / `[[url][text]]`) — `link`
- [x] WikiWord auto-link — `wikiword`
- [x] `[[WikiWord][display]]` link — `wikiword-link`
- [x] image (`<img src=...>` HTML or `%ATTACHURL%`) — `image`
- [x] forced line break (`%BR%`) — `line-break`
- [x] `%VARIABLE%` macro expansion — `variable-macro`
- [x] color (`%RED%text%ENDCOLOR%`) — `color-macro`

## Properties

- [x] heading levels h3–h6 — `heading-levels`
- [x] link display text — `link-display`
- [x] table header cells — covered by `table-header`
- [ ] table cell alignment (`|  centered  |`, `|      right|`) — not supported by TWiki syntax
- [ ] ordered list start value — TWiki always uses `1.`

## Composition (integration)

- [x] nested lists (2+ levels) — `nested-lists-2`
- [x] inline markup inside table cells — `inline-in-table`
- [x] inline markup inside list items — `inline-in-list`
- [x] verbatim block inside table cell — `verbatim-in-table`

## Adversarial

- [x] empty document — `adv-empty`
- [x] unclosed bold/italic — `adv-unclosed-bold`
- [x] unclosed `<verbatim>` block — `adv-unclosed-verbatim`
- [x] table with missing closing `|` — `adv-missing-pipe`
- [x] WikiWord that should not be linked (escaped with `!`) — `adv-escaped-wikiword`

## Pathological

- [x] deeply nested lists (5+ levels) — `deep-nested-list`
- [x] very wide table (20+ columns) — `wide-table`
- [x] heading containing inline markup — `heading-inline-markup`
- [x] paragraph with many consecutive inline spans — `many-inline-spans`
