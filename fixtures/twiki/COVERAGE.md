# TWiki Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

TWiki markup reference: https://twiki.org/cgi-bin/view/TWiki/TextFormattingRules

## Block constructs

- [x] paragraph — `paragraph`
- [x] heading h1 (`---+`) — `heading`
- [x] heading h2 (`---++`) — `heading-h2`
- [ ] heading h3–h6 (`---+++` through `---++++++`) — (missing)
- [x] horizontal rule (`---`) — `horizontal-rule`
- [x] unordered list (`   *`) — `list-unordered`
- [x] ordered list (`   1.` or `   1`) — `list-ordered`
- [ ] nested list (2+ levels) — (missing)
- [ ] definition list (`   $ term: definition`) — (missing)
- [x] code block / verbatim (`<verbatim>`) — `code-block`
- [x] table (pipe-based `|`) — `table`
- [x] table with header row (`*bold cell*` or `|*header*|`) — `table-header`
- [ ] blockquote (indented text via `<blockquote>`) — (missing)
- [ ] `%INCLUDE{}%` macro — (missing)
- [ ] `%TOC%` macro — (missing)
- [ ] HTML passthrough — (missing)

## Inline constructs

- [x] bold (`*text*`) — `bold`
- [x] italic (`_text_`) — `italic`
- [x] bold+italic (`__text__`) — `rare-bold-italic`
- [x] bold fixed-width (`==text==`) — `rare-bold-fixed`
- [x] inline code / fixed-width (`=text=`) — `code-inline`
- [ ] strikethrough (`<del>text</del>` or `---text---`) — (missing)
- [ ] underline (`<u>text</u>`) — (missing)
- [ ] subscript (`<sub>text</sub>`) — (missing)
- [ ] superscript (`<sup>text</sup>`) — (missing)
- [x] link (`[[url]]` / `[[url][text]]`) — `link`
- [ ] WikiWord auto-link — (missing)
- [ ] `[[WikiWord][display]]` link — (missing)
- [ ] image (`<img src=...>` HTML or `%ATTACHURL%`) — (missing)
- [ ] forced line break (`%BR%`) — (missing)
- [ ] `%VARIABLE%` macro expansion — (missing)
- [ ] color (`%RED%text%ENDCOLOR%`) — (missing)

## Properties

- [ ] heading levels h3–h6 — (missing)
- [ ] link display text — (missing)
- [ ] table header cells — covered by `table-header`
- [ ] table cell alignment (`|  centered  |`, `|      right|`) — (missing)
- [ ] ordered list start value — (missing)

## Composition (integration)

- [ ] nested lists (2+ levels) — (missing)
- [ ] inline markup inside table cells — (missing)
- [ ] inline markup inside list items — (missing)
- [ ] verbatim block inside table cell — (missing)

## Adversarial

- [x] empty document — `adv-empty`
- [ ] unclosed bold/italic — (missing)
- [ ] unclosed `<verbatim>` block — (missing)
- [ ] table with missing closing `|` — (missing)
- [ ] WikiWord that should not be linked (escaped with `!`) — (missing)

## Pathological

- [ ] deeply nested lists (5+ levels) — (missing)
- [ ] very wide table (20+ columns) — (missing)
- [ ] heading containing inline markup — (missing)
- [ ] paragraph with many consecutive inline spans — (missing)
