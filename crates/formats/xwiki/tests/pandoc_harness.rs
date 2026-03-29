//! Pandoc oracle harness for XWiki.
//!
//! XWiki is not in pandoc's `--list-input-formats`, so there is no oracle test.
//! We keep a basic no-panic test for CI.

use xwiki::parse;

#[test]
fn parse_sample_no_panic() {
    let samples = [
        "",
        "= heading =",
        "== heading 2 ==",
        "=== heading 3 ===",
        "==== heading 4 ====",
        "===== heading 5 =====",
        "====== heading 6 ======",
        "**bold text**",
        "//italic text//",
        "__underline text__",
        "--strikethrough text--",
        "##monospace text##",
        "^^superscript^^",
        "~~subscript~~",
        "[[label>>http://example.com]]",
        "[[http://example.com]]",
        "[[image:photo.png]]",
        "[[image:photo.png||alt=\"A photo\" width=\"200\"]]",
        "* item 1\n* item 2\n* item 3",
        "1. first\n1. second\n1. third",
        "|=Header 1|=Header 2|\n|cell 1|cell 2|",
        "{{code language=\"java\"}}\npublic class Foo {}\n{{/code}}",
        "{{code}}\nplain code\n{{/code}}",
        "----",
        "{{quote}}\nquoted text\n{{/quote}}",
        "{{info}}\ninfo message\n{{/info}}",
        "{{warning}}\nwarning message\n{{/warning}}",
        "{{error}}\nerror message\n{{/error}}",
        "{{success}}\nsuccess message\n{{/success}}",
        "{{toc/}}",
        "{{velocity}}\n$variable\n{{/velocity}}",
        "line one\\\\ line two",
        // Adversarial inputs
        "**unclosed bold",
        "//unclosed italic",
        "__unclosed underline",
        "--unclosed strike",
        "##unclosed mono",
        "^^unclosed super",
        "~~unclosed sub",
        "[[unclosed link",
        "{{code}}\nunclosed code block",
        "{{info}}\nunclosed info",
        "{{unknown}}\ncontent\n{{/unknown}}",
        "{{unknown_self_closing/}}",
        // Pathological
        "* * * * * deeply nested",
        "||||||||||||||||||||",
        &"= ".repeat(100),
        &"**a** ".repeat(100),
    ];
    for sample in &samples {
        let _ = parse(sample);
    }
}
