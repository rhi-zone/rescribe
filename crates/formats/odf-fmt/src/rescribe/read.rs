//! ODT (OpenDocument Text) reader for rescribe.
//!
//! Parses ODF/ODT documents into rescribe's document IR by delegating to
//! the rest of this crate for all ZIP unpacking and XML parsing.

use crate::ast::{
    DrawShape, DrawShapeContent, Inline, ListItem, NoteClass, OdfBody, OdfDocument,
    PresentationBody, SheetCell, SpreadsheetBody, StyleEntry, TextBlock,
};
use rescribe_core::{
    ConversionResult, Document, FidelityWarning, ParseError, ParseOptions, Properties, Resource,
    ResourceId, ResourceMap, Severity, WarningKind,
};
use rescribe_std::{Node, node, prop};

/// Parse ODT input into a document.
pub fn parse(input: &[u8]) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse ODT input into a document with options.
pub fn parse_with_options(
    input: &[u8],
    _options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let result = crate::parse(input).map_err(|e| ParseError::Invalid(e.to_string()))?;
    let odf_doc = result.value;
    convert_document(odf_doc)
}

// ── Document conversion ───────────────────────────────────────────────────────

fn convert_document(odf: OdfDocument) -> Result<ConversionResult<Document>, ParseError> {
    // Metadata
    let mut metadata = Properties::new();
    if let Some(v) = &odf.meta.title {
        metadata.set("title", v.as_str());
    }
    if let Some(v) = &odf.meta.creator {
        metadata.set("author", v.as_str());
    }
    if let Some(v) = &odf
        .meta
        .modification_date
        .as_ref()
        .or(odf.meta.creation_date.as_ref())
    {
        metadata.set("date", v.as_str());
    }
    if let Some(v) = &odf.meta.description {
        metadata.set("description", v.as_str());
    }
    if let Some(v) = &odf.meta.subject {
        metadata.set("subject", v.as_str());
    }
    if !odf.meta.keywords.is_empty() {
        metadata.set("keywords", odf.meta.keywords.join(", "));
    }
    if let Some(v) = &odf.meta.language {
        metadata.set("language", v.as_str());
    }
    for (name, value) in &odf.meta.user_defined {
        metadata.set(format!("meta:{name}"), value.as_str());
    }

    // Page layout from first page layout entry
    if let Some(pl) = odf.page_layouts.first() {
        if let Some(v) = &pl.page_width {
            metadata.set("page-width", v.as_str());
        }
        if let Some(v) = &pl.page_height {
            metadata.set("page-height", v.as_str());
        }
        if let Some(v) = &pl.margin_top {
            metadata.set("margin-top", v.as_str());
        }
        if let Some(v) = &pl.margin_bottom {
            metadata.set("margin-bottom", v.as_str());
        }
        if let Some(v) = &pl.margin_left {
            metadata.set("margin-left", v.as_str());
        }
        if let Some(v) = &pl.margin_right {
            metadata.set("margin-right", v.as_str());
        }
    }

    // Embedded images
    let mut resources = ResourceMap::new();
    let mut image_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for (path, data) in &odf.images {
        if !data.is_empty() {
            let mime = mime_from_name(path);
            let res_id = ResourceId::new();
            let id_str = res_id.as_str().to_owned();
            resources.insert(
                res_id,
                Resource::new(mime, data.clone()).with_name(path.clone()),
            );
            image_map.insert(path.clone(), id_str);
        }
    }

    // Other raw-preserved package parts (settings.xml, RDF metadata) have no
    // IR node representation (see `ast::OdfDocument::extra_parts`), but are
    // exposed as resources so a caller that round-trips through `Document`
    // doesn't lose them, and so a caller that *does* want the RDF graph can
    // get the bytes without going around this crate's `rescribe` feature.
    for (path, data) in &odf.extra_parts {
        if data.is_empty() {
            continue;
        }
        let mime = if path.ends_with(".rdf") {
            "application/rdf+xml"
        } else {
            "text/xml"
        };
        resources.insert(
            ResourceId::new(),
            Resource::new(mime, data.clone()).with_name(path.clone()),
        );
    }

    // Style maps: merge named + automatic
    let ctx = StyleCtx {
        named: &odf.named_styles,
        auto: &odf.automatic_styles,
        image_map: &image_map,
        list_styles: &odf.list_styles,
    };

    // Convert body
    match &odf.body {
        OdfBody::Text(blocks) => {
            let doc = convert_text_body(blocks, &ctx);
            Ok(ConversionResult::ok(Document {
                content: doc,
                resources,
                metadata,
                source: None,
            }))
        }
        OdfBody::Empty => {
            let doc = convert_text_body(&[], &ctx);
            Ok(ConversionResult::ok(Document {
                content: doc,
                resources,
                metadata,
                source: None,
            }))
        }
        OdfBody::Spreadsheet(body) => {
            let (doc, warnings) = convert_spreadsheet_body(body, &ctx);
            Ok(ConversionResult::with_warnings(
                Document {
                    content: doc,
                    resources,
                    metadata,
                    source: None,
                },
                warnings,
            ))
        }
        OdfBody::Presentation(body) => {
            let (doc, warnings) = convert_presentation_body(body, &ctx);
            Ok(ConversionResult::with_warnings(
                Document {
                    content: doc,
                    resources,
                    metadata,
                    source: None,
                },
                warnings,
            ))
        }
    }
}

/// Convert an `office:text` body's blocks into the `document` node,
/// resolving blockquote runs and footnote defs (see `pending_blockquote`/
/// `pending_footnotes` below).
fn convert_text_body(body_blocks: &[TextBlock], ctx: &StyleCtx<'_>) -> Node {
    let mut doc = Node::new(node::DOCUMENT);
    let mut pending_footnotes: Vec<Node> = Vec::new();
    let mut pending_blockquote: Option<Vec<Node>> = None;

    for block in body_blocks {
        let (nodes, footnotes) = convert_block(block, ctx);
        pending_footnotes.extend(footnotes);
        for n in nodes {
            let is_bq = n.kind.as_str() == node::PARAGRAPH
                && n.props.get_str("odt:is-blockquote").is_some();
            if is_bq {
                pending_blockquote.get_or_insert_with(Vec::new).push({
                    let mut stripped = n.clone();
                    stripped.props.remove("odt:is-blockquote");
                    stripped
                });
            } else {
                flush_pending_blockquote(&mut pending_blockquote, &mut doc);
                doc = doc.child(n);
                for fn_def in pending_footnotes.drain(..) {
                    doc = doc.child(fn_def);
                }
            }
        }
    }
    flush_pending_blockquote(&mut pending_blockquote, &mut doc);
    doc
}

// ── Spreadsheet body conversion (ADR 0015) ─────────────────────────────────────

