# CLAUDE.md

Behavioral rules for Claude Code in the rescribe repository.

## Core value proposition: losslessness

**rescribe's primary differentiation from Pandoc is losslessness.** Pandoc silently drops
constructs it can't represent; rescribe never silently drops anything. Every construct must
either be modeled in the IR or preserved verbatim so a round-trip produces the original.

There is no "layout-only = fine to drop" exception. Tab stops, margins, border widths — a
user converting RTF→IR→RTF expects to get the same RTF back. Dropping tab stops is still
loss, even if it's "just layout."

### Two levels of preservation

**Semantic modeling** — construct has a cross-format meaning; represent it in the IR:
- Paragraph alignment → `style:align` property
- Font size → `style:size` on a `span` node
- Bold/italic/color → existing inline node kinds
- Footnotes → `footnote_ref` + `footnote_def` node kinds (once implemented)

Semantic constructs that can't yet be modeled must emit a `ConversionResult` fidelity
warning so the caller knows exactly what was lost.

**Raw preservation** — construct is format-specific with no cross-format equivalent;
capture it verbatim in a format-namespaced property so the writer can re-emit it:
- RTF paragraph layout words (tab stops, indents, borders) → `rtf:para-props` string
  property on the paragraph node, containing the raw RTF control words verbatim
- RTF character layout words (baseline twips, char spacing) → `rtf:char-props` on a span
- Other formats follow the same pattern: `html:attr`, `docx:rpr`, etc.

The IR supports this via:
- Format-specific property namespaces (`rtf:`, `html:`, `docx:`, etc.)
- `raw_inline` and `raw_block` node kinds for structured raw content
- Open `Properties` bag on every node (no construct is unrepresentable)

### The test

A format reader is correct when `parse(emit(parse(input))) == parse(input)` for all inputs,
where `emit` is the format writer and `parse` is the format reader. Every dropped construct
breaks this. "Unknown control word: \tx720" in a diagnostic is acceptable only if `\tx720`
is also in the IR somewhere that the writer will re-emit it.

## Project Overview

rescribe is a universal document conversion library, inspired by Pandoc but with:
- Open node kinds (not fixed enum)
- Property bags for extensibility
- Fidelity tracking (know what was lost)
- Embedded resource handling
- Roundtrip-friendly design

