# ODT Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

## Block constructs
- [x] paragraph (`<text:p>`) — `paragraph`
- [x] heading (`<text:h>`) — `heading`
- [ ] heading levels 1–6 individually — (missing; `heading` may cover only one level)
- [x] unordered list (`<text:list>` with bullet style) — `list`
- [ ] ordered list (`<text:list>` with number style) — (missing)
- [ ] nested list — (missing)
- [ ] table (`<table:table>`) — (missing)
- [ ] table with header row — (missing)
- [ ] table with colspan/rowspan — (missing)
- [ ] code block (preformatted paragraph style) — (missing)
- [ ] blockquote (Quotations paragraph style) — (missing)
- [ ] horizontal rule (paragraph border or draw:line) — (missing)
- [ ] text box / frame (`<draw:text-box>`) — (missing)
- [ ] definition list (no native ODF construct; style-based) — (missing)

## Inline constructs
- [x] line break (`<text:line-break>`) — `line-break`
- [ ] bold (`fo:font-weight="bold"`) — (missing)
- [ ] italic (`fo:font-style="italic"`) — (missing)
- [ ] underline (`style:text-underline-style`) — (missing)
- [ ] strikeout (`style:text-line-through-style`) — (missing)
- [ ] subscript (`style:text-position`) — (missing)
- [ ] superscript (`style:text-position`) — (missing)
- [ ] small caps (`fo:font-variant="small-caps"`) — (missing)
- [ ] font color (`fo:color`) — (missing)
- [ ] font size (`fo:font-size`) — (missing)
- [ ] font name (`fo:font-family`) — (missing)
- [ ] hyperlink (`<text:a>`) — (missing)
- [ ] footnote (`<text:footnote>`) — (missing)
- [ ] endnote (`<text:endnote>`) — (missing)
- [ ] image / frame (`<draw:frame><draw:image>`) — (missing)
- [ ] bookmark (`<text:bookmark>`) — (missing)
- [ ] annotation / comment (`<office:annotation>`) — (missing)
- [ ] tab stop (`<text:tab>`) — (missing)
- [ ] soft hyphen (`<text:soft-hyphen>`) — (missing)
- [ ] non-breaking space (`<text:s>` / `&#160;`) — (missing)

## Paragraph properties
- [ ] paragraph alignment (`fo:text-align`) — (missing)
- [ ] paragraph indent (`fo:margin-left`, `fo:text-indent`) — (missing)
- [ ] paragraph spacing (`fo:margin-top`, `fo:margin-bottom`) — (missing)
- [ ] paragraph style name (`text:style-name`) — (missing)
- [ ] paragraph border (`fo:border`) — (missing)
- [ ] paragraph background color — (missing)
- [ ] line height — (missing)
- [ ] keep-together / keep-with-next — (missing)

## Document metadata
- [ ] title (`<dc:title>`) — (missing)
- [ ] author (`<dc:creator>`) — (missing)
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
