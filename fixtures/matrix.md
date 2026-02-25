# Fixture Coverage Matrix

This file is the canonical definition of what "100% fixture coverage" means.
Every row represents a (format, construct) pair that must have a passing fixture.

100% pass rate on all fixtures = 100% implementation support for that format.

See `spec.md` for the fixture file format.

## How to read this

- **Fixture**: directory name under `fixtures/{format}/`
- **Category**: `happy` (normal), `rare` (valid but uncommon), `adversarial` (malformed/extreme)
- **Status**: ✓ done · — pending

---

## markdown

Readers: CommonMark + GFM extensions (tables, task lists, strikethrough, footnotes, YAML frontmatter).

### Block constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| paragraph | `paragraph` | happy | ✓ |
| heading h1 | `heading` | happy | ✓ |
| heading h2 | `heading-h2` | happy | ✓ |
| heading h3 | `heading-h3` | happy | ✓ |
| heading h4 | `heading-h4` | happy | ✓ |
| heading h5 | `heading-h5` | happy | ✓ |
| heading h6 | `heading-h6` | happy | ✓ |
| fenced code block (with language) | `code-block-with-lang` | happy | ✓ |
| fenced code block (no language) | `code-block-no-lang` | happy | ✓ |
| blockquote | `blockquote` | happy | ✓ |
| unordered list | `list-unordered` | happy | ✓ |
| ordered list | `list-ordered` | happy | ✓ |
| nested list | `list-nested` | happy | ✓ |
| GFM table | `table` | happy | ✓ |
| horizontal rule | `horizontal-rule` | happy | ✓ |
| raw HTML block | `raw-html-block` | happy | ✓ |
| task list | `task-list` | happy | ✓ |
| footnote definition | `footnote` | happy | ✓ |
| YAML frontmatter → metadata | `frontmatter-yaml` | happy | ✓ |
| setext heading | `rare-setext-heading` | rare | ✓ |
| ordered list with non-1 start | `rare-ordered-list-start` | rare | ✓ |
| tilde-fenced code block | `rare-fenced-tilde` | rare | ✓ |
| indented code block | `rare-indented-code` | rare | ✓ |

### Inline constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| plain text | `paragraph` | happy | ✓ |
| emphasis (italic) | `italic` | happy | ✓ |
| strong (bold) | `bold` | happy | ✓ |
| strikethrough | `strikeout` | happy | ✓ |
| inline code | `code-inline` | happy | ✓ |
| link | `link` | happy | ✓ |
| image | `image` | happy | ✓ |
| hard line break | `line-break` | happy | ✓ |
| soft break | `soft-break` | happy | ✓ |
| footnote reference | `footnote` | happy | ✓ |
| raw HTML inline | `raw-html-inline` | happy | ✓ |
| link with title | `rare-link-with-title` | rare | ✓ |
| image with title | `rare-image-with-title` | rare | ✓ |
| nested emphasis (bold-italic) | `rare-nested-emphasis` | rare | ✓ |

### Adversarial

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| empty document | `adv-empty` | adversarial | ✓ |
| whitespace-only | `adv-whitespace-only` | adversarial | ✓ |
| unmatched emphasis | `adv-unmatched-emphasis` | adversarial | ✓ |
| unclosed code fence | `adv-unclosed-fence` | adversarial | ✓ |
| broken link syntax | `adv-broken-link` | adversarial | ✓ |
| deeply nested blockquotes | `adv-deeply-nested-blockquotes` | adversarial | ✓ |

### Not yet covered

| Construct | Notes |
|-----------|-------|
| math inline / display | Requires checking which pulldown-cmark option enables it |
| definition list | Not standard CommonMark/GFM; MultiMarkdown extension |
| subscript / superscript | MultiMarkdown extension; not in pulldown-cmark by default |
| TOML frontmatter | `+++` style; low priority |
| figure (block image) | Pandoc-specific; not emitted by pulldown-cmark |

---

## html

Readers: html5ever; handles block/inline classification, data-URI embedding, metadata.

### Block constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| paragraph | `paragraph` | happy | ✓ |
| heading h1 | `heading-h1` | happy | ✓ |
| heading h2 | `heading-h2` | happy | ✓ |
| heading h3 | `heading-h3` | happy | ✓ |
| heading h4 | `heading-h4` | happy | ✓ |
| heading h5 | `heading-h5` | happy | ✓ |
| heading h6 | `heading-h6` | happy | ✓ |
| blockquote | `blockquote` | happy | ✓ |
| ordered list | `list-ordered` | happy | ✓ |
| unordered list | `list-unordered` | happy | ✓ |
| code block (with language) | `code-block` | happy | ✓ |
| code block (no language) | `code-block-no-lang` | happy | ✓ |
| table (thead/tbody) | `table` | happy | ✓ |
| horizontal rule | `horizontal-rule` | happy | ✓ |
| div (with id/class) | `div` | happy | ✓ |
| definition list | `rare-definition-list` | rare | ✓ |
| title metadata | `metadata-title` | happy | ✓ |
| meta tag metadata | `metadata-meta` | happy | ✓ |

### Inline constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| emphasis (`<em>`) | `emphasis` | happy | ✓ |
| strong (`<strong>`) | `strong` | happy | ✓ |
| strikethrough (`<del>`) | `strikeout` | happy | ✓ |
| underline (`<u>`) | `underline` | happy | ✓ |
| inline code (`<code>`) | `code-inline` | happy | ✓ |
| link (`<a>`) | `link` | happy | ✓ |
| image (`<img>`) | `image` | happy | ✓ |
| line break (`<br>`) | `line-break` | happy | ✓ |
| superscript (`<sup>`) | `superscript` | happy | ✓ |
| subscript (`<sub>`) | `subscript` | happy | ✓ |
| link with title | `rare-link-with-title` | rare | ✓ |
| image with title | `rare-image-with-title` | rare | ✓ |
| ordered list with start | `rare-ordered-list-start` | rare | ✓ |