fn convert_spreadsheet_body(
    body: &SpreadsheetBody,
    ctx: &StyleCtx<'_>,
) -> (Node, Vec<FidelityWarning>) {
    let mut doc = Node::new(node::DOCUMENT);
    let mut warnings = Vec::new();
    for sheet in &body.sheets {
        let mut sheet_node = Node::new(node::SHEET);
        if let Some(name) = &sheet.name {
            sheet_node = sheet_node.prop("odf:name", name.as_str());
        }
        if let Some(sn) = &sheet.style_name {
            sheet_node = sheet_node.prop("odf:style-name", sn.as_str());
        }
        if sheet.print {
            sheet_node = sheet_node.prop("odf:print", true);
        }
        for row in &sheet.rows {
            let mut row_node = Node::new(node::SHEET_ROW);
            if let Some(sn) = &row.style_name {
                row_node = row_node.prop("odf:style-name", sn.as_str());
            }
            if let Some(r) = row.repeated.filter(|&v| v > 1) {
                row_node = row_node.prop("odf:repeated", r as i64);
            }
            for cell in &row.cells {
                row_node = row_node.child(convert_sheet_cell(cell, ctx));
            }
            sheet_node = sheet_node.child(row_node);
        }
        // Floating shapes anchored to the sheet (`<table:shapes>`), e.g. an
        // embedded chart (ADR 0016) — siblings of the `sheet_row` children,
        // reusing the same `positioned_container` shape conversion as ODP.
        for (i, shape) in sheet.shapes.iter().enumerate() {
            let (shape_node, mut w) = convert_draw_shape(shape, i as i64, ctx);
            warnings.append(&mut w);
            sheet_node = sheet_node.child(shape_node);
        }
        doc = doc.child(sheet_node);
    }
    (doc, warnings)
}

fn convert_sheet_cell(cell: &SheetCell, ctx: &StyleCtx<'_>) -> Node {
    let mut n = Node::new(node::SHEET_CELL);
    if let Some(vt) = &cell.value_type {
        n = n.prop(prop::VALUE_TYPE, map_odf_value_type(vt));
    }
    if let Some(v) = &cell.value {
        n = n.prop(prop::VALUE, v.as_str());
    }
    if let Some(f) = &cell.formula {
        n = n.prop(prop::VALUE_FORMULA, f.as_str());
    }
    if let Some(sn) = &cell.style_name {
        n = n.prop("odf:style-name", sn.as_str());
    }
    if let Some(cs) = cell.col_span.filter(|&v| v > 1) {
        n = n.prop(prop::COLSPAN, cs as i64);
    }
    if let Some(rs) = cell.row_span.filter(|&v| v > 1) {
        n = n.prop(prop::ROWSPAN, rs as i64);
    }
    if let Some(r) = cell.repeated.filter(|&v| v > 1) {
        n = n.prop("odf:repeated", r as i64);
    }
    if cell.covered {
        n = n.prop("odf:covered", true);
    }
    for block in &cell.content {
        let (nodes, _footnotes) = convert_block(block, ctx);
        for cn in nodes {
            n = n.child(cn);
        }
    }
    n
}

/// Map ODF's `office:value-type` string to `prop::VALUE_TYPE`'s vocabulary
/// (ADR 0015). Only `"float"` differs in spelling (`"number"` in the IR,
/// matching OOXML SpreadsheetML's `Number` case — see ADR 0015 Decision 2);
/// every other ODF value-type name is passed through unchanged since it
/// already matches the IR's union vocabulary.
fn map_odf_value_type(odf_type: &str) -> &'static str {
    match odf_type {
        "float" => "number",
        "percentage" => "percentage",
        "currency" => "currency",
        "date" => "date",
        "time" => "time",
        "boolean" => "boolean",
        "string" => "string",
        _ => "string",
    }
}

// ── Presentation body conversion (ADR 0015) ────────────────────────────────────

fn convert_presentation_body(
    body: &PresentationBody,
    ctx: &StyleCtx<'_>,
) -> (Node, Vec<FidelityWarning>) {
    let mut doc = Node::new(node::DOCUMENT);
    let mut warnings = Vec::new();
    for page in &body.pages {
        let mut page_node = Node::new(node::DIV).prop("odf:type", "slide");
        if let Some(name) = &page.name {
            page_node = page_node.prop("odf:name", name.as_str());
        }
        if let Some(sn) = &page.style_name {
            page_node = page_node.prop("odf:style-name", sn.as_str());
        }
        if let Some(mp) = &page.master_page_name {
            page_node = page_node.prop("odf:master-page-name", mp.as_str());
        }
        if let Some(ln) = &page.layout_name {
            page_node = page_node.prop("odf:layout-name", ln.as_str());
        }
        for (i, shape) in page.shapes.iter().enumerate() {
            let (shape_node, mut w) = convert_draw_shape(shape, i as i64, ctx);
            warnings.append(&mut w);
            page_node = page_node.child(shape_node);
        }
        if let Some(notes) = &page.notes {
            let mut notes_node = Node::new(node::DIV).prop("odf:type", "notes");
            if let Some(sn) = &notes.style_name {
                notes_node = notes_node.prop("odf:style-name", sn.as_str());
            }
            for (i, shape) in notes.shapes.iter().enumerate() {
                let (shape_node, mut w) = convert_draw_shape(shape, i as i64, ctx);
                warnings.append(&mut w);
                notes_node = notes_node.child(shape_node);
            }
            page_node = page_node.child(notes_node);
        }
        doc = doc.child(page_node);
    }
    (doc, warnings)
}

fn convert_draw_shape(
    shape: &DrawShape,
    doc_order_index: i64,
    ctx: &StyleCtx<'_>,
) -> (Node, Vec<FidelityWarning>) {
    let mut n = Node::new(node::POSITIONED_CONTAINER);
    let mut warnings = Vec::new();

    if let Some(x) = &shape.x {
        n = n.prop("odf:x", x.as_str());
        if let Some(emu) = parse_odf_length(x) {
            n = n.prop(prop::POSITION_X, emu);
        }
    }
    if let Some(y) = &shape.y {
        n = n.prop("odf:y", y.as_str());
        if let Some(emu) = parse_odf_length(y) {
            n = n.prop(prop::POSITION_Y, emu);
        }
    }
    if let Some(w) = &shape.width {
        n = n.prop("odf:width", w.as_str());
        if let Some(emu) = parse_odf_length(w) {
            n = n.prop(prop::POSITION_WIDTH, emu);
        }
    }
    if let Some(h) = &shape.height {
        n = n.prop("odf:height", h.as_str());
        if let Some(emu) = parse_odf_length(h) {
            n = n.prop(prop::POSITION_HEIGHT, emu);
        }
    }

    if let Some(t) = &shape.transform {
        n = n.prop("odf:transform", t.as_str());
        match parse_pure_rotate_degrees(t) {
            Some(degrees) => {
                n = n.prop(prop::POSITION_ROTATION, degrees_to_ooxml_units(degrees));
            }
            None => {
                warnings.push(FidelityWarning::new(
                    Severity::Minor,
                    WarningKind::Simplified("draw:transform".to_owned()),
                    "shape draw:transform is not a pure rotate(); preserved verbatim as \
                     odf:transform but not projected to position:rotation (ADR 0015: only a \
                     pure rotate() around a center-equivalent pivot converts losslessly)",
                ));
            }
        }
    }

    // ADR 0015: z-order round-trips losslessly everywhere, so it's always
    // set — from ODF's explicit draw:z-index when present, or derived from
    // document order (ODF's own default stacking rule) otherwise.
    n = n.prop(
        prop::POSITION_Z_ORDER,
        shape.z_index.unwrap_or(doc_order_index),
    );

    if let Some(name) = &shape.name {
        n = n.prop("odf:name", name.as_str());
    }
    if let Some(sn) = &shape.style_name {
        n = n.prop("odf:style-name", sn.as_str());
    }
    if let Some(ts) = &shape.text_style_name {
        n = n.prop("odf:text-style-name", ts.as_str());
    }
    if let Some(pc) = &shape.presentation_class {
        n = n.prop("odf:presentation-class", pc.as_str());
    }

    match &shape.content {
        DrawShapeContent::TextBox(blocks) => {
            for block in blocks {
                let (nodes, _footnotes) = convert_block(block, ctx);
                for cn in nodes {
                    n = n.child(cn);
                }
            }
        }
        DrawShapeContent::Image { href, mime_type } => {
            let src = ctx
                .image_map
                .get(href)
                .map(String::as_str)
                .unwrap_or(href.as_str());
            let mut img = Node::new(node::IMAGE).prop("src", src);
            if let Some(mt) = mime_type {
                img = img.prop("odf:mime-type", mt.as_str());
            }
            n = n.child(img);
        }
        DrawShapeContent::Other(raw) => {
            n = n.child(
                Node::new(node::RAW_BLOCK)
                    .prop(prop::FORMAT, "odf")
                    .prop(prop::CONTENT, raw.as_str()),
            );
        }
        DrawShapeContent::Chart { chart, .. } => {
            n = n.child(convert_chart(chart));
        }
        DrawShapeContent::Empty => {}
    }

    (n, warnings)
}

