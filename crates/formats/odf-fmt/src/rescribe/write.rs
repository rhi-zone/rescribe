//! ODT (OpenDocument Text) writer for rescribe.
//!
//! Generates ODF/ODT documents from rescribe's document IR by delegating to
//! the rest of this crate for all ZIP building and XML serialisation.

use crate::ast::{
    DrawPage, DrawShape, DrawShapeContent, Frame, FrameChild, Heading, Hyperlink, Inline, List,
    ListItem, NotesPage, OdfBody, OdfDocument, OdfMeta, Paragraph, ParagraphProperties,
    PresentationBody, Sheet, SheetCell, SheetRow, Span, SpreadsheetBody, StyleEntry, Table,
    TableCell, TableRow, TextBlock, TextProperties,
};
use rescribe_core::{
    ConversionResult, Document, EmitError, EmitOptions, FidelityWarning, Node, ResourceMap,
    Severity, WarningKind,
};
use rescribe_std::{node, prop};
use std::collections::HashMap;

/// Emit a document to ODT.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    emit_with_options(doc, &EmitOptions::default())
}

/// Emit a document to ODT with options.
pub fn emit_with_options(
    doc: &Document,
    _options: &EmitOptions,
) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let (odf, warnings) = convert_document(doc);
    let bytes =
        crate::emit(&odf).map_err(|e| EmitError::Io(std::io::Error::other(e.to_string())))?;
    Ok(ConversionResult::with_warnings(bytes, warnings))
}

// ── Document conversion ───────────────────────────────────────────────────────

/// Per-document mutable state threaded through block/inline conversion:
/// resolves `image` nodes' `src` prop (a `ResourceId`) against the source
/// document's resources and collects the embedded bytes under the ZIP paths
/// the writer will emit them at.
struct WriteCtx<'a> {
    resources: &'a ResourceMap,
    images: HashMap<String, Vec<u8>>,
    next_image: usize,
    next_chart: usize,
}

impl<'a> WriteCtx<'a> {
    fn new(resources: &'a ResourceMap) -> Self {
        Self {
            resources,
            images: HashMap::new(),
            next_image: 0,
            next_chart: 0,
        }
    }

    /// Resolve an `image` node's `src` prop to a `Pictures/…` ZIP path,
    /// embedding the resource bytes the first time each id is seen.
    fn resolve_image(&mut self, src: &str) -> Option<(String, Option<String>)> {
        let id = rescribe_core::ResourceId::from_string(src.to_owned());
        let resource = self.resources.get(&id)?;
        let ext = extension_for_mime(&resource.mime_type);
        self.next_image += 1;
        let path = format!("Pictures/image{}{ext}", self.next_image);
        self.images.insert(path.clone(), resource.data.clone());
        Some((path, Some(resource.mime_type.clone())))
    }

    /// Allocate an embedded-object directory name for a chart (ADR 0016),
    /// e.g. `"Object 1"` (no `./` prefix, no trailing slash). The chart's
    /// `content.xml` bytes themselves are built by `writer::emit` (which
    /// walks the final `OdfDocument` AST directly), not staged here — a
    /// chart embedded via a directly-constructed AST (bypassing this
    /// `rescribe` module entirely) must round-trip identically, so that
    /// content-building logic lives in the crate's core writer, not this
    /// IR-adapter layer.
    fn resolve_chart(&mut self) -> String {
        self.next_chart += 1;
        format!("Object {}", self.next_chart)
    }
}

fn extension_for_mime(mime: &str) -> &'static str {
    match mime {
        "image/jpeg" | "image/jpg" => ".jpg",
        "image/gif" => ".gif",
        "image/svg+xml" => ".svg",
        "image/bmp" => ".bmp",
        "image/tiff" => ".tiff",
        _ => ".png",
    }
}

/// Which of ODF's three body shapes a `Document` maps to. Detected from the
/// top-level content nodes rather than carried as an explicit field, since
/// the IR has no document-type marker — mirrors how the reader recognizes
/// `office:text`/`office:spreadsheet`/`office:presentation` from the body's
/// own element name.
enum BodyKind {
    Text,
    Spreadsheet,
    Presentation,
}

fn detect_body_kind(content: &Node) -> BodyKind {
    match content.children.first() {
        Some(n) if n.kind.as_str() == node::SHEET => BodyKind::Spreadsheet,
        Some(n) if n.kind.as_str() == node::DIV && n.props.get_str("odf:type") == Some("slide") => {
            BodyKind::Presentation
        }
        _ => BodyKind::Text,
    }
}

fn build_meta(doc: &Document) -> OdfMeta {
    OdfMeta {
        title: doc.metadata.get_str("title").map(str::to_owned),
        creator: doc.metadata.get_str("author").map(str::to_owned),
        ..OdfMeta::default()
    }
}

/// Round-trip any raw-preserved package parts (settings.xml, RDF metadata)
/// that `rescribe::read` exposed as resources — see its comment for why
/// these have no IR node representation.
fn build_extra_parts(doc: &Document) -> HashMap<String, Vec<u8>> {
    let mut extra_parts = HashMap::new();
    for resource in doc.resources.values() {
        let Some(name) = &resource.name else { continue };
        if resource.mime_type == "application/rdf+xml" || name == "settings.xml" {
            extra_parts.insert(name.clone(), resource.data.clone());
        }
    }
    extra_parts
}

