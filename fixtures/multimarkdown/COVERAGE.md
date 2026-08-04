# MultiMarkdown Fixture Coverage

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

MultiMarkdown (MMD) extends Markdown with tables, footnotes, definition lists,
math, metadata, cross-references, glossaries, abbreviations, and more.

**2026-08-04: backed by `multimarkdown-fmt`** (was previously a hand-rolled
`pulldown-cmark` wrapper directly in `rescribe-read-multimarkdown`, a
CLAUDE.md "adapter layer must never contain parsing logic" violation).
`multimarkdown-fmt` depends on `commonmark-fmt` for every CommonMark/GFM/
footnotes/definition-lists/math construct and implements only genuine
MMD-uniques (metadata blocks, citations, cross-references) itself — see that
crate's module docs. Two known, confirmed gaps from this rework:
- **subscript/superscript are no longer supported.** `~sub~`/`^sup^` are
  pulldown-cmark's own `ENABLE_SUBSCRIPT`/`ENABLE_SUPERSCRIPT` extensions,
  which `commonmark-fmt` does not expose (its feature set is tables/
  task-lists/strikethrough/frontmatter/footnotes/definition-lists/math —
  confirmed by reading its `Cargo.toml`). Single-tilde `~sub~` additionally
  collides with `commonmark-fmt`'s own strikethrough feature. Fixing this
  requires adding subscript/superscript to `commonmark-fmt` itself (a
  separate vertical), not something achievable inside `multimarkdown-fmt`.
  `fixtures/multimarkdown/subscript` and `.../superscript` still exist and
  are still correct fixtures for the *format* — they are skipped by name in
  `rescribe-fixtures/tests/run.rs`'s `multimarkdown()` test with a comment
  explaining why, not deleted. Tracked in `TODO.md`.
- **The inline citation-content form** (`text.[#Full citation content.]`,
  defining a citation's content inline rather than via a separate
  `[#refname]:` line) is not implemented. Tracked in `TODO.md`.

## Block constructs (Markdown baseline)
- [x] paragraph — `paragraph`
- [x] heading — `heading`
- [ ] heading levels h2–h6 individually — (missing)
- [ ] setext heading — (missing)
- [x] fenced code block — `code-block`
- [ ] indented code block — (missing)
- [x] blockquote — `blockquote`
- [x] unordered list — `list-unordered`
- [x] ordered list — `list-ordered`
- [x] horizontal rule — `horizontal-rule`
- [x] raw HTML block — `raw-html-block`
- [ ] link reference definition — (missing)

## Block constructs (MMD extensions)
- [x] table (with alignment) — `table`
- [x] footnote definition — `footnote`
- [x] definition list — `definition-list`
- [x] metadata block, bare (`Key: value`) — `metadata-bare`
- [x] metadata block, `---`-delimited — `metadata-delimited`
- [x] citation definition (`[#refname]: content`) — `citation-definition`
- [ ] table of contents placeholder (`{{TOC}}`) — (missing)
- [ ] abbreviation definition — (missing)
- [ ] glossary term definition — (missing)
- [ ] file transclusion (`{{file}}`) — (missing)
- [ ] comment block (`<!--` ... `-->`) — (missing)

## Inline constructs (Markdown baseline)
- [x] emphasis (italic) — `emphasis`
- [x] strong (bold) — `strong`
- [x] strikethrough — `strikeout`
- [x] inline code — `code-inline`
- [x] link (inline) — `link`
- [x] image — `image`
- [x] raw HTML inline — `raw-html-inline`
- [x] hard line break — `line-break`
- [x] soft line break — `soft-break`
- [ ] autolink — (missing)
- [ ] backslash escape — (missing)
- [ ] entity reference — (missing)

## Inline constructs (MMD extensions)
- [x] footnote reference — `footnote`
- [ ] subscript — not supported by `commonmark-fmt`; see caveat above (`subscript` fixture skipped, tracked in TODO.md)
- [ ] superscript — not supported by `commonmark-fmt`; see caveat above (`superscript` fixture skipped, tracked in TODO.md)
- [x] inline math — `math-inline`
- [x] display math — `math-display`
- [x] citation with locator (`[locator][#refname]`) — `citation-with-locator`
- [x] citation without locator (`[][#refname]`) — `citation-no-locator`
- [ ] citation, inline-content form (`text.[#content]`) — (missing; not yet implemented, see caveat above)
- [x] cross-reference, shortcut form (`[Anchor]`) — `cross-reference-shortcut`
- [x] cross-reference, collapsed form (`[Header Text][]`) — `cross-reference-collapsed`
- [x] heading anchor label (`### Heading [Anchor] ###`) — `heading-anchor`
- [ ] image with dimensions (`![alt][ref]{width=...}`) — (missing)
- [ ] critic markup (addition/deletion/substitution) — (missing)
- [ ] abbreviation inline — (missing)
- [ ] glossary reference — (missing)

## Properties
- [ ] fenced code block language — (missing; `code-block` present but lang not separately tested)
- [ ] table column alignment — (missing)
- [ ] ordered list start number — (missing)
- [ ] link title — (missing)
- [ ] image alt text — `image`
- [ ] image title — (missing)
- [ ] image dimensions — (missing)
- [ ] heading level — `heading`
- [ ] metadata title — (missing)
- [ ] metadata author — (missing)
- [ ] metadata date — (missing)
- [ ] footnote reference label — `footnote`

## Composition (integration)
- [ ] emphasis inside table cell — (missing)
- [ ] footnote reference inside list item — (missing)
- [ ] math inside blockquote — (missing)
- [ ] definition list inside blockquote — (missing)
- [ ] nested list with footnotes — (missing)
- [ ] table with formatted cells — (missing)
- [x] cross-reference to heading — `cross-reference-shortcut` (also exercised in `e2e-academic-document`)
- [x] citation inside a list item — `integration-citation-in-list`
- [x] citation inside a blockquote — `integration-citation-in-blockquote`

## End-to-end
- [x] realistic academic document (metadata, heading anchor, citation, cross-reference, math, table, footnote, citation definition) — `e2e-academic-document`

## Rare
- [ ] setext heading — (missing)
- [ ] indented code block — (missing)
- [ ] table with colspan (MMD extension) — (missing)
- [ ] multiline table cell — (missing)
- [ ] footnote with multiple paragraphs — (missing)
- [ ] nested footnote references — (missing)
- [ ] abbreviation definition and inline use — (missing)
- [ ] file transclusion — (missing)
- [ ] TOC placeholder — (missing)

## Adversarial
- [ ] empty document — (missing)
- [ ] whitespace-only document — (missing)
- [ ] unclosed fenced code block — (missing)
- [ ] unclosed emphasis — (missing)
- [ ] broken link — (missing)
- [ ] footnote reference with no definition — (missing)
- [ ] malformed math (unclosed `$`) — (missing)
- [ ] malformed table — (missing)
- [x] unclosed citation-definition bracket — `adv-malformed-citation-def`
- [x] citation with empty label — `adv-empty-citation-label`

## Pathological
- [ ] 1000-item list — (missing)
- [ ] deeply nested blockquotes — (missing)
- [ ] very long paragraph (>64 KB) — (missing)
- [ ] large table (many rows/columns) — (missing)
- [ ] many footnotes — (missing)
- [ ] large math block — (missing)
