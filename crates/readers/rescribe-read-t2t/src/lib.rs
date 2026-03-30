//! txt2tags (t2t) reader for rescribe.
//!
//! Thin adapter that uses the `t2t` crate to parse txt2tags markup
//! into the rescribe document model.

use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions, Properties};
use rescribe_std::{node, prop};
use t2t::{Block, Inline};

/// Parse txt2tags markup.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse txt2tags markup with custom options.
pub fn parse_with_options(
    input: &str,
    _options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let (t2t_doc, _diagnostics) = t2t::parse(input);

    let mut metadata = Properties::new();
    if let Some(title) = &t2t_doc.title {
        metadata.set(prop::TITLE, title.clone());
    }
    if let Some(author) = &t2t_doc.author {
        metadata.set("author", author.clone());
    }
    if let Some(date) = &t2t_doc.date {
        metadata.set("date", date.clone());
    }

    let mut nodes = Vec::new();
    for block in &t2t_doc.blocks {
        nodes.push(block_to_node(block));
    }

    let root = Node::new(node::DOCUMENT).children(nodes);
    let doc = Document::new().with_content(root).with_metadata(metadata);

    Ok(ConversionResult::ok(doc))
}

fn block_to_node(block: &Block) -> Node {
    match block {
        Block::Paragraph { inlines, .. } => {
            let children: Vec<Node> = inlines.iter().map(inline_to_node).collect();
            Node::new(node::PARAGRAPH).children(children)
        }

        Block::Heading {
            level,
            numbered,
            inlines,
            ..
        } => {
            let children: Vec<Node> = inlines.iter().map(inline_to_node).collect();
            let mut heading = Node::new(node::HEADING)
                .prop(prop::LEVEL, *level as i64)
                .children(children);

            if *numbered {
                heading = heading.prop("numbered", true);
            }

            heading
        }

        Block::CodeBlock { content, .. } => {
            Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone())
        }

        Block::RawBlock { content, .. } => {
            Node::new(node::RAW_BLOCK).prop(prop::CONTENT, content.clone())
        }

        Block::Blockquote { children, .. } => {
            let para_children: Vec<Node> = children.iter().map(block_to_node).collect();
            Node::new(node::BLOCKQUOTE).children(para_children)
        }

        Block::List { ordered, items, .. } => {
            let list_items: Vec<Node> = items
                .iter()
                .map(|item_blocks| {
                    let item_children: Vec<Node> = item_blocks.iter().map(block_to_node).collect();
                    Node::new(node::LIST_ITEM).children(item_children)
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
                        .map(|cell_inlines| {
                            let children: Vec<Node> =
                                cell_inlines.iter().map(inline_to_node).collect();
                            if row.is_header {
                                Node::new(node::TABLE_HEADER).children(children)
                            } else {
                                Node::new(node::TABLE_CELL).children(children)
                            }
                        })
                        .collect();
                    Node::new(node::TABLE_ROW).children(cells)
                })
                .collect();

            Node::new(node::TABLE).children(table_rows)
        }

        Block::HorizontalRule { .. } => Node::new(node::HORIZONTAL_RULE),

        Block::DefinitionList { items, .. } => {
            let children: Vec<Node> = items
                .iter()
                .flat_map(|(term, desc)| {
                    let term_children: Vec<Node> = term.iter().map(inline_to_node).collect();
                    let term_node = Node::new(node::DEFINITION_TERM).children(term_children);
                    let desc_children: Vec<Node> = desc.iter().map(block_to_node).collect();
                    let desc_node = Node::new(node::DEFINITION_DESC).children(desc_children);
                    vec![term_node, desc_node]
                })
                .collect();
            Node::new(node::DEFINITION_LIST).children(children)
        }
    }
}

