use ooxml_wml::CoreProperties;
use ooxml_wml::types;
use ooxml_wml::writer::{DocumentBuilder, Drawing, ListType};
use rescribe_core::{
    ConversionResult, Document, EmitError, FidelityWarning, Node, PropValue, ResourceId, Severity,
    WarningKind,
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

    // Section properties: page size/margins/orientation (raw-preserved metadata).
    let has_sect_pr = m.get_int("docx:page-width-twips").is_some()
        || m.get_int("docx:page-height-twips").is_some()
        || m.get_str("docx:page-orientation").is_some()
        || m.get_str("docx:margin-top-twips").is_some();
    if has_sect_pr {
        let pg_sz = if m.get_int("docx:page-width-twips").is_some()
            || m.get_int("docx:page-height-twips").is_some()
            || m.get_str("docx:page-orientation").is_some()
        {
            Some(Box::new(types::PageSize {
                width: m.get_int("docx:page-width-twips").map(|v| v.to_string()),
                height: m.get_int("docx:page-height-twips").map(|v| v.to_string()),
                orient: m
                    .get_str("docx:page-orientation")
                    .and_then(|s| s.parse().ok()),
                code: None,
                extra_attrs: Default::default(),
            }))
        } else {
            None
        };
        let pg_mar = if m.get_str("docx:margin-top-twips").is_some() {
            Some(Box::new(types::PageMargins {
                top: m
                    .get_str("docx:margin-top-twips")
                    .unwrap_or("1440")
                    .to_string(),
                right: m
                    .get_str("docx:margin-right-twips")
                    .unwrap_or("1440")
                    .to_string(),
                bottom: m
                    .get_str("docx:margin-bottom-twips")
                    .unwrap_or("1440")
                    .to_string(),
                left: m
                    .get_str("docx:margin-left-twips")
                    .unwrap_or("1440")
                    .to_string(),
                header: "720".to_string(),
                footer: "720".to_string(),
                gutter: "0".to_string(),
                extra_attrs: Default::default(),
            }))
        } else {
            None
        };
        builder
            .body_mut()
            .set_section_properties(types::SectionProperties {
                pg_sz,
                pg_mar,
                ..Default::default()
            });
    }

    // Document-level default language, into styles.xml docDefaults/rPrDefault/rPr/lang.
    if let Some(lang) = m.get_str(prop::LANGUAGE) {
        let styles = types::Styles {
            doc_defaults: Some(Box::new(types::DocumentDefaults {
                r_pr_default: Some(Box::new(types::RunPropertiesDefault {
                    r_pr: Some(Box::new(types::RunProperties {
                        lang: Some(Box::new(types::LanguageElement {
                            value: Some(lang.to_string()),
                            east_asia: None,
                            bidi: None,
                            extra_attrs: Default::default(),
                        })),
                        ..Default::default()
                    })),
                    extra_children: Vec::new(),
                })),
                p_pr_default: None,
                extra_children: Vec::new(),
            })),
            ..Default::default()
        };
        builder.set_styles(styles);
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

fn warn(warnings: &mut Vec<FidelityWarning>, message: impl Into<String>) {
    warnings.push(FidelityWarning::new(
        Severity::Minor,
        WarningKind::FeatureLost("docx".to_string()),
        message,
    ));
}

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
            warn(
                warnings,
                "code_block emitted as plain paragraph; monospace styling and language lost",
            );
            let content = node.props.get_str(prop::CONTENT).unwrap_or("");
            let para = builder.body_mut().add_paragraph();
            para.add_run().set_text(content);
        }
        node::BLOCKQUOTE => {
            warn(warnings, "blockquote flattened; indentation/styling lost");
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
        other => {
            // For unhandled block nodes, warn and try to preserve content.
            if other != node::DOCUMENT {
                warn(
                    warnings,
                    format!(
                        "'{}' node not natively supported in DOCX; structure lost",
                        other
                    ),
                );
            }
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
    // Paragraph style (raw-preserved; meaningless without the source styles.xml)
    if let Some(style) = node.props.get_str("docx:pStyle") {
        let ppr = para
            .p_pr
            .get_or_insert_with(|| Box::new(types::ParagraphProperties::default()));
        ppr.paragraph_style = Some(Box::new(types::CTString {
            value: style.to_string(),
            extra_attrs: Default::default(),
        }));
    }
    if node.props.get_bool("docx:keep-next").unwrap_or(false) {
        let ppr = para
            .p_pr
            .get_or_insert_with(|| Box::new(types::ParagraphProperties::default()));
        ppr.keep_next = Some(Box::new(types::OnOffElement {
            value: None,
            extra_attrs: Default::default(),
        }));
    }
    if node.props.get_bool("docx:keep-lines").unwrap_or(false) {
        let ppr = para
            .p_pr
            .get_or_insert_with(|| Box::new(types::ParagraphProperties::default()));
        ppr.keep_lines = Some(Box::new(types::OnOffElement {
            value: None,
            extra_attrs: Default::default(),
        }));
    }
    // Page break before (semantic prop, shared with the inline page-break marker)
    if node
        .props
        .get_bool(prop::LAYOUT_PAGE_BREAK)
        .unwrap_or(false)
    {
        let ppr = para
            .p_pr
            .get_or_insert_with(|| Box::new(types::ParagraphProperties::default()));
        ppr.page_break_before = Some(Box::new(types::OnOffElement {
            value: None,
            extra_attrs: Default::default(),
        }));
    }
    // Paragraph shading
    if let Some(fill) = node.props.get_str(prop::STYLE_BG_COLOR) {
        let ppr = para
            .p_pr
            .get_or_insert_with(|| Box::new(types::ParagraphProperties::default()));
        ppr.shading = Some(Box::new(types::CTShd {
            value: types::STShd::Clear,
            color: None,
            theme_color: None,
            theme_tint: None,
            theme_shade: None,
            fill: Some(fill.to_string()),
            theme_fill: None,
            theme_fill_tint: None,
            theme_fill_shade: None,
            extra_attrs: Default::default(),
        }));
    }
    // Paragraph border
    apply_para_border(para, node, "top");
    apply_para_border(para, node, "bottom");
    apply_para_border(para, node, "left");
    apply_para_border(para, node, "right");
}

/// Re-apply a raw-preserved `docx:para-border-{side}` prop to a paragraph.
fn apply_para_border(para: &mut types::Paragraph, node: &Node, side: &str) {
    let Some(raw) = node.props.get_str(&format!("docx:para-border-{side}")) else {
        return;
    };
    let mut parts = raw.splitn(3, ';');
    let (Some(style_str), Some(size_str), Some(color)) = (parts.next(), parts.next(), parts.next())
    else {
        return;
    };
    let Ok(value) = style_str.parse::<types::STBorder>() else {
        return;
    };
    let border = types::CTBorder {
        value,
        color: if color.is_empty() {
            None
        } else {
            Some(color.to_string())
        },
        theme_color: None,
        theme_tint: None,
        theme_shade: None,
        size: size_str.parse().ok(),
        space: None,
        shadow: None,
        frame: None,
        extra_attrs: Default::default(),
    };
    let ppr = para
        .p_pr
        .get_or_insert_with(|| Box::new(types::ParagraphProperties::default()));
    let bdr = ppr
        .paragraph_border
        .get_or_insert_with(|| Box::new(types::CTPBdr::default()));
    match side {
        "top" => bdr.top = Some(Box::new(border)),
        "bottom" => bdr.bottom = Some(Box::new(border)),
        "left" => bdr.left = Some(Box::new(border)),
        "right" => bdr.right = Some(Box::new(border)),
        _ => {}
    }
}

/// Re-apply a raw-preserved `docx:cell-border-{side}` prop to a table cell.
fn apply_cell_border(cell: &mut types::TableCell, node: &Node, side: &str) {
    let Some(raw) = node.props.get_str(&format!("docx:cell-border-{side}")) else {
        return;
    };
    let mut parts = raw.splitn(3, ';');
    let (Some(style_str), Some(size_str), Some(color)) = (parts.next(), parts.next(), parts.next())
    else {
        return;
    };
    let Ok(value) = style_str.parse::<types::STBorder>() else {
        return;
    };
    let border = types::CTBorder {
        value,
        color: if color.is_empty() {
            None
        } else {
            Some(color.to_string())
        },
        theme_color: None,
        theme_tint: None,
        theme_shade: None,
        size: size_str.parse().ok(),
        space: None,
        shadow: None,
        frame: None,
        extra_attrs: Default::default(),
    };
    let tcpr = cell
        .cell_properties
        .get_or_insert_with(|| Box::new(types::TableCellProperties::default()));
    let borders = tcpr
        .tc_borders
        .get_or_insert_with(|| Box::new(types::CTTcBorders::default()));
    match side {
        "top" => borders.top = Some(Box::new(border)),
        "bottom" => borders.bottom = Some(Box::new(border)),
        "left" => borders.left = Some(Box::new(border)),
        "right" => borders.right = Some(Box::new(border)),
        _ => {}
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

/// A real (non-continuation) cell, positioned on the table grid.
struct GridCell<'a> {
    col: i64,
    colspan: i64,
    rowspan: i64,
    node: &'a Node,
}

/// A `vMerge`-continue placeholder cell scheduled for a later row by an
/// earlier row's `rowspan`.
struct ContinuationCell {
    col: i64,
    colspan: i64,
}

fn write_table(
    builder: &mut DocumentBuilder,
    table_node: &Node,
    warnings: &mut Vec<FidelityWarning>,
    hyperlink_map: &HashMap<String, String>,
    footnote_map: &HashMap<String, i64>,
    image_map: &HashMap<String, types::CTDrawing>,
) -> Result<(), EmitError> {
    let row_nodes: Vec<&Node> = table_node
        .children
        .iter()
        .filter(|n| n.kind.as_str() == node::TABLE_ROW)
        .collect();

    // Single forward pass: a row's real cells (as stored in the IR -- the reader
    // already dropped vMerge-continuation placeholders, folding them into the
    // origin cell's rowspan) only account for the columns *not* covered by a
    // still-open rowspan from an earlier row. So column assignment must skip
    // over columns an active merge still occupies, not just run 0..colspan
    // within the row in isolation.
    struct ActiveMerge {
        col: i64,
        colspan: i64,
        rows_left: i64,
    }
    let mut active: Vec<ActiveMerge> = Vec::new();
    let mut grid: Vec<Vec<GridCell>> = Vec::with_capacity(row_nodes.len());
    let mut continuations: Vec<Vec<ContinuationCell>> =
        (0..row_nodes.len()).map(|_| Vec::new()).collect();

    for (row_idx, row_node) in row_nodes.iter().enumerate() {
        for am in &active {
            continuations[row_idx].push(ContinuationCell {
                col: am.col,
                colspan: am.colspan,
            });
        }

        let mut col = 0i64;
        let mut cells = Vec::new();
        for cell_node in &row_node.children {
            let kind = cell_node.kind.as_str();
            if kind != node::TABLE_CELL && kind != node::TABLE_HEADER {
                continue;
            }
            // Skip past any column currently reserved by an open rowspan.
            while let Some(a) = active.iter().find(|a| a.col == col) {
                col += a.colspan;
            }
            let colspan = cell_node.props.get_int(prop::COLSPAN).unwrap_or(1).max(1);
            let rowspan = cell_node.props.get_int(prop::ROWSPAN).unwrap_or(1).max(1);
            cells.push(GridCell {
                col,
                colspan,
                rowspan,
                node: cell_node,
            });
            col += colspan;
        }

        for am in &mut active {
            am.rows_left -= 1;
        }
        active.retain(|a| a.rows_left > 0);
        for cell in &cells {
            if cell.rowspan > 1 {
                active.push(ActiveMerge {
                    col: cell.col,
                    colspan: cell.colspan,
                    rows_left: cell.rowspan - 1,
                });
            }
        }

        grid.push(cells);
    }

    let table = builder.body_mut().add_table();
    for (row_idx, cells) in grid.into_iter().enumerate() {
        let row = table.add_row();
        // Merge real cells and scheduled continuations in column order.
        let mut entries: Vec<(i64, Option<&GridCell>, Option<&ContinuationCell>)> = cells
            .iter()
            .map(|c| (c.col, Some(c), None))
            .chain(
                continuations[row_idx]
                    .iter()
                    .map(|c| (c.col, None, Some(c))),
            )
            .collect();
        entries.sort_by_key(|(col, ..)| *col);

        for (_, real, cont) in entries {
            if let Some(cell) = real {
                let cell_node = cell.node;
                let out_cell = row.add_cell();
                if cell.colspan > 1 {
                    out_cell.set_grid_span(cell.colspan as u32);
                }
                if cell.rowspan > 1 {
                    out_cell.set_vertical_merge(ooxml_wml::convenience::VMergeType::Restart);
                }
                if let Some(bg) = cell_node.props.get_str(prop::STYLE_BG_COLOR) {
                    out_cell.set_background_color(bg);
                }
                apply_cell_border(out_cell, cell_node, "top");
                apply_cell_border(out_cell, cell_node, "bottom");
                apply_cell_border(out_cell, cell_node, "left");
                apply_cell_border(out_cell, cell_node, "right");
                for para_node in &cell_node.children {
                    let para = out_cell.add_paragraph();
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
            } else if let Some(cont) = cont {
                let out_cell = row.add_cell();
                if cont.colspan > 1 {
                    out_cell.set_grid_span(cont.colspan as u32);
                }
                out_cell.set_vertical_merge(ooxml_wml::convenience::VMergeType::Continue);
                out_cell.add_paragraph();
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
    language: Option<String>,
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
            node::SPAN if node.props.get_str("docx:tracked-change").is_some() => {
                write_tracked_change_to_para(para, node);
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
                if let Some(lang) = node.props.get_str(prop::LANGUAGE) {
                    next.language = Some(lang.to_string());
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
            node::LINE_BREAK => {
                let run = para.add_run();
                apply_run_formatting(run, fmt);
                let br_type = if node
                    .props
                    .get_bool(prop::LAYOUT_PAGE_BREAK)
                    .unwrap_or(false)
                {
                    Some(types::STBrType::Page)
                } else if node.props.get_bool(prop::LAYOUT_COLUMN).unwrap_or(false) {
                    Some(types::STBrType::Column)
                } else {
                    None
                };
                run.run_content
                    .push(types::RunContent::Br(Box::new(types::CTBr {
                        r#type: br_type,
                        clear: None,
                        extra_attrs: Default::default(),
                    })));
            }
            node::SOFT_BREAK => {
                // DOCX has no "soft wrap" markup; a soft break reflows as a space.
                emit_run(para, " ", fmt);
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

/// Re-emit a tracked-insertion/-deletion span as `<w:ins>`/`<w:del>`.
///
/// Limitation: nested inline formatting inside the tracked-change span is
/// flattened to plain text (`add_tracked_insertion`/`add_tracked_deletion`
/// only take flat text) -- acceptable for round-tripping the tracked-change
/// record and its text, but bold/italic/etc *within* a tracked change would
/// need a lower-level CTRunTrackChange builder to fully preserve.
fn write_tracked_change_to_para(para: &mut types::Paragraph, node: &Node) {
    let kind = node.props.get_str("docx:tracked-change").unwrap_or("");
    let id = node.props.get_int("docx:tracked-change-id").unwrap_or(1);
    let author = node
        .props
        .get_str("docx:tracked-change-author")
        .unwrap_or("unknown")
        .to_string();
    let date = node
        .props
        .get_str("docx:tracked-change-date")
        .map(|s| s.to_string());
    let mut text = String::new();
    flatten_text(&node.children, &mut text);
    match kind {
        "ins" => {
            para.add_tracked_insertion(id, &author, date.as_deref(), &text);
        }
        "del" => {
            para.add_tracked_deletion(id, &author, date.as_deref(), &text);
        }
        _ => {}
    }
}

/// Recursively concatenate all `text` node content under `nodes`.
fn flatten_text(nodes: &[Node], out: &mut String) {
    for node in nodes {
        if node.kind.as_str() == node::TEXT {
            out.push_str(node.props.get_str(prop::CONTENT).unwrap_or(""));
        } else {
            flatten_text(&node.children, out);
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
    apply_run_formatting(run, fmt);
}

/// Apply formatting (but not text content) to a run.
fn apply_run_formatting(run: &mut types::Run, fmt: &FormattingState) {
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
    if let Some(ref lang) = fmt.language {
        let rpr = run
            .r_pr
            .get_or_insert_with(|| Box::new(types::RunProperties::default()));
        rpr.lang = Some(Box::new(types::LanguageElement {
            value: Some(lang.clone()),
            east_asia: None,
            bidi: None,
            extra_attrs: Default::default(),
        }));
    }
}
