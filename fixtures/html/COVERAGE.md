# HTML Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

## Block constructs
- [x] paragraph — `paragraph`
- [x] heading h1 — `heading-h1`
- [x] heading h2 — `heading-h2`
- [x] heading h3 — `heading-h3`
- [x] heading h4 — `heading-h4`
- [x] heading h5 — `heading-h5`
- [x] heading h6 — `heading-h6`
- [x] blockquote — `blockquote`
- [x] code block (fenced, with language) — `code-block-lang`
- [x] code block (no language) — `code-block-no-lang`
- [x] code block (`<pre><code>`) — `code-block`
- [x] unordered list — `list-unordered`
- [x] ordered list — `list-ordered`
- [x] ordered list with start attribute — `rare-ordered-list-start`
- [x] nested list — `nested-list`
- [x] table — `table`
- [x] table with `<tfoot>` — `table-foot`
- [x] table with `<colgroup>`/`<col>` — `table-colgroup`
- [x] table with colspan/rowspan — `table-colspan-rowspan`
- [x] horizontal rule — `horizontal-rule`
- [x] figure (`<figure>`) — `figure`
- [x] definition list (`<dl>/<dt>/<dd>`) — `rare-definition-list`
- [x] div — `div`
- [x] `<details>`/`<summary>` — `details-summary`
- [x] `<section>` — `semantic-section`
- [x] `<article>`, `<aside>`, `<main>`, `<nav>`, `<header>`, `<footer>` — `rare-semantic-article`
- [x] `<address>` — `rare-address`

## Inline constructs
- [x] emphasis (`<em>`) — `emphasis`
- [x] strong (`<strong>`) — `strong`
- [x] strikeout (`<s>`, `<del>`) — `strikeout`
- [x] underline (`<u>`) — `underline`
- [x] subscript (`<sub>`) — `subscript`
- [x] superscript (`<sup>`) — `superscript`
- [x] inline code (`<code>`) — `code-inline`
- [x] link (`<a href>`) — `link`
- [x] link with title — `rare-link-with-title`
- [x] image (`<img>`) — `image`
- [x] image with title — `rare-image-with-title`
- [x] line break (`<br>`) — `line-break`
- [x] span (`<span>`) — `span`
- [x] small caps (CSS `font-variant: small-caps`) — `small-caps`
- [x] quoted (`<q>`) — `quoted`
- [x] abbreviation (`<abbr>`) — `abbr`
- [x] mark (`<mark>`) — `mark`
- [x] keyboard (`<kbd>`) — `kbd`
- [x] variable (`<var>`) — `var`
- [x] sample output (`<samp>`) — `samp`
- [x] citation (`<cite>`) — `cite`
- [x] inserted text (`<ins>`) — `ins`
- [ ] footnote (no native HTML construct; `<a>` anchor convention) — (deferred: complex pattern detection)
- [ ] inline math (`<math>` MathML) — (deferred: MathML conversion complexity)

## Properties / Metadata
- [x] metadata title (`<title>`) — `metadata-title`
- [x] metadata `<meta>` tags — `metadata-meta`
- [x] lang attribute — `attr-lang`
- [x] dir attribute (bidi) — `attr-dir`
- [x] id attribute (anchor) — `attr-id`
- [x] class attribute — `attr-class`
- [x] style attribute (inline CSS) — `attr-style`
- [x] `<link rel="stylesheet">` — `attr-link-stylesheet`
- [x] `<base href>` — `attr-base-href`
- [x] Open Graph meta tags — `attr-og-meta`
- [x] charset declaration — `attr-charset`

## Composition (integration)
- [x] nested blockquote — `comp-nested-blockquote`
- [x] list items containing block-level content (paragraphs, code blocks) — `comp-list-block-items`
- [x] table cells containing inline formatting — `comp-table-inline-formatting`
- [x] figure with caption (`<figcaption>`) — `comp-figure-caption`
- [x] heading with inline formatting — `comp-heading-inline`
- [x] link wrapping image — `comp-link-wrapping-image`
- [x] definition list term with multiple descriptions — `comp-definition-list-multi`
- [x] deeply nested inline formatting (bold inside italic inside link) — `comp-deep-inline`

## Adversarial
- [x] unclosed tags — `adv-unclosed-tags`
- [x] script/style stripped — `adv-script-stripped`
- [x] empty document — `adv-empty`
- [x] deeply nested elements — `adv-deeply-nested`
- [x] malformed character references (`&amp;`, `&#x;`, unknown `&foo;`) — `adv-malformed-char-refs`
- [x] duplicate attributes on same element — `adv-duplicate-attrs`
- [x] self-closing non-void elements (`<div/>`) — `adv-self-closing-non-void`
- [x] invalid nesting (block inside inline, e.g. `<p><div>`) — `adv-invalid-nesting`
- [x] null bytes and control characters — `adv-null-bytes`
- [x] very long attribute values — `adv-long-attrs`

## Pathological
- [x] document with thousands of paragraphs — `path-large-document`
- [x] table with hundreds of columns — `path-wide-table`
- [x] deeply nested lists (20+ levels) — `path-deep-nested-list`
- [ ] very large inline content (multi-megabyte text node) — (deferred: large test file)
- [x] extremely long URLs — `path-long-url`