fn convert_document(doc: &Document) -> (OdfDocument, Vec<FidelityWarning>) {
    let mut ctx = WriteCtx::new(&doc.resources);
    let mut warnings = Vec::new();

    let (mimetype, body) = match detect_body_kind(&doc.content) {
        BodyKind::Spreadsheet => (
            "application/vnd.oasis.opendocument.spreadsheet",
            OdfBody::Spreadsheet(convert_spreadsheet_document(
                &doc.content,
                &mut ctx,
                &mut warnings,
            )),
        ),
        BodyKind::Presentation => (
            "application/vnd.oasis.opendocument.presentation",
            OdfBody::Presentation(convert_presentation_document(
                &doc.content,
                &mut ctx,
                &mut warnings,
            )),
        ),
        BodyKind::Text => (
            "application/vnd.oasis.opendocument.text",
            OdfBody::Text(convert_nodes(&doc.content.children, &mut ctx)),
        ),
    };

    let odf = OdfDocument {
        mimetype: mimetype.to_owned(),
        meta: build_meta(doc),
        named_styles: build_named_styles(),
        body,
        images: ctx.images,
        extra_parts: build_extra_parts(doc),
        ..OdfDocument::default()
    };
    (odf, warnings)
}

// ── Spreadsheet document conversion (ADR 0015) ─────────────────────────────────

fn convert_spreadsheet_document(
    content: &Node,
    ctx: &mut WriteCtx<'_>,
    warnings: &mut Vec<FidelityWarning>,
) -> SpreadsheetBody {
    let mut sheets = Vec::new();
    for sheet_node in &content.children {
        if sheet_node.kind.as_str() != node::SHEET {
            continue;
        }
        let mut rows = Vec::new();
        let mut shapes = Vec::new();
        for (i, child) in sheet_node.children.iter().enumerate() {
            if child.kind.as_str() == node::SHEET_ROW {
                let cells = child
                    .children
                    .iter()
                    .filter(|c| c.kind.as_str() == node::SHEET_CELL)
                    .map(|c| convert_sheet_cell_node(c, ctx))
                    .collect();
                rows.push(SheetRow {
                    style_name: child.props.get_str("odf:style-name").map(str::to_owned),
                    default_cell_style_name: None,
                    repeated: child.props.get_int("odf:repeated").map(|v| v as u32),
                    cells,
                });
            } else if child.kind.as_str() == node::POSITIONED_CONTAINER {
                // Floating shapes anchored to the sheet (`<table:shapes>`),
                // e.g. an embedded chart (ADR 0016) — siblings of
                // `sheet_row` in the IR, same `positioned_container` shape
                // ODP uses.
                shapes.push(convert_positioned_container(child, ctx, i as i64, warnings));
            }
        }
        sheets.push(Sheet {
            name: sheet_node.props.get_str("odf:name").map(str::to_owned),
            style_name: sheet_node
                .props
                .get_str("odf:style-name")
                .map(str::to_owned),
            print: sheet_node.props.get_bool("odf:print").unwrap_or(false),
            columns: Vec::new(),
            rows,
            shapes,
        });
    }
    SpreadsheetBody {
        sheets,
        named_ranges: Vec::new(),
    }
}

fn convert_sheet_cell_node(n: &Node, ctx: &mut WriteCtx<'_>) -> SheetCell {
    SheetCell {
        style_name: n.props.get_str("odf:style-name").map(str::to_owned),
        value_type: n
            .props
            .get_str(prop::VALUE_TYPE)
            .map(map_ir_value_type_to_odf)
            .map(str::to_owned),
        value: n.props.get_str(prop::VALUE).map(str::to_owned),
        formula: n.props.get_str(prop::VALUE_FORMULA).map(str::to_owned),
        col_span: n.props.get_int(prop::COLSPAN).map(|v| v as u32),
        row_span: n.props.get_int(prop::ROWSPAN).map(|v| v as u32),
        repeated: n.props.get_int("odf:repeated").map(|v| v as u32),
        covered: n.props.get_bool("odf:covered").unwrap_or(false),
        content: convert_nodes(&n.children, ctx),
    }
}

/// Inverse of `rescribe::read::map_odf_value_type` (ADR 0015 Decision 2).
fn map_ir_value_type_to_odf(ir_type: &str) -> &'static str {
    match ir_type {
        "number" => "float",
        "percentage" => "percentage",
        "currency" => "currency",
        "date" => "date",
        "time" => "time",
        "boolean" => "boolean",
        // A formula's computed-result type with no more specific IR type
        // recorded: "float" is ODF's default value-type for formula cells.
        "formula-result" => "float",
        _ => "string",
    }
}

// ── Presentation document conversion (ADR 0015) ────────────────────────────────

fn convert_presentation_document(
    content: &Node,
    ctx: &mut WriteCtx<'_>,
    warnings: &mut Vec<FidelityWarning>,
) -> PresentationBody {
    let mut pages = Vec::new();
    for page_node in &content.children {
        if page_node.kind.as_str() != node::DIV
            || page_node.props.get_str("odf:type") != Some("slide")
        {
            continue;
        }
        let mut shapes = Vec::new();
        let mut notes = None;
        for (i, child) in page_node.children.iter().enumerate() {
            if child.kind.as_str() == node::DIV && child.props.get_str("odf:type") == Some("notes")
            {
                let notes_shapes = child
                    .children
                    .iter()
                    .enumerate()
                    .map(|(j, nchild)| {
                        convert_positioned_container(nchild, ctx, j as i64, warnings)
                    })
                    .collect();
                notes = Some(Box::new(NotesPage {
                    style_name: child.props.get_str("odf:style-name").map(str::to_owned),
                    shapes: notes_shapes,
                }));
            } else if child.kind.as_str() == node::POSITIONED_CONTAINER {
                shapes.push(convert_positioned_container(child, ctx, i as i64, warnings));
            }
        }
        pages.push(DrawPage {
            name: page_node.props.get_str("odf:name").map(str::to_owned),
            style_name: page_node.props.get_str("odf:style-name").map(str::to_owned),
            master_page_name: page_node
                .props
                .get_str("odf:master-page-name")
                .map(str::to_owned),
            layout_name: page_node
                .props
                .get_str("odf:layout-name")
                .map(str::to_owned),
            shapes,
            notes,
        });
    }
    PresentationBody { pages }
}

