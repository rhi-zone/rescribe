//! DOCX (Word) writer for rescribe.
//!
//! Emits rescribe documents as Word documents (.docx) using the ooxml-wml crate.
//!
//! # Example
//!
//! ```ignore
//! use rescribe_write_docx::emit;
//!
//! let doc = // ... create a rescribe Document
//! let bytes = emit(&doc)?.value;
//! std::fs::write("output.docx", bytes)?;
//! ```

use ooxml_wml::CoreProperties;
use ooxml_wml::types;
use ooxml_wml::writer::{DocumentBuilder, Drawing, ListType};
use rescribe_core::{
    ConversionResult, Document, EmitError, FidelityWarning, Node, PropValue, ResourceId,
};
use rescribe_std::{node, prop};
use std::collections::HashMap;

/// Emit a rescribe Document as DOCX bytes.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let mut warnings = Vec::new();
    let mut builder = DocumentBuilder::new();

    // Write metadata from doc.metadata → core/app properties.
    write_metadata(&mut builder, doc);

    // Pre-registration pass: register hyperlinks, footnotes, and images before writing body.
    // This is necessary because `para` borrows from `builder`, preventing builder
    // mutations while a paragraph reference is live.
    let hyperlink_map = pre_register_hyperlinks(&mut builder, &doc.content);
    let footnote_map = pre_register_footnotes(&mut builder, &doc.content, &mut warnings);
    let image_map = pre_register_images(&mut builder, &doc.content, doc);

    convert_node(
        &mut builder,
        &doc.content,
        &mut warnings,
        &hyperlink_map,
        &footnote_map,
        &image_map,
    )?;

    let mut bytes = Vec::new();
    builder
        .write(&mut std::io::Cursor::new(&mut bytes))
        .map_err(|e| EmitError::Io(std::io::Error::other(e.to_string())))?;

    Ok(ConversionResult {
        value: bytes,
        warnings,
    })
}

// ── Metadata writing ──────────────────────────────────────────────────────────

fn write_metadata(builder: &mut DocumentBuilder, doc: &Document) {
    let m = &doc.metadata;
    let has_core = m.get_str("title").is_some()
        || m.get_str("author").is_some()
        || m.get_str("subject").is_some()
        || m.get_str("description").is_some()
        || m.get_str("keywords").is_some()
        || m.get_str("category").is_some()
        || m.get_str("created").is_some()
        || m.get_str("modified").is_some();

    if has_core {
        builder.set_core_properties(CoreProperties {
            title: m.get_str("title").map(|s| s.to_string()),
            creator: m.get_str("author").map(|s| s.to_string()),
            subject: m.get_str("subject").map(|s| s.to_string()),
            description: m.get_str("description").map(|s| s.to_string()),
            keywords: m.get_str("keywords").map(|s| s.to_string()),
            category: m.get_str("category").map(|s| s.to_string()),
            created: m.get_str("created").map(|s| s.to_string()),
            modified: m.get_str("modified").map(|s| s.to_string()),
            ..Default::default()
        });
    }
}

// ── Pre-registration: hyperlinks ──────────────────────────────────────────────

/// Recursively collect all external hyperlink URLs from the IR tree and register
/// them with the builder. Returns a URL → relationship-id map.
fn pre_register_hyperlinks(builder: &mut DocumentBuilder, node: &Node) -> HashMap<String, String> {
    let mut map = HashMap::new();
    collect_hyperlinks(builder, node, &mut map);
    map
}

fn collect_hyperlinks(
    builder: &mut DocumentBuilder,
    node: &Node,
    map: &mut HashMap<String, String>,
) {
    if node.kind.as_str() == node::LINK
        && let Some(url) = node.props.get_str(prop::URL)
        && !url.starts_with('#')
        && !map.contains_key(url)
    {
        let rel_id = builder.add_hyperlink(url);
        map.insert(url.to_string(), rel_id);
    }
    for child in &node.children {
        collect_hyperlinks(builder, child, map);
    }
}

// ── Pre-registration: footnotes ───────────────────────────────────────────────

/// Recursively find all `footnote_ref` nodes in the IR and register them as
/// DOCX footnotes. Returns a label-string → footnote-id map.
fn pre_register_footnotes(
    builder: &mut DocumentBuilder,
    node: &Node,
    warnings: &mut Vec<FidelityWarning>,
) -> HashMap<String, i64> {
    let mut map = HashMap::new();
    collect_footnotes(builder, node, &mut map, warnings);
    map
}

