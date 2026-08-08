//! ODT (OpenDocument Text) writer for rescribe.
//!
//! Generates ODF/ODT documents from rescribe's document IR by delegating to
//! the rest of this crate for all ZIP building and XML serialisation.

use crate::ast::{
    Frame, FrameChild, Heading, Hyperlink, Inline, List, ListItem, OdfBody, OdfDocument, OdfMeta,
    Paragraph, ParagraphProperties, Span, StyleEntry, Table, TableCell, TableRow, TextBlock,
    TextProperties,
};
use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node, ResourceMap};
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
    let odf = convert_document(doc);
    let bytes =
        crate::emit(&odf).map_err(|e| EmitError::Io(std::io::Error::other(e.to_string())))?;
    Ok(ConversionResult::ok(bytes))
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
}

impl<'a> WriteCtx<'a> {
    fn new(resources: &'a ResourceMap) -> Self {
        Self {
            resources,
            images: HashMap::new(),
            next_image: 0,
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

fn convert_document(doc: &Document) -> OdfDocument {
    let mut ctx = WriteCtx::new(&doc.resources);
    let blocks = convert_nodes(&doc.content.children, &mut ctx);
    OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.text".to_owned(),
        meta: OdfMeta {
            title: doc.metadata.get_str("title").map(str::to_owned),
            creator: doc.metadata.get_str("author").map(str::to_owned),
            ..OdfMeta::default()
        },
        named_styles: build_named_styles(),
        body: OdfBody::Text(blocks),
        images: ctx.images,
        ..OdfDocument::default()
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
}
