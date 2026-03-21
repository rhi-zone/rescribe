# PPTX Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

## Slide structure
- [x] single slide with text — `slide`
- [ ] multi-slide presentation — (missing)
- [ ] slide with layout/master inheritance — (missing)
- [ ] slide with slide number placeholder — (missing)
- [ ] hidden slide (`show="0"`) — (missing)

## Text content (shapes / placeholders)
- [x] title placeholder — `slide`
- [ ] subtitle placeholder — (missing)
- [x] body text / paragraph — `slide`
- [x] heading within slide — `inline-bold` (partially)
- [x] bold text — `inline-bold`
- [ ] italic text — (missing)
- [ ] underline — (missing)
- [ ] strikeout — (missing)
- [ ] subscript / superscript — (missing)
- [ ] font color — (missing)
- [ ] font size — (missing)
- [ ] font name — (missing)
- [ ] highlight — (missing)
- [ ] hyperlink in text run — (missing)
- [ ] line break within paragraph (`<a:br>`) — (missing)

## Paragraph properties
- [ ] paragraph alignment (left/center/right/justify) — (missing)
- [ ] paragraph indent / margin — (missing)
- [ ] paragraph spacing (before/after) — (missing)
- [ ] line spacing — (missing)

## Bullet / list constructs
- [x] bulleted list (character bullet) — `bullets`
- [ ] numbered list (auto-numbered) — (missing)
- [ ] multi-level bullet list — (missing)
- [ ] custom bullet character — (missing)
- [ ] bullet with image/picture — (missing)

## Tables
- [x] basic table — `table`
- [ ] table with header row — (missing)
- [ ] table with colspan/rowspan — (missing)
- [ ] table with cell formatting — (missing)
- [ ] table with borders — (missing)

## Speaker notes
- [x] speaker notes — `notes`
- [ ] notes with inline formatting — (missing)
- [ ] notes with multiple paragraphs — (missing)

## Images / media
- [ ] inline image (`<p:pic>`) — (missing)
- [ ] image with alt text — (missing)
- [ ] embedded video — (missing)
- [ ] linked media — (missing)
- [ ] background image — (missing)

## Shapes / drawing
- [ ] text box (`<p:sp>` non-placeholder) — (missing)
- [ ] grouped shapes (`<p:grpSp>`) — (missing)
- [ ] connectors / lines — (missing)
- [ ] SmartArt — (missing)
- [ ] chart — (missing)

## Slide transitions / animations
- [ ] slide transition (`<p:transition>`) — (missing)
- [ ] animation effect (`<p:timing>`) — (missing)

## Presentation metadata
- [ ] presentation title — (missing)
- [ ] author / last-modified-by — (missing)
- [ ] slide dimensions — (missing)
- [ ] slide layout name — (missing)
- [ ] theme name and colors — (missing)

## Composition (integration)
- [ ] slide with title, bullet list, and image — (missing)
- [ ] table with formatted cell content — (missing)
- [ ] bullet list with hyperlinks — (missing)
- [ ] notes with bullet list — (missing)
- [ ] slide with multiple text boxes — (missing)

## Adversarial
- [ ] malformed zip archive — (missing)
- [ ] missing ppt/presentation.xml — (missing)
- [ ] slide with no content — (missing)
- [ ] corrupt relationship file — (missing)
- [ ] unknown namespace in XML — (missing)
- [ ] empty presentation (zero slides) — (missing)
- [ ] corrupt image binary in media/ — (missing)

## Pathological
- [ ] presentation with hundreds of slides — (missing)
- [ ] slide with thousands of text runs — (missing)
- [ ] deeply nested grouped shapes — (missing)
- [ ] very large embedded image — (missing)
- [ ] table with hundreds of rows — (missing)
