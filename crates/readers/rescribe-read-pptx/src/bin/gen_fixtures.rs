/// Generate PPTX fixture files for the rescribe-read-pptx test suite.
///
/// Run with: cargo run -p rescribe-read-pptx --bin gen_fixtures
use ooxml_pml::writer::{PresentationBuilder, TableBuilder, TextRun};
use rescribe_core::Node;
use rescribe_std::node;
use std::io::Cursor;

// ── PPTX construction helpers ──────────────────────────────────────────────

fn make_pptx(build: impl FnOnce(&mut PresentationBuilder)) -> Vec<u8> {
    let mut builder = PresentationBuilder::new();
    build(&mut builder);
    let mut buf = Cursor::new(Vec::new());
    builder.write(&mut buf).unwrap();
    buf.into_inner()
}

// ── Expected JSON generation ───────────────────────────────────────────────

fn node_to_assertions(node: &Node, path: &str, out: &mut Vec<serde_json::Value>) {
    let kind = node.kind.as_str();
    let mut obj = serde_json::json!({ "path": path, "kind": kind });

    let mut props_map = serde_json::Map::new();
    // String props — skip "url" on image nodes (resource ID is non-deterministic)
    let skip_url = kind == node::IMAGE;
    for key in &["content", "url", "alt"] {
        if *key == "url" && skip_url {
            continue;
        }
        if let Some(val) = node.props.get_str(key) {
            props_map.insert(key.to_string(), serde_json::Value::String(val.to_string()));
        }
    }
    // Int props
    if let Some(level) = node.props.get_int("level") {
        props_map.insert("level".to_string(), serde_json::Value::Number(level.into()));
    }
    if let Some(slide) = node.props.get_int("slide") {
        props_map.insert("slide".to_string(), serde_json::Value::Number(slide.into()));
    }
    // Bool props
    if let Some(ordered) = node.props.get_bool("ordered") {
        props_map.insert("ordered".to_string(), serde_json::Value::Bool(ordered));
    }
    if let Some(notes) = node.props.get_bool("notes") {
        props_map.insert("notes".to_string(), serde_json::Value::Bool(notes));
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

fn generate_expected_json(desc: &str, category: &str, pptx_bytes: &[u8]) -> String {
    let result = rescribe_read_pptx::parse(pptx_bytes).expect("parse failed");
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

fn write_fixture(name: &str, pptx_bytes: Vec<u8>, desc: &str) {
    write_fixture_cat(name, pptx_bytes, desc, "happy");
}

fn write_fixture_cat(name: &str, pptx_bytes: Vec<u8>, desc: &str, category: &str) {
    let dir = format!("fixtures/pptx/{name}");
    std::fs::create_dir_all(&dir).unwrap();
    let expected = generate_expected_json(desc, category, &pptx_bytes);
    std::fs::write(format!("{dir}/input.pptx"), &pptx_bytes).unwrap();
    std::fs::write(format!("{dir}/expected.json"), &expected).unwrap();
    println!("wrote {dir}/");
}

fn write_error_fixture(name: &str, pptx_bytes: Vec<u8>, desc: &str) {
    let dir = format!("fixtures/pptx/{name}");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(format!("{dir}/input.pptx"), &pptx_bytes).unwrap();
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

// ── Helpers for XML patching (for bullet-requiring features) ───────────────

/// Patch a PPTX zip: replace a text run placeholder in slide1.xml.
fn patch_slide_xml(pptx_bytes: &[u8], from: &str, to: &str) -> Vec<u8> {
    use std::io::{Read, Write};
    let reader = zip::ZipArchive::new(Cursor::new(pptx_bytes)).unwrap();
    let mut output = Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut output);
        for i in 0..reader.len() {
            let mut cloned = reader.clone();
            let mut file = cloned.by_index(i).unwrap();
            let name = file.name().to_string();
            let mut contents = Vec::new();
            file.read_to_end(&mut contents).unwrap();

            let options = zip::write::SimpleFileOptions::default();
            writer.start_file(&name, options).unwrap();
            if name.contains("slide1.xml") && !name.contains("rels") {
                let xml = String::from_utf8(contents).unwrap();
                writer.write_all(xml.replace(from, to).as_bytes()).unwrap();
            } else {
                writer.write_all(&contents).unwrap();
            }
        }
        writer.finish().unwrap();
    }
    output.into_inner()
}

// ── Main ───────────────────────────────────────────────────────────────────

fn main() {
    // ── Slide structure ───────────────────────────────────────────────────

    // Regenerate the existing slide fixture
    write_fixture(
        "slide",
        make_pptx(|b| {
            let s = b.add_slide();
            s.add_title("Slide Title");
            s.add_text("Content paragraph.");
        }),
        "PPTX slide with title and content paragraph",
    );

    write_fixture(
        "multi-slide",
        make_pptx(|b| {
            let s1 = b.add_slide();
            s1.add_title("First Slide");
            s1.add_text("Introduction content.");
            let s2 = b.add_slide();
            s2.add_title("Second Slide");
            s2.add_text("Main content.");
            let s3 = b.add_slide();
            s3.add_title("Third Slide");
            s3.add_text("Conclusion content.");
        }),
        "PPTX with three slides — each becomes a div[slide] node",
    );

    // ── Text formatting ───────────────────────────────────────────────────

    // Regenerate existing inline-bold fixture
    write_fixture(
        "inline-bold",
        make_pptx(|b| {
            let s = b.add_slide();
            s.add_title("Formatting Test");
            s.add_content_runs(vec![
                TextRun::text("This is "),
                TextRun::text("bold").set_bold(true),
                TextRun::text(" text."),
            ]);
        }),
        "PPTX bold text run mapped to strong node",
    );

    write_fixture(
        "inline-italic",
        make_pptx(|b| {
            let s = b.add_slide();
            s.add_title("Italic Test");
            s.add_content_runs(vec![
                TextRun::text("This is "),
                TextRun::text("italic").set_italic(true),
                TextRun::text(" text."),
            ]);
        }),
        "PPTX italic text run mapped to emphasis node",
    );

    write_fixture(
        "inline-underline",
        make_pptx(|b| {
            let s = b.add_slide();
            s.add_title("Underline Test");
            s.add_content_runs(vec![
                TextRun::text("This is "),
                TextRun::text("underlined").set_underline(true),
                TextRun::text(" text."),
            ]);
        }),
        "PPTX underlined text run mapped to underline node",
    );

    write_fixture(
        "font-size",
        make_pptx(|b| {
            let s = b.add_slide();
            s.add_title("Font Size Test");
            s.add_content_runs(vec![
                TextRun::text("Normal text and "),
                TextRun::text("large text").set_font_size(36.0),
                TextRun::text("."),
            ]);
        }),
        "PPTX text run with explicit font size",
    );

    write_fixture(
        "hyperlink",
        make_pptx(|b| {
            let s = b.add_slide();
            s.add_title("Hyperlink Test");
            s.add_hyperlink("Visit Example", "https://example.com", 2743200, 3810000, 4000000, 600000);
        }),
        "PPTX hyperlink in text run mapped to link node",
    );

    // ── Speaker notes ─────────────────────────────────────────────────────

    // Regenerate existing notes fixture
    write_fixture(
        "notes",
        make_pptx(|b| {
            let s = b.add_slide();
            s.add_title("Slide With Notes");
            s.add_text("Main content.");
            s.set_notes("These are the speaker notes for this slide.");
        }),
        "PPTX speaker notes become a div[notes] node",
    );

    write_fixture(
        "notes-multi-para",
        make_pptx(|b| {
            let s = b.add_slide();
            s.add_title("Multi-Para Notes");
            s.add_text("Content.");
            s.set_notes("First paragraph of notes.\nSecond paragraph of notes.\nThird paragraph.");
        }),
        "PPTX speaker notes with multiple paragraphs produce multiple paragraph nodes",
    );

    // ── Tables ────────────────────────────────────────────────────────────

    // Regenerate existing table fixture
    write_fixture(
        "table",
        make_pptx(|b| {
            let s = b.add_slide();
            s.add_title("Table Slide");
            s.add_table(
                TableBuilder::new()
                    .add_row(["Name", "Age", "City"])
                    .add_row(["Alice", "30", "London"])
                    .add_row(["Bob", "25", "Paris"]),
                914400,
                1600200,
                8229600,
                3000000,
            );
        }),
        "PPTX table shape mapped to table/table_row/table_cell nodes",
    );

    write_fixture(
        "table-multiple-rows",
        make_pptx(|b| {
            let s = b.add_slide();
            s.add_title("Larger Table");
            s.add_table(
                TableBuilder::new()
                    .add_row(["Product", "Price", "Stock"])
                    .add_row(["Widget", "$9.99", "42"])
                    .add_row(["Gadget", "$29.99", "17"])
                    .add_row(["Doohickey", "$4.99", "123"]),
                914400,
                1600200,
                8229600,
                3600000,
            );
        }),
        "PPTX table with 4 data rows",
    );

    // ── Images ────────────────────────────────────────────────────────────

    // Minimal 1×1 red PNG (37 bytes).
    let tiny_png: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a,
        0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
        0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
        0xde, 0x00, 0x00, 0x00, 0x0c, 0x49, 0x44, 0x41,
        0x54, 0x08, 0xd7, 0x63, 0xf8, 0xcf, 0xc0, 0x00,
        0x00, 0x00, 0x02, 0x00, 0x01, 0xe2, 0x21, 0xbc,
        0x33, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e,
        0x44, 0xae, 0x42, 0x60, 0x82,
    ];

    write_fixture(
        "image",
        make_pptx(|b| {
            let s = b.add_slide();
            s.add_title("Image Slide");
            s.add_image(tiny_png.to_vec(), 914400, 1600200, 2743200, 2057400);
        }),
        "PPTX inline image mapped to image node with url property",
    );

    write_fixture(
        "image-alt-text",
        make_pptx(|b| {
            let s = b.add_slide();
            s.add_title("Image With Alt Text");
            s.add_image_with_description(
                tiny_png.to_vec(),
                914400,
                1600200,
                2743200,
                2057400,
                "A decorative red square",
            );
        }),
        "PPTX image with description mapped to image node with alt property",
    );

    // ── Bullet lists (XML patching required) ──────────────────────────────

    let bullet_base = make_pptx(|b| {
        let s = b.add_slide();
        s.add_title("Bullet Slide");
        s.add_text("PLACEHOLDER_BULLETS");
    });

    let bullet_xml = concat!(
        r#"<a:p><a:pPr lvl="1"><a:buChar char="&#x2022;"/></a:pPr>"#,
        r#"<a:r><a:rPr lang="en-US" sz="2400"/><a:t>First bullet</a:t></a:r></a:p>"#,
        r#"<a:p><a:pPr lvl="1"><a:buChar char="&#x2022;"/></a:pPr>"#,
        r#"<a:r><a:rPr lang="en-US" sz="2400"/><a:t>Second bullet</a:t></a:r></a:p>"#,
        r#"<a:p><a:pPr lvl="1"><a:buChar char="&#x2022;"/></a:pPr>"#,
        r#"<a:r><a:rPr lang="en-US" sz="2400"/><a:t>Third bullet</a:t></a:r></a:p>"#,
    );

    write_fixture(
        "bullets",
        patch_slide_xml(
            &bullet_base,
            r#"<a:p><a:r><a:rPr lang="en-US" sz="2400"/><a:t>PLACEHOLDER_BULLETS</a:t></a:r></a:p>"#,
            bullet_xml,
        ),
        "PPTX bullet paragraphs grouped into list/list_item nodes",
    );

    let numbered_xml = concat!(
        r#"<a:p><a:pPr lvl="1"><a:buAutoNum type="arabicPeriod"/></a:pPr>"#,
        r#"<a:r><a:rPr lang="en-US" sz="2400"/><a:t>First item</a:t></a:r></a:p>"#,
        r#"<a:p><a:pPr lvl="1"><a:buAutoNum type="arabicPeriod"/></a:pPr>"#,
        r#"<a:r><a:rPr lang="en-US" sz="2400"/><a:t>Second item</a:t></a:r></a:p>"#,
        r#"<a:p><a:pPr lvl="1"><a:buAutoNum type="arabicPeriod"/></a:pPr>"#,
        r#"<a:r><a:rPr lang="en-US" sz="2400"/><a:t>Third item</a:t></a:r></a:p>"#,
    );

    let numbered_base = make_pptx(|b| {
        let s = b.add_slide();
        s.add_title("Numbered List");
        s.add_text("PLACEHOLDER_NUMBERED");
    });

    write_fixture(
        "numbered-list",
        patch_slide_xml(
            &numbered_base,
            r#"<a:p><a:r><a:rPr lang="en-US" sz="2400"/><a:t>PLACEHOLDER_NUMBERED</a:t></a:r></a:p>"#,
            numbered_xml,
        ),
        "PPTX numbered (auto-numbered) paragraphs grouped into ordered list",
    );

    // ── Adversarial ───────────────────────────────────────────────────────

    // Empty presentation (zero slides)
    write_fixture_cat(
        "adv-empty-presentation",
        make_pptx(|_b| {}),
        "PPTX presentation with zero slides produces empty document",
        "adversarial",
    );

    // Malformed ZIP (not a valid ZIP archive)
    write_error_fixture(
        "adv-malformed-zip",
        b"not a zip file at all".to_vec(),
        "Malformed zip bytes return a parse error without panic",
    );

    // Empty bytes
    write_error_fixture(
        "adv-empty-bytes",
        b"".to_vec(),
        "Empty input bytes return a parse error without panic",
    );

    // ── Pathological ──────────────────────────────────────────────────────

    write_fixture_cat(
        "path-many-slides",
        make_pptx(|b| {
            for i in 1..=20u32 {
                let s = b.add_slide();
                s.add_title(format!("Slide {i}"));
                s.add_text(format!("Content of slide {i}."));
            }
        }),
        "PPTX with 20 slides — all parsed without panic",
        "pathological",
    );

    write_fixture_cat(
        "path-many-text-runs",
        make_pptx(|b| {
            let s = b.add_slide();
            s.add_title("Many Text Runs");
            let runs: Vec<TextRun> = (0..50)
                .flat_map(|i| {
                    vec![
                        TextRun::text(format!("word{i} ")),
                        TextRun::text(format!("bold{i} ")).set_bold(true),
                    ]
                })
                .collect();
            s.add_content_runs(runs);
        }),
        "PPTX slide with 100 text runs (alternating plain and bold)",
        "pathological",
    );

    // ── Composition (integration) ─────────────────────────────────────────

    write_fixture(
        "slide-with-title-and-table",
        make_pptx(|b| {
            let s = b.add_slide();
            s.add_title("Summary");
            s.add_table(
                TableBuilder::new()
                    .add_row(["Item", "Status"])
                    .add_row(["Task A", "Done"])
                    .add_row(["Task B", "In Progress"]),
                914400,
                1600200,
                6000000,
                2400000,
            );
        }),
        "PPTX slide combining title and table",
    );

    write_fixture(
        "multi-slide-with-notes",
        make_pptx(|b| {
            let s1 = b.add_slide();
            s1.add_title("Slide One");
            s1.add_text("First slide content.");
            s1.set_notes("Notes for slide one.");
            let s2 = b.add_slide();
            s2.add_title("Slide Two");
            s2.add_text("Second slide content.");
            s2.set_notes("Notes for slide two.");
        }),
        "PPTX multi-slide presentation where each slide has speaker notes",
    );

    write_fixture(
        "mixed-formatting",
        make_pptx(|b| {
            let s = b.add_slide();
            s.add_title("Formatting Mix");
            s.add_content_runs(vec![
                TextRun::text("Plain, "),
                TextRun::text("bold").set_bold(true),
                TextRun::text(", "),
                TextRun::text("italic").set_italic(true),
                TextRun::text(", "),
                TextRun::text("underline").set_underline(true),
                TextRun::text(", and "),
                TextRun::text("bold-italic").set_bold(true).set_italic(true),
                TextRun::text("."),
            ]);
        }),
        "PPTX slide with mixed inline formatting in a single text shape",
    );

    println!("Done.");
}
