//! ZimWiki reader for rescribe.
//!
//! Thin adapter layer converting zimwiki AST to rescribe Document model.

use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions};
use rescribe_std::{node, prop};
use zimwiki::{Block, Inline, ListItem, parse as parse_zimwiki};

/// Parse ZimWiki markup.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse ZimWiki markup with custom options.
pub fn parse_with_options(
    input: &str,
    _options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let zimwiki_doc = parse_zimwiki(input).map_err(|e| ParseError::Invalid(e.to_string()))?;

    let mut nodes = Vec::new();
    for block in &zimwiki_doc.blocks {
        nodes.push(convert_block(block));
    }

    let root = Node::new(node::DOCUMENT).children(nodes);
    let doc = Document::new().with_content(root);

    Ok(ConversionResult::ok(doc))
}

fn convert_block(block: &Block) -> Node {
    match block {
        Block::Paragraph { inlines } => {
            let children: Vec<Node> = inlines.iter().map(convert_inline).collect();
            Node::new(node::PARAGRAPH).children(children)
        }

        Block::Heading { level, inlines } => {
            let children: Vec<Node> = inlines.iter().map(convert_inline).collect();
            Node::new(node::HEADING)
                .prop(prop::LEVEL, *level as i64)
                .children(children)
        }

        Block::CodeBlock { content } => {
            Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone())
        }

        Block::Blockquote { children } => {
            let child_nodes: Vec<Node> = children.iter().map(convert_block).collect();
            Node::new(node::BLOCKQUOTE).children(child_nodes)
        }

        Block::List { ordered, items } => {
            let child_nodes: Vec<Node> = items.iter().map(convert_list_item).collect();
            Node::new(node::LIST)
                .prop(prop::ORDERED, *ordered)
                .children(child_nodes)
        }

        Block::Table { rows } => {
            let child_nodes: Vec<Node> = rows
                .iter()
                .map(|row| {
                    let cells: Vec<Node> = row
                        .cells
                        .iter()
                        .map(|cell| {
                            let inlines: Vec<Node> = cell.iter().map(convert_inline).collect();
                            Node::new(node::TABLE_CELL).children(inlines)
                        })
                        .collect();
                    Node::new(node::TABLE_ROW).children(cells)
                })
                .collect();
            Node::new(node::TABLE).children(child_nodes)
        }

        Block::HorizontalRule => Node::new(node::HORIZONTAL_RULE),
    }
}

fn convert_list_item(item: &ListItem) -> Node {
    let children: Vec<Node> = item.children.iter().map(convert_block).collect();
    let mut list_item = Node::new(node::LIST_ITEM).children(children);
    if let Some(checked) = item.checked {
        list_item = list_item.prop("checked", checked);
    }
    list_item
}

