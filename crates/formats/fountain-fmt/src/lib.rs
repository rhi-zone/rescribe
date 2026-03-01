//! Fountain screenplay format parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-fountain` and `rescribe-write-fountain` as thin adapter layers.

use std::collections::BTreeMap;

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct FountainError(pub String);

impl std::fmt::Display for FountainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Fountain error: {}", self.0)
    }
}

impl std::error::Error for FountainError {}

// ── AST ───────────────────────────────────────────────────────────────────────

/// A parsed Fountain document.
#[derive(Debug, Clone, Default)]
pub struct FountainDoc {
    pub metadata: BTreeMap<String, String>,
    pub blocks: Vec<Block>,
}

/// Block-level element.
#[derive(Debug, Clone)]
pub enum Block {
    /// Scene heading (INT./EXT./EST.)
    SceneHeading { text: String },
    /// Action/narrative text
    Action { text: String },
    /// Character name (possibly with dual dialogue marker)
    Character { name: String, dual: bool },
    /// Dialogue line
    Dialogue { text: String },
    /// Parenthetical direction
    Parenthetical { text: String },
    /// Transition (CUT TO:, FADE OUT, etc.)
    Transition { text: String },
    /// Centered text
    Centered { text: String },
    /// Lyric (singing, musical notation)
    Lyric { text: String },
    /// Note/comment [[text]]
    Note { text: String },
    /// Synopsis =text
    Synopsis { text: String },
    /// Section heading (#, ##, etc.)
    Section { level: usize, text: String },
    /// Page break (===)
    PageBreak,
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Parse a Fountain string into a [`FountainDoc`].
pub fn parse(input: &str) -> Result<FountainDoc, FountainError> {
    let mut parser = Parser::new(input);
    parser.parse()
}

struct Parser<'a> {
    lines: Vec<&'a str>,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            lines: input.lines().collect(),
            pos: 0,
        }
    }

    fn parse(&mut self) -> Result<FountainDoc, FountainError> {
        let metadata = self.parse_title_page();
        let blocks = self.parse_screenplay();

        Ok(FountainDoc { metadata, blocks })
    }

    fn parse_title_page(&mut self) -> BTreeMap<String, String> {
        let mut metadata = BTreeMap::new();

        // Valid title page fields
        let valid_fields = [
            "title",
            "credit",
            "author",
            "authors",
            "source",
            "draft date",
            "contact",
            "copyright",
            "notes",
        ];

        // Title page consists of key: value pairs at the start
        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            // Empty line ends title page
            if line.trim().is_empty() {
                self.pos += 1;
                break;
            }

            // Check for key: value pattern
            if let Some((key, value)) = line.split_once(':') {
                let key_lower = key.trim().to_lowercase();

                // Only accept known title page fields
                if !valid_fields.contains(&key_lower.as_str()) {
                    break;
                }

                let value = value.trim();

                // Multi-line values are indented
                let mut full_value = value.to_string();
                self.pos += 1;

                while self.pos < self.lines.len() {
                    let next_line = self.lines[self.pos];
                    if next_line.starts_with("   ") || next_line.starts_with('\t') {
                        full_value.push('\n');
                        full_value.push_str(next_line.trim());
                        self.pos += 1;
                    } else {
                        break;
                    }
                }

                metadata.insert(key_lower.replace(' ', "_"), full_value);
            } else {
                // Not a title page element
                break;
            }
        }

        metadata
    }

    fn parse_screenplay(&mut self) -> Vec<Block> {
        let mut blocks = Vec::new();

        while self.pos < self.lines.len() {
            if let Some(mut element_blocks) = self.parse_element_blocks() {
                blocks.append(&mut element_blocks);
            }
        }

        blocks
    }

    fn parse_element_blocks(&mut self) -> Option<Vec<Block>> {
        if self.pos >= self.lines.len() {
            return None;
        }

        let line = self.lines[self.pos];

        // Skip empty lines
        if line.trim().is_empty() {
            self.pos += 1;
            return None;
        }

        // Character (all caps, possibly with dialogue following) - special handling
        if self.is_character(line) {
            let mut blocks = vec![self.parse_character()];
            // Now collect following dialogue and parenthetical blocks
            while self.pos < self.lines.len() {
                let next_line = self.lines[self.pos].trim();
                if next_line.is_empty() {
                    self.pos += 1;
                    break;
                }
                if next_line.starts_with('(') && next_line.ends_with(')') {
                    let text = next_line.to_string();
                    blocks.push(Block::Parenthetical { text });
                    self.pos += 1;
                } else if !self.is_scene_heading(next_line)
                    && !self.is_transition(next_line)
                    && !self.is_character(next_line)
                {
                    let text = next_line.to_string();
                    blocks.push(Block::Dialogue { text });
                    self.pos += 1;
                } else {
                    break;
                }
            }
            return Some(blocks);
        }

        // For other elements, just return a vec with the single block
        self.parse_element().map(|b| vec![b])
    }

    fn parse_element(&mut self) -> Option<Block> {
        if self.pos >= self.lines.len() {
            return None;
        }

        let line = self.lines[self.pos];

        // Skip empty lines
        if line.trim().is_empty() {
            self.pos += 1;
            return None;
        }

        // Page break: ===
        if line.trim() == "===" {
            self.pos += 1;
            return Some(Block::PageBreak);
        }

        // Section: # heading
        if line.starts_with('#') {
            return Some(self.parse_section());
        }

        // Synopsis: = text
        if line.starts_with('=') && !line.starts_with("===") {
            return Some(self.parse_synopsis());
        }

        // Note: [[text]]
        if line.contains("[[") {
            return Some(self.parse_note());
        }

        // Centered text: >text<
        if line.starts_with('>') && line.ends_with('<') {
            return Some(self.parse_centered());
        }

        // Transition: text ending in TO: or starting with >
        if self.is_transition(line) {
            return Some(self.parse_transition());
        }

        // Scene heading
        if self.is_scene_heading(line) {
            return Some(self.parse_scene_heading());
        }

        // Character (all caps, possibly with dialogue following)
        if self.is_character(line) {
            return Some(self.parse_character_and_dialogue());
        }

        // Lyric: ~text
        if line.starts_with('~') {
            return Some(self.parse_lyric());
        }

        // Default: action
        Some(self.parse_action())
    }

    fn is_scene_heading(&self, line: &str) -> bool {
        let line = line.trim();
        // Forced scene heading
        if line.starts_with('.') && line.len() > 1 {
            return true;
        }
        // Standard scene heading prefixes
        let upper = line.to_uppercase();
        upper.starts_with("INT ")
            || upper.starts_with("INT.")
            || upper.starts_with("EXT ")
            || upper.starts_with("EXT.")
            || upper.starts_with("INT/EXT")
            || upper.starts_with("I/E")
            || upper.starts_with("EST ")
            || upper.starts_with("EST.")
    }

    fn is_transition(&self, line: &str) -> bool {
        let line = line.trim();
        // Forced transition
        if line.starts_with('>') && !line.ends_with('<') {
            return true;
        }
        // Standard transitions end in TO:
        line.to_uppercase().ends_with("TO:") && line == line.to_uppercase()
    }

    fn is_character(&self, line: &str) -> bool {
        let line = line.trim();
        if line.is_empty() {
            return false;
        }
        // Forced character
        if line.starts_with('@') {
            return true;
        }
        // Must be all uppercase (allowing parentheticals like (V.O.))
        let name_part = if let Some(paren_pos) = line.find('(') {
            &line[..paren_pos]
        } else {
            line
        };
        let name_part = name_part.trim();
        !name_part.is_empty()
            && name_part
                .chars()
                .all(|c| c.is_uppercase() || c.is_whitespace() || c == '^')
            && name_part.chars().any(|c| c.is_alphabetic())
    }

    fn parse_scene_heading(&mut self) -> Block {
        let line = self.lines[self.pos].trim();
        self.pos += 1;

        // Remove forced marker if present
        let text = line.strip_prefix('.').unwrap_or(line);

        Block::SceneHeading {
            text: text.to_string(),
        }
    }

    fn parse_transition(&mut self) -> Block {
        let line = self.lines[self.pos].trim();
        self.pos += 1;

        // Remove forced marker if present
        let text = line.strip_prefix('>').map(|s| s.trim()).unwrap_or(line);

        Block::Transition {
            text: text.to_string(),
        }
    }

    fn parse_character(&mut self) -> Block {
        let char_line = self.lines[self.pos].trim();
        self.pos += 1;

        // Remove forced marker if present
        let char_name = char_line.strip_prefix('@').unwrap_or(char_line);

        // Check for dual dialogue marker
        let dual = char_name.ends_with('^');
        let char_name = char_name.trim_end_matches('^').trim();

        Block::Character {
            name: char_name.to_string(),
            dual,
        }
    }

    fn parse_character_and_dialogue(&mut self) -> Block {
        // This is now deprecated and only used by old parse_element path
        // New code should use parse_element_blocks
        self.parse_character()
    }

    fn parse_action(&mut self) -> Block {
        let mut lines = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            if line.trim().is_empty() {
                self.pos += 1;
                break;
            }

            // Check if this starts a new element
            if self.is_scene_heading(line)
                || self.is_transition(line)
                || self.is_character(line)
                || line.starts_with('#')
                || line.starts_with('=')
                || line.starts_with('~')
                || line.contains("[[")
            {
                break;
            }

            // Handle forced action with !
            let text = line.strip_prefix('!').unwrap_or(line);

            lines.push(text.to_string());
            self.pos += 1;
        }

        let text = lines.join("\n");
        Block::Action { text }
    }

    fn parse_section(&mut self) -> Block {
        let line = self.lines[self.pos];
        self.pos += 1;

        // Count # symbols for level
        let level = line.chars().take_while(|&c| c == '#').count();
        let text = line[level..].trim();

        Block::Section {
            level,
            text: text.to_string(),
        }
    }

    fn parse_synopsis(&mut self) -> Block {
        let line = self.lines[self.pos].trim();
        self.pos += 1;

        let text = line[1..].trim(); // Remove = prefix

        Block::Synopsis {
            text: text.to_string(),
        }
    }

    fn parse_note(&mut self) -> Block {
        let line = self.lines[self.pos];
        self.pos += 1;

        // Extract note content between [[ and ]]
        let start = line.find("[[").unwrap_or(0);
        let end = line.find("]]").unwrap_or(line.len());
        let text = &line[start + 2..end];

        Block::Note {
            text: text.to_string(),
        }
    }

    fn parse_centered(&mut self) -> Block {
        let line = self.lines[self.pos].trim();
        self.pos += 1;

        // Remove > and < markers
        let text = &line[1..line.len() - 1];

        Block::Centered {
            text: text.to_string(),
        }
    }

    fn parse_lyric(&mut self) -> Block {
        let line = self.lines[self.pos].trim();
        self.pos += 1;

        let text = &line[1..]; // Remove ~ prefix

        Block::Lyric {
            text: text.to_string(),
        }
    }
}

