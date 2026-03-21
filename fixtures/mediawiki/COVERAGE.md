# MediaWiki Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

MediaWiki markup reference: https://www.mediawiki.org/wiki/Help:Formatting

## Block constructs

- [x] paragraph — `paragraph`
- [x] heading h1 (`=`) — `heading`
- [ ] heading h2 (`==`) — (missing)
- [x] heading h3 (`===`) — `heading-h3`
- [ ] heading h4–h6 — (missing)
- [x] horizontal rule (`----`) — `horizontal-rule`
- [x] unordered list (`*`) — `list-unordered`
- [x] ordered list (`#`) — `list-ordered`
- [ ] nested list (mixed `*` and `#`, 2+ levels) — (missing)
- [ ] definition list (`;term :definition`) — (missing)
- [ ] indented text (`:text`) — (missing)
- [x] preformatted / code block (`<pre>`) — `code-block`
- [x] table (wiki table syntax `{| ... |}`) — `table`
- [ ] table with caption (`|+`) — (missing)
- [ ] table header row (`!`) — (missing)
- [ ] table cell alignment attributes — (missing)
- [ ] `<blockquote>` — (missing)
- [ ] `<poem>` extension — (missing)
- [ ] magic words (`__TOC__`, `__NOTOC__`, etc.) — (missing)

## Inline constructs

- [x] bold (`'''text'''`) — `bold`
- [x] italic (`''text''`) — `italic`
- [x] underline (`<u>text</u>`) — `underline`
- [x] strikethrough (`<s>text</s>` / `<del>`) — `strikeout`
- [x] subscript (`<sub>text</sub>`) — `subscript`
- [x] superscript (`<sup>text</sup>`) — `superscript`
- [x] inline code (`<code>text</code>`) — `code-inline`
- [x] external link (`[url text]`) — `link-external`
- [x] internal link (`[[Page]]`) — `link-internal`
- [x] link with display text (`[[Page|display]]`) — `rare-link-display`
- [x] image (`[[File:name.png]]`) — `image`
- [x] forced line break (`<br>`) — `line-break`
- [ ] nowiki (`<nowiki>text</nowiki>`) — (missing)
- [ ] template transclusion (`{{Template}}`) — partially via `adv-template` (adversarial)
- [ ] parser function (`{{#if:...}}`) — (missing)
- [ ] variable (`{{PAGENAME}}`) — (missing)
- [ ] reference / footnote (`<ref>`) — (missing)
- [ ] `<ref group>` / `<references />` — (missing)
- [ ] small (`<small>`) / big (`<big>`) — (missing)
- [ ] `<mark>` highlight — (missing)
- [ ] `<abbr>` — (missing)
- [ ] HTML entity references — (missing)

## Properties

- [ ] heading levels h4–h6 — (missing)
- [ ] link display text (internal) — covered by `rare-link-display`
- [ ] image alt text — (missing)
- [ ] image width/height/thumbnail/frame — (missing)
- [ ] image alignment (left/right/center) — (missing)
- [ ] image caption — (missing)
- [ ] table cell/row attributes (colspan, rowspan, style) — (missing)
- [ ] ordered list start value — (missing)
- [ ] language / `xml:lang` attribute — (missing)

## Composition (integration)

- [ ] nested lists (2+ levels) — (missing)
- [ ] inline markup inside table cells — (missing)
- [ ] inline markup inside list items — (missing)
- [ ] template inside paragraph — (missing)
- [ ] reference list (`<references />`) paired with `<ref>` — (missing)
- [ ] image with caption inside table — (missing)

## Adversarial

- [x] empty document — `adv-empty`
- [x] template / transclusion (parser must not crash or infinitely recurse) — `adv-template`
- [ ] unclosed bold/italic — (missing)
- [ ] unclosed `<ref>` — (missing)
- [ ] deeply nested template calls — (missing)
- [ ] table with missing `|}` close — (missing)
- [ ] self-referential template (infinite loop guard) — (missing)

## Pathological

- [ ] deeply nested lists (5+ levels) — (missing)
- [ ] very wide table (20+ columns) — (missing)
- [ ] heading containing inline markup — (missing; `rare-heading-deep` covers depth, not inline)
- [x] heading levels beyond h2 (stress test) — `rare-heading-deep`
- [ ] article with 100+ references — (missing)
- [ ] paragraph with many consecutive inline spans — (missing)
