//! MediaWiki reader for rescribe.
//!
//! Parses MediaWiki markup into rescribe's document IR.
//! Uses `mediawiki-fmt` crate for format parsing and adapts to rescribe IR.
//!
//! # Example
//!
//! ```
//! use rescribe_read_mediawiki::parse;
//!
//! let result = parse("== Heading ==\n\nSome '''bold''' text.").unwrap();
//! let doc = result.value;
//! ```

use mediawiki_fmt::{Block, Inline, parse as parse_mediawiki};
use rescribe_core::{ConversionResult, Document, Node, ParseError, Properties};
use rescribe_std::{node, prop};

/// Parse MediaWiki text into a document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    let (fmt_doc, _diags) = parse_mediawiki(input);

    let children: Vec<Node> = fmt_doc.blocks.iter().map(block_to_node).collect();

    let document = Document {
        content: Node::new(node::DOCUMENT).children(children),
        resources: Default::default(),
        metadata: Properties::new(),
        source: None,
    };

    Ok(ConversionResult::with_warnings(document, vec![]))
}

fn block_to_node(block: &Block) -> Node {
    match block {
        Block::Paragraph { inlines, .. } => {
            let children: Vec<Node> = inlines.iter().map(inline_to_node).collect();
            Node::new(node::PARAGRAPH).children(children)
        }

        Block::Heading { level, inlines, .. } => {
            let children: Vec<Node> = inlines.iter().map(inline_to_node).collect();
            Node::new(node::HEADING)
                .prop(prop::LEVEL, *level as i64)
                .children(children)
        }

        Block::CodeBlock { language, content, .. } => {
            let mut n = Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone());
            if let Some(lang) = language {
                n = n.prop(prop::LANGUAGE, lang.clone());
            }
            n
        }

        Block::List { ordered, items, .. } => {
            let children: Vec<Node> = items
                .iter()
                .map(|item_blocks| {
                    let para_children: Vec<Node> = item_blocks
                        .iter()
                        .flat_map(|block| {
                            if let Block::Paragraph { inlines, .. } = block {
                                inlines.iter().map(inline_to_node).collect::<Vec<_>>()
                            } else {
                                vec![block_to_node(block)]
                            }
                        })
                        .collect();

                    Node::new(node::LIST_ITEM)
                        .child(Node::new(node::PARAGRAPH).children(para_children))
                })
                .collect();

            Node::new(node::LIST)
                .prop(prop::ORDERED, *ordered)
                .children(children)
        }

        Block::DefinitionList { items, .. } => {
            let children: Vec<Node> = items
                .iter()
                .flat_map(|item| {
                    let term_children: Vec<Node> = item.term.iter().map(inline_to_node).collect();
                    let desc_children: Vec<Node> = item.desc.iter().map(inline_to_node).collect();
                    vec![
                        Node::new(node::DEFINITION_TERM).children(term_children),
                        Node::new(node::DEFINITION_DESC).children(desc_children),
                    ]
                })
                .collect();
            Node::new(node::DEFINITION_LIST).children(children)
        }

        Block::HorizontalRule => Node::new(node::HORIZONTAL_RULE),

        Block::Blockquote { children, .. } => {
            let child_nodes: Vec<Node> = children.iter().map(block_to_node).collect();
            Node::new(node::BLOCKQUOTE).children(child_nodes)
        }

        Block::PreBlock { content, .. } => {
            Node::new("pre_block").prop(prop::CONTENT, content.clone())
        }

        Block::RawBlock { content, .. } => {
            Node::new(node::RAW_BLOCK)
                .prop(prop::FORMAT, "mediawiki")
                .prop(prop::CONTENT, content.clone())
        }

        Block::Table { rows, .. } => {
            let children: Vec<Node> = rows
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
                            let children: Vec<Node> =
                                cell.inlines.iter().map(inline_to_node).collect();
                            Node::new(kind).children(children)
                        })
                        .collect();
                    Node::new(node::TABLE_ROW).children(cells)
                })
                .collect();

            Node::new(node::TABLE).children(children)
        }
    }
}

fn inline_to_node(inline: &Inline) -> Node {
    match inline {
        Inline::Text(s) => Node::new(node::TEXT).prop(prop::CONTENT, s.clone()),

        Inline::Bold(children) => {
            let child_nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
            Node::new(node::STRONG).children(child_nodes)
        }

        Inline::Italic(children) => {
            let child_nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
            Node::new(node::EMPHASIS).children(child_nodes)
        }

        Inline::Code(s) => Node::new(node::CODE).prop(prop::CONTENT, s.clone()),

        Inline::Link { url, text } => Node::new(node::LINK)
            .prop(prop::URL, url.clone())
            .child(Node::new(node::TEXT).prop(prop::CONTENT, text.clone())),

        Inline::Image { url, alt } => Node::new(node::IMAGE)
            .prop(prop::URL, url.clone())
            .prop(prop::ALT, alt.clone()),

        Inline::LineBreak => Node::new(node::LINE_BREAK),

        Inline::Strikeout(children) => {
            let child_nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
            Node::new(node::STRIKEOUT).children(child_nodes)
        }

        Inline::Underline(children) => {
            let child_nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
            Node::new(node::UNDERLINE).children(child_nodes)
        }

        Inline::Subscript(children) => {
            let child_nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
            Node::new(node::SUBSCRIPT).children(child_nodes)
        }

        Inline::Superscript(children) => {
            let child_nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
            Node::new(node::SUPERSCRIPT).children(child_nodes)
        }

        Inline::FootnoteRef { label, content } => {
            let mut n = Node::new(node::FOOTNOTE_REF).prop(prop::LABEL, label.clone());
            if let Some(c) = content {
                n = n.prop(prop::CONTENT, c.clone());
            }
            n
        }

        Inline::MathInline { source } => {
            Node::new("math_inline").prop(prop::CONTENT, source.clone())
        }

        Inline::Template { content } => {
            Node::new("template").prop(prop::CONTENT, content.clone())
        }

        Inline::Nowiki { content } => {
            Node::new("nowiki").prop(prop::CONTENT, content.clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_heading() {
        let result = parse("== Heading ==").unwrap();
        let doc = result.value;
        assert_eq!(doc.content.children.len(), 1);
        let heading = &doc.content.children[0];
        assert_eq!(heading.kind.as_str(), node::HEADING);
        assert_eq!(heading.props.get_int(prop::LEVEL), Some(2));
    }

    #[test]
    fn test_parse_bold() {
        let result = parse("'''bold'''").unwrap();
        let doc = result.value;
        let para = &doc.content.children[0];
        let strong = &para.children[0];
        assert_eq!(strong.kind.as_str(), node::STRONG);
    }

    #[test]
    fn test_parse_italic() {
        let result = parse("''italic''").unwrap();
        let doc = result.value;
        let para = &doc.content.children[0];
        let em = &para.children[0];
        assert_eq!(em.kind.as_str(), node::EMPHASIS);
    }

    #[test]
    fn test_parse_list() {
        let result = parse("* Item 1\n* Item 2").unwrap();
        let doc = result.value;
        let list = &doc.content.children[0];
        assert_eq!(list.kind.as_str(), node::LIST);
        assert_eq!(list.props.get_bool(prop::ORDERED), Some(false));
        assert_eq!(list.children.len(), 2);
    }

    #[test]
    fn test_parse_link() {
        let result = parse("[[Title|Link text]]").unwrap();
        let doc = result.value;
        let para = &doc.content.children[0];
        let link = &para.children[0];
        assert_eq!(link.kind.as_str(), node::LINK);
        assert_eq!(link.props.get_str(prop::URL), Some("Title"));
    }
}
