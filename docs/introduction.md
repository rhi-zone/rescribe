# Introduction

**rescribe** is a universal document conversion library and CLI for Rust,
inspired by Pandoc but designed around a fundamentally different IR.

## Use cases

**Document conversion** — convert between formats with explicit fidelity
reporting. `rescribe convert input.docx output.md` tells you exactly what was
lost; Pandoc doesn't.

**Document processing** — use rescribe as a library to query, filter, and
transform documents programmatically. Extract all code blocks tagged `python`
from an EPUB. Merge documents and rewrite cross-references. Shift all heading
levels. Replace image URLs matching a pattern. This is the space Pandoc's
filter system gestures at but never fully inhabits — rescribe's IR is a native
Rust tree you manipulate directly, not JSON piped through an external process.

Both use cases are first-class. The CLI and the library are the same thing.

## The problem with Pandoc's IR

Pandoc is the de facto standard for document conversion and is excellent at
what it does. But its architecture has a hard ceiling:

**The AST is a closed enum.** Every format gets flattened into a fixed set of
`Block` and `Inline` variants. Anything that doesn't fit — DOCX paragraph
styles, HTML `data-*` attributes, LaTeX environments, Typst functions — is
either approximated or silently dropped. You can't know what was lost, and you
can't extend the AST without forking Pandoc.

**Conversion is silent.** Pandoc does its best and gives you output. When it
drops a table feature or loses a cross-reference, you find out by inspecting
the result. There is no programmatic signal.

**It's not a library.** Pandoc is a Haskell binary. You can invoke it as a
subprocess or use its Haskell API, but you cannot embed it in a Rust
application, compile it to WASM, or use it via FFI.

## rescribe's approach

rescribe represents documents using an **open IR**:

- **Open node kinds** — node kinds are plain strings, not an enum. Format-specific
  constructs (`html:div`, `docx:style`, `latex:env`) survive the round-trip
  instead of being dropped. New node kinds don't require library changes.
- **Property bags** — nodes carry arbitrary key-value properties. Metadata
  that doesn't fit a fixed schema isn't thrown away.
- **Fidelity tracking** — every parse and emit returns a `ConversionResult`
  carrying structured warnings about what couldn't be represented. You know
  exactly what was lost.
- **Embedded resources** — images, fonts, and binary data are first-class
  `Resource` objects, not external file references.
- **Rust library** — embeddable in any Rust application, compilable to WASM,
  usable via FFI. No subprocess, no Haskell runtime.

## vs Pandoc

| | Pandoc | rescribe |
|---|---|---|
| AST | Closed Haskell enum | Open string-keyed nodes |
| Format-specific data | Dropped or approximated | Preserved in property bags |
| Conversion warnings | None | Structured fidelity tracking |
| Extensibility | Fork required | New node kinds, no fork |
| Embedding | Subprocess only | Native Rust library |
| Maturity | 15+ years, battle-tested | Early development |
| CLI | Excellent | Present, improving |

The honest take: **Pandoc is more mature today.** For simple one-off
conversions from the command line, it will serve you well. rescribe's
advantages compound when you're *building an application* that processes
documents — where you need to embed the library, inspect fidelity, handle
custom node types, or process the IR programmatically. As rescribe matures,
the CLI should be better too: faster (Rust vs Haskell startup), more faithful
(open IR loses less), and with explicit fidelity reporting.

## Format coverage

See [format-tiers.md](./format-tiers.md) for how formats are classified. In
short:

- **Tier 1** (full roundtrip guarantee): Markdown, HTML, Org, RST, DOCX, EPUB,
  and most markup formats
- **Tier 2** (write-primary, partial read): Typst, LaTeX — document programming
  languages where the reader extracts static content only
- **Tier 3** (extract-only): PDF, XLSX, bibliographic formats

## Quick start

```rust
use rescribe_read_markdown::parse;
use rescribe_write_html::emit;

let result = parse("# Hello\n\nWorld")?;

for warning in &result.warnings {
    eprintln!("fidelity: {}", warning.message);
}

let html = emit(&result.value)?;
```
