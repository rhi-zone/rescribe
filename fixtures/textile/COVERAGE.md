# Textile Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

## Block constructs
- [x] paragraph — `paragraph`
- [x] heading h1 (h1.) — `heading`
- [x] heading h2 (h2.) — `heading-h2`
- [ ] heading h3–h6 — (missing)
- [x] unordered list (* item) — `list-unordered`
- [x] ordered list (# item) — `list-ordered`
- [x] nested list — `list-nested`
- [x] blockquote (bq.) — `rare-blockquote`
- [x] code block (bc.) — `code-block`
- [x] table — `table`
- [ ] horizontal rule (---) — (missing)
- [ ] definition list (- term := definition) — (missing)
- [ ] pre block (pre.) — (missing)
- [ ] notextile block (notextile.) — (missing)
- [ ] extended block (p.. or bq..) — (missing)
- [ ] footnote definition (fn1.) — (missing)

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
- [ ] footnote reference ([1]) — (missing)
- [ ] deleted text (-text-) — (missing; overlaps strikethrough)
- [ ] inserted text (+text+) — (missing; overlaps underline)
- [ ] span (% attributes %text%) — (missing)
- [ ] notextile inline (== raw ==) — (missing)
- [ ] line break (newline within paragraph) — (missing)
- [ ] em dash (--) — (missing)
- [ ] en dash (-) — (missing)
- [ ] ellipsis (...) — (missing)
- [ ] typographic quotes — (missing)
- [ ] dimension sign (xDIMx) — (missing)
- [ ] registered/trademark/copyright symbols — (missing)

## Properties
- [ ] block attributes (class, id, style, lang — p(class).) — (missing)
- [ ] inline span attributes — (missing)
- [ ] table column alignment — (missing)
- [ ] table row attributes — (missing)
- [ ] table header row (|_. header|) — (missing)
- [ ] table cell alignment and padding — (missing)
- [ ] image alt text (!url(alt)!) — (missing)
- [ ] image dimensions (!url width!) — (missing)
- [ ] link title ("label(title)":url) — (missing)
- [ ] list item continuation — (missing)
- [ ] code block language — (missing)
- [ ] paragraph alignment (p<. p>. p=. p<>.) — (missing)
- [ ] indentation (p(. p).) — (missing)

## Composition (integration)
- [ ] nested blockquotes — (missing)
- [ ] table with inline formatting in cells — (missing)
- [ ] list item containing a code block — (missing)
- [ ] nested lists (unordered inside ordered) — `list-nested`
- [ ] heading with inline formatting — (missing)
- [ ] link containing formatted text — (missing)

## Adversarial
- [x] empty document — `adv-empty`
- [x] unmatched markup delimiter — `adv-unmatched`
- [ ] malformed link syntax — (missing)
- [ ] unclosed span — (missing)
- [ ] block type with invalid attribute — (missing)
- [ ] deeply nested inline markup — (missing)

## Pathological
- [ ] document with hundreds of sections — (missing)
- [ ] very large table — (missing)
- [ ] deeply nested lists — (missing)
- [ ] very long paragraph — (missing)
- [ ] many footnotes — (missing)
