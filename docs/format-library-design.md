# Format Library Design

What a production-grade standalone format crate looks like. All verticals follow this shape.

See also: `CLAUDE.md` for behavioral rules, `docs/format-audit.md` for per-format status.

---

## The three reader APIs

Each crate exposes three independent reader APIs. They are **not** derived from one another
— each has its own optimal implementation. They share state-transition logic as plain
functions, not a common runtime primitive.

```rust
// 1. AST — full tree in memory.
//    Direct recursive descent. No events, no intermediate representation.
//    Fastest path to a materialised tree.
//    Text fields: owned Strings (must survive the input's lifetime).
pub fn parse(input: &[u8]) -> (Ast, Vec<Diagnostic>);

// 2. Streaming — pull iterator over events.
//    The parser IS the iterator: EventIter holds the parser state, next()
//    advances it. Standard Iterator trait.
//    Text fields: Cow::Borrowed slices from input where no escape decoding
//    is required; Cow::Owned otherwise. Input must be fully in memory.
pub fn events(input: &[u8]) -> EventIter<'_>;

// 3. Chunked streaming — callback model for arbitrarily large input.
//    feed() advances the state machine and calls handler.handle(event) for
//    each event produced. handle() is stack-scoped: Event<'_> borrows from
//    the internal source-buffer window and drops when handle() returns,
//    allowing the buffer to compact. Memory: O(largest token + nesting depth).
pub struct StreamingParser<H: Handler> { ... }
impl<H: Handler> StreamingParser<H> {
    pub fn new(handler: H) -> Self;
    pub fn feed(&mut self, chunk: &[u8]);
    pub fn finish(self);
}

pub trait Handler {
    fn handle(&mut self, event: Event<'_>);
}
```

### Why three APIs, not one universal primitive

The tempting design is a single `StateMachine::advance()` that everything derives from.
It doesn't work cleanly:

- `advance(&mut self) -> Event<'_>` — `Event<'_>` would borrow from `self`'s source
  buffer. This is a **lending iterator**: you can't call `advance()` again while holding
  the previous event. The standard `Iterator` trait cannot express it.
- For `events()` over a fully-loaded `&'a [u8]`, the borrow is from the *caller's*
  slice (lifetime `'a`, independent of `&mut self`), so `Iterator` works fine.
- For `StreamingParser`, the borrow is from an *owned* internal buffer. The only safe
  expression is the callback model: borrow is scoped to the `handle()` call stack.

The three APIs are therefore genuinely distinct, not stylistic variants.

### Shared implementation

The parsers for all three APIs share the same state-transition functions. The difference
is only in how those functions are called:

```
parse_block(...) ─────────────────────┐
parse_inline(...) ────────────────────┤── shared logic, plain functions
parse_escape(...) ────────────────────┘
       │                  │                      │
       ▼                  ▼                      ▼
  parse() builds     events() calls        StreamingParser
  Ast directly       advance_state()       calls advance_state()
  (recursive         and yields one        and dispatches to
  descent)           Event per call        handler.handle()
```

No trait objects. The compiler inlines through all three paths. Each pays only for what
it does.

### `parse()` is NOT `events().collect()`

That formulation is elegant but wrong for production code:

- Forces materialization through the event dispatch layer (one `match` per node)
- Prevents direct struct construction (must go through `Start*/End*` round-trip)
- Pays `into_owned()` on every `Cow` field even when the Ast could construct owned
  strings directly
- Loses the ability to do forward-reference resolution in one pass (footnotes, links)

The behavior of `parse()` and `events().collect()` must be **semantically equivalent**;
the implementations should not share a code path.

---

## Zero-copy and the Cow contract

`Event<'a>` uses `Cow<'a, str>` for all text fields:

```rust
pub enum Event<'a> {
    Text(Cow<'a, str>),
    Code(Cow<'a, str>),
    // ...
    StartHeading { level: usize },  // structural variants: no text, no Cow needed
}
pub type OwnedEvent = Event<'static>;
```

### When `Borrowed` is possible

`Cow::Borrowed` requires that the text run in the output is a verbatim slice of the
input — no escape decoding, no entity expansion, no character transformation. Whether
this is possible depends on the format:

- **CommonMark** — inline code spans and fenced code blocks often have no escapes;
  plain text runs between punctuation are verbatim. `Borrowed` is common.
