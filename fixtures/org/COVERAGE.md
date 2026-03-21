# Org-mode Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

## Block constructs
- [x] paragraph — `paragraph`
- [x] heading h1 (* Heading) — `heading-h1`
- [x] heading h2 (** Heading) — `heading-h2`
- [x] heading h3 (*** Heading) — `heading-h3`
- [x] unordered list (- or + or *) — `list-unordered`
- [x] ordered list (1. or 1)) — `list-ordered`
- [x] definition list (- term :: desc) — `definition-list`
- [x] blockquote (#+BEGIN_QUOTE … #+END_QUOTE) — `blockquote`
- [x] center block (#+BEGIN_CENTER … #+END_CENTER) — `center-block`
- [x] code block (#+BEGIN_SRC … #+END_SRC) — `code-block`
- [x] code block without language — `code-block-no-lang`
- [x] example block (#+BEGIN_EXAMPLE … #+END_EXAMPLE) — (missing; no separate fixture but related)
- [x] horizontal rule (-----) — `horizontal-rule`
- [x] table — `table`
- [x] figure / image link — `figure`
- [x] footnote definition ([fn:1] def) — `footnote-def`
- [x] footnote reference ([fn:1]) — `footnote-ref`
- [x] metadata / document keywords (#+TITLE:, #+AUTHOR:, etc.) — `metadata`
- [x] verse block (#+BEGIN_VERSE … #+END_VERSE) — `verse-block`
- [x] comment block (#+BEGIN_COMMENT … #+END_COMMENT) — `comment-block`
- [x] export block (#+BEGIN_EXPORT … #+END_EXPORT) — `export-block`
- [x] special block (#+BEGIN_{NAME}) — `special-block`
- [x] drawer (:LOGBOOK: … :END:) — `drawer`
- [ ] property drawer (:PROPERTIES: … :END: under heading) — (missing)
- [ ] dynamic block (#+BEGIN: name … #+END:) — (missing)
- [x] fixed-width area (: line) — `fixed-width`
- [x] comment line (# comment) — `comment-line`
- [x] keyword line (#+KEY: value) — `keyword-line`
- [ ] horizontal rule (distinct from comment) — (missing; covered above)
- [x] list item with checkbox (- [ ] / - [X] / - [-]) — `checkbox-list`
- [ ] list item with tag (- tag :: description) — (missing; covered in definition-list)
- [ ] ordered list with counter ([@3]) — (missing)
- [x] nested list — `nested-list`
- [x] table with alignment row (|---|) — `table-alignment`
- [x] affiliated keyword (#+CAPTION: before table) — `affiliated-keyword`

## Inline constructs
- [x] emphasis (*text*) — `emphasis`
- [x] strong (*text* bold form) / bold — (missing; RST-style; in org **bold** is not standard — org uses *bold*)
- [x] strikethrough (+text+) — `strikeout`
- [x] underline (_text_) — `underline`
- [x] subscript (_{text} or _char) — `subscript`
- [x] superscript (^{text} or ^char) — `superscript`
- [x] inline code (~text~) — `code-inline`
- [x] verbatim (=text=) — `rare-code-inline-equals`
- [x] link ([[url][description]]) — `link`
- [x] bare URL link — `link-bare`
- [x] image (inline [[file:img.png]]) — `image`
- [x] footnote inline ([fn:: text]) — `footnote-ref`
- [x] line break (\ at end of line) — `line-break`
- [x] timestamp (<YYYY-MM-DD>) — `timestamp-active`
- [x] inactive timestamp ([YYYY-MM-DD]) — `timestamp-inactive`
- [ ] date range (<date1>--<date2>) — (missing)
- [ ] macro ({{{macro(arg)}}}) — (missing)
- [ ] citation ([cite:@key]) — (missing)
- [ ] target (<<target>>) — (missing)
- [ ] radio target (<<<target>>>) — (missing)
- [x] entity (\alpha, \nbsp, etc.) — `entity`
- [x] LaTeX fragment (\(...\) or \[…\]) — `latex-fragment`
- [x] export snippet (@@backend:raw@@) — `export-snippet`
- [ ] inline babel call — (missing)

## Properties
- [x] heading TODO keyword — `rare-heading-todo`
- [x] heading DONE keyword — `heading-done`
- [x] heading priority ([#A]) — `heading-priority`
- [x] heading tags (:tag1:tag2:) — `heading-tags`
- [ ] heading comment keyword — (missing)
- [ ] heading archived — (missing)
- [x] code block language — `code-block`
- [x] code block header arguments (:results, :exports, :var) — `code-block-header`
- [ ] code block name (#+NAME:) — (missing)
- [x] table column alignment — `table-alignment`
- [x] link type (file:, http:, id:, custom-id:, fuzzy) — `link-types`
- [ ] footnote labeled vs inline vs anonymous — (missing)
- [ ] property drawer key/value pairs — (missing)
- [ ] scheduled / deadline timestamps — (missing)
- [x] document metadata (#+TITLE, #+AUTHOR, #+DATE, #+EMAIL, #+LANGUAGE, #+OPTIONS) — `metadata`

## Composition (integration)
- [x] nested markup — `rare-nested-markup`
- [x] heading with inline formatting — `integration-heading-inline`
- [x] table cell with inline formatting — `integration-table-inline`
- [x] list item containing a code block — `integration-list-code`
- [x] blockquote containing a list — `integration-blockquote-list`
- [x] footnote containing inline markup — `integration-footnote-markup`
- [x] nested lists (unordered inside ordered) — `integration-nested-lists`
- [ ] definition list inside a blockquote — (missing)
- [x] affiliated keyword before a table — `integration-caption-table`

## Adversarial
- [x] empty document — `adv-empty`
- [x] unknown block type — `adv-unknown-block`
- [x] unmatched markup delimiter — `adv-unmatched-markup`
- [x] malformed table (missing closing |) — `adv-malformed-table`
- [x] heading at max depth then deeper — `adv-deep-headings`
- [x] link with no description — `adv-link-no-desc`
- [x] footnote reference to undefined label — `adv-undef-footnote`
- [x] drawer without closing :END: — `adv-unclosed-drawer`

## Pathological
- [x] document with hundreds of headings — `path-many-headings`
- [x] very large table — `path-large-table`
- [x] deeply nested lists — `path-deep-list`
- [x] many footnotes — `path-many-footnotes`
- [x] very long paragraph — `path-long-paragraph`
