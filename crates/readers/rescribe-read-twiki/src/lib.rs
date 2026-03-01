//! TWiki reader for rescribe.
//!
//! Thin adapter layer that parses TWiki markup into rescribe's document IR.

use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions};
use rescribe_std::{node, prop};
use twiki::{self, Block, Inline};

/// Parse TWiki markup into a document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse TWiki markup with options.
pub fn parse_with_options(
    input: &str,
    _options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let twiki_doc = twiki::parse(input).map_err(|e| ParseError::Invalid(e.0))?;
    let mut result = Vec::new();

    for block in twiki_doc.blocks {
        result.push(block_to_node(&block));
    }

    let document = Document {
        content: Node::new(node::DOCUMENT).children(result),
        resources: Default::default(),
        metadata: Default::default(),
        source: None,
    };

    Ok(ConversionResult::ok(document))
}

fn block_to_node(block: &Block) -> Node {
    match block {
        Block::Paragraph { inlines } => {
            Node::new(node::PARAGRAPH).children(inlines_to_nodes(inlines))
        }
        Block::Heading { level, inlines } => Node::new(node::HEADING)
            .prop(prop::LEVEL, *level as i64)
            .children(inlines_to_nodes(inlines)),
        Block::CodeBlock { content } => {
            Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone())
        }
        Block::List { ordered, items } => {
            let list_items: Vec<Node> = items
                .iter()
                .map(|item_inlines| {
                    Node::new(node::LIST_ITEM)
                        .child(Node::new(node::PARAGRAPH).children(inlines_to_nodes(item_inlines)))
                })
                .collect();
            Node::new(node::LIST)
                .prop(prop::ORDERED, *ordered)
                .children(list_items)
        }
        Block::Table { rows } => {
            let row_nodes: Vec<Node> = rows
                .iter()
                .map(|row| {
                    let cells: Vec<Node> = row
                        .cells
                        .iter()
                        .map(|cell| {
                            if cell.is_header {
                                Node::new(node::TABLE_HEADER)
                                    .children(inlines_to_nodes(&cell.inlines))
                            } else {
                                Node::new(node::TABLE_CELL)
                                    .children(inlines_to_nodes(&cell.inlines))
                            }
                        })
                        .collect();
                    Node::new(node::TABLE_ROW).children(cells)
                })
                .collect();
            Node::new(node::TABLE).children(row_nodes)
        }
        Block::HorizontalRule => Node::new(node::HORIZONTAL_RULE),
    }
}

fn inlines_to_nodes(inlines: &[Inline]) -> Vec<Node> {
    let mut nodes = Vec::new();
    for inline in inlines {
        nodes.push(inline_to_node(inline));
    }
    nodes
}

fn inline_to_node(inline: &Inline) -> Node {
    match inline {
        Inline::Text(s) => Node::new(node::TEXT).prop(prop::CONTENT, s.clone()),
        Inline::Bold(children) => Node::new(node::STRONG).children(inlines_to_nodes(children)),
        Inline::Italic(children) => Node::new(node::EMPHASIS).children(inlines_to_nodes(children)),
        Inline::BoldItalic(children) => Node::new(node::STRONG)
            .child(Node::new(node::EMPHASIS).children(inlines_to_nodes(children))),
        Inline::Code(s) => Node::new(node::CODE).prop(prop::CONTENT, s.clone()),
        Inline::BoldCode(children) => Node::new(node::STRONG)
            .child(Node::new(node::CODE).prop(prop::CONTENT, children_to_text(children))),
        Inline::Link { url, label } => Node::new(node::LINK)
            .prop(prop::URL, url.clone())
            .child(Node::new(node::TEXT).prop(prop::CONTENT, label.clone())),
        Inline::LineBreak => Node::new(node::LINE_BREAK),
    }
}

fn children_to_text(children: &[Inline]) -> String {
    let mut s = String::new();
    for child in children {
        match child {
            Inline::Text(t) => s.push_str(t),
            Inline::Bold(ch) | Inline::Italic(ch) | Inline::BoldItalic(ch) => {
                s.push_str(&children_to_text(ch));
            }
            Inline::Code(c) => s.push_str(c),
            Inline::BoldCode(ch) => s.push_str(&children_to_text(ch)),
            Inline::Link { label, .. } => s.push_str(label),
            Inline::LineBreak => s.push('\n'),
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_heading() {
        let result = parse("---+ Heading 1\n---++ Heading 2").unwrap();
        assert_eq!(result.value.content.children.len(), 2);
    }

    #[test]
    fn test_parse_bold() {
        let result = parse("This is *bold* text").unwrap();
        assert!(!result.value.content.children.is_empty());
    }

    #[test]
    fn test_parse_table() {
        let result = parse("| A | B |\n| C | D |").unwrap();
        assert_eq!(result.value.content.children.len(), 1);
        assert_eq!(result.value.content.children[0].kind.as_str(), node::TABLE);
    }
}
