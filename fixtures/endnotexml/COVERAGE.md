# EndNote XML Fixture Coverage

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

EndNote XML uses a proprietary XML schema exported by EndNote. The root element is `<xml>`,
records are inside `<records>`, each record is a `<record>` element with a `<ref-type>`
child specifying the reference type by name and numeric code.

Records parse into `bibliography`/`bibliography_entry`/`bibliography_field` IR nodes (see
ADR 0005 and `rescribe_std::node`), not the legacy flat `definition_list` shape. Each field
node carries both `field:role` (the semantic vocabulary) and `endnote:field` (the exact
source element path, e.g. `titles/secondary-title`, `urls/related-urls/url`). EndNote XML
is the one of these four formats whose fields can carry `<style face="...">` markup runs —
those become real `emphasis`/`strong`/`underline`/`superscript`/`subscript` inline nodes
(see `rare-style-markup`, `style-variety`) instead of being flattened to plain text.

**2026-08-04:** `rescribe-read-endnotexml`/`rescribe-write-endnotexml` were relocated onto a
new standalone `endnotexml-fmt` crate (`crates/formats/endnotexml-fmt`), following the
`opml-fmt` template — see that crate's `ast.rs`/`events.rs` module docs for the native AST/
event design. The IR mapping documented above is unchanged; this pass added dedicated
fixtures for constructs the old reader already handled but no fixture exercised
(`all-contributors`, `all-titles`, `periodical`, `identifiers`, `pdf-urls`,
`keywords-multi`, `abstract-and-notes`, `pub-date`, `custom-fields`, `style-variety`,
`adv-malformed-xml`), and closed several genuine (if rare) silent-drop gaps the new AST's
per-container `extra` buckets fix as a natural consequence of proper modeling (unknown
children of `<contributors>`/`<titles>`/`<periodical>`/`<urls>`/`<dates>`/`<foreign-keys>`
now raw-preserve instead of being dropped) — none of those specific sub-gaps has a dedicated
fixture yet (no known real-world exporter emits them; tracked as future work, not asserted
as fixed by a test).

## Reference types (ref-type)

### Common types (with EndNote numeric codes)
- [x] Journal Article (17) — `article`
- [x] Book (6) — `book`
- [x] Book Section (5) — `book-section`
- [x] Conference Paper (47) — `conference`
- [x] Report (27) — `report`
- [x] Thesis (32) — `thesis`
- [x] Web Page (12) — `webpage`
- [ ] Audiovisual Material (34) — (missing)
- [ ] Bill (53) — (missing)
- [ ] Blog (56) — (missing)
- [ ] Case (17, law) — (missing)
- [ ] Catalog (96) — (missing)
- [ ] Chart or Table (38) — (missing)
- [ ] Classical Work (49) — (missing)
- [ ] Computer Program (9) — (missing)
- [ ] Conference Proceedings (10) — (missing)
- [ ] Dataset (59) — (missing)
- [ ] Dictionary (52) — (missing)
- [ ] Edited Book (28) — (missing)
- [ ] Electronic Article (43) — (missing)
- [ ] Electronic Book (45) — (missing)
- [ ] Electronic Book Section (60) — (missing)
- [ ] Encyclopedia (55) — (missing)
- [ ] Figure (48) — (missing)
- [ ] Film or Broadcast (21) — (missing)
- [ ] Generic (13) — (missing)
- [ ] Government Document (46) — (missing)
- [ ] Grant (31) — (missing)
- [ ] Hearing (19) — (missing)
- [ ] Journal (46) — (missing)
- [ ] Legal Rule or Regulation (50) — (missing)
- [ ] Magazine Article (19) — (missing)
- [ ] Manuscript (36) — (missing)
- [ ] Map (20) — (missing)
- [ ] Music (61) — (missing)
- [ ] Newspaper Article (23) — (missing)
- [ ] Online Database (45) — (missing)
- [ ] Online Multimedia (48) — (missing)
- [ ] Pamphlet (54) — (missing)
- [ ] Patent (25) — (missing)
- [ ] Personal Communication (26) — (missing)
- [ ] Press Release (57) — (missing)
- [ ] Serial (57) — (missing)
- [ ] Standard (58) — (missing)
- [ ] Statute (50) — (missing)
- [ ] Unpublished Work (34) — (missing)
- [ ] Video Recording (33) — (missing)

## Fields (XML elements within `<record>`)

### Identifier / type
- [x] ref-type (name and code) — all fixtures
- [x] rec-number — `conference`, `thesis`, `webpage` (used as cite-key fallback when `label` absent)
- [x] foreign-keys / key (with app/db-id attrs) — `conference`, `thesis`, `webpage`
- [ ] ref-source — (missing)
- [ ] database — (missing)

### Contributor fields (`<contributors>`)
- [x] authors / author (single) — `article`
- [x] authors / author (multiple) — `multi-author`
- [x] secondary-authors (editors) — `all-contributors` (`field:role` = `editor`)
- [x] tertiary-authors / subsidiary-authors — `all-contributors` (raw-preserved as `misc`)
- [ ] translated-authors — not raw-preserved with its own dedicated wrapper name today; falls
      into `Contributors::extra` (endnotexml-fmt AST) and would surface as a `misc` field
      tagged `contributors/translated-authors`, but no fixture exercises this path — (missing)

