//! Fixture integration tests.
//!
//! Discovers fixtures from `fixtures/{format}/` at the workspace root and
//! runs each against the appropriate reader.  Tests skip gracefully if a
//! format directory doesn't exist yet.

use rescribe_fixtures::run_format_fixtures;
use std::path::PathBuf;

fn fixtures_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = crates/rescribe-fixtures
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap() // crates/
        .parent()
        .unwrap() // workspace root
        .join("fixtures")
}

#[test]
fn markdown() {
    run_format_fixtures(&fixtures_root(), "markdown", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_markdown::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn html() {
    run_format_fixtures(&fixtures_root(), "html", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_html::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn asciidoc() {
    run_format_fixtures(&fixtures_root(), "asciidoc", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_asciidoc::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn mediawiki() {
    run_format_fixtures(&fixtures_root(), "mediawiki", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_mediawiki::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn latex() {
    run_format_fixtures(&fixtures_root(), "latex", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_latex::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn rst() {
    run_format_fixtures(&fixtures_root(), "rst", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_rst::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn org() {
    run_format_fixtures(&fixtures_root(), "org", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_org::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn creole() {
    run_format_fixtures(&fixtures_root(), "creole", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_creole::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn djot() {
    run_format_fixtures(&fixtures_root(), "djot", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_djot::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn textile() {
    run_format_fixtures(&fixtures_root(), "textile", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_textile::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn muse() {
    run_format_fixtures(&fixtures_root(), "muse", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_muse::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn t2t() {
    run_format_fixtures(&fixtures_root(), "t2t", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_t2t::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn tikiwiki() {
    run_format_fixtures(&fixtures_root(), "tikiwiki", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_tikiwiki::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn twiki() {
    run_format_fixtures(&fixtures_root(), "twiki", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_twiki::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn vimwiki() {
    run_format_fixtures(&fixtures_root(), "vimwiki", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_vimwiki::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn dokuwiki() {
    run_format_fixtures(&fixtures_root(), "dokuwiki", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_dokuwiki::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn jira() {
    run_format_fixtures(&fixtures_root(), "jira", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_jira::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn haddock() {
    run_format_fixtures(&fixtures_root(), "haddock", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_haddock::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn pod() {
    run_format_fixtures(&fixtures_root(), "pod", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_pod::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn man() {
    run_format_fixtures(&fixtures_root(), "man", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_man::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}
