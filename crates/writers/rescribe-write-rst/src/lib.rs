//! reStructuredText writer for rescribe.
//!
//! Thin adapter over [`rst_fmt`]: maps the rescribe document model to
//! the `rst_fmt` AST, then builds RST output.

use rescribe_core::{
    ConversionResult, Document, EmitError, EmitOptions, FidelityWarning, Node, Severity,
    WarningKind,
};
use rescribe_std::{node, prop};
use rst_fmt::{Block, DefinitionItem, Inline, RstDoc, TableRow};

/// Emit a document as RST.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    emit_with_options(doc, &EmitOptions::default())
}

/// Emit a document as RST with custom options.
pub fn emit_with_options(
    doc: &Document,
    _options: &EmitOptions,
) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let mut warnings = Vec::new();
    let rst = doc_to_rst(doc, &mut warnings);
    let output = rst_fmt::build(&rst);
    Ok(ConversionResult::with_warnings(
        output.into_bytes(),
        warnings,
    ))
}

fn doc_to_rst(doc: &Document, warnings: &mut Vec<FidelityWarning>) -> RstDoc {
    RstDoc {
        blocks: nodes_to_blocks(&doc.content.children, warnings),
    }
}

fn nodes_to_blocks(nodes: &[Node], warnings: &mut Vec<FidelityWarning>) -> Vec<Block> {
    nodes
        .iter()
        .flat_map(|n| node_to_blocks(n, warnings))
        .collect()
}

fn node_to_blocks(node: &Node, warnings: &mut Vec<FidelityWarning>) -> Vec<Block> {
    match node.kind.as_str() {
        node::DOCUMENT => nodes_to_blocks(&node.children, warnings),

        node::PARAGRAPH => vec![Block::Paragraph {
            inlines: nodes_to_inlines(&node.children, warnings),
        }],

        node::HEADING => {
            let level = node.props.get_int(prop::LEVEL).unwrap_or(1);
            vec![Block::Heading {
                level,
                inlines: nodes_to_inlines(&node.children, warnings),
            }]
        }

        node::CODE_BLOCK => {
            let language = node.props.get_str(prop::LANGUAGE).map(|s| s.to_string());
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            vec![Block::CodeBlock { language, content }]
        }

        node::BLOCKQUOTE => vec![Block::Blockquote {
            children: nodes_to_blocks(&node.children, warnings),
        }],

        node::LIST => {
            let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
            let items: Vec<Vec<Block>> = node
                .children
                .iter()
                .filter(|c| c.kind.as_str() == node::LIST_ITEM)
                .map(|item| nodes_to_blocks(&item.children, warnings))
                .collect();
            vec![Block::List { ordered, items }]
        }

        node::LIST_ITEM => nodes_to_blocks(&node.children, warnings),

        node::DEFINITION_LIST => {
            let items = definition_list_to_items(&node.children, warnings);
            vec![Block::DefinitionList { items }]
        }

        node::FIGURE => {
            let img = node
                .children
                .iter()
                .find(|c| c.kind.as_str() == node::IMAGE);
            if let Some(img_node) = img {
                let url = img_node.props.get_str(prop::URL).unwrap_or("").to_string();
                let alt = img_node.props.get_str(prop::ALT).map(|s| s.to_string());
                let caption_node = node
                    .children
                    .iter()
                    .find(|c| c.kind.as_str() == node::CAPTION);
                let caption = caption_node.map(|cap| nodes_to_inlines(&cap.children, warnings));
                vec![Block::Figure { url, alt, caption }]
            } else {
                vec![]
            }
        }

        node::IMAGE => {
            let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
            let alt = node.props.get_str(prop::ALT).map(|s| s.to_string());
            let title = node.props.get_str(prop::TITLE).map(|s| s.to_string());
            vec![Block::Image { url, alt, title }]
        }

        node::TABLE => {
            let rows = collect_table_rows(node, warnings);
            vec![Block::Table { rows }]
        }

        node::HORIZONTAL_RULE => vec![Block::HorizontalRule],

        node::DIV | node::SPAN => nodes_to_blocks(&node.children, warnings),

        node::RAW_BLOCK | node::RAW_INLINE => {
            let format = node.props.get_str(prop::FORMAT).unwrap_or("").to_string();
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            vec![Block::RawBlock { format, content }]
        }

        node::DEFINITION_TERM | node::DEFINITION_DESC => {
            // These are handled inside DEFINITION_LIST
            vec![]
        }

        node::FOOTNOTE_DEF => {
            let label = node.props.get_str(prop::LABEL).unwrap_or("").to_string();
            let inlines = nodes_to_inlines(&node.children, warnings);
            vec![Block::FootnoteDef { label, inlines }]
        }

        "math_display" => {
            let source = node.props.get_str("math:source").unwrap_or("").to_string();
            vec![Block::MathDisplay { source }]
        }

        "admonition" => {
            let admonition_type = node
                .props
                .get_str("admonition_type")
                .unwrap_or("note")
                .to_lowercase();
            let children = nodes_to_blocks(&node.children, warnings);
            vec![Block::Admonition {
                admonition_type,
                children,
            }]
        }

        // Inline nodes at block level: wrap in a paragraph
        node::TEXT | node::STRONG | node::EMPHASIS | node::CODE | node::LINK => {
            vec![Block::Paragraph {
                inlines: nodes_to_inlines(std::slice::from_ref(node), warnings),
            }]
        }

        _ => {
            warnings.push(FidelityWarning::new(
                Severity::Minor,
                WarningKind::UnsupportedNode(node.kind.as_str().to_string()),
                format!("Unknown node type for RST: {}", node.kind.as_str()),
            ));
            nodes_to_blocks(&node.children, warnings)
        }
    }
}

