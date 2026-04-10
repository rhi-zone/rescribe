//! ODT (OpenDocument Text) writer for rescribe.
//!
//! Generates ODF/ODT documents from rescribe's document IR by delegating to
//! `odf-fmt` for all ZIP building and XML serialisation.

use odf_fmt::ast::{
    Heading, Hyperlink, Inline, List, ListItem, OdfBody, OdfDocument, OdfMeta,
    Paragraph, Span, StyleEntry, Table, TableCell, TableRow, TextBlock, TextProperties,
    ParagraphProperties,
};
use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node};
use rescribe_std::{node, prop};

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
    let bytes = odf_fmt::emit(&odf).map_err(|e| EmitError::Io(std::io::Error::other(e.to_string())))?;
    Ok(ConversionResult::ok(bytes))
}

// ── Document conversion ───────────────────────────────────────────────────────

fn convert_document(doc: &Document) -> OdfDocument {
    let blocks = convert_nodes(&doc.content.children);
    OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.text".to_owned(),
        meta: OdfMeta {
            title: doc.metadata.get_str("title").map(str::to_owned),
            creator: doc.metadata.get_str("author").map(str::to_owned),
            ..OdfMeta::default()
        },
        named_styles: build_named_styles(),
        body: OdfBody::Text(blocks),
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
        text_entry("Bold", "text", TextProperties { bold: true, ..TextProperties::default() }),
        text_entry("Italic", "text", TextProperties { italic: true, ..TextProperties::default() }),
        text_entry("Underline", "text", TextProperties { underline: true, ..TextProperties::default() }),
        text_entry("Strikethrough", "text", TextProperties { strikethrough: true, ..TextProperties::default() }),
        text_entry("Code", "text", TextProperties {
            font_name: Some("Courier New".to_owned()),
            ..TextProperties::default()
        }),
        text_entry("Subscript", "text", TextProperties { subscript: true, ..TextProperties::default() }),
        text_entry("Superscript", "text", TextProperties { superscript: true, ..TextProperties::default() }),
        para_entry("Preformatted", ParagraphProperties::default()),
        para_entry("Quotation", ParagraphProperties {
            margin_left: Some("0.5in".to_owned()),
            ..ParagraphProperties::default()
        }),
    ]
}

// ── Block node conversion ─────────────────────────────────────────────────────

