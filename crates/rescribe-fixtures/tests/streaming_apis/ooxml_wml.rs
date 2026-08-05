//! Streaming-API cross-checks for ooxml_wml. Split out of the former monolithic
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
// ooxml-wml (docx) events(): Text-drop / End-tag reversal — fixed.
//
// `read_props` scans ahead of a container (`<w:p>`, `<w:r>`, …) looking for
// its optional props element (`<w:pPr>`, `<w:rPr>`, …). When a child
// container starts before the props element does (the common case: no
// `<w:pPr>` before `<w:r>`), the scan recurses into that child via `open()`.
// Two bugs compounded here: `open()` pushed a container's own stack frame
// only *after* `build_start_event` returned, so a child opened during the
// parent's props scan pushed its frame *before* the parent's — inverting
// `End` tag pop order; and `queue()` overwrote the pending-event slot
// instead of prepending, so a `Text` event queued by a deeply nested scan
// (e.g. `<w:p><w:r><w:t>`) was clobbered by the parent's own queued event.
// Fixed by pushing the frame before scanning (`open()`) and prepending
// instead of overwriting (`queue()`) in `ooxml-wml/src/events.rs`.
// ---------------------------------------------------------------------------

/// Minimal, realistic WML fragment: a paragraph containing a single run with
/// no `<w:pPr>` before the run — the most common shape in real DOCX body
/// content. Wrapped in `<w:document><w:body>…</w:body></w:document>` since
/// `events()` takes the raw `word/document.xml` content.
const WML_SIMPLE_PARAGRAPH: &[u8] = br#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body><w:p><w:r><w:t>Hello world</w:t></w:r></w:p></w:body>
</w:document>"#;

#[test]
fn wml_events_reaches_and_correctly_orders_paragraph_text() {
    let events: Vec<_> = ooxml_wml::events::events(WML_SIMPLE_PARAGRAPH).collect();

    let has_text = events
        .iter()
        .any(|e| matches!(e, ooxml_wml::WmlEvent::Text(t) if t.contains("Hello world")));

    let well_nested = {
        // A minimal well-nestedness check: EndRun must come before EndParagraph
        // (the run opened after, and inside, the paragraph).
        let end_run_idx = events
            .iter()
            .position(|e| matches!(e, ooxml_wml::WmlEvent::EndRun));
        let end_para_idx = events
            .iter()
            .position(|e| matches!(e, ooxml_wml::WmlEvent::EndParagraph));
        matches!((end_run_idx, end_para_idx), (Some(r), Some(p)) if r < p)
    };

    let result = if has_text && well_nested {
        Ok(())
    } else {
        Err(format!(
            "expected a Text(\"Hello world\") event and EndRun before EndParagraph; got {events:?}"
        ))
    };
    assert_or_known_failure("docx", "events", result);
}
