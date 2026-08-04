# Changelog

All notable changes to `commonmark-fmt` are documented here.

## Unreleased

### Changed (breaking)

- `events::Event` gained a new variant, `ListTightnessResolved { tight: bool }`. Any code
  matching on `Event` exhaustively (without a wildcard arm) needs a new match arm.

  **Why:** CommonMark list tightness ("is any item blank-line-separated from its neighbor, or
  does any item directly contain two block-level children separated by a blank line") is a
  property of the *entire* list, uniform across every item — pulldown-cmark itself only wraps
  item content in real `Paragraph` tags when the whole list is loose, never per item.
  Determining it can require seeing every item, which isn't bounded by a constant lookahead, so
  `EventIter` cannot always know it in time for `StartList`.

  `EventIter` still emits `StartList { tight: true, .. }` optimistically (the common case: most
  lists are tight). If the list turns out loose, it now emits `ListTightnessResolved { tight:
  false }` exactly once, immediately before the matching `EndList`, to correct it — the one
  point in the stream where the answer is always known, without buffering the whole list. It is
  never emitted for a genuinely tight list.

  `writer::Writer` was updated to act on the correction: it retroactively splices in the
  blank-line separators it skipped under the (now-corrected) optimistic assumption — both
  between items and between two blank-line-separated blocks inside the same item — so its
  output stays byte-identical to `emit::emit()` regardless of exactly where in the list the
  correction fires.

  See `events::Event::ListTightnessResolved`'s doc comment and `writer::Writer::write_event`'s
  handling of it for the full design.
