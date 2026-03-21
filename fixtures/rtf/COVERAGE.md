# RTF Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

RTF reference: Microsoft RTF Specification 1.9.1.
RTF format crate is at 5-Production; this checklist validates full construct coverage.

## Block constructs

- [x] paragraph — `paragraph` (`\par`)
- [x] multiple paragraphs — `multiple_paragraphs`
- [x] heading — `heading` (`\outlinelevel0`, `\outlinelevel1`, etc.)
- [x] unordered list — `list-bullet` (`{\*\pn\pnlvlblt…}`)
- [x] ordered list — `list-ordered` (`{\*\pn\pnlvlbody…}`)
- [x] table (simple) — `table-simple` (`\trowd` / `\cellx` / `\intbl` / `\cell` / `\row`)
- [x] table with formatted cells — `table-formatted-cells`
- [x] line break (within paragraph) — `line_break` (`\line`)
- [ ] code block / preformatted — (missing; typically `\f{courier}` + `\pard` block with monospace font, no single control word)
- [ ] blockquote — (missing; typically `\li` indented paragraph; no single control word)
- [ ] section page break — (missing; `\page` or `\sect` / `\sectd`)
- [ ] column break — (missing; `\column`)
- [ ] multi-column layout — (missing; `\cols`, `\colsx`, `\colno`)
- [ ] header / footer — (missing; `{\header …}`, `{\footer …}`, `{\headerf …}`, `{\footerf …}`)
- [ ] list using `{\*\listtable}` — (missing; Word 97+ list table via `\ls` / `\listid` rather than `\pn`)
- [ ] nested list — (missing; multi-level `\ilvl` or nested `\pn` groups)

## Inline constructs

- [x] bold — `bold`, `mixed_bold`, `adjacent_bold`, `nested_bold_italic`, `nested_bold_underline`
- [x] italic — `italic`, `mixed_italic`, `nested_bold_italic`
- [x] underline — `underline`, `mixed_underline`, `nested_bold_underline`
- [x] strikethrough — `strikethrough` (`\strike`)
- [x] subscript — `subscript` (`\sub`)
- [x] superscript — `superscript` (`\super`)
- [x] small caps — `small_caps` (`\scaps`)
- [x] all caps — `all_caps` (`\caps`)
- [x] hidden text — `hidden` (`\v`)
- [x] color (foreground) — `color`, `color_font_size` (`\cf` + `\colortbl`)
- [x] background color — `background-color` (`\cb` + `\colortbl`)
- [x] font face — `font-face` (`\f` + `\fonttbl`)
- [x] font size — `font_size`, `color_font_size` (`\fs`)
- [x] language — `language` (`\lang` + LCID)
- [x] footnote — `footnote` (`{\footnote …}`)
- [x] special characters — `special_chars` (`\emdash`, `\endash`, `\lquote`, `\rquote`, `\ldblquote`, `\rdblquote`)
- [x] character properties (raw) — `char_props` (`\dn` lowered baseline; `rtf:char-props`)
- [ ] double strikethrough — (missing; `\strikedl`)
- [ ] outline — (missing; `\outl`)
- [ ] shadow — (missing; `\shad`)
- [ ] emboss / engrave — (missing; `\embo`, `\impr`)
- [ ] animated text — (missing; `\animtext`)
- [ ] hyperlink — (missing; `{\field{\*\fldinst HYPERLINK "…"}{\fldrslt …}}`)
- [ ] field (general) — (missing; `{\field{\*\fldinst …}{\fldrslt …}}`)
- [ ] bookmark — (missing; `{\*\bkmkstart …}` / `{\*\bkmkend …}`)
- [ ] endnote — (missing; `{\*\footnote\ftnalt …}`)
- [ ] comment / annotation — (missing; `{\*\annotation …}`)
- [ ] revision mark (tracked change) — (missing; `\revised`, `\deleted`, `\revtbl`)
- [ ] double underline — (missing; `\uld`)
- [ ] word underline — (missing; `\ulw`)
- [ ] dotted underline — (missing; `\uld`)
- [ ] colored underline — (missing; `\ulc`)
- [ ] superscript (alternative `\up`) — (missing; `\up6` half-point raise, distinct from `\super`)
- [ ] subscript (alternative `\dn`) — (missing; `\dn6` half-point lower, distinct from `\sub`)

