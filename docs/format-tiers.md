# Format Tiers

Not all formats are equal roundtrip partners. rescribe classifies formats into
three tiers based on what the reader/writer pair can guarantee.

---

## Tier 1 — Full roundtrip

The source file *is* the document. Every feature the format can express maps to
rescribe IR and back. Reader + writer must be lossless for everything in the
spec; fidelity warnings should fire only for genuinely unrepresentable
constructs.

The roundtrip fixture guarantee applies strictly here:
`input → parse → emit → parse → IR == IR`

**Formats:** Markdown / CommonMark / GFM, HTML, Org, RST, AsciiDoc, Djot,
MediaWiki, DokuWiki, VimWiki, TWiki, TikiWiki, Creole, XWiki, ZimWiki,
Textile, Jira, Haddock, BBCode, Muse, t2t, Markua, Fountain, POD, man,
DOCX, ODT, EPUB, FB2, JATS, DocBook, TEI, OpenDocument formats.

---

## Tier 2 — Write-primary, partial read

The format is a document *programming language*. The writer is the real
product — any rescribe IR can be expressed in these formats. The reader
extracts static authored content only and cannot recover programmatically
generated content without executing the language runtime.

Fidelity warnings **must** fire for every construct the reader skips
(`#let`, `#set`, `\newcommand`, `\foreach`, etc.). Fixtures test the
static-content roundtrip only — the subset of the format that is pure markup.

**Formats:** Typst, LaTeX / Beamer, ConTeXt

---

## Tier 3 — Read-only / extract-only

Content can be extracted but the format is not a viable general write target,
or the write path is so structurally lossy that a roundtrip guarantee is
meaningless.

**Formats:** PDF (text extraction only), XLSX / PPTX (structured data
extraction; write is supported but not a document roundtrip partner),
CSV / TSV (tabular data only), BibTeX / BibLaTeX / CSL-JSON / RIS /
EndNote XML (bibliographic data, not prose documents), ANSI (terminal
output, write only in practice), Pandoc JSON (interchange format — not
a user-facing roundtrip target).

---

## Implications for new format work

When adding a reader/writer pair, declare its tier in the crate's `lib.rs`
doc comment. The tier determines what the fixture suite must prove:

| Tier | Required fixtures |
|------|------------------|
| 1    | Feature coverage + roundtrip (both directions) |
| 2    | Feature coverage for static subset + fidelity warning assertions |
| 3    | Feature coverage for extraction; no roundtrip required |