fn collect_footnotes(
    builder: &mut DocumentBuilder,
    node: &Node,
    map: &mut HashMap<String, i64>,
    _warnings: &mut Vec<FidelityWarning>,
) {
    if node.kind.as_str() == node::FOOTNOTE_REF {
        let label = node.props.get_str(prop::LABEL).unwrap_or("").to_string();
        map.entry(label).or_insert_with(|| {
            let mut fn_builder = builder.add_footnote();
            let fn_id = fn_builder.id() as i64;
            // Write footnote body content (block-level children).
            for child in &node.children {
                write_block_to_note_body(fn_builder.body_mut(), child);
            }
            fn_id
        });
        return; // Don't recurse into footnote_ref children (already handled)
    }
    for child in &node.children {
        collect_footnotes(builder, child, map, _warnings);
    }
}

// ── Pre-registration: images ──────────────────────────────────────────────────

/// Walk the IR tree, register every `image` resource with the builder, and
/// pre-build a `CTDrawing` for each. Returns a resource-id → CTDrawing map.
///
/// Pre-building avoids borrow conflicts: `Drawing::build` only needs a
/// `&mut usize` counter (not `&mut builder`), so we can build all drawings
/// here before any paragraph borrows the builder.
fn pre_register_images(
    builder: &mut DocumentBuilder,
    node: &Node,
    doc: &Document,
) -> HashMap<String, types::CTDrawing> {
    let mut drawing_id = 1usize;
    let mut map = HashMap::new();
    collect_images(builder, node, doc, &mut map, &mut drawing_id);
    map
}

fn collect_images(
    builder: &mut DocumentBuilder,
    node: &Node,
    doc: &Document,
    map: &mut HashMap<String, types::CTDrawing>,
    drawing_id: &mut usize,
) {
    if node.kind.as_str() == node::IMAGE
        && let Some(url) = node.props.get_str(prop::URL)
        && let Some(res_id_str) = url.strip_prefix("resource:")
        && !map.contains_key(res_id_str)
    {
        let res_id = ResourceId::from_string(res_id_str);
        if let Some(resource) = doc.resource(&res_id) {
            let rel_id = builder.add_image(resource.data.clone(), &resource.mime_type);
            let mut drawing = Drawing::new();
            drawing.add_image(&rel_id);
            let ct_drawing = drawing.build(drawing_id);
            map.insert(res_id_str.to_string(), ct_drawing);
        }
    }
    for child in &node.children {
        collect_images(builder, child, doc, map, drawing_id);
    }
}

/// Write a single block-level IR node into a footnote/endnote body.
/// Only handles `paragraph` with simple inline content (no nested hyperlinks).
fn write_block_to_note_body(body: &mut types::FootnoteEndnote, node: &Node) {
    match node.kind.as_str() {
        node::PARAGRAPH | node::HEADING => {
            let para = body.add_paragraph();
            write_simple_inline(para, &node.children);
        }
        _ => {
            // Flatten other block types (e.g. list_item) into a paragraph.
            let para = body.add_paragraph();
            write_simple_inline(para, &node.children);
        }
    }
}

/// Write inline nodes into a paragraph without needing builder (no hyperlink/footnote).
fn write_simple_inline(para: &mut types::Paragraph, nodes: &[Node]) {
    for node in nodes {
        match node.kind.as_str() {
            node::TEXT => {
                let text = node.props.get_str(prop::CONTENT).unwrap_or("");
                if !text.is_empty() {
                    para.add_run().set_text(text);
                }
            }
            _ => write_simple_inline(para, &node.children),
        }
    }
}

// ── Main conversion ───────────────────────────────────────────────────────────

