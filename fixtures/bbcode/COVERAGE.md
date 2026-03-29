# BBCode Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

## Block constructs

- [x] paragraph (plain text block) — `paragraph`
- [x] unordered list ([list]) — `list-unordered`
- [x] ordered list ([list=1]) — `list-ordered`
- [x] code block ([code]) — `code-block`
- [x] table ([table] / [tr] / [td]) — `table`
- [x] table with header ([th]) — `table-header`
- [x] blockquote ([quote]) — `rare-blockquote`
- [x] named/attributed quote ([quote=Author]) — `quote-attributed`
- [x] horizontal rule ([hr]) — `horizontal-rule`
- [x] heading ([h1]–[h6]) — `heading`
- [x] center alignment ([center]) — `align-center`
- [x] left/right alignment ([left] / [right]) — `align-left-right`
- [x] indented block ([indent]) — `indent`
- [x] spoiler ([spoiler]) — `spoiler`
- [x] preformatted ([pre]) — `preformatted`

## Inline constructs

- [x] bold ([b]) — `bold`
- [x] italic ([i]) — `italic`
- [x] underline ([u]) — `underline`
- [x] strikethrough ([s]) — `rare-strikeout`
- [x] subscript ([sub]) — `subscript`
- [x] superscript ([sup]) — `superscript`
- [x] inline code ([icode] / [inlinecode]) — `rare-code-inline`
- [x] color ([color=...]) — `span-color`
- [x] link ([url]) — `link`
- [x] link with display text ([url=href]text[/url]) — `link`
- [x] image ([img]) — `image`
- [x] image with dimensions ([img=WxH]) — `image-dimensions`
- [x] font size ([size=N]) — `span-size`
- [x] font name ([font=name]) — `span-font`
- [x] email link ([email]) — `email`
- [x] noparse / no bbcode ([noparse]) — `noparse`

## List structure

- [x] list item ([*]) — `list-item`
- [x] nested list — `list-nested`
- [x] list item with inline markup — `list-item-markup`

## Properties

- [x] color value on span — `span-color`
- [x] url attribute on link — `link`
- [x] size attribute on font — `prop-size`
- [x] language/type attribute on code block — `prop-code-language`
- [x] named quote attribution — `quote-attributed`

## Composition (integration)

- [x] bold inside list item — `comp-bold-in-list`
- [x] link inside table cell — `comp-link-in-table`
- [x] color + bold combined — `comp-color-bold`
- [x] code block inside blockquote — `comp-code-in-quote`
- [x] nested lists — `comp-nested-lists`
- [x] image inside link — `comp-image-in-link`

## Adversarial

- [x] empty document — `adv-empty`
- [x] unclosed tag ([b] with no [/b]) — `adv-unclosed-tag`
- [x] unknown/unrecognized tag ([foo]) — `adv-unknown-tag`
- [x] mismatched closing tag ([b]...[/i]) — `adv-mismatched-tag`
- [x] tag with no content — `adv-empty-tag`
- [x] stray closing tag ([/b] with no open) — `adv-stray-close`
- [x] deeply nested unclosed tags — `adv-deeply-nested-unclosed`

## Pathological

- [x] very long line (>1 KB) — `path-long-line`
- [x] deeply nested formatting tags — `path-deeply-nested`
- [x] very large table — `path-large-table`
- [x] list with many items — `path-many-list-items`
