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
