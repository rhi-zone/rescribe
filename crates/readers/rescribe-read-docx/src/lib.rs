//! DOCX (Word) reader for rescribe.
//!
//! Parses Word documents (.docx) into rescribe's document IR using the ooxml-wml crate.
//!
//! # Example
//!
//! ```ignore
//! use rescribe_read_docx::parse_file;
//!
//! let result = parse_file("document.docx")?;
//! let doc = result.value;
//! // Process the document...
//! ```

use ooxml_wml::Document as OoxmlDocument;
use ooxml_wml::ext::{
    CellExt, DrawingExt, HyperlinkExt, ParagraphExt, RowExt, RunExt, RunPropertiesExt, TableExt,
};
use ooxml_wml::types::{
    BlockContent, BlockContentChoice, Hyperlink, Paragraph, ParagraphContent, Run, RunContent,
    RunContentChoice, Table,
};
use rescribe_core::{
    ConversionResult, Document, FidelityWarning, Node, ParseError, Properties, Resource,
    ResourceId, ResourceMap, Severity, SourceInfo, WarningKind,
};
use rescribe_std::{node, prop};
use std::io::{Read, Seek};
use std::path::Path;

/// Parse a DOCX file from a path.
pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<ConversionResult<Document>, ParseError> {
    let doc = OoxmlDocument::open(path)
        .map_err(|e| ParseError::Invalid(format!("Failed to open DOCX: {}", e)))?;
    convert_document(doc)
}

/// Parse DOCX from a reader that implements Read + Seek.
pub fn parse<R: Read + Seek>(reader: R) -> Result<ConversionResult<Document>, ParseError> {
    let doc = OoxmlDocument::from_reader(reader)
        .map_err(|e| ParseError::Invalid(format!("Failed to parse DOCX: {}", e)))?;
    convert_document(doc)
}

/// Parse DOCX from bytes.
pub fn parse_bytes(bytes: &[u8]) -> Result<ConversionResult<Document>, ParseError> {
    let cursor = std::io::Cursor::new(bytes);
    parse(cursor)
}

/// Converter state for tracking resources and warnings during conversion.
struct Converter {
    warnings: Vec<FidelityWarning>,
    resources: ResourceMap,
}

impl Converter {
    fn new() -> Self {
        Self {
            warnings: Vec::new(),
            resources: ResourceMap::new(),
        }
    }

    fn warn(&mut self, message: impl Into<String>) {
        self.warnings.push(FidelityWarning::new(
            Severity::Minor,
            WarningKind::FeatureLost("docx".to_string()),
            message,
        ));
    }

    fn warn_lost(&mut self, message: impl Into<String>) {
        self.warnings.push(FidelityWarning::new(
            Severity::Major,
            WarningKind::FeatureLost("docx".to_string()),
            message,
        ));
    }

    fn add_resource(&mut self, data: Vec<u8>, content_type: &str) -> ResourceId {
        let id = ResourceId::new();
        let resource = Resource::new(content_type.to_string(), data);
        self.resources.insert(id.clone(), resource);
        id
    }
}

fn convert_document<R: Read + Seek>(
    mut doc: OoxmlDocument<R>,
) -> Result<ConversionResult<Document>, ParseError> {
    let mut converter = Converter::new();

    // Clone the body content to avoid borrow issues
    let body_content = doc.body().block_content.clone();

    // Convert body content
    let content = convert_body(&mut converter, &mut doc, &body_content)?;

    // Extract metadata
    let metadata = extract_metadata(&doc);

    // Build the final document
    let document = Document {
        content,
        resources: converter.resources,
        metadata,
        source: Some(SourceInfo {
            format: "docx".to_string(),
            metadata: Properties::new(),
        }),
    };

    Ok(ConversionResult {
        value: document,
        warnings: converter.warnings,
    })
}