### Adversarial

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| empty document | `adv-empty` | adversarial | ✓ |
| unclosed tags | `adv-unclosed-tags` | adversarial | ✓ |
| script injection | `adv-script-stripped` | adversarial | ✓ |
| deeply nested elements | `adv-deeply-nested` | adversarial | ✓ |
---

## org

Reader: custom hand-rolled org-mode parser.

### Block constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| paragraph | `paragraph` | happy | ✓ |
| heading h1 | `heading-h1` | happy | ✓ |
| heading h2 | `heading-h2` | happy | ✓ |
| heading h3 | `heading-h3` | happy | ✓ |
| source block | `code-block` | happy | ✓ |
| source block (no lang) | `code-block-no-lang` | happy | ✓ |
| quote block | `blockquote` | happy | ✓ |
| unordered list | `list-unordered` | happy | ✓ |
| ordered list | `list-ordered` | happy | ✓ |
| horizontal rule | `horizontal-rule` | happy | ✓ |

### Inline constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| bold | `strong` | happy | ✓ |
| italic | `emphasis` | happy | ✓ |
| underline | `underline` | happy | ✓ |
| strikethrough | `strikeout` | happy | ✓ |
| code | `code-inline` | happy | ✓ |
| link (with desc) | `link` | happy | ✓ |
| link (bare URL) | `link-bare` | happy | ✓ |

### Metadata

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| #+TITLE / #+AUTHOR | `metadata` | happy | ✓ |

### Rare

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| heading with TODO keyword | `rare-heading-todo` | rare | ✓ |
| verbatim inline (= delimiters) | `rare-code-inline-equals` | rare | ✓ |
| nested markup | `rare-nested-markup` | rare | ✓ |

### Adversarial

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| empty document | `adv-empty` | adversarial | ✓ |
| unmatched markup | `adv-unmatched-markup` | adversarial | ✓ |
| unknown block type | `adv-unknown-block` | adversarial | ✓ |

### Not yet covered

| Construct | Notes |
|-----------|-------|
| table | org-mode tables not yet implemented in reader |
| drawer / property drawer | `:PROPERTIES:` blocks |
| footnote | `[fn:1]` style |
| tags on headings | `:tag1:tag2:` suffix |

---

## rst

Reader: custom hand-rolled RST parser. Heading levels are inferred dynamically from underline character order.

### Block constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| paragraph | `paragraph` | happy | ✓ |
| section heading h1 | `heading` | happy | ✓ |
| section heading h2 | `heading-h2` | happy | ✓ |
| code block (:: style) | `code-block` | happy | ✓ |
| code-block directive | `code-block-directive` | happy | ✓ |
| blockquote | `blockquote` | happy | ✓ |
| bullet list | `list-unordered` | happy | ✓ |
| enumerated list | `list-ordered` | happy | ✓ |
| definition list | `definition-list` | happy | ✓ |

### Inline constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| emphasis | `emphasis` | happy | ✓ |
| strong | `strong` | happy | ✓ |
| inline code | `code-inline` | happy | ✓ |
| hyperlink (embedded) | `link` | happy | ✓ |

### Rare

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| named hyperlink reference | `rare-link-named` | rare | ✓ |
| image directive | `rare-image` | rare | ✓ |
| note admonition | `rare-admonition` | rare | ✓ |

### Adversarial

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| empty document | `adv-empty` | adversarial | ✓ |
| unknown directive | `adv-unknown-directive` | adversarial | ✓ |
| unmatched emphasis | `adv-unmatched-emphasis` | adversarial | ✓ |

### Not yet covered

| Construct | Notes |
|-----------|-------|
| field list | Not implemented in reader |
| table (simple/grid) | Not implemented in reader |
| footnotes | `[1]_` style |
| substitution definitions | `\|name\|` syntax |

---

## latex

Reader: handwritten LaTeX parser (default feature). `\[...\]` and `$$...$$` display math is parsed inline (wrapped in a paragraph). List items have a leading space before content.

### Block constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| paragraph | `paragraph` | happy | ✓ |
| section heading h1 | `heading` | happy | ✓ |
| section heading h2 | `heading-h2` | happy | ✓ |
| itemize list | `list-unordered` | happy | ✓ |
| enumerate list | `list-ordered` | happy | ✓ |
| verbatim code block | `code-block` | happy | ✓ |
| tabular table | `table` | happy | ✓ |
| display math (\[...\]) | `math-display` | happy | ✓ |

### Inline constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| bold (\textbf) | `bold` | happy | ✓ |
| italic (\textit) | `italic` | happy | ✓ |
| underline (\underline) | `underline` | happy | ✓ |
| inline math ($...$) | `math-inline` | happy | ✓ |
| link (\href) | `link` | happy | ✓ |
| inline code (\texttt) | `code-inline` | happy | ✓ |

### Rare

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| lstlisting code block | `rare-lstlisting` | rare | ✓ |
| \url command | `rare-url` | rare | ✓ |
| \emph command | `rare-emph` | rare | ✓ |
| document with preamble | `rare-preamble` | rare | ✓ |

### Adversarial

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| empty document | `adv-empty` | adversarial | ✓ |
| unknown environment | `adv-unknown-env` | adversarial | ✓ |

### Not yet covered

| Construct | Notes |
|-----------|-------|
| \subsubsection heading | level 3 |
| figure / \includegraphics | figure environment |
| \begin{equation} math | display math in environment |
| strikeout (\sout) | requires ulem package |
| footnote (\footnote) | not implemented in reader |

---

## mediawiki

Reader: custom hand-rolled MediaWiki parser. Headings start at level 2 (==). List items wrap content in a paragraph node. Tables are not implemented.

### Block constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| paragraph | `paragraph` | happy | ✓ |
| heading level 2 (==) | `heading` | happy | ✓ |
| heading level 3 (===) | `heading-h3` | happy | ✓ |
| unordered list | `list-unordered` | happy | ✓ |
| ordered list | `list-ordered` | happy | ✓ |
| code block (space-indented) | `code-block` | happy | ✓ |
| horizontal rule | `horizontal-rule` | happy | ✓ |

### Inline constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| bold (''') | `bold` | happy | ✓ |
| italic ('') | `italic` | happy | ✓ |
| internal link | `link-internal` | happy | ✓ |
| external link | `link-external` | happy | ✓ |

