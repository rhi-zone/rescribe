/// Generate XLSX fixture files for the rescribe-fmt-ooxml xlsx test suite.
///
/// Run with: cargo run -p rescribe-fmt-ooxml --bin gen_xlsx_fixtures --features xlsx
use ooxml_sml::writer::{
    CellStyle, CfColor, CfValue, ColorScaleRule, ConditionalFormat, DataBarRule, IconSetRule,
    WorkbookBuilder,
};
use rescribe_core::{Node, PropValue};
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

/// Convert a `PropValue` to its `serde_json::Value` equivalent, matching
/// `rescribe-fixtures`'s `prop_value_matches` comparator exactly (String,
/// Int/Float -> Number, Bool, List, Map) so every prop on a node — not just
/// an allowlisted subset — can be asserted on.
fn propvalue_to_json(v: &PropValue) -> serde_json::Value {
    match v {
        PropValue::String(s) => serde_json::Value::String(s.clone()),
        PropValue::Int(i) => serde_json::json!(i),
        PropValue::Float(f) => serde_json::json!(f),
        PropValue::Bool(b) => serde_json::Value::Bool(*b),
        PropValue::List(items) => {
            serde_json::Value::Array(items.iter().map(propvalue_to_json).collect())
        }
        PropValue::Map(map) => serde_json::Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), propvalue_to_json(v)))
                .collect(),
        ),
    }
}

