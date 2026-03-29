# Creole Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

Creole 1.0 is a lightweight wiki markup standard defined at wikicreole.org.

## Block constructs

- [x] paragraph — `paragraph`
- [x] heading h1 — `heading`
- [x] heading h2 — `heading-h2`
- [x] heading h3–h6 — `heading-h3-h6`
- [x] horizontal rule (`----`) — `horizontal-rule`
- [x] unordered list (`*`) — `list-unordered`
- [x] ordered list (`#`) — `list-ordered`
- [x] nested list (mixed ordered/unordered, 2+ levels) — `nested-list`
- [x] preformatted / code block (`{{{...}}}`) — `code-block`
- [x] table — `table`
- [x] forced line break (`\\`) — `line-break`
- [x] definition list (`;` term / `:` description) — `definition-list`
- [x] blockquote (`>` prefix) — `blockquote`

## Inline constructs

- [x] bold (`**text**`) — `bold`
- [x] italic (`//text//`) — `italic`
- [x] inline nowiki / code (`{{{text}}}`) — `rare-code-inline`
- [x] link (`[[url]]` / `[[url|text]]`) — `link`
- [x] bare URL (auto-link) — `rare-link-bare`
- [x] image (`{{url}}` / `{{url|alt}}`) — `rare-image`
- [x] bold+italic combined — `bold-italic`
- [x] escape character (`~`) — `escape`

## Properties

- [x] heading levels h3–h6 — `heading-h3-h6`
- [x] link with explicit display text — `link` (tests `url` and child text)
- [x] image with alt text — `rare-image` (tests `url` and `alt`)
- [ ] table cell alignment (extension) — (not part of Creole 1.0 spec)

## Composition (integration)

- [x] nested lists (2+ levels) — `nested-list`
- [x] inline markup inside table cells — `comp-inline-in-table`
- [x] inline markup inside list items — `comp-inline-in-list`
- [x] list immediately following heading — `comp-list-after-heading`
- [x] multiple paragraphs with mixed inline — `comp-mixed-paragraphs`

## Adversarial

- [x] empty document — `adv-empty`
- [x] unclosed bold/italic — `adv-unclosed-bold`
- [x] unclosed `{{{` preformatted block — `adv-unclosed-nowiki`
- [x] link with no closing `]]` — `adv-unclosed-link`
- [x] image with no closing `}}` — `adv-unclosed-image`
- [x] table row with missing closing `|` — `adv-table-missing-pipe`

## Pathological

- [x] deeply nested lists (5+ levels) — `path-deep-list`
- [x] very wide table (20+ columns) — `path-wide-table`
- [x] heading containing inline markup — `path-heading-inline`
- [x] paragraph with many consecutive inline spans — `path-many-inlines`
