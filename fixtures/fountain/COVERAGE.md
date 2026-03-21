# Fountain Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

Fountain is a plain-text screenplay format. The reference is the Fountain spec
(https://fountain.io/syntax).

## Block constructs
- [x] scene heading (INT./EXT. or forced .) — `scene-heading`
- [x] action (plain paragraph) — `action`
- [x] dialogue — `dialogue`
- [x] parenthetical inside dialogue — `parenthetical-in-dialogue`
- [x] transition (TO: suffix or forced >) — `transition`
- [x] centered text (> text <) — `centered`
- [x] page break (===) — `page-break`
- [x] section heading (# / ## / ###) — `section-heading`
- [x] synopsis (= text) — `rare-synopsis`
- [x] note ([[ text ]]) — `note`
- [x] lyric (~ text) — `rare-lyric`
- [x] forced scene heading — `rare-forced-scene`
- [ ] forced action (! prefix) — (missing)
- [ ] forced character name (@ prefix) — (missing)
- [ ] forced transition (> prefix without trailing <) — (missing; transition covers this)
- [ ] dual dialogue (^ character) — (missing)
- [ ] boneyard / block comment (/* … */) — (missing)
- [ ] title page (key: value block before first scene) — `metadata`

## Inline constructs
- [ ] bold (**text**) — (missing)
- [ ] italic (*text*) — (missing)
- [ ] bold-italic (***text***) — (missing)
- [ ] underline (_text_) — (missing)
- [ ] note inline ([[ text ]]) — (missing; note block covered)
- [ ] line break (explicit newline within action) — (missing)

## Properties
- [x] title page key/value pairs (Title, Credit, Author, Source, Draft date, Contact) — `metadata`
- [ ] scene heading location type (INT./EXT./INT./EXT./EST.) — (missing)
- [ ] scene heading time of day (DAY/NIGHT/etc.) — (missing)
- [ ] scene number (#number#) — (missing)
- [ ] section heading level (# / ## / ### / #### / #####) — (missing)
- [ ] character extension (V.O., O.S., O.C., CONT'D) — (missing)
- [ ] dual dialogue flag (^) — (missing)
- [ ] synopsis text — (missing; rare-synopsis exists but properties unverified)

## Composition (integration)
- [ ] multiple dialogues in sequence — (missing)
- [ ] action followed immediately by scene heading — (missing)
- [ ] dual dialogue block — (missing)
- [ ] note inside action — (missing)
- [ ] section hierarchy (h1 containing h2 sections) — (missing)
- [ ] inline formatting inside action — (missing)
- [ ] inline formatting inside dialogue — (missing)
- [ ] title page followed by first scene — (missing)
- [ ] parenthetical then more dialogue then another parenthetical — (missing)

## Adversarial
- [x] empty document — `adv-empty`
- [ ] scene heading with no following action — (missing)
- [ ] dialogue with no character name above — (missing)
- [ ] unclosed boneyard comment — (missing)
- [ ] note with missing closing bracket — (missing)
- [ ] title page key with no value — (missing)
- [ ] transition not at end of line — (missing)

## Pathological
- [ ] screenplay with hundreds of scenes — (missing)
- [ ] very long action block — (missing)
- [ ] deeply nested sections — (missing)
- [ ] many characters in rapid dialogue interchange — (missing)
- [ ] many notes — (missing)