/// Convert an embedded `Chart` (ADR 0016) to a `chart` IR node.
fn convert_chart(chart: &crate::ast::Chart) -> Node {
    let mut n = Node::new(node::CHART);
    if let Some(title) = &chart.title {
        n = n.prop(prop::TITLE, title.as_str());
    }
    if let Some(class) = &chart.chart_class {
        n = n.prop(prop::CHART_TYPE, class.as_str());
    }
    n = n.prop(prop::CHART_LEGEND, chart.legend);
    if chart.legend
        && let Some(pos) = &chart.legend_position
    {
        n = n.prop(prop::CHART_LEGEND_POSITION, pos.as_str());
    }
    n = n.prop(prop::CHART_HAS_CATEGORY_AXIS, chart.has_category_axis);
    n = n.prop(prop::CHART_HAS_VALUE_AXIS, chart.has_value_axis);
    // Unconditional raw fallback (ADR 0016 Decision 4) — the v1 semantic
    // fields above are a subset, not a lossy projection; this is what keeps
    // read/write round-trippable regardless of what the subset doesn't cover.
    n = n.prop(prop::ODF_CHART_XML, chart.raw_xml.as_str());

    for series in &chart.series {
        let mut s = Node::new(node::CHART_SERIES);
        if let Some(vref) = &series.values_cell_range_address {
            s = s.prop(prop::CHART_VALUES_REF, vref.as_str());
        }
        if let Some(cref) = &series.categories_cell_range_address {
            s = s.prop(prop::CHART_CATEGORIES_REF, cref.as_str());
        }
        // ODF has no per-series title text (only a cell reference via
        // `chart:label-cell-address`, and ODF has no cached-value snapshot
        // to resolve it against — see `ast::ChartSeries`'s doc comment), so
        // `TITLE` is intentionally left unset here.
        n = n.child(s);
    }

    n
}

/// Parse an ODF `svg:length`/`text:coordinate` attribute (a decimal number
/// with a mandatory unit suffix) into EMU (ADR 0015 Decision 4). Returns
/// `None` for unrecognized or missing units rather than guessing.
fn parse_odf_length(s: &str) -> Option<i64> {
    let s = s.trim();
    let split_at = s.find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == '+'))?;
    let (num_part, unit) = s.split_at(split_at);
    let value: f64 = num_part.parse().ok()?;
    let emu_per_unit = match unit {
        "in" => 914_400.0,
        "cm" => 360_000.0,
        "mm" => 36_000.0,
        "pt" => 12_700.0,
        "pc" => 152_400.0,
        "px" => 9_525.0,
        _ => return None,
    };
    Some((value * emu_per_unit).round() as i64)
}

/// Parse a `draw:transform` string as a *pure* `rotate(<angle>)` — the only
/// case ADR 0015 Decision 5 accepts as losslessly convertible to a single
/// OOXML-style rotation angle, since ODF's rotation pivot is a documented
/// interop ambiguity for any more complex transform. Returns `None` for
/// anything else (combined transforms, non-numeric arguments, absence).
///
/// ODF's `angle` datatype (used by `rotate()`'s argument) is decimal
/// **degrees by default**, with an optional `deg`/`grad`/`rad` unit suffix
/// (ADR 0015 Decision 5) — unlike SVG's `transform` attribute, which this
/// otherwise resembles syntactically but which defaults to degrees too, so
/// there is no cross-format radians default to assume here.
fn parse_pure_rotate_degrees(t: &str) -> Option<f64> {
    let t = t.trim();
    let inner = t.strip_prefix("rotate(")?.strip_suffix(')')?.trim();
    let (num_part, unit) = match inner.find(|c: char| {
        !(c.is_ascii_digit() || c == '.' || c == '-' || c == '+' || c == 'e' || c == 'E')
    }) {
        Some(idx) => inner.split_at(idx),
        None => (inner, ""),
    };
    let value: f64 = num_part.parse().ok()?;
    match unit.trim() {
        "" | "deg" => Some(value),
        "grad" => Some(value * 0.9),
        "rad" => Some(value.to_degrees()),
        _ => None,
    }
}

/// Convert degrees to OOXML `ST_Angle`-style 60,000ths-of-a-degree (ADR 0015).
fn degrees_to_ooxml_units(degrees: f64) -> i64 {
    (degrees * 60_000.0).round() as i64
}

// ── Style context ─────────────────────────────────────────────────────────────

struct StyleCtx<'a> {
    named: &'a [StyleEntry],
    auto: &'a [StyleEntry],
    image_map: &'a std::collections::HashMap<String, String>,
    list_styles: &'a [(String, bool)],
}

impl<'a> StyleCtx<'a> {
    fn find_style(&self, name: &str) -> Option<&StyleEntry> {
        self.auto
            .iter()
            .find(|s| s.name == name)
            .or_else(|| self.named.iter().find(|s| s.name == name))
    }

    fn is_ordered_list_style(&self, name: &str) -> Option<bool> {
        self.list_styles
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, o)| *o)
    }
}

// ── Para-kind resolution ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum ParaKind {
    Normal,
    Heading(u8),
    Code,
    Blockquote,
    HorizontalRule,
}

