//! TWiki writer for rescribe.
//!
//! Thin adapter layer that serializes rescribe's document IR to TWiki markup.

use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node};
use rescribe_std::{node, prop};
use twiki::{self, Block, Inline, Span, TableCell, TableRow, TwikiDoc};

/// Emit a document to TWiki markup.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    emit_with_options(doc, &EmitOptions::default())
}

/// Emit a document to TWiki markup with options.
pub fn emit_with_options(
    doc: &Document,
    _options: &EmitOptions,
) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let blocks: Vec<Block> = doc
        .content
        .children
        .iter()
        .filter_map(node_to_block)
        .collect();

    let twiki_doc = TwikiDoc { blocks, span: Span::NONE };
    let output = twiki::build(&twiki_doc);
    Ok(ConversionResult::ok(output.into_bytes()))
}

fn node_to_block(node: &Node) -> Option<Block> {
    match node.kind.as_str() {
        node::DOCUMENT => {
            // Document nodes should have been flattened; skip
            None
        }

        node::HEADING => {
            let level = node.props.get_int(prop::LEVEL).unwrap_or(1) as u8;
            Some(Block::Heading {
                level,
                inlines: node_children_to_inlines(&node.children),
                span: Span::NONE,
            })
        }

        node::PARAGRAPH => Some(Block::Paragraph {
            inlines: node_children_to_inlines(&node.children),
            span: Span::NONE,
        }),

        node::CODE_BLOCK => {
            let content = node
                .props
                .get_str(prop::CONTENT)
                .map(|s| s.to_string())
                .unwrap_or_default();
            Some(Block::CodeBlock { content, span: Span::NONE })
        }

        node::LIST => {
            let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
            let items: Vec<Vec<Inline>> = node
                .children
                .iter()
                .filter(|child| child.kind.as_str() == node::LIST_ITEM)
                .map(|item| {
                    item.children
                        .iter()
                        .find(|c| c.kind.as_str() == node::PARAGRAPH)
                        .map(|para| node_children_to_inlines(&para.children))
                        .unwrap_or_default()
                })
                .collect();
            Some(Block::List { ordered, items, span: Span::NONE })
        }

        node::TABLE => {
            let rows: Vec<TableRow> = node
                .children
                .iter()
                .filter(|child| child.kind.as_str() == node::TABLE_ROW)
                .map(|row| {
                    let cells: Vec<TableCell> = row
                        .children
                        .iter()
                        .map(|cell| {
                            let is_header = cell.kind.as_str() == node::TABLE_HEADER;
                            let inlines = node_children_to_inlines(&cell.children);
                            TableCell { inlines, is_header, span: Span::NONE }
                        })
                        .collect();
                    TableRow { cells, span: Span::NONE }
                })
                .collect();
            Some(Block::Table { rows, span: Span::NONE })
        }

        node::HORIZONTAL_RULE => Some(Block::HorizontalRule { span: Span::NONE }),

        node::DIV | node::SPAN | node::FIGURE => {
            // These are container nodes; flatten their children
            None
        }

        _ => None,
    }
}

fn node_children_to_inlines(nodes: &[Node]) -> Vec<Inline> {
    let mut inlines = Vec::new();
    for node in nodes {
        if let Some(inline) = node_to_inline(node) {
            inlines.push(inline);
        }
    }
    inlines
}

fn node_to_inline(node: &Node) -> Option<Inline> {
    match node.kind.as_str() {
        node::TEXT => {
            let content = node
                .props
                .get_str(prop::CONTENT)
                .map(|s| s.to_string())
                .unwrap_or_default();
            Some(Inline::Text(content, Span::NONE))
        }

        node::STRONG => {
            // Check if child is emphasis (bold italic)
            if node.children.len() == 1 && node.children[0].kind.as_str() == node::EMPHASIS {
                let children = node_children_to_inlines(&node.children[0].children);
                Some(Inline::BoldItalic(children, Span::NONE))
            } else if node.children.len() == 1 && node.children[0].kind.as_str() == node::CODE {
                // Bold code
                let children = node_children_to_inlines(&node.children);
                Some(Inline::BoldCode(children, Span::NONE))
            } else {
                let children = node_children_to_inlines(&node.children);
                Some(Inline::Bold(children, Span::NONE))
            }
        }

        node::EMPHASIS => {
            let children = node_children_to_inlines(&node.children);
            Some(Inline::Italic(children, Span::NONE))
        }

        node::CODE => {
            let content = node
                .props
                .get_str(prop::CONTENT)
                .map(|s| s.to_string())
                .unwrap_or_default();
            Some(Inline::Code(content, Span::NONE))
        }

        node::LINK => {
            let url = node
                .props
                .get_str(prop::URL)
                .map(|s| s.to_string())
                .unwrap_or_default();
            let label = node
                .children
                .iter()
                .find(|c| c.kind.as_str() == node::TEXT)
                .and_then(|c| c.props.get_str(prop::CONTENT))
                .map(|s| s.to_string())
                .unwrap_or_else(|| url.clone());
            Some(Inline::Link { url, label, span: Span::NONE })
        }

        node::LINE_BREAK => Some(Inline::LineBreak { span: Span::NONE }),
        node::SOFT_BREAK => Some(Inline::Text(" ".to_string(), Span::NONE)),

        _ => None,
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
        assert!(emit_str(&doc).contains("---+ Title"));
    }

    #[test]
    fn test_emit_bold() {
        let doc = doc(|d| d.para(|p| p.strong(|s| s.text("bold"))));
        assert!(emit_str(&doc).contains("*bold*"));
    }

    #[test]
    fn test_emit_italic() {
        let doc = doc(|d| d.para(|p| p.em(|e| e.text("italic"))));
        assert!(emit_str(&doc).contains("_italic_"));
    }

    #[test]
    fn test_emit_link() {
        let doc = doc(|d| d.para(|p| p.link("http://example.com", |l| l.text("Example"))));
        assert!(emit_str(&doc).contains("[[http://example.com][Example]]"));
    }

    #[test]
    fn test_emit_list() {
        let doc = doc(|d| d.bullet_list(|l| l.item(|i| i.text("one")).item(|i| i.text("two"))));
        let output = emit_str(&doc);
        assert!(output.contains("   * one"));
        assert!(output.contains("   * two"));
    }
}
