# VimWiki Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

VimWiki syntax reference: https://vimwiki.github.io/vimwikiwiki/VimWiki%20Markup%20Language.html
(Default VimWiki syntax; Markdown dialect is a separate mode and out of scope here.)

## Block constructs

- [x] paragraph — `paragraph`
- [x] heading h1 (`= h1 =`) — `heading`
- [x] heading h2 (`== h2 ==`) — `heading-h2`
- [x] heading h3–h6 — `heading-h3-h6`
- [x] horizontal rule (`----`) — `horizontal-rule`
- [x] unordered list (`*` / `-`) — `list-unordered`
- [x] ordered list (`#`) — `list-ordered`
- [x] nested list (2+ levels) — `nested-list`
- [x] code block (`{{{...}}}`) — `code-block`
- [x] blockquote (`>`) — `blockquote`
- [x] table — `table`
- [x] task list (`- [ ]` / `- [X]`) — `task-list`
- [x] definition list (`; term` / `: definition`) — `definition-list`
- [ ] math block (`{{$...}}`) — (missing)
- [ ] `%title`, `%date`, `%toc` header directives — (missing)

## Inline constructs

- [x] bold (`*text*`) — `bold`
- [x] italic (`_text_`) — `italic`
- [x] strikethrough (`~~text~~`) — `rare-strikeout`
- [x] inline code (`` `text` `` or `{{{text}}}`) — `rare-code-inline`
- [x] link (`[[url]]` / `[[url|text]]`) — `link`
- [x] image (`{{url}}` / `{{url|alt}}`) — `rare-image`
- [x] bold+italic combined — `bold-italic`
- [x] superscript (`^text^`) — `superscript`
- [x] subscript (`,,text,,`) — `subscript`
- [ ] bare URL auto-link — (missing)
- [ ] WikiWord auto-link — (missing)
- [ ] tag (`#tag`) — (missing)
- [ ] math inline (`$...$`) — (missing)

## Properties

- [x] heading levels h3–h6 — `heading-h3-h6`
- [x] link with display text — `link-display`
- [x] image with alt text and style — `image-alt`
- [x] task list item state (done / not done) — `task-list`
- [ ] table cell alignment — (missing)
- [x] code block language (`{{{lang\n...}}}`) — `code-block-lang`

## Composition (integration)

- [x] nested lists (2+ levels) — `nested-list`
- [x] inline markup inside table cells — `inline-in-table`
- [x] inline markup inside list items — `inline-in-list`
- [x] task list nested under regular list — `task-nested`
- [x] blockquote with inline markup — `blockquote-inline`

## Adversarial

- [x] empty document — `adv-empty`
- [x] unclosed bold/italic — `adv-unclosed-bold`
- [x] unclosed `{{{` code block — `adv-unclosed-code`
- [x] link with no closing `]]` — `adv-unclosed-link`
- [x] table with missing closing `|` — `adv-table-missing-pipe`

## Pathological

- [x] deeply nested lists (5+ levels) — `deep-nested-list`
- [x] very wide table (20+ columns) — `wide-table`
- [x] heading containing inline markup — `heading-inline`
- [x] paragraph with many consecutive inline spans — `many-inline-spans`