fn resolve_para_kind(
    style_name: Option<&str>,
    is_heading_tag: bool,
    outline_level: Option<u32>,
    ctx: &StyleCtx<'_>,
) -> ParaKind {
    if is_heading_tag {
        let level = outline_level.unwrap_or(1).min(6) as u8;
        return ParaKind::Heading(level.max(1));
    }

    let name = match style_name {
        Some(n) if !n.is_empty() => n,
        _ => return ParaKind::Normal,
    };

    // Check style entry first
    if let Some(entry) = ctx.find_style(name) {
        // Use display_name or name for heuristics
        let check = entry.display_name.as_deref().unwrap_or(&entry.name);
        if let k @ (ParaKind::Code
        | ParaKind::Blockquote
        | ParaKind::HorizontalRule
        | ParaKind::Heading(_)) = para_kind_from_name(check)
        {
            return k;
        }
        // Also check parent style
        if let Some(parent) = &entry.parent_style_name
            && let Some(parent_entry) = ctx.find_style(parent)
        {
            let pcheck = parent_entry
                .display_name
                .as_deref()
                .unwrap_or(&parent_entry.name);
            if let k @ (ParaKind::Code
            | ParaKind::Blockquote
            | ParaKind::HorizontalRule
            | ParaKind::Heading(_)) = para_kind_from_name(pcheck)
            {
                return k;
            }
        }
    }

    // Heuristic on raw style name
    para_kind_from_name(name)
}

fn para_kind_from_name(name: &str) -> ParaKind {
    let lower = name.to_lowercase();
    if lower.starts_with("heading") {
        let suffix = lower.trim_start_matches("heading").trim();
        if let Some(c) = suffix.chars().next().filter(|c| c.is_ascii_digit()) {
            let level = ((c as u8) - b'0').min(6);
            return ParaKind::Heading(level.max(1));
        }
        return ParaKind::Heading(1);
    }
    if lower.contains("preformat")
        || lower.contains("code")
        || lower.contains("monospace")
        || lower.contains("verbatim")
        || lower == "source text"
    {
        return ParaKind::Code;
    }
    if lower.contains("quotation") || lower.contains("blockquote") || lower.contains("quote") {
        return ParaKind::Blockquote;
    }
    if lower.contains("horizontal") || lower.contains("hrule") || lower.contains("h-rule") {
        return ParaKind::HorizontalRule;
    }
    ParaKind::Normal
}

// ── Block conversion ──────────────────────────────────────────────────────────

/// Returns (block nodes, footnote_def nodes collected during conversion).
fn convert_block(block: &TextBlock, ctx: &StyleCtx<'_>) -> (Vec<Node>, Vec<Node>) {
    match block {
        TextBlock::Paragraph(p) => {
            let kind = resolve_para_kind(p.style_name.as_deref(), false, None, ctx);
            let (children, footnotes) = convert_inlines(&p.content, ctx);

            let node = match kind {
                ParaKind::Code => {
                    let content = extract_text_from_children(&children);
                    Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content)
                }
                ParaKind::HorizontalRule => Node::new(node::HORIZONTAL_RULE),
                ParaKind::Blockquote => {
                    // Mark for blockquote accumulation
                    let mut n = Node::new(node::PARAGRAPH);
                    for c in children {
                        n = n.child(c);
                    }
                    n = n.prop("odt:is-blockquote", "1");
                    if let Some(sn) = &p.style_name
                        && !sn.is_empty()
                    {
                        n = n.prop("odt:style-name", sn.as_str());
                    }
                    return (vec![n], footnotes);
                }
                _ => {
                    let mut n = Node::new(node::PARAGRAPH);
                    for c in children {
                        n = n.child(c);
                    }
                    if let Some(sn) = &p.style_name
                        && !sn.is_empty()
                    {
                        n = n.prop("odt:style-name", sn.as_str());
                    }
                    n
                }
            };

            // Apply para layout props from style
            let node = if let Some(sn) = &p.style_name {
                apply_para_props_from_style(node, sn, ctx)
            } else {
                node
            };

            (vec![node], footnotes)
        }

        TextBlock::Heading(h) => {
            let level = h.outline_level.unwrap_or(1).min(6) as u8;
            let (children, footnotes) = convert_inlines(&h.content, ctx);
            let mut n = Node::new(node::HEADING).prop(prop::LEVEL, level as i64);
            for c in children {
                n = n.child(c);
            }
            (vec![n], footnotes)
        }

        TextBlock::List(list) => {
            let ordered = is_ordered_list(list.style_name.as_deref(), ctx);
            let mut list_node = Node::new(node::LIST);
            if ordered {
                list_node = list_node.prop("ordered", true);
            }
            let mut all_footnotes = Vec::new();

            for item in &list.items {
                let (item_node, fn_defs) = convert_list_item(item, ctx);
                list_node = list_node.child(item_node);
                all_footnotes.extend(fn_defs);
            }

            (vec![list_node], all_footnotes)
        }

        TextBlock::Table(t) => {
            let mut table_node = Node::new(node::TABLE);
            let mut all_footnotes = Vec::new();

            for row in &t.rows {
                let mut row_node = Node::new(node::TABLE_ROW);
                for cell in &row.cells {
                    let mut cell_node = Node::new(node::TABLE_CELL);
                    if let Some(cs) = cell.col_span.filter(|&v| v > 1) {
                        cell_node = cell_node.prop(prop::COLSPAN, cs as i64);
                    }
                    if let Some(rs) = cell.row_span.filter(|&v| v > 1) {
                        cell_node = cell_node.prop(prop::ROWSPAN, rs as i64);
                    }
                    for block in &cell.content {
                        let (nodes, fn_defs) = convert_block(block, ctx);
                        for n in nodes {
                            cell_node = cell_node.child(n);
                        }
                        all_footnotes.extend(fn_defs);
                    }
                    row_node = row_node.child(cell_node);
                }
                table_node = table_node.child(row_node);
            }

            (vec![table_node], all_footnotes)
        }

        TextBlock::Section(s) => {
            let mut all_nodes = Vec::new();
            let mut all_footnotes = Vec::new();
            for block in &s.content {
                let (nodes, fn_defs) = convert_block(block, ctx);
                all_nodes.extend(nodes);
                all_footnotes.extend(fn_defs);
            }
            (all_nodes, all_footnotes)
        }

        TextBlock::Frame(frame) => convert_frame(frame, ctx),

        TextBlock::Unknown { .. } => (Vec::new(), Vec::new()),
    }
}

