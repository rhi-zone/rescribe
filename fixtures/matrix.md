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

## Formats pending initial fixture authoring

The following formats have readers but no fixtures yet.

| Format | Reader status | Priority |
|--------|--------------|----------|
| typst | ~5% coverage | 7 |
| docx, epub, pdf, … | library-backed | 10 |

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