fn node_to_assertions(node: &Node, path: &str, out: &mut Vec<serde_json::Value>) {
    let kind = node.kind.as_str();
    let mut obj = serde_json::json!({ "path": path, "kind": kind });

    let mut props_map = serde_json::Map::new();
    for (key, val) in node.props.iter() {
        props_map.insert(key.clone(), propvalue_to_json(val));
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
    let result = rescribe_fmt_ooxml::xlsx::parse_bytes(xlsx_bytes).expect("parse failed");
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
        "XLSX workbook with two sheets — each sheet produces a `sheet` node",
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
        "XLSX formula cells have value:formula (source) and value:type=formula-result (computed value) preserved for round-trip",
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
        "XLSX numeric cell values — integers and floats mapped to value:type=number sheet_cell props",
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
        "XLSX boolean cells map to value:type=boolean, value:data=true/false",
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

    write_fixture(
        "number-formats",
        make_xlsx(|wb| {
            let s = wb.add_sheet("NumberFormats");
            s.set_cell("A1", "Label");
            s.set_cell("B1", "Value");
            s.set_cell("A2", "Percentage");
            s.set_cell_styled("B2", 0.5, CellStyle::new().with_number_format("0.00%"));
            s.set_cell("A3", "Currency");
            s.set_cell_styled(
                "B3",
                19.99,
                CellStyle::new().with_number_format("$#,##0.00"),
            );
            s.set_cell("A4", "Date");
            s.set_cell_styled(
                "B4",
                45000.0,
                CellStyle::new().with_number_format("yyyy-mm-dd"),
            );
            s.set_cell("A5", "Time");
            s.set_cell_styled("B5", 0.5, CellStyle::new().with_number_format("hh:mm:ss"));
            s.set_cell("A6", "DateTime");
            s.set_cell_styled(
                "B6",
                45000.5,
                CellStyle::new().with_number_format("m/d/yy h:mm"),
            );
            s.set_cell("A7", "PlainNumber");
            s.set_cell("B7", 42i64);
        }),
        "XLSX number-format-derived cell types — a cell's numFmtId is classified (ooxml_sml::classify_format_code) into value:type=percentage/currency/date/time (a combined date+time format maps to \"date\", matching ODF's own value-type convention); the raw format code round-trips verbatim via xlsx:number_format",
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
        "XLSX merged cells — modeled via rowspan/colspan on the top-left sheet_cell, no fidelity warning needed",
    );

    write_fixture(
        "conditional-formatting",
        make_xlsx(|wb| {
            let s = wb.add_sheet("ConditionalFormatting");
            s.set_cell("A1", 10.0);
            s.set_cell("A2", 20.0);
            s.set_cell("A3", 30.0);

            // A cellIs rule and a colorScale rule sharing one range.
            let cf = ConditionalFormat::new("A1:A3")
                .add_cell_is_rule("greaterThan", "15", 1, Some(0))
                .add_color_scale_rule(
                    ColorScaleRule {
                        cfvo: vec![
                            CfValue::min_max(ooxml_sml::types::ConditionalValueType::Min),
                            CfValue::min_max(ooxml_sml::types::ConditionalValueType::Max),
                        ],
                        colors: vec![CfColor::rgb("FF0000"), CfColor::rgb("00FF00")],
                    },
                    2,
                );
            s.add_conditional_format(cf);

            // A dataBar rule and an iconSet rule on a second range.
            s.set_cell("B1", 5.0);
            s.set_cell("B2", 50.0);
            let cf2 = ConditionalFormat::new("B1:B2")
                .add_data_bar_rule(
                    DataBarRule {
                        min_length: Some(10),
                        max_length: Some(90),
                        show_value: Some(true),
                        cfvo: vec![
                            CfValue::min_max(ooxml_sml::types::ConditionalValueType::Min),
                            CfValue::min_max(ooxml_sml::types::ConditionalValueType::Max),
                        ],
                        color: CfColor::rgb("0000FF"),
                    },
                    1,
                )
                .add_icon_set_rule(
                    IconSetRule {
                        icon_set: Some(ooxml_sml::types::IconSetType::_3TrafficLights1),
                        show_value: Some(true),
                        percent: Some(true),
                        reverse: Some(false),
                        cfvo: vec![
                            CfValue::new(ooxml_sml::types::ConditionalValueType::Percent, "0"),
                            CfValue::new(ooxml_sml::types::ConditionalValueType::Percent, "33"),
                            CfValue::new(ooxml_sml::types::ConditionalValueType::Percent, "67"),
                        ],
                    },
                    2,
                );
            s.add_conditional_format(cf2);
        }),
        "XLSX conditional formatting (cfRule: cellIs, colorScale, dataBar, iconSet) — modeled as xlsx:conditional_format/xlsx:conditional_format_rule child nodes on the sheet (OOXML-namespaced, not rescribe-std vocabulary — see the node-kind constants' doc comment in xlsx.rs for why), no fidelity warning needed",
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
        "XLSX sheet with no data rows produces an empty sheet node (no sheet_row children)",
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

    // ── Charts (ADR 0016) ─────────────────────────────────────────────────

    write_fixture(
        "chart-bar",
        make_xlsx(|wb| {
            let s = wb.add_sheet("Sheet1");
            s.set_cell("A1", "Quarter");
            s.set_cell("B1", "Revenue");
            s.set_cell("A2", "Q1");
            s.set_cell("B2", 10i64);
            s.set_cell("A3", "Q2");
            s.set_cell("B3", 20i64);
            s.set_cell("A4", "Q3");
            s.set_cell("B4", 15i64);
            s.set_cell("A5", "Q4");
            s.set_cell("B5", 25i64);
            // A minimal but complete `<c:chartSpace>` (ECMA-376 §21.2): one
            // bar-chart series whose categories/values are cell-range
            // references (`Sheet1!$A$2:$A$5` / `Sheet1!$B$2:$B$5`) paired
            // with a cached snapshot (`strCache`/`numCache`) of the
            // referenced cells' contents — exercising ADR 0016 Decisions
            // 1-2 (both `-ref` and cached `chart:values`/`chart:categories`
            // populated together), plus a title, legend, and both axes.
            let chart_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <c:chart>
    <c:title><c:tx><c:rich><a:p><a:r><a:t>Quarterly Revenue</a:t></a:r></a:p></c:rich></c:tx></c:title>
    <c:plotArea>
      <c:layout/>
      <c:barChart>
        <c:ser>
          <c:idx val="0"/>
          <c:order val="0"/>
          <c:tx><c:strRef><c:f>Sheet1!$B$1</c:f><c:strCache><c:ptCount val="1"/><c:pt idx="0"><c:v>Revenue</c:v></c:pt></c:strCache></c:strRef></c:tx>
          <c:cat>
            <c:strRef>
              <c:f>Sheet1!$A$2:$A$5</c:f>
              <c:strCache>
                <c:ptCount val="4"/>
                <c:pt idx="0"><c:v>Q1</c:v></c:pt>
                <c:pt idx="1"><c:v>Q2</c:v></c:pt>
                <c:pt idx="2"><c:v>Q3</c:v></c:pt>
                <c:pt idx="3"><c:v>Q4</c:v></c:pt>
              </c:strCache>
            </c:strRef>
          </c:cat>
          <c:val>
            <c:numRef>
              <c:f>Sheet1!$B$2:$B$5</c:f>
              <c:numCache>
                <c:ptCount val="4"/>
                <c:pt idx="0"><c:v>10</c:v></c:pt>
                <c:pt idx="1"><c:v>20</c:v></c:pt>
                <c:pt idx="2"><c:v>15</c:v></c:pt>
                <c:pt idx="3"><c:v>25</c:v></c:pt>
              </c:numCache>
            </c:numRef>
          </c:val>
        </c:ser>
        <c:axId val="1"/>
        <c:axId val="2"/>
      </c:barChart>
      <c:catAx>
        <c:axId val="1"/>
        <c:scaling><c:orientation val="minMax"/></c:scaling>
        <c:delete val="0"/>
        <c:axPos val="b"/>
        <c:crossAx val="2"/>
      </c:catAx>
      <c:valAx>
        <c:axId val="2"/>
        <c:scaling><c:orientation val="minMax"/></c:scaling>
        <c:delete val="0"/>
        <c:axPos val="l"/>
        <c:crossAx val="1"/>
      </c:valAx>
    </c:plotArea>
    <c:legend><c:legendPos val="b"/></c:legend>
    <c:plotVisOnly val="1"/>
  </c:chart>
</c:chartSpace>"#;
            s.embed_chart(chart_xml, 3, 0, 8, 15);
        }),
        "XLSX worksheet with a bar chart (title, legend, category/value axes, one series with cell-range-referenced values/categories and a cached snapshot)",
    );

    println!("Done.");
}
