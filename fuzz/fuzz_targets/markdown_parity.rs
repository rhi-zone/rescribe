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

    // CommonMark ASCII punctuation set (used in multiple skip rules below).
    let is_cm_punct = |b: u8| {
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

    // Skip inputs where a `*` or `_` delimiter run is left-flanking only via CommonMark
    // Rule 2b (preceded by whitespace/start or ASCII punctuation, AND immediately
    // followed by non-`*`/`_` ASCII punctuation). tree-sitter-md's inline grammar does
    // not implement this branch of the flanking rules, causing divergence when
    // pulldown-cmark correctly recognises emphasis. E.g. *$*$ — the * is followed by $
    // (punctuation) and preceded by start-of-string (= whitespace in CommonMark).
    {
        let bytes = s.as_bytes();
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

    // Skip inputs that contain a GFM table delimiter row immediately following a line
    // with a pipe: pulldown-cmark recognises any pipe line followed by a
    // |---|---|…-style delimiter row as a table (header + delimiter); tree-sitter-md
    // does not recognise minimal or non-standard table forms (e.g. |-)\n|-, |-\n|-).
    // Detection: if two consecutive lines exist where line N contains '|' and line N+1
    // consists only of |, -, :, and whitespace with at least one '-', skip.
    if s.contains('|') {
        let lines: Vec<&str> = s.lines().collect();
        for i in 0..lines.len().saturating_sub(1) {
            let next = lines[i + 1];
            let next_is_delim = next.contains('|')
                && next.contains('-')
                && next.bytes().all(|b| matches!(b, b'|' | b'-' | b':' | b' ' | b'\t'));
            if next_is_delim && lines[i].contains('|') {
                return;
            }
        }
    }

    // Skip inputs where a `*` or `_` is preceded by non-delimiter ASCII punctuation
    // AND followed by a non-whitespace, non-punctuation character: tree-sitter-md
    // may incorrectly use such a delimiter as an emphasis closer even though it is
    // not right-flanking per CommonMark (condition 2b fails: preceded by punctuation
    // but not followed by whitespace or punctuation). pulldown-cmark correctly
    // produces no emphasis when no valid closer exists. E.g. *a-*a-*a: the `*` at
    // pos 3 is preceded by `-` (punct) and followed by `a`, so it cannot close.
    {
        let bytes = s.as_bytes();
        for i in 0..bytes.len() {
            let delim = bytes[i];
            if delim == b'*' || delim == b'_' {
                if let Some(&prev) = i.checked_sub(1).and_then(|j| bytes.get(j)) {
                    if is_cm_punct(prev) && prev != delim {
                        if let Some(&next) = bytes.get(i + 1) {
                            if !next.is_ascii_whitespace() && !is_cm_punct(next) {
                                return;
                            }
                        }
                    }
                }
            }
        }
    }

    // Skip inputs where any `*` or `_` is immediately adjacent to (preceded by or
    // followed by) a non-whitespace ASCII control character in range 0x01-0x08 or
    // 0x0e-0x1f or 0x7f: pulldown-cmark and tree-sitter-md differ in how these
    // characters affect emphasis flanking analysis. E.g. \x0cR*\x14**: tree-sitter
    // finds emphasis while pulldown does not.
    {
        let bytes = s.as_bytes();
        for i in 0..bytes.len() {
            let b = bytes[i];
            if b == b'*' || b == b'_' {
                let prev = i.checked_sub(1).and_then(|j| bytes.get(j)).copied();
                let next = bytes.get(i + 1).copied();
                for adj in prev.into_iter().chain(next) {
                    if matches!(adj, 0x01..=0x08 | 0x0e..=0x1f | 0x7f) {
                        return;
                    }
                }
            }
        }
    }

    // Skip inputs where ≥3 single `*` (or `_`) delimiter runs appear with only
    // alphanumeric characters between consecutive runs: tree-sitter-md does not
    // implement CommonMark's left-to-right ("earliest-first") stack precedence for
    // emphasis, and may open emphasis at a later delimiter than pulldown. Detected
    // by: any `*`/`_` that is both preceded AND followed by an ASCII alphanumeric
    // character (a "both-flanking word-adjacent" delimiter), AND the total count of
    // that delimiter character is ≥ 3. E.g. *a*a*a: the `*` at pos 2 is surrounded
    // by `a` on both sides; tree-sitter opens at pos 2 rather than pos 0.
    {
        let bytes = s.as_bytes();
        let has_mid_star = bytes.windows(3).any(|w| {
            w[1] == b'*' && w[0].is_ascii_alphanumeric() && w[2].is_ascii_alphanumeric()
        });
        let has_mid_under = bytes.windows(3).any(|w| {
            w[1] == b'_' && w[0].is_ascii_alphanumeric() && w[2].is_ascii_alphanumeric()
        });
        let star_count = bytes.iter().filter(|&&b| b == b'*').count();
        let under_count = bytes.iter().filter(|&&b| b == b'_').count();
        if (has_mid_star && star_count >= 3) || (has_mid_under && under_count >= 3) {
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
