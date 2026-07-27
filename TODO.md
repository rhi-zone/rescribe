## Status Indicator
- Current: ◐ Fleshed Out — kept despite high commit count (112 commits, 207 Rust files)
- Needs hardening/verification work before upgrading to ● Potentially Mature
- Lots of code, but needs more verification to count as mature

# Rescribe Roadmap

Per-format status is tracked in `docs/format-audit.md` using the maturity pipeline
(0-Stub → 1-Partial → 2-Fixtures → 3-Harness → 4-Fuzz → 5-Production).
This file describes milestones, format tiers, and cross-cutting work.

---

## Open Threads

*Open threads from a previous session. Treat as starting context, not instructions — verify relevance before acting.*

- **`docbook-fmt` fixture suite closed to 88/94** (checked 2026-07-27, this session) —
  up from 30/94. Real reader/writer work, not just fixture-writing: added CALS table
  attributes (frame/colsep/rowsep/colspec/spanning), formal-table titles, list
  numeration/spacing, xml:lang/xlink attrs applied uniformly via
  `attach_generic_attrs`, `procedure`/`step`/`substeps` -> ordered list mapping,
  `screen`/`literallayout`/`synopsis`/`address` -> tagged `code_block`,
  `epigraph`/`attribution`, `bridgehead`, `footnoteref` -> `footnote_ref`,
  `mediaobject`/`textobject` -> image `alt` folding, and ~20 phrase-level semantic
  inlines (abbrev/acronym/trademark/keycap/guilabel/etc) verified individually
  against the DocBook 5.2 reference (tdg.docbook.org) and closed via the existing
  `generic_span` raw-preservation mechanism. Full commit list on `master`, newest
  first: `feat(docbook): fold mediaobject alt text into image; close composition
  fixtures`, `feat(docbook): map example/screen/synopsis/procedure/epigraph/
  bridgehead/address`, `fix(docbook): stop corrupting <title> round-trip in
  non-sectioning containers`, `test(docbook): close the Adversarial and
  Pathological COVERAGE dimensions`, `feat(docbook): map footnoteref; fixture the
  phrase-level semantic inlines`, `test(docbook): fixtures for xref/anchor/
  personname/filename/revhistory/pubdate`, `feat(docbook): model CALS table
  attributes, formal tables, cell spanning`, `feat(docbook): add xml:lang, link
  xlink attrs, list numeration/spacing`.

  **Real bugs found and fixed along the way** (discovered while verifying
  parse -> emit -> parse round-trips for new fixtures, not just one-way reader
  assertions — docbook has no dedicated writer fixture suite, so these were
  latent): (1) any `<title>` whose parent wasn't a genuine sectioning container
  always became a `HEADING`, which the writer always wraps in a fresh `<sectN>`
  on emit — so e.g. `<example><title>T</title>...` round-tripped as a spurious
  nested `<sect1>` inside the example, corrupting every non-sectioning titled
  container (example, figure, admonitions, qandaset, refentry, ...). Fixed via
  `heading_level_for_parent` + a new `CAPTION` node kind/write arm. (2) a
  `generic_span` landing directly in a raw-preserved block container's children
  (e.g. `<arg>` inside `<cmdsynopsis>`) silently lost its tag on write. (3)
  `<abstract>` (the one dedicated DIV mapping without `docbook:tag`) was dropped
  entirely by the writer's DIV arm. (4) `FOOTNOTE_DEF` embedded inline (e.g. in a
  table cell) had no `write_inline` arm and silently lost its `<footnote>`
  wrapper. All four fixed this session.

  **Left open, genuine design forks (not lookup-resolvable), 6 of 94 boxes**:
  `qandaset`/`qandaentry` (no Q&A-list IR shape attempted — still raw-preserves
  generically via `generic_div`, just unverified with a fixture); `equation`/
  `inlineequation` (MathML modeling choice — reuse `rescribe-math`'s
  `math_inline`/`math_display` with the MathML captured as raw content, or
  something else — genuinely undecided); `programlistingco`/`co`/"callout listing
  + callout list" (three boxes, all paired: `co` only has meaning alongside a
  `<calloutlist>` that references it back, so designing one without the other
  would be premature).

  **Found but NOT fixed this session** (real, disclosed, out of scope for the
  fixture-closing pass): a `DIV` containing a `HEADING` plus following block
  siblings (any section with more than just a title) does not reassemble into
  one shared `<sectN>` on write — `write_node`'s `HEADING` arm always wraps only
  the title itself in a fresh `<sectN>`, leaving the section's actual body
  content as siblings *outside* that new element on round-trip. Exposed by the
  `nested-section` fixture (whose *reader* output is correct — fixtures only
  test the reader per `fixtures/spec.md`, so the fixture was still added). Fixing
  this needs the writer's section-boundary detection redesigned (recognizing "a
  DIV whose first child is a HEADING" as one section unit to serialize together)
  — a real architecture decision, not a quick patch. Also found: `<figure>`'s
  `<caption>` child (mapped to a custom `figcaption` node kind, pre-existing,
  unrelated to this session's changes) has no writer arm and silently drops the
  `<caption>` wrapper on round-trip, leaving a bare `<para>` — pre-existing,
  not fixed, not gating any box closed this session.

- **`jats-fmt` fixture suite closed to 99/106** (checked 2026-07-27, this session) —
  up from 32/106. Built on top of the classifier-verification pass below (same
  session): `crates/formats/jats-fmt/` itself needed no changes (JATS's AST is
  generic XML), all work landed in the AST↔IR adapter layer
  (`crates/readers/rescribe-read-jats/src/lib.rs`,
  `crates/writers/rescribe-write-jats/src/lib.rs`) plus 73 new fixture pairs under
  `fixtures/jats/*/`. Added: table properties (id/lang, `ext-link-type`, spanning),
  the inline `code`/`monospace` distinction, `xref` variants, generic-fallback
  attribute preservation, nested-section depth tracking, `abstract` metadata
  capture, front-matter metadata fields, adversarial-dimension fixtures,
  back-matter structural elements, `underline-style`, and the composition/
  pathological dimensions. Commits, newest first: `1eb2ffc14d` (stop folding
  disp-formula label into math:source; close composition/pathological dims),
  `c133f49562` (adversarial + back-matter + underline-style), `3f4db4a90d`
  (front-matter metadata), `4b3b2c1604` (nested-section depth + abstract-drop
  fix), `65245a6a12` (inline code/monospace/xref variants), `7dfacccd47` (table
  properties + table-wrap double-wrap fix).

  **Real bugs found and fixed along the way** (via parse→emit→parse round-trip
  checks, same discovery pattern as docbook's session): (1) the `TABLE` write arm
  always synthesized its own `<table-wrap>`, double-wrapping tables that
  originated from a `<table-wrap>` — fixed via `jats:tag="table-wrap"` tagging
  plus a shared `table_element()` helper. (2) block-position `SPAN` (e.g.
  `<label>` inside `<fig>`) and the `figcaption` node kind had no `write_node`
  arm and silently dropped their tags — fixed with dedicated arms. (3)
  `<abstract>` was dropped entirely: its `DIV` mapping never set `jats:tag`, so
  it missed the front-matter capture path — fixed. (4) nested `<sec>` heading
  level was hardcoded to `2` regardless of depth (JATS reuses `<sec>` at every
  nesting level) — fixed by threading a real depth counter. (5) `math:source`
  for `<disp-formula>`/`<inline-formula>` absorbed the `<label>` text into the
  math content — fixed with a `split_label()` helper.

  **Left open, genuine design forks (not lookup-resolvable), 7 of 106 boxes**:
  MathML `<math>` as an alternative to `<tex-math>` inside
  `disp-formula`/`inline-formula` — the same math-modeling fork docbook's
  `equation`/`inlineequation` hit, genuinely undecided; citation/reference-list
  IR shape (`ref-list`/`ref`/`mixed-citation`/`element-citation`, 5 of the 7
  boxes including two dependents — no dedicated bibliography IR shape attempted,
  still raw-preserves generically, unverified with a fixture); `<alternatives>`
  (JATS's own Tag Library page states it "is neither inherently block nor
  inherently inline in nature... determined by context and usage" — JATS itself
  declines to commit, so no IR classification was guessed).

  **Found but NOT fixed this session** (real, disclosed, out of scope): a
  titleless `<sec>` loses its wrapper on write — mirror of docbook's disclosed
  section-writer gap (a DIV without a leading HEADING doesn't reassemble into
  one shared element on emit). `generic_div` wraps bare-PCDATA children in a
  synthetic `<p>` even for elements whose content model forbids it (e.g.
  `<verse-line>`) — pre-existing, not gating any box closed this session.
  `<journal-meta>` fields get spliced into a reconstructed `<article-meta>` on
  write, losing origin-wrapper distinction — pre-existing, the flat metadata
  namespace has no origin tracking.

- **`jats-fmt` `is_block_element` classifier schema-verified against JATS 1.3**
  (checked 2026-07-27, this session, commit `20c27d032e`) — following the same
  pattern as `docbook-fmt` (docbook.org, three misclassifications corrected) and
  `tei-fmt` (TEI P5 Guidelines, zero misclassifications, three missing elements
  added). Checked every element in
  `crates/readers/rescribe-read-jats/src/lib.rs`'s `is_block_element` against the
  JATS 1.3 (NISO Z39.96-2019) Tag Library at jats.nlm.nih.gov, fetching each
  element's actual page (expanded content model + "May be contained in" list)
  rather than relying on memory. **Found and fixed**: `related-article` was
  misclassified as block — its Tag Library page documents it as a phrase-level
  link element (like `xref`/`ext-link`) that can appear inside `<p>`, `<italic>`,
  `<sub>`, etc. — removed from the block list. **Added, previously missing**:
  `speech`, `speaker`, `supplementary-material`, `block-alternatives`, all
  confirmed block-shaped via their JATS content models. Plain `<alternatives>`
  was deliberately left unclassified (defaults to inline) since JATS's own docs
  decline to classify it either way (see the fixture-suite bullet above).
  **Verified correct, no change**: nine metadata-container entries
  (`contrib-group`/`aff`/`pub-date`/`permissions`/`history`/
  `custom-meta-group`/`custom-meta`/`product`/`sig`/`sig-block`) plus
  `statement`/`verse-group`/`table-wrap-group`/`tfoot`/`disp-formula-group`/
  `kwd-group`/`ack`/`app-group`/`app`/`notes` — full citation trail in the doc
  comment above `is_block_element`.
- **The docbook/jats/tei extraction-and-closing arc is essentially wound down**
  as of 2026-07-27. All three verticals have had their fixture suites deepened
  (tei 118/118, docbook 88/94, jats 99/106) and their `is_block_element`
  classifiers schema-verified against each format's authoritative reference,
  with real round-trip bugs found and fixed along the way in every case. The
  actual residue for a future session, accurately inventoried (not urgent):
  docbook's 6 open design-fork boxes (qandaset/qandaentry, equation/
  inlineequation MathML, programlistingco/co/calloutlist) plus its 2 disclosed
  writer bugs (section reassembly, figure caption drop); jats's 7 open
  design-fork boxes (MathML, citation/ref-list IR shape, alternatives) plus its
  3 disclosed writer gaps (titleless-sec reassembly, generic_div bare-PCDATA
  wrapping, journal-meta origin tracking); and DTD-aware entity resolution
  across all three (see below). The docbook/jats fuzz campaigns (previously
  only a ~60s validation run) were brought to parity with tei's multi-hour
  campaign in a later session (2026-07-27): `fuzz_docbook_fmt_reader`
  8,918,090 runs clean, `fuzz_docbook_fmt_roundtrip` 6,012,874 runs clean,
  `fuzz_jats_fmt_reader` 8,162,993 runs clean, `fuzz_jats_fmt_roundtrip`
  5,696,913 runs clean — no crashes, no panics, no artifacts, no bugs found.
  All four fuzz targets across all three verticals (tei/docbook/jats) are now
  at the same extended-campaign scale. None of the remaining residue items
  block calling any of the three verticals 3-Harness; they gate 5-Production
  only.
- **Oracle harness not yet run for `docbook-fmt`/`jats-fmt`; applicability confirmed.**
  `pandoc --list-input-formats` (checked 2026-07-27) includes both `docbook` and `jats`, so per
  TODO.md's Tier B oracle-harness guidance ("skip for formats Pandoc can't read") the harness step
  applies to both and is still open — `docs/format-audit.md` shows both at oracle-harness status
  "harness" (applicable, not yet done). `tei` is **not** in pandoc's input-formats list (only
  output); `docs/format-audit.md` now marks TEI's oracle-harness step N/A (2026-07-27), the same
  way `asciidoc`'s was.
