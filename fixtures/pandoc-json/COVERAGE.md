# Pandoc JSON Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

Pandoc JSON is defined by Pandoc's internal AST serialization format. The top-level object
has `pandoc-api-version`, `meta`, and `blocks` fields. Block and inline elements mirror
Pandoc's Haskell ADT. Reference: https://hackage.haskell.org/package/pandoc-types

## Block elements (Pandoc AST → rescribe IR)

- [x] Para → paragraph — `paragraph`
- [x] Header → heading — `heading`
- [x] CodeBlock → code_block — `code-block`
- [x] BlockQuote → blockquote — `blockquote`
- [x] BulletList → list (unordered) — `list-unordered`
- [x] OrderedList → list (ordered) — `list-ordered`
- [x] Table → table — `table`
- [x] HorizontalRule → horizontal_rule — `horizontal-rule`
- [x] Div → div — `div`
- [x] RawBlock → raw_block — `raw-block`
- [x] DefinitionList → definition_list — `definition-list`
- [x] LineBlock → paragraph with line_break — `line-block`
- [ ] Plain (unwrapped block) → paragraph — (missing)
- [ ] Figure → figure — (missing)
- [ ] Null block — (missing)

## Inline elements (Pandoc AST → rescribe IR)

- [x] Str → text — `paragraph`
- [x] Emph → emphasis — `italic`
- [x] Strong → strong — `bold`
- [x] Strikeout → strikeout — `strikeout`
- [x] Underline → underline — `underline`
- [x] Subscript → subscript — `subscript`
- [x] Superscript → superscript — `superscript`
- [x] Code → code (inline) — `code-inline`
- [x] Link → link — `link`
- [x] Image → image — `image`
- [x] LineBreak → line_break — `line-break`
- [x] SoftBreak → soft_break — `soft-break`
- [x] RawInline → raw_inline — `raw-inline`
- [x] Cite → cite — `cite`
- [x] Quoted → quoted — `quoted`
- [x] SmallCaps → small_caps — `small-caps`
- [x] Span → span — `span`
- [x] Math (InlineMath) → math_inline — `math-inline`
- [x] Math (DisplayMath) → math_display — `math-display`
- [x] Note (footnote) → footnote_def — `footnote`
- [ ] Space → text with space — (missing)
- [ ] SoftBreak between words — (missing; `soft-break` tests isolated SoftBreak)

## Properties

### Heading
- [x] level (integer) — `heading`

### Code block
- [x] content — `code-block`
- [x] language — `code-block`
- [ ] id attribute — (missing)
- [ ] classes beyond language — (missing)
- [ ] key-value attributes — (missing)

### Link
- [x] url — `link`
- [x] title — `link`
- [ ] id and classes — (missing)

### Image
- [x] url — `image`
- [x] alt — `image`
- [x] title — `image`

### Div / Span
- [x] id — `div`, `span`
- [x] classes — `div`, `span`
- [ ] key-value attributes — (missing)

### Raw nodes
- [x] format — `raw-block`, `raw-inline`
- [x] content — `raw-block`, `raw-inline`

### List
- [x] ordered (true/false) — `list-ordered`, `list-unordered`
- [ ] list number style (Decimal, LowerAlpha, UpperAlpha, etc.) — (missing)
- [ ] list number delimiter (Period, OneParen, TwoParens) — (missing)
- [ ] list start number — (missing)

### Quoted
- [x] quote_type ("double") — `quoted`
- [ ] quote_type ("single") — (missing)

### Math
- [x] math:source — `math-inline`, `math-display`

### Table
- [x] table_row, table_header, table_cell structure — `table`
- [ ] table caption — (missing)
- [ ] column alignment — (missing)
- [ ] column width — (missing)
- [ ] multi-row table body — (missing)
- [ ] table head / foot separation — (missing)

## Metadata

- [ ] doc title in meta — (missing)
- [ ] doc author in meta — (missing)
- [ ] doc date in meta — (missing)
- [ ] arbitrary meta key — (missing)

## Pandoc API version

- [ ] pandoc-api-version field preserved/checked — (missing)

## Composition (integration)

- [ ] inline markup (bold, italic) inside list item — (missing)
- [ ] table with formatted cells — (missing)
- [ ] footnote inside paragraph — `footnote` (partially)
- [ ] nested lists — (missing)
- [ ] div containing table — (missing)

## Adversarial

- [x] empty document — `adv-empty`
- [ ] unknown block type — (missing)
- [ ] unknown inline type — (missing)
- [ ] malformed JSON — (missing)
- [ ] missing `blocks` field — (missing)
- [ ] missing `meta` field — (missing)

## Pathological

- [ ] document with 10,000 paragraphs — (missing)
- [ ] 100-level deep nesting — (missing)
- [ ] table with 1000 rows — (missing)
