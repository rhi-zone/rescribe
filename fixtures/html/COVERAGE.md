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
- [ ] nested list — (missing)
- [x] table — `table`
- [x] table with `<tfoot>` — `table-foot`
- [ ] table with `<colgroup>`/`<col>` — (missing)
- [ ] table with colspan/rowspan — (missing)
- [x] horizontal rule — `horizontal-rule`
- [x] figure (`<figure>`) — `figure`
- [x] definition list (`<dl>/<dt>/<dd>`) — `rare-definition-list`
- [x] div — `div`
- [ ] `<details>`/`<summary>` — (missing)
- [ ] `<section>` — (missing)
- [ ] `<article>`, `<aside>`, `<main>`, `<nav>`, `<header>`, `<footer>` — (missing)
- [ ] `<address>` — (missing)

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
- [ ] abbreviation (`<abbr>`) — (missing)
- [ ] mark (`<mark>`) — (missing)
- [ ] keyboard (`<kbd>`) — (missing)
- [ ] variable (`<var>`) — (missing)
- [ ] sample output (`<samp>`) — (missing)
- [ ] citation (`<cite>`) — (missing)
- [ ] inserted text (`<ins>`) — (missing)
- [ ] footnote (no native HTML construct; `<a>` anchor convention) — (missing)
- [ ] inline math (`<math>` MathML) — (missing)

## Properties / Metadata
- [x] metadata title (`<title>`) — `metadata-title`
- [x] metadata `<meta>` tags — `metadata-meta`
- [ ] lang attribute — (missing)
- [ ] dir attribute (bidi) — (missing)
- [ ] id attribute (anchor) — (missing)
- [ ] class attribute — (missing)
- [ ] style attribute (inline CSS) — (missing)
- [ ] `<link rel="stylesheet">` — (missing)
- [ ] `<base href>` — (missing)
- [ ] Open Graph meta tags — (missing)
- [ ] charset declaration — (missing)

## Composition (integration)
- [ ] nested blockquote — (missing)
- [ ] list items containing block-level content (paragraphs, code blocks) — (missing)
- [ ] table cells containing inline formatting — (missing)
- [ ] figure with caption (`<figcaption>`) — (missing)
- [ ] heading with inline formatting — (missing)
- [ ] link wrapping image — (missing)
- [ ] definition list term with multiple descriptions — (missing)
- [ ] deeply nested inline formatting (bold inside italic inside link) — (missing)

## Adversarial
- [x] unclosed tags — `adv-unclosed-tags`
- [x] script/style stripped — `adv-script-stripped`
- [x] empty document — `adv-empty`
- [x] deeply nested elements — `adv-deeply-nested`
- [ ] malformed character references (`&amp;`, `&#x;`, unknown `&foo;`) — (missing)
- [ ] duplicate attributes on same element — (missing)
- [ ] self-closing non-void elements (`<div/>`) — (missing)
- [ ] invalid nesting (block inside inline, e.g. `<p><div>`) — (missing)
- [ ] null bytes and control characters — (missing)
- [ ] very long attribute values — (missing)

## Pathological
- [ ] document with thousands of paragraphs — (missing)
- [ ] table with hundreds of columns — (missing)
- [ ] deeply nested lists (20+ levels) — (missing)
- [ ] very large inline content (multi-megabyte text node) — (missing)
- [ ] extremely long URLs — (missing)
