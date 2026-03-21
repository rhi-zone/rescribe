# Rescribe Fixture Specification

Version: 1.2
License: MIT

This document defines the cross-language fixture format used by rescribe tests.
Fixtures are plain files — any language can implement a validator.

## Directory layout

```
fixtures/
  {format}/
    {feature}/
      input.{ext}      ← input document in the format under test
      expected.json    ← assertions about the parsed result
```

`{format}` is the rescribe format name (`markdown`, `html`, `rst`, etc.).
`{feature}` is a short descriptive name for the feature being tested.
The input extension matches the format (`.md`, `.html`, `.rst`, …).

**Naming conventions for feature directories:**

| Prefix | Meaning |
|--------|---------|
| (none) | Happy path — standard, valid input |
| `rare-` | Valid but uncommon/obscure syntax |
| `adv-` | Adversarial — malformed or extreme input |

## `expected.json` schema

```json
{
  "description": "Human-readable description of what is being tested",
  "category": "happy",
  "expect_error": false,
  "metadata": {
    "title": "My Document"
  },
  "assertions": [
    { "path": "/0",   "kind": "paragraph" },
    { "path": "/0/0", "kind": "text", "props": { "content": "hello" } }
  ]
}
```

### Top-level fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `description` | string | yes | — | Free-text description |
| `category` | string | no | `"happy"` | One of `"happy"`, `"rare"`, `"adversarial"` |
| `expect_error` | bool | no | `false` | If true, a parse error is acceptable (skip assertions). Parser must still not panic. |
| `metadata` | object | no | `{}` | Assertions about document-level metadata (same value semantics as `props`) |
| `assertions` | array | no | `[]` | List of node assertions |

**`category` meanings:**

| Value | Meaning |
|-------|---------|
| `"happy"` | Single construct in isolation — the minimal case that proves recognition works |
| `"integration"` | Multiple constructs interacting — emphasis inside a list, table in a blockquote, etc. |
| `"e2e"` | A realistic whole document — tests that a full document round-trips correctly |
| `"rare"` | Valid but obscure or uncommon syntax — tests edge-case coverage |
| `"adversarial"` | Malformed, truncated, or invalid input — tests robustness (must not panic) |
| `"pathological"` | Valid but stress-inducing — deeply nested structures, very long lines, large tables; tests that the parser doesn't blow up on unusual-but-legal input |

### Assertion fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `path` | string | yes | Path from content root (see below) |
| `kind` | string | no | Expected node kind |
| `props` | object | no | Expected property values |
| `children_count` | integer | no | Expected number of children |

## Path semantics

Paths are `/`-delimited sequences of non-negative integers.

The root of the path tree is `document.content` — the top-level document node
(kind `"document"`). Each integer component indexes into the `children` array
of the current node.

| Path | Meaning |
|------|---------|
| `""` | `document.content` itself (the document node) |
| `/0` | `content.children[0]` |
| `/0/0` | `content.children[0].children[0]` |
| `/0/2/1` | `content.children[0].children[2].children[1]` |

The empty path `""` is useful in adversarial tests to assert top-level structure
(e.g., `{ "path": "", "kind": "document", "children_count": 0 }` for an empty doc).

## Property matching

Each key in `props` (or `metadata`) is a property name. The value specifies what to expect:

| JSON value type | Matches rescribe prop type |
|-----------------|---------------------------|
| `"string"` | `PropValue::String` |
| integer (e.g. `1`) | `PropValue::Int` |
| float (e.g. `1.5`) | `PropValue::Float` |
| `true` / `false` | `PropValue::Bool` |
| `null` | prop must be **absent** |

## Metadata assertions

The `metadata` object asserts against document-level metadata (e.g., YAML frontmatter,
HTML `<meta>` tags). Keys and value semantics are identical to `props` assertions.

```json
{
  "description": "YAML frontmatter title is parsed into metadata",
  "metadata": { "title": "My Doc" },
  "assertions": [
    { "path": "/0", "kind": "paragraph" }
  ]
}
```

## Fixture suite completeness

A fixture suite for a format is **complete** when `fixtures/{format}/COVERAGE.md` has all
items checked. That file is the source of truth for what's missing.

