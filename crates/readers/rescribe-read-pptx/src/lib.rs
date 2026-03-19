//! PPTX (PowerPoint) reader for rescribe.
//!
//! Parses PPTX presentations into rescribe's document IR using ooxml-pml.
//! Each slide becomes a `div` with a `slide` property. Titles become level-1
//! headings, body paragraphs become paragraphs, tables become table nodes,
//! images are stored as resources, and speaker notes become a nested div.

use ooxml_dml::ext::{TextParagraphExt, TextRunExt};
use ooxml_dml::types::{EGTextBullet, TextParagraph};
use ooxml_pml::types::STPlaceholderType;
use ooxml_pml::{PictureExt, Presentation, Shape, ShapeExt};
use rescribe_core::{
    ConversionResult, Document, FidelityWarning, ParseError, ParseOptions, Properties, Resource,
    ResourceId, ResourceMap, Severity, WarningKind,
};
use rescribe_std::{Node, node, prop};
use std::io::Cursor;

/// Parse PPTX input into a document.
pub fn parse(input: &[u8]) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse PPTX input into a document with options.
pub fn parse_with_options(
    input: &[u8],
    _options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let cursor = Cursor::new(input);
    let mut pres = Presentation::from_reader(cursor)
        .map_err(|e| ParseError::Invalid(format!("Invalid PPTX: {}", e)))?;

    let mut doc = Node::new(node::DOCUMENT);
    let mut resources = ResourceMap::new();
    let mut warnings: Vec<FidelityWarning> = Vec::new();

    let slides = pres
        .slides()
        .map_err(|e| ParseError::Invalid(format!("Failed to read slides: {}", e)))?;

    // First pass: collect image data (needs &mut pres and &slide simultaneously).
    // slides is an owned Vec so there's no borrow conflict with pres.
    let mut slide_image_resources: Vec<Vec<(ResourceId, String)>> = vec![Vec::new(); slides.len()];

    for (idx, slide) in slides.iter().enumerate() {
        let mut x_offset: i64 = 0;
        let mut y_offset: i64 = 1600200; // below a typical title

        for pic in slide.pictures() {
            if let Ok(image_data) = pres.get_image_data(slide, pic) {
                let id = ResourceId::new();
                let resource = Resource::new(image_data.content_type.clone(), image_data.data);
                resources.insert(id.clone(), resource);
                let alt = pic.description().unwrap_or("").to_string();
                let _ = (x_offset, y_offset, alt.as_str()); // used below
                slide_image_resources[idx].push((id, alt));
                x_offset += 914400; // 1 inch spacing (not used for reading, just for tracking)
                y_offset += 914400;
            }
        }
    }

    // Second pass: build document nodes.
    for (idx, slide) in slides.iter().enumerate() {
        let slide_num = slide.index() + 1;
        let mut slide_node = Node::new(node::DIV).prop("slide", slide_num as i64);

        // Charts embedded in this slide.
        if !slide.chart_rel_ids().is_empty() {
            warn(
                &mut warnings,
                format!(
                    "Slide {}: {} embedded chart(s) detected; chart data not represented in IR",
                    slide_num,
                    slide.chart_rel_ids().len()
                ),
            );
        }

        // SmartArt diagrams embedded in this slide.
        if !slide.smartart_rel_ids().is_empty() {
            warn(
                &mut warnings,
                format!(
                    "Slide {}: {} SmartArt diagram(s) detected; diagram data not represented in IR",
                    slide_num,
                    slide.smartart_rel_ids().len()
                ),
            );
        }

        // Title shape → heading level 1
        if let Some(title_shape) = slide.shapes().iter().find(|s| is_title_shape(s)) {
            let inline = convert_shape_paragraphs(title_shape);
            if !inline.is_empty() {
                let heading = Node::new(node::HEADING)
                    .prop(prop::LEVEL, 1)
                    .children(inline);
                slide_node = slide_node.child(heading);
            }
        }

        // Body shapes → paragraphs and lists.
        // Consecutive bullet paragraphs are grouped into list/list_item nodes.
        // Ordered vs unordered is detected from the paragraph's bullet type.
        struct BodyPara {
            inline: Vec<Node>,
            is_bullet: bool,
            is_ordered: bool,
        }

        let mut body_paras: Vec<BodyPara> = Vec::new();
        let mut has_nested_bullets = false;

        for shape in slide.shapes() {
            if is_title_shape(shape) {
                continue;
            }
            for pml_para in shape.paragraphs() {
                let inline = convert_pptx_paragraph(pml_para);
                if inline.is_empty() {
                    continue;
                }
                let level = pml_para.level().unwrap_or(0);
                let is_bullet = level > 0 || has_explicit_bullet(pml_para);
                let is_ordered = is_ordered_bullet(pml_para);
                if is_bullet && level > 1 {
                    has_nested_bullets = true;
                }
                body_paras.push(BodyPara {
                    inline,
                    is_bullet,
                    is_ordered,
                });
            }
        }

        // Group consecutive bullet paragraphs into list/list_item nodes.
        let mut pi = 0;
        while pi < body_paras.len() {
            if body_paras[pi].is_bullet {
                // Scan ahead to find end of bullet group and detect ordering.
                let start = pi;
                let mut ordered = false;
                while pi < body_paras.len() && body_paras[pi].is_bullet {
                    if body_paras[pi].is_ordered {
                        ordered = true;
                    }
                    pi += 1;
                }
                let mut list_node = Node::new(node::LIST).prop(prop::ORDERED, ordered);
                for bp in &mut body_paras[start..pi] {
                    let item = Node::new(node::LIST_ITEM)
                        .child(Node::new(node::PARAGRAPH).children(std::mem::take(&mut bp.inline)));
                    list_node = list_node.child(item);
                }
                slide_node = slide_node.child(list_node);
            } else {
                let para =
                    Node::new(node::PARAGRAPH).children(std::mem::take(&mut body_paras[pi].inline));
                slide_node = slide_node.child(para);
                pi += 1;
            }
        }

        if has_nested_bullets {
            warn(
                &mut warnings,
                format!(
                    "Slide {}: nested bullet levels detected; list structure flattened to single level",
                    slide_num
                ),
            );
        }

        // Tables
        for table in slide.tables() {
            let grid = table.to_text_grid();
            if grid.is_empty() {
                continue;
            }
            let mut table_node = Node::new(node::TABLE);
            for row in &grid {
                let mut row_node = Node::new(node::TABLE_ROW);
                for cell_text in row {
                    let cell_node = Node::new(node::TABLE_CELL).child(
                        Node::new(node::PARAGRAPH)
                            .child(Node::new(node::TEXT).prop(prop::CONTENT, cell_text.clone())),
                    );
                    row_node = row_node.child(cell_node);
                }
                table_node = table_node.child(row_node);
            }
            slide_node = slide_node.child(table_node);
        }

        // Images (resources collected in first pass)
        for (resource_id, alt) in &slide_image_resources[idx] {
            let mut img = Node::new(node::IMAGE).prop(prop::URL, resource_id.as_str().to_owned());
            if !alt.is_empty() {
                img = img.prop(prop::ALT, alt.clone());
            }
            slide_node = slide_node.child(img);
        }

        // Speaker notes → nested div with "notes" property.
        // Plain text only; rich text formatting inside notes is not modelled.
        if let Some(notes) = slide.notes() {
            let notes = notes.trim();
            if !notes.is_empty() {
                warn(
                    &mut warnings,
                    format!(
                        "Slide {}: speaker notes rendered as plain text; rich text formatting inside notes not represented in IR",
                        slide_num
                    ),
                );
                let notes_div = Node::new(node::DIV).prop("notes", true).child(
                    Node::new(node::PARAGRAPH)
                        .child(Node::new(node::TEXT).prop(prop::CONTENT, notes.to_string())),
                );
                slide_node = slide_node.child(notes_div);
            }
        }

        if !slide_node.children.is_empty() {
            doc = doc.child(slide_node);
        }
    }

    Ok(ConversionResult::with_warnings(
        Document {
            content: doc,
            resources,
            metadata: Properties::new(),
            source: None,
        },
        warnings,
    ))
}

