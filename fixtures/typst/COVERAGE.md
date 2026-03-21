# Typst Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

Reference: Typst documentation — https://typst.app/docs/

## Block constructs

- [x] paragraph — `paragraph`
- [x] heading level 1 (= Heading) — `heading`
- [x] heading level 2 (== Heading) — `heading-h2`
- [ ] heading level 3 (=== Heading) — (missing)
- [ ] heading level 4–6 — (missing)
- [x] unordered list (- item) — `list-unordered`
- [x] ordered list (+ item or 1. item) — `list-ordered`
- [x] definition / term list (/ term: desc) — `definition-list`
- [x] code block (``` ``` ```) — `code-block`
- [x] blockquote (#quote[...]) — `blockquote`
- [x] figure (#figure(...)) — `figure`
- [x] table (#table(...)) — `table`
- [x] display math ($$ ... $$) — `math-display`
- [x] raw block (#raw(...)) — `raw-block`
- [ ] grid (#grid(...)) — (missing)
- [ ] horizontal line (--- / #line) — (missing)
- [ ] bibliography (#bibliography) — (missing)
- [ ] outline / table of contents (#outline) — (missing)
- [ ] page break (#pagebreak) — (missing)

## Inline constructs

- [x] bold (*text*) — `bold`
- [x] italic (_text_) — `italic`
- [x] underline (#underline[...]) — `underline`
- [x] strikethrough (#strike[...]) — `strikeout`
- [x] subscript (#sub[...]) — `subscript`
- [x] superscript (#super[...]) — `superscript`
- [x] inline code (`code`) — `code-inline`
- [x] inline math ($...$) — `math-inline`
- [x] link (#link("url")[text]) — `link`
- [x] image (#image("file")) — `image`
- [x] line break (#linebreak()) — `line-break`
- [x] footnote (#footnote[...]) — `footnote-def`
- [ ] reference (@label) — (missing)
- [ ] label (<label>) — (missing)
- [ ] smartquote ("..." / '...') — (missing)
- [ ] highlight (#highlight[...]) — (missing)
- [ ] overline (#overline[...]) — (missing)
- [ ] small caps (#smallcaps[...]) — (missing)
- [ ] text color (#text(fill: ...)[...]) — (missing)
- [ ] font size (#text(size: ...)[...]) — (missing)
- [ ] raw inline (`code`) — (missing; covered partially by code-inline)

## Math constructs

- [x] inline math — `math-inline`
- [x] display / block math — `math-display`
- [ ] fractions (a/b inside math) — (missing)
- [ ] subscript in math — (missing)
- [ ] superscript in math — (missing)
- [ ] math functions (sum, int, etc.) — (missing)

## Functions / Show rules / Metadata

- [ ] #set heading — (missing)
- [ ] #set text(font: ...) — (missing)
- [ ] #set page(...) — (missing)
- [ ] #show rule — (missing)
- [ ] document title via #set document(title:) — (missing)
- [ ] author via #set document(author:) — (missing)
- [ ] #include — (missing)

## Properties

- [ ] heading level — (heading/heading-h2 fixtures cover two levels)
- [ ] code block language — (missing dedicated property fixture)
- [ ] link URL — (missing dedicated property fixture)
- [ ] image source path — (missing dedicated property fixture)
- [ ] table column alignment — (missing)
- [ ] figure caption — (missing)

## Composition (integration)

- [ ] bold inside list item — (missing)
- [ ] table with inline formatting in cells — (missing)
- [ ] footnote inside paragraph — (missing)
- [ ] image inside figure with caption — (missing)
- [ ] math inside heading — (missing)
- [ ] nested lists — (missing)
- [ ] code block inside blockquote — (missing)

## Adversarial

- [x] empty document — `adv-empty`
- [ ] unclosed bold (*text without closing *) — (missing)
- [ ] unclosed function call (#link( without )) — (missing)
- [ ] unknown function (#foo[...]) — (missing)
- [ ] unclosed code block — (missing)
- [ ] malformed math expression — (missing)

## Pathological

- [ ] very long paragraph (>64 KB) — (missing)
- [ ] deeply nested lists — (missing)
- [ ] very large table — (missing)
- [ ] heading depth at maximum level — (missing)
- [ ] hundreds of footnotes — (missing)
