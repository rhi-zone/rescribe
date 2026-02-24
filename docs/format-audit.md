# Format Implementation Audit

Assessed 2026-02-24. 54 readers, 64 writers.

## Library-backed (good)

| Format(s) | Library |
|-----------|---------|
| commonmark, gfm, markdown, markdown-strict, multimarkdown | `pulldown-cmark` |
| djot | `jotdown` |
| html | `html5ever` + `markup5ever_rcdom` |
| docx | `ooxml-wml` |
| xlsx (read) | `ooxml-sml` |
| xlsx (write) | `ooxml-sml` |
| pptx | `ooxml-pml` |
| epub (read) | `epub` crate |
| epub (write) | `epub-builder` |
| bibtex, biblatex | `biblatex` crate |
| csl-json, ipynb, pandoc-json | `serde_json` |
| pdf (read) | `pdf-extract` |

## Handwritten — acceptable

Simple line-based markup formats where hand-rolling is reasonable:

- **Wiki formats**: mediawiki, creole, dokuwiki, vimwiki, zimwiki, xwiki, twiki, tikiwiki, jira
- **Document markup**: rst, asciidoc, textile, muse, t2t, markua, texinfo, typst
- **Niche formats**: bbcode, fountain, haddock, pod, ansi, man
- **Data formats**: csv, tsv, ris, native

## Risk areas

### RTF — high risk
- 454-line reader, 385-line writer, zero library backing
- RTF is genuinely complex: group nesting, hex escaping, codepage handling, binary data blobs
- Correctness is hard to verify without a reference impl
- `rtf-parser` on crates.io is a candidate replacement for the reader

### ODT writer — medium risk
- 404 lines building ODF zip by hand (no schema library)
- The reader uses `quick-xml` at least; the writer generates raw XML strings
- No ODT equivalent of `ooxml-wml`/`ooxml-pml` exists yet in the ecosystem

### RST reader — medium risk
- 1,263 lines, handwritten
- RST has genuinely tricky rules: underline-based heading priority, inline markup precedence
- No independent test harness; hard to verify edge cases

### AsciiDoc reader — low-medium risk
- 1,290 lines, handwritten
- `asciidoc-rs` exists on crates.io but is immature

## Already resolved

| Issue | Resolution |
|-------|-----------|
| PPTX reader/writer (hand-rolled ZIP+XML) | Migrated to `ooxml-pml` (2026-02-24) |
| DOCX reader/writer | Uses `ooxml-wml` |
