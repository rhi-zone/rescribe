/// Generate XLSX fixture files for the rescribe-read-xlsx test suite.
///
/// Run with: cargo run -p rescribe-read-xlsx --bin gen_fixtures
use ooxml_sml::writer::WorkbookBuilder;
use rescribe_core::Node;
use std::io::Cursor;

// ── XLSX construction helpers ──────────────────────────────────────────────

fn make_xlsx(build: impl FnOnce(&mut WorkbookBuilder)) -> Vec<u8> {
    let mut wb = WorkbookBuilder::new();
    build(&mut wb);
    let mut buf = Cursor::new(Vec::new());
    wb.write(&mut buf).unwrap();
    buf.into_inner()
}

// ── Expected JSON generation ───────────────────────────────────────────────

fn node_to_assertions(node: &Node, path: &str, out: &mut Vec<serde_json::Value>) {
    let kind = node.kind.as_str();
    let mut obj = serde_json::json!({ "path": path, "kind": kind });

    let mut props_map = serde_json::Map::new();
    // String props
    for key in &["content", "xlsx:cell-type", "xlsx:formula"] {
        if let Some(val) = node.props.get_str(key) {
            props_map.insert(key.to_string(), serde_json::Value::String(val.to_string()));
        }
    }
    // Int props
    if let Some(level) = node.props.get_int("level") {
        props_map.insert("level".to_string(), serde_json::Value::Number(level.into()));
    }
    if !props_map.is_empty() {
        obj["props"] = serde_json::Value::Object(props_map);
    }

    out.push(obj);

    for (i, child) in node.children.iter().enumerate() {
        let child_path = if path == "/" {
            format!("/{i}")
        } else {
            format!("{path}/{i}")
        };
        node_to_assertions(child, &child_path, out);
    }
}

fn generate_expected_json(desc: &str, category: &str, xlsx_bytes: &[u8]) -> String {
    let result = rescribe_read_xlsx::parse_bytes(xlsx_bytes).expect("parse failed");
    let doc = result.value;

    let mut assertions: Vec<serde_json::Value> = vec![serde_json::json!({
        "path": "/",
        "kind": "document",
    })];

    for (i, child) in doc.content.children.iter().enumerate() {
        node_to_assertions(child, &format!("/{i}"), &mut assertions);
    }

    serde_json::to_string_pretty(&serde_json::json!({
        "description": desc,
        "category": category,
        "assertions": assertions,
    }))
    .unwrap()
}

fn write_fixture(name: &str, xlsx_bytes: Vec<u8>, desc: &str) {
    write_fixture_cat(name, xlsx_bytes, desc, "happy");
}

fn write_fixture_cat(name: &str, xlsx_bytes: Vec<u8>, desc: &str, category: &str) {
    let dir = format!("fixtures/xlsx/{name}");
    std::fs::create_dir_all(&dir).unwrap();
    let expected = generate_expected_json(desc, category, &xlsx_bytes);
    std::fs::write(format!("{dir}/input.xlsx"), &xlsx_bytes).unwrap();
    std::fs::write(format!("{dir}/expected.json"), &expected).unwrap();
    println!("wrote {dir}/");
}

fn write_error_fixture(name: &str, xlsx_bytes: Vec<u8>, desc: &str) {
    let dir = format!("fixtures/xlsx/{name}");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(format!("{dir}/input.xlsx"), &xlsx_bytes).unwrap();
    let expected = serde_json::to_string_pretty(&serde_json::json!({
        "description": desc,
        "category": "adversarial",
        "expect_error": true,
        "assertions": []
    }))
    .unwrap();
    std::fs::write(format!("{dir}/expected.json"), &expected).unwrap();
    println!("wrote {dir}/");
}

// ── Main ───────────────────────────────────────────────────────────────────