/// Convert a `<draw:frame>`'s children. A frame with a single `Image` or
/// `TextBox` child converts to a bare `image`/`div` node as before; a frame
/// with an image *and* a text-box (the common image+caption pattern) wraps
/// both in a `figure`/`caption` pair so neither child is dropped.
fn convert_frame(frame: &crate::ast::Frame, ctx: &StyleCtx<'_>) -> (Vec<Node>, Vec<Node>) {
    use crate::ast::FrameChild;

    let mut all_footnotes = Vec::new();
    let mut image_nodes = Vec::new();
    let mut other_nodes = Vec::new();

    for child in &frame.content.children {
        match child {
            FrameChild::Image { href, .. } => {
                let mut img = Node::new(node::IMAGE);
                let src = ctx
                    .image_map
                    .get(href)
                    .map(String::as_str)
                    .unwrap_or(href.as_str());
                img = img.prop("src", src);
                if let Some(n) = &frame.name {
                    img = img.prop("odt:name", n.as_str());
                }
                image_nodes.push(img);
            }
            FrameChild::TextBox(blocks) => {
                let mut div = Node::new(node::DIV);
                for block in blocks {
                    let (nodes, fn_defs) = convert_block(block, ctx);
                    for n in nodes {
                        div = div.child(n);
                    }
                    all_footnotes.extend(fn_defs);
                }
                other_nodes.push(div);
            }
            FrameChild::Other(_) => {
                // No cross-format meaning; nothing else to build a node from.
            }
        }
    }

    match (image_nodes.len(), other_nodes.len()) {
        (0, 0) => (Vec::new(), all_footnotes),
        (1, 0) => (image_nodes, all_footnotes),
        (0, _) => (other_nodes, all_footnotes),
        _ => {
            // Image plus at least one text-box (caption): wrap in `figure` so
            // both survive. The text-box's own div wrapper is kept as the
            // caption body (ODF's text-box may itself contain several
            // paragraphs, not just a single caption line).
            let mut figure = Node::new(node::FIGURE);
            for img in image_nodes {
                figure = figure.child(img);
            }
            for div in other_nodes {
                let caption = Node::new(node::CAPTION).children(div.children);
                figure = figure.child(caption);
            }
            (vec![figure], all_footnotes)
        }
    }
}

fn convert_list_item(item: &ListItem, ctx: &StyleCtx<'_>) -> (Node, Vec<Node>) {
    let mut item_node = Node::new(node::LIST_ITEM);
    let mut all_footnotes = Vec::new();

    for block in &item.content {
        let (nodes, fn_defs) = convert_block(block, ctx);
        for n in nodes {
            item_node = item_node.child(n);
        }
        all_footnotes.extend(fn_defs);
    }

    (item_node, all_footnotes)
}

// ── Inline conversion ─────────────────────────────────────────────────────────

fn convert_inlines(inlines: &[Inline], ctx: &StyleCtx<'_>) -> (Vec<Node>, Vec<Node>) {
    let mut nodes: Vec<Node> = Vec::new();
    let mut footnotes = Vec::new();

    for inline in inlines {
        let (mut ns, mut fns) = convert_inline(inline, ctx);
        footnotes.append(&mut fns);
        for n in ns.drain(..) {
            // Coalesce adjacent text nodes into a single node.
            if n.kind.as_str() == node::TEXT {
                let new_content = n.props.get_str(prop::CONTENT).unwrap_or("").to_owned();
                if let Some(last) = nodes.last_mut()
                    && last.kind.as_str() == node::TEXT
                    && last.children.is_empty()
                {
                    let prev = last.props.get_str(prop::CONTENT).unwrap_or("").to_owned();
                    let merged = prev + &new_content;
                    last.props.set(prop::CONTENT, merged.as_str());
                    continue;
                }
            }
            nodes.push(n);
        }
    }

    (nodes, footnotes)
}

fn convert_inline(inline: &Inline, ctx: &StyleCtx<'_>) -> (Vec<Node>, Vec<Node>) {
    match inline {
        Inline::Text(s) => {
            if s.is_empty() {
                (Vec::new(), Vec::new())
            } else {
                (
                    vec![Node::new(node::TEXT).prop(prop::CONTENT, s.as_str())],
                    Vec::new(),
                )
            }
        }

        Inline::Tab => (
            vec![Node::new(node::TEXT).prop(prop::CONTENT, "\t")],
            Vec::new(),
        ),

        Inline::SoftHyphen => (
            vec![Node::new(node::TEXT).prop(prop::CONTENT, "\u{00AD}")],
            Vec::new(),
        ),

        Inline::Space { count } => {
            let spaces = " ".repeat(*count as usize);
            (
                vec![Node::new(node::TEXT).prop(prop::CONTENT, spaces)],
                Vec::new(),
            )
        }

        Inline::LineBreak => (vec![Node::new(node::LINE_BREAK)], Vec::new()),

        Inline::SoftPageBreak => (Vec::new(), Vec::new()),

        Inline::Span(span) => {
            let (children, footnotes) = convert_inlines(&span.content, ctx);
            if children.is_empty() {
                return (Vec::new(), footnotes);
            }

            let style_name = span.style_name.as_deref().unwrap_or("");
            let wrapper = inline_kind_from_style(style_name, ctx);

            let result = wrap_inline_nodes(children, wrapper, style_name, ctx);
            (result, footnotes)
        }

        Inline::Hyperlink(link) => {
            let (children, footnotes) = convert_inlines(&link.content, ctx);
            let href = link.href.as_deref().unwrap_or("");
            let mut n = Node::new(node::LINK).prop(prop::URL, href);
            if let Some(title) = &link.title
                && !title.is_empty()
            {
                n = n.prop(prop::TITLE, title.as_str());
            }
            for c in children {
                n = n.child(c);
            }
            (vec![n], footnotes)
        }

        Inline::Note(note) => {
            let id = note.id.clone().unwrap_or_default();

            // Footnote ref inline
            let ref_node = Node::new(node::FOOTNOTE_REF).prop(prop::LABEL, id.as_str());

            // Footnote def node (collected and emitted after the paragraph)
            let mut def = Node::new(node::FOOTNOTE_DEF).prop(prop::LABEL, id.as_str());
            if note.note_class == NoteClass::Endnote {
                def = def.prop("odt:note-class", "endnote");
            }
            for block in &note.body {
                let (nodes, _) = convert_block(block, ctx);
                for n in nodes {
                    def = def.child(n);
                }
            }

            (vec![ref_node], vec![def])
        }

        Inline::Frame(frame) => {
            let (nodes, footnotes) = convert_block(&TextBlock::Frame(frame.clone()), ctx);
            (nodes, footnotes)
        }

        Inline::Field { value, .. } => {
            if value.is_empty() {
                (Vec::new(), Vec::new())
            } else {
                (
                    vec![Node::new(node::TEXT).prop(prop::CONTENT, value.as_str())],
                    Vec::new(),
                )
            }
        }

        Inline::Bookmark { name } => {
            if name.is_empty() {
                (Vec::new(), Vec::new())
            } else {
                let n = Node::new(node::SPAN).prop(prop::ID, name.as_str());
                (vec![n], Vec::new())
            }
        }

        Inline::Annotation { content } => {
            let n = Node::new(node::SPAN).prop("odt:annotation", content.as_str());
            (vec![n], Vec::new())
        }

        Inline::Unknown { .. } => (Vec::new(), Vec::new()),
    }
}

// ── Inline kind resolution from style ────────────────────────────────────────

#[derive(Clone)]
enum InlineKind {
    Plain,
    Strong,
    Emphasis,
    Underline,
    Strikeout,
    Code,
    Subscript,
    Superscript,
    Span {
        color: Option<String>,
        font_size: Option<String>,
        font_name: Option<String>,
        small_caps: bool,
    },
}

