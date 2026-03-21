# BibLaTeX Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

## Entry types

BibLaTeX defines regular entry types, type aliases, and unsupported/custom types.

### Regular entry types
- [x] article — `article`
- [x] book — `book`
- [ ] bookinbook — (missing)
- [ ] booklet — (missing)
- [ ] collection — (missing)
- [ ] dataset — (missing)
- [ ] image — (missing)
- [x] inbook — `inbook`
- [x] incollection — `incollection`
- [x] inproceedings — `inproceedings`
- [ ] inreference — (missing)
- [ ] jurisdiction — (missing)
- [ ] legal — (missing)
- [ ] legislation — (missing)
- [ ] letter — (missing)
- [ ] manual — (missing)
- [x] misc — `misc`
- [ ] movie — (missing)
- [ ] music — (missing)
- [x] online — `online`
- [ ] patent — (missing)
- [ ] performance — (missing)
- [ ] periodical — (missing)
- [ ] proceedings — (missing)
- [ ] reference — (missing)
- [ ] report — (missing)
- [ ] review — (missing)
- [ ] set — (missing)
- [ ] software — (missing)
- [ ] standard — (missing)
- [ ] suppbook — (missing)
- [ ] suppcollection — (missing)
- [ ] suppperiodical — (missing)
- [x] thesis — `thesis` (covers @phdthesis alias via type= field)
- [x] mastersthesis — `mastersthesis` (alias entry type)
- [ ] unpublished — (missing)
- [ ] video — (missing)
- [ ] xdata — (missing)

### Type aliases (BibTeX compatibility)
- [x] @phdthesis — `thesis` (maps to thesis with type=phdthesis)
- [x] @mastersthesis — `mastersthesis`
- [ ] @conference — (missing; alias for inproceedings)
- [ ] @techreport — (missing; alias for report)
- [ ] @www — (missing; alias for online)

## Fields

### Required / commonly required fields
- [x] author — `article`, `book`
- [x] title — `article`
- [x] date — `article`, `book`
- [x] journaltitle — `article`
- [x] publisher — `book`

### Author/editor fields
- [x] author (single) — `article`
- [ ] author (multiple) — (missing; biblatex uses `and` separator)
- [ ] editor — (missing)
- [ ] editora / editorb / editorc — (missing)
- [ ] editortype — (missing)
- [ ] translator — (missing)
- [ ] annotator — (missing)
- [ ] commentator — (missing)
- [ ] introduction — (missing)
- [ ] foreword — (missing)
- [ ] afterword — (missing)
- [ ] holder (patent) — (missing)
- [ ] namea / nameb / namec — (missing)

### Title fields
- [x] title — `article`
- [x] subtitle — `rare-with-subtitle`
- [ ] titleaddon — (missing)
- [ ] booktitle — `inbook`, `incollection`
- [ ] booksubtitle — (missing)
- [ ] booktitleaddon — (missing)
- [ ] maintitle — (missing)
- [ ] mainsubtitle — (missing)
- [ ] maintitleaddon — (missing)
- [ ] journaltitle — `article`
- [ ] journalsubtitle — (missing)
- [ ] issuetitle — (missing)
- [ ] issuesubtitle — (missing)
- [ ] eventtitle — (missing)
- [ ] origtitle — (missing)
- [ ] reprinttitle — (missing)
- [ ] series — (missing)
- [ ] shorttitle — (missing)
- [ ] shortjournal — (missing)
- [ ] sorttitle — (missing)
- [ ] indextitle — (missing)
- [ ] indexsorttitle — (missing)

### Date fields
- [x] date — `article` (YYYY format)
- [ ] date (range) — (missing; YYYY/YYYY format)
- [ ] eventdate — (missing)
- [ ] origdate — (missing)
- [ ] urldate — (missing)
- [ ] year — (missing; BibTeX compat field)
- [ ] month — (missing; BibTeX compat field)

### Identifier fields
- [ ] isbn — (missing)
- [ ] issn — (missing)
- [ ] isrn — (missing)
- [ ] doi — (missing; biblatex uses doi=)
- [ ] eprint — (missing)
- [ ] eprintclass — (missing)
- [ ] eprinttype — (missing)
- [ ] url — `online`
- [ ] eid — (missing)

### Publication fields
- [x] publisher — `book`
- [ ] institution — `mastersthesis`, `thesis`
- [ ] organization — (missing)
- [ ] location — (missing)
- [ ] address — (missing; BibTeX compat)
- [ ] venue — (missing)
- [ ] edition — (missing)
- [ ] version — (missing)
- [ ] volumes — (missing)
- [ ] volume — `article`
- [ ] number — (missing)
- [ ] issue — (missing)
- [ ] pages — `article`
- [ ] pagetotal — (missing)
- [ ] chapter — `inbook`
- [ ] part — (missing)
- [ ] howpublished — (missing)
- [ ] type — (missing)

### Annotation / note fields
- [ ] abstract — (missing)
- [ ] addendum — (missing)
- [ ] annotation — (missing)
- [ ] file — (missing)
- [ ] library — (missing)
- [ ] note — `misc`
- [ ] pubstate — (missing)

### Cross-reference fields
- [ ] crossref — (missing)
- [ ] entryset — (missing)
- [ ] execute — (missing)
- [ ] gender — (missing)
- [ ] langid — (missing)
- [ ] langidopts — (missing)
- [ ] label — (missing)
- [ ] options — (missing)
- [ ] presort — (missing)
- [ ] related — (missing)
- [ ] relatedoptions — (missing)
- [ ] relatedstring — (missing)
- [ ] relatedtype — (missing)
- [ ] shorthand — (missing)
- [ ] shorthandintro — (missing)
- [ ] sortkey — (missing)
- [ ] sortname — (missing)
- [ ] sortshorthand — (missing)
- [ ] sortyear — (missing)
- [ ] xdata — (missing)
- [ ] xref — (missing)
- [ ] keywords — (missing)
- [ ] ids — (missing)

## Special syntax

- [ ] name list with `and` separator (multiple authors) — (missing)
- [ ] `and others` in author list — (missing)
- [ ] literal string with `{...}` — (missing)
- [ ] string concatenation with `#` — (missing)
- [ ] @string definitions — (missing)
- [ ] @preamble — (missing)
- [ ] @comment — (missing)
- [ ] multiple entries in one file — (missing)
- [ ] entry key special characters — (missing)
- [ ] cross-referencing via crossref= — (missing)

## Composition (integration)

- [ ] article with all standard fields — (missing)
- [ ] book with editor instead of author — (missing)
- [ ] inproceedings with pages and doi — (missing)
- [ ] multiple entries in one file — (missing)

## Adversarial

- [x] empty file — `adv-empty`
- [ ] missing required field (no title) — (missing)
- [ ] malformed entry (unclosed brace) — (missing)
- [ ] unknown entry type — (missing)
- [ ] unknown field name — (missing)
- [ ] entry with no key — (missing)
- [ ] duplicate entry keys — (missing)
- [ ] circular crossref — (missing)

## Pathological

- [ ] file with 1000 entries — (missing)
- [ ] entry with very long field value — (missing)
- [ ] deeply nested braces in field value — (missing)
- [ ] all fields present on a single entry — (missing)