The suite must cover all six test dimensions:

| Dimension | What it tests |
|-----------|---------------|
| **Happy path** | Every construct the format defines, in isolation |
| **Integration** | Constructs interacting — e.g., inline markup inside a table cell |
| **End-to-end** | Realistic whole documents, not just isolated constructs |
| **Rare** | Obscure but valid syntax that implementations often get wrong |
| **Adversarial** | Malformed, truncated, or invalid input — parser must not panic |
| **Pathological** | Valid but stress-inducing — deeply nested, very large, unusual but legal |

A format's fixture suite is not complete until all six dimensions have meaningful coverage
for all constructs. "One fixture per construct" is the floor, not the ceiling.

### COVERAGE.md

Each format has `fixtures/{format}/COVERAGE.md` listing every construct defined by the
format spec, with checkboxes and fixture names. The done signal is all boxes checked.

Template:

```markdown
# {Format} Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

## Block constructs
- [ ] paragraph — `paragraph`
- [ ] heading — `heading`

## Inline constructs
- [ ] emphasis — `emphasis`

## Properties
- [ ] language on code block — `code-block-lang`

## Composition (integration)
- [ ] emphasis inside list item — (missing)
- [ ] table inside blockquote — (missing)

## Adversarial
- [ ] empty document — `adv-empty`
- [ ] unclosed inline markup — (missing)

## Pathological
- [ ] 100-level deep nesting — (missing)
- [ ] very long paragraph (>64 KB) — (missing)
```

Fixture names in parentheses marked `(missing)` are gaps. Add them before declaring the
suite complete.

## Adversarial fixtures

Fixtures with `"category": "adversarial"` test robustness. Rules:

- The parser **must not panic** under any circumstances.
- If `expect_error` is false (default), the parser must return a document (even if degraded).
- If `expect_error` is true, a parse error is acceptable; no assertions are checked.
- Assertions may be empty (`[]`) when the only goal is no-panic verification.

```json
{
  "description": "Unclosed code fence is handled gracefully",
  "category": "adversarial",
  "assertions": []
}
```

## Rescribe node JSON representation

For reference, the rescribe document IR serialises as:

```json
{
  "kind": "document",
  "props": {},
  "children": [
    {
      "kind": "paragraph",
      "props": {},
      "children": [
        { "kind": "text", "props": { "content": "hello" }, "children": [] }
      ]
    }
  ]
}
```

A validator can:
1. Invoke `rescribe convert --from {format} --to native-json < input.{ext}`
2. Parse the resulting JSON
3. Check `metadata` assertions against the top-level `metadata` field
4. Walk paths and check node assertions

## Writer fixtures

Write-only formats (presentation writers, etc.) use a parallel directory tree
under `fixtures/writers/`:

```
fixtures/
  writers/
    {format}/
      {feature}/
        input.json   ← pandoc-json document (the IR input)
        expected.json ← output assertions (see below)
```

`input.json` is a pandoc-json document.  The runner parses it with
`rescribe_read_pandoc_json`, then passes the resulting `Document` to the emitter.

### Writer `expected.json` schema

```json
{
  "description": "Human-readable description",
  "category": "happy",
  "expect_error": false,
  "output_contains": ["\\documentclass{beamer}", "\\begin{document}"]
}
```

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `description` | string | yes | — | Free-text description |
| `category` | string | no | `"happy"` | Same values as reader fixtures |
| `expect_error` | bool | no | `false` | If true, an emit error is acceptable |
| `output_contains` | array of strings | no | `[]` | Substrings that must appear in the emitted output |

A validator can:
1. Parse `input.json` as pandoc-json into a rescribe document
2. Invoke `rescribe convert --from pandoc-json --to {format} < input.json`
3. Check that each `output_contains` string appears somewhere in the output

---

## Example

Input (`markdown/bold/input.md`):
```markdown
**hello**
```

Assertions (`markdown/bold/expected.json`):
```json
{
  "description": "Bold text wraps content in a strong node",
  "assertions": [
    { "path": "/0",     "kind": "paragraph" },
    { "path": "/0/0",   "kind": "strong" },
    { "path": "/0/0/0", "kind": "text", "props": { "content": "hello" } }
  ]
}
```