- **RTF** — `\'XX` hex escapes, `\uN` Unicode escapes, and code-page decoding mean
  almost no text run is verbatim. `Borrowed` is rare; `Owned` is the default.
- **Org / RST / AsciiDoc** — escape sequences exist but are infrequent; most plain
  text is verbatim. `Borrowed` is the common case.

The API does not distinguish these: callers always receive `Cow<'_, str>` and call
`.as_ref()` or `.into_owned()` as needed.

### Materialising the AST from events

When collecting events into an `Ast`, text fields must be `.into_owned()` because the
`Ast` outlives the input slice. This is the **only unavoidable allocation** in the
materialisation path: tree-node allocations are necessary regardless.

For non-materialising consumers (search indexers, linters, syntax highlighters),
`Cow::Borrowed` avoids all text allocations — the most impactful optimisation for
read-only streaming use cases.

### Source buffer compaction in `StreamingParser`

The chunked streaming API keeps an internal `SourceBuffer: Vec<u8>`. As chunks arrive
via `feed()`, they are appended. The `handle()` call borrows from the buffer window.
When `handle()` returns, the borrow drops and the buffer can compact (discard bytes
before the current parse position).

For tokens that span a chunk boundary (e.g. a word split across two `feed()` calls),
`Cow::Owned` is used: the token is assembled from the tail of the previous chunk and
the head of the new one. This is O(token size) allocation — unavoidable and bounded.

Memory profile: O(largest token + nesting depth). Never O(document size).

---

## The writer APIs

```rust
// 1. Builder — construct AST, serialise all at once.
pub fn emit(ast: &Ast) -> Vec<u8>;

// 2. Streaming writer — emit bytes as events arrive, no intermediate buffer.
pub struct Writer<W: Write> {
    sink: W,
    state: WriterState,
}
impl<W: Write> Writer<W> {
    pub fn new(sink: W) -> Self;
    pub fn write_event(&mut self, event: Event<'_>);
    pub fn finish(self) -> W;
}
```

`emit()` is implemented as `Writer::new(Vec::new())` + feed all AST nodes + `finish()`.
The streaming writer is the primitive here (unlike the reader, where the AST builder
is the primitive).

---

## AST requirements

**Every node carries a source span** (byte offsets into the original input):

```rust
pub struct Span { pub start: usize, pub end: usize }
```

Without spans: no diagnostics, no editor integration, no linting, no faithful
attribution. Non-negotiable.

**The tree is owned.** No lifetime ties to the input after construction. Nodes are
`Vec`-backed children. The shape mirrors the format's actual structure, not rescribe's IR.

**`strip_spans()`** removes all spans for comparison in roundtrip tests:
```rust
impl Ast { pub fn strip_spans(&mut self); }
```

---

## Diagnostics

Returned alongside the AST, never tangled into the tree:

```rust
pub struct Diagnostic {
    pub span: Span,
    pub severity: Severity,  // Warning | Info
    pub message: String,
    pub code: &'static str,  // e.g. "rst::ambiguous-underline"
}
```

Most markup formats are error-tolerant — they always produce output. Diagnostics carry
warnings for ambiguous constructs, recovery notes for skipped content, and deprecation
notices for format variants. Callers that don't care ignore the `Vec<Diagnostic>`.

---

## No rescribe types

`Document`, `Node`, `Properties` are invisible to the standalone crate. The rescribe
adapter (`rescribe-read-{fmt}`, `rescribe-write-{fmt}`) is the only place they appear,
and it must be thin — ideally under 300 lines per side.

---

## The `commonmark-fmt` exception

`commonmark-fmt` wraps pulldown-cmark (77M+ weekly downloads; the de facto Rust
CommonMark ecosystem). This is a deliberate choice:

- `events()` and `parse()`: max perf — pulldown-cmark is a true pull iterator with
  `into_offset_iter()` for spans. The wrapper is a thin event-type translation layer.
- `StreamingParser`: **not at max perf** — pulldown-cmark requires full `&str` input.
  `StreamingParser::feed()` buffers all chunks; `finish()` runs pulldown on the
  complete input. This is documented explicitly in the crate-level doc comment.

