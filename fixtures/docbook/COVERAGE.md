# DocBook Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

DocBook 5 reference: https://tdg.docbook.org/tdg/5.2/

## Block constructs

- [x] paragraph — `paragraph` (`<para>`)
- [x] section — `section` (`<section>` with `<title>`)
- [x] heading — `heading` (`<title>` at article level)
- [x] blockquote — `blockquote` (`<blockquote>`)
- [x] code block — `code-block` (`<programlisting>` with `language` attribute)
- [x] ordered list — `list-ordered` (`<orderedlist>`)
- [x] unordered list — `list-unordered` (`<itemizedlist>`)
- [x] definition list — `definition-list`, `definition-list-multi-entry`,
  `definition-list-multi-term` (`<variablelist>` / `<varlistentry>`; flat
  `DEFINITION_TERM`/`DEFINITION_DESC` runs as direct `definition_list`
  children — no `docbook:varlistentry` wrapper node — matching the
  convention `rescribe-read-markdown`/`rescribe-read-html` already use for
  the same IR shape. Fixed a pre-existing bug this session: the old
  `docbook:varlistentry`-wrapped shape and the writer's flat-pairs
  assumption disagreed, so any `<variablelist>` with more than one
  `<varlistentry>` wrote back corrupted (entries bled together); see
  `write_definition_list` in `rescribe-write-docbook`)