fn definition_list_to_items(
    nodes: &[Node],
    warnings: &mut Vec<FidelityWarning>,
) -> Vec<DefinitionItem> {
    let mut items = Vec::new();
    let mut i = 0;
    while i < nodes.len() {
        if nodes[i].kind.as_str() == node::DEFINITION_TERM {
            let term = nodes_to_inlines(&nodes[i].children, warnings);
            let desc = if i + 1 < nodes.len() && nodes[i + 1].kind.as_str() == node::DEFINITION_DESC
            {
                let d = nodes_to_inlines(&nodes[i + 1].children, warnings);
                i += 1;
                d
            } else {
                vec![]
            };
            items.push(DefinitionItem { term, desc });
        }
        i += 1;
    }
    items
}

fn collect_table_rows(node: &Node, warnings: &mut Vec<FidelityWarning>) -> Vec<TableRow> {
    let mut rows = Vec::new();
    collect_table_rows_inner(&node.children, &mut rows, warnings);
    rows
}

fn collect_table_rows_inner(
    nodes: &[Node],
    rows: &mut Vec<TableRow>,
    warnings: &mut Vec<FidelityWarning>,
) {
    for n in nodes {
        match n.kind.as_str() {
            node::TABLE_HEAD | node::TABLE_BODY | node::TABLE_FOOT => {
                collect_table_rows_inner(&n.children, rows, warnings);
            }
            node::TABLE_ROW => {
                let cells: Vec<Vec<Inline>> = n
                    .children
                    .iter()
                    .map(|cell| nodes_to_inlines(&cell.children, warnings))
                    .collect();
                rows.push(TableRow {
                    cells,
                    is_header: false,
                });
            }
            node::TABLE_HEADER => {
                let cells: Vec<Vec<Inline>> = n
                    .children
                    .iter()
                    .map(|cell| nodes_to_inlines(&cell.children, warnings))
                    .collect();
                rows.push(TableRow {
                    cells,
                    is_header: true,
                });
            }
            _ => {}
        }
    }
}

fn nodes_to_inlines(nodes: &[Node], warnings: &mut Vec<FidelityWarning>) -> Vec<Inline> {
    nodes.iter().map(|n| node_to_inline(n, warnings)).collect()
}

