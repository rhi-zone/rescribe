# DOCX Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

## Block constructs
- [x] paragraph — `paragraph`
- [x] heading — `heading`
- [ ] heading levels 1–6 individually — (missing; `heading` covers only one level)
- [x] unordered list — `list`
- [x] ordered list — `list_ordered`
- [ ] nested list — (missing)
- [ ] definition list — (missing)
- [x] table — `table`
- [x] table with header row — `table_header`
- [ ] table with colspan/rowspan (gridSpan/vMerge) — (missing)
- [ ] table with borders/shading — (missing)
- [ ] code block (monospace paragraph style) — (missing)
- [ ] blockquote (indented paragraph style) — (missing)
- [ ] horizontal rule (`<w:p><w:pBdr><w:bottom>`) — (missing)
- [ ] text box / frame (`<w:txbxContent>`) — (missing)
- [ ] SDT (structured document tag / content control) — (missing)

## Inline constructs
- [x] bold (`<w:b>`) — `inline_bold`
- [x] italic (`<w:i>`) — (covered by inline formatting fixtures)
- [x] underline (`<w:u>`) — `inline_underline`
- [x] strikeout (`<w:strike>`) — `inline_strikeout`
- [x] subscript (`<w:vertAlign w:val="subscript">`) — `inline_subscript`
- [x] superscript (`<w:vertAlign w:val="superscript">`) — `inline_superscript`
- [x] small caps (`<w:smallCaps>`) — `inline_small_caps`
- [x] all caps (`<w:caps>`) — `inline_all_caps`
- [x] hidden text (`<w:vanish>`) — `inline_hidden`
- [x] highlight (`<w:highlight>`) — `inline_highlight`
- [x] font color (`<w:color>`) — `inline_color`
- [x] font size (`<w:sz>`) — `inline_font_size`
- [x] hyperlink — `hyperlink`
- [x] footnote reference — `footnote`
- [x] endnote reference — `endnote`
- [x] image (inline `<w:drawing>`) — `image`
- [ ] inline code (monospace run style) — (missing)
- [ ] line break (`<w:br w:type="textWrapping">`) — (missing)
- [ ] page break (`<w:br w:type="page">`) — (missing)
- [ ] column break (`<w:br w:type="column">`) — (missing)
- [ ] tab stop (`<w:tab>`) — (missing)
- [ ] field code (`<w:fldChar>`/`<w:instrText>`) — (missing)
- [ ] bookmark (`<w:bookmarkStart>`/`<w:bookmarkEnd>`) — (missing)
- [ ] comment reference (`<w:commentReference>`) — (missing)
- [ ] revision marks (tracked changes: ins/del) — (missing)
- [ ] font name (`<w:rFonts>`) — (missing)
- [ ] language (`<w:lang>`) — (missing)

## Paragraph properties
- [x] paragraph alignment (left/center/right/justify) — `alignment`
- [x] paragraph indent — `para_indent`
- [x] paragraph spacing (before/after) — `para_spacing`
- [ ] paragraph border — (missing)
- [ ] paragraph shading — (missing)
- [ ] paragraph style (`<w:pStyle>`) — (missing)
- [ ] keep-together / keep-with-next — (missing)
- [ ] page break before — (missing)
- [ ] outline level — (missing)
- [ ] numbering properties (separate from list fixture) — (missing)

## Document properties / Metadata
- [ ] core properties (title, author, description, created, modified) — (missing)
- [ ] custom properties — (missing)
- [ ] document language (`<w:defaultTabStop>`, `<w:lang>`) — (missing)
- [ ] page size and margins — (missing)
- [ ] section properties (`<w:sectPr>`) — (missing)
- [ ] theme fonts and colors — (missing)
- [ ] styles.xml named styles — (missing)

## Composition (integration)
- [ ] table cells with formatted runs — (missing)
- [ ] list items with inline formatting — (missing)
- [ ] footnote with formatted content — (missing)
- [ ] image with caption (figure style) — (missing)
- [ ] heading with inline formatting — (missing)
- [ ] hyperlink containing formatted text — (missing)
- [ ] table of contents (TOC field code) — (missing)

## Adversarial
- [ ] malformed zip archive — (missing)
- [ ] missing word/document.xml — (missing)
- [ ] corrupt relationship file (_rels/.rels) — (missing)
- [ ] unknown XML namespace — (missing)
- [ ] empty document (no paragraphs) — (missing)
- [ ] corrupt image binary in media/ — (missing)
- [ ] circular relationship references — (missing)
- [ ] extremely long style names — (missing)

## Pathological
- [ ] document with thousands of paragraphs — (missing)
- [ ] deeply nested tables (table inside table cell) — (missing)
- [ ] list with 20+ nesting levels — (missing)
- [ ] paragraph with hundreds of runs — (missing)
- [ ] very large embedded image — (missing)
- [ ] document with hundreds of footnotes — (missing)