fn convert_node(
    builder: &mut DocumentBuilder,
    node: &Node,
    warnings: &mut Vec<FidelityWarning>,
    hyperlink_map: &HashMap<String, String>,
    footnote_map: &HashMap<String, i64>,
    image_map: &HashMap<String, types::CTDrawing>,
) -> Result<(), EmitError> {
    match node.kind.as_str() {
        node::DOCUMENT => {
            for child in &node.children {
                convert_node(
                    builder,
                    child,
                    warnings,
                    hyperlink_map,
                    footnote_map,
                    image_map,
                )?;
            }
        }
        node::PARAGRAPH => {
            let para = builder.body_mut().add_paragraph();
            apply_para_props(para, node);
            write_inline_to_para(
                para,
                &node.children,
                &FormattingState::default(),
                warnings,
                hyperlink_map,
                footnote_map,
                image_map,
            );
        }
        node::HEADING => {
            let level = node.props.get_int(prop::LEVEL).unwrap_or(1) as u8;
            let para = builder.body_mut().add_paragraph();
            para.set_properties(ooxml_wml::types::ParagraphProperties {
                paragraph_style: Some(Box::new(ooxml_wml::types::CTString {
                    value: format!("Heading{}", level),
                    extra_attrs: std::collections::HashMap::new(),
                })),
                ..Default::default()
            });
            apply_para_props(para, node);
            write_inline_to_para(
                para,
                &node.children,
                &FormattingState::default(),
                warnings,
                hyperlink_map,
                footnote_map,
                image_map,
            );
        }
        node::IMAGE => {
            // Image at block level — wrap in a paragraph with a single image run.
            let para = builder.body_mut().add_paragraph();
            emit_image_to_para(para, node, image_map);
        }
        node::LIST => {
            let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
            let list_type = if ordered {
                ListType::Decimal
            } else {
                ListType::Bullet
            };
            let num_id = builder.add_list(list_type);
            for child in &node.children {
                match child.kind.as_str() {
                    node::LIST_ITEM => {
                        let para = builder.body_mut().add_paragraph();
                        para.set_numbering(num_id, 0);
                        write_inline_to_para(
                            para,
                            &child.children,
                            &FormattingState::default(),
                            warnings,
                            hyperlink_map,
                            footnote_map,
                            image_map,
                        );
                    }
                    _ => {
                        convert_node(
                            builder,
                            child,
                            warnings,
                            hyperlink_map,
                            footnote_map,
                            image_map,
                        )?;
                    }
                }
            }
        }
        node::LIST_ITEM => {
            // List item outside a list — emit as bullet paragraph
            let num_id = builder.add_list(ListType::Bullet);
            let para = builder.body_mut().add_paragraph();
            para.set_numbering(num_id, 0);
            write_inline_to_para(
                para,
                &node.children,
                &FormattingState::default(),
                warnings,
                hyperlink_map,
                footnote_map,
                image_map,
            );
        }
        node::TABLE => {
            write_table(
                builder,
                node,
                warnings,
                hyperlink_map,
                footnote_map,
                image_map,
            )?;
        }
        node::CODE_BLOCK => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("");
            let para = builder.body_mut().add_paragraph();
            para.add_run().set_text(content);
        }
        node::BLOCKQUOTE => {
            for child in &node.children {
                convert_node(
                    builder,
                    child,
                    warnings,
                    hyperlink_map,
                    footnote_map,
                    image_map,
                )?;
            }
        }
        node::FOOTNOTE_DEF => {
            // Footnote defs at document level: content was already written during
            // pre-registration. Skip.
        }
        _ => {
            // For unknown block nodes, try to emit children or extract text
            if node.children.is_empty() {
                // Leaf node with content property
                if let Some(text) = node.props.get_str(prop::CONTENT)
                    && !text.is_empty()
                {
                    builder.add_paragraph(text);
                }
            } else {
                for child in &node.children {
                    convert_node(
                        builder,
                        child,
                        warnings,
                        hyperlink_map,
                        footnote_map,
                        image_map,
                    )?;
                }
            }
        }
    }
    Ok(())
}