fn convert_inline(inline: &Inline) -> Node {
    match inline {
        Inline::Text(s) => Node::new(node::TEXT).prop(prop::CONTENT, s.clone()),

        Inline::Bold(children) => {
            let inlines: Vec<Node> = children.iter().map(convert_inline).collect();
            Node::new(node::STRONG).children(inlines)
        }

        Inline::Italic(children) => {
            let inlines: Vec<Node> = children.iter().map(convert_inline).collect();
            Node::new(node::EMPHASIS).children(inlines)
        }

        Inline::Underline(children) => {
            let inlines: Vec<Node> = children.iter().map(convert_inline).collect();
            Node::new(node::UNDERLINE).children(inlines)
        }

        Inline::Strikethrough(children) => {
            let inlines: Vec<Node> = children.iter().map(convert_inline).collect();
            Node::new(node::STRIKEOUT).children(inlines)
        }

        Inline::Subscript(children) => {
            let inlines: Vec<Node> = children.iter().map(convert_inline).collect();
            Node::new(node::SUBSCRIPT).children(inlines)
        }

        Inline::Superscript(children) => {
            let inlines: Vec<Node> = children.iter().map(convert_inline).collect();
            Node::new(node::SUPERSCRIPT).children(inlines)
        }

        Inline::Code(s) => Node::new(node::CODE).prop(prop::CONTENT, s.clone()),

        Inline::Link { url, children } => {
            let inlines: Vec<Node> = children.iter().map(convert_inline).collect();
            Node::new(node::LINK)
                .prop(prop::URL, url.clone())
                .children(inlines)
        }

        Inline::Image { url } => Node::new(node::IMAGE).prop(prop::URL, url.clone()),

        Inline::LineBreak => Node::new(node::LINE_BREAK),

        Inline::SoftBreak => Node::new(node::SOFT_BREAK),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_str(input: &str) -> Document {
        parse(input).unwrap().value
    }

    #[test]
    fn test_parse_heading_level1() {
        let doc = parse_str("====== Title ======\n");
        assert_eq!(doc.content.children.len(), 1);
        assert_eq!(doc.content.children[0].kind.as_str(), node::HEADING);
        assert_eq!(doc.content.children[0].props.get_int(prop::LEVEL), Some(1));
    }

    #[test]
    fn test_parse_heading_level2() {
        let doc = parse_str("===== Subtitle =====\n");
        assert_eq!(doc.content.children[0].props.get_int(prop::LEVEL), Some(2));
    }

    #[test]
    fn test_parse_paragraph() {
        let doc = parse_str("Hello world\n");
        assert_eq!(doc.content.children.len(), 1);
        assert_eq!(doc.content.children[0].kind.as_str(), node::PARAGRAPH);
    }

    #[test]
    fn test_parse_bold() {
        let doc = parse_str("**bold**\n");
        let para = &doc.content.children[0];
        assert_eq!(para.children[0].kind.as_str(), node::STRONG);
    }

    #[test]
    fn test_parse_italic() {
        let doc = parse_str("//italic//\n");
        let para = &doc.content.children[0];
        assert_eq!(para.children[0].kind.as_str(), node::EMPHASIS);
    }

    #[test]
    fn test_parse_strikethrough() {
        let doc = parse_str("~~strike~~\n");
        let para = &doc.content.children[0];
        assert_eq!(para.children[0].kind.as_str(), node::STRIKEOUT);
    }

    #[test]
    fn test_parse_code() {
        let doc = parse_str("''code''\n");
        let para = &doc.content.children[0];
        assert_eq!(para.children[0].kind.as_str(), node::CODE);
    }

    #[test]
    fn test_parse_link() {
        let doc = parse_str("[[MyPage]]\n");
        let para = &doc.content.children[0];
        let link = &para.children[0];
        assert_eq!(link.kind.as_str(), node::LINK);
        assert_eq!(link.props.get_str(prop::URL), Some("MyPage"));
    }

    #[test]
    fn test_parse_link_with_label() {
        let doc = parse_str("[[MyPage|click here]]\n");
        let para = &doc.content.children[0];
        let link = &para.children[0];
        assert_eq!(link.props.get_str(prop::URL), Some("MyPage"));
    }

    #[test]
    fn test_parse_unordered_list() {
        let doc = parse_str("* item1\n* item2\n");
        let list = &doc.content.children[0];
        assert_eq!(list.kind.as_str(), node::LIST);
        assert_eq!(list.children.len(), 2);
    }

    #[test]
    fn test_parse_checkbox_list() {
        let doc = parse_str("[ ] unchecked\n[*] checked\n");
        let list = &doc.content.children[0];
        assert_eq!(list.children[0].props.get_bool("checked"), Some(false));
        assert_eq!(list.children[1].props.get_bool("checked"), Some(true));
    }

    #[test]
    fn test_parse_verbatim() {
        let doc = parse_str("'''\ncode here\n'''\n");
        assert_eq!(doc.content.children[0].kind.as_str(), node::CODE_BLOCK);
    }
}
