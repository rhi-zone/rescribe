# Format Implementation Audit

Assessed 2026-02-24; stages updated 2026-03-21 (wiki formats 2→4; csv/tsv/ris/texinfo 2→4; mediawiki 3→4; odt/fb2/docbook/jats/opml/tei 3→4; commonmark/gfm/markdown-strict/multimarkdown 3→4; pulldown-cmark upgraded to 0.13; beamer/revealjs/slidy/s5/dzslides/slideous/context/ms/icml/chunkedhtml/plaintext writers 2→4); RST/Org/AsciiDoc promoted to 5-Production 2026-03-22 (benchmarks added, all vertical gates passed).

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
| **5-Production** | All gates passed (see "Vertical completion checklist" in CLAUDE.md) |

**Conventions:**
`†` = library-backed (upstream provides correctness guarantee; wrapper still needs fixtures and fuzz).
`–` = not applicable (no crate exists, or stage is not meaningful for this format).
Stage 3 is marked `–` for formats Pandoc cannot read — their path skips directly from fixtures to fuzz.

---

## Format Table

### Markdown family

| Format | R | W | Library | R-next | W-next |
|--------|---|---|---------|--------|--------|
| commonmark | 4† | 3† | pulldown-cmark | production | fuzz |
| gfm | 4† | 3† | pulldown-cmark | production | fuzz |
| markdown | 4† | 4† | pulldown-cmark | production | production |
| markdown-strict | 4† | 2† | pulldown-cmark | production | harness |
| multimarkdown | 4† | 2† | pulldown-cmark | production | harness |

### Lightweight markup

| Format | R | W | Library | R-next | W-next |
|--------|---|---|---------|--------|--------|
| djot | 4† | 4† | jotdown | production | production |
| org | 5 | 2 | hand | – | harness 100% (writer.org) fixtures (88) benchmarks ✓ |
| rst | 5 | 2 | hand | – | harness 100% fixtures (80) benchmarks ✓ |
| asciidoc | 5 | 2 | hand | – | harness N/A fixtures (84) benchmarks ✓ |
| textile | 4 | 2 | hand | fuzz | harness |
| muse | 4 | 2 | hand | production | harness |
| t2t | 4 | 2 | hand | fuzz | harness |
| markua | 4 | 2 | hand | fuzz | harness |
| fountain | 4 | 2 | hand | – (harness N/A) | coverage |
| typst | 1 | 2 | hand | partial→fixtures | harness |
| texinfo | 4 | 4 | hand | – (harness N/A) | production |
| bbcode | 4 | 2 | hand | – (harness N/A; Pandoc cannot read BBCode) | coverage |
| pod | 4 | 2 | hand | fuzz | harness |
| haddock | 4 | 2 | hand | fuzz | harness |
| ansi | 4 | 2 | hand | – (harness N/A; production) | coverage |
| man | 4 | 2 | hand | coverage | fuzz |

† Pandoc cannot read AsciiDoc (`--from asciidoc` unsupported); consider asciidoctor as alternate oracle.

### Wiki formats

| Format | R | W | Library | R-next | W-next |
|--------|---|---|---------|--------|--------|
| mediawiki | 4 | 4 | hand | production | harness |
| creole | 4 | 2 | hand | production | harness |
| dokuwiki | 4 | 2 | hand | production | harness |
| vimwiki | 4 | 2 | hand | production | harness |
| zimwiki | 4 | 2 | hand | production | harness |
| xwiki | 4 | 2 | hand | production | harness |
| twiki | 4 | 2 | hand | production | harness |
| tikiwiki | 4 | 2 | hand | production | harness |
| jira | 4 | 2 | hand | production | harness |

### Office / binary

| Format | R | W | Library | R-next | W-next |
|--------|---|---|---------|--------|--------|
| docx | 5† | 4† | ooxml-wml | – | production |
| odt | 4 | 2 | quick-xml / hand | production | harness |
| epub | 4† | 3† | epub / epub-builder | production | fuzz |
| fb2 | 4 | 2 | hand | production | harness |
| pptx | 4† | 3† | ooxml-pml | production | fuzz |
| xlsx | 4† | 3† | ooxml-sml | production | fuzz |
| pdf | 4† | – | pdf-extract | production | – |
| rtf | 5 | 5 | rtf-fmt (standalone) | – | – |
| mobi | – | – | – (planned) | – | – |
| azw3 | – | – | – (planned) | – | – |
| kfx | – | – | – (planned) | – | – |

### HTML and structured XML

| Format | R | W | Library | R-next | W-next |
|--------|---|---|---------|--------|--------|
| html | 4† | 3 | html5ever / hand | production | fuzz |
| docbook | 4 | 2 | hand | production | harness |
| jats | 4 | 2 | hand | production | harness |
| tei | 4 | 2 | hand | production | harness |
| opml | 4 | 2 | hand | production | harness |
| ipynb | 4† | 2† | serde_json | production | harness |
| latex | 4 | 2 | hand | production | harness |