fn convert_positioned_container(
    n: &Node,
    ctx: &mut WriteCtx<'_>,
    doc_order_index: i64,
    warnings: &mut Vec<FidelityWarning>,
) -> DrawShape {
    let x = resolve_length(n, "odf:x", prop::POSITION_X);
    let y = resolve_length(n, "odf:y", prop::POSITION_Y);
    let width = resolve_length(n, "odf:width", prop::POSITION_WIDTH);
    let height = resolve_length(n, "odf:height", prop::POSITION_HEIGHT);

    let transform = if let Some(raw) = n.props.get_str("odf:transform") {
        Some(raw.to_owned())
    } else if let Some(units) = n.props.get_int(prop::POSITION_ROTATION) {
        warnings.push(FidelityWarning::new(
            Severity::Minor,
            WarningKind::Simplified("position:rotation".to_owned()),
            "position:rotation synthesized as an ODF draw:transform rotate(); ODF's rotation \
             pivot may not match the pivot implied by the value's originating format, since \
             this Document carried no raw odf:transform to preserve verbatim (ADR 0015)",
        ));
        // ODF's angle datatype is degrees by default (ADR 0015 Decision 5),
        // so no unit suffix is needed here — unlike a radians-first grammar.
        let degrees = units as f64 / 60_000.0;
        Some(format!("rotate({degrees})"))
    } else {
        None
    };

    let z_index = n.props.get_int(prop::POSITION_Z_ORDER);

    let content = if n.children.len() == 1 && n.children[0].kind.as_str() == node::IMAGE {
        let img = &n.children[0];
        let (href, mime_type) = img
            .props
            .get_str("src")
            .and_then(|src| ctx.resolve_image(src))
            .unwrap_or_else(|| {
                (
                    img.props.get_str("src").unwrap_or_default().to_owned(),
                    img.props.get_str("odf:mime-type").map(str::to_owned),
                )
            });
        DrawShapeContent::Image { href, mime_type }
    } else if n.children.len() == 1
        && n.children[0].kind.as_str() == node::RAW_BLOCK
        && n.children[0].props.get_str(prop::FORMAT) == Some("odf")
    {
        DrawShapeContent::Other(
            n.children[0]
                .props
                .get_str(prop::CONTENT)
                .unwrap_or_default()
                .to_owned(),
        )
    } else if n.children.len() == 1 && n.children[0].kind.as_str() == node::CHART {
        let chart_ast = convert_chart_node_to_ast(&n.children[0]);
        let href = ctx.resolve_chart();
        DrawShapeContent::Chart {
            href,
            chart: chart_ast,
        }
    } else if n.children.is_empty() {
        DrawShapeContent::Empty
    } else {
        DrawShapeContent::TextBox(convert_nodes(&n.children, ctx))
    };

    DrawShape {
        style_name: n.props.get_str("odf:style-name").map(str::to_owned),
        text_style_name: n.props.get_str("odf:text-style-name").map(str::to_owned),
        name: n.props.get_str("odf:name").map(str::to_owned),
        presentation_class: n.props.get_str("odf:presentation-class").map(str::to_owned),
        x,
        y,
        width,
        height,
        transform,
        z_index: z_index.or(Some(doc_order_index)),
        content,
    }
}

/// Prefer the raw ODF coordinate string (byte-exact preservation) over
/// synthesizing one from the EMU projection (ADR 0015 Decision 4).
/// Synthesized lengths use `cm` as an arbitrary-but-fixed unit.
fn resolve_length(n: &Node, raw_key: &str, emu_key: &str) -> Option<String> {
    if let Some(raw) = n.props.get_str(raw_key) {
        return Some(raw.to_owned());
    }
    n.props
        .get_int(emu_key)
        .map(|emu| format!("{}cm", emu as f64 / 360_000.0))
}

// ── Chart conversion (ADR 0016) ─────────────────────────────────────────────

/// Convert a `chart` IR node back into `ast::Chart`.
fn convert_chart_node_to_ast(n: &Node) -> crate::ast::Chart {
    let series = n
        .children
        .iter()
        .filter(|c| c.kind.as_str() == node::CHART_SERIES)
        .map(|c| crate::ast::ChartSeries {
            series_class: None,
            values_cell_range_address: c.props.get_str(prop::CHART_VALUES_REF).map(str::to_owned),
            categories_cell_range_address: c
                .props
                .get_str(prop::CHART_CATEGORIES_REF)
                .map(str::to_owned),
        })
        .collect();
    crate::ast::Chart {
        chart_class: n.props.get_str(prop::CHART_TYPE).map(str::to_owned),
        title: n.props.get_str(prop::TITLE).map(str::to_owned),
        legend: n.props.get_bool(prop::CHART_LEGEND).unwrap_or(false),
        legend_position: n
            .props
            .get_str(prop::CHART_LEGEND_POSITION)
            .map(str::to_owned),
        has_category_axis: n
            .props
            .get_bool(prop::CHART_HAS_CATEGORY_AXIS)
            .unwrap_or(false),
        has_value_axis: n
            .props
            .get_bool(prop::CHART_HAS_VALUE_AXIS)
            .unwrap_or(false),
        series,
        // Present unconditionally on every ODF-sourced chart (ADR 0016
        // Decision 4); absent for a chart constructed synthetically (e.g.
        // projected from another format's IR) — `build_chart_content_xml`
        // synthesizes a minimal chart body in that case.
        raw_xml: n
            .props
            .get_str(prop::ODF_CHART_XML)
            .unwrap_or("")
            .to_owned(),
    }
}

