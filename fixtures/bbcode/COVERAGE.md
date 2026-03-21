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
- [ ] named/attributed quote ([quote=Author]) — (missing)
- [ ] horizontal rule ([hr]) — (missing)
- [ ] center alignment ([center]) — (missing)
- [ ] left/right alignment ([left] / [right]) — (missing)
- [ ] indented block ([indent]) — (missing)

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
- [ ] image with dimensions ([img=WxH]) — (missing)
- [ ] font size ([size=N]) — (missing)
- [ ] font name ([font=name]) — (missing)
- [ ] email link ([email]) — (missing)
- [ ] spoiler ([spoiler]) — (missing)
- [ ] preformatted inline ([pre]) — (missing)
- [ ] noparse / no bbcode ([noparse]) — (missing)

## List structure

- [ ] list item ([*]) — (covered via list-unordered/list-ordered but no dedicated fixture)
- [ ] nested list — (missing)
- [ ] list item with inline markup — (missing)

## Properties

- [x] color value on span — `span-color`
- [ ] url attribute on link — (covered in `link` but no dedicated property fixture)
- [ ] size attribute on font — (missing)
- [ ] language/type attribute on code block — (missing)
- [ ] named quote attribution — (missing)

## Composition (integration)

- [ ] bold inside list item — (missing)
- [ ] link inside table cell — (missing)
- [ ] color + bold combined — (missing)
- [ ] code block inside blockquote — (missing)
- [ ] nested lists — (missing)
- [ ] image inside link — (missing)

## Adversarial

- [x] empty document — `adv-empty`
- [ ] unclosed tag ([b] with no [/b]) — (missing)
- [ ] unknown/unrecognized tag ([foo]) — (missing)
- [ ] mismatched closing tag ([b]...[/i]) — (missing)
- [ ] tag with no content — (missing)
- [ ] stray closing tag ([/b] with no open) — (missing)
- [ ] deeply nested unclosed tags — (missing)

## Pathological

- [ ] very long line (>64 KB) — (missing)
- [ ] deeply nested formatting tags — (missing)
- [ ] very large table — (missing)
- [ ] list with hundreds of items — (missing)
