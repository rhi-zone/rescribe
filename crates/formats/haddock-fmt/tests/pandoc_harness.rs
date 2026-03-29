//! Pandoc oracle harness for Haddock.
//!
//! Checks whether Pandoc can read Haddock and, if so, provides an `#[ignore]`d
//! oracle test that can be enabled manually.

use std::process::Command;

fn pandoc_supports_haddock() -> bool {
    let Ok(output) = Command::new("pandoc").arg("--list-input-formats").output() else {
        return false;
    };
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines().any(|l| l.trim() == "haddock")
}

#[test]
fn pandoc_availability_check() {
    if pandoc_supports_haddock() {
        eprintln!("pandoc supports haddock input — oracle tests available (run with --ignored)");
    } else {
        eprintln!("pandoc does not support haddock or is not installed — oracle tests skipped");
    }
}

#[test]
#[ignore]
fn pandoc_oracle_heading() {
    use std::io::Write;
    use std::process::Stdio;

    if !pandoc_supports_haddock() {
        return;
    }
    let mut child = Command::new("pandoc")
        .args(["--from", "haddock", "--to", "json"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to spawn pandoc");

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(b"= Heading\n").expect("failed to write stdin");
    }

    let output = child.wait_with_output().expect("pandoc failed");
    let json = String::from_utf8_lossy(&output.stdout);
    // Just verify Pandoc produces some JSON output; detailed comparison is manual.
    assert!(
        json.contains("Header") || json.contains("Str"),
        "unexpected pandoc output: {json}"
    );
}

#[test]
fn parse_sample_no_panic() {
    // CI gate: parsing arbitrary samples must never panic.
    let samples: Vec<&str> = vec![
        "",
        "Hello",
        "= H",
        "== H",
        "=== H",
        "==== H",
        "__b__",
        "/i/",
        "@c@",
        "* x",
        "(1) x",
        "> code",
        "[t] d",
        ">>> expr",
        "@since 1.0",
        "@deprecated",
        "@param x d",
        "@returns d",
        "\"M\"",
        "<https://e.com>",
        "@\ncode\n@",
        "____",
        "//",
        "@@",
        "''",
        "'x'",
    ];
    for s in samples {
        let _ = haddock_fmt::parse(s);
    }
}
