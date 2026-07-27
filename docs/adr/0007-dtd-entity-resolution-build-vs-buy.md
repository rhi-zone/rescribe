# 7. DTD-aware entity resolution: buy the entity table, build the DTD subset parser

## Status

Accepted (design decision only; **not yet implemented** — 2026-07-27 research pass, this
session). Tracked in `TODO.md` under "DTD-aware entity resolution."

## Context

`docbook-fmt`, `jats-fmt`, and `tei-fmt` currently raw-preserve any named XML entity beyond the
5 XML-predefined ones (`lt`/`gt`/`amp`/`apos`/`quot`) and numeric character references
losslessly as `Node::EntityRef` / IR `raw_inline` with a format-namespaced property
(`docbook:entity`/`jats:entity`/`tei:entity`) — never silently dropped, per CLAUDE.md's
raw-preservation rule. But they are never *resolved* to their actual replacement text/character,
because DocBook/JATS/TEI documents can declare custom entities in a DTD internal subset
(`<!ENTITY name "replacement">`) and/or reference standard ISO 8879 / ISO/IEC 9573 entity sets
(ISOlat1, ISOnum, ISOpub, ISOtech, ISOgrk, mmlalias, etc.) that quick-xml has no support for
resolving.

Before deciding to build this, a research pass checked whether a production-grade Rust crate
already covers it (checked crates.io, docs.rs, and GitHub issue trackers, not general
knowledge):

- **quick-xml** does not parse `<!ENTITY>` declarations in the internal/external subset at all.
  GitHub issue #258 ("Declared entity is not recognized") has been open since Feb 2021, tagged
  `enhancement`/`help wanted`, with no maintainer commitment.
- **xml-rs** (actively maintained, last release Oct 2025) gives no indication of DTD
  ENTITY-declaration parsing either; DTD validation is documented as unsupported.
- **roxmltree**'s `EntityResolver` and `xml_dom`'s `EntityResolver` trait are callback hooks for
  resolving entities during tree-building, not bundled DTD parsers — they still need something
  upstream to have discovered the declarations in the first place.
- **`dtd`** (0.1.0, last published 2019) and **`dtd-parser`** (0.1.0-alpha3, last published
  2021) both exist but are early-stage/unmaintained, parse DTD grammar into a data structure at
  best, and provide no entity-resolution layer, no bundled ISO/HTML5 tables, and no adoption
  evidence.
- On the other half: **`entities`** (~6.9M downloads) and **`html-escape`** (~33.9M downloads,
  actively maintained) both bundle the WHATWG HTML5 named-character-reference table, which
  overlaps heavily with ISOnum/ISOlat1/ISOamsa/etc. (HTML5's table absorbed most of MathML's
  ISO sets) but is not identical — DocBook/TEI/JATS DTDs also declare their own custom,
  format-specific entities beyond the ISO sets, which no generic table covers. No crate
  packages the ISO 8879/9573 `.ent` files directly as a Rust table (`entity_table` on crates.io
  is an unrelated ECS crate, a false-positive name collision).

So the gap splits cleanly into two halves, and only one has a maintained buy option.

## Decision

**Buy** the standard entity-table half: depend on `html-escape` or `entities` for the
WHATWG/HTML5 named-character-reference table as the fallback/default entity set. **Build** a
thin DTD internal-subset parser — scoped narrowly to just discovering `<!ENTITY name
"replacement">` declarations, not a full DTD validator — since nothing maintained exists for
that half. Layer per-document DTD-declared entities over the bought default table (document
declarations take precedence; the bought table is the fallback for entities the document didn't
locally redeclare).

This is consistent with rescribe's "no path dependencies, contribute upstream when possible"
posture: building a small crate here isn't a judgment call to avoid outside dependencies for
their own sake, it's the only option, because the narrow-but-real gap (DTD entity-declaration
discovery) has no maintained crate to buy.

## Consequences (once implemented — currently design-only)

- Closes the one remaining ceiling flagged across all three XML verticals' `TODO.md` entries:
  named entities currently raw-preserve losslessly but never resolve to real text/characters.
- Adds one new small supporting crate (a DTD internal-subset entity parser) to the workspace,
  scoped to declaration-parsing only — explicitly not a general DTD/schema validator, to keep
  the build side as narrow as the actual gap.
- This ADR documents the decision, not the implementation — TODO.md should be updated (and this
  status line changed to plain "Accepted") once the crate exists and is wired into
  docbook-fmt/jats-fmt/tei-fmt.

## Alternatives considered

- **Depend on one of the unmaintained DTD crates (`dtd`/`dtd-parser`) instead of building**:
  rejected — both are alpha-stage, last published years ago, with no adoption evidence; using
  them would trade a small build effort for an unmaintained dependency doing the same narrow
  job worse.
- **Skip DTD-declared custom entities, resolve only the standard ISO/HTML5 table**: rejected —
  would still leave format-specific custom entity declarations (which DocBook/TEI/JATS documents
  routinely declare) unresolved, which is exactly the gap the standard table doesn't cover; only
  raw-preservation (the pre-existing behavior) would remain for those, not actual resolution.
- **Wait for quick-xml issue #258 to be implemented upstream**: rejected as the primary path —
  open since Feb 2021 with no maintainer commitment; nothing prevents contributing upstream
  later, but it isn't a plan to depend on now.
