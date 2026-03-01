//! TikiWiki writer for rescribe.
//!
//! Thin adapter converting rescribe's document IR to tikiwiki AST.

use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node};
use rescribe_std::{node, prop};
use tikiwiki::{Block as TwBlock, Inline as TwInline};

/// Emit a document to TikiWiki markup.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    emit_with_options(doc, &EmitOptions::default())
}

/// Emit a document to TikiWiki markup with options.
pub fn emit_with_options(
    doc: &Document,
    _options: &EmitOptions,
) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let mut blocks = Vec::new();
    for node in &doc.content.children {
        if let Some(block) = node_to_block(node) {
            blocks.push(block);
        }
    }

    let tw_doc = tikiwiki::TikiwikiDoc { blocks };
    let output = tikiwiki::build(&tw_doc);
    Ok(ConversionResult::ok(output.into_bytes()))
}

fn node_to_block(node: &Node) -> Option<TwBlock> {
    match node.kind.as_str() {
        node::DOCUMENT => None,

        node::HEADING => {
            let level = node.props.get_int(prop::LEVEL).unwrap_or(1) as u8;
            let inlines = nodes_to_inlines(&node.children);
            Some(TwBlock::Heading { level, inlines })
        }

        node::PARAGRAPH => {
            let inlines = nodes_to_inlines(&node.children);
            Some(TwBlock::Paragraph { inlines })
        }

        node::CODE_BLOCK => {
            let content = node
                .props
                .get_str(prop::CONTENT)
                .unwrap_or_default()
                .to_string();
            let language = node.props.get_str(prop::LANGUAGE).map(|s| s.to_string());
            Some(TwBlock::CodeBlock { content, language })
        }

        node::BLOCKQUOTE => {
            let inlines = nodes_to_inlines(&node.children);
            Some(TwBlock::Blockquote { inlines })
        }

        node::LIST => {
            let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
            let mut items = Vec::new();
            for child in &node.children {
                if child.kind.as_str() == node::LIST_ITEM {
                    let inlines = nodes_to_inlines(&child.children);
                    items.push(inlines);
                }
            }
            Some(TwBlock::List { ordered, items })
        }

        node::TABLE => {
            let mut rows = Vec::new();
            for row_node in &node.children {
                if row_node.kind.as_str() == node::TABLE_ROW {
                    let mut cells = Vec::new();
                    for cell_node in &row_node.children {
                        if cell_node.kind.as_str() == node::TABLE_CELL {
                            let inlines = nodes_to_inlines(&cell_node.children);
                            cells.push(inlines);
                        }
                    }
                    rows.push(tikiwiki::TableRow { cells });
                }
            }
            Some(TwBlock::Table { rows })
        }

        node::HORIZONTAL_RULE => Some(TwBlock::HorizontalRule),

        node::DIV | node::SPAN | node::FIGURE => {
            // For containers, extract first block child if any
            for child in &node.children {
                if let Some(block) = node_to_block(child) {
                    return Some(block);
                }
            }
            None
        }

        _ => None,
    }
}

fn nodes_to_inlines(nodes: &[Node]) -> Vec<TwInline> {
    nodes.iter().filter_map(node_to_inline).collect()
}

fn node_to_inline(node: &Node) -> Option<TwInline> {
    match node.kind.as_str() {
        node::TEXT => {
            let content = node
                .props
                .get_str(prop::CONTENT)
                .unwrap_or_default()
                .to_string();
            if !content.is_empty() {
                Some(TwInline::Text(content))
            } else {
                None
            }
        }

        node::STRONG => {
            let children = nodes_to_inlines(&node.children);
            if !children.is_empty() {
                Some(TwInline::Bold(children))
            } else {
                None
            }
        }

        node::EMPHASIS => {
            let children = nodes_to_inlines(&node.children);
            if !children.is_empty() {
                Some(TwInline::Italic(children))
            } else {
                None
            }
        }

        node::UNDERLINE => {
            let children = nodes_to_inlines(&node.children);
            if !children.is_empty() {
                Some(TwInline::Underline(children))
            } else {
                None
            }
        }

        node::STRIKEOUT => {
            let children = nodes_to_inlines(&node.children);
            if !children.is_empty() {
                Some(TwInline::Strikethrough(children))
            } else {
                None
            }
        }

        node::CODE => {
            let content = node
                .props
                .get_str(prop::CONTENT)
                .unwrap_or_default()
                .to_string();
            if !content.is_empty() {
                Some(TwInline::Code(content))
            } else {
                None
            }
        }

        node::LINK => {
            let url = node
                .props
                .get_str(prop::URL)
                .unwrap_or_default()
                .to_string();
            let children = nodes_to_inlines(&node.children);
            Some(TwInline::Link { url, children })
        }

        node::IMAGE => {
            let url = node
                .props
                .get_str(prop::URL)
                .unwrap_or_default()
                .to_string();
            let alt = node
                .props
                .get_str(prop::ALT)
                .unwrap_or_default()
                .to_string();
            Some(TwInline::Image { url, alt })
        }

        node::LINE_BREAK => Some(TwInline::LineBreak),

        node::SOFT_BREAK => Some(TwInline::Text(" ".to_string())),

        _ => {
            let children = nodes_to_inlines(&node.children);
            if !children.is_empty() {
                // Return first inline from children
                children.into_iter().next()
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rescribe_core::Properties;
    use rescribe_std::builder::*;

    fn emit_str(doc: &Document) -> String {
        String::from_utf8(emit(doc).unwrap().value).unwrap()
    }

    #[test]
    fn test_emit_heading() {
        let doc = doc(|d| d.heading(1, |h| h.text("Title")));
        assert!(emit_str(&doc).contains("!Title"));
    }

    #[test]
    fn test_emit_heading_level2() {
        let doc = doc(|d| d.heading(2, |h| h.text("Section")));
        assert!(emit_str(&doc).contains("!!Section"));
    }

    #[test]
    fn test_emit_bold() {
        let doc = doc(|d| d.para(|p| p.strong(|s| s.text("bold"))));
        assert!(emit_str(&doc).contains("__bold__"));
    }

    #[test]
    fn test_emit_italic() {
        let doc = doc(|d| d.para(|p| p.em(|e| e.text("italic"))));
        assert!(emit_str(&doc).contains("''italic''"));
    }

    #[test]
    fn test_emit_link() {
        let doc = doc(|d| d.para(|p| p.link("http://example.com", |l| l.text("Example"))));
        assert!(emit_str(&doc).contains("[http://example.com|Example]"));
    }

    #[test]
    fn test_emit_list() {
        let doc = doc(|d| d.bullet_list(|l| l.item(|i| i.text("one")).item(|i| i.text("two"))));
        let output = emit_str(&doc);
        assert!(output.contains("*one"));
        assert!(output.contains("*two"));
    }

    #[test]
    fn test_emit_table() {
        let doc = Document {
            content: Node::new(node::DOCUMENT).child(
                Node::new(node::TABLE).child(
                    Node::new(node::TABLE_ROW)
                        .child(
                            Node::new(node::TABLE_CELL)
                                .child(Node::new(node::TEXT).prop(prop::CONTENT, "A")),
                        )
                        .child(
                            Node::new(node::TABLE_CELL)
                                .child(Node::new(node::TEXT).prop(prop::CONTENT, "B")),
                        ),
                ),
            ),
            resources: Default::default(),
            metadata: Properties::new(),
            source: None,
        };
        assert!(emit_str(&doc).contains("||A|B||"));
    }
}
