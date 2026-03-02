# Format Library Design

What a good standalone format crate looks like. All verticals follow this shape.

---

## API layers

Two layers, one built on the other:

```rust
// Low-level: pull parser — zero allocation, streaming
pub fn events(input: &str) -> impl Iterator<Item = Event> + '_;

// High-level: owned AST — what most users want
pub fn parse(input: &str) -> (Ast, Vec<Diagnostic>);
```

The rescribe adapter uses whichever is more convenient. Both are public.

---

## AST requirements

**Every node carries a source span** (byte offsets into the original input).

```rust
pub struct Span {
    pub start: usize,
    pub end: usize,
}
```

Without spans you cannot do diagnostics, editor integration, linting, or
faithful round-trip attribution. This is non-negotiable. (jotdown, for
comparison, does not expose spans on the AST at all.)

**The tree is owned** — no lifetime ties to the input string after construction.
Nodes are `Vec`-backed children, properties are typed fields, not stringly-typed
bags. The shape mirrors the format's actual structure, not rescribe's IR.

---

## Emitter

Every library crate ships an emitter alongside the parser.

```rust
pub fn emit(ast: &Ast) -> String;
```

**Round-trip guarantee:** `parse(emit(parse(input).0).0` produces an AST
identical to `parse(input).0`. This is tested by a fuzz harness (see below).
A library without an emitter is only half the story.

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

Most markup formats don't have hard parse errors — they always produce output.
But you still want warnings for ambiguous constructs, recovery notes for skipped
content, and deprecation notices for format variants. Callers that don't care
can ignore the `Vec<Diagnostic>`.

---

## No rescribe types

`Document`, `Node`, `Properties` are invisible to the standalone crate.
The rescribe adapter (`rescribe-read-{fmt}`, `rescribe-write-{fmt}`) is the
only place they appear, and it should be thin — ideally under 300 lines.

---

## Fuzz harness (in `fuzz/`)

Two targets per format, same as the djot precedent:

1. **No-panic gate** — feed arbitrary bytes, assert no panic or OOM:
   ```rust
   fuzz_target!(|data: &[u8]| {
       if let Ok(s) = std::str::from_utf8(data) { let _ = parse(s); }
   });
   ```

2. **Round-trip property** — parse → emit → re-parse, assert AST equivalence:
   ```rust
   fuzz_target!(|data: &[u8]| {
       if let Ok(s) = std::str::from_utf8(data) {
           let (ast1, _) = parse(s);
           let (ast2, _) = parse(&emit(&ast1));
           assert_eq!(ast1, ast2);
       }
   });
   ```

---

## Memory model and zero-copy

**Why zero-copy doesn't apply to format text content.**
Every markup format requires transformation during parsing — RTF decodes `\'XX`
hex escapes (Windows-1252 → Unicode), named control words (`\emdash` → U+2014),
`\uN` Unicode escapes, and backslash-escaped literals. The decoded text is never
a verbatim slice of the input, so `&'a str` borrowing from the source buffer is
not possible for text nodes. `Cow<'a, str>` would help only for runs with zero
escapes, which are uncommon in real-world files of any format.

The same applies to format-specific raw-preservation fields like RTF's
`para_props`: they are reconstructed from the parsed token stream, not lifted
verbatim from the input, so they are owned `String`s.

**Why this is not a problem in practice.**
The AST heap cost is proportional to document size, and the document was already
in memory as the input `&str`. `para_props` adds ~20 bytes per paragraph on
average (a typical indent+spacing definition). For a 10,000-paragraph document
that is ~200 KB — negligible relative to the input itself.

**When it would matter: the `events()` pull iterator.**
For very large files or streaming pipelines, the right tool is the low-level
pull iterator — not feature flags or zero-copy tricks. The iterator emits one
event at a time and requires only O(nesting depth) working state, so the full
AST is never materialised. The rescribe adapter can choose to use `events()`
internally instead of `parse()` if memory is a concern.

Feature flags should be reserved for *optional dependencies* (e.g. `serde` for
`Serialize`/`Deserialize` on AST types), not for behavioral toggles like
"skip layout props". Behavioral options belong in a `ParseOptions` struct passed
to `parse_with_options()`.

---

## Crate layout

```
crates/formats/{name}/
├── Cargo.toml          # no rescribe-* dependencies
├── src/
│   ├── lib.rs          # pub use; top-level doc comment
│   ├── ast.rs          # Ast, Node variants, Span, Diagnostic
│   ├── parse.rs        # parse() + events()
│   └── emit.rs         # emit()
└── tests/
    └── roundtrip.rs    # property tests with proptest or similar
```

---

## Checklist per vertical

- [ ] `Ast` type with `Span` on every node
- [ ] `parse(input) -> (Ast, Vec<Diagnostic>)`
- [ ] `events(input) -> impl Iterator<Item = Event>`
- [ ] `emit(ast) -> String`
- [ ] Round-trip fuzz target passes clean
- [ ] No-panic fuzz target passes clean
- [ ] Published to crates.io (or ready to publish)
- [ ] Thin rescribe adapter (reader + writer, ≤300 lines each)
- [ ] Rescribe fixture suite at 3-Harness
