# Zim Wiki Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

Zim wiki markup reference: https://zim-wiki.org/manual/Help/Wiki_Syntax.html

## Block constructs

- [x] paragraph — `paragraph`
- [x] heading h1 (`====== h1 ======`) — `heading`
- [x] heading h2 (`===== h2 =====`) — `heading-h2`
- [x] heading h3 (`==== h3 ====`) — `heading-h3`
- [x] heading h4 (`=== h4 ===`) — `heading-h4`
- [x] heading h5 (`== h5 ==`) — `heading-h5`
- [x] horizontal rule (`---`) — `horizontal-rule`
- [x] unordered list (`* item`) — `list-unordered`
- [x] ordered list (`1. item`) — `list-ordered`
- [x] nested list (multiple items) — `nested-list`
- [x] code block (`'''...'''`) — `code-block`
- [x] verbatim block (`'''...'''`) — `verbatim-block`
- [x] table — `table`
- [x] table with header row — `table-header`
- [x] blockquote (`> text`) — `blockquote`

## Inline constructs

- [x] bold (`**text**`) — `bold`
- [x] italic (`//text//`) — `italic`
- [x] strikethrough (`~~text~~`) — `rare-strikeout`
- [x] monospace (`''text''`) — `rare-monospace`
- [x] subscript (`_{text}`) — `subscript`
- [x] superscript (`^{text}`) — `superscript`
- [x] link (`[[url]]` / `[[url|text]]`) — `link`
- [x] image (`{{./image.png}}`) — `rare-image`
- [x] underline (`__text__`) — `underline`
- [x] inline verbatim (`''text''` — same as monospace in Zim) — covered by `rare-monospace`
- [x] WikiLink to sub-page (`[[+subpage]]`) — `wikilink-subpage`
- [x] interwiki link (`[[wp?PageName]]`) — `interwiki-link`
- [x] checked/unchecked checkbox inline (`[*]` / `[ ]`) — `checkbox`

## Properties

- [x] heading levels h3–h5 — `heading-h3`, `heading-h4`, `heading-h5`
- [x] link with display text — `link-display-text`
- [x] image in paragraph context — `image-in-paragraph`
- [x] table header row — `table-header`

## Composition (integration)

- [x] inline markup inside table cells — `inline-in-table`
- [x] inline markup inside list items — `inline-in-list`
- [x] image inside paragraph with text — `image-in-paragraph`

## Adversarial

- [x] empty document — `adv-empty`
- [x] unclosed bold/italic — `adv-unclosed-bold`
- [x] unclosed `'''` code block — `adv-unclosed-code`
- [x] link with no closing `]]` — `adv-unclosed-link`
- [x] table with missing closing row — `adv-unclosed-table`

## Pathological

- [x] long list (20 items) — `path-deep-nesting`
- [x] very wide table (20 columns) — `path-wide-table`
- [x] heading containing inline markup — `path-heading-inline`
- [x] paragraph with many consecutive inline spans — `path-many-inline-spans`
