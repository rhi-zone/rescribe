//! Streaming-API cross-checks for odf. Split out of the former monolithic
//! `streaming_apis.rs` (see `crates/rescribe-fixtures/tests/streaming_apis.rs`
//! for the harness overview and `common.rs` for shared helpers) so concurrent
//! per-format edits stop colliding on one file.

#[allow(unused_imports)]
use crate::common::{assert_streaming_parser_is_incremental, find_input, fixtures_root};
#[allow(unused_imports)]
use rescribe_fixtures::streaming_harness::{
    CAPABILITIES, NOT_YET_AUDITED, ObservableSink, adversarial_chunkings, assert_or_known_failure,
};
#[allow(unused_imports)]
use rescribe_format_api::{Emit, Events, Handler, Parse, StreamingParse, StreamingWrite};
#[allow(unused_imports)]
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// odf-fmt (odt/ods/odp): events() is genuinely independent of parse() (a
// direct quick_xml scan of content.xml — see
// crates/formats/odf-fmt/src/events.rs's `parse_content_events`, not a walk
// over `parse()`'s `OdfDocument`), correcting a prior assessment that called
// it a parse()-derived fake. It is, however, eagerly and fully buffered —
// `EventIter::new` calls `extract_events` synchronously and queues every
// event before the first `next()` call, self-documented in events.rs's
// module doc ("Events are pre-buffered... For large files consider... a
// future StreamingParser") — so it is not memory-bounded the way
// docbook/jats/tei's `EventIter` is, even though it is a real independent
// implementation. No `StreamingParser<H>` type exists in the crate at all
// (batch.rs only has `BatchParser`, which is a legitimate buffer-until-
// finish AST builder, and `Writer`); the module doc calls a true chunked
// streaming parser "future" work. Both left `NotYetWired` below rather than
// writing a same-format ast_to_events projection under time pressure — ODF's
// `OdfEvent` enum spans three sibling document-body shapes (text/spreadsheet/
// presentation) and a faithful hand-written projection is a substantial
// undertaking; see TODO.md.
//
// The streaming Writer *is* checkable now: `write_event()` incrementally
// builds an `OdfDocument` via `DocBuilder::process` (real per-event work,
// the same sanctioned shape as ooxml-sml's `SmlWriter` above), only
// deferring ZIP byte serialization to `finish()` (inherent to ZIP's
// central-directory-at-end layout, documented in batch.rs). This check used
// to find a real, distinct defect: `OdfEvent` had no variant carrying the
// document's `mimetype`, `meta` (title/author/date), `styles.xml`, or
// embedded image bytes — `events()` only covered document *body* content
// (paragraphs, tables, slides, etc.), so `DocBuilder`'s reconstructed
// `OdfDocument` always had `mimetype: ""` (never set anywhere in batch.rs)
// and default/empty `meta`/`styles`/`images`, while `parse()`'s AST carried
// all of them from the ZIP's other parts — the same defect class as
// org-fmt's missing-metadata-variant gap: an `Event` enum expressiveness
// gap, not a logic bug. Fixed by adding `OdfEvent::Mimetype`, `Meta`,
// `AutomaticStyle`, `NamedStyle`, `ListStyle`, `PageLayout`, and
// `EmbeddedImage` variants (the last named to avoid colliding with the
// pre-existing inline `Image { href }` body-content event), produced by
// `events::extract_events` reading `mimetype`/`meta.xml`/`styles.xml`/
// `content.xml`'s `<office:automatic-styles>`/`Pictures`+`media` via the
// same free-function helpers `parser::parse` uses (`read_zip_text`,
// `parse_meta_xml`, `parse_styles_xml`, `parse_auto_styles_block`, now
// `pub(crate)`), and consumed by `batch::DocBuilder::process`. Fixing that
// exposed two directly-adjacent bugs blocking its own verification —
// `StartFrame` had no `width`/`height` (fixture adv-corrupt-image) and
// self-closing `<office:text/>` wasn't recognized by `events.rs`'s own scan
// (fixture adv-empty) — both fixed too. Fixing that surfaced a much larger,
// pre-existing, and unrelated set of `OdfEvent` gaps for inline/block body
// content — 12 of 66 odt fixtures. Those are now fixed too (2026-08-03), and
// the check passes on all 66. Diagnosing them one by one (diffing the
// `content.xml` build() emits against the one the streaming Writer emits)
// found *seven* distinct root causes, not the single "vocabulary gap" bucket
// the old note implied, and one item in that bucket was simply wrong:
//
//  1. Note internals were unmodelled. `OdfEvent` had `StartNote`/`EndNote`
//     but nothing for `<text:note-citation>` or `<text:note-body>`, so a
//     note's marker and its entire body were dropped — `DocBuilder` had a
//     `NoteBody` frame that nothing ever pushed (`#[allow(dead_code)]`).
//     Added `NoteCitation`, `StartNoteBody`, `EndNoteBody`.
//     (footnote, footnote-formatted, endnote)
//  2. Empty-element inline leaves had no variants at all: `<text:soft-hyphen/>`,
//     `<text:soft-page-break/>`, `<text:bookmark/>`/`<text:bookmark-start/>`.
//     Added `SoftHyphen`, `SoftPageBreak`, `Bookmark`. **The "heading-only
//     divergence" nobody had diagnosed is this same bug, not a heading bug**:
//     fixtures/odt/heading's `<text:h>` simply contains a `<text:bookmark/>`.
//     (bookmark, heading, soft-hyphen)
//  3. `<office:annotation>` produced an `Unknown` event whose *children* were
//     then scanned normally, so the annotation's `<dc:creator>` text leaked
//     into the enclosing paragraph as body text while the comment itself was
//     lost — corruption, not just loss. Added `Annotation`, flattened via
//     `parser::read_annotation_text` exactly as `parse()` does. (annotation)
//  4. `StartCell` carried neither `table:number-columns-spanned` nor
//     `table:number-rows-spanned` nor the typed value, and self-closing
//     `<table:covered-table-cell/>` was only recognized in a *spreadsheet*
//     body, never a text body — so an entire covered cell vanished and the
//     row shapes drifted apart. (colspan-rowspan)
//  5. A block-level `<draw:frame>` was unreachable: `DocBuilder` routed
//     `EndFrame` through `push_inline` only, which matches nothing when the
//     open frame is `<office:text>` / a cell / a list item, so the whole frame
//     was dropped. Now routed to `TextBlock::Frame` outside inline context.
//     `<draw:text-box>` also had no text-body handling at all. (text-box,
//     image-caption)
//  6. Character/entity references arrive as their own `quick_xml`
//     `Event::GeneralRef`, never as part of the surrounding `Event::Text`.
//     `parser.rs` has an arm for them; `events.rs` did not, so `&#160;` was
//     silently dropped. (non-breaking-space)
//  7. `OdfEvent::Unknown` carried only a name, no raw payload, so
//     raw-preservation was impossible through the event path. It now carries
//     the verbatim XML and consumes its subtree, matching `parse()`. This
//     needed events.rs to track the enclosing content model, because the same
//     element name means different things in each: a `<table:table>` under
//     `<office:text>` is a table, while one nested inside a `<text:p>` is
//     something `parser::parse_inlines` does not model at all and captures
//     raw. (path-deeply-nested-table)
//
// Two genuine losslessness gaps found during this work are *not* streaming
// defects and remain open on the `parse()` side, where both APIs lose
// equally: (a) `FrameContent` is an either/or enum and `parse()` gives
// `<draw:image>` priority, so a `<draw:frame>` holding both an image and a
// `<draw:text-box>` caption loses the caption (fixtures/odt/image-caption's
// "Figure 1: A photo." is absent from build()'s own output, so the streaming
// Writer had to reproduce the same loss to be byte-identical); (b) field
// elements are recognized only in their `<text:date>…</text:date>` form —
// the self-closing form is dropped by both paths. See TODO.md.
#[test]
fn odf_streaming_writer_byte_identical_to_builder_over_all_fixtures() {
    let root = fixtures_root().join("odt");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/odt dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let Ok(parsed) = odf_fmt::parser::parse(&input) else {
            continue;
        };
        let Ok(built) = odf_fmt::writer::emit(&parsed.value) else {
            continue;
        };

        let mut w = odf_fmt::batch::Writer::new(Vec::<u8>::new());
        for e in odf_fmt::events(&input) {
            w.write_event(e);
        }
        let Ok(streamed) = w.finish() else {
            if result.is_ok() {
                result = Err(format!(
                    "streaming Writer::finish failed for fixture {name}"
                ));
            }
            checked += 1;
            continue;
        };

        if built != streamed && result.is_ok() {
            result = Err(format!(
                "streaming Writer diverged from build() for fixture {name} (built {} bytes, \
                 streamed {} bytes)",
                built.len(),
                streamed.len()
            ));
        }
        checked += 1;
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of odt fixtures, got {checked}"
    );
    assert_or_known_failure("odt", "streaming_writer", result);
}
