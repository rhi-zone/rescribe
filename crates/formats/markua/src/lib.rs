//! Markua (Leanpub) parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-markua` and `rescribe-write-markua` as thin adapter layers.

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct MarkuaError(pub String);

impl std::fmt::Display for MarkuaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Markua error: {}", self.0)
    }
}

impl std::error::Error for MarkuaError {}

// ── AST ───────────────────────────────────────────────────────────────────────

/// A parsed Markua document.
#[derive(Debug, Clone, Default)]
pub struct MarkuaDoc {
    pub blocks: Vec<Block>,
}

/// Block-level element.
#[derive(Debug, Clone)]
pub enum Block {
    Paragraph {
        inlines: Vec<Inline>,
    },
    Heading {
        level: u8,
        inlines: Vec<Inline>,
    },
    CodeBlock {
        content: String,
        language: Option<String>,
    },
    Blockquote {
        children: Vec<Block>,
    },
    List {
        ordered: bool,
        items: Vec<Vec<Block>>,
    },
    Table {
        rows: Vec<TableRow>,
    },
    HorizontalRule,
    SpecialBlock {
        block_type: String,
        inlines: Vec<Inline>,
    },
}

/// A table row.
#[derive(Debug, Clone)]
pub struct TableRow {
    pub cells: Vec<Vec<Inline>>,
}

/// Inline element.
#[derive(Debug, Clone)]
pub enum Inline {
    Text(String),
    Strong(Vec<Inline>),
    Emphasis(Vec<Inline>),
    Strikethrough(Vec<Inline>),
    Code(String),
    Link { url: String, children: Vec<Inline> },
    Image { url: String, alt: String },
    LineBreak,
    SoftBreak,
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Parse a Markua string into a [`MarkuaDoc`].
pub fn parse(input: &str) -> Result<MarkuaDoc, MarkuaError> {
    let mut p = Parser::new(input);
    let blocks = p.parse();
    Ok(MarkuaDoc { blocks })
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

    fn parse(&mut self) -> Vec<Block> {
        let mut nodes = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            if line.trim().is_empty() {
                self.pos += 1;
                continue;
            }

            // ATX headings: # Title
            if let Some(block) = self.try_parse_atx_heading(line) {
                nodes.push(block);
                self.pos += 1;
                continue;
            }

            // Scene break: * * * or - - - or *** or ---
            if self.is_scene_break(line) {
                nodes.push(Block::HorizontalRule);
                self.pos += 1;
                continue;
            }

            // Fenced code block
            if line.trim_start().starts_with("```") || line.trim_start().starts_with("~~~") {
                nodes.push(self.parse_fenced_code_block());
                continue;
            }

            // Markua special blocks: A>, B>, W>, T>, E>, D>, Q>, I>
            if let Some(block_type) = Self::get_special_block_type(line) {
                nodes.push(self.parse_special_block(block_type));
                continue;
            }

            // Blockquote: > text
            if line.trim_start().starts_with("> ") || line.trim_start() == ">" {
                nodes.push(self.parse_blockquote());
                continue;
            }

            // Unordered list: - or * or +
            let trimmed = line.trim_start();
            if (trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ "))
                && !self.is_scene_break(line)
            {
                nodes.push(self.parse_list(false));
                continue;
            }

            // Ordered list: 1. or 1)
            if self.is_ordered_list_item(line) {
                nodes.push(self.parse_list(true));
                continue;
            }

            // Regular paragraph
            nodes.push(self.parse_paragraph());
        }

        nodes
    }

    fn try_parse_atx_heading(&self, line: &str) -> Option<Block> {
        let trimmed = line.trim_start();
        if !trimmed.starts_with('#') {
            return None;
        }

        let level = trimmed.chars().take_while(|&c| c == '#').count();
        if level == 0 || level > 6 {
            return None;
        }

        let rest = &trimmed[level..];
        // Heading must be followed by space or be empty
        if !rest.is_empty() && !rest.starts_with(' ') {
            return None;
        }

        // Remove trailing # if present
        let title = rest.trim().trim_end_matches('#').trim();
        let inline_nodes = parse_inline(title);

        Some(Block::Heading {
            level: level as u8,
            inlines: inline_nodes,
        })
    }

    fn is_scene_break(&self, line: &str) -> bool {
        let trimmed = line.trim();
        // * * * or - - - or *** or --- (at least 3 characters)
        if trimmed.len() < 3 {
            return false;
        }

        let chars: Vec<char> = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
        if chars.len() < 3 {
            return false;
        }

        let first = chars[0];
        (first == '*' || first == '-' || first == '_') && chars.iter().all(|&c| c == first)
    }

    fn get_special_block_type(line: &str) -> Option<String> {
        let trimmed = line.trim_start();
        let prefixes = [
            ("A> ", "aside"),
            ("B> ", "blurb"),
            ("W> ", "warning"),
            ("T> ", "tip"),
            ("E> ", "error"),
            ("D> ", "discussion"),
            ("Q> ", "question"),
            ("I> ", "information"),
        ];

        for (prefix, block_type) in prefixes {
            if trimmed.starts_with(prefix) {
                return Some(block_type.to_string());
            }
        }
        None
    }

    fn parse_special_block(&mut self, block_type: String) -> Block {
        let prefix = match block_type.as_str() {
            "aside" => "A> ",
            "blurb" => "B> ",
            "warning" => "W> ",
            "tip" => "T> ",
            "error" => "E> ",
            "discussion" => "D> ",
            "question" => "Q> ",
            "information" => "I> ",
            _ => {
                return Block::Paragraph {
                    inlines: Vec::new(),
                };
            }
        };

        let mut content = String::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            let trimmed = line.trim_start();

            if let Some(rest) = trimmed.strip_prefix(prefix) {
                if !content.is_empty() {
                    content.push(' ');
                }
                content.push_str(rest);
                self.pos += 1;
            } else if trimmed.is_empty() {
                self.pos += 1;
                break;
            } else {
                break;
            }
        }

        let inline_nodes = parse_inline(&content);
        Block::SpecialBlock {
            block_type,
            inlines: inline_nodes,
        }
    }

