# EPUB Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

EPUB3 is a zip-based format containing XHTML content documents, an OPF package document,
an NCX/nav document, and optional CSS/media resources. Coverage below reflects what a
reader extracting document structure and metadata should handle.

## Package / Spine structure
- [x] single-chapter document — `paragraph`
- [x] multi-chapter document (multiple spine items) — `multi-chapter`
- [ ] spine reading order vs. document order mismatch — (missing)
- [ ] spine item with `linear="no"` — (missing)
- [ ] manifest item types (XHTML, CSS, image, font, audio, video) — (missing)

## Document metadata (OPF `<metadata>`)
- [x] metadata extraction (title, author, language, etc.) — `metadata`
- [ ] title — (missing as a dedicated fixture; covered partially by `metadata`)
- [ ] author / creator with role — (missing)
- [ ] publisher — (missing)
- [ ] publication date — (missing)
- [ ] language (`dc:language`) — (missing)
- [ ] identifier (ISBN, UUID) — (missing)
- [ ] subject / keywords — (missing)
- [ ] description — (missing)
- [ ] rights / license — (missing)
- [ ] cover image (`<meta name="cover">` / `properties="cover-image"`) — (missing)
- [ ] series metadata (Calibre extensions) — (missing)
- [ ] EPUB3 refined metadata (`<meta refines>`) — (missing)

## Navigation (NCX / nav document)
- [ ] table of contents (nav `<ol>`) — (missing)
- [ ] NCX navMap (EPUB2 compatibility) — (missing)
- [ ] page list — (missing)
- [ ] landmarks — (missing)
- [ ] nested TOC (multi-level) — (missing)

## Block constructs (XHTML content)
- [x] paragraph — `paragraph`
- [ ] heading h1–h6 — (missing)
- [ ] unordered list — (missing)
- [ ] ordered list — (missing)
- [ ] nested list — (missing)
- [ ] table — (missing)
- [ ] table with header — (missing)
- [ ] blockquote — (missing)
- [ ] code block (`<pre><code>`) — (missing)
- [ ] horizontal rule — (missing)
- [ ] figure with caption — (missing)
- [ ] definition list — (missing)
- [ ] section / div — (missing)

## Inline constructs (XHTML content)
- [ ] emphasis / italic — (missing)
- [ ] strong / bold — (missing)
- [ ] underline — (missing)
- [ ] strikeout — (missing)
- [ ] subscript / superscript — (missing)
- [ ] inline code — (missing)
- [ ] link (`<a href>`) — (missing)
- [ ] cross-document link (link to another spine item) — (missing)
- [ ] footnote / endnote (EPUB3 aside or linked footnote) — (missing)
- [ ] image — (missing)
- [ ] line break — (missing)
- [ ] span with class/style — (missing)

## Embedded resources
- [ ] cover image — (missing)
- [ ] inline image (referenced from content) — (missing)
- [ ] embedded font — (missing)
- [ ] audio/video media (EPUB3 MO) — (missing)

## EPUB3-specific
- [ ] media overlays (SMIL) — (missing)
- [ ] MathML — (missing)
- [ ] SVG content document — (missing)
- [ ] EPUB CFI (canonical fragment identifier) — (missing)
- [ ] semantic inflection (`epub:type`) — (missing)

## Composition (integration)
- [ ] multi-chapter with shared stylesheet — (missing)
- [ ] chapter with heading, paragraphs, and inline formatting — (missing)
- [ ] footnotes linking between content and notes file — (missing)
- [ ] table of contents linking to chapter headings — (missing)
- [ ] image in paragraph — (missing)

## Adversarial
- [ ] malformed zip archive — (missing)
- [ ] missing OPF file — (missing)
- [ ] OPF with no spine items — (missing)
- [ ] content document with invalid XHTML — (missing)
- [ ] broken media type (content doc listed as image) — (missing)
- [ ] circular spine references — (missing)
- [ ] missing media file referenced in manifest — (missing)
- [ ] EPUB2 document (no EPUB3 nav) — (missing)

## Pathological
- [ ] document with hundreds of chapters — (missing)
- [ ] very large single content document — (missing)
- [ ] deeply nested table of contents — (missing)
- [ ] chapter with thousands of paragraphs — (missing)
