# TikiWiki Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

TikiWiki markup reference: https://doc.tiki.org/Wiki-Syntax

## Block constructs

- [x] paragraph — `paragraph`
- [x] heading h1 (`!`) — `heading`
- [x] heading h2 (`!!`) — `heading-h2`
- [ ] heading h3–h6 (`!!!` through `!!!!!!`) — (missing)
- [x] horizontal rule (`---`) — `horizontal-rule`
- [x] unordered list (`*`) — `list-unordered`
- [x] ordered list (`#`) — `list-ordered`
- [ ] nested list (2+ levels) — (missing)
- [x] code block (`{CODE}` / `{code}`) — `code-block`
- [x] table (wiki table syntax `||`) — `table`
- [ ] table with header row — (missing)
- [ ] blockquote (`>`) — (missing)
- [ ] `{BOX}` plugin — (missing)
- [ ] `{QUOTE}` plugin — (missing)
- [ ] `{DIV}` plugin — (missing)
- [ ] preformatted (`~np~...~/np~` or `{HTML}`) — (missing)

## Inline constructs

- [x] bold (`__text__`) — `bold`
- [x] italic (`''text''`) — `italic`
- [x] underline (`===text===`) — `rare-underline`
- [x] strikethrough (`--text--` / `~~text~~`) — `strikethrough`
- [x] inline code / monospace (`-+text+-`) — `rare-code-inline`
- [ ] subscript (`~~text~~`) — (missing; syntax conflicts with some strikethrough notation)
- [ ] superscript (`^^text^^`) — (missing)
- [x] link (`((page))` / `[url|text]`) — `link`
- [ ] external link (`[url]`) — (missing; may be covered by `link`)
- [ ] image (`{img fileId=N}`) — (missing)
- [ ] anchor (`{ANAME}anchor{ANAME}`) — (missing)
- [ ] color (`~~#RRGGBB:text~~`) — (missing)
- [ ] `{FANCYTABLE}` / `{SPLIT}` plugins — (missing)

## Properties

- [ ] heading levels h3–h6 — (missing)
- [ ] link display text — (missing)
- [ ] image dimensions/alignment/caption — (missing)
- [ ] table cell alignment — (missing)
- [ ] code block language — (missing)

## Composition (integration)

- [ ] nested lists (2+ levels) — (missing)
- [ ] inline markup inside table cells — (missing)
- [ ] inline markup inside list items — (missing)
- [ ] plugin inside paragraph — (missing)

## Adversarial

- [x] empty document — `adv-empty`
- [ ] unclosed bold/italic — (missing)
- [ ] unclosed `{CODE}` block — (missing)
- [ ] table with missing row delimiter — (missing)
- [ ] nested tables — (missing)

## Pathological

- [ ] deeply nested lists (5+ levels) — (missing)
- [ ] very wide table (20+ columns) — (missing)
- [ ] heading containing inline markup — (missing)
- [ ] paragraph with many consecutive inline spans — (missing)