## Properties

- [x] paragraph alignment — `alignment` (`\ql`, `\qc`, `\qr`, `\qj`)
- [x] paragraph indents / raw para-props — `para_props` (`\li`, `\ri`, `\fi`; stored as `rtf:para-props`)
- [x] code page — `codepage-1250` (`\ansicpg`)
- [ ] paragraph spacing (`\sb`, `\sa`) — (missing as a distinct fixture; `\sa180` appears in `paragraph` / `heading` but not tested as a semantic construct)
- [ ] tab stops (`\tx`) — (missing; `rtf:para-props` stores them raw but no dedicated fixture)
- [ ] border on paragraph — (missing; `\brdrt`, `\brdrb`, `\brdrl`, `\brdrr` in para-props)
- [ ] page size / margins (document info) — (missing; `\paperw`, `\paperh`, `\margl`, `\margr`, `\margt`, `\margb`)
- [ ] default font — (missing; `\deff` in document header)
- [ ] info block (metadata) — (missing; `{\info {\title …} {\author …} {\creatim …}}`)
- [ ] stylesheet — (missing; `{\stylesheet {\cs … style name;}}` — paragraph/character style names)
- [ ] table cell width / alignment — (missing; `\cellx` widths covered in table fixtures but alignment/padding not tested)
- [ ] table row height — (missing; `\trrh`)
- [ ] table cell vertical alignment — (missing; `\clvertalc`, `\clvertalt`, `\clvertalb`)
- [ ] table cell border — (missing; `\clbrdrt`, `\clbrdrb`, `\clbrdrl`, `\clbrdrr`)
- [ ] table cell background color — (missing; `\clcbpat`)
- [ ] table nested (Word 97+ `\*\nesttableprops`) — (missing)
- [ ] Unicode escape (`\u` + fallback) — (missing; `\u8364?€` pattern)
- [ ] picture (`\pict`) — (missing; embedded image data)
- [ ] object (`\object`) — (missing; OLE object)

## Composition (integration)

- [x] mixed bold + normal text — `mixed_bold`
- [x] mixed italic + normal text — `mixed_italic`
- [x] mixed underline + normal text — `mixed_underline`
- [x] bold + italic nested — `nested_bold_italic`
- [x] bold + underline nested — `nested_bold_underline`
- [x] color + font size combined — `color_font_size`
- [ ] table with footnote — (missing)
- [ ] list with bold items — (missing)
- [ ] heading levels (h1 through h6) — (missing; only `\outlinelevel0` tested in `heading`)
- [ ] paragraph with hyperlink — (missing)
- [ ] document with info block + body — (missing)
- [ ] multiple font faces in one document — (missing; `font-face` tests two fonts but both in same para)

## Adversarial

- [x] empty document — `adv-empty`
- [x] truncated input — `adv-truncated`
- [x] binary / non-RTF input — `adv-binary`
- [ ] unknown control word — (missing; `\xyzzy123` reader must skip gracefully)
- [ ] deeply nested groups — (missing; `{{{{{…}}}}}` many levels)
- [ ] unmatched closing brace — (missing; extra `}` at end)
- [ ] unmatched opening brace — (missing; missing closing `}`)
- [ ] `\bin` binary data — (missing; `\binN` binary blob skip)
- [ ] invalid code page (`\ansicpg9999`) — (missing)
- [ ] `\uN` with no fallback character — (missing)
- [ ] `\fonttbl` with duplicate font indices — (missing)
- [ ] `\colortbl` with out-of-range `\cf` index — (missing)
- [ ] `\pict` with unsupported image type — (missing; graceful skip)

## Pathological

- [ ] very large table — (missing; table with many rows and columns)
- [ ] deeply nested list (5+ levels) — (missing)
- [ ] document with many footnotes — (missing)
- [ ] long document (many paragraphs) — (missing; performance / memory)
- [ ] heavily formatted paragraph (many inline spans) — (missing)
- [ ] fonttbl with many fonts — (missing; stress test font index lookup)
- [ ] colortbl with many colors — (missing)