Part of the [rhi ecosystem](https://rhi.zone).

## Priority hierarchy: broadest reach first

Work should be prioritized by how many people benefit from a given deliverable:

1. **All language ecosystems** — the `fixtures/` suite. A comprehensive, language-agnostic
   test suite for a format benefits every implementation in every language. This is the
   highest-leverage output: a Python, Go, or JS author implementing RST can use our
   fixtures to verify correctness. Prioritize formats where no authoritative cross-language
   fixture suite currently exists (RST, Org, AsciiDoc) over formats already well-served
   (CommonMark has the spec suite; HTML has W3C).

2. **Rust ecosystem (any consumer)** — the standalone format crates. A well-designed
   `rst-fmt` crate with a clean public API benefits any Rust project that needs to parse
   RST, entirely outside rescribe. Every format gets a proper standalone crate here.

3. **Rust ecosystem (single consumer)** — completing the reader/writer API modes matrix
   (AST / streaming / batch reader; streaming / builder writer). This benefits a specific
   crate consumer who needs, say, a streaming reader. Lower priority than #1 and #2.

4. **rescribe** — the IR adapter layer. The rescribe integration is the narrowest
   beneficiary. It should be thin (≤300 lines per side) and not drive format crate design.

**When choosing what to work on next, ask which level it serves.** A fixture that closes
a coverage gap serves level 1. A new streaming API serves level 3. Don't invest in level
3 or 4 while level 1 has gaps.

## The real goal: fix the Rust document ecosystem

**rescribe itself may or may not take off. The standalone format libraries are the more
durable deliverable.** The Rust ecosystem is missing solid, well-designed crates for most
document formats. We fix that by building them here as a byproduct — proper standalone
libraries useful entirely outside rescribe.

**Every format without a quality ecosystem crate gets one here.** The target state is that
the API coverage matrix in `docs/format-audit.md` is all checkmarks: every standalone
format crate ships AST, streaming (iterator), batch (chunk-driven), streaming writer, and
builder writer — as separate Cargo features, all on by default. Third-party library-backed formats (pulldown-cmark, html5ever) are out of scope here — contribute
upstream if gaps exist. The ooxml-* crates (ooxml-wml, ooxml-sml, ooxml-pml) are ours and held
to the same standard; they're largely codegen'd so raising them to full API coverage is tractable.

This is not a nice-to-have. It is the primary reason to build format crates as proper
standalone libraries rather than internal rescribe adapters.

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

## The fixture suite is the primary deliverable

**The `fixtures/` directory is the real product.** The Rust crates are the reference
implementation that proves the fixtures are correct — but the fixture suite is what
matters for the ecosystem. Any implementation in any language should be able to take
`fixtures/{format}/{feature}/input.{ext}` + `expected.json` and use them as a complete
correctness test. Passing 100% of fixtures for a format means you have correctly
implemented that format — no gaps, no silent drops, no ambiguity.

This shapes what "done" means for a format:

- **Coverage**: every construct the format can express has at least one fixture —
  every block kind, inline kind, significant property, and composition case
- **Edge cases and adversarial inputs**: empty files, deeply nested structures,
  degenerate tables, malformed/truncated input, unusual but valid encodings —
  anything a real implementation might get wrong
- **Unambiguous**: each fixture has exactly one correct output; if a correct
  alternative implementation would be uncertain what to produce, the fixture is
  underspecified and must be strengthened
- **Language-agnostic**: `fixtures/{format}/{feature}/input.{ext}` + `expected.json`;
  no Rust-specific assumptions
- **See `fixtures/spec.md`** for the fixture format spec

When adding fixtures, always ask: "would a correct alternative implementation of this
format, reading this fixture, know exactly what to produce?" If not, the fixture is
underspecified.

**When you add support for a new parsed construct, add a fixture for it in the same commit.**
No new feature without a fixture. Use `transition_analysis` to verify the fixture closes
the gap. Fixtures that test new features should be added before calling a vertical "done."

## Conventions

- Crate names: `rescribe-{name}` (no rhi prefix per ecosystem convention)
- Reader/writer crates: `rescribe-read-{format}`, `rescribe-write-{format}`
- Node kinds: lowercase with underscores (`code_block`)
- Format-specific kinds: `{format}:{name}` (`html:div`)
- Properties: lowercase, colons for namespacing

## What "5-Production" actually means

**5-Production requires 100% construct coverage — not "enough for common cases."**

A format vertical is not production-grade until every construct the format can express is
either modeled in the IR or raw-preserved. "Most documents work" is not the bar. The bar
is: a correct alternative implementation reading the fixture suite would know exactly what
to produce for every construct, including:

- All block types (tables, lists, footnotes, code blocks, blockquotes, …)
- All inline types (all formatting, hyperlinks, images, footnote refs, …)
- All significant properties (font, color, alignment, language, size, …)
- Structure (nested lists, table cells with formatting, footnote content, …)

If a construct appears in real documents and the parser silently drops it — even if "most
documents don't use it" — the vertical is not 5-Production. It should be marked 4-Fuzz
(infrastructure solid, coverage incomplete) until every gap is closed.

**The ignored match arm is a debt list, not an exemption list.** Every control word in
the ignored arm that carries semantic content is a gap. Document them in TODO.md.

## Vertical completion checklist

Each standalone format crate (`crates/formats/{name}/`) must satisfy all of:

**Reader:**
- **AST** (`feature = "ast"`, default on): `parse(input: &[u8]) -> (Ast, Vec<Diagnostic>)` — full tree, infallible, Span on every node
- **Streaming** (`feature = "streaming"`, default on): `events(input: &[u8]) -> impl Iterator<Item = Event>` — no full AST; full input in memory; can borrow slices, supports lookahead
- **Batch** (`feature = "batch"`, default on): chunk-driven `Parser` (feed/finish) — O(working state) memory; handles files too large to load; cannot borrow across chunk boundaries

**Writer:**
- **Streaming** (`feature = "writer-streaming"`, default on): closure/visitor API — emits bytes as closures execute, no full tree required, well-formedness enforced by types
- **Builder** (`feature = "writer-builder"`, default on): `emit(ast: &Ast) -> Vec<u8>` — trivial wrapper over the streaming writer that walks the AST; ~20 lines

**Feature gating philosophy:** all features are on by default. Feature gating is not about
binary size or compile time — it's about **contract scoping**. A consumer who enables only
`features = ["ast"]` is explicitly signing up for that API and cannot accidentally couple
to the streaming or batch APIs. `default-features = false` is a statement of intent.

**Fuzz:**
- No-panic gate: arbitrary bytes must not panic — run until clean
- Round-trip fuzz: `parse(emit(arbitrary_ast)).strip_spans() == arbitrary_ast` — run until clean

**Rescribe integration:**
- Thin adapter ≤300 lines each side
- Fixture suite at 3-Harness
- Rescribe-level round-trip fuzz: arbitrary rescribe `Document` → emit → parse → assert equal
- **100% construct coverage** — no silently-dropped semantic constructs; see above

See `docs/format-library-design.md` for the full spec.
**A vertical is not done until both fuzz targets pass clean AND coverage is 100%.**

### Roundtrip direction matters

There are two wrong directions and one right one.

**Wrong (direction 1):** `parse(emit(parse(bytes))) == parse(bytes)`
Tests parse→emit→parse consistency. If the parser is lossy, the first parse already drops
content; the assertion only checks that dropped content stays dropped. "Zero roundtrip
failures" here is not evidence of losslessness.

**Also valid:** `parse(emit(arbitrary_rescribe_doc)) == arbitrary_rescribe_doc`
rescribe's IR is open by design — `NodeKind` is a free string, `Properties` is an open
bag, format-specific constructs live in `rtf:`/`html:`/etc. namespaces, and `raw_inline`/
`raw_block` exist for anything else. An RTF field code can be `rtf:field`; a color run
can carry `style:color`. So arbitrary IR generation CAN cover format-specific constructs
— once the IR modeling is complete. If this roundtrip shows zero failures, that's not
evidence of losslessness; it's evidence that constructs are being dropped instead of
modeled. Anything dropped silently can't be generated, can't fail.

**Correct:** `parse(emit(arbitrary_format_ast)) == arbitrary_format_ast`
Start from an arbitrary instance of the *format crate's own `Ast` type*. The native AST
is the ground truth for what the format can express. Emit it to wire bytes. Parse those
bytes back. Assert equality. This is the definitive test for the standalone format crate:
it covers the full surface area of the format regardless of IR modeling completeness.

### Fidelity warnings are not optional

Silent drops are failures. Every format construct the reader encounters but cannot represent
in the IR **must** emit a fidelity warning via `ConversionResult`. A reader that drops
font colors, footnotes, or paragraph alignment without warning is incorrect, not "lossy by
design." The goal is losslessness; where true losslessness is impossible, the loss must be
tracked.

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

## Core Rule

**Conversation is not memory.** Anything said in chat evaporates at session end. If it implies future behavior change, write it to CLAUDE.md or a memory file immediately — or it will not happen.

**Warning — these phrases mean something needs to be written down right now:**
- "I won't do X again" / "I'll remember to..." / "I've learned that..."
- "Next time I'll..." / "From now on I'll..."
- Any acknowledgement of a recurring error without a corresponding CLAUDE.md or memory edit

**When the user corrects you:** Ask what rule would have prevented this, and write it before proceeding. **"The rule exists, I just didn't follow it" is never the diagnosis** — a rule that doesn't prevent the failure it describes is incomplete; fix the rule, not your behavior.

**Something unexpected is a signal, not noise.** Surprising output, anomalous numbers, files containing what they shouldn't — stop and ask why before continuing. Don't accept anomalies and move on.

## When something goes wrong: update CLAUDE.md first

When you discover a design mistake, a wrong mental model, a subtle gotcha, or a case where
the obvious approach is wrong — **stop and update CLAUDE.md before continuing**. Don't
leave the lesson in conversation history; it will be lost. CLAUDE.md is the only memory
that survives across sessions.

This applies to:
- A test that passes for the wrong reason (like a lossless-looking roundtrip on a lossy parser)
- A framing that sounded right but isn't (like "start from IR to test losslessness")
- A constraint that wasn't written down and caused a mistake (like silent drops being treated as acceptable)
- Any time the user corrects a design assumption

The rule: **lesson learned → CLAUDE.md updated → then continue**. Never the other way.

### Why "I'll write it down" isn't enough

The instruction above already says to update CLAUDE.md. That alone hasn't been sufficient —
past sessions have explained mistakes conversationally and moved on without recording them,
leaving the same traps for the next session.

**The enforcement mechanism:** When the user pushes back on something you said or did,
treat it as a mandatory CLAUDE.md edit gate. You may not write another line of code or
explanation until the lesson is captured in CLAUDE.md. Not "I'll add it at the end of the
session." Not "I'll update memory." Right now, as the first response to the pushback.

Concrete triggers — stop and update CLAUDE.md immediately when:
- The user says "why did you do X" and X was wrong
- The user says "that's wrong" or "that's not right"
- The user asks a clarifying question that reveals your mental model was off
- You realize mid-explanation that the explanation implies a design flaw you introduced

The update must capture: what was wrong, why it was wrong, and what the correct approach is.
A one-line entry is fine. The goal is that the next session doesn't repeat the mistake.

### Recorded lessons (update this list, never delete entries)

- **XLSX cell-type inference (2026-03-03):** The XLSX writer used `.parse::<f64>()` on every
  text node to "auto-detect" numbers, converting `"007"` → `0.0` → `"7"`. This is wrong.
  The reader knows the actual cell type (`CellValue::Number` vs `CellValue::String`). The fix:
  reader tags `xlsx:cell-type = "n"/"s"/"b"/"e"` on each paragraph; writer reads that prop
  instead of guessing. Never infer type from string content when the IR already carries the type.

## Negative Constraints

Do not:
- Announce actions ("I will now...") - just do them
- Leave work uncommitted
- Use interactive git commands (`git add -p`, `git add -i`, `git rebase -i`) — these block on stdin and hang in non-interactive shells; stage files by name instead
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

**Always commit completed work.** After tests pass, commit immediately — don't wait to be asked. When a plan has multiple phases, commit after each phase passes. Do not accumulate changes across phases. Uncommitted work is lost work.

**Use `normalize view` for structural exploration:**
```bash
~/git/rhizone/normalize/target/debug/normalize view <file>    # outline with line numbers
~/git/rhizone/normalize/target/debug/normalize view <dir>     # directory structure
```

## Context Management

**Use subagents to protect the main context window.** For broad exploration or mechanical multi-file work, delegate to an Explore or general-purpose subagent rather than running searches inline. The subagent returns a distilled summary; raw tool output stays out of the main context.

Rules of thumb:
- Research tasks (investigating a question, surveying patterns) → subagent; don't pollute main context with exploratory noise
- Searching >5 files or running >3 rounds of grep/read → use a subagent
- Codebase-wide analysis (architecture, patterns, cross-file survey) → always subagent
- Mechanical work across many files (applying the same change everywhere) → parallel subagents
- Single targeted lookup (one file, one symbol) → inline is fine

## Session Handoff

Use plan mode as a handoff mechanism when:
- A task is fully complete (committed, pushed, docs updated)
- The session has drifted from its original purpose
- Context has accumulated enough that a fresh start would help

**For handoffs:** enter plan mode, write a short plan pointing at TODO.md, and ExitPlanMode. **Do NOT investigate first** — the session is context-heavy and about to be discarded. The fresh session investigates after approval.

**For mid-session planning** on a different topic: investigating inside plan mode is fine — context isn't being thrown away.

Before the handoff plan, update TODO.md and memory files with anything worth preserving.

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
