# Jira Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

Jira wiki markup reference (Atlassian): https://jira.atlassian.com/secure/WikiRendererHelpAction.jspa

## Block constructs

- [x] paragraph — `paragraph`
- [x] heading h1 (`h1.`) — `heading`
- [x] heading h2 (`h2.`) — `heading-h2`
- [ ] heading h3–h6 — (missing)
- [x] horizontal rule (`----`) — `horizontal-rule`
- [x] unordered list (`*`) — `list-unordered`
- [x] ordered list (`#`) — `list-ordered`
- [ ] nested list (mixed `*` and `#`, 2+ levels) — (missing)
- [x] code block (`{code}` / `{code:lang}`) — `code-block`
- [x] code block with language — `code-block-lang`
- [x] blockquote (`{quote}`) — `rare-blockquote`
- [x] panel (`{panel}`) — `panel`
- [ ] info / note / tip / warning macros (`{info}`, `{note}`, `{tip}`, `{warning}`) — (missing)
- [ ] noformat block (`{noformat}`) — (missing)
- [x] table — `table`
- [ ] table with header row (`||`) — (missing)

## Inline constructs

- [x] bold (`*text*`) — `bold`
- [x] italic (`_text_`) — `italic`
- [x] underline (`+text+`) — `underline`
- [x] strikethrough (`-text-`) — `rare-strikeout`
- [x] subscript (`~text~`) — `subscript`
- [x] superscript (`^text^`) — `superscript`
- [x] inline code / monospace (`{{text}}`) — `rare-code-inline`
- [x] link (`[url]` / `[text|url]`) — `link`
- [x] image (`!image.png!`) — `image`
- [ ] image with attributes (`!image.png|width=100!`) — (missing)
- [ ] named anchor (`{anchor:name}`) — (missing)
- [ ] mention (`@user`) — (missing)
- [ ] emoji (`:smile:`) — (missing)
- [ ] color macro (`{color:red}text{color}`) — (missing)

## Properties

- [ ] heading levels h3–h6 — (missing)
- [ ] link display text — (missing; `link` fixture may not cover display text explicitly)
- [ ] image width/height/thumbnail attributes — (missing)
- [ ] code block language — covered by `code-block-lang`
- [ ] panel with title/border/color attributes — (missing; `panel` may not test attributes)
- [ ] table header row (`||`) — (missing)

## Composition (integration)

- [ ] nested lists (2+ levels, mixed ordered/unordered) — (missing)
- [ ] inline markup inside table cells — (missing)
- [ ] inline markup inside list items — (missing)
- [ ] panel containing a code block — (missing)
- [ ] blockquote with inline markup — (missing)

## Adversarial

- [x] empty document — `adv-empty`
- [ ] unclosed bold/italic — (missing)
- [ ] unclosed `{code}` block — (missing)
- [ ] unclosed `{panel}` or `{quote}` — (missing)
- [ ] table with missing closing `|` — (missing)
- [ ] nested tables (Jira does not support; parser must not crash) — (missing)

## Pathological

- [ ] deeply nested lists (5+ levels) — (missing)
- [ ] very wide table (20+ columns) — (missing)
- [ ] heading containing inline markup — (missing)
- [ ] code block with thousands of lines — (missing)
