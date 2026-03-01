//! MediaWiki writer for rescribe.
//!
//! Emits rescribe's document IR as MediaWiki markup.
//! Uses `mediawiki-fmt` crate for format building and adapts from rescribe IR.
//!
//! # Example
//!
//! ```ignore
//! use rescribe_write_mediawiki::emit;
//!
//! let doc = Document::new();
//! let result = emit(&doc)?;
//! let wiki = String::from_utf8(result.value).unwrap();
//! ```

use mediawiki_fmt::{Block, Inline, MediawikiDoc, TableCell, TableRow, build as build_mediawiki};
use rescribe_core::{ConversionResult, Document, EmitError, Node};
use rescribe_std::{node, prop};

/// Emit a document as MediaWiki markup.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let blocks = doc
        .content
        .children
        .iter()
        .flat_map(node_to_block)
        .collect();

    let fmt_doc = MediawikiDoc { blocks };
    let output = build_mediawiki(&fmt_doc);

    Ok(ConversionResult::with_warnings(output.into_bytes(), vec![]))
}

fn node_to_block(node: &Node) -> Vec<Block> {
    match node.kind.as_str() {
        node::PARAGRAPH => {
            let inlines = node.children.iter().flat_map(node_to_inline).collect();
            vec![Block::Paragraph { inlines }]
        }

        node::HEADING => {
            let level = node.props.get_int(prop::LEVEL).unwrap_or(1) as u8;
            let inlines = node.children.iter().flat_map(node_to_inline).collect();
            vec![Block::Heading { level, inlines }]
        }

        node::CODE_BLOCK => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            vec![Block::CodeBlock { content }]
        }

        node::LIST => {
            let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
            let items = node
                .children
                .iter()
                .filter(|child| child.kind.as_str() == node::LIST_ITEM)
                .map(|item_node| item_node.children.iter().flat_map(node_to_block).collect())
                .collect();
            vec![Block::List { ordered, items }]
        }

        node::BLOCKQUOTE => {
            // Flatten blockquote into its children for MediaWiki output
            node.children
                .iter()
                .flat_map(node_to_block)
                .collect::<Vec<_>>()
        }

        node::HORIZONTAL_RULE => {
            vec![Block::HorizontalRule]
        }

        node::TABLE => {
            let rows = node
                .children
                .iter()
                .filter(|child| child.kind.as_str() == node::TABLE_ROW)
                .map(|row_node| {
                    let cells = row_node
                        .children
                        .iter()
                        .map(|cell_node| {
                            let is_header = cell_node.kind.as_str() == node::TABLE_HEADER;
                            let inlines =
                                cell_node.children.iter().flat_map(node_to_inline).collect();
                            TableCell { is_header, inlines }
                        })
                        .collect();
                    TableRow { cells }
                })
                .collect();
            vec![Block::Table { rows }]
        }

        _ => {
            // Skip unknown block types
            vec![]
        }
    }
}

fn node_to_inline(node: &Node) -> Vec<Inline> {
    match node.kind.as_str() {
        node::TEXT => {
            if let Some(content) = node.props.get_str(prop::CONTENT) {
                vec![Inline::Text(content.to_string())]
            } else {
                vec![]
            }
        }

        node::STRONG => {
            let children = node.children.iter().flat_map(node_to_inline).collect();
            vec![Inline::Bold(children)]
        }

        node::EMPHASIS => {
            let children = node.children.iter().flat_map(node_to_inline).collect();
            vec![Inline::Italic(children)]
        }

        node::CODE => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            vec![Inline::Code(content)]
        }

        node::LINK => {
            let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
            let text = extract_text(node);
            vec![Inline::Link { url, text }]
        }

        node::IMAGE => {
            let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
            let alt = node.props.get_str(prop::ALT).unwrap_or("").to_string();
            vec![Inline::Image { url, alt }]
        }

        node::LINE_BREAK => {
            vec![Inline::LineBreak]
        }

        node::STRIKEOUT => {
            let children = node.children.iter().flat_map(node_to_inline).collect();
            vec![Inline::Strikeout(children)]
        }

        node::UNDERLINE => {
            let children = node.children.iter().flat_map(node_to_inline).collect();
            vec![Inline::Underline(children)]
        }

        node::SUBSCRIPT => {
            let children = node.children.iter().flat_map(node_to_inline).collect();
            vec![Inline::Subscript(children)]
        }

        node::SUPERSCRIPT => {
            let children = node.children.iter().flat_map(node_to_inline).collect();
            vec![Inline::Superscript(children)]
        }

        node::SOFT_BREAK => {
            vec![Inline::Text(" ".to_string())]
        }

        _ => {
            // Recursively emit children
            node.children.iter().flat_map(node_to_inline).collect()
        }
    }
}

fn extract_text(node: &Node) -> String {
    let mut result = String::new();
    extract_text_recursive(node, &mut result);
    result
}

fn extract_text_recursive(node: &Node, output: &mut String) {
    if node.kind.as_str() == node::TEXT
        && let Some(content) = node.props.get_str(prop::CONTENT)
    {
        output.push_str(content);
    }
    for child in &node.children {
        extract_text_recursive(child, output);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rescribe_std::builder::doc;

    #[test]
    fn test_emit_heading() {
        let document = doc(|d| d.heading(2, |i| i.text("Title")));
        let result = emit(&document).unwrap();
        let output = String::from_utf8(result.value).unwrap();
        assert!(output.contains("== Title =="));
    }

    #[test]
    fn test_emit_bold() {
        let document = doc(|d| d.para(|i| i.strong(|i| i.text("bold"))));
        let result = emit(&document).unwrap();
        let output = String::from_utf8(result.value).unwrap();
        assert!(output.contains("'''bold'''"));
    }

    #[test]
    fn test_emit_italic() {
        let document = doc(|d| d.para(|i| i.em(|i| i.text("italic"))));
        let result = emit(&document).unwrap();
        let output = String::from_utf8(result.value).unwrap();
        assert!(output.contains("''italic''"));
    }

    #[test]
    fn test_emit_list() {
        let document =
            doc(|d| d.bullet_list(|l| l.item(|i| i.text("Item 1")).item(|i| i.text("Item 2"))));
        let result = emit(&document).unwrap();
        let output = String::from_utf8(result.value).unwrap();
        assert!(output.contains("* Item 1"));
        assert!(output.contains("* Item 2"));
    }

    #[test]
    fn test_emit_link() {
        let document = doc(|d| d.para(|i| i.link("https://example.com", |i| i.text("Example"))));
        let result = emit(&document).unwrap();
        let output = String::from_utf8(result.value).unwrap();
        assert!(output.contains("[https://example.com Example]"));
    }
}
