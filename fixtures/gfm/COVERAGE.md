# GFM (GitHub Flavored Markdown) Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

**Coverage-completeness caveat (2026-07-28):** the checklist below is a hand-curated list of
constructs, not yet verified against a spec-derived, machine-readable construct index. An
audit of `fixtures/docbook/COVERAGE.md` and `fixtures/jats/COVERAGE.md` against authoritative
element indexes found hundreds of element names enumerated nowhere, moving denominators
mid-session purely from incidentally-noticed gaps -- a ratio over a hand-written list like this
one is not a coverage measurement. See `docs/format-audit.md`'s "Construct Coverage (CC)"
section for the full evidence; this format's `CC` status there is `U` (unverified) until a
construct registry (in design, see `docs/adr/`) checks this list against the format's own
spec.

GFM is a strict superset of CommonMark. All CommonMark constructs apply; this file adds
GFM-specific extensions on top.

## Block constructs (CommonMark baseline)
- [x] paragraph — `paragraph`
- [x] heading (ATX, levels 1–6) — `heading`
- [x] heading levels h2–h6 individually — `heading-h2`, `heading-h3`, `heading-h4`, `heading-h5`, `heading-h6`
- [x] setext heading — `setext-heading`
- [x] fenced code block — `code-block`
- [x] indented code block — `indented-code-block`
- [x] blockquote — `blockquote`
- [x] unordered list — `list-unordered`
- [x] ordered list — `list-ordered`
- [x] loose list (blank lines between items) — `loose-list`
- [x] horizontal rule (thematic break) — `horizontal-rule`
- [x] raw HTML block — `raw-html-block`
- [x] link reference definition — `link-reference`

## Block constructs (GFM extensions)
- [x] table — `table`
- [x] table with alignment (left, right, center, none) — `table-alignment`
- [x] task list — `task-list`
- [x] footnote (GFM does not define footnotes; if supported, fixture needed) — `footnote-gfm`

## Inline constructs (CommonMark baseline)
- [x] emphasis (italic) — `emphasis`
- [x] strong (bold) — `strong`
- [x] inline code — `code-inline`
- [x] link (inline) — `link`
- [x] link (reference style) — `link-reference`
- [x] image — `image`
- [x] raw HTML inline — `raw-html-inline`
- [x] hard line break — `line-break`
- [x] soft line break — `soft-break`
- [x] autolink (angle brackets) — `autolink`
- [x] backslash escape — `backslash-escape`
- [x] entity reference — `entity-reference`

## Inline constructs (GFM extensions)
- [x] strikethrough (`~~...~~`) — `strikeout`
- [x] autolink literal (bare URL without angle brackets) — `autolink-literal`
- [x] disallowed raw HTML (GFM tag filter) — `disallowed-raw-html`

## Properties
- [x] fenced code block language — `code-block-lang`
- [x] ordered list start number — `ordered-list-start`
- [x] link title — `link-title`
- [x] image alt text — `image-alt`
- [x] image title — `image-title`
- [x] heading level (1–6) — `heading`
- [x] table column alignment — `table-alignment`
- [x] task list item checked state — `task-list`

## Composition (integration)
- [x] emphasis inside table cell — `integration-emphasis-in-table-cell`
- [x] code inside list item — `integration-code-in-list-item`
- [x] link inside strikethrough — `integration-link-in-strikethrough`
- [x] task list inside blockquote — `integration-task-list-in-blockquote`
- [x] heading with inline markup — `integration-heading-with-inline`
- [x] nested blockquotes — `integration-nested-blockquotes`
- [x] nested list — `integration-nested-list`
- [x] table with formatted cells — `integration-table-formatted-cells`
- [x] strikethrough inside emphasis — `integration-strikethrough-in-emphasis`

## End-to-end
- [x] realistic multi-section document — `e2e-multi-section`
- [x] document with table, task list, and strikethrough — `e2e-table-tasklist-strikethrough`

## Rare
- [x] setext heading — `setext-heading`
- [x] indented code block — `indented-code-block`
- [x] tilde-fenced code block — `rare-tilde-fenced-code`
- [x] reference-style link — `link-reference`
- [x] link with title — `link-title`
- [x] ordered list with non-1 start — `ordered-list-start`
- [x] table with no alignment row — `adv-malformed-table`
- [x] autolink literal (bare URL) — `autolink-literal`
- [x] nested emphasis/strong — `rare-nested-emphasis-strong`
- [x] backslash escape — `backslash-escape`

## Adversarial
- [x] empty document — `adv-empty`
- [x] whitespace-only document — `adv-whitespace`
- [x] unclosed fenced code block — `adv-unclosed-fence`
- [x] unclosed emphasis — `adv-unclosed-emphasis`
- [x] broken link — `adv-broken-link`
- [x] malformed table (mismatched column counts) — `adv-malformed-table`
- [x] task list item with no space after bracket — `adv-task-no-space`
- [x] deeply nested blockquotes — `adv-deep-blockquote`

## Pathological
- [x] 1000-item list — `path-large-list`
- [x] deeply nested blockquotes (100 levels) — `path-deep-blockquote`
- [x] very long paragraph (>64 KB) — `path-long-paragraph`
- [x] large table (many rows/columns) — `path-large-table`
- [x] table with very wide cells — `path-wide-cells`
