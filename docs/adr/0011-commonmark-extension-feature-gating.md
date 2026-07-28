# 11. `commonmark-fmt` construct extensions are opt-in Cargo features, not default-on

## Status

Accepted (2026-07-28).

## Context

`commonmark-fmt` wraps pulldown-cmark. Before this change, the crate modeled a strict
subset of what pulldown-cmark can parse: paragraphs, headings, code blocks, blockquotes,
lists, emphasis/strong, links/images, HTML — plus GFM strikethrough, which was turned on
unconditionally via `Options::ENABLE_STRIKETHROUGH` with no feature gate at all.

Six pulldown-cmark extension families were unimplemented: YAML/TOML front matter, GFM
tables, GFM task lists, GFM (bare-URL) autolinks, footnotes, definition lists, and math.
Because the crate's Cargo features (`reader-ast`, `reader-streaming`, `reader-batch`,
`writer-streaming`, `writer-builder`) only gate *API modes*, not *constructs*, and because
`rescribe-read-markdown`'s default backend used this crate without enabling any construct
beyond strikethrough, real input hit a specific failure mode worse than "unsupported":
YAML front matter (`---\ntitle: X\n---`) parsed as a `ThematicBreak` followed by a setext
`Heading` containing the literal YAML text — a structurally wrong node inserted into the
document body, with no fidelity warning. TOML front matter (`+++`) wasn't recognized as a
delimiter at all and merged into a plain paragraph. This is the silent-misparse bug this
ADR's companion work fixes.

### The naming problem

`commonmark-fmt` is a spec-named crate — the natural expectation for a Rust library named
"CommonMark" is that it parses/emits the CommonMark spec, not GitHub Flavored Markdown or
other pulldown-cmark-specific extensions. Turning every construct on by default (matching
the crate's existing "all API-mode features on by default" convention, and CLAUDE.md's
general "Gating is about contract scoping, not binary size" guidance for API modes) would
make `commonmark-fmt::parse("plain text")` silently accept and round-trip GFM/extension
syntax no CommonMark spec implementation defines — surprising for a crate whose name makes
a spec claim.

### Options considered

1. **All constructs on by default** (matches existing API-mode convention). Simplest,
   fewest feature combinations to reason about. Rejected: breaks the crate's name-implied
   spec-compliance contract; a caller who wants pure CommonMark has no way to get it short
   of pre-filtering their own Markdown source for extension syntax.
2. **One `extensions` feature bundling everything**, off by default. Coarser than
   necessary: a caller who only wants tables is forced to compile (and reason about the
   API surface of) footnotes, math, and definition lists too.
3. **Individual feature per construct, off by default, with umbrella aliases for
   convenience.** Chosen.

## Decision

Every construct beyond bare CommonMark is gated behind its own Cargo feature, off by
default: `tables`, `task-lists`, `strikethrough`, `frontmatter`, `footnotes`,
`definition-lists`, `math`. `strikethrough` — previously unconditional — is now gated like
every other extension; this is a breaking change for any existing consumer relying on the
old always-on behavior, but consistency was judged more valuable than backward
compatibility for a crate still at `0.1.0`.

Two umbrella aliases exist purely as Cargo feature composition (`feature = [...]` lists,
no independent code path):
- `gfm = ["tables", "task-lists", "strikethrough"]` — the GitHub Flavored Markdown spec's
  own construct list (tables, task lists, strikethrough). See the autolinks note below for
  why autolinks are *not* in this list.
- `extensions = ["gfm", "frontmatter", "footnotes", "definition-lists", "math"]` — every
  construct feature.

Plain CommonMark (`commonmark-fmt` with only the API-mode features, i.e. `--no-default-features
--features reader-ast,reader-streaming,reader-batch,writer-streaming,writer-builder`, or
equivalently the *default* feature set with no construct features added) now round-trips
`~~text~~`, `| a | b |`, `---\nfoo\n---`, etc. as literal spec-CommonMark text — no
extension syntax is recognized — matching the crate's name.

### Autolinks — a discovered gap, not a feature

The GFM spec defines bare-URL autolinking (`https://example.com` linkified without
`<...>`) as an extension. Angle-bracket autolinks (`<https://example.com>`) are core
CommonMark and were already unconditionally supported (pulldown-cmark parses `Tag::Link`
with `LinkType::Autolink` regardless of `Options`). Investigating pulldown-cmark 0.13.1's
`Options` bitflags directly (`grep ENABLE_GFM` across the vendored source) found that
`Options::ENABLE_GFM` gates only GitHub-style blockquote alert tags (`[!NOTE]` etc.) — bare-URL
autolinking is not implemented in pulldown-cmark 0.13 as a togglable Option at all. There is
therefore no Cargo feature for "autolinks" in this crate: adding one would be adding a knob
that does nothing, which is worse than not adding it. This is tracked as a real, disclosed
gap in TODO.md, not silently worked around.

### Feature independence and interaction risk

Each construct feature only toggles its own `pulldown_cmark::Options` bit (see
`options.rs::build_options`) and its own AST variant / event variant / emit arm. They were
verified to compile and pass their own tests individually
(`cargo test -p commonmark-fmt --no-default-features --features reader-ast,reader-streaming,
reader-batch,writer-streaming,writer-builder,<one-feature>` for each of the seven).
Exhaustive pairwise/combination testing was **not** done — only the "all off" (plain
CommonMark), "each one alone", and "all on" (`--all-features`) points in the combination
space are verified. One real interaction was found during pulldown-cmark source
investigation (not guessed): `Options::ENABLE_GFM` — irrelevant to any construct feature in
this crate as shipped, since it's only used for blockquote alert tags — also affects
footnote reference *kind* internally in pulldown-cmark's firstpass parser
(`firstpass.rs:231`). Since neither `footnotes` nor any autolinks feature exists yet, this
interaction is inert today; it must be re-examined before `footnotes` support is added if
that implementation ever turns on `ENABLE_GFM`.

## Consequences

- `rescribe-read-commonmark` and `rescribe-read-markdown` (and their writer counterparts)
  must explicitly request the construct features they need from `commonmark-fmt` in their
  own `Cargo.toml` — they now depend on `features = ["frontmatter", "tables", "task-lists",
  "strikethrough"]` rather than getting strikethrough for free and everything else never.
- `commonmark-fmt`'s own test suite must be run with `--all-features` to exercise every
  construct; a plain `cargo test -p commonmark-fmt` (default features) only exercises the
  spec-CommonMark core plus the five API-mode paths.
- `footnotes`, `definition-lists`, and `math` are declared as empty (inert) Cargo features
  today — reserved names, no `Options` bit wired up yet, no AST variant. This is
  intentional: the feature names are settled so downstream `Cargo.toml` files depending on
  `commonmark-fmt` can request them now and get real behavior later without a breaking
  rename. See TODO.md for the tracked implementation gap.
