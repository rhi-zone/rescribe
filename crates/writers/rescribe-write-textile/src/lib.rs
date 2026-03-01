//! Textile markup writer for rescribe.
//!
//! Thin adapter converting rescribe document model to textile-fmt AST and building output.

use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node};
use rescribe_std::{node, prop};
use textile_fmt::{Block, Inline, TableCell, TableRow, TextileDoc, build as build_textile};

/// Emit a document as Textile markup.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    emit_with_options(doc, &EmitOptions::default())
}

/// Emit a document as Textile markup with custom options.
pub fn emit_with_options(
    doc: &Document,
    _options: &EmitOptions,
) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let blocks = doc
        .content
        .children
        .iter()
        .map(convert_node_to_block)
        .collect::<Vec<_>>();

    let textile_doc = TextileDoc { blocks };
    let output = build_textile(&textile_doc);

    Ok(ConversionResult::ok(output.into_bytes()))
}

fn convert_node_to_block(node: &Node) -> Block {
    match node.kind.as_str() {
        node::HEADING => {
            let level = node.props.get_int(prop::LEVEL).unwrap_or(1).min(6) as u8;
            let inlines = node.children.iter().map(convert_node_to_inline).collect();
            Block::Heading { level, inlines }
        }

        node::PARAGRAPH => {
            let inlines = node.children.iter().map(convert_node_to_inline).collect();
            Block::Paragraph { inlines }
        }

        node::CODE_BLOCK => {
            let content = node
                .props
                .get_str(prop::CONTENT)
                .map(|s| s.to_string())
                .unwrap_or_default();
            Block::CodeBlock { content }
        }

        node::BLOCKQUOTE => {
            // Extract text from blockquote children (usually paragraphs)
            let mut inlines = Vec::new();
            for child in &node.children {
                if child.kind.as_str() == node::PARAGRAPH {
                    inlines.extend(child.children.iter().map(convert_node_to_inline));
                }
            }
            Block::Blockquote { inlines }
        }

        node::LIST => {
            let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
            let items: Vec<Vec<Block>> = node
                .children
                .iter()
                .filter(|child| child.kind.as_str() == node::LIST_ITEM)
                .map(|item| item.children.iter().map(convert_node_to_block).collect())
                .collect();
            Block::List { ordered, items }
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
                            let inlines: Vec<Inline> =
                                cell.children.iter().map(convert_node_to_inline).collect();
                            TableCell { is_header, inlines }
                        })
                        .collect();
                    TableRow { cells }
                })
                .collect();
            Block::Table { rows }
        }

        node::DOCUMENT => {
            let inlines: Vec<Inline> = node.children.iter().map(convert_node_to_inline).collect();
            Block::Paragraph { inlines }
        }

        node::DIV | node::SPAN => {
            let inlines: Vec<Inline> = node.children.iter().map(convert_node_to_inline).collect();
            Block::Paragraph { inlines }
        }

        node::FIGURE => {
            let inlines: Vec<Inline> = node.children.iter().map(convert_node_to_inline).collect();
            Block::Paragraph { inlines }
        }

        _ => {
            let inlines: Vec<Inline> = node.children.iter().map(convert_node_to_inline).collect();
            Block::Paragraph { inlines }
        }
    }
}

