# DokuWiki Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

DokuWiki syntax reference: https://www.dokuwiki.org/wiki:syntax

## Block constructs

- [x] paragraph — `paragraph`
- [x] heading h1 (`======`) — `heading`
- [x] heading h2 (`=====`) — `heading-h2`
- [ ] heading h3–h5 (levels 3–5) — (missing)
- [x] horizontal rule (`----`) — `horizontal-rule`
- [x] unordered list (`  *`) — `list-unordered`
- [x] ordered list (`  -`) — `list-ordered`
- [ ] nested list (2+ levels) — (missing)
- [x] code block (`<code>`) — `code-block`
- [x] blockquote (`>`) — `rare-blockquote`
- [ ] file block (`<file>`) — (missing)
- [ ] HTML block (`<html>`) — (missing)
- [ ] PHP block (`<php>`) — (missing)
- [ ] indented block / `<WRAP>` plugin — (missing)
- [ ] table — (missing)
- [ ] note/box plugin constructs — (missing)

## Inline constructs

- [x] bold (`**text**`) — `bold`
- [x] italic (`//text//`) — `italic`
- [x] underline (`__text__`) — `underline`
- [x] inline code (`''text''`) — `rare-code-inline`
- [ ] monospace (same as inline code in DokuWiki) — (missing)
- [ ] strikethrough (`<del>text</del>`) — (missing)
- [ ] subscript (`<sub>text</sub>`) — (missing)
- [ ] superscript (`<sup>text</sup>`) — (missing)
- [x] link (internal `[[page]]` / external `[[url|text]]`) — `link`
- [x] image (`{{image.png}}`) — `rare-image`
- [ ] image with alignment (`{{ image.png}}` / `{{image.png }}`) — (missing)
- [ ] image with size (`{{image.png?200}}`) — (missing)
- [ ] footnote (`((footnote text))`) — (missing)
- [ ] forced line break (`\\`) — (missing)
- [ ] smart quotes / typographic replacements — (missing)
- [ ] email address auto-link — (missing)
- [ ] bare URL auto-link — (missing)

## Properties

- [ ] heading levels h3–h5 — (missing)
- [ ] link display text — (missing; `link` fixture may not test display text explicitly)
- [ ] image alt text — (missing)
- [ ] image alignment (left/right/center) — (missing)
- [ ] image width/height — (missing)
- [ ] code block language (`<code lang>`) — (missing; `code-block` may not test language tag)
- [ ] ordered list start value — (missing)

## Composition (integration)

- [ ] nested lists (2+ levels) — (missing)
- [ ] inline markup inside table cells — (missing)
- [ ] inline markup inside list items — (missing)
- [ ] blockquote with inline markup — (missing)
- [ ] multiple paragraphs with mixed inline — (missing)

## Adversarial

- [x] empty document — `adv-empty`
- [ ] unclosed bold/italic — (missing)
- [ ] unclosed `<code>` block — (missing)
- [ ] unclosed footnote `((` — (missing)
- [ ] nested tables (not supported; parser must not crash) — (missing)

## Pathological

- [ ] deeply nested lists (5+ levels) — (missing)
- [ ] very wide table (20+ columns) — (missing)
- [ ] heading containing inline markup — (missing)
- [ ] paragraph with many consecutive inline spans — (missing)
