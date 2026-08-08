# ODT Fixture Coverage

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

## Block constructs
- [x] paragraph (`<text:p>`) — `paragraph`
- [x] heading (`<text:h>`) — `heading`
- [x] heading levels 1–6 individually — `heading-levels`
- [x] unordered list (`<text:list>` with bullet style) — `list`
- [x] ordered list (`<text:list>` with number style) — `ordered-list`
- [x] nested list — `nested-list`
- [x] table (`<table:table>`) — `table`
- [x] table with header row — `table-header`
- [x] table with colspan/rowspan — `colspan-rowspan`
- [x] code block (preformatted paragraph style) — `code-block`
- [x] blockquote (Quotations paragraph style) — `blockquote`
- [x] horizontal rule (Horizontal Line paragraph style) — `horizontal-rule`
- [x] text box / frame (`<draw:text-box>`) — `text-box`
- [x] definition list (no native ODF construct; style-based) — not applicable (ODF has no native DL)

## Inline constructs
- [x] line break (`<text:line-break>`) — `line-break`
- [x] bold (`fo:font-weight="bold"`) — `bold`
- [x] italic (`fo:font-style="italic"`) — `italic`
- [x] underline (`style:text-underline-style`) — `underline`
- [x] strikeout (`style:text-line-through-style`) — `strikeout`
- [x] subscript (`style:text-position`) — `subscript`
- [x] superscript (`style:text-position`) — `superscript`
- [x] small caps (`fo:font-variant="small-caps"`) — `small-caps`
- [x] font color (`fo:color`) — `font-color`
- [x] font size (`fo:font-size`) — `font-size`
- [x] font name (`fo:font-family`) — `font-name`
- [x] hyperlink (`<text:a>`) — `hyperlink`
- [x] footnote (`<text:note text:note-class="footnote">`) — `footnote`
- [x] endnote (`<text:note text:note-class="endnote">`) — `endnote`
- [x] image / frame (`<draw:frame><draw:image>`) — `image`
- [x] bookmark (`<text:bookmark>`) — `bookmark`
- [x] annotation / comment (`<office:annotation>`) — `annotation`
- [x] tab stop (`<text:tab>`) — `tab`
- [x] soft hyphen (`<text:soft-hyphen>`) — `soft-hyphen`
- [x] non-breaking space (`&#160;`) — `non-breaking-space`

## Paragraph properties
- [x] paragraph alignment (`fo:text-align`) — `para-align`
- [x] paragraph indent (`fo:margin-left`, `fo:text-indent`) — `para-indent`
- [x] paragraph spacing (`fo:margin-top`, `fo:margin-bottom`) — `para-spacing`
- [x] paragraph style name (`text:style-name`) — `para-style-name`
- [x] paragraph border (`fo:border`) — `para-border`
- [x] paragraph background color — `para-background`
- [x] line height — `line-height`
- [x] keep-together / keep-with-next — `keep-together`

## Document metadata
- [x] title (`<dc:title>`) — `meta-title`
- [x] author (`<dc:creator>`) — `meta-author`
- [x] description (`<dc:description>`) — `meta-description`
- [x] creation/modification date — `meta-date`
- [x] language (`<dc:language>`) — `meta-language`
- [x] custom user-defined metadata — `meta-custom`
- [x] page size and margins (`<style:page-layout>`) — `page-layout`

## Composition (integration)
- [x] table cells with formatted inline content — `table-cells-formatted`
- [x] list items with inline formatting — `list-items-formatted`
- [x] footnote with formatted content — `footnote-formatted`
- [x] image with caption — `image-caption`
- [x] heading with inline formatting — `heading-formatted`
- [x] hyperlink containing formatted text — `link-formatted`
- [x] nested blockquote — `nested-blockquote`

## Adversarial
- [x] malformed zip archive — `adv-malformed-zip`
- [x] missing content.xml — `adv-missing-content`
- [x] corrupt styles.xml — `adv-corrupt-styles`
- [x] unknown XML namespace — `adv-unknown-namespace`
- [x] empty document — `adv-empty`
- [x] corrupt image binary — `adv-corrupt-image`
- [x] non-ODF zip (wrong mimetype) — `adv-wrong-mimetype`

## Pathological
- [x] document with thousands of paragraphs — `path-many-paragraphs`
- [x] deeply nested tables — `path-deeply-nested-table`
- [x] list with many nesting levels — `path-deeply-nested-list`
- [x] paragraph with hundreds of character runs — `path-many-char-runs`
- [x] very large embedded image — `path-large-image`

## Rare/edge inline constructs
- [x] field elements (`<text:page-number>`, `<text:date>`, etc., both the
      `<tag>value</tag>` and self-closing `<tag/>` forms) — `rare-fields`
      (in `fixtures/odf/`, wired into the `odf` fixture-suite test since
      2026-08-08 — see the "Suite fragmentation" note below) plus
      `rare-field-self-closing` for the self-closing case specifically.
      Both forms modeled as `Inline::Field`/`OdfEvent::Field`; not yet
      surfaced as its own IR node (currently dropped as empty text on the
      `rescribe` side unless the cached value is non-empty — see TODO.md).
- [x] frame holding multiple children (image + caption text-box together) —
      `image-caption` (fixed 2026-08-08; was previously an either/or loss,
      see TODO.md)
- [ ] ranged bookmark (`<text:bookmark-start>`/`<text:bookmark-end>` pair
      spanning multiple runs) — currently collapsed to a single point
      bookmark at the start position; the end position is dropped. Only a
      `<text:bookmark>`/`<text:bookmark-start>` fixture (single point) is
      covered (`bookmark`). Real gap, not yet fixtured or fixed.
