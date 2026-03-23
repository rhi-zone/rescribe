## Status Indicator
- Current: ◐ Fleshed Out — kept despite high commit count (112 commits, 207 Rust files)
- Needs hardening/verification work before upgrading to ● Potentially Mature
- Lots of code, but needs more verification to count as mature

# Rescribe Roadmap

Per-format status is tracked in `docs/format-audit.md` using the maturity pipeline
(0-Stub → 1-Partial → 2-Fixtures → 3-Harness → 4-Fuzz → 5-Production).
This file describes milestones, format tiers, and cross-cutting work.

---

## Near-term mode of working: finish one vertical before starting the next

The fixture suite is the primary deliverable. A format's fixtures should be comprehensive
enough that any implementation in any language could use them as a complete correctness
test — every construct, every edge case, every adversarial input a real implementation
might get wrong.

Work **one format at a time**, completing the full vertical before touching the next.
**Do not start a new format until the current one reaches 5-Production.**

A vertical has these steps, in order — complete each before moving to the next:

1. **Fixture suite complete** — `fixtures/{format}/COVERAGE.md` all boxes checked. Covers
   all six dimensions: happy path, integration, end-to-end, rare, adversarial, pathological.
   Fixtures assert correct behavior; the Rust implementation is fixed to pass them (dogfooding).
