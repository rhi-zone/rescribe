# Native (rescribe-native) Fixture Coverage

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

The `native` format is rescribe's own JSON serialization of the IR. Every node kind defined
in `rescribe-std` should round-trip losslessly through native JSON. Coverage = one fixture
per node kind.

## Block node kinds

- [x] document — `adv-empty` (empty document), all fixtures (root)
- [x] paragraph — `paragraph`
- [x] heading — `heading`
- [x] code_block — `code-block`
- [x] blockquote — `blockquote`
- [x] list (unordered) — `list-unordered`
- [x] list (ordered) — `list-ordered`
- [x] list_item — `list-unordered`, `list-ordered`
- [x] table — `table`
- [x] table_row — `table`
- [x] table_cell — `table`
- [ ] table_header — (missing; `table` fixture only shows table_cell)
- [x] horizontal_rule — `horizontal-rule`
- [x] div — `nested`
- [x] raw_block — `raw-block`
- [x] definition_list — `definition-list`
- [x] definition_term — `definition-list`
- [x] definition_desc — `definition-list`
- [ ] figure — (missing)
- [ ] line_block — (missing)

## Inline node kinds

- [x] text — `paragraph`
- [x] emphasis — `emphasis`
- [x] strong — `strong`
- [x] strikeout — `strikeout`
- [x] underline — `underline`
- [x] subscript — `subscript`
- [x] superscript — `superscript`
- [x] code (inline) — `code-inline`
- [x] link — `link`
- [x] image — `image`
- [x] line_break — `line-break`
- [x] soft_break — `soft-break`
- [x] raw_inline — `raw-inline`
- [x] footnote_def — `footnote`
- [ ] footnote_ref — (missing; `footnote` only tests footnote_def)
- [ ] span — (missing)
- [ ] math_inline — (missing)
- [ ] math_block — (missing)
- [ ] cite — (missing)
- [ ] quoted — (missing)
- [ ] small_caps — (missing)

## Properties

### Heading
- [x] level prop on heading — `heading`

### Code block
- [x] content prop on code_block — `code-block`
- [x] language prop on code_block — `code-block`

### List
- [x] ordered prop on list (false) — `list-unordered`
- [x] ordered prop on list (true) — `list-ordered`

### Link
- [x] url prop on link — `link`

### Image
- [x] url prop on image — `image`
- [x] alt prop on image — `image`

### Raw nodes
- [x] format prop on raw_block — `raw-block`
- [x] format prop on raw_inline — `raw-inline`

### Text
- [x] content prop on text — `paragraph`

### Code inline
- [x] content prop on code — `code-inline`

## Composition (integration)

- [x] paragraph inside div — `nested`
- [x] text inside various inline nodes — `emphasis`, `strong`, etc.
- [ ] inline markup inside list item — (missing)
- [ ] table with table_header and table_cell rows — (missing)
- [ ] footnote_ref + footnote_def pair — (missing)
- [ ] nested lists — (missing)

## Adversarial

- [x] empty document — `adv-empty`
- [ ] node with unknown kind (forward compat) — (missing)
- [ ] node with extra unknown properties — (missing)
- [ ] malformed JSON — (missing)
- [ ] children array missing — (missing)

## Pathological

- [ ] document with 10,000 paragraphs — (missing)
- [ ] 100-level deep nesting — (missing)
- [ ] paragraph with 10,000 inline nodes — (missing)
