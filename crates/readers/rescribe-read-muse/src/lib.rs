//! Muse markup reader for rescribe.
//!
//! Parses Emacs Muse markup into the rescribe document model.

use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions};
use rescribe_std::{node, prop};

/// Parse Muse markup.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse Muse markup with custom options.
pub fn parse_with_options(
    input: &str,
    _options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    // Parse using the format-specific crate
    let muse_doc = muse_fmt::parse(input).map_err(|e| ParseError::Invalid(e.0))?;

    // Convert muse_doc to rescribe Document
    let blocks = convert_blocks(&muse_doc.blocks);
    let root = Node::new(node::DOCUMENT).children(blocks);
    let doc = Document::new().with_content(root);

    Ok(ConversionResult::ok(doc))
}

fn convert_blocks(blocks: &[muse_fmt::Block]) -> Vec<Node> {
    blocks.iter().map(convert_block).collect()
}

fn convert_block(block: &muse_fmt::Block) -> Node {
    match block {
        muse_fmt::Block::Paragraph { inlines } => {
            Node::new(node::PARAGRAPH).children(convert_inlines(inlines))
        }

        muse_fmt::Block::Heading { level, inlines } => Node::new(node::HEADING)
            .prop(prop::LEVEL, *level as i64)
            .children(convert_inlines(inlines)),

        muse_fmt::Block::CodeBlock { content } => {
            Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone())
        }

        muse_fmt::Block::Blockquote { children } => {
            Node::new(node::BLOCKQUOTE).children(convert_blocks(children))
        }

        muse_fmt::Block::List { ordered, items } => {
            let list_items: Vec<Node> = items
                .iter()
                .map(|item_blocks| {
                    let item_nodes = convert_blocks(item_blocks);
                    Node::new(node::LIST_ITEM).children(item_nodes)
                })
                .collect();
            Node::new(node::LIST)
                .prop(prop::ORDERED, *ordered)
                .children(list_items)
        }

        muse_fmt::Block::DefinitionList { items } => {
            let mut children: Vec<Node> = Vec::new();
            for (term_inlines, desc_blocks) in items {
                let term_node =
                    Node::new(node::DEFINITION_TERM).children(convert_inlines(term_inlines));
                let desc_node =
                    Node::new(node::DEFINITION_DESC).children(convert_blocks(desc_blocks));
                children.push(term_node);
                children.push(desc_node);
            }
            Node::new(node::DEFINITION_LIST).children(children)
        }

        muse_fmt::Block::HorizontalRule => Node::new(node::HORIZONTAL_RULE),
    }
}

fn convert_inlines(inlines: &[muse_fmt::Inline]) -> Vec<Node> {
    inlines.iter().map(convert_inline).collect()
}

fn convert_inline(inline: &muse_fmt::Inline) -> Node {
    match inline {
        muse_fmt::Inline::Text(s) => Node::new(node::TEXT).prop(prop::CONTENT, s.clone()),

        muse_fmt::Inline::Bold(children) => {
            Node::new(node::STRONG).children(convert_inlines(children))
        }

        muse_fmt::Inline::Italic(children) => {
            Node::new(node::EMPHASIS).children(convert_inlines(children))
        }

        muse_fmt::Inline::Code(s) => Node::new(node::CODE).prop(prop::CONTENT, s.clone()),

        muse_fmt::Inline::Link { url, children } => Node::new(node::LINK)
            .prop(prop::URL, url.clone())
            .children(convert_inlines(children)),
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
        let doc = parse_str("* Title\n");
        assert_eq!(doc.content.children.len(), 1);
        assert_eq!(doc.content.children[0].kind.as_str(), node::HEADING);
        assert_eq!(doc.content.children[0].props.get_int(prop::LEVEL), Some(1));
    }

    #[test]
    fn test_parse_heading_levels() {
        let doc = parse_str("** Level 2\n*** Level 3\n");
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
        let doc = parse_str("**bold**\n");
        let para = &doc.content.children[0];
        assert_eq!(para.children.len(), 1);
        assert_eq!(para.children[0].kind.as_str(), node::STRONG);
    }

    #[test]
    fn test_parse_emphasis() {
        let doc = parse_str("text with *emphasis*\n");
        let para = &doc.content.children[0];
        assert!(
            para.children
                .iter()
                .any(|n| n.kind.as_str() == node::EMPHASIS)
        );
    }

    #[test]
    fn test_parse_code() {
        let doc = parse_str("=code=\n");
        let para = &doc.content.children[0];
        assert_eq!(para.children.len(), 1);
        assert_eq!(para.children[0].kind.as_str(), node::CODE);
    }

    #[test]
    fn test_parse_link() {
        let doc = parse_str("[[https://example.com][Example]]\n");
        let para = &doc.content.children[0];
        let link = &para.children[0];
        assert_eq!(link.kind.as_str(), node::LINK);
        assert_eq!(link.props.get_str(prop::URL), Some("https://example.com"));
    }

    #[test]
    fn test_parse_unordered_list() {
        let doc = parse_str(" - item1\n - item2\n");
        assert_eq!(doc.content.children.len(), 1);
        let list = &doc.content.children[0];
        assert_eq!(list.kind.as_str(), node::LIST);
        assert_eq!(list.children.len(), 2);
    }

    #[test]
    fn test_parse_example_block() {
        let doc = parse_str("<example>\ncode here\n</example>\n");
        assert_eq!(doc.content.children.len(), 1);
        assert_eq!(doc.content.children[0].kind.as_str(), node::CODE_BLOCK);
    }
}