    fn parse_fenced_code_block(&mut self) -> Block {
        let first_line = self.lines[self.pos].trim_start();
        let fence_char = first_line.chars().next().unwrap_or('`');
        let fence_len = first_line.chars().take_while(|&c| c == fence_char).count();

        // Extract info string (language)
        let info_string = first_line[fence_len..].trim();
        let language = if info_string.is_empty() {
            None
        } else {
            Some(
                info_string
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_string(),
            )
        };

        self.pos += 1;
        let mut content = String::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            let trimmed = line.trim_start();

            // Check for closing fence
            if trimmed.starts_with(fence_char)
                && trimmed.chars().take_while(|&c| c == fence_char).count() >= fence_len
            {
                self.pos += 1;
                break;
            }

            content.push_str(line);
            content.push('\n');
            self.pos += 1;
        }

        Block::CodeBlock {
            content: content.trim_end().to_string(),
            language,
        }
    }

    fn parse_blockquote(&mut self) -> Block {
        let mut content = String::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            let trimmed = line.trim_start();

            if let Some(rest) = trimmed.strip_prefix("> ") {
                if !content.is_empty() {
                    content.push(' ');
                }
                content.push_str(rest);
                self.pos += 1;
            } else if trimmed == ">" {
                // Empty blockquote line
                self.pos += 1;
            } else if trimmed.is_empty() {
                self.pos += 1;
                break;
            } else {
                break;
            }
        }

        let inline_nodes = parse_inline(&content);
        let para = Block::Paragraph {
            inlines: inline_nodes,
        };
        Block::Blockquote {
            children: vec![para],
        }
    }

    fn is_ordered_list_item(&self, line: &str) -> bool {
        let trimmed = line.trim_start();
        // Check for digit(s) followed by . or ) and space
        let mut chars = trimmed.chars();
        let mut has_digit = false;

        while let Some(c) = chars.next() {
            if c.is_ascii_digit() {
                has_digit = true;
            } else if has_digit && (c == '.' || c == ')') {
                // Check if followed by space or end
                match chars.next() {
                    Some(' ') | None => return true,
                    _ => return false,
                }
            } else {
                return false;
            }
        }
        false
    }

    fn parse_list(&mut self, ordered: bool) -> Block {
        let mut items = Vec::new();
        let base_indent = self.lines[self.pos].len() - self.lines[self.pos].trim_start().len();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            let indent = line.len() - line.trim_start().len();
            let trimmed = line.trim_start();

            if trimmed.is_empty() {
                self.pos += 1;
                continue;
            }

            if indent < base_indent {
                break;
            }

            let is_bullet =
                trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ");
            let is_numbered = self.is_ordered_list_item(line);

            if !is_bullet && !is_numbered {
                break;
            }

            let content = if is_bullet {
                &trimmed[2..]
            } else {
                // Find position after number and delimiter
                let marker_end = trimmed.find(". ").or_else(|| trimmed.find(") "));
                if let Some(pos) = marker_end {
                    &trimmed[pos + 2..]
                } else {
                    break;
                }
            };

            let inline_nodes = parse_inline(content);
            let para = Block::Paragraph {
                inlines: inline_nodes,
            };
            items.push(vec![para]);
            self.pos += 1;
        }

        Block::List { ordered, items }
    }

    fn parse_paragraph(&mut self) -> Block {
        let mut text = String::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            if line.trim().is_empty() {
                break;
            }

            // Check for block elements
            let trimmed = line.trim_start();
            if self.try_parse_atx_heading(line).is_some()
                || self.is_scene_break(line)
                || trimmed.starts_with("```")
                || trimmed.starts_with("~~~")
                || trimmed.starts_with("> ")
                || Self::get_special_block_type(line).is_some()
                || trimmed.starts_with("- ")
                || trimmed.starts_with("* ")
                || trimmed.starts_with("+ ")
                || self.is_ordered_list_item(line)
            {
                break;
            }

            if !text.is_empty() {
                text.push(' ');
            }
            text.push_str(line.trim());
            self.pos += 1;
        }

        let inline_nodes = parse_inline(&text);
        Block::Paragraph {
            inlines: inline_nodes,
        }
    }
}

