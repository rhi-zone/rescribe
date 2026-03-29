# Jira Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

Jira wiki markup reference (Atlassian): https://jira.atlassian.com/secure/WikiRendererHelpAction.jspa

## Block constructs

- [x] paragraph — `paragraph`
- [x] heading h1 (`h1.`) — `heading`
- [x] heading h2 (`h2.`) — `heading-h2`
- [x] heading h3 (`h3.`) — `heading-h3`
- [x] heading h4 (`h4.`) — `heading-h4`
- [x] heading h5 (`h5.`) — `heading-h5`
- [x] heading h6 (`h6.`) — `heading-h6`
- [x] horizontal rule (`----`) — `horizontal-rule`
- [x] unordered list (`*`) — `list-unordered`
- [x] ordered list (`#`) — `list-ordered`
- [x] nested list (mixed `*` and `#`, 2+ levels) — `nested-list`
- [x] code block (`{code}` / `{code:lang}`) — `code-block`
- [x] code block with language — `code-block-lang`
- [x] blockquote (`{quote}`) — `rare-blockquote`
- [x] panel (`{panel}`) — `panel`
- [x] noformat block (`{noformat}`) — `noformat`
- [x] table — `table`
- [x] table with header row (`||`) — `table-header`

## Inline constructs

- [x] bold (`*text*`) — `bold`
- [x] italic (`_text_`) — `italic`
- [x] underline (`+text+`) — `underline`
- [x] strikethrough (`-text-`) — `rare-strikeout`
- [x] subscript (`~text~`) — `subscript`
- [x] superscript (`^text^`) — `superscript`
- [x] inline code / monospace (`{{text}}`) — `rare-code-inline`
- [x] link (`[url]` / `[text|url]`) — `link`
- [x] image (`!image.png!`) — `image`
- [x] image with attributes (`!image.png|width=100!`) — `image-attrs`
- [x] mention (`@user`) — `mention`
- [x] color macro (`{color:red}text{color}`) — `color`

## Properties

- [x] heading levels h3–h6 — `heading-h3`, `heading-h4`, `heading-h5`, `heading-h6`
- [x] link display text — covered by `link`
- [x] image width/height/thumbnail attributes — `image-attrs`
- [x] code block language — `code-block-lang`
- [x] panel with title attribute — `panel-title`
- [x] table header row (`||`) — `table-header`

## Composition (integration)

- [x] nested lists (2+ levels, mixed ordered/unordered) — `nested-list`
- [x] inline markup inside table cells — `comp-inline-table`
- [x] inline markup inside list items — `comp-inline-list`
- [x] panel containing a code block — `comp-panel-code`
- [x] blockquote with inline markup — `comp-blockquote-inline`

## Adversarial

- [x] empty document — `adv-empty`
- [x] unclosed bold/italic — `adv-unclosed-bold`
- [x] unclosed `{code}` block — `adv-unclosed-code`
- [x] unclosed `{panel}` or `{quote}` — `adv-unclosed-panel`
- [x] table with missing closing `|` — `adv-missing-pipe`
- [x] nested tables (Jira does not support; parser must not crash) — `adv-nested-table`

## Pathological

- [x] deeply nested lists (5+ levels) — `path-deep-list`
- [x] very wide table (20+ columns) — `path-wide-table`
- [x] heading containing inline markup — `path-heading-inline`
- [x] code block with thousands of lines — `path-long-code`
