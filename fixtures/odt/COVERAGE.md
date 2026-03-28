# ODT Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

## Block constructs
- [x] paragraph (`<text:p>`) — `paragraph`
- [x] heading (`<text:h>`) — `heading`
- [x] heading levels 1–6 individually — `heading-levels`
- [x] unordered list (`<text:list>` with bullet style) — `list`
- [x] ordered list (`<text:list>` with number style) — `ordered-list`
- [x] nested list — `nested-list`
- [x] table (`<table:table>`) — `table`
- [x] table with header row — `table-header`
- [ ] table with colspan/rowspan — (missing)
- [x] code block (preformatted paragraph style) — `code-block`
- [x] blockquote (Quotations paragraph style) — `blockquote`
- [ ] horizontal rule (paragraph border or draw:line) — (missing)
- [ ] text box / frame (`<draw:text-box>`) — (missing)
- [ ] definition list (no native ODF construct; style-based) — (missing)

## Inline constructs
- [x] line break (`<text:line-break>`) — `line-break`
- [x] bold (`fo:font-weight="bold"`) — `bold`
- [x] italic (`fo:font-style="italic"`) — `italic`
- [x] underline (`style:text-underline-style`) — `underline`
- [x] strikeout (`style:text-line-through-style`) — `strikeout`
- [x] subscript (`style:text-position`) — `subscript`
- [x] superscript (`style:text-position`) — `superscript`
- [x] small caps (`fo:font-variant="small-caps"`) — `small-caps`
- [x] font color (`fo:color`) — `font-color`
- [x] font size (`fo:font-size`) — `font-size`
- [x] font name (`fo:font-family`) — `font-name`
- [x] hyperlink (`<text:a>`) — `hyperlink`
- [ ] footnote (`<text:footnote>`) — (missing)
- [ ] endnote (`<text:endnote>`) — (missing)
- [ ] image / frame (`<draw:frame><draw:image>`) — (missing)
- [ ] bookmark (`<text:bookmark>`) — (missing)
- [ ] annotation / comment (`<office:annotation>`) — (missing)
- [x] tab stop (`<text:tab>`) — `tab`
- [x] soft hyphen (`<text:soft-hyphen>`) — `soft-hyphen`
- [x] non-breaking space (`&#160;`) — `non-breaking-space`

## Paragraph properties
- [x] paragraph alignment (`fo:text-align`) — `para-align`
- [x] paragraph indent (`fo:margin-left`, `fo:text-indent`) — `para-indent`
- [x] paragraph spacing (`fo:margin-top`, `fo:margin-bottom`) — `para-spacing`
- [ ] paragraph style name (`text:style-name`) — (missing)
- [x] paragraph border (`fo:border`) — `para-border`
- [x] paragraph background color — `para-background`
- [x] line height — `line-height`
- [x] keep-together / keep-with-next — `keep-together`

## Document metadata
- [x] title (`<dc:title>`) — `meta-title`
- [x] author (`<dc:creator>`) — `meta-author`
- [ ] description (`<dc:description>`) — (missing)
- [ ] creation/modification date — (missing)
- [ ] language (`<dc:language>`) — (missing)
- [ ] custom user-defined metadata — (missing)
- [ ] page size and margins (`<style:page-layout>`) — (missing)

## Composition (integration)
- [ ] table cells with formatted inline content — (missing)
- [ ] list items with inline formatting — (missing)
- [ ] footnote with formatted content — (missing)
- [ ] image with caption — (missing)
- [ ] heading with inline formatting — (missing)
- [ ] hyperlink containing formatted text — (missing)
- [ ] nested blockquote — (missing)

## Adversarial
- [ ] malformed zip archive — (missing)
- [ ] missing content.xml — (missing)
- [ ] corrupt styles.xml — (missing)
- [ ] unknown XML namespace — (missing)
- [ ] empty document — (missing)
- [ ] corrupt image binary — (missing)
- [ ] non-ODF zip (wrong mimetype) — (missing)

## Pathological
- [ ] document with thousands of paragraphs — (missing)
- [ ] deeply nested tables — (missing)
- [ ] list with many nesting levels — (missing)
- [ ] paragraph with hundreds of character runs — (missing)
- [ ] very large embedded image — (missing)
