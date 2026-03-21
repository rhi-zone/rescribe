# Markua Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

Markua is a Markdown dialect designed for writing books (Leanpub). The reference is the
Markua spec (Peter Armstrong, 2016–2021).

## Block constructs
- [x] paragraph — `paragraph`
- [x] heading h1 (# Heading) — `heading`
- [x] heading h2 (## Heading) — `heading-h2`
- [ ] heading h3–h6 — (missing)
- [x] unordered list — `list-unordered`
- [x] ordered list — `list-ordered`
- [ ] nested list — (missing)
- [ ] definition list — (missing)
- [x] blockquote (> ) — `rare-blockquote`
- [x] code block (fenced ``` or indented) — `code-block`
- [x] horizontal rule (--- or ***) — `horizontal-rule`
- [x] image (![alt](url)) — `image`
- [x] special block ({type} … /type) — `rare-special-block`
- [ ] aside block ({aside} … /aside) — (missing)
- [ ] blurb block ({blurb} … /blurb) — (missing)
- [ ] warning block ({warning} … /warning) — (missing)
- [ ] information block ({information} … /information) — (missing)
- [ ] error block ({error} … /error) — (missing)
- [ ] tip block ({tip} … /tip) — (missing)
- [ ] discussion block ({discussion} … /discussion) — (missing)
- [ ] exercise block ({exercise} … /exercise) — (missing)
- [ ] table (GFM pipe table) — (missing)
- [ ] crosslink (![](path)) — (missing)
- [ ] include directive ({include: file.md}) — (missing)
- [ ] page break ({pagebreak}) — (missing)
- [ ] sample / excerpt marker — (missing)
- [ ] figure with caption — (missing)

## Inline constructs
- [x] italic (*text* or _text_) — `italic`
- [x] bold (**text** or __text__) — `bold`
- [x] strikethrough (~~text~~) — `strikethrough`
- [x] inline code (`text`) — `code-inline`
- [x] link ([text](url)) — `link`
- [ ] subscript (~text~) — (missing)
- [ ] superscript (^text^) — (missing)
- [ ] underline — (missing)
- [ ] small-caps — (missing)
- [ ] footnote reference (^[text] inline or [^ref]) — (missing)
- [ ] index term (i[term]) — (missing)
- [ ] crosslink reference — (missing)
- [ ] math inline ($expr$) — (missing)
- [ ] line break (two spaces or backslash) — (missing)
- [ ] span with attributes ({class: foo}text{/class}) — (missing)

## Properties
- [ ] book metadata (title, author, series) — (missing)
- [x] code block language — `code-block`
- [ ] image alt text, title, width, height — (missing)
- [ ] resource attributes ({width: 100%}) — (missing)
- [ ] heading id / anchor — (missing)
- [ ] special block type attribute — (missing)
- [ ] list marker style — (missing)
- [ ] page break type (before/after chapter) — (missing)
- [ ] table alignment — (missing)

## Composition (integration)
- [ ] special block containing a list — (missing)
- [ ] code block inside a blockquote — (missing)
- [ ] image with caption and attributes — (missing)
- [ ] nested inline formatting — (missing)
- [ ] footnote with inline markup — (missing)
- [ ] link with formatted label — (missing)
- [ ] heading with inline code — (missing)

## Adversarial
- [x] empty document — `adv-empty`
- [ ] unclosed special block — (missing)
- [ ] unknown special block type — (missing)
- [ ] malformed resource attributes — (missing)
- [ ] unmatched inline delimiter — (missing)

## Pathological
- [ ] document with many chapters — (missing)
- [ ] many special blocks of different types — (missing)
- [ ] deeply nested lists — (missing)
- [ ] very large table — (missing)
- [ ] many footnotes — (missing)