- [ ] index mark / table-of-contents entry (`<text:toc>`,
      `<text:table-of-content>`, `<text:alphabetical-index>`, `<text:*-mark>`
      family) — not modeled at all; falls through to the `Unknown`/raw-XML
      catch-all in `parser.rs`'s block and inline readers, so structurally
      present but not given any specific IR shape. No fixture yet.
- [ ] change tracking (`<text:tracked-changes>`, `<text:change-start>`/
      `<text:change-end>`, `<text:change>`) — ODF's own spec is
      underspecified here (see CLAUDE.md's guidance on not forcing
      artificial completeness where the spec itself doesn't define a target)
      and this crate does not model editorial deltas as an IR construct
      anywhere; per CLAUDE.md's raw-preservation tier this belongs as
      `raw_inline`/`raw_block`-equivalent preservation once the AST reaches
      that element, same as any other `Unknown` element. Falls through to
      the existing raw-preservation catch-all already; not specifically
      fixtured.

## Master pages / list numbering (styles.xml surface)
- [x] page layout (`<style:page-layout>`) — `page-layout`
- [ ] master page definition (`<style:master-page>`) binding a page-layout
      to header/footer content — `page_layouts` in the AST only captures
      the `<style:page-layout>` geometry, not `<style:master-page>`'s
      header/footer `<style:header>`/`<style:footer>` content. Real gap
      (headers/footers are silently dropped — not currently raw-preserved
      either, since `parser.rs` never visits `<office:master-styles>` at
      all). No fixture yet.
- [ ] list numbering format detail (`<text:list-level-style-number>`'s
      `style:num-format`, prefix/suffix, `text:display-levels`) — the AST's
      `list_styles: Vec<(String, bool)>` only records whether a list style
      is ordered, not its numbering format/prefix/suffix. Real gap.

## Document-level metadata beyond Dublin Core
- [x] Dublin Core core fields, custom user-defined metadata, document
      statistics — `meta-title`, `meta-author`, `meta-description`,
      `meta-date`, `meta-language`, `meta-custom`, `rare-doc-stats` (in
      `fixtures/odf/`, see "Suite fragmentation" note)
- [x] `settings.xml` (application view state — no cross-format IR meaning)
      and ODF 1.2+ package-level RDF metadata (`META-INF/manifest.rdf` and
      any other `*.rdf` part it names) — raw-preserved verbatim via
      `ast::OdfDocument::extra_parts`, not parsed into an RDF triple store
      (see that field's doc comment for why: RDF/XML triples have no
      cross-format IR equivalent this crate's node kinds could hold, and a
      hand-rolled RDF/XML parser is out of scope for a document-structure
      library). `rare-settings-and-rdf`.

## Out of scope for this crate (concrete reasons, not deferred)
- **Forms** (`<office:forms>`, `<form:*>` control elements) — form controls
  are an application/UI concept (buttons, checkboxes, list boxes bound to
  data sources), not a document-content construct any format this crate's
  IR targets has an equivalent for. No IR node kind fits; would need
  raw-preservation at minimum. Not yet touched — flagged as a real gap
  rather than silently ignored, but not attempted this pass given the size
  (a forms model is closer in scope to a UI-toolkit serialization format
  than a document format).
- **OLE / embedded objects** (`<draw:object>`, `<draw:object-ole>`, embedded
  spreadsheets/charts/formula objects as sub-documents) — each embedded
  object is itself a nested ODF (or foreign-format) package inside the ZIP;
  representing it losslessly means recursively parsing an embedded document
  and attaching it as a sub-`Document`, which the current `rescribe`
  integration point (one `Document` per top-level parse) doesn't have a
  slot for. Real gap; not attempted this pass.
- **Digital signatures** (`META-INF/documentsignatures.xml`) — signature
  bytes over the *other* package parts; preserving them verbatim is
  possible (same shape as `extra_parts`) but validating or regenerating
  them is out of scope for a document-format library, and preserving them
  without validating is close to pointless (any content edit invalidates
  the signature anyway). Not attempted this pass; a future raw-preservation
  pass could add it to `extra_parts` alongside `settings.xml`/RDF.
- **Macros / scripting** (`Basic/`, `Scripts/` package folders,
  `<office:scripts>`) — ODF defines no standardized macro language (each
  implementation's Basic dialect differs); per CLAUDE.md's guidance this is
  not a well-defined completeness target for any implementation. A
  raw-preservation pass (same `extra_parts` shape) would be the correct
  eventual treatment, not full modeling. Not attempted this pass.

## Suite fragmentation — resolved 2026-08-08

`fixtures/odf/` was a **separate, second fixture directory** for this format
(30 subdirectories, including `.odt`/`.ods`/`.odp` fixtures with more
constructs than this file lists: `rare-fields`, `rare-doc-stats`,
`rare-endnote`, `rare-space-run`, `ods-body`, `odp-body`, ...) that was not
wired into any `rescribe-fixtures` test. It is now wired (the `odf` test in
`crates/rescribe-fixtures/tests/run.rs`), which required implementing the
spreadsheet/presentation `Document` translation this note used to flag as
missing (`odf-fmt`'s `rescribe` feature, per ADR 0015 — see
`fixtures/odf/COVERAGE.md` and TODO.md's "Spreadsheet/presentation IR shape"
entry) and rewriting all 30 fixtures' `expected.json` files, which had never
matched `fixtures/spec.md`'s document-tree path semantics. `fixtures/odf/`
remains a distinct directory from this one (it is not being merged into
`fixtures/odt/`) since it also exercises `.ods`/`.odp` bodies this directory
has no equivalent for.
