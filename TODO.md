## Status Indicator
- Current: ◐ Fleshed Out — kept despite high commit count (112 commits, 207 Rust files)
- Needs hardening/verification work before upgrading to ● Potentially Mature
- Lots of code, but needs more verification to count as mature

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

## Architecture: Format Crate Split (M0-style, ongoing)

### Motivation

`rescribe-read-{format}` and `rescribe-write-{format}` should be **thin IR adapters only** —
they translate between rescribe's `Document` IR and the format, nothing more.

Hand-rolled format logic (tokenizer, AST, builder) belongs in a **standalone crate** with
no rescribe dependency, so it can be used, tested, and fuzzed independently.

Library-backed formats (pulldown-cmark, html5ever, ooxml-*, etc.) already follow this
pattern — we wrap them. Hand-rolled formats should look the same from the outside.

### Naming convention

- `{format}` when the crates.io name is available (e.g. `asciidoc`, `odt`, `docbook`)
- `{format}-fmt` when taken (e.g. `rst-fmt`, `rtf-fmt`, `latex-fmt`)

### Crate layout (target state)

```
crates/
├── formats/             ← standalone format libraries, no rescribe dep
│   ├── rst-fmt/         # RST parser + builder API
│   ├── asciidoc/        # AsciiDoc parser + builder API
│   ├── rtf-fmt/         # RTF tokenizer + builder API
│   ├── org-fmt/         # Org-mode parser + builder API
│   ├── latex-fmt/       # LaTeX parser + builder API
│   └── ...              # one per hand-rolled format
├── readers/             ← thin IR adapters: {format} → rescribe Document
└── writers/             ← thin IR adapters: rescribe Document → {format}
```

### Name availability (checked 2026-03-01)

Available (use plain name): asciidoc, t2t, markua, texinfo, creole, dokuwiki, zimwiki,
xwiki, twiki, tikiwiki, docbook, native, ris, endnotexml, odt

Need `-fmt` suffix: rst, org, rtf, textile, mediawiki, muse, fountain, bbcode, pod,
haddock, ansi, man, vimwiki, jira, fb2, opml, tsv, tei, typst (already `typst-syntax`),
djot (already `jotdown`), latex

### Migration order (suggested)

1. `rtf-fmt` — highest risk, most isolated logic, good proving ground
2. `rst-fmt` — large parser, complex spec, highest correctness benefit
3. `asciidoc` — similar to RST
4. `org-fmt` — already partially modular (handwritten.rs), 3-Harness reader
5. Remaining formats incrementally

### What each standalone crate exposes

- **Parser**: takes raw bytes/str → returns owned AST (no rescribe types)
- **Builder**: typed API for constructing valid output → returns `Vec<u8>` or `String`
- **No `Document`, `Node`, or `Properties`** anywhere in the standalone crate

---

## Milestones

### M1: Fixture CI — all formats at ≥2-Fixtures, running in CI ✓

- [x] Write fixture runner (`rescribe-fixtures`, `tests/run.rs`)
- [x] Hook fixture runner into CI (`cargo test --all-targets`)
- [x] Fill gaps: all formats at ≥2-Fixtures
- [x] Presentation writers (Tier D): writer fixture infrastructure + one fixture each
- [x] Fixture spec v1.2: writer fixture format documented

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

- [ ] Marp (CommonMark + slide separators + speaker-note comments; ~50 lines on top of GFM reader; write support is Beamer/revealjs-style)
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