### Bibliographic

| Format | R | W | Library | R-next | W-next |
|--------|---|---|---------|--------|--------|
| bibtex | 4† | 2† | biblatex | production | harness |
| biblatex | 4† | 2† | biblatex | production | harness |
| csl-json | 4† | 2† | serde_json | production | harness |
| ris | 4 | 4 | hand | – (harness N/A) | production |
| endnotexml | 4 | 2 | hand | – (harness N/A) | fuzz |

### Data / interchange

| Format | R | W | Library | R-next | W-next |
|--------|---|---|---------|--------|--------|
| csv | 4 | 4 | hand | – (harness N/A) | production |
| tsv | 4 | 4 | hand | – (harness N/A) | production |
| pandoc-json | 4† | 3† | serde_json | production | fuzz |
| native | 4 | 2 | hand | production | harness |

### Presentation / output-only

These formats have no reader; stage 3 (harness) is not applicable.

| Format | W | Library | W-next |
|--------|---|---------|--------|
| beamer | 4 | hand | coverage |
| revealjs | 4 | hand | coverage |
| slidy | 4 | hand | coverage |
| s5 | 4 | hand | coverage |
| dzslides | 4 | hand | coverage |
| slideous | 4 | hand | coverage |
| context | 4 | hand | coverage |
| ms | 4 | hand | coverage |
| icml | 4 | hand | coverage |
| chunkedhtml | 4 | hand | coverage |
| plaintext | 4 | hand | coverage |

---

## Standalone format crate API coverage

**Goal: every format without a quality ecosystem crate gets a proper standalone library here.
The target state is all checkmarks in this table.**

The Rust ecosystem is missing solid crates for most document formats. rescribe fixes this as
a byproduct: each hand-written format vertical produces a publishable standalone crate with
a full API surface. Library-backed formats fall into two categories:
- **Third-party** (pulldown-cmark, html5ever, etc.) — not our codebase; contribute upstream if gaps exist.
- **Ours** (ooxml-wml, ooxml-sml, ooxml-pml) — same standard applies; propose changes directly.
  ooxml-* is largely codegen'd so raising it to full API coverage is cheaper than it looks.

Features (all ship as Cargo features, all on by default — see `docs/format-library-design.md`):
- `ast` — `parse(input) -> (Ast, Vec<Diagnostic>)`, Span on every node
- `stream` — `events(input) -> impl Iterator<Item = Event>`, no full AST, full input in memory
- `batch` — chunk-driven `Parser` (feed/finish), O(working state), handles arbitrarily large files
- `w-stream` — closure/visitor writer, emits bytes immediately, no full tree required
- `w-build` — `emit(ast)` builder, trivial wrapper over `w-stream`

`✓` = complete · `~` = MVP (full-input iterator or simple builder, not yet chunk-driven/fully streaming) · ` ` = not started

### Priority formats (actively worked)

| Crate | ast | stream | batch | w-stream | w-build |
|-------|-----|--------|-------|----------|---------|
| rtf-fmt | ✓ | ~ | | | ✓ |
| rst-fmt | ✓ | | | ~ | |
| asciidoc | ✓ | | | ~ | ✓ |
| org-fmt | ✓ | | | | ✓ |
| djot-fmt | | | | | |
| textile-fmt | ✓ | | | | ✓ |

### Remaining hand-written formats (crate exists, API not started)

