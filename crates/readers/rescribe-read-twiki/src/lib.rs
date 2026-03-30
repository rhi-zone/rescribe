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
    let (twiki_doc, _diags) = twiki::parse(input);
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
        Block::Paragraph { inlines, .. } => {
            Node::new(node::PARAGRAPH).children(inlines_to_nodes(inlines))
        }
        Block::Heading { level, inlines, .. } => Node::new(node::HEADING)
            .prop(prop::LEVEL, *level as i64)
            .children(inlines_to_nodes(inlines)),
        Block::CodeBlock { content, .. } => {
            Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone())
        }
        Block::List { ordered, items, .. } => {
            let list_items: Vec<Node> = items
                .iter()
                .map(|item| {
                    let mut item_node = Node::new(node::LIST_ITEM).child(
                        Node::new(node::PARAGRAPH).children(inlines_to_nodes(&item.inlines)),
                    );
                    for child_block in &item.children {
                        item_node = item_node.child(block_to_node(child_block));
                    }
                    item_node
                })
                .collect();
            Node::new(node::LIST)
                .prop(prop::ORDERED, *ordered)
                .children(list_items)
        }
        Block::RawBlock { content, .. } => {
            Node::new(node::RAW_BLOCK)
                .prop(prop::FORMAT, "twiki")
                .prop(prop::CONTENT, content.clone())
        }
        Block::DefinitionList { items, .. } => {
            let def_nodes: Vec<Node> = items
                .iter()
                .map(|item| {
                    Node::new("definition_item")
                        .child(
                            Node::new(node::DEFINITION_TERM)
                                .children(inlines_to_nodes(&item.term)),
                        )
                        .child(
                            Node::new(node::DEFINITION_DESC)
                                .children(inlines_to_nodes(&item.desc)),
                        )
                })
                .collect();
            Node::new(node::DEFINITION_LIST).children(def_nodes)
        }

        Block::Blockquote { children, .. } => {
            let block_nodes: Vec<_> = children.iter().map(block_to_node).collect();
            Node::new(node::BLOCKQUOTE).children(block_nodes)
        }
        Block::Table { rows, .. } => {
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
        Block::HorizontalRule { .. } => Node::new(node::HORIZONTAL_RULE),
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
        Inline::Text(s, _) => Node::new(node::TEXT).prop(prop::CONTENT, s.clone()),
        Inline::Bold(children, _) => Node::new(node::STRONG).children(inlines_to_nodes(children)),
        Inline::Italic(children, _) => Node::new(node::EMPHASIS).children(inlines_to_nodes(children)),
        Inline::BoldItalic(children, _) => Node::new(node::STRONG)
            .child(Node::new(node::EMPHASIS).children(inlines_to_nodes(children))),
        Inline::Code(s, _) => Node::new(node::CODE).prop(prop::CONTENT, s.clone()),
        Inline::BoldCode(children, _) => Node::new(node::STRONG)
            .child(Node::new(node::CODE).prop(prop::CONTENT, children_to_text(children))),
        Inline::Link { url, label, .. } => Node::new(node::LINK)
            .prop(prop::URL, url.clone())
            .child(Node::new(node::TEXT).prop(prop::CONTENT, label.clone())),
        Inline::Strikethrough(children, _) => {
            Node::new("strikethrough").children(inlines_to_nodes(children))
        }
        Inline::Superscript(children, _) => {
            Node::new(node::SUPERSCRIPT).children(inlines_to_nodes(children))
        }
        Inline::Subscript(children, _) => {
            Node::new(node::SUBSCRIPT).children(inlines_to_nodes(children))
        }
        Inline::Underline(children, _) => {
            Node::new(node::UNDERLINE).children(inlines_to_nodes(children))
        }
        Inline::Image { url, alt, .. } => {
            let mut n = Node::new(node::IMAGE).prop(prop::URL, url.clone());
            if !alt.is_empty() {
                n = n.prop(prop::ALT, alt.clone());
            }
            n
        }
        Inline::RawInline { content, .. } => Node::new(node::RAW_INLINE)
            .prop(prop::FORMAT, "twiki")
            .prop(prop::CONTENT, content.clone()),
        Inline::WikiWord { word, .. } => Node::new("wikiword").prop("word", word.clone()),
        Inline::LineBreak { .. } => Node::new(node::LINE_BREAK),
    }
}

fn children_to_text(children: &[Inline]) -> String {
    let mut s = String::new();
    for child in children {
        match child {
            Inline::Text(t, _) => s.push_str(t),
            Inline::Bold(ch, _)
            | Inline::Italic(ch, _)
            | Inline::BoldItalic(ch, _)
            | Inline::Strikethrough(ch, _)
            | Inline::Superscript(ch, _)
            | Inline::Subscript(ch, _)
            | Inline::Underline(ch, _) => {
                s.push_str(&children_to_text(ch));
            }
            Inline::Code(c, _) => s.push_str(c),
            Inline::BoldCode(ch, _) => s.push_str(&children_to_text(ch)),
            Inline::Link { label, .. } => s.push_str(label),
            Inline::LineBreak { .. } => s.push('\n'),
            Inline::Image { alt, .. } => s.push_str(alt),
            Inline::RawInline { content, .. } => s.push_str(content),
            Inline::WikiWord { word, .. } => s.push_str(word),
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
