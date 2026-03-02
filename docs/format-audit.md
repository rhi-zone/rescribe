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
| djot | 4† | 4† | jotdown | production | production |
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
| rtf | 4 | 4 | rtf-fmt (standalone) | production | production |
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

### RTF — on track

- Promoted to standalone `rtf-fmt` library (2026-03-02): proper AST with source spans,
  `parse(input) -> (RtfDoc, Vec<Diagnostic>)`, `emit(ast) -> String`, `events()` pull iterator
- Group-state stack bug fixed (formatting now scopes correctly to `{...}` groups)
- Windows-1252 codepage decoding added
- Reader promoted to 4-Fuzz (2026-03-02): 3 fuzz bugs found and fixed, both fuzz
  targets pass clean (reader: 4.3M execs, roundtrip: 2.2M execs, no crashes)
  - Fixed: `\'XX` hex escape used byte-level slice that panicked on multibyte UTF-8 boundaries
  - Fixed: `}` group close split adjacent Text nodes; merge_text_inlines() normalises output
  - Fixed: `\r`/`\n` in Text content emitted as bare chars (stripped on re-parse); now `\'0d`/`\'0a`
- Rescribe reader at 4-Fuzz (fixture suite: paragraph, heading, bold, italic, underline,
  strikethrough, superscript, subscript, special_chars, multiple_paragraphs + adversarial)
- Writer promoted to 3 (2026-03-02): 15 writer fixtures covering all inline and
  block constructs (paragraph, heading, strong, emphasis, underline, strikeout,
  code-inline, code-block, link, list-unordered, list-ordered, blockquote,
  horizontal-rule, superscript, subscript); all passing in CI
- Writer promoted to 4-Fuzz (2026-03-02): fuzz_rtf_writer (read→write pipeline,
  2.8M execs, no crashes); both reader and writer at 4-Fuzz
- Raw preservation of paragraph layout words via `para_props: String` on `Block::Paragraph` (2026-03-02):
  - Parser accumulates paragraph-scoped RTF control words (indents, spacing, tab stops, borders, etc.) verbatim
  - Emitter re-emits `para_props` after `\pard` so RTF→IR→RTF is lossless for layout formatting
  - rescribe-read-rtf surfaces as `rtf:para-props` string property; writer reads it back on re-emit
  - Added fixture `fixtures/rtf/para_props/` and unit test `test_roundtrip_para_props`
- Extended AST with paragraph alignment, inline font-size, and inline color (2026-03-02):
  - Parser: \colortbl pre-scan, \ql/\qr/\qc/\qj alignment words, \fs font-size, \cf color index
  - Emitter: color table emission, alignment words, FontSize/Color group emission
  - Roundtrip fuzz direction corrected: arbitrary canonical AST → emit → parse → assert equal
  - Three fuzz bugs found and fixed: color_table sentinel (0,0,0) mismatch, trailing ';'
    in colortbl creating spurious entries, color added to table from empty-text leaves
  - 510K roundtrip fuzz execs clean at new direction; 7 new corpus fixtures
- Production sign-off (2026-03-02): govdocs1 RTF corpus (1,125 real-world files):
  - 0 panics / crashes
  - 0 files with empty parsed output
  - \plain resets TextState; footnote/endnote/annotation/tc/xe/listable groups skip correctly
  - ~100 layout/formatting/revision-tracking control words added to ignored list
  - 89% of files have diagnostics for genuinely unknown niche control words (acceptable)

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