fn inline_kind_from_style(style_name: &str, ctx: &StyleCtx<'_>) -> InlineKind {
    if style_name.is_empty() {
        return InlineKind::Plain;
    }

    if let Some(entry) = ctx.find_style(style_name) {
        let p = &entry.text_props;
        // Check monospace font → code
        let is_mono = p
            .font_name
            .as_ref()
            .map(|f| {
                let lf = f.to_lowercase();
                lf.contains("courier")
                    || lf.contains("mono")
                    || lf.contains("consol")
                    || lf.contains("fixed")
                    || lf.contains("inconsolata")
                    || lf.contains("menlo")
                    || lf == "code2000"
                    || lf == "source code pro"
            })
            .unwrap_or(false);

        if is_mono {
            return InlineKind::Code;
        }
        if p.subscript {
            return InlineKind::Subscript;
        }
        if p.superscript {
            return InlineKind::Superscript;
        }
        if p.bold {
            return InlineKind::Strong;
        }
        if p.italic {
            return InlineKind::Emphasis;
        }
        if p.underline {
            return InlineKind::Underline;
        }
        if p.strikethrough {
            return InlineKind::Strikeout;
        }
        let is_small_caps = p.font_variant.as_deref() == Some("small-caps");
        if is_small_caps || p.color.is_some() || p.font_size.is_some() || p.font_name.is_some() {
            let is_non_mono_font = p.font_name.as_ref().map(|_f| !is_mono).unwrap_or(false);
            let font_name_for_span = if is_non_mono_font {
                p.font_name.clone()
            } else {
                None
            };
            return InlineKind::Span {
                color: p.color.clone(),
                font_size: p.font_size.clone(),
                font_name: font_name_for_span,
                small_caps: is_small_caps,
            };
        }
    }

    // Heuristic on style name
    let lower = style_name.to_lowercase();
    if lower.contains("code")
        || lower.contains("preformat")
        || lower.contains("verbatim")
        || lower.contains("monospace")
    {
        return InlineKind::Code;
    }
    if lower.contains("subscript") || lower == "sub" {
        return InlineKind::Subscript;
    }
    if lower.contains("superscript") || lower == "sup" {
        return InlineKind::Superscript;
    }
    if lower.contains("bold") {
        return InlineKind::Strong;
    }
    if lower.contains("italic") || lower.contains("oblique") {
        return InlineKind::Emphasis;
    }
    if lower.contains("underline") {
        return InlineKind::Underline;
    }
    if lower.contains("strike") {
        return InlineKind::Strikeout;
    }

    InlineKind::Plain
}

