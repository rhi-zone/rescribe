# rescribe

Universal document conversion library - a pandoc-inspired approach with lossless intermediate representation.

## Features

- **Extensible IR** - Open node kinds and property bags instead of fixed schemas
- **Fidelity tracking** - Know what was lost during conversion
- **Embedded resources** - First-class handling of images, fonts, data
- **Roundtrip friendly** - Source format info preserved

## Quick Start

```rust
use rescribe::{Document, Parser, Emitter};
use rescribe_markdown::MarkdownParser;
use rescribe_html::HtmlEmitter;

let parser = MarkdownParser::new();
let emitter = HtmlEmitter::new();

let result = parser.parse(markdown_bytes, &ParseOptions::default())?;
let output = emitter.emit(&result.value, &EmitOptions::default())?;
```

## CLI

```bash
# Convert between formats
rescribe convert doc.md -t html -o doc.html

# Run a jq expression against a document's IR
rescribe query '.metadata' doc.md
rescribe query '[.. | .kind?] | group_by(.) | map({kind: .[0], count: length})' doc.md
```

`rescribe query` embeds the [jaq](https://github.com/01mf02/jaq) jq engine, so metadata
inspection and construct-census tooling are just queries rather than dedicated
subcommands — see `rescribe query --help`.

## Why not Pandoc?

| Aspect | Pandoc | rescribe |
|--------|--------|----------|
| Schema | Fixed Haskell ADT | Open `NodeKind` + Properties |
| Format-specific data | Mostly lost | Namespaced properties preserved |
| Style/layout info | Not represented | `style:*` / `layout:*` properties |
| Embedded resources | External references | First-class `ResourceMap` |
| Fidelity tracking | None | Warnings on conversion |

## Project Structure

```
rescribe/
├── crates/
│   ├── rescribe-core/       # Core IR types and traits
│   ├── rescribe-markdown/   # Markdown parser/emitter
│   ├── rescribe-html/       # HTML parser/emitter
│   └── rescribe-transforms/ # Standard transformers
└── docs/                    # Documentation site
```

## Development

```bash
# Enter dev shell
nix develop

# Run tests
cargo test

# Run docs locally
cd docs && bun dev
```

The Rust toolchain (rustc/cargo/rustfmt/clippy) is pinned in `rust-toolchain.toml` and
built into the dev shell via `flake.nix`, so `cargo fmt` produces identical output for
everyone using `nix develop`/direnv, and CI (`.github/workflows/ci.yml`) is pinned to
the same version. Formatting options live in `rustfmt.toml`. Before this pin, a bare
`cargo fmt` could reformat hundreds of unrelated files depending on which rustfmt
version happened to be installed; the whole tree was reformatted once under the pinned
version and that single commit is recorded in `.git-blame-ignore-revs` so it doesn't
obscure `git blame` history.

To make local `git blame` skip that reformat commit too (GitHub's web blame view
already does this automatically, no setup needed there), run once per clone — this
isn't tracked by git, so each contributor sets it individually:

```bash
git config blame.ignoreRevsFile .git-blame-ignore-revs
```

## Part of rhi ecosystem

rescribe is part of the [rhi ecosystem](https://rhi.zone) - a collection of tools for data transformation and content processing.

## License

MIT OR Apache-2.0
