use ooxml_wml::CoreProperties;
use ooxml_wml::types;
use ooxml_wml::writer::{DocumentBuilder, Drawing, ListType, NumberingLevel};
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
            // Collect one numbering-level definition per nesting depth this
            // list tree uses (a `list_item` whose children include a nested
            // `list` node -- see read.rs's `ListFrame`/`close_list_frame`),
            // then register a single custom multi-level list for the whole
            // tree so nested items land at the right `ilvl` under one
            // `numId`, instead of the old flat behavior (every `list_item`
            // written at ilvl 0, silently discarding nesting).
            let mut level_ordered: HashMap<u32, bool> = HashMap::new();
            collect_list_levels(node, 0, &mut level_ordered);
            let max_depth = level_ordered.keys().copied().max().unwrap_or(0);
            let levels: Vec<NumberingLevel> = (0..=max_depth)
                .map(|d| {
                    let ordered = level_ordered.get(&d).copied().unwrap_or(false);
                    if ordered {
                        NumberingLevel::decimal(d)
                    } else {
                        NumberingLevel::bullet(d)
                    }
                })
                .collect();
            let num_id = builder.add_custom_list(levels);
            write_list_items(
                builder,
                node,
                0,
                num_id,
                warnings,
                hyperlink_map,
                footnote_map,
                image_map,
            )?;
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
            // Restore the exact source `pStyle` if the reader captured one
            // (see read.rs's `code_block_style`); otherwise fall back to
            // Word's own built-in preformatted style so the output is still
            // recognizable as code when opened in Word.
            let style = node
                .props
                .get_str("docx:pStyle")
                .unwrap_or("HTMLPreformatted")
                .to_string();
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            let para = builder.body_mut().add_paragraph();
            para.set_properties(types::ParagraphProperties {
                paragraph_style: Some(Box::new(types::CTString {
                    value: style,
                    extra_attrs: Default::default(),
                })),
                ..Default::default()
            });
            para.add_run().set_text(&content);
        }
        node::BLOCKQUOTE => {
            // Indentation is the construct's defining feature (see read.rs's
            // `is_blockquote_para`); restore the captured twips (or the
            // detection threshold's default if this blockquote node didn't
            // come from a DOCX read.rs invocation) on every child paragraph.
            let indent_left = node.props.get_int("docx:indent-left").unwrap_or(720) as u32;
            let indent_right = node.props.get_int("docx:indent-right").unwrap_or(720) as u32;
            for child in &node.children {
                if child.kind.as_str() == node::PARAGRAPH {
                    let para = builder.body_mut().add_paragraph();
                    apply_para_props(para, child);
                    para.set_indent_left(indent_left);
                    para.set_indent_right(indent_right);
                    write_inline_to_para(
                        para,
                        &child.children,
                        &FormattingState::default(),
                        warnings,
                        hyperlink_map,
                        footnote_map,
                        image_map,
                    );
                } else {
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
        node::HORIZONTAL_RULE => {
            // An empty paragraph with just a bottom paragraph border (see
            // read.rs's `detect_horizontal_rule`).
            let para = builder.body_mut().add_paragraph();
            apply_para_border(para, node, "hr", "bottom");
        }
        node::FOOTNOTE_DEF => {
            // Footnote defs at document level: content was already written during
            // pre-registration. Skip.
        }
        node::DIV
            if node.props.get_str("docx:sdt-tag").is_some()
                || node.props.get_str("docx:sdt-alias").is_some()
                || node.props.get_str("docx:sdt-type").is_some() =>
        {
            let sdt = build_sdt_block(node, warnings, hyperlink_map, footnote_map, image_map)?;
            builder
                .body_mut()
                .block_content
                .push(types::BlockContent::Sdt(Box::new(sdt)));
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
    apply_para_border(para, node, "para", "top");
    apply_para_border(para, node, "para", "bottom");
    apply_para_border(para, node, "para", "left");
    apply_para_border(para, node, "para", "right");
    // Paragraph frame (`<w:framePr>`; see read.rs's matching comment).
    if node.props.get_bool("docx:frame").unwrap_or(false) {
        let ppr = para
            .p_pr
            .get_or_insert_with(|| Box::new(types::ParagraphProperties::default()));
        ppr.frame_pr = Some(Box::new(types::CTFramePr {
            width: node
                .props
                .get_str("docx:frame-width")
                .map(|s| s.to_string()),
            height: node
                .props
                .get_str("docx:frame-height")
                .map(|s| s.to_string()),
            wrap: node
                .props
                .get_str("docx:frame-wrap")
                .and_then(|s| s.parse().ok()),
            h_anchor: node
                .props
                .get_str("docx:frame-h-anchor")
                .and_then(|s| s.parse().ok()),
            v_anchor: node
                .props
                .get_str("docx:frame-v-anchor")
                .and_then(|s| s.parse().ok()),
            x: node.props.get_str("docx:frame-x").map(|s| s.to_string()),
            y: node.props.get_str("docx:frame-y").map(|s| s.to_string()),
            ..Default::default()
        }));
    }
}

/// Re-apply a raw-preserved `docx:{scope}-border-{side}` prop to a paragraph
/// (`scope` is `"para"` for ordinary paragraph borders, `"hr"` for the
/// bottom-only border read.rs uses to detect/emit a horizontal rule).
fn apply_para_border(para: &mut types::Paragraph, node: &Node, scope: &str, side: &str) {
    let Some(raw) = node.props.get_str(&format!("docx:{scope}-border-{side}")) else {
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

/// Build a `<w:sdt>` (structured document tag) block from a `div` node
/// carrying `docx:sdt-*` raw props (see read.rs's `convert_sdt_block`).
/// Only `paragraph`/`heading`, `table`, and nested `div`-as-SDT children are
/// supported inside the content control; anything else is dropped with a
/// fidelity warning (a reasonable subset -- SDT content covers the full
/// block-content grammar, and this bridge doesn't have a use case for
/// permission ranges/proof errors/etc. nested inside a content control).
fn build_sdt_block(
    node: &Node,
    warnings: &mut Vec<FidelityWarning>,
    hyperlink_map: &HashMap<String, String>,
    footnote_map: &HashMap<String, i64>,
    image_map: &HashMap<String, types::CTDrawing>,
) -> Result<types::CTSdtBlock, EmitError> {
    let mut block_content = Vec::new();
    for child in &node.children {
        match child.kind.as_str() {
            node::PARAGRAPH | node::HEADING => {
                let mut para = types::Paragraph::default();
                apply_para_props(&mut para, child);
                write_inline_to_para(
                    &mut para,
                    &child.children,
                    &FormattingState::default(),
                    warnings,
                    hyperlink_map,
                    footnote_map,
                    image_map,
                );
                block_content.push(types::BlockContentChoice::P(Box::new(para)));
            }
            node::TABLE => {
                let mut table = types::Table {
                    range_markup: Vec::new(),
                    table_properties: Box::new(types::TableProperties::default()),
                    tbl_grid: Box::new(types::TableGrid::default()),
                    rows: Vec::new(),
                    extra_children: Vec::new(),
                };
                write_table_into(
                    &mut table,
                    child,
                    warnings,
                    hyperlink_map,
                    footnote_map,
                    image_map,
                )?;
                block_content.push(types::BlockContentChoice::Tbl(Box::new(table)));
            }
            node::DIV
                if child.props.get_str("docx:sdt-tag").is_some()
                    || child.props.get_str("docx:sdt-alias").is_some()
                    || child.props.get_str("docx:sdt-type").is_some() =>
            {
                let nested =
                    build_sdt_block(child, warnings, hyperlink_map, footnote_map, image_map)?;
                block_content.push(types::BlockContentChoice::Sdt(Box::new(nested)));
            }
            other => {
                warn(
                    warnings,
                    format!("'{}' inside SDT content not supported; dropped", other),
                );
            }
        }
    }

    let mut sdt_pr = types::CTSdtPr::default();
    if let Some(tag) = node.props.get_str("docx:sdt-tag") {
        sdt_pr.tag = Some(Box::new(types::CTString {
            value: tag.to_string(),
            extra_attrs: Default::default(),
        }));
    }
    if let Some(alias) = node.props.get_str("docx:sdt-alias") {
        sdt_pr.alias = Some(Box::new(types::CTString {
            value: alias.to_string(),
            extra_attrs: Default::default(),
        }));
    }
    if let Some(sdt_type) = node.props.get_str("docx:sdt-type") {
        apply_sdt_type(&mut sdt_pr, sdt_type);
    }

    Ok(types::CTSdtBlock {
        sdt_pr: Some(Box::new(sdt_pr)),
        sdt_end_pr: None,
        sdt_content: Some(Box::new(types::CTSdtContentBlock {
            block_content,
            extra_children: Vec::new(),
        })),
        extra_children: Vec::new(),
    })
}

/// Set the one `CTSdtPr` "type" child matching a `docx:sdt-type` value
/// raw-preserved by read.rs's `sdt_type_name`. Sub-structure of the
/// richer variants (`text`, `comboBox`, `dropDownList`, `date`,
/// `docPartObj`, `docPartList`) isn't itself raw-preserved, so round-trip
/// restores the *kind* of content control but not e.g. a combo box's list
/// of choices -- a known, documented limitation of this reasonable-subset
/// SDT implementation (see COVERAGE.md).
fn apply_sdt_type(pr: &mut types::CTSdtPr, sdt_type: &str) {
    match sdt_type {
        "text" => pr.text = Some(Box::new(types::CTSdtText::default())),
        "comboBox" => pr.combo_box = Some(Box::new(types::CTSdtComboBox::default())),
        "dropDownList" => pr.drop_down_list = Some(Box::new(types::CTSdtDropDownList::default())),
        "date" => pr.date = Some(Box::new(types::CTSdtDate::default())),
        "richText" => pr.rich_text = Some(Box::new(types::CTEmpty)),
        "picture" => pr.picture = Some(Box::new(types::CTEmpty)),
        "citation" => pr.citation = Some(Box::new(types::CTEmpty)),
        "group" => pr.group = Some(Box::new(types::CTEmpty)),
        "bibliography" => pr.bibliography = Some(Box::new(types::CTEmpty)),
        "equation" => pr.equation = Some(Box::new(types::CTEmpty)),
        "docPartObj" => pr.doc_part_obj = Some(Box::new(types::CTSdtDocPart::default())),
        "docPartList" => pr.doc_part_list = Some(Box::new(types::CTSdtDocPart::default())),
        _ => {}
    }
}

/// Walk a `list` IR node (and any `list` nodes nested inside its
/// `list_item` children) recording the `ordered` flag seen at each nesting
/// depth. If a depth is reached by more than one list in the tree with
/// different `ordered` values, the first one encountered (document order)
/// wins -- DOCX's numbering levels are per-`numId`, not per-list-node, so a
/// single custom list can only carry one format per depth.
fn collect_list_levels(list_node: &Node, depth: u32, levels: &mut HashMap<u32, bool>) {
    let ordered = list_node.props.get_bool(prop::ORDERED).unwrap_or(false);
    levels.entry(depth).or_insert(ordered);
    for item in &list_node.children {
        if item.kind.as_str() != node::LIST_ITEM {
            continue;
        }
        // A raw-preserved `docx:ilvl` (see read.rs) can exceed the tree
        // depth in a level-skipping source list; make sure a level
        // definition still exists for it so `write_list_items`'s
        // `set_numbering(num_id, ilvl)` never references an undefined level.
        if let Some(raw_ilvl) = item.props.get_int("docx:ilvl") {
            levels.entry(raw_ilvl as u32).or_insert(ordered);
        }
        for child in &item.children {
            if child.kind.as_str() == node::LIST {
                collect_list_levels(child, depth + 1, levels);
            }
        }
    }
}

/// Write a `list` node's items at the given depth, recursing into any
/// nested `list` node found among a `list_item`'s children at `depth + 1`.
/// All items across the whole nested tree share one `num_id` (registered by
/// the caller via `collect_list_levels` + `add_custom_list`); only `ilvl`
/// changes with depth.
#[allow(clippy::too_many_arguments)]
fn write_list_items(
    builder: &mut DocumentBuilder,
    list_node: &Node,
    depth: u32,
    num_id: u32,
    warnings: &mut Vec<FidelityWarning>,
    hyperlink_map: &HashMap<String, String>,
    footnote_map: &HashMap<String, i64>,
    image_map: &HashMap<String, types::CTDrawing>,
) -> Result<(), EmitError> {
    for item in &list_node.children {
        if item.kind.as_str() != node::LIST_ITEM {
            continue;
        }
        let para = builder.body_mut().add_paragraph();
        // Prefer the raw-preserved source `ilvl` (see read.rs's `docx:ilvl`
        // comment) over the depth recomputed from IR nesting structure --
        // this restores level-skipping numbering (e.g. a list that starts
        // directly at ilvl=2) that the tree-derived depth can't represent.
        let ilvl = item
            .props
            .get_int("docx:ilvl")
            .map(|v| v as u32)
            .unwrap_or(depth);
        para.set_numbering(num_id, ilvl);
        let inline_children: Vec<&Node> = item
            .children
            .iter()
            .filter(|c| c.kind.as_str() != node::LIST)
            .collect();
        // write_inline_to_para takes owned slices of Node, not references;
        // items without a nested list can pass their children directly.
        if inline_children.len() == item.children.len() {
            write_inline_to_para(
                para,
                &item.children,
                &FormattingState::default(),
                warnings,
                hyperlink_map,
                footnote_map,
                image_map,
            );
        } else {
            let owned: Vec<Node> = inline_children.into_iter().cloned().collect();
            write_inline_to_para(
                para,
                &owned,
                &FormattingState::default(),
                warnings,
                hyperlink_map,
                footnote_map,
                image_map,
            );
        }
        for child in &item.children {
            if child.kind.as_str() == node::LIST {
                write_list_items(
                    builder,
                    child,
                    depth + 1,
                    num_id,
                    warnings,
                    hyperlink_map,
                    footnote_map,
                    image_map,
                )?;
            }
        }
    }
    Ok(())
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
    let table = builder.body_mut().add_table();
    write_table_into(
        table,
        table_node,
        warnings,
        hyperlink_map,
        footnote_map,
        image_map,
    )
}

/// Write a `table` IR node into an existing (possibly nested, i.e. inside a
/// table cell) `types::Table`. Split out from [`write_table`] so a nested
/// table (a `table` node appearing among a cell's children) can be written
/// without needing a `&mut DocumentBuilder` -- only the top-level call gets
/// its `types::Table` from `builder.body_mut().add_table()`.
fn write_table_into(
    table: &mut types::Table,
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
                for child_node in &cell_node.children {
                    if child_node.kind.as_str() == node::TABLE {
                        let mut nested = types::Table {
                            range_markup: Vec::new(),
                            table_properties: Box::new(types::TableProperties::default()),
                            tbl_grid: Box::new(types::TableGrid::default()),
                            rows: Vec::new(),
                            extra_children: Vec::new(),
                        };
                        write_table_into(
                            &mut nested,
                            child_node,
                            warnings,
                            hyperlink_map,
                            footnote_map,
                            image_map,
                        )?;
                        out_cell
                            .block_content
                            .push(types::BlockContent::Tbl(Box::new(nested)));
                    } else {
                        let para = out_cell.add_paragraph();
                        write_inline_to_para(
                            para,
                            &child_node.children,
                            &FormattingState::default(),
                            warnings,
                            hyperlink_map,
                            footnote_map,
                            image_map,
                        );
                    }
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
    run_style: Option<String>,
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
                // Restore the exact source `rStyle` if the reader captured
                // one (see read.rs's `is_code_run_style`); otherwise fall
                // back to Word's own built-in monospace run style.
                let mut next = fmt.clone();
                next.run_style = Some(
                    node.props
                        .get_str("docx:rStyle")
                        .unwrap_or("HTMLTypewriter")
                        .to_string(),
                );
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
            node::RAW_INLINE if node.props.get_str("docx:field-instr").is_some() => {
                write_field_to_para(
                    para,
                    node,
                    fmt,
                    warnings,
                    hyperlink_map,
                    footnote_map,
                    image_map,
                );
            }
            node::RAW_INLINE if node.props.get_str(prop::FORMAT) == Some("docx") => {
                write_raw_docx_marker(para, node);
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

/// Re-emit a raw-preserved bookmark/comment-range marker (see read.rs's
/// `raw_inline` construction for BookmarkStart/End and
/// CommentRangeStart/End) as the matching `ParagraphContent` variant.
/// Re-emit a raw-preserved complex field (see read.rs's `FieldPhase`/
/// `convert_run` field-code handling) as the `fldChar[begin]` /
/// `instrText` / `fldChar[separate]` / display-content / `fldChar[end]`
/// run sequence.
fn write_field_to_para(
    para: &mut types::Paragraph,
    node: &Node,
    fmt: &FormattingState,
    warnings: &mut Vec<FidelityWarning>,
    hyperlink_map: &HashMap<String, String>,
    footnote_map: &HashMap<String, i64>,
    image_map: &HashMap<String, types::CTDrawing>,
) {
    let instr = node.props.get_str("docx:field-instr").unwrap_or("");

    let begin_run = para.add_run();
    apply_run_formatting(begin_run, fmt);
    begin_run
        .run_content
        .push(types::RunContent::FldChar(Box::new(field_char(
            types::STFldCharType::Begin,
        ))));

    if !instr.is_empty() {
        let instr_run = para.add_run();
        apply_run_formatting(instr_run, fmt);
        instr_run
            .run_content
            .push(types::RunContent::InstrText(Box::new(types::Text {
                text: Some(instr.to_string()),
                extra_children: Vec::new(),
            })));
    }

    let separate_run = para.add_run();
    apply_run_formatting(separate_run, fmt);
    separate_run
        .run_content
        .push(types::RunContent::FldChar(Box::new(field_char(
            types::STFldCharType::Separate,
        ))));

    write_inline_to_para(
        para,
        &node.children,
        fmt,
        warnings,
        hyperlink_map,
        footnote_map,
        image_map,
    );

    let end_run = para.add_run();
    apply_run_formatting(end_run, fmt);
    end_run
        .run_content
        .push(types::RunContent::FldChar(Box::new(field_char(
            types::STFldCharType::End,
        ))));
}

fn field_char(fld_char_type: types::STFldCharType) -> types::CTFldChar {
    types::CTFldChar {
        fld_char_type,
        fld_lock: None,
        dirty: None,
        fld_data: None,
        ff_data: None,
        numbering_change: None,
        extra_attrs: Default::default(),
        extra_children: Vec::new(),
    }
}

fn write_raw_docx_marker(para: &mut types::Paragraph, node: &Node) {
    if let Some(id) = node.props.get_str("docx:bookmark-start-id") {
        let name = node.props.get_str("docx:bookmark-start-name").unwrap_or("");
        if let Ok(id) = id.parse::<i64>() {
            para.add_bookmark_start(id, name);
        }
    } else if let Some(id) = node.props.get_str("docx:bookmark-end-id") {
        if let Ok(id) = id.parse::<i64>() {
            para.add_bookmark_end(id);
        }
    } else if let Some(id) = node.props.get_str("docx:comment-range-start-id") {
        if let Ok(id) = id.parse::<u32>() {
            para.add_comment_range_start(id);
        }
    } else if let Some(id) = node.props.get_str("docx:comment-range-end-id") {
        if let Ok(id) = id.parse::<u32>() {
            para.add_comment_range_end(id);
        }
    } else if let Some(id) = node.props.get_str("docx:comment-ref-id")
        && let Ok(id) = id.parse::<i64>()
    {
        let run = para.add_run();
        run.add_comment_ref(id);
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
    if let Some(ref style) = fmt.run_style {
        let rpr = run
            .r_pr
            .get_or_insert_with(|| Box::new(types::RunProperties::default()));
        rpr.run_style = Some(Box::new(types::CTString {
            value: style.clone(),
            extra_attrs: Default::default(),
        }));
    }
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
