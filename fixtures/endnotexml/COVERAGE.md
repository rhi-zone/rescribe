# EndNote XML Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

EndNote XML uses a proprietary XML schema exported by EndNote. The root element is `<xml>`,
records are inside `<records>`, each record is a `<record>` element with a `<ref-type>`
child specifying the reference type by name and numeric code.

Records parse into `bibliography`/`bibliography_entry`/`bibliography_field` IR nodes (see
ADR 0005 and `rescribe_std::node`), not the legacy flat `definition_list` shape. Each field
node carries both `field:role` (the semantic vocabulary) and `endnote:field` (the exact
source element path, e.g. `titles/secondary-title`, `urls/related-urls/url`). EndNote XML
is the one of these four formats whose fields can carry `<style face="...">` markup runs —
those become real `emphasis`/`strong`/`underline`/`superscript`/`subscript` inline nodes
(see `rare-style-markup`) instead of being flattened to plain text.

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
- [x] secondary-authors (editors) — supported (`field:role` = `editor`), no dedicated fixture yet — (missing fixture)
- [x] tertiary-authors / subsidiary-authors — raw-preserved as `misc`, no dedicated fixture yet — (missing fixture)
- [ ] translated-authors — (missing)

### Title fields
- [x] titles / title — `article`
- [x] titles / secondary-title (journal / container) — `article`
- [x] titles / tertiary-title (series) — raw-preserved as `misc`, no dedicated fixture yet — (missing fixture)
- [ ] titles / short-title — (missing)
- [ ] titles / translated-title — (missing)
- [ ] titles / alt-title — (missing)

### Date fields
- [x] dates / year — `article`
- [x] dates / pub-dates / date — raw-preserved as `misc` (no unambiguous month-name parse without guessing locale), no dedicated fixture yet — (missing fixture)
- [ ] dates / access-date — (missing)

### Periodical fields
- [ ] periodical / full-title — (missing)
- [ ] periodical / abbr-1 — (missing)
- [ ] periodical / abbr-2 — (missing)
- [ ] periodical / abbr-3 — (missing)

### Volume / pages
- [x] volume — `article`
- [x] number — supported (`field:role` = `issue`), no dedicated fixture yet — (missing fixture)
- [x] pages (numeric range split into page_first/page_last) — `article`, `book-section`
- [ ] num-vols — (missing)
- [ ] edition — (missing)
- [ ] section — (missing)

### Publisher fields
- [x] publisher — `book`
- [x] pub-location — supported (`field:role` = `publisher_location`), no dedicated fixture yet — (missing fixture)

### Identifier fields
- [x] electronic-resource-num (DOI) — `with-doi`
- [x] urls / related-urls / url — `webpage`
- [x] urls / pdf-urls / url — supported, no dedicated fixture yet — (missing fixture)
- [x] bare top-level url (non-standard placement) — `with-url`
- [x] isbn / issn (each own identifier field, not conflated) — supported, no dedicated fixture yet — (missing fixture)
- [ ] accession-num — (missing)
- [ ] call-num — (missing)
- [ ] custom1 through custom7 — raw-preserved as `misc` via the generic record-level fallback, no dedicated fixture yet — (missing fixture)

### Content fields
- [x] abstract — supported, no dedicated fixture yet — (missing fixture)
- [x] notes — supported, no dedicated fixture yet — (missing fixture)
- [x] keywords / keyword — supported, no dedicated fixture yet — (missing fixture)
- [ ] research-notes — (missing; would raw-preserve via generic fallback)
- [ ] work-type — (missing; would raw-preserve via generic fallback)
- [ ] reviewed-item — (missing; would raw-preserve via generic fallback)
- [ ] language — (missing; would raw-preserve via generic fallback)

### Rich text in fields (`<style face="...">`)
- [x] italic — `rare-style-markup`
- [x] bold — `rare-style-markup`
- [ ] underline — (missing; supported, no fixture)
- [ ] superscript / subscript — (missing; supported, no fixture)

### Source (book info)
- [ ] source-app — (missing)

## Structure

- [ ] multiple records in one file — (missing)
- [ ] record with all standard fields — (missing)

## Composition (integration)

- [ ] article with volume, pages, journal, and DOI — (missing)
- [ ] book section with secondary title (book) and publisher — (missing)
- [ ] multiple records in one `<xml>` document — (missing)

## Adversarial

- [x] empty / minimal XML — `adv-empty`
- [ ] record with unknown ref-type — (missing)
- [ ] record with missing ref-type — (missing)
- [ ] malformed XML — (missing)
- [ ] record with empty title — (missing)
- [ ] XML with unknown elements — (missing)

## Pathological

- [ ] file with 1000 records — (missing)
- [ ] record with 100 keyword elements — (missing)
- [ ] very long abstract — (missing)
