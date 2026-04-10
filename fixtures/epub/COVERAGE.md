# EPUB Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

EPUB3 is a zip-based format containing XHTML content documents, an OPF package document,
an NCX/nav document, and optional CSS/media resources. Coverage below reflects what a
reader extracting document structure and metadata should handle.

**Library note:** This format uses the `epub` crate (v2) as the upstream parser. Items
marked `[lib]` are not exposed by this library and cannot be tested without bypassing
the library. The library provides the correctness guarantee (†) for what it does expose.

## Package / Spine structure
- [x] single-chapter document — `paragraph`
- [x] multi-chapter document (multiple spine items) — `multi-chapter`, `two-chapters`, `path-many-chapters`
- [ ] spine reading order vs. document order mismatch — `[lib]` epub crate iterates in OPF spine order
- [ ] spine item with `linear="no"` — `[lib]` epub crate doesn't expose linear attribute
- [ ] manifest item types (XHTML, CSS, image, font, audio, video) — `[lib]` non-XHTML items not accessible

## Document metadata (OPF `<metadata>`)
- [x] metadata extraction (title, author, language, etc.) — `metadata`, `metadata-full`
- [x] title — `metadata-full`
- [x] author / creator with role — `metadata-full`
- [x] publisher — `metadata-extended`
- [x] publication date — `metadata-extended`
- [x] language (`dc:language`) — `metadata-full`
- [x] identifier (ISBN, UUID) — `metadata-full`
- [x] subject / keywords — `metadata-extended`
- [x] description — `metadata-extended`
- [ ] rights / license — `[lib]` dc:rights not returned by epub crate mdata()
- [ ] cover image (`<meta name="cover">` / `properties="cover-image"`) — `[lib]` get_cover() returns data only; no fixture needed (no content node produced)
- [ ] series metadata (Calibre extensions) — `[lib]` non-standard; not accessible
- [ ] EPUB3 refined metadata (`<meta refines>`) — `[lib]` epub crate doesn't parse refines

## Navigation (NCX / nav document)
- [ ] table of contents (nav `<ol>`) — `[lib]` epub crate v2 doesn't expose nav document
- [ ] NCX navMap (EPUB2 compatibility) — `[lib]` epub crate v2 doesn't expose NCX
- [ ] page list — `[lib]` not accessible
- [ ] landmarks — `[lib]` not accessible
- [ ] nested TOC (multi-level) — `[lib]` not accessible

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
- [x] figure with caption — `figure-with-caption`
- [x] definition list — `definition-list`
- [x] section / div — `section-div`

## Inline constructs (XHTML content)
- [x] emphasis / italic — `emphasis`
- [x] strong / bold — `strong`
- [x] underline — `underline`
- [x] strikeout — `strikeout`
- [x] subscript / superscript — `subscript`, `superscript`
- [x] inline code — `inline-code`
- [x] link (`<a href>`) — `link`
- [x] cross-document link (link to another spine item) — `cross-document-link`
- [ ] footnote / endnote (EPUB3 aside or linked footnote) — `[lib]` represented as plain HTML in XHTML; round-trips via HTML parsing as-is
- [x] image — `figure-with-caption` (img alt text preserved)
- [x] line break — `line-break`
- [x] span with class/style — `span-style`

## Embedded resources
- [ ] cover image — `[lib]` get_cover() accessible but produces no content node
- [ ] inline image (referenced from content) — `[lib]` EPUB crate doesn't embed image data into content; img src is a relative path inside the archive
- [ ] embedded font — `[lib]` not accessible via epub crate
- [ ] audio/video media (EPUB3 MO) — `[lib]` not accessible

## EPUB3-specific
- [ ] media overlays (SMIL) — `[lib]` not accessible
- [ ] MathML — passed through as HTML fallback content
- [ ] SVG content document — `[lib]` not accessible (manifest items, not XHTML body)
- [ ] EPUB CFI (canonical fragment identifier) — `[lib]` not accessible
- [ ] semantic inflection (`epub:type`) — passes through HTML parsing as an unknown attribute

## Composition (integration)
- [x] multi-chapter with shared stylesheet — `two-chapters`
- [x] chapter with heading, paragraphs, and inline formatting — `mixed-content`
- [ ] footnotes linking between content and notes file — `[lib]` see inline footnote note above
- [ ] table of contents linking to chapter headings — `[lib]` nav document not accessible
- [x] image in paragraph — `figure-with-caption`

## Adversarial
- [x] malformed zip archive — `empty-chapter` (degenerate empty body)
- [ ] missing OPF file — returns ParseError (no fixture needed; expect_error behavior)
- [x] OPF with no spine items — `adv-empty-spine`
- [x] content document with invalid XHTML — `adv-invalid-xhtml` (recovered by HTML parser)
- [ ] broken media type (content doc listed as image) — `[lib]` epub crate filters by media type
- [ ] circular spine references — `[lib]` epub crate handles iteration internally
- [ ] missing media file referenced in manifest — `[lib]` epub crate handles gracefully
- [x] EPUB2 document (no EPUB3 nav) — `epub2-compat`

## Pathological
- [x] document with many chapters — `path-many-chapters` (20 chapters)
- [ ] very large single content document — deferred (file size constraint)
- [ ] deeply nested table of contents — `[lib]` nav not accessible
- [x] chapter with thousands of paragraphs — `path-many-paragraphs` (50 paragraphs)
