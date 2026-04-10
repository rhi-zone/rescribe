# ADR-005: Per-Application Crate Split (wml / sml / pml)

## Status

Accepted

## Context

ECMA-376 defines three application-specific markup languages within the
OOXML standard:

- **WML** (WordprocessingML) — `.docx` Word documents (Part 1 §17)
- **SML** (SpreadsheetML) — `.xlsx` Excel workbooks (Part 1 §18)
- **PML** (PresentationML) — `.pptx` PowerPoint presentations (Part 1 §19)

They share a common packaging layer (OPC / ZIP), XML utilities, DrawingML
(DML) for embedded graphics, and Office Math ML (OMML). The application
schemas themselves are entirely separate: `pml.rnc`, `sml.rnc`, and the
WML portions of `wml.rnc` do not reference each other.

The question was whether to expose these as a single `ooxml-fmt` crate
(gated by features) or as three separate crates.

### Option A — Single `ooxml-fmt` crate

```
ooxml-fmt
  feature = "wml"   → exposes Word types
  feature = "sml"   → exposes Excel types
  feature = "pml"   → exposes PowerPoint types
```

### Option B — Separate per-application crates (current)

```
ooxml-wml   → Word types, reader, writer
ooxml-sml   → Excel types, reader, writer
ooxml-pml   → PowerPoint types, reader, writer
ooxml-dml   → shared DrawingML types
ooxml-opc   → shared OPC/ZIP layer
ooxml-xml   → shared XML primitives
```

## Decision

**Option B — separate per-application crates.**

### Rationale

**1. Generated code volume.** Each application's `generated.rs` is large
and independent:

| Crate | `generated.rs` |
|-------|---------------|
| `ooxml-wml` | 15,216 lines |
| `ooxml-sml` | 18,606 lines |
| `ooxml-pml` |  6,447 lines |

Under Option A, a consumer that only reads Word documents would compile
all 40k lines of generated types regardless of feature selection, because
Rust compiles all items in a module even when a feature gates the public
API. Separate crates allow the linker/compiler to omit entire compilation
units.

**2. Domain-specific hand-written code has no shared surface.** The
extension traits, writer, and high-level API in each crate are tightly
coupled to their application's type hierarchy:

| Crate | `ext.rs` | `writer.rs` |
|-------|---------|------------|
| `ooxml-wml` | 4,909 lines | 3,056 lines |
| `ooxml-sml` | 1,970 lines | 6,606 lines |
| `ooxml-pml` |   513 lines | 5,319 lines |

`wml::Paragraph`, `sml::Cell`, and `pml::Shape` are completely different
type hierarchies with no common base type. A unified crate would contain
~20k lines of hand-written domain logic with no reuse between the three
sections.

**3. Separate feature matrices.** Each crate has its own domain-specific
feature flags (`wml-math`, `wml-comments`, `sml-pivot`, `pml-transitions`)
that do not overlap. Flattening them into a single namespace would be
confusing and would make the dependency surface harder to audit.

**4. Mirrors the upstream specification.** ECMA-376 Part 1 dedicates
separate numbered clauses (§17 WML, §18 SML, §19 PML) to each application.
The crate split follows the same boundary the standards body drew.

**5. Shared code is already factored out.** `ooxml-opc`, `ooxml-xml`, and
`ooxml-dml` are proper shared crates that WML, SML, and PML all depend on.
There is no remaining duplication that a merger would eliminate.

## Consequences

**Easier:**
- Consumers declare only the dependency they need (`ooxml-wml` for DOCX
  processing; no SML or PML symbols compiled).
- Each crate can evolve its API independently — SML can add pivot table
  support without touching WML.
- Separate crates have separate version histories, making changelog
  attribution straightforward.

**Harder:**
- A consumer that processes all three formats (e.g. the `rescribe` CLI)
  must list three dependencies.
- The `batch.rs` and `streaming.rs` stubs (~45–51 and ~193–304 lines
  respectively) are near-identical across the three crates. This is
  acceptable duplication given the benefits above; if it grows, it can
  be absorbed into `ooxml-xml` or a new `ooxml-core` crate.

## References

- ECMA-376 Part 1, 5th edition — §17 (WML), §18 (SML), §19 (PML)
- [ADR-003](./003-generated-types-architecture.md) — generated types as primary data model
- Workspace `Cargo.toml` — `ooxml-wml`, `ooxml-sml`, `ooxml-pml` entries
