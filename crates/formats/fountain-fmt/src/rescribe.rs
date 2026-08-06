//! AST↔`rescribe::Document` translation for Fountain.
//!
//! This module only translates between [`FountainDoc`](crate::FountainDoc)
//! and rescribe's `Document` IR — no Fountain tokenizing/parsing/emitting
//! happens here (that all lives in the rest of this crate; see `crate::parse`
//! and `crate::emit`). Enabled by the `rescribe` feature; each direction is
//! additionally gated on the reader/writer mode feature it depends on, so
//! enabling `rescribe` alone (with no mode feature) compiles nothing.

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
mod read {
    use crate::Block;
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
        let (fountain, _diags) = crate::parse(input);

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
            let extension = rest
                .find(')')
                .map(|paren_end| rest[..paren_end].trim().to_string());
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
            if chars[pos] == '*' && (pos + 1 >= chars.len() || chars[pos + 1] != '*') {
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
            if chars[pos] == '_'
                && let Some(end) = find_closing(&chars, pos + 1, "_")
            {
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
}

#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
mod write {
    use crate::{Block, FountainDoc, Span};
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
        let output = crate::build(&fountain);
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
                            Node::new(NodeKind::from("text"))
                                .prop("content", "INT. COFFEE SHOP - DAY"),
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
                            Node::new(NodeKind::from("text"))
                                .prop("content", "The door slowly opens."),
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
}

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
pub use read::{parse, parse_with_options};
#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
pub use write::{emit, emit_with_options};
