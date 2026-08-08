# DOCX Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

**Coverage-completeness caveat (2026-07-28):** the checklist below is a hand-curated list of
constructs, not yet verified against a spec-derived, machine-readable construct index. An
audit of `fixtures/docbook/COVERAGE.md` and `fixtures/jats/COVERAGE.md` against authoritative
element indexes found hundreds of element names enumerated nowhere, moving denominators
mid-session purely from incidentally-noticed gaps -- a ratio over a hand-written list like this
one is not a coverage measurement. See `docs/format-audit.md`'s "Construct Coverage (CC)"
section for the full evidence; this format's `CC` status there is `U` (unverified) until a
construct registry (in design, see `docs/adr/`) checks this list against the format's own
spec.

## Block constructs
- [x] paragraph — `paragraph`
- [x] heading — `heading`
- [x] heading levels 1–6 individually — `heading_levels`
- [x] unordered list — `list`
- [x] ordered list — `list_ordered`
- [ ] nested list — (missing)
- [ ] definition list — (missing)
- [x] table — `table`
- [x] table with header row — `table_header`
- [x] table with colspan/rowspan (gridSpan/vMerge) — `table_colspan`, `table_rowspan`
- [x] table with borders/shading — `table_borders`, `table_shading`
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
- [x] line break (`<w:br w:type="textWrapping">`) — `inline_line_break`
- [x] page break (`<w:br w:type="page">`) — `inline_page_break`
- [x] column break (`<w:br w:type="column">`) — `inline_column_break`
- [x] tab stop (`<w:tab>`) — `inline_tab_stop`
- [ ] field code (`<w:fldChar>`/`<w:instrText>`) — (missing)
- [x] bookmark (`<w:bookmarkStart>`/`<w:bookmarkEnd>`) — `inline_bookmark` (raw-preserved as `raw_inline` markers; previously silently dropped)
- [x] comment reference (`<w:commentReference>`) — `inline_comment_reference` (raw-preserved as `raw_inline` markers, including `commentRangeStart`/`End`, which were also previously silently dropped; the referenced comment body in comments.xml is not resolved/inlined)
- [x] revision marks (tracked changes: ins/del) — `revision_ins`, `revision_del` (content wrapped in a `span` with `docx:tracked-change`; nested inline formatting *inside* a tracked change is flattened to plain text on write — see write.rs `write_tracked_change_to_para` doc comment)
- [x] font name (`<w:rFonts>`) — `inline_font_name`
- [x] language (`<w:lang>`) — `inline_language`

## Paragraph properties
- [x] paragraph alignment (left/center/right/justify) — `alignment`
- [x] paragraph indent — `para_indent`
- [x] paragraph spacing (before/after) — `para_spacing`
- [x] paragraph border — `para_border`
- [x] paragraph shading — `para_shading`
- [x] paragraph style (`<w:pStyle>`) — `para_style`
- [x] keep-together / keep-with-next — `para_keep`
- [x] page break before — `para_page_break_before`
- [x] outline level — covered via `heading_levels`/`heading` (outlineLvl already drives heading-level detection in `detect_heading_level`; there is no separate non-heading outline-level fixture since DOCX's own semantics tie outline level to heading style)
- [ ] numbering properties (separate from list fixture) — (missing)

## Document properties / Metadata
- [x] core properties (title, author, description, created, modified) — `doc_core_properties`
- [ ] custom properties — (deferred: DOCX custom properties live in `docProps/custom.xml`, which uses the `vt:` variant-type schema (`vt:lpwstr`, `vt:i4`, `vt:filetime`, `vt:bool`, ...). ooxml-wml has no reader/writer for this part at all yet -- this is new parser/writer work in ooxml-wml, not a bridge wiring gap. Out of scope for this pass.)
- [x] document language (`<w:lang>`) — `doc_language` (styles.xml `docDefaults/rPrDefault/rPr/lang` maps to the semantic `language` document-metadata field); `<w:defaultTabStop>` itself (a numeric layout default, not a language construct despite being grouped with it in this checklist item) is not separately covered -- deferred, low value
- [x] page size and margins — `doc_page_layout`
- [x] section properties (`<w:sectPr>`) — `doc_page_layout` (page size/margins only; header/footer references, columns, and other `sectPr` children are not yet bridged)
- [ ] theme fonts and colors — (deferred — full theme1.xml color/font-scheme extraction is a multi-day effort involving a new ooxml-dml theme parser; out of scope for this pass)
- [ ] styles.xml named styles — (deferred — full named-style resolution (style inheritance chains, `basedOn`, style categories) is large; `StyleContext`/`RunPropertiesExt` in ooxml-wml already support *reading* resolved run properties through the style chain, but the bridge doesn't yet expose named styles as IR constructs beyond the existing `docx:pStyle` raw string. Out of scope for this pass.)

## Composition (integration)
- [x] table cells with formatted runs — `integration_table_formatted_runs`
- [x] list items with inline formatting — `integration_list_formatted`
- [x] footnote with formatted content — `integration_footnote_formatted`
- [ ] image with caption (figure style) — (deferred: DOCX has no native figure/caption construct -- it's conventionally an image run followed by a "Caption"-styled paragraph, or a SEQ field. Grouping those two adjacent block nodes into a single `figure`/`caption` pair is a heuristic the bridge doesn't implement yet; doing it without a real construct to key off risks silently mis-grouping unrelated adjacent content.)
- [x] heading with inline formatting — `integration_heading_formatted`
- [x] hyperlink containing formatted text — `integration_hyperlink_formatted`
- [ ] table of contents (TOC field code) — (deferred: depends on field code support, which is itself unimplemented — see the "field code" inline-construct item)

## Adversarial
- [x] malformed zip archive — `adv-malformed-zip`
- [x] missing word/document.xml — `adv-missing-document-xml`
- [x] corrupt relationship file (_rels/.rels) — `adv-corrupt-rels`
- [x] unknown XML namespace — `adv-unknown-namespace`
- [x] empty document (no paragraphs) — `adv-empty-document`
- [x] corrupt image binary in media/ — `adv-corrupt-image`
- [x] circular relationship references — `adv-circular-relationships`
- [x] extremely long style names — `adv-long-style-name`

## Pathological
- [x] document with thousands of paragraphs — `pathological_many_paragraphs` (5000)
- [x] deeply nested tables (table inside table cell) — `pathological_nested_table` (fixed a real gap: `convert_table`'s cell handling only walked `cell.paragraphs()`, silently dropping any nested `<w:tbl>` in a cell; now walks `cell.block_content` directly and recurses. Writer gained a matching `write_table_into` split so a nested `table` IR node can be written into a cell without a `&mut DocumentBuilder`.)
- [x] list with 20+ nesting levels — `pathological_deep_list_nesting` (25 levels; note the fixture's own description flags that ilvl-based nesting is the pre-existing "nested list" gap above -- all levels currently flatten into one list)
- [x] paragraph with hundreds of runs — `pathological_many_runs` (500)
- [x] very large embedded image — `pathological_large_image` (8 MiB)
- [x] document with hundreds of footnotes — `pathological_many_footnotes` (300)
