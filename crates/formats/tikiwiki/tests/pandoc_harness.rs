//! Pandoc cannot read TikiWiki, so there is no oracle comparison available.
//! This harness verifies that `parse` does not panic on representative samples.

use tikiwiki::parse;

const SAMPLES: &[&str] = &[
    "",
    "! Heading 1",
    "!! Heading 2\n!!! Heading 3",
    "Just a paragraph.",
    "__bold__ ''italic'' ===underline=== --strike-- -+code+-",
    "^super^ ,,sub,,",
    "[https://example.com|Link]",
    "((WikiWord))",
    "{img src=photo.png alt=\"Alt text\"}",
    "~np~raw __text__~/np~",
    "* item 1\n* item 2\n** nested",
    "# one\n# two\n# three",
    "||A|B||\n||C|D||",
    "||^H1^|^H2^||\n||D1|D2||",
    "{CODE(lang=rust)}\nfn main() {}\n{CODE}",
    "{QUOTE()}\nQuoted text.\n{QUOTE}",
    "---",
    "\n\n\n",
    "!",
    "__unclosed bold",
    "''unclosed italic",
    "{CODE()\nunclosed code block",
    "||unclosed|table",
    "[unclosed link",
    "((unclosed wikilink",
    "~np~unclosed nowiki",
    "^unclosed super",
    ",,unclosed sub",
    "--unclosed strike",
    "{img src=}",
];

#[test]
fn parse_sample_no_panic() {
    for (i, sample) in SAMPLES.iter().enumerate() {
        let result = std::panic::catch_unwind(|| parse(sample));
        assert!(result.is_ok(), "Sample {} panicked: {:?}", i, &sample[..sample.len().min(80)]);
    }

    // Stress inputs (runtime-generated)
    let stars = "*".repeat(1000);
    let bangs = "!".repeat(100);
    let underscores = "__".repeat(500);
    for stress in [stars.as_str(), bangs.as_str(), underscores.as_str()] {
        let result = std::panic::catch_unwind(|| parse(stress));
        assert!(result.is_ok(), "Stress sample panicked");
    }
}
