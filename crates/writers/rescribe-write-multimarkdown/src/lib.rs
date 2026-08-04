//! MultiMarkdown writer for rescribe.
//!
//! A thin IR→AST translator: all MultiMarkdown emission (metadata blocks,
//! citations, cross-references, and everything CommonMark/GFM defines)
//! lives in the standalone `multimarkdown-fmt` crate. This crate only
//! converts a rescribe `Document` into `multimarkdown_fmt::MmdDoc`.

use multimarkdown_fmt::{
    MetadataEntry, MetadataStyle, MmdBlock, MmdDefinitionListItem, MmdDoc, MmdInline, MmdListItem,
    MmdTableCell, MmdTableRow, Span,
};
use rescribe_core::{
    ConversionResult, Document, EmitError, EmitOptions, FidelityWarning, Node, PropValue, Severity,
    WarningKind,
};
use rescribe_std::{node, prop};

/// Emit a document as MultiMarkdown.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    emit_with_options(doc, &EmitOptions::default())
}

/// Emit a document as MultiMarkdown with options.
pub fn emit_with_options(
    doc: &Document,
    _options: &EmitOptions,
) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let metadata: Vec<MetadataEntry> = doc
        .metadata
        .iter()
        .filter_map(|(key, value)| match value {
            PropValue::String(s) => Some(MetadataEntry {
                key: key.clone(),
                value: s.clone(),
            }),
            _ => None,
        })
        .collect();
    let metadata_style = if metadata.is_empty() {
        MetadataStyle::None
    } else {
        MetadataStyle::Bare
    };

    let mut warnings = Vec::new();
    let blocks = nodes_to_blocks(&doc.content.children, &mut warnings);

    let mmd_doc = MmdDoc {
        metadata,
        metadata_style,
        blocks,
        link_defs: Vec::new(),
    };

    let bytes = multimarkdown_fmt::emit(&mmd_doc);
    Ok(ConversionResult::with_warnings(bytes, warnings))
}

fn unsupported(kind: &str, warnings: &mut Vec<FidelityWarning>) {
    warnings.push(FidelityWarning::new(
        Severity::Minor,
        WarningKind::UnsupportedNode(kind.to_string()),
        format!("node kind \"{kind}\" has no MultiMarkdown equivalent; dropped"),
    ));
}

fn nodes_to_blocks(nodes: &[Node], warnings: &mut Vec<FidelityWarning>) -> Vec<MmdBlock> {
    nodes.iter().map(|n| node_to_block(n, warnings)).collect()
}

fn node_to_block(n: &Node, warnings: &mut Vec<FidelityWarning>) -> MmdBlock {
    match n.kind.as_str() {
        "paragraph" => MmdBlock::Paragraph {
            inlines: nodes_to_inlines(&n.children, warnings),
            span: Span::NONE,
        },
        "heading" => MmdBlock::Heading {
            level: n.props.get_int(prop::LEVEL).unwrap_or(1) as u8,
            inlines: nodes_to_inlines(&n.children, warnings),
            anchor: n.props.get_str(prop::ID).map(str::to_string),
            span: Span::NONE,
        },
        "code_block" => MmdBlock::CodeBlock {
            language: n.props.get_str(prop::LANGUAGE).map(str::to_string),
            content: n.props.get_str(prop::CONTENT).unwrap_or("").to_string(),
            span: Span::NONE,
        },
        "raw_block" => MmdBlock::HtmlBlock {
            content: n.props.get_str(prop::CONTENT).unwrap_or("").to_string(),
            span: Span::NONE,
        },
        "blockquote" => MmdBlock::Blockquote {
            blocks: nodes_to_blocks(&n.children, warnings),
            span: Span::NONE,
        },
        "list" => {
            let ordered = n.props.get_bool(prop::ORDERED).unwrap_or(false);
            let items = n
                .children
                .iter()
                .filter(|c| c.kind.as_str() == node::LIST_ITEM)
                .map(|item| MmdListItem {
                    blocks: nodes_to_blocks(&item.children, warnings),
                    span: Span::NONE,
                    checked: item.props.get_bool(prop::CHECKED),
                })
                .collect();
            MmdBlock::List {
                kind: if ordered {
                    multimarkdown_fmt::ListKind::Ordered {
                        start: 1,
                        marker: multimarkdown_fmt::OrderedMarker::Period,
                    }
                } else {
                    multimarkdown_fmt::ListKind::Unordered { marker: '-' }
                },
                items,
                tight: n.props.get_bool(prop::TIGHT).unwrap_or(true),
                span: Span::NONE,
            }
        }
        "horizontal_rule" => MmdBlock::ThematicBreak { span: Span::NONE },
        "table" => {
            let mut rows = n
                .children
                .iter()
                .filter(|c| c.kind.as_str() == node::TABLE_ROW);
            let head = rows
                .next()
                .map(|r| table_row(r, warnings))
                .unwrap_or(MmdTableRow {
                    cells: Vec::new(),
                    span: Span::NONE,
                });
            let rows = rows.map(|r| table_row(r, warnings)).collect();
            MmdBlock::Table {
                alignments: Vec::new(),
                head,
                rows,
                span: Span::NONE,
            }
        }
        "footnote_def" => MmdBlock::FootnoteDefinition {
            label: n.props.get_str(prop::ID).unwrap_or("").to_string(),
            blocks: nodes_to_blocks(&n.children, warnings),
            span: Span::NONE,
        },
        "definition_list" => {
            let mut items: Vec<MmdDefinitionListItem> = Vec::new();
            for child in &n.children {
                match child.kind.as_str() {
                    "definition_term" => items.push(MmdDefinitionListItem {
                        term: nodes_to_inlines(&child.children, warnings),
                        definitions: Vec::new(),
                        span: Span::NONE,
                    }),
                    "definition_desc" => {
                        if let Some(last) = items.last_mut() {
                            last.definitions
                                .push(nodes_to_blocks(&child.children, warnings));
                        }
                    }
                    _ => {}
                }
            }
            MmdBlock::DefinitionList {
                items,
                tight: true,
                span: Span::NONE,
            }
        }
        "mmd:citation_def" => MmdBlock::CitationDefinition {
            label: n.props.get_str(prop::ID).unwrap_or("").to_string(),
            content: nodes_to_inlines(&n.children, warnings),
            span: Span::NONE,
        },
        other => {
            unsupported(other, warnings);
            MmdBlock::Paragraph {
                inlines: nodes_to_inlines(&n.children, warnings),
                span: Span::NONE,
            }
        }
    }
}

