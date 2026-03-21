# VimWiki Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

VimWiki syntax reference: https://vimwiki.github.io/vimwikiwiki/VimWiki%20Markup%20Language.html
(Default VimWiki syntax; Markdown dialect is a separate mode and out of scope here.)

## Block constructs

- [x] paragraph — `paragraph`
- [x] heading h1 (`= h1 =`) — `heading`
- [x] heading h2 (`== h2 ==`) — `heading-h2`
- [ ] heading h3–h6 — (missing)
- [x] horizontal rule (`----`) — `horizontal-rule`
- [x] unordered list (`*` / `-`) — `list-unordered`
- [x] ordered list (`#`) — `list-ordered`
- [ ] nested list (2+ levels) — (missing)
- [x] code block (`{{{...}}}`) — `code-block`
- [x] blockquote (`    ` indented / `>`) — `blockquote`
- [x] table — `table`
- [x] task list (`- [ ]` / `- [X]`) — `task-list`
- [ ] definition list (`term:: definition`) — (missing)
- [ ] math block (`{{$...}}`) — (missing)
- [ ] `%title`, `%date`, `%toc` header directives — (missing)

## Inline constructs

- [x] bold (`*text*`) — `bold`
- [x] italic (`_text_`) — `italic`
- [x] strikethrough (`~~text~~`) — `rare-strikeout`
- [x] inline code (`` `text` `` or `{{{text}}}`) — `rare-code-inline`
- [x] link (`[[url]]` / `[[url|text]]`) — `link`
- [x] image (`{{url}}` / `{{url|alt}}`) — `rare-image`
- [ ] bold+italic combined — (missing)
- [ ] superscript (`^text^`) — (missing)
- [ ] subscript (`,,text,,`) — (missing)
- [ ] bare URL auto-link — (missing)
- [ ] WikiWord auto-link — (missing)
- [ ] tag (`#tag`) — (missing)
- [ ] math inline (`$...$`) — (missing)

## Properties

- [ ] heading levels h3–h6 — (missing)
- [ ] link with display text — (missing; `link` may not cover display text)
- [ ] image with alt text — (missing; `rare-image` may not test alt)
- [ ] task list item state (done / not done / in progress / etc.) — partially covered by `task-list`
- [ ] table cell alignment — (missing)
- [ ] code block language (`{{{lang\n...}}}`) — (missing)

## Composition (integration)

- [ ] nested lists (2+ levels) — (missing)
- [ ] inline markup inside table cells — (missing)
- [ ] inline markup inside list items — (missing)
- [ ] task list nested under regular list — (missing)
- [ ] blockquote with inline markup — (missing)

## Adversarial

- [x] empty document — `adv-empty`
- [ ] unclosed bold/italic — (missing)
- [ ] unclosed `{{{` code block — (missing)
- [ ] link with no closing `]]` — (missing)
- [ ] table with missing closing `|` — (missing)

## Pathological

- [ ] deeply nested lists (5+ levels) — (missing)
- [ ] very wide table (20+ columns) — (missing)
- [ ] heading containing inline markup — (missing)
- [ ] paragraph with many consecutive inline spans — (missing)
