//! XWiki writer for rescribe.
//!
//! Thin adapter layer that converts rescribe's document IR to XWiki format.

use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node};
use rescribe_std::{node, prop};
use xwiki::{self, Block, Inline};

/// Emit a document to XWiki markup.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    emit_with_options(doc, &EmitOptions::default())
}

/// Emit a document to XWiki markup with options.
pub fn emit_with_options(
    doc: &Document,
    _options: &EmitOptions,
) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let blocks: Vec<Block> = doc.content.children.iter().map(node_to_block).collect();
    let xwiki_doc = xwiki::XwikiDoc { blocks };
    let output = xwiki::build(&xwiki_doc);
    Ok(ConversionResult::ok(output.into_bytes()))
}

fn node_to_block(node: &Node) -> Block {
    match node.kind.as_str() {
        node::HEADING => {
            let level = node.props.get_int(prop::LEVEL).unwrap_or(1) as u8;
            Block::Heading {
                level: level.min(6),
                inlines: nodes_to_inlines(&node.children),
            }
        }

        node::PARAGRAPH => Block::Paragraph {
            inlines: nodes_to_inlines(&node.children),
        },

        node::CODE_BLOCK => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            let language = node.props.get_str(prop::LANGUAGE).map(|s| s.to_string());
            Block::CodeBlock { content, language }
        }

        node::TABLE => {
            let rows = node
                .children
                .iter()
                .filter(|child| child.kind.as_str() == node::TABLE_ROW)
                .map(|row| {
                    let cells = row
                        .children
                        .iter()
                        .map(|cell| {
                            let is_header = cell.kind.as_str() == node::TABLE_HEADER;
                            xwiki::TableCell {
                                is_header,
                                inlines: nodes_to_inlines(&cell.children),
                            }
                        })
                        .collect();
                    xwiki::TableRow { cells }
                })
                .collect();
            Block::Table { rows }
        }

        node::LIST => {
            let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
            let items = node
                .children
                .iter()
                .filter(|child| child.kind.as_str() == node::LIST_ITEM)
                .map(|item| item.children.iter().map(node_to_block).collect())
                .collect();
            Block::List { ordered, items }
        }

        node::HORIZONTAL_RULE => Block::HorizontalRule,

        _ => {
            // For unhandled node kinds, recursively process children
            let blocks: Vec<Block> = node.children.iter().map(node_to_block).collect();
            if blocks.len() == 1 {
                blocks.into_iter().next().unwrap()
            } else if !blocks.is_empty() {
                blocks[0].clone()
            } else {
                Block::Paragraph { inlines: vec![] }
            }
        }
    }
}

fn nodes_to_inlines(nodes: &[Node]) -> Vec<Inline> {
    nodes.iter().map(node_to_inline).collect()
}

fn node_to_inline(node: &Node) -> Inline {
    match node.kind.as_str() {
        node::TEXT => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            Inline::Text(content)
        }

        node::STRONG => Inline::Bold(nodes_to_inlines(&node.children)),

        node::EMPHASIS => Inline::Italic(nodes_to_inlines(&node.children)),

        node::UNDERLINE => Inline::Underline(nodes_to_inlines(&node.children)),

        node::STRIKEOUT => Inline::Strikeout(nodes_to_inlines(&node.children)),

        node::CODE => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            Inline::Code(content)
        }

        node::LINK => {
            let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
            let label = if node.children.is_empty() {
                url.clone()
            } else {
                nodes_to_inlines(&node.children)
                    .iter()
                    .map(|i| match i {
                        Inline::Text(s) => s.clone(),
                        _ => String::new(),
                    })
                    .collect::<Vec<_>>()
                    .join("")
            };
            Inline::Link { url, label }
        }

        node::IMAGE => {
            let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
            Inline::Image { url }
        }

        node::LINE_BREAK => Inline::LineBreak,

        node::SOFT_BREAK => Inline::SoftBreak,

        _ => Inline::Text(String::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rescribe_std::builder::*;

    fn emit_str(doc: &Document) -> String {
        String::from_utf8(emit(doc).unwrap().value).unwrap()
    }

    #[test]
    fn test_emit_heading() {
        let doc = doc(|d| d.heading(1, |h| h.text("Title")));
        assert!(emit_str(&doc).contains("= Title ="));
    }

    #[test]
    fn test_emit_bold() {
        let doc = doc(|d| d.para(|p| p.strong(|s| s.text("bold"))));
        assert!(emit_str(&doc).contains("**bold**"));
    }

    #[test]
    fn test_emit_italic() {
        let doc = doc(|d| d.para(|p| p.em(|e| e.text("italic"))));
        assert!(emit_str(&doc).contains("//italic//"));
    }

    #[test]
    fn test_emit_link() {
        let doc = doc(|d| d.para(|p| p.link("http://example.com", |l| l.text("Example"))));
        assert!(emit_str(&doc).contains("[[Example>>http://example.com]]"));
    }

    #[test]
    fn test_emit_list() {
        let doc = doc(|d| d.bullet_list(|l| l.item(|i| i.text("one")).item(|i| i.text("two"))));
        let output = emit_str(&doc);
        assert!(output.contains("* one"));
        assert!(output.contains("* two"));
    }
}