/// Build a minimal set of named styles for the IR constructs the writer emits.
fn build_named_styles() -> Vec<StyleEntry> {
    fn text_entry(name: &str, family: &str, props: TextProperties) -> StyleEntry {
        StyleEntry {
            name: name.to_owned(),
            family: Some(family.to_owned()),
            text_props: props,
            ..StyleEntry::default()
        }
    }
    fn para_entry(name: &str, props: ParagraphProperties) -> StyleEntry {
        StyleEntry {
            name: name.to_owned(),
            family: Some("paragraph".to_owned()),
            para_props: props,
            ..StyleEntry::default()
        }
    }

    vec![
        text_entry(
            "Bold",
            "text",
            TextProperties {
                bold: true,
                ..TextProperties::default()
            },
        ),
        text_entry(
            "Italic",
            "text",
            TextProperties {
                italic: true,
                ..TextProperties::default()
            },
        ),
        text_entry(
            "Underline",
            "text",
            TextProperties {
                underline: true,
                ..TextProperties::default()
            },
        ),
        text_entry(
            "Strikethrough",
            "text",
            TextProperties {
                strikethrough: true,
                ..TextProperties::default()
            },
        ),
        text_entry(
            "Code",
            "text",
            TextProperties {
                font_name: Some("Courier New".to_owned()),
                ..TextProperties::default()
            },
        ),
        text_entry(
            "Subscript",
            "text",
            TextProperties {
                subscript: true,
                ..TextProperties::default()
            },
        ),
        text_entry(
            "Superscript",
            "text",
            TextProperties {
                superscript: true,
                ..TextProperties::default()
            },
        ),
        para_entry("Preformatted", ParagraphProperties::default()),
        para_entry(
            "Quotation",
            ParagraphProperties {
                margin_left: Some("0.5in".to_owned()),
                ..ParagraphProperties::default()
            },
        ),
    ]
}

// ── Block node conversion ─────────────────────────────────────────────────────

fn convert_nodes(nodes: &[Node], ctx: &mut WriteCtx<'_>) -> Vec<TextBlock> {
    let mut blocks = Vec::new();
    let mut blockquote_buf: Vec<TextBlock> = Vec::new();

    for n in nodes {
        match n.kind.as_str() {
            node::DOCUMENT => {
                let inner = convert_nodes(&n.children, ctx);
                flush_blockquote(&mut blockquote_buf, &mut blocks);
                blocks.extend(inner);
            }

            node::PARAGRAPH => {
                flush_blockquote(&mut blockquote_buf, &mut blocks);
                let style = n.props.get_str("odt:style-name").map(str::to_owned);
                let content = collect_inlines(&n.children, ctx);
                blocks.push(TextBlock::Paragraph(Paragraph {
                    style_name: style,
                    content,
                    ..Paragraph::default()
                }));
            }

            node::HEADING => {
                flush_blockquote(&mut blockquote_buf, &mut blocks);
                let level = n.props.get_int(prop::LEVEL).unwrap_or(1) as u32;
                let content = collect_inlines(&n.children, ctx);
                blocks.push(TextBlock::Heading(Heading {
                    outline_level: Some(level.min(6)),
                    content,
                    ..Heading::default()
                }));
            }

            node::CODE_BLOCK => {
                flush_blockquote(&mut blockquote_buf, &mut blocks);
                let text = n.props.get_str(prop::CONTENT).unwrap_or("").to_owned();
                // Emit as a preformatted paragraph with line breaks for embedded newlines
                let mut inlines: Vec<Inline> = Vec::new();
                for (i, line) in text.lines().enumerate() {
                    if i > 0 {
                        inlines.push(Inline::LineBreak);
                    }
                    if !line.is_empty() {
                        inlines.push(Inline::Text(line.to_owned()));
                    }
                }
                blocks.push(TextBlock::Paragraph(Paragraph {
                    style_name: Some("Preformatted".to_owned()),
                    content: inlines,
                    ..Paragraph::default()
                }));
            }

            node::BLOCKQUOTE => {
                flush_blockquote(&mut blockquote_buf, &mut blocks);
                for child in &n.children {
                    let content = collect_inlines(&child.children, ctx);
                    blockquote_buf.push(TextBlock::Paragraph(Paragraph {
                        style_name: Some("Quotation".to_owned()),
                        content,
                        ..Paragraph::default()
                    }));
                }
                flush_blockquote(&mut blockquote_buf, &mut blocks);
            }

            node::LIST => {
                flush_blockquote(&mut blockquote_buf, &mut blocks);
                let ordered = n.props.get_bool("ordered").unwrap_or(false);
                let style_name = if ordered {
                    Some("List Number".to_owned())
                } else {
                    Some("List Bullet".to_owned())
                };
                let items = n
                    .children
                    .iter()
                    .filter(|c| c.kind.as_str() == node::LIST_ITEM)
                    .map(|c| {
                        let content = convert_nodes(&c.children, ctx);
                        ListItem {
                            content,
                            ..ListItem::default()
                        }
                    })
                    .collect();
                blocks.push(TextBlock::List(List {
                    style_name,
                    items,
                    ..List::default()
                }));
            }

            node::TABLE => {
                flush_blockquote(&mut blockquote_buf, &mut blocks);
                let rows = n
                    .children
                    .iter()
                    .filter(|r| r.kind.as_str() == node::TABLE_ROW)
                    .map(|r| {
                        let cells = r
                            .children
                            .iter()
                            .map(|c| {
                                let colspan = c.props.get_int(prop::COLSPAN).map(|v| v as u32);
                                let rowspan = c.props.get_int(prop::ROWSPAN).map(|v| v as u32);
                                let content = convert_nodes(&c.children, ctx);
                                TableCell {
                                    col_span: colspan,
                                    row_span: rowspan,
                                    content,
                                    ..TableCell::default()
                                }
                            })
                            .collect();
                        TableRow {
                            cells,
                            ..TableRow::default()
                        }
                    })
                    .collect();
                blocks.push(TextBlock::Table(Table {
                    rows,
                    ..Table::default()
                }));
            }

            node::HORIZONTAL_RULE => {
                flush_blockquote(&mut blockquote_buf, &mut blocks);
                // ODF has no native HR; emit as a Horizontal Line paragraph
                blocks.push(TextBlock::Paragraph(Paragraph {
                    style_name: Some("Horizontal Line".to_owned()),
                    content: Vec::new(),
                    ..Paragraph::default()
                }));
            }

            node::FOOTNOTE_DEF => {
                // Footnote defs are embedded in their ref; skip top-level defs
            }

            node::FIGURE => {
                flush_blockquote(&mut blockquote_buf, &mut blocks);
                if let Some(frame) = convert_figure_to_frame(n, ctx) {
                    blocks.push(TextBlock::Frame(frame));
                } else {
                    let inner = convert_nodes(&n.children, ctx);
                    blocks.extend(inner);
                }
            }

            node::DIV => {
                flush_blockquote(&mut blockquote_buf, &mut blocks);
                let inner = convert_nodes(&n.children, ctx);
                blocks.extend(inner);
            }

            _ => {
                // Unknown block: recurse into children
                flush_blockquote(&mut blockquote_buf, &mut blocks);
                let inner = convert_nodes(&n.children, ctx);
                blocks.extend(inner);
            }
        }
    }

    flush_blockquote(&mut blockquote_buf, &mut blocks);
    blocks
}

