//! Fountain screenplay format writer for rescribe.
//!
//! Thin adapter over [`fountain_fmt`]: maps the rescribe document model to
//! the `fountain_fmt` AST, then builds Fountain output.

use fountain_fmt::{Block, FountainDoc, Span};
use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node};
use rescribe_std::{node, prop};
use std::collections::BTreeMap;

/// Emit a document as Fountain.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    emit_with_options(doc, &EmitOptions::default())
}

/// Emit a document as Fountain with custom options.
pub fn emit_with_options(
    doc: &Document,
    _options: &EmitOptions,
) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let fountain = doc_to_fountain(doc);
    let output = fountain_fmt::build(&fountain);
    Ok(ConversionResult::ok(output.into_bytes()))
}

fn doc_to_fountain(doc: &Document) -> FountainDoc {
    // Extract metadata
    let mut metadata = BTreeMap::new();
    let fountain_fields = [
        "title",
        "credit",
        "author",
        "authors",
        "source",
        "draft_date",
        "contact",
        "copyright",
        "notes",
    ];

    for field in fountain_fields {
        let key = format!("fountain:{}", field);
        if let Some(value) = doc.metadata.get_str(&key) {
            metadata.insert(field.to_string(), value.to_string());
        }
    }

    // Convert nodes to blocks
    let blocks = nodes_to_blocks(&doc.content.children);

    FountainDoc {
        metadata,
        blocks,
        span: Span::NONE,
    }
}

fn nodes_to_blocks(nodes: &[Node]) -> Vec<Block> {
    let mut blocks = Vec::new();

    for node in nodes {
        blocks.extend(node_to_blocks(node));
    }

    blocks
}

fn node_to_blocks(node: &Node) -> Vec<Block> {
    let fountain_type = node.props.get_str("fountain:type").unwrap_or("");

    match fountain_type {
        "scene_heading" => {
            let text = get_text_content(node);
            vec![Block::SceneHeading {
                text,
                span: Span::NONE,
            }]
        }

        "action" => {
            let text = get_text_content(node);
            vec![Block::Action {
                text,
                span: Span::NONE,
            }]
        }

        "transition" => {
            let text = get_text_content(node);
            vec![Block::Transition {
                text,
                span: Span::NONE,
            }]
        }

        "centered" => {
            let text = get_text_content(node);
            vec![Block::Centered {
                text,
                span: Span::NONE,
            }]
        }

        "lyric" => {
            let text = get_text_content(node);
            vec![Block::Lyric {
                text,
                span: Span::NONE,
            }]
        }

        "note" => {
            let text = get_text_content(node);
            vec![Block::Note {
                text,
                span: Span::NONE,
            }]
        }

        "synopsis" => {
            let text = get_text_content(node);
            vec![Block::Synopsis {
                text,
                span: Span::NONE,
            }]
        }

        "section" => {
            let level = node.props.get_int(prop::LEVEL).unwrap_or(1) as usize;
            let text = get_text_content(node);
            vec![Block::Section {
                level,
                text,
                span: Span::NONE,
            }]
        }

        "page_break" => {
            vec![Block::PageBreak { span: Span::NONE }]
        }

        "dialogue_block" => {
            // Extract character, dialogue, and parenthetical from dialogue block
            let mut blocks = Vec::new();
            let dual = node.props.get_bool("fountain:dual").unwrap_or(false);

            for child in &node.children {
                let child_type = child.props.get_str("fountain:type").unwrap_or("");
                match child_type {
                    "character" => {
                        let name = get_text_content(child);
                        blocks.push(Block::Character {
                            name,
                            dual,
                            span: Span::NONE,
                        });
                    }
                    "dialogue" => {
                        let text = get_text_content(child);
                        blocks.push(Block::Dialogue {
                            text,
                            span: Span::NONE,
                        });
                    }
                    "parenthetical" => {
                        let text = get_text_content(child);
                        blocks.push(Block::Parenthetical {
                            text,
                            span: Span::NONE,
                        });
                    }
                    _ => {}
                }
            }

            blocks
        }

        _ => {
            // Generic handling
            match node.kind.as_str() {
                node::DOCUMENT => nodes_to_blocks(&node.children),

                node::HEADING => {
                    let level = node.props.get_int(prop::LEVEL).unwrap_or(1) as usize;
                    if level == 2 {
                        // Treat as scene heading
                        let text = get_text_content(node);
                        vec![Block::SceneHeading {
                            text,
                            span: Span::NONE,
                        }]
                    } else {
                        // Treat as section
                        let text = get_text_content(node);
                        vec![Block::Section {
                            level,
                            text,
                            span: Span::NONE,
                        }]
                    }
                }

                node::PARAGRAPH => {
                    let text = get_text_content(node);
                    vec![Block::Action {
                        text,
                        span: Span::NONE,
                    }]
                }

                node::HORIZONTAL_RULE => vec![Block::PageBreak { span: Span::NONE }],

                node::DIV | node::SPAN => nodes_to_blocks(&node.children),

                _ => nodes_to_blocks(&node.children),
            }
        }
    }
}