fn table_row(n: &Node, warnings: &mut Vec<FidelityWarning>) -> MmdTableRow {
    MmdTableRow {
        cells: n
            .children
            .iter()
            .map(|c| MmdTableCell {
                inlines: nodes_to_inlines(&c.children, warnings),
                span: Span::NONE,
            })
            .collect(),
        span: Span::NONE,
    }
}

fn nodes_to_inlines(nodes: &[Node], warnings: &mut Vec<FidelityWarning>) -> Vec<MmdInline> {
    nodes.iter().map(|n| node_to_inline(n, warnings)).collect()
}

fn node_to_inline(n: &Node, warnings: &mut Vec<FidelityWarning>) -> MmdInline {
    match n.kind.as_str() {
        "text" => MmdInline::Text {
            content: n.props.get_str(prop::CONTENT).unwrap_or("").to_string(),
            span: Span::NONE,
        },
        "soft_break" => MmdInline::SoftBreak { span: Span::NONE },
        "line_break" => MmdInline::HardBreak { span: Span::NONE },
        "emphasis" => MmdInline::Emphasis {
            inlines: nodes_to_inlines(&n.children, warnings),
            span: Span::NONE,
        },
        "strong" => MmdInline::Strong {
            inlines: nodes_to_inlines(&n.children, warnings),
            span: Span::NONE,
        },
        "strikeout" => MmdInline::Strikethrough {
            inlines: nodes_to_inlines(&n.children, warnings),
            span: Span::NONE,
        },
        "code" => MmdInline::Code {
            content: n.props.get_str(prop::CONTENT).unwrap_or("").to_string(),
            span: Span::NONE,
        },
        "raw_inline" => MmdInline::HtmlInline {
            content: n.props.get_str(prop::CONTENT).unwrap_or("").to_string(),
            span: Span::NONE,
        },
        "link" => MmdInline::Link {
            inlines: nodes_to_inlines(&n.children, warnings),
            url: n.props.get_str(prop::URL).unwrap_or("").to_string(),
            title: n.props.get_str(prop::TITLE).map(str::to_string),
            span: Span::NONE,
        },
        "image" => MmdInline::Image {
            alt: n.props.get_str(prop::ALT).unwrap_or("").to_string(),
            url: n.props.get_str(prop::URL).unwrap_or("").to_string(),
            title: n.props.get_str(prop::TITLE).map(str::to_string),
            span: Span::NONE,
        },
        "footnote_ref" => MmdInline::FootnoteReference {
            label: n.props.get_str(prop::ID).unwrap_or("").to_string(),
            span: Span::NONE,
        },
        "math_inline" => MmdInline::InlineMath {
            source: n.props.get_str(prop::CONTENT).unwrap_or("").to_string(),
            span: Span::NONE,
        },
        "math_block" => MmdInline::DisplayMath {
            source: n.props.get_str(prop::CONTENT).unwrap_or("").to_string(),
            span: Span::NONE,
        },
        "mmd:citation" => MmdInline::Citation {
            locator: n.props.get_str("locator").map(str::to_string),
            label: n.props.get_str("label").unwrap_or("").to_string(),
            span: Span::NONE,
        },
        "mmd:cross_reference" => MmdInline::CrossReference {
            target: n.props.get_str("target").unwrap_or("").to_string(),
            collapsed: n.props.get_bool("collapsed").unwrap_or(false),
            span: Span::NONE,
        },
        other => {
            unsupported(other, warnings);
            MmdInline::Text {
                content: String::new(),
                span: Span::NONE,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rescribe_core::NodeKind;

    fn emit_str(doc: &Document) -> String {
        String::from_utf8(emit(doc).unwrap().value).unwrap()
    }

    #[test]
    fn test_emit_basic() {
        let doc = Document::new().with_content(
            Node::new(NodeKind::from("document")).child(
                Node::new(NodeKind::from("paragraph"))
                    .child(Node::new(NodeKind::from("text")).prop("content", "Hello world")),
            ),
        );

        let output = emit_str(&doc);
        assert!(output.contains("Hello world"));
    }

    #[test]
    fn test_emit_footnote() {
        let doc = Document::new().with_content(
            Node::new(NodeKind::from("document"))
                .child(
                    Node::new(NodeKind::from("paragraph"))
                        .child(Node::new(NodeKind::from("text")).prop("content", "Text"))
                        .child(Node::new(NodeKind::from("footnote_ref")).prop("id", "1")),
                )
                .child(
                    Node::new(NodeKind::from("footnote_def"))
                        .prop("id", "1")
                        .child(Node::new(NodeKind::from("paragraph")).child(
                            Node::new(NodeKind::from("text")).prop("content", "Footnote text"),
                        )),
                ),
        );

        let output = emit_str(&doc);
        assert!(output.contains("[^1]"));
        assert!(output.contains("[^1]: Footnote text"));
    }

    #[test]
    fn test_emit_definition_list() {
        let doc =
            Document::new().with_content(
                Node::new(NodeKind::from("document")).child(
                    Node::new(NodeKind::from("definition_list"))
                        .child(
                            Node::new(NodeKind::from("definition_term"))
                                .child(Node::new(NodeKind::from("text")).prop("content", "Term")),
                        )
                        .child(Node::new(NodeKind::from("definition_desc")).child(
                            Node::new(NodeKind::from("paragraph")).child(
                                Node::new(NodeKind::from("text")).prop("content", "Definition"),
                            ),
                        )),
                ),
            );

        let output = emit_str(&doc);
        assert!(output.contains("Term"));
        assert!(output.contains("Definition"));
    }

    #[test]
    fn test_emit_math() {
        let doc = Document::new().with_content(
            Node::new(NodeKind::from("document")).child(
                Node::new(NodeKind::from("paragraph"))
                    .child(Node::new(NodeKind::from("math_inline")).prop("content", "x^2")),
            ),
        );

        let output = emit_str(&doc);
        assert!(output.contains("x^2"));
    }

    #[test]
    fn test_emit_table() {
        let doc = Document::new().with_content(
            Node::new(NodeKind::from("document")).child(
                Node::new(NodeKind::from("table"))
                    .child(
                        Node::new(NodeKind::from("table_row"))
                            .child(
                                Node::new(NodeKind::from("table_header"))
                                    .child(Node::new(NodeKind::from("text")).prop("content", "A")),
                            )
                            .child(
                                Node::new(NodeKind::from("table_header"))
                                    .child(Node::new(NodeKind::from("text")).prop("content", "B")),
                            ),
                    )
                    .child(
                        Node::new(NodeKind::from("table_row"))
                            .child(
                                Node::new(NodeKind::from("table_cell"))
                                    .child(Node::new(NodeKind::from("text")).prop("content", "1")),
                            )
                            .child(
                                Node::new(NodeKind::from("table_cell"))
                                    .child(Node::new(NodeKind::from("text")).prop("content", "2")),
                            ),
                    ),
            ),
        );

        let output = emit_str(&doc);
        assert!(output.contains("A"));
        assert!(output.contains("1"));
    }

    #[test]
    fn test_emit_citation() {
        let doc = Document::new().with_content(
            Node::new(NodeKind::from("document")).child(
                Node::new(NodeKind::from("paragraph")).child(
                    Node::new(NodeKind::from("mmd:citation"))
                        .prop("locator", "p. 23")
                        .prop("label", "Doe:2006"),
                ),
            ),
        );
        let output = emit_str(&doc);
        let (parsed, diags) = multimarkdown_fmt::parse(output.as_bytes());
        assert!(diags.is_empty());
        assert!(matches!(
            &parsed.blocks[0],
            MmdBlock::Paragraph { inlines, .. }
                if matches!(&inlines[0], MmdInline::Citation { label, .. } if label == "Doe:2006")
        ));
    }
}
