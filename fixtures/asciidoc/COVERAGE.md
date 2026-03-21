# AsciiDoc Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

## Block constructs
- [x] paragraph — `paragraph`
- [x] heading (h1 / = Title) — `heading`
- [x] heading (h2 / == Section) — `heading-h2`
- [ ] heading (h3–h6) — (missing)
- [x] unordered list (* or -) — `list-unordered`
- [x] ordered list (. prefix) — `list-ordered`
- [ ] nested list — (missing)
- [x] description list (term:: description) — `rare-description-list`
- [x] blockquote — `rare-blockquote`
- [x] code block (---- or [source,...]) — `code-block`
- [x] code block with source language attribute — `code-block-source`
- [x] horizontal rule (''') — `horizontal-rule`
- [x] page break (<<<) — `page-break`
- [x] image block (image:: macro) — `figure`
- [x] admonition (NOTE:, TIP:, IMPORTANT:, WARNING:, CAUTION:) — `rare-admonition`
- [x] raw passthrough block (++++) — `raw-block`
- [ ] example block (====) — (missing)
- [ ] sidebar block (**** ) — (missing)
- [ ] quote block (____) — (missing)
- [ ] verse block (____  with [verse]) — (missing)
- [ ] literal block (.....) — (missing)
- [ ] open block (--) — (missing)
- [ ] comment block (////) — (missing)
- [ ] table (|=== syntax) — (missing)
- [ ] discrete heading ([discrete]) — (missing)
- [ ] include directive (include::) — (missing)
- [ ] conditional directives (ifdef::, ifndef::, ifeval::) — (missing)
- [ ] attribute definition (:attr: value) — (missing)
- [ ] block title (.Title) — (missing)
- [ ] list continuation (+) — (missing)
- [ ] ordered list start offset — (missing)
- [ ] checklist (- [ ] / - [x]) — (missing)
- [ ] callout list (<1> ... <1> annotations) — (missing)
- [ ] bibliography list (- [ref]) — (missing)
- [ ] footnote block — (missing)

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
- [ ] footnote macro (footnote:[text]) — (missing)
- [ ] anchor (([[id]])) — (missing)
- [ ] bibliography ref (<<bib>>) — (missing)
- [ ] keyboard macro (kbd:[Ctrl+C]) — (missing)
- [ ] button macro (btn:[OK]) — (missing)
- [ ] menu macro (menu:File[Open]) — (missing)
- [ ] attribute reference ({attr}) — (missing)
- [ ] indexterm macro ((((term)))) — (missing)
- [ ] pass macro (pass:[raw]) — (missing)
- [ ] inline literal passthrough (+raw+) — (missing)

## Properties
- [ ] document title metadata (author, revnumber, revdate) — (missing)
- [x] image alt text, width, height — `figure`
- [x] code block language — `code-block-source`
- [ ] table column spec (cols= attribute) — (missing)
- [ ] table header row — (missing)
- [ ] link window target — (missing)
- [ ] block id ([#id]) — (missing)
- [ ] block role ([.role]) — (missing)
- [ ] block option ([%option]) — (missing)
- [ ] list marker style (arabic, loweralpha, etc.) — (missing)
- [ ] section numbering attribute — (missing)
- [x] unknown attribute — `adv-unknown-attr`

## Composition (integration)
- [ ] admonition containing a list — (missing)
- [ ] table cell with inline formatting — (missing)
- [ ] nested lists (unordered inside ordered) — (missing)
- [ ] list continuation block — (missing)
- [ ] example block containing a code block — (missing)
- [ ] link with formatted text label — (missing)
- [ ] heading with inline formatting — (missing)
- [ ] callout annotation in code block — (missing)

## Adversarial
- [x] empty document — `adv-empty`
- [x] unknown attribute — `adv-unknown-attr`
- [ ] unclosed delimiter block — (missing)
- [ ] malformed table — (missing)
- [ ] unmatched inline markup — (missing)
- [ ] attribute reference to undefined attribute — (missing)
- [ ] deeply nested blocks — (missing)

## Pathological
- [ ] document with hundreds of sections — (missing)
- [ ] very large table — (missing)
- [ ] deeply nested lists — (missing)
- [ ] many attribute definitions — (missing)
- [ ] very long line — (missing)
