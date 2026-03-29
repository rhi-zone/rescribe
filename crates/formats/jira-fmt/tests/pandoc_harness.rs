//! Pandoc oracle harness for jira-fmt.
//!
//! Pandoc does not support reading Jira wiki markup, so the oracle test is
//! skipped. This file contains a no-panic CI test that exercises the parser
//! on a representative sample.

use jira_fmt::parse;

const SAMPLE: &str = r#"
h1. Document Title

This is a paragraph with *bold*, _italic_, +underline+, -strikethrough-,
^superscript^, ~subscript~, and {{monospace}} text.

h2. Links and Images

[Example|https://example.com]

!photo.png|thumbnail!

h2. Lists

* Unordered item 1
* Unordered item 2
** Nested item

# Ordered item 1
# Ordered item 2

h2. Code

{code:java}
public class Main {
    public static void main(String[] args) {
        System.out.println("hello");
    }
}
{code}

h2. Quote

{quote}
This is a blockquote.
{quote}

h2. Panel

{panel:title=Note}
Panel content.
{panel}

h2. Table

||Header 1||Header 2||
|Cell 1|Cell 2|
|Cell 3|Cell 4|

----

h2. Other

{noformat}
Preformatted text.
{noformat}

{color:red}Colored text{color}

Hello @username!
"#;

#[test]
fn parse_sample_no_panic() {
    let (doc, _diags) = parse(SAMPLE);
    assert!(!doc.blocks.is_empty(), "parsed document should have blocks");
}

#[test]
fn roundtrip_sample() {
    let (doc, _) = parse(SAMPLE);
    let emitted = jira_fmt::build(&doc);
    let (doc2, _) = parse(&emitted);
    assert_eq!(
        doc.blocks.len(),
        doc2.blocks.len(),
        "roundtrip should preserve block count"
    );
}
