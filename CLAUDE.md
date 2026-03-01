# CLAUDE.md

Behavioral rules for Claude Code in the rescribe repository.

## Project Overview

rescribe is a universal document conversion library, inspired by Pandoc but with:
- Open node kinds (not fixed enum)
- Property bags for extensibility
- Fidelity tracking (know what was lost)
- Embedded resource handling
- Roundtrip-friendly design

Part of the [rhi ecosystem](https://rhi.zone).

## Architecture

```
crates/
├── rescribe-core/           # Core IR only: Document, Node, Properties, Resource, traits
├── nodes/
│   ├── rescribe-std/        # Standard node kinds (paragraph, heading, list, etc.)
│   └── rescribe-math/       # Math node kinds (math_inline, fraction, matrix, etc.)
├── readers/
│   ├── rescribe-read-markdown/
│   ├── rescribe-read-html/
│   └── ...
├── writers/
│   ├── rescribe-write-markdown/
│   ├── rescribe-write-html/
│   └── ...
├── rescribe-transforms/     # Standard transformers
└── rescribe-cli/            # CLI tool
```

## Key Types (in rescribe-core)

- `Document` - Root container with content, resources, metadata
- `Node` - Tree node with kind, properties, children
- `NodeKind` - Open string type for node classification (no constants in core)
- `Properties` - Key-value bag for node attributes
- `PropValue` - Property value enum (String, Int, Float, Bool, List, Map)
- `Resource` - Embedded binary (images, fonts, etc.)
- `ConversionResult<T>` - Result with fidelity warnings

## Traits (in rescribe-core)

- `Parser` - Parse bytes → Document
- `Emitter` - Document → bytes
- `Transformer` - Document → Document

## Standard Node Kinds (in rescribe-std)

Block: `document`, `paragraph`, `heading`, `code_block`, `blockquote`, `list`, `list_item`, `table`, `table_row`, `table_cell`, `table_header`, `figure`, `horizontal_rule`, `div`, `raw_block`, `definition_list`, `definition_term`, `definition_desc`

Inline: `text`, `emphasis`, `strong`, `strikeout`, `underline`, `subscript`, `superscript`, `code`, `link`, `image`, `line_break`, `soft_break`, `span`, `raw_inline`, `footnote_ref`, `footnote_def`

## Property Namespaces

- Semantic: `level`, `url`, `language`, `content`, `ordered`, `title`, `alt`, etc.
- Style: `style:font`, `style:color`, etc.
- Layout: `layout:page_break`, `layout:float`, etc.
- Format-specific: `html:class`, `latex:env`, `docx:style`, etc.

## Development

```bash
nix develop        # Enter dev shell
cargo test         # Run tests
cargo clippy       # Lint
cd docs && bun dev # Local docs
```

## Testing

Pandoc fixtures at `~/git/pandoc/test/` can be used as local reference inputs (GPL - don't copy into repo). Run rescribe against them to validate parsing.

## Long-term Goal: 100% Spec Coverage

The owned fixture suite (`fixtures/`) is the primary deliverable for correctness:

- **Language-agnostic**: any implementation in any language uses `fixtures/{format}/{feature}/input.{ext}` + `expected.json`
- **100% pass rate = 100% implementation support** — rich enough that passing all fixtures means you have correctly implemented the format's full construct set
- **Coverage definition**: one fixture per (format × construct) pair, including all block kinds, all inline kinds, all significant properties, and key composition cases
- **See `fixtures/spec.md`** for the fixture format spec

When adding fixtures, always think: "would a correct alternative implementation of this format, reading this fixture, know exactly what to produce?" If not, the fixture is underspecified.

## Conventions

- Crate names: `rescribe-{name}` (no rhi prefix per ecosystem convention)
- Reader/writer crates: `rescribe-read-{format}`, `rescribe-write-{format}`
- Node kinds: lowercase with underscores (`code_block`)
- Format-specific kinds: `{format}:{name}` (`html:div`)
- Properties: lowercase, colons for namespacing

## Vertical completion checklist

Each standalone format crate (`crates/formats/{name}/`) must satisfy all of:
- `Ast` type with `Span` on every node
- `parse(input) -> (Ast, Vec<Diagnostic>)` — infallible
- `events(input) -> impl Iterator<Item = Event>` — pull tokenizer
- `emit(ast) -> String` — round-trip guarantee
- No-panic fuzz gate: arbitrary bytes must not panic (run 1h+)
- Round-trip fuzz: `parse(emit(parse(s).0)).0.strip_spans() == parse(s).0.strip_spans()` (run 1h+)
- Thin rescribe adapter ≤300 lines each side
- Rescribe fixture suite at 3-Harness

See `docs/format-library-design.md` for the full spec.
**A vertical is not done until both fuzz targets have run clean for 1h+.**

## Marathon Mode

When working autonomously:
1. Work through todo list systematically
2. For each format vertical: fixtures → harness → fuzz → commit
3. Commit working increments (don't batch too much)
4. Progress in vertical priority order (see TODO.md)
5. Keep this file updated with architecture changes
6. Don't stop early - continue until blocked or todo list exhausted

## Design Principles

**Unify, don't multiply.** One interface for multiple cases > separate interfaces. Plugin systems > hardcoded switches.

**Simplicity over cleverness.** HashMap > inventory crate. OnceLock > lazy_static. Functions > traits until you need the trait. Use ecosystem tooling over hand-rolling.

**Explicit over implicit.** Log when skipping. Show what's at stake before refusing.

**Separate niche from shared.** Don't bloat shared config with feature-specific data. Use separate files for specialized data.

## Negative Constraints

Do not:
- Announce actions ("I will now...") - just do them
- Leave work uncommitted
- Use path dependencies in Cargo.toml - causes clippy to stash changes across repos
- Use `--no-verify` - fix the issue or fix the hook
- Assume tools are missing - check if `nix develop` is available for the right environment

## Workflow

**Batch cargo commands** to minimize round-trips:
```bash
cargo clippy --all-targets --all-features -- -D warnings && cargo test
```
After editing multiple files, run the full check once — not after each edit. Formatting is handled automatically by the pre-commit hook (`cargo fmt`).

**When making the same change across multiple crates**, edit all files first, then build once.

**Minimize file churn.** When editing a file, read it once, plan all changes, and apply them in one pass. Avoid read-edit-build-fail-read-fix cycles by thinking through the complete change before starting.

**Use `normalize view` for structural exploration:**
```bash
~/git/rhizone/normalize/target/debug/normalize view <file>    # outline with line numbers
~/git/rhizone/normalize/target/debug/normalize view <dir>     # directory structure
```

## Session Handoff

Use plan mode as a handoff mechanism when:
- A task is fully complete (committed, pushed, docs updated)
- The session has drifted from its original purpose
- Context has accumulated enough that a fresh start would help

Before entering plan mode:
- Update TODO.md with any remaining work
- Update memory files with anything worth preserving across sessions

Then enter plan mode and write a plan file that either:
- Proposes the next task if it's clear: "next up: X — see TODO.md"
- Flags that direction is needed: "task complete / session drifted — see TODO.md"

ExitPlanMode hands control back to the user to approve, redirect, or stop.

## Commit Convention

Use conventional commits: `type(scope): message`

Types:
- `feat` - New feature
- `fix` - Bug fix
- `refactor` - Code change that neither fixes a bug nor adds a feature
- `docs` - Documentation only
- `chore` - Maintenance (deps, CI, etc.)
- `test` - Adding or updating tests

Scope is optional but recommended for multi-crate repos.
