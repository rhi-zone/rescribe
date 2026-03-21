# LaTeX Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

Scope: constructs that Pandoc/rescribe actually reads from LaTeX. The full LaTeX universe
is out of scope; this covers the document markup constructs a reader must handle.

## Block constructs

- [x] paragraph — `paragraph`
- [x] heading (\section) — `heading`
- [x] heading level 2 (\subsection) — `heading-h2`
- [ ] heading level 3 (\subsubsection) — (missing)
- [ ] heading level 4 (\paragraph) — (missing)
- [ ] heading level 5 (\subparagraph) — (missing)
- [x] unordered list (itemize) — `list-unordered`
- [x] ordered list (enumerate) — `list-ordered`
- [ ] description list (description environment) — (missing)
- [x] code block (verbatim environment) — `code-block`
- [x] lstlisting environment — `rare-lstlisting`
- [ ] minted environment — (missing)
- [x] blockquote (quote / quotation environment) — `blockquote`
- [x] figure environment — `figure`
- [x] table (tabular environment) — `table`
- [ ] longtable environment — (missing)
- [x] horizontal rule (\hrule / \rule) — `horizontal-rule`
- [x] display math (equation / displaymath / $$...$$) — `math-display`
- [ ] align environment — (missing)
- [ ] abstract environment — (missing)
- [ ] center environment — (missing)
- [ ] flushleft / flushright environments — (missing)
- [ ] minipage environment — (missing)

## Inline constructs

- [x] bold (\textbf) — `bold`
- [x] italic (\textit) — `italic`
- [x] underline (\underline) — `underline`
- [x] strikethrough (\sout / \st) — `strikeout`
- [x] small caps (\textsc) — `small-caps`
- [x] subscript (\textsubscript / $_{}$) — `subscript`
- [x] superscript (\textsuperscript / $^{}$) — `superscript`
- [x] inline code (\texttt / \verb) — `code-inline`
- [x] emphasis (\emph) — `rare-emph`
- [x] hyperlink (\href) — `link`
- [x] URL (\url) — `rare-url`
- [x] inline math ($...$) — `math-inline`
- [ ] footnote (\footnote) — (missing)
- [ ] cite (\cite) — (missing)
- [ ] label (\label) — (missing)
- [ ] ref (\ref) — (missing)
- [ ] image (\includegraphics) — (missing; figure fixture tests the environment, not inline img)
- [ ] line break (\\\\) — `line-break`
- [ ] non-breaking space (~) — (missing)
- [ ] em dash (---) / en dash (--) — (missing)
- [ ] special characters (\&, \$, \%, \#, etc.) — (missing)

## Environments / Commands

- [x] preamble (\documentclass, \usepackage, etc.) — `rare-preamble`
- [ ] \begin{document} / \end{document} — (covered in preamble, no dedicated fixture)
- [ ] \maketitle — (missing)
- [ ] \tableofcontents — (missing)
- [ ] custom environment (unknown \begin{foo}) — `adv-unknown-env`
- [ ] \input / \include — (missing)
- [ ] \newcommand / \renewcommand — (missing)

## Metadata

- [ ] \title — (missing)
- [ ] \author — (missing)
- [ ] \date — (missing)

## Composition (integration)

- [ ] bold inside list item — (missing)
- [ ] table with inline formatting in cells — (missing)
- [ ] footnote inside paragraph — (missing)
- [ ] math inside heading — (missing)
- [ ] nested lists — (missing)
- [ ] figure with caption — (missing)

## Adversarial

- [x] empty document — `adv-empty`
- [x] unknown environment — `adv-unknown-env`
- [ ] unclosed environment (\begin{foo} with no \end{foo}) — (missing)
- [ ] unclosed brace group — (missing)
- [ ] unknown command (\foobar) — (missing)
- [ ] malformed math — (missing)

## Pathological

- [ ] very long paragraph (>64 KB) — (missing)
- [ ] deeply nested environments — (missing)
- [ ] table with many columns — (missing)
- [ ] deeply nested lists — (missing)
