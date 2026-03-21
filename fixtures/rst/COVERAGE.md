# RST Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

## Block constructs
- [x] paragraph — `paragraph`
- [x] heading (h1) — `heading`
- [x] heading (h2) — `heading-h2`
- [ ] heading (h3–h6) — (missing)
- [x] unordered list — `list-unordered`
- [x] ordered list — `list-ordered`
- [ ] nested list — (missing)
- [ ] definition list — `definition-list`
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
- [ ] topic (.. topic:: directive) — (missing)
- [ ] sidebar (.. sidebar:: directive) — (missing)
- [ ] rubric (.. rubric:: directive) — (missing)
- [ ] epigraph (.. epigraph:: directive) — (missing)
- [ ] highlights (.. highlights:: directive) — (missing)
- [ ] pull-quote (.. pull-quote:: directive) — (missing)
- [ ] compound (.. compound:: directive) — (missing)
- [ ] container (.. container:: directive) — (missing)
- [ ] table (.. table:: directive) — (missing)
- [ ] csv-table (.. csv-table:: directive) — (missing)
- [ ] list-table (.. list-table:: directive) — (missing)
- [ ] line block (| prefix) — (missing)
- [ ] bullet list with auto-enumeration (#.) — (missing)
- [ ] field list — (missing)
- [ ] option list — (missing)
- [ ] footnote definition (.. [1] or .. [*]) — (missing)
- [ ] citation definition (.. [label]) — (missing)
- [ ] comment (.. ) — (missing)
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
- [ ] footnote reference ([1]_ or [*]_) — (missing)
- [ ] citation reference ([label]_) — (missing)
- [ ] substitution reference (|sub|) — (missing)
- [ ] substitution definition (.. |sub| replace::) — (missing)
- [ ] interpreted text (`:role:`text``) — (missing)
- [ ] image inline (|image_sub|) — (missing)
- [ ] line break (hard) — (missing)

## Properties
- [ ] image URI, alt, width, height, scale, align, target — (missing)
- [ ] figure caption and legend — (missing)
- [x] code block language — `code-block-directive`
- [ ] link target URL — (missing)
- [ ] admonition title (custom) — (missing)
- [ ] list item continuation — (missing)
- [ ] heading overline style — (missing)
- [ ] raw directive format attribute — (missing)
- [ ] table widths, header-rows, stub-columns — (missing)
- [ ] role options (class, language) — (missing)

## Composition (integration)
- [ ] nested blockquotes — (missing)
- [ ] list item containing a blockquote — (missing)
- [ ] list item containing a code block — (missing)
- [ ] heading followed immediately by a list — (missing)
- [ ] inline markup inside a link label — (missing)
- [ ] admonition containing a list — (missing)
- [ ] table cell with inline formatting — (missing)
- [ ] figure with alt text and caption — (missing)

## Adversarial
- [x] empty document — `adv-empty`
- [x] unknown directive — `adv-unknown-directive`
- [x] unmatched emphasis delimiter — `adv-unmatched-emphasis`
- [ ] malformed hyperlink target — (missing)
- [ ] overline/underline mismatch — (missing)
- [ ] duplicate section title — (missing)
- [ ] unterminated inline literal — (missing)
- [ ] deeply nested sections — (missing)

## Pathological
- [ ] document with hundreds of sections — (missing)
- [ ] very long paragraph (no newlines) — (missing)
- [ ] deeply nested lists (10+ levels) — (missing)
- [ ] large table (100+ columns) — (missing)
- [ ] many substitution definitions — (missing)
