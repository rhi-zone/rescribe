# 7. DTD-aware entity resolution: a standalone crate, layered over quick-xml and a bought standard table

## Status

Accepted; implemented. `xml-entities` (`crates/formats/xml-entities`) is a standalone crate
(commit `37253485d0`), depended on by `docbook-fmt`, `jats-fmt`, and `tei-fmt` (each crate's
`Cargo.toml`, `parse.rs`/`events.rs`/`batch.rs`). The underlying XML parser for these three
crates remains `quick-xml`; switching parsers to obtain internal-subset entity discovery
natively was evaluated on its merits (see Alternatives) and rejected.

## Context

`docbook-fmt`, `jats-fmt`, and `tei-fmt` raw-preserve any named XML entity beyond the 5
XML-predefined ones (`lt`/`gt`/`amp`/`apos`/`quot`) and numeric character references losslessly
as `Node::EntityRef` / IR `raw_inline` with a format-namespaced property
(`docbook:entity`/`jats:entity`/`tei:entity`) — never silently dropped, per CLAUDE.md's
raw-preservation rule. But without resolution, that's all a consumer gets: DocBook/JATS/TEI
documents can declare custom entities in a DTD internal subset (`<!ENTITY name
"replacement">`) and/or reference standard ISO 8879 / ISO/IEC 9573 entity sets (ISOlat1,
ISOnum, ISOpub, ISOtech, ISOgrk, mmlalias, etc.), and a consumer that wants the actual
replacement text/character has no path to it from raw preservation alone.

### What quick-xml actually provides (verified against its source, not assumed)

quick-xml 0.39 is not entity-blind. It resolves entities in two independent, already-shipped
ways:

- `quick_xml::escape::resolve_xml_entity` (`src/escape.rs:556`) resolves the five
  XML-predefined entities. `unescape()`/`unescape_with()` (`src/escape.rs:212,255`) also
  resolve numeric character references (`&#60;`, `&#x3C;`) — both are opt-in: a caller must
  call `.unescape()`/`.decode()` on a value; nothing is resolved automatically during
  `read_event()`.