/// Re-apply `docx:*` paragraph layout props preserved by the reader.
fn apply_para_props(para: &mut types::Paragraph, node: &Node) {
    // Alignment (semantic prop)
    if let Some(align) = node.props.get_str(prop::STYLE_ALIGN) {
        use ooxml_wml::types::STJc;
        let jc_val = match align {
            "left" => Some(STJc::Left),
            "right" => Some(STJc::Right),
            "center" => Some(STJc::Center),
            "justify" => Some(STJc::Both),
            _ => None,
        };
        if let Some(jc) = jc_val {
            para.set_alignment(jc);
        }
    }
    // Spacing
    if let Some(v) = node.props.get_int("docx:space-before") {
        para.set_space_before(v as u32);
    }
    if let Some(v) = node.props.get_int("docx:space-after") {
        para.set_space_after(v as u32);
    }
    if let Some(v) = node.props.get_int("docx:line-spacing") {
        let rule = node
            .props
            .get_str("docx:line-spacing-rule")
            .and_then(|s| s.parse::<types::STLineSpacingRule>().ok())
            .unwrap_or(types::STLineSpacingRule::Auto);
        // Set line and lineRule directly on the spacing struct.
        let ppr = para
            .p_pr
            .get_or_insert_with(|| Box::new(types::ParagraphProperties::default()));
        let spacing = ppr
            .spacing
            .get_or_insert_with(|| Box::new(types::CTSpacing::default()));
        spacing.line = Some(v.to_string());
        spacing.line_rule = Some(rule);
    }
    // Indentation
    if let Some(v) = node.props.get_int("docx:indent-left") {
        para.set_indent_left(v as u32);
    }
    if let Some(v) = node.props.get_int("docx:indent-right") {
        para.set_indent_right(v as u32);
    }
    if let Some(v) = node.props.get_int("docx:indent-first-line") {
        para.set_indent_first_line(v as u32);
    }
    if let Some(v) = node.props.get_int("docx:indent-hanging") {
        let ppr = para
            .p_pr
            .get_or_insert_with(|| Box::new(types::ParagraphProperties::default()));
        let ind = ppr
            .indentation
            .get_or_insert_with(|| Box::new(types::CTInd::default()));
        ind.hanging = Some(v.to_string());
    }
}

/// Emit an image node as a drawing run in an existing paragraph.
fn emit_image_to_para(
    para: &mut types::Paragraph,
    node: &Node,
    image_map: &HashMap<String, types::CTDrawing>,
) {
    if let Some(url) = node.props.get_str(prop::URL)
        && let Some(res_id_str) = url.strip_prefix("resource:")
        && let Some(ct_drawing) = image_map.get(res_id_str)
    {
        let run = para.add_run();
        run.add_drawing(ct_drawing.clone());
    }
}

fn write_table(
    builder: &mut DocumentBuilder,
    table_node: &Node,
    warnings: &mut Vec<FidelityWarning>,
    hyperlink_map: &HashMap<String, String>,
    footnote_map: &HashMap<String, i64>,
    image_map: &HashMap<String, types::CTDrawing>,
) -> Result<(), EmitError> {
    let table = builder.body_mut().add_table();

    for row_node in &table_node.children {
        if row_node.kind.as_str() != node::TABLE_ROW {
            continue;
        }
        let row = table.add_row();
        for cell_node in &row_node.children {
            let kind = cell_node.kind.as_str();
            if kind != node::TABLE_CELL && kind != node::TABLE_HEADER {
                continue;
            }
            let cell = row.add_cell();
            for para_node in &cell_node.children {
                let para = cell.add_paragraph();
                write_inline_to_para(
                    para,
                    &para_node.children,
                    &FormattingState::default(),
                    warnings,
                    hyperlink_map,
                    footnote_map,
                    image_map,
                );
            }
        }
    }

    Ok(())
}

/// Accumulated run-level formatting, threaded through the inline tree.
#[derive(Default, Clone)]
struct FormattingState {
    bold: bool,
    italic: bool,
    underline: bool,
    strikethrough: bool,
    small_caps: bool,
    all_caps: bool,
    hidden: bool,
    subscript: bool,
    superscript: bool,
    color: Option<String>,
    font: Option<String>,
    font_size_half_pts: Option<i64>,
}

