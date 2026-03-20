//! Haddock markup reader for rescribe.
//!
//! Parses Haddock documentation markup into the rescribe document model.

use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions};
use rescribe_std::{node, prop};

/// Parse Haddock markup.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse Haddock markup with custom options.
pub fn parse_with_options(
    input: &str,
    _options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let (haddock_doc, _diagnostics) = haddock_fmt::parse(input);
    let nodes = convert_blocks(&haddock_doc.blocks);

    let root = Node::new(node::DOCUMENT).children(nodes);
    let doc = Document::new().with_content(root);

    Ok(ConversionResult::ok(doc))
}

fn convert_blocks(blocks: &[haddock_fmt::Block]) -> Vec<Node> {
    blocks.iter().map(convert_block).collect()
}

fn convert_block(block: &haddock_fmt::Block) -> Node {
    match block {
        haddock_fmt::Block::Heading { level, inlines, .. } => Node::new(node::HEADING)
            .prop(prop::LEVEL, *level as i64)
            .children(convert_inlines(inlines)),

        haddock_fmt::Block::Paragraph { inlines, .. } => {
            Node::new(node::PARAGRAPH).children(convert_inlines(inlines))
        }

        haddock_fmt::Block::CodeBlock { content, .. } => {
            Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone())
        }

        haddock_fmt::Block::UnorderedList { items, .. } => {
            let list_items: Vec<Node> = items
                .iter()
                .map(|item_inlines| {
                    let para = Node::new(node::PARAGRAPH).children(convert_inlines(item_inlines));
                    Node::new(node::LIST_ITEM).children(vec![para])
                })
                .collect();

            Node::new(node::LIST)
                .prop(prop::ORDERED, false)
                .children(list_items)
        }

        haddock_fmt::Block::OrderedList { items, .. } => {
            let list_items: Vec<Node> = items
                .iter()
                .map(|item_inlines| {
                    let para = Node::new(node::PARAGRAPH).children(convert_inlines(item_inlines));
                    Node::new(node::LIST_ITEM).children(vec![para])
                })
                .collect();

            Node::new(node::LIST)
                .prop(prop::ORDERED, true)
                .children(list_items)
        }

        haddock_fmt::Block::DefinitionList { items, .. } => {
            let mut def_items = Vec::new();
            for (term_inlines, desc_inlines) in items {
                def_items
                    .push(Node::new(node::DEFINITION_TERM).children(convert_inlines(term_inlines)));
                def_items.push(Node::new(node::DEFINITION_DESC).children(vec![
                    Node::new(node::PARAGRAPH).children(convert_inlines(desc_inlines)),
                ]));
            }
            Node::new(node::DEFINITION_LIST).children(def_items)
        }
    }
}

fn convert_inlines(inlines: &[haddock_fmt::Inline]) -> Vec<Node> {
    inlines.iter().map(convert_inline).collect()
}

fn convert_inline(inline: &haddock_fmt::Inline) -> Node {
    match inline {
        haddock_fmt::Inline::Text(s, _) => Node::new(node::TEXT).prop(prop::CONTENT, s.clone()),

        haddock_fmt::Inline::Code(s, _) => Node::new(node::CODE).prop(prop::CONTENT, s.clone()),

        haddock_fmt::Inline::Strong(children, _) => {
            Node::new(node::STRONG).children(convert_inlines(children))
        }

        haddock_fmt::Inline::Emphasis(children, _) => {
            Node::new(node::EMPHASIS).children(convert_inlines(children))
        }

        haddock_fmt::Inline::Link { url, text, .. } => {
            let text_node = Node::new(node::TEXT).prop(prop::CONTENT, text.clone());
            Node::new(node::LINK)
                .prop(prop::URL, url.clone())
                .children(vec![text_node])
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
        let doc = parse_str("= Title\n");
        assert_eq!(doc.content.children.len(), 1);
        assert_eq!(doc.content.children[0].kind.as_str(), node::HEADING);
        assert_eq!(doc.content.children[0].props.get_int(prop::LEVEL), Some(1));
    }

    #[test]
    fn test_parse_heading_levels() {
        let doc = parse_str("== Level 2\n=== Level 3\n");
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
        let doc = parse_str("__bold__\n");
        let para = &doc.content.children[0];
        assert_eq!(para.children.len(), 1);
        assert_eq!(para.children[0].kind.as_str(), node::STRONG);
    }

    #[test]
    fn test_parse_italic() {
        let doc = parse_str("/italic/\n");
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
        let doc = parse_str("\"Example\"<https://example.com>\n");
        let para = &doc.content.children[0];
        let link = &para.children[0];
        assert_eq!(link.kind.as_str(), node::LINK);
        assert_eq!(link.props.get_str(prop::URL), Some("https://example.com"));
    }

    #[test]
    fn test_parse_unordered_list() {
        let doc = parse_str("* item1\n* item2\n");
        assert_eq!(doc.content.children.len(), 1);
        let list = &doc.content.children[0];
        assert_eq!(list.kind.as_str(), node::LIST);
        assert_eq!(list.children.len(), 2);
    }

    #[test]
    fn test_parse_code_block() {
        let doc = parse_str("> code here\n");
        assert_eq!(doc.content.children.len(), 1);
        assert_eq!(doc.content.children[0].kind.as_str(), node::CODE_BLOCK);
    }
}
