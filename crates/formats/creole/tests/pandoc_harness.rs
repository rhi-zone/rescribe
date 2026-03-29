//! Pandoc oracle harness for the Creole parser.
//!
//! The `#[ignore]` tests require Pandoc to be installed and the pandoc test
//! directory at `~/git/pandoc/test/`.

use creole::parse;

/// Smoke test: parse a representative Creole sample without panicking.
#[test]
fn parse_sample_no_panic() {
    let sample = r#"
= Heading 1

== Heading 2

=== Heading 3

A paragraph with **bold** and //italic// and {{{code}}} text.

See [[https://example.com|a link]] and {{image.png|alt text}}.

* Unordered one
* Unordered two
** Nested

# Ordered one
# Ordered two

|= Header 1 |= Header 2 |
| Cell A     | Cell B     |

{{{
preformatted block
with multiple lines
}}}

----

> A blockquote line
> Second line

; Term
: Definition

Line one\\Line two

~**escaped bold~**
"#;
    let (doc, diags) = parse(sample);
    assert!(!doc.blocks.is_empty(), "parsed document should not be empty");
    assert!(diags.is_empty(), "no diagnostics expected for valid input");
}

/// Oracle test: parse the pandoc creole-reader test file.
#[test]
#[ignore]
fn pandoc_creole_reader() {
    let home = std::env::var("HOME").expect("HOME not set");
    let path = std::path::PathBuf::from(home).join("git/pandoc/test/creole-reader.txt");

    if !path.exists() {
        eprintln!("Skipping: {:?} not found", path);
        return;
    }

    let content = std::fs::read_to_string(&path).expect("failed to read creole-reader.txt");
    let (doc, _diags) = parse(&content);
    assert!(
        !doc.blocks.is_empty(),
        "pandoc creole-reader.txt should produce a non-empty document"
    );
}
