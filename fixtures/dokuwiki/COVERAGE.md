# DokuWiki Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

DokuWiki syntax reference: https://www.dokuwiki.org/wiki:syntax

## Block constructs

- [x] paragraph — `paragraph`
- [x] heading h1 (`======`) — `heading`
- [x] heading h2 (`=====`) — `heading-h2`
- [x] heading h3 (`====`) — `heading-h3`
- [x] heading h4 (`===`) — `heading-h4`
- [x] heading h5 (`==`) — `heading-h5`
- [x] horizontal rule (`----`) — `horizontal-rule`
- [x] unordered list (`  *`) — `list-unordered`
- [x] ordered list (`  -`) — `list-ordered`
- [x] nested list (2+ levels) — `nested-list`
- [x] code block (`<code>`) — `code-block`
- [x] blockquote (`>`) — `rare-blockquote`
- [x] file block (`<file>`) — `file-block`
- [x] HTML block (`<html>`) — `rare-html-block`
- [x] PHP block (`<php>`) — `rare-php-block`
- [x] table — `table`
- [x] definition list (`;` / `:`) — `definition-list`

## Inline constructs

- [x] bold (`**text**`) — `bold`
- [x] italic (`//text//`) — `italic`
- [x] underline (`__text__`) — `underline`
- [x] inline code (`''text''`) — `rare-code-inline`
- [x] strikethrough (`<del>text</del>`) — `strikethrough`
- [x] subscript (`<sub>text</sub>`) — `subscript`
- [x] superscript (`<sup>text</sup>`) — `superscript`
- [x] link (internal `[[page]]` / external `[[url|text]]`) — `link`
- [x] image (`{{image.png}}`) — `rare-image`
- [x] footnote (`((footnote text))`) — `footnote`
- [x] forced line break (`\\`) — `line-break`
- [x] nowiki / literal (`%%...%%`) — `nowiki`

## Properties

- [x] heading levels h3–h5 — `heading-h3`, `heading-h4`, `heading-h5`
- [x] link display text — `link-display-text`
- [x] image alt text — `image-alt`
- [x] code block language (`<code lang>`) — `code-block-lang`

## Composition (integration)

- [x] nested lists (2+ levels) — `int-nested-lists`
- [x] inline markup inside table cells — `int-inline-in-table`
- [x] inline markup inside list items — `int-inline-in-list`
- [x] blockquote with inline markup — `int-blockquote-inline`
- [x] multiple paragraphs with mixed inline — `int-mixed-paragraphs`

## Adversarial

- [x] empty document — `adv-empty`
- [x] unclosed bold/italic — `adv-unclosed-bold`
- [x] unclosed `<code>` block — `adv-unclosed-code`
- [x] unclosed footnote `((` — `adv-unclosed-footnote`
- [x] nested tables (not supported; parser must not crash) — `adv-nested-tables`

## Pathological

- [x] deeply nested lists (5+ levels) — `path-deep-nested-lists`
- [x] very wide table (20+ columns) — `path-wide-table`
- [x] heading containing inline markup — `path-heading-inline`
- [x] paragraph with many consecutive inline spans — `path-many-inline-spans`
