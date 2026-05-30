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
- Footnotes → `footnote_ref` + `footnote_def` node kinds

Semantic constructs that can't yet be modeled must emit a `ConversionResult` fidelity
warning so the caller knows exactly what was lost.

**Raw preservation** — construct is format-specific with no cross-format equivalent;
capture it verbatim in a format-namespaced property so the writer can re-emit it:
- RTF paragraph layout words → `rtf:para-props` string property on the paragraph node
- RTF character layout words → `rtf:char-props` on a span
- Other formats follow the same pattern: `html:attr`, `docx:rpr`, etc.

The IR supports this via:
- Format-specific property namespaces (`rtf:`, `html:`, `docx:`, etc.)
- `raw_inline` and `raw_block` node kinds for structured raw content
- Open `Properties` bag on every node (no construct is unrepresentable)

### The test

A format reader is correct when `parse(emit(parse(input))) == parse(input)` for all inputs,
where `emit` is the format writer and `parse` is the format reader. Every dropped construct
breaks this.

### Fidelity warnings are not optional

Silent drops are failures. Every format construct the reader encounters but cannot represent
in the IR **must** emit a fidelity warning via `ConversionResult`. A reader that drops
font colors, footnotes, or paragraph alignment without warning is incorrect, not "lossy by
design." The goal is losslessness; where true losslessness is impossible, the loss must be
tracked.

**The ignored match arm is a debt list, not an exemption list.** Every arm that silently
drops semantic content is a gap. Document gaps in TODO.md.

## Project Overview

rescribe is a universal document conversion library, inspired by Pandoc but with:
- Open node kinds (not fixed enum)
- Property bags for extensibility
- Fidelity tracking (know what was lost)
- Embedded resource handling
- Roundtrip-friendly design