// ── Builder ───────────────────────────────────────────────────────────────────

/// Build a Fountain string from a [`FountainDoc`].
pub fn build(doc: &FountainDoc) -> String {
    let mut ctx = BuildContext::new();

    // Emit title page metadata
    emit_title_page(&doc.metadata, &mut ctx);

    // Emit content
    emit_blocks(&doc.blocks, &mut ctx);

    ctx.output
}

struct BuildContext {
    output: String,
}

impl BuildContext {
    fn new() -> Self {
        Self {
            output: String::new(),
        }
    }

    fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn writeln(&mut self, s: &str) {
        self.output.push_str(s);
        self.output.push('\n');
    }

    fn ensure_blank_line(&mut self) {
        if !self.output.is_empty() && !self.output.ends_with("\n\n") {
            if self.output.ends_with('\n') {
                self.output.push('\n');
            } else {
                self.output.push_str("\n\n");
            }
        }
    }
}

fn emit_title_page(metadata: &BTreeMap<String, String>, ctx: &mut BuildContext) {
    if metadata.is_empty() {
        return;
    }

    // Standard title page fields in order
    let field_order = [
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

    let mut has_output = false;
    for field in field_order {
        if let Some(value) = metadata.get(field) {
            let display_key = field
                .split('_')
                .map(|s| {
                    let mut c = s.chars();
                    match c.next() {
                        None => String::new(),
                        Some(f) => f.to_uppercase().chain(c).collect(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");

            ctx.write(&display_key);
            ctx.write(": ");
            ctx.writeln(value);
            has_output = true;
        }
    }

    // Add any non-standard fields (shouldn't happen, but just in case)
    for (key, value) in metadata.iter() {
        if !field_order.contains(&key.as_str()) {
            let display_key = key
                .split('_')
                .map(|s| {
                    let mut c = s.chars();
                    match c.next() {
                        None => String::new(),
                        Some(f) => f.to_uppercase().chain(c).collect(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");

            ctx.write(&display_key);
            ctx.write(": ");
            ctx.writeln(value);
            has_output = true;
        }
    }

    if has_output {
        ctx.writeln("");
    }
}

fn emit_blocks(blocks: &[Block], ctx: &mut BuildContext) {
    let mut idx = 0;
    while idx < blocks.len() {
        emit_block(&blocks[idx], ctx);
        idx += 1;
    }
}

fn emit_block(block: &Block, ctx: &mut BuildContext) {
    match block {
        Block::SceneHeading { text } => {
            ctx.ensure_blank_line();
            ctx.writeln(text);
        }

        Block::Action { text } => {
            ctx.ensure_blank_line();
            ctx.writeln(text);
        }

        Block::Character { name, dual } => {
            ctx.ensure_blank_line();
            if *dual {
                ctx.writeln(&format!("{} ^", name.to_uppercase()));
            } else {
                ctx.writeln(&name.to_uppercase());
            }
        }

        Block::Dialogue { text } => {
            ctx.writeln(text);
        }

        Block::Parenthetical { text } => {
            ctx.writeln(text);
        }

        Block::Transition { text } => {
            ctx.ensure_blank_line();
            // If it doesn't look like a standard transition, force it with >
            if !text.to_uppercase().ends_with("TO:") {
                ctx.write(">");
            }
            ctx.writeln(&text.to_uppercase());
        }

        Block::Centered { text } => {
            ctx.ensure_blank_line();
            ctx.write(">");
            ctx.write(text);
            ctx.writeln("<");
        }

        Block::Lyric { text } => {
            ctx.write("~");
            ctx.writeln(text);
        }

        Block::Note { text } => {
            ctx.write("[[");
            ctx.write(text);
            ctx.writeln("]]");
        }

        Block::Synopsis { text } => {
            ctx.write("= ");
            ctx.writeln(text);
        }

        Block::Section { level, text } => {
            ctx.ensure_blank_line();
            ctx.write(&"#".repeat(*level));
            ctx.write(" ");
            ctx.writeln(text);
        }

        Block::PageBreak => {
            ctx.ensure_blank_line();
            ctx.writeln("===");
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_title_page() {
        let input = "Title: My Screenplay\nAuthor: John Doe\n\nINT. HOUSE - DAY";
        let doc = parse(input).unwrap();
        assert_eq!(
            doc.metadata.get("title").map(|s| s.as_str()),
            Some("My Screenplay")
        );
        assert_eq!(
            doc.metadata.get("author").map(|s| s.as_str()),
            Some("John Doe")
        );
    }

    #[test]
    fn test_parse_scene_heading() {
        let input = "INT. COFFEE SHOP - DAY";
        let doc = parse(input).unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::SceneHeading { .. }));
    }

    #[test]
    fn test_parse_dialogue() {
        let input = "JOHN\nHello, how are you?";
        let doc = parse(input).unwrap();
        // Character and Dialogue blocks are parsed separately in the AST,
        // but will be grouped together when converted to rescribe nodes
        assert!(!doc.blocks.is_empty());
        assert!(matches!(doc.blocks[0], Block::Character { .. }));
        // The dialogue block should be present
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_parse_action() {
        let input = "The door slowly opens. A figure emerges from the shadows.";
        let doc = parse(input).unwrap();
        assert!(!doc.blocks.is_empty());
        assert!(matches!(doc.blocks[0], Block::Action { .. }));
    }

    #[test]
    fn test_parse_transition() {
        let input = "CUT TO:";
        let doc = parse(input).unwrap();
        assert!(!doc.blocks.is_empty());
        assert!(matches!(doc.blocks[0], Block::Transition { .. }));
    }

    #[test]
    fn test_build_simple() {
        let mut doc = FountainDoc::default();
        doc.blocks.push(Block::SceneHeading {
            text: "INT. OFFICE - DAY".to_string(),
        });
        doc.blocks.push(Block::Action {
            text: "John enters.".to_string(),
        });
        let output = build(&doc);
        assert!(output.contains("INT. OFFICE - DAY"));
        assert!(output.contains("John enters."));
    }

    #[test]
    fn test_build_with_metadata() {
        let mut metadata = BTreeMap::new();
        metadata.insert("title".to_string(), "My Script".to_string());
        metadata.insert("author".to_string(), "Jane Doe".to_string());

        let mut doc = FountainDoc {
            metadata,
            blocks: vec![],
        };
        doc.blocks.push(Block::Action {
            text: "Fade in.".to_string(),
        });

        let output = build(&doc);
        assert!(output.contains("Title: My Script"));
        assert!(output.contains("Author: Jane Doe"));
    }

    #[test]
    fn test_parse_section() {
        let input = "# ACT ONE\n\nINT. HOUSE - DAY";
        let doc = parse(input).unwrap();
        assert!(
            doc.blocks
                .iter()
                .any(|b| matches!(b, Block::Section { .. }))
        );
    }

    #[test]
    fn test_parse_note() {
        let input = "This is action [[with a note]]";
        let doc = parse(input).unwrap();
        assert!(doc.blocks.iter().any(|b| matches!(b, Block::Note { .. })));
    }

    #[test]
    fn test_parse_centered() {
        let input = ">CENTERED TEXT<";
        let doc = parse(input).unwrap();
        assert!(
            doc.blocks
                .iter()
                .any(|b| matches!(b, Block::Centered { .. }))
        );
    }

    #[test]
    fn test_parse_lyric() {
        let input = "~And the music plays on...";
        let doc = parse(input).unwrap();
        assert!(doc.blocks.iter().any(|b| matches!(b, Block::Lyric { .. })));
    }

    #[test]
    fn test_parse_page_break() {
        let input = "Action\n\n===\n\nMore action";
        let doc = parse(input).unwrap();
        assert!(doc.blocks.iter().any(|b| matches!(b, Block::PageBreak)));
    }

    #[test]
    fn test_build_transition() {
        let mut doc = FountainDoc::default();
        doc.blocks.push(Block::Transition {
            text: "CUT TO:".to_string(),
        });
        let output = build(&doc);
        assert!(output.contains("CUT TO:"));
    }

    #[test]
    fn test_build_character_dual() {
        let mut doc = FountainDoc::default();
        doc.blocks.push(Block::Character {
            name: "JOHN".to_string(),
            dual: true,
        });
        let output = build(&doc);
        assert!(output.contains("JOHN ^"));
    }
}