fn convert_node_to_inline(node: &Node) -> Inline {
    match node.kind.as_str() {
        node::TEXT => {
            let s = node
                .props
                .get_str(prop::CONTENT)
                .map(|x| x.to_string())
                .unwrap_or_default();
            Inline::Text(s)
        }

        node::STRONG => {
            let children = node.children.iter().map(convert_node_to_inline).collect();
            Inline::Bold(children)
        }

        node::EMPHASIS => {
            let children = node.children.iter().map(convert_node_to_inline).collect();
            Inline::Italic(children)
        }

        node::UNDERLINE => {
            let children = node.children.iter().map(convert_node_to_inline).collect();
            Inline::Underline(children)
        }

        node::STRIKEOUT => {
            let children = node.children.iter().map(convert_node_to_inline).collect();
            Inline::Strikethrough(children)
        }

        node::CODE => {
            let s = node
                .props
                .get_str(prop::CONTENT)
                .map(|x| x.to_string())
                .unwrap_or_default();
            Inline::Code(s)
        }

        node::LINK => {
            let url = node
                .props
                .get_str(prop::URL)
                .map(|x| x.to_string())
                .unwrap_or_default();
            let children = node.children.iter().map(convert_node_to_inline).collect();
            Inline::Link { url, children }
        }

        node::IMAGE => {
            let url = node
                .props
                .get_str(prop::URL)
                .map(|x| x.to_string())
                .unwrap_or_default();
            let alt = node.props.get_str(prop::ALT).map(|x| x.to_string());
            Inline::Image { url, alt }
        }

        node::SUPERSCRIPT => {
            let children = node.children.iter().map(convert_node_to_inline).collect();
            Inline::Superscript(children)
        }

        node::SUBSCRIPT => {
            let children = node.children.iter().map(convert_node_to_inline).collect();
            Inline::Subscript(children)
        }

        node::LINE_BREAK => Inline::Text("\n".to_string()),

        node::SOFT_BREAK => Inline::Text(" ".to_string()),

        _ => {
            let children: Vec<Inline> = node.children.iter().map(convert_node_to_inline).collect();
            if children.is_empty() {
                Inline::Text(String::new())
            } else if children.len() == 1 {
                children.into_iter().next().unwrap()
            } else {
                // Wrap multiple children as text sequence
                Inline::Text(String::new())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rescribe_std::builder::*;

    fn emit_str(doc: &Document) -> String {
        let result = emit(doc).unwrap();
        String::from_utf8(result.value).unwrap()
    }

    #[test]
    fn test_emit_heading() {
        let doc = doc(|d| d.heading(1, |h| h.text("Title")));
        let output = emit_str(&doc);
        assert!(output.contains("h1. Title"));
    }

    #[test]
    fn test_emit_heading_level2() {
        let doc = doc(|d| d.heading(2, |h| h.text("Subtitle")));
        let output = emit_str(&doc);
        assert!(output.contains("h2. Subtitle"));
    }

    #[test]
    fn test_emit_paragraph() {
        let doc = doc(|d| d.para(|p| p.text("Hello, world!")));
        let output = emit_str(&doc);
        assert!(output.contains("Hello, world!"));
    }

    #[test]
    fn test_emit_bold() {
        let doc = doc(|d| d.para(|p| p.strong(|s| s.text("bold"))));
        let output = emit_str(&doc);
        assert!(output.contains("*bold*"));
    }

    #[test]
    fn test_emit_italic() {
        let doc = doc(|d| d.para(|p| p.em(|e| e.text("italic"))));
        let output = emit_str(&doc);
        assert!(output.contains("_italic_"));
    }

    #[test]
    fn test_emit_code() {
        let doc = doc(|d| d.para(|p| p.code("code")));
        let output = emit_str(&doc);
        assert!(output.contains("@code@"));
    }

    #[test]
    fn test_emit_link() {
        let doc = doc(|d| d.para(|p| p.link("https://example.com", |l| l.text("click"))));
        let output = emit_str(&doc);
        assert!(output.contains("\"click\":https://example.com"));
    }

    #[test]
    fn test_emit_list() {
        let doc = doc(|d| d.bullet_list(|l| l.item(|i| i.text("one")).item(|i| i.text("two"))));
        let output = emit_str(&doc);
        assert!(output.contains("* one"));
        assert!(output.contains("* two"));
    }

    #[test]
    fn test_emit_ordered_list() {
        let doc =
            doc(|d| d.ordered_list(|l| l.item(|i| i.text("first")).item(|i| i.text("second"))));
        let output = emit_str(&doc);
        assert!(output.contains("# first"));
        assert!(output.contains("# second"));
    }

    #[test]
    fn test_emit_code_block() {
        let doc = doc(|d| d.code_block("print('hi')"));
        let output = emit_str(&doc);
        assert!(output.contains("bc. "));
        assert!(output.contains("print('hi')"));
    }
}
