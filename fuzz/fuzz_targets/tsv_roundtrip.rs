#![no_main]
//! TSV roundtrip fuzz target.
//!
//! Generates an arbitrary TsvDoc, emits it to TSV text, parses it back,
//! strips spans, and asserts the result is equal to the original.
//!
//! Direction: arbitrary TsvDoc → emit → parse → strip_spans → assert equal

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use tsv_fmt::{Cell, Row, Span, TsvDoc};

#[derive(Arbitrary, Debug)]
struct FuzzCell {
    value: String,
}

#[derive(Arbitrary, Debug)]
struct FuzzRow {
    cells: Vec<FuzzCell>,
}

fn sanitise(s: &str) -> String {
    // Strip control chars (tabs, newlines, nulls) that would break TSV parsing
    s.chars()
        .filter(|c| !matches!(*c, '\x00'..='\x1f'))
        .collect()
}

fuzz_target!(|rows: Vec<FuzzRow>| {
    let doc = TsvDoc {
        rows: rows
            .into_iter()
            .filter_map(|r| {
                let cells: Vec<Cell> = r
                    .cells
                    .into_iter()
                    .map(|c| Cell { value: sanitise(&c.value), span: Span::NONE })
                    .collect();
                if cells.is_empty() {
                    return None;
                }
                // The parser skips any line where line.trim().is_empty().
                // Mirror that check here: join cells with \t and trim.
                let joined: String =
                    cells.iter().map(|c| c.value.as_str()).collect::<Vec<_>>().join("\t");
                if joined.trim().is_empty() {
                    return None;
                }
                Some(Row { cells, span: Span::NONE })
            })
            .collect(),
        span: Span::NONE,
    };

    let tsv_text = tsv_fmt::emit(&doc);

    let (parsed, _diags) = tsv_fmt::parse(&tsv_text);
    let parsed = parsed.strip_spans();
    let expected = doc.strip_spans();

    assert_eq!(
        expected,
        parsed,
        "TSV roundtrip mismatch\n  emitted: {tsv_text:?}"
    );
});
