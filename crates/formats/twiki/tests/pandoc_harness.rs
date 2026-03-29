//! Pandoc oracle harness and no-panic smoke test for TWiki.

const SAMPLE: &str = r#"---+ Main Title

This is a paragraph with *bold* and _italic_ text.

---++ Section Two

   * Item one
   * Item two
   * Item three

| *Name* | *Age* |
| Alice | 30 |
| Bob | 25 |

<verbatim>
fn main() {}
</verbatim>

---

Visit [[https://example.com][Example Site]].
"#;

/// Ensure parsing arbitrary-ish content does not panic.
#[test]
fn parse_sample_no_panic() {
    let (doc, _diags) = twiki::parse(SAMPLE);
    assert!(!doc.blocks.is_empty(), "parsed doc should have blocks");
    // Roundtrip
    let emitted = twiki::build(&doc);
    let (doc2, _) = twiki::parse(&emitted);
    assert_eq!(doc.blocks.len(), doc2.blocks.len(), "roundtrip block count mismatch");
}

/// Pandoc oracle test — compare our parse output against pandoc's.
/// Requires `pandoc` on PATH; skipped in CI via `#[ignore]`.
#[test]
#[ignore]
fn pandoc_oracle() {
    use std::process::Command;

    // Write sample to a temp file
    let tmp = std::env::temp_dir().join("twiki_oracle_input.twiki");
    std::fs::write(&tmp, SAMPLE).expect("write tmp");

    let output = Command::new("pandoc")
        .args(["-f", "twiki", "-t", "json"])
        .arg(&tmp)
        .output()
        .expect("pandoc should be available");

    assert!(
        output.status.success(),
        "pandoc failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // We don't do structural comparison yet — just confirm pandoc can parse it.
    let json = String::from_utf8_lossy(&output.stdout);
    assert!(json.contains("\"blocks\""), "pandoc JSON should contain blocks");

    let _ = std::fs::remove_file(&tmp);
}
