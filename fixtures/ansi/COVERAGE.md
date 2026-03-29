# ANSI Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

## SGR (Select Graphic Rendition) — text styling

- [x] bold (SGR 1) — `bold`
- [x] italic (SGR 3) — `italic`
- [x] underline (SGR 4) — `underline`
- [x] strikethrough (SGR 9) — `rare-strikeout`
- [x] dim / faint (SGR 2) — `dim`
- [x] blink (SGR 5) — `blink`
- [x] reverse video (SGR 7) — `reverse`
- [x] hidden / invisible (SGR 8) — `hidden`
- [x] double underline (SGR 21) — `double-underline`
- [x] overline (SGR 53) — `overline`
- [x] reset / SGR 0 — `reset`
- [x] bold + italic combined (SGR 1 + 3) — `rare-bold-italic`

## SGR — foreground color

- [x] standard foreground colors (SGR 30–37) — `fg-standard`
- [x] default foreground (SGR 39) — `fg-default`
- [x] bright foreground colors (SGR 90–97) — `fg-bright`
- [x] 256-color foreground (SGR 38;5;n) — `fg-256`
- [x] truecolor foreground (SGR 38;2;r;g;b) — `fg-truecolor`

## SGR — background color

- [x] standard background colors (SGR 40–47) — `bg-standard`
- [x] default background (SGR 49) — `bg-default`
- [x] bright background colors (SGR 100–107) — `bg-bright`
- [x] 256-color background (SGR 48;5;n) — `bg-256`
- [x] truecolor background (SGR 48;2;r;g;b) — `bg-truecolor`

## Text structure

- [x] plain paragraph — `paragraph`
- [x] inline markup interleaved with plain text — `rare-inline-in-text`
- [x] multiple paragraphs separated by newlines — `multi-paragraph`
- [x] line break (explicit newline mid-paragraph) — `line-break`

## Cursor movement (CSI sequences)

- [x] cursor up (CSI A) — `cursor-up`
- [x] cursor down (CSI B) — `cursor-down`
- [x] cursor forward (CSI C) — `cursor-forward`
- [x] cursor back (CSI D) — `cursor-back`
- [x] cursor position (CSI H / CSI f) — `cursor-position`
- [x] erase in display (CSI J) — `erase-display`
- [x] erase in line (CSI K) — `erase-line`

## Other escape sequences

- [x] OSC hyperlink (OSC 8) — `hyperlink`
- [x] hyperlink with URI and text — `rare-hyperlink-uri`

## Composition (integration)

- [x] bold + color on same span — `bold-color`
- [x] nested SGR resets correctly — `nested-sgr-reset`
- [x] styled text inside a paragraph alongside plain text — `styled-in-paragraph`

## Adversarial

- [x] empty document — `adv-empty`
- [x] incomplete escape sequence (truncated CSI) — `adv-truncated-csi`
- [x] unknown SGR parameter — `adv-unknown-sgr`
- [x] bare ESC without bracket — `adv-bare-esc`
- [x] ESC followed by end-of-input — `adv-esc-eof`

## Pathological

- [x] very long line (>64 KB) with no escape sequences — `path-long-line`
- [x] deeply nested SGR resets (many opens before reset) — `path-deep-sgr`
- [x] thousands of color changes on one line — `path-many-colors`
