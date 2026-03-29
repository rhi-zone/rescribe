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

- **`ooxml-fmt` is the priority target for the full three-API architecture.** DOCX,
  XLSX, and PPTX routinely exceed RAM on large corpora; `StreamingParser` is not
  optional there. The ooxml-fmt rework (after commonmark-fmt and the five hand-rolled
  crate upgrades) is the most important streaming work in the queue.

- Format crate design decisions (AST shape, event types, error model, span semantics)
  must be made for the widest plausible user, not the narrowest known consumer.

## Priority hierarchy: broadest reach first

Work should be prioritized by how many people benefit:

1. **All language ecosystems** — the `fixtures/` suite. Prioritize formats where no
   authoritative cross-language fixture suite currently exists (RST, Org, AsciiDoc)
   over formats already well-served (CommonMark has the spec suite; HTML has W3C).

2. **Rust ecosystem (any consumer)** — the standalone format crates. A well-designed
   `rst-fmt` crate benefits any Rust project that needs RST, entirely outside rescribe.

3. **Rust ecosystem (single consumer)** — completing the reader/writer API modes matrix.

4. **rescribe** — the IR adapter layer. Thin (≤300 lines per side); doesn't drive
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

## Design Principles

**Unify, don't multiply.** One interface for multiple cases > separate interfaces. Plugin systems > hardcoded switches.

**Simplicity over cleverness.** HashMap > inventory crate. OnceLock > lazy_static. Functions > traits until you need the trait. Use ecosystem tooling over hand-rolling.

**Explicit over implicit.** Log when skipping. Show what's at stake before refusing.

**Separate niche from shared.** Don't bloat shared config with feature-specific data. Use separate files for specialized data.

## Core Rules

**Conversation is not memory.** Anything said in chat evaporates at session end. If it implies a future behavior change, write it to CLAUDE.md immediately — or it will not happen.

**Warning — these phrases mean something needs to be written down right now:**
- "I won't do X again" / "I'll remember to..." / "I've learned that..."
- "Next time I'll..." / "From now on I'll..."
- Any acknowledgement of a recurring error without a corresponding CLAUDE.md edit

**When the user corrects you:** Write the corrected rule to CLAUDE.md before proceeding.
**"The rule exists, I just didn't follow it" is never the diagnosis** — a rule that doesn't
prevent the failure it describes is incomplete; fix the rule.

**Something unexpected is a signal, not noise.** Surprising output, anomalous numbers,
files containing what they shouldn't — stop and ask why before continuing.

**Note things down immediately:**
- Problems, tech debt, issues → TODO.md now, in the same response
- Design decisions, key insights → CLAUDE.md
- Future/deferred scope → TODO.md **before** writing any code

## Negative Constraints

Do not:
- Use Claude Code's auto-memory system (`~/.claude/projects/.../memory/`) — it is unversioned, invisible to the user, and can't be diffed or backed up. Write behavioral changes and project context to CLAUDE.md instead
- Announce actions ("I will now...") - just do them
- Leave work uncommitted
- Use interactive git commands (`git add -p`, `git add -i`, `git rebase -i`) — these block on stdin and hang in non-interactive shells; stage files by name instead
- Use path dependencies in Cargo.toml - causes clippy to stash changes across repos
- Use `--no-verify` - fix the issue or fix the hook
- Assume tools are missing - check if `nix develop` is available for the right environment

## Workflow

**Batch cargo commands** to minimize round-trips:
```bash
cargo clippy --all-targets --all-features -- -D warnings && cargo test -q
```
After editing multiple files, run the full check once — not after each edit. Formatting is handled automatically by the pre-commit hook (`cargo fmt`).

**Prefer `cargo test -q`** over `cargo test` — quiet mode only prints failures, significantly reducing output noise and context usage.

**When making the same change across multiple crates**, edit all files first, then build once.

**Minimize file churn.** When editing a file, read it once, plan all changes, and apply them in one pass.

**Always commit completed work.** After tests pass, commit immediately. When a plan has multiple phases, commit after each phase passes.

**Use `normalize view` for structural exploration:**
```bash
~/git/rhizone/normalize/target/debug/normalize view <file>    # outline with line numbers
~/git/rhizone/normalize/target/debug/normalize view <dir>     # directory structure
```

## Context Management

**Use subagents to protect the main context window.** For broad exploration or mechanical multi-file work, delegate to an Explore or general-purpose subagent.

Rules of thumb:
- Research tasks → subagent
- Searching >5 files or running >3 rounds of grep/read → use a subagent
- Codebase-wide analysis → always subagent
- Mechanical work across many files → parallel subagents
- Single targeted lookup → inline is fine

## Session Handoff

Use plan mode as a handoff mechanism when:
- A task is fully complete (committed, pushed, docs updated)
- The session has drifted from its original purpose
- Context has accumulated enough that a fresh start would help

**For handoffs:** enter plan mode, write a plan containing only: next tasks, blocked/pending items, and what was done this session (only if it directly affects what comes next). Nothing else — no commands, no build steps, no context summaries. Those belong in CLAUDE.md or TODO.md. The next session reads both fresh. **Do NOT investigate first** — the session is context-heavy and about to be discarded.

**For mid-session planning** on a different topic: investigating inside plan mode is fine — context isn't being thrown away.

**TODO.md is the lossless record.** Flush any new items to TODO.md before the handoff.

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
