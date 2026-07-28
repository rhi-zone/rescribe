# Format Implementation Audit

Assessed 2026-02-24; stages updated 2026-03-21 (wiki formats 2→4; csv/tsv/ris/texinfo 2→4; mediawiki 3→4; odt/fb2/docbook/jats/opml/tei 3→4; commonmark/gfm/markdown-strict/multimarkdown 3→4; pulldown-cmark upgraded to 0.13; beamer/revealjs/slidy/s5/dzslides/slideous/context/ms/icml/chunkedhtml/plaintext writers 2→4); RST/Org/AsciiDoc writer APIs added 2026-03-23 (streaming + builder); 2026-03-29: definition of 5-Production tightened — reader-only no longer qualifies; RST/Org/AsciiDoc demoted from R:5 to R:4 due to construct gaps (tables, footnotes); writer column updated from 2→4 (API modes complete, fuzz clean, construct gaps remain). djot-fmt + textile-fmt signed off at 5-Production (2026-03-29). RST/AsciiDoc/Org signed off at 5-Production (2026-03-29; all construct gaps closed: tables, footnotes, math, nested blockquotes, figure/caption). 2026-03-30: muse/t2t/man/markua/creole/dokuwiki/vimwiki/zimwiki/xwiki/twiki/tikiwiki/jira/mediawiki all completed to R:4/W:4; fountain/texinfo/bbcode/pod/haddock/ansi same (all constructs + API modes + fixtures; need fuzz re-run). 2026-04-10: commonmark/gfm writers promoted W:3→W:5 (fuzz_commonmark_reader 284K runs clean, fuzz_commonmark_roundtrip 197K runs clean; all writer API modes already implemented). 2026-03-31: all 44 fuzz targets (22 format pairs) ran clean — 12 fuzz failures found and fixed (djot-fmt char/byte panics, sanitiser gaps across textile/twiki/muse/mediawiki/haddock/t2t/markua); all 19 R:4/W:4 formats promoted to 5-Production. 2026-04-10: odf-fmt signed off at 5-Production (ODS/ODP full AST support, complete fixture suite, batch API, streaming writer, fuzz targets wired). docx writer promoted W:4→W:5 (fuzz_docx_reader 3.47M runs clean, fuzz_docx_roundtrip 119K runs clean; all construct coverage and API modes already complete). epub promoted R:4→R:5/W:3→W:5: 9 new fixtures (figure-with-caption, definition-list, section-div, span-style, cross-document-link, metadata-extended, adv-invalid-xhtml, adv-empty-spine, path-many-chapters); fuzz_epub_reader and fuzz_epub_roundtrip both run clean (300s each, 189K roundtrip runs); library limitations documented in COVERAGE.md. 2026-04-10: odf-fmt fuzz confirmed clean (fuzz_odf_fmt_reader 1.95M runs, fuzz_odf_fmt_roundtrip 124K runs). fb2 promoted W:2→W:4: roundtrip fuzz target added, 10 new fixtures (epigraph, empty-line, subtitle, author-metadata, lang-metadata, genre-metadata, internal-link, adv-malformed, adv-entity-refs, adv-empty-section), reader fixes (entity decoding via GeneralRef events, metadata extraction for lang/genre/keywords, section id preservation, metadata container children now discarded to prevent leakage), writer fixes (epigraph/poem/stanza/text-author detected from IR props); fuzz_fb2_reader 6.3M runs clean, fuzz_fb2_roundtrip clean. html promoted W:3→W:4: fuzz_html_reader 1.73M runs clean, fuzz_html_roundtrip 1.21M runs clean. 2026-04-10 (html 5†): 44 new fixtures (82/85 COVERAGE.md items checked); semantic HTML5 elements (section/article/aside/nav/header/footer/address/details/summary) preserved as div with html:tag prop; global attributes (lang/dir/style/id/class) propagated to all block/inline nodes; <ins> separated from <u>; abbr/mark/kbd/var/samp/cite added as span{html:tag}; colgroup/col silently stripped; extract_metadata extended (html@lang, meta@charset, link@stylesheet, base@href); writer respects html:tag on div/span for lossless re-emission; fixture runner added. 3 items deferred: footnote anchor convention (requires tree-level pattern detection), inline MathML (separate embedded language), multi-megabyte pathological test (file size). html R:4†/W:4† (fuzz already clean; fixture coverage 82/85; remaining 3 items block 5-Production). 2026-04-10 (architecture): fb2-fmt standalone crate created; rescribe-read-fb2/rescribe-write-fb2 now thin adapters (no quick-xml/base64 in adapter deps); fb2-fmt events() pull iterator and StreamingParser<H> implemented; fb2 fixture suite advanced to 47/63 items (19 new: date, keywords, translator, src-lang, series-sequence, cover-image, publisher-info, document-info, custom-info, image-alt-text, xml-lang-body, inline-image, poem-epigraph, adv-missing-xmlns, adv-broken-image-ref, adv-numeric-charref, deeply-nested-sections, many-paragraphs, table-many-cells); remaining 16 items require footnote/binary infrastructure. rescribe-read-odt/rescribe-write-odt rewritten to use odf_fmt::parse()/emit() — adapter no longer calls quick-xml/zip directly; odf-fmt bug fixed (self-closing style:text-properties was consuming office:body). parse_numbering_order() moved from rescribe-read-docx into ooxml-wml::numbering; quick-xml removed from docx adapter deps. 2026-04-10 (fb2 5†): fb2-fmt footnote support (FootnoteRef AST node, notes body parsing/emitting), streaming writer (Writer<W: Write>), binary embedding fixtures, 6 additional coverage fixtures (annotation, link-title, table-alignment, adv-invalid-base64, adv-broken-footnote-ref, pathological-large-binary) — COVERAGE.md 63/63 checked; fuzz_fb2_reader 644K runs clean, fuzz_fb2_roundtrip 5.98M runs clean (1 crash found and fixed: <code> content was trimmed, dropping leading whitespace); fb2 R:5†/W:5†. odf-fmt: 12 new constructs added (SoftHyphen, Bookmark, Annotation, font_variant, user_defined meta, page-layout props, list ordering); all ODT fixture regressions from odt rewrite resolved. 2026-07-26 (html 5†): closed the 3 remaining gaps blocking html 5-Production. Footnote anchor convention: reader had zero footnote recognition (write-only before this); now recognizes `<sup class="footnote-ref"><a href="#fn-{label}">`/`<div id="fn-{label}" class="footnote"><sup class="footnote-label">…<span class="footnote-content">…</span><a class="footnote-back">` and reconstructs footnote_ref/footnote_def losslessly (marker/backlink are regenerated from the label, not read back, so only the content span needs to round-trip). Inline MathML: added html-fmt::emit_fragment (general-purpose subtree serializer, not adapter-specific) and reader support for `<math>…</math>` — raw-preserved verbatim as math_inline/math_display with `math:format="mathml"` + `math:source` holding the exact MathML markup (full structural modeling into math:* nodes deferred as out of scope per CLAUDE.md's raw-preservation pattern); writer now branches on math:format so MathML round-trips byte-for-byte while LaTeX math:source keeps the existing \\(…\\)/\\[…\\] convention. Added path-large-inline-text pathological fixture (~4.9MB single text node). fixtures/html/COVERAGE.md now 85/85; 8 new/updated unit tests (round-trip assertions) + 3 new fixture dirs (footnote, inline-math-mathml, path-large-inline-text). html R:5†/W:5†. 2026-07-26 (docbook architecture): `docbook-fmt` standalone crate created (`crates/formats/docbook-fmt`) wrapping quick-xml, with its own `DocBookDoc`/`Node` AST (`Element`/`Text`/`Cdata`/`Comment`/`ProcessingInstruction`/`Doctype`/`EntityRef`), `parse()`, a genuinely independent SAX-style `events()` (XML's well-nestedness means — unlike HTML5 — no tree needs to be built first), an incrementally-draining `StreamingParser<H>` (dispatches events as soon as provably complete, buffer bounded by the largest in-progress token; verified with chunk-boundary-split tests for both text and tags), `emit()` builder writer, and a streaming `Writer<W: Write>`. `rescribe-read-docbook`/`rescribe-write-docbook` rewired to thin AST↔IR translators (no `quick-xml` left in adapter production code); all prior construct mappings preserved with parity, plus one incidental bug fix (`xlink:href` link-attribute matching was dead code in the old adapter — it stripped namespace prefixes before comparing against the literal string `"xlink:href"`, so it could never match; `docbook-fmt` preserves the raw prefixed attribute name, so it now works). Unresolvable named XML entities (DTD-defined, not one of the 5 predefined or a numeric char ref) are now raw-preserved as `raw_inline`/`docbook:entity` rather than silently dropped. Stage numbers unchanged (R:4/W:2) — this pass is the architecture extraction only; fuzz targets and the oracle harness run are still open (`TODO.md`). 2026-07-26 (jats architecture): `jats-fmt` standalone crate created (`crates/formats/jats-fmt`), mirroring `docbook-fmt`'s generic-XML AST (`JatsDoc`/`Node` with `Element`/`Text`/`Cdata`/`Comment`/`ProcessingInstruction`/`Doctype`/`EntityRef`) since JATS is also plain well-nested XML: `parse()`, an independent SAX-style `events()`, an incrementally-draining `StreamingParser<H>`, `emit()` builder writer, and a streaming `Writer<W: Write>`. `rescribe-read-jats`/`rescribe-write-jats` rewired to thin AST↔IR translators (no `quick-xml` left in adapter production code); all prior construct mappings preserved with parity, plus one incidental fidelity fix (`<xref ref-type="…">` now preserves `jats:ref-type` for both the self-closing and full-element shapes — the old hand-rolled reader only attached it for the self-closing case). Unresolvable named XML entities are now raw-preserved as `raw_inline`/`jats:entity` rather than silently dropped (the old reader had no entity handling at all). Stage numbers unchanged (R:4/W:2) — this pass is the architecture extraction only; fuzz targets and the remaining `fixtures/jats/COVERAGE.md` gaps are still open (`TODO.md`). 2026-07-26 (tei architecture): `tei-fmt` standalone crate created (`crates/formats/tei-fmt`), mirroring `docbook-fmt`/`jats-fmt`'s generic-XML AST (`TeiDoc`/`Node` with `Element`/`Text`/`Cdata`/`Comment`/`ProcessingInstruction`/`Doctype`/`EntityRef`) since TEI is also plain well-nested XML: `parse()`, an independent SAX-style `events()`, an incrementally-draining `StreamingParser<H>`, `emit()` builder writer, and a streaming `Writer<W: Write>`. `rescribe-read-tei`/`rescribe-write-tei` rewired to thin AST↔IR translators (no `quick-xml` left in adapter production code); all prior construct mappings preserved with parity, plus one real fidelity bug fixed: the old hand-rolled reader captured `xml:id` and `n` attributes into a `FrameAttrs` struct on every element but never read either field back out when building IR nodes, so both were parsed and then silently discarded everywhere (dead-code capture, same family of bug as docbook's `xlink:href`). `xml:id` now round-trips as the standard `id` property; `n` round-trips as `tei:n`. Unresolvable named XML entities are now raw-preserved as `raw_inline`/`tei:entity` rather than silently dropped (the old reader had no entity handling at all, and also had no handling for `<!-- comments -->`/PIs, which are now surfaced as fidelity warnings instead of a bare silent drop). Stage numbers unchanged (R:4/W:2) — this pass is the architecture extraction only; fuzz targets and the remaining `fixtures/tei/COVERAGE.md` gaps (currently 31/117 items checked) are still open (`TODO.md`). 2026-07-26 (docbook/jats/tei fuzz targets added): six new fuzz targets wired into `fuzz/Cargo.toml` — `fuzz_{docbook,jats,tei}_fmt_reader` (no-panic gate: arbitrary bytes through `parse()` and `events()`) and `fuzz_{docbook,jats,tei}_fmt_roundtrip` (arbitrary-AST-first per CLAUDE.md: hand-rolled `Gen` builds an arbitrary `{DocBook,Jats,Tei}Doc` from fuzz bytes, `emit()`s it, `parse()`s it back, asserts `strip_spans()` equality — mirrors the `odf-fmt`/`djot-fmt` harness pattern, no `arbitrary` crate dependency needed since these are hand-rolled byte-driven generators). One generator bug found and fixed before any library bug could be reached: the attribute generator could emit two attributes with the same name on one element, which is invalid XML — quick-xml correctly reports it as a diagnostic, and the harness now suffixes attribute names with their index to guarantee uniqueness. All six targets ran clean for 60s each (0 crashes) after the fix: docbook reader 1.69M runs, docbook roundtrip 573K runs, jats reader 1.61M runs, jats roundtrip 553K runs, tei reader 1.59M runs, tei roundtrip 527K runs. No panics or roundtrip mismatches found in the three `-fmt` crates themselves. Stage numbers unchanged (R:4/W:2 each) — this is initial fuzz-target validation, not an exhaustive campaign; longer runs plus the fixture-suite/oracle-harness gaps noted above remain open in `TODO.md`. 2026-07-27 (tei fixture suite complete): `fixtures/tei/COVERAGE.md` closed from 31/117 to 117/117 (85 new fixtures across block, inline, teiHeader-metadata, property, integration/e2e, adversarial, and pathological categories) — vertical checklist step 1 (fixture-suite-complete) reached. Required real `rescribe-read-tei`/`rescribe-write-tei` changes, not just fixtures: ~35 new element mappings (drama/speech `sp`/`speaker`/`stage`, prefatory `epigraph`/`argument`, letter structure `dateline`/`salute`/`signed`, `castList`, `ab`, `gap`/`space`, deep div levels, list `type` variants/`label`) plus a generic `span`-tagged (`tei:tag=`) raw-preservation path covering the editorial-apparatus and named-entity inline vocabulary (`choice`/`abbr`/`expan`/`orig`/`reg`/`sic`/`corr`/`add`/`del`/`supplied`/`unclear`/`persName`/`placeName`/`orgName`/`name`/`date`/`title`/`num`/`measure`/`anchor`/`milestone`/`seg`/`w`/`pc`/`foreign`/`bibl`); `xml:lang`/`corresp`/`sameAs` generic attributes; `style:align` derived from alignment `rend` values; `<formula type="inline">` now correctly produces `math_inline`; teiHeader metadata extraction deepened to capture author/editor/publisher/idno/language/abstract/keywords/revisions (previously title-only) with full write-back. **Bug found and fixed**: the reader's catch-all `_ => None` arm silently unwrapped any unrecognized element into its parent, discarding the element identity with no warning — changed to raw-preserve as a tagged `span`; a matching catch-all fidelity warning was added for unrecognized teiHeader fields (previously scanned-and-discarded with zero signal). `cargo clippy -D warnings` and full test/fixture suite clean. Stage numbers unchanged (R:4/W:2) — fixture-suite completeness is step 1 of 5 in the vertical checklist; the oracle harness, a longer fuzz campaign, and two documented known limitations (teiHeader sub-structure beyond flat metadata; block-level unknown elements round-tripping with an extra `<p>` wrapper) remain open in `TODO.md` before 5-Production. 2026-07-27 (docbook bug fix): two silent-drop bugs closed, mirroring the tei fix. Unrecognized element names previously hit a catch-all `_ => None` that spliced the element's children straight into the parent, discarding the tag with no warning — `rescribe-read-docbook` gains `is_block_element()`/`generic_div`/`generic_span` (mirroring tei/html) so an unrecognized element now raw-preserves as a `docbook:tag`-tagged div or span instead. `docbook-fmt` gains `Node::Raw`/`emit_fragment()` (mirroring tei-fmt/html-fmt) so `<info>` front-matter fields beyond `title` (author, authorgroup, date, copyright, legalnotice, pubdate, releaseinfo, revhistory, revision, or any other unmodeled field) are now raw-captured verbatim as `{tag}_raw` metadata (plus a flattened `{tag}` text summary) instead of being silently dropped — generalized directly to the `{tag}_raw`/`is_modeled_header_field` pattern from the start (not the two-hardcoded-names intermediate step tei went through first). New fixtures: `adv-unknown-block-element`, `adv-unknown-inline-element`, `header-author`. `cargo clippy -D warnings` and full test/fixture suite clean. Stage numbers unchanged (R:4/W:2) — fixture-suite completeness, the oracle harness, and a longer fuzz campaign remain open in `TODO.md` before 5-Production. 2026-07-27 (jats bug fix): the same two silent-drop bugs closed, using docbook's fix (this same date) as the direct template rather than tei's superseded intermediate form. `jats-fmt` gains `Node::Raw`/`emit_fragment()`. `rescribe-read-jats` gains `is_block_element()`/`generic_div`/`generic_span` so an unrecognized element (e.g. `<statement>`, `<styled-content>`) now raw-preserves as a `jats:tag`-tagged div/span instead of vanishing with its children spliced into the parent. `<article-meta>`/`<journal-meta>` front-matter beyond `title`/`article-title` (contrib-group, pub-date, volume, issue, fpage, lpage, permissions, history, or any other unmodeled field) is now raw-captured verbatim as `{tag}_raw` metadata via the generalized `is_modeled_header_field` allow-list, with an explicit `<title-group>` pass-through arm added so the already-modeled title/journal-title reaches `extract_metadata` as a direct sibling instead of being buried inside a raw-captured blob. New fixtures: `adv-unknown-block-element`, `adv-unknown-inline-element`, `header-contrib-group`. `cargo clippy -D warnings` and full test/fixture suite clean. Stage numbers unchanged (R:4/W:2) — fixture-suite completeness, the oracle harness, and a longer fuzz campaign remain open in `TODO.md` before 5-Production. 2026-07-28 (commonmark-fmt construct features + default-backend silent-misparse fix): `commonmark-fmt` gains individual Cargo features per pulldown-cmark extension (`tables`, `task-lists`, `strikethrough`, `frontmatter`; `footnotes`/`definition-lists`/`math` reserved as inert names) plus `gfm`/`extensions` umbrella aliases — see `docs/adr/0011-commonmark-extension-feature-gating.md`. Fixes the actual production bug this was chasing: `rescribe_read_markdown::parse()` (the DEFAULT path, no backend override) was silently misparsing YAML front matter as a bogus `horizontal_rule` + setext `heading` in the document body (TOML front matter merged into a plain paragraph); both now populate `doc.metadata` correctly on the default path. `rescribe-fixtures/tests/run.rs`'s `markdown` test switched from `backend_pulldown::parse` to the default `rescribe_read_markdown::parse` (this is the change that turned the bug from silent to a failing fixture); a new `markdown_backends_agree` parity test compares node-kind shape between both backends per fixture. Two real, pre-existing bugs found via that parity test and fixed: (1) `commonmark-fmt`'s tight-list-item builder flushed accumulated leading inline text into an implicit paragraph only at `End(Item)`, landing it *after* a sibling nested-list block that had already been pushed mid-item (`- outer\n  - inner\n` reordered to list-then-paragraph instead of paragraph-then-list) — fixed by flushing at `push_block` time, using the flushed inlines' own span bounds rather than the item's. (2) the pulldown backend's `Options` never included `ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS`, so `backend_pulldown::parse` didn't support TOML front matter at all despite already having the `Tag::MetadataBlock(PlusesStyle)` handling arm — one-line fix. `footnotes`/`definition-lists`/`math` are explicitly deferred (feature names reserved, no implementation) per the task's own scoping; the markdown `footnote` fixture is excluded from the default-backend fixture run via a new `run_format_fixtures_excluding` helper, tracked in `TODO.md`. Writer-side symmetry: `rescribe-write-markdown`/`rescribe-write-commonmark` gained flat YAML front-matter emission from `doc.metadata` and task-list checkbox emission (write-commonmark); `rescribe-write-commonmark`'s pre-existing hand-rolled-emitter architecture violation (noted in the table below, row 252) was left as-is — out of scope for this pass. `commonmark`/`markdown` stage numbers unchanged pending a fuller reader/writer parity pass; this entry is the silent-misparse fix + feature-gating groundwork, not a stage promotion. 2026-07-28 (rst-fmt demotion, root cause found): confirmed and demoted the false 5-Production claim flagged in `TODO.md`'s 2026-07-28 ADR audit — `events.rs`/`batch.rs`/`writer.rs` were never wired into `lib.rs` and don't currently compile even once wired in, both because `crate::EventIter`/`crate::events()` no longer exist (deleted in commit `79ea2ce7af`, the same evening as the original sign-off, as collateral damage from an unrelated footnote-parsing refactor) and because `events.rs`/`writer.rs` reference a `Block::LineBlock` variant that a later commit replaced with `Block::Div{class:"line-block"}`. rst demoted R:5→R:4/W:5→W:2 (reader-ast and writer-builder are real and unaffected; reader-streaming/reader-batch/writer-streaming are non-functional despite the `Cargo.toml` features implying otherwise). Full findings in the "RST reader" section below and in `TODO.md`; re-implementing `events()`/`EventIter` as a genuine pull iterator (not a wiring fix) is left as follow-up work. 2026-07-28 (rst-fmt orphan-API recovery): all five required APIs are real and wired. Root-caused the merge commit precisely: `git log --follow` on `lib.rs` showed the `pub mod events/batch/writer` lines present at every commit up to `395a7ee532`, absent from `79ea2ce7af` onward — the actual damage is merge commit `383d4e6adf` (`Merge: 395a7ee532 79ea2ce7af`), which took the topic branch's entire `lib.rs` (1443 lines shorter) instead of merging it, discarding `mod` declarations, `EventIter`, and `Block::LineBlock` (already superseded by `Block::Div{class:"line-block"}` on mainline) as collateral. `events.rs`/`batch.rs`/`writer.rs` on disk were untouched by the bad merge and matched the pre-merge blobs exactly (`diff` confirmed byte-identical), so they were salvaged as reference material rather than rewritten from scratch — but `EventIter` itself (previously a ~1300-line duplicate of the whole recursive-descent grammar, living only in the deleted `lib.rs` region) was *not* resurrected uncritically: it is now a thin composition over the existing `Parser` (constructed via `Parser::new` + the same `collect_link_targets`/`collect_anonymous_targets`/`collect_substitutions` prescan `parse()` runs), so there is exactly one implementation of RST grammar, and `expand_block`/`expand_inline` lazily turn one already-parsed top-level `Block` into a `Vec<Frame>`-stack event sequence (`O(nesting depth)`, not `O(full document)`) — this both satisfies ADR 0003 ("parser IS the iterator, not `parse().collect()`") and removes ~1300 lines of duplicate parsing logic the old design carried. `writer.rs`'s `Writer` was rewritten outright: the salvaged version buffered the entire `Vec<Event>` and only built+emitted at `finish()` — exactly the "fake streaming, wraps the tree builder" pattern CLAUDE.md rejects — replaced with a frame stack that flushes each *top-level* block to the sink via `build_block` the moment its `End*` event arrives (`O(largest top-level block + nesting depth)`). `batch.rs`'s `StreamingParser` needed no rewrite: it already re-parses accumulated blank-line-delimited blocks through `events()` as they complete, genuinely `O(largest block)`. New tests: `events()`/`parse()` shape-equivalence (11 inputs spanning every `Block`/`Inline` variant, reduced to discriminant-only tag sequences so the comparison doesn't require identical Rust types); 6 chunked `StreamingParser` tests feeding input one byte at a time with awkward splits (mid-directive-keyword, mid-heading-underline, mid-UTF-8-char, mid-table-border, mid-footnote-continuation); a full-construct-mix round-trip through `Writer` (headings/lists/code/tables/definition-lists/footnotes/blockquote/transition). All 45 crate tests + 3 doctests pass; `cargo clippy --all-targets --all-features -D warnings` clean. Two *pre-existing, unrelated* `build_block` bugs were found (not fixed, out of scope for this pass, logged in `TODO.md`): admonition directives (`.. note::` etc.) lose their directive wrapper on write-back (the `Block::Div{class,..}` builder arm ignores `class`), and `FootnoteDef` emits only a single trailing `\n` instead of a blank-line separator, so a construct immediately following a footnote with ≥3-space indentation gets swallowed into the footnote body on re-parse — both reproduce with plain `crate::build()`, with no streaming API involved. Added `tests/no_orphan_modules.rs`: walks `src/` from `lib.rs`, follows every file-backed `mod name;` declaration, and fails if any `.rs` file is unreachable — verified it catches the class of bug that started this (confirmed cargo build succeeds silently with a genuinely unreferenced new file present; the test fails). A one-off workspace-wide sweep using the same logic found zero real orphans elsewhere (3 files flagged, all confirmed false positives from heuristic gaps: two crates with sibling `lib.rs`+`main.rs` in one `src/`, and one `#[path = "..."]` redirect in jats-fmt). rst promoted R:4→R:4 (unchanged number, now honest) / W:2→W:4: all API modes now real and unit-tested; not promoted to 5 because the existing `fuzz_rst_reader`/`fuzz_rst_roundtrip` targets only ever exercised `parse()`/`build()` via the `rescribe-read-rst`/`rescribe-write-rst` adapters — no fuzz target yet drives `events()`, `StreamingParser`, or `Writer` specifically, so the CLAUDE.md fuzz requirement for those three modes is still open (tracked in `TODO.md`). Also discovered (workspace-wide, out of scope to fix here): the central `rescribe-fixtures` harness (`crates/rescribe-fixtures/tests/run.rs`) tests every format crate via `parse()`/`emit()` only — no format's fixtures are wired through `events()`/`StreamingParser`/a streaming `Writer`, for any crate, not just rst. This is the same class of blind spot flagged in the markdown suite; logged in `TODO.md` as a cross-cutting gap, not an rst-specific one.

## Maturity Pipeline

```
0-Stub → 1-Partial → 2-Fixtures → 3-Harness → 4-Fuzz → 5-Production
```

| Stage | Meaning |
|-------|---------|
| **0-Stub** | Crate compiles; little or no real implementation |
| **1-Partial** | Handles common constructs; known gaps remain |
| **2-Fixtures** | Owned fixture suite authored and passing in CI |
| **3-Harness** | Oracle harness run; all differences understood and documented |
| **4-Fuzz** | No-panic + roundtrip fuzz targets exist and have been run clean |
| **5-Production** | Reader complete + writer complete + all API modes + fuzz clean + fixtures complete (see CLAUDE.md) |

**2026-07-28 status note:** CLAUDE.md's definition of 5-Production requires "100% construct
coverage." No format's construct list has been verified against a spec-derived source — see
the new `CC` (Construct Coverage) column below, `U` for every format. Existing `5` values in
this table are not being retracted (the API/fuzz/fixture-suite work is real), but should be
read as "5 on every dimension except construct-list completeness, which is unverified" until
`CC` is closed out.

**Conventions:**
`†` = library-backed (upstream provides correctness guarantee; wrapper still needs fixtures and fuzz).
`–` = not applicable (no crate exists, or stage is not meaningful for this format).
Stage 3 is marked `–` for formats Pandoc cannot read — their path skips directly from fixtures to fuzz.

## Construct Coverage (CC) — a separate, currently-unverified dimension

**Added 2026-07-28. Every format's `CC` value below is `U` (unverified) — no exceptions.**

The `R`/`W` stage numbers above measure API surface, fuzz cleanliness, and whether a
*hand-curated* fixture checklist (`fixtures/{format}/COVERAGE.md`) is fully checked off.
They do not, and never did, measure whether that checklist itself enumerates every
construct the format actually defines. `CC` is that missing measurement, tracked
separately rather than folded into `R`/`W` so the real, unquestioned work those numbers
represent (fuzz campaigns that genuinely ran, writers that genuinely exist, round-trips
that genuinely pass) isn't erased by a blanket demotion.

**Why every format is `U`, not just the ones touched most recently:** four findings from
the 2026-07-28 session, none of which are specific to any one format:

1. **Hand-written COVERAGE.md denominators proved badly wrong when checked.** An audit
   (commit `c2d6028c9a`) diffed `fixtures/docbook/COVERAGE.md` and `fixtures/jats/COVERAGE.md`
   against authoritative element indexes (DocBook 5.2's `tdg.docbook.org` reference, JATS
   1.3's Archiving Tag Library alpha-index) and found 265 DocBook and 216 JATS element names
   enumerated nowhere in either checklist. The denominators moved 94→105→117 (DocBook) and
   106→109→133 (JATS) across the session purely as gaps were noticed incidentally — a ratio
   over a list built this way is not a coverage measurement, for any format, hand-written the
   same way.
2. **The classifier-verification methodology used to "verify" block/inline classification was
   structurally incapable of finding omissions.** Commit `abd6dd447d` (DocBook's
   `is_block_element` "schema-verification pass") only re-checked elements *already on the
   list* against the spec — it never asked which block elements were absent from the list
   entirely. A later full re-check against DocBook's ~392-element index (commit `be578fb98c`)
   found 17 additional genuine misclassifications beyond the 4 the original audit had named.
   The identical method was then used on JATS (`20c27d032e`) and TEI (`3e3d84bcef`) in the
   same session, so those "N misclassifications found" results carry the same blind spot — a
   clean result from a presence-only check is not evidence of completeness. See
   `docs/adr/0004-xml-classifier-schema-verification-methodology.md`, amended 2026-07-28 to
   record this.
3. **Checkmarks only ever proved happy-path.** Per `fixtures/spec.md`, a fixture suite is
   measured across six dimensions (Block, Inline, Metadata, Properties, Integration/E2E,
   Adversarial, Pathological); a `[x]` in a COVERAGE.md's Block/Inline/Metadata/Properties
   section means one basic fixture exists for that construct name, while Adversarial and
   Pathological are covered globally by a handful of file-bottom fixtures, not per-construct.
   The checkmarks assert one dimension out of six.
4. **A fixture harness was found validating a code path users don't run.**
   `crates/rescribe-fixtures/tests/run.rs` called `backend_pulldown::parse` for the markdown
   suite while the default backend (`rescribe_read_markdown::parse`, what every real caller
   gets) silently misparsed front matter (fixed in `1574db80e8`). A green fixture suite is not
   evidence the default configuration works — the same category of gap (fixtures target a path
   nobody actually calls) is unverified for every other format's suite too.

**What would satisfy `CC` for a format:** a construct list generated or checked against a
machine-readable, spec-derived registry for that format (rather than typed by hand from
memory/typical-usage judgment), with every registry entry accounted for as modeled,
raw-preserved, or explicitly out-of-scope. A construct-registry ADR and pilot are being
designed separately (see `docs/adr/` — not yet landed as of this writing); `CC` moves from
`U` to `✓` per format only once that registry (or an equivalent spec-derived check) has
actually verified the format's construct list, not on a re-read of the existing hand-written
checklist.

**`CC` does not retroactively demote `R`/`W`.** A format can legitimately be `R:5†/W:5†/CC:U`
— its API/fuzz/fixture-suite work is real and done; its construct-list completeness is simply
not yet known. Per CLAUDE.md's own definition, "100% construct coverage" is part of what
5-Production means — `CC:U` is the flag that this specific part of every existing
5-Production claim in this document, `TODO.md`, and the `fixtures/*/COVERAGE.md` headers is
currently unverified, not that the claim is false.

---

## Format Table

### Markdown family

| Format | R | W | CC | Library | R-next | W-next |
|--------|---|---|----|---------|--------|--------|
| commonmark | 5† | 5† | U | pulldown-cmark | – | – |
| gfm | 5† | 5† | U | pulldown-cmark | – | – |
| markdown | 4† | 4† | U | pulldown-cmark | production | production |
| markdown-strict | 4† | 2† | U | pulldown-cmark | production | harness |
| multimarkdown | 4† | 2† | U | pulldown-cmark | production | harness |

### Lightweight markup

| Format | R | W | CC | Library | R-next | W-next |
|--------|---|---|----|---------|--------|--------|
| djot | 5 | 5 | U | djot-fmt | – | – |
| org | 5 | 5 | U | hand | – | – |
| rst | 4 | 4 | U | hand | fuzz events()/StreamingParser | fuzz Writer specifically |
| asciidoc | 5 | 5 | U | hand | – | – |
| textile | 5 | 5 | U | hand | – | – |
| muse | 5 | 5 | U | hand | – | – |
| t2t | 5 | 5 | U | hand | – | – |
| markua | 5 | 5 | U | hand | – | – |
| fountain | 5 | 5 | U | hand | – | – |
| typst | 1 | 2 | U | hand | partial→fixtures | harness |
| texinfo | 5 | 5 | U | hand | – | – |
| bbcode | 5 | 5 | U | hand | – | – |
| pod | 5 | 5 | U | hand | – | – |
| haddock | 5 | 5 | U | hand | – | – |
| ansi | 5 | 5 | U | hand | – | – |
| man | 5 | 5 | U | hand | – | – |

† Pandoc cannot read AsciiDoc (`--from asciidoc` unsupported); consider asciidoctor as alternate oracle.

### Wiki formats

| Format | R | W | CC | Library | R-next | W-next |
|--------|---|---|----|---------|--------|--------|
| mediawiki | 5 | 5 | U | hand | – | – |
| creole | 5 | 5 | U | hand | – | – |
| dokuwiki | 5 | 5 | U | hand | – | – |
| vimwiki | 5 | 5 | U | hand | – | – |
| zimwiki | 5 | 5 | U | hand | – | – |
| xwiki | 5 | 5 | U | hand | – | – |
| twiki | 5 | 5 | U | hand | – | – |
| tikiwiki | 5 | 5 | U | hand | – | – |
| jira | 5 | 5 | U | hand | – | – |

### Office / binary

| Format | R | W | CC | Library | R-next | W-next |
|--------|---|---|----|---------|--------|--------|
| docx | 5† | 5† | U | ooxml-wml | – | – |
| odt | 5 | 5 | U | odf-fmt (standalone) | – | – |
| epub | 5† | 5† | U | epub / epub-builder | – | – |
| fb2 | 5† | 5† | U | fb2-fmt | – | – |
| pptx | 5† | 5† | U | ooxml-pml | – | – |
| xlsx | 5† | 5† | U | ooxml-sml | – | – |
| pdf | 4† | – | U | pdf-extract | production | – |
| rtf | 5 | 5 | U | rtf-fmt (standalone) | – | – |
| mobi | – | – | – | – (planned) | – | – |
| azw3 | – | – | – | – (planned) | – | – |
| kfx | – | – | – | – (planned) | – | – |

### HTML and structured XML

| Format | R | W | CC | Library | R-next | W-next |
|--------|---|---|----|---------|--------|--------|
| html | 5† | 5† | U | html5ever / hand | – | – |
| docbook | 4 | 2 | U | quick-xml (docbook-fmt) | production | harness |
| jats | 4 | 2 | U | quick-xml (jats-fmt) | production | harness |
| tei | 4 | 2 | U | quick-xml (tei-fmt) | production | – |
| opml | 4 | 2 | U | hand | production | harness |
| ipynb | 4† | 2† | U | serde_json | production | harness |
| latex | 4 | 2 | U | hand | production | harness |

‡ Pandoc cannot read TEI (`--from tei` unsupported, output-only per
`pandoc --list-input-formats`); oracle-harness stage is N/A, same as AsciiDoc.

**docbook/jats/tei `CC:U` note:** these three are the formats the 2026-07-28 audit actually
touched — the ones with a *measured* denominator swing (94→105→117, 106→109→133) and a
*measured* classifier blind spot (17 additional DocBook misclassifications found on
re-check). Every other format's `CC:U` is the same unverified status for a different
reason: nobody has yet run the equivalent audit against it, not that the audit found it
clean.

### Bibliographic

| Format | R | W | CC | Library | R-next | W-next |
|--------|---|---|----|---------|--------|--------|
| bibtex | 4† | 2† | U | biblatex | production | harness |
| biblatex | 4† | 2† | U | biblatex | production | harness |
| csl-json | 4† | 2† | U | serde_json | production | harness |
| ris | 4 | 4 | U | hand | – (harness N/A) | production |
| endnotexml | 4 | 2 | U | hand | – (harness N/A) | fuzz |

### Data / interchange

| Format | R | W | CC | Library | R-next | W-next |
|--------|---|---|----|---------|--------|--------|
| csv | 4 | 4 | U | hand | – (harness N/A) | production |
| tsv | 4 | 4 | U | hand | – (harness N/A) | production |
| pandoc-json | 4† | 3† | U | serde_json | production | fuzz |
| native | 4 | 2 | U | hand | production | harness |

### Presentation / output-only

These formats have no reader; stage 3 (harness) is not applicable. They also have no
`fixtures/{format}/COVERAGE.md` construct checklist to begin with (no fixture suite exists
in the same sense as reader-bearing formats), so `CC` is `–` (not applicable) rather than
`U` (unverified) — there is no existing claim to qualify.

| Format | W | CC | Library | W-next |
|--------|---|----|---------|--------|
| beamer | 4 | – | hand | coverage |
| revealjs | 4 | – | hand | coverage |
| slidy | 4 | – | hand | coverage |
| s5 | 4 | – | hand | coverage |
| dzslides | 4 | – | hand | coverage |
| slideous | 4 | – | hand | coverage |
| context | 4 | – | hand | coverage |
| ms | 4 | – | hand | coverage |
| icml | 4 | – | hand | coverage |
| chunkedhtml | 4 | – | hand | coverage |
| plaintext | 4 | – | hand | coverage |

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
| rst-fmt | ✓ | ✓ | ✓ | ✓ | ✓ |
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

*(stale — superseded by the full inventory below; odt/fb2/docbook/jats/tei/native now have crates.)*

---

## Adapter parsing/emitting-logic inventory (audited 2026-07-28)

Full sweep of all 54 reader crates and 64 writer crates against CLAUDE.md's rule
"the adapter layer must never contain parsing or writing logic". Method: read the
non-`#[cfg(test)]`, non-`[[bin]]` functions in each adapter's `src/*.rs` and check
whether they call a `crates/formats/` crate or do the byte-level work themselves.
`Cargo.toml` was treated as a weak signal only (verified per CLAUDE.md).

**65 formats audited: 38 clean, 14 violating, 13 uncertain.**

This section supersedes the "Formats still needing a standalone crate" line above and
is the single inventory for this dimension; `TODO.md`'s DEBT section links here rather
than repeating it.

### Clean (38) — adapter is a thin AST↔IR translator on both sides

Backed by a repo-local `crates/formats/` crate: org, rst, asciidoc, textile, muse, t2t,
markua, fountain, texinfo, bbcode, pod, haddock, man, mediawiki, creole, dokuwiki,
vimwiki, zimwiki, xwiki, twiki, tikiwiki, jira, docx (`ooxml-wml`), pptx (`ooxml-pml`),
xlsx (`ooxml-sml`), odt (`odf-fmt`), fb2 (`fb2-fmt`), docbook (`docbook-fmt`),
jats (`jats-fmt`), tei (`tei-fmt`), html (`html-fmt`), rtf (`rtf-fmt`), ris (`ris`),
csv (`csv-fmt`), tsv (`tsv-fmt`), native (`native`).

Backed by a sanctioned third-party library with no hand-rolled logic layered on top:
epub (`epub` / `epub-builder`), pdf (`pdf-extract`, read-only).

Residual `zip::` usage in `rescribe-read-odt` and `rescribe-read-pptx` is confined to
`#[cfg(test)]` fixture builders — verified, not a violation.

### Violating (14)

`-fmt?` = a usable standalone crate exists in `crates/formats/`. `R`/`W` = reader /
writer adapter contains parsing / emitting logic. Tier is eyeballed from adapter line
count and the fraction that is format syntax rather than AST↔IR translation.

| Format | -fmt? | R | W | Tier | Notes |
|--------|-------|---|---|------|-------|
| commonmark | yes | no | **yes** | large | **PARTIAL MIGRATION** — reader uses `commonmark_fmt::parse`; writer (276 ln) hand-rolls markdown and never calls the existing `commonmark_fmt::emit` |
| djot | yes | no | **yes** | large | **PARTIAL MIGRATION** — reader uses `djot_fmt`; writer (641 ln) hand-rolls, `djot_fmt::emit` unused, crate not even a dependency |
| ansi | yes | no | **yes** | large | **PARTIAL MIGRATION** — `ansi-fmt` *is* a writer dependency but deliberately bypassed; the file's own doc comment says "Does not go through the ansi-fmt AST — sequences are emitted directly" |
| markdown | yes | no | **yes** | large | reader dispatches to `commonmark_fmt`/`pulldown_cmark`; writer (1606 ln) fully hand-rolled |
| gfm | (pulldown) | no | **yes** | large | reader walks `pulldown_cmark` events (sanctioned); writer (350 ln) hand-rolled with no backing crate |
| markdown-strict | (pulldown) | no | **yes** | large | same shape as gfm; writer 377 ln |
| multimarkdown | (pulldown) | no | **yes** | large | same shape as gfm; writer 552 ln |
| typst | no | no | **yes** | large | reader is thin over third-party `typst-syntax`; writer (508 ln) hand-rolls Typst markup with no emit crate to delegate to |
| latex | **no** | **yes** | **yes** | large | worst case: `handwritten.rs` (895 ln) is a full recursive-descent LaTeX parser *inside the reader adapter*, plus a 662-ln tree-sitter backend; writer `builder.rs` (717 ln) is a hand-written emitter |
| opml | **no** | **yes** | **yes** | small–medium | `quick_xml::Reader` / `quick_xml::Writer` driven directly in production code; reader 307 ln / writer 278 ln, essentially all of it |
| endnotexml | **no** | **yes** | **yes** | large | `quick_xml::Reader`/`Writer` plus a hand-rolled generic-XML tree walker; 722 / 967 ln |
| bibtex | **no** | **yes** | **yes** | medium | reader calls third-party `biblatex::Bibliography::parse` directly; writer (643 ln) hand-rolls BibTeX syntax + escaping |
| biblatex | **no** | **yes** | **yes** | medium | identical shape to bibtex |
| csl-json | **no** | **yes** | **yes** | large | the CSL-JSON *schema* (`CslItem`/`CslName`/`CslDate`) lives in the adapter, both sides, over raw `serde_json` |

The three PARTIAL MIGRATION cases (commonmark, djot, ansi) are the highest-signal
finding: from `Cargo.toml` alone the vertical looks migrated, but the writer half never
was. `djot`'s writer does not even declare the dependency.

### Uncertain (13) — recorded rather than forced to yes/no

**JSON-schema-in-adapter class (2): `pandoc-json`, `ipynb`.** Neither has a standalone
crate; both define the format's schema structs in the adapter and go through
`serde_json`, which does the actual byte-level tokenizing. By the literal test (no
`quick_xml`/`zip`/`regex`/hand-rolled state machine) they pass; by the reasoning already
applied to `csl-json` in `TODO.md` — that owning the schema *is* owning the format
knowledge — they fail. They are the same class as `csl-json`, so whatever call stands
for `csl-json` should apply to both. `pandoc-json`'s reader additionally walks raw
`serde_json::Value` block/inline trees (805 ln), which leans further toward violation
than `ipynb` does.

**Output-only rendering targets (11):** beamer, revealjs, slidy, s5, dzslides, slideous,
context, ms, icml, chunkedhtml, plaintext. All 11 hand-emit their target syntax (LaTeX,
HTML, troff/ms macros, ICML XML, plain text) via `write!`/`push_str`, and none depends on
any `crates/formats/` crate or reuses `rescribe-write-html` / `rescribe-write-latex` —
there is no cross-writer reuse at all. By the letter of the rule this is emitting logic
in an adapter. The open question is scope: the rule is framed around a *native AST* that
a reader and writer both round-trip, and these formats have no reader and no round-trip
consumer, so whether a `beamer-fmt` crate would serve any real ecosystem user is a
judgment call for a human. Recorded as uncertain, not as a verdict. `plaintext` is the
weakest case (no escaping or structural rules at all — arguably light string
manipulation). Sizes: beamer/slideous/chunkedhtml 250–435 ln, revealjs/context/ms/icml
185–330 ln, slidy/s5/dzslides 185–226 ln.

### Formats with no standalone crate at all

latex, opml, endnotexml, bibtex, biblatex, csl-json, pandoc-json, ipynb, typst (writer
side), and the 11 output-only targets.

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

### odf-fmt — 5-Production (2026-04-10)
- Standalone `odf-fmt` crate covering ODT/ODS/ODP; no rescribe dependency
- Full AST: TextBlock, Inline, SpreadsheetBody, PresentationBody, styles, metadata
- API modes: parse(), events(), emit(), batch::BatchParser, batch::Writer
- Fixture suite complete: 30 fixtures, all COVERAGE.md boxes checked
- Fuzz targets: fuzz_odf_fmt_reader (no-panic) + fuzz_odf_fmt_roundtrip (AST roundtrip)
- ADR-001 documents unified-crate decision (vs per-application-type split)

### RST reader — 5-Production (2026-03-22)
- Pandoc harness: 100% word coverage on rst-reader.rst (ref=618, ours=668)
- fuzz_rst_reader: 201K runs clean; fuzz_rst_roundtrip: 467K runs clean (2026-03-22)
- Parser fixes: "text::" introductory paragraph now emitted before code block (pending_block
  pattern); `<url>`_ empty link text uses URL as display text; pending_block loop in main
  parse avoids losing blocks at EOF
- Fixtures: 80 total; COVERAGE.md all boxes checked
- Benchmarks: rst_parse_small 3.3µs, rst_parse_medium 30µs, rst_emit_medium 2.5µs

**2026-07-28 correction — rst-fmt demoted from R:5/W:5 to R:4/W:2, root cause found.**
`crates/formats/rst-fmt/src/events.rs`, `batch.rs`, and `writer.rs` exist on disk but have
never had `mod` declarations in `lib.rs`, so they have never been compiled — confirmed by
attempting to wire them in (`pub mod events; pub mod batch; pub mod writer;`, mirroring
`docbook-fmt`'s unconditional-mod-declaration pattern) and running `cargo build -p rst-fmt
--all-features`. Root cause identified via `git log --follow` bisection: commit
`79ea2ce7af` (2026-03-29, same evening as the 5-Production sign-off), while fixing
multi-line footnote continuation, replaced the `EventIter` struct (which implemented
`Iterator` and was the actual engine behind `pub fn events()`) with a plain non-iterator
`Parser` struct, and deleted the `mod events;`/`mod batch;`/`mod writer;` declarations and
the `pub fn events()` free function — collateral damage from an unrelated parser refactor,
not a deliberate architecture decision. The three orphaned files still reference the
deleted `crate::EventIter`/`crate::events()` API and additionally now diverge from the
current AST: `events.rs` and `writer.rs` both match/construct `Block::LineBlock { lines }`,
a variant that no longer exists on `Block` — line blocks are represented as
`Block::Div { class: Some("line-block"), .. }` since a later commit. Reconstructing a
working `events()`/`EventIter` is not a wiring fix: it requires re-deriving a genuine
lazy pull-iterator state machine (frame stack for block/inline nesting, per ADR 0003's
"parser IS the iterator" requirement) that covers strictly more constructs than the
deleted implementation ever did (grid/simple tables and footnote parsing were both added
*after* the Iterator impl was removed, so the old code is not a valid starting point
either). This was judged out of scope for a wiring pass per this document's own vertical-
completion rules — see `TODO.md` for the full writeup and required follow-up.
Verified-working today: `parse()` (reader-ast, unaffected) and `crate::build()`
(writer-builder, unaffected — `writer.rs`'s `Writer` type was never the code path
`rescribe-write-rst` or `build()` used). `reader-streaming`, `reader-batch`, and
`writer-streaming` are all non-functional despite `Cargo.toml` declaring those features
on by default; `cargo build -p rst-fmt --all-features` fails once the orphaned modules are
wired in, for both reasons above.

**2026-07-28 recovery — all five APIs real and tested; rst is R:4/W:4 (not yet 5).**
Root cause pinned exactly: not `79ea2ce7af` itself but merge commit `383d4e6adf`
(`Merge: 395a7ee532 79ea2ce7af`), which took the `79ea2ce7af` topic branch's entire
`lib.rs` in place of a real merge — losing 1443 lines mainline had gained in parallel
(`mod` declarations, `EventIter`, `Block::LineBlock`→already-superseded-by-`Div`). Fixed:
- `events()`/`EventIter`: rebuilt as a thin wrapper composing the existing `Parser` (not
  a resurrection of the old ~1300-line duplicate grammar) — `expand_block`/`expand_inline`
  lazily convert one already-parsed top-level `Block` into a `Vec<Frame>` stack,
  `O(nesting depth)` to traverse. Satisfies ADR 0003: `parse()` and `events()` share the
  parser, `events()` is not `parse().collect()`.
- `batch::StreamingParser`: needed no rewrite — already re-parses each accumulated
  blank-line-delimited block through `events()`, `O(largest block)`, not `O(full input)`.
- `writer::Writer`: rewritten. The salvaged version buffered the whole `Vec<Event>` and
  only built+emitted RST text at `finish()` — the "fake streaming, wraps the tree
  builder" pattern CLAUDE.md explicitly rejects. Now a frame stack that flushes each
  completed *top-level* block to the sink immediately via the shared `build_block`,
  `O(largest top-level block + nesting depth)`.
- Tests added: `events()`≡`parse()` shape equivalence (discriminant-tag comparison,
  type-agnostic, across 11 inputs covering every `Block`/`Inline` variant); 6
  `StreamingParser` tests feeding input one byte at a time with awkward splits
  (mid-keyword, mid-underline, mid-UTF-8-char, mid-table-border, mid-footnote-body);
  one full-construct-mix round-trip through `Writer`. 45 unit tests + 3 doctests pass;
  `cargo clippy --all-targets --all-features -D warnings` clean.
- `tests/no_orphan_modules.rs` added: walks `src/` from `lib.rs` over `mod` declarations,
  fails if any `.rs` file is unreachable — confirmed it catches exactly this bug class
  (`cargo build` stays green with a genuinely-orphaned new file present; this test fails).
  A workspace-wide sweep with the same logic found no real orphans elsewhere (3 files
  flagged were heuristic false positives: sibling `lib.rs`+`main.rs` crates, one
  `#[path=...]` redirect).
- Two **pre-existing, unrelated** `build_block` bugs found (not fixed — out of scope,
  logged in `TODO.md`): admonition directives lose their `.. note::`-style wrapper on
  write-back (`Block::Div{class,..}`'s builder arm ignores `class`); `FootnoteDef` emits
  only a single `\n` instead of a blank-line separator, so a following ≥3-space-indented
  block gets swallowed into the footnote body on re-parse. Both reproduce with plain
  `crate::build()` — no streaming API involved.
- Not promoted to 5: `fuzz_rst_reader`/`fuzz_rst_roundtrip` only ever drove `parse()`/
  `build()` via the adapters; no fuzz target exercises `events()`, `StreamingParser`, or
  `Writer` specifically. That is the concrete remaining gap before re-claiming
  5-Production (tracked in `TODO.md`).
- Also found, workspace-wide, out of scope to fix here: `crates/rescribe-fixtures`
  tests every format via `parse()`/`emit()` only, for every crate — no format's fixtures
  exercise `events()`/`StreamingParser`/a streaming writer. Not rst-specific; logged in
  `TODO.md`.

### Djot reader/writer — 5-Production (2026-03-23)
- Pandoc harness: 100% word coverage on djot-reader.djot (ref=931, ours=930); single missing
  word "html" is in a raw HTML block (Pandoc parses raw content text; we preserve verbatim — correct)
- Reader fix: `push_text` now merges adjacent text nodes — jotdown emits smart-quote events
  (RightSingleQuote etc.) as separate events between Str events; merging prevents "Here's" from
  being split into ["Here", "\u{2019}", "s..."] and losing the joined word from harness coverage
- Fixtures: 79 total; COVERAGE.md all boxes checked (60 new fixtures added 2026-03-22)
- Benchmarks: djot_parse_small 7.8µs, djot_parse_medium 49µs, djot_emit_medium 9.8µs
- fuzz_djot_roundtrip rewritten to correct direction: arbitrary rescribe doc → emit → parse
  (old direction was parse(bytes) → emit → parse, which is vacuous if reader drops constructs)
- New fuzz target uses FuzzBlock/FuzzInline pattern (same as rst_roundtrip, asciidoc_roundtrip)
- Sanitiser strips Djot markup chars: `*`, `_`, `#`, `-`, `.`, `)`, `+`, `^`, `~`, `[`, `]`,
  `{`, `}`, `\`, `$`, `<`, `>`, `'`, `"`, `|` — prevents roundtrip failures from inline/block
  marker reinterpretation. Code inlines excluded: adjacent code spans produce ```` `` ````
  delimiters that jotdown re-parses as a 2-backtick verbatim span (TODO: fix writer).
- 1,005,513 fuzz runs clean (300s, 2026-03-21)
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
