# Rescribe Roadmap

Per-format status is tracked in `docs/format-audit.md` using the maturity pipeline
(0-Stub → 1-Partial → 2-Fixtures → 3-Harness → 4-Fuzz → 5-Production).
This file describes milestones, format tiers, and cross-cutting work.

---

## Completed

- [x] CLI tool (`rescribe-cli`)
- [x] Metadata handling (YAML frontmatter, HTML meta tags)
- [x] Resource embedding (images, data URIs)
- [x] ParseOptions / EmitOptions implementation
- [x] Transforms crate (ShiftHeadings, StripEmpty, MergeText, etc.)
- [x] Pandoc JSON compatibility layer
- [x] DOCX reader/writer (via `ooxml-wml`)
- [x] PDF reader (text extraction via `pdf-extract`)
- [x] PPTX reader/writer (migrated to `ooxml-pml`)
- [x] XLSX reader/writer (via `ooxml-sml`)
- [x] 54 readers, 64 writers — comprehensive format coverage
- [x] Pandoc harness — 25/25 parsers, 20/25 at ≥90% coverage

---

## Format Tiers

Tiers determine how much investment a format gets. Higher tiers reach production first;
lower tiers get fixtures and correctness but not necessarily fuzz hardening.

### Tier A — Production priority

The formats people actually use for document authoring and conversion.
Target: **5-Production**.

Markdown family (commonmark, gfm, markdown, markdown-strict, multimarkdown), HTML,
DOCX, EPUB, AZW3, Org, RST, AsciiDoc, LaTeX, Djot, ODT, PPTX, XLSX, PDF

### Tier B — Correctness, not urgent

Formats with real use cases but lower conversion frequency.
Target: **3-Harness** (or 2-Fixtures where harness is N/A), fuzz as bandwidth allows.

Typst, MediaWiki, DocBook, JATS, TEI, FB2, RTF, Man,
BibTeX, BibLaTeX, CSL-JSON, RIS, EndNote XML,
CSV, TSV, OPML, iPynb, Pandoc JSON, Native,
MOBI, KFX

### Tier C — Best-effort

Niche formats; fixtures are sufficient, no production guarantee.
Target: **2-Fixtures**.

Creole, DokuWiki, VimWiki, ZimWiki, XWiki, TWiki, TikiWiki, Jira,
BBCode, ANSI, Fountain, Haddock, Muse, t2t, Markua, Texinfo, POD

### Tier D — Output-only, low investment

Write-only presentation formats. Correctness is hard to verify programmatically.
Target: **2-Fixtures** (round-trip not required).

Beamer, reveal.js, Slidy, S5, DZSlides, Slideous, ConTeXt, ms, ICML,
Chunked HTML, Plaintext

---

## Milestones

### M1: Fixture CI — all formats at ≥2-Fixtures, running in CI

The owned fixture suite (`fixtures/`) exists but does not yet run in CI.

- [ ] Write fixture runner: parse input, serialize to rescribe JSON, diff vs expected
- [ ] Hook fixture runner into CI
- [ ] Fill gaps: any format still at 0-Stub or 1-Partial gets minimum fixture coverage
- [ ] Presentation writers (Tier D): author at least one fixture each

**Done when:** CI is green, every format has at least one passing fixture.

### M2: Tier A correct — all Tier A formats at ≥3-Harness

- [ ] Improve low-coverage parsers: twiki 79%, haddock 88%, pod 87%
- [ ] Typst reader: currently at 5% — needs significant work before harness is meaningful
- [ ] AsciiDoc: Pandoc oracle unavailable; set up asciidoctor as alternate reference
- [ ] Expand Pandoc harness corpus entries (some formats have narrow test files)
- [ ] AZW3 reader/writer: implement (boko as reference, MIT attribution)

**Done when:** All Tier A formats at 3-Harness (or equivalent for harness-N/A formats).

### M3: Tier A hardened — all Tier A formats at 4-Fuzz

Existing fuzz targets: html (reader + roundtrip), markdown (reader + roundtrip),
latex (reader), org (reader), pandoc-json (reader), pdf (reader).

- [ ] Add fuzz targets for remaining Tier A formats: epub, docx, odt, pptx, xlsx, rst,
  asciidoc, djot, azw3
- [ ] Run all fuzz targets for meaningful duration (hours, not seconds)
- [ ] Fix all panics/crashes found

**Done when:** All Tier A formats at 4-Fuzz.

### M4: Tier B correct — all Tier B formats at ≥3-Harness / 2-Fixtures

- [ ] RTF: evaluate `rtf-parser` crate; replace hand-rolled impl if viable
- [ ] ODT writer: validate via LibreOffice headless in CI
- [ ] MOBI reader: implement (boko as reference, MIT attribution)
- [ ] KFX reader/writer: implement (Ion spec + boko as reference for schema layer)
- [ ] Remaining Tier B formats: audit and bring to 3-Harness or 2-Fixtures

**Done when:** All Tier B formats at their target stage.

### M5: Tier A at 5-Production

Final production pass for Tier A formats.

- [ ] Audit each Tier A format for known gaps and edge cases
- [ ] Ensure fidelity warnings fire correctly for all lossy conversions
- [ ] Roundtrip fixture coverage: input → parse → emit → parse → IR == IR

**Done when:** All Tier A formats at 5-Production.

---

## Someday/Maybe

Low priority; add if there's demand.

- [ ] Gemtext (Gemini protocol markup)
- [ ] Mermaid (diagram markup)
- [ ] PlantUML (UML diagrams)
- [ ] GraphViz DOT (graph descriptions)
- [ ] PHP Markdown Extra
- [ ] Setext (original lightweight markup)
- [ ] troff/nroff variants beyond man
- [ ] DITA (technical documentation)
- [ ] Confluence wiki markup
- [ ] Notion export format
- [ ] Roam Research export
- [ ] Logseq export
