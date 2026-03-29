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
- [x] forced action (! prefix) — `forced-action`
- [x] forced character name (@ prefix) — `forced-character`
- [x] forced transition (> prefix without trailing <) — `forced-transition`
- [x] dual dialogue (^ character) — `dual-dialogue`
- [x] boneyard / block comment (/* … */) — `boneyard`
- [x] title page (key: value block before first scene) — `title-page`

## Inline constructs
- [x] bold (**text**) — `bold`
- [x] italic (*text*) — `italic`
- [x] bold-italic (***text***) — `bold-italic`
- [x] underline (_text_) — `underline`
- [x] note inline ([[ text ]]) — `note-inline`
- [x] line break (explicit newline within action) — `line-break`

## Properties
- [x] title page key/value pairs (Title, Credit, Author, Source, Draft date, Contact) — `metadata`
- [x] scene heading location type (INT./EXT./INT./EXT./EST.) — `scene-heading-location`
- [x] scene heading time of day (DAY/NIGHT/etc.) — `scene-heading-time`
- [x] scene number (#number#) — `scene-number`
- [x] section heading level (# / ## / ### / #### / #####) — `section-level`
- [x] character extension (V.O., O.S., O.C., CONT'D) — `character-extension`
- [x] dual dialogue flag (^) — `dual-dialogue-flag`
- [x] synopsis text — `synopsis-text`

## Composition (integration)
- [x] multiple dialogues in sequence — `multi-dialogue`
- [x] action followed immediately by scene heading — `action-then-scene`
- [x] dual dialogue block — `dual-dialogue-block`
- [x] note inside action — `note-inside-action`
- [x] section hierarchy (h1 containing h2 sections) — `section-hierarchy`
- [x] inline formatting inside action — `inline-in-action`
- [x] inline formatting inside dialogue — `inline-in-dialogue`
- [x] title page followed by first scene — `title-then-scene`
- [x] parenthetical then more dialogue then another parenthetical — `paren-dialogue-paren`

## Adversarial
- [x] empty document — `adv-empty`
- [x] scene heading with no following action — `adv-scene-no-action`
- [x] dialogue with no character name above — `adv-dialogue-no-character`
- [x] unclosed boneyard comment — `adv-unclosed-boneyard`
- [x] note with missing closing bracket — `adv-unclosed-note`
- [x] title page key with no value — `adv-title-key-no-value`
- [x] transition not at end of line — `adv-transition-not-eol`

## Pathological
- [x] screenplay with hundreds of scenes — `path-hundreds-scenes`
- [x] very long action block — `path-long-action`
- [x] deeply nested sections — `path-deep-sections`
- [x] many characters in rapid dialogue interchange — `path-rapid-dialogue`
- [x] many notes — `path-many-notes`
