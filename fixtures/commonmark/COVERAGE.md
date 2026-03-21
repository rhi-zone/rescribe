# CommonMark Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

## Block constructs
- [x] paragraph — `paragraph`
- [x] heading (ATX, levels 1–6) — `heading`
- [ ] heading levels h2–h6 individually — (missing)
- [ ] setext heading (underline style) — (missing)
- [x] fenced code block — `code-block`
- [ ] indented code block — (missing)
- [x] blockquote — `blockquote`
- [x] unordered list — `list-unordered`
- [x] ordered list — `list-ordered`
- [ ] loose list (blank lines between items) — (missing)
- [ ] tight list — (missing)
- [x] horizontal rule (thematic break) — `horizontal-rule`
- [x] raw HTML block — `raw-html-block`
- [ ] link reference definition — (missing)

## Inline constructs
- [x] emphasis (italic) — `emphasis`
- [x] strong (bold) — `strong`
- [x] inline code — `code-inline`
- [x] link (inline) — `link`
- [ ] link (reference style) — (missing)
- [ ] link (collapsed reference) — (missing)
- [ ] link (full reference) — (missing)
- [x] image — `image`
- [ ] image (reference style) — (missing)
- [x] raw HTML inline — `raw-html-inline`
- [x] hard line break (backslash or two-space) — `line-break`
- [x] soft line break — `soft-break`
- [ ] autolink (bare URL / email in angle brackets) — (missing)
- [ ] backslash escape — (missing)
- [ ] entity reference (`&amp;`, `&#x26;`, etc.) — (missing)

## Properties
- [x] fenced code block language — `code-block-lang`
- [ ] ordered list start number — (missing)
- [ ] link title — (missing)
- [ ] image alt text — `image`
- [ ] image title — (missing)
- [ ] heading level (1–6) — `heading`

## Composition (integration)
- [ ] emphasis inside blockquote — (missing)
- [ ] inline code inside list item — (missing)
- [ ] link inside emphasis — (missing)
- [ ] image inside link — (missing)
- [ ] heading with inline markup — (missing)
- [ ] nested blockquotes — (missing)
- [ ] nested list (list inside list item) — (missing)
- [ ] code block inside blockquote — (missing)
- [ ] list item with multiple paragraphs (loose item) — (missing)
- [ ] table inside blockquote — (missing)

## End-to-end
- [ ] realistic multi-section document — (missing)

## Rare
- [ ] setext heading — (missing)
- [ ] indented code block — (missing)
- [ ] tilde-fenced code block — (missing)
- [ ] reference-style link — (missing)
- [ ] collapsed reference link — (missing)
- [ ] link with title — (missing)
- [ ] ordered list with non-1 start — (missing)
- [ ] tight vs loose list distinction — (missing)
- [ ] blank line inside blockquote — (missing)
- [ ] autolink — (missing)
- [ ] backslash escape of punctuation — (missing)
- [ ] entity reference — (missing)
- [ ] hard line break (two-space variant) — (missing)
- [ ] emphasis with underscores — (missing)
- [ ] nested emphasis/strong — (missing)

## Adversarial
- [ ] empty document — (missing)
- [ ] whitespace-only document — (missing)
- [ ] unclosed fenced code block — (missing)
- [ ] unclosed emphasis — (missing)
- [ ] broken link (no closing bracket) — (missing)
- [ ] deeply nested blockquotes — (missing)
- [ ] malformed ATX heading (too many hashes) — (missing)
- [ ] setext heading with no content — (missing)

## Pathological
- [ ] 1000-item list — (missing)
- [ ] deeply nested blockquotes (100 levels) — (missing)
- [ ] very long paragraph (>64 KB) — (missing)
- [ ] large table — (missing)
- [ ] many link reference definitions — (missing)