fn main() {
    // ── Basic structure ───────────────────────────────────────────────────

    // Regen existing basic fixture
    write_fixture(
        "basic",
        make_xlsx(|wb| {
            let s = wb.add_sheet("Sheet1");
            s.set_cell("A1", "Name");
            s.set_cell("B1", "Value");
            s.set_cell("A2", "Alice");
            s.set_cell("B2", 42i64);
        }),
        "XLSX spreadsheet with header row and one data row",
    );

    // Regen existing multi-sheet fixture
    write_fixture(
        "multi-sheet",
        make_xlsx(|wb| {
            let s1 = wb.add_sheet("People");
            s1.set_cell("A1", "Name");
            s1.set_cell("B1", "Age");
            s1.set_cell("A2", "Alice");
            s1.set_cell("B2", 30i64);
            s1.set_cell("A3", "Bob");
            s1.set_cell("B3", 25i64);
            let s2 = wb.add_sheet("Products");
            s2.set_cell("A1", "Product");
            s2.set_cell("B1", "Price");
            s2.set_cell("A2", "Widget");
            s2.set_cell("B2", 9.99f64);
        }),
        "XLSX workbook with two sheets — each sheet produces heading + table",
    );

    // ── Cell value types ──────────────────────────────────────────────────

    // Regen existing formula fixture
    write_fixture(
        "formula",
        make_xlsx(|wb| {
            let s = wb.add_sheet("Calculations");
            s.set_cell("A1", "A");
            s.set_cell("B1", "B");
            s.set_cell("C1", "Sum");
            s.set_cell("A2", 10i64);
            s.set_cell("B2", 20i64);
            s.set_formula("C2", "A2+B2");
        }),
        "XLSX formula cells have xlsx:formula property preserved for round-trip",
    );

    write_fixture(
        "numbers",
        make_xlsx(|wb| {
            let s = wb.add_sheet("Numbers");
            s.set_cell("A1", "Label");
            s.set_cell("B1", "Value");
            s.set_cell("A2", "Integer");
            s.set_cell("B2", 42i64);
            s.set_cell("A3", "Float");
            s.set_cell("B3", std::f64::consts::PI);
            s.set_cell("A4", "Negative");
            s.set_cell("B4", -7i64);
            s.set_cell("A5", "Zero");
            s.set_cell("B5", 0i64);
            s.set_cell("A6", "Large");
            s.set_cell("B6", 1_000_000i64);
        }),
        "XLSX numeric cell values — integers and floats mapped to text nodes",
    );

    write_fixture(
        "booleans",
        make_xlsx(|wb| {
            let s = wb.add_sheet("Booleans");
            s.set_cell("A1", "Label");
            s.set_cell("B1", "Value");
            s.set_cell("A2", "True");
            s.set_cell("B2", true);
            s.set_cell("A3", "False");
            s.set_cell("B3", false);
        }),
        "XLSX boolean cells map to TRUE/FALSE text with xlsx:cell-type=b",
    );

    write_fixture(
        "cell-types-mixed",
        make_xlsx(|wb| {
            let s = wb.add_sheet("Mixed");
            s.set_cell("A1", "Type");
            s.set_cell("B1", "Value");
            s.set_cell("A2", "String");
            s.set_cell("B2", "hello");
            s.set_cell("A3", "Number");
            s.set_cell("B3", 42i64);
            s.set_cell("A4", "Boolean");
            s.set_cell("B4", true);
            s.set_cell("A5", "Formula");
            s.set_formula("B5", "B3*2");
        }),
        "XLSX sheet with mixed cell types — string, number, boolean, formula",
    );

    // ── Structural features ───────────────────────────────────────────────

    write_fixture(
        "merged-cells",
        make_xlsx(|wb| {
            let s = wb.add_sheet("MergedCells");
            s.set_cell("A1", "Section Header");
            s.merge_cells("A1:C1");
            s.set_cell("A2", "Col1");
            s.set_cell("B2", "Col2");
            s.set_cell("C2", "Col3");
            s.set_cell("A3", "data1");
            s.set_cell("B3", "data2");
            s.set_cell("C3", "data3");
        }),
        "XLSX merged cells — fidelity warning emitted, cell content still parsed",
    );

    write_fixture(
        "freeze-pane",
        make_xlsx(|wb| {
            let s = wb.add_sheet("FreezePanes");
            s.set_cell("A1", "Name");
            s.set_cell("B1", "Value");
            for i in 2u32..=5 {
                s.set_cell_at(i, 1, format!("Row {i}"));
                s.set_cell_at(i, 2, i as i64);
            }
            s.set_freeze_pane(1, 0);
        }),
        "XLSX frozen header row — freeze not represented in IR, content preserved",
    );

    write_fixture(
        "auto-filter",
        make_xlsx(|wb| {
            let s = wb.add_sheet("AutoFilter");
            s.set_cell("A1", "Product");
            s.set_cell("B1", "Category");
            s.set_cell("C1", "Price");
            s.set_cell("A2", "Widget");
            s.set_cell("B2", "Tools");
            s.set_cell("C2", 9.99f64);
            s.set_cell("A3", "Gadget");
            s.set_cell("B3", "Electronics");
            s.set_cell("C3", 29.99f64);
            s.set_auto_filter("A1:C1");
        }),
        "XLSX auto-filter range — filter not represented in IR, content preserved",
    );

    write_fixture(
        "hyperlinks",
        make_xlsx(|wb| {
            let s = wb.add_sheet("Links");
            s.set_cell("A1", "Description");
            s.set_cell("B1", "URL");
            s.set_cell("A2", "Example site");
            s.set_cell("B2", "https://example.com");
            s.add_hyperlink("B2", "https://example.com");
        }),
        "XLSX cell with hyperlink — URL not represented in IR, cell text preserved",
    );

    write_fixture(
        "comments",
        make_xlsx(|wb| {
            let s = wb.add_sheet("Comments");
            s.set_cell("A1", "Data");
            s.set_cell("B1", "Notes");
            s.set_cell("A2", "important value");
            s.set_cell("B2", "see comment");
            s.add_comment("A2", "This cell has a comment");
        }),
        "XLSX cell comments — comment text not represented in IR, cell text preserved",
    );

    write_fixture(
        "column-widths",
        make_xlsx(|wb| {
            let s = wb.add_sheet("Widths");
            s.set_cell("A1", "Narrow");
            s.set_cell("B1", "Wide");
            s.set_cell("C1", "Normal");
            s.set_cell("A2", "short");
            s.set_cell("B2", "a longer piece of text");
            s.set_cell("C2", "medium text");
            s.set_column_width("A", 5.0);
            s.set_column_width("B", 30.0);
        }),
        "XLSX column widths — not represented in IR, cell content preserved",
    );

    write_fixture(
        "row-heights",
        make_xlsx(|wb| {
            let s = wb.add_sheet("Heights");
            s.set_cell("A1", "Header");
            s.set_cell("A2", "Tall row");
            s.set_cell("A3", "Normal row");
            s.set_row_height(2, 40.0);
        }),
        "XLSX custom row heights — not represented in IR, cell content preserved",
    );

    // ── Adversarial ───────────────────────────────────────────────────────

    write_fixture_cat(
        "adv-empty-workbook",
        make_xlsx(|_wb| {}),
        "XLSX workbook with no sheets produces empty document",
        "adversarial",
    );

    write_fixture_cat(
        "adv-empty-sheet",
        make_xlsx(|wb| {
            let _ = wb.add_sheet("EmptySheet");
        }),
        "XLSX sheet with no data rows produces empty document",
        "adversarial",
    );

    write_error_fixture(
        "adv-malformed-zip",
        b"not a zip file at all".to_vec(),
        "Malformed zip bytes return a parse error without panic",
    );

    write_error_fixture(
        "adv-empty-bytes",
        b"".to_vec(),
        "Empty input bytes return a parse error without panic",
    );

    // ── Pathological ──────────────────────────────────────────────────────

    write_fixture_cat(
        "path-many-rows",
        make_xlsx(|wb| {
            let s = wb.add_sheet("ManyRows");
            s.set_cell("A1", "Index");
            s.set_cell("B1", "Value");
            for i in 2u32..=51 {
                s.set_cell_at(i, 1, (i - 1) as i64);
                s.set_cell_at(i, 2, format!("row {}", i - 1));
            }
        }),
        "XLSX sheet with 50 data rows — all parsed without panic",
        "pathological",
    );

    write_fixture_cat(
        "path-many-columns",
        make_xlsx(|wb| {
            let s = wb.add_sheet("ManyColumns");
            for col in 1u32..=10 {
                s.set_cell_at(1, col, format!("Col{col}"));
                s.set_cell_at(2, col, col as i64);
            }
        }),
        "XLSX sheet with 10 columns — all parsed without panic",
        "pathological",
    );

    write_fixture_cat(
        "path-many-sheets",
        make_xlsx(|wb| {
            for i in 1u32..=10 {
                let s = wb.add_sheet(format!("Sheet{i}"));
                s.set_cell("A1", format!("Sheet {i} Header"));
                s.set_cell("A2", format!("Sheet {i} Data"));
            }
        }),
        "XLSX workbook with 10 sheets — all parsed without panic",
        "pathological",
    );

    // ── Composition ────────────────────────────────────────────────────────

    write_fixture(
        "mixed-content",
        make_xlsx(|wb| {
            let s1 = wb.add_sheet("Summary");
            s1.set_cell("A1", "Metric");
            s1.set_cell("B1", "Value");
            s1.set_cell("A2", "Total");
            s1.set_cell("B2", 100i64);
            s1.set_cell("A3", "Passed");
            s1.set_cell("B3", true);
            s1.set_cell("A4", "Rate");
            s1.set_formula("B4", "B3/B2");

            let s2 = wb.add_sheet("Details");
            s2.set_cell("A1", "Name");
            s2.set_cell("B1", "Result");
            s2.set_cell("A2", "Test A");
            s2.set_cell("B2", "passed");
            s2.set_cell("A3", "Test B");
            s2.set_cell("B3", "passed");
        }),
        "XLSX multi-sheet workbook with mixed cell types including formula",
    );

    println!("Done.");
}