fn flush_blockquote(buf: &mut Vec<TextBlock>, out: &mut Vec<TextBlock>) {
    out.append(buf);
}

/// Reconstruct a `<draw:frame>` from a `figure` node holding an `image`
/// child and (optionally) a `caption` child — the counterpart of
/// `rescribe::read::convert_frame`, which builds that same `figure` shape
/// from a frame with an image *and* a text-box. Returns `None` for a
/// `figure` with no `image` child, which the caller falls back to
/// flattening (as it already did before `figure` had frame-reconstruction
/// support).
fn convert_figure_to_frame(n: &Node, ctx: &mut WriteCtx<'_>) -> Option<Frame> {
    let image_node = n.children.iter().find(|c| c.kind.as_str() == node::IMAGE)?;
    let src = image_node.props.get_str("src")?;
    let (href, mime_type) = ctx.resolve_image(src)?;
    let mut children = vec![FrameChild::Image { href, mime_type }];

    for c in &n.children {
        if c.kind.as_str() == node::CAPTION {
            let content = convert_nodes(&c.children, ctx);
            children.push(FrameChild::TextBox(content));
        }
    }

    Some(Frame {
        name: image_node.props.get_str("odt:name").map(str::to_owned),
        content: crate::ast::FrameContent { children },
        ..Frame::default()
    })
}

// ── Inline node conversion ────────────────────────────────────────────────────

fn collect_inlines(nodes: &[Node], ctx: &mut WriteCtx<'_>) -> Vec<Inline> {
    nodes
        .iter()
        .flat_map(|n| convert_inline_node(n, ctx))
        .collect()
}

