//! Local Pandoc corpus harness.
//!
//! These tests are `#[ignore]` by default — they require both:
//!   - `~/git/pandoc/test/` (GPL corpus, never committed to the repo)
//!   - `pandoc` binary on PATH (add to dev shell via `nix develop`)
//!
//! Run with:
//!   cargo test -p rescribe-fixtures -- --ignored --nocapture
//!
//! Tests report but do NOT fail on low text coverage — the goal is to
//! catalogue deficiencies so they can be fixed incrementally.  Tests DO
//! fail on panics (crashes are bugs).

use rescribe_core::Document;
use rescribe_fixtures::pandoc_harness::{
    self, CorpusEntry, RunResult, corpus_dir, find_pandoc, run_entry,
};

fn run_formats(
    entries: &[CorpusEntry],
    parse: impl Fn(&str, &[u8]) -> Result<Document, String> + Copy + Send + 'static,
) {
    let Some(corpus) = corpus_dir() else {
        eprintln!("SKIP: ~/git/pandoc/test/ not found");
        return;
    };
    let pandoc = find_pandoc();
    if pandoc.is_none() {
        eprintln!("NOTE: pandoc not found on PATH — coverage comparison disabled");
    }

    let results: Vec<RunResult> = entries
        .iter()
        .map(|e| {
            eprintln!("testing {}/{}", e.format, e.filename);
            let fmt = e.format;
            run_entry(e, &corpus, pandoc.as_deref(), move |bytes| {
                parse(fmt, bytes)
            })
        })
        .collect();

    pandoc_harness::print_report(&results, pandoc.is_some());

    // Fail only on parse panics (already propagated) or outright parse errors
    // that aren't "file not found" (those are just missing corpus files).
    for r in &results {
        if !r.parse_ok
            && let Some(e) = &r.parse_error
            && !e.starts_with("cannot read")
        {
            panic!("FAIL {}/{}: parse error: {e}", r.format, r.filename);
        }
    }
}

fn parse_format(format: &str, input: &[u8]) -> Result<Document, String> {
    let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
    match format {
        "markdown" => rescribe_read_markdown::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "rst" => rescribe_read_rst::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "html" => rescribe_read_html::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "latex" => rescribe_read_latex::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "org" => rescribe_read_org::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "djot" => rescribe_read_djot::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "mediawiki" => rescribe_read_mediawiki::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "creole" => rescribe_read_creole::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "textile" => rescribe_read_textile::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "haddock" => rescribe_read_haddock::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "jira" => rescribe_read_jira::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "tikiwiki" => rescribe_read_tikiwiki::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "twiki" => rescribe_read_twiki::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "vimwiki" => rescribe_read_vimwiki::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "t2t" => rescribe_read_t2t::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "pod" => rescribe_read_pod::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "man" => rescribe_read_man::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "asciidoc" => rescribe_read_asciidoc::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "typst" => rescribe_read_typst::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "docbook" => rescribe_read_docbook::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "jats" => rescribe_read_jats::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "dokuwiki" => rescribe_read_dokuwiki::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "muse" => rescribe_read_muse::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "fb2" => rescribe_read_fb2::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        _ => Err(format!("no reader registered for {format:?}")),
    }
}

/// Run all corpus entries in a single test for a concise report.
#[test]
#[ignore]
fn all_formats() {
    run_formats(pandoc_harness::CORPUS, |fmt, bytes| {
        parse_format(fmt, bytes)
    });
}
