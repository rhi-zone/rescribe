# Markdown Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

"markdown" here refers to the original Markdown.pl-compatible dialect (Gruber Markdown),
including commonly supported extensions (tables, strikethrough, task lists, footnotes,
front matter). This is the broad-compatibility dialect, not CommonMark-strict.

## Block constructs
- [x] paragraph — `paragraph`
- [x] heading (ATX h1) — `heading`
- [x] heading h2 — `heading-h2`
- [x] heading h3 — `heading-h3`
- [x] heading h4 — `heading-h4`
- [x] heading h5 — `heading-h5`
- [x] heading h6 — `heading-h6`
- [x] setext heading — `rare-setext-heading`
- [x] fenced code block (no lang) — `code-block-no-lang`
- [x] fenced code block (with lang) — `code-block-with-lang`
- [x] indented code block — `rare-indented-code`
- [x] tilde-fenced code block — `rare-fenced-tilde`
- [x] blockquote — `blockquote`
- [x] unordered list — `list-unordered`
- [x] ordered list — `list-ordered`
- [x] nested list — `list-nested`
- [x] ordered list with non-1 start — `rare-ordered-list-start`
- [x] horizontal rule — `horizontal-rule`
- [x] table — `table`
- [x] task list — `task-list`
- [x] raw HTML block — `raw-html-block`
- [x] footnote definition — `footnote`
- [x] YAML front matter — `frontmatter-yaml`
- [ ] definition list — (missing)
- [ ] loose list (blank lines between items) — (missing)
- [ ] link reference definition — (missing)

## Inline constructs
- [x] emphasis (italic) — `italic`
- [x] strong (bold) — `bold`
- [x] strikethrough — `strikeout`
- [x] inline code — `code-inline`
- [x] link (inline) — `link`
- [x] link with title — `rare-link-with-title`
- [x] image — `image`
- [x] image with title — `rare-image-with-title`
- [x] raw HTML inline — `raw-html-inline`
- [x] hard line break — `line-break`
- [x] soft line break — `soft-break`
- [x] nested emphasis — `rare-nested-emphasis`
- [x] footnote reference — `footnote`
- [ ] autolink (angle brackets) — (missing)
- [ ] backslash escape — (missing)
- [ ] entity reference — (missing)
- [ ] subscript — (missing)
- [ ] superscript — (missing)
- [ ] underline — (missing)
- [ ] mark / highlight — (missing)

## Properties
- [x] fenced code block language — `code-block-with-lang`
- [x] ordered list start number — `rare-ordered-list-start`
- [x] link title — `rare-link-with-title`
- [x] image alt text — `image`
- [x] image title — `rare-image-with-title`
- [x] heading level (1–6) — `heading` through `heading-h6`
- [ ] table column alignment — (missing)
- [ ] task list item checked state — `task-list`
- [ ] front matter title — `frontmatter-yaml`
- [ ] front matter author/date — (missing)

## Composition (integration)
- [ ] emphasis inside list item — (missing)
- [ ] code inside blockquote — (missing)
- [ ] link inside emphasis — (missing)
- [ ] image inside link — (missing)
- [ ] heading with inline markup — (missing)
- [ ] table with formatted cells — (missing)
- [ ] footnote with multiple paragraphs — (missing)
- [ ] nested blockquotes — (missing)
- [ ] list inside blockquote — (missing)

## End-to-end
- [ ] realistic multi-section document with front matter — (missing)

## Rare
- [x] setext heading — `rare-setext-heading`
- [x] indented code block — `rare-indented-code`
- [x] tilde-fenced code block — `rare-fenced-tilde`
- [x] link with title — `rare-link-with-title`
- [x] image with title — `rare-image-with-title`
- [x] nested emphasis — `rare-nested-emphasis`
- [x] ordered list with non-1 start — `rare-ordered-list-start`
- [ ] reference-style link — (missing)
- [ ] collapsed reference link — (missing)
- [ ] autolink — (missing)
- [ ] backslash escape — (missing)
- [ ] entity reference — (missing)
- [ ] blank line inside blockquote — (missing)
- [ ] emphasis with underscores — (missing)

## Adversarial
- [x] empty document — `adv-empty`
- [x] whitespace-only document — `adv-whitespace-only`
- [x] unclosed fenced code block — `adv-unclosed-fence`
- [x] unmatched emphasis delimiters — `adv-unmatched-emphasis`
- [x] broken link (no closing bracket) — `adv-broken-link`
- [x] deeply nested blockquotes — `adv-deeply-nested-blockquotes`
- [ ] malformed table — (missing)
- [ ] unclosed raw HTML tag — (missing)
- [ ] footnote reference with no definition — (missing)

## Pathological
- [ ] 1000-item list — (missing)
- [ ] deeply nested blockquotes (100 levels) — (missing)
- [ ] very long paragraph (>64 KB) — (missing)
- [ ] large table (many rows/columns) — (missing)
- [ ] many footnotes — (missing)
