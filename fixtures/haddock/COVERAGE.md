# Haddock Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

Haddock is the documentation markup language used in Haskell source code (GHC Haddock tool).
Reference: https://haskell-haddock.readthedocs.io/

## Block constructs

- [x] paragraph — `paragraph`
- [x] heading level 1 (=) — `heading`
- [x] heading level 2 (==) — `heading-h2`
- [ ] heading level 3 (===) — (missing)
- [ ] heading level 4 (====) — (missing)
- [x] unordered list (* item) — `list-unordered`
- [x] ordered list (1. item) — `list-ordered`
- [x] definition list ([@term] description) — `definition-list`
- [x] code block (bird-track style: > prefix) — `code-block`
- [ ] code block (@ style) — (missing)
- [ ] blockquote — (missing)
- [ ] property / attribute block (@since, @version, etc.) — (missing)
- [ ] examples block (@examples) — (missing)

## Inline constructs

- [x] bold (__text__) — `bold`
- [x] italic (/text/) — `italic`
- [x] monospace / code (@text@ or @...@) — `code-inline`
- [x] link (module/identifier reference) — `link`
- [x] bare URL (http://...) — `rare-link-bare`
- [x] identifier reference ('Foo or 'Foo.bar) — `rare-identifier`
- [ ] emphasis (emphasis) — (missing; Haddock uses /.../ but no dedicated fixture beyond italic)
- [ ] module link ("Module") — (missing)
- [ ] string gap / special chars — (missing)
- [ ] math inline ($...$) — (missing)

## Properties

- [ ] module name on link — (missing)
- [ ] identifier namespace (type vs term) — (missing)
- [ ] since / version annotations — (missing)
- [ ] deprecated annotation — (missing)

## Composition (integration)

- [ ] bold inside list item — (missing)
- [ ] code inside paragraph — (missing)
- [ ] link inside bold — (missing)
- [ ] definition list with inline markup in description — (missing)
- [ ] nested lists — (missing)

## Adversarial

- [x] empty document — `adv-empty`
- [ ] unclosed bold markup — (missing)
- [ ] unknown @ command — (missing)
- [ ] malformed identifier reference — (missing)
- [ ] bird-track code with trailing spaces — (missing)

## Pathological

- [ ] very long paragraph (>64 KB) — (missing)
- [ ] deeply nested lists — (missing)
- [ ] large definition list (hundreds of entries) — (missing)
- [ ] very long identifier reference — (missing)
