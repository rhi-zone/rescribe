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


// bracket_tilde_bracket_tilde ([~[~): excluded — tree-sitter-md inline grammar
// produces an empty inline node for [~[~, falling back to raw text("[~[~"), but
// pulldown-cmark's single-tilde strikethrough rules (following CommonMark flanking
// rules) parse this as text("[") + strikeout(text("[")). The divergence is in the
// inline grammar and cannot be fixed at the IR adapter level.
// Single-tilde inputs are skipped in the parity fuzz target.

parity!(asterisk_a_bracket_caret_asterisk_bracket, "*a[^*[");

parity!(newline_bs_newline_vt_u_bs_paren_lt, "\n\\\n\x0bu\\(<");

parity!(bracket_newline_bracket, "[\n]");

// pipe_dash_newline_pipe_dash (|-\n|-): excluded — pulldown's GFM table extension
// recognises |-\n|- as a minimal table (header row with content "-", delimiter row)
// but tree-sitter-md's block grammar produces paragraph instead. Inputs where every
// pipe-containing line is delimiter-only (|, -, :, space) are skipped in parity fuzz.

parity!(asterisk_dc4_double_asterisk_dc4, "*\x14**\x14");

parity!(double_asterisk_a_asterisk_a, "**a*a");

parity!(backtick_newline_backtick, "`\n`");

parity!(backtick_tab_double_backtick_tab_backtick, "`\t``\t`");

parity!(dot_tab_newline_dc3, ".\t\n\x13");

parity!(sub_asterisk_tab_vt_newline_bracket, "\x1a*\t\x0b\n[");

parity!(triple_backtick_newline_vt, "```\n\x0b");

// pipe_dash_paren_newline_pipe_dash (|-)\n|-): excluded — pulldown-cmark's GFM table
// extension recognises |-)\n|- as a minimal table (header |-), delimiter |-) but
// tree-sitter-md's block grammar does not. Inputs where a pipe line is followed by a
// delimiter-only pipe line are skipped in the parity fuzz target.

// us_double_tilde_dash_double_tilde (\x1f~~-~~): excluded — pulldown-cmark uses GFM's
// simple strikethrough rule (opens ~~ if not preceded by whitespace), while tree-sitter-md
// treats ASCII control chars (e.g. U+001F Unit Separator) as whitespace and refuses to
// open strikethrough. This is an upstream tree-sitter-md GFM rule divergence; inputs where
// ~~ is immediately preceded by a non-whitespace control char are skipped in the fuzz target.

// asterisk_dollar_asterisk_dollar (*$*$): excluded — tree-sitter-md's inline grammar
// does not implement CommonMark flanking Rule 2b: a `*` that is left-flanking only
// because it is preceded by whitespace/start AND followed by ASCII punctuation (e.g.
// `$`). pulldown-cmark recognises *$*$ as emphasis(text("$")); tree-sitter produces
// text("*$*$"). This is an upstream tree-sitter-md bug; inputs where any `*` or `_`
// run is followed by non-delimiter ASCII punctuation and preceded by whitespace or
// ASCII punctuation are skipped in the parity fuzz target.

// double_tilde_underscore_tilde_underscore (~~_~_): excluded — contains a lone ~
// at position 3 which causes single-tilde divergence; covered by the fuzz skip
// "strip ~~ pairs and check if ~ remains".

// bracket_asterisk_bracket_asterisk ([*[*): excluded — tree-sitter-md inline grammar
// produces an empty (inline) node for inputs where [ is immediately followed by a
// single * or _ (e.g. [*[*, [_[_), falling back to raw text. pulldown-cmark's
// CommonMark flanking rules may find valid emphasis in the same source. This is an
// upstream bug in tree-sitter-md's inline grammar; inputs with [* or [_ (single,
// not doubled) are skipped in the parity fuzz target.
