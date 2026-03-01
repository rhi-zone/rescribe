# Format Implementation Audit

Assessed 2026-02-24; stages updated 2026-03-01.

## Maturity Pipeline

```
0-Stub → 1-Partial → 2-Fixtures → 3-Harness → 4-Fuzz → 5-Production
```

| Stage | Meaning |
|-------|---------|
| **0-Stub** | Crate compiles; little or no real implementation |
| **1-Partial** | Handles common constructs; known gaps remain |
| **2-Fixtures** | Owned fixture suite authored and passing in CI |
| **3-Harness** | Pandoc oracle harness ≥90% word coverage |
| **4-Fuzz** | Fuzz target exists and has been run |
| **5-Production** | All gates passed |

**Conventions:**
`†` = library-backed (upstream provides correctness guarantee; wrapper still needs fixtures and fuzz).
`–` = not applicable (no crate exists, or stage is not meaningful for this format).
Stage 3 is marked `–` for formats Pandoc cannot read — their path skips directly from fixtures to fuzz.

---

## Format Table

### Markdown family

| Format | R | W | Library | R-next | W-next |
|--------|---|---|---------|--------|--------|
| commonmark | 3† | 3† | pulldown-cmark | fuzz | fuzz |
| gfm | 3† | 3† | pulldown-cmark | fuzz | fuzz |
| markdown | 4† | 4† | pulldown-cmark | production | production |
| markdown-strict | 3† | 2† | pulldown-cmark | fuzz | harness |
| multimarkdown | 3† | 2† | pulldown-cmark | fuzz | harness |

### Lightweight markup

| Format | R | W | Library | R-next | W-next |
|--------|---|---|---------|--------|--------|
| djot | 3† | 3† | jotdown | fuzz | fuzz |
| org | 4 | 2 | hand | production | harness |
| rst | 3 | 2 | hand | fuzz | harness |
| asciidoc | 2 | 2 | hand | alt harness† | harness |
| textile | 3 | 2 | hand | fuzz | harness |
| muse | 3 | 2 | hand | fuzz | harness |
| t2t | 3 | 2 | hand | fuzz | harness |
| markua | 2 | 2 | hand | harness | harness |
| fountain | 2 | 2 | hand | – (harness N/A) | fuzz |
| typst | 1 | 2 | hand | partial→fixtures | harness |
| texinfo | 2 | 2 | hand | – (harness N/A) | fuzz |
| bbcode | 2 | 2 | hand | – (harness N/A) | fuzz |
| pod | 2 | 2 | hand | harness (87%) | harness |
| haddock | 2 | 2 | hand | harness (88%) | harness |
| ansi | 2 | 2 | hand | – (harness N/A) | fuzz |
| man | 3 | 2 | hand | fuzz | harness |

† Pandoc cannot read AsciiDoc (`--from asciidoc` unsupported); consider asciidoctor as alternate oracle.

### Wiki formats

| Format | R | W | Library | R-next | W-next |
|--------|---|---|---------|--------|--------|
| mediawiki | 3 | 2 | hand | fuzz | harness |
| creole | 2 | 2 | hand | – (harness N/A) | fuzz |
| dokuwiki | 2 | 2 | hand | – (harness N/A) | fuzz |
| vimwiki | 2 | 2 | hand | – (harness N/A) | fuzz |
| zimwiki | 2 | 2 | hand | – (harness N/A) | fuzz |
| xwiki | 2 | 2 | hand | – (harness N/A) | fuzz |
| twiki | 2 | 2 | hand | harness (79%) | harness |
| tikiwiki | 2 | 2 | hand | harness | harness |
| jira | 2 | 2 | hand | – (harness N/A) | fuzz |

### Office / binary

| Format | R | W | Library | R-next | W-next |
|--------|---|---|---------|--------|--------|
| docx | 3† | 3† | ooxml-wml | fuzz | fuzz |
| odt | 3 | 2 | quick-xml / hand | fuzz | harness |
| epub | 3† | 3† | epub / epub-builder | fuzz | fuzz |
| fb2 | 3 | 2 | hand | fuzz | harness |
| pptx | 3† | 3† | ooxml-pml | fuzz | fuzz |
| xlsx | 3† | 3† | ooxml-sml | fuzz | fuzz |
| pdf | 4† | – | pdf-extract | production | – |
| rtf | 2 | 2 | hand (⚠ high risk) | harness | harness |
| mobi | – | – | – (planned) | – | – |
| azw3 | – | – | – (planned) | – | – |
| kfx | – | – | – (planned) | – | – |

