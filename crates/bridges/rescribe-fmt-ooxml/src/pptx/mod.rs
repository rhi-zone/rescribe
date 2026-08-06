//! PPTX (PowerPoint) reader + writer for rescribe.
//!
//! Translates between PowerPoint presentations (.pptx) and rescribe's
//! document IR using `ooxml-pml` and `ooxml-dml`.
//!
//! # Reader
//!
//! Each slide becomes a `div` with a `slide` property. Titles become
//! level-1 headings, body paragraphs become paragraphs, tables become
//! table nodes, images are stored as resources, and speaker notes become a
//! nested div.
//!
//! # Writer
//!
//! Two input layouts are supported:
//! - **Structured**: top-level `div` nodes that carry a `slide` property.
//!   Each such div is mapped to one slide.
//! - **Flat**: any other structure is split on level-1 headings, each
//!   heading starting a new slide.

mod read;
mod write;

pub use read::{parse, parse_with_options};
pub use write::{emit, emit_with_options};

#[cfg(test)]
mod tests {
    use super::*;
    use ooxml_pml::PresentationBuilder;
    use rescribe_std::node;
    use std::io::Cursor;

    /// Build a minimal valid PPTX using PresentationBuilder so we have no extra deps.
    fn create_test_pptx() -> Vec<u8> {
        let mut builder = PresentationBuilder::new();
        let slide = builder.add_slide();
        slide.add_title("Test Title");
        slide.add_text("Content text");
        let mut buf = Cursor::new(Vec::new());
        builder.write(&mut buf).unwrap();
        buf.into_inner()
    }

