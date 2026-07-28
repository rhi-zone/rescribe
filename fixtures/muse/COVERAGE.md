# Muse Fixture Coverage

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
- [N/A] heading anchor / id — Muse does not have a dedicated heading anchor syntax;
  anchors are placed as `<anchor name>` inline elements independent of headings.
- [x] link title — `link-title`
- [x] image alt text — `image` (via [[img][alt]] syntax)
- [x] code block language — `src-block`
- [x] document header directives (#title, #author, #date, #desc, #keywords) — `document-header`
- [N/A] table column alignment — Muse simple table syntax (| col |) does not define
  column alignment; alignment is presentation-only and not encoded in the markup.
- [N/A] tag attributes (style=, class=) — Muse does not define generic HTML-style
  attributes on block tags; the tag set is fixed (<verse>, <center>, <right>, etc.)
  with no attribute syntax.

## Composition (integration)
- [x] heading followed by list — `heading-followed-by-list`
- [x] nested lists — `nested-lists`
- [x] blockquote containing a list — `blockquote-list`
- [x] list item containing inline code — `list-inline-code`
- [x] verse block with multiple stanzas — `verse-multi-stanza`
- [x] link inside bold — `link-in-bold`
- [x] definition list inside blockquote — `deflist-in-blockquote`

## Adversarial
- [x] empty document — `adv-empty`
- [x] unmatched inline delimiter — `adv-unmatched`
- [x] unclosed tag block — `adv-unclosed-tag`
- [x] link with missing closing bracket — `adv-missing-bracket`
- [x] footnote reference to undefined label — `adv-undef-footnote`

## Pathological
- [x] document with many sections — `path-many-sections`
- [x] very large table — `path-large-table`
- [x] deeply nested lists — `path-deep-lists`
- [x] very long paragraph — `path-long-paragraph`
- [x] many footnotes — `path-many-footnotes`
