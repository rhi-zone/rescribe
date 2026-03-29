# XWiki Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

XWiki syntax 2.1 reference: https://www.xwiki.org/xwiki/bin/view/Documentation/UserGuide/Features/XWikiSyntax/

## Block constructs

- [x] paragraph — `paragraph`
- [x] heading h1 (`= h1 =`) — `heading`
- [x] heading h2 (`== h2 ==`) — `heading-h2`
- [x] heading h3–h6 — `heading-h3-h6`
- [x] horizontal rule (`----`) — `horizontal-rule`
- [x] unordered list (`* item`) — `list-unordered`
- [x] ordered list (`1. item`) — `list-ordered`
- [x] nested list (2+ levels) — `nested-list`
- [x] code block (`{{{...}}}` or `{{code}}...{{/code}}`) — `code-block`
- [x] code block with language (`{{code language="java"}}`) — `code-block-lang`
- [x] table — `table`
- [x] table with header row (`|=header|=`) — `table-header`
- [x] blockquote (`{{quote}}...{{/quote}}`) — `blockquote`
- [x] `{{info}}` macro — `macro-info`
- [x] `{{warning}}` macro — `macro-warning`
- [x] `{{error}}` macro — `macro-error`
- [x] `{{success}}` macro — `macro-success`
- [x] `{{box}}` macro — `macro-box`
- [x] `{{toc /}}` table of contents macro — `macro-toc`
- [x] `{{include /}}` transclusion macro — `macro-include`
- [x] `{{velocity}}` / `{{groovy}}` script blocks — `macro-velocity`

## Inline constructs

- [x] bold (`**text**`) — `bold`
- [x] italic (`//text//`) — `italic`
- [x] underline (`__text__`) — `underline`
- [x] strikethrough (`--text--`) — `strikeout`
- [x] monospace / inline code (`##text##`) — `rare-monospace`
- [x] superscript (`^^text^^`) — `superscript`
- [x] subscript (`~~text~~`) — `subscript`
- [x] link (`[[label>>url]]` / `[[url]]`) — `link`
- [x] image (`[[image:name.png]]`) — `image`
- [x] image with parameters (`[[image:name.png||width=100]]`) — `image-params`
- [x] anchor (`{{id name="anchor"/}}`) — `anchor`
- [x] mention (`{{mention reference="user"/}}`) — `mention`
- [x] forced line break (`\\`) — `line-break`
- [x] HTML passthrough (`{{html}}...{{/html}}`) — `html-passthrough`

## Properties

- [x] heading levels h3–h6 — `heading-h3-h6`
- [x] link with display label — `link-label`
- [x] image alt text / dimensions — `image-params`
- [x] table header cells — `table-header`
- [x] code block language — covered by `code-block-lang`

## Composition (integration)

- [x] nested lists (2+ levels) — `nested-list`
- [x] inline markup inside table cells — `inline-in-table`
- [x] inline markup inside list items — `inline-in-list`
- [x] macro containing inline markup — `macro-inline`
- [x] info macro containing a code block — `info-code-block`

## Adversarial

- [x] empty document — `adv-empty`
- [x] unclosed bold/italic — `adv-unclosed-bold`
- [x] unclosed `{{{` block — `adv-unclosed-block`
- [x] unclosed macro `{{code}}` without `{{/code}}` — `adv-unclosed-macro`
- [x] table with missing closing `|` — `adv-table-missing-pipe`
- [x] nested tables — `adv-nested-tables`

## Pathological

- [x] deeply nested lists (5+ levels) — `path-deep-list`
- [x] very wide table (20+ columns) — `path-wide-table`
- [x] heading containing inline markup — `path-heading-inline`
- [x] paragraph with many consecutive inline spans — `path-many-spans`
