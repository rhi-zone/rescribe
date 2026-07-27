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
- [x] definition list — `definition-list` (`<variablelist>` / `<varlistentry>`)
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
- [ ] qandaset — (missing; `<qandaset>` / `<qandaentry>` — left open: no
  dedicated mapping attempted this session, still raw-preserved generically
  via `generic_div`/`generic_span` per the catch-all, but not verified with a
  fixture; a real Q&A-list IR mapping is a design choice, see TODO.md)
- [ ] equation (display math) — (missing; `<equation>` / `<mathphrase>` or
  MathML — left open together with `inlineequation`, same MathML-modeling
  design fork, see TODO.md)
- [x] mediaobject (block image) — `mediaobject-block` (`<mediaobject>` as a
  direct block child, not inside `<figure>`, passes through to a standard
  `image` node; the writer already had a dedicated block-position IMAGE ->
  `<mediaobject>` arm, so this just adds the missing fixture)
- [ ] programlistingco (callout listing) — (missing; `<programlistingco>` +
  `<calloutlist>` — left open together with the `co` inline construct above:
  designing one without the other would be premature, see TODO.md)
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
- [ ] inlineequation (inline math) — (missing; `<inlineequation>` / MathML —
  left open: modeling this needs a real design decision — whether to reuse
  rescribe-math's `math_inline` node with the MathML captured as
  `math:source`/raw content, or something else — not a lookup-verifiable
  answer, see TODO.md)
- [x] footnoteref — `footnoteref` (`<footnoteref linkend="…">`, mapped to the
  standard `footnote_ref` node with `linkend` as the standard `label` property)
- [ ] co (callout reference) — (missing; `<co>` — left open together with the
  `programlistingco (callout listing)` block construct below: `co` only has
  meaning paired with a `<calloutlist>` that references it back, so mapping
  one without designing the other would be premature; a real design decision,
  not a lookup, see TODO.md)

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
- [ ] callout listing + callout list — (missing; left open together with `co`/
  `programlistingco` above — same design fork, see TODO.md)
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
