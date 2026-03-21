# RST Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

## Block constructs
- [x] paragraph — `paragraph`
- [x] heading (h1) — `heading`
- [x] heading (h2) — `heading-h2`
- [x] heading (h3–h6) — `heading-h3`
- [x] unordered list — `list-unordered`
- [x] ordered list — `list-ordered`
- [x] nested list (flattened by parser) — `nested-list`
- [x] definition list — `definition-list`
- [x] blockquote — `blockquote`
- [x] code block (fenced / `::`) — `code-block`
- [x] code block (.. code-block:: directive) — `code-block-directive`
- [x] horizontal rule (transition) — `horizontal-rule`
- [x] figure (.. figure:: directive) — `figure`
- [x] image (.. image:: directive) — `rare-image`
- [x] admonition (.. note::, .. warning::) — `admonition-warning`
- [x] admonition important — `admonition-important`
- [x] admonition tip — `admonition-tip`
- [x] rare admonitions (danger, caution, hint, attention, error) — `rare-admonition`
- [x] math block (.. math:: directive) — `math-display`
- [x] raw block (.. raw:: directive) — `raw-block`
- [x] topic (.. topic:: directive) — `topic`
- [x] sidebar (.. sidebar:: directive) — `sidebar`
- [x] rubric (.. rubric:: directive) — `rubric`
- [x] epigraph (.. epigraph:: directive) — `epigraph`
- [x] highlights (.. highlights:: directive) — `highlights`
- [x] pull-quote (.. pull-quote:: directive) — `pull-quote`
- [x] compound (.. compound:: directive) — `compound`
- [x] container (.. container:: directive) — `container`
- [x] table (.. table:: directive) — `table-directive`
- [x] csv-table (.. csv-table:: directive) — `csv-table`
- [x] list-table (.. list-table:: directive) — `list-table`
- [x] grid table (not parsed; treated as paragraph) — `table-grid`
- [x] simple table (not parsed; treated as paragraph) — `table-simple`
- [x] line block (| prefix) — `line-block`
- [x] bullet list with auto-enumeration (#.) — `list-auto-enum`
- [x] field list (parsed as paragraph; no field-list support) — `field-list`
- [ ] option list — (missing)
- [x] footnote definition (dropped as comment; ref stays as text) — `footnote-def`
- [x] citation definition (dropped as comment; ref stays as text) — `citation`
- [x] comment (.. ) — `comment`
- [ ] section numbering (.. sectnum::) — (missing)
- [ ] include directive (.. include::) — (missing)
- [ ] class directive (.. class::) — (missing)

## Inline constructs
- [x] emphasis (*text*) — `emphasis`
- [x] strong (**text**) — `strong`
- [x] inline code (``text``) — `code-inline`
- [x] hyperlink (standalone URL or `text <url>`_) — `link`
- [x] named hyperlink reference — `rare-link-named`
- [x] subscript (:sub:`text`) — `subscript`
- [x] superscript (:sup:`text`) — `superscript`
- [x] strikeout — `strikeout`
- [x] underline — `underline`
- [x] small-caps — `small-caps`
- [x] math inline (:math:`expr`) — `math-inline`
- [x] custom role span (.. role:: or :role:`text`) — `rst-span`
- [ ] anonymous hyperlink (`text`__) — (missing)
- [ ] embedded URI (`text <url>`_) — (missing)
- [x] footnote reference ([1]_ stays as literal text) — `footnote-def`
- [x] citation reference ([label]_ stays as literal text) — `citation`
- [ ] substitution reference (|sub|) — (missing)
- [ ] substitution definition (.. |sub| replace::) — (missing)
- [ ] interpreted text (`:role:`text``) — (missing)
- [ ] image inline (|image_sub|) — (missing)
- [ ] line break (hard) — (missing)

## Properties
- [x] image URI and alt (width/height present in source but not modeled in IR) — `image-props`
- [x] figure caption and legend — `figure-caption`
- [x] code block language — `code-block-directive`
- [ ] link target URL — (missing)
- [ ] admonition title (custom) — (missing)
- [ ] list item continuation — (missing)
- [ ] heading overline style — (missing)
- [ ] raw directive format attribute — (missing)
- [ ] table widths, header-rows, stub-columns — (missing)
- [ ] role options (class, language) — (missing)

## Composition (integration)
- [x] nested blockquotes (produces sequential blockquotes, not nested) — `integration-nested-blockquote`
- [ ] list item containing a blockquote — (missing)
- [x] list item containing a code block — `integration-code-in-list`
- [x] heading followed immediately by a list — `integration-heading-then-list`
- [ ] inline markup inside a link label — (missing)
- [x] admonition containing a list (content flattened to paragraph) — `integration-list-in-admonition`
- [x] table cell with inline formatting (not parsed as table) — `integration-inline-in-table`
- [x] figure with alt text and caption — `figure-caption`

## Adversarial
- [x] empty document — `adv-empty`
- [x] unknown directive — `adv-unknown-directive`
- [x] unmatched emphasis delimiter — `adv-unmatched-emphasis`
- [x] malformed hyperlink target — `adv-malformed-hyperlink`
- [x] overline/underline mismatch — `adv-heading-mismatch`
- [x] duplicate section title — `adv-duplicate-heading`
- [x] unterminated inline literal — `adv-unterminated-literal`
- [x] deeply nested sections — `adv-deeply-nested`
- [x] truncated document — `adv-truncated`

## Pathological
- [x] document with 50 sections — `path-many-sections`
- [x] very long paragraph (no newlines) — `path-long-paragraph`
- [x] deeply nested lists (9 levels) — `path-deep-list`
- [x] wide table (20 columns) — `path-wide-table`
- [ ] many substitution definitions — (missing)