| Crate | ast | stream | batch | w-stream | w-build |
|-------|-----|--------|-------|----------|---------|
| muse-fmt | ✓ | | | | ✓ |
| t2t | ✓ | | ✓ | | ✓ |
| markua | ast.rs parse.rs emit.rs | Span+Diagnostic; infallible parse; strip_spans() | fuzz_markua_reader (559K runs) fuzz_markua_roundtrip (759K runs) | – | – |
| fountain-fmt | ✓ | | ✓ | | ✓ |
| mediawiki-fmt | | | | | |
| creole | ast.rs parse.rs emit.rs | Span+Diagnostic; infallible parse; strip_spans() | fuzz_creole_reader (842K runs) fuzz_creole_roundtrip (403K runs) | – | – |
| dokuwiki | ast.rs parse.rs emit.rs | Span+Diagnostic; infallible parse; strip_spans() | fuzz_dokuwiki_reader (628K runs) fuzz_dokuwiki_roundtrip (378K runs) | – | – |
| vimwiki-fmt | ast.rs parse.rs emit.rs | Span+Diagnostic; infallible parse; strip_spans() | fuzz_vimwiki_reader (610K runs) fuzz_vimwiki_roundtrip (361K runs) | – | – |
| zimwiki | ast.rs parse.rs emit.rs | Span+Diagnostic; infallible parse; strip_spans() | fuzz_zimwiki_reader (416K runs) fuzz_zimwiki_roundtrip (390K runs) | – | – |
| xwiki | ast.rs parse.rs emit.rs | Span+Diagnostic; infallible parse; strip_spans() | fuzz_xwiki_reader (489K runs) fuzz_xwiki_roundtrip (427K runs) | – | – |
| twiki | ast.rs parse.rs emit.rs | Span+Diagnostic; infallible parse; strip_spans() | fuzz_twiki_reader (1017K runs) fuzz_twiki_roundtrip (442K runs) | – | – |
| tikiwiki | ast.rs parse.rs emit.rs | Span+Diagnostic; infallible parse; strip_spans() | fuzz_tikiwiki_reader (429K runs) fuzz_tikiwiki_roundtrip (425K runs) | – | – |
| jira-fmt | ast.rs parse.rs emit.rs | Span+Diagnostic; infallible parse; strip_spans() | fuzz_jira_reader (416K runs) fuzz_jira_roundtrip (333K runs) | – | – |
| typst (TBD) | | | | | |
| texinfo | ast.rs parse.rs emit.rs | Span+Diagnostic; infallible parse; strip_spans(); fixed unterminated-command panic + unknown-directive infinite loop | fuzz_texinfo_reader (1.5M runs) fuzz_texinfo_roundtrip (592K runs) | – | – |
| bbcode-fmt | ast.rs parse.rs emit.rs | Span+Diagnostic; infallible parse; strip_spans() | fuzz_bbcode_reader (1.3M runs) fuzz_bbcode_roundtrip (348K runs) | – | – |
| pod-fmt | ast.rs parse.rs emit.rs | Span+Diagnostic; infallible parse; strip_spans() | fuzz_pod_reader (863K runs) fuzz_pod_roundtrip (375K runs) | – | – |
| haddock-fmt | ast.rs parse.rs emit.rs | Span+Diagnostic; infallible parse; strip_spans() | fuzz_haddock_reader (1.1M runs) fuzz_haddock_roundtrip (415K runs) | – | – |
| ansi-fmt | ast.rs parse.rs emit.rs | Span+Diagnostic; infallible parse; strip_spans() | fuzz_ansi_reader + fuzz_ansi_roundtrip | – | – |
| man-fmt | ast.rs parse.rs emit.rs | Span+Diagnostic; infallible parse | fuzz_man_reader (2M runs) fuzz_man_roundtrip (855K runs) | – | – |
| mediawiki-fmt | ast.rs parse.rs emit.rs | Span+Diagnostic; infallible parse; strip_spans(); adapter crates updated | fuzz_mediawiki_reader (1.5M runs) fuzz_mediawiki_roundtrip (850K runs) | – | – |
| csv-fmt | ast.rs parse.rs emit.rs | Span+Diagnostic; infallible parse; strip_spans(); adapter crates updated | fuzz_csv_reader (807K runs) fuzz_csv_roundtrip (clean) | – | – |
| tsv-fmt | ast.rs parse.rs emit.rs | Span+Diagnostic; infallible parse; strip_spans(); adapter crates updated; fixed whitespace-only row filter | fuzz_tsv_reader (1.1M runs) fuzz_tsv_roundtrip (670K runs) | – | – |
| ris | ast.rs parse.rs emit.rs | Span+Diagnostic; infallible parse; strip_spans(); fixed char-boundary panic on multi-byte tag chars | fuzz_ris_reader (1.1M runs) fuzz_ris_roundtrip (241K runs) | – | – |

### Formats still needing a standalone crate

odt, fb2, docbook, jats, tei, opml, latex, endnotexml, native

---

## Risk areas

### RTF — production (2026-03-02)

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
- **Promoted to 5-Production (2026-03-02)**: all gates passed
- Semantic character words modelled: `\caps`→AllCaps, `\scaps`→SmallCaps, `\v`/`\webhidden`→Hidden;
  `ALL_CAPS` and `HIDDEN` added to rescribe-std; all have fixtures and roundtrip tests
- **`rtf:char-props` implemented**: `\dn`/`\up`/`\shad`/`\shading`/`\expnd`/`\expndtw`/
  `\kerning`/`\charscalex`/`\jcompress`/`\jexpand`/`\chcfpat`/`\chcbpat`/`\chshdng`/
  `\highlight` accumulated verbatim as `Inline::CharSpan { char_props }`, surfaced as
  `rtf:char-span` node with `rtf:char-props` property; losslessly round-tripped
