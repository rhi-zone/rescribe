//! XWiki reader for rescribe.
//!
//! Thin adapter layer that converts XWiki AST to rescribe's document IR.

use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions};
use rescribe_std::{node, prop};
use xwiki::{self, Block, Inline};

/// Parse XWiki markup into a document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse XWiki markup with options.
pub fn parse_with_options(
    input: &str,
    _options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let doc = xwiki::parse(input).map_err(|e| ParseError::Invalid(e.to_string()))?;

    let mut result = Vec::new();
    for block in &doc.blocks {
        result.push(block_to_node(block));
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
        Block::Heading { level, inlines } => Node::new(node::HEADING)
            .prop(prop::LEVEL, *level as i64)
            .children(inlines_to_nodes(inlines)),

        Block::Paragraph { inlines } => {
            Node::new(node::PARAGRAPH).children(inlines_to_nodes(inlines))
        }

        Block::CodeBlock { content, language } => {
            let mut node = Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone());
            if let Some(lang) = language {
                node = node.prop(prop::LANGUAGE, lang.clone());
            }
            node
        }

        Block::Table { rows } => {
            let table_rows: Vec<Node> = rows
                .iter()
                .map(|row| {
                    let cells: Vec<Node> = row
                        .cells
                        .iter()
                        .map(|cell| {
                            let kind = if cell.is_header {
                                node::TABLE_HEADER
                            } else {
                                node::TABLE_CELL
                            };
                            Node::new(kind).children(inlines_to_nodes(&cell.inlines))
                        })
                        .collect();
                    Node::new(node::TABLE_ROW).children(cells)
                })
                .collect();
            Node::new(node::TABLE).children(table_rows)
        }

        Block::List { ordered, items } => {
            let list_items: Vec<Node> = items
                .iter()
                .map(|item_blocks| {
                    let children: Vec<Node> = item_blocks.iter().map(block_to_node).collect();
                    Node::new(node::LIST_ITEM).children(children)
                })
                .collect();
            Node::new(node::LIST)
                .prop(prop::ORDERED, *ordered)
                .children(list_items)
        }

        Block::HorizontalRule => Node::new(node::HORIZONTAL_RULE),
    }
}

fn inlines_to_nodes(inlines: &[Inline]) -> Vec<Node> {
    inlines.iter().map(inline_to_node).collect()
}

fn inline_to_node(inline: &Inline) -> Node {
    match inline {
        Inline::Text(s) => Node::new(node::TEXT).prop(prop::CONTENT, s.clone()),

        Inline::Bold(children) => Node::new(node::STRONG).children(inlines_to_nodes(children)),

        Inline::Italic(children) => Node::new(node::EMPHASIS).children(inlines_to_nodes(children)),

        Inline::Underline(children) => {
            Node::new(node::UNDERLINE).children(inlines_to_nodes(children))
        }

        Inline::Strikeout(children) => {
            Node::new(node::STRIKEOUT).children(inlines_to_nodes(children))
        }

        Inline::Code(s) => Node::new(node::CODE).prop(prop::CONTENT, s.clone()),

        Inline::Link { url, label } => Node::new(node::LINK)
            .prop(prop::URL, url.clone())
            .child(Node::new(node::TEXT).prop(prop::CONTENT, label.clone())),

        Inline::Image { url } => Node::new(node::IMAGE).prop(prop::URL, url.clone()),

        Inline::LineBreak => Node::new(node::LINE_BREAK),

        Inline::SoftBreak => Node::new(node::SOFT_BREAK),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_heading() {
        let result = parse("= Heading 1 =\n== Heading 2 ==").unwrap();
        assert_eq!(result.value.content.children.len(), 2);
    }

    #[test]
    fn test_parse_bold() {
        let result = parse("This is **bold** text").unwrap();
        assert!(!result.value.content.children.is_empty());
    }

    #[test]
    fn test_parse_italic() {
        let result = parse("This is //italic// text").unwrap();
        assert!(!result.value.content.children.is_empty());
    }

    #[test]
    fn test_parse_link() {
        let result = parse("[[Example>>http://example.com]]").unwrap();
        assert!(!result.value.content.children.is_empty());
    }

    #[test]
    fn test_parse_list() {
        let result = parse("* Item 1\n* Item 2").unwrap();
        assert_eq!(result.value.content.children.len(), 1);
    }
}