**Superseding pulldown-cmark is a non-goal.** A caller needing true chunked CommonMark
streaming should use pulldown-cmark directly or wait for a future native parser. The
`commonmark-fmt` crate's value is a consistent API surface, spans, diagnostics, and
compatibility with the rest of the format ecosystem — not streaming performance.

---

## `ooxml-fmt` and the streaming imperative

DOCX, XLSX, and PPTX files routinely exceed available RAM in batch-over-corpus
scenarios (legal discovery, academic corpus analysis, enterprise search indexing).
`StreamingParser` is not optional for these formats — it is the primary use case.

The `ooxml-fmt` rework (consolidating `ooxml-wml/sml/pml` into one crate) must
implement the full three-API architecture with a real `StreamingParser`:

- Chunked ZIP entry streaming (OPC layer reads entries without decompressing all at once)
- XML SAX events fed directly to the format state machine
- `Handler` callbacks with O(nesting depth) working set

This is the most important streaming work in the queue after the five hand-rolled
crate upgrades.

---

## Fuzz harness

Two targets per format:

1. **No-panic gate** — arbitrary bytes must not panic or OOM:
   ```rust
   fuzz_target!(|data: &[u8]| { let _ = parse(data); });
   ```

2. **Round-trip property** — `parse(emit(arbitrary_ast)) == arbitrary_ast`:
   ```rust
   fuzz_target!(|ast: Ast| {
       let (ast2, _) = parse(&emit(&ast));
       assert_eq!(ast.strip_spans(), ast2.strip_spans());
   });
   ```

   Starting from an arbitrary `Ast` (not from `parse(bytes)`) is critical: it covers
   the full AST surface regardless of what the parser currently accepts.

---

## Feature flags

All on by default. Gating is about contract scoping, not binary size:

```toml
[features]
default = ["ast", "streaming", "batch", "writer-streaming", "writer-builder"]
ast = []
streaming = []          # events(); requires Event types, independent of ast feature
batch = []              # StreamingParser<H> + Handler trait
writer-streaming = []   # Writer<W: Write>
writer-builder = []     # emit(); wraps writer-streaming internally
```

A consumer who writes `default-features = false, features = ["ast"]` is explicitly
contracting for only `parse()`. They cannot accidentally couple to the streaming or
batch APIs.

---

## Crate layout

```
crates/formats/{name}/
├── Cargo.toml          # no rescribe-* dependencies
├── src/
│   ├── lib.rs          # pub use; crate-level doc comment with limitations
│   ├── ast.rs          # Ast, Block/Inline enums, Span, Diagnostic, strip_spans()
│   ├── parse.rs        # parse() — direct recursive descent
│   ├── events.rs       # Event<'a>, OwnedEvent, EventIter, collect_*() helpers
│   ├── batch.rs        # StreamingParser<H>, Handler trait, SourceBuffer
│   ├── emit.rs         # emit() — builder writer
│   └── writer.rs       # Writer<W: Write> — streaming writer
└── tests/
    └── roundtrip.rs    # proptest round-trip property tests
```

---

## Vertical completion checklist

- [ ] `ast.rs` — `Ast`, all node variants, `Span` on every node, `strip_spans()`
- [ ] `parse.rs` — `parse(input: &[u8]) -> (Ast, Vec<Diagnostic>)`, direct recursion
- [ ] `events.rs` — `Event<'a>` with `Cow<'a, str>` text fields, `EventIter<'a>`,
      parser-IS-iterator (no `collect_block_events` indirection)
- [ ] `batch.rs` — `StreamingParser<H: Handler>`, `SourceBuffer`, chunked feed
- [ ] `emit.rs` — `emit(ast: &Ast) -> Vec<u8>`
- [ ] `writer.rs` — `Writer<W: Write>`, `write_event`, `finish`
- [ ] Feature flags: `ast`, `streaming`, `batch`, `writer-streaming`, `writer-builder`
- [ ] No-panic fuzz gate
- [ ] Round-trip fuzz (from arbitrary Ast, not from bytes)
- [ ] `rescribe-read-{fmt}` adapter ≤300 lines
- [ ] `rescribe-write-{fmt}` adapter ≤300 lines
- [ ] Fixture suite: `fixtures/{format}/COVERAGE.md` all boxes checked
- [ ] `docs/format-audit.md` updated to 5-Production
