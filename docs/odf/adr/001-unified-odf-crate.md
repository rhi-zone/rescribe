# ADR-001: Unified `odf-fmt` Crate for All ODF Document Types

## Status

Accepted

## Context

The OpenDocument Format (ODF) standard defines three primary document types:

- **ODT** — text documents (`.odt`) — analogous to DOCX
- **ODS** — spreadsheets (`.ods`) — analogous to XLSX
- **ODP** — presentations (`.odp`) — analogous to PPTX

All three are ZIP archives with a common packaging structure (mimetype,
META-INF/manifest.xml) and share a **single RELAX NG schema** published by
OASIS. This contrasts with OOXML, where WML, SML, and PML are defined by
separate schema documents within ECMA-376.

The question was whether to model `odf-fmt` after the OOXML split
(`ooxml-wml` / `ooxml-sml` / `ooxml-pml`) or to keep a single unified crate.

### Option A — Unified `odf-fmt`

```
odf-fmt
  feature = "odf-text"      → ODT text body types and parser
  feature = "odf-tables"    → ODS spreadsheet body types and parser
  feature = "odf-drawings"  → ODP presentation body types and parser
```

### Option B — Separate per-document-type crates

```
odt-fmt   → text document types, reader, writer
ods-fmt   → spreadsheet types, reader, writer
odp-fmt   → presentation types, reader, writer
```

## Decision

**Option A — unified `odf-fmt` crate.**

### Rationale

**1. Single schema, single generated artifact.** The ODF 1.3 RELAX NG
schema (`odf-1.3.rnc`) is one file covering all three document types.
Running `ooxml-codegen` against it produces a single `generated.rs`
(~19,500 lines). There is no per-application schema boundary at which to
split, unlike ECMA-376's §17/§18/§19 structure.

**2. Shared structural elements are pervasive.** The ODF schema makes
heavy use of shared definitions across document types:

- `style:style`, `style:paragraph-properties`, `style:text-properties` —
  used identically in ODT, ODS, and ODP
- `draw:frame`, `draw:image`, `draw:text-box` — shared between ODT and ODP
- `meta:*`, `office:meta` — identical across all three
- `table:table` — appears in both ODT (embedded tables) and ODS (sheets)

Splitting would require either duplicating these definitions or introducing
a shared `odf-common` crate, reproducing the complexity without the benefit.

**3. Generated code volume is modest per application.** The OOXML crates'
per-application `generated.rs` files are 6k–18k lines each; a split was
justified to avoid compiling 40k lines for a single-format consumer.
ODF's single `generated.rs` is ~19.5k lines total for all three types —
comparable to one OOXML crate. The marginal cost of including ODS types in
an ODT-only consumer is small.

**4. Feature flags provide sufficient granularity.** The `Cargo.toml`
already gates domain-specific types and APIs:

```toml
odf-text     = []   # text:* — paragraphs, headings, lists, notes, fields
odf-styles   = []   # style:* — automatic and named styles
odf-tables   = []   # table:* — tables, rows, cells
odf-drawings = []   # draw:* — frames, images, shapes
odf-meta     = []   # office:meta / dc:* — document metadata
```

A consumer that only processes text documents enables `odf-text` and
`odf-styles`; the ODS/ODP-specific parser code paths are dead-code
eliminated at compile time.

**5. Single ZIP/packaging implementation.** ODT, ODS, and ODP all use
the same ZIP structure and manifest format. A unified crate shares this
parsing and writing code naturally; splitting it would require a shared
`odf-opc` crate analogous to `ooxml-opc`.

**6. Consistent with ODF's own presentation.** OASIS publishes one
specification document and one schema file covering all three document
types. The crate boundary follows the standard's own modularity.

## Consequences

**Easier:**
- One dependency declaration covers all ODF document types.
- Shared types (styles, metadata, frames) are implemented once.
- Schema updates require regenerating one file.
- The `OdfDocument` / `OdfBody` abstraction cleanly encodes which
  document type was opened without requiring different parse calls.

**Harder:**
- Compile times for an ODT-only consumer include the ODS/ODP generated
  symbols, even if dead-code eliminated. Acceptable given the modest size.
- If ODS or ODP hand-written code grows very large (>10k lines each),
  revisiting the split may be warranted. Track in COVERAGE.md.

## References

- OASIS ODF 1.3 specification — single schema document
- `spec/odf/odf-1.2.rnc` — committed ODF 1.2 schema (permissive license)
- `spec/odf/README.md` — schema provenance and download instructions
- `crates/formats/odf-fmt/Cargo.toml` — feature flag definitions
- [ooxml ADR-005](../../ooxml/adr/005-per-application-crate-split.md) — the contrasting OOXML decision
