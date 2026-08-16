use ooxml_pml::{PresentationBuilder, TableBuilder};
use rescribe_core::{
    ConversionResult, Document, EmitError, EmitOptions, FidelityWarning, Node, Severity,
    WarningKind,
};
use rescribe_std::{node, prop};
use std::io::Cursor;

/// Emit a document to PPTX.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    emit_with_options(doc, &EmitOptions::default())
}

/// Emit a document to PPTX with options.
pub fn emit_with_options(
    doc: &Document,
    _options: &EmitOptions,
) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let mut builder = PresentationBuilder::new();
    let mut warnings: Vec<FidelityWarning> = Vec::new();

    // Check whether the document uses the structured `div[slide]` layout.
    let has_slide_divs = doc
        .content
        .children
        .iter()
        .any(|n| n.kind.as_str() == node::DIV && n.props.get("slide").is_some());

    if has_slide_divs {
        emit_structured(&mut builder, &doc.content.children, doc, &mut warnings);
    } else {
        emit_flat(&mut builder, &doc.content.children, &mut warnings);
    }

    let mut cursor = Cursor::new(Vec::new());
    builder
        .write(&mut cursor)
        .map_err(|e| EmitError::Io(std::io::Error::other(e.to_string())))?;

    Ok(ConversionResult::with_warnings(
        cursor.into_inner(),
        warnings,
    ))
}

// ---------------------------------------------------------------------------
// Structured layout: one div[slide] → one slide
// ---------------------------------------------------------------------------

fn emit_structured(
    builder: &mut PresentationBuilder,
    nodes: &[Node],
    doc: &Document,
    warnings: &mut Vec<FidelityWarning>,
) {
    for node in nodes {
        if node.kind.as_str() == node::DIV && node.props.get("slide").is_some() {
            let slide = builder.add_slide();
            emit_slide_children(slide, &node.children, doc, warnings);
        }
    }
}

