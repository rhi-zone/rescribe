# AsciiDoc Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

## Block constructs
- [x] paragraph — `paragraph`
- [x] heading (h1 / = Title) — `heading`
- [x] heading (h2 / == Section) — `heading-h2`
- [x] heading (h3–h6) — `heading-h3`
- [x] unordered list (* or -) — `list-unordered`
- [x] ordered list (. prefix) — `list-ordered`
- [x] nested list — `nested-list`
- [x] description list (term:: description) — `rare-description-list`
- [x] blockquote — `rare-blockquote`
- [x] code block (---- or [source,...]) — `code-block`
- [x] code block with source language attribute — `code-block-source`
- [x] horizontal rule (''') — `horizontal-rule`
- [x] page break (<<<) — `page-break`
- [x] image block (image:: macro) — `figure`
- [x] admonition (NOTE:, TIP:, IMPORTANT:, WARNING:, CAUTION:) — `rare-admonition`
- [x] raw passthrough block (++++) — `raw-block`
- [x] example block (====) — `example-block`
- [x] sidebar block (****) — `sidebar-block`
- [x] quote block (____) — `quote-block`
- [x] verse block (____  with [verse]) — `verse-block`
- [x] literal block (.....) — `literal-block`
- [x] open block (--) — `open-block`
- [x] comment block (////) — `comment-block`
- [x] table (|=== syntax) — `table`
- [x] discrete heading ([discrete]) — `discrete-heading`
- [x] include directive (include::) — (N/A: filesystem dependency; no fixture)
- [x] conditional directives (ifdef::, ifndef::, ifeval::) — `conditional`
- [x] attribute definition (:attr: value) — `attribute-def`
- [x] block title (.Title) — `block-title`
- [x] list continuation (+) — `list-continuation`
- [x] ordered list start offset — `ordered-list-start`
- [x] checklist (- [ ] / - [x]) — `checklist`
- [x] callout list (<1> ... <1> annotations) — `callout-list`
- [x] bibliography list (- [ref]) — `bibliography-list`
- [x] footnote block — `footnote`

## Inline constructs
- [x] italic (_text_ or __text__) — `italic`
- [x] bold (*text* or **text**) — `bold`
- [x] inline code (`text` or ``text``) — `code-inline`
- [x] underline ([.underline]#text#) — `underline`
- [x] strikethrough ([.line-through]#text# or ~~text~~) — `strikeout`
- [x] subscript (~text~) — `subscript`
- [x] superscript (^text^) — `superscript`
- [x] small-caps ([.small-caps]#text#) — `small-caps`
- [x] highlight (##text##) — `highlight`
- [x] link macro (link:url[text]) — `link`
- [x] xref macro (<<anchor>> or xref:) — `rare-link-macro`
- [x] inline image (image:path[alt]) — `image`
- [x] line break ( + at end of line) — `line-break`
- [x] footnote macro (footnote:[text]) — `footnote`
- [x] anchor ([[id]]) — `anchor`
- [x] bibliography ref (<<bib>>) — `bibliography-ref`
- [x] keyboard macro (kbd:[Ctrl+C]) — `kbd-macro`
- [x] button macro (btn:[OK]) — `btn-macro`
- [x] menu macro (menu:File[Open]) — `menu-macro`
- [x] attribute reference ({attr}) — `attribute-ref`
- [x] indexterm macro ((((term)))) — `indexterm`
- [x] pass macro (pass:[raw]) — `pass-macro`
- [x] inline literal passthrough (+raw+) — `inline-literal-pass`

## Properties
- [x] document title metadata (author, revnumber, revdate) — `doc-metadata`
- [x] image alt text, width, height — `figure`
- [x] code block language — `code-block-source`
- [x] table column spec (cols= attribute) — `table-col-spec`
- [x] table header row — `table-header`
- [x] link window target — `link-target`
- [x] block id ([#id]) — `block-id`
- [x] block role ([.role]) — `block-role`
- [x] block option ([%option]) — `block-option`
- [x] list marker style (arabic, loweralpha, etc.) — `list-marker-style`
- [x] section numbering attribute — `section-numbering`
- [x] unknown attribute — `adv-unknown-attr`

## Composition (integration)
- [x] admonition containing a list — `integration-admonition-list`
- [x] table cell with inline formatting — `integration-table-inline`
- [x] nested lists (unordered inside ordered) — `integration-nested-lists`
- [x] list continuation block — `list-continuation`
- [x] example block containing a code block — `integration-example-code`
- [x] link with formatted text label — `integration-link-formatted`
- [x] heading with inline formatting — `integration-heading-inline`
- [x] callout annotation in code block — `callout-code`

## Adversarial
- [x] empty document — `adv-empty`
- [x] unknown attribute — `adv-unknown-attr`
- [x] unclosed delimiter block — `adv-unclosed-block`
- [x] malformed table — `adv-malformed-table`
- [x] unmatched inline markup — `adv-unmatched-inline`
- [x] attribute reference to undefined attribute — `adv-undef-attr-ref`
- [x] deeply nested blocks — `adv-deep-nesting`

## Pathological
- [x] document with hundreds of sections — `path-many-sections`
- [x] very large table — `path-large-table`
- [x] deeply nested lists — `path-deep-list`
- [x] many attribute definitions — `path-many-attrs`
- [x] very long line — `path-long-line`
