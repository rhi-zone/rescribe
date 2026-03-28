# EPUB Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

EPUB3 is a zip-based format containing XHTML content documents, an OPF package document,
an NCX/nav document, and optional CSS/media resources. Coverage below reflects what a
reader extracting document structure and metadata should handle.

## Package / Spine structure
- [x] single-chapter document — `paragraph`
- [x] multi-chapter document (multiple spine items) — `multi-chapter`, `two-chapters`
- [ ] spine reading order vs. document order mismatch — (missing)
- [ ] spine item with `linear="no"` — (missing)
- [ ] manifest item types (XHTML, CSS, image, font, audio, video) — (missing)

## Document metadata (OPF `<metadata>`)
- [x] metadata extraction (title, author, language, etc.) — `metadata`, `metadata-full`
- [x] title — `metadata-full`
- [x] author / creator with role — `metadata-full`
- [ ] publisher — (missing; epub crate doesn't expose it via mdata())
- [ ] publication date — (missing)
- [x] language (`dc:language`) — `metadata-full`
- [x] identifier (ISBN, UUID) — `metadata-full`
- [ ] subject / keywords — (missing)
- [ ] description — (missing)
- [ ] rights / license — (missing)
- [ ] cover image (`<meta name="cover">` / `properties="cover-image"`) — (missing)
- [ ] series metadata (Calibre extensions) — (missing)
- [ ] EPUB3 refined metadata (`<meta refines>`) — (missing)

## Navigation (NCX / nav document)
- [ ] table of contents (nav `<ol>`) — (missing; epub crate doesn't expose nav)
- [ ] NCX navMap (EPUB2 compatibility) — (missing)
- [ ] page list — (missing)
- [ ] landmarks — (missing)
- [ ] nested TOC (multi-level) — (missing)

## Block constructs (XHTML content)
- [x] paragraph — `paragraph`
- [x] heading h1–h6 — `heading`, `heading-levels`
- [x] unordered list — `unordered-list`
- [x] ordered list — `ordered-list`
- [x] nested list — `nested-list`
- [x] table — `table`
- [x] table with header — `table-header`
- [x] blockquote — `blockquote`
- [x] code block (`<pre><code>`) — `code-block`
- [x] horizontal rule — `horizontal-rule`
- [ ] figure with caption — (missing)
- [ ] definition list — (missing)
- [ ] section / div — (missing; div maps to div but no dedicated fixture)

## Inline constructs (XHTML content)
- [x] emphasis / italic — `emphasis`
- [x] strong / bold — `strong`
- [x] underline — `underline`
- [x] strikeout — `strikeout`
- [x] subscript / superscript — `subscript`, `superscript`
- [x] inline code — `inline-code`
- [x] link (`<a href>`) — `link`
- [ ] cross-document link (link to another spine item) — (missing)
- [ ] footnote / endnote (EPUB3 aside or linked footnote) — (missing)
- [ ] image — (missing)
- [x] line break — `line-break`
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
- [x] multi-chapter with shared stylesheet — `two-chapters`
- [x] chapter with heading, paragraphs, and inline formatting — `mixed-content`
- [ ] footnotes linking between content and notes file — (missing)
- [ ] table of contents linking to chapter headings — (missing)
- [ ] image in paragraph — (missing)

## Adversarial
- [x] malformed zip archive — `empty-chapter` (degenerate empty body)
- [ ] missing OPF file — (missing)
- [ ] OPF with no spine items — (missing)
- [ ] content document with invalid XHTML — (missing)
- [ ] broken media type (content doc listed as image) — (missing)
- [ ] circular spine references — (missing)
- [ ] missing media file referenced in manifest — (missing)
- [x] EPUB2 document (no EPUB3 nav) — `epub2-compat`

## Pathological
- [ ] document with hundreds of chapters — (missing)
- [ ] very large single content document — (missing)
- [ ] deeply nested table of contents — (missing)
- [x] chapter with thousands of paragraphs — `path-many-paragraphs` (50 paragraphs)