fn node_to_inline(node: &Node, warnings: &mut Vec<FidelityWarning>) -> Inline {
    match node.kind.as_str() {
        node::TEXT => {
            let s = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            Inline::Text(s)
        }

        node::EMPHASIS => Inline::Emphasis(nodes_to_inlines(&node.children, warnings)),

        node::STRONG => Inline::Strong(nodes_to_inlines(&node.children, warnings)),

        node::STRIKEOUT => Inline::Strikeout(nodes_to_inlines(&node.children, warnings)),

        node::UNDERLINE => Inline::Underline(nodes_to_inlines(&node.children, warnings)),

        node::SUBSCRIPT => Inline::Subscript(nodes_to_inlines(&node.children, warnings)),

        node::SUPERSCRIPT => Inline::Superscript(nodes_to_inlines(&node.children, warnings)),

        node::CODE => {
            let s = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            Inline::Code(s)
        }

        node::LINK => {
            let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
            Inline::Link {
                url,
                children: nodes_to_inlines(&node.children, warnings),
            }
        }

        node::IMAGE => {
            let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
            let alt = node.props.get_str(prop::ALT).unwrap_or("").to_string();
            Inline::Image { url, alt }
        }

        node::LINE_BREAK => Inline::LineBreak,

        node::SOFT_BREAK => Inline::SoftBreak,

        node::FOOTNOTE_REF => {
            let label = node.props.get_str(prop::LABEL).unwrap_or("").to_string();
            Inline::FootnoteRef { label }
        }

        node::FOOTNOTE_DEF => {
            let label = node.props.get_str(prop::LABEL).unwrap_or("").to_string();
            Inline::FootnoteDef {
                label,
                children: nodes_to_inlines(&node.children, warnings),
            }
        }

        node::SMALL_CAPS => Inline::SmallCaps(nodes_to_inlines(&node.children, warnings)),

        node::QUOTED => {
            let quote_type = node
                .props
                .get_str(prop::QUOTE_TYPE)
                .unwrap_or("double")
                .to_string();
            Inline::Quoted {
                quote_type,
                children: nodes_to_inlines(&node.children, warnings),
            }
        }

        node::SPAN => {
            let role = node.props.get_str("rst:role").unwrap_or("span").to_string();
            Inline::RstSpan {
                role,
                children: nodes_to_inlines(&node.children, warnings),
            }
        }

        node::RAW_INLINE => {
            let format = node.props.get_str(prop::FORMAT).unwrap_or("");
            let content = node.props.get_str(prop::CONTENT).unwrap_or("");
            if format == "rst" {
                Inline::Text(content.to_string())
            } else {
                Inline::Text(String::new())
            }
        }

        "math_inline" => {
            let source = node.props.get_str("math:source").unwrap_or("").to_string();
            Inline::MathInline { source }
        }

        _ => {
            // Unknown inline: recurse into children
            let children = nodes_to_inlines(&node.children, warnings);
            if children.is_empty() {
                Inline::Text(String::new())
            } else if children.len() == 1 {
                children.into_iter().next().unwrap()
            } else {
                Inline::Strong(children)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rescribe_std::builder::*;

    fn emit_str(doc: &Document) -> String {
        let result = emit(doc).unwrap();
        String::from_utf8(result.value).unwrap()
    }

    #[test]
    fn test_emit_paragraph() {
        let doc = doc(|d| d.para(|p| p.text("Hello, world!")));
        let output = emit_str(&doc);
        assert!(output.contains("Hello, world!"));
    }

    #[test]
    fn test_emit_heading() {
        let doc = doc(|d| d.heading(1, |h| h.text("Title")));
        let output = emit_str(&doc);
        assert!(output.contains("====="));
        assert!(output.contains("Title"));
    }

    #[test]
    fn test_emit_heading_level2() {
        let doc = doc(|d| d.heading(2, |h| h.text("Subtitle")));
        let output = emit_str(&doc);
        assert!(output.contains("--------"));
        assert!(output.contains("Subtitle"));
    }

    #[test]
    fn test_emit_emphasis() {
        let doc = doc(|d| d.para(|p| p.em(|e| e.text("italic"))));
        let output = emit_str(&doc);
        assert!(output.contains("*italic*"));
    }

    #[test]
    fn test_emit_strong() {
        let doc = doc(|d| d.para(|p| p.strong(|s| s.text("bold"))));
        let output = emit_str(&doc);
        assert!(output.contains("**bold**"));
    }

    #[test]
    fn test_emit_code() {
        let doc = doc(|d| d.para(|p| p.code("code")));
        let output = emit_str(&doc);
        assert!(output.contains("``code``"));
    }

    #[test]
    fn test_emit_link() {
        let doc = doc(|d| d.para(|p| p.link("https://example.com", |l| l.text("click"))));
        let output = emit_str(&doc);
        assert!(output.contains("`click <https://example.com>`_"));
    }

    #[test]
    fn test_emit_code_block() {
        let doc = doc(|d| d.code_block_lang("print('hi')", "python"));
        let output = emit_str(&doc);
        assert!(output.contains(".. code-block:: python"));
        assert!(output.contains("   print('hi')"));
    }

    #[test]
    fn test_emit_list() {
        let doc = doc(|d| d.bullet_list(|l| l.item(|i| i.text("one")).item(|i| i.text("two"))));
        let output = emit_str(&doc);
        assert!(output.contains("- one"));
        assert!(output.contains("- two"));
    }
}
