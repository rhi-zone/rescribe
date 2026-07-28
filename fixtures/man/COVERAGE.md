# Man (troff/groff) Fixture Coverage

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

Scope: troff/groff man macro set (man(7)) as used for Unix manual pages.

## Block constructs (man macros)

- [x] paragraph (.PP / .P) — `paragraph`
- [x] section heading (.SH) — `heading`
- [x] subsection heading (.SS) — `heading-ss`
- [x] definition list / tagged paragraph (.TP) — `definition-list`
- [x] code block / literal block (.nf / .fi) — `code-block`
- [x] horizontal rule (.sp with rule) — `horizontal-rule`
- [x] indented paragraph (.IP) — `indented-para`
- [ ] relative indent start/end (.RS / .RE) — (modeled as skip; tested in `path-deep-rs-re`)
- [x] example block (.EX / .EE, groff extension) — `example-block`
- [ ] synopsis block (.SY / .OP / .YS, groff extension) — (missing)
- [ ] synopsis section convention (NAME, SYNOPSIS, etc.) — (missing)

## Inline constructs (font/style requests and macros)

- [x] bold (.B standalone paragraph) — `bold`
- [x] italic (.I standalone paragraph) — `italic`
- [x] inline bold (.B inline via font escape \fB...\fR) — `inline-bold`
- [x] inline italic (.I inline via font escape \fI...\fR) — `inline-italic`
- [x] alternating bold/italic (.BI, .IB, .BR, .RB, .RI, .IR macros) — `rare-alternating`
- [x] hyperlink / URL (.UR / .UE) — `link`
- [x] inline code / monospace (\f(CW ... \fR) — `inline-code`
- [ ] small caps — (no native man syntax)
- [ ] superscript — (modeled in AST; no dedicated fixture — no native man syntax)
- [ ] subscript — (modeled in AST; no dedicated fixture — no native man syntax)

## Font escapes

- [x] \fB (bold) / \fR (roman) — (covered via `inline-bold`)
- [x] \fI (italic) — (covered via `inline-italic`)
- [x] \f(CW (constant width) — `inline-code`
- [ ] \fP (previous font) — (missing)
- [ ] \f[fontname] (groff named font) — (missing)

## Special character escapes

- [x] \(em (em dash) — `special-chars`
- [x] \(en (en dash) — `special-chars`
- [x] \(co (copyright) — `special-chars`
- [x] \(rg (registered) — `special-chars`
- [x] \e (backslash) — `special-chars`
- [ ] \~ (non-breaking space) — (modeled in parser; no dedicated fixture)
- [x] \& (zero-width non-joiner) — (modeled in parser as skip)

## Metadata / comments

- [x] .TH (title header: name, section, date, source, manual) — `th-header`
- [x] .\" (comment line) — `comment-line`

## Composition (integration)

- [x] definition list with inline bold in term — `comp-deflist-bold`
- [ ] code block inside indented paragraph — (missing)
- [x] URL with descriptive text — `comp-url-text`
- [x] multiple sections in one document — `comp-multi-section`

## Adversarial

- [x] empty document — `adv-empty`
- [x] unknown macro (.foo) — `adv-unknown-macro`
- [x] unclosed font escape (\fB with no \fR) — `adv-unclosed-font`
- [x] macro with too many arguments — `adv-too-many-args`
- [x] bare troff request (.) — `adv-bare-troff`

## Pathological

- [x] very long line (>64 KB) — `path-long-line`
- [x] deeply nested .RS / .RE — `path-deep-rs-re`
- [x] definition list with hundreds of entries — `path-many-defs`
