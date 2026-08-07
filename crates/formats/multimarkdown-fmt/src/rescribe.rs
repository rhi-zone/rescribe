//! AST↔`rescribe::Document` translation for MultiMarkdown.
//!
//! This module only translates between [`MmdDoc`](crate::MmdDoc) and
//! rescribe's `Document` IR — no MultiMarkdown parsing/emitting happens here
//! (that all lives in the rest of this crate; see `crate::parse` and
//! `crate::emit`). Enabled by the `rescribe` feature; each direction is
//! additionally gated on the reader/writer mode feature it depends on, so
//! enabling `rescribe` alone (with no mode feature) compiles nothing.

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
mod read {
    use crate::{MmdBlock, MmdDoc, MmdInline};
    use rescribe_core::{
        ConversionResult, Document, ParseError, ParseOptions, PropValue, Properties,
    };
    use rescribe_format_api::Parse as _;
    use rescribe_std::{Node, node, prop};

    /// Parse MultiMarkdown input into a document.
    pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
        parse_with_options(input, &ParseOptions::default())
    }

    /// Parse MultiMarkdown input into a document with options.
    pub fn parse_with_options(
        input: &str,
        _options: &ParseOptions,
    ) -> Result<ConversionResult<Document>, ParseError> {
        let (doc, diags) = MmdDoc::parse(input.as_bytes());

        let mut metadata = Properties::new();
        for entry in &doc.metadata {
            metadata.set(entry.key.clone(), PropValue::String(entry.value.clone()));
        }

        let content = Node::new(node::DOCUMENT).children(blocks_to_nodes(&doc.blocks));
        let document = Document {
            content,
            resources: Default::default(),
            metadata,
            source: None,
        };

        let warnings = diags
            .into_iter()
            .map(|d| {
                rescribe_core::FidelityWarning::new(
                    rescribe_core::Severity::Minor,
                    rescribe_core::WarningKind::FeatureLost(d.code.to_string()),
                    d.message,
                )
            })
            .collect();
        Ok(ConversionResult::with_warnings(document, warnings))
    }

    fn blocks_to_nodes(blocks: &[MmdBlock]) -> Vec<Node> {
        blocks.iter().map(block_to_node).collect()
    }

    fn block_to_node(block: &MmdBlock) -> Node {
        match block {
            MmdBlock::Paragraph { inlines, .. } => {
                Node::new(node::PARAGRAPH).children(inlines_to_nodes(inlines))
            }
            MmdBlock::Heading {
                level,
                inlines,
                anchor,
                ..
            } => {
                let mut heading = Node::new(node::HEADING)
                    .prop(prop::LEVEL, *level as i64)
                    .children(inlines_to_nodes(inlines));
                if let Some(anchor) = anchor {
                    heading = heading.prop(prop::ID, anchor.clone());
                }
                heading
            }
            MmdBlock::CodeBlock {
                language, content, ..
            } => {
                let mut code = Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone());
                if let Some(lang) = language {
                    code = code.prop(prop::LANGUAGE, lang.clone());
                }
                code
            }
            MmdBlock::HtmlBlock { content, .. } => Node::new(node::RAW_BLOCK)
                .prop(prop::FORMAT, "html")
                .prop(prop::CONTENT, content.clone()),
            MmdBlock::Blockquote { blocks, .. } => {
                Node::new(node::BLOCKQUOTE).children(blocks_to_nodes(blocks))
            }
            MmdBlock::List { items, tight, .. } => {
                let ordered = matches!(items_ordered(block), Some(true));
                let mut list = Node::new(node::LIST)
                    .prop(prop::ORDERED, ordered)
                    .prop(prop::TIGHT, *tight);
                for item in items {
                    let mut item_node =
                        Node::new(node::LIST_ITEM).children(blocks_to_nodes(&item.blocks));
                    if let Some(checked) = item.checked {
                        item_node = item_node.prop(prop::CHECKED, checked);
                    }
                    list = list.child(item_node);
                }
                list
            }
            MmdBlock::ThematicBreak { .. } => Node::new(node::HORIZONTAL_RULE),
            MmdBlock::Table { head, rows, .. } => {
                let mut table = Node::new(node::TABLE);
                let mut head_row = Node::new(node::TABLE_ROW);
                for cell in &head.cells {
                    head_row = head_row.child(
                        Node::new(node::TABLE_HEADER).children(inlines_to_nodes(&cell.inlines)),
                    );
                }
                table = table.child(head_row);
                for row in rows {
                    let mut row_node = Node::new(node::TABLE_ROW);
                    for cell in &row.cells {
                        row_node = row_node.child(
                            Node::new(node::TABLE_CELL).children(inlines_to_nodes(&cell.inlines)),
                        );
                    }
                    table = table.child(row_node);
                }
                table
            }
            MmdBlock::FootnoteDefinition { label, blocks, .. } => Node::new(node::FOOTNOTE_DEF)
                .prop(prop::ID, label.clone())
                .children(blocks_to_nodes(blocks)),
            MmdBlock::DefinitionList { items, .. } => {
                let mut dl = Node::new(node::DEFINITION_LIST);
                for item in items {
                    dl = dl.child(
                        Node::new(node::DEFINITION_TERM).children(inlines_to_nodes(&item.term)),
                    );
                    for def in &item.definitions {
                        dl = dl
                            .child(Node::new(node::DEFINITION_DESC).children(blocks_to_nodes(def)));
                    }
                }
                dl
            }
            MmdBlock::CitationDefinition { label, content, .. } => Node::new("mmd:citation_def")
                .prop(prop::ID, label.clone())
                .children(inlines_to_nodes(content)),
        }
    }

    /// `MmdBlock::List`'s `kind` field distinguishes ordered/unordered; pulled
    /// out to a helper only to keep `block_to_node`'s `List` arm's destructuring
    /// simple (the `kind` variant carries different fields per case).
    fn items_ordered(block: &MmdBlock) -> Option<bool> {
        match block {
            MmdBlock::List { kind, .. } => Some(matches!(kind, crate::ListKind::Ordered { .. })),
            _ => None,
        }
    }

    fn inlines_to_nodes(inlines: &[MmdInline]) -> Vec<Node> {
        inlines.iter().map(inline_to_node).collect()
    }

    fn inline_to_node(inline: &MmdInline) -> Node {
        match inline {
            MmdInline::Text { content, .. } => {
                Node::new(node::TEXT).prop(prop::CONTENT, content.clone())
            }
            MmdInline::SoftBreak { .. } => Node::new(node::SOFT_BREAK),
            MmdInline::HardBreak { .. } => Node::new(node::LINE_BREAK),
            MmdInline::Emphasis { inlines, .. } => {
                Node::new(node::EMPHASIS).children(inlines_to_nodes(inlines))
            }
            MmdInline::Strong { inlines, .. } => {
                Node::new(node::STRONG).children(inlines_to_nodes(inlines))
            }
            MmdInline::Strikethrough { inlines, .. } => {
                Node::new(node::STRIKEOUT).children(inlines_to_nodes(inlines))
            }
            MmdInline::Code { content, .. } => {
                Node::new(node::CODE).prop(prop::CONTENT, content.clone())
            }
            MmdInline::HtmlInline { content, .. } => Node::new(node::RAW_INLINE)
                .prop(prop::FORMAT, "html")
                .prop(prop::CONTENT, content.clone()),
            MmdInline::Link {
                inlines,
                url,
                title,
                ..
            } => {
                let mut link = Node::new(node::LINK)
                    .prop(prop::URL, url.clone())
                    .children(inlines_to_nodes(inlines));
                if let Some(title) = title {
                    link = link.prop(prop::TITLE, title.clone());
                }
                link
            }
            MmdInline::Image {
                alt, url, title, ..
            } => {
                let mut img = Node::new(node::IMAGE)
                    .prop(prop::URL, url.clone())
                    .prop(prop::ALT, alt.clone());
                if let Some(title) = title {
                    img = img.prop(prop::TITLE, title.clone());
                }
                img
            }
            MmdInline::FootnoteReference { label, .. } => {
                Node::new(node::FOOTNOTE_REF).prop(prop::ID, label.clone())
            }
            MmdInline::InlineMath { source, .. } => {
                Node::new("math_inline").prop(prop::CONTENT, source.clone())
            }
            MmdInline::DisplayMath { source, .. } => {
                Node::new("math_block").prop(prop::CONTENT, source.clone())
            }
            MmdInline::Citation { locator, label, .. } => {
                let mut n = Node::new("mmd:citation").prop("label", label.clone());
                if let Some(locator) = locator {
                    n = n.prop("locator", locator.clone());
                }
                n
            }
            MmdInline::CrossReference {
                target, collapsed, ..
            } => Node::new("mmd:cross_reference")
                .prop("target", target.clone())
                .prop("collapsed", *collapsed),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_parse_basic() {
            let md = "# Hello\n\nThis is a paragraph.";
            let result = parse(md).unwrap();
            let doc = result.value;
            assert_eq!(doc.content.children.len(), 2);
        }

        #[test]
        fn test_parse_footnote() {
            let md = "Here is a footnote[^1].\n\n[^1]: This is the footnote.";
            let result = parse(md).unwrap();
            let doc = result.value;
            assert!(!doc.content.children.is_empty());
        }

        #[test]
        fn test_parse_definition_list() {
            let md = "Term\n: Definition";
            let result = parse(md).unwrap();
            let doc = result.value;
            assert!(!doc.content.children.is_empty());
        }

        #[test]
        fn test_parse_table() {
            let md = "| A | B |\n|---|---|\n| 1 | 2 |";
            let result = parse(md).unwrap();
            let doc = result.value;
            let table = &doc.content.children[0];
            assert_eq!(table.kind.as_str(), node::TABLE);
        }

        #[test]
        fn test_parse_math() {
            let md = "Inline $x^2$ math and display $$y = mx + b$$ math.";
            let result = parse(md).unwrap();
            let doc = result.value;
            assert!(!doc.content.children.is_empty());
        }

        #[test]
        fn test_parse_metadata() {
            let md = "Title: My Document\nAuthor: Jane Doe\n\n# Heading\n";
            let result = parse(md).unwrap();
            let doc = result.value;
            assert_eq!(doc.metadata.get_str("Title"), Some("My Document"));
            assert_eq!(doc.content.children.len(), 1);
        }

        #[test]
        fn test_parse_citation() {
            let md = "See[p. 23][#Doe:2006] for details.\n\n[#Doe:2006]: John Doe.\n";
            let result = parse(md).unwrap();
            let doc = result.value;
            let para = &doc.content.children[0];
            assert!(
                para.children
                    .iter()
                    .any(|n| n.kind.as_str() == "mmd:citation")
            );
            assert_eq!(doc.content.children[1].kind.as_str(), "mmd:citation_def");
        }

        #[test]
        fn test_parse_cross_reference() {
            let md =
                "### Overview [MultiMarkdownOverview] ###\n\nSee [MultiMarkdownOverview] above.\n";
            let result = parse(md).unwrap();
            let doc = result.value;
            assert_eq!(
                doc.content.children[0].props.get_str(prop::ID),
                Some("MultiMarkdownOverview")
            );
            let para = &doc.content.children[1];
            assert!(
                para.children
                    .iter()
                    .any(|n| n.kind.as_str() == "mmd:cross_reference")
            );
        }
    }
}

