# Zim Wiki Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

Zim wiki markup reference: https://zim-wiki.org/manual/Help/Wiki_Syntax.html

## Block constructs

- [x] paragraph — `paragraph`
- [x] heading h1 (`====== h1 ======`) — `heading`
- [x] heading h2 (`===== h2 =====`) — `heading-h2`
- [ ] heading h3–h5 — (missing)
- [x] horizontal rule (`---`) — `horizontal-rule`
- [x] unordered list (`* item`) — `list-unordered`
- [x] ordered list (`1. item`) — `list-ordered`
- [ ] nested list (2+ levels) — (missing)
- [x] code block (`{{{...}}}`) — `code-block`
- [x] table — `table`
- [ ] table with header row — (missing)
- [ ] verbatim block (`'''...'''`) — (missing)

## Inline constructs

- [x] bold (`**text**`) — `bold`
- [x] italic (`//text//`) — `italic`
- [x] strikethrough (`~~text~~`) — `rare-strikeout`
- [x] monospace (`''text''`) — `rare-monospace`
- [x] subscript (`_{text}`) — `subscript`
- [x] superscript (`^{text}`) — `superscript`
- [x] link (`[[url]]` / `[[url|text]]`) — `link`
- [x] image (`{{./image.png}}`) — `rare-image`
- [ ] underline (`__text__`) — (missing; Zim does not support underline natively — confirm)
- [ ] highlight (`__text__` or `{mark}text{/mark}`) — (missing)
- [ ] inline verbatim (`''text''` — same as monospace in Zim) — covered by `rare-monospace`
- [ ] WikiLink to sub-page (`[[+subpage]]`) — (missing)
- [ ] interwiki link (`[[wp?PageName]]`) — (missing)
- [ ] checked/unchecked checkbox inline (`[*]` / `[ ]`) — (missing)

## Properties

- [ ] heading levels h3–h5 — (missing)
- [ ] link with display text — (missing; `link` may not test display text explicitly)
- [ ] image alt text / dimensions — (missing; `rare-image` may not test alt)
- [ ] table header row — (missing)
- [ ] ordered list start value — (missing)

## Composition (integration)

- [ ] nested lists (2+ levels) — (missing)
- [ ] inline markup inside table cells — (missing)
- [ ] inline markup inside list items — (missing)
- [ ] image inside paragraph with text — (missing)

## Adversarial

- [x] empty document — `adv-empty`
- [ ] unclosed bold/italic — (missing)
- [ ] unclosed `{{{` code block — (missing)
- [ ] link with no closing `]]` — (missing)
- [ ] table with missing closing row — (missing)

## Pathological

- [ ] deeply nested lists (5+ levels) — (missing)
- [ ] very wide table (20+ columns) — (missing)
- [ ] heading containing inline markup — (missing)
- [ ] paragraph with many consecutive inline spans — (missing)
