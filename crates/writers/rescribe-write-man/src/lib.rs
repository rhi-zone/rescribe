//! Man page (roff/troff) writer for rescribe.
//!
//! Thin adapter layer around the `man-fmt` crate.
//! Emits documents as Unix man page format using common macros.

use man_fmt::{Block, Inline, ManDoc, Span};
use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node};
use rescribe_std::{node, prop};

/// Emit a document as man page format.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    emit_with_options(doc, &EmitOptions::default())
}

/// Emit a document as man page format with custom options.
pub fn emit_with_options(
    doc: &Document,
    _options: &EmitOptions,
) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let man_doc = convert_from_document(doc);
    let output = man_fmt::build(&man_doc);

    Ok(ConversionResult::ok(output.into_bytes()))
}

fn convert_from_document(doc: &Document) -> ManDoc {
    let mut man = ManDoc {
        title: doc.metadata.get_str("title").map(|s| s.to_string()),
        section: doc.metadata.get_str("man:section").map(|s| s.to_string()),
        blocks: Vec::new(),
        span: Span::NONE,
    };

    for node in &doc.content.children {
        if let Some(block) = convert_node(node) {
            man.blocks.push(block);
        }
    }

    man
}

fn convert_node(node: &Node) -> Option<Block> {
    match node.kind.as_str() {
        node::DOCUMENT => {
            // Should not appear here, but handle it gracefully
            None
        }

        node::HEADING => {
            let level = node.props.get_int(prop::LEVEL).unwrap_or(1) as u8;
            let inlines = convert_nodes_to_inlines(&node.children);
            Some(Block::Heading {
                level,
                inlines,
                span: Span::NONE,
            })
        }

        node::PARAGRAPH => {
            let inlines = convert_nodes_to_inlines(&node.children);
            Some(Block::Paragraph {
                inlines,
                span: Span::NONE,
            })
        }

        node::CODE_BLOCK => {
            let content = node
                .props
                .get_str(prop::CONTENT)
                .map(|s| s.to_string())
                .unwrap_or_default();
            Some(Block::CodeBlock {
                content,
                span: Span::NONE,
            })
        }

        node::BLOCKQUOTE => {
            // Convert blockquote children as regular blocks
            let mut items = Vec::new();
            let inlines = convert_nodes_to_inlines(&node.children);
            if !inlines.is_empty() {
                items.push((
                    vec![],
                    vec![Block::Paragraph {
                        inlines,
                        span: Span::NONE,
                    }],
                ));
            }
            if items.is_empty() {
                None
            } else {
                Some(Block::DefinitionList {
                    items,
                    span: Span::NONE,
                })
            }
        }

        node::LIST => {
            let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
            let mut items = Vec::new();
            for child in &node.children {
                if child.kind.as_str() == node::LIST_ITEM {
                    let mut item_blocks = Vec::new();
                    for list_child in &child.children {
                        if let Some(block) = convert_node(list_child) {
                            item_blocks.push(block);
                        }
                    }
                    if !item_blocks.is_empty() {
                        items.push(item_blocks);
                    }
                }
            }
            if items.is_empty() {
                None
            } else {
                Some(Block::List {
                    ordered,
                    items,
                    span: Span::NONE,
                })
            }
        }

        node::LIST_ITEM => {
            // Handled by LIST
            None
        }

        node::DEFINITION_LIST => {
            let mut items = Vec::new();
            let mut i = 0;
            while i < node.children.len() {
                let child = &node.children[i];
                if child.kind.as_str() == node::DEFINITION_TERM {
                    let term_inlines = convert_nodes_to_inlines(&child.children);
                    let mut content_blocks = Vec::new();

                    // Collect following definition_desc nodes
                    i += 1;
                    while i < node.children.len() {
                        let next = &node.children[i];
                        if next.kind.as_str() == node::DEFINITION_DESC {
                            for desc_child in &next.children {
                                if let Some(block) = convert_node(desc_child) {
                                    content_blocks.push(block);
                                }
                            }
                            i += 1;
                        } else {
                            break;
                        }
                    }

                    if content_blocks.is_empty() {
                        content_blocks.push(Block::Paragraph {
                            inlines: vec![],
                            span: Span::NONE,
                        });
                    }
                    items.push((term_inlines, content_blocks));
                } else {
                    i += 1;
                }
            }

            if items.is_empty() {
                None
            } else {
                Some(Block::DefinitionList {
                    items,
                    span: Span::NONE,
                })
            }
        }

        node::DEFINITION_TERM | node::DEFINITION_DESC => {
            // Handled by DEFINITION_LIST
            None
        }

        node::HORIZONTAL_RULE => Some(Block::HorizontalRule { span: Span::NONE }),

        node::DIV | node::SPAN => {
            // Convert children as a paragraph if there are inlines
            let inlines = convert_nodes_to_inlines(&node.children);
            if inlines.is_empty() {
                None
            } else {
                Some(Block::Paragraph {
                    inlines,
                    span: Span::NONE,
                })
            }
        }

        node::FIGURE => {
            let inlines = convert_nodes_to_inlines(&node.children);
            if inlines.is_empty() {
                None
            } else {
                Some(Block::Paragraph {
                    inlines,
                    span: Span::NONE,
                })
            }
        }

        // Block-level inline elements - wrap in paragraph
        node::TEXT | node::STRONG | node::EMPHASIS | node::CODE | node::LINK | node::IMAGE => {
            convert_node_to_inline(node).map(|inline| Block::Paragraph {
                inlines: vec![inline],
                span: Span::NONE,
            })
        }

        _ => None,
    }
}

