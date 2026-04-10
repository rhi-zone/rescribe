//! Fixture tests for the HTML reader.

use rescribe_fixtures::run_format_fixtures;
use std::path::PathBuf;

fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap() // crates/readers/
        .parent()
        .unwrap() // crates/
        .parent()
        .unwrap() // workspace root
        .join("fixtures")
}

#[test]
fn html_fixtures() {
    run_format_fixtures(&fixtures_root(), "html", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_html::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}
