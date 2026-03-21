#![no_main]
//! CSV roundtrip fuzz target.
//!
//! Generates an arbitrary CsvDoc, emits it to CSV text, parses it back,
//! strips spans, and asserts the result is equal to the original.
//!
//! Direction: arbitrary CsvDoc → emit → parse → strip_spans → assert equal

use arbitrary::Arbitrary;
use csv_fmt::{Cell, CsvDoc, Row, Span};
use libfuzzer_sys::fuzz_target;

/// A fuzz cell: value must not contain commas, quotes, or newlines to avoid
/// ambiguity that the emitter resolves via quoting/escaping.
#[derive(Arbitrary, Debug)]
struct FuzzCell {
    value: String,
}

#[derive(Arbitrary, Debug)]
struct FuzzRow {
    cells: Vec<FuzzCell>,
}

fn sanitise(s: &str) -> String {
    // Strip control characters and null bytes.
    // Also trim whitespace since the CSV parser trims cell values.
    s.chars()
        .filter(|c| !matches!(*c, '\x00'..='\x1f'))
        .collect::<String>()
        .trim()
        .to_string()
}

fuzz_target!(|rows: Vec<FuzzRow>| {
    let doc = CsvDoc {
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
                // The parser skips lines where the entire row trims to empty.
                // That happens when ALL cells are empty. Keep rows with at least one
                // non-empty cell.
                if cells.iter().all(|c| c.value.is_empty()) {
                    return None;
                }
                Some(Row { cells, span: Span::NONE })
            })
            .collect(),
        span: Span::NONE,
    };

    let csv_text = csv_fmt::emit(&doc);

    let (parsed, _diags) = csv_fmt::parse(&csv_text);
    let parsed = parsed.strip_spans();
    let expected = doc.strip_spans();

    assert_eq!(
        expected,
        parsed,
        "CSV roundtrip mismatch\n  emitted: {csv_text:?}"
    );
});