- [x] table — `table` (`<informaltable>` with `<thead>` / `<tbody>`)
- [x] figure — `figure` (`<figure>` with `<caption>` and `<mediaobject>`)
- [x] note admonition — `note` (`<note>`)
- [x] tip admonition — `tip` (`<tip>`)
- [x] warning admonition — `warning` (`<warning>`)
- [x] caution admonition — `caution` (`<caution>`)
- [x] important admonition — `important` (`<important>`)
- [x] formal table — `formal-table` (`<table>` with `<title>`, distinct from
  `<informaltable>`; title captured as the table's `title` property)
- [x] example — `example` (`<example>` block with `<title>`; raw-preserved as a
  tagged `div` with its title as an ordinary `caption` child — see the
  `heading_level_for_parent`/`CAPTION` title-round-trip fix)
- [x] screen / literallayout — `screen-literallayout` (mapped to `code_block`
  like `<programlisting>`, tagged `docbook:tag` so the writer restores the
  exact original element)
- [x] synopsis / cmdsynopsis — `synopsis-cmdsynopsis` (`<synopsis>` is
  verbatim like `<screen>`, mapped to `code_block`; `<cmdsynopsis>` has
  structured `command`/`arg`/`group` children rather than plain text, so it
  stays raw-preserved as a tagged `div`)
- [x] procedure — `procedure` (`<procedure>`/`<step>`/`<substeps>` are
  structurally a numbered list of instructions — mapped to the standard
  ordered `list`/`list_item` nodes, tagged so the writer restores the
  original element names)
- [x] nested section — `nested-section` (`<section>` inside `<section>`, 2+
  levels deep; reader output is correct — a writer bug independent of this
  fixture, where a `DIV` containing a `HEADING` plus following siblings
  doesn't reassemble them into one shared `<sectN>` on emit, is disclosed in
  TODO.md, not fixed this session — it needs the writer's section-boundary
  detection redesigned, a real design decision, not a lookup)
- [x] sidebar — `adv-unknown-block-element` (unrecognized block-level element,
  raw-preserved as a tagged `div` rather than silently dropped)
- [x] abstract — `abstract` (mapped to a `div` tagged `html:class=abstract`;
  writer previously silently dropped this tag on round-trip — fixed this
  session)
- [x] epigraph — `epigraph-attribution` (structurally a blockquote, reuses
  the `blockquote`/`docbook:type` convention already used for admonitions)
- [x] bridgehead — `bridgehead` (a free-floating `heading`, tagged
  `docbook:tag=bridgehead` so the writer re-emits a bare `<bridgehead
  renderas="sectN">` instead of wrapping it in a spurious `<sectN>` section
  the way a real nested-section heading would be)
- [x] qandaset — `qandaset` (`<qandaset defaultlabel="...">` /
  `<qandaentry>` / `<question>` / `<answer>`; resolved without any new
  `rescribe-std` node kind — `qandaset`/`qandadiv` map to `DIV` tagged
  `docbook:tag`, since `DIV` already nests arbitrarily the same way
  `generic_div`/sectioning containers do, and each `qandaentry`'s
  `question`/`answer`s flatten into a synthesized `DEFINITION_LIST` tagged
  `docbook:list-kind = "qanda"`, reusing the same flat run-grouped
  `definition_list` convention `<varlistentry>` uses — see
  `wrap_qanda_entries` in `rescribe-read-docbook` and
  `write_definition_list` in `rescribe-write-docbook`. `defaultlabel`
  preserved as `docbook:qanda-defaultlabel`; multi-answer and
  zero-answer entries both covered)
- [ ] document division elements (book/chapter/part/appendix) — (missing;
  `<book>`, `<chapter>`, `<part>`, `<appendix>` — genuine code gap, not just a
  checklist gap: the reader maps these to a bare `DIV` with no `docbook:tag`,
  unlike every other recognized element; the writer's `DIV` arm only re-emits
  a tag when `docbook:tag` is present, so on round-trip these collapse into
  plain nested sections inferred purely from heading level — the specific
  element identity is lost, not just re-nested; same root cause as the
  disclosed nested-section writer bug above but broader in scope; see
  `rescribe-read-docbook::convert_element`'s `"article" | "book" | "chapter"
  | "part" | "appendix"` arm. Full-schema audit, see TODO.md)
- [ ] front-matter/back-matter division elements — (missing;
  `<preface>`, `<colophon>`, `<dedication>`, `<glossary>`, `<index>` —
  missing-but-handled: raw-preserved via the generic-div catch-all
  (`docbook:tag` round-trips correctly), just not enumerated or
  fixture-tested. Full-schema audit, see TODO.md)
- [ ] reference/refentry structure — (missing; `<reference>`, `<refentry>`,
  `<refsect1>`/`<refsect2>`/`<refsect3>` — container handled via the
  generic-div catch-all (bookkeeping gap); their child elements
  `<refnamediv>`, `<refname>`, `<refpurpose>`, `<refsynopsisdiv>`,
  `<refmeta>`, `<refmiscinfo>`, `<refentrytitle>`, `<refclass>`,
  `<refdescriptor>` are genuinely unhandled — absent from `is_block_element`
  entirely, so a `<refnamediv>`/`<refsynopsisdiv>`/`<refmeta>` (block-shaped
  in the real content model) is misclassified as an inline span by the
  catch-all, a real fidelity risk beyond simple non-enumeration. Full-schema
  audit, see TODO.md)
- [ ] glossary entry structure — (missing; `<glossentry>`, `<glossterm>`,
  `<glossdef>`, `<glossdiv>`, `<glosslist>`, `<glosssee>`, `<glossseealso>` —
  genuinely unhandled: none appear in `rescribe-read-docbook` at all, and
  absent from `is_block_element` so `<glossentry>` (block-shaped: a
  term+definition pair) is misclassified as an inline span by the catch-all.
  Full-schema audit, see TODO.md)
- [ ] index structure — (missing; `<indexterm>`, `<indexentry>`, `<indexdiv>`,
  `<primary>`/`<secondary>`/`<tertiary>`, `<see>`/`<seealso>` — genuinely
  unhandled: none appear in `rescribe-read-docbook`; `<indexentry>`/
  `<indexdiv>` are block-shaped and misclassified as inline by the catch-all
  the same way as glossentry above. Full-schema audit, see TODO.md)
- [x] Q&A sub-structure — `qandaset-qandadiv` (`<qandadiv>` nests
  recursively inside `<qandaset>`, each with its own title — `DIV`'s
  existing arbitrary-nesting support handles this directly, no separate
  gap from the `qandaset` box above)
- [ ] entry table (nested table in a cell) — (missing; `<entrytbl>`,
  `<colgroup>`, `<col>`, `<spanspec>` — genuinely unhandled: none appear in
  the reader; absent from `is_block_element`, so `<entrytbl>` — a table
  nested inside a table cell — is misclassified as inline text rather than
  raw-preserved as a block. Full-schema audit, see TODO.md)
- [ ] programming-language synopsis family — (missing; `<classsynopsis>`,
  `<fieldsynopsis>`, `<methodsynopsis>`, `<constructorsynopsis>`,
  `<destructorsynopsis>`, `<enumsynopsis>`/`<enumitem>`/`<enumvalue>`,
  `<typedefsynopsis>`, `<funcdef>`/`<funcparams>`/`<paramdef>`/`<void>`/
  `<varargs>`/`<initializer>`/`<modifier>` — `<funcsynopsis>` itself is
  recognized in `is_block_element` and raw-preserved as a tagged div via the
  catch-all, but its structured children are not in `is_block_element` and
  fall through as inline spans nested inside — text content survives but the
  fine-grained structure (which span is `<funcdef>` vs `<funcparams>`) is
  flattened to untagged spans; narrow API-reference-doc constructs, lower
  priority. Full-schema audit, see TODO.md)
- [x] equation (display math) — `equation-mathml`, `equation-mathphrase`
  (`<equation>` / `<informalequation>` map to `math_display`, per the DocBook 5.2
  content model's three mutually-exclusive alternatives — `<mml:math>` MathML
  raw-preserved as `math:source`/`math:format="mathml"` following the HTML
  precedent, `<mathphrase>` phrase-level markup kept as real child nodes rather
  than flattened text, `<mediaobject>` kept as a plain child `image` node;
  previously mis-classified as a genuine design fork — see TODO.md's "MathML
  resolved" entry)
- [x] mediaobject (block image) — `mediaobject-block` (`<mediaobject>` as a
  direct block child, not inside `<figure>`, passes through to a standard
  `image` node; the writer already had a dedicated block-position IMAGE ->
  `<mediaobject>` arm, so this just adds the missing fixture)
- [x] programlistingco (callout listing) — `programlistingco-areaspec`
  (`<programlistingco>` — content model `areaspec?, programlisting` per the
  DocBook 5.2 reference — maps to a `div` tagged `docbook:tag =
  "programlistingco"` wrapping the `code_block`; the paired `<areaspec>`'s
  `<area>`/`<areaset>` coordinate records fold into the `code_block`'s
  `docbook:areaspec` property rather than surviving as an unrelated sibling
  node, since `coords`/`units`/`label` never carry nested markup per ADR
  0006's content-model test. Resolved this session — see TODO.md's
  "resolved" writeup for the design)
- [x] address block — `address` (verbatim like `<screen>`, mapped to
  `code_block`)

## Inline constructs

- [x] emphasis (italic) — `emphasis` (`<emphasis>`)
- [x] strong (bold) — `strong` (`<emphasis role="strong">`)
- [x] subscript — `subscript` (`<subscript>`)
- [x] superscript — `superscript` (`<superscript>`)
- [x] code (inline) — `literal` (`<literal>`)
- [x] link — `link` (`<link url="…">`)
- [x] image (inline) — `image` (`<inlinemediaobject>` / `<imagedata>`)
- [x] line break — `line-break` (`<sbr>`)
- [x] footnote — `footnote-def` (`<footnote>`)
- [x] xref (cross-reference) — `xref` (`<xref linkend="…">`, mapped to the
  standard `link` node with a synthesized `#linkend` url)
- [x] anchor — `anchor` (`<anchor xml:id="…">`, mapped to an id-only `link` node)
- [x] abbrev / acronym — `inline-abbrev-acronym` (DocBook 5.2 reference:
  plain phrase elements, #PCDATA plus common attributes; no cross-format
  equivalent — raw-preserved as tagged spans)
- [x] trademark — `inline-trademark`
- [x] keycap / keycombo — `inline-keycap-keycombo`
- [x] guilabel / guimenu / guibutton — `inline-gui-elements`
- [x] filename / command / option — `filename-command-option` (mapped to the
  standard `code` node, same as `<literal>`)
- [x] varname / function / parameter — `inline-varname-function-parameter`
- [x] classname / methodname / interfacename — `inline-oop-elements`
- [x] replaceable — `inline-replaceable`
- [x] systemitem / envar / prompt — `inline-systemitem-envar-prompt`
- [x] citetitle — `inline-citetitle`
- [x] personname — `personname` (outside `<info>`, `<personname>`/`<firstname>`/
  `<surname>` have no dedicated node — their text passes through in place)
- [x] quote — `adv-unknown-inline-element` (unrecognized inline element,
  raw-preserved as a tagged `span` in place rather than silently dropped)
- [x] phrase — `inline-phrase`
- [x] token — `inline-token`
- [x] markup — `inline-markup`
- [x] tag — `inline-tag`
- [x] uri — `inline-uri`
- [x] inlineequation (inline math) — `inlineequation-mathml` (`<inlineequation>`
  maps to `math_inline`; same content-model handling as `equation` above, using
  `<inlinemediaobject>` in place of `<mediaobject>` for the image alternative —
  see TODO.md's "MathML resolved" entry)
- [ ] indexterm / primary / secondary / tertiary / see / seealso — (missing;
  genuinely unhandled — see the index-structure entry under Block constructs;
  not modeled, falls through the generic-span catch-all. Full-schema audit,
  see TODO.md)
- [ ] person/org detail phrases — (missing; `<honorific>`, `<lineage>`,
  `<jobtitle>`, `<email>` — no dedicated mapping; `personname`'s explicit
  child-element allowlist (`firstname`/`surname`/`othername`) doesn't include
  them, so they fall to the outer generic-span catch-all rather than being
  text-extracted like their siblings. Full-schema audit, see TODO.md)
- [ ] technical/UI phrase elements — (missing; `<menuchoice>`, `<shortcut>`,
  `<mousebutton>`, `<keycode>`, `<keysym>`, `<remark>`, `<firstterm>`,
  `<foreignphrase>`, `<wordasword>`, `<database>`, `<hardware>`,
  `<application>`, `<productname>`, `<productnumber>` — all raw-preserved
  correctly via the generic-span catch-all (`adv-unknown-inline-element`
  already exercises this fallback path generically); no code gap, lower
  priority, listed here for enumeration completeness. Full-schema audit, see
  TODO.md)
- [ ] keyword / keywordset / subjectset / subjectterm — (missing; document
  classification metadata, no dedicated mapping, falls to the generic-span/
  div catch-all depending on position; narrow, low priority. Full-schema
  audit, see TODO.md)
- [x] footnoteref — `footnoteref` (`<footnoteref linkend="…">`, mapped to the
  standard `footnote_ref` node with `linkend` as the standard `label` property)
- [x] co (callout reference) — `co-callout-inline` (`<co/>` embedded inline
  in a verbatim element's mixed content — valid directly inside a bare
  `<programlisting>`, no `<programlistingco>` wrapper required, per the
  DocBook 5.2 reference's `%co.class;` content-model inclusion. `<co>` itself
  is EMPTY (no markup to preserve per ADR 0006), but its *position* in the
  flat text is real information, so it's captured as a
  `docbook:callout-markers` list property on the `code_block` — one
  `{id, offset, label}` map per marker — rather than extending
  `code_block`'s flat-string `content` contract. Resolved this session — see
  TODO.md's "resolved" writeup for the design)

## Properties

- [x] code language — `code-block` (`language` attribute on `<programlisting>`)
- [x] link role / type — `prop-link-role-type` (`xlink:type`, `xlink:role` on
  `<link>`, raw-preserved as `docbook:xlink-type`/`docbook:xlink-role`)
- [x] section xml:id — `prop-section-xml-id`
- [x] list numeration — `prop-list-numeration` (`numeration` attribute on
  `<orderedlist>` maps to the standard `list_style` property; `startingnumber`
  maps to `start`)
- [x] list spacing — `prop-list-spacing` (`spacing="compact"` maps to the
  standard `tight` property)
- [x] table frame / colsep / rowsep — `table-frame-colsep-rowsep` (raw-preserved
  as `docbook:frame`/`docbook:colsep`/`docbook:rowsep`; the same attributes on
  `<tgroup>` as a finer-grained override are not separately captured — `tgroup`
  stays a pass-through wrapper — a disclosed narrow gap, not this fixture's claim)
- [x] table colspec widths — `table-colspec-widths` (`<colspec>` modeled as a
  structured `docbook:colspec` child carrying `docbook:colname`/`docbook:colwidth`/
  `align`, kept out of the row list)
- [x] table spanning cells — `table-spanning-cells` (`morerows` maps to the
  standard `rowspan` property; `namest`/`nameend` raw-preserved verbatim since
  resolving a column-name span to a column count needs sibling-colspec lookup
  context the per-entry conversion doesn't have)
- [x] xml:lang — `prop-xml-lang` (standard `language` property, applied
  uniformly to every element via `attach_generic_attrs`, not just `<para>`)
- [x] revision / revhistory — `header-revhistory` (raw-preserved verbatim via
  `revhistory_raw` metadata, alongside a flattened summary — the general
  `<info>` front-matter fallback, same mechanism as `header-author`)
- [x] author / orgname in info — `header-author` (`<author>` has no
  dedicated semantic mapping; raw-preserved verbatim as `author_raw`
  metadata, alongside a flattened `author` summary — exercises the general
  `<info>` front-matter fallback, not a hardcoded special case)
- [x] pubdate / publisher — `header-pubdate-publisher` (raw-preserved verbatim
  via `pubdate_raw`/`publisher_raw` metadata, same general fallback as
  `header-author`)

## Bibliography / citation

Cross-format IR shape (`bibliography`/`bibliography_entry`/`bibliography_field`
node kinds, `field:role`/`field:scheme`/`date` properties — see
`rescribe-std`'s `node`/`prop` doc comments and `docs/adr/0005-citation-bibliography-ir-shape.md`)
schema-verified against DocBook 5.2, JATS 1.3, TEI P5, and OOXML's `b:`
namespace. This section covers the DocBook side only.

- [x] bibliography container — `citation-simple-author` (`<bibliography>`
  mapped to the standard `bibliography` node; its own `<title>` is a heading,
  same as `<chapter>`, per the `heading_level_for_parent` fix)
- [x] biblioentry, single structured author — `citation-simple-author`
  (`<biblioentry>` mapped to `bibliography_entry`; `<author>`/`<title>`/
  `<publisher>`/`<publishername>`/`<pagenums>`/`<biblioid class="...">` each
  mapped to a `bibliography_field` with the matching `field:role`; `class`
  round-trips as `field:scheme`)
- [x] biblioentry, multiple authors — `citation-multi-author` (`<authorgroup>`
  of two `<author>`s becomes two sibling `field:role=author` nodes in
  document order, not merged or overwritten; also covers `<volumenum>`/
  `<issuenum>`)
- [x] markup nested inside a field — `citation-markup-in-field` (`<emphasis>`
  inside a `<title>` survives as a real `emphasis` node inside the
  `bibliography_field`, concretely proving the field-node design — not a
  flat string property — actually preserves nested markup; round-trip
  verified through `rescribe-read-docbook` → `rescribe-write-docbook` →
  reparse)
- [x] bibliomixed (mixed free-text content) — `citation-bibliomixed`
  (`<bibliomixed>` mapped to `bibliography_entry` tagged
  `docbook:tag=bibliomixed`; plain text interspersed between `<author>`/
  `<title>` stays as ordinary sibling text nodes rather than being wrapped
  in a spurious field — writer fix: free text inside an entry is re-emitted
  via `write_inline`, not routed through the field writer)
- [x] biblioset nesting — `citation-biblioset` (`<biblioset>` grouping a
  sub-citation, e.g. a journal/article split — modeled as a nested
  `bibliography_entry`; `relation` attribute raw-preserved as
  `docbook:biblioset-relation`)
- [x] page range splitting — `citation-simple-author` (`<pagenums>12-34</pagenums>`
  splits into `page_first`/`page_last` fields; the writer's round trip
  recombines them back into one `<pagenums>`). The ambiguous/non-numeric
  case (e.g. `"12, 34, 56"`) is not covered by a dedicated fixture, only by
  manual round-trip verification during development — it's kept whole as a
  `misc` field with the original string additionally raw-preserved under
  `docbook:pagenums` rather than guessed at
- [x] bibliographic date — exercised only via `rescribe-read-docbook`'s
  manual round-trip verification, not a dedicated fixture (no fixture format
  currently asserts `PropValue::Map` properties — see `fixtures/spec.md`
  v1.2's property-matching table); an ISO 8601 `<date>`/`<pubdate>` becomes
  the structured `prop::DATE` map, a free-text one (e.g. `"Spring 2020"`) is
  kept as an ordinary `misc` field instead of being guessed at

## Composition (integration)

- [x] nested list — `int-nested-list` (`<itemizedlist>` inside `<listitem>`)
- [x] table with inline formatting in cells — `int-table-inline`
- [x] section with admonition and code block — `int-section-admonition-code`
- [x] blockquote with attribution — `int-blockquote-attribution` (`<attribution>`
  inside a plain `<blockquote>`, not just `<epigraph>`)
- [x] figure with alt text — `int-figure-alt-text` (`<textobject><phrase>` inside
  `<mediaobject>` folded into the standard `alt` property on the image node,
  rather than left as an unrelated sibling — a real gap closed this session,
  see rescribe-read-docbook's new "mediaobject"/"inlinemediaobject" arm)
- [x] footnote in table cell — `int-footnote-table-cell` (also fixed a writer
  bug found while verifying round-trip: `FOOTNOTE_DEF` embedded inline, e.g.
  in a table cell, had no `write_inline` arm and silently lost its
  `<footnote>` wrapper, splicing the note's text straight into the cell)
- [x] callout listing + callout list — `co-callout-inline`,
  `programlistingco-areaspec` (both fixtures pair a `<calloutlist>` with its
  target: the inline-marker flavor's `<co>` ids and the external-coordinates
  flavor's `<area>` ids, respectively. `<calloutlist>`/`<callout>` map to
  `list`/`list_item` tagged `docbook:tag`, matching the existing
  `procedure`/`step` convention — `<callout>`'s content is ordinary block
  markup (real prose, per ADR 0006), so it stays real child nodes; `arearefs`
  is plain IDREFS attribute data, raw-preserved as a `docbook:arearefs`
  string property. Resolved this session — see TODO.md's "resolved" writeup)
- [x] article-level metadata (info block) — `e2e-article-metadata` (`<info>`
  with `<title>`, `<author>`, `<pubdate>`)

## Adversarial

- [x] empty document — `adv-empty`
- [x] malformed XML (unclosed tag) — `adv-malformed-xml` (recovers best-effort,
  reports diagnostics, never panics)
- [x] unknown DocBook element — `adv-unknown-block-element` (block-shaped,
  raw-preserved as a tagged `div`), `adv-unknown-inline-element`
  (inline-shaped, raw-preserved as a tagged `span` in place) — neither is
  silently dropped
- [x] missing required namespace declaration — `adv-no-namespace` (elements
  matched by local name, not namespace-qualified)
- [x] entity references (&amp;, &lt;, &gt;, &apos;, &quot;) — `adv-entity-references`
- [x] numeric character references (&#160;, &#x2019;) — `adv-numeric-char-ref`
- [x] entity declared in the document's own DOCTYPE internal subset,
  resolved via the `xml-entities` crate — `dtd-entity-resolution`
- [x] named entity resolved via the standard WHATWG/ISO table with no
  DOCTYPE present — `rare-named-entity-standard-table`
- [x] named entity unresolvable by either layer, still raw-preserved as
  `raw_inline` — `adv-unresolvable-entity`
- [x] deeply nested sections (6+ levels) — `adv-deeply-nested-sections`
- [x] empty para — `adv-empty-para` (`<para/>`)
- [x] para with only whitespace — `adv-whitespace-para`

## Pathological

- [x] very large table (many rows/columns) — `path-large-table` (50 rows x 6 columns)
- [x] deeply nested lists (4+ levels) — `path-deeply-nested-lists` (6 levels)
- [x] section nesting at maximum DocBook depth — `path-max-section-depth`
  (15 levels — `<section>` is recursive with no schema-enforced depth limit)
- [x] long document with many sections — `path-long-document` (100 sibling
  top-level sections)
