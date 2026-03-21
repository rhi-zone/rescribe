# Texinfo Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

Reference: GNU Texinfo manual — https://www.gnu.org/software/texinfo/manual/texinfo/

## Block constructs

- [x] paragraph — `paragraph`
- [x] chapter heading (@chapter) — `heading`
- [x] section heading (@section) — `heading-h2`
- [ ] @subsection — (missing)
- [ ] @subsubsection — (missing)
- [ ] @unnumbered / @appendix variants — (missing)
- [x] unordered list (@itemize) — `list-unordered`
- [x] ordered list (@enumerate) — `list-ordered`
- [x] definition list (@table) — `definition-list`
- [x] code / verbatim block (@example) — `code-block`
- [x] verbatim block (@verbatim) — `rare-verbatim`
- [ ] @smallexample — (missing)
- [ ] @lisp block — (missing)
- [ ] @display block — (missing)
- [ ] @format block — (missing)
- [x] quotation block (@quotation) — `rare-blockquote`
- [ ] @float (figure/table with caption) — (missing)
- [ ] @multitable (multi-column table) — (missing)
- [ ] @menu — (missing)
- [ ] @direntry — (missing)

## Inline constructs

- [x] bold (@strong{}) — `bold`
- [x] italic / emphasis (@emph{}) — `italic`
- [x] inline code (@code{}) — `code-inline`
- [x] hyperlink / @uref{} — `link`
- [x] line break (@* or @sp) — `line-break`
- [x] subscript (@sub{}) — `subscript`
- [x] superscript (@sup{}) — `superscript`
- [x] footnote (@footnote{}) — `footnote-def`
- [ ] @file{} — (missing)
- [ ] @kbd{} — (missing)
- [ ] @key{} — (missing)
- [ ] @samp{} — (missing)
- [ ] @var{} — (missing)
- [ ] @acronym{} — (missing)
- [ ] @abbr{} — (missing)
- [ ] @cite{} — (missing)
- [ ] @env{} — (missing)
- [ ] @command{} — (missing)
- [ ] @dfn{} — (missing)
- [ ] @option{} — (missing)
- [ ] @r{} (Roman font) — (missing)
- [ ] @sc{} (small caps) — (missing)
- [ ] @i{} / @b{} / @t{} direct font commands — (missing)
- [ ] @tie{} (non-breaking space) — (missing)
- [ ] @dots{} / @enddots{} — (missing)
- [ ] @xref{} / @ref{} / @pxref{} (cross-reference) — (missing)
- [ ] @anchor{} — (missing)

## Structuring / Header commands

- [ ] @setfilename — (missing)
- [ ] @settitle — (missing)
- [ ] @author — (missing)
- [ ] @copying / @end copying — (missing)
- [ ] @titlepage — (missing)
- [ ] @node — (missing)
- [ ] @top — (missing)
- [ ] @ifinfo / @end ifinfo — (missing)
- [ ] @ifhtml / @end ifhtml — (missing)
- [ ] @iftex / @end iftex — (missing)
- [ ] @set / @value{} — (missing)
- [ ] @include — (missing)

## Properties

- [ ] heading level — (heading/heading-h2 fixtures cover two levels)
- [ ] ordered list start number — (missing)
- [ ] link URL vs display text — (missing)
- [ ] footnote content — `footnote-def`

## Composition (integration)

- [ ] bold inside list item — (missing)
- [ ] code block inside quotation — (missing)
- [ ] footnote inside heading — (missing)
- [ ] nested lists — (missing)
- [ ] inline markup inside definition list — (missing)

## Adversarial

- [x] empty document — `adv-empty`
- [ ] unknown @-command — (missing)
- [ ] unclosed @{} group — (missing)
- [ ] unterminated inline command (@samp{ with no }) — (missing)
- [ ] @end for unknown environment — (missing)
- [ ] mismatched @end — (missing)

## Pathological

- [ ] very long paragraph (>64 KB) — (missing)
- [ ] deeply nested @itemize — (missing)
- [ ] @multitable with many columns — (missing)
- [ ] deeply nested @quotation blocks — (missing)
