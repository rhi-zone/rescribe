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
            // Parse extension from character name: "JOHN (V.O.)" → name="JOHN", ext="V.O."
            let (char_name, extension) = parse_character_name(name);

            let mut char_para = Node::new(node::PARAGRAPH)
                .prop("fountain:type", "character")
                .child(Node::new(node::TEXT).prop(prop::CONTENT, char_name));
            if let Some(ext) = extension {
                char_para = char_para.prop("fountain:extension", ext);
            }
            if *dual {
                char_para = char_para.prop("fountain:dual", true);
            }

            let mut dialogue_node = Node::new(node::DIV)
                .prop("fountain:type", "dialogue_block")
                .child(char_para);

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
                                .children(parse_inline_markup(text)),
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

/// Parse a character name, splitting off any parenthetical extension.
///
/// `"JOHN (V.O.)"` → `("JOHN", Some("V.O."))`
/// `"JOHN"` → `("JOHN", None)`
fn parse_character_name(name: &str) -> (String, Option<String>) {
    let name = name.trim();
    if let Some(paren_start) = name.find('(') {
        let char_name = name[..paren_start].trim().to_string();
        let rest = &name[paren_start + 1..];
        let extension = rest.find(')').map(|paren_end| rest[..paren_end].trim().to_string());
        (char_name, extension)
    } else {
        (name.to_string(), None)
    }
}

/// Parse scene heading text into its components.
///
/// Scene headings have the form:
/// `INT. COFFEE SHOP - DAY` → location_type="INT", time_of_day="DAY"
/// `INT. OFFICE - DAY #42#` → location_type="INT", time_of_day="DAY", scene_number="42"
fn parse_scene_heading(text: &str) -> (Option<String>, Option<String>, Option<String>) {
    let text = text.trim();

    // Extract scene number: #N# suffix
    let (main_text, scene_number) = if let Some(hash_start) = text.rfind('#') {
        if let Some(hash_end) = text[..hash_start].rfind('#') {
            let number = text[hash_end + 1..hash_start].trim().to_string();
            let before = text[..hash_end].trim().to_string();
            (before, Some(number))
        } else {
            (text.to_string(), None)
        }
    } else {
        (text.to_string(), None)
    };

    // Split on " - " to get location and time of day
    let (location_part, time_of_day) = if let Some(dash_pos) = main_text.rfind(" - ") {
        let loc = main_text[..dash_pos].trim().to_string();
        let tod = main_text[dash_pos + 3..].trim().to_string();
        (loc, Some(tod))
    } else {
        (main_text.clone(), None)
    };

    // Extract location type from start of location_part
    let location_type = if location_part.to_uppercase().starts_with("INT.") {
        Some("INT".to_string())
    } else if location_part.to_uppercase().starts_with("EXT.") {
        Some("EXT".to_string())
    } else if location_part.to_uppercase().starts_with("INT/EXT") {
        Some("INT/EXT".to_string())
    } else if location_part.to_uppercase().starts_with("I/E") {
        Some("I/E".to_string())
    } else if location_part.to_uppercase().starts_with("EST.") {
        Some("EST".to_string())
    } else if location_part.to_uppercase().starts_with("INT ") {
        Some("INT".to_string())
    } else if location_part.to_uppercase().starts_with("EXT ") {
        Some("EXT".to_string())
    } else {
        None
    };

    (location_type, time_of_day, scene_number)
}

