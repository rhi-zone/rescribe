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
- [ ] formal table — (missing; `<table>` with `<title>`, distinct from `<informaltable>`)
- [ ] example — (missing; `<example>` block with `<title>`)
- [ ] screen / literallayout — (missing; `<screen>`, `<literallayout>`)
- [ ] synopsis / cmdsynopsis — (missing; `<synopsis>`, `<cmdsynopsis>`)
- [ ] procedure — (missing; `<procedure>` with `<step>`)
- [ ] nested section — (missing; `<section>` inside `<section>`, 2+ levels deep)
- [ ] sidebar — (missing; `<sidebar>`)
- [ ] abstract — (missing; `<abstract>`)
- [ ] epigraph — (missing; `<epigraph>`)
- [ ] bridgehead — (missing; floating `<bridgehead>` not tied to a section)
- [ ] qandaset — (missing; `<qandaset>` / `<qandaentry>`)
- [ ] equation (display math) — (missing; `<equation>` / `<mathphrase>` or MathML)
- [ ] mediaobject (block image) — (missing; `<mediaobject>` as a direct block child, not inside `<figure>`)
- [ ] programlistingco (callout listing) — (missing; `<programlistingco>` + `<calloutlist>`)
- [ ] address block — (missing; `<address>`)

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
- [ ] xref (cross-reference) — (missing; `<xref linkend="…">`)
- [ ] anchor — (missing; `<anchor xml:id="…">`)
- [ ] abbrev / acronym — (missing; `<abbrev>`, `<acronym>`)
- [ ] trademark — (missing; `<trademark>`)
- [ ] keycap / keycombo — (missing; `<keycap>`, `<keycombo>`)
- [ ] guilabel / guimenu / guibutton — (missing; GUI inline elements)
- [ ] filename / command / option — (missing; `<filename>`, `<command>`, `<option>`)
- [ ] varname / function / parameter — (missing; `<varname>`, `<function>`, `<parameter>`)
- [ ] classname / methodname / interfacename — (missing; OOP semantic inlines)
- [ ] replaceable — (missing; `<replaceable>`)
- [ ] systemitem / envar / prompt — (missing; system inline elements)
- [ ] citetitle — (missing; `<citetitle>`)
- [ ] personname — (missing; `<personname>`)
- [ ] quote — (missing; `<quote>`)
- [ ] phrase — (missing; `<phrase>`)
- [ ] token — (missing; `<token>`)
- [ ] markup — (missing; `<markup>`)
- [ ] tag — (missing; `<tag>`)
- [ ] uri — (missing; `<uri>`)
- [ ] inlineequation (inline math) — (missing; `<inlineequation>` / MathML)
- [ ] footnoteref — (missing; `<footnoteref linkend="…">`)
- [ ] co (callout reference) — (missing; `<co>`)

## Properties

- [x] code language — `code-block` (`language` attribute on `<programlisting>`)
- [ ] link role / type — (missing; `xlink:type`, `xlink:role` on `<link>`)
- [ ] section xml:id — (missing; `xml:id` attribute on `<section>`)
- [ ] list numeration — (missing; `numeration` attribute on `<orderedlist>` — arabic, loweralpha, etc.)
- [ ] list spacing — (missing; `spacing` attribute — compact vs normal)
- [ ] table frame / colsep / rowsep — (missing; CALS table model attributes)
- [ ] table colspec widths — (missing; `<colspec colwidth="…">`)
- [ ] table spanning cells — (missing; `morerows`, `namest`/`nameend`)
- [ ] xml:lang — (missing; language attribute on any element)
- [ ] revision / revhistory — (missing; `<revhistory>` / `<revision>` in `<info>`)
- [ ] author / orgname in info — (missing; `<author>`, `<orgname>` in `<info>`)
- [ ] pubdate / publisher — (missing; metadata in `<info>`)

## Composition (integration)

- [ ] nested list — (missing; `<itemizedlist>` inside `<listitem>`)
- [ ] table with inline formatting in cells — (missing)
- [ ] section with admonition and code block — (missing)
- [ ] blockquote with attribution — (missing; `<attribution>` inside `<blockquote>`)
- [ ] figure with alt text — (missing; `<textobject>` as alt in `<mediaobject>`)
- [ ] footnote in table cell — (missing)
- [ ] callout listing + callout list — (missing)
- [ ] article-level metadata (info block) — (missing; `<info>` with `<title>`, `<author>`, etc.)

## Adversarial

- [x] empty document — `adv-empty`
- [ ] malformed XML (unclosed tag) — (missing)
- [ ] unknown DocBook element — (missing; element not in spec that reader must skip gracefully)
- [ ] missing required namespace declaration — (missing)
- [ ] entity references (&amp;, &lt;, &gt;, &apos;, &quot;) — (missing)
- [ ] numeric character references (&#160;, &#x2019;) — (missing)
- [ ] deeply nested sections (6+ levels) — (missing)
- [ ] empty para — (missing; `<para/>`)
- [ ] para with only whitespace — (missing)

## Pathological

- [ ] very large table (many rows/columns) — (missing)
- [ ] deeply nested lists (4+ levels) — (missing)
- [ ] section nesting at maximum DocBook depth — (missing)
- [ ] long document with many sections — (missing)
