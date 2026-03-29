//! Oracle harness for zimwiki.
//!
//! Pandoc does not support ZimWiki as an input format (`pandoc --list-input-formats`
//! does not include `zimwiki`), so there is no oracle comparison available.
//!
//! This file contains a smoke test that parses a representative sample document
//! without panicking.

/// Smoke test: parse the built-in representative ZimWiki sample without panicking.
/// Runs in normal CI (not `#[ignore]`).
#[test]
fn parse_sample_no_panic() {
    let sample = include_str!("../../../../fixtures/zimwiki/oracle/input.zimwiki");
    let (doc, _diags) = zimwiki::parse(sample);
    assert!(
        !doc.blocks.is_empty(),
        "expected at least one block from sample input"
    );
}