/// Parse inline markup from a Fountain text string.
///
/// Fountain supports:
/// - `**text**` → bold/strong
/// - `*text*` → italic/emphasis
/// - `_text_` → underline
///
/// Returns a list of rescribe nodes representing the parsed inlines.
fn parse_inline_markup(text: &str) -> Vec<Node> {
    let mut nodes = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut pos = 0;
    let mut current_text = String::new();

    while pos < chars.len() {
        // Check for bold: **text**
        if pos + 1 < chars.len() && chars[pos] == '*' && chars[pos + 1] == '*' {
            // Find closing **
            if let Some(end) = find_closing(&chars, pos + 2, "**") {
                if !current_text.is_empty() {
                    nodes.push(Node::new(node::TEXT).prop(prop::CONTENT, current_text.clone()));
                    current_text.clear();
                }
                let inner: String = chars[pos + 2..end].iter().collect();
                nodes.push(
                    Node::new(node::STRONG)
                        .child(Node::new(node::TEXT).prop(prop::CONTENT, inner)),
                );
                pos = end + 2;
                continue;
            }
        }

        // Check for italic: *text* (not **)
        if chars[pos] == '*'
            && (pos + 1 >= chars.len() || chars[pos + 1] != '*')
        {
            // Find closing * (not **)
            if let Some(end) = find_closing_single_star(&chars, pos + 1) {
                if !current_text.is_empty() {
                    nodes.push(Node::new(node::TEXT).prop(prop::CONTENT, current_text.clone()));
                    current_text.clear();
                }
                let inner: String = chars[pos + 1..end].iter().collect();
                nodes.push(
                    Node::new(node::EMPHASIS)
                        .child(Node::new(node::TEXT).prop(prop::CONTENT, inner)),
                );
                pos = end + 1;
                continue;
            }
        }

        // Check for underline: _text_
        if chars[pos] == '_' && let Some(end) = find_closing(&chars, pos + 1, "_") {
            if !current_text.is_empty() {
                nodes.push(Node::new(node::TEXT).prop(prop::CONTENT, current_text.clone()));
                current_text.clear();
            }
            let inner: String = chars[pos + 1..end].iter().collect();
            nodes.push(
                Node::new(node::UNDERLINE)
                    .child(Node::new(node::TEXT).prop(prop::CONTENT, inner)),
            );
            pos = end + 1;
            continue;
        }

        current_text.push(chars[pos]);
        pos += 1;
    }

    if !current_text.is_empty() {
        nodes.push(Node::new(node::TEXT).prop(prop::CONTENT, current_text));
    }

    // If no markup was found, return a single text node
    if nodes.is_empty() {
        nodes.push(Node::new(node::TEXT).prop(prop::CONTENT, text.to_string()));
    }

    nodes
}

/// Find the position of closing marker (e.g. "**" or "_") starting from `start`.
fn find_closing(chars: &[char], start: usize, marker: &str) -> Option<usize> {
    let marker_chars: Vec<char> = marker.chars().collect();
    let mlen = marker_chars.len();
    let mut i = start;
    while i + mlen <= chars.len() {
        if chars[i..i + mlen] == marker_chars[..] {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Find the closing single `*` that is not part of `**`.
fn find_closing_single_star(chars: &[char], start: usize) -> Option<usize> {
    let mut i = start;
    while i < chars.len() {
        if chars[i] == '*' {
            // Make sure it's not **
            if i + 1 < chars.len() && chars[i + 1] == '*' {
                i += 2;
                continue;
            }
            return Some(i);
        }
        i += 1;
    }
    None
}

fn block_to_node(block: &Block) -> Node {
    match block {
        Block::SceneHeading { text, .. } => {
            let (location_type, time_of_day, scene_number) = parse_scene_heading(text);
            let mut node = Node::new(node::HEADING)
                .prop(prop::LEVEL, 2i64)
                .prop("fountain:type", "scene_heading")
                .child(Node::new(node::TEXT).prop(prop::CONTENT, text.clone()));
            if let Some(lt) = location_type {
                node = node.prop("fountain:location_type", lt);
            }
            if let Some(tod) = time_of_day {
                node = node.prop("fountain:time_of_day", tod);
            }
            if let Some(sn) = scene_number {
                node = node.prop("fountain:scene_number", sn);
            }
            node
        }

        Block::Action { text, .. } => Node::new(node::PARAGRAPH)
            .prop("fountain:type", "action")
            .children(parse_inline_markup(text)),

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

        Block::Boneyard { text, .. } => Node::new(node::RAW_BLOCK)
            .prop(prop::FORMAT, "fountain")
            .prop("fountain:type", "boneyard")
            .prop(prop::CONTENT, text.clone()),

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
