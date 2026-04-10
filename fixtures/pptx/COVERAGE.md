# PPTX Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

Items marked `[lib]` are not exposed by the upstream `ooxml-pml` library; they cannot be
tested via the builder API and are documented as library limitations.

## Slide structure
- [x] single slide with text — `slide`
- [x] multi-slide presentation — `multi-slide`
- [lib] slide with layout/master inheritance — PresentationBuilder does not expose layout/master XML
- [lib] slide with slide number placeholder — not in PresentationBuilder API
- [lib] hidden slide (`show="0"`) — not in PresentationBuilder API

## Text content (shapes / placeholders)
- [x] title placeholder — `slide`
- [lib] subtitle placeholder — PresentationBuilder does not produce subtitle placeholders
- [x] body text / paragraph — `slide`
- [x] bold text — `inline-bold`
- [x] italic text — `inline-italic`
- [x] underline — `inline-underline`
- [lib] strikeout — not in `TextRun` API
- [lib] subscript / superscript — not in `TextRun` API
- [lib] font color — not in `TextRun` API
- [x] font size — `font-size`
- [lib] font name — not in `TextRun` API
- [lib] highlight — not in `TextRun` API
- [x] hyperlink in text run — `hyperlink`
- [lib] line break within paragraph (`<a:br>`) — not in `TextRun` API

## Paragraph properties
- [lib] paragraph alignment (left/center/right/justify) — not in `Paragraph` API
- [lib] paragraph indent / margin — not in `Paragraph` API
- [lib] paragraph spacing (before/after) — not in `Paragraph` API
- [lib] line spacing — not in `Paragraph` API

## Bullet / list constructs
- [x] bulleted list (character bullet) — `bullets` (XML patching)
- [x] numbered list (auto-numbered) — `numbered-list` (XML patching)
- [lib] multi-level bullet list — XML patching required; not added
- [lib] custom bullet character — XML patching required; not added
- [lib] bullet with image/picture — not in PresentationBuilder API

## Tables
- [x] basic table — `table`
- [x] table with multiple data rows — `table-multiple-rows`
- [lib] table with header row — `TableBuilder` has no header-row concept
- [lib] table with colspan/rowspan — not in `TableBuilder` API
- [lib] table with cell formatting — not in `TableBuilder` API
- [lib] table with borders — not in `TableBuilder` API

## Speaker notes
- [x] speaker notes — `notes`
- [x] notes with multiple paragraphs — `notes-multi-para`
- [lib] notes with inline formatting — `set_notes` accepts plain text only

## Images / media
- [x] inline image (`<p:pic>`) — `image`
- [x] image with alt text — `image-alt-text`
- [lib] embedded video — not in PresentationBuilder API
- [lib] linked media — not in PresentationBuilder API
- [lib] background image — not in PresentationBuilder API

## Shapes / drawing
- [lib] text box (`<p:sp>` non-placeholder) — not in PresentationBuilder API
- [lib] grouped shapes (`<p:grpSp>`) — not in PresentationBuilder API
- [lib] connectors / lines — not in PresentationBuilder API
- [lib] SmartArt — not in PresentationBuilder API
- [lib] chart — not in PresentationBuilder API; fidelity warning emitted

## Slide transitions / animations
- [lib] slide transition (`<p:transition>`) — not in PresentationBuilder API
- [lib] animation effect (`<p:timing>`) — not in PresentationBuilder API

## Presentation metadata
- [lib] presentation title — not in PresentationBuilder API
- [lib] author / last-modified-by — not in PresentationBuilder API
- [lib] slide dimensions — not in PresentationBuilder API
- [lib] slide layout name — not in PresentationBuilder API
- [lib] theme name and colors — not in PresentationBuilder API

## Composition (integration)
- [x] slide with title and table — `slide-with-title-and-table`
- [x] multi-slide with speaker notes — `multi-slide-with-notes`
- [x] mixed inline formatting — `mixed-formatting`

## Adversarial
- [x] malformed zip archive — `adv-malformed-zip`
- [x] empty bytes — `adv-empty-bytes`
- [x] empty presentation (zero slides) — `adv-empty-presentation`
- [lib] missing ppt/presentation.xml — not constructible via PresentationBuilder
- [lib] corrupt relationship file — not constructible via PresentationBuilder
- [lib] unknown namespace in XML — not constructible via PresentationBuilder
- [lib] corrupt image binary in media/ — not constructible via PresentationBuilder

## Pathological
- [x] presentation with many slides — `path-many-slides` (20 slides)
- [x] slide with many text runs — `path-many-text-runs` (100 runs)
- [lib] deeply nested grouped shapes — not in PresentationBuilder API
