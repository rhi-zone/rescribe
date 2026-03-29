# Muse Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

Muse refers to Emacs Muse (also used by Ikiwiki). The reference spec is the Emacs Muse
manual.

## Block constructs
- [x] paragraph — `paragraph`
- [x] heading h1 (* Heading) — `heading`
- [x] heading h2 (** Heading) — `heading-h2`
- [x] heading h3–h4 (*** / ****) — `heading-h3-h4`
- [x] unordered list (- item) — `list-unordered`
- [x] ordered list (1. item) — `list-ordered`
- [x] definition list (term :: definition) — `definition-list`
- [x] blockquote (two-space indent) — `blockquote`
- [x] extended blockquote (six-space indent / <quote>) — `rare-blockquote`
- [x] code block (<example> … </example>) — `code-block`
- [x] verse block (<verse> … </verse>) — `verse-block`
- [x] horizontal rule (---- ) — `horizontal-rule`
- [x] centered block (<center> … </center>) — `center-right`
- [x] right-aligned block (<right> … </right>) — `center-right`
- [x] literal block (<literal> … </literal>) — `literal-block`
- [x] src block (<src lang="…"> … </src>) — `src-block`
- [x] comment (;; text or <comment> … </comment>) — `comment`
- [x] table (simple | | | syntax) — `table`
- [x] footnote definition ([1] text) — `footnote`

## Inline constructs
- [x] italic (*text*) — `italic`
- [x] bold (**text**) — `bold`
- [x] inline code (=text=) — `code-inline`
- [x] link ([[url][desc]] or [[url]]) — `link`
- [x] bare URL link — `rare-link-bare`
- [x] underline (_text_) — `underline`
- [x] superscript (^text^ or <sup>text</sup>) — `superscript-subscript`
- [x] subscript (<sub>text</sub>) — `superscript-subscript`
- [x] strikethrough (~~text~~) — `strikethrough`
- [x] footnote reference ([1]) — `footnote`
- [x] line break (<br>) — `line-break`
- [x] anchor (<anchor id>) — `anchor`
- [x] image ([[file.png]]) — `image`
- [x] inline literal (=text=) — covered by `code-inline`

## Properties
- [ ] heading anchor / id — (missing)
- [ ] link title — (missing)
- [x] image alt text — `image` (via [[img][alt]] syntax)
- [x] code block language — `src-block`
- [x] document header directives (#title, #author, #date, #desc, #keywords) — `document-header`
- [ ] table column alignment — (missing; Muse does not define column alignment)
- [ ] tag attributes (style=, class=) — (missing; Muse does not define generic attributes)

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