fn wrap_inline_nodes(
    children: Vec<Node>,
    kind: InlineKind,
    style_name: &str,
    _ctx: &StyleCtx<'_>,
) -> Vec<Node> {
    match kind {
        InlineKind::Plain => {
            // Pass-through: plain spans contribute no wrapper node
            let _ = style_name;
            children
        }
        InlineKind::Strong => {
            let mut n = Node::new(node::STRONG);
            for c in children {
                n = n.child(c);
            }
            vec![n]
        }
        InlineKind::Emphasis => {
            let mut n = Node::new(node::EMPHASIS);
            for c in children {
                n = n.child(c);
            }
            vec![n]
        }
        InlineKind::Underline => {
            let mut n = Node::new(node::UNDERLINE);
            for c in children {
                n = n.child(c);
            }
            vec![n]
        }
        InlineKind::Strikeout => {
            let mut n = Node::new(node::STRIKEOUT);
            for c in children {
                n = n.child(c);
            }
            vec![n]
        }
        InlineKind::Code => {
            let content = extract_text_from_children(&children);
            vec![Node::new(node::CODE).prop(prop::CONTENT, content)]
        }
        InlineKind::Subscript => {
            let mut n = Node::new(node::SUBSCRIPT);
            for c in children {
                n = n.child(c);
            }
            vec![n]
        }
        InlineKind::Superscript => {
            let mut n = Node::new(node::SUPERSCRIPT);
            for c in children {
                n = n.child(c);
            }
            vec![n]
        }
        InlineKind::Span {
            color,
            font_size,
            font_name,
            small_caps,
        } => {
            let mut n = Node::new(node::SPAN);
            if let Some(c) = color {
                n = n.prop("style:color", c);
            }
            if let Some(s) = font_size {
                n = n.prop("style:size", s);
            }
            if let Some(f) = font_name {
                n = n.prop("style:font", f);
            }
            if small_caps {
                n = n.prop("style:variant", "small-caps");
            }
            for c in children {
                n = n.child(c);
            }
            vec![n]
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn extract_text_from_children(nodes: &[Node]) -> String {
    nodes
        .iter()
        .map(extract_text_node)
        .collect::<Vec<_>>()
        .join("")
}

fn extract_text_node(n: &Node) -> String {
    if n.kind.as_str() == node::TEXT {
        n.props.get_str(prop::CONTENT).unwrap_or("").to_owned()
    } else if n.kind.as_str() == node::LINE_BREAK {
        "\n".to_owned()
    } else {
        extract_text_from_children(&n.children)
    }
}

fn is_ordered_list(style_name: Option<&str>, ctx: &StyleCtx<'_>) -> bool {
    let name = match style_name {
        Some(n) if !n.is_empty() => n,
        _ => return false,
    };
    // Check parsed list style info first
    if let Some(ordered) = ctx.is_ordered_list_style(name) {
        return ordered;
    }
    // Fall through to heuristic on style name
    let lower = name.to_lowercase();
    lower.contains("numb")
        || lower.contains("order")
        || lower.contains("decimal")
        || lower == "list number"
        || lower == "list_number"
}

fn apply_para_props_from_style(mut n: Node, style_name: &str, ctx: &StyleCtx<'_>) -> Node {
    if let Some(entry) = ctx.find_style(style_name) {
        let p = &entry.para_props;
        if let Some(v) = &p.align {
            n = n.prop("style:align", v.as_str());
        }
        if let Some(v) = &p.margin_left {
            n = n.prop("style:margin-left", v.as_str());
        }
        if let Some(v) = &p.margin_right {
            n = n.prop("style:margin-right", v.as_str());
        }
        if let Some(v) = &p.margin_top {
            n = n.prop("style:margin-top", v.as_str());
        }
        if let Some(v) = &p.margin_bottom {
            n = n.prop("style:margin-bottom", v.as_str());
        }
        if let Some(v) = &p.text_indent {
            n = n.prop("style:text-indent", v.as_str());
        }
        if let Some(v) = &p.line_height {
            n = n.prop("style:line-height", v.as_str());
        }
        if let Some(v) = &p.border {
            n = n.prop("style:border", v.as_str());
        }
        if let Some(v) = &p.background_color {
            n = n.prop("style:background", v.as_str());
        }
        if p.keep_together {
            n = n.prop("style:keep-together", "always");
        }
        if p.keep_with_next {
            n = n.prop("style:keep-with-next", "always");
        }
    }
    n
}

fn flush_pending_blockquote(pending: &mut Option<Vec<Node>>, doc: &mut Node) {
    if let Some(paras) = pending.take()
        && !paras.is_empty()
    {
        let mut bq = Node::new(node::BLOCKQUOTE);
        for p in paras {
            bq = bq.child(p);
        }
        *doc = doc.clone().child(bq);
    }
}

fn mime_from_name(name: &str) -> &'static str {
    let lower = name.to_lowercase();
    if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".gif") {
        "image/gif"
    } else if lower.ends_with(".svg") {
        "image/svg+xml"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else if lower.ends_with(".tiff") || lower.ends_with(".tif") {
        "image/tiff"
    } else if lower.ends_with(".bmp") {
        "image/bmp"
    } else {
        "application/octet-stream"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_odt_bytes(content_xml: &str) -> Vec<u8> {
        use std::io::{Cursor, Write};
        use zip::ZipWriter;
        use zip::write::SimpleFileOptions;

        let mut buf = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut buf);
            let options = SimpleFileOptions::default();
            zip.start_file("mimetype", options).unwrap();
            zip.write_all(b"application/vnd.oasis.opendocument.text")
                .unwrap();
            zip.start_file("content.xml", options).unwrap();
            zip.write_all(content_xml.as_bytes()).unwrap();
            zip.finish().unwrap();
        }
        buf.into_inner()
    }

    fn ns() -> &'static str {
        r#"xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
           xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
           xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
           xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0"
           xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0"
           xmlns:xlink="http://www.w3.org/1999/xlink""#
    }

    fn body(content: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content {ns}>
  <office:body>
    <office:text>
      {content}
    </office:text>
  </office:body>
</office:document-content>"#,
            ns = ns(),
            content = content
        )
    }

    fn body_with_styles(auto_styles: &str, content: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content {ns}>
  <office:automatic-styles>
    {auto_styles}
  </office:automatic-styles>
  <office:body>
    <office:text>
      {content}
    </office:text>
  </office:body>
</office:document-content>"#,
            ns = ns(),
            auto_styles = auto_styles,
            content = content
        )
    }

    #[test]
    fn test_parse_basic() {
        let odt = make_odt_bytes(&body("<text:p>Hello world</text:p>"));
        let result = parse(&odt).unwrap();
        assert!(!result.value.content.children.is_empty());
    }

    #[test]
    fn test_parse_heading() {
        let odt = make_odt_bytes(&body(r#"<text:h text:outline-level="1">Title</text:h>"#));
        let result = parse(&odt).unwrap();
        let heading = &result.value.content.children[0];
        assert_eq!(heading.kind.as_str(), node::HEADING);
        assert_eq!(heading.props.get_int(prop::LEVEL), Some(1));
    }

    #[test]
    fn test_parse_bold_named_style() {
        let xml = body(
            r#"<text:p>Some <text:span text:style-name="Bold">bold</text:span> text.</text:p>"#,
        );
        let odt = make_odt_bytes(&xml);
        let result = parse(&odt).unwrap();
        let para = &result.value.content.children[0];
        assert_eq!(para.kind.as_str(), node::PARAGRAPH);
        let strong = para
            .children
            .iter()
            .find(|c| c.kind.as_str() == node::STRONG);
        assert!(strong.is_some(), "should have a strong node");
    }

    #[test]
    fn test_parse_bold_auto_style() {
        let auto = r#"<style:style style:name="T1" style:family="text">
            <style:text-properties fo:font-weight="bold"/>
        </style:style>"#;
        let xml = body_with_styles(
            auto,
            r#"<text:p>Some <text:span text:style-name="T1">bold</text:span> text.</text:p>"#,
        );
        let odt = make_odt_bytes(&xml);
        let result = parse(&odt).unwrap();
        let para = &result.value.content.children[0];
        let strong = para
            .children
            .iter()
            .find(|c| c.kind.as_str() == node::STRONG);
        assert!(
            strong.is_some(),
            "auto-style T1 with fo:font-weight=bold should produce strong node"
        );
    }

    #[test]
    fn test_parse_italic() {
        let xml =
            body(r#"<text:p><text:span text:style-name="Italic">italic</text:span></text:p>"#);
        let odt = make_odt_bytes(&xml);
        let result = parse(&odt).unwrap();
        let para = &result.value.content.children[0];
        let em = para
            .children
            .iter()
            .find(|c| c.kind.as_str() == node::EMPHASIS);
        assert!(em.is_some(), "should have an emphasis node");
    }

    #[test]
    fn test_parse_hyperlink() {
        let xml = body(
            r#"<text:p><text:a xlink:type="simple" xlink:href="https://example.com">link text</text:a></text:p>"#,
        );
        let odt = make_odt_bytes(&xml);
        let result = parse(&odt).unwrap();
        let para = &result.value.content.children[0];
        let link = para.children.iter().find(|c| c.kind.as_str() == node::LINK);
        let Some(link) = link else {
            panic!("should have a link node");
        };
        assert_eq!(link.props.get_str(prop::URL), Some("https://example.com"));
    }

    #[test]
    fn test_parse_table() {
        let xml = body(
            r#"
        <table:table>
          <table:table-row>
            <table:table-cell><text:p>Cell 1</text:p></table:table-cell>
            <table:table-cell><text:p>Cell 2</text:p></table:table-cell>
          </table:table-row>
        </table:table>"#,
        );
        let odt = make_odt_bytes(&xml);
        let result = parse(&odt).unwrap();
        let table = &result.value.content.children[0];
        assert_eq!(table.kind.as_str(), node::TABLE);
        assert_eq!(table.children.len(), 1);
        let row = &table.children[0];
        assert_eq!(row.kind.as_str(), node::TABLE_ROW);
        assert_eq!(row.children.len(), 2);
    }

    #[test]
    fn test_parse_ordered_list() {
        let auto = r#"<text:list-style style:name="L1">
            <text:list-level-style-number text:level="1" style:num-format="1"/>
        </text:list-style>"#;
        let xml = body_with_styles(
            auto,
            r#"
        <text:list text:style-name="L1">
          <text:list-item><text:p>one</text:p></text:list-item>
          <text:list-item><text:p>two</text:p></text:list-item>
        </text:list>"#,
        );
        let odt = make_odt_bytes(&xml);
        let result = parse(&odt).unwrap();
        let list = &result.value.content.children[0];
        assert_eq!(list.kind.as_str(), node::LIST);
        // Ordered detection via style name: "L1" is not a recognized ordered pattern,
        // so this tests the list structure at minimum.
        assert_eq!(list.children.len(), 2);
    }

    #[test]
    fn settings_and_rdf_parts_exposed_as_resources_and_round_trip() {
        use std::io::{Cursor, Write};
        use zip::ZipWriter;
        use zip::write::SimpleFileOptions;

        let xml = body("<text:p>Hello</text:p>");
        let mut buf = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut buf);
            let options = SimpleFileOptions::default();
            zip.start_file("mimetype", options).unwrap();
            zip.write_all(b"application/vnd.oasis.opendocument.text")
                .unwrap();
            zip.start_file("content.xml", options).unwrap();
            zip.write_all(xml.as_bytes()).unwrap();
            zip.start_file("settings.xml", options).unwrap();
            zip.write_all(b"<?xml version=\"1.0\"?><office:document-settings/>")
                .unwrap();
            zip.start_file("META-INF/manifest.rdf", options).unwrap();
            zip.write_all(b"<?xml version=\"1.0\"?><rdf:RDF/>").unwrap();
            zip.finish().unwrap();
        }
        let odt = buf.into_inner();

        let result = parse(&odt).unwrap();
        let has_settings = result
            .value
            .resources
            .values()
            .any(|r| r.name.as_deref() == Some("settings.xml"));
        let has_rdf = result.value.resources.values().any(|r| {
            r.name.as_deref() == Some("META-INF/manifest.rdf")
                && r.mime_type == "application/rdf+xml"
        });
        assert!(has_settings, "settings.xml not exposed as a resource");
        assert!(has_rdf, "manifest.rdf not exposed as a resource");

        // Round-trip through the rescribe writer.
        let bytes = crate::rescribe::write::emit(&result.value).unwrap().value;
        let reparsed = crate::parser::parse(&bytes).unwrap().value;
        assert!(reparsed.extra_parts.contains_key("settings.xml"));
        assert!(reparsed.extra_parts.contains_key("META-INF/manifest.rdf"));
    }

    // ── Spreadsheet / presentation translation (ADR 0015) ───────────────────

    #[test]
    fn parse_spreadsheet_sheet_row_cell() {
        let ods = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../fixtures/odf/ods-body/input.ods"
        ))
        .unwrap();
        let r = parse(&ods).unwrap();
        assert!(r.warnings.is_empty());

        let sheet = &r.value.content.children[0];
        assert_eq!(sheet.kind.as_str(), node::SHEET);
        assert_eq!(sheet.props.get_str("odf:name"), Some("Sales"));

        let header_row = &sheet.children[0];
        assert_eq!(header_row.kind.as_str(), node::SHEET_ROW);
        let product_cell = &header_row.children[0];
        assert_eq!(product_cell.kind.as_str(), node::SHEET_CELL);
        assert_eq!(product_cell.props.get_str(prop::VALUE_TYPE), Some("string"));
        assert_eq!(product_cell.props.get_str(prop::VALUE), Some("Product"));

        let data_row = &sheet.children[1];
        let revenue_cell = &data_row.children[1];
        assert_eq!(revenue_cell.props.get_str(prop::VALUE_TYPE), Some("number"));
        assert_eq!(revenue_cell.props.get_str(prop::VALUE), Some("1500"));
    }

    #[test]
    fn parse_presentation_slide_positioned_shapes() {
        let odp = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../fixtures/odf/odp-body/input.odp"
        ))
        .unwrap();
        let r = parse(&odp).unwrap();
        assert!(r.warnings.is_empty());

        let page = &r.value.content.children[0];
        assert_eq!(page.kind.as_str(), node::DIV);
        assert_eq!(page.props.get_str("odf:type"), Some("slide"));
        assert_eq!(page.props.get_str("odf:name"), Some("slide1"));
        assert_eq!(page.props.get_str("odf:master-page-name"), Some("Default"));

        let title = &page.children[0];
        assert_eq!(title.kind.as_str(), node::POSITIONED_CONTAINER);
        assert_eq!(title.props.get_str("odf:presentation-class"), Some("title"));
        // svg:x="5.01cm" -> EMU, via ADR 0015's exact cm ratio (360,000 EMU/cm).
        assert_eq!(title.props.get_int(prop::POSITION_X), Some(1_803_600));
        assert_eq!(title.props.get_str("odf:x"), Some("5.01cm"));
        assert_eq!(title.props.get_int(prop::POSITION_Z_ORDER), Some(0));

        let subtitle = &page.children[1];
        assert_eq!(
            subtitle.props.get_str("odf:presentation-class"),
            Some("subtitle")
        );
        assert_eq!(subtitle.props.get_int(prop::POSITION_Z_ORDER), Some(1));
    }

    #[test]
    fn pure_rotate_transform_projects_to_position_rotation() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"
  xmlns:presentation="urn:oasis:names:tc:opendocument:xmlns:presentation:1.0"
  xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0">
  <office:body>
    <office:presentation>
      <draw:page draw:name="slide1">
        <draw:frame svg:x="0cm" svg:y="0cm" svg:width="1cm" svg:height="1cm"
          draw:transform="rotate(90)">
        </draw:frame>
      </draw:page>
    </office:presentation>
  </office:body>
