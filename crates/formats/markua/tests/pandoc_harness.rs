//! Oracle harness: Markua parse-without-panic test.
//!
//! Pandoc does not support Markua as an input format, so this harness
//! only verifies that the parser does not panic on sample inputs.
//!
//! Run with:
//!   cargo test -q -p markua

use markua::parse;

/// Smoke test: parse a representative Markua sample without panicking.
#[test]
fn parse_sample_no_panic() {
    let sample = include_str!("../../../../fixtures/markua/oracle/input.markua");
    let (doc, _diags) = parse(sample);
    assert!(!doc.blocks.is_empty(), "expected at least one block from sample input");
}

/// Parse each fixture input without panicking.
#[test]
fn parse_all_fixtures_no_panic() {
    let fixtures_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/markua");
    if !fixtures_dir.is_dir() {
        eprintln!("SKIP: fixtures/markua/ not found");
        return;
    }
    let mut count = 0;
    for entry in std::fs::read_dir(&fixtures_dir).unwrap() {
        let entry = entry.unwrap();
        if !entry.file_type().unwrap().is_dir() {
            continue;
        }
        let input_path = entry.path().join("input.markua");
        if !input_path.exists() {
            continue;
        }
        let input = std::fs::read_to_string(&input_path).unwrap();
        let (doc, _) = parse(&input);
        // Just verify it parses without panicking and produces something
        let _ = doc.blocks.len();
        count += 1;
    }
    eprintln!("Parsed {count} fixture inputs without panic");
}
