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
- [ ] verse block (#+BEGIN_VERSE … #+END_VERSE) — (missing)
- [ ] comment block (#+BEGIN_COMMENT … #+END_COMMENT) — (missing)
- [ ] export block (#+BEGIN_EXPORT … #+END_EXPORT) — (missing)
- [ ] special block (#+BEGIN_{NAME}) — (missing)
- [ ] drawer (:PROPERTIES: … :END:) — (missing)
- [ ] property drawer (:PROPERTIES: … :END: under heading) — (missing)
- [ ] dynamic block (#+BEGIN: name … #+END:) — (missing)
- [ ] fixed-width area (: line) — (missing)
- [ ] comment line (# comment) — (missing)
- [ ] keyword line (#+KEY: value) — (missing)
- [ ] horizontal rule (distinct from comment) — (missing; covered above)
- [ ] list item with checkbox (- [ ] / - [X] / - [-]) — (missing)
- [ ] list item with tag (- tag :: description) — (missing; covered in definition-list)
- [ ] ordered list with counter ([@3]) — (missing)
- [ ] nested list — (missing)
- [ ] table with alignment row (|---|) — (missing)
- [ ] affiliated keyword (#+CAPTION:, #+NAME:, #+ATTR_HTML:) — (missing)

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
- [ ] line break (\ at end of line) — (missing)
- [ ] timestamp (<YYYY-MM-DD>) — (missing)
- [ ] inactive timestamp ([YYYY-MM-DD]) — (missing)
- [ ] date range (<date1>--<date2>) — (missing)
- [ ] macro ({{{macro(arg)}}}) — (missing)
- [ ] citation ([cite:@key]) — (missing)
- [ ] target (<<target>>) — (missing)
- [ ] radio target (<<<target>>>) — (missing)
- [ ] entity (\alpha, \nbsp, etc.) — (missing)
- [ ] LaTeX fragment (\(...\) or \[…\]) — (missing)
- [ ] export snippet (@@backend:raw@@) — (missing)
- [ ] inline babel call — (missing)

## Properties
- [x] heading TODO keyword — `rare-heading-todo`
- [ ] heading DONE keyword — (missing)
- [ ] heading priority ([#A]) — (missing)
- [ ] heading tags (:tag1:tag2:) — (missing)
- [ ] heading comment keyword — (missing)
- [ ] heading archived — (missing)
- [x] code block language — `code-block`
- [ ] code block header arguments (:results, :exports, :var) — (missing)
- [ ] code block name (#+NAME:) — (missing)
- [ ] table column alignment — (missing)
- [ ] link type (file:, http:, id:, custom-id:, fuzzy) — (missing)
- [ ] footnote labeled vs inline vs anonymous — (missing)
- [ ] property drawer key/value pairs — (missing)
- [ ] scheduled / deadline timestamps — (missing)
- [ ] document metadata (#+TITLE, #+AUTHOR, #+DATE, #+EMAIL, #+LANGUAGE, #+OPTIONS) — `metadata`

## Composition (integration)
- [x] nested markup — `rare-nested-markup`
- [ ] heading with inline formatting — (missing)
- [ ] table cell with inline formatting — (missing)
- [ ] list item containing a code block — (missing)
- [ ] blockquote containing a list — (missing)
- [ ] footnote containing inline markup — (missing)
- [ ] nested lists (unordered inside ordered) — (missing)
- [ ] definition list inside a blockquote — (missing)
- [ ] affiliated keyword before a table — (missing)

## Adversarial
- [x] empty document — `adv-empty`
- [x] unknown block type — `adv-unknown-block`
- [x] unmatched markup delimiter — `adv-unmatched-markup`
- [ ] malformed table (missing closing |) — (missing)
- [ ] heading at max depth then deeper — (missing)
- [ ] link with no description — (missing)
- [ ] footnote reference to undefined label — (missing)
- [ ] drawer without closing :END: — (missing)

## Pathological
- [ ] document with hundreds of headings — (missing)
- [ ] very large table — (missing)
- [ ] deeply nested lists — (missing)
- [ ] many footnotes — (missing)
- [ ] very long paragraph — (missing)
