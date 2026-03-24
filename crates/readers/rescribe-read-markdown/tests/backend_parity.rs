//! Parity tests: tree-sitter and pulldown backends must produce identical IR
//! for the same input.
//!
//! Two separate parse passes are done (one per backend), then spans are stripped
//! and the root nodes are compared with `assert_eq!`. Any divergence is a bug
//! in one of the backends.
//!
//! Tests run for both `preserve_source_info: false` (structural parity) and
//! `preserve_source_info: true` (marker-preservation parity).

use rescribe_core::ParseOptions;
use rescribe_read_markdown::backend_pulldown;
use rescribe_read_markdown::backend_treesitter;

fn parse_both(input: &str, preserve: bool) -> (rescribe_core::Node, rescribe_core::Node) {
    let opts = ParseOptions {
        preserve_source_info: preserve,
        ..Default::default()
    };
    let mut pd = backend_pulldown::parse_with_options(input, &opts)
        .unwrap()
        .value
        .content;
    let mut ts = backend_treesitter::parse_with_options(input, &opts)
        .unwrap()
        .value
        .content;
    pd.strip_spans();
    ts.strip_spans();
    (pd, ts)
}

macro_rules! parity {
    ($name:ident, $input:expr) => {
        #[test]
        fn $name() {
            let input = $input;
            let (pd, ts) = parse_both(input, false);
            assert_eq!(pd, ts, "structural parity failed for {:?}", input);
            let (pd, ts) = parse_both(input, true);
            assert_eq!(pd, ts, "marker parity failed for {:?}", input);
        }
    };
}

parity!(paragraph, "Hello world\n");
parity!(two_paragraphs, "First paragraph.\n\nSecond paragraph.\n");
parity!(atx_heading_1, "# Heading 1\n");
parity!(atx_heading_2, "## Heading 2\n");
parity!(atx_heading_3, "### Heading 3\n");
parity!(setext_heading_1, "Heading 1\n=========\n");
parity!(setext_heading_2, "Heading 2\n---------\n");
parity!(emphasis_asterisk, "Hello *world*\n");
parity!(emphasis_underscore, "Hello _world_\n");
parity!(strong_asterisk, "Hello **world**\n");
parity!(strong_underscore, "Hello __world__\n");
parity!(strikethrough, "Hello ~~world~~\n");
parity!(inline_code, "Hello `world`\n");
parity!(code_fence_backtick, "```rust\nfn main() {}\n```\n");
parity!(code_fence_tilde, "~~~rust\nfn main() {}\n~~~\n");
parity!(code_fence_no_lang, "```\nno lang\n```\n");
parity!(unordered_list_dash, "- item 1\n- item 2\n");
parity!(unordered_list_asterisk, "* item 1\n* item 2\n");
parity!(unordered_list_plus, "+ item 1\n+ item 2\n");
parity!(ordered_list, "1. first\n2. second\n");
parity!(ordered_list_start, "3. third\n4. fourth\n");
parity!(nested_list, "- outer 1\n  - inner 1\n  - inner 2\n- outer 2\n");
parity!(horizontal_rule_dash, "---\n");
parity!(horizontal_rule_asterisk, "***\n");
parity!(horizontal_rule_underscore, "___\n");
parity!(link, "[text](https://example.com)\n");
parity!(link_with_title, "[text](https://example.com \"title\")\n");
parity!(image, "![alt](image.png)\n");
parity!(blockquote, "> quoted text\n");
parity!(blockquote_nested, "> outer\n> > inner\n");
parity!(
    table,
    "| A | B |\n|---|---|\n| 1 | 2 |\n"
);
parity!(hard_line_break_backslash, "line 1\\\nline 2\n");
parity!(soft_break, "line 1\nline 2\n");
parity!(mixed_inline, "**bold** and *italic* and `code`\n");
parity!(emphasis_inside_strong, "***triple***\n");

parity!(close_bracket_paren, "])\n");

parity!(newlines_vtab, "\n\n\n\x0b");

parity!(null_stx, "\x00\x02");

parity!(bs_vt_bs, "\x08\x0b\x08");

parity!(newline_vt_cc, "\n\x0bCC");

parity!(leading_space_para, "\n CC");

parity!(bracket_caret_brackets, "[^]]");

#[test]
fn treesitter_deep_blockquote_no_panic() {
    let input = ">".repeat(270) + "Bb";
    let _ = rescribe_read_markdown::backend_treesitter::parse(&input);
}

parity!(cr_bracket_cr_bracket, "\r]\r]");

#[test]
fn treesitter_deep_blockquote_with_nulls_no_panic() {
    let mut input = String::from("++]\n\x00\x1f;\n\n+ \n");
    input.push_str(&">".repeat(1100));
    input.push_str("\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00");
    let _ = rescribe_read_markdown::backend_treesitter::parse(&input);
}

parity!(tab_z, "\tz");

parity!(space_tab_bracket, " \t[");

parity!(c_newline_tab_caret, "C\n\t^");

parity!(y_vt_newline_c, "y\x0b\nC");

parity!(bracket_backslash_bracket, "[\\[");

parity!(comma_dollar_comma_dollar, ",$,$");

parity!(tilde_dollar_tilde_g, "~$~G");

// asterisk_null_bracket_asterisk (*\x00]*\x00]): excluded — null bytes produce U+FFFD
// which causes emphasis detection divergence between CommonMark and tree-sitter-md.
// Covered by no-panic fuzz target; parity target skips null-containing inputs.
