//! Fountain screenplay format reader for rescribe.
//!
//! Thin adapter over [`fountain_fmt`]: parses Fountain into the `fountain_fmt` AST,
//! then maps it to the rescribe document model.

use fountain_fmt::Block;
use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions};
use rescribe_std::{node, prop};

/// Parse a Fountain document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse a Fountain document with custom options.
pub fn parse_with_options(
    input: &str,
    _options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let (fountain, _diags) = fountain_fmt::parse(input);

    let mut metadata = rescribe_core::Properties::new();
    for (key, value) in &fountain.metadata {
        metadata.set(
            format!("fountain:{}", key),
            rescribe_core::PropValue::String(value.clone()),
        );
    }

    let nodes = blocks_to_nodes(&fountain.blocks);
    let root = Node::new(node::DOCUMENT).children(nodes);
    let doc = Document::new().with_content(root).with_metadata(metadata);

    Ok(ConversionResult::ok(doc))
}

fn blocks_to_nodes(blocks: &[Block]) -> Vec<Node> {
    let mut nodes = Vec::new();
    let mut i = 0;

    while i < blocks.len() {
        // Group Character + Dialogue + Parenthetical into a dialogue_block
        if let Block::Character { name, dual, .. } = &blocks[i] {
            let mut dialogue_node = Node::new(node::DIV)
                .prop("fountain:type", "dialogue_block")
                .child(
                    Node::new(node::PARAGRAPH)
                        .prop("fountain:type", "character")
                        .child(Node::new(node::TEXT).prop(prop::CONTENT, name.clone())),
                );

            if *dual {
                dialogue_node = dialogue_node.prop("fountain:dual", true);
            }

            i += 1;

            // Collect following dialogue and parenthetical blocks
            while i < blocks.len() {
                match &blocks[i] {
                    Block::Dialogue { text, .. } => {
                        dialogue_node = dialogue_node.child(
                            Node::new(node::PARAGRAPH)
                                .prop("fountain:type", "dialogue")
                                .child(Node::new(node::TEXT).prop(prop::CONTENT, text.clone())),
                        );
                        i += 1;
                    }
                    Block::Parenthetical { text, .. } => {
                        dialogue_node = dialogue_node.child(
                            Node::new(node::PARAGRAPH)
                                .prop("fountain:type", "parenthetical")
                                .child(Node::new(node::TEXT).prop(prop::CONTENT, text.clone())),
                        );
                        i += 1;
                    }
                    _ => break,
                }
            }

            nodes.push(dialogue_node);
        } else {
            nodes.push(block_to_node(&blocks[i]));
            i += 1;
        }
    }

    nodes
}

fn block_to_node(block: &Block) -> Node {
    match block {
        Block::SceneHeading { text, .. } => Node::new(node::HEADING)
            .prop(prop::LEVEL, 2i64)
            .prop("fountain:type", "scene_heading")
            .child(Node::new(node::TEXT).prop(prop::CONTENT, text.clone())),

        Block::Action { text, .. } => Node::new(node::PARAGRAPH)
            .prop("fountain:type", "action")
            .child(Node::new(node::TEXT).prop(prop::CONTENT, text.clone())),

        Block::Transition { text, .. } => Node::new(node::PARAGRAPH)
            .prop("fountain:type", "transition")
            .child(Node::new(node::TEXT).prop(prop::CONTENT, text.clone())),

        Block::Centered { text, .. } => Node::new(node::PARAGRAPH)
            .prop("fountain:type", "centered")
            .child(Node::new(node::TEXT).prop(prop::CONTENT, text.clone())),

        Block::Lyric { text, .. } => Node::new(node::PARAGRAPH)
            .prop("fountain:type", "lyric")
            .child(Node::new(node::TEXT).prop(prop::CONTENT, text.clone())),

        Block::Note { text, .. } => Node::new(node::PARAGRAPH)
            .prop("fountain:type", "note")
            .child(Node::new(node::TEXT).prop(prop::CONTENT, text.clone())),

        Block::Synopsis { text, .. } => Node::new(node::PARAGRAPH)
            .prop("fountain:type", "synopsis")
            .child(Node::new(node::TEXT).prop(prop::CONTENT, text.clone())),

        Block::Section { level, text, .. } => Node::new(node::HEADING)
            .prop(prop::LEVEL, *level as i64)
            .prop("fountain:type", "section")
            .child(Node::new(node::TEXT).prop(prop::CONTENT, text.clone())),

        Block::PageBreak { .. } => {
            Node::new(node::HORIZONTAL_RULE).prop("fountain:type", "page_break")
        }

        // These shouldn't appear at top level in the output AST,
        // but handle them gracefully
        Block::Character { .. } | Block::Dialogue { .. } | Block::Parenthetical { .. } => {
            Node::new(node::PARAGRAPH)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_title_page() {
        let input = "Title: My Screenplay\nAuthor: John Doe\n\nINT. HOUSE - DAY";
        let result = parse(input).unwrap();
        let doc = result.value;
        assert!(doc.metadata.get_str("fountain:title").is_some());
    }

    #[test]
    fn test_parse_scene_heading() {
        let input = "INT. COFFEE SHOP - DAY";
        let result = parse(input).unwrap();
        let doc = result.value;
        assert_eq!(doc.content.children.len(), 1);
        assert_eq!(
            doc.content.children[0].props.get_str("fountain:type"),
            Some("scene_heading")
        );
    }

    #[test]
    fn test_parse_dialogue() {
        let input = "JOHN\nHello, how are you?";
        let result = parse(input).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
        assert_eq!(
            doc.content.children[0].props.get_str("fountain:type"),
            Some("dialogue_block")
        );
    }

    #[test]
    fn test_parse_action() {
        let input = "The door slowly opens. A figure emerges from the shadows.";
        let result = parse(input).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
        assert_eq!(
            doc.content.children[0].props.get_str("fountain:type"),
            Some("action")
        );
    }

    #[test]
    fn test_parse_transition() {
        let input = "CUT TO:";
        let result = parse(input).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
        assert_eq!(
            doc.content.children[0].props.get_str("fountain:type"),
            Some("transition")
        );
    }
}