Part of the [rhi ecosystem](https://rhi.zone).

## The -fmt crates are not rescribe internals

**This is the most important thing to understand about this codebase.**

`rst-fmt`, `asciidoc`, `djot-fmt`, `org-fmt`, `rtf-fmt`, and every other standalone
format crate are **first-class Rust ecosystem libraries**. They are not helpers for
rescribe's document conversion pipeline. rescribe's IR adapter is a thin consumer of
them — one of many possible consumers.

Someone will use `rst-fmt` to build a documentation site generator. Someone else will
use it for a language server, a search indexer, a linter, a syntax highlighter, a
corpus analysis tool. These use cases exist whether rescribe does or not.

**Consequences:**

- Never evaluate a design decision through the lens of "what does rescribe need?" or
  "is this good enough for document conversion?" That is always the wrong question.
  Ask: "what does a general-purpose user of this format library need?"

- A fake streaming API (one that builds the full AST internally then wraps it) is a
  broken API. It fails silently for any caller that needs true incremental processing,
  low-memory operation, or event-driven pipelines. **"Good enough for conversion" is
  not a valid reason to ship a hollow streaming interface.**

- **The three APIs are independent implementations, not derived from one another.**
  `parse()`, `events()`, and `StreamingParser<H>` each have their own optimal
  implementation. They share state-transition logic as functions — not a common
  primitive type. See `docs/format-library-design.md` for the full architecture.

- **`parse()` = direct recursive descent into the AST.** No events, no intermediate
  representation. Fastest path to a materialized tree.

- **`events()` = the parser IS the iterator.** `EventIter` holds the parser state;
  `next()` advances it and returns one event. `Cow::Borrowed` slices from the input
  `&[u8]` where the format allows zero-copy (no escape sequences in the span).
  Input must be fully in memory. Standard `Iterator` trait.

- **`StreamingParser<H: Handler>` = callback model for chunked input.** `feed(chunk)`
  advances the state machine, calling `handler.handle(event)` for each event produced.
  The `handle()` call is stack-scoped: `Event<'_>` borrows from the internal source
  buffer window, drops when `handle()` returns, allowing the buffer to compact.
  Memory: O(largest token + nesting depth). This is the right API for files too large
  to load into memory and for network/pipe inputs.

- **`parse()` is NOT `events().collect()`.** That formulation sacrifices performance
  for code elegance: it forces materialization through the event dispatch layer and
  prevents direct struct construction. The behavior must be equivalent; the
  implementation should not be.

- **`commonmark-fmt` wraps pulldown-cmark.** `events()` and `parse()` are at max perf.
  `StreamingParser` buffers all input before parsing (pulldown requires full `&str`).
  This limitation is documented explicitly in the crate. Superseding pulldown-cmark
  (77M+ weekly downloads) is a non-goal. Callers needing true chunked CommonMark
  streaming should use pulldown-cmark directly or wait for a future native parser.

- **Library-backed formats still require a proper `-fmt` crate.** Wrapping an
  upstream library does not reduce the requirements — the `-fmt` crate must still
  expose all three reader APIs and both writer APIs, each independently optimal.
  When the upstream library supports multiple modes (tree building, event/SAX,
  chunked input), the `-fmt` crate must use the right mode for each API. A wrapper
  that funnels everything through the tree builder is a fake streaming API.
  **If a library cannot support all five APIs at full performance, we cannot use
  that library.** The library must enable performance, not constrain it.
  pulldown-cmark is the sole exception (superseding 77M weekly downloads is a
  non-goal); its `StreamingParser` buffering limitation is documented explicitly.

- **`ooxml-fmt` is the priority target for the full three-API architecture.** DOCX,
  XLSX, and PPTX routinely exceed RAM on large corpora; `StreamingParser` is not
  optional there. The ooxml-fmt rework (after commonmark-fmt and the five hand-rolled
  crate upgrades) is the most important streaming work in the queue.

- Format crate design decisions (AST shape, event types, error model, span semantics)
  must be made for the widest plausible user, not the narrowest known consumer.

### The adapter layer must never contain parsing or writing logic

**`rescribe-read-{format}` and `rescribe-write-{format}` are not the format library.**
They are translators between the format's native `Ast` and rescribe's `Document`.
All parsing and writing logic belongs in `{format}-fmt`.

**Rule:** Before writing a single line of `rescribe-read-{format}` or
`rescribe-write-{format}`, the `{format}-fmt` standalone crate must already exist (or
be created first in the same vertical). The adapter crate's only job is:
```
rescribe-read-{fmt}: {fmt}-fmt::parse(bytes) → {fmt}_fmt::Ast → rescribe Document
rescribe-write-{fmt}: rescribe Document → {fmt}_fmt::Ast → {fmt}-fmt::emit(ast) → bytes
```

**The violation is format parsing in production adapter code, not line count.**
An adapter that does only AST→IR translation can legitimately be 500+ lines for a
complex format (DOCX, PPTX). A 50-line adapter that calls `quick_xml::Reader` is
broken regardless of its size. The correct test:

> Does the adapter's **production code** contain any tokenizing, parsing, or emitting
> of format bytes? If yes, that code belongs in the `-fmt` crate.

**What counts as parsing/writing logic in production code:**
- Any `quick_xml::Reader`, `quick_xml::Writer`, `regex::Regex`, `zip::ZipArchive`, etc.
  called from functions that are not `#[cfg(test)]` and not in `[[bin]]` tools
- A `parse_something_xml()` helper in the adapter that reads raw bytes
- Format-specific state machines, tokenizers, or emitters

**What does NOT count as a violation:**
- Large adapters doing complex AST→IR translation (DOCX, PPTX are genuinely complex)
- `[[bin]]` binaries (e.g., `gen_fixtures`) using `zip` or `quick-xml` to construct test fixtures
- `#[cfg(test)]` blocks using any dep to build test inputs

**Catch it by reading the production functions:** open `src/lib.rs`, find non-test
functions, check their imports. If you see `quick_xml::Reader`, that's a violation.
`Cargo.toml` is a weaker signal because `[[bin]]` tools and tests legitimately need
format-parser crates.

**This mistake is insidious because it "works" locally** — the rescribe tests pass. But
it means the Rust ecosystem gets no reusable FB2/ODT/etc. library. Every Rust project
that needs FB2 will have to roll their own parser, or reach into rescribe's internals.
That is the failure this rule prevents.

## Priority hierarchy: broadest reach first

Work should be prioritized by how many people benefit:

1. **All language ecosystems** — the `fixtures/` suite. Prioritize formats where no
   authoritative cross-language fixture suite currently exists (RST, Org, AsciiDoc)
   over formats already well-served (CommonMark has the spec suite; HTML has W3C).

2. **Rust ecosystem (any consumer)** — the standalone format crates. A well-designed
   `rst-fmt` crate benefits any Rust project that needs RST, entirely outside rescribe.

3. **Rust ecosystem (single consumer)** — completing the reader/writer API modes matrix.

4. **rescribe** — the IR adapter layer. Only AST↔IR translation; doesn't drive
   format crate design.

**When choosing what to work on next, ask which level it serves.** Don't invest in level
3 or 4 while level 1 has gaps.

**Work one vertical to completion before starting the next.** Never do a horizontal sweep
across formats (fixtures for RST + Org + AsciiDoc without finishing any one). "Fixture
suite complete" is step 1 of 5. Pick up the current vertical at its current step.

## The real goal: fix the Rust document ecosystem

**rescribe itself may or may not take off. The standalone format libraries are the more
durable deliverable.** Every format without a quality ecosystem crate gets one here. The
target state is the API coverage matrix in `docs/format-audit.md` all checkmarks.
Third-party library-backed formats (pulldown-cmark, html5ever) are out of scope — contribute
upstream. The ooxml-* crates are ours and held to the same standard.

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

## Streaming IR (planned architecture)

`Document` is a materialized tree — it requires the full document in memory simultaneously.
This is fine for typical documents but unacceptable for large corpora (legal discovery,
academic corpora, enterprise search) where individual files can exceed available RAM.

**No corners cut:** rescribe must support end-to-end streaming conversion with
O(nesting depth + largest token) memory, never loading the full document into memory
at once.

### Target design

```rust
// Format-agnostic IR event stream (SAX-style open/close pairs, mirroring rescribe-std kinds)
pub enum IrEvent<'a> {
    StartDocument, EndDocument,
    StartParagraph, EndParagraph,
    StartHeading { level: u8 }, EndHeading,
    Text(Cow<'a, str>),
    // ... all IR node kinds as open/close pairs
}

// Streaming reader: feeds format chunks, calls handler per IR event
pub trait StreamingReader {
    fn feed(&mut self, chunk: &[u8]);
    fn finish(self);
}

// Streaming writer: consumes IR events, produces output bytes incrementally
pub trait StreamingWriter {
    fn handle(&mut self, event: IrEvent<'_>);
    fn finish(self) -> Vec<u8>;
}

// Materialized path: StreamingWriter that assembles a Document
pub struct DocumentBuilderHandler { ... }
impl StreamingWriter for DocumentBuilderHandler { ... }
```

### Pipeline model

```
feed(chunk) → [format StreamingParser] → IrEvent → [IrTransformer] → IrEvent → [StreamingWriter] → output chunk
```

`Document` stays. Callers that want the materialized tree use `parse()` → format AST
→ thin adapter → `Document` (current path). The streaming path is additive.

### What this requires from format libraries

Each format library's `StreamingParser<H: Handler>` must be **true Tier 2**:
- `feed()` processes the chunk, calls `handler.handle(event)` for each complete token
- Memory: O(largest token + nesting depth), **not O(full input)**
- Split tokens at chunk boundaries buffered in parser state, not the caller
- The "buffer all input until finish()" stub is explicitly rejected for hand-rolled parsers

`commonmark-fmt` is the **only** exemption — pulldown-cmark requires the full `&str`
and superseding it is a non-goal. Every hand-rolled parser has no such excuse.

## Standard Node Kinds (in rescribe-std)

Block: `document`, `paragraph`, `heading`, `code_block`, `blockquote`, `list`, `list_item`, `table`, `table_row`, `table_cell`, `table_header`, `figure`, `horizontal_rule`, `div`, `raw_block`, `definition_list`, `definition_term`, `definition_desc`

Inline: `text`, `emphasis`, `strong`, `strikeout`, `underline`, `subscript`, `superscript`, `code`, `link`, `image`, `line_break`, `soft_break`, `span`, `raw_inline`, `footnote_ref`, `footnote_def`

## Property Namespaces

- Semantic: `level`, `url`, `language`, `content`, `ordered`, `title`, `alt`, etc.
- Style: `style:font`, `style:color`, etc.
- Layout: `layout:page_break`, `layout:float`, etc.
- Format-specific: `html:class`, `latex:env`, `docx:style`, etc.

**IR span semantics must be explicitly defined, not implementation-defined.** A node's span
covers the full syntactic construct including delimiters (`**bold**` strong spans the outer
`**`…`**`). When two backends disagree on span boundaries, that is a bug in one of them —
not a reason to strip spans. `strip_spans` is valid for structural-only tests; span
correctness must be tested separately. Never paper over backend disagreement by stripping
spans — that hides a design gap.

**Never infer type from string content when the IR already carries the type.** If the
reader tags a node with `xlsx:cell-type = "n"`, the writer reads that prop — it does not
re-parse the string to guess whether it looks like a number.

## Development

```bash
nix develop        # Enter dev shell
cargo test         # Run tests
cargo clippy       # Lint
cd docs && bun dev # Local docs
```

## Testing

Pandoc fixtures at `~/git/pandoc/test/` can be used as local reference inputs (GPL - don't copy into repo).

## The fixture suite is the primary deliverable

**The `fixtures/` directory is the real product.** Any implementation in any language
should be able to take `fixtures/{format}/{feature}/input.{ext}` + `expected.json` and
use them as a complete correctness test.

- **Coverage**: `fixtures/{format}/COVERAGE.md` has all boxes checked — every construct
  the format defines, across all six test dimensions. That file is the done signal.
- **Unambiguous**: each fixture has exactly one correct output; if a correct alternative
  implementation would be uncertain what to produce, the fixture is underspecified.
- **Language-agnostic**: `fixtures/{format}/{feature}/input.{ext}` + `expected.json`;
  no Rust-specific assumptions.
- **See `fixtures/spec.md`** for the fixture format spec and COVERAGE.md template.

**When you add support for a new parsed construct, add a fixture for it in the same commit.**
No new feature without a fixture.

## Conventions

- Crate names: `rescribe-{name}` (no rhi prefix per ecosystem convention)
- Reader/writer crates: `rescribe-read-{format}`, `rescribe-write-{format}`
- Node kinds: lowercase with underscores (`code_block`)
- Format-specific kinds: `{format}:{name}` (`html:div`)
- Properties: lowercase, colons for namespacing

## What "5-Production" actually means

**5-Production means the crate is a complete, published-quality library — reader AND
writer, all API modes, full construct coverage, fuzz clean.**

It is not enough to have a working reader. A crate with a complete reader and a stub
writer is not production-grade — it's an incomplete library. Track reader and writer
separately in the audit table, but do not call a vertical "5-Production" until both
are done.

**5-Production requires 100% construct coverage — not "enough for common cases."**

A format vertical is not production-grade until every construct the format can express is
either modeled in the IR or raw-preserved. It should be marked 4-Fuzz until every gap is
closed.

## Vertical completion checklist

Each standalone format crate must satisfy all of:

**Reader:**
- **AST** (`feature = "ast"`, default on): `parse(input: &[u8]) -> (Ast, Vec<Diagnostic>)` — full tree, infallible, Span on every node
- **Streaming** (`feature = "streaming"`, default on): `events(input: &[u8]) -> impl Iterator<Item = Event>`
- **Batch** (`feature = "batch"`, default on): chunk-driven `Parser` (feed/finish)
- Fuzz: no-panic gate (arbitrary bytes must not panic)
- Fuzz: round-trip property `parse(emit(arbitrary_ast)).strip_spans() == arbitrary_ast`
- 100% construct coverage (all format constructs either modeled or raw-preserved)
- Fixture suite complete (`fixtures/{format}/COVERAGE.md` all boxes checked)

**Writer:**
- **Builder** (`feature = "writer-builder"`, default on): `emit(ast: &Ast) -> Vec<u8>`
- **Streaming** (`feature = "writer-streaming"`, default on): event-driven writer
- Fuzz: round-trip property (same as reader — writer must produce re-parseable output)
- Fixture suite: at least the same constructs covered as reader

**Feature gating:** all on by default. Gating is about contract scoping, not binary size.

**Oracle harness (where applicable):**
- Run against Pandoc or another reference implementation for sanity
- Differences must be understood and documented — not silently ignored
- No numeric threshold (≥90% was arbitrary); the goal is zero unexplained differences
- Skip for formats Pandoc cannot read (e.g. AsciiDoc)

**Rescribe integration:**
- Thin adapter ≤300 lines each side
- **100% construct coverage**

See `docs/format-library-design.md` for the full spec.

### Roundtrip direction matters

**Wrong:** `parse(emit(parse(bytes))) == parse(bytes)` — if the parser is lossy, the first
parse already drops content; this only checks dropped content stays dropped.

**Correct:** `parse(emit(arbitrary_format_ast)) == arbitrary_format_ast` — start from an
arbitrary instance of the format crate's own `Ast` type. The native AST is the ground
truth. This covers the full surface area regardless of IR modeling completeness.

## Work is vertical slices only

**Never track "API modes" or any other concern as a horizontal sweep across crates.**
The only valid unit of work is a format vertical: one crate, taken from current state
to 5-Production. All requirements (reader, writer, all API modes, fuzz, fixtures) are
part of that single vertical — not separate dimensions to be swept across formats.

If a crate has a complete reader but a stub writer, it is not 5-Production. If it has
reader + writer but is missing the streaming API mode, it is not 5-Production. The
vertical is done when the whole crate is done. Nothing else counts.

## Workflow

Batch cargo commands to minimize round-trips:
```bash
cargo clippy --all-targets --all-features -- -D warnings && cargo test -q
```
After editing multiple files, run the full check once. `cargo fmt` runs in the pre-commit hook. Prefer `cargo test -q` — quiet mode only prints failures, reducing output noise and context usage.

When making the same change across multiple crates, edit all files first, then build once.

Use `normalize view` for structural exploration:
```bash
~/git/rhizone/normalize/target/debug/normalize view <file>    # outline with line numbers
~/git/rhizone/normalize/target/debug/normalize view <dir>     # directory structure
```

## Commit Convention

Conventional commits: `type(scope): message`

Types: `feat`, `fix`, `refactor`, `docs`, `chore`, `test`. Scope is optional but recommended for multi-crate repos.

<!-- BEGIN ECOSYSTEM RULES -->

## Ecosystem Design Principles

Cross-cutting principles distilled from the ecosystem's own decisions (synthesized in `docs/decisions/throughlines.md`). Apply them when building new repos and recording decisions. (Already-encoded principles — independent-tools / no-path-deps, the delegation model, CLAUDE.md-as-control-surface — live in their own sections and are not repeated here.)

- **Prefer data over code at every seam.** Serializable AST / struct / JSON over closures, embedded DSLs, or source text — so artifacts cache, replay, transport, and diff.
- **Library-first; projection-from-one-definition.** The typed library is the source of truth; CLI / HTTP / MCP / WebSocket / JSON surfaces are generated projections, never hand-rolled per surface.
- **Capability security.** Hosts grant pre-opened handles; code only attenuates what it is given; nothing forges authority; allow-list over deny-list.
- **The LLM is an oracle at the leaves, never the control loop.** Determinism is a hard invariant: seeded RNG, event-log replay, build-time-only inference. Per-query LLM in the hot loop is a defect.
- **Trust comes from verifiable evidence, not authority.** Verbatim snippets, pinned-commit permalinks, claim→node citation — never a bare reference.
- **Retire, don't deprecate; collapse asymmetries to primitives.** Remove backward-compat aliases rather than carry them; reduce N special cases to their irreducible primitives.
- **Validate against reality; tests are the spec.** Load-bearing substrates are validated against real corpora; fixtures and tests define correctness, not aspirational specs.

## Delegation

The main session is an orchestrator. Allowed actions: `Agent`/`Task*`/`AskUserQuestion`/plan-mode/`ScheduleWakeup`, and Bash limited to `git commit`, `git push`, `git status`, `git log --oneline`. Everything else delegates to a subagent. The hook is evidence of a prompting failure, not a behavioral guide. If a tool call hits the hook AT ALL, the prompt failed to prevent it. Delegate before the decision point, not after.

### Triggers

Before calling Read, Grep, Glob, or any Bash beyond the four git commands — stop. Dispatch an Agent instead.

Before editing any file — stop. Dispatch an Agent. This includes plan files in `~/.claude/plans/`: in plan mode, dispatch a subagent to write to the plan file; do not Write it yourself. The plan file's content must not enter main context.

When you need git context beyond status/log-oneline (a diff, a blame, a show) — dispatch an Agent.

When a tool call is denied by the hook — do not retry, do not narrate. Dispatch the equivalent Agent and continue.

When a code-modifying subagent returns — `git status`, then `git commit` before any user-facing reply.

Before dispatching an Agent that modifies code — scan your prompt for "do not commit" or "based on your findings". Delete them.

Before dispatching: if your prompt says "if you find", "based on your findings", or "as appropriate" — stop. Investigate first; dispatch with the decision made.

When you can't verify something — do not speculate or guess at file locations, names, or contents. Dispatch a Read subagent or ask. Confabulation is failure.

### Model Tiers

- Sonnet — exploration, lookup, mechanical multi-file edits, implementation, default.
- Opus — architectural judgment, design, subagents that themselves spawn subagents.

Always set `subagent_type` and `model` explicitly.

### Prompt Rules

- Never tell a subagent "do not commit." Code-modifying subagents commit their own work.
- Don't ask for a diff summary. After a code-modifying subagent, `git status` in main and dispatch a review Agent if you need to see the diff.
- Don't re-explain CLAUDE.md. Subagents inherit it.
- Cite locations by content ("the block that does X"), not line numbers — files shift between reads.
- Name files explicitly; don't outsource the grep.
- Match agent type to deliverable: `Explore` for lookup/search, `general-purpose` for reports and file-modifying work.
- On unsatisfying output, change something before retrying. Same prompt + same tier = same result.
- Dispatch independent subagents in parallel (multiple Agent blocks in one message).
- Pair `isolation: worktree` with `run_in_background: true`.
- Code-modifying subagents must verify their own changes before returning (re-read the diff, run tests, etc.). The orchestrator does not get a second pass with git diff — that's hook-blocked.

### Workflows

Workflows are allowed in the main session (orchestration tool). Lessons (observed 2026-05-30):

- **Resume does not adopt newly-passed `args`.** `resumeFromRunId` reuses the original run's args; args you pass on resume are ignored. Never branch run-mode (e.g. dry-run vs write) on an arg you intend to flip across a resume — it won't flip. Bake the mode into a script constant (the script IS re-read on resume) or use a separate script.
- **Never route large content through one agent for verbatim reproduction.** An agent asked to echo ~100k tokens is slow, costly, and silently truncates. The workflow JS sandbox cannot write files, so all writes go through agents — keep each agent's write payload small and batch many small files per agent, not one giant blob through one agent. For review data, prefer the workflow's structured return value over having an agent transcribe a report file.
- **A resume that produces no expected output is a signal — find the cause before patching a symptom.** (Here: the first write-resume wrote nothing and re-ran a giant report agent; the real cause was args not flipping across resume, not the report agent. Guarding the report agent alone did not fix it.)
- **Gate expensive fan-outs behind a dry-run, and confirm cache reuse before the costly stage.** Mining/read fan-out is the dominant cost; verify it's cached (not re-running) before resuming into write.

## Hard Constraints

- No Edit/Write/NotebookEdit in main. Plan files in `~/.claude/plans/` are written by subagents, not by main.
- No Read/Grep/Glob/NotebookRead in main. Delegate.
- No Bash in main beyond `git commit`, `git push`, `git status`, `git log --oneline`.
- No `--no-verify`. Fix the issue or fix the hook.
- No path dependencies in `Cargo.toml` — they couple repos and break independent publishing.
- No interactive git (no `git rebase -i`, no `git add -i`, no `--no-edit` on rebase).
- No suggesting project names. LLMs are bad at this; refine the conceptual space only.
- No tracking cross-project issues in conversation — they go in TODO.md in the affected repo.
- No ecosystem changes without checking all affected repos.
- No assuming a tool is missing without checking `nix develop`.
- Commit completed work in the same turn it finishes. Uncommitted work is lost work.

## Meta

- Something unexpected is a signal. Stop and find out why. Do not accept the anomaly and proceed.
- Corrections from the user are conversation, not material for new rules. Rules are added when a failure mode is observed repeatedly.

<!-- END ECOSYSTEM RULES -->
