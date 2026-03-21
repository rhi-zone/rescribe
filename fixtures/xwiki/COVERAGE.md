# XWiki Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

XWiki syntax 2.1 reference: https://www.xwiki.org/xwiki/bin/view/Documentation/UserGuide/Features/XWikiSyntax/

## Block constructs

- [x] paragraph — `paragraph`
- [x] heading h1 (`= h1 =`) — `heading`
- [x] heading h2 (`== h2 ==`) — `heading-h2`
- [ ] heading h3–h6 — (missing)
- [x] horizontal rule (`----`) — `horizontal-rule`
- [x] unordered list (`* item`) — `list-unordered`
- [x] ordered list (`1. item`) — `list-ordered`
- [ ] nested list (2+ levels) — (missing)
- [x] code block (`{{{...}}}` or `{{code}}...{{/code}}`) — `code-block`
- [x] code block with language (`{{code language="java"}}`) — `code-block-lang`
- [x] table — `table`
- [ ] table with header row (`|=header|=`) — (missing)
- [ ] blockquote (`> text`) — (missing)
- [ ] `{{info}}` / `{{warning}}` / `{{error}}` / `{{success}}` macros — (missing)
- [ ] `{{box}}` macro — (missing)
- [ ] `{{toc /}}` table of contents macro — (missing)
- [ ] `{{include /}}` transclusion macro — (missing)
- [ ] `{{velocity}}` / `{{groovy}}` script blocks — (missing)

## Inline constructs

- [x] bold (`**text**`) — `bold`
- [x] italic (`//text//`) — `italic`
- [x] underline (`__text__`) — `underline`
- [x] strikethrough (`--text--`) — `strikeout`
- [x] monospace / inline code (`##text##`) — `rare-monospace`
- [ ] superscript (`^^text^^`) — (missing)
- [ ] subscript (`,,text,,`) — (missing)
- [x] link (`[[label>>url]]` / `[[url]]`) — `link`
- [ ] image (`[[image:name.png]]`) — (missing)
- [ ] image with parameters (`[[image:name.png||width=100]]`) — (missing)
- [ ] anchor (`{{id name="anchor"/}}`) — (missing)
- [ ] mention (`{{mention reference="user"/}}`) — (missing)
- [ ] forced line break (`\\`) — (missing)
- [ ] HTML passthrough (`{{html}}...{{/html}}`) — (missing)

## Properties

- [ ] heading levels h3–h6 — (missing)
- [ ] link with display label — (missing; `link` fixture may not cover label explicitly)
- [ ] image alt text / dimensions — (missing)
- [ ] table header cells — (missing)
- [ ] code block language — covered by `code-block-lang`
- [ ] ordered list start value / style — (missing)

## Composition (integration)

- [ ] nested lists (2+ levels) — (missing)
- [ ] inline markup inside table cells — (missing)
- [ ] inline markup inside list items — (missing)
- [ ] macro containing inline markup — (missing)
- [ ] info macro containing a code block — (missing)

## Adversarial

- [x] empty document — `adv-empty`
- [ ] unclosed bold/italic — (missing)
- [ ] unclosed `{{{` block — (missing)
- [ ] unclosed macro `{{code}}` without `{{/code}}` — (missing)
- [ ] table with missing closing `|` — (missing)
- [ ] nested tables — (missing)

## Pathological

- [ ] deeply nested lists (5+ levels) — (missing)
- [ ] very wide table (20+ columns) — (missing)
- [ ] heading containing inline markup — (missing)
- [ ] paragraph with many consecutive inline spans — (missing)