/// Parse inline formatting.
fn parse_inline(text: &str) -> Vec<Inline> {
    let mut nodes = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Strong: **text** or __text__
        if i + 1 < chars.len()
            && ((chars[i] == '*' && chars[i + 1] == '*')
                || (chars[i] == '_' && chars[i + 1] == '_'))
        {
            let marker = chars[i];
            if let Some((end, content)) = find_double_marker(&chars, i + 2, marker) {
                if !current.is_empty() {
                    nodes.push(Inline::Text(current.clone()));
                    current.clear();
                }
                let inner = parse_inline(&content);
                nodes.push(Inline::Strong(inner));
                i = end + 2;
                continue;
            }
        }

        // Strikethrough: ~~text~~
        if i + 1 < chars.len()
            && chars[i] == '~'
            && chars[i + 1] == '~'
            && let Some((end, content)) = find_double_marker(&chars, i + 2, '~')
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone()));
                current.clear();
            }
            let inner = parse_inline(&content);
            nodes.push(Inline::Strikethrough(inner));
            i = end + 2;
            continue;
        }

        // Emphasis: *text* or _text_
        if chars[i] == '*' || chars[i] == '_' {
            let marker = chars[i];
            if let Some((end, content)) = find_single_marker(&chars, i + 1, marker) {
                if !current.is_empty() {
                    nodes.push(Inline::Text(current.clone()));
                    current.clear();
                }
                let inner = parse_inline(&content);
                nodes.push(Inline::Emphasis(inner));
                i = end + 1;
                continue;
            }
        }

        // Inline code: `code`
        if chars[i] == '`'
            && let Some((end, content)) = find_backtick_content(&chars, i)
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone()));
                current.clear();
            }
            nodes.push(Inline::Code(content));
            i = end;
            continue;
        }

        // Link: [text](url) or [text][ref]
        if chars[i] == '['
            && let Some((end, link_text, url)) = parse_link(&chars, i)
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone()));
                current.clear();
            }
            let text_nodes = parse_inline(&link_text);
            nodes.push(Inline::Link {
                url,
                children: text_nodes,
            });
            i = end;
            continue;
        }

        // Image: ![alt](url)
        if chars[i] == '!'
            && i + 1 < chars.len()
            && chars[i + 1] == '['
            && let Some((end, alt, url)) = parse_link(&chars, i + 1)
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone()));
                current.clear();
            }
            nodes.push(Inline::Image { url, alt });
            i = end;
            continue;
        }

        current.push(chars[i]);
        i += 1;
    }

    if !current.is_empty() {
        nodes.push(Inline::Text(current));
    }

    nodes
}

