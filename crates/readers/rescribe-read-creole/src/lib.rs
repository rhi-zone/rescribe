//! Creole wiki markup reader for rescribe.
//!
//! Thin adapter translating the format-independent `creole` crate
//! into the rescribe document model.
use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions};
use rescribe_std::{node, prop};

/// Parse Creole markup.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse Creole markup with custom options.
pub fn parse_with_options(
    input: &str,
    _options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let (creole_doc, _diagnostics) = creole::parse(input);
    let nodes = convert_blocks(&creole_doc.blocks);
    let root = Node::new(node::DOCUMENT).children(nodes);
    let doc = Document::new().with_content(root);
    Ok(ConversionResult::ok(doc))
}

fn convert_blocks(blocks: &[creole::Block]) -> Vec<Node> {
    blocks.iter().map(convert_block).collect()
}

fn convert_block(block: &creole::Block) -> Node {
    match block {
        creole::Block::Paragraph { inlines, .. } => {
            Node::new(node::PARAGRAPH).children(convert_inlines(inlines))
        }

        creole::Block::Heading { level, inlines, .. } => Node::new(node::HEADING)
            .prop(prop::LEVEL, *level as i64)
            .children(convert_inlines(inlines)),

        creole::Block::CodeBlock { content, .. } => {
            Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone())
        }

        creole::Block::Blockquote { children, .. } => {
            Node::new(node::BLOCKQUOTE).children(convert_blocks(children))
        }

        creole::Block::List { ordered, items, .. } => {
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

        creole::Block::Table { rows, .. } => {
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
                            Node::new(kind).children(convert_inlines(&cell.inlines))
                        })
                        .collect();
                    Node::new(node::TABLE_ROW).children(cells)
                })
                .collect();
            Node::new(node::TABLE).children(table_rows)
        }

        creole::Block::DefinitionList { items, .. } => {
            let children: Vec<Node> = items
                .iter()
                .flat_map(|item| {
                    let term_children: Vec<Node> = item.term.iter().map(convert_inline).collect();
                    let desc_children: Vec<Node> = item.desc.iter().map(convert_inline).collect();
                    vec![
                        Node::new(node::DEFINITION_TERM).children(term_children),
                        Node::new(node::DEFINITION_DESC).children(desc_children),
                    ]
                })
                .collect();
            Node::new(node::DEFINITION_LIST).children(children)
        }

        creole::Block::HorizontalRule(_) => Node::new(node::HORIZONTAL_RULE),
    }
}

fn convert_inlines(inlines: &[creole::Inline]) -> Vec<Node> {
    inlines.iter().map(convert_inline).collect()
}

fn convert_inline(inline: &creole::Inline) -> Node {
    match inline {
        creole::Inline::Text(s, _) => Node::new(node::TEXT).prop(prop::CONTENT, s.clone()),

        creole::Inline::Bold(children, _) => {
            Node::new(node::STRONG).children(convert_inlines(children))
        }

        creole::Inline::Italic(children, _) => {
            Node::new(node::EMPHASIS).children(convert_inlines(children))
        }

        creole::Inline::Code(s, _) => Node::new(node::CODE).prop(prop::CONTENT, s.clone()),

        creole::Inline::Link { url, children, .. } => Node::new(node::LINK)
            .prop(prop::URL, url.clone())
            .children(convert_inlines(children)),

        creole::Inline::Image { url, alt, .. } => {
            let mut img = Node::new(node::IMAGE).prop(prop::URL, url.clone());
            if let Some(alt_text) = alt {
                img = img.prop(prop::ALT, alt_text.clone());
            }
            img
        }

        creole::Inline::LineBreak(_) => Node::new(node::LINE_BREAK),
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
        let doc = parse_str("**bold**\n");
        let para = &doc.content.children[0];
        assert_eq!(para.children.len(), 1);
        assert_eq!(para.children[0].kind.as_str(), node::STRONG);
    }

    #[test]
    fn test_parse_italic() {
        let doc = parse_str("//italic//\n");
        let para = &doc.content.children[0];
        assert_eq!(para.children.len(), 1);
        assert_eq!(para.children[0].kind.as_str(), node::EMPHASIS);
    }

    #[test]
    fn test_parse_link() {
        let doc = parse_str("[[https://example.com|Example]]\n");
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
    fn test_parse_nowiki() {
        let doc = parse_str("{{{code}}}\n");
        assert_eq!(doc.content.children.len(), 1);
        assert_eq!(doc.content.children[0].kind.as_str(), node::CODE_BLOCK);
    }
}
