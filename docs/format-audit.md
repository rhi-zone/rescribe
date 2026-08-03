# Format Implementation Audit

Assessed 2026-02-24; stages updated 2026-03-21 (wiki formats 2→4; csv/tsv/ris/texinfo 2→4; mediawiki 3→4; odt/fb2/docbook/jats/opml/tei 3→4; commonmark/gfm/markdown-strict/multimarkdown 3→4; pulldown-cmark upgraded to 0.13; beamer/revealjs/slidy/s5/dzslides/slideous/context/ms/icml/chunkedhtml/plaintext writers 2→4); RST/Org/AsciiDoc writer APIs added 2026-03-23 (streaming + builder); 2026-03-29: definition of 5-Production tightened — reader-only no longer qualifies; RST/Org/AsciiDoc demoted from R:5 to R:4 due to construct gaps (tables, footnotes); writer column updated from 2→4 (API modes complete, fuzz clean, construct gaps remain). djot-fmt + textile-fmt signed off at 5-Production (2026-03-29). RST/AsciiDoc/Org signed off at 5-Production (2026-03-29; all construct gaps closed: tables, footnotes, math, nested blockquotes, figure/caption). 2026-03-30: muse/t2t/man/markua/creole/dokuwiki/vimwiki/zimwiki/xwiki/twiki/tikiwiki/jira/mediawiki all completed to R:4/W:4; fountain/texinfo/bbcode/pod/haddock/ansi same (all constructs + API modes + fixtures; need fuzz re-run). 2026-04-10: commonmark/gfm writers promoted W:3→W:5 (fuzz_commonmark_reader 284K runs clean, fuzz_commonmark_roundtrip 197K runs clean; all writer API modes already implemented). 2026-03-31: all 44 fuzz targets (22 format pairs) ran clean — 12 fuzz failures found and fixed (djot-fmt char/byte panics, sanitiser gaps across textile/twiki/muse/mediawiki/haddock/t2t/markua); all 19 R:4/W:4 formats promoted to 5-Production. 2026-04-10: odf-fmt signed off at 5-Production (ODS/ODP full AST support, complete fixture suite, batch API, streaming writer, fuzz targets wired). docx writer promoted W:4→W:5 (fuzz_docx_reader 3.47M runs clean, fuzz_docx_roundtrip 119K runs clean; all construct coverage and API modes already complete). epub promoted R:4→R:5/W:3→W:5: 9 new fixtures (figure-with-caption, definition-list, section-div, span-style, cross-document-link, metadata-extended, adv-invalid-xhtml, adv-empty-spine, path-many-chapters); fuzz_epub_reader and fuzz_epub_roundtrip both run clean (300s each, 189K roundtrip runs); library limitations documented in COVERAGE.md. 2026-04-10: odf-fmt fuzz confirmed clean (fuzz_odf_fmt_reader 1.95M runs, fuzz_odf_fmt_roundtrip 124K runs). fb2 promoted W:2→W:4: roundtrip fuzz target added, 10 new fixtures (epigraph, empty-line, subtitle, author-metadata, lang-metadata, genre-metadata, internal-link, adv-malformed, adv-entity-refs, adv-empty-section), reader fixes (entity decoding via GeneralRef events, metadata extraction for lang/genre/keywords, section id preservation, metadata container children now discarded to prevent leakage), writer fixes (epigraph/poem/stanza/text-author detected from IR props); fuzz_fb2_reader 6.3M runs clean, fuzz_fb2_roundtrip clean. html promoted W:3→W:4: fuzz_html_reader 1.73M runs clean, fuzz_html_roundtrip 1.21M runs clean. 2026-04-10 (html 5†): 44 new fixtures (82/85 COVERAGE.md items checked); semantic HTML5 elements (section/article/aside/nav/header/footer/address/details/summary) preserved as div with html:tag prop; global attributes (lang/dir/style/id/class) propagated to all block/inline nodes; <ins> separated from <u>; abbr/mark/kbd/var/samp/cite added as span{html:tag}; colgroup/col silently stripped; extract_metadata extended (html@lang, meta@charset, link@stylesheet, base@href); writer respects html:tag on div/span for lossless re-emission; fixture runner added. 3 items deferred: footnote anchor convention (requires tree-level pattern detection), inline MathML (separate embedded language), multi-megabyte pathological test (file size). html R:4†/W:4† (fuzz already clean; fixture coverage 82/85; remaining 3 items block 5-Production). 2026-04-10 (architecture): fb2-fmt standalone crate created; rescribe-read-fb2/rescribe-write-fb2 now thin adapters (no quick-xml/base64 in adapter deps); fb2-fmt events() pull iterator and StreamingParser<H> implemented; fb2 fixture suite advanced to 47/63 items (19 new: date, keywords, translator, src-lang, series-sequence, cover-image, publisher-info, document-info, custom-info, image-alt-text, xml-lang-body, inline-image, poem-epigraph, adv-missing-xmlns, adv-broken-image-ref, adv-numeric-charref, deeply-nested-sections, many-paragraphs, table-many-cells); remaining 16 items require footnote/binary infrastructure. rescribe-read-odt/rescribe-write-odt rewritten to use odf_fmt::parse()/emit() — adapter no longer calls quick-xml/zip directly; odf-fmt bug fixed (self-closing style:text-properties was consuming office:body). parse_numbering_order() moved from rescribe-read-docx into ooxml-wml::numbering; quick-xml removed from docx adapter deps. 2026-04-10 (fb2 5†): fb2-fmt footnote support (FootnoteRef AST node, notes body parsing/emitting), streaming writer (Writer<W: Write>), binary embedding fixtures, 6 additional coverage fixtures (annotation, link-title, table-alignment, adv-invalid-base64, adv-broken-footnote-ref, pathological-large-binary) — COVERAGE.md 63/63 checked; fuzz_fb2_reader 644K runs clean, fuzz_fb2_roundtrip 5.98M runs clean (1 crash found and fixed: <code> content was trimmed, dropping leading whitespace); fb2 R:5†/W:5†. odf-fmt: 12 new constructs added (SoftHyphen, Bookmark, Annotation, font_variant, user_defined meta, page-layout props, list ordering); all ODT fixture regressions from odt rewrite resolved. 2026-07-26 (html 5†): closed the 3 remaining gaps blocking html 5-Production. Footnote anchor convention: reader had zero footnote recognition (write-only before this); now recognizes `<sup class="footnote-ref"><a href="#fn-{label}">`/`<div id="fn-{label}" class="footnote"><sup class="footnote-label">…<span class="footnote-content">…</span><a class="footnote-back">` and reconstructs footnote_ref/footnote_def losslessly (marker/backlink are regenerated from the label, not read back, so only the content span needs to round-trip). Inline MathML: added html-fmt::emit_fragment (general-purpose subtree serializer, not adapter-specific) and reader support for `<math>…</math>` — raw-preserved verbatim as math_inline/math_display with `math:format="mathml"` + `math:source` holding the exact MathML markup (full structural modeling into math:* nodes deferred as out of scope per CLAUDE.md's raw-preservation pattern); writer now branches on math:format so MathML round-trips byte-for-byte while LaTeX math:source keeps the existing \\(…\\)/\\[…\\] convention. Added path-large-inline-text pathological fixture (~4.9MB single text node). fixtures/html/COVERAGE.md now 85/85; 8 new/updated unit tests (round-trip assertions) + 3 new fixture dirs (footnote, inline-math-mathml, path-large-inline-text). html R:5†/W:5†. 2026-07-26 (docbook architecture): `docbook-fmt` standalone crate created (`crates/formats/docbook-fmt`) wrapping quick-xml, with its own `DocBookDoc`/`Node` AST (`Element`/`Text`/`Cdata`/`Comment`/`ProcessingInstruction`/`Doctype`/`EntityRef`), `parse()`, a genuinely independent SAX-style `events()` (XML's well-nestedness means — unlike HTML5 — no tree needs to be built first), an incrementally-draining `StreamingParser<H>` (dispatches events as soon as provably complete, buffer bounded by the largest in-progress token; verified with chunk-boundary-split tests for both text and tags), `emit()` builder writer, and a streaming `Writer<W: Write>`. `rescribe-read-docbook`/`rescribe-write-docbook` rewired to thin AST↔IR translators (no `quick-xml` left in adapter production code); all prior construct mappings preserved with parity, plus one incidental bug fix (`xlink:href` link-attribute matching was dead code in the old adapter — it stripped namespace prefixes before comparing against the literal string `"xlink:href"`, so it could never match; `docbook-fmt` preserves the raw prefixed attribute name, so it now works). Unresolvable named XML entities (DTD-defined, not one of the 5 predefined or a numeric char ref) are now raw-preserved as `raw_inline`/`docbook:entity` rather than silently dropped. Stage numbers unchanged (R:4/W:2) — this pass is the architecture extraction only; fuzz targets and the oracle harness run are still open (`TODO.md`). 2026-07-26 (jats architecture): `jats-fmt` standalone crate created (`crates/formats/jats-fmt`), mirroring `docbook-fmt`'s generic-XML AST (`JatsDoc`/`Node` with `Element`/`Text`/`Cdata`/`Comment`/`ProcessingInstruction`/`Doctype`/`EntityRef`) since JATS is also plain well-nested XML: `parse()`, an independent SAX-style `events()`, an incrementally-draining `StreamingParser<H>`, `emit()` builder writer, and a streaming `Writer<W: Write>`. `rescribe-read-jats`/`rescribe-write-jats` rewired to thin AST↔IR translators (no `quick-xml` left in adapter production code); all prior construct mappings preserved with parity, plus one incidental fidelity fix (`<xref ref-type="…">` now preserves `jats:ref-type` for both the self-closing and full-element shapes — the old hand-rolled reader only attached it for the self-closing case). Unresolvable named XML entities are now raw-preserved as `raw_inline`/`jats:entity` rather than silently dropped (the old reader had no entity handling at all). Stage numbers unchanged (R:4/W:2) — this pass is the architecture extraction only; fuzz targets and the remaining `fixtures/jats/COVERAGE.md` gaps are still open (`TODO.md`). 2026-07-26 (tei architecture): `tei-fmt` standalone crate created (`crates/formats/tei-fmt`), mirroring `docbook-fmt`/`jats-fmt`'s generic-XML AST (`TeiDoc`/`Node` with `Element`/`Text`/`Cdata`/`Comment`/`ProcessingInstruction`/`Doctype`/`EntityRef`) since TEI is also plain well-nested XML: `parse()`, an independent SAX-style `events()`, an incrementally-draining `StreamingParser<H>`, `emit()` builder writer, and a streaming `Writer<W: Write>`. `rescribe-read-tei`/`rescribe-write-tei` rewired to thin AST↔IR translators (no `quick-xml` left in adapter production code); all prior construct mappings preserved with parity, plus one real fidelity bug fixed: the old hand-rolled reader captured `xml:id` and `n` attributes into a `FrameAttrs` struct on every element but never read either field back out when building IR nodes, so both were parsed and then silently discarded everywhere (dead-code capture, same family of bug as docbook's `xlink:href`). `xml:id` now round-trips as the standard `id` property; `n` round-trips as `tei:n`. Unresolvable named XML entities are now raw-preserved as `raw_inline`/`tei:entity` rather than silently dropped (the old reader had no entity handling at all, and also had no handling for `<!-- comments -->`/PIs, which are now surfaced as fidelity warnings instead of a bare silent drop). Stage numbers unchanged (R:4/W:2) — this pass is the architecture extraction only; fuzz targets and the remaining `fixtures/tei/COVERAGE.md` gaps (currently 31/117 items checked) are still open (`TODO.md`). 2026-07-26 (docbook/jats/tei fuzz targets added): six new fuzz targets wired into `fuzz/Cargo.toml` — `fuzz_{docbook,jats,tei}_fmt_reader` (no-panic gate: arbitrary bytes through `parse()` and `events()`) and `fuzz_{docbook,jats,tei}_fmt_roundtrip` (arbitrary-AST-first per CLAUDE.md: hand-rolled `Gen` builds an arbitrary `{DocBook,Jats,Tei}Doc` from fuzz bytes, `emit()`s it, `parse()`s it back, asserts `strip_spans()` equality — mirrors the `odf-fmt`/`djot-fmt` harness pattern, no `arbitrary` crate dependency needed since these are hand-rolled byte-driven generators). One generator bug found and fixed before any library bug could be reached: the attribute generator could emit two attributes with the same name on one element, which is invalid XML — quick-xml correctly reports it as a diagnostic, and the harness now suffixes attribute names with their index to guarantee uniqueness. All six targets ran clean for 60s each (0 crashes) after the fix: docbook reader 1.69M runs, docbook roundtrip 573K runs, jats reader 1.61M runs, jats roundtrip 553K runs, tei reader 1.59M runs, tei roundtrip 527K runs. No panics or roundtrip mismatches found in the three `-fmt` crates themselves. Stage numbers unchanged (R:4/W:2 each) — this is initial fuzz-target validation, not an exhaustive campaign; longer runs plus the fixture-suite/oracle-harness gaps noted above remain open in `TODO.md`. 2026-07-27 (tei fixture suite complete): `fixtures/tei/COVERAGE.md` closed from 31/117 to 117/117 (85 new fixtures across block, inline, teiHeader-metadata, property, integration/e2e, adversarial, and pathological categories) — vertical checklist step 1 (fixture-suite-complete) reached. Required real `rescribe-read-tei`/`rescribe-write-tei` changes, not just fixtures: ~35 new element mappings (drama/speech `sp`/`speaker`/`stage`, prefatory `epigraph`/`argument`, letter structure `dateline`/`salute`/`signed`, `castList`, `ab`, `gap`/`space`, deep div levels, list `type` variants/`label`) plus a generic `span`-tagged (`tei:tag=`) raw-preservation path covering the editorial-apparatus and named-entity inline vocabulary (`choice`/`abbr`/`expan`/`orig`/`reg`/`sic`/`corr`/`add`/`del`/`supplied`/`unclear`/`persName`/`placeName`/`orgName`/`name`/`date`/`title`/`num`/`measure`/`anchor`/`milestone`/`seg`/`w`/`pc`/`foreign`/`bibl`); `xml:lang`/`corresp`/`sameAs` generic attributes; `style:align` derived from alignment `rend` values; `<formula type="inline">` now correctly produces `math_inline`; teiHeader metadata extraction deepened to capture author/editor/publisher/idno/language/abstract/keywords/revisions (previously title-only) with full write-back. **Bug found and fixed**: the reader's catch-all `_ => None` arm silently unwrapped any unrecognized element into its parent, discarding the element identity with no warning — changed to raw-preserve as a tagged `span`; a matching catch-all fidelity warning was added for unrecognized teiHeader fields (previously scanned-and-discarded with zero signal). `cargo clippy -D warnings` and full test/fixture suite clean. Stage numbers unchanged (R:4/W:2) — fixture-suite completeness is step 1 of 5 in the vertical checklist; the oracle harness, a longer fuzz campaign, and two documented known limitations (teiHeader sub-structure beyond flat metadata; block-level unknown elements round-tripping with an extra `<p>` wrapper) remain open in `TODO.md` before 5-Production. 2026-07-27 (docbook bug fix): two silent-drop bugs closed, mirroring the tei fix. Unrecognized element names previously hit a catch-all `_ => None` that spliced the element's children straight into the parent, discarding the tag with no warning — `rescribe-read-docbook` gains `is_block_element()`/`generic_div`/`generic_span` (mirroring tei/html) so an unrecognized element now raw-preserves as a `docbook:tag`-tagged div or span instead. `docbook-fmt` gains `Node::Raw`/`emit_fragment()` (mirroring tei-fmt/html-fmt) so `<info>` front-matter fields beyond `title` (author, authorgroup, date, copyright, legalnotice, pubdate, releaseinfo, revhistory, revision, or any other unmodeled field) are now raw-captured verbatim as `{tag}_raw` metadata (plus a flattened `{tag}` text summary) instead of being silently dropped — generalized directly to the `{tag}_raw`/`is_modeled_header_field` pattern from the start (not the two-hardcoded-names intermediate step tei went through first). New fixtures: `adv-unknown-block-element`, `adv-unknown-inline-element`, `header-author`. `cargo clippy -D warnings` and full test/fixture suite clean. Stage numbers unchanged (R:4/W:2) — fixture-suite completeness, the oracle harness, and a longer fuzz campaign remain open in `TODO.md` before 5-Production. 2026-07-27 (jats bug fix): the same two silent-drop bugs closed, using docbook's fix (this same date) as the direct template rather than tei's superseded intermediate form. `jats-fmt` gains `Node::Raw`/`emit_fragment()`. `rescribe-read-jats` gains `is_block_element()`/`generic_div`/`generic_span` so an unrecognized element (e.g. `<statement>`, `<styled-content>`) now raw-preserves as a `jats:tag`-tagged div/span instead of vanishing with its children spliced into the parent. `<article-meta>`/`<journal-meta>` front-matter beyond `title`/`article-title` (contrib-group, pub-date, volume, issue, fpage, lpage, permissions, history, or any other unmodeled field) is now raw-captured verbatim as `{tag}_raw` metadata via the generalized `is_modeled_header_field` allow-list, with an explicit `<title-group>` pass-through arm added so the already-modeled title/journal-title reaches `extract_metadata` as a direct sibling instead of being buried inside a raw-captured blob. New fixtures: `adv-unknown-block-element`, `adv-unknown-inline-element`, `header-contrib-group`. `cargo clippy -D warnings` and full test/fixture suite clean. Stage numbers unchanged (R:4/W:2) — fixture-suite completeness, the oracle harness, and a longer fuzz campaign remain open in `TODO.md` before 5-Production. 2026-07-28 (commonmark-fmt construct features + default-backend silent-misparse fix): `commonmark-fmt` gains individual Cargo features per pulldown-cmark extension (`tables`, `task-lists`, `strikethrough`, `frontmatter`; `footnotes`/`definition-lists`/`math` reserved as inert names) plus `gfm`/`extensions` umbrella aliases — see `docs/adr/0011-commonmark-extension-feature-gating.md`. Fixes the actual production bug this was chasing: `rescribe_read_markdown::parse()` (the DEFAULT path, no backend override) was silently misparsing YAML front matter as a bogus `horizontal_rule` + setext `heading` in the document body (TOML front matter merged into a plain paragraph); both now populate `doc.metadata` correctly on the default path. `rescribe-fixtures/tests/run.rs`'s `markdown` test switched from `backend_pulldown::parse` to the default `rescribe_read_markdown::parse` (this is the change that turned the bug from silent to a failing fixture); a new `markdown_backends_agree` parity test compares node-kind shape between both backends per fixture. Two real, pre-existing bugs found via that parity test and fixed: (1) `commonmark-fmt`'s tight-list-item builder flushed accumulated leading inline text into an implicit paragraph only at `End(Item)`, landing it *after* a sibling nested-list block that had already been pushed mid-item (`- outer\n  - inner\n` reordered to list-then-paragraph instead of paragraph-then-list) — fixed by flushing at `push_block` time, using the flushed inlines' own span bounds rather than the item's. (2) the pulldown backend's `Options` never included `ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS`, so `backend_pulldown::parse` didn't support TOML front matter at all despite already having the `Tag::MetadataBlock(PlusesStyle)` handling arm — one-line fix. `footnotes`/`definition-lists`/`math` are explicitly deferred (feature names reserved, no implementation) per the task's own scoping; the markdown `footnote` fixture is excluded from the default-backend fixture run via a new `run_format_fixtures_excluding` helper, tracked in `TODO.md`. Writer-side symmetry: `rescribe-write-markdown`/`rescribe-write-commonmark` gained flat YAML front-matter emission from `doc.metadata` and task-list checkbox emission (write-commonmark); `rescribe-write-commonmark`'s pre-existing hand-rolled-emitter architecture violation (noted in the table below, row 252) was left as-is — out of scope for this pass. `commonmark`/`markdown` stage numbers unchanged pending a fuller reader/writer parity pass; this entry is the silent-misparse fix + feature-gating groundwork, not a stage promotion. 2026-07-28 (rst-fmt demotion, root cause found): confirmed and demoted the false 5-Production claim flagged in `TODO.md`'s 2026-07-28 ADR audit — `events.rs`/`batch.rs`/`writer.rs` were never wired into `lib.rs` and don't currently compile even once wired in, both because `crate::EventIter`/`crate::events()` no longer exist (deleted in commit `79ea2ce7af`, the same evening as the original sign-off, as collateral damage from an unrelated footnote-parsing refactor) and because `events.rs`/`writer.rs` reference a `Block::LineBlock` variant that a later commit replaced with `Block::Div{class:"line-block"}`. rst demoted R:5→R:4/W:5→W:2 (reader-ast and writer-builder are real and unaffected; reader-streaming/reader-batch/writer-streaming are non-functional despite the `Cargo.toml` features implying otherwise). Full findings in the "RST reader" section below and in `TODO.md`; re-implementing `events()`/`EventIter` as a genuine pull iterator (not a wiring fix) is left as follow-up work. 2026-07-28 (rst-fmt orphan-API recovery): all five required APIs are real and wired. Root-caused the merge commit precisely: `git log --follow` on `lib.rs` showed the `pub mod events/batch/writer` lines present at every commit up to `395a7ee532`, absent from `79ea2ce7af` onward — the actual damage is merge commit `383d4e6adf` (`Merge: 395a7ee532 79ea2ce7af`), which took the topic branch's entire `lib.rs` (1443 lines shorter) instead of merging it, discarding `mod` declarations, `EventIter`, and `Block::LineBlock` (already superseded by `Block::Div{class:"line-block"}` on mainline) as collateral. `events.rs`/`batch.rs`/`writer.rs` on disk were untouched by the bad merge and matched the pre-merge blobs exactly (`diff` confirmed byte-identical), so they were salvaged as reference material rather than rewritten from scratch — but `EventIter` itself (previously a ~1300-line duplicate of the whole recursive-descent grammar, living only in the deleted `lib.rs` region) was *not* resurrected uncritically: it is now a thin composition over the existing `Parser` (constructed via `Parser::new` + the same `collect_link_targets`/`collect_anonymous_targets`/`collect_substitutions` prescan `parse()` runs), so there is exactly one implementation of RST grammar, and `expand_block`/`expand_inline` lazily turn one already-parsed top-level `Block` into a `Vec<Frame>`-stack event sequence (`O(nesting depth)`, not `O(full document)`) — this both satisfies ADR 0003 ("parser IS the iterator, not `parse().collect()`") and removes ~1300 lines of duplicate parsing logic the old design carried. `writer.rs`'s `Writer` was rewritten outright: the salvaged version buffered the entire `Vec<Event>` and only built+emitted at `finish()` — exactly the "fake streaming, wraps the tree builder" pattern CLAUDE.md rejects — replaced with a frame stack that flushes each *top-level* block to the sink via `build_block` the moment its `End*` event arrives (`O(largest top-level block + nesting depth)`). `batch.rs`'s `StreamingParser` needed no rewrite: it already re-parses accumulated blank-line-delimited blocks through `events()` as they complete, genuinely `O(largest block)`. New tests: `events()`/`parse()` shape-equivalence (11 inputs spanning every `Block`/`Inline` variant, reduced to discriminant-only tag sequences so the comparison doesn't require identical Rust types); 6 chunked `StreamingParser` tests feeding input one byte at a time with awkward splits (mid-directive-keyword, mid-heading-underline, mid-UTF-8-char, mid-table-border, mid-footnote-continuation); a full-construct-mix round-trip through `Writer` (headings/lists/code/tables/definition-lists/footnotes/blockquote/transition). All 45 crate tests + 3 doctests pass; `cargo clippy --all-targets --all-features -D warnings` clean. Two *pre-existing, unrelated* `build_block` bugs were found (not fixed, out of scope for this pass, logged in `TODO.md`): admonition directives (`.. note::` etc.) lose their directive wrapper on write-back (the `Block::Div{class,..}` builder arm ignores `class`), and `FootnoteDef` emits only a single trailing `\n` instead of a blank-line separator, so a construct immediately following a footnote with ≥3-space indentation gets swallowed into the footnote body on re-parse — both reproduce with plain `crate::build()`, with no streaming API involved. Added `tests/no_orphan_modules.rs`: walks `src/` from `lib.rs`, follows every file-backed `mod name;` declaration, and fails if any `.rs` file is unreachable — verified it catches the class of bug that started this (confirmed cargo build succeeds silently with a genuinely unreferenced new file present; the test fails). A one-off workspace-wide sweep using the same logic found zero real orphans elsewhere (3 files flagged, all confirmed false positives from heuristic gaps: two crates with sibling `lib.rs`+`main.rs` in one `src/`, and one `#[path = "..."]` redirect in jats-fmt). rst promoted R:4→R:4 (unchanged number, now honest) / W:2→W:4: all API modes now real and unit-tested; not promoted to 5 because the existing `fuzz_rst_reader`/`fuzz_rst_roundtrip` targets only ever exercised `parse()`/`build()` via the `rescribe-read-rst`/`rescribe-write-rst` adapters — no fuzz target yet drives `events()`, `StreamingParser`, or `Writer` specifically, so the CLAUDE.md fuzz requirement for those three modes is still open (tracked in `TODO.md`). Also discovered (workspace-wide, out of scope to fix here): the central `rescribe-fixtures` harness (`crates/rescribe-fixtures/tests/run.rs`) tests every format crate via `parse()`/`emit()` only — no format's fixtures are wired through `events()`/`StreamingParser`/a streaming `Writer`, for any crate, not just rst. This is the same class of blind spot flagged in the markdown suite; logged in `TODO.md` as a cross-cutting gap, not an rst-specific one. 2026-07-29 (rst-fmt: streaming Writer subtree-reconstruction fix + events() zero-copy scoped): a benchmark investigation found two CLAUDE.md contract deviations. (1) The streaming `Writer` (`crates/formats/rst-fmt/src/writer.rs`) reconstructed a full `Block`/`Inline` enum subtree per top-level block via its `Frame` stack, then called the same `build_block`/`build_inlines` `build()` uses on that freshly-built tree — measured ~7-8x slower than `build()` for equivalent pre-materialized input, the exact "fake streaming API funneling through the tree builder" pattern CLAUDE.md rejects. Fixed by rewriting `Writer` so every frame accumulates already-formatted `String` buffers (plus a parallel plain-text buffer where genuinely needed — `Heading` underline sizing, `TableCell` content) and each `End*` event renders and splices that frame's final text directly, with no `Block`/`Inline` value ever constructed (commit `01472e3027`); a follow-up pass (commit `4daecb99`) found and gated an unconditional plain-text-tracking cost that a targeted allocation-count comparison showed was *not* the dominant factor. **Wall-clock did not improve** (~7-8x vs `build()`, unchanged before/after both commits) — real root cause, confirmed via allocation-count instrumentation, is that `build()` grows one `String` for the whole document (amortized) while the rewritten `Writer` gives every frame its own short-lived buffer (alloc count ~9x higher, tracking the time ratio almost exactly); closing that gap needs a further, separate redesign (write directly into the nearest ancestor's buffer, isolating a buffer only where post-processing genuinely requires one) — fenced as follow-up in `TODO.md` rather than attempted here, per CLAUDE.md's explicit-fencing-over-silent-half-completion principle. The architectural defect itself (subtree reconstruction) is confirmed gone — new tests: `test_writer_roundtrip_nested_lists`, `test_writer_no_subtree_reconstruction_blowup` (allocation-growth regression guard). All pre-existing tests (events()≡parse() equivalence, 6 StreamingParser chunk-split tests, writer roundtrip tests) still pass; `cargo clippy -D warnings` clean. (2) `events()` was independently confirmed non-zero-copy — read `EventIter`/`expand_block`/`expand_inline` directly (not just `parse_inline_content`, which is only part of the story): `EventIter::next()` calls the same per-block parser `parse()` uses to build a fully owned `Block`/`Inline` tree, then flattens that owned tree into events, so it shares the *entire* per-block parse path, not just the inline tokenizer. `parse_inline_content` itself scans a `Vec<char>` copy of each span with no byte-offset tracking anywhere, and ~15 call sites already join multi-line content into owned `String`s before it runs. Making this genuinely zero-copy requires a parallel byte-offset-based tokenizer yielding `Cow`s (following the `djot-fmt` `Frame::InlineText`/`base_offset` precedent) *and* plumbing real spans through those ~15 block-extraction call sites — confirmed to be a large, cross-cutting redesign, not a local tweak, so it was not attempted in this pass; scoped precisely in `TODO.md`. Stage numbers unchanged (R:4/W:4) — the fuzz-coverage gap for `events()`/`StreamingParser`/`Writer` noted above is still open, and is now joined by the two items above. 2026-07-29 (rst-fmt: streaming Writer buffer-per-frame closed; escaping bug fixed; events() zero-copy re-fenced): follow-up to the entry immediately above, closing the first of its two open items and fixing a correctness bug found while scoping the second. (1) **Writer buffer strategy.** The `Frame` stack now holds *marks* — a `usize` offset into one shared `Writer::out` buffer that grows once for the whole document (mirroring `BuildContext::output`) — instead of one `String` per frame. Children write straight through into the shared buffer; a frame that must decorate its own content post-processes the `out[mark..]` range in place. Construct classification (the substantive design output): **write-through, prefix fully known at `Start*`** — paragraphs, lists, list items (`build_list_item`'s per-child dispatch is decidable when the *child* opens, so continuation indents are emitted then rather than reconstructed at `EndListItem`), divs, definition lists/terms/descriptions, footnote defs, image/rule/math/raw leaves, and every inline span including links (whose closing text needs only the URL the opening event already carried); **write-through plus one in-place `insert_str` once content is known** — heading underline width (derived from the shared plain-text buffer, not the formatted bytes) and the figure caption lead-in (emitted only if a caption actually arrived); **deferred per-line transform** — blockquote, admonition and code-block re-indent, which walk their own `out[mark..]` range through a *pooled and reused* scratch buffer, so the pool costs `O(nesting depth)` allocations for an entire document rather than one per construct. Tables remain the one genuinely content-dependent-prefix construct (column widths are unknown until the last cell) and still collect cells, but now render straight into the shared buffer and *borrow* the collected cells for the width pass instead of cloning them into a parallel `Vec<Vec<String>>`; `calculate_column_widths`/`emit_table_border` were reshaped (iterator-of-rows, `&mut String`) so both emission paths share the border geometry without cloning or duplicating it. Measured with the same harness on both sides (release, best-of-30, synthetic construct-mix doc at 50/500/2000 sections, both writers starting from an already-materialized representation; "net" subtracts the harness's own event clone-and-drop baseline, a cost `build()` does not pay): allocations `3,560 → 425` / `35,914 → 4,029` / `143,916 → 16,031` (~9x fewer, now **0.73x of `build()`'s own count**), wall-clock vs `build()` `6.03/5.80/5.64x → 2.74/2.61/2.67x` net, or `8.00/7.72/7.50x → 4.64/4.43/4.53x` with the clone baseline left in on both sides (the figure directly comparable to the previously recorded 7-8x). **Reported honestly: this did not land at ~1.1x.** Allocation count is no longer the discriminator — it is now *below* `build()`'s — so the residual ~2.7x is per-event `match` dispatch and frame-stack traffic, i.e. the intrinsic cost of the event API versus a direct recursive tree walk; fenced in `TODO.md` with the recommendation to accept it as the event-API tax unless a caller demonstrates otherwise. New test `test_writer_byte_identical_to_builder` asserts the streaming path is **byte-identical** to `build()` across 18 construct-mix inputs (the pre-existing tests only compared re-parsed block shapes, which cannot catch formatting drift between two independent emission paths); `test_writer_no_subtree_reconstruction_blowup` is unchanged and still passes. Commit `f87b3d62ef`. (2) **Backslash escaping was a real correctness bug, now fixed** (commit `1c430173f4`) — the inline tokenizer had *no* backslash handling at all, so `\*not emphasis\*`, the RST spec's own example, parsed as live `Emphasis`: a silent misparse of valid input with no diagnostic. The reader now resolves escapes in the text scanner (escaped whitespace removed, per the `word\ *markup*` adjacency idiom); `find_closing`/`find_closing_char` refuse to close a span on an escaped delimiter and copy the escape *through* rather than resolving it, so it resolves exactly once at the level that emits the text; inline literals pass `escapes: false` per the spec's exemption and `:math:` content stays verbatim. Because a reader-only fix would break `parse(emit(parse(x))) == parse(x)` (a literal `*` recovered from `\*` would be re-emitted bare and re-read as markup), `build_inline` and the streaming `Writer` both re-escape `\`, `*` and `` ` `` on emit — borrowing unless an escape is actually needed — and `collect_text_from_inlines` counts the escaped form so heading underline widths and table column widths match the bytes actually written. New fixtures `fixtures/rst/escaped-markup` and `fixtures/rst/escaped-whitespace` plus the two `COVERAGE.md` rows they close (escaping was enumerated nowhere in that checklist before), and four new unit tests. (3) **`events()` zero-copy re-measured and re-fenced, not attempted.** Post-fix measurement on the same synthetic input: `Cow::Borrowed` fires for **0 of 50,000** text events at 2000 sections, and `events()`'s allocation count is `1.000x` `parse()`'s at every size — unchanged. Three distinct blockers, all of which must clear together (so partial work buys nothing): (a) `Inline::Text(String)` is *public API*, so borrowing needs `Inline<'a>`/`Block<'a>`/`RstDoc<'a>` — a breaking change across the crate and both rescribe adapters, which is itself the signal that `events()` must stop being a projection of the tree at all; (b) `parse_paragraph` joins `line.trim()` per line with `' '` into a fresh `String`, so a multi-line paragraph's text *does not exist as a contiguous slice of the input* — borrowing it requires emitting per-line borrowed text plus explicit breaks, i.e. a change to the event stream's shape and therefore to `test_events_matches_parse_shape` and every consumer; single-line spans (most headings, table cells, list items) are contiguous and could borrow, but only after (a); (c) the tokenizer is char-indexed (`Vec<char>` per span, owned `String`s built during the scan) and needs byte offsets throughout. Now that escapes exist, the Owned/Borrowed split at least has a real meaning: a span borrows unless it contains an escape or a substitution reference. Scoped in `TODO.md` as its own vertical-sized piece of work. Stage numbers unchanged (R:4/W:4). 2026-07-29 (later, same day): **`events()` zero-copy done — the fence above is closed.** The AST is now lifetime-generic (`RstDoc<'a>`/`Block<'a>`/`Inline<'a>`/`DefinitionItem<'a>`/`TableRow<'a>`/`Event<'a>` with `Cow<'a, str>` payloads; `rst-fmt` is unpublished, so the breaking change was paid now rather than later) and the shared inline tokenizer is byte-indexed over the input: `find_closing`/`find_closing_char` return a byte offset instead of a rebuilt `String` (they already passed escapes through verbatim, so the span text was always exactly `content[start..end]`), text runs materialize an owned buffer only when they actually contain an escape, and `merge_text_nodes`'s post-pass became merge-on-push that widens the borrowed slice for adjacent borrowed runs. Blocker (a) was cleared head-on and the inference attached to it — that this "means `events()` must stop deriving from the tree" — was **wrong**: once the tree can borrow, deriving events from it costs nothing extra and both paths keep sharing exactly one grammar. Blocker (b) was resolved by keeping the joined-string representation (`join_cow`/`join_words` borrow when one source line survives) rather than switching to per-line spans, on correctness grounds: RST inline markup may span a soft line break, so per-line tokenizing changes what parses; the event stream's shape is therefore unchanged. Measured on a synthetic construct-mix document (99KB/1.0MB, release, temporary global-allocator harness deleted after measuring): `parse()` 25,826→8,925 allocs and 109.4→144.2 MB/s at 200 sections, 252,633→88,132 allocs and 108.9→137.0 MB/s at 2000; `events()` 25,818→8,917 allocs and 93.8→115.2 MB/s, and 252,621→88,120 allocs and 96.6→118.0 MB/s. That is −65% allocations and +22-32% throughput on *both* paths, with **93.0% of emitted spans `Cow::Borrowed`**. The two paths' allocation counts remain near-identical, which is correct: what is left is the `Vec<Inline>`/`Vec<Block>` nodes both paths build, not per-span `String`s. The residual 7% owned is the multi-line wrapped paragraph and `CodeBlockContent` (the directive collector keeps a trailing blank line in the body, so it joins two lines); escape-bearing runs and synthesised `:ref:`/`:doc:` URLs are the other two owned cases. Pinned by two tests asserting on the `Cow` **variant**, not the string value. Also: `Event::into_owned` is an exhaustive match now instead of an unsafe lifetime transmute, `Writer::write_event` takes `Event<'_>` rather than requiring `'static`, and `rescribe-write-rst` builds an RST AST that borrows from the `Document` instead of copying every string out of it. Stage numbers unchanged (R:4/W:4); the remaining rst gap is fuzz coverage of `events()`/`StreamingParser`/`Writer`. 2026-07-29 (rst-fmt: streaming `Writer` residual gap actually profiled): the ~2.6-2.7x wall-clock gap left after the buffer-per-frame fix above was previously *asserted* to be "per-event dispatch overhead" without measurement — this pass used real `perf record`/`perf report` (`--call-graph fp`; dwarf unwinding was corrupted in this sandbox) against a build with `CARGO_PROFILE_RELEASE_STRIP=none` (the workspace's `.cargo/config.toml` strips release binaries by default, which silently makes `perf report` useless otherwise) and found the actual breakdown: ~29-36% of `Writer`-loop self-time is `Vec<Frame>::push` (writing the `Frame` enum's bytes once per event that opens any construct), ~18-22% is buffer-growth/reindent memmove, everything else (dispatch, `escape_text`, table border emission) is under 3% each. Re-measured ratio on this harness/machine came out ~1.5-2.0x, not 2.6-2.7x (different harness/machine, not a claim the earlier figure was wrong). Attempted the fix the profile implied — shrink `Frame`'s widest variants (`CloseDelim` enum replacing `Inline.close: &'static str`; a genuinely dead `content: usize` field removed from `Heading`/`Blockquote`, proven always `== mark` since `block_start(BlockKind::Other)` writes nothing) — but `size_of::<Frame>()` measured 40 bytes before *and* after, because `Table`/`TableRow`/`Link` are tied for the size ceiling, so shrinking one variant doesn't shrink the enum; wall-clock was correspondingly unchanged. Two further attempted shrinks (`Link.url: String → Box<str>`; boxing `Table.rows`/`TableRow.cells`) were reverted after being caught as real regressions (`into_boxed_str()` forces a shrink-realloc when `cap != len`; `Box::new(vec![])` allocates immediately even for an empty `Vec`) — validated by the existing `test_writer_no_subtree_reconstruction_blowup` allocation-count guard plus a manual check, not by inspection. Kept `CloseDelim` (a real representational improvement independent of the perf question) and the dead-field removal (pure hygiene). Honest conclusion: the residual gap is structural to the event-driven writer shape — a direct tree walk gets "what construct am I in" for free from the call stack, while an event-driven writer surviving across separate `write_event()` calls must reify that as an explicit `Vec<Frame>` push/pop per event, which a recursive walker never pays — not a contained rst-fmt bug, and not attempted further per CLAUDE.md's scope (actually lowering the size floor needs `Link`/`Table` to stop storing payload inline, e.g. an index into a side arena — real architectural work). Full writeup (including the two sandbox-specific profiling obstacles and fixes, applicable to whichever of the other ~25 hollow-streaming-writer crates gets profiled next) in `TODO.md` and the session scratchpad it references. All tests, `cargo clippy --all-targets --all-features -D warnings`, and `cargo fmt --check` clean. Stage numbers unchanged (R:4/W:4). 2026-07-29 (rst-fmt: streaming `Writer` `Frame` shrunk via side-stack, `out` pre-reserved — real wins, plus a benchmark-methodology finding): follow-up to the profiling entry immediately above, trying the two contained avenues it left open. **Avenue 1 (shrink `Frame`).** The prior attempt's diagnosis held (`Table`/`TableRow`/`Link` tied for `Frame`'s 40-byte ceiling), but boxing those three together behind one `Frame::Wide(Box<WideFrame>)` variant — tried first — was measured and rejected: `size_of::<Frame>()` dropped 40→32 bytes with **zero** wall-clock change (two A/B'd configurations, both ~1300µs/iter at 2000 sections), while allocation count on a table/link-heavy synthetic rose 4,518→6,518 (+44%), because `Box::new` allocates eagerly on every `Table`/`TableRow`/`Link` push where the previous inline `Vec::new()` didn't. Reverted. The side-stack design tried next kept the shrink without the cost: `Frame::Wide` became a zero-payload marker, and `Table`/`TableRow`/`Link` payloads moved to a separate `Writer::side: Vec<WideFrame>` that mirrors the well-nested push/pop order of just the wide frames (so `side.pop()` always returns the right entry with no stored index) — same `size_of::<Frame>() = 32`, but `side` grows via ordinary amortized `Vec::push`, not one eager allocation per wide-frame open. Measured: allocation count on the same table/link-heavy synthetic came out 4,519 vs the original's 4,518 (no regression), and isolated writer-only wall-clock (git-stash A/B, same harness, 4 trials each at 4000 sections) dropped from a ~2,571-2,917µs/iter baseline (machine noise shifted the absolute numbers between runs, see below) to ~2,373-2,735µs/iter — a consistent **~6-10% reduction**, confirmed by `perf`: `Vec<Frame>::push`'s share of self-time fell from ~29-36% (original investigation) to ~12% after this change. Kept. **Avenue 2 (pre-reserve buffers).** `Writer::out` started from `String::new()` (matching `BuildContext::output`, which also doesn't pre-reserve); added `Writer::with_capacity(sink, out_capacity)` plus a `DEFAULT_OUT_CAPACITY = 4096` used by `Writer::new`. Measured: small additional wall-clock win on top of the side-stack change (4000 sections: ~2,341µs/iter mean vs ~2,373µs/iter without the reserve) and **fewer** allocations on the table-heavy synthetic (4,515 vs 4,519) since fewer of `out`'s early doublings are needed. A supplementary check (pre-sizing the *sink* `Vec<u8>` to `input.len()`, not just `out`) was tried and made things ~20% **slower**, not faster — plausibly a large single upfront allocation crossing into a different allocator/mmap path with lazy page-fault costs a series of smaller incremental reallocs doesn't pay; not pursued further, not part of the change kept. **A significant methodology finding, reported honestly because it changes how the historical 1.5-2.6x figures in this file should be read**: profiling the fully-optimized build (side-stack + pre-reserve) showed `<Event as Clone>::clone` — the harness's own per-iteration `.clone()` of pre-materialized events, used across all `Writer`-vs-`build()` measurements in this file to reuse one parsed event stream across many timed iterations — consuming **~40% of measured "Writer" self-time**, confirmed by isolating it directly (`clone_only` mode: cloning the event `Vec` alone costs ~1,095-1,135µs/iter out of a ~2,664-2,710µs/iter total "writer" measurement at 4000 sections). This clone never happens in real `Writer` usage (a live event stream is consumed once, not cloned), so it is a pure benchmark artifact, present in every ratio this file and the two entries above it have reported — evenly across configurations being A/B'd (so the *relative* improvements measured above remain valid), but inflating the *absolute* Writer-vs-`build()` ratio. Subtracting the isolated clone cost from the raw measurements gives an adjusted ratio of roughly **0.90-1.39x** across 500/2000/4000 sections (sometimes faster than `build()`, not the previously reported 1.5-2.6x) — noisy and not to be over-read as a precise number, but enough to say the real production gap, in a genuine zero-clone streaming pipeline, is materially smaller than every previous absolute figure in this file suggested. **Conclusion**: the two contained changes here are real, kept, and validated (allocation-neutral-to-positive, measurable wall-clock win); the "structural, event-API-tax" framing from the prior entry still holds directionally (an explicit frame stack is real, irreducible overhead a tree walk doesn't pay), but its magnitude was overstated by an uncontrolled benchmark artifact, now identified and documented for whoever profiles the next of the ~25 crates using this same clone-based harness pattern. All tests, `cargo clippy --all-targets --all-features -D warnings`, and `cargo fmt --check` clean. Stage numbers unchanged (R:4/W:4). 2026-08-03 (ooxml-wml/ooxml-pml: `events()` Text-drop/end-tag-reversal and pptx txBody-unreachable bugs fixed, `‡` retired): both `KnownFailure` entries tracked since 2026-07-29 are closed — see the `events()`/`StreamingParser`/streaming-writer matrix's `docx`/`pptx` rows below for full root-cause detail. Summary: `ooxml-wml`'s `read_props` scan-ahead-for-props logic had two compounding bugs (frame pushed after, not before, `build_start_event` recursed into a child's own `open()`; `queue()` overwrote instead of prepended the single pending-event slot) that dropped the `Text` event and reversed `EndRun`/`EndParagraph` order for the ubiquitous `<w:p><w:r><w:t>` shape (no `<w:pPr>`) — fixed by pushing the frame first and switching `pending` from a fixed 2-slot array to a prepending `VecDeque`. `ooxml-pml` is an independent implementation (not shared with `ooxml-wml`), so it needed the same fix ported, plus its own separate, more severe bug: `dispatch_start()` had no entry for `<p:sld>`/`<p:cSld>`/`<p:spTree>`/`<p:txBody>` (none of which have a dedicated `PmlEvent`), so they fell into `skip_element()` and `events()` produced *zero* shape events at all on a real slide XML file (confirmed against `fixtures/pptx/slide/input.pptx`), not just missing text — fixed via an `is_transparent_wrapper` descend-without-emitting path mirroring wml's, applied to `read_shape_transform` as well (a `<p:sp>` scanning for `<p:spPr>`, which schema-valid content may omit). Fixing `read_shape_transform` this way surfaced a further pre-existing fragility shared by `read_props` in both crates — any text node, even pretty-printed whitespace, ended a props/geometry scan early — harmless for real Word/PowerPoint output (never pretty-printed) but a real regression risk for hand-authored/reformatted XML from any other consumer of these general-purpose crates, so scans now only stop on non-whitespace text. New crate-level fixture tests in `crates/rescribe-fixtures/tests/streaming_apis.rs` (`wml_events_reaches_and_correctly_orders_paragraph_text`, `pml_events_reaches_slide_text`, `pml_events_reaches_slide_text_with_sppr_present`) plus manual verification against the real `fixtures/docx/paragraph/input.docx` and `fixtures/pptx/slide/input.pptx` `events()` output (both correctly reach and order all text/nesting). `streaming_harness::KNOWN_FAILURES`'s `docx`/`pptx` `events` entries removed, `FormatCapabilities.events` promoted to `ApiState::Wired` for both; the table-only `‡` footnote marker and its definition are retired as stale. Full workspace `cargo test -q` clean except the pre-existing `ooxml-codegen`/`wml.rnc`-missing-file gap (independently confirmed via `git stash` to reproduce identically with none of this session's changes present — a parallel-test/shared-target-dir flake, not caused by this fix). Stage numbers unchanged (R/W:5† each) — `events()` was already excluded from the R/W score per this file's own convention (the score is about `parse()`/`emit()`); `streaming_parser`/`streaming_writer` remain `NotYetWired` for both formats, untouched by this pass.

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
| rst | 4 | 4 | U | hand | fuzz events()/StreamingParser | fuzz Writer specifically; Writer alloc-count parity with build() (scoped in TODO.md) |
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

‡ (retired 2026-08-03) Previously noted that `events()` (the standalone SAX-style reader
API, not the whole-document `parse()` the R score above is about) had a known Text-drop /
end-tag-reordering bug for the common `<w:p><w:r><w:t>` paragraph shape (`ooxml-wml`),
shared by `ooxml-pml`'s `events()`, which additionally could not reach slide text at all
(`<p:txBody>` unhandled in `dispatch_start`). Found/confirmed 2026-07-29 while wiring
`crates/rescribe-fixtures/tests/streaming_apis.rs`. Both fixed 2026-08-03 — see the
`events()`/`StreamingParser`/streaming-writer matrix below (`docx`/`pptx` rows) for the root
cause and fix; the `‡` marker is dropped from the table above since `events()` is now Wired
for both formats.

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
| rtf-fmt | ✓ | ~ | ✓§Δ | ✓¥ | ✓ |
| rst-fmt | ✓ | ✓ | ✓‡ | ✓ | ✓ |
| asciidoc | ✓ | ✓ | ✓§ | ✓ | ✓ |
| org-fmt | ✓ | ✓ | ✓ | ✓§ | ✓ |
| djot-fmt | ✓ | ✓ | ✓§ | ✓§ | ✓ |
| textile-fmt | ✓ | ✓ | ✓ | ✓ | ✓ |
| texinfo | ✓ | ✓ | ✓ | ~§ | ✓ |
| fb2-fmt | ✓ | ✓§ | ✓ | ✓§ | ✓ |
| commonmark-fmt (commonmark/gfm/markdown) | ✓ | ✓§ | N/A¶ | ✓§ | ✓ |
| html-fmt | ✓ | N/A¶ | N/A¶ | ✓ | ✓ |

`✓§` = the API is a real, independently-implemented code path (not a `parse()`-then-wrap
stub), fixture-driven-checked by `crates/rescribe-fixtures/tests/streaming_apis.rs`, and
**currently diverges from its reference on a specific, root-caused, tracked bug** — see the
"Cross-API harness inventory" subsection below for the one-line defect per cell and
`streaming_harness::KNOWN_FAILURES` for the full description. `~§` = same, except the
implementation itself is architecturally hollow (buffers all input/events and only does real
work in `finish()` — a fake streaming API per CLAUDE.md), not just producing wrong output on
some fixtures. `N/A¶` = the crate structurally does not have an independent implementation of
this API, for a documented reason (see below) — not a gap to close.

‡ rst-fmt's `batch` (`StreamingParser`) has one known construct gap found 2026-07-29 by the
new `crates/rescribe-fixtures/tests/streaming_apis.rs` adversarial-chunking check run across
the full `fixtures/rst/` suite (previously only 6 hand-picked cases were covered): a
multi-item definition list is split into one `StartDefinitionList`/`EndDefinitionList` pair
per item instead of one list spanning all items. Tracked as a `KnownFailure` in
`streaming_harness.rs` and in TODO.md; not fixed here.

Δ rtf-fmt's `batch` (`StreamingParser`) was rewritten 2026-08-04 from a buffer-then-finish stub
to a genuinely incremental reader (`O(header size + largest single paragraph or still-open
table/list + nesting depth)`, verified via `alloc_probe`: peak bytes 1372 @200 paragraphs vs.
1376 @20,000, ratio 1.00) — see `crates/formats/rtf-fmt/src/batch.rs`'s module doc and
`crates/formats/rtf-fmt/src/incremental.rs` for the design. One structural (not a chunking bug)
divergence from `events()` remains, in `Event::StartDocument.fonts`/`colors` only: `events()`
computes these by walking the entire already-parsed document, information that provably cannot
exist before `StartDocument`, which must be the first event a genuinely incremental reader
emits; `StreamingParser` reports the header's own declared `\fonttbl`/`\colortbl` tables instead
(available in `O(header size)` bytes). Confirmed against all 38 `fixtures/rtf/*` fixtures: 0
body-event mismatches, 33/38 differ only in `StartDocument`, 5/38 match exactly. Tracked as a
narrowed `KnownFailure` in `streaming_harness.rs` (`rtf`/`streaming_parser`), not `NotApplicable`
(the divergence is real and field-specific, not "this API doesn't apply here").

¥ rtf-fmt's `w-stream` is now `crate::sem_writer::Writer` (fixed 2026-08-03), a new module
consuming the crate's own semantic `Event`/`OwnedEvent` type directly — the low-level
`writer::Writer`/`TokenEvent` path this cell used to point at is a separate, lower-level API,
not the tracked `w-stream` contract. It solves RTF's genuine structural constraint (the
`\fonttbl`/`\colortbl` header must precede any `\f<n>`/`\cf<n>` body reference, but which
fonts/colors are used can only be known by having walked the whole document) without
buffering the body: `sem_events::Event` gained a `StartDocument { fonts, colors }` variant,
always the first event, carrying the exact tables `emit()` computes
(`crate::tables::build_font_map`/`build_color_map`, factored out so the two independent
emission paths can't diverge) — computed for free from the AST `events()` already fully
parses before yielding anything. `Writer` writes and flushes the header immediately on
`StartDocument`, then writes every construct straight through, flushing again whenever its
small `O(nesting depth)` context stack (tracking only blockquote/list-item paragraph elision,
per-table-row cell count via one in-place `\cellx` insert, and the link empty-children
fallback) returns to empty — never buffering the whole document. Confirmed byte-identical to
`build()` over every rtf fixture, and genuinely incremental (bytes reach the sink before
`finish()`; peak memory ratio < 20x across a 10x increase in synthetic paragraph count,
`crates/formats/rtf-fmt/src/sem_writer.rs::test_writer_peak_memory_bounded`). Separately, the
low-level `writer::Writer`/`TokenEvent` path had an independent, deeper bug: the tokenizer
silently discarded whether a control word's optional trailing-space delimiter was present in
the source, so no re-serialization policy keyed off `name`/`param` could reconstruct it
(`\f0 Times` has a delimiter space, `\u65?` does not, both are param-carrying — `emit()`'s
spacing is a stylistic per-call-site choice, not a function of token shape). Fixed by adding
`TokenEvent::ControlWord::had_delimiter_space`, populated by the tokenizer and consumed
verbatim instead of any heuristic. See TODO.md.

### Cross-API harness inventory (2026-07-30)

Full per-format, per-API status as declared in `crates/rescribe-fixtures/src/streaming_harness.rs::CAPABILITIES`
(the executable, tested source of truth — this table is a snapshot of it, not the other way
around; if the two ever disagree, the source file is correct). Every row below has a real,
fixture-driven check exercising the format crate's own `events()`/`StreamingParser`/streaming
writer directly, not just the rescribe adapter's `parse()`/`emit()`. Formats not listed here
are still in `streaming_harness::NOT_YET_AUDITED` — an honest "nobody has individually
audited this format's cross-API status yet" placeholder, not a claim of absence or health.

| Format | `events()` | `StreamingParser` | streaming writer |
|--------|------------|--------------------|--------------------|
| rst | Wired | Wired — fixed 2026-08-03: bullet/numbered lists spanning blank lines (incl. nested sub-lists), which used to split into multiple `List` pairs, now flush as one. Same `emit_block()` blank-line-flush root cause already fixed for the multi-item `DefinitionList`-split and cross-block heading-level-numbering variants; `feed_line` now defers the blank-line flush whenever the accumulated block's last line is a bullet/numbered list-item marker (any indentation), confirming continuation from the next line rather than replicating `parse_bullet_list`/`parse_numbered_list`'s full grammar — safe because `emit_block()` always re-parses the merged text via a fresh `EventIter`. Verified on `nested-list`/`path-deep-list` and via `rst_streaming_parser_matches_events_under_adversarial_chunking` (all fixtures/chunkings); see TODO.md | Wired |
| djot | Wired (independent recursive-descent vs. frame-stack impls) | Wired (fixed 2026-08-03): four distinct `batch.rs` bugs fixed — `BlockState::InDiv` now carries a `depth: usize` counter (was a flag) tracking nested `::: class` openers vs. bare `:::` closers, mirroring `find_div_close_generic`'s depth tracking in `parse.rs`; `StreamingParser` now keeps a persistent `link_defs` table accumulated across every block fed plus a `deferred`/`deferred_mode` queue so a block containing an explicit reference-style link (`][`) is held (along with everything after it) until `finish()`, then resolved against the fully-accumulated table via a new `EventIter::new_with_extra_link_defs` constructor — which in turn needed a `local_link_def_count` field on `EventIter` so injected resolution-only extras aren't re-emitted as duplicate trailing `LinkDef` events; pending `{.attr}` block-attribute lines are now carried into the following fence/div block instead of being flushed away as their own throwaway block; and blank lines are now held (`held_blanks`) until the next non-blank line reveals whether they're a real block boundary or a loose-list separator, fixing the definition-list split. Bare shortcut references (`[label]` with no second bracket) are not covered by the `][` heuristic and are not deferred — documented as a caveat on `StreamingParser`'s doc comment, not tracked as a `KnownFailure` since no fixture currently exercises it. Confirmed byte-for-byte equal to `events()` under adversarial chunking over all 79 fixtures (see TODO.md) | Wired — rewritten to a single shared-buffer write-straight-through design (2026-07-31, mirroring `rst-fmt`'s `Writer`); byte-identical to `emit()` over all fixtures (including `Event::LinkDef` and `Event::TableCaption`) and genuinely incremental; peak memory ~385x smaller than `parse()`+`emit()` on a 5000-section synthetic document (see TODO.md) |
| asciidoc | Wired (validates the AST→event expansion layer; `parse()` and `events()` share the same `try_parse_block()` loop; `Event::Metadata { key, value }` added — `:key: value` document attribute declarations now round-trip through `events()`/`StreamingParser`/the streaming `Writer`, emitted at their real source position, more faithfully than `AsciiDoc.attributes`'s own unordered `HashMap` can represent — see TODO.md) | Wired (fixed 2026-08-03): three distinct `batch.rs` bugs fixed — `feed_line` now holds back a trailing run of attribute/`.Title` lines and flushes them together with the delimited block they precede rather than flushing them away in isolation; `is_delimited_block_marker` now recognizes the `\|===` table delimiter (AsciiDoc's only non-identical-run delimiter); `StartDocument` is emitted unconditionally in `StreamingParser::new()` with `EndDocument` always paired in `finish()`, matching `events("")`'s Start/EndDocument pair on empty input. All 85 fixtures pass under adversarial chunking; see TODO.md | Wired |
| org | Wired (independent of `parse()`; dependency runs the other way) | Wired — fixed 2026-08-03: `BlockState::InSpecialBlock` gained a `depth: usize` nesting counter (mirroring `parse::EventIter::parse_block`'s own, `parse.rs:521`) so a nested `#+BEGIN_QUOTE`'s `#+END_QUOTE` no longer closes the outer block early; a trailing run of affiliated-keyword lines (`#+NAME:`, etc.) immediately before a `#+BEGIN_` line now stays attached to the same accumulated block instead of being flushed and re-parsed alone, so `pending_name` threads through correctly; the `#+BEGIN_` test now matches the untrimmed line (no `trim()`), mirroring `parse::EventIter::parse_next_block` (`parse.rs:252`), so an indented code fence inside a list item stays continuation text instead of being misread as a top-level block. Matches `events()` on all 89 org fixtures under adversarial chunking (previously 86/89) | Wired — rewritten to a single shared-buffer write-straight-through design (2026-07-31, mirroring `rst-fmt`'s `Writer`); this harness previously claimed Wired without an incrementality probe while the writer was in fact architecturally hollow — that gap is now closed and confirmed genuinely incremental. `Event::Metadata` round-trips content-correctly but is a documented partial divergence from `build()`'s exact "move all metadata to the top" semantics for the (currently unobserved in any fixture) case of metadata appearing after body content — see TODO.md. Peak memory ~384x smaller than `parse()`+`build()` on a synthetic document, though throughput ~1.8x slower (per-event dispatch overhead vs the builder's simpler tree walk, same tradeoff shape as t2t) |
| html | NotApplicable: `events()` is literally a walk over the html5ever-built tree (`events_from_doc(&parse(input).0)`) — HTML5 tree construction (foster parenting, adoption agency) makes true incremental delivery impossible per the crate's own docs and CLAUDE.md's html5ever out-of-scope carve-out | NotApplicable: `feed()` is a bare buffer append; all parsing happens in `finish()`, for the same HTML5-tree-construction reason (chunk-boundary/UTF-8-split integrity is separately checked and passes, but that is not the same claim as incremental delivery) | Wired |
| texinfo | Wired (includes `Event::Title` for `@settitle`; note: `events()` itself eagerly parses the full document and materializes the complete event vec before yielding the first event — not incremental, though not tracked as a `KnownFailure` since the harness only checks event *content* equivalence, not delivery timing, for this API) | Wired — rewritten (2026-07-31) from buffer-fed-bytes-then-parse-in-finish() to a line-buffered splitter that flushes each top-level unit (paragraph, heading, or `@directive...@end directive` environment) to the handler as soon as its boundary is confirmed, re-parsing only that unit's text via `crate::events::events`; byte-for-byte identical to `events()` over all fixtures under adversarial chunking, and a deterministic pre-finish probe confirms `feed()` delivers events before `finish()` is called; peak live heap flat (1.00x) across a 10x increase in synthetic-document size, vs. 8.72x for the prior buffer-everything implementation (see TODO.md) | Wired — rewritten to a single shared-buffer write-straight-through design (2026-07-31, mirroring `rst-fmt`'s `Writer`); byte-identical to `emit()` over all fixtures and genuinely incremental (bytes reach the sink before `finish()`); peak memory 470x smaller than `parse()`+`emit()` on a 5000-section synthetic document (see TODO.md) |
| fb2 | KnownFailure, narrowed (2026-08-03): the originally-tracked bug (`events()` silently dropping `Metadata` whenever input lacks a literal `<description>`) is fixed — `SemanticState` now synthesizes a default `Metadata` right before the first other top-level child of `<FictionBook>`, or at end-of-input for a childless `<FictionBook>`. Fixing it unmasked four further, independent, pre-existing `events()`-vs-AST-projection bugs (same first-divergence-only masking pattern as the commonmark row below) — three now also fixed: text immediately before an inline `<code>` was swallowed into the `Code` element instead of flushed as its own `Text` first; `<table>`/`<tr>`/`<td>`/`<th>` push no stack frame, but `handle_end` still unconditionally popped the generic stack before its table-specific arms ran, silently corrupting state and losing the table plus every event after it; `<a type="note" href="#...">` was never recognized as a footnote reference, always built as a generic `Link`, unlike `parse()`'s AST. Two fixtures still diverge, left open: `adv-malformed` is a genuine architectural divergence (an unclosed `<FictionBook ...>` start tag swallows the following `<body>` text per quick_xml's own tokenization, so a `Section`/`Paragraph` gets fully tokenized and events-yielded before the reader errors on the orphaned `</body>`; `parse()`'s AST defers attaching a body's sections until its `</body>` End event actually dispatches, so it silently drops the orphaned subtree instead — recovering this in `events()` would require buffering/deferred emission, exactly what `EventIter`'s no-pre-materialization design forbids); `annotation` is a genuine unimplemented feature (the book's own `<description><title-info><annotation>` sub-content, with `<p>`/`<poem>`/`<cite>`/`<subtitle>`/`<table>`/empty-line children, isn't modeled at all in `events.rs`'s description parsing — a real second content-model builder, not a quick fix). A related bug also fixed this pass: a description-context self-closing `<image>` (e.g. inside `<title-info><coverpage>`) used to leak out as a stray top-level `Event::Image`; now a no-op, matching `parse()` (which also doesn't populate `TitleInfo.coverpage` — a separate, pre-existing, `parse()`-side gap affecting both APIs equally). A `parse.rs`-only bug (not `events()`) was fixed alongside: `<custom-info>` text content was stolen by an unconditional `flush_text()` call before `handle_end` could read it, so `parse()`'s own AST used to report empty content | Wired (fixed 2026-07-31): `StreamingParser` rewritten from buffer-all-fed-bytes-then-parse-in-`finish()` to a true incremental drain reusing the same `SemanticState` dispatch machine `events()`/`EventIter` already used internally — the same "rebuild a `Reader` over just the unconsumed tail, `Err` means wait for more bytes" technique `docbook-fmt` pioneered, plus a hand-tracked `open_stack` replicating quick-xml's `check_end_names` validation (also mirroring docbook-fmt) so malformed-XML fatal-stop behavior matches `events()` exactly. Confirmed byte-for-byte equal to `events()` under adversarial chunking (whole/single-byte/chunks-of-N/mid-UTF-8-char) over every fb2 fixture; peak memory measured flat (~3.09KB) across a 10x input-size increase (200 vs 2000 synthetic sections), not O(full input) | KnownFailure, narrowed (2026-08-03): the writer itself is genuinely incremental and fed by `events()`, so every `events()`-side bug fixed this pass no longer reproduces here either; the two `events()`-side gaps still open (`adv-malformed`, `annotation`) still reproduce downstream, for the same underlying reasons documented in the `events()` column |
| textile | Wired | Wired (fixed 2026-07-31): `StreamingParser` rewritten to be genuinely incremental — `feed()` accumulates lines into a small pending buffer and re-runs the same block-boundary logic `parse()`/`events()` use (`parse::BlockCursor`, wrapping a `parse_next_block()` step function extracted from the previously-monolithic `parse_blocks()` loop) over just that pending tail; a block flushes to the handler the instant a later buffered line proves its boundary decision can't change (every block-parsing arm only ever inspects lines up to and including the one that ends it, never further ahead), and only the still-open block's lines stay buffered. Memory: O(largest block), not O(full input); measured ~28x–62x peak-memory reduction vs. the old buffer-then-`finish()` approach across 20/200/2000-section synthetic documents (see TODO.md) | Wired (fixed 2026-07-31): `Writer` rewritten to emit incrementally per event (shared output buffer, O(depth) frame stack) instead of buffering into a `Vec<TextileEvent>` and delegating to `emit::build()` in `finish()`; ~4.4x faster. One pre-existing quirk replicated exactly, not "fixed": a `Paragraph` directly inside a `Blockquote`/list item silently ignores its own `align`/`attrs` (an O(1) parent-frame lookup) |
| commonmark / gfm / markdown (shared `commonmark-fmt`) | KnownFailure: one bug remains — `StartList` always reports `tight: true`, never corrected once real tightness is known (fixtures `rare-loose-list`, `integration-loose-list-item`, `rare-tight-vs-loose`); verified against the CommonMark spec that this needs whole-list lookahead (tightness is uniform across all of a list's items) and so can't be determined at `StartList`-emission time within a genuinely single-pass, O(nesting depth) `EventIter` without either buffering the whole list (defeats streaming for pathological long lists) or changing `Event`'s public shape — left open, see TODO.md. Five other bugs found the same way (this harness's `ast_to_events` projection check, which reports only the first diverging fixture per run, so later ones stayed masked behind earlier ones) were fixed 2026-08-03: (1) image alt-text `Text` event was emitted before `StartImage` instead of between `StartImage`/`EndImage`, duplicating alt text in the output; (2) consecutive `Text` events from pulldown-cmark weren't coalesced the way `parse()`'s AST deliberately does; (3) tight list items' bare inline content (no `Paragraph` tags from pulldown-cmark) wasn't wrapped in a synthetic `StartParagraph`/`EndParagraph` pair the way `parse()`'s AST always does (`parse.rs`'s `flush_tight_inlines`); (4) table-head cells weren't wrapped in a synthetic `StartTableRow`/`EndTableRow` pair the way `parse()`'s AST always does, since pulldown-cmark never emits `Tag::TableRow` for the header row; (5) GFM email autolinks (`<user@example.com>`) weren't normalized to a `mailto:` URL the way `parse()`'s AST does. `Event::LinkDef` (reference-style `[label]: url` definitions, which pulldown-cmark's own event stream never surfaces) continues to round-trip through `events()`/`StreamingParser`/the streaming `Writer` | NotApplicable: buffering all input before parsing with pulldown-cmark is the sole documented CLAUDE.md-sanctioned exemption (pulldown-cmark requires the full `&str`) | KnownFailure: the writer is a genuine write-through streaming design (not architecturally hollow) and byte-identical to `build()` for every construct in the crate's own tests; its table-head handling was updated (new `Frame::TableHead` marker) to match `events()`'s corrected `StartTableHead`/`StartTableRow` shape without double-writing the row's leading `\|`. The one residual divergence is downstream of the `events()` list-tightness bug (not a writer bug): a loose list's `StartList` carries `tight: true`, so the Writer omits the blank lines between items a loose list requires |
| docx | Wired (fixed 2026-08-03): `events()` was dropping the `Text` event and reversing `EndRun`/`EndParagraph` order for the common `<w:p><w:r><w:t>` shape (no `<w:pPr>`). Root cause was two compounding bugs in `read_props`'s scan-ahead-for-props logic: `open()` pushed a container's own stack frame only *after* `build_start_event` returned, so a child container found during the parent's props scan (e.g. `<w:r>` found while scanning `<w:p>` for `<w:pPr>`) pushed its frame *before* the parent's, inverting `End`-tag pop order; and `queue()` overwrote the single pending-event slot instead of prepending, so a `Text` event queued by a deeply nested scan (`<w:p><w:r><w:t>`, three levels) was clobbered by the parent's own queued event. Fixed by pushing the frame before scanning and replacing the fixed 2-slot `pending` array with a `VecDeque` that `queue()` prepends to. See `crates/rescribe-fixtures/tests/streaming_apis.rs`'s `wml_events_reaches_and_correctly_orders_paragraph_text` | NotYetWired | NotYetWired |
| pptx | Wired (fixed 2026-08-03): `events()` could not reach any slide text at all. `ooxml-pml` is an independent implementation, not shared with `ooxml-wml`, so it needed its own port of the docx fix (same push-before-build/prepend-not-overwrite pattern) plus a separate reachability gap: `dispatch_start()` has no dedicated `PmlEvent` for `<p:sld>`/`<p:cSld>`/`<p:spTree>`/`<p:txBody>` — real `ppt/slideN.xml` root structure and shape text wrapper — so they fell into `skip_element()` and the entire shape/paragraph/run/text subtree was silently dropped (verified against `fixtures/pptx/slide/input.pptx`'s actual `slide1.xml`: without a transparent-wrapper path, `events()` on the real file produced zero shape events at all, not just missing text). Fixed via a `is_transparent_wrapper` descend-without-emitting path mirroring ooxml-wml's, applied to `read_xml_info`/`read_xml_info_or_props` *and* `read_shape_transform` (a `<p:sp>` scanning for `<p:spPr>`, which schema-valid content may omit entirely — confirmed via a fixture with `<p:txBody>` and no `<p:spPr>`). Fixing `read_shape_transform` this way surfaced a further pre-existing fragility, shared with `read_props` in both crates: any text node — even pretty-printed whitespace between sibling elements — ended the scan early; harmless for real Word/PowerPoint output (never pretty-printed) but broke a hand-formatted crate-level fixture, so scans now only stop on non-whitespace text. See `pml_events_reaches_slide_text`/`pml_events_reaches_slide_text_with_sppr_present` in the same test file | NotYetWired | NotYetWired |
| xlsx | Wired | NotYetWired | Wired |
| docbook | Wired (auto-close recovery fixed 2026-08-03; `line-break`/`<sbr></sbr>` gap fixed 2026-08-03 follow-up: `Node::Element` gained a `self_closing: bool` field, threaded through `parse.rs`/`emit.rs`/`events.rs`'s `walk_node`/`collect_doc`, so the AST — previously the lossy side, not `EventIter` — now distinguishes an explicitly-empty non-self-closing tag from a self-closing one. See TODO.md) | Wired (fixed 2026-07-30, entity-coalescing; 2026-08-03, malformed-XML): `StreamingParser` tracks open-element names by hand in a `Vec<String>` that persists across `drain()` calls (`check_end_names`/`allow_unmatched_ends` stay disabled on the per-drain-call `Reader` itself — architecturally required — but tag balance is enforced manually); also got the entity-coalescing fix via a `pending_text` field, and a `close_unclosed_elements()` method mirroring `EventIter::finalize`'s auto-close recovery at every terminal point (`drain()`'s EOF branches and both manual tag-mismatch branches) | Wired (fixed 2026-08-03 follow-up alongside events(): `emit.rs`'s `emit_node` now also respects `self_closing`, not just `children.is_empty()`, so `build()` is no longer the lossy side either — both it and the streaming Writer faithfully reproduce `<sbr></sbr>` vs. `<sbr/>` from the source) |
| jats | Wired (fixed 2026-08-03 alongside docbook-fmt, byte-identical `events.rs` shape): same malformed-XML auto-close recovery; unlike docbook-fmt, no jats fixture hits the explicitly-empty-tag defect, so this is genuinely Wired, not narrowed | Wired (fixed 2026-07-31): the mismatch/unmatched-end-tag bug is fixed the same way as docbook (2026-07-30); the remaining failure was a probe-methodology artifact, not a defect — the adversarial-chunking incrementality probe's fixed 50%-byte split of fixture `adv-malformed-xml` landed mid-attribute-value inside the still-open root start tag, so zero events delivered there was already correct behavior. Replaced with a hand-built synthetic sample with a provably complete prefix (same fix already applied to fb2-fmt/texinfo/xwiki/textile-fmt/pod-fmt); the new probe passes | Wired (fixed 2026-08-03): the streaming `Writer` is fed by `events()`, so its new malformed-XML auto-close recovery flows through for free; no jats fixture hits the explicitly-empty-tag defect either |
| tei | Wired (fixed 2026-08-03 alongside docbook-fmt): same malformed-XML auto-close recovery, on a fixture combining both mismatched *and* unclosed end tags; no tei fixture hits the explicitly-empty-tag defect, so this is genuinely Wired | Wired (fixed 2026-07-30): same fix as docbook | Wired (fixed 2026-08-03): same as jats — fed by `events()`, picks up the fix for free |
| odt (`odf-fmt`) | NotYetWired: `events()` is a genuine independent quick_xml scan of `content.xml` (not a `parse()`-then-walk fake — corrects a prior assessment), but eagerly buffers every event before the first `next()` call (not memory-bounded); a faithful `ast_to_events` projection spanning odt/ods/odp body shapes is substantial follow-up work | NotYetWired: no `StreamingParser<H>` type exists in the crate at all yet (`batch.rs` calls it "future" work) | Wired (fixed 2026-08-03): all remaining 12/66 fixture divergences behind the byte-identical-to-builder check are fixed. Diffing per-fixture emitted `content.xml` (not just the first divergence the harness check reports) found seven distinct root causes, not the single "OdfEvent vocabulary gaps" bucket the entry previously described: (1) note internals unmodeled — no event for `<text:note-citation>`/`<text:note-body>`, so a note's marker and entire body were dropped (`DocBuilder` even had a `NoteBody` frame nothing pushed); added `NoteCitation`/`StartNoteBody`/`EndNoteBody` (footnote, footnote-formatted, endnote); (2) empty-element inline leaves had no variants for `<text:soft-hyphen/>`, `<text:soft-page-break/>`, `<text:bookmark/>`/`-start` — the previously-undiagnosed "heading-only divergence" is this same bug, not a heading bug (`fixtures/odt/heading`'s `<text:h>` contains a `<text:bookmark/>`) (bookmark, heading, soft-hyphen); (3) `<office:annotation>` corrupted output rather than merely losing it — emitted `Unknown` and the scan then descended into its children, leaking `<dc:creator>` text into the enclosing paragraph while the comment itself vanished; added `Annotation`, flattened the same way `parse()` does (annotation); (4) `StartCell` carried neither `table:number-columns-spanned` nor `table:number-rows-spanned` nor the typed value, and self-closing `<table:covered-table-cell/>` was recognized only in a spreadsheet body (colspan-rowspan); (5) block-level `<draw:frame>` was structurally unreachable — `EndFrame` routed through `push_inline` only, which matches nothing at block level; `<draw:text-box>` also had no text-body handling (text-box, image-caption); (6) `quick_xml::Event::GeneralRef` (character/entity references) had no arm in `events.rs`, though `parser.rs` has one, so `&#160;` was silently dropped (non-breaking-space); (7) `OdfEvent::Unknown` carried no raw payload, making raw preservation impossible through the event path — now carries verbatim XML and consumes its subtree, which required `events.rs` to track the enclosing content model (a `<table:table>` under `<office:text>` is a table; one inside `<text:p>` is unmodeled by `parser::parse_inlines` and captured raw) (path-deeply-nested-table). Field elements (`text:date`, `text:page-number`, ...) were named in the old description but no odt fixture exercises them — an `OdfEvent::Field` variant was added anyway for parity with `parse()`'s `Inline::Field`, though it fixed none of the 12. Two real losslessness gaps found during this work are *not* fixed, both on the `parse()` side where the AST path and the event path lose equally (so neither is a streaming defect and neither shows up in this check) — see TODO.md: `ast::FrameContent` is an either/or enum giving `<draw:image>` priority over a sibling `<draw:text-box>`, so a frame holding both an image and a caption loses the caption entirely; and self-closing field elements (as opposed to `<text:date>...</text:date>`) are dropped by both paths |
| ansi | NotYetWired: `parse()`'s `AnsiNode` has no variant for a bare SGR sequence at all (folded into a running `style` var, no node emitted), the reverse of the usual defect shape, so no faithful `ast_to_events` projection exists; the spurious-`ResetStyle`-on-no-op-SGR bug found via the checks below is fixed (2026-07-30): `apply_sgr_event()` now returns whether it saw an explicit reset code, only that triggers `ResetStyle` | Wired (2026-08-03 follow-up): all four bugs originally found are fixed. `drain_complete` carries running style across drains, no longer inherits spurious-`ResetStyle`, coalesces adjacent `Text` events via a `pending_text` accumulator (all 2026-07-30), and — the fourth, previously left open — `find_safe_boundary` gained a second pass (`truncate_before_unclosed_osc8_hyperlink`) that scans for a complete OSC 8 *opening* sequence with no matching closer yet in the safe prefix and backs the boundary off to before it, deferring the whole open..close span to a later call instead of exposing an isolated opener to `EventIter`. See TODO.md | KnownFailure: the `adv-unknown-sgr` divergence this entry originally tracked is fixed (2026-08-04) — `AnsiNode` gained a `ResetStyle { span }` variant, emitted by `parse_csi`'s `m` arm exactly when an explicit reset code (`0`/empty) is seen *and* the running style was already empty beforehand (`apply_sgr` now returns whether it saw an explicit reset, mirroring `events.rs`'s `apply_sgr_event`); narrowed to that condition specifically so it doesn't also fire on an ordinary bold→default reset, which the next Text/Hyperlink node's own `style` field already captures without a redundant node (an unconditional emission was tried first and shifted node indices, breaking 4 passing fixtures — `nested-sgr-reset`, `rare-inline-in-text`, `reset`, `styled-in-paragraph`). `build()`'s `emit_node` writes the literal `\x1b[0m` for this node unconditionally, matching `events()`/`Writer`'s own unconditional `ResetStyle` handling. Confirmed byte-identical for `adv-unknown-sgr` specifically. **But the check still fails overall — verifying it exposed a much larger, wholly unrelated, previously-masked gap**: 24 of 46 ansi fixtures diverge between `build()` and the streaming `Writer` (e.g. `bold`: `build()` defers the trailing reset to end-of-document, past the fixture's trailing `\n`, giving `"...Hello\n\x1b[0m"` vs. the source-faithful `"...Hello\x1b[0m\n"`; `rare-bold-italic`: `build()` recomputes one merged `\x1b[1;3m` from the resulting style rather than preserving the source's two separate `\x1b[1m`/`\x1b[3m` groups, which `events()`/`Writer` reproduce verbatim). Root cause: `parse()`'s AST only ever attaches a *resulting* style to the next Text/Hyperlink node, discarding both the SGR grouping in the source and its position relative to intervening non-text nodes (e.g. a `Newline` between a reset and the text after it) — this is the AST-side gap already named by this row's `events` cell ("no variant for a bare SGR sequence at all"), just previously undiscovered because the check (`ansi_streaming_writer_byte_identical_to_builder_over_all_fixtures`) only records the *first* diverging fixture per run, and `std::fs::read_dir` order is filesystem-dependent — on the environment that first wrote this entry, `adv-unknown-sgr` happened to sort first and masked all 24; on this one, `bold` does. Closing this fully likely means giving `parse()`'s AST an explicit per-SGR-group node (like the narrow `ResetStyle` above, but unconditional) and having `build()` emit style transitions from those nodes in source order instead of deferring to the next Text/Hyperlink node — a substantially larger change than this pass's narrow fix, left open. See TODO.md |
| bbcode | Wired (`events()` is literally `parse::parse(input)` + a tree walk — same non-independent shape as html's, scoped honestly per the asciidoc precedent rather than declared NotApplicable, since nothing format-structural forces it) | Wired — genuine incremental line-buffered state machine (`batch.rs`'s `feed_line`/`emit_block`), confirmed equivalent to `events()` over all 53 bbcode fixtures plus several hand-built adversarial cases | Wired (fixed 2026-07-31): `Writer` rewritten to emit incrementally per event instead of buffering into a `Vec<OwnedEvent>` and delegating to `emit()` in `finish()`; fully write-through, no deferred constructs; ~2.4-4.3x faster and ~1000x lower peak memory |
| creole | Wired (`events()` is literally `EventIter::new`'s `parse::parse(input)` + `collect_events(&doc)` tree walk — same non-independent shape as bbcode's/html's, scoped honestly per the bbcode/asciidoc precedent rather than declared NotApplicable) | Wired — genuine incremental line-buffered state machine (`batch.rs`'s `feed_line`/`emit_block`), confirmed equivalent to `events()` over all 35 creole fixtures plus an incrementality probe; one inspected edge case (a nowiki block closed by a line with trailing content after `"}}}"`) degrades incrementality but not correctness, verified by hand | Wired (fixed 2026-07-31): `Writer` rewritten to emit incrementally per event instead of buffering into a `Vec<OwnedEvent>` and delegating to `build()` in `finish()`; fully write-through, no deferred constructs; ~2.4-4.3x faster and ~1000x lower peak memory |
| dokuwiki | Wired (`events()` is literally `InputEventIter::new`'s `parse::parse(input)` + `EventIter` tree walk — same non-independent shape as bbcode's/creole's, scoped honestly per that precedent rather than declared NotApplicable) | Wired — genuine incremental line-buffered state machine (`batch.rs`'s `feed_line`/`emit_block`), confirmed equivalent to `events()` over every dokuwiki fixture plus an incrementality probe, with no coarser-boundary caveat needed (unlike bbcode/creole) since `parse.rs`'s `Parser` has no cross-block state at all | Wired (fixed 2026-07-31): `Writer` rewritten to emit incrementally per event instead of buffering into a `Vec<OwnedEvent>` and delegating to `build()` in `finish()`; fully write-through, no deferred constructs; ~2.4-4.3x faster and ~1000x lower peak memory |
| jira | Wired (`events()` is literally `crate::parse::parse(input)` + `emit_doc_events` tree walk — same non-independent shape as bbcode's/creole's/dokuwiki's, scoped honestly per that precedent rather than declared NotApplicable; `Event` carries every field every `Block`/`Inline` variant holds, no expressiveness gap found) | Wired — genuine incremental line-buffered state machine (`batch.rs`'s `feed_line`/`emit_block`), confirmed equivalent to `events()` over every jira fixture plus an incrementality probe, with no coarser-boundary caveat needed (like dokuwiki) since `parse.rs`'s `Parser` has no cross-block state and no decorator-line-preceding-a-fence construct (`{code:lang}`/`{panel:title=...}` params are on the fence line itself) | Wired (fixed 2026-07-31): `Writer` rewritten to emit incrementally per event instead of buffering into a `Vec<OwnedEvent>` and delegating to `build()` in `finish()`; nearly all write-through, one O(1) deferred piece (a table row's closing `\|\|`/`\|` depends on its first cell's header-ness, tracked as `Option<bool>`); ~1.9-2.5x faster, peak memory 176MB → 4.2KB on a 50,000-section doc |
| mediawiki | Wired (validates the AST→event walk layer; `events()` is architecturally `parse()`-then-walk, like html, but unlike html's generic tree walk it makes real per-`Block`/`Inline`-variant mapping decisions, so the check has teeth) | Wired | Wired (fixed 2026-07-31): `Writer` rewritten to emit incrementally per event instead of buffering into a `Vec<OwnedEvent>` and delegating to `emit()` in `finish()`; two genuine deferrals — `Link` text (top-level `Text` children only, matching `build_inline`'s own logic) and the document-level trailing-whitespace collapse (held back at `flush()`, always exactly one final `"\n"` at `finish()`); ~1.66x faster, peak memory 176MB → 4.3KB on a 50,000-section doc |
| tikiwiki | Wired (same narrower claim as mediawiki) | Wired | Wired (fixed 2026-07-31): `Writer` rewritten to emit incrementally per event instead of buffering into a `Vec<OwnedEvent>` and delegating to `build()` in `finish()`; fully write-through, no deferred constructs |
| twiki | Wired (same narrower claim; `events()` also has a non-standard signature — it takes `&TwikiDoc`, not raw input, an existing deviation from the vertical-completion contract, tracked in TODO.md) | Wired | Wired (fixed 2026-07-31): `Writer` rewritten to emit incrementally per event instead of buffering into a `Vec<OwnedEvent>` and delegating to `build()` in `finish()`; fully write-through except `Link` (plain-text label buffer); the byte-identical check caught a real, independent ordering bug — `build_list_items` wrote an item's own newline before recursing into nested lists instead of after — fixed with a `wrote_own_line` flag; ~3.9x faster (122→483 MB/s) |
| vimwiki | Wired (same narrower claim) | Wired (fixed 2026-07-30): `parse_list()` had the same "loop only checks that some marker matched, never that it matches this list's `ordered` flag" defect as zimwiki's/markua's below (independently found here, and confirmed to reproduce even under whole-input non-adversarial chunking, so not a chunk-boundary bug) — an unordered list, ordered list, and checklist separated only by blank lines got merged by `parse()`/`events()` into one `Block::List` with a single `ordered` flag; now that `parse_list()` stops at a marker-type change, `parse()`/`events()` and `StreamingParser` agree (fixture `mixed-list-markers`) | Wired (fixed 2026-07-31): `Writer` rewritten to emit incrementally per event instead of buffering into a `Vec<OwnedEvent>` and delegating to `collect_doc_from_events()`+`build()` in `finish()`; fully write-through except `Blockquote` (dissolves/merges paragraph children, replicated exactly) and `Link` (equality-based separator); ~5.4x faster (83→447 MB/s) |
| xwiki | Wired (`EventIter::next()` is a genuine lazy pull-iterator over `&XwikiDoc`, events.rs:168-385 — not eager materialization like zimwiki/markua/muse below) | Wired (fixed 2026-07-31, reader): `StreamingParser` rewritten from a bare `buf.extend_from_slice`-then-parse-in-`finish()` wrapper to a genuine block-at-a-time incremental parser — `feed()` accumulates one top-level block (paragraph/heading/list/table/code/macro/quote block, the same boundaries `parse::Parser::parse`'s dispatch loop uses) and flushes it (reparsed in isolation via `parse::parse` + walked with `events::events`) as soon as its boundary is confirmed; confirmed equivalent to `events()` over all xwiki fixtures under adversarial chunking (whole/single-byte/N-byte/mid-UTF-8-char); peak memory flat across a 10x input-size increase (~1.97 KB) vs. the old implementation's ~9.8x near-linear growth (1.30 MB → 12.74 MB) on the same synthetic multi-block document | Wired (fixed 2026-07-31, writer — same day as the reader fix above but a separate change): `Writer` rewritten to emit incrementally per event instead of buffering into a `Vec<OwnedEvent>` and delegating to `emit::build()` in `finish()`; fully write-through except `Link`, which needed genuine reordering (`[[label>>url]]` — label before url, url known first) via holding `url` on the frame; ~6.4x faster (59→382 MB/s) |
| zimwiki | Wired (validates the AST→event expansion layer; `events()` is parse()+eager-materialize-then-walk) | Wired (fixed 2026-07-30): `parse_list()` merged a blank-line-separated unordered list immediately followed by an ordered list into one `Block::List` tagged with the first item's `ordered` value (a whole-document `parse()`-level bug, not a streaming-specific one, since its loop condition only checked "some marker matched," never that it matched the list's own `ordered` flag) — `parse_list()` now stops at a marker-type change, so `parse()`/`events()` and `StreamingParser` agree (fixture `mixed-list-markers`) | Wired (fixed 2026-07-31): `Writer` rewritten to emit incrementally per event instead of buffering into a `Vec<OwnedEvent>` and delegating to `emit::build()` in `finish()`; fully write-through except the link `\|` separator (in-place insert); ~5.7x faster (75→432 MB/s) |
| markua | Wired (validates the AST→event expansion layer; `events()` is parse()+eager-tree-build-then-walk) | Wired (fixed 2026-07-30): `parse_list()` had the identical structural bug as zimwiki's (copy-paste lineage, independently found) — merged a blank-line-separated unordered+ordered list pair into one mislabeled list at the whole-document `parse()` level; fixed the same way (fixture `comp-mixed-list-markers`) | Wired (fixed 2026-07-31): `Writer` rewritten to the rst-fmt pattern (shared output buffer + frame stack, in-place inserts for figure captions) instead of buffering into a `Vec<OwnedMarkuaEvent>` and delegating to `emit::emit()` in `finish()`; found and fixed a real, independent bug: `EndFigure` always built `caption: vec![]`, silently dropping captions — the new writer carries it correctly; ~4-7x faster. `MarkuaDoc::title`/`author`/`description` remain permanently `None` (a reader-side gap, `parse()` never populates them — out of scope here) |
| muse | Wired (eagerly materializes a `VecDeque` in `EventIter::new`, but a real independently-checkable walk over `&MuseDoc`) | Wired (fixed 2026-07-31): rewritten from buffer-until-`finish()` to a genuine line-buffered block splitter — `feed()` accumulates lines only until a top-level block boundary is confirmed (blank line / different-block-kind line / a tag block's own closing tag / a single-line construct), then immediately re-parses just that block's text via the new `parse::parse_blocks` (runs `Parser::parse_block_loop` without the document-header phase, so a `#`-led line mid-document is never misread as a header directive) and forwards its events — before `finish()`. Boundary classification reuses the same pure predicate functions `Parser::parse_block_loop` itself now calls (`heading_level`, `is_over_leveled_heading`, `is_horizontal_rule`, `is_unordered_list_start`, `is_ordered_list_item`, `is_definition_list_line`, `is_indented_code_start`, `is_footnote_def_start`, `tag_open_close`), so the two can't drift apart. Tag blocks (`<verse>`/`<example>`/etc.) do not support nesting in `parse()` itself; `StreamingParser` intentionally reproduces that rather than "fixing" it, to stay aligned with `events()`. Peak-memory guard: 10x paragraph count → peak ratio stayed well under 4x (vs. O(full document) before) | Wired (fixed 2026-07-31): `Writer` rewritten to emit incrementally per event instead of buffering into a `Vec<OwnedMuseEvent>` and delegating to `emit::build()` in `finish()`; straight-through except `Paragraph`'s O(1) parent-lookup terminator and O(field-count) metadata buffering. The previously-tracked expressiveness gap is also fixed: `MuseEvent` now has a `Metadata` variant, since `parse()` genuinely populates title/author/date/desc/keywords from real Muse syntax; ~1.8x faster |
| t2t | Wired (validates the AST→event expansion layer; `events()` is `parse()`+lazy-frame-stack-walk) | Wired (2026-08-03 follow-up): all three root causes named in the previous `KnownFailure` entry are fixed. `StreamingParser::feed_line` (`batch.rs`) now defers the blank-line flush decision for an accumulating `": "`-prefixed definition-list block, same one-line-lookahead pattern as rst-fmt's/djot-fmt's DefinitionList fixes. `Parser::try_parse_header` (`parse.rs`) now also rejects a first line starting with 1-5 `'='`s or `'['` as header title text even when unclosed — this was a genuine data-loss bug in `parse()`/`events()` themselves (an unclosed heading/link opener as line 1 caused the next two lines to be blindly swallowed as author/date even across a real block boundary), not a `StreamingParser` divergence. `StreamingParser::finish()` now replicates the whole-document parser's trailing-newline artifact for an EOF-terminated, never-closed fence. See TODO.md for the full writeup | Wired — rewritten to a single shared-buffer write-straight-through design (2026-07-31, mirroring `rst-fmt`'s `Writer`); every t2t construct turned out write-straight-through with no generic sibling-separator rule needed at all (see TODO.md); byte-identical to `emit()` over all fixtures including document-header, genuinely incremental; peak memory ~278x smaller than `parse()`+`emit()` on a synthetic document, though throughput is ~2.8x *slower* than the (very lightweight) builder — finer event-dispatch granularity outweighing the builder's simpler tree walk for this particular format, a genuine tradeoff not a harness artifact (see TODO.md) |
| pod | Wired (validates the AST→event expansion layer) | Wired (fixed 2026-07-31): `batch.rs`'s `StreamingParser` rewritten from buffer-all-bytes-then-parse-on-`finish()` to a line-buffered incremental block state machine (mirrors `rst-fmt`'s `batch.rs` shape) — headings/`=for`/`=encoding`/stray commands flush as single-line blocks, `=over`/`=back` lists track nesting depth (a `=cut` at any depth unwinds the whole list), `=begin`/`=end` regions accumulate to their literal `=end` line, paragraphs/verbatim blocks flush on `parse.rs`'s own boundary rules, and the cross-block `in_pod` flag is carried across flushes (paragraph/verbatim re-parses get a synthetic leading `=pod` line to reproduce it). Peak memory confirmed flat via a `thread_local!` allocator probe shared with `writer.rs`'s pre-existing one (`crate::alloc_probe`): a synthetic 200- vs 2000-section (10x) document holds ~2.25 KB peak either way; a throwaway before/after comparison measured the prior implementation at ~12-13x the input byte count (328 KB @ 25 KB input → 35.2 MB @ 2.67 MB input) against the new ~2.25-2.29 KB flat (~15,000x reduction at the largest size). Does not diverge from `events()` under adversarial chunking. The harness's original incrementality check (a fixed 50%-byte split of each real fixture) failed on 35 of 36 checked fixtures for a probe-methodology reason (the pod fixture suite is overwhelmingly one block per fixture, and a single block cannot emit any event before its own boundary is reached — same gap already documented for jats-fmt) rather than a real defect; replaced with a hand-built probe with a guaranteed-complete prefix (a full heading + a full paragraph) followed by deliberately unterminated trailing content, the same fix already applied this session to fb2-fmt/texinfo/xwiki — this passes cleanly, so `streaming_parser` is promoted to Wired and the `KNOWN_FAILURES` entry removed | Wired (fixed 2026-07-31): `Writer` rewritten to emit incrementally per event instead of buffering into a `Vec<OwnedEvent>` and delegating to `emit::build()` in `finish()`; entirely write-straight-through (POD has no computed prefixes, `Link`/verbatim blocks are self-contained); ~3.3x faster. Byte-identical check also surfaced an unrelated, real bug in the builder path itself: `emit.rs`'s `<`/`>` escaping used two sequential `String::replace` calls that corrupted each other's output — fixed to escape char-by-char |
| haddock | Wired (validates the AST→event expansion layer) | Wired — genuinely incremental with no divergence found: every block-termination rule in `parse.rs` depends only on the block's own content, never cross-block state, so isolated re-parse recovers the same boundaries | Wired (fixed 2026-07-31): `Writer` rewritten to emit incrementally per event instead of buffering into a `Vec<OwnedEvent>` and delegating to `emit::build()` in `finish()`; straight-through except `Property`'s lazy description-separator space (O(1) bool flag). The byte-identical check caught a real, independent bug: `events()` emits a redundant `Text` child inside `Link` that the builder never reads — fixed by suppressing it via a dedicated frame; ~2.9x faster |
| fountain | Wired (validates the AST→event expansion layer; `events()` returns `OwnedEventIter`) | Wired (fixed 2026-07-30): `StreamingParser` now owns exactly one `StartDocument`/`EndDocument` pair for the whole stream, filtering both out of every per-block re-parse; only the first accumulated block goes through full title-page detection (`crate::events::events`), every later block goes through the new `crate::events::events_body`/`crate::parse::parse_screenplay_only` (skips title-page parsing entirely, so a body line matching a title-page field name is no longer misread as metadata and swallowed) — new fixture `adv-body-line-looks-like-title-field` | Wired (fixed 2026-07-31): `Writer` rewritten to emit incrementally per event instead of buffering into a `Vec<OwnedEvent>` and delegating to `emit()` in `finish()`; mostly straight-through, with `Character`/`Transition` needing bounded per-block text buffering (uppercase transform) and title-page metadata needing O(field-count) buffering. The byte-identical check caught a real, independent bug: `flush_metadata_if_pending` fired on `StartDocument` itself, permanently discarding metadata before it arrived — fixed. ~1.33x faster than the old buffer-then-emit writer, though still slower than the plain builder (789us vs 357us/iter) |
| man | Wired (validates the AST→event expansion layer; `events()` is `EventIter::new(&parse(input).0)` eagerly collected — same pattern as t2t/pod/haddock/fountain. A real, previously-unknown bug was found and fixed while wiring this check: every inline container — Bold/Italic/Superscript/Subscript/Link — pushed a synthetic children-walking frame with a "dummy" `CloseKind::Paragraph` that was never actually inert, unconditionally emitting a spurious `EndParagraph` when the children frame was exhausted; fixed by adding a real `CloseKind::None` that emits nothing, replacing the dummy at all five inline-container sites. `ManEvent` now also carries a `Metadata` variant (title/section/date/source/manual), emitted once by `EventIter` right after `StartDocument`, mirroring t2t-fmt's `Event::Header`) | Wired (fixed 2026-08-03): `StreamingParser` used to re-parse each accumulated block in isolation via `events()`, which always wraps its output in its own `StartDocument`/`EndDocument` pair, so it emitted one such pair per block instead of one for the whole document (same root-cause class as t2t/fountain's pre-fix bug). Fixed the same way as fountain: `StreamingParser` now dispatches its own single `StartDocument` in `new()` and `EndDocument` in `finish()`, and `emit_block()` filters `StartDocument`/`EndDocument` out of each re-parsed block's forwarded events; unlike fountain, man-fmt's `events()` has no title-page-only-on-first-block wrinkle, so no `events_body()`-style second entry point was needed. Confirmed byte-identical to `events()` over all man fixtures under adversarial chunking | Wired (fixed 2026-08-03): `Writer` rewritten to a genuine incremental writer (single shared `out: String` buffer + a frame stack of marks/scalars) instead of buffering all events into a `Vec<OwnedManEvent>` and reconstructing the AST + calling `emit::build()` in `finish()`; almost entirely write-straight-through, with two bounded (not O(document)) buffered constructs — the `.TH` line's five fields (O(field count), via the new `ManEvent::Metadata`) and a heading's flattened text (O(heading text length) — `emit.rs`'s `Block::Heading` arm uses `extract_text()`, which drops all inline markup, not `build_inlines()`). Byte-identical to `build()` over all fixtures. Peak memory ~511x smaller than `parse()`+`build()` on a synthetic 5000-section document (4,235 bytes vs 2,162,703 bytes peak); throughput ~0.83x of the builder (noted honestly, not chased further) |
| native | NotYetWired: no `events()`/`EventIter` anywhere in `crates/formats/native/src/lib.rs` (its only source file, read in full) — only the eager whole-string `parse()`/recursive `build()`; nothing format-structural blocks adding one | NotYetWired: no `StreamingParser<H>` type exists in the crate at all | NotYetWired: no event-driven `Writer` exists; only the eager `build(doc: &NativeDoc) -> String` |
| csv | NotYetWired: no `events()`/`EventIter` anywhere in csv-fmt (ast.rs/parse.rs/emit.rs read in full, zero matches for `EventIter`/`StreamingParser`/`mod events`/`impl Iterator`) — CSV's flat, no-cross-row-state grammar has no structural barrier to one | NotYetWired: no `StreamingParser<H>` type exists in the crate at all; only the eager `parse(input: &str) -> (CsvDoc, Vec<Diagnostic>)` | NotYetWired: no event-driven `Writer` exists; only the eager `emit(doc: &CsvDoc) -> String` |
| tsv | NotYetWired: shares csv-fmt's exact module shape (ast.rs/parse.rs/emit.rs, no events module); same reasoning as csv | NotYetWired: no `StreamingParser<H>` type exists in the crate at all; only the eager `parse(input: &str) -> (TsvDoc, Vec<Diagnostic>)` | NotYetWired: no event-driven `Writer` exists; only the eager `emit(doc: &TsvDoc) -> String` |
| ris | NotYetWired: no `events()`/`EventIter` anywhere in `ris` (ast.rs/parse.rs/emit.rs read in full, zero matches) — RIS entries are self-contained (`TY`...`ER`) with no cross-entry parse state, so an entry-at-a-time iterator is structurally straightforward | NotYetWired: no `StreamingParser<H>` type exists in the crate at all; only the eager `parse(input: &str) -> (RisDoc, Vec<Diagnostic>)` | NotYetWired: no event-driven `Writer` exists; only the eager `emit(doc: &RisDoc) -> String` |

**Session tally (2026-07-30):** 26 formats moved from `NOT_YET_AUDITED` to a real, audited
`CAPABILITIES` entry across three passes this session — org, html, asciidoc, djot, texinfo,
fb2, textile, commonmark, gfm, markdown in the first pass; bbcode, creole, dokuwiki, jira,
mediawiki, tikiwiki, twiki, vimwiki, xwiki, zimwiki, markua, muse in a second, wiki-verticals
pass; then t2t, pod, haddock, fountain in a third follow-up pass — on top of the 4
pre-existing entries (rst, docx, pptx, xlsx) from the harness's initial wiring. 30 formats now
have a `CAPABILITIES` row and 33 remain in `NOT_YET_AUDITED`. `streaming_harness::KNOWN_FAILURES`
now has 45 entries total. None were weakened or hidden to make a check pass; every divergence
found a real, root-caused, tracked `KnownFailure` entry instead.

Headline findings across the second and third passes: bbcode-fmt was the first format in this
table whose `StreamingParser` was audited and found to be genuinely, not just nominally,
Wired; creole, dokuwiki, and jira followed the same pattern, with dokuwiki/jira additionally
having no cross-block parser state at all, so their adversarial-chunking equivalence check
needed no coarser-boundary caveat. mediawiki, tikiwiki, and twiki's `events()`/`StreamingParser`
were also Wired with no divergence found on any fixture; twiki's `events()` has a non-standard
signature (`&TwikiDoc` instead of raw input), tracked as a follow-up in TODO.md. vimwiki's
`StreamingParser` genuinely diverges from `events()` even feeding the whole input in one
`feed()` call — a list-boundary disagreement, not a chunk-boundary bug. xwiki's `events()` is
the first genuinely lazy pull-iterator found in this format family, unlike
zimwiki/markua/muse's eager materialize-then-walk. zimwiki and markua independently share an
identical `parse()`-level (not streaming-specific) block-merging bug in their `parse_list()`
functions — both accept either bullet or numbered marker in one loop with no transition check,
and skip blank lines instead of breaking. haddock's `streaming_parser` is the one fully `Wired`
(no `KnownFailure`) result found in the third pass — unlike t2t/fountain, every haddock
block-termination rule depends only on the block's own content, never cross-block state, so its
per-block isolated re-parse recovers the same boundaries `events()` finds inline. All sixteen
formats' streaming writers found across these three passes are architecturally hollow
buffer-then-emit, confirmed via the `ObservableSink` incrementality probe rather than assumed
from module docs.

**Merge reconciliation:** the two tallies immediately above were computed independently in
parallel worktrees against the same 49-entry `NOT_YET_AUDITED` baseline — one session covering
docbook/jats/tei/odt/ansi (5 formats, 12 `KnownFailure` entries), the other covering the 16
bbcode..fountain wiki/lightweight-markup formats (45 `KnownFailure` entries) — so their
"44 remain" / "33 remain" figures are each correct only against their own single-session diff,
not against the merged result. Recomputed directly from the merged
`streaming_harness::CAPABILITIES` / `NOT_YET_AUDITED` / `KNOWN_FAILURES` tables after merging
both branches: **35** formats now have a `CAPABILITIES` row (14 pre-existing + 5 + 16), **28**
remain in `NOT_YET_AUDITED` (49 − 5 − 16), and `KNOWN_FAILURES` has **57** entries total
(12 + 45). No format appears in both `CAPABILITIES` and `NOT_YET_AUDITED`, and every
`KnownFailure` refers to a format present in `CAPABILITIES` — verified programmatically, not
just asserted.

**Second merge reconciliation (2026-07-30, bugfix session):** two further parallel worktrees
fixed real bugs the checks above had found rather than just auditing/documenting them — one
covering docbook/jats/tei mismatched-end-tag detection, wiki-family `parse_list()`
ordered/unordered merging (zimwiki, markua, vimwiki, twiki, dokuwiki), docbook/jats/tei
entity-Text coalescing, fountain's spurious per-block Start/EndDocument, and ansi's spurious
ResetStyle/cross-chunk state loss; the other adding `Event` variants that closed real
expressiveness gaps in org-fmt (`Event::Metadata`), texinfo (`Event::Title`), djot-fmt
(`Event::LinkDef`), odf-fmt (seven new `OdfEvent` variants), and t2t (`Event::Header`). Neither
branch changed which formats have a `CAPABILITIES` row or which remain in `NOT_YET_AUDITED`, so
those two counts are unchanged from the paragraph above (**35** / **28**). `KNOWN_FAILURES`
shrank from 57 to **50** entries: seven were deleted outright because the bug they tracked is
fixed and the format+API is now genuinely `Wired` — docbook `streaming_parser`, tei
`streaming_parser`, vimwiki `streaming_parser`, zimwiki `streaming_parser`, markua
`streaming_parser`, fountain `streaming_parser` (all from the first branch), and org
`streaming_writer` (from the second). A further three were narrowed in place, not deleted,
because part of what they tracked is fixed but a distinct, still-open issue remains under the
same format+API: texinfo `streaming_writer` (title-drop fixed via `Event::Title`; hollow
buffer-then-emit remains), djot `streaming_writer` (`LinkDef`-drop fixed; hollow
buffer-then-emit remains), and t2t (`streaming_parser`: document-header misread fixed via
`Event::Header`, per-block spurious Start/EndDocument remains; `streaming_writer`: title/author/
date drop fixed, hollow buffer-then-emit remains). odf-fmt's `streaming_writer` entry was also
narrowed (mimetype/meta/styles/images resource-loss fixed; a much larger body-content
`OdfEvent`-vocabulary gap, 12/66 odt fixtures, remains open). ansi's `streaming_parser` and
`streaming_writer` entries were corrected in place rather than narrowed or deleted: two of
ansi's three original bugs are fixed, but each entry's remaining issue is a different, newly
distinct bug (an OSC-8-hyperlink-atomicity gap in `find_safe_boundary`, and a `parse()`/
`build()` fidelity gap for no-op SGR groups) rather than a subset of the original description.
No `KnownFailure` was deleted or narrowed without a fix landing in the same merge; verified by
re-running the full `streaming_harness` test suite after merging (`assert_or_known_failure`
panics if a tracked failure starts passing, which would catch a stale entry immediately).

**Streaming-writer incrementality sweep (2026-07-31):** the 16 wiki/lightweight-markup formats'
streaming `Writer`s (bbcode, creole, dokuwiki, jira, mediawiki, tikiwiki, twiki, vimwiki, xwiki,
zimwiki, markua, muse, textile, pod, haddock, fountain) were all rewritten from the
architecturally-hollow buffer-then-`finish()` pattern flagged above to genuinely incremental
per-event writers, following the `rst-fmt`/`ooxml-wml`/`ooxml-sml` template (a single shared
output buffer plus offset marks, an O(nesting-depth) frame stack, deferral only for constructs
whose prefix depends on content not yet seen). Classification was close to uniform across the
family: nearly every construct is write-straight-through; the handful of genuine deferrals were
small and local (an O(1) bool/`Option` flag, an O(1) parent-frame lookup, O(depth) list/
blockquote nesting counters, one real reordering case in xwiki's `[[label>>url]]` link syntax,
and O(field-count) title-page/document-metadata buffering) — no format in this family needed
true unbounded buffering. All 16 `streaming_writer` `KnownFailure` entries were removed (not
narrowed) as a result; `KNOWN_FAILURES` dropped from 50 to **34** entries. The byte-identical-
to-builder checks each writer got surfaced five previously-unknown, independent content bugs,
all fixed in the same sweep: markua's `EndFigure` silently dropping figure captions,
twiki's nested-list newline emitted before instead of after recursing into child lists,
haddock's `events()` emitting a redundant `Text` child inside `Link` that the builder never
read, fountain's `flush_metadata_if_pending` discarding title-page metadata by firing on
`StartDocument` itself, and pod-fmt's `emit.rs` corrupting escaped `<`/`>` via two sequential
`String::replace` calls that rewrote each other's output (the streaming writer's own
char-by-char escaping never had this bug, which is what exposed it). Measured before/after
throughput gains ranged roughly 1.3x-7x depending on format, with peak memory reductions in the
1,000x-40,000x range on large synthetic documents where a peak-memory regression test was
practical to write; a separate, real test-infrastructure bug was found and fixed alongside this
work — 12 of the 16 crates' new peak-memory tests tracked current/peak bytes via a
`static AtomicUsize` shared across `cargo test`'s parallel threads, letting unrelated
concurrently-running tests inflate the measured peak (confirmed as a real flake, not
hypothetical: a spurious 407x ratio under full-workspace `cargo test -q`), fixed by converting
to `thread_local!` counters. Not touched: `bbcode`'s and other formats' remaining
`streaming_parser`/`events` `KnownFailure` entries (out of scope for this writer-focused sweep),
and the `xwiki`/`textile`/`muse`/`pod` `streaming_parser` `KnownFailure` entries, which are
independent, pre-existing, still-open gaps.

**Third merge reconciliation (2026-07-31):** two more parallel worktrees both targeted hollow
streaming `Writer`s, against the same 50-entry `KNOWN_FAILURES` baseline — one rewriting
texinfo, djot-fmt, t2t, and org-fmt (removing the `texinfo`/`djot`/`t2t` `streaming_writer`
`KnownFailure` entries; org-fmt's `streaming_writer` had no entry to remove, since it was
already deleted in the second reconciliation above — that branch instead added the
incrementality probe org-fmt had never had, confirming the writer genuinely incremental), the
other covering the 16-format wiki/lightweight-markup sweep documented immediately above. Naive
arithmetic (50 − 3 − 16 = 31) and the naive "4 + 16 removed" framing some in-flight notes used
both undercount/overcount for the same reason: only 3, not 4, `streaming_writer` entries existed
for the first branch to remove. Recomputed directly from the merged `streaming_harness` tables,
not assumed: `CAPABILITIES` still has **35** rows and `NOT_YET_AUDITED` still has **28** entries
(neither branch added or removed a format row, only flipped existing `ApiState`s), and
`KNOWN_FAILURES` now has **31** entries total (50 − 3 − 16, no overlap — the two branches never
touched the same format). Table self-consistency reverified after merging: every format appears
in exactly one of `CAPABILITIES`/`NOT_YET_AUDITED`, every `KNOWN_FAILURES` entry's format+api
has a matching `CAPABILITIES` row, and there is a 1:1 correspondence between `KNOWN_FAILURES`
entries and `ApiState::KnownFailure(_)` occurrences — verified programmatically via
`streaming_harness`'s own self-consistency test, not just asserted.

**Fourth session (2026-07-31): muse probe repair + man wired.** `muse`'s
`streaming_parser` incrementality probe (the one call site of
`assert_streaming_parser_is_incremental` still using a fixture-driven 50%-byte split
instead of a hand-built synthetic sample — skipped in the earlier repair pass since
muse's `streaming_parser` was already `Wired`) was replaced with a hand-built sample,
matching the other six call sites; no `ApiState` changed. `man` was moved from
`NOT_YET_AUDITED` to `CAPABILITIES` (`events: Wired`, `streaming_parser: KnownFailure`,
`streaming_writer: KnownFailure`) — chosen for its 29-fixture corpus, the largest
`NOT_YET_AUDITED` entry with a confirmed real crate that was picked up this session. A
real, previously-unknown bug in `man-fmt`'s own `events.rs` was found (via this
harness's events()-vs-AST-projection check, first exercise ever) and fixed in-session
as a small, well-scoped change: see the `man` row above for the `CloseKind::None` fix.
`CAPABILITIES` now has **36** rows, `NOT_YET_AUDITED` has **27** entries, and
`KNOWN_FAILURES` has **26** entries (24 + man's 2). Separately, the "no `-fmt` crate"
claim for `epub`/`bibtex`/`biblatex`/`csl-json`/`endnotexml`/`opml`/`ipynb`/`typst`/
`pandoc-json`/`latex` was independently verified (not assumed) by listing
`crates/formats/` and confirming no matching entry exists for any of the ten; for
`latex`, `crates/readers/rescribe-read-latex/src/handwritten.rs` (895 lines) and
`src/treesitter.rs` (662 lines) were both read in full and confirmed to contain real
parsing logic directly in the adapter crate, a CLAUDE.md violation left undocumented
until now — see `NOT_YET_AUDITED`'s inline comments and TODO.md. Fixing it is out of
scope for this pass. See TODO.md for the full writeup, including the two verified
`man-fmt` `KnownFailure` repro commands.

**`rtf-fmt` wired** (same session, immediately after `man`; this narrative paragraph was
not added at the time — recorded here for the record): `events: Wired`,
`streaming_parser: KnownFailure` (`batch::StreamingParser` buffers all fed bytes and only
calls `sem_events::events()` once inside `finish()` — a confirmed buffer-then-finish stub,
`feed()` alone delivers zero events), `streaming_writer: KnownFailure` (`writer::Writer`
only accepts the low-level `TokenEvent` stream, not the crate's own semantic `Event` type
`events()`/`StreamingParser` produce — no semantic-event-consuming writer exists at all;
even re-tokenizing `emit()`'s own output and feeding it back through `Writer` doesn't
reproduce the canonical bytes, a delimiter-space-policy mismatch confirmed on fixture
`adjacent_bold`). `CAPABILITIES` reached **37** rows, `NOT_YET_AUDITED` **26**,
`KNOWN_FAILURES` **28** after this. See TODO.md and the `rtf` row above.

**`native`/`csv`/`tsv`/`ris` audited (2026-08-01, fourth pass on this harness).** All four
were confirmed, by reading every source file in each crate in full (`native`'s single
`lib.rs`; `csv-fmt`/`tsv-fmt`/`ris`'s `ast.rs`+`parse.rs`+`emit.rs`) and grepping each for
`StreamingParser`/`EventIter`/`mod events`/`mod batch`/`mod writer`/`impl Iterator` (zero
matches in all four, and zero dependencies in each `Cargo.toml` — no library could be
hiding a streaming mode behind), to have **only** `parse()` (AST reader) and
`emit()`/`build()` (AST builder) — no `events()`, `StreamingParser<H>`, or streaming writer
exists in any of the four crates. All twelve cells are declared `NotYetWired`, not
`NotApplicable`: unlike html-fmt/commonmark-fmt's genuine, documented, spec-imposed
barriers, nothing about CSV/TSV/RIS (flat, record- or line-oriented, no cross-row/
cross-entry parser state) or `native` (a small recursive tree-of-nodes debug format) makes
a chunked reader or an event-driven writer structurally impossible — they simply haven't
been built, and building three new APIs from scratch across four crates is a substantial
body of work, not a small in-scope defect fix, so it was left as an honest, specific gap
rather than attempted here. This also corrects a stale, speculative example in
`streaming_harness.rs`'s own `ApiState::NotApplicable` doc comment, written before any of
these four crates had actually been read, which cited "csv/tsv/ris/native have no
meaningful streaming writer" as a `NotApplicable` precedent — that comment has been
corrected alongside this entry to point at html-fmt's genuine structural example instead.
No defects were found or fixed in the four crates' existing `parse()`/`emit()` — all four
are already marked production/fuzz-clean elsewhere in this document, and no streaming code
existed to audit for bugs. `CAPABILITIES` now has **41** rows, `NOT_YET_AUDITED` has **22**
entries, and `KNOWN_FAILURES` is unchanged at **28** entries (no `KnownFailure` was added —
every one of the twelve new cells is `NotYetWired`, not a failing check).

### Remaining hand-written formats (crate exists, API not started)

| Crate | ast | stream | batch | w-stream | w-build |
|-------|-----|--------|-------|----------|---------|
| typst (TBD) | | | | | |
| ansi-fmt | ast.rs parse.rs emit.rs | Span+Diagnostic; infallible parse; strip_spans() | fuzz_ansi_reader + fuzz_ansi_roundtrip | – | – |
| man-fmt | ast.rs parse.rs emit.rs | Span+Diagnostic; infallible parse | fuzz_man_reader (2M runs) fuzz_man_roundtrip (855K runs) | – | – |
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

## Streaming reader/writer fidelity inventory (audited 2026-07-28)

Full sweep of all 43 crates under `crates/formats/`, triggered by `rst-fmt`'s
streaming `Writer` measured ~7-8x slower than its builder `emit()` — root cause:
it reconstructed a full `Block`/`Inline` subtree per top-level block via a frame
stack and called the same `build_block` the builder uses. That is the writer-side
"fake streaming" pattern CLAUDE.md explicitly rejects ("A fake streaming API...
builds the full AST internally then wraps it... 'good enough for conversion' is not
a valid reason"). This sweep checks every crate for the same shape on both the
writer side (streaming `Writer`) and the reader side (`StreamingParser`, `events()`).
Method: read production `lib.rs`/`writer.rs`/`batch.rs`/`events.rs` directly, not
`Cargo.toml` features or this doc's own prior stage claims. Full findings, code
quotes, and per-crate detail: `/tmp` audit transcript folded into this section below.

**25 of 43 crates (58%) have a hollow streaming writer** — buffers all events,
reconstructs a full AST via a frame/`DocBuilder` stack, then calls the builder's own
`emit`/`build` function, exactly the rst-fmt anti-pattern: `bbcode-fmt`, `creole`,
`dokuwiki`, `asciidoc`, `djot-fmt`, `fountain-fmt`, `haddock-fmt`,
`jira-fmt`, `man-fmt`, `markua`, `mediawiki-fmt`, `muse-fmt`, `pod-fmt`, `org-fmt`,
`ooxml-pml`, `ooxml-wml`, `odf-fmt`, `textile-fmt`, `texinfo`, `tikiwiki`, `twiki`,
`vimwiki-fmt`, `xwiki`, `zimwiki`, `t2t`. (`commonmark-fmt` was in this list at the
original 2026-07-28 audit; fixed 2026-08-03, see the "Clean" bullet below.)

**`StreamingParser` buffers O(full input)** (violates the contract; bounded
per-token/per-block buffering like docbook-fmt's or rst-fmt's is fine) in:
`bbcode-fmt`, `creole`, `dokuwiki`, `muse-fmt`, `texinfo`, `textile-fmt`, `xwiki`,
`ooxml-wml` (documented, "future work"), `pod-fmt` (documented, weak "docs are
small" rationale), `fb2-fmt` (documented, no formal exemption but a real gap —
`docbook-fmt` proves bounded XML streaming is achievable with the same underlying
`quick_xml` library). `html-fmt` also buffers-all but with a structurally honest
justification (HTML5 tree construction can rearrange already-seen nodes — foster
parenting/adoption agency — matching html5ever's own architecture), so it is not
counted as a violation.

**`events()` derived from `parse()`+walk** (the ADR 0003 violation) in: `bbcode-fmt`,
`dokuwiki`, `tikiwiki`, `creole`, `fountain-fmt`, `haddock-fmt`, `jira-fmt` (its own
`EagerEventIter` name self-admits it), `man-fmt`, `markua` (doc comment claims "the
parser IS the iterator" while `EventIter::new` calls `p.parse()` immediately —
contradicts itself), `mediawiki-fmt`, `muse-fmt`, `pod-fmt` (dead `unreachable!()`
stub in its own internal `events()` helper), `odf-fmt` (pre-drains into a
`VecDeque` before returning — O(full event stream), same effective cost as
materializing the AST despite its module doc's claim otherwise), `texinfo`,
`textile-fmt`, `t2t`. This is still an open architectural gap for all of these —
none of them are a genuine standalone incremental parser at the free-function
`events(input)` entry point.

**Memory-safety/leak subset fixed 2026-07-29** (verified and corrected in a
follow-up pass; the underlying "derived from `parse()`" architecture gap above
is *not* resolved by this fix and remains tracked in TODO.md):
- `dokuwiki` — `events::InputEventIter` previously used `unsafe { transmute }` to
  build a self-referential struct around a locally-parsed `DokuwikiDoc`; the
  reference was taken *before* the doc was moved into the returned struct, so it
  could dangle. This was genuinely unsound, not just a performance smell. Fixed
  by eagerly collecting into an owned `Vec<OwnedEvent>` inside `new()` (no
  self-reference, no `unsafe`); `events::EventIter` (the real O(depth) walker
  over a caller-supplied `&DokuwikiDoc`) was already sound and is unchanged.
- `man-fmt` — `events::events()` used `Box::leak` to manufacture a `'static`
  reference to the parsed `ManDoc`, permanently leaking one `ManDoc` per call.
  This was a genuine memory leak (safe Rust, but a real resource leak), not UB.
  Fixed the same way as `dokuwiki`: eager collection into an owned `Vec` inside
  the function, so the parsed doc is dropped normally. `events::EventIter`
  (O(depth) walker over a caller-supplied `&ManDoc`) was already sound.
- `bbcode-fmt` — on inspection, `events()` itself had no `unsafe` and was not
  self-referential (every event it builds is already `Cow::Owned`, so the `'a`
  lifetime on the returned `EventIter<'a>` was vacuous). The one `unsafe {
  transmute }` was in `Event::into_owned()`'s catch-all arm converting
  lifetime-free variants — currently sound (transmuting `Event<'a>` to
  `Event<'static>` is safe when the matched variant provably holds no `'a` data)
  but fragile: a future `Cow`-bearing variant added without updating that arm
  would silently mis-convert. Replaced the catch-all with an exhaustive
  explicit match so this is a compile error instead of a latent hazard.
- `tikiwiki` — the `unsafe { transmute::<Event<'static>, Event<'a>> }` in
  `EventIter::new` was checked and found to be actually sound (widening a
  `'static` lifetime to any `'a` cannot dangle), just unnecessary — all pushed
  events already own their data. Removed by making `emit_block`/`emit_inlines`
  generic over the output lifetime directly.

All four crates now carry `#![deny(unsafe_code)]` at the crate root (with a
narrowly-scoped `#[allow(unsafe_code)]` on man-fmt's test-only `GlobalAlloc`
harness), each has a regression test targeting its specific issue
(`man-fmt::events::tests::test_events_no_per_call_leak` is an allocation-growth
guard verified to fail against the old `Box::leak` code; `dokuwiki`/`tikiwiki`
have iterator-lifetime-churn tests; `bbcode-fmt` has an exhaustive
`into_owned()` round-trip test), and full workspace `cargo test`/`cargo clippy
-D warnings` pass. `cargo miri` was not available in the dev shell (no
`rustup`, and `cargo miri` isn't otherwise installed), so the dokuwiki fix
was verified by code inspection and the borrow checker rather than by an
actual Miri run.

**Feature-declared-but-missing modules: none found** — every declared
`reader-streaming`/`reader-batch`/`writer-streaming` feature has some code behind it
everywhere. But real feature *gating* is nearly absent: only `commonmark-fmt` and
`creole` actually gate `mod events`/`batch`/`writer` behind `#[cfg(feature = ...)]`;
every other crate compiles all modules unconditionally, so the Cargo.toml flags are
cosmetic elsewhere.

**No streaming API at all (honest gap, not a violation):** `csv-fmt` (no
`[features]`, no supporting files), `ris`, `tsv-fmt`, `native` (pre-vertical, only
`parse()`/`build()` exist).

**Clean / model implementations:** `docbook-fmt`, `jats-fmt`, `tei-fmt` (direct
`quick_xml` passthrough both ways, bounded `StreamingParser`, genuinely independent
`events()`), `ansi-fmt` (O(1)-state writer, bounded-boundary `StreamingParser`),
`ooxml-sml` (the one clean writer among the zip/OPC formats — proves incremental
writers are achievable there too, unlike `ooxml-pml`/`ooxml-wml`/`odf-fmt`),
`rst-fmt` (confirmed genuinely fixed — writer flushes each completed top-level
block via `stack: Vec<Frame>`, not a whole-document rebuild; has the only
`no_orphan_modules.rs` guard against silent `mod` drops from bad merges),
`html-fmt` (writer clean; `StreamingParser`'s full-buffer behavior is an honestly
documented, structurally-justified HTML5 limitation), `commonmark-fmt` (writer
fixed 2026-08-03 — same `stack: Vec<Frame>` + single shared `out: String` buffer
shape as `rst-fmt`; flushes each completed top-level block; byte-identical to
`emit()` for every construct exercised in the crate's own tests; `events()`/
`StreamingParser` untouched — the latter is the sanctioned pulldown-cmark
exemption, the former still has its own pre-existing, separate bugs, now three
instead of two — see the commonmark/events KnownFailure in
`rescribe-fixtures`).

**Most consequential single finding:** `ooxml-wml` (DOCX) — CLAUDE.md names OOXML
(DOCX/XLSX/PPTX) as the priority target for the full three-API architecture because
these routinely exceed RAM on large corpora, yet its writer is hollow (buffers
`OwnedWmlEvent`s, reconstructs full `Paragraph`/`Run`/`Table`/`TableRow`/
`TableCell` AST via a `WmlFrame` stack into the same `DocumentBuilder` the
non-streaming path uses) and its `StreamingParser` also buffers the whole input.
`ooxml-pml`'s writer is additionally lossy (flattens shape/table-cell text to plain
strings, discarding geometry for most event types).

**Severity ranking (worst → clean), by how many of the three reader/writer surfaces
are affected:**
1. Writer + `events()` + `StreamingParser` all hollow: `texinfo`, `textile-fmt`,
   `bbcode-fmt`, `dokuwiki`, `tikiwiki`, `pod-fmt`, `man-fmt`, `xwiki`, `muse-fmt`,
   `ooxml-wml`, `ooxml-pml`.
2. Writer + `events()` hollow, `StreamingParser` legitimately bounded: `fountain-fmt`,
   `haddock-fmt`, `jira-fmt`, `markua`, `mediawiki-fmt`, `t2t`, `twiki`,
   `vimwiki-fmt`, `zimwiki`, `creole`.
3. Writer hollow only, both reader-side APIs legitimate: `asciidoc`, `djot-fmt`,
   `odf-fmt`, `org-fmt`.
4. No streaming API claimed, honest gap: `csv-fmt`, `ris`, `tsv-fmt`, `native`.
5. Clean: `docbook-fmt`, `jats-fmt`, `tei-fmt`, `ansi-fmt`, `ooxml-sml`, `rst-fmt`,
   `commonmark-fmt` (writer fixed 2026-08-03),
   `html-fmt` (one documented exception), `fb2-fmt` (one undocumented-but-honest gap
   on `StreamingParser` only).

Out of scope (shared support libraries / non-format crates, no `events()`/streaming
API of their own to evaluate): `ooxml-dml`, `ooxml-omml`, `ooxml-opc`, `ooxml-xml`,
`xml-entities`.

This inventory does not fix anything — it is a map for prioritizing the next
vertical's streaming-API work, per CLAUDE.md's "work one vertical to completion"
rule. Given the severity ranking, `ooxml-wml` (DOCX) is the highest-priority fix by
consequence; the Tier-1 wiki/small-format family is the highest-count fix by breadth.

### Update 2026-07-29 — `ooxml-wml` writer fixed and measured

The `ooxml-wml` **writer** finding above is closed. `WmlWriter` now emits each
event straight into the open `word/document.xml` ZIP entry through a fixed 64 KiB
window; there is no event buffer and no AST reconstruction. Its severity-ranking
entry moves from tier 1 to "clean" on the writer axis (its `StreamingParser` gap,
below, keeps it out of the clean list overall).

Measured, release build, 100k paragraphs, discarding sink, inputs prepared outside
the timed region (`crates/formats/ooxml-wml/examples/streaming_writer_throughput.rs`
and `tests/streaming_writer_memory.rs` reproduce both):

| | peak live heap, 1k paras | peak live heap, 100k paras | growth | vs `DocumentBuilder` |
|---|---|---|---|---|
| before | 1,865,922 B | 160,263,096 B | 85.9x | 7.74x slower (first incremental cut) |
| after | 486,474 B | 486,474 B | 1.00x | 0.52x — i.e. 1.9x *faster* |

The residual 486 KB is the deflate window plus the output buffer, both fixed. The
"7.74x slower" figure is the intermediate state where each tag was handed to the
deflate encoder individually; the shipped version buffers a 64 KiB window, which is
O(1) and not a return to document-sized buffering.

Genuinely deferred in the writer, and why (each is a *count*, not a content size):
`word/_rels/document.xml.rels` (a relationship is only known once the event
referencing it is seen; written after the body — ZIP entry order is not significant
to OPC consumers), `[Content_Types].xml` (same shape, written by
`PackageWriter::finish`), and the ZIP central directory (the container's own
structure, written by `ZipWriter`). Everything else — paragraphs, runs, text,
breaks, hyperlinks, tables, rows, cells, images, footnote/endnote references — is
straight-through. Image bytes registered *after* the first event are held until
`finish()` because a ZIP archive permits only one open entry at a time; registering
before the first event (the documented usage) retains nothing.

Three reader bugs surfaced while building the round-trip test and were fixed in the
same pass — `events()` was never exercised by any test or caller, so the earlier
audit's "the one clean piece" verdict was structural, not behavioural:
`<w:document>` was treated as an untracked element and skipped wholesale, so
`events()` returned three events for any real `word/document.xml`; escaped text came
back verbatim (`a &lt; b`) because quick-xml 0.39 surfaces entity references as
separate `GeneralRef` events; and `<w:p></w:p>` pushed a stack frame nothing popped,
desynchronising every later end event.

**Correction to this inventory's `ooxml-sml` entry.** `ooxml-sml` is listed above as
"the one clean writer among the zip/OPC formats". That is right about what it
*avoids* — no event buffer, no AST-frame reconstruction — but wrong about its memory
class. `SmlWriter` holds a `WorkbookBuilder` and calls `sheet.set_cell(...)` per
cell, so the entire workbook accumulates in memory and is written only at
`finish()`: O(full document), not O(nesting depth). It is a *better* shape than
`ooxml-wml`'s was (no double materialisation) but it is not an incremental writer,
and it should not be cited as proof that the zip/OPC container is not the obstacle —
`ooxml-wml` now is that proof. `ooxml-sml`'s writer needs the same rework.

**`ooxml-wml`'s reader-batch gap is unchanged and is fenced, not fixed.** See
TODO.md for the precise boundary; in short, the zip container is *not* the blocker
(zip 7's `ZipStreamReader` walks local file headers sequentially without the central
directory), but `BatchParser::feed`/`finish` is a push API returning a materialised
`Document<Cursor<Vec<u8>>>`, and a genuine bounded reader is a new
`StreamingParser<H: Handler>` surface that ooxml-wml does not have at all — a
separate vertical, not a patch.

### Update 2026-07-29 — `ooxml-pml` geometry loss fixed; writer hollowness still open

`ooxml-pml`'s **lossiness** finding above (shape/table-cell text flattened, geometry
discarded for most event types) is narrowed and the geometry half is closed.
Verified directly (not assumed) that the loss was confined to the events()/
`PmlWriter` pair: the AST/`parse()` path (`Presentation`/`Slide`) wraps the
generated `types::Shape` and already round-trips full `spPr` geometry fidelity
through the generated FromXml/ToXml. `PmlEvent::StartShape` now carries a
`ShapeGeometry` (preset name + `<a:avLst>` adjustment values, modeled; `<a:custGeom>`
raw-preserved verbatim via `ooxml_xml::RawXmlElement`) alongside the pre-existing
`ShapeTransform`; `events.rs` populates it, `PmlWriter` emits it via the full
`ShapeBuilder` instead of hardcoding `Rect`. 8 tests in
`ooxml-pml/tests/streaming_writer_geometry.rs`. Two further, cross-cutting bugs
surfaced while writing those tests and are fenced (not fixed) in TODO.md: (1)
`ooxml-dml`'s generated `CTPath2D` parser/serializer put point coordinates directly
on `<a:moveTo>`/`<a:lnTo>` rather than a nested `<a:pt>`, so real-PowerPoint-shaped
custGeom fails to round-trip through the typed `CTCustomGeometry2D` on the writer
path (raw capture on the reader side is unaffected); the writer falls back to `Rect`
in that case rather than corrupting output. (2) `events.rs`'s true SAX reader was
never exercised by any test before now and does not treat `<p:txBody>` (or the
`p:sld`/`p:cSld`/`p:spTree` wrapper elements needed to drive it from a real slide
part) as transparent containers, so real slide text is currently unreachable
through `events()` — a `<p:sld>`-wrapped fixture through `pml_events()` produces
zero shapes.

`ooxml-pml`'s **writer-hollowness** finding (buffers every `OwnedPmlEvent` into a
`Vec`, replays it through a hand-rolled little state machine to reconstruct calls
against `PresentationBuilder`, which does the actual `write()` at `finish()`) is
unchanged — still O(full input), still delegates to the builder's emit path. Not
started this pass, deliberately fenced: see TODO.md for the classification-so-far
and the `ooxml-wml` commits (`849480a98c`, `c966059d32`) to use as a template.

### Update 2026-07-29 — `ooxml-sml` writer fixed and measured

The correction above is closed. `SmlWriter` no longer holds a `WorkbookBuilder`;
each `SmlEvent` is written straight into the open `xl/worksheets/sheetN.xml` ZIP
entry through the same fixed 64 KiB output window `ooxml-wml` uses, reusing the
`Row`/`Cell` props structs' own generated `ToXml::write_attrs` to open tags rather
than reconstructing an AST.

Measured, release build, 100k rows x 3 cells (20 distinct strings), discarding
sink, inputs prepared outside the timed region
(`crates/formats/ooxml-sml/tests/bench_streaming.rs`, `#[ignore]`-gated;
`tests/streaming_writer_memory.rs` is the permanent memory-guard test):

| | peak live heap | wall time |
|---|---|---|
| before (streaming, via `WorkbookBuilder`) | 233,578,753 B (222.76 MB) | 456.9 ms |
| after (streaming, incremental) | 484,831 B (0.46 MB) | 137.3 ms |
| `WorkbookBuilder` path itself (for reference) | 296,247,374 B (282.5 MB) | 390.9 ms |

481.8x less peak memory, 3.3x faster than the old streaming writer, 2.9x faster
than the `WorkbookBuilder` path it used to wrap. Unlike `ooxml-wml`, no per-tag
throughput regression was hit during the rework — the fixed output window was
applied from the start rather than discovered after profiling a slowdown.

Genuinely deferred, and why (each a *count*, not a content size): `xl/workbook.xml`
+ its `.rels` (the sheet list — O(sheet count) — is only complete once every
`StartWorksheet` has been seen); `xl/sharedStrings.xml` (string *values* are
interned into a dedup table incrementally — a new string's index is assigned and
written into the sheet XML the moment it is first seen, and is never renumbered —
but the part listing every distinct string, O(distinct strings), can only be
written once streaming ends — this is the same bound SST deduplication always
costs, streaming or not); `[Content_Types].xml` and the ZIP central directory, same
shape as `ooxml-wml`. `<dimension>` and `<row spans="...">` are omitted outright
(not deferred) — both are optional per ECMA-376 §18.3.1.35 and would otherwise
require buffering a whole sheet's cell references before its first `<row>` could be
written. Everything else in the `SmlEvent` surface — worksheets, rows, cells
(reference, style index, type, value, formula), inline-string fragments — is
straight-through; styles/charts/comments/pivot tables/merged cells have no
`SmlEvent` representation at all (`WorkbookBuilder`-only), so they are out of scope
for this writer rather than deferred by it.

Two pre-existing fidelity gaps were fixed as a natural consequence of the rework:
row attributes (including the row number itself) and cell `style_index` were
previously dropped entirely by the event-driven writer (the old code only ever
read `props.reference`/`props.cell_type` off `StartCell`, and grouped
`StartRow { .. }` into a no-op match arm); both now pass through, since the
incremental writer has direct access to the full `Row`/`Cell` props at exactly the
point it needs to emit them.

`ooxml-sml`'s severity-ranking entry moves from "writer wrongly cited as clean, was
actually O(document))" to genuinely clean on the writer axis. Its reader side
(`StreamingParser<H>`) remains out of scope for this fix — filed in TODO.md as
separate follow-up work, same boundary as `ooxml-wml`'s reader-batch gap above.

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
