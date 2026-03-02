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
    BlockContent, BlockContentChoice, FootnoteEndnote, Hyperlink, Paragraph, ParagraphContent, Run,
    RunContent, RunContentChoice, STJc, Table,
};
use rescribe_core::{
    ConversionResult, Document, FidelityWarning, Node, ParseError, Properties, Resource,
    ResourceId, ResourceMap, Severity, SourceInfo, WarningKind,
};
use rescribe_std::{node, prop};
use std::collections::HashMap;
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
    /// Footnote content keyed by footnote id, for inline lookup.
    footnotes: HashMap<i64, Vec<Node>>,
}

impl Converter {
    fn new() -> Self {
        Self {
            warnings: Vec::new(),
            resources: ResourceMap::new(),
            footnotes: HashMap::new(),
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

    // Pre-load footnotes into converter state so convert_run can look them up.
    if let Ok(footnotes) = doc.get_footnotes() {
        let body_content_clone: Vec<BlockContent> = Vec::new(); // placeholder
        // We need to convert each footnote's block content to IR nodes.
        // To avoid borrow conflicts, collect footnote data first.
        let footnote_data: Vec<(i64, Vec<BlockContent>)> = footnotes
            .footnote
            .into_iter()
            .filter(|f| f.id > 0) // skip separator footnotes (id <= 0)
            .map(|f: FootnoteEndnote| (f.id, f.block_content))
            .collect();
        let _ = body_content_clone; // silence warning

        for (id, block_content) in footnote_data {
            let fn_node = convert_body_content(&mut converter, &mut doc, &block_content)?;
            converter.footnotes.insert(id, fn_node.children);
        }
    }

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

/// Convert a slice of BlockContent into a document-level node.
fn convert_body_content<R: Read + Seek>(
    converter: &mut Converter,
    doc: &mut OoxmlDocument<R>,
    content: &[BlockContent],
) -> Result<Node, ParseError> {
    let mut children = Vec::new();
    convert_block_content_into(converter, doc, content, &mut children)?;
    Ok(Node::new(node::DOCUMENT).children(children))
}

/// Inner helper: push converted nodes into `out`, grouping list paragraphs.
fn convert_block_content_into<R: Read + Seek>(
    converter: &mut Converter,
    doc: &mut OoxmlDocument<R>,
    content: &[BlockContent],
    out: &mut Vec<Node>,
) -> Result<(), ParseError> {
    // Pending list accumulator: (num_id, items)
    let mut pending_list: Option<(i64, Vec<Node>)> = None;

    for block in content {
        match block {
            BlockContent::P(para) => {
                // Check for list membership
                if let Some((num_id, _ilvl)) = para.numbering() {
                    let item_children = convert_paragraph_content(converter, doc, para)?;
                    let item = Node::new(node::LIST_ITEM).children(item_children);
                    match &mut pending_list {
                        Some((cur_id, items)) if *cur_id == num_id => {
                            items.push(item);
                        }
                        _ => {
                            // Flush old list if any
                            flush_pending_list(&mut pending_list, out);
                            pending_list = Some((num_id, vec![item]));
                        }
                    }
                } else {
                    flush_pending_list(&mut pending_list, out);
                    if let Some(n) = convert_paragraph(converter, doc, para)? {
                        out.push(n);
                    }
                }
            }
            BlockContent::Tbl(table) => {
                flush_pending_list(&mut pending_list, out);
                out.push(convert_table(converter, doc, table)?);
            }
            BlockContent::Sdt(ctrl) => {
                flush_pending_list(&mut pending_list, out);
                if let Some(content) = &ctrl.sdt_content {
                    for inner_block in &content.block_content {
                        match inner_block {
                            BlockContentChoice::P(para) => {
                                if let Some(n) = convert_paragraph(converter, doc, para)? {
                                    out.push(n);
                                }
                            }
                            BlockContentChoice::Tbl(table) => {
                                out.push(convert_table(converter, doc, table)?);
                            }
                            _ => {}
                        }
                    }
                }
            }
            BlockContent::CustomXml(xml) => {
                flush_pending_list(&mut pending_list, out);
                for inner_block in &xml.block_content {
                    match inner_block {
                        BlockContentChoice::P(para) => {
                            if let Some(n) = convert_paragraph(converter, doc, para)? {
                                out.push(n);
                            }
                        }
                        BlockContentChoice::Tbl(table) => {
                            out.push(convert_table(converter, doc, table)?);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    flush_pending_list(&mut pending_list, out);
    Ok(())
}

fn flush_pending_list(pending: &mut Option<(i64, Vec<Node>)>, out: &mut Vec<Node>) {
    if let Some((_num_id, items)) = pending.take() {
        out.push(
            Node::new(node::LIST)
                .prop(prop::ORDERED, false)
                .children(items),
        );
    }
}

fn convert_body<R: Read + Seek>(
    converter: &mut Converter,
    doc: &mut OoxmlDocument<R>,
    content: &[BlockContent],
) -> Result<Node, ParseError> {
    let mut children = Vec::new();
    convert_block_content_into(converter, doc, content, &mut children)?;
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
        Ok(Some(
            Node::new(node::HEADING)
                .prop(prop::LEVEL, level as i64)
                .children(inline_children),
        ))
    } else {
        let mut node = Node::new(node::PARAGRAPH).children(inline_children);

        // Apply paragraph alignment
        if let Some(align) = para.alignment() {
            let align_str = match align {
                STJc::Left | STJc::Start => "left",
                STJc::Right | STJc::End => "right",
                STJc::Center => "center",
                STJc::Both => "justify",
                _ => "",
            };
            if !align_str.is_empty() {
                node = node.prop(prop::STYLE_ALIGN, align_str.to_string());
            }
        }

        Ok(Some(node))
    }
}

fn detect_heading_level(para: &Paragraph) -> Option<u8> {
    if let Some(props) = para.properties() {
        if let Some(outline) = &props.outline_lvl {
            let level = outline.value as u8;
            return Some(level + 1);
        }

        if let Some(style) = &props.paragraph_style {
            let style_lower = style.value.to_lowercase();
            if style_lower.starts_with("heading") || style_lower.starts_with("titre") {
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
                for n in convert_run(converter, doc, run)? {
                    children.push(n);
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
                    if let RunContentChoice::R(run) = item {
                        for n in convert_run(converter, doc, run)? {
                            children.push(n);
                        }
                    }
                }
            }
            ParagraphContent::Del(_del) => {
                converter.warn("Tracked deletion content skipped");
            }
            ParagraphContent::FldSimple(field) => {
                // Extract displayed text from simple fields; instruction is in field.instruction
                converter.warn("Field instruction lost (display text preserved)");
                for item in &field.paragraph_content {
                    if let ParagraphContent::R(run) = item {
                        for n in convert_run(converter, doc, run)? {
                            children.push(n);
                        }
                    }
                }
            }
            ParagraphContent::BookmarkStart(_) | ParagraphContent::BookmarkEnd(_) => {
                converter.warn("Bookmark marker not representable in IR");
            }
            ParagraphContent::CommentRangeStart(_) | ParagraphContent::CommentRangeEnd(_) => {
                converter.warn("Comment range marker not representable in IR");
            }
            _ => {}
        }
    }

    Ok(children)
}

/// Convert a run, returning zero or more nodes (a run may produce a footnote_ref + text).
fn convert_run<R: Read + Seek>(
    converter: &mut Converter,
    doc: &mut OoxmlDocument<R>,
    run: &Run,
) -> Result<Vec<Node>, ParseError> {
    let mut result = Vec::new();

    // Handle footnote reference (takes precedence over text)
    if let Some(fn_ref) = run.footnote_ref() {
        let fn_id = fn_ref.id;
        let content = converter.footnotes.remove(&fn_id).unwrap_or_default();
        result.push(
            Node::new(node::FOOTNOTE_REF)
                .prop(prop::LABEL, fn_id.to_string())
                .children(content),
        );
        return Ok(result);
    }

    // Handle endnote reference
    if let Some(en_ref) = run.endnote_ref() {
        let en_id = en_ref.id;
        converter.warn(format!("Endnote {} content not yet represented", en_id));
        result.push(Node::new(node::FOOTNOTE_REF).prop(prop::LABEL, format!("en{}", en_id)));
        return Ok(result);
    }

    // Handle DrawingML images in the run
    for drawing in run.drawings() {
        for rel_id in drawing.all_image_rel_ids() {
            if let Some(image_node) = convert_image(converter, doc, rel_id)? {
                result.push(image_node);
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

    let text = run.text();

    // If we already pushed image nodes, skip empty text
    if text.is_empty() {
        return Ok(result);
    }

    // Create text node with formatting
    let text_node = create_text_node(&text);
    let formatted = apply_formatting(run, text_node);
    result.push(formatted);

    Ok(result)
}

fn convert_image<R: Read + Seek>(
    converter: &mut Converter,
    doc: &mut OoxmlDocument<R>,
    rel_id: &str,
) -> Result<Option<Node>, ParseError> {
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

    for run in link.runs() {
        for n in convert_run(converter, doc, run)? {
            children.push(n);
        }
    }

    if children.is_empty() {
        return Ok(None);
    }

    let mut node = Node::new(node::LINK);

    if let Some(rel_id) = link.rel_id() {
        if let Some(url) = doc.get_hyperlink_url(rel_id) {
            node = node.prop(prop::URL, url.to_string());
        }
    } else if let Some(anchor) = link.anchor_str() {
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
    // --- Span-level styling (color, font, size, background) ---
    // Collect properties that go onto a span wrapper.
    let mut span_props = Properties::new();

    if let Some(props) = run.properties() {
        if let Some(color) = props.color_hex() {
            // "auto" is the default; skip it
            if color != "auto" && !color.is_empty() {
                span_props.set(prop::STYLE_COLOR, color.to_string());
            }
        }
        if let Some(font) = props.font_ascii()
            && !font.is_empty()
        {
            span_props.set(prop::STYLE_FONT, font.to_string());
        }
        if let Some(size_pts) = props.font_size_points() {
            span_props.set(prop::STYLE_SIZE, size_pts);
        }
        if let Some(highlight) = props.highlight_color() {
            let color_str = highlight.to_string();
            if color_str != "none" {
                span_props.set(prop::STYLE_BG_COLOR, color_str);
            }
        }
    }

    if !span_props.is_empty() {
        let mut span_node = Node::new(node::SPAN);
        span_node.props = span_props;
        node = span_node.child(node);
    }

    // --- Semantic inline node wrappers ---

    if run.properties().is_some_and(|p| p.is_hidden()) {
        node = Node::new(node::HIDDEN).child(node);
    }

    if run.properties().is_some_and(|p| p.is_small_caps()) {
        node = Node::new(node::SMALL_CAPS).child(node);
    }

    if run.properties().is_some_and(|p| p.is_all_caps()) {
        node = Node::new(node::ALL_CAPS).child(node);
    }

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
