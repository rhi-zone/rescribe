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
| heading h1–h6 | — | happy | — |
| blockquote | — | happy | — |
| ordered list | — | happy | — |
| unordered list | — | happy | — |
| code block (`<pre><code>`) | — | happy | — |
| table | — | happy | — |
| horizontal rule | — | happy | — |
| div | — | happy | — |

### Inline constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| emphasis (`<em>`) | — | happy | — |
| strong (`<strong>`) | — | happy | — |
| strikethrough (`<del>`) | — | happy | — |
| inline code (`<code>`) | — | happy | — |
| link (`<a>`) | — | happy | — |
| image (`<img>`) | — | happy | — |
| line break (`<br>`) | — | happy | — |
| superscript (`<sup>`) | — | happy | — |
| subscript (`<sub>`) | — | happy | — |

### Adversarial

| Scenario | Fixture | Category | Status |
|----------|---------|----------|--------|
| unclosed tags | — | adversarial | — |
| script injection | — | adversarial | — |
| empty document | — | adversarial | — |

---

## org

### Block constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| paragraph | — | happy | — |
| heading (levels 1–3) | — | happy | — |
| source block | — | happy | — |
| quote block | — | happy | — |
| unordered list | — | happy | — |
| ordered list | — | happy | — |
| table | — | happy | — |
| horizontal rule | — | happy | — |

### Inline constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| bold | — | happy | — |
| italic | — | happy | — |
| code | — | happy | — |
| strikethrough | — | happy | — |
| link | — | happy | — |

---

## rst

### Block constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| paragraph | — | happy | — |
| section heading | — | happy | — |
| code block (:: style) | — | happy | — |
| code-block directive | — | happy | — |
| blockquote | — | happy | — |
| bullet list | — | happy | — |
| enumerated list | — | happy | — |
| field list | — | happy | — |
| table (simple) | — | happy | — |

### Inline constructs

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| emphasis | — | happy | — |
| strong | — | happy | — |
| inline code | — | happy | — |
| hyperlink | — | happy | — |

---

## latex

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| section | — | happy | — |
| paragraph | — | happy | — |
| itemize | — | happy | — |
| enumerate | — | happy | — |
| verbatim / lstlisting | — | happy | — |
| bold / italic / underline | — | happy | — |
| math inline | — | happy | — |
| table (tabular) | — | happy | — |

---

## mediawiki

| Construct | Fixture | Category | Status |
|-----------|---------|----------|--------|
| paragraph | — | happy | — |
| heading (== style) | — | happy | — |
| bold / italic | — | happy | — |
| link / external link | — | happy | — |
| unordered / ordered list | — | happy | — |
| table | — | happy | — |

---

## Formats pending initial fixture authoring

The following formats have readers but no fixtures yet. They should be
addressed in priority order: org, rst, latex, mediawiki, html (above is
partially done), then the remaining ~45 formats.

| Format | Reader status | Priority |
|--------|--------------|----------|
| org | ✓ complete | 1 |
| rst | medium risk | 2 |
| latex | ✓ complete | 3 |
| mediawiki | ✓ complete | 4 |
| html | ✓ complete | 5 |
| asciidoc | low-medium risk | 6 |
| typst | ~5% coverage | 7 |
| all wiki formats | ✓ complete | 8 |
| textile, muse, t2t, … | ✓ complete | 9 |
| docx, epub, pdf, … | library-backed | 10 |
