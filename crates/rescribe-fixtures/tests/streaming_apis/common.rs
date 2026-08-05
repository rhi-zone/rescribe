//! Shared helpers for the per-format streaming-API cross-check modules under
//! `tests/streaming_apis/`. See `tests/streaming_apis.rs` for the harness
//! overview and the `mod` list of per-format files.

use rescribe_fixtures::streaming_harness::{CAPABILITIES, NOT_YET_AUDITED};
use std::path::{Path, PathBuf};

pub(crate) fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures")
}

/// Every format tested in `tests/run.rs` must appear either in
/// `streaming_harness::CAPABILITIES` (a real per-API declaration) or in
/// `NOT_YET_AUDITED` (an honest "nobody has looked yet" placeholder) — never
/// silently absent from both.
#[test]
fn every_run_rs_format_has_a_capability_entry() {
    // Kept in sync with the #[test] fns in tests/run.rs by hand; see the
    // comment at the end of this list if it drifts.
    const RUN_RS_FORMATS: &[&str] = &[
        "markdown",
        "commonmark",
        "gfm",
        "html",
        "asciidoc",
        "mediawiki",
        "latex",
        "rst",
        "org",
        "creole",
        "djot",
        "textile",
        "muse",
        "t2t",
        "tikiwiki",
        "twiki",
        "vimwiki",
        "dokuwiki",
        "jira",
        "haddock",
        "pod",
        "man",
        "xwiki",
        "zimwiki",
        "bbcode",
        "texinfo",
        "markua",
        "fountain",
        "ansi",
        "csl-json",
        "native",
        "pandoc-json",
        "docbook",
        "fb2",
        "ipynb",
        "csv",
        "tsv",
        "opml",
        "ris",
        "bibtex",
        "biblatex",
        "typst",
        "jats",
        "endnotexml",
        "tei",
        "docx",
        "odt",
        "epub",
        "pptx",
        "xlsx",
        "pdf",
        "rtf",
        "multimarkdown",
        // writer-only formats (also worth a declaration even though this
        // harness doesn't yet wire writer-side events()/StreamingParser
        // equivalents — there is no reader-side events() to check for them,
        // but a streaming-writer declaration still belongs here):
        "beamer",
        "revealjs",
        "slidy",
        "s5",
        "dzslides",
        "slideous",
        "context",
        "ms",
        "icml",
        "chunkedhtml",
    ];
    for fmt in RUN_RS_FORMATS {
        let declared =
            CAPABILITIES.iter().any(|c| &c.format == fmt) || NOT_YET_AUDITED.contains(fmt);
        assert!(
            declared,
            "format {fmt:?} is tested in tests/run.rs but has no entry in \
             streaming_harness::CAPABILITIES or NOT_YET_AUDITED — every format must have an \
             explicit, reviewable capability declaration, never silent absence"
        );
    }
}

pub(crate) fn find_input(dir: &Path) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| p.file_stem().is_some_and(|s| s == "input"))
}

/// Asserts that a format's `StreamingParser` delivers events incrementally as
/// input is fed, rather than buffering everything until `finish()`.
///
/// This probe is intentionally NOT fixture-driven. The original per-fixture
/// incrementality probe required partial event delivery at exactly the
/// 50%-byte offset of a fixture file. A block-granular parser (one that only
/// emits events at block boundaries, which is correct and expected behavior
/// for most formats) cannot satisfy an arbitrary 50%-byte split unless that
/// split happens to land on a block boundary. Because fixtures/spec.md's
/// "one focused construct per fixture" convention means most fixtures ARE a
/// single block, the 50%-byte-offset probe was structurally unable to pass
/// for the large majority of fixtures in some crates -- producing false
/// KnownFailure entries against implementations that were already correct.
/// Concretely: pod-fmt had 35 of its 36 fixtures structurally unable to pass
/// the old probe, and textile-fmt's `acronym` fixture failed because its
/// first logical line was 66 bytes into a 124-byte file (not at the 50%
/// mark). Do not "simplify" this back to a byte-offset split of a fixture --
/// that reintroduces exactly the false failures this helper was built to
/// eliminate. Instead, callers hand-build a synthetic sample per format with
/// a block-boundary-complete prefix and a block-boundary-incomplete tail.
///
/// Returns `Ok(())` when `delivered_something` is true, or an `Err`
/// carrying a message naming `format_name` otherwise. A `Result` (rather
/// than a direct `assert!`) is deliberate: most call sites are folding this
/// probe's outcome into a running `Result<(), String>` that ultimately goes
/// through [`assert_or_known_failure`], so a genuine regression here can
/// still be acknowledged via `KNOWN_FAILURES` instead of unconditionally
/// hard-panicking. Callers that want a bare panic (no known-failure
/// acknowledgement path) can do so with `.unwrap()` or an explicit
/// `if let Err(e) = ... { panic!("{e}") }`.
pub(crate) fn assert_streaming_parser_is_incremental(
    format_name: &str,
    delivered_something: bool,
) -> Result<(), String> {
    if delivered_something {
        Ok(())
    } else {
        Err(format!(
            "{format_name} StreamingParser delivered zero events to the handler after feed() \
             with a complete prefix (deliberately followed by unterminated/incomplete trailing \
             content) and before finish() was ever called — feed() must advance real \
             incremental parser state, not buffer input until finish()"
        ))
    }
}
