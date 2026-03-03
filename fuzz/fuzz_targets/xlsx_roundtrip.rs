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

/// Sanitise text: strip NUL and XML-illegal control characters.
/// ooxml-sml ≥ 0.1.1-alpha.1 correctly emits xml:space="preserve" on shared
/// strings, so leading/trailing spaces and internal whitespace are preserved.
fn sanitise(s: &str) -> String {
    s.chars()
        .filter(|c| {
            !matches!(
                *c,
                '\0' | '\x01'..='\x08' | '\x0b' | '\x0c' | '\x0e'..='\x1f'
            )
        })
        .collect()
}

fn cell_text(val: &FuzzCellValue) -> String {
    match val {
        FuzzCellValue::Text(s) => {
            // Sanitise only; no trim() or numeric coercion needed.
            // xlsx:cell-type = "s" (set in make_cell_node) ensures the writer
            // calls set_cell(str) and the value roundtrips exactly.
            sanitise(s)
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
    // Set xlsx:cell-type so the writer uses the correct API (set_cell(f64) vs
    // set_cell(str)) without guessing from string content.
    let para = match val {
        FuzzCellValue::Text(_) => Node::new(node::PARAGRAPH)
            .prop("xlsx:cell-type", "s")
            .child(text_node),
        FuzzCellValue::Number(_) => Node::new(node::PARAGRAPH)
            .prop("xlsx:cell-type", "n")
            .child(text_node),
        FuzzCellValue::Bool(_) => Node::new(node::PARAGRAPH)
            .prop("xlsx:cell-type", "b")
            .child(text_node),
    };
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
