# Muse Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

Muse refers to Emacs Muse (also used by Ikiwiki). The reference spec is the Emacs Muse
manual.

## Block constructs
- [x] paragraph — `paragraph`
- [x] heading h1 (* Heading) — `heading`
- [x] heading h2 (** Heading) — `heading-h2`
- [ ] heading h3–h4 (*** / ****) — (missing)
- [x] unordered list (- item) — `list-unordered`
- [x] ordered list (1. item) — `list-ordered`
- [x] definition list (term :: definition) — `definition-list`
- [x] blockquote (two-space indent) — `blockquote`
- [x] extended blockquote (six-space indent / <quote>) — `rare-blockquote`
- [x] code block (<example> … </example>) — `code-block`
- [x] verse block (<verse> … </verse>) — `verse-block`
- [x] horizontal rule (---- ) — `horizontal-rule`
- [ ] centered block (<center> … </center>) — (missing)
- [ ] right-aligned block (<right> … </right>) — (missing)
- [ ] literal block (<literal> … </literal>) — (missing)
- [ ] src block (<src lang="…"> … </src>) — (missing)
- [ ] comment (;; text or <comment> … </comment>) — (missing)
- [ ] table (simple | | | syntax) — (missing)
- [ ] footnote definition ([1] text) — (missing)

## Inline constructs
- [x] italic (*text*) — `italic`
- [x] bold (**text**) — `bold`
- [x] inline code (=text=) — `code-inline`
- [x] link ([[url][desc]] or [[url]]) — `link`
- [x] bare URL link — `rare-link-bare`
- [ ] underline (_text_) — (missing)
- [ ] superscript (<sup>text</sup>) — (missing)
- [ ] subscript (<sub>text</sub>) — (missing)
- [ ] strikethrough (~~text~~) — (missing)
- [ ] footnote reference ([1]) — (missing)
- [ ] line break (<br>) — (missing)
- [ ] anchor (<anchor id>) — (missing)
- [ ] image ([[file.png]]) — (missing)
- [ ] inline literal (=text=) — (missing; covered by code-inline)

## Properties
- [ ] heading anchor / id — (missing)
- [ ] link title — (missing)
- [ ] image alt text — (missing)
- [ ] code block language — (missing)
- [ ] document header directives (#title, #author, #date, #desc, #keywords) — (missing)
- [ ] table column alignment — (missing)
- [ ] tag attributes (style=, class=) — (missing)

## Composition (integration)
- [ ] heading followed by list — (missing)
- [ ] nested lists — (missing)
- [ ] blockquote containing a list — (missing)
- [ ] list item containing inline code — (missing)
- [ ] verse block with multiple stanzas — (missing)
- [ ] link inside bold — (missing)
- [ ] definition list inside blockquote — (missing)

## Adversarial
- [x] empty document — `adv-empty`
- [x] unmatched inline delimiter — `adv-unmatched`
- [ ] unclosed tag block — (missing)
- [ ] link with missing closing bracket — (missing)
- [ ] footnote reference to undefined label — (missing)

## Pathological
- [ ] document with many sections — (missing)
- [ ] very large table — (missing)
- [ ] deeply nested lists — (missing)
- [ ] very long paragraph — (missing)
- [ ] many footnotes — (missing)