fn warn(warnings: &mut Vec<FidelityWarning>, message: impl Into<String>) {
    warnings.push(FidelityWarning::new(
        Severity::Minor,
        WarningKind::FeatureLost("pptx".to_string()),
        message,
    ));
}

/// Convert a shape's text paragraphs into a flat list of inline IR nodes.
///
/// Used for title shapes where all paragraphs are combined into one heading.
fn convert_shape_paragraphs(shape: &Shape) -> Vec<Node> {
    let mut inline = Vec::new();
    for para in shape.paragraphs() {
        inline.extend(convert_pptx_paragraph(para));
    }
    inline
}

/// Convert one DML `TextParagraph` into inline IR nodes with run-level formatting.
fn convert_pptx_paragraph(para: &ooxml_dml::types::TextParagraph) -> Vec<Node> {
    let mut nodes = Vec::new();
    for run in para.runs() {
        let text = run.text();
        if text.is_empty() {
            continue;
        }
        let text_node = Node::new(node::TEXT).prop(prop::CONTENT, text.to_string());
        let mut node = text_node;
        // Apply run-level formatting (innermost first, outermost last — same
        // convention as the DOCX reader).
        if run.is_underlined() {
            node = Node::new(node::UNDERLINE).child(node);
        }
        if run.is_italic() {
            node = Node::new(node::EMPHASIS).child(node);
        }
        if run.is_bold() {
            node = Node::new(node::STRONG).child(node);
        }
        nodes.push(node);
    }
    nodes
}

/// Check if a paragraph has an explicit bullet (character or auto-number).
fn has_explicit_bullet(para: &TextParagraph) -> bool {
    para.p_pr.as_ref().is_some_and(|p| {
        p.text_bullet.as_ref().is_some_and(|b| {
            matches!(
                b.as_ref(),
                EGTextBullet::BuChar(_) | EGTextBullet::BuAutoNum(_)
            )
        })
    })
}

/// Check if a paragraph has an ordered (auto-number) bullet.
fn is_ordered_bullet(para: &TextParagraph) -> bool {
    para.p_pr.as_ref().is_some_and(|p| {
        p.text_bullet
            .as_ref()
            .is_some_and(|b| matches!(b.as_ref(), EGTextBullet::BuAutoNum(_)))
    })
}

/// Return true if the shape is a title or centre-title placeholder.
///
/// Checks both the OOXML placeholder type (set by real PowerPoint files) and the
/// shape name "Title" (set by `PresentationBuilder`).
fn is_title_shape(shape: &Shape) -> bool {
    // Check placeholder type attribute (authoritative for Office-generated files).
    if let Some(ph_type) = shape
        .non_visual_properties
        .nv_pr
        .ph
        .as_ref()
        .and_then(|ph| ph.r#type.as_ref())
        && matches!(
            ph_type,
            STPlaceholderType::Title | STPlaceholderType::CtrTitle
        )
    {
        return true;
    }
    // Fallback: shapes written by PresentationBuilder use the name "Title".
    shape.non_visual_properties.c_nv_pr.name == "Title"
}

#[cfg(test)]
mod tests {
    use super::*;
    use ooxml_pml::PresentationBuilder;

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
    /// `cargo test -p rescribe-read-pptx -- generate_bullet_fixture --ignored`
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
}
