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
    // Binary formats — handle before UTF-8 conversion.
    if format == "odt" {
        return odf_fmt::rescribe::parse(input)
            .map(|r| r.value)
            .map_err(|e| e.to_string());
    }
    let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
    match format {
        "markdown" => rescribe_read_markdown::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "gfm" => rescribe_read_gfm::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "rst" => rst_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "html" => html_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "latex" => rescribe_read_latex::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "org" => org_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "djot" => djot_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "mediawiki" => mediawiki_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "creole" => creole::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "textile" => textile_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "haddock" => haddock_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "jira" => jira_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "tikiwiki" => tikiwiki::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "twiki" => twiki::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "vimwiki" => vimwiki_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "t2t" => t2t::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "pod" => pod_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "man" => man_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "asciidoc" => asciidoc::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "typst" => typst_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "docbook" => docbook_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "jats" => jats_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "dokuwiki" => dokuwiki::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "muse" => muse_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string()),
        "fb2" => fb2_fmt::rescribe::parse(s)
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
