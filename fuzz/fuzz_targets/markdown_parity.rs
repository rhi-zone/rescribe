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

    // Skip inputs that contain a ~ not part of a ~~ pair: pulldown-cmark and
    // tree-sitter-md implement different single-tilde (~text~) strikethrough rules.
    // Both agree on ~~text~~ (GFM standard). Divergence arises whenever any ~
    // appears that isn't paired (e.g. [~[~, ~~_~_).
    // Detection: strip all ~~ pairs; if ~ still present, skip.
    if s.replace("~~", "").contains('~') {
        return;
    }

    // Skip inputs where [ is immediately followed by a single (non-doubled) * or _:
    // tree-sitter-md's inline grammar produces an empty (inline) node for these
    // patterns (e.g. [*[*, [_[_), causing divergence when pulldown's CommonMark
    // flanking rules find valid emphasis in the same source. This is an upstream
    // bug in tree-sitter-md's inline grammar that cannot be fixed at the adapter level.
    {
        let bytes = s.as_bytes();
        for i in 0..bytes.len().saturating_sub(1) {
            if bytes[i] == b'[' {
                let m = bytes[i + 1];
                if (m == b'*' || m == b'_') && bytes.get(i + 2) != Some(&m) {
                    return;
                }
            }
        }
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
