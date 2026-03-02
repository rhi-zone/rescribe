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

use ooxml_wml::types;
use ooxml_wml::writer::{DocumentBuilder, ListType};
use rescribe_core::{ConversionResult, Document, EmitError, FidelityWarning, Node, PropValue};
use rescribe_std::{node, prop};
use std::collections::HashMap;

/// Emit a rescribe Document as DOCX bytes.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let mut warnings = Vec::new();
    let mut builder = DocumentBuilder::new();

    // Pre-registration pass: register hyperlinks and footnotes before writing body.
    // This is necessary because `para` borrows from `builder`, preventing builder
    // mutations while a paragraph reference is live.
    let hyperlink_map = pre_register_hyperlinks(&mut builder, &doc.content);
    let footnote_map = pre_register_footnotes(&mut builder, &doc.content, &mut warnings);

    convert_node(
        &mut builder,
        &doc.content,
        &mut warnings,
        &hyperlink_map,
        &footnote_map,
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
) -> Result<(), EmitError> {
    match node.kind.as_str() {
        node::DOCUMENT => {
            for child in &node.children {
                convert_node(builder, child, warnings, hyperlink_map, footnote_map)?;
            }
        }
        node::PARAGRAPH => {
            let para = builder.body_mut().add_paragraph();
            // Apply paragraph alignment
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
            write_inline_to_para(
                para,
                &node.children,
                &FormattingState::default(),
                warnings,
                hyperlink_map,
                footnote_map,
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
            write_inline_to_para(
                para,
                &node.children,
                &FormattingState::default(),
                warnings,
                hyperlink_map,
                footnote_map,
            );
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
                        );
                    }
                    _ => {
                        convert_node(builder, child, warnings, hyperlink_map, footnote_map)?;
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
            );
        }
        node::TABLE => {
            write_table(builder, node, warnings, hyperlink_map, footnote_map)?;
        }
        node::CODE_BLOCK => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("");
            let para = builder.body_mut().add_paragraph();
            para.add_run().set_text(content);
        }
        node::BLOCKQUOTE => {
            for child in &node.children {
                convert_node(builder, child, warnings, hyperlink_map, footnote_map)?;
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
                    convert_node(builder, child, warnings, hyperlink_map, footnote_map)?;
                }
            }
        }
    }
    Ok(())
}

fn write_table(
    builder: &mut DocumentBuilder,
    table_node: &Node,
    warnings: &mut Vec<FidelityWarning>,
    hyperlink_map: &HashMap<String, String>,
    footnote_map: &HashMap<String, i64>,
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
) {
    for node in nodes {
        match node.kind.as_str() {
            node::TEXT => {
                let text = node.props.get_str(prop::CONTENT).unwrap_or("");
                if !text.is_empty() {
                    emit_run(para, text, fmt);
                }
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
                );
            }
            node::LINK => {
                write_hyperlink_to_para(para, node, fmt, warnings, hyperlink_map, footnote_map);
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
            // Wrap nodes that don't change run formatting — just recurse
            node::SUBSCRIPT
            | node::SUPERSCRIPT
            | node::CODE
            | node::SMALL_CAPS
            | node::ALL_CAPS
            | node::HIDDEN => {
                write_inline_to_para(
                    para,
                    &node.children,
                    fmt,
                    warnings,
                    hyperlink_map,
                    footnote_map,
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