/// Walk inline nodes and emit runs into `para`.
#[allow(clippy::only_used_in_recursion)]
fn write_inline_to_para(
    para: &mut types::Paragraph,
    nodes: &[Node],
    fmt: &FormattingState,
    warnings: &mut Vec<FidelityWarning>,
    hyperlink_map: &HashMap<String, String>,
    footnote_map: &HashMap<String, i64>,
    image_map: &HashMap<String, types::CTDrawing>,
) {
    for node in nodes {
        match node.kind.as_str() {
            node::TEXT => {
                let text = node.props.get_str(prop::CONTENT).unwrap_or("");
                if !text.is_empty() {
                    emit_run(para, text, fmt);
                }
            }
            node::IMAGE => {
                emit_image_to_para(para, node, image_map);
            }
            node::STRONG => {
                let mut next = fmt.clone();
                next.bold = true;
                write_inline_to_para(
                    para,
                    &node.children,
                    &next,
                    warnings,
                    hyperlink_map,
                    footnote_map,
                    image_map,
                );
            }
            node::EMPHASIS => {
                let mut next = fmt.clone();
                next.italic = true;
                write_inline_to_para(
                    para,
                    &node.children,
                    &next,
                    warnings,
                    hyperlink_map,
                    footnote_map,
                    image_map,
                );
            }
            node::UNDERLINE => {
                let mut next = fmt.clone();
                next.underline = true;
                write_inline_to_para(
                    para,
                    &node.children,
                    &next,
                    warnings,
                    hyperlink_map,
                    footnote_map,
                    image_map,
                );
            }
            node::STRIKEOUT => {
                let mut next = fmt.clone();
                next.strikethrough = true;
                write_inline_to_para(
                    para,
                    &node.children,
                    &next,
                    warnings,
                    hyperlink_map,
                    footnote_map,
                    image_map,
                );
            }
            node::SPAN => {
                let mut next = fmt.clone();
                if let Some(color) = node.props.get_str(prop::STYLE_COLOR) {
                    next.color = Some(color.to_string());
                }
                if let Some(font) = node.props.get_str(prop::STYLE_FONT) {
                    next.font = Some(font.to_string());
                }
                if let Some(size_pts) = node.props.get("style:size") {
                    let half_pts = match size_pts {
                        PropValue::Float(f) => Some((*f * 2.0) as i64),
                        PropValue::Int(i) => Some(*i * 2),
                        _ => None,
                    };
                    next.font_size_half_pts = half_pts;
                }
                write_inline_to_para(
                    para,
                    &node.children,
                    &next,
                    warnings,
                    hyperlink_map,
                    footnote_map,
                    image_map,
                );
            }
            node::LINK => {
                write_hyperlink_to_para(
                    para,
                    node,
                    fmt,
                    warnings,
                    hyperlink_map,
                    footnote_map,
                    image_map,
                );
            }
            node::FOOTNOTE_REF => {
                // Look up the pre-registered footnote ID.
                let label = node.props.get_str(prop::LABEL).unwrap_or("").to_string();
                if let Some(&fn_id) = footnote_map.get(&label) {
                    let run = para.add_run();
                    run.add_footnote_ref(fn_id);
                }
                // Note: footnote content was already written during pre-registration.
            }
            node::SUBSCRIPT => {
                let mut next = fmt.clone();
                next.subscript = true;
                write_inline_to_para(
                    para,
                    &node.children,
                    &next,
                    warnings,
                    hyperlink_map,
                    footnote_map,
                    image_map,
                );
            }
            node::SUPERSCRIPT => {
                let mut next = fmt.clone();
                next.superscript = true;
                write_inline_to_para(
                    para,
                    &node.children,
                    &next,
                    warnings,
                    hyperlink_map,
                    footnote_map,
                    image_map,
                );
            }
            node::SMALL_CAPS => {
                let mut next = fmt.clone();
                next.small_caps = true;
                write_inline_to_para(
                    para,
                    &node.children,
                    &next,
                    warnings,
                    hyperlink_map,
                    footnote_map,
                    image_map,
                );
            }
            node::ALL_CAPS => {
                let mut next = fmt.clone();
                next.all_caps = true;
                write_inline_to_para(
                    para,
                    &node.children,
                    &next,
                    warnings,
                    hyperlink_map,
                    footnote_map,
                    image_map,
                );
            }
            node::HIDDEN => {
                let mut next = fmt.clone();
                next.hidden = true;
                write_inline_to_para(
                    para,
                    &node.children,
                    &next,
                    warnings,
                    hyperlink_map,
                    footnote_map,
                    image_map,
                );
            }
            node::CODE => {
                // Code inline — no monospace font available in base DOCX without
                // a style definition; just recurse for now.
                write_inline_to_para(
                    para,
                    &node.children,
                    fmt,
                    warnings,
                    hyperlink_map,
                    footnote_map,
                    image_map,
                );
            }
            node::LINE_BREAK | node::SOFT_BREAK => {
                // Emit an empty run as a line break approximation
                let _run = para.add_run();
            }
            _ => {
                // Recurse into children
                write_inline_to_para(
                    para,
                    &node.children,
                    fmt,
                    warnings,
                    hyperlink_map,
                    footnote_map,
                    image_map,
                );
            }
        }
    }
}

