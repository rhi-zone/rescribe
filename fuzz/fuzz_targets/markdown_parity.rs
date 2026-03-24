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

    // Skip inputs where ~~ is immediately preceded by a non-whitespace ASCII control
    // character (U+0001..U+0008, U+000E..U+001F, U+007F): pulldown-cmark uses GFM's
    // simple strikethrough rule (not preceded by whitespace), while tree-sitter-md
    // treats these control chars as whitespace and does not open strikethrough.
    // E.g. \x1f~~-~~: pulldown → strikeout; tree-sitter → plain text.
    {
        let bytes = s.as_bytes();
        for i in 0..bytes.len().saturating_sub(1) {
            if bytes[i] == b'~' && bytes.get(i + 1) == Some(&b'~') {
                if let Some(&prev) = i.checked_sub(1).and_then(|j| bytes.get(j)) {
                    let is_ctrl_non_ws = matches!(prev, 0x01..=0x08 | 0x0e..=0x1f | 0x7f);
                    if is_ctrl_non_ws {
                        return;
                    }
                }
            }
        }
    }

    // Skip inputs where a `*` or `_` delimiter run is left-flanking only via CommonMark
    // Rule 2b (preceded by whitespace/start or ASCII punctuation, AND immediately
    // followed by non-`*`/`_` ASCII punctuation). tree-sitter-md's inline grammar does
    // not implement this branch of the flanking rules, causing divergence when
    // pulldown-cmark correctly recognises emphasis. E.g. *$*$ — the * is followed by $
    // (punctuation) and preceded by start-of-string (= whitespace in CommonMark).
    {
        let bytes = s.as_bytes();
        let is_cm_punct = |b: u8| {
            // CommonMark ASCII punctuation set
            matches!(
                b,
                b'!' | b'"'
                    | b'#'
                    | b'$'
                    | b'%'
                    | b'&'
                    | b'\''
                    | b'('
                    | b')'
                    | b'*'
                    | b'+'
                    | b','
                    | b'-'
                    | b'.'
                    | b'/'
                    | b':'
                    | b';'
                    | b'<'
                    | b'='
                    | b'>'
                    | b'?'
                    | b'@'
                    | b'['
                    | b'\\'
                    | b']'
                    | b'^'
                    | b'_'
                    | b'`'
                    | b'{'
                    | b'|'
                    | b'}'
                    | b'~'
            )
        };
        let mut i = 0;
        while i < bytes.len() {
            let delim = bytes[i];
            if delim == b'*' || delim == b'_' {
                // Measure the run
                let run_start = i;
                while i < bytes.len() && bytes[i] == delim {
                    i += 1;
                }
                let run_end = i;
                let after = bytes.get(run_end).copied();
                let before = if run_start == 0 {
                    Some(b' ') // start-of-string counts as whitespace
                } else {
                    bytes.get(run_start - 1).copied()
                };
                // Left-flanking by rule 2b only: after is non-delim ASCII punct, before
                // is whitespace or ASCII punct.
                if let (Some(a), Some(b)) = (after, before) {
                    if is_cm_punct(a) && a != delim {
                        if b.is_ascii_whitespace() || is_cm_punct(b) {
                            return;
                        }
                    }
                }
            } else {
                i += 1;
            }
        }
    }

    // Skip inputs where every pipe-containing line consists only of |, -, :, and
    // whitespace: pulldown-cmark's GFM table extension recognises these as minimal
    // tables (e.g. |-\n|-) but tree-sitter-md's block grammar does not. Inputs
    // with at least one pipe-line that contains non-delimiter characters are fine
    // (tree-sitter handles fully-formed tables correctly).
    if s.contains('|') {
        let all_pipe_lines_delimiter_only = s
            .lines()
            .filter(|line| line.contains('|'))
            .all(|line| line.bytes().all(|b| matches!(b, b'|' | b'-' | b':' | b' ' | b'\t')));
        if all_pipe_lines_delimiter_only {
            return;
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
