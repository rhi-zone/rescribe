# CSL-JSON Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

CSL-JSON is defined by the Citation Style Language specification.
Reference: https://citeproc-js.readthedocs.io/en/latest/csl-json/markup.html

## Item types (CSL `type` field)

### Common types
- [x] article-journal — `article-journal`
- [ ] article-magazine — (missing)
- [ ] article-newspaper — (missing)
- [x] book — `book`
- [ ] broadcast — (missing)
- [x] chapter — `chapter`
- [ ] classic — (missing)
- [ ] collection — (missing)
- [ ] dataset — (missing)
- [ ] document — (missing)
- [ ] entry — (missing)
- [ ] entry-dictionary — (missing)
- [ ] entry-encyclopedia — (missing)
- [ ] event — (missing)
- [ ] figure — (missing)
- [ ] graphic — (missing)
- [ ] hearing — (missing)
- [ ] interview — (missing)
- [ ] legal_case — (missing)
- [ ] legislation — (missing)
- [ ] manuscript — (missing)
- [ ] map — (missing)
- [ ] motion_picture — (missing)
- [ ] musical_score — (missing)
- [ ] pamphlet — (missing)
- [ ] paper-conference — (missing)
- [ ] patent — (missing)
- [ ] performance — (missing)
- [ ] periodical — (missing)
- [ ] personal_communication — (missing)
- [ ] post — (missing)
- [ ] post-weblog — (missing)
- [x] report — `report`
- [ ] review — (missing)
- [ ] review-book — (missing)
- [ ] software — (missing)
- [ ] song — (missing)
- [ ] speech — (missing)
- [ ] standard — (missing)
- [x] thesis — `thesis`
- [ ] treaty — (missing)
- [x] webpage — `webpage`

## Fields

### Identifier fields
- [x] id — all fixtures
- [ ] citation-key — (missing)
- [x] DOI — `with-doi`
- [ ] ISBN — (missing)
- [ ] ISSN — (missing)
- [ ] PMID — (missing)
- [ ] PMCID — (missing)
- [ ] URL — `webpage`

### Name fields (person name objects)
- [x] author (single) — `article-journal`
- [x] author (multiple) — `multi-author`
- [ ] editor — (missing)
- [ ] translator — (missing)
- [ ] collection-editor — (missing)
- [ ] composer — (missing)
- [ ] container-author — (missing)
- [ ] director — (missing)
- [ ] editorial-director — (missing)
- [ ] interviewer — (missing)
- [ ] illustrator — (missing)
- [ ] original-author — (missing)
- [ ] recipient — (missing)
- [ ] reviewed-author — (missing)

### Name object structure
- [x] family + given — `article-journal`
- [ ] literal (non-parsed name) — (missing)
- [ ] dropping-particle — (missing)
- [ ] non-dropping-particle — (missing)
- [ ] suffix — (missing)
- [ ] comma-suffix — (missing)
- [ ] static-ordering — (missing)

### Date fields (date objects)
- [x] issued (date-parts array) — `article-journal`
- [x] issued (literal) — `rare-literal-date`
- [ ] accessed — (missing)
- [ ] available-date — (missing)
- [ ] event-date — (missing)
- [ ] original-date — (missing)
- [ ] submitted — (missing)
- [ ] date-parts with season — (missing)
- [ ] date-parts with circa — (missing)
- [ ] date range (two date-parts arrays) — (missing)

### Title fields
- [x] title — all fixtures
- [ ] title-short — (missing)
- [ ] container-title — `chapter`
- [ ] container-title-short — (missing)
- [ ] collection-title — (missing)
- [ ] original-title — (missing)
- [ ] reviewed-title — (missing)

### Publication fields
- [x] publisher — `book`, `chapter`, `report`
- [ ] publisher-place — (missing)
- [ ] archive — (missing)
- [ ] archive_location — (missing)
- [ ] archive-place — (missing)
- [ ] authority — (missing)
- [ ] call-number — (missing)
- [ ] collection-number — (missing)
- [ ] edition — (missing)
- [ ] event — (missing)
- [ ] event-place — (missing)
- [ ] first-reference-note-number — (missing)
- [ ] genre — (missing)
- [ ] jurisdiction — (missing)
- [ ] keyword — (missing)
- [ ] locator — (missing)
- [ ] medium — (missing)
- [ ] note — (missing)
- [ ] number — (missing)
- [ ] number-of-pages — (missing)
- [ ] number-of-volumes — (missing)
- [ ] original-publisher — (missing)
- [ ] original-publisher-place — (missing)
- [ ] page — (missing)
- [ ] page-first — (missing)
- [ ] references — (missing)
- [ ] scale — (missing)
- [ ] section — (missing)
- [ ] source — (missing)
- [ ] status — (missing)
- [ ] version — (missing)
- [ ] volume — (missing)
- [ ] volume-title — (missing)
- [ ] year-suffix — (missing)

### Rich text in fields
- [ ] bold markup in title (`<b>`) — (missing)
- [ ] italic markup in title (`<i>`) — (missing)
- [ ] small-caps in title (`<sc>`) — (missing)
- [ ] superscript (`<sup>`) — (missing)
- [ ] subscript (`<sub>`) — (missing)
- [ ] no-decor (`<span style="font-variant:normal">`) — (missing)

## Array / collection
- [ ] multiple items in the array — (missing; all existing fixtures have 1 item per file except multi-author)
- [ ] empty array — `adv-empty`

## Composition (integration)

- [ ] article-journal with all standard fields — (missing)
- [ ] chapter with container-title, editor, publisher — (missing)
- [ ] item with both DOI and URL — (missing)

## Adversarial

- [x] empty array — `adv-empty`
- [ ] item with unknown type — (missing)
- [ ] missing id field — (missing)
- [ ] malformed date-parts — (missing)
- [ ] name with only literal field — (missing)
- [ ] truncated JSON — (missing)
- [ ] non-array top level — (missing)

## Pathological

- [ ] array with 1000 items — (missing)
- [ ] item with all fields present — (missing)
- [ ] author list with 100 names — (missing)