#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
mod write {
    use crate::{
        MetadataEntry, MetadataStyle, MmdBlock, MmdDefinitionListItem, MmdDoc, MmdInline,
        MmdListItem, MmdTableCell, MmdTableRow, Span,
    };
    use rescribe_core::{
        ConversionResult, Document, EmitError, EmitOptions, FidelityWarning, Node, PropValue,
        Severity, WarningKind,
    };
    use rescribe_format_api::Emit as _;
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
        let mut footnotes = Vec::new();
        let mut blocks = nodes_to_blocks(&doc.content.children, &mut warnings, &mut footnotes);
        blocks.extend(footnotes);

        let mmd_doc = MmdDoc {
            metadata,
            metadata_style,
            blocks,
            link_defs: Vec::new(),
        };

        let bytes = mmd_doc.emit();
        Ok(ConversionResult::with_warnings(bytes, warnings))
    }

    fn unsupported(kind: &str, warnings: &mut Vec<FidelityWarning>) {
        warnings.push(FidelityWarning::new(
            Severity::Minor,
            WarningKind::UnsupportedNode(kind.to_string()),
            format!("node kind \"{kind}\" has no MultiMarkdown equivalent; dropped"),
        ));
    }

    fn nodes_to_blocks(
        nodes: &[Node],
        warnings: &mut Vec<FidelityWarning>,
        footnotes: &mut Vec<MmdBlock>,
    ) -> Vec<MmdBlock> {
        nodes
            .iter()
            .map(|n| node_to_block(n, warnings, footnotes))
            .collect()
    }

    fn node_to_block(
        n: &Node,
        warnings: &mut Vec<FidelityWarning>,
        footnotes: &mut Vec<MmdBlock>,
    ) -> MmdBlock {
        match n.kind.as_str() {
            "paragraph" => MmdBlock::Paragraph {
                inlines: nodes_to_inlines(&n.children, warnings, footnotes),
                span: Span::NONE,
            },
            "heading" => MmdBlock::Heading {
                level: n.props.get_int(prop::LEVEL).unwrap_or(1) as u8,
                inlines: nodes_to_inlines(&n.children, warnings, footnotes),
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
                blocks: nodes_to_blocks(&n.children, warnings, footnotes),
                span: Span::NONE,
            },
            "list" => {
                let ordered = n.props.get_bool(prop::ORDERED).unwrap_or(false);
                let items = n
                    .children
                    .iter()
                    .filter(|c| c.kind.as_str() == node::LIST_ITEM)
                    .map(|item| MmdListItem {
                        blocks: nodes_to_blocks(&item.children, warnings, footnotes),
                        span: Span::NONE,
                        checked: item.props.get_bool(prop::CHECKED),
                    })
                    .collect();
                MmdBlock::List {
                    kind: if ordered {
                        crate::ListKind::Ordered {
                            start: 1,
                            marker: crate::OrderedMarker::Period,
                        }
                    } else {
                        crate::ListKind::Unordered { marker: '-' }
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
                    .map(|r| table_row(r, warnings, footnotes))
                    .unwrap_or(MmdTableRow {
                        cells: Vec::new(),
                        span: Span::NONE,
                    });
                let rows = rows.map(|r| table_row(r, warnings, footnotes)).collect();
                MmdBlock::Table {
                    alignments: Vec::new(),
                    head,
                    rows,
                    span: Span::NONE,
                }
            }
            "footnote_def" => MmdBlock::FootnoteDefinition {
                label: n.props.get_str(prop::ID).unwrap_or("").to_string(),
                blocks: nodes_to_blocks(&n.children, warnings, footnotes),
                span: Span::NONE,
            },
            "definition_list" => {
                let mut items: Vec<MmdDefinitionListItem> = Vec::new();
                for child in &n.children {
                    match child.kind.as_str() {
                        "definition_term" => items.push(MmdDefinitionListItem {
                            term: nodes_to_inlines(&child.children, warnings, footnotes),
                            definitions: Vec::new(),
                            span: Span::NONE,
                        }),
                        "definition_desc" => {
                            if let Some(last) = items.last_mut() {
                                last.definitions.push(nodes_to_blocks(
                                    &child.children,
                                    warnings,
                                    footnotes,
                                ));
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
                content: nodes_to_inlines(&n.children, warnings, footnotes),
                span: Span::NONE,
            },
            other => {
                unsupported(other, warnings);
                MmdBlock::Paragraph {
                    inlines: nodes_to_inlines(&n.children, warnings, footnotes),
                    span: Span::NONE,
                }
            }
        }
    }

    fn table_row(
        n: &Node,
        warnings: &mut Vec<FidelityWarning>,
        footnotes: &mut Vec<MmdBlock>,
    ) -> MmdTableRow {
        MmdTableRow {
            cells: n
                .children
                .iter()
                .map(|c| MmdTableCell {
                    inlines: cell_inlines(&c.children, warnings, footnotes),
                    span: Span::NONE,
                })
                .collect(),
            span: Span::NONE,
        }
    }

    /// Flatten a table cell's children into inlines, unwrapping a `paragraph`
    /// wrapper (rescribe-fmt-pandoc-json wraps every table cell's block
    /// content — including single-`Plain`-block cells — in a `paragraph`
    /// node; this crate's own reader, and MultiMarkdown's cell model, treat
    /// a cell as inline-only content with no block wrapper). Without this,
    /// `nodes_to_inlines` would hit the `paragraph`-has-no-inline-mapping
    /// fallback and drop every cell's content.
    fn cell_inlines(
        nodes: &[Node],
        warnings: &mut Vec<FidelityWarning>,
        footnotes: &mut Vec<MmdBlock>,
    ) -> Vec<MmdInline> {
        let mut out = Vec::new();
        for n in nodes {
            if n.kind.as_str() == node::PARAGRAPH {
                out.extend(cell_inlines(&n.children, warnings, footnotes));
            } else {
                out.push(node_to_inline(n, warnings, footnotes));
            }
        }
        out
    }

    fn nodes_to_inlines(
        nodes: &[Node],
        warnings: &mut Vec<FidelityWarning>,
        footnotes: &mut Vec<MmdBlock>,
    ) -> Vec<MmdInline> {
        nodes
            .iter()
            .map(|n| node_to_inline(n, warnings, footnotes))
            .collect()
    }

    fn node_to_inline(
        n: &Node,
        warnings: &mut Vec<FidelityWarning>,
        footnotes: &mut Vec<MmdBlock>,
    ) -> MmdInline {
        match n.kind.as_str() {
            "text" => MmdInline::Text {
                content: n.props.get_str(prop::CONTENT).unwrap_or("").to_string(),
                span: Span::NONE,
            },
            "soft_break" => MmdInline::SoftBreak { span: Span::NONE },
            "line_break" => MmdInline::HardBreak { span: Span::NONE },
            "emphasis" => MmdInline::Emphasis {
                inlines: nodes_to_inlines(&n.children, warnings, footnotes),
                span: Span::NONE,
            },
            "strong" => MmdInline::Strong {
                inlines: nodes_to_inlines(&n.children, warnings, footnotes),
                span: Span::NONE,
            },
            "strikeout" => MmdInline::Strikethrough {
                inlines: nodes_to_inlines(&n.children, warnings, footnotes),
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
                inlines: nodes_to_inlines(&n.children, warnings, footnotes),
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
            // A `footnote_def` found in inline position (e.g. from
            // rescribe-fmt-pandoc-json, which nests `Note` content directly
            // where it occurs rather than splitting it into a separate
            // ref/def pair like this crate's own reader does) has no
            // MultiMarkdown-native inline representation — MMD footnotes are
            // always a `[^label]` reference plus a same-labeled block-level
            // definition. Rather than dropping the content (the previous
            // behavior, via the `other` fallback below), synthesize a
            // sequential label, emit a reference here, and queue the
            // definition to be appended as a top-level block by the caller.
            "footnote_def" => {
                let label = {
                    let existing = n.props.get_str(prop::ID).filter(|s| !s.is_empty());
                    existing
                        .map(str::to_string)
                        .unwrap_or_else(|| format!("fn{}", footnotes.len() + 1))
                };
                let blocks = nodes_to_blocks(&n.children, warnings, footnotes);
                footnotes.push(MmdBlock::FootnoteDefinition {
                    label: label.clone(),
                    blocks,
                    span: Span::NONE,
                });
                MmdInline::FootnoteReference {
                    label,
                    span: Span::NONE,
                }
            }
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
        use rescribe_format_api::Parse as _;

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
            let doc = Document::new().with_content(
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
            let doc =
                Document::new().with_content(
                    Node::new(NodeKind::from("document")).child(
                        Node::new(NodeKind::from("table"))
                            .child(
                                Node::new(NodeKind::from("table_row"))
                                    .child(Node::new(NodeKind::from("table_header")).child(
                                        Node::new(NodeKind::from("text")).prop("content", "A"),
                                    ))
                                    .child(Node::new(NodeKind::from("table_header")).child(
                                        Node::new(NodeKind::from("text")).prop("content", "B"),
                                    )),
                            )
                            .child(
                                Node::new(NodeKind::from("table_row"))
                                    .child(Node::new(NodeKind::from("table_cell")).child(
                                        Node::new(NodeKind::from("text")).prop("content", "1"),
                                    ))
                                    .child(Node::new(NodeKind::from("table_cell")).child(
                                        Node::new(NodeKind::from("text")).prop("content", "2"),
                                    )),
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
            let (parsed, diags) = MmdDoc::parse(output.as_bytes());
            assert!(diags.is_empty());
            assert!(matches!(
                &parsed.blocks[0],
                MmdBlock::Paragraph { inlines, .. }
                    if matches!(&inlines[0], MmdInline::Citation { label, .. } if label == "Doe:2006")
            ));
        }
    }
}

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
pub use read::{parse, parse_with_options};
#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
pub use write::{emit, emit_with_options};
