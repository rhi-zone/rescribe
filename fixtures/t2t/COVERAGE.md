# txt2tags Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

txt2tags (t2t) uses a three-section document structure: header (lines 1–3), settings
(%%...%%), and body. The reference is the txt2tags user guide and source.

## Block constructs
- [x] paragraph — `paragraph`
- [x] heading h1 (= Heading =) — `heading`
- [x] heading h2 (== Heading ==) — `heading-h2`
- [x] numbered heading (+ Heading +) — `heading-numbered`
- [ ] heading h3–h5 (=== … =====) — (missing)
- [ ] numbered heading h2–h5 (++ … +++++) — (missing)
- [x] unordered list (- item) — `list-unordered`
- [x] ordered list (+ item) — `list-ordered`
- [ ] definition list (: term : definition) — (missing)
- [ ] nested list — (missing)
- [x] blockquote (\t indent) — `blockquote`
- [x] code block (``` … ```) — `code-block`
- [x] horizontal rule (= = = = = =  or - - - - - -) — `horizontal-rule`
- [x] table — `table`
- [x] table with header row — `table-header`
- [x] image (!image.png!) — `image`
- [x] raw block block (%!postproc or ``` raw ```) — `raw-block`
- [x] comment line (%) — `rare-comment`
- [ ] tagged block (\`\`\` tagged \`\`\`) — (missing)
- [ ] multi-line comment (%% … %%) — (missing)
- [ ] separator line (- - - - or = = = =) — (missing; distinct from hr)
- [ ] title with anchor — (missing)
- [ ] include macro (%!include) — (missing)

## Inline constructs
- [x] italic (/text/) — `italic`
- [x] bold (**text**) — `bold`
- [x] strikethrough (--text--) — `strikethrough`
- [x] underline (__text__) — `rare-underline`
- [x] inline code (``text``) — `rare-code-inline`
- [x] link ([label url] or bare URL) — `link`
- [ ] image inline (!img.png!) — (missing; image block covered)
- [ ] named link anchor ([label #anchor]) — (missing)
- [ ] verbatim (""text"") — (missing)
- [ ] tagged inline (''text'') — (missing)
- [ ] line break — (missing)

## Properties
- [ ] document header (title, author, date — lines 1–3) — (missing)
- [ ] settings section (%!setting) — (missing)
- [ ] postproc / preproc macros — (missing)
- [ ] target format in tagged block — (missing)
- [ ] image dimensions / align — (missing)
- [ ] table column alignment — (missing)
- [ ] list item continuation — (missing)
- [ ] heading anchor — (missing)

## Composition (integration)
- [ ] nested lists — (missing)
- [ ] table with inline formatting in cells — (missing)
- [ ] blockquote containing a list — (missing)
- [ ] list item with inline code — (missing)
- [ ] heading followed immediately by list — (missing)
- [ ] link inside bold — (missing)

## Adversarial
- [x] empty document — `adv-empty`
- [x] unknown / unrecognized construct — `adv-unknown`
- [ ] heading without closing marker — (missing)
- [ ] malformed table — (missing)
- [ ] unclosed code block — (missing)
- [ ] link with missing closing bracket — (missing)

## Pathological
- [ ] document with many sections — (missing)
- [ ] very large table — (missing)
- [ ] deeply nested lists — (missing)
- [ ] very long paragraph — (missing)
- [ ] heading at every level — (missing)