fn convert_body<R: Read + Seek>(
    converter: &mut Converter,
    doc: &mut OoxmlDocument<R>,
    content: &[BlockContent],
) -> Result<Node, ParseError> {
    let mut children = Vec::new();

    for block in content {
        match block {
            BlockContent::P(para) => {
                if let Some(node) = convert_paragraph(converter, doc, para)? {
                    children.push(node);
                }
            }
            BlockContent::Tbl(table) => {
                children.push(convert_table(converter, doc, table)?);
            }
            BlockContent::Sdt(ctrl) => {
                if let Some(content) = &ctrl.sdt_content {
                    for inner_block in &content.block_content {
                        match inner_block {
                            BlockContentChoice::P(para) => {
                                if let Some(node) = convert_paragraph(converter, doc, para)? {
                                    children.push(node);
                                }
                            }
                            BlockContentChoice::Tbl(table) => {
                                children.push(convert_table(converter, doc, table)?);
                            }
                            _ => {}
                        }
                    }
                }
            }
            BlockContent::CustomXml(xml) => {
                for inner_block in &xml.block_content {
                    match inner_block {
                        BlockContentChoice::P(para) => {
                            if let Some(node) = convert_paragraph(converter, doc, para)? {
                                children.push(node);
                            }
                        }
                        BlockContentChoice::Tbl(table) => {
                            children.push(convert_table(converter, doc, table)?);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    Ok(Node::new(node::DOCUMENT).children(children))
}

fn convert_paragraph<R: Read + Seek>(
    converter: &mut Converter,
    doc: &mut OoxmlDocument<R>,
    para: &Paragraph,
) -> Result<Option<Node>, ParseError> {
    // Check if this is a heading
    let heading_level = detect_heading_level(para);

    // Convert paragraph content
    let inline_children = convert_paragraph_content(converter, doc, para)?;

    // Skip empty paragraphs (unless they have special meaning)
    if inline_children.is_empty() {
        return Ok(None);
    }

    if let Some(level) = heading_level {
        // Create heading node
        Ok(Some(
            Node::new(node::HEADING)
                .prop(prop::LEVEL, level as i64)
                .children(inline_children),
        ))
    } else {
        // Create paragraph node
        Ok(Some(Node::new(node::PARAGRAPH).children(inline_children)))
    }
}

fn detect_heading_level(para: &Paragraph) -> Option<u8> {
    // Check for outline level in paragraph properties
    if let Some(props) = para.properties() {
        // Outline level 0-8 maps to heading levels 1-9
        if let Some(outline) = &props.outline_lvl {
            let level = outline.value as u8;
            return Some(level + 1);
        }

        // Check style name for heading patterns
        if let Some(style) = &props.paragraph_style {
            let style_lower = style.value.to_lowercase();
            if style_lower.starts_with("heading") || style_lower.starts_with("titre") {
                // Try to extract number from style name
                for c in style_lower.chars() {
                    if let Some(digit) = c.to_digit(10)
                        && (1..=9).contains(&digit)
                    {
                        return Some(digit as u8);
                    }
                }
            }
        }
    }

    None
}

fn convert_paragraph_content<R: Read + Seek>(
    converter: &mut Converter,
    doc: &mut OoxmlDocument<R>,
    para: &Paragraph,
) -> Result<Vec<Node>, ParseError> {
    let mut children = Vec::new();

    for content in &para.paragraph_content {
        match content {
            ParagraphContent::R(run) => {
                if let Some(node) = convert_run(converter, doc, run)? {
                    children.push(node);
                }
            }
            ParagraphContent::Hyperlink(link) => {
                if let Some(node) = convert_hyperlink(converter, doc, link)? {
                    children.push(node);
                }
            }
            ParagraphContent::Ins(ins) => {
                // Include inserted content (from tracked changes)
                for item in &ins.run_content {
                    if let RunContentChoice::R(run) = item
                        && let Some(node) = convert_run(converter, doc, run)?
                    {
                        children.push(node);
                    }
                }
            }
            ParagraphContent::Del(_del) => {
                // Skip deleted content (tracked changes)
                converter.warn("Tracked deletion content skipped");
            }
            ParagraphContent::FldSimple(field) => {
                // Extract displayed text from simple fields
                for item in &field.paragraph_content {
                    if let ParagraphContent::R(run) = item
                        && let Some(node) = convert_run(converter, doc, run)?
                    {
                        children.push(node);
                    }
                }
            }
            ParagraphContent::BookmarkStart(_)
            | ParagraphContent::BookmarkEnd(_)
            | ParagraphContent::CommentRangeStart(_)
            | ParagraphContent::CommentRangeEnd(_) => {
                // Skip bookmark and comment markers
            }
            _ => {}
        }
    }

    Ok(children)
}

fn convert_run<R: Read + Seek>(
    converter: &mut Converter,
    doc: &mut OoxmlDocument<R>,
    run: &Run,
) -> Result<Option<Node>, ParseError> {
    let text = run.text();

    // Handle DrawingML images in the run
    for drawing in run.drawings() {
        for rel_id in drawing.all_image_rel_ids() {
            if let Some(image_node) = convert_image(converter, doc, rel_id)? {
                return Ok(Some(image_node));
            }
        }
    }

    // Handle VML pictures (legacy format)
    if run
        .run_content
        .iter()
        .any(|c| matches!(c, RunContent::Pict(_)))
    {
        converter.warn_lost("VML picture content not fully supported");
    }

    // Skip empty text runs
    if text.is_empty() {
        return Ok(None);
    }

    // Create text node with formatting
    let text_node = create_text_node(&text);

    // Apply formatting wrappers
    let formatted = apply_formatting(run, text_node);

    Ok(Some(formatted))
}

fn convert_image<R: Read + Seek>(
    converter: &mut Converter,
    doc: &mut OoxmlDocument<R>,
    rel_id: &str,
) -> Result<Option<Node>, ParseError> {
    // Try to load image data
    match doc.get_image_data(rel_id) {
        Ok(image_data) => {
            let resource_id = converter.add_resource(image_data.data, &image_data.content_type);
            let node = Node::new(node::IMAGE)
                .prop(prop::URL, format!("resource:{}", resource_id.as_str()));
            Ok(Some(node))
        }
        Err(_) => {
            converter.warn_lost(format!("Failed to load image: {}", rel_id));
            Ok(None)
        }
    }
}

fn convert_hyperlink<R: Read + Seek>(
    converter: &mut Converter,
    doc: &mut OoxmlDocument<R>,
    link: &Hyperlink,
) -> Result<Option<Node>, ParseError> {
    let mut children = Vec::new();

    // Convert runs inside the hyperlink
    for run in link.runs() {
        if let Some(node) = convert_run(converter, doc, run)? {
            children.push(node);
        }
    }

    if children.is_empty() {
        return Ok(None);
    }

    let mut node = Node::new(node::LINK);

    // Get URL from relationship or anchor
    if let Some(rel_id) = link.rel_id() {
        if let Some(url) = doc.get_hyperlink_url(rel_id) {
            node = node.prop(prop::URL, url.to_string());
        }
    } else if let Some(anchor) = link.anchor_str() {
        // Internal bookmark link
        node = node.prop(prop::URL, format!("#{}", anchor));
    }

    Ok(Some(node.children(children)))
}

fn convert_table<R: Read + Seek>(
    converter: &mut Converter,
    doc: &mut OoxmlDocument<R>,
    table: &Table,
) -> Result<Node, ParseError> {
    let mut rows = Vec::new();

    for row in table.rows() {
        let mut cells = Vec::new();

        // Determine if this is a header row via tblHeader property
        let is_header = row
            .properties()
            .and_then(|p| p.tbl_header.as_ref())
            .map(|h| match &h.value {
                None => true,
                Some(v) => matches!(v.as_str(), "1" | "true" | "on"),
            })
            .unwrap_or(false);

        for cell in row.cells() {
            let mut cell_children = Vec::new();

            for para in cell.paragraphs() {
                if let Some(node) = convert_paragraph(converter, doc, para)? {
                    cell_children.push(node);
                }
            }

            let cell_kind = if is_header {
                node::TABLE_HEADER
            } else {
                node::TABLE_CELL
            };

            cells.push(Node::new(cell_kind).children(cell_children));
        }

        rows.push(Node::new(node::TABLE_ROW).children(cells));
    }

    Ok(Node::new(node::TABLE).children(rows))
}

fn create_text_node(text: &str) -> Node {
    Node::new(node::TEXT).prop(prop::CONTENT, text.to_string())
}

fn apply_formatting(run: &Run, mut node: Node) -> Node {
    // Apply formatting in order: subscript/superscript, strikethrough, underline, italic, bold
    // Inner-most formatting is applied first

    if run.properties().is_some_and(|p| p.is_subscript()) {
        node = Node::new(node::SUBSCRIPT).child(node);
    } else if run.properties().is_some_and(|p| p.is_superscript()) {
        node = Node::new(node::SUPERSCRIPT).child(node);
    }

    if run.is_strikethrough()
        || run
            .properties()
            .is_some_and(|p| p.is_double_strikethrough())
    {
        node = Node::new(node::STRIKEOUT).child(node);
    }

    if run.is_underline() {
        node = Node::new(node::UNDERLINE).child(node);
    }

    if run.is_italic() {
        node = Node::new(node::EMPHASIS).child(node);
    }

    if run.is_bold() {
        node = Node::new(node::STRONG).child(node);
    }

    node
}

fn extract_metadata<R: Read + Seek>(doc: &OoxmlDocument<R>) -> Properties {
    let mut metadata = Properties::new();

    if let Some(core) = doc.core_properties() {
        if let Some(title) = &core.title {
            metadata.set("title", title.clone());
        }
        if let Some(creator) = &core.creator {
            metadata.set("author", creator.clone());
        }
        if let Some(subject) = &core.subject {
            metadata.set("subject", subject.clone());
        }
        if let Some(description) = &core.description {
            metadata.set("description", description.clone());
        }
        if let Some(keywords) = &core.keywords {
            metadata.set("keywords", keywords.clone());
        }
        if let Some(category) = &core.category {
            metadata.set("category", category.clone());
        }
        if let Some(created) = &core.created {
            metadata.set("created", created.clone());
        }
        if let Some(modified) = &core.modified {
            metadata.set("modified", modified.clone());
        }
    }

    if let Some(app) = doc.app_properties() {
        if let Some(app_name) = &app.application {
            metadata.set("application", app_name.clone());
        }
        if let Some(pages) = app.pages {
            metadata.set("pages", pages as i64);
        }
        if let Some(words) = app.words {
            metadata.set("words", words as i64);
        }
        if let Some(paragraphs) = app.paragraphs {
            metadata.set("paragraphs", paragraphs as i64);
        }
    }

    metadata
}

#[cfg(test)]
mod tests {
    // Tests would go here, but require actual DOCX files
    // Integration tests can be added with test fixtures
}
