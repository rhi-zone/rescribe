//! VimWiki reader for rescribe.
//!
//! Parses VimWiki markup into the rescribe document model.
//! Thin adapter over `vimwiki-fmt` crate.

use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions};
use rescribe_std::{node, prop};
use vimwiki_fmt::*;

/// Parse VimWiki markup.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse VimWiki markup with custom options.
pub fn parse_with_options(
    input: &str,
    _options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let (doc, _diags) = vimwiki_fmt::parse(input);

    let nodes: Vec<Node> = doc.blocks.iter().map(block_to_node).collect();
    let root = Node::new(node::DOCUMENT).children(nodes);
    let rescribe_doc = Document::new().with_content(root);

    Ok(ConversionResult::ok(rescribe_doc))
}

fn block_to_node(block: &Block) -> Node {
    match block {
        Block::Paragraph { inlines, .. } => {
            let inline_nodes: Vec<Node> = inlines.iter().map(inline_to_node).collect();
            Node::new(node::PARAGRAPH).children(inline_nodes)
        }

        Block::Heading { level, inlines, .. } => {
            let inline_nodes: Vec<Node> = inlines.iter().map(inline_to_node).collect();
            Node::new(node::HEADING)
                .prop(prop::LEVEL, *level as i64)
                .children(inline_nodes)
        }

        Block::CodeBlock { language, content, .. } => {
            let mut n = Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone());
            if let Some(lang) = language {
                n = n.prop(prop::LANGUAGE, lang.clone());
            }
            n
        }

        Block::Blockquote { inlines, .. } => {
            let inline_nodes: Vec<Node> = inlines.iter().map(inline_to_node).collect();
            let para = Node::new(node::PARAGRAPH).children(inline_nodes);
            Node::new(node::BLOCKQUOTE).children(vec![para])
        }

        Block::List { ordered, items, .. } => {
            let list_items: Vec<Node> = items
                .iter()
                .map(|item| {
                    let inline_nodes: Vec<Node> = item.inlines.iter().map(inline_to_node).collect();
                    let para = Node::new(node::PARAGRAPH).children(inline_nodes);
                    let mut list_item = Node::new(node::LIST_ITEM).children(vec![para]);
                    if let Some(checked) = item.checked {
                        list_item = list_item.prop("checked", checked);
                    }
                    list_item
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
                            let inline_nodes: Vec<Node> = cell.iter().map(inline_to_node).collect();
                            Node::new(node::TABLE_CELL).children(inline_nodes)
                        })
                        .collect();
                    Node::new(node::TABLE_ROW).children(cells)
                })
                .collect();

            Node::new(node::TABLE).children(table_rows)
        }

        Block::HorizontalRule { .. } => Node::new(node::HORIZONTAL_RULE),

        Block::DefinitionList { items, .. } => {
            let mut children = Vec::new();
            for item in items {
                let term = Node::new(node::DEFINITION_TERM)
                    .children(item.term.iter().map(inline_to_node).collect::<Vec<_>>());
                let desc = Node::new(node::DEFINITION_DESC)
                    .children(item.desc.iter().map(inline_to_node).collect::<Vec<_>>());
                children.push(term);
                children.push(desc);
            }
            Node::new(node::DEFINITION_LIST).children(children)
        }
    }
}

fn inline_to_node(inline: &Inline) -> Node {
    match inline {
        Inline::Text(s, _) => Node::new(node::TEXT).prop(prop::CONTENT, s.clone()),

        Inline::Bold(children, _) => {
            let inner: Vec<Node> = children.iter().map(inline_to_node).collect();
            Node::new(node::STRONG).children(inner)
        }

        Inline::Italic(children, _) => {
            let inner: Vec<Node> = children.iter().map(inline_to_node).collect();
            Node::new(node::EMPHASIS).children(inner)
        }

        Inline::Strikethrough(children, _) => {
            let inner: Vec<Node> = children.iter().map(inline_to_node).collect();
            Node::new(node::STRIKEOUT).children(inner)
        }

        Inline::Code(s, _) => Node::new(node::CODE).prop(prop::CONTENT, s.clone()),

        Inline::Link { url, label, .. } => {
            let text_node = Node::new(node::TEXT).prop(prop::CONTENT, label.clone());
            Node::new(node::LINK)
                .prop(prop::URL, url.clone())
                .children(vec![text_node])
        }

        Inline::Image { url, alt, style, .. } => {
            let mut img = Node::new(node::IMAGE).prop(prop::URL, url.clone());
            if let Some(a) = alt {
                img = img.prop(prop::ALT, a.clone());
            }
            if let Some(s) = style {
                img = img.prop("vimwiki:style", s.clone());
            }
            img
        }

        Inline::Superscript(children, _) => {
            Node::new(node::SUPERSCRIPT).children(children.iter().map(inline_to_node).collect::<Vec<_>>())
        }

        Inline::Subscript(children, _) => {
            Node::new(node::SUBSCRIPT).children(children.iter().map(inline_to_node).collect::<Vec<_>>())
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
    fn test_parse_paragraph() {
        let doc = parse_str("Hello world\n");
        assert_eq!(doc.content.children.len(), 1);
        assert_eq!(doc.content.children[0].kind.as_str(), node::PARAGRAPH);
    }

    #[test]
    fn test_parse_bold() {
        let doc = parse_str("*bold*\n");
        let para = &doc.content.children[0];
        assert_eq!(para.children[0].kind.as_str(), node::STRONG);
    }

    #[test]
    fn test_parse_italic() {
        let doc = parse_str("_italic_\n");
        let para = &doc.content.children[0];
        assert_eq!(para.children[0].kind.as_str(), node::EMPHASIS);
    }

    #[test]
    fn test_parse_code() {
        let doc = parse_str("`code`\n");
        let para = &doc.content.children[0];
        assert_eq!(para.children[0].kind.as_str(), node::CODE);
    }

    #[test]
    fn test_parse_wiki_link() {
        let doc = parse_str("[[MyPage]]\n");
        let para = &doc.content.children[0];
        let link = &para.children[0];
        assert_eq!(link.kind.as_str(), node::LINK);
        assert_eq!(link.props.get_str(prop::URL), Some("MyPage"));
    }

    #[test]
    fn test_parse_wiki_link_with_description() {
        let doc = parse_str("[[MyPage|click here]]\n");
        let para = &doc.content.children[0];
        let link = &para.children[0];
        assert_eq!(link.kind.as_str(), node::LINK);
        assert_eq!(link.props.get_str(prop::URL), Some("MyPage"));
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
    fn test_parse_ordered_list() {
        let doc = parse_str("1. first\n2. second\n");
        let list = &doc.content.children[0];
        assert_eq!(list.kind.as_str(), node::LIST);
        assert_eq!(list.props.get_bool(prop::ORDERED), Some(true));
    }

    #[test]
    fn test_parse_preformatted() {
        let doc = parse_str("{{{\ncode here\n}}}\n");
        assert_eq!(doc.content.children[0].kind.as_str(), node::CODE_BLOCK);
    }

    #[test]
    fn test_parse_checkbox() {
        let doc = parse_str("* [ ] unchecked\n* [X] checked\n");
        let list = &doc.content.children[0];
        assert_eq!(list.children[0].props.get_bool("checked"), Some(false));
        assert_eq!(list.children[1].props.get_bool("checked"), Some(true));
    }
}
