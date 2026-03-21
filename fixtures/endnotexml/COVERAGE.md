# EndNote XML Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

EndNote XML uses a proprietary XML schema exported by EndNote. The root element is `<xml>`,
records are inside `<records>`, each record is a `<record>` element with a `<ref-type>`
child specifying the reference type by name and numeric code.

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
- [ ] rec-number — (missing)
- [ ] foreign-keys — (missing)
- [ ] ref-source — (missing)
- [ ] database — (missing)

### Contributor fields (`<contributors>`)
- [x] authors / author (single) — `article`
- [x] authors / author (multiple) — `multi-author`
- [ ] secondary-authors (editors) — (missing)
- [ ] tertiary-authors — (missing)
- [ ] subsidiary-authors — (missing)
- [ ] translated-authors — (missing)

### Title fields
- [x] titles / title — `article`
- [ ] titles / secondary-title (journal / container) — (missing)
- [ ] titles / tertiary-title (series) — (missing)
- [ ] titles / short-title — (missing)
- [ ] titles / translated-title — (missing)
- [ ] titles / alt-title — (missing)

### Date fields
- [x] dates / year — `article`
- [ ] dates / pub-dates / date — (missing)
- [ ] dates / access-date — (missing)

### Periodical fields
- [ ] periodical / full-title — (missing)
- [ ] periodical / abbr-1 — (missing)
- [ ] periodical / abbr-2 — (missing)
- [ ] periodical / abbr-3 — (missing)

### Volume / pages
- [ ] volume — (missing)
- [ ] number — (missing)
- [ ] pages — (missing)
- [ ] num-vols — (missing)
- [ ] edition — (missing)
- [ ] section — (missing)

### Publisher fields
- [ ] publisher — (missing)
- [ ] pub-location — (missing)

### Identifier fields
- [x] electronic-resource-num (DOI) — `with-doi`
- [x] urls / related-urls / url — `with-url`, `webpage`
- [ ] isbn — (missing)
- [ ] label — (missing)
- [ ] accession-num — (missing)
- [ ] call-num — (missing)
- [ ] custom1 through custom7 — (missing)

### Content fields
- [ ] abstract — (missing)
- [ ] notes — (missing)
- [ ] keywords / keyword — (missing)
- [ ] research-notes — (missing)
- [ ] work-type — (missing)
- [ ] reviewed-item — (missing)
- [ ] language — (missing)

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