### HTML and structured XML

| Format | R | W | Library | R-next | W-next |
|--------|---|---|---------|--------|--------|
| html | 4† | 3 | html5ever / hand | production | fuzz |
| docbook | 3 | 2 | hand | fuzz | harness |
| jats | 3 | 2 | hand | fuzz | harness |
| tei | 3 | 2 | hand | fuzz | harness |
| opml | 3 | 2 | hand | fuzz | harness |
| ipynb | 3† | 2† | serde_json | fuzz | harness |
| latex | 4 | 2 | hand | production | harness |

### Bibliographic

| Format | R | W | Library | R-next | W-next |
|--------|---|---|---------|--------|--------|
| bibtex | 3† | 2† | biblatex | fuzz | harness |
| biblatex | 3† | 2† | biblatex | fuzz | harness |
| csl-json | 3† | 2† | serde_json | fuzz | harness |
| ris | 2 | 2 | hand | – (harness N/A) | fuzz |
| endnotexml | 2 | 2 | hand | – (harness N/A) | fuzz |

### Data / interchange

| Format | R | W | Library | R-next | W-next |
|--------|---|---|---------|--------|--------|
| csv | 2 | 2 | hand | – (harness N/A) | fuzz |
| tsv | 2 | 2 | hand | – (harness N/A) | fuzz |
| pandoc-json | 4† | 3† | serde_json | production | fuzz |
| native | 3 | 2 | hand | fuzz | harness |

### Presentation / output-only

These formats have no reader; stage 3 (harness) is not applicable.

| Format | W | Library | W-next |
|--------|---|---------|--------|
| beamer | 2 | hand | fuzz |
| revealjs | 2 | hand | fuzz |
| slidy | 2 | hand | fuzz |
| s5 | 2 | hand | fuzz |
| dzslides | 2 | hand | fuzz |
| slideous | 2 | hand | fuzz |
| context | 2 | hand | fuzz |
| ms | 2 | hand | fuzz |
| icml | 2 | hand | fuzz |
| chunkedhtml | 2 | hand | fuzz |
| plaintext | 2 | hand | fuzz |

---

## Risk areas

### RTF — high risk
- 454-line reader, 385-line writer, zero library backing
- RTF is genuinely complex: group nesting, hex escaping, codepage handling, binary data blobs
- `rtf-parser` on crates.io is a candidate replacement for the reader
- See Priority 1 in TODO.md

### ODT writer — medium risk
- 404 lines building ODF zip by hand (no schema library)
- Reader uses `quick-xml`; writer generates raw XML strings
- No ODT equivalent of `ooxml-wml`/`ooxml-pml` exists in the ecosystem
- Reader promoted to 3-Harness (100% coverage, 6 corpus files, 2026-03-01)

### RST reader — resolved
- Pandoc harness: 96% word coverage (ref=618, ours=639) — promoted to 3-Harness
- Next: fuzz target

### AsciiDoc reader — low-medium risk
- 1,290 lines, handwritten
- `asciidoc-rs` exists on crates.io but is immature
- Pandoc oracle unavailable; asciidoctor is the alternate reference

### KFX / AZW3 / MOBI — planned, not yet started
- KFX uses Amazon Ion binary format (public spec: amazon-ion.github.io); Ion layer would be
  hand-rolled against spec (ion-rs has ~1 year of unreleased commits, not suitable as dep)
- KFX schema/structure layer is reverse-engineered; boko (MIT) is the reference implementation
- AZW3 (KF8) is EPUB3 content in a Mobipocket container; tractable with boko as reference
- MOBI (KF7) is PalmDOC/HuffCDIC; read-only target, boko as reference

### Typst reader — currently incomplete
- Pandoc harness at ~5% word coverage (ref=552 words, ours=36)
- At stage 1 (Partial) rather than 2; needs significant work before fixtures are meaningful

---

## Already resolved

| Issue | Resolution |
|-------|-----------|
| PPTX reader/writer (hand-rolled ZIP+XML) | Migrated to `ooxml-pml` (2026-02-24) |
| DOCX reader/writer | Uses `ooxml-wml` |
