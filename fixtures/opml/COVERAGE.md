# OPML Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

OPML (Outline Processor Markup Language) version 2.0 is defined at
http://dev.opml.org/spec2.html. An OPML document has a `<head>` with metadata and a
`<body>` containing a tree of `<outline>` elements.

## Document structure

- [x] basic flat outline (multiple top-level items) — `basic`
- [x] single item — `single-item`
- [x] empty body — `adv-empty`
- [x] minimal (no head) — `adv-minimal`
- [x] nested outline (parent with children) — `nested`

## Head / metadata elements

- [x] title — `metadata`
- [ ] dateCreated — (missing)
- [ ] dateModified — (missing)
- [ ] ownerName — (missing)
- [ ] ownerEmail — (missing)
- [ ] ownerId — (missing)
- [ ] docs — (missing)
- [ ] expansionState — (missing)
- [ ] vertScrollState — (missing)
- [ ] windowTop / windowLeft / windowBottom / windowRight — (missing)

## Outline element attributes

- [x] text (required) — `basic`
- [x] xmlUrl — `with-url`
- [x] htmlUrl — `rare-two-url-attrs`
- [ ] type — (missing; common values: "rss", "atom", "link", "include")
- [ ] type="rss" — (missing)
- [ ] type="atom" — (missing)
- [ ] type="link" — (missing)
- [ ] type="include" — (missing)
- [ ] isComment — (missing)
- [ ] isBreakpoint — (missing)
- [ ] created — (missing)
- [ ] category — (missing)
- [ ] description — (missing)
- [ ] language — (missing)
- [ ] title (different from text) — (missing)
- [ ] version — (missing; e.g., "RSS2" for RSS outline)
- [ ] url — (missing; general URL, distinct from xmlUrl)

## Nesting

- [x] two-level nesting (parent + children) — `nested`
- [ ] three-level nesting — (missing)
- [ ] sibling items at the same level — `nested` (partially; has siblings after nested group)

## Special outline types

- [x] subscription list (xmlUrl present) — `with-url`
- [x] item with both xmlUrl and htmlUrl — `rare-two-url-attrs`
- [ ] type="link" outline — (missing)
- [ ] type="include" outline (reference to external OPML) — (missing)
- [ ] outline with isComment="true" — (missing)

## Composition (integration)

- [ ] head metadata + nested outline + url items — (missing)
- [ ] mixed flat and nested items — (missing)
- [ ] outline with all attributes — (missing)

## Adversarial

- [x] empty body — `adv-empty`
- [x] no head element — `adv-minimal`
- [ ] outline with no text attribute — (missing)
- [ ] malformed XML — (missing)
- [ ] OPML 1.0 version attribute — (missing)
- [ ] unknown attributes on outline — (missing)
- [ ] unknown elements in head — (missing)

## Pathological

- [ ] 1000 top-level items — (missing)
- [ ] 100-level deep nesting — (missing)
- [ ] item with very long text attribute — (missing)
