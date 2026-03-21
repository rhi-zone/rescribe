# Creole Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

Creole 1.0 is a lightweight wiki markup standard defined at wikicreole.org.

## Block constructs

- [x] paragraph — `paragraph`
- [x] heading h1 — `heading`
- [x] heading h2 — `heading-h2`
- [ ] heading h3–h6 — (missing)
- [x] horizontal rule (`----`) — `horizontal-rule`
- [x] unordered list (`*`) — `list-unordered`
- [x] ordered list (`#`) — `list-ordered`
- [ ] nested list (mixed ordered/unordered, 2+ levels) — (missing)
- [x] preformatted / code block (`{{{...}}}`) — `code-block`
- [x] table — `table`
- [x] forced line break (`\\`) — `line-break`

## Inline constructs

- [x] bold (`**text**`) — `bold`
- [x] italic (`//text//`) — `italic`
- [x] inline nowiki / code (`{{{text}}}`) — `rare-code-inline`
- [x] link (`[[url]]` / `[[url|text]]`) — `link`
- [x] bare URL (auto-link) — `rare-link-bare`
- [x] image (`{{url}}` / `{{url|alt}}`) — `rare-image`
- [ ] bold+italic combined — (missing)
- [ ] escape character (`~`) — (missing)

## Properties

- [ ] heading levels h3–h6 — (missing; only h1 and h2 covered)
- [ ] link with explicit display text — (missing; `link` fixture may only cover bare URL)
- [ ] image with alt text — (missing; `rare-image` fixture may not test alt)
- [ ] table cell alignment (extension) — (missing)

## Composition (integration)

- [ ] nested lists (2+ levels) — (missing)
- [ ] inline markup inside table cells — (missing)
- [ ] inline markup inside list items — (missing)
- [ ] list immediately following heading — (missing)
- [ ] multiple paragraphs with mixed inline — (missing)

## Adversarial

- [x] empty document — `adv-empty`
- [ ] unclosed bold/italic — (missing)
- [ ] unclosed `{{{` preformatted block — (missing)
- [ ] link with no closing `]]` — (missing)
- [ ] image with no closing `}}` — (missing)
- [ ] table row with missing closing `|` — (missing)

## Pathological

- [ ] deeply nested lists (5+ levels) — (missing)
- [ ] very wide table (20+ columns) — (missing)
- [ ] heading containing inline markup — (missing)
- [ ] paragraph with many consecutive inline spans — (missing)