fn get_text_content(node: &Node) -> String {
    let mut result = String::new();
    collect_text(node, &mut result);
    result
}

fn collect_text(node: &Node, result: &mut String) {
    if let Some(content) = node.props.get_str(prop::CONTENT) {
        result.push_str(content);
    }
    for child in &node.children {
        collect_text(child, result);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rescribe_core::NodeKind;

    fn emit_str(doc: &Document) -> String {
        String::from_utf8(emit(doc).unwrap().value).unwrap()
    }

    #[test]
    fn test_emit_scene_heading() {
        let doc = Document::new().with_content(
            Node::new(NodeKind::from("document")).child(
                Node::new(NodeKind::from("heading"))
                    .prop("fountain:type", "scene_heading")
                    .prop("level", 2i64)
                    .child(
                        Node::new(NodeKind::from("text")).prop("content", "INT. COFFEE SHOP - DAY"),
                    ),
            ),
        );

        let output = emit_str(&doc);
        assert!(output.contains("INT. COFFEE SHOP - DAY"));
    }

    #[test]
    fn test_emit_dialogue() {
        let doc = Document::new().with_content(
            Node::new(NodeKind::from("document")).child(
                Node::new(NodeKind::from("div"))
                    .prop("fountain:type", "dialogue_block")
                    .child(
                        Node::new(NodeKind::from("paragraph"))
                            .prop("fountain:type", "character")
                            .child(Node::new(NodeKind::from("text")).prop("content", "John")),
                    )
                    .child(
                        Node::new(NodeKind::from("paragraph"))
                            .prop("fountain:type", "dialogue")
                            .child(
                                Node::new(NodeKind::from("text"))
                                    .prop("content", "Hello, how are you?"),
                            ),
                    ),
            ),
        );

        let output = emit_str(&doc);
        assert!(output.contains("JOHN"));
        assert!(output.contains("Hello, how are you?"));
    }

    #[test]
    fn test_emit_transition() {
        let doc = Document::new().with_content(
            Node::new(NodeKind::from("document")).child(
                Node::new(NodeKind::from("paragraph"))
                    .prop("fountain:type", "transition")
                    .child(Node::new(NodeKind::from("text")).prop("content", "CUT TO:")),
            ),
        );

        let output = emit_str(&doc);
        assert!(output.contains("CUT TO:"));
    }

    #[test]
    fn test_emit_action() {
        let doc = Document::new().with_content(
            Node::new(NodeKind::from("document")).child(
                Node::new(NodeKind::from("paragraph"))
                    .prop("fountain:type", "action")
                    .child(
                        Node::new(NodeKind::from("text")).prop("content", "The door slowly opens."),
                    ),
            ),
        );

        let output = emit_str(&doc);
        assert!(output.contains("The door slowly opens."));
    }

    #[test]
    fn test_emit_page_break() {
        let doc = Document::new().with_content(Node::new(NodeKind::from("document")).child(
            Node::new(NodeKind::from("horizontal_rule")).prop("fountain:type", "page_break"),
        ));

        let output = emit_str(&doc);
        assert!(output.contains("==="));
    }
}
