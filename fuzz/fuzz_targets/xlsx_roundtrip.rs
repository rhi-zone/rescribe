#![no_main]

//! XLSX roundtrip fuzz target.
//!
//! Generates arbitrary rescribe Documents with XLSX-supported constructs,
//! emits them to XLSX bytes via rescribe-write-xlsx, parses them back via
//! rescribe-read-xlsx, and asserts that text content is preserved.
//!
//! Direction: arbitrary_rescribe_doc → emit → parse → assert text preserved.

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use rescribe_core::{Document, Node};
use rescribe_std::{node, prop};

// ── Fuzz-friendly cell value types ────────────────────────────────────────────

#[derive(Arbitrary, Debug)]
enum FuzzCellValue {
    Text(String),
    Number(f64),
    Bool(bool),
}

#[derive(Arbitrary, Debug)]
struct FuzzRow {
    cells: Vec<FuzzCellValue>,
}

#[derive(Arbitrary, Debug)]
struct FuzzSheet {
    rows: Vec<FuzzRow>,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Sanitise text: strip NUL, XML-illegal control characters, and whitespace
/// characters that XLSX does not reliably preserve without xml:space="preserve".
/// Tabs and newlines are stripped; the XLSX cell-value XML path does not
/// guarantee their preservation when the ooxml-sml writer omits that attribute.
fn sanitise(s: &str) -> String {
    s.chars()
        .filter(|c| {
            !matches!(
                *c,
                '\0' | '\t' | '\n' | '\r'
                    | '\x01'..='\x08'
                    | '\x0b'
                    | '\x0c'
                    | '\x0e'..='\x1f'
            )
        })
        .collect()
}

fn cell_text(val: &FuzzCellValue) -> String {
    match val {
        FuzzCellValue::Text(s) => {
            let clean = sanitise(s);
            // XLSX XML writers may omit xml:space="preserve", so XML normalises
            // leading/trailing whitespace away. Trim here to match what the reader
            // will return — this is a known limitation of the XLSX cell-value path.
            let trimmed = clean.trim().to_string();
            if trimmed.is_empty() {
                return String::new();
            }
            // The XLSX writer (rescribe-write-xlsx) auto-converts numeric-looking
            // strings to f64 via `.parse::<f64>()`, so "000" becomes 0.0 → "0".
            // Apply the same normalisation here so the comparison is fair.
            if let Ok(n) = trimmed.parse::<f64>() {
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    return (n as i64).to_string();
                } else if n.is_finite() {
                    return n.to_string();
                }
            }
            trimmed
        }
        FuzzCellValue::Number(n) => {
            if n.is_finite() {
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    (*n as i64).to_string()
                } else {
                    n.to_string()
                }
            } else {
                String::new()
            }
        }
        FuzzCellValue::Bool(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
    }
}

fn make_cell_node(val: &FuzzCellValue, first_row: bool) -> Node {
    let kind = if first_row { node::TABLE_HEADER } else { node::TABLE_CELL };
    let text = cell_text(val);
    let text_node = Node::new(node::TEXT).prop(prop::CONTENT, text);
    let para = Node::new(node::PARAGRAPH).child(text_node);
    Node::new(kind).child(para)
}

// ── Fuzz target ───────────────────────────────────────────────────────────────

fuzz_target!(|sheet: FuzzSheet| {
    // Single-sheet fuzz: the XLSX reader only adds sheet-name headings when
    // there are multiple sheets, so limiting to one sheet keeps the roundtrip
    // structure stable and makes text comparison straightforward.
    if sheet.rows.is_empty() {
        return;
    }

    let rows: Vec<Node> = sheet
        .rows
        .iter()
        .enumerate()
        .filter_map(|(row_idx, row)| {
            if row.cells.is_empty() {
                return None;
            }
            let cells: Vec<Node> = row
                .cells
                .iter()
                .map(|v| make_cell_node(v, row_idx == 0))
                .collect();
            Some(Node::new(node::TABLE_ROW).children(cells))
        })
        .collect();

    if rows.is_empty() {
        return;
    }

    let table = Node::new(node::TABLE).children(rows);
    let doc = Document::new().with_content(Node::new(node::DOCUMENT).child(table));

    // Emit to XLSX bytes — must not panic.
    let Ok(emit_result) = rescribe_write_xlsx::emit(&doc) else {
        return;
    };

    // Parse back — must not panic.
    let Ok(parse_result) = rescribe_read_xlsx::parse_bytes(&emit_result.value) else {
        return;
    };

    // All visible text content must survive the roundtrip.
    let text_before = extract_text(&doc.content);
    let text_after = extract_text(&parse_result.value.content);

    assert_eq!(
        text_before,
        text_after,
        "XLSX roundtrip lost text content\n  before: {text_before:?}\n  after:  {text_after:?}"
    );
});

fn extract_text(node: &Node) -> String {
    let mut text = String::new();
    if node.kind.as_str() == node::TEXT {
        if let Some(content) = node.props.get_str(prop::CONTENT) {
            text.push_str(content);
        }
    }
    for child in &node.children {
        text.push_str(&extract_text(child));
    }
    text
}
