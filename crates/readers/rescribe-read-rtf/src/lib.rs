//! RTF (Rich Text Format) reader for rescribe.
//!
//! Thin adapter over [`rtf_fmt`]: parses RTF into the `rtf_fmt` AST,
//! then maps it to the rescribe document model.

use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions};
use rescribe_std::{node, prop};
use rtf_fmt::{Block, Inline, RtfDoc};

/// Parse an RTF document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse an RTF document with custom options.
pub fn parse_with_options(
    input: &str,
    _options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let (rtf, _diagnostics) = rtf_fmt::parse(input);
    let nodes = doc_to_nodes(&rtf);
    let root = Node::new(node::DOCUMENT).children(nodes);
    let doc = Document::new().with_content(root);
    Ok(ConversionResult::ok(doc))
}

fn doc_to_nodes(rtf: &RtfDoc) -> Vec<Node> {
    rtf.blocks.iter().map(block_to_node).collect()
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

        Block::Blockquote { children, .. } => {
            Node::new(node::BLOCKQUOTE).children(children.iter().map(block_to_node))
        }

        Block::List { ordered, items, .. } => {
            let list_items: Vec<Node> = items
                .iter()
                .map(|item_blocks| {
                    Node::new(node::LIST_ITEM).children(item_blocks.iter().map(block_to_node))
                })
                .collect();
            Node::new(node::LIST)
                .prop(prop::ORDERED, *ordered)
                .children(list_items)
        }

        Block::Table { rows, .. } => {
            let row_nodes: Vec<Node> = rows
                .iter()
                .map(|row| {
                    let cells: Vec<Node> = row
                        .cells
                        .iter()
                        .map(|cell| Node::new(node::TABLE_CELL).children(inlines_to_nodes(cell)))
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
    inlines.iter().map(inline_to_node).collect()
}

fn inline_to_node(inline: &Inline) -> Node {
    match inline {
        Inline::Text { text, .. } => Node::new(node::TEXT).prop(prop::CONTENT, text.clone()),

        Inline::Bold { children, .. } => {
            Node::new(node::STRONG).children(inlines_to_nodes(children))
        }

        Inline::Italic { children, .. } => {
            Node::new(node::EMPHASIS).children(inlines_to_nodes(children))
        }

        Inline::Underline { children, .. } => {
            Node::new(node::UNDERLINE).children(inlines_to_nodes(children))
        }

        Inline::Strikethrough { children, .. } => {
            Node::new(node::STRIKEOUT).children(inlines_to_nodes(children))
        }

        Inline::Code { text, .. } => Node::new(node::CODE).prop(prop::CONTENT, text.clone()),

        Inline::Link { url, children, .. } => Node::new(node::LINK)
            .prop(prop::URL, url.clone())
            .children(inlines_to_nodes(children)),

        Inline::Image { url, alt, .. } => Node::new(node::IMAGE)
            .prop(prop::URL, url.clone())
            .prop(prop::ALT, alt.clone()),

        Inline::LineBreak { .. } => Node::new(node::LINE_BREAK),

        Inline::SoftBreak { .. } => Node::new(node::SOFT_BREAK),

        Inline::Superscript { children, .. } => {
            Node::new(node::SUPERSCRIPT).children(inlines_to_nodes(children))
        }

        Inline::Subscript { children, .. } => {
            Node::new(node::SUBSCRIPT).children(inlines_to_nodes(children))
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
    fn test_parse_simple_text() {
        let doc = parse_str(r"{\rtf1 Hello world\par}");
        assert_eq!(doc.content.children.len(), 1);
        assert_eq!(doc.content.children[0].kind.as_str(), node::PARAGRAPH);
    }

    #[test]
    fn test_parse_bold() {
        let doc = parse_str(r"{\rtf1 \b bold text\b0 normal\par}");
        let para = &doc.content.children[0];
        assert!(
            para.children
                .iter()
                .any(|n| n.kind.as_str() == node::STRONG)
        );
    }

    #[test]
    fn test_parse_italic() {
        let doc = parse_str(r"{\rtf1 \i italic\i0\par}");
        let para = &doc.content.children[0];
        assert!(
            para.children
                .iter()
                .any(|n| n.kind.as_str() == node::EMPHASIS)
        );
    }

    #[test]
    fn test_parse_underline() {
        let doc = parse_str(r"{\rtf1 \ul underlined\ulnone\par}");
        let para = &doc.content.children[0];
        assert!(
            para.children
                .iter()
                .any(|n| n.kind.as_str() == node::UNDERLINE)
        );
    }

    #[test]
    fn test_parse_multiple_paragraphs() {
        let doc = parse_str(r"{\rtf1 First paragraph\par Second paragraph\par}");
        assert_eq!(doc.content.children.len(), 2);
    }

    #[test]
    fn test_parse_escaped_chars() {
        let doc = parse_str(r"{\rtf1 Open \{ and close \}\par}");
        let para = &doc.content.children[0];
        let text = get_all_text(para);
        assert!(text.contains('{'));
        assert!(text.contains('}'));
    }

    #[test]
    fn test_parse_special_chars() {
        let doc = parse_str(r"{\rtf1 Em\emdash dash\par}");
        let para = &doc.content.children[0];
        let text = get_all_text(para);
        assert!(text.contains('\u{2014}'));
    }

    fn get_all_text(node: &Node) -> String {
        let mut text = String::new();
        if let Some(content) = node.props.get_str(prop::CONTENT) {
            text.push_str(content);
        }
        for child in &node.children {
            text.push_str(&get_all_text(child));
        }
        text
    }
}
