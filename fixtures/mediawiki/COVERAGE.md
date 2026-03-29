# MediaWiki Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

MediaWiki markup reference: https://www.mediawiki.org/wiki/Help:Formatting

## Block constructs

- [x] paragraph -- `paragraph`
- [x] heading h1 (`=`) -- `heading`
- [x] heading h2 (`==`) -- `heading-h2`
- [x] heading h3 (`===`) -- `heading-h3`
- [x] heading h4 (`====`) -- `heading-h4`
- [x] heading h5 (`=====`) -- `heading-h5`
- [x] heading h6 (`======`) -- `heading-h6`
- [x] horizontal rule (`----`) -- `horizontal-rule`
- [x] unordered list (`*`) -- `list-unordered`
- [x] ordered list (`#`) -- `list-ordered`
- [x] nested list (mixed `*` and `#`, 2+ levels) -- `nested-list`
- [x] definition list (`;term :definition`) -- `definition-list`
- [x] preformatted / code block (`<pre>`) -- `code-block`, `pre-block`
- [x] table (wiki table syntax `{| ... |}`) -- `table`
- [x] table with caption (`|+`) -- `table-caption`
- [x] table header row (`!`) -- `table-header`
- [x] `<blockquote>` -- `blockquote`
- [x] `<syntaxhighlight>`/`<source>` code blocks -- `syntaxhighlight`
- [x] magic words (`__TOC__`, `__NOTOC__`, etc.) -- `magic-words`

## Inline constructs

- [x] bold (`'''text'''`) -- `bold`
- [x] italic (`''text''`) -- `italic`
- [x] underline (`<u>text</u>`) -- `underline`
- [x] strikethrough (`<s>text</s>` / `<del>`) -- `strikeout`
- [x] subscript (`<sub>text</sub>`) -- `subscript`
- [x] superscript (`<sup>text</sup>`) -- `superscript`
- [x] inline code (`<code>text</code>`) -- `code-inline`
- [x] external link (`[url text]`) -- `link-external`
- [x] internal link (`[[Page]]`) -- `link-internal`
- [x] link with display text (`[[Page|display]]`) -- `rare-link-display`
- [x] image (`[[File:name.png]]`) -- `image`
- [x] forced line break (`<br>`) -- `line-break`
- [x] nowiki (`<nowiki>text</nowiki>`) -- `nowiki`
- [x] template transclusion (`{{Template}}`) -- `template-inline`, `adv-template`
- [x] reference / footnote (`<ref>`) -- `footnote-ref`
- [x] inline math (`<math>`) -- `math-inline`

## Properties

- [x] heading levels h4-h6 -- `heading-h4`, `heading-h5`, `heading-h6`
- [x] link display text (internal) -- covered by `rare-link-display`

## Composition (integration)

- [x] nested lists (2+ levels) -- `nested-list`
- [x] inline markup inside table cells -- `inline-in-table`
- [x] inline markup inside list items -- `inline-in-list`

## Adversarial

- [x] empty document -- `adv-empty`
- [x] template / transclusion (parser must not crash or infinitely recurse) -- `adv-template`
- [x] unclosed bold/italic -- `adv-unclosed-bold`
- [x] unclosed `<ref>` -- `adv-unclosed-ref`
- [x] table with missing `|}` close -- `adv-table-unclosed`

## Pathological

- [x] deeply nested lists (5+ levels) -- `nested-list-deep`
- [x] very wide table (20+ columns) -- `wide-table`
- [x] heading containing inline markup -- `heading-inline-markup`
- [x] heading levels beyond h2 (stress test) -- `rare-heading-deep`
- [x] paragraph with many consecutive inline spans -- `many-inline-spans`

## Oracle

- [x] comprehensive sample -- `oracle`
