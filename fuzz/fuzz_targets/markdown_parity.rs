#![no_main]

use libfuzzer_sys::fuzz_target;
use rescribe_core::ParseOptions;

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };

    // Skip inputs with null bytes: CommonMark replaces \x00 with U+FFFD, and
    // tree-sitter-md's emphasis detection diverges from CommonMark for text
    // adjacent to FFFD. Null-byte safety is covered by the no-panic target.
    if s.contains('\x00') {
        return;
    }

    // Skip inputs with single tildes: pulldown-cmark and tree-sitter-md implement
    // different single-tilde (~text~) strikethrough rules. Both agree on ~~text~~
    // (GFM standard); single-tilde behavior diverges because tree-sitter-md's
    // inline grammar produces empty nodes for some patterns (e.g. [~[~) while
    // pulldown's flanking-rule logic parses them differently.
    if s.contains('~') && !s.contains("~~") {
        return;
    }

    let opts = ParseOptions {
        preserve_source_info: false,
        ..Default::default()
    };

    let Ok(pd_result) = rescribe_read_markdown::backend_pulldown::parse_with_options(s, &opts)
    else {
        return;
    };
    let Ok(ts_result) = rescribe_read_markdown::backend_treesitter::parse_with_options(s, &opts)
    else {
        return;
    };

    let mut pd = pd_result.value.content;
    let mut ts = ts_result.value.content;
    pd.strip_spans();
    ts.strip_spans();

    assert_eq!(pd, ts, "backend parity failure for input: {:?}", s);
});
