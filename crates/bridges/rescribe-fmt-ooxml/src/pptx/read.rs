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

        // Charts embedded in this slide (ADR 0016). PPTX chart parts
        // (`ppt/charts/chartN.xml`) share the exact same DrawingML
        // `<c:chartSpace>` schema as XLSX chart parts, so this reuses
        // `ooxml_sml::parse_chart_xml` (the same hand-rolled walker
        // `xlsx.rs` uses) rather than a second, duplicated parser built on
        // `ooxml-pml`'s generated `dml-charts` model (out of scope for this
        // pass — see ADR 0016 Consequences). The `chart` node is appended
        // as a child of this slide's `div` node: a chart is block-level
        // (ADR 0016) slide content, unlike xlsx's chart-to-sheet sibling
        // placement, which exists there only because a `sheet` node's
        // children are constrained to `sheet_row`s.
        for rel_id in slide.chart_rel_ids() {
            match pres.get_chart_xml(slide, rel_id) {
                Ok(xml) => match ooxml_sml::parse_chart_xml(&xml) {
                    Ok(chart) => {
                        slide_node = slide_node.child(crate::chart::convert_chart(&chart));
                    }
                    Err(e) => warn(
                        &mut warnings,
                        format!(
                            "Slide {}: failed to parse embedded chart (rel {}): {}",
                            slide_num, rel_id, e
                        ),
                    ),
                },
                Err(e) => warn(
                    &mut warnings,
                    format!(
                        "Slide {}: failed to read embedded chart part (rel {}): {}",
                        slide_num, rel_id, e
                    ),
                ),
            }
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
