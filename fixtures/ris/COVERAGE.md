# RIS Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

**Coverage-completeness caveat (2026-07-28):** the checklist below is a hand-curated list of
constructs, not yet verified against a spec-derived, machine-readable construct index. An
audit of `fixtures/docbook/COVERAGE.md` and `fixtures/jats/COVERAGE.md` against authoritative
element indexes found hundreds of element names enumerated nowhere, moving denominators
mid-session purely from incidentally-noticed gaps -- a ratio over a hand-written list like this
one is not a coverage measurement. See `docs/format-audit.md`'s "Construct Coverage (CC)"
section for the full evidence; this format's `CC` status there is `U` (unverified) until a
construct registry (in design, see `docs/adr/`) checks this list against the format's own
spec.

RIS format is defined by Research Information Systems. Each record begins with a `TY` tag
and ends with `ER`. Fields are two-letter tags followed by two spaces, a dash, two spaces,
and the value.

Records parse into `bibliography`/`bibliography_entry`/`bibliography_field` IR nodes (see
ADR 0005 and `rescribe_std::node`), not the legacy flat `definition_list` shape. Each field
node carries both `field:role` (the semantic vocabulary) and `ris:field` (the exact RIS tag).
`SP`/`EP` map directly to `page_first`/`page_last` (RIS already keeps them as separate tags,
unlike BibTeX/CSL-JSON's combined page string). Any RIS tag with no `field:role` equivalent
(`AB`, `N1`/`N2`, `KW`, `AN`, ...) round-trips as a `misc` field distinguished by `ris:field`.

## Record types (TY tag)

### Common types
- [x] JOUR (journal article) — `article`
- [x] BOOK — `book`
- [x] CONF (conference paper) — `conference`
- [x] THES (thesis) — `thesis`
- [x] ELEC (electronic/web) — `with-url`
- [ ] ABST (abstract) — (missing)
- [ ] ADVS (audiovisual material) — (missing)
- [ ] AGGR (aggregated database) — (missing)
- [ ] ANCIENT (ancient text) — (missing)
- [ ] ART (art work) — (missing)
- [ ] BILL (bill/resolution) — (missing)
- [ ] BLOG (blog) — (missing)
- [ ] CASE (case) — (missing)
- [ ] CHAP (book chapter) — (missing)
- [ ] CHART (chart) — (missing)
- [ ] CLSWK (classical work) — (missing)
- [ ] COMP (computer program) — (missing)
- [ ] CPAPER (conference paper — alias) — (missing)
- [ ] CTLG (catalog) — (missing)
- [ ] DATA (data file) — (missing)
- [ ] DBASE (online database) — (missing)
- [ ] DICT (dictionary) — (missing)
- [ ] EBOOK (electronic book) — (missing)
- [ ] ECHAP (electronic book section) — (missing)
- [ ] EDBOOK (edited book) — (missing)
- [ ] EJOUR (electronic article) — (missing)
- [ ] EQUA (equation) — (missing)
- [ ] FIGURE (figure) — (missing)
- [ ] GEN (generic) — (missing)
- [ ] GOVDOC (government document) — (missing)
- [ ] GRANT (grant) — (missing)
- [ ] HEAR (hearing) — (missing)
- [ ] ICOMM (internet communication) — (missing)
- [ ] INPR (in press) — (missing)
- [ ] JFULL (journal, full) — (missing)
- [ ] MAP (map) — (missing)
- [ ] MANSCPT (manuscript) — (missing)
- [ ] MGZN (magazine article) — (missing)
- [ ] MPCT (motion picture) — (missing)
- [ ] MULTI (online multimedia) — (missing)
- [ ] MUSIC (music score) — (missing)
- [ ] NEWS (newspaper) — (missing)
- [ ] PAMP (pamphlet) — (missing)
- [ ] PAT (patent) — (missing)
- [ ] PCOMM (personal communication) — (missing)
- [ ] RPRT (report) — (missing)
- [ ] SER (serial) — (missing)
- [ ] SLIDE (slide) — (missing)
- [ ] SOUND (sound recording) — (missing)
- [ ] STAND (standard) — (missing)
- [ ] STAT (statute) — (missing)
- [ ] STD (standard — alias) — (missing)
- [ ] UNPB (unpublished work) — (missing)
- [ ] VIDEO (video recording) — (missing)
- [ ] WEB (web page) — (missing)

## Tags (fields)

### Identifier / type tags
- [x] TY (type) — all fixtures
- [x] ID (reference ID) — `article` (used as record key)

### Author / contributor tags
- [x] AU (author, single) — `article`
- [x] AU (multiple — repeated tag) — `multi-author`
- [ ] A1 (first author — alias) — (missing)
- [ ] A2 (secondary author / editor) — (missing)
- [ ] A3 (tertiary author) — (missing)
- [ ] A4 (subsidiary author) — (missing)
- [ ] ED (editor) — (missing)

### Title tags
- [x] TI (title) — `article`
- [ ] T1 (primary title — alias) — (missing)
- [ ] T2 (secondary title / container) — (missing)
- [ ] T3 (tertiary title / series) — (missing)
- [ ] BT (book title — BibTeX compat) — (missing)
- [ ] CT (caption title) — (missing)
- [ ] ST (short title) — (missing)
- [ ] TT (translated title) — (missing)

### Date tags
- [x] PY (publication year) — `article`
- [ ] Y1 (primary date — alias for PY) — (missing)
- [ ] Y2 (access date) — (missing)
- [ ] DA (date) — (missing)

### Source / publication tags
- [x] JO (journal name) — `article`
- [ ] JF (journal name — full) — (missing)
- [ ] JA (journal name — abbrev) — (missing)
- [ ] J1 / J2 (alternate journal names) — (missing)
- [x] PB (publisher) — `book`
- [ ] CY (city/place) — (missing)
- [ ] PP (place of publication) — (missing)

### Volume / issue / page tags
- [ ] VL (volume) — (missing)
- [ ] IS (issue) — (missing)
- [ ] SP (start page) — (missing)
- [ ] EP (end page) — (missing)
- [ ] LP (last page — alias) — (missing)
- [ ] CP (chapter / issue — alias) — (missing)

### Identifier tags
- [x] DO (DOI) — `with-doi`
- [x] UR (URL) — `with-url`
- [ ] SN (ISSN/ISBN) — (missing)
- [ ] AN (accession number) — (missing)
- [ ] M1 / M2 / M3 (misc fields) — (missing)

### Content tags
- [ ] AB (abstract) — (missing)
- [ ] N1 (notes) — (missing)
- [ ] N2 (abstract — alias) — (missing)
- [ ] KW (keywords — repeated) — (missing)
- [ ] RP (reprint status) — (missing)

### Record terminator
- [x] ER (end of record) — `article`
- [x] missing ER (implied end) — `rare-no-er`

## Special syntax

- [ ] multiple records in one file — (missing)
- [ ] repeated tags for multi-value fields — `multi-author` (AU repeated)
- [ ] CR/LF vs LF line endings — (missing)
- [ ] UTF-8 encoded values — (missing)
- [ ] tag with no value (empty) — (missing)

## Composition (integration)

- [ ] journal article with volume, issue, pages, and DOI — (missing)
- [ ] book with publisher, city, and ISBN — (missing)
- [ ] multiple records in one file — (missing)

## Adversarial

- [x] empty file — `adv-empty`
- [ ] record without TY tag — (missing)
- [ ] unknown TY value — (missing)
- [ ] unknown tag — (missing)
- [ ] malformed tag line (no `  - ` separator) — (missing)
- [ ] record without ER terminator (last record) — `rare-no-er`
- [ ] truncated file mid-record — (missing)

## Pathological

- [ ] file with 1000 records — (missing)
- [ ] record with 100 keyword tags — (missing)
- [ ] very long abstract field — (missing)
