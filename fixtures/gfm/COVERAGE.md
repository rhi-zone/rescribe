# GFM (GitHub Flavored Markdown) Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

GFM is a strict superset of CommonMark. All CommonMark constructs apply; this file adds
GFM-specific extensions on top.

## Block constructs (CommonMark baseline)
- [x] paragraph — `paragraph`
- [x] heading (ATX, levels 1–6) — `heading`
- [ ] heading levels h2–h6 individually — (missing)
- [ ] setext heading — (missing)
- [x] fenced code block — `code-block`
- [ ] indented code block — (missing)
- [x] blockquote — `blockquote`
- [x] unordered list — `list-unordered`
- [x] ordered list — `list-ordered`
- [ ] loose list (blank lines between items) — (missing)
- [x] horizontal rule (thematic break) — `horizontal-rule`
- [x] raw HTML block — `raw-html-block`
- [ ] link reference definition — (missing)

## Block constructs (GFM extensions)
- [x] table — `table`
- [ ] table with alignment (left, right, center, none) — (missing)
- [x] task list — `task-list`
- [ ] footnote (GFM does not define footnotes; if supported, fixture needed) — (missing)

## Inline constructs (CommonMark baseline)
- [x] emphasis (italic) — `emphasis`
- [x] strong (bold) — `strong`
- [x] inline code — `code-inline`
- [x] link (inline) — `link`
- [ ] link (reference style) — (missing)
- [x] image — `image`
- [x] raw HTML inline — `raw-html-inline`
- [x] hard line break — `line-break`
- [x] soft line break — `soft-break`
- [ ] autolink (angle brackets) — (missing)
- [ ] backslash escape — (missing)
- [ ] entity reference — (missing)

## Inline constructs (GFM extensions)
- [x] strikethrough (`~~...~~`) — `strikeout`
- [ ] autolink literal (bare URL without angle brackets) — (missing)
- [ ] disallowed raw HTML (GFM tag filter) — (missing)

## Properties
- [x] fenced code block language — `code-block-lang`
- [ ] ordered list start number — (missing)
- [ ] link title — (missing)
- [ ] image alt text — `image`
- [ ] image title — (missing)
- [ ] heading level (1–6) — `heading`
- [ ] table column alignment — (missing)
- [ ] task list item checked state — `task-list`

## Composition (integration)
- [ ] emphasis inside table cell — (missing)
- [ ] code inside list item — (missing)
- [ ] link inside strikethrough — (missing)
- [ ] task list inside blockquote — (missing)
- [ ] heading with inline markup — (missing)
- [ ] nested blockquotes — (missing)
- [ ] nested list — (missing)
- [ ] table with formatted cells — (missing)
- [ ] strikethrough inside emphasis — (missing)

## End-to-end
- [ ] realistic multi-section document — (missing)
- [ ] document with table, task list, and strikethrough — (missing)

## Rare
- [ ] setext heading — (missing)
- [ ] indented code block — (missing)
- [ ] tilde-fenced code block — (missing)
- [ ] reference-style link — (missing)
- [ ] link with title — (missing)
- [ ] ordered list with non-1 start — (missing)
- [ ] table with no alignment row — (missing)
- [ ] autolink literal (bare URL) — (missing)
- [ ] nested emphasis/strong — (missing)
- [ ] backslash escape — (missing)

## Adversarial
- [ ] empty document — (missing)
- [ ] whitespace-only document — (missing)
- [ ] unclosed fenced code block — (missing)
- [ ] unclosed emphasis — (missing)
- [ ] broken link — (missing)
- [ ] malformed table (mismatched column counts) — (missing)
- [ ] task list item with no space after bracket — (missing)
- [ ] deeply nested blockquotes — (missing)

## Pathological
- [ ] 1000-item list — (missing)
- [ ] deeply nested blockquotes (100 levels) — (missing)
- [ ] very long paragraph (>64 KB) — (missing)
- [ ] large table (many rows/columns) — (missing)
- [ ] table with very wide cells — (missing)