fn convert_nodes_to_inlines(nodes: &[Node]) -> Vec<Inline> {
    let mut inlines = Vec::new();
    for node in nodes {
        if let Some(inline) = convert_node_to_inline(node) {
            inlines.push(inline);
        }
    }
    inlines
}

fn convert_node_to_inline(node: &Node) -> Option<Inline> {
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
            let children = convert_nodes_to_inlines(&node.children);
            Some(Inline::Bold(children, Span::NONE))
        }

        node::EMPHASIS => {
            let children = convert_nodes_to_inlines(&node.children);
            Some(Inline::Italic(children, Span::NONE))
        }

        node::CODE => {
            let content = node
                .props
                .get_str(prop::CONTENT)
                .map(|s| s.to_string())
                .unwrap_or_default();
            let text = if content.is_empty() {
                let children = convert_nodes_to_inlines(&node.children);
                let mut text = String::new();
                for child in children {
                    if let Inline::Text(s, _) = child {
                        text.push_str(&s);
                    }
                }
                text
            } else {
                content
            };
            Some(Inline::Bold(
                vec![Inline::Text(text, Span::NONE)],
                Span::NONE,
            ))
        }

        node::LINK => {
            let url = node
                .props
                .get_str(prop::URL)
                .map(|s| s.to_string())
                .unwrap_or_default();
            let children = convert_nodes_to_inlines(&node.children);
            Some(Inline::Link {
                url,
                children,
                span: Span::NONE,
            })
        }

        node::IMAGE => {
            let url = node
                .props
                .get_str(prop::URL)
                .map(|s| s.to_string())
                .unwrap_or_default();
            let alt = node
                .props
                .get_str(prop::ALT)
                .map(|s| s.to_string())
                .unwrap_or_default();
            let label = if !alt.is_empty() { alt } else { url };
            Some(Inline::Text(format!("[Image: {}]", label), Span::NONE))
        }

        node::SUBSCRIPT | node::SUPERSCRIPT => {
            // No native support, just emit children
            let children = convert_nodes_to_inlines(&node.children);
            if children.is_empty() {
                None
            } else if children.len() == 1 {
                Some(children.into_iter().next().unwrap())
            } else {
                Some(Inline::Text(
                    children
                        .iter()
                        .filter_map(|i| {
                            if let Inline::Text(s, _) = i {
                                Some(s.clone())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(""),
                    Span::NONE,
                ))
            }
        }

        node::LINE_BREAK => Some(Inline::Text("\n".to_string(), Span::NONE)),

        node::SOFT_BREAK => Some(Inline::Text(" ".to_string(), Span::NONE)),

        node::DIV | node::SPAN => {
            // Container, unwrap children
            let children = convert_nodes_to_inlines(&node.children);
            if children.is_empty() {
                None
            } else if children.len() == 1 {
                Some(children.into_iter().next().unwrap())
            } else {
                // Multiple children - create a compound structure if possible
                // For now, just take the first one
                Some(children.into_iter().next().unwrap())
            }
        }

        _ => None,
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
    fn test_emit_basic() {
        let doc = doc(|d| d.para(|p| p.text("Hello, world!")));
        let output = emit_str(&doc);
        assert!(output.contains(".TH"));
        assert!(output.contains(".PP"));
        assert!(output.contains("Hello, world!"));
    }

    #[test]
    fn test_emit_heading() {
        let doc = doc(|d| d.heading(2, |h| h.text("Section Title")));
        let output = emit_str(&doc);
        assert!(output.contains(".SH SECTION TITLE"));
    }

    #[test]
    fn test_emit_subsection() {
        let doc = doc(|d| d.heading(3, |h| h.text("Subsection")));
        let output = emit_str(&doc);
        assert!(output.contains(".SS SUBSECTION"));
    }

    #[test]
    fn test_emit_bold() {
        let doc = doc(|d| d.para(|p| p.strong(|s| s.text("bold"))));
        let output = emit_str(&doc);
        assert!(output.contains("\\fBbold\\fR"));
    }

    #[test]
    fn test_emit_italic() {
        let doc = doc(|d| d.para(|p| p.em(|e| e.text("italic"))));
        let output = emit_str(&doc);
        assert!(output.contains("\\fIitalic\\fR"));
    }

    #[test]
    fn test_emit_code_block() {
        let doc = doc(|d| d.code_block("fn main() {}"));
        let output = emit_str(&doc);
        assert!(output.contains(".nf"));
        assert!(output.contains("fn main() {}"));
        assert!(output.contains(".fi"));
    }

    #[test]
    fn test_emit_list() {
        let doc =
            doc(|d| d.bullet_list(|l| l.item(|i| i.text("Item 1")).item(|i| i.text("Item 2"))));
        let output = emit_str(&doc);
        assert!(output.contains(".IP \\(bu"));
        assert!(output.contains("Item 1"));
        assert!(output.contains("Item 2"));
    }
}