- **`docbook-fmt`/`jats-fmt` fuzz targets initially only had a ~60s validation run each**
  (docbook: 1.69M reader / 573K roundtrip runs; jats: 1.61M / 553K; both clean, one fuzz-harness
  generator bug fixed, no library bugs) — not the multi-hour/multi-million-run campaign
  `commonmark-fmt` got before its 5-Production sign-off. `tei-fmt`'s fuzz targets got that longer
  campaign in an earlier session (2026-07-27): `fuzz_tei_fmt_reader` 7,518,438 runs clean,
  `fuzz_tei_fmt_roundtrip` 6,611,996 runs clean (15 min/target via `cargo fuzz run <target> --
  -max_total_time=900`), no crashes/panics/artifacts. **Closed (2026-07-27, later same day)**:
  docbook and jats got the same extended campaign — `fuzz_docbook_fmt_reader` 8,918,090 runs
  clean, `fuzz_docbook_fmt_roundtrip` 6,012,874 runs clean, `fuzz_jats_fmt_reader` 8,162,993
  runs clean, `fuzz_jats_fmt_roundtrip` 5,696,913 runs clean. No crashes, no panics, no
  artifacts, no roundtrip mismatches, no bugs found — all three XML verticals' fuzz targets
  are now at the same extended-campaign scale. The fixture-suite gaps above remain open but
  are independent of this closed item.
