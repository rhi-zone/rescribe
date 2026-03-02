//! PPTX (PowerPoint) reader for rescribe.
//!
//! Parses PPTX presentations into rescribe's document IR using ooxml-pml.
//! Each slide becomes a `div` with a `slide` property. Titles become level-1
//! headings, body paragraphs become paragraphs, tables become table nodes,
//! images are stored as resources, and speaker notes become a nested div.

use ooxml_dml::ext::{TextParagraphExt, TextRunExt};
use ooxml_pml::types::STPlaceholderType;
use ooxml_pml::{PictureExt, Presentation, Shape, ShapeExt};
use rescribe_core::{
    ConversionResult, Document, ParseError, ParseOptions, Properties, Resource, ResourceId,
    ResourceMap,
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

        // Body shapes → paragraphs (with run-level formatting)
        for shape in slide.shapes() {
            if is_title_shape(shape) {
                continue;
            }
            for pml_para in shape.paragraphs() {
                let inline = convert_pptx_paragraph(pml_para);
                if !inline.is_empty() {
                    let para = Node::new(node::PARAGRAPH).children(inline);
                    slide_node = slide_node.child(para);
                }
            }
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

        // Speaker notes → nested div with "notes" property
        if let Some(notes) = slide.notes() {
            let notes = notes.trim();
            if !notes.is_empty() {
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

    Ok(ConversionResult::ok(Document {
        content: doc,
        resources,
        metadata: Properties::new(),
        source: None,
    }))
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
}