fn convert_inline_node(n: &Node, ctx: &mut WriteCtx<'_>) -> Vec<Inline> {
    match n.kind.as_str() {
        node::TEXT => {
            let content = n.props.get_str(prop::CONTENT).unwrap_or("");
            if content.is_empty() {
                return Vec::new();
            }
            // Expand \t and handle spaces
            if content == "\t" {
                vec![Inline::Tab]
            } else {
                vec![Inline::Text(content.to_owned())]
            }
        }

        node::LINE_BREAK => vec![Inline::LineBreak],
        node::SOFT_BREAK => vec![Inline::Text(" ".to_owned())],

        node::STRONG => vec![Inline::Span(Span {
            style_name: Some("Bold".to_owned()),
            content: collect_inlines(&n.children, ctx),
        })],

        node::EMPHASIS => vec![Inline::Span(Span {
            style_name: Some("Italic".to_owned()),
            content: collect_inlines(&n.children, ctx),
        })],

        node::UNDERLINE => vec![Inline::Span(Span {
            style_name: Some("Underline".to_owned()),
            content: collect_inlines(&n.children, ctx),
        })],

        node::STRIKEOUT => vec![Inline::Span(Span {
            style_name: Some("Strikethrough".to_owned()),
            content: collect_inlines(&n.children, ctx),
        })],

        node::CODE => {
            let content = n.props.get_str(prop::CONTENT).unwrap_or("").to_owned();
            let inner = if content.is_empty() {
                collect_inlines(&n.children, ctx)
            } else {
                vec![Inline::Text(content)]
            };
            vec![Inline::Span(Span {
                style_name: Some("Code".to_owned()),
                content: inner,
            })]
        }

        node::SUBSCRIPT => vec![Inline::Span(Span {
            style_name: Some("Subscript".to_owned()),
            content: collect_inlines(&n.children, ctx),
        })],

        node::SUPERSCRIPT => vec![Inline::Span(Span {
            style_name: Some("Superscript".to_owned()),
            content: collect_inlines(&n.children, ctx),
        })],

        node::LINK => {
            let href = n.props.get_str(prop::URL).map(str::to_owned);
            let title = n.props.get_str(prop::TITLE).map(str::to_owned);
            vec![Inline::Hyperlink(Hyperlink {
                href,
                title,
                content: collect_inlines(&n.children, ctx),
                ..Hyperlink::default()
            })]
        }

        node::IMAGE => {
            let Some(src) = n.props.get_str("src") else {
                return Vec::new();
            };
            let Some((href, mime_type)) = ctx.resolve_image(src) else {
                // `src` didn't resolve to a resource in `doc.resources` (e.g. a
                // bare URL rather than a `ResourceId`) — preserve it as the
                // frame's `href` directly rather than dropping the image.
                return vec![Inline::Frame(Frame {
                    name: n.props.get_str("odt:name").map(str::to_owned),
                    content: crate::ast::FrameContent {
                        children: vec![FrameChild::Image {
                            href: src.to_owned(),
                            mime_type: None,
                        }],
                    },
                    ..Frame::default()
                })];
            };
            vec![Inline::Frame(Frame {
                name: n.props.get_str("odt:name").map(str::to_owned),
                content: crate::ast::FrameContent {
                    children: vec![FrameChild::Image { href, mime_type }],
                },
                ..Frame::default()
            })]
        }

        node::SPAN => {
            // Re-attach any style properties as a named or auto span
            let style_name = n.props.get_str("odf:style-name").map(str::to_owned);
            vec![Inline::Span(Span {
                style_name,
                content: collect_inlines(&n.children, ctx),
            })]
        }

        node::FOOTNOTE_REF => {
            // Footnote refs: in the rescribe IR the def is a sibling, not embedded.
            // ODT encodes the body inside <text:note>. We emit an empty citation here;
            // full round-trip would need the def bodies passed in.
            Vec::new()
        }

        _ => collect_inlines(&n.children, ctx),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rescribe_std::builder::*;

    #[test]
    fn test_emit_basic() {
        let document = doc(|d| {
            d.heading(1, |h| h.text("Title"))
                .para(|p| p.text("Hello world"))
        });
        let result = emit(&document).unwrap();
        assert!(!result.value.is_empty());
        // Check it's a valid ZIP starting with PK
        assert_eq!(&result.value[0..2], b"PK");
    }

    #[test]
    fn test_emit_roundtrip_heading() {
        let document = doc(|d| d.heading(2, |h| h.text("Section")));
        let result = emit(&document).unwrap();
        assert_eq!(&result.value[0..2], b"PK");
    }

    #[test]
    fn test_emit_roundtrip_para_with_bold() {
        let document =
            doc(|d| d.para(|p| p.text("plain ").strong(|s| s.text("bold")).text(" end")));
        let result = emit(&document).unwrap();
        assert_eq!(&result.value[0..2], b"PK");
    }

    #[test]
    fn test_emit_list() {
        let document =
            doc(|d| d.bullet_list(|l| l.item(|i| i.text("first")).item(|i| i.text("second"))));
        let result = emit(&document).unwrap();
        assert_eq!(&result.value[0..2], b"PK");
    }

    #[test]
    fn test_emit_table() {
        let document = doc(|d| d.table(|t| t.row(|r| r.cell(|c| c.text("cell")))));
        let result = emit(&document).unwrap();
        assert_eq!(&result.value[0..2], b"PK");
    }

    // ── Regression: image nodes used to be dropped silently on write ───────────
    //
    // `node::IMAGE` used to convert to an empty `Vec` unconditionally ("images
    // require ZIP embedding; for now emit an empty span as placeholder" — no
    // fidelity warning either). Any `Document` with an image lost it entirely
    // on `emit()`. `WriteCtx::resolve_image` now embeds the resource bytes and
    // reconstructs a real `<draw:frame><draw:image>`.

    #[test]
    fn image_node_survives_emit_and_reparse() {
        use rescribe_core::{Resource, ResourceId, ResourceMap};

        let id = ResourceId::from_string("img1");
        let mut resources = ResourceMap::new();
        resources.insert(id.clone(), Resource::png(vec![0x89, 0x50, 0x4e, 0x47]));

        let document = Document {
            content: Node::new(rescribe_std::node::DOCUMENT).child(
                Node::new(rescribe_std::node::PARAGRAPH)
                    .child(Node::new(rescribe_std::node::IMAGE).prop("src", id.as_str())),
            ),
            resources,
            metadata: Default::default(),
            source: None,
        };

        let bytes = emit(&document).unwrap().value;
        let parsed = crate::parser::parse(&bytes).expect("parse failed");
        assert_eq!(parsed.value.images.len(), 1, "image was not embedded");
        let data = parsed.value.images.values().next().unwrap();
        assert_eq!(data, &vec![0x89, 0x50, 0x4e, 0x47]);
    }

    // ── Regression: figure(image, caption) used to flatten and drop the
    // image, since `FIGURE` fell into the generic "recurse into children"
    // branch that itself dropped IMAGE. Now `convert_figure_to_frame`
    // reconstructs a `<draw:frame>` with both the image and the caption
    // text-box, mirroring `rescribe::read::convert_frame`'s figure/caption
    // read-side reconstruction.

    #[test]
    fn figure_with_image_and_caption_survives_emit() {
        use rescribe_core::{Resource, ResourceId, ResourceMap};

        let id = ResourceId::from_string("img1");
        let mut resources = ResourceMap::new();
        resources.insert(id.clone(), Resource::png(vec![1, 2, 3, 4]));

        let figure = Node::new(rescribe_std::node::FIGURE)
            .child(Node::new(rescribe_std::node::IMAGE).prop("src", id.as_str()))
            .child(
                Node::new(rescribe_std::node::CAPTION).child(
                    Node::new(rescribe_std::node::PARAGRAPH)
                        .child(Node::new(rescribe_std::node::TEXT).prop("content", "Figure 1.")),
                ),
            );

        let document = Document {
            content: Node::new(rescribe_std::node::DOCUMENT).child(figure),
            resources,
            metadata: Default::default(),
            source: None,
        };

        let bytes = emit(&document).unwrap().value;
        assert_eq!(&bytes[0..2], b"PK");
        let parsed = crate::parser::parse(&bytes).expect("parse failed");
        assert_eq!(parsed.value.images.len(), 1, "image was not embedded");

        let crate::ast::OdfBody::Text(blocks) = &parsed.value.body else {
            panic!("expected Text body");
        };
        let crate::ast::TextBlock::Frame(frame) = &blocks[0] else {
            panic!("expected Frame block, got {:?}", blocks[0]);
        };
        assert_eq!(
            frame.content.children.len(),
            2,
            "expected image + caption text-box, got {:?}",
            frame.content
        );
    }

    // ── Spreadsheet / presentation translation (ADR 0015) ───────────────────

    #[test]
    fn spreadsheet_document_round_trips_through_ast() {
        let sheet = Node::new(rescribe_std::node::SHEET)
            .prop("odf:name", "Sales")
            .child(
                Node::new(rescribe_std::node::SHEET_ROW).child(
                    Node::new(rescribe_std::node::SHEET_CELL)
                        .prop(prop::VALUE_TYPE, "number")
                        .prop(prop::VALUE, "42")
                        .prop(prop::VALUE_FORMULA, "of:=1+41"),
                ),
            );
        let document = Document {
            content: Node::new(rescribe_std::node::DOCUMENT).child(sheet),
            resources: Default::default(),
            metadata: Default::default(),
            source: None,
        };

        let bytes = emit(&document).unwrap().value;
        let parsed = crate::parser::parse(&bytes).expect("parse failed");
        assert_eq!(
            parsed.value.mimetype,
            "application/vnd.oasis.opendocument.spreadsheet"
        );
        let crate::ast::OdfBody::Spreadsheet(body) = &parsed.value.body else {
            panic!("expected Spreadsheet body, got {:?}", parsed.value.body);
        };
        assert_eq!(body.sheets[0].name.as_deref(), Some("Sales"));
        let cell = &body.sheets[0].rows[0].cells[0];
        assert_eq!(cell.value_type.as_deref(), Some("float"));
        assert_eq!(cell.value.as_deref(), Some("42"));
        assert_eq!(cell.formula.as_deref(), Some("of:=1+41"));

        // Full cycle back through the rescribe reader.
        let reread = crate::rescribe::read::parse(&bytes).unwrap();
        assert!(reread.warnings.is_empty());
        let reread_sheet = &reread.value.content.children[0];
        assert_eq!(reread_sheet.kind.as_str(), rescribe_std::node::SHEET);
        let reread_cell = &reread_sheet.children[0].children[0];
        assert_eq!(reread_cell.props.get_str(prop::VALUE_TYPE), Some("number"));
        assert_eq!(reread_cell.props.get_str(prop::VALUE), Some("42"));
        assert_eq!(
            reread_cell.props.get_str(prop::VALUE_FORMULA),
            Some("of:=1+41")
        );
    }

    // ── Chart translation (ADR 0016) ─────────────────────────────────────

    #[test]
    fn synthesized_chart_round_trips_through_ast_and_reader() {
        let series = Node::new(rescribe_std::node::CHART_SERIES)
            .prop(prop::CHART_VALUES_REF, "Sales.B2:B3")
            .prop(prop::CHART_CATEGORIES_REF, "Sales.A2:A3");
        let chart = Node::new(rescribe_std::node::CHART)
            .prop(prop::TITLE, "Revenue")
            .prop(prop::CHART_TYPE, "bar")
            .prop(prop::CHART_LEGEND, true)
            .prop(prop::CHART_LEGEND_POSITION, "end")
            .prop(prop::CHART_HAS_CATEGORY_AXIS, true)
            .prop(prop::CHART_HAS_VALUE_AXIS, true)
            .child(series);
        let shape = Node::new(rescribe_std::node::POSITIONED_CONTAINER)
            .prop(prop::POSITION_X, 0_i64)
            .prop(prop::POSITION_Y, 0_i64)
            .prop(prop::POSITION_WIDTH, 3_600_000_i64)
            .prop(prop::POSITION_HEIGHT, 2_400_000_i64)
            .prop(prop::POSITION_Z_ORDER, 0_i64)
            .child(chart);
        let sheet = Node::new(rescribe_std::node::SHEET)
            .prop("odf:name", "Sales")
            .child(
                Node::new(rescribe_std::node::SHEET_ROW).child(
                    Node::new(rescribe_std::node::SHEET_CELL)
                        .prop(prop::VALUE_TYPE, "string")
                        .prop(prop::VALUE, "Widget"),
                ),
            )
            .child(shape);
        let document = Document {
            content: Node::new(rescribe_std::node::DOCUMENT).child(sheet),
            resources: Default::default(),
            metadata: Default::default(),
            source: None,
        };

        let result = emit(&document).unwrap();
        let bytes = result.value;
        assert_eq!(&bytes[0..2], b"PK");

        let parsed = crate::parser::parse(&bytes).expect("parse failed");
        let crate::ast::OdfBody::Spreadsheet(body) = &parsed.value.body else {
            panic!("expected Spreadsheet body, got {:?}", parsed.value.body);
        };
        let ast_sheet = &body.sheets[0];
        assert_eq!(ast_sheet.shapes.len(), 1, "chart shape not written");
        let crate::ast::DrawShapeContent::Chart { href, chart } = &ast_sheet.shapes[0].content
        else {
            panic!(
                "expected Chart shape content, got {:?}",
                ast_sheet.shapes[0].content
            );
        };
        assert_eq!(href, "Object 1");
        assert_eq!(chart.chart_class.as_deref(), Some("bar"));
        assert_eq!(chart.title.as_deref(), Some("Revenue"));
        assert!(chart.legend);
        assert_eq!(chart.legend_position.as_deref(), Some("end"));
        assert!(chart.has_category_axis);
        assert!(chart.has_value_axis);
        assert_eq!(
            chart.series[0].values_cell_range_address.as_deref(),
            Some("Sales.B2:B3")
        );
        assert_eq!(
            chart.series[0].categories_cell_range_address.as_deref(),
            Some("Sales.A2:A3")
        );

        // Full cycle back through the rescribe reader.
        let reread = crate::rescribe::read::parse(&bytes).unwrap();
        let reread_sheet = &reread.value.content.children[0];
        let reread_chart_container = reread_sheet
            .children
            .iter()
            .find(|c| c.kind.as_str() == rescribe_std::node::POSITIONED_CONTAINER)
            .expect("no positioned_container in reread sheet");
        let reread_chart = &reread_chart_container.children[0];
        assert_eq!(reread_chart.kind.as_str(), rescribe_std::node::CHART);
        assert_eq!(reread_chart.props.get_str(prop::TITLE), Some("Revenue"));
        assert_eq!(reread_chart.props.get_str(prop::CHART_TYPE), Some("bar"));
        assert_eq!(reread_chart.props.get_bool(prop::CHART_LEGEND), Some(true));
        assert!(reread_chart.props.get_str(prop::ODF_CHART_XML).is_some());
        let reread_series = &reread_chart.children[0];
        assert_eq!(
            reread_series.kind.as_str(),
            rescribe_std::node::CHART_SERIES
        );
        assert_eq!(
            reread_series.props.get_str(prop::CHART_VALUES_REF),
            Some("Sales.B2:B3")
        );
        assert_eq!(
            reread_series.props.get_str(prop::CHART_CATEGORIES_REF),
            Some("Sales.A2:A3")
        );
    }

    #[test]
    fn raw_odf_chart_xml_round_trips_verbatim() {
        // A chart with `odf:chart-xml` set (as if read from a real ODF
        // file) must re-emit that exact blob rather than synthesizing one
        // (ADR 0016 Decision 4).
        let raw = "<office:chart><chart:chart chart:class=\"chart:pie\">\
                    <chart:title><text:p>Custom</text:p></chart:title>\
                    </chart:chart></office:chart>";
        let chart = Node::new(rescribe_std::node::CHART)
            .prop(prop::ODF_CHART_XML, raw)
            .prop(prop::CHART_LEGEND, false)
            .prop(prop::CHART_HAS_CATEGORY_AXIS, false)
            .prop(prop::CHART_HAS_VALUE_AXIS, false);
        let shape = Node::new(rescribe_std::node::POSITIONED_CONTAINER)
            .prop(prop::POSITION_Z_ORDER, 0_i64)
            .child(chart);
        let sheet = Node::new(rescribe_std::node::SHEET)
            .prop("odf:name", "S1")
            .child(shape);
        let document = Document {
            content: Node::new(rescribe_std::node::DOCUMENT).child(sheet),
            resources: Default::default(),
            metadata: Default::default(),
            source: None,
        };

        let bytes = emit(&document).unwrap().value;
        let parsed = crate::parser::parse(&bytes).expect("parse failed");
        let crate::ast::OdfBody::Spreadsheet(body) = &parsed.value.body else {
            panic!("expected Spreadsheet body");
        };
        let crate::ast::DrawShapeContent::Chart { chart, .. } = &body.sheets[0].shapes[0].content
        else {
            panic!("expected Chart shape content");
        };
        assert_eq!(
            chart.raw_xml, raw,
            "raw chart XML did not round-trip verbatim"
        );
    }

    #[test]
    fn presentation_document_round_trips_through_ast() {
        let shape = Node::new(rescribe_std::node::POSITIONED_CONTAINER)
            .prop(prop::POSITION_X, 914_400_i64) // 1 inch
            .prop(prop::POSITION_Y, 0_i64)
            .prop(prop::POSITION_WIDTH, 1_828_800_i64)
            .prop(prop::POSITION_HEIGHT, 914_400_i64)
            .prop(prop::POSITION_ROTATION, 5_400_000_i64) // 90 degrees
            .prop(prop::POSITION_Z_ORDER, 0_i64)
            .prop("odf:presentation-class", "title")
            .child(
                Node::new(rescribe_std::node::PARAGRAPH)
                    .child(Node::new(rescribe_std::node::TEXT).prop(prop::CONTENT, "Title")),
            );
        let page = Node::new(rescribe_std::node::DIV)
            .prop("odf:type", "slide")
            .prop("odf:name", "slide1")
            .child(shape);
        let document = Document {
            content: Node::new(rescribe_std::node::DOCUMENT).child(page),
            resources: Default::default(),
            metadata: Default::default(),
            source: None,
        };

        let result = emit(&document).unwrap();
        // No raw odf:transform was present, so rotation had to be synthesized —
        // that's the documented lossy-pivot case and must warn (ADR 0015).
        assert_eq!(result.warnings.len(), 1);
        let bytes = result.value;
        let parsed = crate::parser::parse(&bytes).expect("parse failed");
        assert_eq!(
            parsed.value.mimetype,
            "application/vnd.oasis.opendocument.presentation"
        );
        let crate::ast::OdfBody::Presentation(body) = &parsed.value.body else {
            panic!("expected Presentation body, got {:?}", parsed.value.body);
        };
        assert_eq!(body.pages[0].name.as_deref(), Some("slide1"));
        let ast_shape = &body.pages[0].shapes[0];
        // 914_400 EMU == 1in == 2.54cm exactly (ADR 0015's EMU/cm ratio).
        assert_eq!(ast_shape.x.as_deref(), Some("2.54cm"));
        assert!(
            ast_shape
                .transform
                .as_deref()
                .unwrap()
                .starts_with("rotate(")
        );
        assert_eq!(ast_shape.z_index, Some(0));

        // Full cycle back through the rescribe reader: the synthesized
        // rotate() must project back to the same OOXML-style angle.
        let reread = crate::rescribe::read::parse(&bytes).unwrap();
        let reread_shape = &reread.value.content.children[0].children[0];
        assert_eq!(
            reread_shape.props.get_int(prop::POSITION_ROTATION),
            Some(5_400_000)
        );
        assert_eq!(reread_shape.props.get_int(prop::POSITION_Z_ORDER), Some(0));
    }
}
