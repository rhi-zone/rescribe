# OPML Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

**Coverage-completeness caveat (2026-07-28):** the checklist below is a hand-curated list of
constructs, not yet verified against a spec-derived, machine-readable construct index. An
audit of `fixtures/docbook/COVERAGE.md` and `fixtures/jats/COVERAGE.md` against authoritative
element indexes found hundreds of element names enumerated nowhere, moving denominators
mid-session purely from incidentally-noticed gaps -- a ratio over a hand-written list like this
one is not a coverage measurement. See `docs/format-audit.md`'s "Construct Coverage (CC)"
section for the full evidence; this format's `CC` status there is `U` (unverified) until a
construct registry (in design, see `docs/adr/`) checks this list against the format's own
spec.

OPML (Outline Processor Markup Language) version 2.0 is defined at
http://dev.opml.org/spec2.html. An OPML document has a `<head>` with metadata and a
`<body>` containing a tree of `<outline>` elements.

## Document structure

- [x] basic flat outline (multiple top-level items) — `basic`
- [x] single item — `single-item`
- [x] empty body — `adv-empty`
- [x] minimal (no head) — `adv-minimal`
- [x] nested outline (parent with children) — `nested`
- [x] OPML root `version` attribute preserved (1.0 vs 2.0) — `opml-1-0-version`

## Head / metadata elements

- [x] title — `metadata`
- [x] dateCreated — `head-date-fields`
- [x] dateModified — `head-date-fields`
- [x] ownerName — `head-owner-and-window`
- [x] ownerEmail — `head-owner-and-window`
- [x] ownerId — `head-owner-and-window`
- [x] docs — `head-owner-and-window`
- [x] expansionState — `head-owner-and-window`
- [x] vertScrollState — `head-owner-and-window`
- [x] windowTop / windowLeft / windowBottom / windowRight — `head-owner-and-window`
- [x] unrecognized head child element (future/extension field) — `head-unknown-elements`

## Outline element attributes

- [x] text (required) — `basic`
- [x] xmlUrl — `with-url`
- [x] htmlUrl — `rare-two-url-attrs`
- [x] type — `type-rss`
- [x] type="rss" — `type-rss`
- [x] type="atom" — `type-atom`
- [x] type="link" — `type-link`
- [ ] type="include" (reference to external OPML) — (missing; no cross-document resolution
      is modeled or expected — this crate treats it as an ordinary outline with a `type`
      attribute, same as any other `type` value)
- [x] isComment — `is-comment`
- [x] isBreakpoint — `is-breakpoint`
- [x] created — `created-and-category`
- [x] category — `created-and-category`
- [ ] description — (missing; not in the OPML 2.0 core attribute set, would fall through
      the generic `opml:attr:*` raw-preservation path like any unknown attribute — covered
      in spirit by `unknown-outline-attribute`, not by a dedicated fixture)
- [ ] language — (missing; same generic-raw-preservation note as `description` above)
- [x] title (different from text) — `outline-all-attributes`
- [ ] version (e.g. "RSS2" for an RSS outline) — (missing; same generic-raw-preservation
      note as `description` above)
- [x] url (general URL, distinct from xmlUrl) — `type-link`
- [x] outline with no text attribute (spec-required but omittable) — `adv-outline-no-text`
- [x] unknown/application-specific/namespace-prefixed attribute — `unknown-outline-attribute`

## Nesting

- [x] two-level nesting (parent + children) — `nested`
- [x] three-level nesting — `three-level-nesting`
- [x] sibling items at the same level — `basic`, `nested`
- [x] mixed flat and nested items interleaved — `mixed-flat-and-nested`

## Special outline types

- [x] subscription list (xmlUrl present) — `with-url`
- [x] item with both xmlUrl and htmlUrl — `rare-two-url-attrs`
- [x] type="link" outline — `type-link`
- [ ] type="include" outline (reference to external OPML) — (missing; see the "Outline
      element attributes" note above)
- [x] outline with isComment="true" — `is-comment`

## Composition (integration)

- [x] head metadata + nested outline + url items — `head-owner-and-window` (metadata) +
      `nested`/`mixed-flat-and-nested` (structure) + `with-url` (url); no single fixture
      currently exercises all three at once
- [x] mixed flat and nested items — `mixed-flat-and-nested`
- [x] outline with all attributes — `outline-all-attributes`

## Adversarial

- [x] empty body — `adv-empty`
- [x] no head element — `adv-minimal`
- [x] outline with no text attribute — `adv-outline-no-text`
- [x] malformed XML (unclosed tag) — `adv-malformed-xml`
- [x] OPML 1.0 version attribute — `opml-1-0-version`
- [x] unknown attributes on outline — `unknown-outline-attribute`
- [x] unknown elements in head — `head-unknown-elements`

## Pathological

- [x] 1000 top-level items — `path-many-top-items`
- [x] 100-level deep nesting — `path-deep-nesting`
- [ ] item with very long text attribute — (missing)
