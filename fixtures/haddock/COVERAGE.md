# Haddock Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

Haddock is the documentation markup language used in Haskell source code (GHC Haddock tool).
Reference: https://haskell-haddock.readthedocs.io/

## Block constructs

- [x] paragraph — `paragraph`
- [x] heading level 1 (=) — `heading`
- [x] heading level 2 (==) — `heading-h2`
- [x] heading level 3 (===) — `heading-h3`
- [x] heading level 4 (====) — `heading-h4`
- [x] unordered list (* item) — `list-unordered`
- [x] ordered list (1. item) — `list-ordered`
- [x] definition list ([term] description) — `definition-list`
- [x] code block (bird-track style: > prefix) — `code-block`
- [x] code block (@ style) — `code-block-at`
- [x] doc-test example (>>> expr) — `doctest`
- [x] property @since — `property-since`
- [x] property @deprecated — `property-deprecated`
- [x] property @param — `property-param`
- [x] property @returns — `property-returns`

## Inline constructs

- [x] bold (__text__) — `bold`
- [x] italic / emphasis (/text/) — `italic`
- [x] monospace / code (@text@ or `text`) — `code-inline`
- [x] link ("text"<url>) — `link`
- [x] bare URL (<http://...>) — `rare-link-bare`
- [x] identifier reference ('ident') — `rare-identifier`
- [x] module link ("Module.Name") — `module-link`
- [x] string gap / special chars — `special-chars`

## Composition (integration)

- [x] bold inside list item — `bold-in-list`
- [x] code inside paragraph — `code-in-paragraph`
- [x] link inside bold — `link-in-bold`
- [x] definition list with inline markup in description — `deflist-inline-markup`
- [x] nested lists — `nested-lists`

## Adversarial

- [x] empty document — `adv-empty`
- [x] unclosed bold markup — `adv-unclosed-bold`
- [x] unknown @ command — `adv-unknown-command`
- [x] malformed identifier reference — `adv-malformed-ident`
- [x] bird-track code with trailing spaces — `adv-bird-trailing-spaces`

## Pathological

- [x] very long paragraph (>64 KB) — `path-long-paragraph`
- [x] deeply nested lists — `path-deeply-nested-lists`
- [x] large definition list (hundreds of entries) — `path-large-deflist`
- [x] very long identifier reference — `path-long-ident-ref`
