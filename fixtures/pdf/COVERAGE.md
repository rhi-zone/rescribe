# PDF Fixture Coverage

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

PDF text extraction recovers text content and structure from the page stream. This
coverage list reflects what a text-extraction-based reader (e.g. pdf-extract) can
reasonably recover, not what a full PDF renderer would handle.

## Text content
- [x] paragraph text — `paragraph`
- [ ] multi-paragraph document — (missing)
- [ ] text with font encoding (WinAnsi, MacRoman, UTF-16BE) — (missing)
- [ ] text with ligatures (fi, fl, ff) — (missing)
- [ ] text with Unicode (non-ASCII characters) — (missing)
- [ ] right-to-left text (Arabic, Hebrew) — (missing)
- [ ] mixed LTR/RTL in one page — (missing)
- [ ] whitespace-only page — (missing)

## Structure inference (heuristic)
- [ ] heading detection (larger font size = heading) — (missing)
- [ ] list item detection (bullet prefix) — (missing)
- [ ] table detection (column-aligned text blocks) — (missing)
- [ ] multi-column layout — (missing)
- [ ] page header / footer detection — (missing)
- [ ] page number detection — (missing)

## Document metadata (PDF Info dictionary / XMP)
- [ ] title — (missing)
- [ ] author — (missing)
- [ ] subject — (missing)
- [ ] keywords — (missing)
- [ ] creation date — (missing)
- [ ] modification date — (missing)
- [ ] producer / creator application — (missing)

## Document structure (PDF/UA logical structure tree)
- [ ] tagged PDF: paragraph — (missing)
- [ ] tagged PDF: heading (`<H1>`–`<H6>`) — (missing)
- [ ] tagged PDF: list and list items — (missing)
- [ ] tagged PDF: table with headers — (missing)
- [ ] tagged PDF: figure with alt text — (missing)
- [ ] tagged PDF: link with URL — (missing)
- [ ] tagged PDF: footnote — (missing)

## Inline formatting (where recoverable)
- [ ] bold text (font weight heuristic) — (missing)
- [ ] italic text (font style heuristic) — (missing)
- [ ] hyperlink annotation (`/URI`) — (missing)
- [ ] internal link annotation (`/GoTo`) — (missing)

## Embedded content
- [ ] embedded image (JPEG, PNG, JBIG2) — (missing)
- [ ] form field (AcroForm) — (missing)
- [ ] embedded file attachment — (missing)

## Composition (integration)
- [ ] page with headings, paragraphs, and a table — (missing)
- [ ] document with page headers and footnotes — (missing)
- [ ] multi-page document with consistent structure — (missing)

## Adversarial
- [ ] corrupt PDF header (`%PDF-` missing) — (missing)
- [ ] truncated xref table — (missing)
- [ ] linearized PDF — (missing)
- [ ] encrypted PDF (user password) — (missing)
- [ ] encrypted PDF (owner password only, content accessible) — (missing)
- [ ] PDF with no text (scanned image only) — (missing)
- [ ] cross-reference stream (PDF 1.5+) — (missing)
- [ ] object streams (compressed objects) — (missing)
- [ ] PDF 2.0 features — (missing)

## Pathological
- [ ] PDF with thousands of pages — (missing)
- [ ] single page with thousands of text objects — (missing)
- [ ] deeply nested content streams — (missing)
- [ ] very large embedded image — (missing)
- [ ] font with custom encoding / ToUnicode map — (missing)
