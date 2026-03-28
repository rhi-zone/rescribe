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
- [x] notextile block (notextile.) — `notextile-block`
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
- [x] citation (??text??) — `citation`
- [x] acronym (ABC(title)) — `acronym`
- [x] footnote reference ([1]) — `footnote`
- [x] deleted text (-text-) — covered by `rare-strikeout` (same syntax as strikethrough)
- [x] inserted text (+text+) — covered by `rare-underline` (same syntax as underline)
- [x] span (%text%) — `span`
- [x] notextile inline (==raw==) — `notextile-inline`
- [x] line break (newline within paragraph) — `line-break`
- [x] em dash (--) — not applicable: Textile transforms `--` to `&mdash;` at render time; lossless library keeps as-is
- [x] en dash (-) — not applicable: same as em dash; text passthrough preserves original
- [x] ellipsis (...) — not applicable: Textile renders `...` to `&hellip;`; library keeps literal text
- [x] typographic quotes — not applicable: transform at render time; library is lossless, keeps literal chars
- [x] dimension sign (xDIMx) — not applicable: render-time transform; library keeps literal text
- [x] registered/trademark/copyright symbols — not applicable: render-time HTML entities; library keeps literal

## Properties
- [x] block attributes (class, id, style, lang — p(class).) — `block-attrs`
- [ ] inline span attributes (%{style}text%) — (missing: attribute parsing inside %)
- [ ] table column alignment — (missing)
- [ ] table row attributes — (missing)
- [x] table header row (|_. header|) — `table`
- [x] table cell alignment (|<. >. =. <>.|) — `table-cell-align`
- [x] image alt text (!url(alt)!) — `image`
- [x] image dimensions — not applicable: non-standard extension; standard Textile uses {style} attributes
- [x] link title ("label(title)":url) — `link-title`
- [ ] list item continuation — (missing)
- [x] code block language (bc(lang).) — `code-block-lang`
- [x] paragraph alignment (p<. p>. p=. p<>.) — `paragraph-align`
- [x] indentation (p(. p).) — covered by `block-attrs` (parsed as indent_left/indent_right props)

## Composition (integration)
- [ ] nested blockquotes — (missing)
- [x] table with inline formatting in cells — `table-inline`
- [x] list item containing inline code — `list-with-code`
- [x] nested lists (unordered inside ordered) — `list-nested`, `nested-list-mixed`
- [x] heading with inline formatting — `heading-inline`
- [x] link containing formatted text — `link-formatted`

## Adversarial
- [x] empty document — `adv-empty`
- [x] unmatched markup delimiter — `adv-unmatched`
- [x] malformed link syntax — `adv-malformed-link` (parsed as plain text)
- [x] unclosed span — `adv-unclosed-span` (treated as plain text, no panic)
- [x] block type with invalid attribute — `adv-invalid-block-attr` (h7., h0. treated as paragraphs)
- [x] deeply nested inline markup — `adv-deeply-nested` (parsed without panic)

## Pathological
- [x] document with hundreds of sections — `path-many-sections` (50 sections)
- [x] very large table — `path-large-table` (20×10)
- [x] deeply nested lists — `path-deeply-nested-lists`
- [x] very long paragraph — `path-very-long-paragraph`
- [x] many footnotes — `path-many-footnotes` (20 refs + 20 defs)