/// Write a hyperlink node into a paragraph.
fn write_hyperlink_to_para(
    para: &mut types::Paragraph,
    node: &Node,
    fmt: &FormattingState,
    warnings: &mut Vec<FidelityWarning>,
    hyperlink_map: &HashMap<String, String>,
    footnote_map: &HashMap<String, i64>,
    image_map: &HashMap<String, types::CTDrawing>,
) {
    let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
    let hyperlink = para.add_hyperlink();

    if url.starts_with('#') {
        // Anchor-only link: set anchor attribute directly.
        hyperlink.set_anchor(url.trim_start_matches('#'));
    } else if let Some(rel_id) = hyperlink_map.get(&url) {
        // External link: use pre-registered relationship ID.
        hyperlink.set_rel_id(rel_id);
    }
    // else: missing URL — hyperlink will have no destination (degenerate)

    // Write child runs into the hyperlink's paragraph content.
    write_inline_to_hyperlink(
        hyperlink,
        &node.children,
        fmt,
        warnings,
        hyperlink_map,
        footnote_map,
        image_map,
    );
}

/// Write inline nodes into a hyperlink's paragraph_content.
fn write_inline_to_hyperlink(
    hyperlink: &mut types::Hyperlink,
    nodes: &[Node],
    fmt: &FormattingState,
    _warnings: &mut Vec<FidelityWarning>,
    _hyperlink_map: &HashMap<String, String>,
    _footnote_map: &HashMap<String, i64>,
    _image_map: &HashMap<String, types::CTDrawing>,
) {
    for node in nodes {
        match node.kind.as_str() {
            node::TEXT => {
                let text = node.props.get_str(prop::CONTENT).unwrap_or("");
                if !text.is_empty() {
                    let run = hyperlink.add_run();
                    emit_run_content(run, text, fmt);
                }
            }
            node::STRONG => {
                let mut next = fmt.clone();
                next.bold = true;
                write_inline_to_hyperlink(
                    hyperlink,
                    &node.children,
                    &next,
                    _warnings,
                    _hyperlink_map,
                    _footnote_map,
                    _image_map,
                );
            }
            node::EMPHASIS => {
                let mut next = fmt.clone();
                next.italic = true;
                write_inline_to_hyperlink(
                    hyperlink,
                    &node.children,
                    &next,
                    _warnings,
                    _hyperlink_map,
                    _footnote_map,
                    _image_map,
                );
            }
            _ => {
                write_inline_to_hyperlink(
                    hyperlink,
                    &node.children,
                    fmt,
                    _warnings,
                    _hyperlink_map,
                    _footnote_map,
                    _image_map,
                );
            }
        }
    }
}

/// Emit a text run with the given formatting into a paragraph.
fn emit_run(para: &mut types::Paragraph, text: &str, fmt: &FormattingState) {
    let run = para.add_run();
    emit_run_content(run, text, fmt);
}

/// Apply text and formatting to an existing run reference.
fn emit_run_content(run: &mut types::Run, text: &str, fmt: &FormattingState) {
    run.set_text(text);
    if fmt.bold {
        run.set_bold(true);
    }
    if fmt.italic {
        run.set_italic(true);
    }
    if fmt.underline {
        run.set_underline(types::STUnderline::Single);
    }
    if fmt.strikethrough {
        run.set_strikethrough(true);
    }
    if fmt.small_caps {
        run.set_small_caps(true);
    }
    if fmt.all_caps {
        run.set_all_caps(true);
    }
    if fmt.hidden {
        run.set_vanish(true);
    }
    if fmt.subscript {
        let rpr = run
            .r_pr
            .get_or_insert_with(|| Box::new(types::RunProperties::default()));
        rpr.vert_align = Some(Box::new(types::CTVerticalAlignRun {
            value: types::STVerticalAlignRun::Subscript,
            extra_attrs: HashMap::new(),
        }));
    }
    if fmt.superscript {
        let rpr = run
            .r_pr
            .get_or_insert_with(|| Box::new(types::RunProperties::default()));
        rpr.vert_align = Some(Box::new(types::CTVerticalAlignRun {
            value: types::STVerticalAlignRun::Superscript,
            extra_attrs: HashMap::new(),
        }));
    }
    if let Some(ref color) = fmt.color {
        run.set_color(color);
    }
    if let Some(half_pts) = fmt.font_size_half_pts {
        run.set_font_size(half_pts);
    }
    if let Some(ref font_name) = fmt.font {
        run.set_fonts(types::Fonts {
            ascii: Some(font_name.clone()),
            h_ansi: Some(font_name.clone()),
            ..Default::default()
        });
    }
}

#[cfg(test)]
mod tests {
    // Tests would go here
}