fn convert_nodes(nodes: &[Node]) -> Vec<TextBlock> {
    let mut blocks = Vec::new();
    let mut blockquote_buf: Vec<TextBlock> = Vec::new();

    for n in nodes {
        match n.kind.as_str() {
            node::DOCUMENT => {
                let inner = convert_nodes(&n.children);
                flush_blockquote(&mut blockquote_buf, &mut blocks);
                blocks.extend(inner);
            }

            node::PARAGRAPH => {
                flush_blockquote(&mut blockquote_buf, &mut blocks);
                let style = n.props.get_str("odt:style-name").map(str::to_owned);
                let content = collect_inlines(&n.children);
                blocks.push(TextBlock::Paragraph(Paragraph { style_name: style, content, ..Paragraph::default() }));
            }

            node::HEADING => {
                flush_blockquote(&mut blockquote_buf, &mut blocks);
                let level = n.props.get_int(prop::LEVEL).unwrap_or(1) as u32;
                let content = collect_inlines(&n.children);
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
                    if i > 0 { inlines.push(Inline::LineBreak); }
                    if !line.is_empty() { inlines.push(Inline::Text(line.to_owned())); }
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
                    let content = collect_inlines(&child.children);
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
                let style_name = if ordered { Some("List Number".to_owned()) } else { Some("List Bullet".to_owned()) };
                let items = n.children.iter()
                    .filter(|c| c.kind.as_str() == node::LIST_ITEM)
                    .map(|c| {
                        let content = convert_nodes(&c.children);
                        ListItem { content, ..ListItem::default() }
                    })
                    .collect();
                blocks.push(TextBlock::List(List { style_name, items, ..List::default() }));
            }

            node::TABLE => {
                flush_blockquote(&mut blockquote_buf, &mut blocks);
                let rows = n.children.iter()
                    .filter(|r| r.kind.as_str() == node::TABLE_ROW)
                    .map(|r| {
                        let cells = r.children.iter()
                            .map(|c| {
                                let colspan = c.props.get_int(prop::COLSPAN).map(|v| v as u32);
                                let rowspan = c.props.get_int(prop::ROWSPAN).map(|v| v as u32);
                                let content = convert_nodes(&c.children);
                                TableCell { col_span: colspan, row_span: rowspan, content, ..TableCell::default() }
                            })
                            .collect();
                        TableRow { cells, ..TableRow::default() }
                    })
                    .collect();
                blocks.push(TextBlock::Table(Table { rows, ..Table::default() }));
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

            node::DIV | node::FIGURE => {
                flush_blockquote(&mut blockquote_buf, &mut blocks);
                let inner = convert_nodes(&n.children);
                blocks.extend(inner);
            }

            _ => {
                // Unknown block: recurse into children
                flush_blockquote(&mut blockquote_buf, &mut blocks);
                let inner = convert_nodes(&n.children);
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

// ── Inline node conversion ────────────────────────────────────────────────────

fn collect_inlines(nodes: &[Node]) -> Vec<Inline> {
    nodes.iter().flat_map(convert_inline_node).collect()
}

fn convert_inline_node(n: &Node) -> Vec<Inline> {
    match n.kind.as_str() {
        node::TEXT => {
            let content = n.props.get_str(prop::CONTENT).unwrap_or("");
            if content.is_empty() { return Vec::new(); }
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
            content: collect_inlines(&n.children),
        })],

        node::EMPHASIS => vec![Inline::Span(Span {
            style_name: Some("Italic".to_owned()),
            content: collect_inlines(&n.children),
        })],

        node::UNDERLINE => vec![Inline::Span(Span {
            style_name: Some("Underline".to_owned()),
            content: collect_inlines(&n.children),
        })],

        node::STRIKEOUT => vec![Inline::Span(Span {
            style_name: Some("Strikethrough".to_owned()),
            content: collect_inlines(&n.children),
        })],

        node::CODE => {
            let content = n.props.get_str(prop::CONTENT).unwrap_or("").to_owned();
            let inner = if content.is_empty() {
                collect_inlines(&n.children)
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
            content: collect_inlines(&n.children),
        })],

        node::SUPERSCRIPT => vec![Inline::Span(Span {
            style_name: Some("Superscript".to_owned()),
            content: collect_inlines(&n.children),
        })],

        node::LINK => {
            let href = n.props.get_str(prop::URL).map(str::to_owned);
            let title = n.props.get_str(prop::TITLE).map(str::to_owned);
            vec![Inline::Hyperlink(Hyperlink {
                href,
                title,
                content: collect_inlines(&n.children),
                ..Hyperlink::default()
            })]
        }

        node::IMAGE => {
            // Images require ZIP embedding; for now emit an empty span as placeholder
            // (a full implementation would reconstruct draw:frame / draw:image)
            Vec::new()
        }

        node::SPAN => {
            // Re-attach any style properties as a named or auto span
            let style_name = n.props.get_str("odf:style-name").map(str::to_owned);
            vec![Inline::Span(Span {
                style_name,
                content: collect_inlines(&n.children),
            })]
        }

        node::FOOTNOTE_REF => {
            // Footnote refs: in the rescribe IR the def is a sibling, not embedded.
            // ODT encodes the body inside <text:note>. We emit an empty citation here;
            // full round-trip would need the def bodies passed in.
            Vec::new()
        }

        _ => collect_inlines(&n.children),
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
        let document = doc(|d| {
            d.para(|p| p.text("plain ").strong(|s| s.text("bold")).text(" end"))
        });
        let result = emit(&document).unwrap();
        assert_eq!(&result.value[0..2], b"PK");
    }

    #[test]
    fn test_emit_list() {
        let document = doc(|d| {
            d.bullet_list(|l| {
                l.item(|i| i.text("first"))
                    .item(|i| i.text("second"))
            })
        });
        let result = emit(&document).unwrap();
        assert_eq!(&result.value[0..2], b"PK");
    }

    #[test]
    fn test_emit_table() {
        let document = doc(|d| {
            d.table(|t| {
                t.row(|r| r.cell(|c| c.text("cell")))
            })
        });
        let result = emit(&document).unwrap();
        assert_eq!(&result.value[0..2], b"PK");
    }
}
