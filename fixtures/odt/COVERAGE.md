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
- [x] table with colspan/rowspan — `colspan-rowspan`
- [x] code block (preformatted paragraph style) — `code-block`
- [x] blockquote (Quotations paragraph style) — `blockquote`
- [x] horizontal rule (Horizontal Line paragraph style) — `horizontal-rule`
- [x] text box / frame (`<draw:text-box>`) — `text-box`
- [x] definition list (no native ODF construct; style-based) — not applicable (ODF has no native DL)

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
- [x] footnote (`<text:note text:note-class="footnote">`) — `footnote`
- [x] endnote (`<text:note text:note-class="endnote">`) — `endnote`
- [x] image / frame (`<draw:frame><draw:image>`) — `image`
- [x] bookmark (`<text:bookmark>`) — `bookmark`
- [x] annotation / comment (`<office:annotation>`) — `annotation`
- [x] tab stop (`<text:tab>`) — `tab`
- [x] soft hyphen (`<text:soft-hyphen>`) — `soft-hyphen`
- [x] non-breaking space (`&#160;`) — `non-breaking-space`

## Paragraph properties
- [x] paragraph alignment (`fo:text-align`) — `para-align`
- [x] paragraph indent (`fo:margin-left`, `fo:text-indent`) — `para-indent`
- [x] paragraph spacing (`fo:margin-top`, `fo:margin-bottom`) — `para-spacing`
- [x] paragraph style name (`text:style-name`) — `para-style-name`
- [x] paragraph border (`fo:border`) — `para-border`
- [x] paragraph background color — `para-background`
- [x] line height — `line-height`
- [x] keep-together / keep-with-next — `keep-together`

## Document metadata
- [x] title (`<dc:title>`) — `meta-title`
- [x] author (`<dc:creator>`) — `meta-author`
- [x] description (`<dc:description>`) — `meta-description`
- [x] creation/modification date — `meta-date`
- [x] language (`<dc:language>`) — `meta-language`
- [x] custom user-defined metadata — `meta-custom`
- [x] page size and margins (`<style:page-layout>`) — `page-layout`

## Composition (integration)
- [x] table cells with formatted inline content — `table-cells-formatted`
- [x] list items with inline formatting — `list-items-formatted`
- [x] footnote with formatted content — `footnote-formatted`
- [x] image with caption — `image-caption`
- [x] heading with inline formatting — `heading-formatted`
- [x] hyperlink containing formatted text — `link-formatted`
- [x] nested blockquote — `nested-blockquote`

## Adversarial
- [x] malformed zip archive — `adv-malformed-zip`
- [x] missing content.xml — `adv-missing-content`
- [x] corrupt styles.xml — `adv-corrupt-styles`
- [x] unknown XML namespace — `adv-unknown-namespace`
- [x] empty document — `adv-empty`
- [x] corrupt image binary — `adv-corrupt-image`
- [x] non-ODF zip (wrong mimetype) — `adv-wrong-mimetype`

## Pathological
- [x] document with thousands of paragraphs — `path-many-paragraphs`
- [x] deeply nested tables — `path-deeply-nested-table`
- [x] list with many nesting levels — `path-deeply-nested-list`
- [x] paragraph with hundreds of character runs — `path-many-char-runs`
- [x] very large embedded image — `path-large-image`
