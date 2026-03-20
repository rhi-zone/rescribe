//! Fountain parser: text → AST.

use std::collections::BTreeMap;

use crate::ast::{Block, Diagnostic, FountainDoc, Span};

// ── Public API ────────────────────────────────────────────────────────────────

/// Parse a Fountain string into a [`FountainDoc`].
///
/// Parsing is infallible — all input is accepted.  Diagnostics are returned
/// alongside the document for any construct that could not be interpreted.
pub fn parse(input: &str) -> (FountainDoc, Vec<Diagnostic>) {
    let mut parser = Parser::new(input);
    parser.parse()
}

// ── Parser ────────────────────────────────────────────────────────────────────

struct Parser<'a> {
    lines: Vec<&'a str>,
    pos: usize,
    /// Byte offset corresponding to `lines[pos]`.
    line_offsets: Vec<usize>,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        // Pre-compute byte offsets for each line so we can attach Spans.
        let mut offsets = Vec::new();
        let mut off = 0usize;
        for line in input.lines() {
            offsets.push(off);
            // +1 for the '\n'; lines() strips it so we add it back.
            off += line.len() + 1;
        }
        // Sentinel for EOF.
        offsets.push(off);

        Self {
            lines: input.lines().collect(),
            pos: 0,
            line_offsets: offsets,
        }
    }

    fn current_offset(&self) -> usize {
        self.line_offsets
            .get(self.pos)
            .copied()
            .unwrap_or(*self.line_offsets.last().unwrap_or(&0))
    }

    fn span_for_line(&self, line_idx: usize) -> Span {
        let start = self.line_offsets.get(line_idx).copied().unwrap_or(0);
        let end = self
            .line_offsets
            .get(line_idx + 1)
            .copied()
            .unwrap_or(start);
        Span::new(start, end)
    }

    fn parse(&mut self) -> (FountainDoc, Vec<Diagnostic>) {
        let diags: Vec<Diagnostic> = Vec::new();
        let doc_start = self.current_offset();

        let metadata = self.parse_title_page();
        let blocks = self.parse_screenplay();

        let doc_end = self.current_offset();

        let doc = FountainDoc {
            metadata,
            blocks,
            span: Span::new(doc_start, doc_end),
        };
        (doc, diags)
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
                let line_idx = self.pos;
                let span = self.span_for_line(line_idx);
                if next_line.starts_with('(') && next_line.ends_with(')') {
                    let text = next_line.to_string();
                    blocks.push(Block::Parenthetical { text, span });
                    self.pos += 1;
                } else if !self.is_scene_heading(next_line)
                    && !self.is_transition(next_line)
                    && !self.is_character(next_line)
                {
                    let text = next_line.to_string();
                    blocks.push(Block::Dialogue { text, span });
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
            let span = self.span_for_line(self.pos);
            self.pos += 1;
            return Some(Block::PageBreak { span });
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
        if line.starts_with('>') && line.trim_end().ends_with('<') {
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
            return Some(self.parse_character());
        }

        // Lyric: ~text
        if line.starts_with('~') {
            return Some(self.parse_lyric());
        }

        // Default: action.
        // Note: parse_action will break immediately if the line starts with a
        // structural marker like `=`, `#`, `~`, `[[`, etc. If that happens
        // without advancing pos we'd loop forever. We guard against that by
        // checking whether parse_action would consume at least one line; if
        // not, we forcibly consume the current line as raw action text.
        let pos_before = self.pos;
        let block = self.parse_action();
        if self.pos == pos_before {
            // parse_action stalled — consume the line ourselves.
            let line_idx = self.pos;
            let span = self.span_for_line(line_idx);
            let text = self.lines[self.pos].to_string();
            self.pos += 1;
            return Some(Block::Action { text, span });
        }
        Some(block)
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
        let line_idx = self.pos;
        let line = self.lines[self.pos].trim();
        let span = self.span_for_line(line_idx);
        self.pos += 1;

        // Remove forced marker if present
        let text = line.strip_prefix('.').unwrap_or(line);

        Block::SceneHeading {
            text: text.to_string(),
            span,
        }
    }

    fn parse_transition(&mut self) -> Block {
        let line_idx = self.pos;
        let line = self.lines[self.pos].trim();
        let span = self.span_for_line(line_idx);
        self.pos += 1;

        // Remove forced marker if present
        let text = line.strip_prefix('>').map(|s| s.trim()).unwrap_or(line);

        Block::Transition {
            text: text.to_string(),
            span,
        }
    }

    fn parse_character(&mut self) -> Block {
        let line_idx = self.pos;
        let char_line = self.lines[self.pos].trim();
        let span = self.span_for_line(line_idx);
        self.pos += 1;

        // Remove forced marker if present
        let char_name = char_line.strip_prefix('@').unwrap_or(char_line);

        // Check for dual dialogue marker
        let dual = char_name.ends_with('^');
        let char_name = char_name.trim_end_matches('^').trim();

        Block::Character {
            name: char_name.to_string(),
            dual,
            span,
        }
    }

    fn parse_action(&mut self) -> Block {
        let start_idx = self.pos;
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

        let span = Span::new(
            self.line_offsets.get(start_idx).copied().unwrap_or(0),
            self.current_offset(),
        );
        let text = lines.join("\n");
        Block::Action { text, span }
    }

    fn parse_section(&mut self) -> Block {
        let line_idx = self.pos;
        let line = self.lines[self.pos];
        let span = self.span_for_line(line_idx);
        self.pos += 1;

        // Count # symbols for level
        let level = line.chars().take_while(|&c| c == '#').count();
        let text = line[level..].trim();

        Block::Section {
            level,
            text: text.to_string(),
            span,
        }
    }

    fn parse_synopsis(&mut self) -> Block {
        let line_idx = self.pos;
        let line = self.lines[self.pos].trim();
        let span = self.span_for_line(line_idx);
        self.pos += 1;

        let text = line[1..].trim(); // Remove = prefix

        Block::Synopsis {
            text: text.to_string(),
            span,
        }
    }

    fn parse_note(&mut self) -> Block {
        let line_idx = self.pos;
        let line = self.lines[self.pos];
        let span = self.span_for_line(line_idx);
        self.pos += 1;

        // Extract note content between [[ and ]]
        let start = line.find("[[").unwrap_or(0);
        let end = line.find("]]").unwrap_or(line.len());
        let text = if start + 2 <= end {
            &line[start + 2..end]
        } else {
            ""
        };

        Block::Note {
            text: text.to_string(),
            span,
        }
    }

    fn parse_centered(&mut self) -> Block {
        let line_idx = self.pos;
        let line = self.lines[self.pos].trim();
        let span = self.span_for_line(line_idx);
        self.pos += 1;

        // Remove > prefix and < suffix
        let inner = line.strip_prefix('>').unwrap_or(line);
        let text = inner.trim_end_matches('<');

        Block::Centered {
            text: text.to_string(),
            span,
        }
    }

    fn parse_lyric(&mut self) -> Block {
        let line_idx = self.pos;
        let line = self.lines[self.pos].trim();
        let span = self.span_for_line(line_idx);
        self.pos += 1;

        let text = &line[1..]; // Remove ~ prefix

        Block::Lyric {
            text: text.to_string(),
            span,
        }
    }
}
