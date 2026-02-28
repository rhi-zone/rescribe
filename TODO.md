# Rescribe Backlog

## Completed

- [x] CLI tool (`rescribe-cli`)
- [x] Metadata handling (YAML frontmatter, HTML meta tags)
- [x] Resource embedding (images, data URIs)
- [x] ParseOptions / EmitOptions implementation
- [x] Transforms crate (ShiftHeadings, StripEmpty, MergeText, etc.)
- [x] Pandoc JSON compatibility layer
- [x] DOCX reader/writer (via `ooxml-wml`)
- [x] PDF reader (text extraction)
- [x] PPTX reader/writer (migrated to `ooxml-pml`)
- [x] XLSX reader/writer (via `ooxml-sml`)
- [x] 54 readers, 64 writers — comprehensive format coverage

## Priority 1: Replace RTF with a library

RTF is the highest-risk handwritten impl — complex group nesting, hex escaping, codepage
handling, binary blobs. The `rtf-parser` crate on crates.io is a candidate.

- [ ] Evaluate `rtf-parser` (and alternatives) for feature completeness
- [ ] Replace `rescribe-read-rtf` with library-backed impl
- [ ] Replace `rescribe-write-rtf` with library-backed impl (harder — fewer RTF writers exist)
- [ ] Add fixture tests for RTF round-trip

## Priority 2: Local Pandoc fixture harness

Use the Pandoc test corpus at `~/git/pandoc/test/` as a local correctness oracle.
Fixtures are GPL so they never enter the repo — tests skip gracefully if the path is absent.

**Done:**
- [x] Harness in `crates/rescribe-fixtures/src/pandoc_harness.rs` — discovers corpus, runs
  pandoc as oracle, computes word-coverage, prints report
- [x] Single ignored test `all_formats` covering 25 formats
- [x] Pandoc added to `flake.nix` dev shell

**Done (2026-02-25):**
- [x] Identify the hanging parser — eprintln progress + 10 s timeout thread in `run_entry`
- [x] Add per-format timeout (10 s, distinguish timeout vs panic via channel errors)
- [x] Run with pandoc in PATH — **25/25 parsers OK, 20/25 at ≥90% coverage, ~2 s runtime**
- [x] Fix all hanging/crashing parsers:
  - twiki: `end.max(i+1)` guard against definition-list stall
  - vimwiki: bounds check before heading slice (was panic on bare `=`)
  - pod: handle `=over` / `=...` inside list-item content loop
  - rst/asciidoc: inline parser fallback advance on unmatched markup chars
- [x] Fix fb2 corpus path (`pb_brief.fb2` → `basic.fb2`)

**Remaining:**
- [ ] **Expand corpus entries** — some formats have weak test files (e.g. `org-select-tags.org`
  is a narrow org-mode test); add more entries per format once the suite is green
- [ ] **Improve low-coverage parsers** — twiki 79% (definition lists dropped), haddock 88%,
  pod 87%; investigate and fix the missing constructs
- [ ] **Typst coverage** — only 5% (ref=552 ours=36); the typst reader is very incomplete
- [ ] **AsciiDoc coverage** — pandoc can't read asciidoc (`--from asciidoc` unsupported), so
  oracle comparison is unavailable; consider alternative reference (asciidoctor)

## Priority 3: Owned fixture suite (MIT-licensed, lives in repo, runs in CI)

Complement the local Pandoc harness with our own golden files. Use Pandoc failures as
inspiration for what cases to cover, then write clean fixtures we can commit.

- [ ] Create `tests/fixtures/` directory structure (`{format}/input.{ext}`, `expected.json`)
- [ ] Write fixture runner that parses input, serializes to rescribe JSON, diffs vs expected
- [ ] Author fixtures for: markdown, html, org, rst, mediawiki (start small, grow with bugs)
- [ ] Add fixture tests to CI

## Priority 4: ODT writer correctness

ODT writer generates ODF zip by hand (404 lines, no schema library). No ODT equivalent
of `ooxml-pml` exists, so the path here is testing rather than library replacement.

- [ ] Run ODT output through LibreOffice (`libreoffice --headless --convert-to pdf`) in CI
  to catch malformed XML that Office apps reject
- [ ] Add roundtrip fixture tests (write ODT → re-read → compare node tree)

## Priority 5: RST and AsciiDoc reader correctness

Both are large handwritten parsers (1,263 and 1,290 lines respectively) with tricky specs.

- [ ] Run RST reader against Pandoc fixture harness (Priority 2), catalogue failures
- [ ] Run AsciiDoc reader against sample docs, catalogue failures
- [ ] Fix failures in priority order

## Priority 6: Fuzz testing

Prevent crashes/panics on malformed input for all formats.

- [ ] Audit which formats already have fuzz targets in `fuzz/`
- [ ] Add fuzz targets for formats that parse binary or complex structured data (RTF, ODT, EPUB)
- [ ] Run fuzzer against all handwritten text parsers for at least a few hours

## Someday/Maybe: Niche Formats

Low priority formats that could be added later if there's demand:

- [ ] Gemtext (Gemini protocol markup)
- [ ] Mermaid (diagram markup)
- [ ] PlantUML (UML diagrams)
- [ ] GraphViz DOT (graph descriptions)
- [ ] PHP Markdown Extra
- [ ] Setext (original lightweight markup)
- [ ] troff/nroff variants
- [ ] DITA (technical documentation)
- [ ] Confluence wiki markup
- [ ] Notion export format
- [ ] Roam Research export
- [ ] Logseq export
- [ ] MOBI / Mobipocket reader (KF7 — public spec, large existing corpus, deprecated for new content)
- [ ] AZW3 reader/writer (KF8 — EPUB3 content in Mobipocket container, well reverse-engineered)
- [ ] KFX reader (Kindle Format 10 — proprietary Ion-based binary, no public spec; blocked on ecosystem)
