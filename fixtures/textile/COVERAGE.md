# Textile Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

## Block constructs
- [x] paragraph — `paragraph`
- [x] heading h1 (h1.) — `heading`
- [x] heading h2 (h2.) — `heading-h2`
- [x] heading h3–h6 — `heading-h3-h6`
- [x] unordered list (* item) — `list-unordered`
- [x] ordered list (# item) — `list-ordered`
- [x] nested list — `list-nested`
- [x] blockquote (bq.) — `rare-blockquote`
- [x] code block (bc.) — `code-block`
- [x] table — `table`
- [x] horizontal rule (---) — `horizontal-rule`
- [x] definition list (;term :definition) — `definition-list`
- [x] pre block (pre.) — `pre-block` (maps to code_block)
- [ ] notextile block (notextile.) — (missing)
- [ ] extended block (p.. or bq..) — (missing)
- [x] footnote definition (fn1.) — `footnote`

## Inline constructs
- [x] italic (_text_ or __text__) — `italic`
- [x] bold (*text* or **text**) — `bold`
- [x] inline code (@text@) — `code-inline`
- [x] link ("label":url) — `link`
- [x] image (!url!) — `image`
- [x] strikethrough (-text-) — `rare-strikeout`
- [x] underline (+text+) — `rare-underline`
- [x] subscript (~text~) — `subscript`
- [x] superscript (^text^) — `superscript`
- [ ] citation (??text??) — (missing)
- [ ] acronym (ABC(title)) — (missing)
- [x] footnote reference ([1]) — `footnote`
- [x] deleted text (-text-) — covered by `rare-strikeout` (same syntax as strikethrough)
- [x] inserted text (+text+) — covered by `rare-underline` (same syntax as underline)
- [ ] span (% attributes %text%) — (missing)
- [ ] notextile inline (== raw ==) — (missing)
- [ ] line break (newline within paragraph) — (missing)
- [ ] em dash (--) — not applicable (text passthrough; transform would break roundtrip)
- [ ] en dash (-) — not applicable (text passthrough; transform would break roundtrip)
- [ ] ellipsis (...) — not applicable (text passthrough; transform would break roundtrip)
- [ ] typographic quotes — not applicable (text passthrough; transform would break roundtrip)
- [ ] dimension sign (xDIMx) — not applicable (text passthrough)
- [ ] registered/trademark/copyright symbols — not applicable (text passthrough)

## Properties
- [ ] block attributes (class, id, style, lang — p(class).) — (missing)
- [ ] inline span attributes — (missing)
- [ ] table column alignment — (missing)
- [ ] table row attributes — (missing)
- [x] table header row (|_. header|) — `table`
- [ ] table cell alignment and padding — (missing)
- [x] image alt text (!url(alt)!) — `image`
- [ ] image dimensions (!url width!) — (missing)
- [ ] link title ("label(title)":url) — (missing)
- [ ] list item continuation — (missing)
- [ ] code block language — (missing)
- [ ] paragraph alignment (p<. p>. p=. p<>.) — (missing)
- [ ] indentation (p(. p).) — (missing)

## Composition (integration)
- [ ] nested blockquotes — (missing)
- [x] table with inline formatting in cells — `table-inline`
- [x] list item containing inline code — `list-with-code`
- [x] nested lists (unordered inside ordered) — `list-nested`, `nested-list-mixed`
- [x] heading with inline formatting — `heading-inline`
- [ ] link containing formatted text — (missing)

## Adversarial
- [x] empty document — `adv-empty`
- [x] unmatched markup delimiter — `adv-unmatched`
- [x] malformed link syntax — `adv-malformed-link` (parsed as plain text)
- [ ] unclosed span — (missing)
- [ ] block type with invalid attribute — (missing)
- [x] deeply nested inline markup — `adv-deeply-nested` (parsed without panic)

## Pathological
- [x] document with hundreds of sections — `path-many-sections` (50 sections)
- [x] very large table — `path-large-table` (20×10)
- [ ] deeply nested lists — (missing)
- [ ] very long paragraph — (missing)
- [x] many footnotes — `path-many-footnotes` (20 refs + 20 defs)