fn find_double_marker(chars: &[char], start: usize, marker: char) -> Option<(usize, String)> {
    let mut i = start;
    let mut content = String::new();

    while i + 1 < chars.len() {
        if chars[i] == marker && chars[i + 1] == marker {
            if !content.is_empty() {
                return Some((i, content));
            }
            return None;
        }
        content.push(chars[i]);
        i += 1;
    }
    None
}

fn find_single_marker(chars: &[char], start: usize, marker: char) -> Option<(usize, String)> {
    let mut i = start;
    let mut content = String::new();

    while i < chars.len() {
        if chars[i] == marker {
            // Don't match if this would form a double marker
            if i + 1 < chars.len() && chars[i + 1] == marker {
                content.push(chars[i]);
                i += 1;
                continue;
            }
            if !content.is_empty() {
                return Some((i, content));
            }
            return None;
        }
        content.push(chars[i]);
        i += 1;
    }
    None
}

fn find_backtick_content(chars: &[char], start: usize) -> Option<(usize, String)> {
    // Count opening backticks
    let mut backtick_count = 0;
    let mut i = start;
    while i < chars.len() && chars[i] == '`' {
        backtick_count += 1;
        i += 1;
    }

    // Find matching closing backticks
    let mut content = String::new();
    while i < chars.len() {
        if chars[i] == '`' {
            let mut closing_count = 0;
            let _close_start = i;
            while i < chars.len() && chars[i] == '`' {
                closing_count += 1;
                i += 1;
            }
            if closing_count == backtick_count {
                return Some((i, content.trim().to_string()));
            }
            // Not matching, add to content
            for _ in 0..closing_count {
                content.push('`');
            }
        } else {
            content.push(chars[i]);
            i += 1;
        }
    }
    None
}

fn parse_link(chars: &[char], start: usize) -> Option<(usize, String, String)> {
    if chars[start] != '[' {
        return None;
    }

    // Find closing ]
    let mut i = start + 1;
    let mut link_text = String::new();

    while i < chars.len() && chars[i] != ']' {
        link_text.push(chars[i]);
        i += 1;
    }

    if i >= chars.len() {
        return None;
    }

    i += 1; // Skip ]

    if i >= chars.len() {
        return None;
    }

    // Check for (url)
    if chars[i] == '(' {
        i += 1;
        let mut url = String::new();
        while i < chars.len() && chars[i] != ')' {
            url.push(chars[i]);
            i += 1;
        }
        if i < chars.len() {
            return Some((i + 1, link_text, url));
        }
    }

    None
}

// ── Builder ───────────────────────────────────────────────────────────────────

/// Build a Markua string from a [`MarkuaDoc`].
pub fn build(doc: &MarkuaDoc) -> String {
    let mut ctx = BuildContext::new();
    for block in &doc.blocks {
        build_block(block, &mut ctx);
    }
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
}

