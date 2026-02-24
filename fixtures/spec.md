# Rescribe Fixture Specification

Version: 1.0
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

## `expected.json` schema

```json
{
  "description": "Human-readable description of what is being tested",
  "assertions": [
    { "path": "/0",   "kind": "paragraph" },
    { "path": "/0/0", "kind": "text", "props": { "content": "hello" } }
  ]
}
```

### Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `description` | string | yes | Free-text description |
| `assertions` | array | yes | List of node assertions |

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
| `/0` | `content.children[0]` |
| `/0/0` | `content.children[0].children[0]` |
| `/0/2/1` | `content.children[0].children[2].children[1]` |

## Property matching

Each key in `props` is a property name. The value specifies what to expect:

| JSON value type | Matches rescribe prop type |
|-----------------|---------------------------|
| `"string"` | `PropValue::String` |
| integer (e.g. `1`) | `PropValue::Int` |
| float (e.g. `1.5`) | `PropValue::Float` |
| `true` / `false` | `PropValue::Bool` |
| `null` | prop must be **absent** |

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
3. Walk paths and check assertions

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