    /// Build a PPTX with bullet paragraphs by patching slide XML.
    ///
    /// PresentationBuilder doesn't support bullet properties, so we create a
    /// basic PPTX, then modify the slide XML in the zip to add `<a:pPr>` with
    /// `<a:buChar>` attributes.
    fn create_bullet_pptx() -> Vec<u8> {
        let mut builder = PresentationBuilder::new();
        let slide = builder.add_slide();
        slide.add_title("Bullet Slide");
        // Add text that we'll replace with bulleted paragraphs.
        slide.add_text("PLACEHOLDER_BULLETS");
        let mut buf = Cursor::new(Vec::new());
        builder.write(&mut buf).unwrap();
        let pptx_bytes = buf.into_inner();

        // Read the zip and replace the slide XML.
        use std::io::{Read, Write};
        let reader = zip::ZipArchive::new(Cursor::new(&pptx_bytes)).unwrap();
        let mut output = Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut output);
            for i in 0..reader.len() {
                let mut cloned = reader.clone();
                let mut file = cloned.by_index(i).unwrap();
                let name = file.name().to_string();
                let mut contents = Vec::new();
                file.read_to_end(&mut contents).unwrap();

                if name.contains("slide1.xml") && !name.contains("rels") {
                    // Replace the placeholder text element with bullet paragraphs.
                    let xml = String::from_utf8(contents).unwrap();
                    let bullet_xml = r#"<a:p><a:pPr lvl="1"><a:buChar char="•"/></a:pPr><a:r><a:rPr lang="en-US" sz="2400"/><a:t>First bullet</a:t></a:r></a:p><a:p><a:pPr lvl="1"><a:buChar char="•"/></a:pPr><a:r><a:rPr lang="en-US" sz="2400"/><a:t>Second bullet</a:t></a:r></a:p><a:p><a:pPr lvl="1"><a:buChar char="•"/></a:pPr><a:r><a:rPr lang="en-US" sz="2400"/><a:t>Third bullet</a:t></a:r></a:p>"#;
                    let xml = xml.replace(
                        r#"<a:p><a:r><a:rPr lang="en-US" sz="2400"/><a:t>PLACEHOLDER_BULLETS</a:t></a:r></a:p>"#,
                        bullet_xml,
                    );
                    let options = zip::write::SimpleFileOptions::default();
                    writer.start_file(&name, options).unwrap();
                    writer.write_all(xml.as_bytes()).unwrap();
                } else {
                    let options = zip::write::SimpleFileOptions::default();
                    writer.start_file(&name, options).unwrap();
                    writer.write_all(&contents).unwrap();
                }
            }
            writer.finish().unwrap();
        }
        output.into_inner()
    }

    #[test]
    fn test_parse_basic() {
        let pptx = create_test_pptx();
        let result = parse(&pptx).unwrap();
        let doc = &result.value;
        // Should have at least one slide div
        assert!(!doc.content.children.is_empty());
        let slide = &doc.content.children[0];
        assert_eq!(slide.kind.as_str(), node::DIV);
        // Should have a heading for the title
        assert!(
            slide
                .children
                .iter()
                .any(|c| c.kind.as_str() == node::HEADING)
        );
        // Should have a paragraph for the content
        assert!(
            slide
                .children
                .iter()
                .any(|c| c.kind.as_str() == node::PARAGRAPH)
        );
    }

    #[test]
    fn test_parse_bullets() {
        let pptx = create_bullet_pptx();
        let result = parse(&pptx).unwrap();
        let doc = &result.value;
        let slide = &doc.content.children[0];

        // Should have a heading
        assert_eq!(slide.children[0].kind.as_str(), node::HEADING);

        // Should have a list node (not flat paragraphs)
        let list = slide
            .children
            .iter()
            .find(|c| c.kind.as_str() == node::LIST)
            .expect("Expected a list node for bullet paragraphs");

        // List should be unordered (buChar = character bullets)
        assert_eq!(
            list.props.get("ordered"),
            Some(&rescribe_core::PropValue::Bool(false))
        );

        // Should have 3 list items
        assert_eq!(list.children.len(), 3);
        for item in &list.children {
            assert_eq!(item.kind.as_str(), node::LIST_ITEM);
            // Each item should contain a paragraph
            assert_eq!(item.children[0].kind.as_str(), node::PARAGRAPH);
        }

        // Verify text content
        let first_item_para = &list.children[0].children[0];
        let text_node = &first_item_para.children[0];
        assert_eq!(
            text_node.props.get("content"),
            Some(&rescribe_core::PropValue::String(
                "First bullet".to_string()
            ))
        );
    }

    /// Generate the bullet fixture PPTX file. Run manually:
    /// `cargo test -p rescribe-fmt-ooxml --features pptx -- generate_bullet_fixture --ignored`
    #[test]
    #[ignore]
    fn generate_bullet_fixture() {
        let pptx = create_bullet_pptx();
        let fixture_dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/pptx/bullets");
        std::fs::create_dir_all(&fixture_dir).unwrap();
        std::fs::write(fixture_dir.join("input.pptx"), &pptx).unwrap();
        eprintln!("Wrote fixture to {}", fixture_dir.display());
    }

    #[test]
    fn test_emit_basic() {
        use rescribe_std::builder::*;

        let document = doc(|d| {
            d.heading(1, |h| h.text("Slide 1"))
                .para(|p| p.text("Content 1"))
                .heading(1, |h| h.text("Slide 2"))
                .para(|p| p.text("Content 2"))
        });
        let result = emit(&document).unwrap();
        assert!(!result.value.is_empty());
        // Check it's a valid ZIP starting with PK
        assert_eq!(&result.value[0..2], b"PK");
    }

    #[test]
    fn test_table_xml_special_chars_roundtrip() {
        use rescribe_core::Document as RescribeDocument;
        use rescribe_std::Node as RescribeNode;
        use rescribe_std::prop;

        // Build a structured slide with a table containing XML special chars.
        let table = RescribeNode::new(node::TABLE).child(
            RescribeNode::new(node::TABLE_ROW)
                .child(
                    RescribeNode::new(node::TABLE_CELL).child(
                        RescribeNode::new(node::PARAGRAPH)
                            .child(RescribeNode::new(node::TEXT).prop(prop::CONTENT, "a>b")),
                    ),
                )
                .child(
                    RescribeNode::new(node::TABLE_CELL).child(
                        RescribeNode::new(node::PARAGRAPH)
                            .child(RescribeNode::new(node::TEXT).prop(prop::CONTENT, "c&d")),
                    ),
                )
                .child(
                    RescribeNode::new(node::TABLE_CELL).child(
                        RescribeNode::new(node::PARAGRAPH)
                            .child(RescribeNode::new(node::TEXT).prop(prop::CONTENT, "e'f")),
                    ),
                ),
        );
        let slide_div = RescribeNode::new(node::DIV)
            .prop("slide", 1i64)
            .child(
                RescribeNode::new(node::HEADING)
                    .prop(prop::LEVEL, 1i64)
                    .child(RescribeNode::new(node::TEXT).prop(prop::CONTENT, "Table Test")),
            )
            .child(table);
        let root = RescribeNode::new(node::DOCUMENT).child(slide_div);
        let document = RescribeDocument::new().with_content(root);

        // Emit to PPTX
        let emit_result = emit(&document).unwrap();
        assert!(!emit_result.value.is_empty());

        // Parse back
        let parse_result = parse(&emit_result.value).unwrap();

        // Extract all text from both
        fn extract_text(node: &RescribeNode) -> String {
            let mut text = String::new();
            if node.kind.as_str() == node::TEXT
                && let Some(content) = node.props.get_str(prop::CONTENT)
            {
                text.push_str(content);
            }
            for child in &node.children {
                text.push_str(&extract_text(child));
            }
            text
        }

        let text_before = extract_text(&document.content);
        let text_after = extract_text(&parse_result.value.content);
        assert_eq!(
            text_before, text_after,
            "Table text with XML special chars should roundtrip"
        );
    }

    #[test]
    fn test_emit_structured() {
        use rescribe_core::Document as RescribeDocument;
        use rescribe_std::Node as RescribeNode;
        use rescribe_std::prop;

        // Build the document directly with Node primitives.
        let slide_div = RescribeNode::new(node::DIV)
            .prop("slide", 1i64)
            .child(
                RescribeNode::new(node::HEADING)
                    .prop(prop::LEVEL, 1i64)
                    .child(RescribeNode::new(node::TEXT).prop(prop::CONTENT, "Slide One")),
            )
            .child(
                RescribeNode::new(node::PARAGRAPH)
                    .child(RescribeNode::new(node::TEXT).prop(prop::CONTENT, "Body text")),
            );
        let root = RescribeNode::new(node::DOCUMENT).child(slide_div);
        let document = RescribeDocument::new().with_content(root);

        let result = emit(&document).unwrap();
        assert!(!result.value.is_empty());
        assert_eq!(&result.value[0..2], b"PK");
    }
}