fn emit_slide_children(
    slide: &mut ooxml_pml::SlideBuilder,
    children: &[Node],
    doc: &Document,
    warnings: &mut Vec<FidelityWarning>,
) {
    for child in children {
        match child.kind.as_str() {
            k if k == node::HEADING => {
                let level = child.props.get_int(prop::LEVEL).unwrap_or(1);
                if level == 1 {
                    slide.add_title(get_text_content(child));
                } else {
                    slide.add_text(get_text_content(child));
                }
            }
            k if k == node::PARAGRAPH => {
                let text = get_text_content(child);
                if !text.is_empty() {
                    slide.add_text(text);
                }
            }
            k if k == node::TABLE => {
                emit_table(slide, child);
            }
            k if k == node::IMAGE => {
                emit_image(slide, child, doc);
            }
            k if k == node::DIV => {
                // Notes div
                if child.props.get("notes").is_some() {
                    let notes_text = collect_text_nodes(child);
                    if !notes_text.is_empty() {
                        slide.set_notes(notes_text);
                    }
                } else {
                    // Nested div without slide prop — recurse.
                    emit_slide_children(slide, &child.children, doc, warnings);
                }
            }
            k if k == node::LIST => {
                warn(
                    warnings,
                    "List structure flattened to bullet-prefixed paragraphs; PresentationBuilder lacks bullet/numbering API",
                );
                emit_list(slide, child);
            }
            k if k == node::CODE_BLOCK => {
                if let Some(content) = child.props.get_str(prop::CONTENT) {
                    slide.add_text(content.to_string());
                }
            }
            k if k == node::CHART => {
                emit_chart(slide, child);
            }
            _ => {
                // Fallback: extract any text.
                let text = get_text_content(child);
                if !text.is_empty() {
                    slide.add_text(text);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Flat layout: split on level-1 headings
// ---------------------------------------------------------------------------

fn emit_flat(
    builder: &mut PresentationBuilder,
    nodes: &[Node],
    warnings: &mut Vec<FidelityWarning>,
) {
    struct FlatSlide<'a> {
        title: String,
        content: Vec<&'a Node>,
    }

    let mut slides: Vec<FlatSlide> = Vec::new();
    let mut current_title = String::new();
    let mut current_content: Vec<&Node> = Vec::new();

    for node in nodes {
        if node.kind.as_str() == node::HEADING && node.props.get_int(prop::LEVEL).unwrap_or(1) == 1
        {
            if !current_content.is_empty() || !current_title.is_empty() {
                slides.push(FlatSlide {
                    title: current_title,
                    content: current_content,
                });
                current_content = Vec::new();
            }
            current_title = get_text_content(node);
        } else {
            current_content.push(node);
        }
    }
    if !current_content.is_empty() || !current_title.is_empty() {
        slides.push(FlatSlide {
            title: current_title,
            content: current_content,
        });
    }

    for s in &slides {
        let slide = builder.add_slide();
        if !s.title.is_empty() {
            slide.add_title(s.title.clone());
        }
        for node in &s.content {
            match node.kind.as_str() {
                k if k == node::PARAGRAPH => {
                    let text = get_text_content(node);
                    if !text.is_empty() {
                        slide.add_text(text);
                    }
                }
                k if k == node::TABLE => {
                    emit_table(slide, node);
                }
                k if k == node::LIST => {
                    warn(
                        warnings,
                        "List structure flattened to bullet-prefixed paragraphs; PresentationBuilder lacks bullet/numbering API",
                    );
                    emit_list(slide, node);
                }
                k if k == node::CODE_BLOCK => {
                    if let Some(content) = node.props.get_str(prop::CONTENT) {
                        slide.add_text(content.to_string());
                    }
                }
                _ => {
                    let text = get_text_content(node);
                    if !text.is_empty() {
                        slide.add_text(text);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn warn(warnings: &mut Vec<FidelityWarning>, message: impl Into<String>) {
    warnings.push(FidelityWarning::new(
        Severity::Minor,
        WarningKind::FeatureLost("pptx".to_string()),
        message,
    ));
}

fn emit_table(slide: &mut ooxml_pml::SlideBuilder, table_node: &Node) {
    let mut table = TableBuilder::new();
    for row_node in &table_node.children {
        if row_node.kind.as_str() == node::TABLE_ROW || row_node.kind.as_str() == node::TABLE_HEADER
        {
            let cells: Vec<String> = row_node
                .children
                .iter()
                .filter(|c| {
                    c.kind.as_str() == node::TABLE_CELL || c.kind.as_str() == node::TABLE_HEADER
                })
                .map(get_text_content)
                .collect();
            if !cells.is_empty() {
                table = table.add_row(cells);
            }
        }
    }
    if table.row_count() > 0 {
        // Default position: title area + some margin, full width
        slide.add_table(table, 457200, 1600200, 8229600, 4000000);
    }
}

/// Write a `chart` node (ADR 0016) to the slide. Per ADR 0016 Decision 4, an
/// OOXML-sourced `chart` node always carries its original chart-part XML
/// verbatim (`prop::OOXML_CHART_XML`), so the common case is a byte-exact
/// re-emit; only a chart node with no raw XML at all (constructed
/// programmatically, or from a non-OOXML source) falls back to
/// [`crate::chart::build_minimal_chart_xml`]'s best-effort generator — see
/// that function's doc comment for its (documented) v1 semantic-core scope.
///
/// Like the xlsx writer, the chart's original anchor position isn't
/// captured anywhere in the `chart`/`chart_series` IR shape, so this uses a
/// fixed default anchor (same values `emit_table` above uses for its own
/// default placement) rather than the original location — a known,
/// documented v1 gap, not a silent drop of the chart itself.
fn emit_chart(slide: &mut ooxml_pml::SlideBuilder, chart_node: &Node) {
    let chart_xml: Vec<u8> = match chart_node.props.get_str(prop::OOXML_CHART_XML) {
        Some(xml) => xml.as_bytes().to_vec(),
        None => crate::chart::build_minimal_chart_xml(chart_node).into_bytes(),
    };
    slide.embed_chart(chart_xml, 457200, 1600200, 8229600, 4000000);
}

fn emit_list(slide: &mut ooxml_pml::SlideBuilder, list_node: &Node) {
    for item in &list_node.children {
        if item.kind.as_str() == node::LIST_ITEM {
            let text = format!("• {}", get_text_content(item));
            slide.add_text(text);
        }
    }
}

fn emit_image(slide: &mut ooxml_pml::SlideBuilder, image_node: &Node, doc: &Document) {
    // Look up resource by URL prop (which holds the ResourceId string).
    let resource_id_str = match image_node.props.get_str(prop::URL) {
        Some(s) => s.to_string(),
        None => return,
    };
    // Find the resource by matching its ID string.
    for (id, resource) in &doc.resources {
        if id.as_str() == resource_id_str {
            // Default position and size (full-width, in content area).
            slide.add_image(resource.data.clone(), 457200, 1600200, 4000000, 3000000);
            return;
        }
    }
}

/// Collect all text content from a node tree as a single String (no separator).
fn get_text_content(node: &Node) -> String {
    let mut buf = String::new();
    collect_text(node, &mut buf);
    buf
}

fn collect_text(node: &Node, out: &mut String) {
    if node.kind.as_str() == node::TEXT
        && let Some(content) = node.props.get_str(prop::CONTENT)
    {
        out.push_str(content);
    }
    for child in &node.children {
        collect_text(child, out);
    }
}

/// Collect all text from a node tree joined with newlines.
fn collect_text_nodes(node: &Node) -> String {
    let mut texts: Vec<String> = Vec::new();
    collect_text_paragraphs(node, &mut texts);
    texts.join("\n")
}

fn collect_text_paragraphs(node: &Node, out: &mut Vec<String>) {
    if node.kind.as_str() == node::PARAGRAPH || node.kind.as_str() == node::HEADING {
        let text = get_text_content(node);
        if !text.is_empty() {
            out.push(text);
        }
    } else {
        for child in &node.children {
            collect_text_paragraphs(child, out);
        }
    }
}