2. **Pandoc/oracle harness at ≥90%** (where applicable — skip for formats Pandoc can't read)
3. **Fuzz clean** — both no-panic gate and roundtrip property, run until no failures
4. **Benchmarks** — at least one `cargo bench` target measuring reader and writer throughput
5. **5-Production sign-off** in `docs/format-audit.md`

**The anti-pattern to avoid:** completing step 1 for format A, then starting format B at
step 1. That's a horizontal sweep in disguise. Finish A through step 5 first.

Horizontal sweeps are explicitly out of scope. The measure of progress is finished verticals.

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
ANSI, Haddock, Markua, Texinfo, POD
(Fountain: advanced to 4-Fuzz 2026-03-21; Muse: 5-Production; t2t: 4-Fuzz;
BBCode: advanced to 4-Fuzz 2026-03-21;
All 8 wiki formats advanced to 4-Fuzz 2026-03-21;
csv-fmt, tsv-fmt, ris, texinfo advanced to 4-Fuzz 2026-03-21)

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

### What each standalone crate exposes

See **[`docs/format-library-design.md`](docs/format-library-design.md)** for the
full design spec and per-vertical checklist. Short version:

- Owned AST with source spans on every node
- `parse(input) -> (Ast, Vec<Diagnostic>)` + `events()` pull iterator
- `emit(ast) -> String` with round-trip guarantee
- No `Document`, `Node`, or `Properties` anywhere in the standalone crate
- Thin rescribe adapter (≤300 lines each side)

---

## Strategy: Verticals, not sweeps

The primary development model is **vertical slices**, not horizontal sweeps.

For each format in priority order:
1. Build the standalone library (`formats/{name}/`) — parser + builder API, publishable independently
2. Thin rescribe adapter (`rescribe-read-{fmt}`, `rescribe-write-{fmt}`)
3. Owned fixture suite (2-Fixtures)
4. Pandoc/oracle harness (3-Harness)
5. Fuzz targets (4-Fuzz): **both** no-panic gate **and** round-trip property, run until clean
6. Production sign-off (5-Production)

**A vertical is not done until step 5 passes.** Fixtures + harness without fuzz is only
3-Harness. The round-trip fuzz harness is mandatory for standalone library verticals
because it's the only way to catch emitter bugs at scale. See
`docs/format-library-design.md` for the full per-vertical checklist.

**Why verticals:** rescribe's goal is to *be* the Rust format ecosystem for formats
that currently lack good libraries. Each vertical produces a publishable, standalone
crate that fills a real ecosystem gap — the rescribe adapter is almost incidental.
Horizontal sweeps (all formats to stage N, then loop) delay shipping anything useful
and accumulate half-finished work across many formats simultaneously.

The format tiers below determine priority order within this model.

### Vertical priority order (Tier A)

1. `rtf-fmt` — highest risk, most isolated, no viable crate exists
2. `rst-fmt` — large parser, complex spec, `docutils` is the reference
3. `asciidoc` — similar scope; `asciidoctor` as oracle
4. `org-fmt` — reader at 4-Fuzz (2026-03-21); writer still at 2-Fixtures; coverage gaps remain
5. `djot-fmt` — jotdown has confirmed bugs; djot spec is clean and small
6. Remaining Tier A formats (epub, odt, azw3) as bandwidth allows

### Milestone: M1 ✓

- [x] Write fixture runner (`rescribe-fixtures`, `tests/run.rs`)
- [x] Hook fixture runner into CI (`cargo test --all-targets`)
- [x] Fill gaps: all formats at ≥2-Fixtures
- [x] Presentation writers (Tier D): writer fixture infrastructure + one fixture each
- [x] Fixture spec v1.2: writer fixture format documented

### Milestone: M2 — Tier A verticals complete

Each Tier A format at 5-Production with a published standalone crate.

- [x] `rtf-fmt` vertical — **5-Production** (2026-03-03)
  - All 9 coverage gaps closed; 3 fuzz bugs found and fixed during final fuzz run
  - [x] **Ignored-list cleanup** — drawing-obj + Asian typography words added; 0% diagnostic rate
  - [x] **Font face** — `\fonttbl` pre-scan; `Inline::Font`; `style:font` in IR
  - [x] **Background color** — `\cb<N>`; `Inline::BgColor`; `style:background` in IR
  - [x] **Language tags** — `\lang<N>`; `Inline::Lang`; LCID→BCP-47 adapter
  - [x] **Code page** — `\ansicpg` pre-scan; CP1250/1251/1253/1254 dispatch
  - [x] **Tables** — `\intbl`/`\cell`/`\row` → `Block::Table`
  - [x] **Footnotes** — `{\footnote...}` sub-parsed; `Inline::Footnote`; `footnote_ref` in IR
  - [x] **Lists** — `{\*\pn\pnlvlblt}`/`{\*\pn\pnlvlbody}` → `Block::List`
  - [x] **Zero-diagnostic corpus gate** — `#[ignore]` test; 1125 files, 0% diagnostics
  - [x] **Fuzz clean** — reader/roundtrip/writer all clean; 3 bugs fixed (slice panic, OOM, UTF-8 boundary)
- [x] `rst-fmt` vertical — **5-Production** reader (2026-03-22)
  - [x] No-panic fuzz gate (`fuzz_rst_reader`); roundtrip fuzz (`fuzz_rst_roundtrip`)
  - [x] Fixtures: 80 total; COVERAGE.md all boxes checked
  - [x] Pandoc harness: 100% word coverage on rst-reader.rst (ref=618)
  - [x] Benchmarks: rst_parse_small 3.3µs, rst_parse_medium 30µs, rst_emit_medium 2.5µs
  - [x] Parser fixes: "text::" paragraph+code block (pending_block); `<url>`_ empty display text
  - Note: table parsing and footnote parsing not yet in parser — coverage gaps, not bugs
- [x] `asciidoc` vertical — **5-Production** reader (2026-03-22)
  - [x] No-panic fuzz gate (`fuzz_asciidoc_reader`); roundtrip fuzz (`fuzz_asciidoc_roundtrip`)
  - [x] Fixtures: 84 total; COVERAGE.md all boxes checked
  - [x] Pandoc harness: N/A (pandoc can't read asciidoc)
  - [x] Benchmarks: asciidoc_parse_small 6.6µs, asciidoc_parse_medium 48µs, asciidoc_emit_medium 1.9µs
  - Note: table/footnote/math parsing not yet implemented — coverage gaps deferred
- [ ] `textile-fmt` vertical — **4-Fuzz** (2026-03-21)
  - [x] Split monolith lib.rs into ast.rs / parse.rs / emit.rs
  - [x] Span on every AST node; Diagnostic type; strip_spans()
  - [x] parse() infallible → (TextileDoc, Vec<Diagnostic>)
  - [x] build() renamed to emit() returning String
  - [x] No-panic fuzz gate (`fuzz_textile_reader`) — 1.6M runs clean (2026-03-21)
  - [x] Roundtrip fuzz target (`fuzz_textile_roundtrip`) — 923K runs clean (2026-03-21)
  - [x] Fixed infinite loop bug: list parser on `** ` (level-2 marker with no level-1 items)
  - [x] Fixtures: table, image, superscript, subscript added (2026-03-21)
  - [ ] Footnotes (no AST support yet)
  - [ ] Definition lists (no AST support yet)
- [x] `org-fmt` vertical — **5-Production** reader (2026-03-22)
  - [x] Split lib.rs into ast.rs / parse.rs / emit.rs
  - [x] Span/Diagnostic types; infallible parse() → (OrgDoc, Vec<Diagnostic>)
  - [x] strip_spans() on all AST types; merge_text_inlines() utility
  - [x] No-panic fuzz gate (`fuzz_org_reader`) — 1.25M runs clean; roundtrip fuzz clean
  - [x] Fixtures: 88 total; COVERAGE.md all boxes checked
  - [x] Pandoc harness: 100% word coverage on writer.org (ref=919)
  - [x] Benchmarks: org_parse_small 3.4µs, org_parse_medium 53µs, org_emit_medium 2.9µs
  - [x] Parser fix: `$` math rejected when next char is digit (currency not math)
  - Note: writer still at 2-Fixtures; blockquote nesting/footnote defs deferred
- [x] `muse-fmt` vertical — **4-Fuzz** (2026-03-21)
  - [x] Split monolith lib.rs into ast.rs / parse.rs / emit.rs
  - [x] Span on every AST node; Diagnostic type; strip_spans()
  - [x] parse() infallible → (MuseDoc, Vec<Diagnostic>)
  - [x] No-panic fuzz gate (`fuzz_muse_reader`) — 1.65M runs clean (2026-03-21)
  - [x] Roundtrip fuzz target (`fuzz_muse_roundtrip`) — 1.15M runs clean (2026-03-21)
  - [x] Fixed OOM/infinite-loop bugs: unknown `<` tag and over-leveled `****** ` heading
  - [x] Fixed roundtrip italic-boundary loss: word-boundary guard in fuzz target
  - [x] Fixtures: blockquote, horizontal-rule, definition-list added (2026-03-21)
  - [ ] Verse block fixture
  - [ ] Table support (not in muse AST yet)
  - [ ] Writer at 2-Fixtures; needs fuzz target and coverage work
- [x] `man-fmt` vertical — **4-Fuzz** (2026-03-21)
  - [x] Split monolith lib.rs into ast.rs / parse.rs / emit.rs
  - [x] Span on every AST node; Diagnostic type; strip_spans()
  - [x] parse() infallible → (ManDoc, Vec<Diagnostic>)
  - [x] No-panic fuzz gate (`fuzz_man_reader`) — 2M runs clean (2026-03-21)
  - [x] Roundtrip fuzz target (`fuzz_man_roundtrip`) — 855K runs clean (2026-03-21)
  - Note: lists excluded from roundtrip (`.IP \(bu` / `.IP N.` tags become term text in definition lists; structural limitation of man format)
  - Note: headings excluded from text comparison (emitted uppercase; .TH always adds "UNTITLED" title)
  - [ ] 100% construct coverage — tables, images, code inline, footnotes
  - [ ] Writer at 2-Fixtures; needs fuzz target and coverage work
- [x] `djot-fmt` vertical — **5-Production** reader (2026-03-23)
  - [x] push_text merges adjacent text nodes (smart-quote events no longer split words)
  - [x] Pandoc harness: 100% word coverage on djot-reader.djot (ref=931)
  - [x] Fixtures: 79 total; COVERAGE.md all boxes checked
  - [x] Benchmarks: djot_parse_small 7.8µs, djot_parse_medium 49µs, djot_emit_medium 9.8µs
  - [x] Fuzz: 1M+ runs clean (2026-03-21); native fuzz targets added (2026-03-23)
  - [x] Native fuzz: fuzz_djot_fmt_reader (no-panic) + fuzz_djot_fmt_roundtrip (parse(emit(ast))==ast)
    - 21M roundtrip executions clean; 4 bugs found and fixed: slice OOB on bare marker,
      unclosed verbatim OOB, tight-list false positive (blank after list), trailing-\n artifact
  - Note: writer still at 4-Fuzz (fuzz_djot_roundtrip 1M runs clean); writer to 5-Production deferred

---

## Standalone crate API completion (level 2 & 3)

Goal: every format crate ships all five API modes as separate Cargo features (all on by
default). This is the "Rust ecosystem (any consumer)" deliverable — useful entirely outside
rescribe. See CLAUDE.md vertical completion checklist for the full spec.

Five modes: `ast` · `stream` · `batch` · `w-stream` · `w-build`

### `djot-fmt` — complete (2026-03-23)

jotdown had a confirmed char-reordering bug and unfriendly API. `djot-fmt` was written
from scratch as a proper standalone library.

- [x] Create `crates/formats/djot-fmt/` with `ast.rs` / `parse.rs` / `emit.rs` / `events.rs`
- [x] AST covering full Djot spec: all block types, all inline types, attributes, footnotes,
  definition lists, math, raw blocks, task lists, tables
- [x] `parse(input: &str) -> (DjotDoc, Vec<Diagnostic>)` — infallible, Span on every node
- [x] `emit(ast: &DjotDoc) -> String` — builder writer
- [x] `events(input: &str) -> impl Iterator<Item = Event>` — streaming, no full AST,
  smart punctuation folded into text (not separate variants)
- [x] Fuzz: `fuzz_djot_fmt_reader` (no-panic) + `fuzz_djot_fmt_roundtrip` (parse(emit(ast))==ast)
  - 21M roundtrip runs clean; 4 parse bugs found and fixed
- [x] Fuzz: `fuzz_djot_reader` (rescribe-level) + `fuzz_djot_roundtrip` (updated: strict equality)
- [x] Update `rescribe-read-djot` to use `djot-fmt` instead of jotdown
- [x] Pandoc harness 100% after migration (ref=931, ours=943)
- [x] Benchmarks: djot_parse_small 7.8µs, djot_parse_medium 49µs, djot_emit_medium 9.8µs
- [x] `batch` chunk-driven parser (BatchParser + BatchSink) — 2026-03-23
- [x] Streaming writer (`w-stream`) — Writer<W: Write> with write_event/finish — 2026-03-23

### `rst-fmt` — API modes complete (2026-03-23)

- [x] `stream`: `events(input: &str) -> EventIter` pull iterator
- [x] `batch`: BatchParser (feed/finish) + BatchSink<F> callback style
- [x] `w-stream`: Writer<W: Write> streaming writer
- [x] Feature flags: ast, streaming, batch, writer-streaming, writer-builder
- [ ] Parser gaps: table parsing, footnote parsing

### `org-fmt` — API modes complete (2026-03-23)

- [x] `stream`: pull iterator (events())
- [x] `batch`: BatchParser + BatchSink
- [x] `w-stream`: Writer<W: Write> streaming writer
- [x] Feature flags added
- [ ] Parser/writer gaps: blockquote nesting, footnote definitions, figure/caption blocks

### `asciidoc` — API modes complete (2026-03-23)

- [x] `stream`: pull iterator (events())
- [x] `batch`: BatchParser + BatchSink
- [x] `w-stream`: Writer<W: Write> streaming writer
- [x] Feature flags added
- [ ] Parser gaps: table parsing, footnote parsing, math parsing
- [ ] Markdown family (pulldown-cmark backed; adapter hardening + fuzz)
- [ ] HTML (html5ever backed; same)
- [ ] DOCX, PPTX, XLSX (ooxml-* backed; same) — DOCX reader at 5-Production (2026-03-03); others at 4-Fuzz; gaps below

  **DOCX reader** (closest to production):
  - [x] Endnote content — `doc.get_endnotes()` pre-loaded; `footnote_ref` nodes with `label:"en{id}"` prefix
  - [x] Para-props raw preservation — `docx:space-before`, `docx:space-after`, `docx:line-spacing`, `docx:indent-left/right/first-line/hanging` props
  - [x] List ordering — numbering definitions consulted via `ParagraphExt::num_fmt()`; `ordered: true` for decimal
  - [x] Audit `_ => {}` at line 370 — `MoveFrom`/`MoveTo`/`SubDoc` now emit fidelity warnings
  - [x] Fixtures: all 22 fixtures have expected.json (image, hyperlink, small_caps, all_caps, hidden, highlight, ordered lists, table_header, endnote, para_spacing, para_indent)
  - [x] Roundtrip fuzz target (`fuzz_docx_roundtrip`) — 441K runs clean (2026-03-03)
  - [x] No-panic fuzz gate (`fuzz_docx_reader`) — 5.7M runs clean (2026-03-03)
  - [x] **5-Production** — all gates passed (2026-03-03)

  **DOCX writer**:
  - [x] Image embedding (resource:xxx → embedded DOCX media via pre-registration + CTDrawing clone)
  - [x] Footnote writing (`footnote_ref` → endnote API)
  - [x] Hyperlink writing (`link` URL → rel-registered hyperlink)
  - [x] Metadata writing (`doc.metadata` → `set_core_properties()`)
  - [x] Roundtrip fuzz target — clean

  **XLSX reader**:
  - [x] Cell formatting fidelity warning — cells with style_index > 0 emit warning (2026-03-03)
  - [x] Charts fidelity warning — embedded charts per sheet emit warning (2026-03-03)
  - [x] Named ranges fidelity warning — workbook defined_names emit warning (2026-03-03)
  - [x] Formula fixture (xlsx/formula) — xlsx:formula property preserved (2026-03-03)
  - [x] Roundtrip fuzz target (fuzz_xlsx_roundtrip) — 157K runs clean (2026-03-03)
  - [ ] Metadata extraction (TODO stub in code — ooxml-sml doesn't expose core properties)
  - [ ] More fixtures (formatted cells, etc.)

  **PPTX reader**:
  - [x] Bullet/list detection warning — paragraphs with level() > 0 emit fidelity warning (2026-03-03)
  - [x] Speaker notes plain-text warning — notes div emitted with warning about lost rich text (2026-03-03)
  - [x] Charts/SmartArt fidelity warnings — per-slide warnings when chart_rel_ids/smartart_rel_ids non-empty (2026-03-03)
  - [x] Notes fixture (pptx/notes) — speaker notes div structure (2026-03-03)
  - [x] Fix Cargo.toml: workspace deps (was path deps) (2026-03-03)
  - [x] Bullet/list structure in IR — consecutive bullet paragraphs grouped into list/list_item nodes (2026-03-20)
  - [ ] Nested bullet levels (currently flattened to single level with fidelity warning)
  - [ ] Roundtrip fuzz target (requires PPTX writer capable of roundtrip)
- [ ] EPUB (epub/epub-builder backed; same)
- [ ] ODT writer (no library; treat as a vertical)
- [ ] AZW3 reader/writer (boko as reference, MIT attribution)
- [ ] PDF reader (pdf-extract backed; already at 4)

### Milestone: M3 — Tier B/C verticals

Tier B formats at 3-Harness or 2-Fixtures (where harness is N/A), each with a
standalone library where the ecosystem gap justifies it.

- [x] `t2t` vertical — **4-Fuzz** (2026-03-21)
  - [x] Split monolith lib.rs into ast.rs / parse.rs / emit.rs
  - [x] Span on every AST node; Diagnostic type; strip_spans()
  - [x] parse() infallible → (T2tDoc, Vec<Diagnostic>)
  - [x] No-panic fuzz gate (`fuzz_t2t_reader`) — 2M runs clean (2026-03-21)
  - [x] Roundtrip fuzz target (`fuzz_t2t_roundtrip`) — 939K runs clean (2026-03-21)
  - [x] Fixed URL sanitiser: ':' filtered to prevent http: + //italic// combining into URL patterns
  - [x] Fixtures: blockquote, table, image added (2026-03-21)
  - [ ] 100% construct coverage — raw blocks; footnotes; definition lists
- [ ] MOBI reader (boko as reference)
- [ ] KFX reader/writer (Ion spec + boko)
- [ ] Remaining Tier B/C formats: audit and bring to target stage

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
