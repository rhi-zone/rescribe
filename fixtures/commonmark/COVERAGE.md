# CommonMark Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

## Block constructs
- [x] paragraph — `paragraph`
- [x] heading (ATX, levels 1–6) — `heading`
- [x] heading levels h2–h6 individually — `rare-heading-levels`
- [x] setext heading (underline style) — `rare-setext-heading`
- [x] fenced code block — `code-block`
- [x] indented code block — `rare-indented-code-block`
- [x] blockquote — `blockquote`
- [x] unordered list — `list-unordered`
- [x] ordered list — `list-ordered`
- [x] loose list (blank lines between items) — `rare-loose-list`
- [x] tight list — `tight-list`
- [x] horizontal rule (thematic break) — `horizontal-rule`
- [x] raw HTML block — `raw-html-block`
- [x] link reference definition — `rare-link-ref-def`

## Inline constructs
- [x] emphasis (italic) — `emphasis`
- [x] strong (bold) — `strong`
- [x] inline code — `code-inline`
- [x] link (inline) — `link`
- [x] link (reference style) — `rare-link-ref`
- [x] link (collapsed reference) — `rare-link-collapsed`
- [x] link (full reference) — `rare-link-full`
- [x] image — `image`
- [x] image (reference style) — `rare-image-ref`
- [x] raw HTML inline — `raw-html-inline`
- [x] hard line break (backslash or two-space) — `line-break`
- [x] soft line break — `soft-break`
- [x] autolink (bare URL / email in angle brackets) — `rare-autolink`
- [x] backslash escape — `rare-backslash-escape`
- [x] entity reference (`&amp;`, `&#x26;`, etc.) — `rare-entity-ref`

## Properties
- [x] fenced code block language — `code-block-lang`
- [x] ordered list start number — `rare-ordered-list-start`
- [x] link title — `rare-link-title`
- [x] image alt text — `image`
- [x] image title — `rare-image-title`
- [x] heading level (1–6) — `heading`

## Composition (integration)
- [x] emphasis inside blockquote — `integration-emphasis-in-blockquote`
- [x] inline code inside list item — `integration-code-in-list`
- [x] link inside emphasis — `integration-link-in-emphasis`
- [x] image inside link — `integration-image-in-link`
- [x] heading with inline markup — `integration-heading-with-inline`
- [x] nested blockquotes — `integration-nested-blockquotes`
- [x] nested list (list inside list item) — `integration-nested-list`
- [x] code block inside blockquote — `integration-code-block-in-blockquote`
- [x] list item with multiple paragraphs (loose item) — `integration-loose-list-item`
- [x] table inside blockquote — `integration-table-in-blockquote`

## End-to-end
- [x] realistic multi-section document — `e2e-document`

## Rare
- [x] setext heading — `rare-setext-heading`
- [x] indented code block — `rare-indented-code-block`
- [x] tilde-fenced code block — `rare-tilde-code-block`
- [x] reference-style link — `rare-link-ref`
- [x] collapsed reference link — `rare-link-collapsed`
- [x] link with title — `rare-link-title`
- [x] ordered list with non-1 start — `rare-ordered-list-start`
- [x] tight vs loose list distinction — `rare-tight-vs-loose`
- [x] blank line inside blockquote — `rare-blank-in-blockquote`
- [x] autolink — `rare-autolink`
- [x] backslash escape of punctuation — `rare-backslash-escape`
- [x] entity reference — `rare-entity-ref`
- [x] hard line break (two-space variant) — `rare-hard-line-break-spaces`
- [x] emphasis with underscores — `rare-emphasis-underscore`
- [x] nested emphasis/strong — `rare-nested-emphasis`

## Adversarial
- [x] empty document — `adv-empty`
- [x] whitespace-only document — `adv-whitespace`
- [x] unclosed fenced code block — `adv-unclosed-fence`
- [x] unclosed emphasis — `adv-unclosed-emphasis`
- [x] broken link (no closing bracket) — `adv-broken-link`
- [x] deeply nested blockquotes — `adv-deep-blockquote`
- [x] malformed ATX heading (too many hashes) — `adv-bad-heading`
- [x] setext heading with no content — `adv-setext-no-content`

## Pathological
- [x] 1000-item list — `path-long-list`
- [x] deeply nested blockquotes (100 levels) — `path-deep-blockquote`
- [x] very long paragraph (>64 KB) — `path-long-paragraph`
- [x] large table — `path-large-table`
- [x] many link reference definitions — `path-many-link-defs`
