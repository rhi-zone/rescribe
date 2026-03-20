//! Textile markup reader for rescribe.
//!
//! Thin adapter converting textile-fmt AST to rescribe document model.

use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions};
use rescribe_std::{node, prop};
use textile_fmt::{Block, Inline, parse as parse_textile};

/// Parse Textile markup.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse Textile markup with custom options.
pub fn parse_with_options(
    input: &str,
    _options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let (doc, _diags) = parse_textile(input);

    let blocks = doc.blocks.iter().map(convert_block).collect::<Vec<_>>();

    let root = Node::new(node::DOCUMENT).children(blocks);
    let document = Document::new().with_content(root);

    Ok(ConversionResult::ok(document))
}

fn convert_block(block: &Block) -> Node {
    match block {
        Block::Paragraph { inlines, .. } => {
            let children: Vec<Node> = inlines.iter().map(convert_inline).collect();
            Node::new(node::PARAGRAPH).children(children)
        }

        Block::Heading { level, inlines, .. } => {
            let children: Vec<Node> = inlines.iter().map(convert_inline).collect();
            Node::new(node::HEADING)
                .prop(prop::LEVEL, *level as i64)
                .children(children)
        }

        Block::CodeBlock { content, .. } => {
            Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone())
        }

        Block::Blockquote { inlines, .. } => {
            let para_children: Vec<Node> = inlines.iter().map(convert_inline).collect();
            let para = Node::new(node::PARAGRAPH).children(para_children);
            Node::new(node::BLOCKQUOTE).children(vec![para])
        }

        Block::List { ordered, items, .. } => {
            let list_items: Vec<Node> = items
                .iter()
                .map(|item_blocks| {
                    let children: Vec<Node> = item_blocks.iter().map(convert_block).collect();
                    Node::new(node::LIST_ITEM).children(children)
                })
                .collect();
            Node::new(node::LIST)
                .prop(prop::ORDERED, *ordered)
                .children(list_items)
        }

        Block::Table { rows, .. } => {
            let table_rows: Vec<Node> = rows
                .iter()
                .map(|row| {
                    let cells: Vec<Node> = row
                        .cells
                        .iter()
                        .map(|cell| {
                            let children: Vec<Node> =
                                cell.inlines.iter().map(convert_inline).collect();
                            let kind = if cell.is_header {
                                node::TABLE_HEADER
                            } else {
                                node::TABLE_CELL
                            };
                            Node::new(kind).children(children)
                        })
                        .collect();
                    Node::new(node::TABLE_ROW).children(cells)
                })
                .collect();
            Node::new(node::TABLE).children(table_rows)
        }
    }
}

fn convert_inline(inline: &Inline) -> Node {
    match inline {
        Inline::Text(s, _) => Node::new(node::TEXT).prop(prop::CONTENT, s.clone()),

        Inline::Bold(children, _) => {
            let converted: Vec<Node> = children.iter().map(convert_inline).collect();
            Node::new(node::STRONG).children(converted)
        }

        Inline::Italic(children, _) => {
            let converted: Vec<Node> = children.iter().map(convert_inline).collect();
            Node::new(node::EMPHASIS).children(converted)
        }

        Inline::Underline(children, _) => {
            let converted: Vec<Node> = children.iter().map(convert_inline).collect();
            Node::new(node::UNDERLINE).children(converted)
        }

        Inline::Strikethrough(children, _) => {
            let converted: Vec<Node> = children.iter().map(convert_inline).collect();
            Node::new(node::STRIKEOUT).children(converted)
        }

        Inline::Code(s, _) => Node::new(node::CODE).prop(prop::CONTENT, s.clone()),

        Inline::Link { url, children, .. } => {
            let converted: Vec<Node> = children.iter().map(convert_inline).collect();
            Node::new(node::LINK)
                .prop(prop::URL, url.clone())
                .children(converted)
        }

        Inline::Image { url, alt, .. } => {
            let mut node = Node::new(node::IMAGE).prop(prop::URL, url.clone());
            if let Some(alt_text) = alt {
                node = node.prop(prop::ALT, alt_text.clone());
            }
            node
        }

        Inline::Superscript(children, _) => {
            let converted: Vec<Node> = children.iter().map(convert_inline).collect();
            Node::new(node::SUPERSCRIPT).children(converted)
        }

        Inline::Subscript(children, _) => {
            let converted: Vec<Node> = children.iter().map(convert_inline).collect();
            Node::new(node::SUBSCRIPT).children(converted)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_str(input: &str) -> Document {
        parse(input).unwrap().value
    }

    #[test]
    fn test_parse_heading() {
        let doc = parse_str("h1. Title\n");
        assert_eq!(doc.content.children.len(), 1);
        assert_eq!(doc.content.children[0].kind.as_str(), node::HEADING);
        assert_eq!(doc.content.children[0].props.get_int(prop::LEVEL), Some(1));
    }

    #[test]
    fn test_parse_heading_levels() {
        let doc = parse_str("h2. Level 2\nh3. Level 3\n");
        assert_eq!(doc.content.children.len(), 2);
        assert_eq!(doc.content.children[0].props.get_int(prop::LEVEL), Some(2));
        assert_eq!(doc.content.children[1].props.get_int(prop::LEVEL), Some(3));
    }

    #[test]
    fn test_parse_paragraph() {
        let doc = parse_str("Hello world\n");
        assert_eq!(doc.content.children.len(), 1);
        assert_eq!(doc.content.children[0].kind.as_str(), node::PARAGRAPH);
    }

    #[test]
    fn test_parse_bold() {
        let doc = parse_str("*bold*\n");
        let para = &doc.content.children[0];
        assert_eq!(para.children.len(), 1);
        assert_eq!(para.children[0].kind.as_str(), node::STRONG);
    }

    #[test]
    fn test_parse_italic() {
        let doc = parse_str("_italic_\n");
        let para = &doc.content.children[0];
        assert_eq!(para.children.len(), 1);
        assert_eq!(para.children[0].kind.as_str(), node::EMPHASIS);
    }

    #[test]
    fn test_parse_code() {
        let doc = parse_str("@code@\n");
        let para = &doc.content.children[0];
        assert_eq!(para.children.len(), 1);
        assert_eq!(para.children[0].kind.as_str(), node::CODE);
    }

    #[test]
    fn test_parse_link() {
        let doc = parse_str("\"Example\":https://example.com\n");
        let para = &doc.content.children[0];
        let link = &para.children[0];
        assert_eq!(link.kind.as_str(), node::LINK);
        assert_eq!(link.props.get_str(prop::URL), Some("https://example.com"));
    }

    #[test]
    fn test_parse_list() {
        let doc = parse_str("* item1\n* item2\n");
        assert_eq!(doc.content.children.len(), 1);
        let list = &doc.content.children[0];
        assert_eq!(list.kind.as_str(), node::LIST);
        assert_eq!(list.children.len(), 2);
    }

    #[test]
    fn test_parse_code_block() {
        let doc = parse_str("bc. code here\n");
        assert_eq!(doc.content.children.len(), 1);
        assert_eq!(doc.content.children[0].kind.as_str(), node::CODE_BLOCK);
    }
}
