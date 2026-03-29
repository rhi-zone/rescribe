# Texinfo Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

Reference: GNU Texinfo manual — https://www.gnu.org/software/texinfo/manual/texinfo/

## Block constructs

- [x] paragraph — `paragraph`
- [x] chapter heading (@chapter) — `heading`
- [x] section heading (@section) — `heading-h2`
- [x] @subsection — `heading-h3`
- [x] @subsubsection — `heading-h4`
- [x] @unnumbered / @appendix variants — `heading-unnumbered`, `heading-appendix`
- [x] unordered list (@itemize) — `list-unordered`
- [x] ordered list (@enumerate) — `list-ordered`
- [x] definition list (@table) — `definition-list`
- [x] code / verbatim block (@example) — `code-block`
- [x] verbatim block (@verbatim) — `rare-verbatim`
- [x] @smallexample — `smallexample`
- [x] @lisp block — `lisp-block`
- [x] @display block — `display-block`
- [x] @format block — `format-block`
- [x] quotation block (@quotation) — `rare-blockquote`
- [x] @float (figure/table with caption) — `float`
- [x] @multitable (multi-column table) — `multitable`
- [x] @menu — `menu`
- [x] @direntry — (skipped by parser, no fixture needed)
- [x] @noindent — `noindent`

## Inline constructs

- [x] bold (@strong{}) — `bold`
- [x] italic / emphasis (@emph{}) — `italic`
- [x] inline code (@code{}) — `code-inline`
- [x] hyperlink / @uref{} — `link`
- [x] line break (@* or @sp) — `line-break`
- [x] subscript (@sub{}) — `subscript`
- [x] superscript (@sup{}) — `superscript`
- [x] footnote (@footnote{}) — `footnote-def`
- [x] @file{} — `file-inline`
- [x] @kbd{} — `kbd-inline`
- [x] @key{} — `key-inline`
- [x] @samp{} — `samp-inline`
- [x] @var{} — `var-inline`
- [x] @acronym{} — `acronym-inline`
- [x] @abbr{} — `abbr-inline`
- [x] @cite{} — `cite-inline`
- [x] @env{} — `env-inline`
- [x] @command{} — `command-inline`
- [x] @dfn{} — `dfn-inline`
- [x] @option{} — `option-inline`
- [x] @r{} (Roman font) — `roman-inline`
- [x] @sc{} (small caps) — `smallcaps-inline`
- [x] @i{} / @b{} / @t{} direct font commands — `direct-font`
- [x] @tie{} (non-breaking space) — `tie-symbol`
- [x] @dots{} / @enddots{} — `dots-symbol`
- [x] @xref{} / @ref{} / @pxref{} (cross-reference) — `xref-crossref`
- [x] @anchor{} — `anchor-inline`
- [x] @w{} (no-break) — `nobreak-inline`
- [x] @image{} — `image-inline`
- [x] @email{} — `email-inline`
- [x] @copyright{} / @registeredsymbol{} / @minus{} — `copyright-symbol`

## Structuring / Header commands

- [x] @setfilename — (skipped by parser, no fixture needed)
- [x] @settitle — `settitle-header`
- [x] @author — (skipped by parser, no fixture needed)
- [x] @copying / @end copying — (skipped by parser, no fixture needed)
- [x] @titlepage — (skipped by parser, no fixture needed)
- [x] @node — `node-header`
- [x] @top — (skipped by parser, no fixture needed)
- [x] @ifinfo / @end ifinfo — `iftex-conditional` (all conditional blocks handled same)
- [x] @ifhtml / @end ifhtml — (same as @iftex)
- [x] @iftex / @end iftex — `iftex-conditional`
- [x] @set / @value{} — (skipped by parser, no fixture needed)
- [x] @include — (skipped by parser, no fixture needed)

## Properties

- [x] heading level — `heading`, `heading-h2`, `heading-h3`, `heading-h4`
- [x] ordered list start number — `list-ordered`
- [x] link URL vs display text — `link`
- [x] footnote content — `footnote-def`

## Composition (integration)

- [x] bold inside list item — `comp-bold-in-list`
- [x] code block inside quotation — `comp-code-in-quote`
- [x] footnote inside heading — `comp-footnote-in-heading`
- [x] nested lists — `comp-nested-lists`
- [x] inline markup inside definition list — `comp-inline-in-deflist`

## Adversarial

- [x] empty document — `adv-empty`
- [x] unknown @-command — `adv-unknown-command`
- [x] unclosed @{} group — `adv-unclosed-group`
- [x] unterminated inline command (@samp{ with no }) — `adv-unterminated-inline`
- [x] @end for unknown environment — `adv-unknown-end`
- [x] mismatched @end — `adv-mismatched-end`

## Pathological

- [x] very long paragraph (>64 KB) — `path-long-paragraph`
- [x] deeply nested @itemize — `path-deep-nesting`
- [x] @multitable with many columns — `path-many-columns`
- [x] deeply nested @quotation blocks — `path-deep-quotation`