- `unescape_with<F: FnMut(&str) -> Option<&str>>(raw, resolve_entity)` (`src/escape.rs:255`)
  additionally exposes a custom-entity resolution hook. This was added by contributor
  `pchampin`'s PR #261 in response to quick-xml issue #258 ("Declared entity is not
  recognized") and merged years ago — confirmed live via `gh issue view 258
  --repo tafia/quick-xml --comments`: pchampin's PR is referenced and confirmed merged in
  that same thread ("my PR above was merged"). **The resolution half of issue #258 shipped
  long ago.** (Character references still resolve before the closure runs and can't be
  overridden by it — documented on the same function.)

What quick-xml does **not** do, also verified from source: parse the DOCTYPE internal subset
for `<!ENTITY>` declarations. `Event::DocType` is an opaque blob; the DTD-skipping state
machine (`src/parser/dtd.rs`) only tracks quote/comment/PI nesting well enough to find the
terminating `>` — maintainer `Mingun`, on the #258 thread, confirmed this in so many words:
"currently we fail to parse this... because we simply count `<` and `>` inside DOCTYPE
definition to know where DTD is ended." No entity table is ever built from a document's own
DOCTYPE. **This discovery half is what remains open in #258.** Later on the same thread, a
user (`melissaboiko`) asked for exactly this — a `reader::Config`-level entity map with
transparent expansion into `Event::Text` — and Mingun replied: "That is the plan for further
improvements" (not shipped in 0.39, and no committed timeline).

Collaborator `dralley` raised the billion-laughs / entity-expansion-attack risk on that same
thread as a reason quick-xml may be reluctant to ever fully automate recursive expansion —
relevant here because `xml-entities`' own resolver has to make the same tradeoff, and does so
explicitly (`resolve.rs`: `MAX_RESOLUTION_DEPTH = 64` plus a `HashSet` cycle guard;
`decl.rs`: `MAX_PARAMETER_EXPANSION_DEPTH = 64` for parameter-entity expansion during
discovery). A `<!ENTITY loop "&loop;">` or mutually-recursive pair resolves to a bounded
fallback rather than looping or panicking (see `resolve.rs` tests
`self_referential_entity_does_not_infinite_loop_or_panic`,
`mutually_recursive_entities_do_not_infinite_loop_or_panic`).

Because quick-xml 0.39 emits entity references as their own `Event::GeneralRef` (rather than
folding them into `Text`), `docbook-fmt`/`jats-fmt`/`tei-fmt` dispatch each reference manually
already (`docbook-fmt/src/parse.rs:203` and the JATS/TEI equivalents):
`resolve_char_ref()` → `resolve_predefined_entity` → `entity_resolver.resolve(&name)` →
raw-preserve as `Node::EntityRef` if unresolved. This is a different mechanism from quick-xml's
`unescape_with` closure hook (which resolves inline during unescaping of a single string), not
a duplication of it — none of the three format crates call `unescape_with`.

A research pass also checked crates.io, docs.rs, and GitHub issue trackers for an existing
crate that fills the DOCTYPE-discovery gap specifically:

- **`dtd`** (0.1.0, last published 2019) and **`dtd-parser`** (0.1.0-alpha3, last published
  2021) both exist but are early-stage/unmaintained, parse DTD grammar into a data structure at
  best, and provide no entity-resolution layer, no bundled ISO/HTML5 tables, and no adoption
  evidence.
- **`entities`** (~6.9M downloads, maintained) bundles the WHATWG HTML5 named-character-
  reference table, a close superset of the ISO 8879/9573 sets DocBook/JATS/TEI lean on — but it
  has no notion of a document's own DTD-declared entities, which the ISO/HTML5 table cannot
  cover by definition (they're format- and document-specific, not standardized). This table is
  unrelated to and larger than quick-xml's own optional `escape-html` feature, which is a
  smaller HTML5-quirks-mode table (legacy no-semicolon forms like `&amp`) not intended for
  ISO 8879/9573 coverage.
- No crate packages the ISO 8879/9573 `.ent` files directly as a Rust table (`entity_table` on
  crates.io is an unrelated ECS crate, a false-positive name collision).

## Decision

**Build** a narrow, standalone `xml-entities` crate with three layers, each independently
verifiable against its own module:

1. **DTD internal-subset declaration discovery** (`decl.rs`, `DtdEntities`): parses
   `<!ENTITY name "replacement">` declarations (general and parameter entities,
   `SYSTEM`/`PUBLIC` external entities recorded but never fetched — no network or filesystem
   access anywhere in the crate — numeric character references expanded at declaration time
   per the XML spec, nested general-entity references left for resolve-time expansion). This
   is the half quick-xml does not do and has no committed timeline for (see Context).
2. **The standard table** (`standard.rs`, `resolve_standard`): the WHATWG HTML5 named-
   character-reference table via the `entities` crate, filtered to semicolon-terminated forms
   only (XML has no HTML4 quirks mode), used as the fallback/default entity set.
3. **Combined recursive resolution with cycle/depth guards** (`resolve.rs`,
   `EntityResolver`): document-declared entities take precedence over the standard table;
   resolution is recursive with a bounded cycle/depth guard (`MAX_RESOLUTION_DEPTH = 64` plus
   a seen-set); unknown or external names resolve to an explicit non-error `Resolution`
   variant so callers raw-preserve rather than drop them.

`xml-entities` has no rescribe dependency: it's a first-class standalone library per
CLAUDE.md's `-fmt`-crate philosophy, usable by any Rust project doing XML/SGML-ish entity
work, not just rescribe's three XML verticals.

**Do not switch the underlying XML parser** away from quick-xml to obtain internal-subset
discovery natively. This alternative was fully evaluated against CLAUDE.md's five-API bar
("If a library cannot support all five APIs at full performance, we cannot use that library")
using both source-level capability checks and an independent benchmark (see Alternatives) —
both `xml-rs` and `roxmltree` fail that bar, one structurally and one on measured performance.

## Consequences

- Closes the ceiling flagged across all three XML verticals: named entities now resolve to
  real text/characters, not just raw-preserve.
- Added one new small crate to the workspace (`xml-entities`, dependency: `entities` only),
  wired into `docbook-fmt`, `jats-fmt`, `tei-fmt`.
- The underlying XML parser for all three crates remains `quick-xml`, at full zero-copy,
  chunked-`StreamingParser` performance (`docbook-fmt::batch::StreamingParser`,
  `crates/formats/docbook-fmt/src/batch.rs`, is a genuine O(token + depth) chunked
  implementation on top of it today, using `Err(Syntax(_))`-on-truncation as the "need more
  bytes" signal).
- `xml-entities` duplicates none of quick-xml's own entity machinery: quick-xml's
  `unescape_with` hook is unused by any of the three format crates, which dispatch
  `Event::GeneralRef` manually instead (see Context). The only thing `xml-entities` does that
  quick-xml categorically cannot is DOCTYPE-internal-subset discovery.
- **Future option, not a defect to fix**: quick-xml's own maintainer has stated a
  `reader::Config`-level entity map with transparent expansion is "the plan for further
  improvements" (unshipped, no committed timeline as of this writing). If and when that lands,
  `xml-entities` could shrink to a pure table + resolution layer (dropping `decl.rs`) with zero
  performance cost and no long-term duplication. Contributing internal-subset discovery
  upstream to quick-xml directly (rather than waiting) would serve the wider Rust ecosystem
  per CLAUDE.md's priority hierarchy (level 2, "Rust ecosystem, any consumer" beats level 4,
  rescribe's own adapters) and is logged as open work in `TODO.md`, not committed to here.

## Alternatives considered

- **Depend on one of the unmaintained DTD crates (`dtd`/`dtd-parser`) instead of building**:
  rejected — both are alpha-stage, last published years ago, with no adoption evidence; using
  them would trade a small build effort for an unmaintained dependency doing the same narrow
  job worse.
- **Skip DTD-declared custom entities, resolve only the standard ISO/HTML5 table**: rejected —
  would leave format-specific custom entity declarations (which DocBook/TEI/JATS documents
  routinely declare) unresolved; only raw-preservation (the pre-existing behavior) would remain
  for those, not actual resolution.
- **Wait for quick-xml issue #258's discovery half to ship upstream instead of building now**:
  rejected as the primary path — the resolution half of #258 shipped years ago
  (`unescape_with`, PR #261), but the DOCTYPE-discovery half is explicitly "the plan for
  further improvements" with no committed timeline (Mingun, on the same thread). Nothing
  prevents contributing it upstream later (see Consequences); it isn't a plan to depend on
  now.
- **Switch the underlying XML parser to `xml-rs` or `roxmltree`** to get internal-entity
  discovery for free: evaluated on its merits and rejected.
  - `roxmltree` is disqualified on capability, not speed: `Document::parse(&str)` requires the
    entire document as one already-allocated string; there is no `Read`-based or chunked
    entry point, and the crate's own documentation states it does not support streaming.
    Building rescribe's `StreamingParser` on it would require the "buffer everything, then
    wrap" anti-pattern CLAUDE.md bans by name — it fails the five-API bar structurally, before
    performance is even a question.
  - `xml-rs` is a real contender on capability (pull-iterator over `io::Read`) but loses on two
    independently measured, non-negotiable axes. Zero-copy: `xml-rs` allocates an owned
    `String` per token by design (its own README frames this as a usability tradeoff, not an
    oversight) — every text/attribute/name quick-xml currently borrows as `Cow::Borrowed`
    would become a fresh heap allocation. Throughput: an out-of-repo benchmark (quick-xml
    0.39.4 vs. `xml` 0.8.28 vs. `roxmltree` 0.20.0, current crates.io versions, synthetic
    DocBook-shaped documents up to ~5 MB, release build, best-of-3) measured quick-xml at
    884–1088 MB/s against `xml-rs` at 46–51 MB/s — a 19–21x gap; quick-xml's own published
    README benchmark shows roughly 50x on its own input. Whether `xml-rs`'s error model draws
    the same clean "truncated, wait for more bytes" vs. "genuinely malformed" line that
    `docbook-fmt::batch::StreamingParser` relies on today is unverified and would need
    prototyping before it could be trusted for chunked feeding at all — an additional, unproven
    requirement on top of the measured performance and zero-copy losses.
  - Coupling cost, independent of the above: `quick_xml::` is referenced directly across
    ~2000+ lines of non-test code in `docbook-fmt`/`jats-fmt`/`tei-fmt`
    (`parse.rs`/`events.rs`/`batch.rs`/`emit.rs`/`writer.rs`), plus 15+ other workspace crates
    (`ooxml-*`, `fb2-fmt`, `odf-fmt`, `rescribe-read/write-opml`,
    `rescribe-read/write-endnotexml`). A parser swap is a rewrite of every XML vertical's
    reader/writer internals, not a dependency bump.
  - Maintenance status favors neither option: all three libraries were actively maintained as
    of this writing (quick-xml, `xml-rs`, and `roxmltree` all had commits within the prior
    two months).
  - **Net**: `roxmltree` fails outright on the five-API bar; `xml-rs` fails on two measured,
    non-negotiable axes (throughput, zero-copy) even before its unverified chunked-feed
    viability is considered. Neither is a viable substitute for quick-xml as the base parser,
    so building `xml-entities` as a thin, parser-agnostic add-on over quick-xml was the correct
    call — not merely the first one reached.
