//! Cross-API fixture harness: exercises `events()`, `StreamingParser<H>`,
//! and the streaming writer directly against `{format}-fmt` crates — not
//! just the rescribe adapter's `parse()`/`emit()`, which is all
//! `tests/run.rs` ever drove. See `fixtures/spec.md` ("Cross-API harness")
//! and `crates/rescribe-fixtures/src/streaming_harness.rs` for the
//! equivalence definitions and the capability/known-failure mechanisms this
//! file instantiates.
//!
//! Scope (see the task report for the honest accounting): rst-fmt gets full
//! real checks for all three non-parse/emit APIs. html-fmt gets a real
//! streaming-writer byte-identity check plus a chunk-buffering-integrity
//! check, with its two reader APIs declared `NotApplicable` against the
//! crate's own documentation that html5ever/HTML5 tree construction makes
//! independent streaming readers impossible. ooxml-wml/pml/sml get
//! real `events()` checks (wml/pml fail against a newly-found bug, tracked
//! as a `KnownFailure`; sml passes) and, for sml only, a real
//! streaming-writer fidelity check. Every other format gets an explicit
//! `NotYetWired` capability declaration in `streaming_harness::CAPABILITIES`
//! / `NOT_YET_AUDITED` rather than silently not appearing anywhere.
//!
//! This file is deliberately a bare list of `mod` declarations. The actual
//! test logic lives one-file-per-format under `tests/streaming_apis/`
//! (`common.rs` holds the shared helpers: `fixtures_root`, `find_input`,
//! `assert_streaming_parser_is_incremental`, and the
//! `every_run_rs_format_has_a_capability_entry` meta test). This split
//! replaced a single ~11.7k-line monolith that concurrent per-format
//! migration agents kept colliding on — every format now has its own file,
//! so a per-format edit only ever touches that format's file plus this
//! trivial mod list.
//!
//! Keep the list below alphabetized and append-only to minimize merge
//! friction across concurrent per-format edits.

// A test-binary crate root (this file) resolves `mod foo;` to
// `tests/foo.rs`, not `tests/streaming_apis/foo.rs` — the module-name-as-
// directory convention only applies to non-root modules. `#[path]` is
// required here to actually nest these under `tests/streaming_apis/`.
#[path = "streaming_apis/common.rs"]
mod common;

#[path = "streaming_apis/ansi.rs"]
mod ansi;
#[path = "streaming_apis/asciidoc.rs"]
mod asciidoc;
#[path = "streaming_apis/bbcode.rs"]
mod bbcode;
#[path = "streaming_apis/commonmark.rs"]
mod commonmark;
#[path = "streaming_apis/creole.rs"]
mod creole;
#[path = "streaming_apis/djot.rs"]
mod djot;
#[path = "streaming_apis/docbook.rs"]
mod docbook;
#[path = "streaming_apis/dokuwiki.rs"]
mod dokuwiki;
#[path = "streaming_apis/endnotexml.rs"]
mod endnotexml;
#[path = "streaming_apis/fb2.rs"]
mod fb2;
#[path = "streaming_apis/fountain.rs"]
mod fountain;
#[path = "streaming_apis/haddock.rs"]
mod haddock;
#[path = "streaming_apis/html.rs"]
mod html;
#[path = "streaming_apis/jats.rs"]
mod jats;
#[path = "streaming_apis/jira.rs"]
mod jira;
#[path = "streaming_apis/man.rs"]
mod man;
#[path = "streaming_apis/markua.rs"]
mod markua;
#[path = "streaming_apis/mediawiki.rs"]
mod mediawiki;
#[path = "streaming_apis/muse.rs"]
mod muse;
#[path = "streaming_apis/odf.rs"]
mod odf;
#[path = "streaming_apis/ooxml_pml.rs"]
mod ooxml_pml;
#[path = "streaming_apis/ooxml_sml.rs"]
mod ooxml_sml;
#[path = "streaming_apis/ooxml_wml.rs"]
mod ooxml_wml;
#[path = "streaming_apis/opml.rs"]
mod opml;
#[path = "streaming_apis/org.rs"]
mod org;
#[path = "streaming_apis/pod.rs"]
mod pod;
#[path = "streaming_apis/rst.rs"]
mod rst;
#[path = "streaming_apis/rtf.rs"]
mod rtf;
#[path = "streaming_apis/t2t.rs"]
mod t2t;
#[path = "streaming_apis/tei.rs"]
mod tei;
#[path = "streaming_apis/texinfo.rs"]
mod texinfo;
#[path = "streaming_apis/textile.rs"]
mod textile;
#[path = "streaming_apis/tikiwiki.rs"]
mod tikiwiki;
#[path = "streaming_apis/twiki.rs"]
mod twiki;
#[path = "streaming_apis/vimwiki.rs"]
mod vimwiki;
#[path = "streaming_apis/xwiki.rs"]
mod xwiki;
#[path = "streaming_apis/zimwiki.rs"]
mod zimwiki;