fn build_block(block: &Block, ctx: &mut BuildContext) {
    match block {
        Block::Paragraph { inlines } => {
            build_inlines(inlines, ctx);
            ctx.write("\n\n");
        }

        Block::Heading { level, inlines } => {
            let marker: String = "#".repeat(*level as usize);
            ctx.write(&marker);
            ctx.write(" ");
            build_inlines(inlines, ctx);
            ctx.write("\n\n");
        }

        Block::CodeBlock { content, language } => {
            ctx.write("```");
            if let Some(lang) = language {
                ctx.write(lang);
            }
            ctx.write("\n");
            ctx.write(content);
            if !content.ends_with('\n') {
                ctx.write("\n");
            }
            ctx.write("```\n\n");
        }

        Block::Blockquote { children } => {
            for child in children {
                match child {
                    Block::Paragraph { inlines } => {
                        ctx.write("> ");
                        build_inlines(inlines, ctx);
                        ctx.write("\n");
                    }
                    other => build_block(other, ctx),
                }
            }
            ctx.write("\n");
        }

        Block::List { ordered, items } => {
            let mut num = 1u32;
            for item_blocks in items {
                if *ordered {
                    ctx.write(&format!("{}. ", num));
                    num += 1;
                } else {
                    ctx.write("- ");
                }

                for block in item_blocks {
                    match block {
                        Block::Paragraph { inlines } => build_inlines(inlines, ctx),
                        other => build_block(other, ctx),
                    }
                }
                ctx.write("\n");
            }
            ctx.write("\n");
        }

        Block::Table { rows } => {
            for (row_idx, row) in rows.iter().enumerate() {
                ctx.write("|");
                for cell in &row.cells {
                    ctx.write(" ");
                    build_inlines(cell, ctx);
                    ctx.write(" |");
                }
                ctx.write("\n");

                // Add separator after header row
                if row_idx == 0 {
                    ctx.write("|");
                    for _ in &row.cells {
                        ctx.write(" --- |");
                    }
                    ctx.write("\n");
                }
            }
            ctx.write("\n");
        }

        Block::HorizontalRule => {
            ctx.write("* * *\n\n");
        }

        Block::SpecialBlock {
            block_type,
            inlines,
        } => {
            let prefix = match block_type.as_str() {
                "aside" => "A> ",
                "blurb" => "B> ",
                "warning" => "W> ",
                "tip" => "T> ",
                "error" => "E> ",
                "discussion" => "D> ",
                "question" => "Q> ",
                "information" => "I> ",
                _ => "",
            };

            if !prefix.is_empty() {
                ctx.write(prefix);
                build_inlines(inlines, ctx);
                ctx.write("\n\n");
            }
        }
    }
}

fn build_inlines(inlines: &[Inline], ctx: &mut BuildContext) {
    for inline in inlines {
        build_inline(inline, ctx);
    }
}