fn inline_to_node(inline: &Inline) -> Node {
    match inline {
        Inline::Text(s, _) => Node::new(node::TEXT).prop(prop::CONTENT, s.clone()),

        Inline::Bold(children, _) => {
            let nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
            Node::new(node::STRONG).children(nodes)
        }

        Inline::Italic(children, _) => {
            let nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
            Node::new(node::EMPHASIS).children(nodes)
        }

        Inline::Underline(children, _) => {
            let nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
            Node::new(node::UNDERLINE).children(nodes)
        }

        Inline::Strikethrough(children, _) => {
            let nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
            Node::new(node::STRIKEOUT).children(nodes)
        }

        Inline::Code(content, _) => {
            Node::new(node::CODE).prop(prop::CONTENT, content.clone())
        }

        Inline::Link { url, children, .. } => {
            let nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
            Node::new(node::LINK)
                .prop(prop::URL, url.clone())
                .children(nodes)
        }

        Inline::Image { url, .. } => Node::new(node::IMAGE).prop(prop::URL, url.clone()),

        Inline::LineBreak(_) => Node::new(node::LINE_BREAK),

        Inline::SoftBreak(_) => Node::new(node::SOFT_BREAK),

        Inline::Verbatim(content, _) => {
            Node::new(node::RAW_INLINE).prop(prop::CONTENT, content.clone())
        }

        Inline::Tagged(content, _) => {
            Node::new(node::RAW_INLINE)
                .prop(prop::CONTENT, content.clone())
                .prop("t2t:tagged", true)
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
        let doc = parse_str("= Title =\n");
        assert_eq!(doc.content.children.len(), 1);
        assert_eq!(doc.content.children[0].kind.as_str(), node::HEADING);
        assert_eq!(doc.content.children[0].props.get_int(prop::LEVEL), Some(1));
    }

    #[test]
    fn test_parse_heading_level2() {
        let doc = parse_str("== Subtitle ==\n");
        assert_eq!(doc.content.children[0].props.get_int(prop::LEVEL), Some(2));
    }

    #[test]
    fn test_parse_numbered_heading() {
        let doc = parse_str("+ Numbered +\n");
        assert_eq!(doc.content.children[0].kind.as_str(), node::HEADING);
        assert_eq!(
            doc.content.children[0].props.get_bool("numbered"),
            Some(true)
        );
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
    fn test_parse_underline() {
        let doc = parse_str("__underline__\n");
        let para = &doc.content.children[0];
        assert_eq!(para.children[0].kind.as_str(), node::UNDERLINE);
    }

    #[test]
    fn test_parse_strikethrough() {
        let doc = parse_str("--strike--\n");
        let para = &doc.content.children[0];
        assert_eq!(para.children[0].kind.as_str(), node::STRIKEOUT);
    }

    #[test]
    fn test_parse_monospace() {
        let doc = parse_str("``code``\n");
        let para = &doc.content.children[0];
        assert_eq!(para.children[0].kind.as_str(), node::CODE);
    }

    #[test]
    fn test_parse_unordered_list() {
        let doc = parse_str("- item1\n- item2\n");
        assert_eq!(doc.content.children.len(), 1);
        let list = &doc.content.children[0];
        assert_eq!(list.kind.as_str(), node::LIST);
        assert_eq!(list.children.len(), 2);
    }

    #[test]
    fn test_parse_ordered_list() {
        let doc = parse_str("+ first\n+ second\n");
        let list = &doc.content.children[0];
        assert_eq!(list.kind.as_str(), node::LIST);
        assert_eq!(list.props.get_bool(prop::ORDERED), Some(true));
    }

    #[test]
    fn test_parse_verbatim_block() {
        let doc = parse_str("```\ncode here\n```\n");
        assert_eq!(doc.content.children[0].kind.as_str(), node::CODE_BLOCK);
        assert_eq!(
            doc.content.children[0].props.get_str(prop::CONTENT),
            Some("code here")
        );
    }

    #[test]
    fn test_parse_link() {
        let doc = parse_str("[click here http://example.com]\n");
        let para = &doc.content.children[0];
        let link = &para.children[0];
        assert_eq!(link.kind.as_str(), node::LINK);
        assert_eq!(link.props.get_str(prop::URL), Some("http://example.com"));
    }

    #[test]
    fn test_parse_quote() {
        let doc = parse_str("\tquoted text\n");
        assert_eq!(doc.content.children[0].kind.as_str(), node::BLOCKQUOTE);
    }

    #[test]
    fn test_skip_comments() {
        let doc = parse_str("% comment\ntext\n");
        assert_eq!(doc.content.children.len(), 1);
        assert_eq!(doc.content.children[0].kind.as_str(), node::PARAGRAPH);
    }
}
