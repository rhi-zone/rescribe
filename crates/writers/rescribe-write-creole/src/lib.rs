//! Creole wiki markup writer for rescribe.
//!
//! Thin adapter translating rescribe documents
//! to the format-independent `creole` crate, then to Creole markup.

use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node};
use rescribe_std::{node, prop};

/// Emit a document as Creole markup.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    emit_with_options(doc, &EmitOptions::default())
}

/// Emit a document as Creole markup with custom options.
pub fn emit_with_options(
    doc: &Document,
    _options: &EmitOptions,
) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let blocks = convert_nodes(&doc.content.children);
    let creole_doc = creole::CreoleDoc { blocks };
    let output = creole::build(&creole_doc);
    Ok(ConversionResult::ok(output.into_bytes()))
}

fn convert_nodes(nodes: &[Node]) -> Vec<creole::Block> {
    nodes.iter().map(convert_node).collect()
}

fn convert_node(node: &Node) -> creole::Block {
    match node.kind.as_str() {
        node::DOCUMENT => {
            // Shouldn't happen in well-formed rescribe tree, but handle it
            let children = convert_nodes(&node.children);
            // Return the first block or an empty paragraph
            children
                .into_iter()
                .next()
                .unwrap_or_else(|| creole::Block::Paragraph {
                    inlines: vec![],
                    span: creole::Span::NONE,
                })
        }

        node::PARAGRAPH => creole::Block::Paragraph {
            inlines: convert_inlines(&node.children),
            span: creole::Span::NONE,
        },

        node::HEADING => {
            let level = (node.props.get_int(prop::LEVEL).unwrap_or(1).clamp(1, 6)) as u8;
            creole::Block::Heading {
                level,
                inlines: convert_inlines(&node.children),
                span: creole::Span::NONE,
            }
        }

        node::CODE_BLOCK => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            creole::Block::CodeBlock { content, span: creole::Span::NONE }
        }

        node::BLOCKQUOTE => creole::Block::Blockquote {
            children: convert_nodes(&node.children),
            span: creole::Span::NONE,
        },

        node::LIST => {
            let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
            let items: Vec<Vec<creole::Block>> = node
                .children
                .iter()
                .filter(|child| child.kind.as_str() == node::LIST_ITEM)
                .map(|item| convert_nodes(&item.children))
                .collect();
            creole::Block::List { ordered, items, span: creole::Span::NONE }
        }

        node::TABLE => {
            let rows: Vec<creole::TableRow> = node
                .children
                .iter()
                .filter(|child| child.kind.as_str() == node::TABLE_ROW)
                .map(|row| creole::TableRow {
                    cells: row
                        .children
                        .iter()
                        .map(|cell| creole::TableCell {
                            is_header: cell.kind.as_str() == node::TABLE_HEADER,
                            inlines: convert_inlines(&cell.children),
                            span: creole::Span::NONE,
                        })
                        .collect(),
                    span: creole::Span::NONE,
                })
                .collect();
            creole::Block::Table { rows, span: creole::Span::NONE }
        }

        node::HORIZONTAL_RULE => creole::Block::HorizontalRule(creole::Span::NONE),

        // Handle other nodes by recursing on children
        _ => creole::Block::Paragraph {
            inlines: convert_inlines(&node.children),
            span: creole::Span::NONE,
        },
    }
}

fn convert_inlines(nodes: &[Node]) -> Vec<creole::Inline> {
    nodes.iter().map(convert_inline).collect()
}

fn convert_inline(node: &Node) -> creole::Inline {
    match node.kind.as_str() {
        node::TEXT => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            creole::Inline::Text(content, creole::Span::NONE)
        }

        node::STRONG => creole::Inline::Bold(convert_inlines(&node.children), creole::Span::NONE),

        node::EMPHASIS => {
            creole::Inline::Italic(convert_inlines(&node.children), creole::Span::NONE)
        }

        node::CODE => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            creole::Inline::Code(content, creole::Span::NONE)
        }

        node::LINK => {
            let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
            let children = convert_inlines(&node.children);
            creole::Inline::Link { url, children, span: creole::Span::NONE }
        }

        node::IMAGE => {
            let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
            let alt = node.props.get_str(prop::ALT).map(|s| s.to_string());
            creole::Inline::Image { url, alt, span: creole::Span::NONE }
        }

        node::LINE_BREAK => creole::Inline::LineBreak(creole::Span::NONE),

        // Creole doesn't have strikethrough, underline, superscript, subscript
        // Just emit the children instead
        node::STRIKEOUT | node::UNDERLINE | node::SUPERSCRIPT | node::SUBSCRIPT => {
            // Wrap multiple inlines in a text node if they're all text
            let children = convert_inlines(&node.children);
            if children.len() == 1 {
                children.into_iter().next().unwrap()
            } else {
                creole::Inline::Text(format!("{:?}", children), creole::Span::NONE)
            }
        }

        // Fallback: recurse
        _ => {
            let children = convert_inlines(&node.children);
            if children.is_empty() {
                creole::Inline::Text(String::new(), creole::Span::NONE)
            } else if children.len() == 1 {
                children.into_iter().next().unwrap()
            } else {
                creole::Inline::Text(format!("{:?}", children), creole::Span::NONE)
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
        assert!(output.contains("= Title ="));
    }

    #[test]
    fn test_emit_heading_level2() {
        let doc = doc(|d| d.heading(2, |h| h.text("Subtitle")));
        let output = emit_str(&doc);
        assert!(output.contains("== Subtitle =="));
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
        assert!(output.contains("**bold**"));
    }

    #[test]
    fn test_emit_italic() {
        let doc = doc(|d| d.para(|p| p.em(|e| e.text("italic"))));
        let output = emit_str(&doc);
        assert!(output.contains("//italic//"));
    }

    #[test]
    fn test_emit_code() {
        let doc = doc(|d| d.para(|p| p.code("code")));
        let output = emit_str(&doc);
        assert!(output.contains("{{{code}}}"));
    }

    #[test]
    fn test_emit_link() {
        let doc = doc(|d| d.para(|p| p.link("https://example.com", |l| l.text("click"))));
        let output = emit_str(&doc);
        assert!(output.contains("[[https://example.com|click]]"));
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
        assert!(output.contains("{{{\n"));
        assert!(output.contains("print('hi')"));
        assert!(output.contains("}}}\n"));
    }
}