- **DTD-aware entity resolution — implemented (2026-07-28)** via a new standalone crate,
  `crates/formats/xml-entities` (no rescribe dependency, workspace member `xml-entities`).
  Scope: (1) a narrow DTD internal-subset `<!ENTITY ...>` declaration parser
  (`DtdEntities::parse_doctype`/`parse_subset`) — general *and* parameter entities,
  `SYSTEM`/`PUBLIC` external entities recorded (name + identifiers) but never fetched (no
  network/filesystem access anywhere in the crate), numeric char refs expanded at
  declaration time per the XML spec, internally-declared parameter entities expanded
  in-place (the "combine several `<!ENTITY %`-declared fragments" idiom), external
  parameter-entity references diagnosed rather than silently skipped. Deliberately **not**
  a DTD validator — `<!ELEMENT>`/`<!ATTLIST>`/`<!NOTATION>` are recognized only well enough
  to skip correctly. (2) `EntityResolver`, layering those document-declared entities over
  the WHATWG HTML5 standard table (via the `entities` crate, ~7M downloads/wk) as a
  fallback — HTML5's table absorbed nearly all of the ISO 8879/9573 sets (`ISOlat1`,
  `ISOnum`, `ISOpub`, `ISOtech`, `mmlalias`) DocBook/JATS/TEI lean on, which in practice is
  what resolves most entities from those DTDs anyway since the real-world idiom pulls them
  in via an *externally*-fetched parameter entity (e.g. DocBook's
  `%isolat1; PUBLIC "ISO 8879-1986//ENTITIES Added Latin 1//EN" "isolat1.ent"`) that this
  crate does not fetch. Resolution is recursive (an entity's value referencing another
  entity) with cycle/depth guards. Unknown/external names resolve to a non-error variant
  (`Resolution::Unknown`/`ExternalUnresolved`) so callers raw-preserve rather than drop
  them. 30 unit tests + 1 doctest, clippy clean, no-panic fuzz target
  `fuzz_xml_entities_reader` registered (compiles clean; not run as an extended campaign —
  `cargo-fuzz` isn't installed in this repo's dev shell).

  **Wired into all three format crates' `parse()`, `EventIter` (SAX), and
  `StreamingParser` (batch)** — all three independent reader API surfaces per
  CLAUDE.md's "each API mode is independently implemented" rule, not just the AST path.
  Named entities beyond the 5 XML-predefined ones now try `EntityResolver` (built from
  that document's own DOCTYPE, if any) before falling back to the pre-existing
  `Node::EntityRef` raw-preservation. Malformed DOCTYPE internal subsets surface as
  diagnostics (prefixed `"DOCTYPE internal subset: ..."`) instead of being silently
  discarded. 3 new fixtures per vertical (`dtd-entity-resolution`,
  `rare-named-entity-standard-table`, `adv-unresolvable-entity`) plus 3 new unit tests
  per format crate; COVERAGE.md updated for all three (docbook/jats/tei).

  **Known gap, disclosed rather than silently left**: the rescribe adapter layer
  (`rescribe-read-{docbook,jats,tei}`) is unaffected by this change other than seeing
  fewer `raw_inline`/`*:entity` nodes and more resolved `text` nodes — no adapter code
  changed, since resolution now happens entirely inside the `-fmt` crates before the
  adapter ever sees a `Node::EntityRef`. External (`SYSTEM`/`PUBLIC`) DTD entities and
  entities declared only in an external subset pulled in via a parameter-entity reference
  remain genuinely unresolvable without fetching that external file — this is a
  deliberate, disclosed scope boundary (no network/filesystem access from this crate),
  not a bug; such entities still raw-preserve losslessly exactly as before.

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
   Required for both reader and writer.
2. **Oracle harness** (where applicable — skip for formats Pandoc can't read) — run against
   Pandoc or another reference implementation. No numeric threshold; all differences must be
   understood and documented. The goal is zero unexplained differences.
3. **Fuzz clean** — both no-panic gate and roundtrip property, run until no failures.
   Required for both reader and writer.
4. **All API modes complete** — reader: ast + stream + batch; writer: w-build + w-stream.
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
DOCX, EPUB, AZW3, Org, RST, AsciiDoc, Djot, ODT, PPTX, XLSX, PDF

### Tier A (read-limited) — Production priority, last in queue

Formats where the **write direction is high quality** (IR → LaTeX/Typst produces correct,
well-structured output) but the **read direction is extraction-only**: the authoring
language is Turing-complete, so arbitrary user-defined macros/functions cannot be resolved
without full execution. Round-trip fidelity is architecturally impossible in the read
direction; the write direction is fine.

Read strategy: known constructs (standard packages/builtins) → IR; unknown constructs
→ `raw_inline`/`raw_block` with a fidelity warning. No round-trip fuzz target (the read
direction cannot guarantee it). Quality bar for reading is extraction fidelity for
real-world documents using common packages.

These are last in the Tier A queue because the reader surface area is enormous (just the
common LaTeX packages — amsmath, biblatex, hyperref, geometry, listings — is months of
work) and the reader quality ceiling is fundamentally lower than interchange formats.

LaTeX, Typst

### Tier B — Correctness, not urgent

Formats with real use cases but lower conversion frequency.
Target: **3-Harness** (or 2-Fixtures where harness is N/A), fuzz as bandwidth allows.

MediaWiki, DocBook, JATS, TEI, FB2, RTF, Man,
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
- Rescribe adapter does only AST↔IR translation (no format parsing/writing)

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

**CURRENT TOP PRIORITY: `commonmark-fmt` — see below.**

0. `commonmark-fmt` — write from scratch; tree-sitter-md is explicitly not for
   correctness-critical parsing (its README says so); pulldown-cmark is events-only
   with no proper AST; the Rust ecosystem has no quality CommonMark AST crate.
   This fills the most important ecosystem gap. See "commonmark-fmt vertical" below.
1. `rtf-fmt` — highest risk, most isolated, no viable crate exists ✓
2. `rst-fmt` — large parser, complex spec, `docutils` is the reference ✓
3. `asciidoc` — similar scope; `asciidoctor` as oracle ✓
4. `org-fmt` — reader at 4-Fuzz (2026-03-21); writer still at 2-Fixtures; coverage gaps remain ✓
5. `djot-fmt` — jotdown has confirmed bugs; djot spec is clean and small ✓
6. `odt` — no library; hand-rolled; ODF is a real interchange format
7. `epub` — library-backed (epub/epub-builder)
8. `azw3` — not yet implemented
9. LaTeX, Typst — read-limited; deferred until all other tiers complete; writer is
   high quality but reader quality ceiling is bounded by package recognition.
   See "Tier A (read-limited)" above.

### commonmark-fmt vertical (CURRENT)

**Why wrapping pulldown-cmark, not from scratch:**
pulldown-cmark has 77M+ downloads; it IS the Rust CommonMark ecosystem (used by
mdBook, rustdoc). It already exposes `into_offset_iter()` yielding `(Event, Range<usize>)`
pairs — spans on every event, explicitly designed for AST construction (see its README:
"quite straightforward to construct an AST"). The tree-sitter backend was solving a
problem pulldown already solved; we just weren't using the right API.

**Crate:** `crates/formats/commonmark-fmt/`
Depends on pulldown-cmark. No rescribe dependency. Exposes:
- `parse(&[u8]) -> (Ast, Vec<Diagnostic>)` — drives pulldown's offset iterator,
  assembles (Event, Range) pairs into a full tree with Span on every node
- `emit(ast: &Ast) -> Vec<u8>` — round-trip correct
- `events(&[u8]) -> impl Iterator<Item = Event>` — thin re-export of pulldown events
- Feature flags: ast, streaming, batch, writer-streaming, writer-builder (all default=true)

**Architecture:** `commonmark-fmt` wraps pulldown-cmark. The three reader APIs:
- `parse()` — `TreeBuilder` over pulldown's `into_offset_iter()`. Direct and fast.
- `events()` — thin wrapper over pulldown's iterator; translates `pulldown_cmark::Event`
  to `commonmark_fmt::Event<'_>` with `Cow::Borrowed` slices from the input. Standard
  `Iterator`. Max perf — pulldown IS a true pull parser.
- `StreamingParser<H>` — buffers all chunks, runs pulldown on `finish()`. **Known
  limitation: not true chunked streaming.** Documented in the crate. Superseding
  pulldown-cmark is a non-goal; see `docs/format-library-design.md`.

**Build order:**
1. [x] Complete `fixtures/commonmark/` — all 74 COVERAGE.md boxes checked (2026-03-25)
2. [x] `ast.rs` — Block/Inline enums with Span on every node (2026-03-25)
3. [x] `parse.rs` — TreeBuilder over pulldown offset iterator (2026-03-25)
4. [x] `emit.rs` — Ast → bytes, round-trip guarantee (2026-03-25)
5. [x] `events.rs` — `Event<'a>` with `Cow<'a, str>`; `EventIter` wraps pulldown iterator (2026-03-25)
6. [x] `batch.rs` — `StreamingParser<H>` buffering wrapper; `Handler` trait; limitation documented (2026-03-25)
7. [x] `writer.rs` — `Writer<W: Write>` streaming writer (2026-03-25)
8. [x] No-panic fuzz gate (`fuzz_commonmark_reader`) (2026-03-25)
9. [x] Round-trip fuzz (`fuzz_commonmark_roundtrip`) — compile-verified (2026-03-25)
10. [x] `rescribe-read-markdown` + `rescribe-read-commonmark`: tree-sitter backend dropped; both now use commonmark-fmt (2026-03-25)
11. [x] 5-Production sign-off — fuzz_commonmark_reader 342K runs clean; fuzz_commonmark_roundtrip 4+ hours / ~2M+ runs clean after 12 crash artifacts fixed (2026-03-25)

**GFM extensions** (after base complete):
Tables, strikethrough (`~~text~~`), task list items (`- [x]`), extended autolinks

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
- [x] `rst-fmt` vertical — **5-Production** (2026-03-29)
  - [x] No-panic fuzz gate (`fuzz_rst_reader`); roundtrip fuzz (`fuzz_rst_roundtrip`)
  - [x] Fixtures: 80 total; COVERAGE.md all boxes checked (2 N/A items: include directive, hard break)
  - [x] Oracle harness: 100% word coverage on rst-reader.rst (ref=618)
  - [x] Benchmarks: rst_parse_small 3.3µs, rst_parse_medium 30µs, rst_emit_medium 2.5µs
  - [x] All API modes: ast + stream + batch + w-build + w-stream
  - [x] Table parsing — grid and simple tables with header support (2026-03-29)
  - [x] Footnote parsing — numbered, auto-symbol, auto-numbered, multi-line continuation (2026-03-29)
- [x] `asciidoc` vertical — **5-Production** (2026-03-29)
  - [x] No-panic fuzz gate (`fuzz_asciidoc_reader`); roundtrip fuzz (`fuzz_asciidoc_roundtrip`)
  - [x] Fixtures: 84 total; COVERAGE.md all boxes checked
  - [x] Oracle harness: N/A (pandoc can't read asciidoc)
  - [x] Benchmarks: asciidoc_parse_small 6.6µs, asciidoc_parse_medium 48µs, asciidoc_emit_medium 1.9µs
  - [x] All API modes: ast + stream + batch + w-build + w-stream
  - [x] Table parsing — with header row detection (2026-03-29)
  - [x] Footnote parsing — anonymous + named + back-reference forms (2026-03-29)
  - [x] Math parsing — `stem:[...]` inline + `[stem]\n++++` block (2026-03-29)
- [x] `textile-fmt` vertical — **5-Production** (2026-03-29)
  - [x] Split monolith lib.rs into ast.rs / parse.rs / emit.rs
  - [x] Span on every AST node; Diagnostic type; strip_spans()
  - [x] parse() infallible → (TextileDoc, Vec<Diagnostic>)
  - [x] build() renamed to emit() returning String
  - [x] No-panic fuzz gate (`fuzz_textile_reader`) — 1.6M runs clean (2026-03-21)
  - [x] Roundtrip fuzz target (`fuzz_textile_roundtrip`) — 923K runs clean (2026-03-21)
  - [x] Fixed infinite loop bug: list parser on `** ` (level-2 marker with no level-1 items)
  - [x] Fixtures: table, image, superscript, subscript added (2026-03-21); COVERAGE.md all checked
  - [x] Footnotes — FootnoteDef block + FootnoteRef inline (2026-03-28)
  - [x] Definition lists — DefinitionList block with term/desc pairs (2026-03-28)
  - [x] Oracle harness (`pandoc_textile_corpus` #[ignore]) — pandoc_harness.rs (2026-03-29)
  - [x] Benchmarks: textile_parse_small ~1.9µs, textile_parse_medium ~47µs (2026-03-29)
  - [x] All API modes: ast + stream + batch + w-build + w-stream (2026-03-29)
- [x] `org-fmt` vertical — **5-Production** (2026-03-29)
  - [x] No-panic fuzz gate (`fuzz_org_reader`) — 1.25M runs clean; roundtrip fuzz clean
  - [x] Fixtures: 88 total; COVERAGE.md all boxes checked
  - [x] Oracle harness: 100% word coverage on writer.org (ref=919)
  - [x] Benchmarks: org_parse_small 3.4µs, org_parse_medium 53µs, org_emit_medium 2.9µs
  - [x] All API modes: ast + stream + batch + w-build + w-stream
  - [x] Nested blockquote parsing (depth counter) — (2026-03-29)
  - [x] Footnote definitions — `[fn:label]` block-level (2026-03-29)
  - [x] Figure/caption blocks — `#+CAPTION:`/`#+NAME:` wrapping image/table/code (2026-03-29)
- [ ] `muse-fmt` vertical — **4-Fuzz** → needs re-fuzz after construct expansion
  - [x] Split monolith lib.rs into ast.rs / parse.rs / emit.rs
  - [x] Span on every AST node; Diagnostic type; strip_spans()
  - [x] parse() infallible → (MuseDoc, Vec<Diagnostic>)
  - [x] No-panic fuzz gate (`fuzz_muse_reader`) — 1.65M runs clean (2026-03-21); needs re-run after expansion
  - [x] Roundtrip fuzz target (`fuzz_muse_roundtrip`) — 1.15M runs clean (2026-03-21); needs re-run
  - [x] Constructs: tables, verse, footnotes, centered/right/literal/src blocks, underline, strikethrough, sup, sub, image, anchor, line-break (2026-03-29)
  - [x] Fixtures: COVERAGE.md fully checked (2026-03-29); composition + adversarial + pathological added
  - [x] Oracle harness: `pandoc_muse_corpus` #[ignore] + `parse_sample_no_panic` CI test (2026-03-29)
  - [x] Benchmarks: muse_parse_small, muse_parse_medium, muse_emit_medium (2026-03-29)
  - [x] All API modes: ast + stream + batch + w-build + w-stream (2026-03-29)
  - [ ] Re-run fuzz clean after construct expansion (pre-req for 5-Production)
- [ ] `man-fmt` vertical — **4-Fuzz** → needs re-fuzz after expansion (2026-03-29)
  - [x] Split monolith lib.rs into ast.rs / parse.rs / emit.rs
  - [x] Span on every AST node; Diagnostic type; strip_spans()
  - [x] parse() infallible → (ManDoc, Vec<Diagnostic>)
  - [x] No-panic fuzz gate (`fuzz_man_reader`) — 2M runs clean; needs re-run
  - [x] Roundtrip fuzz target (`fuzz_man_roundtrip`) — 855K runs clean; needs re-run
  - [x] New constructs: IndentedParagraph, ExampleBlock, Comment blocks; Code/Superscript/Subscript inlines; special char escapes; .TH full metadata (2026-03-29)
  - [x] All API modes: ast + stream + batch + w-build + w-stream (2026-03-29)
  - [x] Oracle harness + benchmarks (2026-03-29)
  - [x] Fixtures: COVERAGE.md mostly checked (few N/A items: .SY/.RS synopsis, \fP, \f[name]) (2026-03-29)
  - [ ] Re-run fuzz clean after construct expansion
- [x] `djot-fmt` vertical — **5-Production** (2026-03-29; writer signed off)
  - [x] All API modes: ast + stream + batch + w-build + w-stream
  - [x] Oracle harness: 100% word coverage on djot-reader.djot (ref=931)
  - [x] Fixtures: 79 total; COVERAGE.md all boxes checked
  - [x] Benchmarks: djot_parse_small 7.8µs, djot_parse_medium 49µs, djot_emit_medium 9.8µs
  - [x] Fuzz reader: fuzz_djot_fmt_reader + fuzz_djot_fmt_roundtrip — 21M runs clean
  - [x] Fuzz writer: fuzz_djot_roundtrip — 1M runs clean
  - [x] Writer: no construct gaps vs reader; all Block+Inline variants handled in emit.rs + writer.rs

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
- [x] Fix events() — now a true pull iterator (2026-03-24)
- [x] StreamingParser<H: Handler> + Handler trait — 2026-03-25
- [x] events() frame-stack fix — O(nesting depth), not O(block subtree) (2026-03-28)
- [x] parse() direct recursive descent — independent of events() (2026-03-28)
- [x] StreamingParser<H> Tier 2 — O(largest block) streaming (2026-03-28)
- [x] `Cow::Borrowed` zero-copy text for headings and paragraphs (2026-03-28)
  - `Frame::InlineText { span, content }` carries absolute span + owned fallback
  - `ParseContext::line_offset_at()` provides line→byte mapping (0 for SubParser)
  - `push_heading_frames` / `push_paragraph_frames` pass real base_offset to parse_inlines
  - EventIter::next() checks `&input[span] == content` before borrowing; falls back to Owned
  - Smart punctuation (e.g. `--` → `–`) correctly returns Cow::Owned (content ≠ input slice)
  - SubParser events always Cow::Owned (no input reference available)

### `rtf-fmt` — API modes (2026-03-28)

- [x] `ast`: `parse(input: &[u8]) -> (RtfDoc, Vec<Diagnostic>)` — Span on every node
- [x] `ast`: `emit(ast: &RtfDoc) -> Vec<u8>` — builder writer
- [x] `stream` (token level): `token_events(input: &[u8]) -> TokenEventIter` — raw RTF tokens
- [x] `stream` (semantic): `events(input: &[u8]) -> SemanticEventIter` — document-semantic events;
  internally calls `parse()` first (RTF group/property inheritance requires full context);
  walks parsed RtfDoc with frame-stack; documented limitation
- [x] `batch`: `StreamingParser<H: Handler>` + `Handler` trait (2026-03-28)
  RTF is O(full input) — structural constraint (font/color tables must precede body);
  documented as inherent format limitation, not an implementation shortcut.
- [x] `w-build`: `emit()` builder writer
- [x] `w-stream`: Writer<W: Write> streaming writer — exists as writer::Writer<W> (token-level; 2026-03-28)

### DEBT: Adapter crates containing format parsing logic — identified 2026-04-10

The rule: adapter production code must not contain format parsing/writing.
Large line counts from AST↔IR translation are acceptable (DOCX, PPTX are genuinely complex).
The violation is format-parsing deps (quick-xml, zip, etc.) called from production functions.

- **`rescribe-read-docx`**: CLEAN — `parse_numbering_order()` moved to `ooxml-wml` (fixed 2026-04-10).
- **`rescribe-read-odt`**: CLEAN — rewritten to use `odf_fmt::parse()` (fixed 2026-04-10).
- **`rescribe-write-odt`**: CLEAN — rewritten to use `odf_fmt::emit()` (fixed 2026-04-10).
- **`rescribe-read-pptx`**: `zip` in `[dependencies]` but only used by `gen_fixtures`
  binary and `#[cfg(test)]`. Production parsing path is clean. Acceptable.
- **`rescribe-read-fb2`**: CLEAN — uses `fb2-fmt` (fixed 2026-04-10).
- **`rescribe-write-fb2`**: CLEAN — uses `fb2-fmt` (fixed 2026-04-10).
- **`rescribe-read-docbook`**: CLEAN — uses `docbook-fmt` (fixed 2026-07-26).
- **`rescribe-write-docbook`**: CLEAN — uses `docbook-fmt` (fixed 2026-07-26).
- **`rescribe-read-jats`**: CLEAN — uses `jats-fmt` (fixed 2026-07-26).
- **`rescribe-write-jats`**: CLEAN — uses `jats-fmt` (fixed 2026-07-26).
- **`rescribe-read-tei`**: CLEAN — uses `tei-fmt` (fixed 2026-07-26).
- **`rescribe-write-tei`**: CLEAN — uses `tei-fmt` (fixed 2026-07-26).

Fix each when doing that format's vertical. Do NOT fix all at once (horizontal sweep).

### `docbook-fmt` crate created (2026-07-26)

Standalone DocBook/generic-XML AST + parser + emitter (`crates/formats/docbook-fmt`),
wrapping `quick-xml` — no rescribe dependency. `rescribe-read-docbook` and
`rescribe-write-docbook` rewired to thin AST↔IR translators over `docbook_fmt::Node`
(no `quick-xml` left in either adapter's production code).

- [x] AST: `DocBookDoc { xml_decl, nodes: Vec<Node> }`; `Node::{Element, Text, Cdata,
  Comment, ProcessingInstruction, Doctype, EntityRef}`, `Span`, `Diagnostic`, `strip_spans()`
- [x] `parse(&[u8]) -> (DocBookDoc, Vec<Diagnostic>)` — direct recursive-descent build
  over `quick_xml::Reader`, never panics (malformed input recovered best-effort + diagnostics)
- [x] `events(&[u8]) -> EventIter` — **true SAX streaming**, not derived from `parse()`.
  Unlike `html-fmt` (which must build the full tree because HTML5 tree construction can
  rearrange nodes), XML is well-nested by construction, so `EventIter` wraps
  `quick_xml::Reader` directly and is genuinely O(largest token) memory.
- [x] `StreamingParser<H: Handler>` (`batch.rs`) — genuinely incremental: dispatches every
  event to the handler as soon as it's provably complete and drops the consumed prefix
  from its buffer, so memory is bounded by the largest in-progress token, not the whole
  document. The one non-obvious case: quick-xml can't distinguish "text run ended because
  `<` was found" from "text run ended because the buffer ran out" — a `Text` event is only
  dispatched once it's terminated by an actual `<` boundary or `finish()` confirms EOF.
  Verified with chunk-boundary-splitting tests (text split mid-word, tag split mid-name).
- [x] Entity handling: quick-xml 0.39 tokenizes `&name;`/`&#N;` as its own `GeneralRef`
  event rather than folding it into `Text`. The 5 predefined XML entities and numeric
  character refs are resolved and merged into the surrounding text; any other named
  (DTD-defined) entity is preserved verbatim as `Node::EntityRef` / IR `raw_inline` with
  `docbook:entity` — never silently dropped, per CLAUDE.md's raw-preservation rule.
- [x] `emit(&DocBookDoc) -> Vec<u8>` builder writer + `Writer<W: Write>` streaming writer,
  both via `quick_xml::Writer` for correct escaping
- [x] Full construct parity with the pre-split adapter logic preserved (all node kinds the
  old hand-rolled reader/writer handled still map the same way); `xlink:href` link
  attribute matching now actually works (previously dead code — the old reader stripped
  namespace prefixes before matching the literal string `"xlink:href"`, so it could never
  match; `docbook-fmt` keeps the raw prefixed attribute name)
- [x] `cargo clippy --all-targets --all-features -p docbook-fmt -p rescribe-read-docbook
  -p rescribe-write-docbook -- -D warnings` and full test suite (incl. fixture suite) clean
- [x] Fuzz targets added (2026-07-26): `fuzz_docbook_fmt_reader` (no-panic gate on
  `parse()`/`events()`, 1.69M runs clean in initial 60s validation) and
  `fuzz_docbook_fmt_roundtrip` (arbitrary `DocBookDoc` → `emit()` → `parse()` →
  `strip_spans()` equality, per CLAUDE.md's arbitrary-AST-first direction; 573K runs
  clean). Only a fuzz-harness bug found (duplicate attribute names on one element —
  invalid XML, fixed by suffixing generated names with their index), no library bugs.
  Initial validation only, not an exhaustive campaign — see `docs/format-audit.md`.
  **Superseded (2026-07-27)**: extended campaign run, `-max_total_time=900` (15 min)
  per target via `cargo fuzz run <target> -- -max_total_time=900` in the
  `nix develop .#fuzz` shell — `fuzz_docbook_fmt_reader` 8,918,090 runs clean,
  `fuzz_docbook_fmt_roundtrip` 6,012,874 runs clean. No crashes, no panics, no
  artifacts written, no roundtrip mismatches. No bugs found this pass — now at
  parity with `tei-fmt`'s campaign scale.
- [x] **Bug found and fixed (2026-07-27)**: two silent-drop bugs closed,
  mirroring the tei fix (same audit, applied to this vertical). (1) The
  reader's final `_ => None` catch-all arm silently unwrapped *any*
  unrecognized element into its parent, discarding the fact that the tag
  ever existed with no warning. `rescribe-read-docbook` gained
  `is_block_element(tag: &str) -> bool` (a DocBook block-level vocabulary
  allow-list, mirroring `rescribe-read-tei`/`rescribe-read-html`) plus
  `generic_div`/`generic_span` helpers; the catch-all now raw-preserves an
  unrecognized element as a `docbook:tag`-tagged `div` (block-shaped) or
  `span` (inline-shaped) instead of dropping the tag. New fixtures
  `adv-unknown-block-element` (`<sidebar>`) and `adv-unknown-inline-element`
  (`<quote>` nested in running text) regression-test both branches.
  (2) `<info>`/`<articleinfo>`/`<bookinfo>` front-matter beyond `title`
  (author, authorgroup, date, copyright, legalnotice, pubdate, releaseinfo,
  revhistory, revision, or any other unmodeled field) was silently dropped —
  `extract_metadata` only ever extracted `title`. `docbook-fmt` gained a
  `Node::Raw { content, span }` AST variant plus
  `emit_fragment(nodes: &[Node]) -> Vec<u8>` (mirroring `tei-fmt`/`html-fmt`).
  `convert_children` now threads an `in_header: bool` through its recursion
  (true once inside `<info>`/`<articleinfo>`/`<bookinfo>` or any descendant)
  and, for any child not in the new `is_modeled_header_field` allow-list
  (just `title` — the only field with dedicated semantic extraction before
  this fix, per the current-code check this pass started from), captures
  the whole subtree's original XML via `docbook_fmt::emit_fragment` and
  stores it as `{tag}_raw` metadata (e.g. `author_raw`) alongside a
  flattened `{tag}` text convenience copy — generalized directly to the
  `{tag}_raw` naming from the start (not the two-hardcoded-names
  intermediate step tei's own fix went through first, since docbook had no
  prior per-field modeling to preserve compatibility with).
  `extract_metadata` matches both `span` and `div` node kinds and skips
  recursing into an already-raw-captured subtree's children. A repeatable
  field (e.g. more than one `<author>`) joins its flattened text with `"; "`
  and concatenates its raw XML (valid, since concatenated sibling XML
  elements are themselves valid XML content) rather than a later occurrence
  silently overwriting an earlier one. The fidelity warning path only fires
  if raw capture genuinely fails (non-UTF8 content — not expected for XML
  that parsed at all). `rescribe-write-docbook` now emits an `<info>`
  wrapper (title plus any spliced-back `*_raw` fields, sorted by tag for
  deterministic output) instead of writing a bare `<title>` only. New
  fixture `header-author` (`<info><author>` with nested `<personname>`)
  regression-tests the general fallback. `cargo clippy --all-targets
  --all-features -p docbook-fmt -p rescribe-read-docbook
  -p rescribe-write-docbook -- -D warnings` and full test/fixture suite
  clean.
- [ ] DTD-aware entity resolution and closing the remaining
  `fixtures/docbook/COVERAGE.md` gaps are follow-up work — out of scope for this pass per
  CLAUDE.md (Tier B target is 3-Harness, not 5-Production)

### `jats-fmt` crate created (2026-07-26)

Standalone JATS/generic-XML AST + parser + emitter (`crates/formats/jats-fmt`),
wrapping `quick-xml` — no rescribe dependency. Mirrors `docbook-fmt`'s architecture
exactly since JATS, like DocBook, is well-nested XML with no format-specific AST needs
(element semantics live entirely in the rescribe adapter, not the crate). `rescribe-read-jats`
and `rescribe-write-jats` rewired to thin AST↔IR translators over `jats_fmt::Node`
(no `quick-xml` left in either adapter's production code).

- [x] AST: `JatsDoc { xml_decl, nodes: Vec<Node> }`; `Node::{Element, Text, Cdata,
  Comment, ProcessingInstruction, Doctype, EntityRef}`, `Span`, `Diagnostic`, `strip_spans()`
- [x] `parse(&[u8]) -> (JatsDoc, Vec<Diagnostic>)` — direct recursive-descent build
  over `quick_xml::Reader`, never panics (malformed input recovered best-effort + diagnostics)
- [x] `events(&[u8]) -> EventIter` — true SAX streaming, not derived from `parse()`
  (XML is well-nested, so no tree needs to be built first, unlike `html-fmt`'s HTML5 case)
- [x] `StreamingParser<H: Handler>` (`batch.rs`) — genuinely incremental: dispatches every
  event to the handler as soon as it's provably complete and drops the consumed prefix
  from its buffer, memory bounded by the largest in-progress token. Verified with
  chunk-boundary-splitting tests (text split mid-word, tag split mid-name).
- [x] Entity handling: the 5 predefined XML entities and numeric character refs are
  resolved and merged into surrounding text; any other named (DTD-defined) entity is
  preserved verbatim as `Node::EntityRef` / IR `raw_inline` with `jats:entity` — never
  silently dropped. The pre-split reader had **no** entity handling at all, so this is
  a net fidelity improvement, not just parity.
- [x] `emit(&JatsDoc) -> Vec<u8>` builder writer + `Writer<W: Write>` streaming writer,
  both via `quick_xml::Writer` for correct escaping
- [x] Full construct parity with the pre-split adapter logic preserved (all node kinds the
  old hand-rolled reader/writer handled still map the same way); one incidental fidelity
  fix — `<xref ref-type="…">` now preserves `jats:ref-type` for both the self-closing
  and full-element (`<xref ...>text</xref>`) shapes, where the old reader only attached
  it for the self-closing case
- [x] `cargo clippy --all-targets --all-features -p jats-fmt -p rescribe-read-jats
  -p rescribe-write-jats -- -D warnings` and full test suite (incl. fixture suite) clean
- [x] Fuzz targets added (2026-07-26): `fuzz_jats_fmt_reader` (no-panic gate on
  `parse()`/`events()`, 1.61M runs clean in initial 60s validation) and
  `fuzz_jats_fmt_roundtrip` (arbitrary `JatsDoc` → `emit()` → `parse()` →
  `strip_spans()` equality; 553K runs clean). No library bugs found. Initial
  validation only, not an exhaustive campaign — see `docs/format-audit.md`.
  **Superseded (2026-07-27)**: extended campaign run, `-max_total_time=900` (15 min)
  per target via `cargo fuzz run <target> -- -max_total_time=900` in the
  `nix develop .#fuzz` shell — `fuzz_jats_fmt_reader` 8,162,993 runs clean,
  `fuzz_jats_fmt_roundtrip` 5,696,913 runs clean. No crashes, no panics, no
  artifacts written, no roundtrip mismatches. No bugs found this pass — now at
  parity with `tei-fmt`'s campaign scale.
- [x] **Bug found and fixed (2026-07-27)**: two silent-drop bugs closed,
  mirroring the docbook/tei fix (same audit, applied to this vertical; docbook's
  final generalized form used directly as the template, not the two-hardcoded-
  names intermediate step tei went through first). (1) The reader's final
  `_ => None` catch-all arm silently unwrapped *any* unrecognized element into
  its parent, discarding the fact that the tag ever existed with no warning.
  `rescribe-read-jats` gained `is_block_element(tag: &str) -> bool` (a JATS
  block-level vocabulary allow-list, mirroring `rescribe-read-docbook`) plus
  `generic_div`/`generic_span` helpers; the catch-all now raw-preserves an
  unrecognized element as a `jats:tag`-tagged `div` (block-shaped) or `span`
  (inline-shaped) instead of dropping the tag. New fixtures
  `adv-unknown-block-element` (`<statement>`) and `adv-unknown-inline-element`
  (`<styled-content>` nested in running text) regression-test both branches.
  (2) `<article-meta>`/`<journal-meta>` front-matter beyond `title`/
  `article-title` (contrib-group, pub-date, volume, issue, fpage, lpage,
  permissions, history, or any other unmodeled field) was silently dropped —
  `extract_metadata` only ever extracted `title`. `jats-fmt` gained a
  `Node::Raw { content, span }` AST variant plus
  `emit_fragment(nodes: &[Node]) -> Vec<u8>` (mirroring `docbook-fmt`/
  `tei-fmt`). `convert_children` now threads an `in_header: bool` through its
  recursion (true once inside `<article-meta>`/`<journal-meta>` or any
  descendant) and, for any child not in the new `is_modeled_header_field`
  allow-list (just `title`/`article-title` — the only fields with dedicated
  semantic extraction before this fix), captures the whole subtree's original
  XML via `jats_fmt::emit_fragment` and stores it as `{tag}_raw` metadata
  (e.g. `contrib-group_raw`) alongside a flattened `{tag}` text convenience
  copy. `<title-group>` gained an explicit pass-through arm (it wraps
  `<article-title>`/journal `<title>` under both `<article-meta>` and
  `<journal-meta>`) so the already-modeled title reaches `extract_metadata`
  as a direct sibling instead of being buried inside a `jats:raw` blob
  `extract_metadata` never recurses into. `extract_metadata` matches both
  `span` and `div` node kinds and skips recursing into an already-raw-captured
  subtree's children. A repeatable field (e.g. more than one
  `<contrib-group>`) joins its flattened text with `"; "` and concatenates its
  raw XML rather than a later occurrence silently overwriting an earlier one.
  The fidelity warning path only fires if raw capture genuinely fails
  (non-UTF8 content — not expected for XML that parsed at all).
  `rescribe-write-jats` now emits an `<article-meta>` wrapper (title-group
  plus any spliced-back `*_raw` fields, sorted by tag for deterministic
  output) instead of writing a bare `<title-group>` only. New fixture
  `header-contrib-group` (`<contrib-group>` with nested `<contrib>`/`<name>`)
  regression-tests the general fallback. `cargo clippy --all-targets
  --all-features -p jats-fmt -p rescribe-read-jats -p rescribe-write-jats --
  -D warnings` and full test/fixture suite clean.
- [ ] DTD-aware entity resolution and closing the remaining
  `fixtures/jats/COVERAGE.md` gaps are follow-up work — out of scope for this pass per
  CLAUDE.md (Tier B target is 3-Harness, not 5-Production)

### `tei-fmt` crate created (2026-07-26)

Standalone TEI/generic-XML AST + parser + emitter (`crates/formats/tei-fmt`),
wrapping `quick-xml` — no rescribe dependency. Mirrors `docbook-fmt`/`jats-fmt`'s
architecture exactly since TEI, like DocBook and JATS, is well-nested XML with no
format-specific AST needs (element semantics live entirely in the rescribe adapter, not
the crate). `rescribe-read-tei` and `rescribe-write-tei` rewired to thin AST↔IR
translators over `tei_fmt::Node` (no `quick-xml` left in either adapter's production
code).

- [x] AST: `TeiDoc { xml_decl, nodes: Vec<Node> }`; `Node::{Element, Text, Cdata,
  Comment, ProcessingInstruction, Doctype, EntityRef}`, `Span`, `Diagnostic`, `strip_spans()`
- [x] `parse(&[u8]) -> (TeiDoc, Vec<Diagnostic>)` — direct recursive-descent build
  over `quick_xml::Reader`, never panics (malformed input recovered best-effort + diagnostics)
- [x] `events(&[u8]) -> EventIter` — true SAX streaming, not derived from `parse()`
  (XML is well-nested, so no tree needs to be built first, unlike `html-fmt`'s HTML5 case)
- [x] `StreamingParser<H: Handler>` (`batch.rs`) — genuinely incremental: dispatches every
  event to the handler as soon as it's provably complete and drops the consumed prefix
  from its buffer, memory bounded by the largest in-progress token. Verified with
  chunk-boundary-splitting tests (text split mid-word, tag split mid-name).
- [x] Attribute keys are kept exactly as written (including namespace prefix, e.g.
  `xml:id`, `xml:lang`) rather than local-name-stripped — TEI leans heavily on
  `xml:id`/`xml:lang`, and adapter-layer matching against the literal prefixed name only
  works if the prefix survives parsing.
- [x] Entity handling: the 5 predefined XML entities and numeric character refs are
  resolved and merged into surrounding text; any other named (DTD-defined) entity is
  preserved verbatim as `Node::EntityRef` / IR `raw_inline` with `tei:entity` — never
  silently dropped. The pre-split reader had **no** entity handling at all, so this is
  a net fidelity improvement, not just parity.
- [x] `emit(&TeiDoc) -> Vec<u8>` builder writer + `Writer<W: Write>` streaming writer,
  both via `quick_xml::Writer` for correct escaping
- [x] Full construct parity with the pre-split adapter logic preserved (all node kinds the
  old hand-rolled reader/writer handled still map the same way), plus one real fidelity
  bug fixed: the old reader captured `xml:id` and `n` attributes into a `FrameAttrs`
  struct on every element but never read either field back out when building IR nodes —
  both attributes were parsed and then silently discarded on every element that carried
  them (dead-code capture bug, same family as docbook's `xlink:href` issue). `xml:id`
  now round-trips as the standard `id` property; `n` round-trips as `tei:n`. Comments/PIs
  inside content flow (previously bare-dropped with no signal at all) now emit a
  `Minor` fidelity warning instead of vanishing silently.
- [x] `cargo clippy --all-targets --all-features -p tei-fmt -p rescribe-read-tei
  -p rescribe-write-tei -- -D warnings` and full test suite (incl. fixture suite) clean
- [x] Fuzz targets added (2026-07-26): `fuzz_tei_fmt_reader` (no-panic gate on
  `parse()`/`events()`, 1.59M runs clean in initial 60s validation) and
  `fuzz_tei_fmt_roundtrip` (arbitrary `TeiDoc` → `emit()` → `parse()` →
  `strip_spans()` equality; 527K runs clean). No library bugs found. Initial
  validation only, not an exhaustive campaign — see `docs/format-audit.md`.
  **Superseded (2026-07-27)**: extended campaign run, `-max_total_time=900`
  (15 min) per target via `cargo fuzz run <target> -- -max_total_time=900`
  in the `nix develop .#fuzz` shell — `fuzz_tei_fmt_reader` 7,518,438 runs
  clean, `fuzz_tei_fmt_roundtrip` 6,611,996 runs clean. No crashes, no
  panics, no artifacts written, no roundtrip mismatches. No bugs found this
  pass (the one fuzz-harness generator bug from the initial 2026-07-26 run
  — duplicate attribute names — was already fixed before this campaign).
- [ ] DTD-aware entity resolution is follow-up work — out of scope for this pass
  per CLAUDE.md (Tier B target is 3-Harness, not 5-Production)

### `fixtures/tei/COVERAGE.md` closed to 118/118 (2026-07-27)

Fixture suite completeness (vertical checklist step 1) reached: every item in
`fixtures/tei/COVERAGE.md` now has a passing fixture (85 new `fixtures/tei/*`
directories added across block, inline, teiHeader-metadata, property,
integration/e2e, adversarial, and pathological categories). This closing pass
required real reader/writer changes, not just fixture-writing:

- [x] `rescribe-read-tei`/`rescribe-write-tei`: ~35 new element mappings (`sp`,
  `speaker`, `stage`, `epigraph`, `argument`, `byline`, `dateline`/`salute`/`signed`,
  `trailer`, `castList`/`castItem`, `ab`, `gap`/`space`, `div5`/`div6`, list
  `type` variants, `<label>` items) plus a generic `span`-tagged (`tei:tag=`)
  raw-preservation path for editorial/named-entity apparatus (`choice`,
  `abbr`/`expan`, `orig`/`reg`, `sic`/`corr`, `add`/`del`/`supplied`/`unclear`,
  `persName`/`placeName`/`orgName`/`name`, `date`/`title`/`num`/`measure`,
  `anchor`/`milestone`/`seg`/`w`/`pc`, `foreign`, `bibl`) — this is the same
  `span` node kind already used for exactly this purpose.
- [x] `xml:lang`, `corresp`, `sameAs` added to `attach_generic_attrs` (alongside
  the existing `xml:id`/`n`); `style:align` now derived from alignment-only
  `rend` values (`center`/`right`/`left`/`justify`) on `p`/`div` rather than
  overloading the `<hi>`-only `tei:rend` fallback.
  `<formula type="inline">` now maps to `math_inline` instead of always
  `math_display`; bare `<code>` now maps to inline `code` (previously only
  `<eg>` was reachable, and both aliased to `code_block`).
- [x] teiHeader metadata extraction substantially deepened: `author`/`editor`
  (repeatable, `"; "`-joined), `publisher`/`idno`, `profileDesc/langUsage/language`
  (`ident` → `language`), `abstract`, `textClass/keywords`, `revisionDesc/change`
  (repeatable, timestamped) all now populate `Document::metadata` and round-trip
  through the writer (which previously only ever wrote/read `title`).
  `encodingDesc`/`msDesc` are flattened to plain-text metadata with an explicit
  `Minor` fidelity warning (structure genuinely not modeled — a tracked gap, not
  a silent drop). **Superseded**: see the `2026-07-27` entries below —
  `encodingDesc`/`msDesc`, and every other unmodeled teiHeader field, are now
  raw-preserved byte-for-byte instead of flattened-with-warning.
- [x] **Bug found and fixed**: the reader's final `_ => None` fallback arm
  silently unwrapped *any* unrecognized element into its parent — dropping the
  fact that e.g. `<foo>` ever existed, not just layout. Changed to raw-preserve
  as a generic tagged `span` (`adv-unknown-element` fixture regression-tests
  this). Same fix category added a catch-all fidelity warning for teiHeader
  fields with no known metadata key (previously silently scanned-and-discarded
  with zero signal).
- [x] `cargo clippy --all-targets --all-features -p tei-fmt -p rescribe-read-tei
  -p rescribe-write-tei -- -D warnings` and full test/fixture suite clean
  (all 111 `fixtures/tei/*` fixtures + all existing unit tests pass)
- [x] **Fixed (2026-07-27)**: teiHeader sub-structure (`msDesc`, `encodingDesc`)
  is now raw-preserved byte-for-byte, not just flattened to text. `tei-fmt`
  gained a `Node::Raw { content, span }` AST variant (mirroring `html-fmt`'s)
  plus `emit_fragment(nodes: &[Node]) -> Vec<u8>` (mirroring
  `html_fmt::emit_fragment`, used there to raw-capture inline MathML).
  `rescribe-read-tei` captures the `<msDesc>`/`<encodingDesc>` subtree's
  original XML via `emit_fragment` at the point it still holds the raw
  `tei_fmt::Node` (before IR conversion) and stores it as `ms_desc_raw`/
  `encoding_desc_raw` string metadata, alongside the existing flattened
  `ms_desc`/`encoding_desc` text kept for convenience. `rescribe-write-tei`
  prefers the raw metadata when present, splicing it back in via a
  `tei_fmt::Node::Raw` node; the fidelity warning now only fires if raw
  capture genuinely fails (should not happen for any XML that parsed
  successfully). `fixtures/tei/header-ms-desc` and `header-encoding-desc`
  updated to assert the new `*_raw` metadata keys and the no-warning case.
  **Superseded below (2026-07-27)**: this pass hand-picked only `msDesc`/
  `encodingDesc` for raw-preservation; the generalization that closes the
  gap for *every* unmodeled teiHeader field is the next entry.
- [x] **Fixed (2026-07-27)**: generalized the `msDesc`/`encodingDesc`
  raw-preservation above from a two-element special case to *any* teiHeader
  child element `convert_element` has no dedicated semantic mapping for.
  `convert_children` now threads an `in_header: bool` through its recursion
  (true once inside `<teiHeader>` or any of its descendants) and, for any
  such child not in the new `is_modeled_header_field` allow-list
  (`author`/`editor`/`publisher`/`idno`/`language`/`abstract`/`keywords`/
  `change`/`title` — the fields that already have dedicated semantic
  extraction), captures the whole subtree's original XML via
  `tei_fmt::emit_fragment`, same mechanism as before. The hardcoded
  `msDesc`/`encodingDesc` arms in `convert_element` were removed — they were
  already producing the exact same `generic_span`/`generic_div` node the
  generic catch-all does, so removal is a no-op for those two and the
  general path now covers them plus everything else (`particDesc`,
  `projectDesc`, or any other TEI header element). Metadata key naming
  generalized from the old ad hoc snake_case (`ms_desc_raw`,
  `encoding_desc_raw`) to `{tag}_raw` using the element's actual XML tag
  name (`msDesc_raw`, `encodingDesc_raw`, `particDesc_raw`, ...), plus a
  `{tag}` flattened-text convenience copy — `extract_metadata` now matches
  both `span` and `div` node kinds (previously `div`-shaped unrecognized
  header children were silently invisible to it) and skips recursing into
  an already-raw-captured subtree's children (nothing further to extract;
  previously this could double-process msDesc's internal
  msIdentifier/physDesc/etc. as spurious separate warnings). The old
  per-field fidelity warning only fires now if raw capture genuinely fails
  (non-UTF8 content — not expected for XML that parsed at all).
  `rescribe-write-tei` generalized correspondingly: instead of two hardcoded
  `ms_desc_raw`/`encoding_desc_raw` checks, it scans `Document::metadata`
  for any `*_raw`-suffixed key and splices each back via `tei_fmt::Node::Raw`
  as a `teiHeader` child, sorted by tag for deterministic output.
  `fixtures/tei/header-ms-desc`, `header-encoding-desc`, and
  `path-full-header` updated to the new key names; new fixture
  `header-partic-desc` (`<profileDesc><particDesc>`, an element with no
  explicit semantic mapping and not one of the previously-hardcoded two)
  regression-tests the general path directly. Residual gap: none known —
  every teiHeader child without a dedicated semantic mapping now
  raw-preserves rather than warn-and-drop; the warning path is reachable
  only in the (currently unexercised) genuine-raw-capture-failure case.
- [x] **Fixed (2026-07-27)**: an unrecognized element at block level no
  longer round-trips wrapped in an extra `<p>`. `rescribe-read-tei` gained
  `is_block_element(tag: &str) -> bool` (mirroring
  `rescribe-read-html::is_block_element`) listing TEI's block-level
  vocabulary, plus a `generic_div` helper (the block-level counterpart to
  `generic_span`). The catch-all fallback in `convert_element` now branches:
  unrecognized block-level elements become a `div` tagged `tei:tag`
  (`generic_div`), unrecognized inline elements keep the existing `span`
  path (`generic_span`). `rescribe-write-tei`'s `node::DIV` arm now falls
  back to `tei:tag` (re-emitting the original element name) when no
  `tei:type` matches, mirroring `rescribe-write-html::convert_div`'s
  `html:tag` handling. New fixtures `adv-unknown-block-element` (`<closer>`)
  and `adv-unknown-inline-element` (`<mysteryTag>` nested in running text)
  regression-test both branches; `adv-unknown-element` (the pre-existing
  fixture, an unrecognized element that is *not* in the block-level
  vocabulary, sitting at block-dispatch position) continues to assert the
  bare-span behavior for that case, which is correct — the classification is
  by TEI content-model shape, not by dispatch position.
- [ ] DTD-aware entity resolution remains out of scope for this pass (Tier B
  target is 3-Harness, not 5-Production; fixture-suite-complete is step 1 of 5
  in the vertical checklist — reader/writer completeness beyond the fixture
  suite, the oracle harness, and a longer fuzz campaign are still open).

### DEBT: Streaming architecture — COMPLETED 2026-03-28

**`events()` frame-stack — DONE:**
All four crates use `Vec<Frame>` frame-stack. Memory O(nesting depth). `parse()` is
direct recursive descent, independent of events().

**`StreamingParser<H>` Tier 2 — DONE (line-oriented crates):**
- org-fmt: blank-line separation + #+BEGIN_*…#+END_* (O(largest block))
- rst-fmt: blank-line separation + directive body (O(largest block))
- asciidoc: blank-line separation + delimited blocks (O(largest block))
- djot-fmt: blank-line separation + fenced code / div (O(largest block))
- rtf-fmt: O(full input) — documented structural constraint; cannot be improved
  without significant parser refactoring (font/color table dependency)
- commonmark-fmt: O(full input) — pulldown-cmark requires full `&str`; exemption documented

**`Cow::Borrowed` — DONE for djot-fmt (2026-03-28):**
`Text` events for headings and paragraphs now yield `Cow::Borrowed` when the span maps
cleanly to the original input (no escape processing). Implementation: `Frame::InlineText`,
`ParseContext::line_offset_at()`, real base_offset in push_heading/paragraph_frames.

**Remaining (other crates):**
- [ ] `Cow::Borrowed` for org-fmt — inline parser uses `Span::NONE`; needs span tracking in parse_inline_content before base_offset approach works
- [ ] `Cow::Borrowed` for rst-fmt — same; `Inline::Text(String)` has no span at all
- [ ] `Cow::Borrowed` for asciidoc — same as rst-fmt
- [ ] `Cow::Borrowed` for djot-fmt Verbatim/Math — Verbatim trimming means span ≠ content slice; would need a content-only span separate from the full backtick-construct span

### `rst-fmt` — API modes complete (2026-03-23)

- [x] `stream`: `events(input: &str) -> EventIter` pull iterator
- [x] `batch`: BatchParser (feed/finish) + BatchSink<F> callback style
- [x] `batch`: StreamingParser<H: Handler> + Handler trait (2026-03-25)
- [x] `w-stream`: Writer<W: Write> streaming writer
- [x] Feature flags: ast, streaming, batch, writer-streaming, writer-builder
- [x] Fix events() — now a true pull iterator (2026-03-24)
- [x] events() frame-stack fix — O(nesting depth), not O(block subtree) (2026-03-28)
- [x] parse() direct recursive descent — independent of events() (2026-03-28)
- [x] StreamingParser<H> Tier 2 — O(largest block) streaming (2026-03-28)
- [ ] Parser gaps: table parsing, footnote parsing

### `org-fmt` — API modes complete (2026-03-23)

- [x] `stream`: pull iterator (events())
- [x] `batch`: BatchParser + BatchSink
- [x] `batch`: StreamingParser<H: Handler> + Handler trait (2026-03-25)
- [x] `w-stream`: Writer<W: Write> streaming writer
- [x] Feature flags added
- [x] Fix events() — now a true pull iterator (2026-03-24)
- [x] events() frame-stack fix — O(nesting depth), not O(block subtree) (2026-03-28)
- [x] parse() direct recursive descent — independent of events() (2026-03-28)
- [x] StreamingParser<H> Tier 2 — O(largest block) streaming (2026-03-28)
- [ ] Parser/writer gaps: blockquote nesting, footnote definitions, figure/caption blocks

### `asciidoc` — API modes complete (2026-03-23)

- [x] `stream`: pull iterator (events())
- [x] `batch`: BatchParser + BatchSink
- [x] `batch`: StreamingParser<H: Handler> + Handler trait (2026-03-25)
- [x] `w-stream`: Writer<W: Write> streaming writer
- [x] Feature flags added
- [x] Fix events() — now a true pull iterator (2026-03-24)
- [x] events() frame-stack fix — O(nesting depth), not O(block subtree) (2026-03-28)
- [x] parse() direct recursive descent — independent of events() (2026-03-28)
- [x] StreamingParser<H> Tier 2 — O(largest block) streaming (2026-03-28)
- [ ] Parser gaps: table parsing, footnote parsing, math parsing
- [ ] Markdown family (pulldown-cmark backed; adapter hardening + fuzz)
- [x] HTML (html5ever backed) — **5-Production** (R:5†/W:5†; 85/85 COVERAGE.md items, 2026-07-26)
  - [x] `html-fmt` crate created (2026-04-11): standalone HTML5 AST, parse (html5ever RcDom), events (AST walk), batch (StreamingParser/BatchParser), emit (with pretty-print), streaming writer. `rescribe-read-html` and `rescribe-write-html` rewired as thin adapters over `html_fmt::HtmlDoc`. Note: HTML5 tree construction algorithm requires full tree for correctness (foster parenting, adoption agency), so `events()` and `StreamingParser` build the tree internally — this is a spec limitation, not a library choice, documented in `batch.rs`.
  - [x] Footnote anchor convention (2026-07-26): the reader had **no** footnote recognition at all before this (write-only, unverified). Now recognizes `<sup class="footnote-ref"><a href="#fn-{label}">…</a></sup>` and `<div id="fn-{label}" class="footnote"><sup class="footnote-label">…</sup><span class="footnote-content">…</span><a class="footnote-back">…</a></div>` and reconstructs `footnote_ref`/`footnote_def`. Marker/backlink are regenerated deterministically from the label on write (not read back), so the round-trip only needs the content span to survive — lossless without depending on fragile whitespace/id matching. Fixture: `fixtures/html/footnote/`.
  - [x] Inline MathML (2026-07-26): added `html_fmt::emit_fragment`/`emit_fragment_with_options` (general-purpose subtree serializer in `html-fmt`, usable by any consumer — not adapter-only) plus reader support for `<math>…</math>`. Full structural modeling into `math:*` node kinds was judged out of scope (MathML has its own large presentation/content vocabulary); per CLAUDE.md's raw-preservation pattern it's captured verbatim as `math_inline`/`math_display` with `math:format="mathml"` + `math:source` holding the exact MathML markup (`display="block"` → math_display). Writer now branches on `math:format`: MathML round-trips byte-for-byte via `Raw`, LaTeX `math:source` keeps the existing `\(…\)`/`\[…\]` convention. Fixture: `fixtures/html/inline-math-mathml/`.
  - [x] Megabyte pathological fixture (2026-07-26): `fixtures/html/path-large-inline-text/` — single `<p>` with a ~4.9MB text node.
  - 8 new/updated unit tests added across `rescribe-read-html`/`rescribe-write-html` covering footnote and MathML round-trips; `cargo clippy --all-targets --all-features -p html-fmt -p rescribe-read-html -p rescribe-write-html -- -D warnings` and full test suite both clean.
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

  **DOCX streaming writer** (`WmlWriter<W>`):
  - [x] Image support — `register_image(rel_id, data, content_type)` on `WmlWriter`;
        maps caller rel_ids to builder-assigned rel_ids; `Image { rel_id }` event
        embeds via `DocumentBuilder::add_image` + `Drawing` → `RunContent::Drawing`
  - [ ] Footnote/endnote support — add `register_footnote(id, Vec<OwnedWmlEvent>)` /
        `register_endnote(id, Vec<OwnedWmlEvent>)`; process via same stack machine into
        `FootnoteEndnote` bodies; wire `FootnoteRef`/`EndnoteRef` events to registered bodies

  **XLSX streaming writer** (`SmlWriter<W>`):
  - [x] Shared-string resolution — `set_shared_strings(Vec<String>)` on `SmlWriter`;
        `CellType::SharedString` cells now index into the table instead of emitting
        the raw index as a number

  **PPTX streaming writer** (`PmlWriter<W>`):
  - [x] Multi-slide support — `new_slide()` method records a slide-boundary position;
        `process_pml_events` slices the event buffer per slide and calls `process_slide`
        once each; no `new_slide()` call = single-slide (original behaviour preserved)
  - [x] Table content — `StartTableCell`/`EndTableCell` treated as paragraph boundaries;
        text inside cells collected into current shape's paragraph list
  - [ ] Shape geometry — **design decision required**: add EMU position/size fields to
        `StartShape` in `PmlEvent` (requires YAML + codegen regen); until then, round-trip
        fidelity for shape layout is impossible

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
- [x] EPUB — 3-Harness (30 fixtures, fuzz target compiles, 2026-03-28)
- [ ] ODT writer (no library; treat as a vertical)
- [ ] AZW3 reader/writer (boko as reference, MIT attribution)
- [ ] PDF reader (pdf-extract backed; already at 4)

### ooxml-fmt rework (major milestone — after five-crate streaming upgrade)

The ooxml-* crates are our biggest value proposition: no other Rust ecosystem library
handles DOCX/XLSX/PPTX at production quality. The rework consolidates them and adds
the full three-API streaming architecture from `docs/format-library-design.md`.

**Why streaming is non-optional for OOXML:**
DOCX/XLSX/PPTX files in legal discovery, academic corpora, and enterprise search
routinely exceed available RAM. A library that requires the full file in memory before
parsing starts is unusable for these workloads. `StreamingParser<H>` with O(nesting
depth + largest token) memory is the primary use case, not an afterthought.

**Architecture targets:**
- OPC layer: chunked ZIP entry streaming — decompress one entry at a time, never the
  full archive. The ZIP central directory is parsed first (it's at the end of the file,
  so this requires two passes or a seekable source); entries are decompressed on demand.
- XML layer: SAX-style events from `quick-xml` fed directly to the format state machine.
  No intermediate DOM allocation.
- Format layer (`wml`, `sml`, `pml`): `StreamingParser<H>` translates XML events to
  format-level events. The handler receives `Event::StartParagraph`, `Event::Text(cow)`,
  etc. — no intermediate `Block` allocation.
- `parse()`: direct tree construction from the SAX stream. No events() indirection.
- `events()`: format-level pull iterator over a fully-loaded `&[u8]`. Wraps the same
  state machine as `StreamingParser` but driven by `Iterator::next()`.

**Consolidation:**
- [ ] Merge `ooxml-wml`, `ooxml-sml`, `ooxml-pml`, `ooxml-dml`, `ooxml-omml`,
  `ooxml-opc`, `ooxml-xml` into a single `ooxml-fmt` crate with feature flags.
  Shared infrastructure (`opc`, `xml`) always compiled; `wml`/`sml`/`pml`/`dml`/`omml`
  feature-gated. `crates/tools/ooxml-codegen` stays separate (build tool).
- [ ] Implement `StreamingParser<H>` for DOCX (wml) first — largest user base.
- [ ] Implement `StreamingParser<H>` for XLSX (sml) — critical for data pipelines.
- [ ] Implement `StreamingParser<H>` for PPTX (pml).
- [ ] `parse()` as direct recursive descent (independent of events()).
- [ ] `events()` as true pull iterator (frame-stack, no block-granular buffering).
- [ ] Publish `ooxml-fmt` to crates.io.
- [ ] Deprecate individual crates — final version with deprecation notice pointing to
  `ooxml-fmt`. Keep compiling; mark `#[deprecated]` on the re-exported API surface.

### Milestone: M2.5 — Streaming IR layer

End-to-end streaming conversion with O(nesting depth + largest token) memory.
Never materializes the full document. Required for large-document workloads.
See CLAUDE.md "Streaming IR" section for architecture and rationale.

**Prerequisite:** All five hand-rolled crates at true Tier 2 `StreamingParser`
(see DEBT section above). ooxml-fmt rework also required before OOXML can stream.

**rescribe-core additions:**
- [ ] `IrEvent<'a>` — format-agnostic SAX-style open/close event type, mirroring
  rescribe-std node kinds (StartParagraph/EndParagraph, StartHeading{level}/EndHeading, Text(Cow), etc.)
- [ ] `IrHandler` trait — `fn handle(&mut self, event: IrEvent<'_>)`
- [ ] `StreamingReader` trait — `feed(&mut self, chunk: &[u8])` + `finish(self)`
  where the impl drives a format `StreamingParser` and translates format events to `IrEvent`
- [ ] `StreamingWriter` trait — `handle(&mut self, event: IrEvent<'_>)` + `finish(self) -> Vec<u8>`
- [ ] `IrTransformer` — `IrHandler` wrapper that transforms events and forwards to inner `IrHandler`
- [ ] `DocumentBuilderHandler` — `IrHandler` impl that assembles a `Document` (materialized path)

**Format adapter additions (one per format):**
- [ ] Each `rescribe-read-{fmt}` gains a `StreamingReader` impl that wraps the format
  library's `StreamingParser` and translates format events → `IrEvent`
- [ ] Each `rescribe-write-{fmt}` gains a `StreamingWriter` impl

**Pipeline:**
```
feed(chunk) → StreamingReader → IrEvent → IrTransformer → IrEvent → StreamingWriter → output chunk
```

---

### Milestone: M3 — Tier B/C verticals

Tier B formats at 3-Harness or 2-Fixtures (where harness is N/A), each with a
standalone library where the ecosystem gap justifies it.

- [ ] `t2t` vertical — **4-Fuzz** → needs re-fuzz after expansion (2026-03-29)
  - [x] Split monolith lib.rs into ast.rs / parse.rs / emit.rs
  - [x] Span on every AST node; Diagnostic type; strip_spans()
  - [x] parse() infallible → (T2tDoc, Vec<Diagnostic>)
  - [x] No-panic fuzz gate (`fuzz_t2t_reader`) — 2M runs clean; needs re-run
  - [x] Roundtrip fuzz target (`fuzz_t2t_roundtrip`) — 939K runs clean; needs re-run
  - [x] New constructs: DefinitionList block; Verbatim/Tagged inlines; document header metadata (2026-03-29)
  - [x] All API modes: ast + stream + batch + w-build + w-stream (2026-03-29)
  - [x] Oracle harness + benchmarks (2026-03-29)
  - [x] Fixtures: COVERAGE.md all boxes checked (2026-03-29)
  - [ ] Re-run fuzz clean after construct expansion
- [ ] `markua` vertical — **4-Fuzz** → needs re-fuzz after expansion (2026-03-29)
  - [x] No-panic fuzz gate + roundtrip fuzz — clean on original constructs; needs re-run
  - [x] New constructs: DefinitionList, PageBreak, Figure blocks; SpecialBlock reworked to hold Vec<Block>; Subscript/Superscript/Underline/SmallCaps/FootnoteRef/IndexTerm/MathInline inlines (2026-03-29)
  - [x] All API modes: ast + stream + batch + w-build + w-stream (2026-03-29)
  - [x] Benchmarks: markua_parse_small, markua_parse_medium, markua_emit_medium (2026-03-29)
  - [x] Fixtures: COVERAGE.md all boxes checked (2026-03-29)
  - [ ] Re-run fuzz clean after construct expansion
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

### Update CLAUDE.md — corrections as documentation lag (2026-03-29)

Add to the corrections section:
> **Corrections are documentation lag, not model failure.** When the same mistake recurs, the fix is writing the invariant down — not repeating the correction. Every correction that doesn't produce a CLAUDE.md edit will happen again. Exception: during active design, corrections are the work itself — don't prematurely document a design that hasn't settled yet.

Add to the Session Handoff section:
> **Initiate a handoff after a significant mid-session correction.** When a correction happens after substantial wrong-path work, the wrong reasoning is still in context and keeps pulling. Writing down the invariant and starting fresh beats continuing with poisoned context — the next session loads the invariant from turn 1 before any wrong reasoning exists.

Conventional commit: `docs: add corrections-as-documentation-lag + context-poisoning handoff rule`

---

## Ad-hoc dispatch findings (2026-05-29)

From an ecosystem-wide investigation of ad-hoc dispatch architecture (2026-05-29). The recurring anti-pattern: N parallel dispatch tables keyed on a closed name/enum set where one registry/trait/visitor belongs — strongest tell is DRIFT (parallel tables disagreeing). Each finding names the general mechanism it should have been.

- **R1 — 3 parallel format-match arms bypass the `Parser`/`Emitter` traits.** `rescribe-cli/src/main.rs`: `parse_text` (line ~805), `parse_binary` (line ~874), `emit` (line ~900) each manually enumerate every format and call format-specific free functions; plus a 60-entry `const FORMATS` (lines ~80–676). The library's `Parser`/`Emitter` traits expose `fn formats(&self)` — the exact dispatch mechanism — but the CLI ignores it. Adding a format = 4-place edit, compiler can't enforce consistency. SHOULD BE: registry dispatch via `Parser::formats()`/`Emitter`. This is the cleanest bypassed-abstraction finding in the conversion cluster.

## JATS citation/bibliography IR vertical closed (2026-07-28)

Following DocBook's citation vertical (`8aedfb80fa`), the JATS citation/reference-list
design fork noted above (and in `fixtures/jats/COVERAGE.md`) is resolved: `<ref-list>` ->
`bibliography`, `<ref>` -> `bibliography_entry`, `<element-citation>`/`<mixed-citation>`
fields -> `bibliography_field` children, using the same node kinds added in `4e15c996`.
`jats-fmt` itself needed no changes (its AST is generic XML, like docbook-fmt's) — all the
work is in `rescribe-read-jats`/`rescribe-write-jats`.

One correction to the original task framing worth recording: the date-handling
instructions referenced `<pub-date>`'s `year`/`month`/`day` children, but the JATS 1.3 Tag
Library (fetched live, not from memory) confirms `<element-citation>`'s content model has
no `<pub-date>` child at all — the actual date-bearing elements are bare
`<year>`/`<month>`/`<day>` and/or a `<date>` wrapper, both optionally carrying an
`iso-8601-date` attribute (per the Tag Library's own tagged examples, e.g. `<year
iso-8601-date="2001-11">2001</year>`). Implemented against the schema-verified elements
instead; the attribute-preferred-over-reconstruction design intent was unaffected.

Fixtures: `fixtures/jats/citation-{simple-author,multi-author,markup-in-field,
mixed-citation,date}`, `fixtures/jats/path-many-references`. COVERAGE.md's back-matter/
integration/pathological reference-list boxes are now all checked; the two remaining
open boxes (MathML `<math>` as an alternative to `<tex-math>`, and `<alternatives>`'s
block-vs-inline non-classification) are unrelated pre-existing design forks, untouched.

Also extended `crates/rescribe-fixtures`' `check_prop_in` (and `fixtures/spec.md`) to match
JSON objects against `PropValue::Map` — needed to assert `prop::DATE` in the new `citation-
date` fixture, and a general gap: DocBook's own earlier citation fixtures never exercised
`prop::DATE` at all, for lack of this.

## TEI citation/bibliography IR vertical closed (2026-07-28)

Following DocBook's (`8aedfb80fa`) and JATS's (`060c0858d5`) citation verticals, TEI is
done using the same `bibliography`/`bibliography_entry`/`bibliography_field` node kinds
added in `4e15c996`. `tei-fmt` itself needed no changes (its AST is generic XML, like
docbook-fmt's/jats-fmt's) — all the work is in `rescribe-read-tei`/`rescribe-write-tei`.
`<listBibl>` -> `bibliography`; `<biblStruct>`/a `<bibl>` directly inside `<listBibl>` ->
`bibliography_entry`; a bare `<bibl>` used elsewhere (e.g. inline `<cit>` attribution) is
deliberately left as the pre-existing plain-`span` mapping, per the already-passing
`int-cit-bibl` fixture.

**Analytic/monogr/series fork resolution (implemented as instructed, not re-derived):**
`<biblStruct>`'s `<analytic>` level flattens directly into the entry's own
`bibliography_field` children; `<monogr>`/`<series>` each become their own nested
`bibliography_entry`, mirroring DocBook's `<biblioset>` nesting. `<imprint>` is a third
transparent wrapper (splices into whichever entry it's inside), needed because TEI's own
`monogr` content model groups `<publisher>`/`<pubPlace>`/`<date>` there.

**Date-attribute-semantics fork — resolved, not deferred:** implemented `tei:date-attr`
raw-preservation for the single-attribute case. TEI's `att.datable` class (`@when`/
`@notBefore`/`@notAfter`/`@from`/`@to`, or their `-iso`-suffixed siblings) is judged
adequately captured by parsing into the structured `prop::DATE` map plus a `tei:date-attr`
property recording which attribute was used — a reader can tell a point (`when`) apart
from a one-sided bound (`notBefore`/`notAfter`/`from`/`to`) without the distinction being
lost. **However, when `@notBefore`+`@notAfter` (or `@from`+`@to`) are present *together*,
this reader does NOT populate `prop::DATE` at all** — that pair jointly expresses a
genuine two-point RANGE (a lower bound and an upper bound), which does not fit
`prop::DATE`'s single year/month/day Map even with `tei:date-attr` attached: there is no
single "the" point to store. This is exactly the structural mismatch the original task
brief anticipated as a possible fork. Per CLAUDE.md's no-guessing rule, no new range
representation was invented for it — the range case falls back to raw-preserving
`@notBefore`+`@notAfter` (or `@from`+`@to`) verbatim on a `misc` `bibliography_field`
instead, so nothing is silently dropped; only the *modeling* of a two-point range as a
first-class `prop::DATE`-like property remains open. **Decision needed:** should
`rescribe-std` eventually gain a distinct range-shaped date property (e.g. `prop::
DATE_RANGE` as a `{from: Map, to: Map}` Map-of-Maps, or two Maps under `date:from`/
`date:to`), or is raw-preservation-only sufficient for this case indefinitely? See
`fixtures/tei/citation-date` (the R3 assertion) and `fixtures/tei/COVERAGE.md`'s
Bibliography/citation section for the concrete fixture demonstrating this.

Fixtures: `fixtures/tei/citation-{simple-author,multi-author,markup-in-field,bibl-mixed,
analytic-monogr-series,date}`. `fixtures/tei/COVERAGE.md`'s new Bibliography/citation
section is fully checked except for the range-date modeling question above, which is
tracked here rather than silently marked done.

## DOCX (OOXML `b:` bibliography namespace) citation vertical deferred (2026-07-28)

Following DocBook (`8aedfb80fa`), JATS (`060c0858d5`), and TEI (`b61994215c`), the fourth
planned citation vertical — `b:Sources`/`b:Source` -> `bibliography`/`bibliography_entry`
in `ooxml-wml`/`rescribe-read-docx`/`rescribe-write-docx` — was **not** implemented this
session, per the original brief's explicit stretch-goal clause (defer if the crate isn't
architecturally ready).

Why: DocBook/JATS/TEI all share generic-XML-AST format crates where `<bibliography>`/
`<ref-list>`/`<listBibl>` and their entry elements were already parseable-but-unhandled —
all three verticals only needed adapter-layer work. OOXML's bibliography namespace
(`http://schemas.openxmlformats.org/officeDocument/2006/bibliography`, ECMA-376 Part 4) is
architecturally different: `ooxml-wml`'s `generated.rs`/`generated_parsers.rs`/
`generated_serializers.rs` are codegenned (`build.rs`) from RELAX NG compact schemas —
currently only `wml.rnc` plus `shared-commonSimpleTypes.rnc` are wired into that pipeline.
The bibliography namespace has no schema file in the codegen input set at all (confirmed:
`grep` over `build.rs` and the crate for `b:Sources`/`CTSources`/`bibliography.rnc` finds
nothing beyond an unrelated `w:bibliography` compatibility-settings `CTEmpty` flag). Adding
real support means sourcing/vendoring the bibliography RNC/XSD schema and extending the
codegen input set — a schema-generation-pipeline change, not an adapter-layer fill-in —
before any `rescribe-read-docx`/`rescribe-write-docx` work could even begin. That's a
materially larger and differently-shaped task than the other three verticals, so per
CLAUDE.md's "work one vertical to completion, no horizontal sweeps" and the brief's own
deferral clause, it's left as a clearly-scoped follow-up rather than attempted partially.

**Follow-up vertical, when picked up:** (1) vendor the OOXML bibliography RNC/XSD schema
and wire it into `ooxml-wml/build.rs`'s codegen alongside `wml.rnc`; (2) map `b:Sources` ->
`bibliography`, `b:Source` -> `bibliography_entry` with `bibliography_field` children (all
fields are `ST_String255` — flat, no nested markup possible in this namespace, so each
field's children will just be a single `text` node, unlike DocBook/JATS/TEI); (3) raw-
preserve `b:Tag`/`b:SourceType` as `docx:tag`/`docx:source-type` (round-trip-critical:
Word keys in-text citations off `b:Tag`); (4) `b:Year`/`b:Month`/`b:Day` -> `prop::DATE`.

### Discovered gap: pre-existing bibliography readers don't use this IR shape

While adding the citation IR shape above, noticed that `rescribe-read-bibtex`,
`rescribe-read-csl-json`, `rescribe-read-ris`, and `rescribe-read-endnotexml` (all
pre-existing, not touched this session) use a completely different, ad-hoc representation:
a `definition_list` node with each entry's fields flattened into `Properties` as plain
strings (e.g. `rescribe-read-csl-json/src/lib.rs`'s `convert_item`). This predates the
`bibliography`/`bibliography_entry`/`bibliography_field` node kinds added in `4e15c996` and
was not migrated onto them — those four formats are pure-metadata bibliography formats
(BibTeX/CSL-JSON/RIS/EndNote XML) rather than markup-in-document formats like DocBook/JATS/
TEI, so the flat-string approach may or may not be an actual fidelity problem for them (CSL
fields like `title`/`container-title` are effectively always plain text in practice, unlike
DocBook's/JATS's/TEI's markup-permitting equivalents) — this needs a human call, not a
guess: (a) leave the four metadata-format readers as-is (flat properties) since their
source formats genuinely have no nested markup capability, accepting that rescribe now has
two different bibliography-entry shapes in the IR depending on which format produced them,
or (b) migrate all four onto `bibliography`/`bibliography_entry`/`bibliography_field` for
consistency across the whole bibliography surface, even though the field-children-as-
inline-nodes indirection buys nothing for these formats. Flagging rather than deciding.
