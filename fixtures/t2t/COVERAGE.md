# txt2tags Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

txt2tags (t2t) uses a three-section document structure: header (lines 1-3), settings
(%%...%%), and body. The reference is the txt2tags user guide and source.

## Block constructs
- [x] paragraph — `paragraph`
- [x] heading h1 (= Heading =) — `heading`
- [x] heading h2 (== Heading ==) — `heading-h2`
- [x] numbered heading (+ Heading +) — `heading-numbered`
- [x] heading h3 (=== Heading ===) — `heading-h3`
- [x] heading h4 (==== Heading ====) — `heading-h4`
- [x] heading h5 (===== Heading =====) — `heading-h5`
- [x] numbered heading h2 (++ Heading ++) — `heading-numbered-h2`
- [x] numbered heading h3 (+++ Heading +++) — `heading-numbered-h3`
- [x] unordered list (- item) — `list-unordered`
- [x] ordered list (+ item) — `list-ordered`
- [x] definition list (: term / definition) — `definition-list`
- [x] nested list — `nested-list`
- [x] blockquote (\t indent) — `blockquote`
- [x] code block (``` ... ```) — `code-block`
- [x] horizontal rule (20+ dashes/equals/underscores) — `horizontal-rule`
- [x] table — `table`
- [x] table with header row — `table-header`
- [x] image ([image.png]) — `image`
- [x] raw block (""" ... """) — `raw-block`
- [x] comment line (%) — `rare-comment`

## Inline constructs
- [x] italic (//text//) — `italic`
- [x] bold (**text**) — `bold`
- [x] strikethrough (--text--) — `strikethrough`
- [x] underline (__text__) — `rare-underline`
- [x] inline code (``text``) — `rare-code-inline`
- [x] link ([label url] or bare URL) — `link`
- [x] verbatim (""text"") — `verbatim-inline`
- [x] tagged inline (''text'') — `tagged-inline`
- [x] line break — `line-break`

## Properties
- [x] document header (title, author, date — lines 1-3) — `document-header`

## Composition (integration)
- [x] table with inline formatting in cells — `comp-table-inline`
- [x] blockquote containing text — `comp-blockquote-list`
- [x] list item with inline code — `comp-list-code`
- [x] heading followed immediately by list — `comp-heading-list`
- [x] link inside bold — `comp-link-bold`

## Adversarial
- [x] empty document — `adv-empty`
- [x] unknown / unrecognized construct — `adv-unknown`
- [x] heading without closing marker — `adv-heading-no-close`
- [x] malformed table — `adv-malformed-table`
- [x] unclosed code block — `adv-unclosed-code`
- [x] link with missing closing bracket — `adv-link-no-close`

## Pathological
- [x] document with many sections — `path-many-sections`
- [x] very large table — `path-large-table`
- [x] list with many items — `path-deep-nested-list`
- [x] very long paragraph — `path-long-paragraph`
- [x] heading at every level — `path-all-headings`