fn build_inline(inline: &Inline, ctx: &mut BuildContext) {
    match inline {
        Inline::Text(s) => ctx.write(s),

        Inline::Strong(children) => {
            ctx.write("**");
            build_inlines(children, ctx);
            ctx.write("**");
        }

        Inline::Emphasis(children) => {
            ctx.write("*");
            build_inlines(children, ctx);
            ctx.write("*");
        }

        Inline::Strikethrough(children) => {
            ctx.write("~~");
            build_inlines(children, ctx);
            ctx.write("~~");
        }

        Inline::Code(s) => {
            // Use double backticks if content contains single backtick
            if s.contains('`') {
                ctx.write("`` ");
                ctx.write(s);
                ctx.write(" ``");
            } else {
                ctx.write("`");
                ctx.write(s);
                ctx.write("`");
            }
        }

        Inline::Link { url, children } => {
            ctx.write("[");
            if children.is_empty() {
                ctx.write(url);
            } else {
                build_inlines(children, ctx);
            }
            ctx.write("](");
            ctx.write(url);
            ctx.write(")");
        }

        Inline::Image { url, alt } => {
            ctx.write("![");
            ctx.write(alt);
            ctx.write("](");
            ctx.write(url);
            ctx.write(")");
        }

        Inline::LineBreak => ctx.write("\n"),

        Inline::SoftBreak => ctx.write(" "),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_heading() {
        let doc = parse("# Title\n").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        match &doc.blocks[0] {
            Block::Heading { level, .. } => assert_eq!(*level, 1),
            _ => panic!("expected heading"),
        }
    }

    #[test]
    fn test_parse_heading_level2() {
        let doc = parse("## Subtitle\n").unwrap();
        match &doc.blocks[0] {
            Block::Heading { level, .. } => assert_eq!(*level, 2),
            _ => panic!("expected heading"),
        }
    }

    #[test]
    fn test_parse_paragraph() {
        let doc = parse("Hello world\n").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Paragraph { .. }));
    }

    #[test]
    fn test_parse_bold() {
        let doc = parse("**bold**\n").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Strong(_))));
    }

    #[test]
    fn test_parse_italic() {
        let doc = parse("*italic*\n").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Emphasis(_))));
    }

    #[test]
    fn test_parse_code() {
        let doc = parse("`code`\n").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Code(_))));
    }

    #[test]
    fn test_parse_link() {
        let doc = parse("[click here](https://example.com)\n").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        let link = inlines.iter().find(|i| matches!(i, Inline::Link { .. }));
        assert!(link.is_some());
    }

    #[test]
    fn test_parse_aside() {
        let doc = parse("A> This is an aside.\n").unwrap();
        let block = &doc.blocks[0];
        assert!(matches!(block, Block::SpecialBlock { block_type, .. } if block_type == "aside"));
    }

    #[test]
    fn test_parse_warning() {
        let doc = parse("W> This is a warning.\n").unwrap();
        let block = &doc.blocks[0];
        assert!(matches!(block, Block::SpecialBlock { block_type, .. } if block_type == "warning"));
    }

    #[test]
    fn test_parse_tip() {
        let doc = parse("T> This is a tip.\n").unwrap();
        let block = &doc.blocks[0];
        assert!(matches!(block, Block::SpecialBlock { block_type, .. } if block_type == "tip"));
    }

    #[test]
    fn test_parse_blockquote() {
        let doc = parse("> Quoted text\n").unwrap();
        assert!(matches!(doc.blocks[0], Block::Blockquote { .. }));
    }

    #[test]
    fn test_parse_unordered_list() {
        let doc = parse("- item1\n- item2\n").unwrap();
        let block = &doc.blocks[0];
        match block {
            Block::List { ordered, items } => {
                assert!(!ordered);
                assert_eq!(items.len(), 2);
            }
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn test_parse_ordered_list() {
        let doc = parse("1. first\n2. second\n").unwrap();
        let block = &doc.blocks[0];
        match block {
            Block::List { ordered, .. } => assert!(*ordered),
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn test_parse_code_block() {
        let doc = parse("```\ncode here\n```\n").unwrap();
        assert!(matches!(doc.blocks[0], Block::CodeBlock { .. }));
    }

    #[test]
    fn test_parse_code_block_with_language() {
        let doc = parse("```ruby\nputs 'hello'\n```\n").unwrap();
        let block = &doc.blocks[0];
        match block {
            Block::CodeBlock { language, .. } => {
                assert_eq!(language.as_ref().map(|l| l.as_str()), Some("ruby"));
            }
            _ => panic!("expected code block"),
        }
    }

    #[test]
    fn test_parse_scene_break() {
        let doc = parse("* * *\n").unwrap();
        assert!(matches!(doc.blocks[0], Block::HorizontalRule));
    }

    #[test]
    fn test_parse_image() {
        let doc = parse("![Alt text](image.png)\n").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        let img = inlines.iter().find(|i| matches!(i, Inline::Image { .. }));
        assert!(img.is_some());
    }

    #[test]
    fn test_build_paragraph() {
        let doc = MarkuaDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Hello, world!".into())],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("Hello, world!"));
    }

    #[test]
    fn test_build_bold() {
        let doc = MarkuaDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Strong(vec![Inline::Text("bold".into())])],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("**bold**"));
    }

    #[test]
    fn test_build_heading() {
        let doc = MarkuaDoc {
            blocks: vec![Block::Heading {
                level: 1,
                inlines: vec![Inline::Text("Title".into())],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("# Title"));
    }

    #[test]
    fn test_build_code_block() {
        let doc = MarkuaDoc {
            blocks: vec![Block::CodeBlock {
                content: "print hi".into(),
                language: None,
            }],
        };
        let out = build(&doc);
        assert!(out.contains("```"));
        assert!(out.contains("print hi"));
    }

    #[test]
    fn test_roundtrip_heading() {
        let input = "# Title\n";
        let doc = parse(input).unwrap();
        let output = build(&doc);
        assert!(output.contains("# Title"));
    }

    #[test]
    fn test_roundtrip_bold() {
        let input = "**bold text**\n";
        let doc = parse(input).unwrap();
        let output = build(&doc);
        assert!(output.contains("**bold text**"));
    }
}