### Rare

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| internal link with display text | `rare-link-display` | rare | ✓ |
| heading level 4 (====) | `rare-heading-deep` | rare | ✓ |

### Adversarial

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| empty document | `adv-empty` | adversarial | ✓ |
| template syntax falls through | `adv-template` | adversarial | ✓ |

### Not yet covered

| Construct | Notes |
|-----------|-------|
| table ({&#124; ... &#124;}) | Not implemented in reader |
| image (&#91;&#91;File:...&#93;&#93;) | Treated as internal link |
| template ({{...}}) | Falls through as raw text |
| categories | Not extracted as metadata |

---

## asciidoc

Reader: custom hand-rolled AsciiDoc parser (~1,290 lines). List items have inline children directly (no paragraph wrapper).

### Block constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| paragraph | `paragraph` | happy | ✓ |
| heading h1 (=) | `heading` | happy | ✓ |
| heading h2 (==) | `heading-h2` | happy | ✓ |
| unordered list (*) | `list-unordered` | happy | ✓ |
| ordered list (.) | `list-ordered` | happy | ✓ |
| listing code block (----) | `code-block` | happy | ✓ |
| source block with language | `code-block-source` | happy | ✓ |
| horizontal rule (''') | `horizontal-rule` | happy | ✓ |

### Inline constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| bold (*) | `bold` | happy | ✓ |
| italic (_) | `italic` | happy | ✓ |
| inline code (`) | `code-inline` | happy | ✓ |
| URL link | `link` | happy | ✓ |

### Rare

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| quote block (____) | `rare-blockquote` | rare | ✓ |
| description list (::) | `rare-description-list` | rare | ✓ |
| link macro | `rare-link-macro` | rare | ✓ |
| admonition ([NOTE]) | `rare-admonition` | rare | ✓ |

### Adversarial

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| empty document | `adv-empty` | adversarial | ✓ |
| unknown block attribute | `adv-unknown-attr` | adversarial | ✓ |

### Not yet covered

| Construct | Notes |
|-----------|-------|
| table (|===) | Not implemented in reader |
| image block (image::) | figure/image not yet tested |
| include directive | Not implemented |
| highlight (#) | span with class="highlight" |

---

## creole

Reader: custom hand-rolled Creole parser. List items wrap content in a paragraph node.

### Block constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| paragraph | `paragraph` | happy | ✓ |
| heading h1 (=) | `heading` | happy | ✓ |
| heading h2 (==) | `heading-h2` | happy | ✓ |
| unordered list (*) | `list-unordered` | happy | ✓ |
| ordered list (#) | `list-ordered` | happy | ✓ |
| nowiki code block ({{{) | `code-block` | happy | ✓ |
| horizontal rule (----) | `horizontal-rule` | happy | ✓ |

### Inline constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| bold (**) | `bold` | happy | ✓ |
| italic (//) | `italic` | happy | ✓ |
| link ([[url]]) | `link` | happy | ✓ |

### Rare

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| inline code ({{{...}}}) | `rare-code-inline` | rare | ✓ |
| image ([[Image:...]]) | `rare-image` | rare | ✓ |
| bare URL | `rare-link-bare` | rare | ✓ |

### Adversarial

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| empty document | `adv-empty` | adversarial | ✓ |

---

## djot

Reader: jotdown crate. In djot, `*text*` = strong, `_text_` = emphasis, `{-text-}` = strikeout. Code block content includes trailing newline.

### Block constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| paragraph | `paragraph` | happy | ✓ |
| heading h1 | `heading` | happy | ✓ |
| heading h2 | `heading-h2` | happy | ✓ |
| unordered list | `list-unordered` | happy | ✓ |
| ordered list | `list-ordered` | happy | ✓ |
| fenced code (no lang) | `code-block` | happy | ✓ |
| fenced code (with lang) | `code-block-lang` | happy | ✓ |
| horizontal rule | `horizontal-rule` | happy | ✓ |

### Inline constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| strong (*) | `bold` | happy | ✓ |
| emphasis (_) | `italic` | happy | ✓ |
| inline code (`) | `code-inline` | happy | ✓ |
| link | `link` | happy | ✓ |

### Rare

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| blockquote | `rare-blockquote` | rare | ✓ |
| strikeout ({-text-}) | `rare-strikeout` | rare | ✓ |

### Adversarial

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| empty document | `adv-empty` | adversarial | ✓ |

---

## textile

Reader: custom hand-rolled Textile parser.

### Block constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| paragraph | `paragraph` | happy | ✓ |
| heading h1 (h1.) | `heading` | happy | ✓ |
| heading h2 (h2.) | `heading-h2` | happy | ✓ |
| unordered list (- item) | `list-unordered` | happy | ✓ |
| ordered list (# item) | `list-ordered` | happy | ✓ |
| code block (bc.) | `code-block` | happy | ✓ |

### Inline constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| bold (**) | `bold` | happy | ✓ |
| italic (__) | `italic` | happy | ✓ |
| inline code (@) | `code-inline` | happy | ✓ |
| link | `link` | happy | ✓ |

### Rare

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| blockquote (bq.) | `rare-blockquote` | rare | ✓ |
| strikeout (-) | `rare-strikeout` | rare | ✓ |
| underline (+) | `rare-underline` | rare | ✓ |

### Adversarial

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| empty document | `adv-empty` | adversarial | ✓ |
| unmatched markup | `adv-unmatched` | adversarial | ✓ |

---

## muse

Reader: custom hand-rolled Muse (Emacs Muse) parser.

### Block constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| paragraph | `paragraph` | happy | ✓ |
| heading h1 (*) | `heading` | happy | ✓ |
| heading h2 (**) | `heading-h2` | happy | ✓ |
| unordered list (-) | `list-unordered` | happy | ✓ |
| ordered list (1.) | `list-ordered` | happy | ✓ |
| code block | `code-block` | happy | ✓ |

### Inline constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| bold | `bold` | happy | ✓ |
| italic | `italic` | happy | ✓ |
| inline code (=) | `code-inline` | happy | ✓ |
| link | `link` | happy | ✓ |

### Rare

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| blockquote | `rare-blockquote` | rare | ✓ |
| bare URL link | `rare-link-bare` | rare | ✓ |

### Adversarial

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| empty document | `adv-empty` | adversarial | ✓ |
| unmatched markup | `adv-unmatched` | adversarial | ✓ |

---

## t2t

Reader: custom hand-rolled txt2tags parser.

### Block constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| paragraph | `paragraph` | happy | ✓ |
| heading h1 (=) | `heading` | happy | ✓ |
| heading h2 (==) | `heading-h2` | happy | ✓ |
| unordered list (-) | `list-unordered` | happy | ✓ |
| ordered list (+) | `list-ordered` | happy | ✓ |
| verbatim code block (```) | `code-block` | happy | ✓ |
| horizontal rule (===) | `horizontal-rule` | happy | ✓ |

### Inline constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| bold (**) | `bold` | happy | ✓ |
| italic (//) | `italic` | happy | ✓ |
| link | `link` | happy | ✓ |

### Rare

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| inline code (`` ` ``) | `rare-code-inline` | rare | ✓ |
| comment (%) | `rare-comment` | rare | ✓ |
| underline (__) | `rare-underline` | rare | ✓ |

### Adversarial

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| empty document | `adv-empty` | adversarial | ✓ |
| unknown macro/command | `adv-unknown` | adversarial | ✓ |

---

## tikiwiki

Reader: custom hand-rolled TikiWiki parser. Headings use `!` prefix (count = level). List items wrap in paragraph. Bold: `__text__`, italic: `''text''`.

### Block constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| paragraph | `paragraph` | happy | ✓ |
| heading h1 (!) | `heading` | happy | ✓ |
| heading h2 (!!) | `heading-h2` | happy | ✓ |
| unordered list (*) | `list-unordered` | happy | ✓ |
| ordered list (#) | `list-ordered` | happy | ✓ |
| code block ({CODE()}) | `code-block` | happy | ✓ |
| horizontal rule (---) | `horizontal-rule` | happy | ✓ |

### Inline constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| bold (__) | `bold` | happy | ✓ |
| italic ('') | `italic` | happy | ✓ |
| link ([url\|label]) | `link` | happy | ✓ |

### Rare

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| underline (===) | `rare-underline` | rare | ✓ |
| inline code (-+) | `rare-code-inline` | rare | ✓ |

### Adversarial

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| empty document | `adv-empty` | adversarial | ✓ |

---

## twiki

Reader: custom hand-rolled TWiki parser. Headings: `---+` = h1. Lists need 3-space prefix. `<verbatim>` for code blocks.

### Block constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| paragraph | `paragraph` | happy | ✓ |
| heading h1 (---+) | `heading` | happy | ✓ |
| heading h2 (---++) | `heading-h2` | happy | ✓ |
| unordered list (   * item) | `list-unordered` | happy | ✓ |
| ordered list (   1. item) | `list-ordered` | happy | ✓ |
| verbatim code block | `code-block` | happy | ✓ |

### Inline constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| bold (*text*) | `bold` | happy | ✓ |
| italic (_text_) | `italic` | happy | ✓ |
| inline code (=text=) | `code-inline` | happy | ✓ |
| link ([[url][label]]) | `link` | happy | ✓ |

### Rare

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| bold-italic (__text__) → strong > emphasis | `rare-bold-italic` | rare | ✓ |
| bold-fixed (==text==) → strong > code | `rare-bold-fixed` | rare | ✓ |

### Adversarial

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| empty document | `adv-empty` | adversarial | ✓ |

---

## vimwiki

Reader: custom hand-rolled VimWiki parser. Headings: `= Title =` = h1. Code blocks: `{{{ ... }}}`. List items wrap in paragraph.

### Block constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| paragraph | `paragraph` | happy | ✓ |
| heading h1 (= =) | `heading` | happy | ✓ |
| heading h2 (== ==) | `heading-h2` | happy | ✓ |
| unordered list (- item) | `list-unordered` | happy | ✓ |
| ordered list (1. item) | `list-ordered` | happy | ✓ |
| code block ({{{...}}}) | `code-block` | happy | ✓ |
| horizontal rule (----) | `horizontal-rule` | happy | ✓ |

### Inline constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| bold (*text*) | `bold` | happy | ✓ |
| italic (_text_) | `italic` | happy | ✓ |
| link ([[url\|desc]]) | `link` | happy | ✓ |

### Rare

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| strikeout (~~text~~) | `rare-strikeout` | rare | ✓ |
| image ({{url\|alt}}) | `rare-image` | rare | ✓ |
| inline code (`code`) | `rare-code-inline` | rare | ✓ |

### Adversarial

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| empty document | `adv-empty` | adversarial | ✓ |

---

## dokuwiki

Reader: custom hand-rolled DokuWiki parser. **Headings inverted**: 6 `=` signs = level 1. Ordered lists use `-`, unordered use `*` (opposite convention).

### Block constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| paragraph | `paragraph` | happy | ✓ |
| heading h1 (====== ======) | `heading` | happy | ✓ |
| heading h2 (===== =====) | `heading-h2` | happy | ✓ |
| unordered list (  * item) | `list-unordered` | happy | ✓ |
| ordered list (  - item) | `list-ordered` | happy | ✓ |
| code block (<code>) | `code-block` | happy | ✓ |

### Inline constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| bold (**text**) | `bold` | happy | ✓ |
| italic (//text//) | `italic` | happy | ✓ |
| underline (__text__) | `underline` | happy | ✓ |
| link ([[url\|text]]) | `link` | happy | ✓ |

### Rare

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| monospace (''text'') | `rare-code-inline` | rare | ✓ |
| image ({{url\|alt}}) | `rare-image` | rare | ✓ |
| blockquote (> prefix) | `rare-blockquote` | rare | ✓ |

### Adversarial

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| empty document | `adv-empty` | adversarial | ✓ |

---

## jira

Reader: custom hand-rolled Jira wiki markup parser. Headings: `h1. Title`. Code blocks: `{code:lang}...{code}`.

### Block constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| paragraph | `paragraph` | happy | ✓ |
| heading h1 | `heading` | happy | ✓ |
| heading h2 | `heading-h2` | happy | ✓ |
| unordered list (* item) | `list-unordered` | happy | ✓ |
| ordered list (# item) | `list-ordered` | happy | ✓ |
| code block ({code}) | `code-block` | happy | ✓ |
| code block with language | `code-block-lang` | happy | ✓ |

### Inline constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| bold (*text*) | `bold` | happy | ✓ |
| italic (_text_) | `italic` | happy | ✓ |
| link ([label\|url]) | `link` | happy | ✓ |

### Rare

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| blockquote ({quote}) | `rare-blockquote` | rare | ✓ |
| strikeout (-text-) | `rare-strikeout` | rare | ✓ |
| monospace ({{text}}) | `rare-code-inline` | rare | ✓ |

### Adversarial

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| empty document | `adv-empty` | adversarial | ✓ |

---

## haddock

Reader: custom hand-rolled Haddock parser. Code blocks use bird tracks (`> ` prefix). Content is trim_end() trimmed (no trailing newline).

### Block constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| paragraph | `paragraph` | happy | ✓ |
| heading h1 (= =) | `heading` | happy | ✓ |
| heading h2 (== ==) | `heading-h2` | happy | ✓ |
| unordered list (* item) | `list-unordered` | happy | ✓ |
| ordered list ((1) item) | `list-ordered` | happy | ✓ |
| code block (> bird tracks) | `code-block` | happy | ✓ |
| definition list ([term] desc) | `definition-list` | happy | ✓ |

### Inline constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| bold (__text__) | `bold` | happy | ✓ |
| italic (/text/) | `italic` | happy | ✓ |
| inline code (@text@) | `code-inline` | happy | ✓ |
| link ("text"<url>) | `link` | happy | ✓ |

### Rare

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| identifier ref ('ident') | `rare-identifier` | rare | ✓ |
| bare URL link (<url>) | `rare-link-bare` | rare | ✓ |

### Adversarial

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| empty document | `adv-empty` | adversarial | ✓ |

---

## pod

Reader: custom hand-rolled POD parser. Requires `=pod` or `=head` to activate, `=cut` to deactivate. Verbatim blocks trimmed with trim_end().

### Block constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| paragraph | `paragraph` | happy | ✓ |
| heading h1 (=head1) | `heading` | happy | ✓ |
| heading h2 (=head2) | `heading-h2` | happy | ✓ |
| unordered list (=item *) | `list-unordered` | happy | ✓ |
| ordered list (=item 1.) | `list-ordered` | happy | ✓ |
| verbatim code block (indented) | `code-block` | happy | ✓ |

### Inline formatting

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| bold (B<text>) | `bold` | happy | ✓ |
| italic (I<text>) | `italic` | happy | ✓ |
| code (C<text>) | `code-inline` | happy | ✓ |
| link (L<url>) | `link` | happy | ✓ |

### Rare

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| file/path (F<text>) → emphasis | `rare-formatting-codes` | rare | ✓ |
| definition item (=item Term) | `rare-definition-list` | rare | ✓ |

### Adversarial

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| empty file (no =pod) | `adv-empty` | adversarial | ✓ |

---

## man

Reader: custom hand-rolled groff/troff man page parser. Macro-based format. `.SH` = h2, `.SS` = h3. Font escapes `\fB`, `\fI` for inline styling.

### Block constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| paragraph (.PP) | `paragraph` | happy | ✓ |
| section heading (.SH) | `heading` | happy | ✓ |
| subsection heading (.SS) | `heading-ss` | happy | ✓ |
| preformatted block (.nf/.fi) | `code-block` | happy | ✓ |
| definition list (.TP) | `definition-list` | happy | ✓ |

### Inline constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| bold macro (.B) | `bold` | happy | ✓ |
| italic macro (.I) | `italic` | happy | ✓ |
| inline bold (\fB..\fR) | `inline-bold` | happy | ✓ |
| inline italic (\fI..\fR) | `inline-italic` | happy | ✓ |
| link (.URL) | `link` | happy | ✓ |

### Rare

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| alternating bold-roman (.BR) | `rare-alternating` | rare | ✓ |

### Adversarial

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| empty document | `adv-empty` | adversarial | ✓ |

---

## xwiki

Reader: custom hand-rolled XWiki parser. Bold: `**text**`, italic: `//text//`, monospace: `##text##`. Code blocks via `{{code}}...{{/code}}` (content trimmed). Link: `[[label>>url]]`.

### Block constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| paragraph | `paragraph` | happy | ✓ |
| heading h1 (= =) | `heading` | happy | ✓ |
| heading h2 (== ==) | `heading-h2` | happy | ✓ |
| unordered list (* item) | `list-unordered` | happy | ✓ |
| ordered list (1. item) | `list-ordered` | happy | ✓ |
| code block ({{code}}) | `code-block` | happy | ✓ |
| code block with language | `code-block-lang` | happy | ✓ |
| horizontal rule (----) | `horizontal-rule` | happy | ✓ |

### Inline constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| bold (**text**) | `bold` | happy | ✓ |
| italic (//text//) | `italic` | happy | ✓ |
| link ([[label>>url]]) | `link` | happy | ✓ |

### Rare

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| monospace (##text##) | `rare-monospace` | rare | ✓ |

### Adversarial

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| empty document | `adv-empty` | adversarial | ✓ |

---

## zimwiki

Reader: custom hand-rolled ZimWiki parser. **Headings inverted**: 6 `=` = level 1. Code blocks via `'''...'''` (trimmed).

### Block constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| paragraph | `paragraph` | happy | ✓ |
| heading h1 (====== ======) | `heading` | happy | ✓ |
| heading h2 (===== =====) | `heading-h2` | happy | ✓ |
| unordered list (* item) | `list-unordered` | happy | ✓ |
| ordered list (1. item) | `list-ordered` | happy | ✓ |
| code block (''' fence) | `code-block` | happy | ✓ |

### Inline constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| bold (**text**) | `bold` | happy | ✓ |
| italic (//text//) | `italic` | happy | ✓ |
| link ([[url\|label]]) | `link` | happy | ✓ |

### Rare

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| strikeout (~~text~~) | `rare-strikeout` | rare | ✓ |
| monospace (''text'') | `rare-monospace` | rare | ✓ |
| image ({{url}}) | `rare-image` | rare | ✓ |

### Adversarial

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| empty document | `adv-empty` | adversarial | ✓ |

---

## bbcode

Reader: custom hand-rolled BBCode parser. **No headings**. Tags are case-insensitive. Lists use `[list][*]item[/list]`.

### Block constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| paragraph | `paragraph` | happy | ✓ |
| unordered list ([list]) | `list-unordered` | happy | ✓ |
| ordered list ([list=1]) | `list-ordered` | happy | ✓ |
| code block ([code]) | `code-block` | happy | ✓ |

### Inline constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| bold ([b]) | `bold` | happy | ✓ |
| italic ([i]) | `italic` | happy | ✓ |
| underline ([u]) | `underline` | happy | ✓ |
| link ([url=url]label) | `link` | happy | ✓ |

### Rare

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| blockquote ([quote]) | `rare-blockquote` | rare | ✓ |
| strikeout ([s]) | `rare-strikeout` | rare | ✓ |
| color span ([color=red]) | `rare-code-inline` | rare | ✓ |

### Adversarial

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| empty document | `adv-empty` | adversarial | ✓ |

---

## texinfo

Reader: custom hand-rolled Texinfo parser. `@chapter` = h1, `@section` = h2. **List items directly contain inline nodes** (no paragraph wrapper).

### Block constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| paragraph | `paragraph` | happy | ✓ |
| chapter heading (@chapter) | `heading` | happy | ✓ |
| section heading (@section) | `heading-h2` | happy | ✓ |
| itemize list (@itemize) | `list-unordered` | happy | ✓ |
| enumerate list (@enumerate) | `list-ordered` | happy | ✓ |
| example block (@example) | `code-block` | happy | ✓ |

### Inline constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| bold (@strong) | `bold` | happy | ✓ |
| italic (@emph) | `italic` | happy | ✓ |
| code (@code) | `code-inline` | happy | ✓ |
| link (@uref) | `link` | happy | ✓ |

### Rare

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| blockquote (@quotation) | `rare-blockquote` | rare | ✓ |
| verbatim block (@verbatim) | `rare-verbatim` | rare | ✓ |

### Adversarial

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| empty document | `adv-empty` | adversarial | ✓ |

---

## markua

Reader: custom hand-rolled Markua (Leanpub Markdown) parser. Similar to Markdown. List items wrap in paragraph. Code block content trim_end() trimmed. Special blocks: `A>` aside, `W>` warning, etc.

### Block constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| paragraph | `paragraph` | happy | ✓ |
| heading h1 (#) | `heading` | happy | ✓ |
| heading h2 (##) | `heading-h2` | happy | ✓ |
| unordered list (- item) | `list-unordered` | happy | ✓ |
| ordered list (1. item) | `list-ordered` | happy | ✓ |
| fenced code block | `code-block` | happy | ✓ |

### Inline constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| bold (**text**) | `bold` | happy | ✓ |
| italic (*text*) | `italic` | happy | ✓ |
| inline code (`text`) | `code-inline` | happy | ✓ |
| link ([text](url)) | `link` | happy | ✓ |

### Rare

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| blockquote (> text) | `rare-blockquote` | rare | ✓ |
| special aside block (A>) | `rare-special-block` | rare | ✓ |

### Adversarial

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| empty document | `adv-empty` | adversarial | ✓ |

---

## fountain

Reader: custom hand-rolled Fountain screenplay parser. All elements have `fountain:type` property. No lists or code blocks. Scene headings are `INT./EXT.` lines.

### Screenplay elements

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| action paragraph | `action` | happy | ✓ |
| scene heading (INT./EXT.) | `scene-heading` | happy | ✓ |
| section heading (#) | `section-heading` | happy | ✓ |
| dialogue block (character + dialogue) | `dialogue` | happy | ✓ |
| transition (CUT TO:) | `transition` | happy | ✓ |
| dialogue with parenthetical | `parenthetical-in-dialogue` | happy | ✓ |
| page break (===) | `page-break` | happy | ✓ |

### Rare

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| forced scene heading (.prefix) | `rare-forced-scene` | rare | ✓ |
| lyric line (~prefix) | `rare-lyric` | rare | ✓ |
| synopsis line (= prefix) | `rare-synopsis` | rare | ✓ |

### Adversarial

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| empty document | `adv-empty` | adversarial | ✓ |

---

## ansi

Reader: custom hand-rolled ANSI escape code parser. **No headings, lists, or code blocks** — only paragraphs with inline styling. ESC[1m = bold, ESC[3m = italic, ESC[4m = underline, ESC[9m = strikeout, ESC[0m = reset.

### Inline constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| plain text paragraph | `paragraph` | happy | ✓ |
| bold (ESC[1m) | `bold` | happy | ✓ |
| italic (ESC[3m) | `italic` | happy | ✓ |
| underline (ESC[4m) | `underline` | happy | ✓ |

### Rare

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| strikeout (ESC[9m) | `rare-strikeout` | rare | ✓ |
| nested bold+italic | `rare-bold-italic` | rare | ✓ |
| styled text within plain text | `rare-inline-in-text` | rare | ✓ |

### Adversarial

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| empty document | `adv-empty` | adversarial | ✓ |

---

## Formats pending initial fixture authoring

The following formats have readers but no fixtures yet.

| Format | Reader status | Priority |
|--------|--------------|----------|
| typst | partial coverage | 7 |
| jats | reader exists | 8 |
| endnotexml | reader exists | 8 |
| tei | reader exists | 8 |
| docx, epub, odt, pptx, xlsx | binary/library-backed | 10 |
| pdf, rtf | complex binary | 10 |
| commonmark, gfm, markdown-strict, multimarkdown | alias markdown reader | — |

## Completed formats

| Format | Fixtures | Notes |
|--------|----------|-------|
| markdown | ✓ | CommonMark + GFM |
| html | ✓ | html5ever |
| org | ✓ | hand-rolled |
| rst | ✓ | hand-rolled |
| latex | ✓ | hand-rolled |
| mediawiki | ✓ | hand-rolled |
| asciidoc | ✓ | hand-rolled |
| creole | ✓ | hand-rolled |
| djot | ✓ | jotdown crate |
| textile | ✓ | hand-rolled |
| muse | ✓ | hand-rolled |
| t2t | ✓ | hand-rolled |
| tikiwiki | ✓ | hand-rolled |
| twiki | ✓ | hand-rolled |
| vimwiki | ✓ | hand-rolled |
| dokuwiki | ✓ | hand-rolled |
| jira | ✓ | hand-rolled |
| haddock | ✓ | hand-rolled |
| pod | ✓ | hand-rolled |
| man | ✓ | hand-rolled |
| xwiki | ✓ | hand-rolled |
| zimwiki | ✓ | hand-rolled |
| bbcode | ✓ | hand-rolled |
| texinfo | ✓ | hand-rolled |
| markua | ✓ | hand-rolled |
| fountain | ✓ | hand-rolled |
| ansi | ✓ | hand-rolled |
| csv | ✓ | hand-rolled |
| tsv | ✓ | hand-rolled |
| opml | ✓ | quick-xml |
| ris | ✓ | hand-rolled |
| bibtex | ✓ | biblatex crate |
| biblatex | ✓ | biblatex crate |
| csl-json | ✓ | serde_json |
| native | ✓ | hand-rolled |
| pandoc-json | ✓ | serde_json |
| docbook | ✓ | hand-rolled XML |
| fb2 | ✓ | hand-rolled XML |
| ipynb | ✓ | serde_json |

---

## csv

Reader: custom hand-rolled CSV parser. First row always treated as headers (table_header cells). Subsequent rows are table_cell. Empty file produces an empty table node.

### Constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| basic 2-column table | `basic` | happy | ✓ |
| single-column table | `single-column` | happy | ✓ |
| three-column table | `three-columns` | happy | ✓ |
| quoted field with comma | `rare-comma-in-field` | rare | ✓ |
| empty field | `rare-empty-field` | rare | ✓ |
| empty file → empty table | `adv-empty` | adversarial | ✓ |
| header-only (no data rows) | `adv-header-only` | adversarial | ✓ |

---

## tsv

Reader: custom hand-rolled TSV parser. Same structure as CSV but tab-delimited.

### Constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| basic 2-column table | `basic` | happy | ✓ |
| three-column table | `three-columns` | happy | ✓ |
| quoted field with tab | `rare-quoted-tab` | rare | ✓ |
| empty field | `rare-empty-field` | rare | ✓ |
| empty file → empty table | `adv-empty` | adversarial | ✓ |
| header-only (no data rows) | `adv-header-only` | adversarial | ✓ |

---

## opml

Reader: quick-xml based OPML parser. **Note**: Self-closing outlines at the top level become direct paragraph nodes. Only non-self-closing outlines with children produce a list. Metadata from `<head>` is extracted to document metadata.

### Constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| single top-level outline item | `single-item` | happy | ✓ |
| flat list of 3 items (paragraph per item) | `basic` | happy | ✓ |
| parent with children (flattened list) | `nested` | happy | ✓ |
| outline with xmlUrl → link node | `with-url` | happy | ✓ |
| metadata from head/title | `metadata` | happy | ✓ |
| both xmlUrl and htmlUrl (xmlUrl wins) | `rare-two-url-attrs` | rare | ✓ |
| empty body | `adv-empty` | adversarial | ✓ |
| no head element | `adv-minimal` | adversarial | ✓ |

---

## ris

Reader: custom hand-rolled RIS parser. Produces `definition_list > ris:entry > definition_term + definition_desc`. `ris:type` prop has the raw RIS type code (JOUR, BOOK, ELEC, etc.).

### Constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| journal article (TY JOUR) | `article` | happy | ✓ |
| book (TY BOOK) | `book` | happy | ✓ |
| multiple authors | `multi-author` | happy | ✓ |
| entry with DOI | `with-doi` | happy | ✓ |
| entry with URL (TY ELEC) | `with-url` | happy | ✓ |
| entry without ER terminator | `rare-no-er` | rare | ✓ |
| empty file | `adv-empty` | adversarial | ✓ |

---

## bibtex

Reader: biblatex crate. Produces `definition_list > bibtex:entry > definition_term + definition_desc`.

### Constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| article entry | `article` | happy | ✓ |
| book entry | `book` | happy | ✓ |
| inproceedings entry | `inproceedings` | happy | ✓ |
| misc entry | `misc` | happy | ✓ |
| entry with two authors | `two-authors` | happy | ✓ |
| entry with DOI | `rare-with-doi` | rare | ✓ |
| empty file | `adv-empty` | adversarial | ✓ |

---

## biblatex

Reader: biblatex crate. Same structure as bibtex but uses biblatex:entry node kind.

### Constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| article entry | `article` | happy | ✓ |
| book entry | `book` | happy | ✓ |
| inproceedings entry | `inproceedings` | happy | ✓ |
| entry with subtitle | `rare-with-subtitle` | rare | ✓ |
| empty file | `adv-empty` | adversarial | ✓ |

---

## csl-json

Reader: serde_json based CSL-JSON parser. Produces `definition_list > csl:item > definition_term + definition_desc`.

### Constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| article-journal | `article-journal` | happy | ✓ |
| book | `book` | happy | ✓ |
| chapter | `chapter` | happy | ✓ |
| multiple authors | `multi-author` | happy | ✓ |
| item with DOI | `with-doi` | happy | ✓ |
| date with literal string | `rare-literal-date` | rare | ✓ |
| empty array | `adv-empty` | adversarial | ✓ |

---

## native

Reader: custom parser for rescribe's native text format. Node kinds with colons cannot be used (identifier parser stops at non-alphanumeric characters).

### Constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| paragraph with text | `paragraph` | happy | ✓ |
| heading | `heading` | happy | ✓ |
| unordered list | `list-unordered` | happy | ✓ |
| code block | `code-block` | happy | ✓ |
| nested structure | `nested` | happy | ✓ |
| empty document | `adv-empty` | adversarial | ✓ |

---

## pandoc-json

Reader: serde_json based Pandoc AST JSON parser.

### Constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| paragraph | `paragraph` | happy | ✓ |
| heading | `heading` | happy | ✓ |
| unordered list | `list-unordered` | happy | ✓ |
| ordered list | `list-ordered` | happy | ✓ |
| bold | `bold` | happy | ✓ |
| italic | `italic` | happy | ✓ |
| code block | `code-block` | happy | ✓ |
| inline code | `code-inline` | happy | ✓ |
| empty document | `adv-empty` | adversarial | ✓ |

---

## docbook

Reader: XML-based DocBook parser. `<article>` → div, `<section>` → div, `<title>` → heading, `<para>` → paragraph.

### Constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| paragraph | `paragraph` | happy | ✓ |
| heading (title in section) | `heading` | happy | ✓ |
| section with nested title | `section` | happy | ✓ |
| unordered list (itemizedlist) | `list-unordered` | happy | ✓ |
| ordered list (orderedlist) | `list-ordered` | happy | ✓ |
| code block (programlisting) | `code-block` | happy | ✓ |
| emphasis | `emphasis` | happy | ✓ |
| strong (emphasis role="bold") | `strong` | happy | ✓ |
| link (ulink) | `link` | happy | ✓ |
| empty document | `adv-empty` | adversarial | ✓ |

---

## fb2

Reader: XML-based FictionBook 2 parser. `<body><section>` → div, section `<title><p>` → heading, `<p>` → paragraph. Link uses XLink namespace.

### Constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| paragraph | `paragraph` | happy | ✓ |
| title metadata | `title-metadata` | happy | ✓ |
| section heading | `section-heading` | happy | ✓ |
| emphasis | `emphasis` | happy | ✓ |
| strong | `strong` | happy | ✓ |
| link (l:href XLink) | `link` | happy | ✓ |
| nested section | `nested-section` | happy | ✓ |
| empty body | `adv-empty` | adversarial | ✓ |

---

## ipynb

Reader: serde_json based Jupyter Notebook parser. Markdown cells delegate to the markdown reader. Code cells produce `code_block` with `language` and `ipynb:execution_count` props. Raw cells produce `raw_block`.

### Constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| markdown cell | `markdown-cell` | happy | ✓ |
| heading in markdown cell | `heading-cell` | happy | ✓ |
| code cell | `code-cell` | happy | ✓ |
| code cell with language | `code-cell-with-language` | happy | ✓ |
| raw cell | `raw-cell` | happy | ✓ |
| multiple cells | `multi-cell` | happy | ✓ |
| source as array of strings | `rare-source-array` | rare | ✓ |
| cell with output stream | `rare-output-stream` | rare | ✓ |
| empty notebook | `adv-empty` | adversarial | ✓ |

---

## typst

Reader: typst crate. Paragraphs are buffered from inline elements. Whitespace-only paragraphs are dropped. Adjacent list items are merged into a single list node.

### Block constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| paragraph | `paragraph` | happy | ✓ |
| heading h1 (=) | `heading` | happy | ✓ |
| heading h2 (==) | `heading-h2` | happy | ✓ |
| unordered list (- item) | `list-unordered` | happy | ✓ |
| ordered list (+ item) | `list-ordered` | happy | ✓ |
| fenced code block | `code-block` | happy | ✓ |

### Inline constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| bold (*text*) | `bold` | happy | ✓ |
| italic (_text_) | `italic` | happy | ✓ |
| code inline (`text`) | `code-inline` | happy | ✓ |

### Adversarial

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| empty document | `adv-empty` | adversarial | ✓ |

---

## jats

Reader: quick-xml based JATS (Journal Article Tag Suite) parser. `<article>` → div. `<body>`, `<front>`, `<back>` pass through. Heading level inferred from parent element.

### Constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| paragraph (`<p>`) | `paragraph` | happy | ✓ |
| section heading (`<title>` in `<sec>`) | `heading` | happy | ✓ |
| unordered list (list-type="bullet") | `list-unordered` | happy | ✓ |
| ordered list (list-type="order") | `list-ordered` | happy | ✓ |
| code block (`<preformat>`) | `code-block` | happy | ✓ |
| emphasis (`<italic>`) | `emphasis` | happy | ✓ |
| strong (`<bold>`) | `strong` | happy | ✓ |
| link (`<ext-link>`) | `link` | happy | ✓ |
| empty document | `adv-empty` | adversarial | ✓ |

---

## endnotexml

Reader: quick-xml based EndNote XML parser. Produces `definition_list > endnote:entry > [definition_term, definition_desc]`. `endnote:type` stores the raw numeric type code from the XML (17 = Journal Article, 6 = Book, etc.).

### Constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| journal article (ref-type 17) | `article` | happy | ✓ |
| book (ref-type 6) | `book` | happy | ✓ |
| entry with URL | `with-url` | rare | ✓ |
| empty input | `adv-empty` | adversarial | ✓ |

---

## tei

Reader: quick-xml based TEI (Text Encoding Initiative) parser. `<TEI>`, `<text>`, `<body>` pass through. Heading level from parent div depth (div1 → 1, div2 → 2). `<hi rend="bold">` → strong, `<hi rend="italic">` → emphasis.

### Block constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| paragraph (`<p>`) | `paragraph` | happy | ✓ |
| div1 with head → div + heading(level=1) | `heading` | happy | ✓ |
| div2 with head → div + heading(level=2) | `heading-h2` | happy | ✓ |
| unordered list (`<list>`) | `list-unordered` | happy | ✓ |

### Inline constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| bold (`<hi rend="bold">`) | `strong` | happy | ✓ |
| italic (`<hi rend="italic">`) | `emphasis` | happy | ✓ |
| link (`<ref target="url">`) | `link` | happy | ✓ |

### Adversarial

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| empty body | `adv-empty` | adversarial | ✓ |
