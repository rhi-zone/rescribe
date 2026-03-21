# ANSI Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

## SGR (Select Graphic Rendition) — text styling

- [x] bold (SGR 1) — `bold`
- [x] italic (SGR 3) — `italic`
- [x] underline (SGR 4) — `underline`
- [x] strikethrough (SGR 9) — `rare-strikeout`
- [ ] dim / faint (SGR 2) — (missing)
- [ ] blink (SGR 5) — (missing)
- [ ] reverse video (SGR 7) — (missing)
- [ ] hidden / invisible (SGR 8) — (missing)
- [ ] double underline (SGR 21) — (missing)
- [ ] overline (SGR 53) — (missing)
- [ ] reset / SGR 0 — (missing)
- [x] bold + italic combined (SGR 1 + 3) — `rare-bold-italic`

## SGR — foreground color

- [ ] standard foreground colors (SGR 30–37) — (missing)
- [ ] default foreground (SGR 39) — (missing)
- [ ] bright foreground colors (SGR 90–97) — (missing)
- [ ] 256-color foreground (SGR 38;5;n) — (missing)
- [ ] truecolor foreground (SGR 38;2;r;g;b) — (missing)

## SGR — background color

- [ ] standard background colors (SGR 40–47) — (missing)
- [ ] default background (SGR 49) — (missing)
- [ ] bright background colors (SGR 100–107) — (missing)
- [ ] 256-color background (SGR 48;5;n) — (missing)
- [ ] truecolor background (SGR 48;2;r;g;b) — (missing)

## Text structure

- [x] plain paragraph — `paragraph`
- [x] inline markup interleaved with plain text — `rare-inline-in-text`
- [ ] multiple paragraphs separated by newlines — (missing)
- [ ] line break (explicit newline mid-paragraph) — (missing)

## Cursor movement (CSI sequences)

- [ ] cursor up (CSI A) — (missing)
- [ ] cursor down (CSI B) — (missing)
- [ ] cursor forward (CSI C) — (missing)
- [ ] cursor back (CSI D) — (missing)
- [ ] cursor position (CSI H / CSI f) — (missing)
- [ ] erase in display (CSI J) — (missing)
- [ ] erase in line (CSI K) — (missing)

## Other escape sequences

- [ ] OSC hyperlink (OSC 8) — (missing)
- [ ] hyperlink with URI and text — (missing)

## Composition (integration)

- [ ] bold + color on same span — (missing)
- [ ] nested SGR resets correctly — (missing)
- [ ] styled text inside a paragraph alongside plain text — (missing)

## Adversarial

- [x] empty document — `adv-empty`
- [ ] incomplete escape sequence (truncated CSI) — (missing)
- [ ] unknown SGR parameter — (missing)
- [ ] bare ESC without bracket — (missing)
- [ ] ESC followed by end-of-input — (missing)

## Pathological

- [ ] very long line (>64 KB) with no escape sequences — (missing)
- [ ] deeply nested SGR resets (many opens before reset) — (missing)
- [ ] thousands of color changes on one line — (missing)