</office:document-content>"#;
        let odp = make_odp_bytes(xml);
        let r = parse(&odp).unwrap();
        assert!(r.warnings.is_empty());
        let page = &r.value.content.children[0];
        let shape = &page.children[0];
        assert_eq!(shape.props.get_str("odf:transform"), Some("rotate(90)"));
        // ODF's angle datatype is degrees by default (ADR 0015 Decision 5):
        // 90 degrees == 90 * 60_000 = 5_400_000 OOXML units.
        assert_eq!(
            shape.props.get_int(prop::POSITION_ROTATION),
            Some(5_400_000)
        );
    }

    #[test]
    fn rotate_rad_suffix_converts_to_degrees_first() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"
  xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0">
  <office:body>
    <office:presentation>
      <draw:page draw:name="slide1">
        <draw:frame svg:x="0cm" svg:y="0cm" svg:width="1cm" svg:height="1cm"
          draw:transform="rotate(1.5707963267948966rad)">
        </draw:frame>
      </draw:page>
    </office:presentation>
  </office:body>
</office:document-content>"#;
        let odp = make_odp_bytes(xml);
        let r = parse(&odp).unwrap();
        assert!(r.warnings.is_empty());
        let page = &r.value.content.children[0];
        let shape = &page.children[0];
        // pi/2 rad == 90 deg == 5_400_000 OOXML units.
        assert_eq!(
            shape.props.get_int(prop::POSITION_ROTATION),
            Some(5_400_000)
        );
    }

    #[test]
    fn combined_transform_is_preserved_raw_without_rotation_projection() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"
  xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0">
  <office:body>
    <office:presentation>
      <draw:page draw:name="slide1">
        <draw:frame svg:x="0cm" svg:y="0cm" svg:width="1cm" svg:height="1cm"
          draw:transform="translate(1cm 2cm) rotate(0.5)">
        </draw:frame>
      </draw:page>
    </office:presentation>
  </office:body>
</office:document-content>"#;
        let odp = make_odp_bytes(xml);
        let r = parse(&odp).unwrap();
        assert_eq!(r.warnings.len(), 1);
        let page = &r.value.content.children[0];
        let shape = &page.children[0];
        assert_eq!(
            shape.props.get_str("odf:transform"),
            Some("translate(1cm 2cm) rotate(0.5)")
        );
        assert_eq!(shape.props.get_int(prop::POSITION_ROTATION), None);
    }

    fn make_odp_bytes(content_xml: &str) -> Vec<u8> {
        use std::io::{Cursor, Write};
        use zip::ZipWriter;
        use zip::write::SimpleFileOptions;

        let mut buf = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut buf);
            let options = SimpleFileOptions::default();
            zip.start_file("mimetype", options).unwrap();
            zip.write_all(b"application/vnd.oasis.opendocument.presentation")
                .unwrap();
            zip.start_file("content.xml", options).unwrap();
            zip.write_all(content_xml.as_bytes()).unwrap();
            zip.finish().unwrap();
        }
        buf.into_inner()
    }
}
