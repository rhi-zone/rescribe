# Man (troff/groff) Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

Scope: troff/groff man macro set (man(7)) as used for Unix manual pages.

## Block constructs (man macros)

- [x] paragraph (.PP / .P) — `paragraph`
- [x] section heading (.SH) — `heading`
- [x] subsection heading (.SS) — `heading-ss`
- [x] definition list / tagged paragraph (.TP) — `definition-list`
- [x] code block / literal block (.nf / .fi) — `code-block`
- [x] horizontal rule (.sp with rule) — `horizontal-rule`
- [ ] indented paragraph (.IP) — (missing)
- [ ] relative indent start/end (.RS / .RE) — (missing)
- [ ] example block (.EX / .EE, groff extension) — (missing)
- [ ] synopsis block (.SY / .OP / .YS, groff extension) — (missing)
- [ ] synopsis section convention (NAME, SYNOPSIS, etc.) — (missing)

## Inline constructs (font/style requests and macros)

- [x] bold (.B standalone paragraph) — `bold`
- [x] italic (.I standalone paragraph) — `italic`
- [x] inline bold (.B inline via font escape \fB...\fR) — `inline-bold`
- [x] inline italic (.I inline via font escape \fI...\fR) — `inline-italic`
- [x] alternating bold/italic (.BI, .IB, .BR, .RB, .RI, .IR macros) — `rare-alternating`
- [x] hyperlink / URL (.UR / .UE) — `link`
- [ ] inline code / monospace (\f(CW ... \fR) — (missing)
- [ ] small caps — (missing)
- [ ] superscript — (missing)
- [ ] subscript — (missing)

## Font escapes

- [ ] \fB (bold) / \fR (roman) — (covered via inline-bold, no dedicated escape fixture)
- [ ] \fI (italic) — (covered via inline-italic)
- [ ] \f(CW (constant width) — (missing)
- [ ] \fP (previous font) — (missing)
- [ ] \f[fontname] (groff named font) — (missing)

## Special character escapes

- [ ] \(em (em dash) — (missing)
- [ ] \(en (en dash) — (missing)
- [ ] \(co (copyright) — (missing)
- [ ] \(rg (registered) — (missing)
- [ ] \e (backslash) — (missing)
- [ ] \~ (non-breaking space) — (missing)
- [ ] \& (zero-width non-joiner) — (missing)

## Metadata / comments

- [ ] .TH (title header: name, section, date, source, manual) — (missing)
- [ ] .\" (comment line) — (missing)

## Composition (integration)

- [ ] definition list with inline bold in term — (missing)
- [ ] code block inside indented paragraph — (missing)
- [ ] URL with descriptive text — (missing)
- [ ] multiple sections in one document — (missing)

## Adversarial

- [x] empty document — `adv-empty`
- [ ] unknown macro (.foo) — (missing)
- [ ] unclosed font escape (\fB with no \fR) — (missing)
- [ ] macro with too many arguments — (missing)
- [ ] bare troff request (.) — (missing)

## Pathological

- [ ] very long line (>64 KB) — (missing)
- [ ] deeply nested .RS / .RE — (missing)
- [ ] definition list with hundreds of entries — (missing)