### Title fields
- [x] titles / title — `article`
- [x] titles / secondary-title (journal / container) — `article`
- [x] titles / tertiary-title (series) — `all-titles` (raw-preserved as `misc`)
- [ ] titles / short-title — not raw-preserved with its own dedicated wrapper name today; falls
      into `Titles::extra` and would surface as `misc` tagged `titles/short-title` — (missing fixture)
- [ ] titles / translated-title — same as short-title — (missing fixture)
- [ ] titles / alt-title — same as short-title — (missing fixture)

### Date fields
- [x] dates / year — `article`
- [x] dates / pub-dates / date — `pub-date` (raw-preserved as `misc`, no unambiguous
      month-name parse without guessing locale)
- [ ] dates / access-date — falls into `Dates::extra`, would surface as `misc` tagged
      `dates/access-date` — (missing fixture)

### Periodical fields
- [x] periodical / full-title — `periodical` (raw-preserved as `misc`, tagged
      `periodical/full-title`, distinct from `titles/secondary-title`)
- [ ] periodical / abbr-1 — falls into `Periodical::extra` — (missing fixture)
- [ ] periodical / abbr-2 — falls into `Periodical::extra` — (missing fixture)
- [ ] periodical / abbr-3 — falls into `Periodical::extra` — (missing fixture)

### Volume / pages
- [x] volume — `article`
- [x] number — `all-contributors`/others exercise it incidentally; no *dedicated* fixture —
      supported (`field:role` = `issue`) — (missing dedicated fixture)
- [x] pages (numeric range split into page_first/page_last) — `article`, `book-section`
- [ ] num-vols — falls into `Record::extra` — (missing fixture)
- [ ] edition — falls into `Record::extra` — (missing fixture)
- [ ] section — falls into `Record::extra` — (missing fixture)

### Publisher fields
- [x] publisher — `book`
- [x] pub-location — supported (`field:role` = `publisher_location`), no dedicated fixture yet — (missing fixture)

### Identifier fields
- [x] electronic-resource-num (DOI) — `with-doi`
- [x] urls / related-urls / url — `webpage`
- [x] urls / pdf-urls / url — `pdf-urls`
- [x] bare top-level url (non-standard placement) — `with-url`
- [x] isbn / issn (each own identifier field, not conflated) — `identifiers`
- [ ] accession-num — falls into `Record::extra` — (missing fixture)
- [ ] call-num — falls into `Record::extra` — (missing fixture)
- [x] custom1 through custom7 — `custom-fields` (raw-preserved as `misc` via the generic
      record-level fallback; only `custom1` exercised directly, `custom2`..`custom7` follow
      the identical code path)

### Content fields
- [x] abstract — `abstract-and-notes`
- [x] notes — `abstract-and-notes`, `style-variety` (with markup)
- [x] keywords / keyword — `keywords-multi`
- [x] research-notes — `custom-fields` (raw-preserved via generic fallback)
- [ ] work-type — same code path as `research-notes`, no dedicated fixture — (missing fixture)
- [ ] reviewed-item — same code path — (missing fixture)
- [x] language — `custom-fields` (raw-preserved via generic fallback)

### Rich text in fields (`<style face="...">`)
- [x] italic — `rare-style-markup`
- [x] bold — `rare-style-markup`
- [x] underline — `style-variety`
- [x] superscript / subscript — `style-variety`
- [ ] `<style>` attributes other than `face` (e.g. `font`, `size`, as seen in
      `rare-style-markup`'s input) — not preserved; `endnotexml-fmt`'s `Inline::Style` only
      captures `face` (matches the pre-existing reader's identical scope, not a regression
      introduced by this relocation) — tracked in TODO.md as a known gap, not asserted fixed

### Source (book info)
- [ ] source-app — (missing)

## Structure

- [ ] multiple records in one file — (missing; every current fixture is single-record)
- [ ] record with all standard fields — (missing; no single fixture exercises every field at once)

## Composition (integration)

- [x] article with volume, pages, journal, and DOI — covered piecewise (`article` has
      volume/pages/journal; `with-doi` has DOI) but no single fixture combines all four —
      (missing a combined fixture)
- [ ] book section with secondary title (book) and publisher — `book-section` has secondary
      title only, `book` has publisher only, no fixture combines them — (missing)
- [ ] multiple records in one `<xml>` document — (missing; every current fixture is single-record)

## Adversarial

- [x] empty / minimal XML — `adv-empty`
- [ ] record with unknown ref-type — not distinguished from a known one (ref-type is stored
      verbatim regardless), so this would not exercise different code — (missing fixture)
- [ ] record with missing ref-type — (missing)
- [x] malformed XML — `adv-malformed-xml` (truncated mid-record; must not panic)
- [ ] record with empty title — (missing)
- [x] XML with unknown elements — `custom-fields` (happy-path unknown elements);
      `adv-malformed-xml` covers the truncation case

## Pathological

- [ ] file with 1000 records — (missing)
- [ ] record with 100 keyword elements — (missing)
- [ ] very long abstract — (missing)