- **Parser spec-compliance fix**: control word lexer now requires lowercase-only start
  (`is_ascii_lowercase`), so uppercase sequences from binary garbage no longer generate
  spurious diagnostics
- **`\bin` handler added**: RTF binary embedding (`\binN`) now skips N raw bytes;
  architectural note: parser takes `&str` so files with `\bin` blocks containing
  non-UTF-8 bytes are excluded by the corpus runner (correct); true fix requires
  byte-level parsing (future work)
- govdocs1 RTF corpus (1,077 UTF-8-clean files, 48 skipped as binary):
  - 0 panics / crashes
  - 0 files with empty parsed output
  - **0 files (0%) with diagnostics** — complete elimination via char-props + triage
  - ~150 layout/formatting/revision-tracking control words in ignored list

### ODT writer — medium risk
- 404 lines building ODF zip by hand (no schema library)
- Reader uses `quick-xml`; writer generates raw XML strings
- No ODT equivalent of `ooxml-wml`/`ooxml-pml` exists in the ecosystem
- Reader promoted to 3-Harness (100% coverage, 6 corpus files, 2026-03-01)

### RST reader — 5-Production (2026-03-22)
- Pandoc harness: 100% word coverage on rst-reader.rst (ref=618, ours=668)
- fuzz_rst_reader: 201K runs clean; fuzz_rst_roundtrip: 467K runs clean (2026-03-22)
- Parser fixes: "text::" introductory paragraph now emitted before code block (pending_block
  pattern); `<url>`_ empty link text uses URL as display text; pending_block loop in main
  parse avoids losing blocks at EOF
- Fixtures: 80 total; COVERAGE.md all boxes checked
- Benchmarks: rst_parse_small 3.3µs, rst_parse_medium 30µs, rst_emit_medium 2.5µs

### Djot reader/writer — 4-Fuzz (2026-03-21)
- fuzz_djot_roundtrip rewritten to correct direction: arbitrary rescribe doc → emit → parse
  (old direction was parse(bytes) → emit → parse, which is vacuous if reader drops constructs)
- New fuzz target uses FuzzBlock/FuzzInline pattern (same as rst_roundtrip, asciidoc_roundtrip)
- Sanitiser strips Djot markup chars: `*`, `_`, `#`, `-`, `.`, `)`, `+`, `^`, `~`, `[`, `]`,
  `{`, `}`, `\`, `$`, `<`, `>`, `'`, `"`, `|` — prevents roundtrip failures from inline/block
  marker reinterpretation. Code inlines excluded: adjacent code spans produce ```` `` ````
  delimiters that jotdown re-parses as a 2-backtick verbatim span (TODO: fix writer).
- 1,005,513 fuzz runs clean (300s, 2026-03-21)
- 14 new fixtures added: superscript, subscript, underline, image, table, footnote,
  definition-list, raw-block, raw-inline, math, task-list, div, soft-break, line-break, span
  (total: 29 fixtures)
- Math syntax: `$\`...\`$` / `$$\`...\`$$` (dollar+backtick, not `$...$`)
- Raw block syntax: ` ``` =format` (space before `=`, not `{=format}`)

### Org reader — 5-Production (2026-03-22)
- Pandoc harness: 100% word coverage on writer.org (ref=919, ours=995); org-select-tags.org
  at 97% due to Pandoc applying select_tags document filtering (not a parsing gap)
- fuzz_org_reader: 499K runs clean; fuzz_org_roundtrip: 279K runs clean (2026-03-22)
- Parser fix: `$` math inline rejected when next char is digit (fixes $20 currency being
  parsed as math and consuming surrounding words like "socks")
- Fixtures: 88 total; COVERAGE.md all boxes checked
- Benchmarks: org_parse_small 3.4µs, org_parse_medium 53µs, org_emit_medium 2.9µs

### AsciiDoc reader — 5-Production (2026-03-22)
- lib.rs split into ast.rs / parse.rs / emit.rs; Span/Diagnostic added; parse() now infallible
- strip_spans() implemented on all AST types for roundtrip comparison
- fuzz_asciidoc_reader: 507K runs clean; fuzz_asciidoc_roundtrip: 225K runs clean (2026-03-22)
- Pandoc harness: N/A (`--from asciidoc` unsupported; asciidoctor is the alternate oracle)
- Fixtures: 84 total; COVERAGE.md all boxes checked
- Benchmarks: asciidoc_parse_small 6.6µs, asciidoc_parse_medium 48µs, asciidoc_emit_medium 1.9µs
- Known roundtrip gap: [role]#text# inline syntax (Strikeout/Underline/SmallCaps) emitted as
  [line-through]#text# / [underline]#text# / [small-caps]#text# but re-parsed as Highlight
- `asciidoc-rs` exists on crates.io but is immature; asciidoctor is the alternate oracle

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
