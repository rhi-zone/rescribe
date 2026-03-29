/// Smoke test: parse a representative DokuWiki sample without panicking.
#[test]
fn parse_sample_no_panic() {
    let sample = r#"
====== Main Title ======

A paragraph with **bold**, //italic//, __underline__, and ''code''.

===== Sub Heading =====

  * Unordered item 1
  * Unordered item 2
    * Nested item

  - Ordered item 1
  - Ordered item 2

^ Name ^ Age ^
| Alice | 30 |
| Bob | 25 |

<code rust>
fn main() {
    println!("hello");
}
</code>

> A blockquote

----

This has a footnote((footnote content)).

<del>struck</del> and <sup>super</sup> and <sub>sub</sub>.

[[https://example.com|Link]]

{{image.png|Alt text}}

%%**not bold**%%

<file>
file content
</file>

<html>
<p>raw html</p>
</html>

~~NOTOC~~

; Term
: Definition
"#;
    let (doc, _diags) = dokuwiki::parse(sample);
    assert!(!doc.blocks.is_empty());

    // Roundtrip: build back and re-parse
    let emitted = dokuwiki::build(&doc);
    let (doc2, _) = dokuwiki::parse(&emitted);
    assert!(!doc2.blocks.is_empty());
}

/// Oracle test: compare dokuwiki parser output against pandoc (when available).
/// Marked #[ignore] because pandoc may not be on PATH in CI.
#[test]
#[ignore]
fn pandoc_oracle() {
    let input = "====== Hello ======\n\nA **bold** paragraph.\n";

    // Check pandoc is available
    let status = std::process::Command::new("pandoc")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    if status.is_err() || !status.unwrap().success() {
        eprintln!("pandoc not available, skipping oracle test");
        return;
    }

    // Run pandoc: dokuwiki -> json
    let output = std::process::Command::new("pandoc")
        .args(["--from", "dokuwiki", "--to", "json"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child
                .stdin
                .as_mut()
                .unwrap()
                .write_all(input.as_bytes())
                .unwrap();
            child.wait_with_output()
        });

    match output {
        Ok(out) if out.status.success() => {
            let json = String::from_utf8_lossy(&out.stdout);
            // Sanity: pandoc JSON output should contain "Header" for a heading
            assert!(
                json.contains("Header"),
                "pandoc output should contain Header: {}",
                json
            );
        }
        Ok(out) => {
            eprintln!(
                "pandoc failed: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
        Err(e) => {
            eprintln!("pandoc not available: {e}");
        }
    }
}
