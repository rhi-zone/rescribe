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
- [x] option list (falls through to paragraph; no dedicated parser support) — `option-list`
- [x] footnote definition (dropped as comment; ref stays as text) — `footnote-def`
- [x] citation definition (dropped as comment; ref stays as text) — `citation`
- [x] comment (.. ) — `comment`
- [x] section numbering (.. sectnum:: → div with rst:directive; heading preserved) — `sectnum`
- [ ] include directive (.. include::) — (N/A: requires filesystem; out of scope for fixture testing)
- [x] class directive (.. class:: → div with rst:directive=class) — `class-directive`

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
- [x] anonymous hyperlink (`text`__) — `anonymous-link`
- [x] embedded URI (`text <url>`_) — `link`
- [x] footnote reference ([1]_ stays as literal text) — `footnote-def`
- [x] citation reference ([label]_ stays as literal text) — `citation`
- [x] substitution reference (|sub|) — `substitution`
- [x] substitution definition (.. |sub| replace::) — `substitution`
- [x] interpreted text (`:role:`text`` with unknown role → span; default role → emphasis) — covered by `rst-span` and `emphasis`
- [x] image inline (|image_sub| → literal text; image:: substitutions not expanded) — `image-inline`
- [ ] line break (hard) — (N/A: RST has no hard line break outside line blocks)

## Properties
- [x] image URI and alt (width/height present in source but not modeled in IR) — `image-props`
- [x] figure caption and legend — `figure-caption`
- [x] code block language — `code-block-directive`
- [x] link target URL — `link-target-url`
- [x] admonition title (custom) — `admonition-custom-title`
- [x] list item continuation (produces blockquote) — `integration-list-item-blockquote`
- [x] heading overline style — `heading-overline`
- [x] raw directive format attribute — `raw-format-attr`
- [x] table widths, header-rows, stub-columns (csv-table options not modeled; div with rst:directive) — `table-props`
- [x] role options (.. role:: with :language: → div with rst:directive=role; usage becomes span) — `role-options`

## Composition (integration)
- [x] nested blockquotes (produces sequential blockquotes, not nested) — `integration-nested-blockquote`
- [x] list item containing a blockquote — `integration-list-item-blockquote`
- [x] list item containing a code block — `integration-code-in-list`
- [x] heading followed immediately by a list — `integration-heading-then-list`
- [x] inline markup inside a link label — `integration-inline-in-link`
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
- [x] many substitution definitions (55 defs + refs; all expand without panic) — `path-substitutions`
