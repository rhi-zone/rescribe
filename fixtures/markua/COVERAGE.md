# Markua Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

Markua is a Markdown dialect designed for writing books (Leanpub). The reference is the
Markua spec (Peter Armstrong, 2016-2021).

## Block constructs
- [x] paragraph — `paragraph`
- [x] heading h1 (# Heading) — `heading`
- [x] heading h2 (## Heading) — `heading-h2`
- [x] heading h3 — `heading-h3`
- [x] heading h4 — `heading-h4`
- [x] heading h5 — `heading-h5`
- [x] heading h6 — `heading-h6`
- [x] unordered list — `list-unordered`
- [x] ordered list — `list-ordered`
- [x] nested list — `nested-list`
- [x] definition list — `definition-list`
- [x] blockquote (> ) — `rare-blockquote`
- [x] code block (fenced ``` or indented) — `code-block`
- [x] horizontal rule (--- or ***) — `horizontal-rule`
- [x] image (![alt](url)) — `image`
- [x] special block (A> / W> / T> etc.) — `rare-special-block`
- [x] aside block (A> ) — `special-block-aside`
- [x] blurb block (B> ) — (covered by rare-special-block)
- [x] warning block (W> ) — `special-block-warning`
- [x] information block (I> ) — `special-block-information`
- [x] error block (E> ) — `special-block-error`
- [x] tip block (T> ) — `special-block-tip`
- [x] discussion block (D> ) — `special-block-discussion`
- [x] exercise block (X> ) — `special-block-exercise`
- [x] table (GFM pipe table) — `table`
- [x] page break ({pagebreak}) — `page-break`

## Inline constructs
- [x] italic (*text* or _text_) — `italic`
- [x] bold (**text** or __text__) — `bold`
- [x] strikethrough (~~text~~) — `strikethrough`
- [x] inline code (`text`) — `code-inline`
- [x] link ([text](url)) — `link`
- [x] subscript (~text~) — `subscript`
- [x] superscript (^text^) — `superscript`
- [x] footnote reference (^[text]) — `footnote-ref`
- [x] index term (i[term]) — `index-term`
- [x] math inline ($expr$) — `math-inline`
- [x] line break (backslash-newline) — `line-break`

## Properties
- [x] code block language — `code-block`

## Composition (integration)
- [x] special block containing a list — `comp-special-block-list`, `special-block-children`
- [x] code block inside a blockquote — `comp-code-in-blockquote`
- [x] nested inline formatting — `comp-nested-inline`
- [x] link with formatted label — `comp-link-formatted`
- [x] heading with inline code — `comp-heading-with-code`

## Adversarial
- [x] empty document — `adv-empty`
- [x] unclosed special block — `adv-unclosed-special-block`
- [x] unknown special block type — `adv-unknown-special-block`
- [x] unmatched inline delimiter — `adv-unmatched-delimiter`

## Pathological
- [x] document with many chapters — `path-many-chapters`
- [x] many special blocks of different types — `path-many-special-blocks`
- [x] deeply nested lists — `path-deeply-nested-lists`
