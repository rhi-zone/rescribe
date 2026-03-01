//! txt2tags (t2t) parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-t2t` and `rescribe-write-t2t` as thin adapter layers.

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct T2tError(pub String);

impl std::fmt::Display for T2tError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "txt2tags error: {}", self.0)
    }
}

impl std::error::Error for T2tError {}

// ── AST ───────────────────────────────────────────────────────────────────────

/// A parsed txt2tags document.
#[derive(Debug, Clone, Default)]
pub struct T2tDoc {
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
        numbered: bool,
        inlines: Vec<Inline>,
    },
    CodeBlock {
        content: String,
    },
    RawBlock {
        content: String,
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
}

/// A table row.
#[derive(Debug, Clone)]
pub struct TableRow {
    pub cells: Vec<Vec<Inline>>,
    pub is_header: bool,
}

/// Inline element.
#[derive(Debug, Clone)]
pub enum Inline {
    Text(String),
    Bold(Vec<Inline>),
    Italic(Vec<Inline>),
    Underline(Vec<Inline>),
    Strikethrough(Vec<Inline>),
    Code(String),
    Link { url: String, children: Vec<Inline> },
    Image { url: String },
    LineBreak,
    SoftBreak,
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Parse a txt2tags string into a [`T2tDoc`].
pub fn parse(input: &str) -> Result<T2tDoc, T2tError> {
    let mut p = Parser::new(input);
    let blocks = p.parse();
    Ok(T2tDoc { blocks })
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
        let mut blocks = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            // Skip empty lines
            if line.trim().is_empty() {
                self.pos += 1;
                continue;
            }

            // Comment (% at start of line)
            if line.starts_with('%') {
                self.pos += 1;
                continue;
            }

            // Verbatim block (```)
            if line.trim() == "```" {
                blocks.push(self.parse_verbatim_block());
                continue;
            }

            // Raw block (""")
            if line.trim() == "\"\"\"" {
                blocks.push(self.parse_raw_block());
                continue;
            }

            // Heading = Title = or + Title +
            if let Some(block) = self.try_parse_heading(line) {
                blocks.push(block);
                self.pos += 1;
                continue;
            }

            // Horizontal rule (20+ dashes, equals, or underscores)
            if is_horizontal_rule(line) {
                blocks.push(Block::HorizontalRule);
                self.pos += 1;
                continue;
            }

            // Quote (lines starting with TAB)
            if line.starts_with('\t') {
                blocks.push(self.parse_quote());
                continue;
            }

            // Unordered list (- item)
            if line.trim_start().starts_with("- ") {
                blocks.push(self.parse_list(false));
                continue;
            }

            // Ordered list (+ item)
            if line.trim_start().starts_with("+ ") {
                blocks.push(self.parse_list(true));
                continue;
            }

            // Table (| cell |)
            if line.trim_start().starts_with('|') {
                blocks.push(self.parse_table());
                continue;
            }

            // Regular paragraph
            blocks.push(self.parse_paragraph());
        }

        blocks
    }

    fn try_parse_heading(&self, line: &str) -> Option<Block> {
        let trimmed = line.trim();

        // Check for = or + delimited headings
        for (marker, numbered) in [('=', false), ('+', true)] {
            let level = trimmed.chars().take_while(|&c| c == marker).count();
            if level > 0 && level <= 5 {
                let end_marker_count = trimmed.chars().rev().take_while(|&c| c == marker).count();
                if end_marker_count >= level {
                    // Extract content between markers
                    let content_start = level;
                    let content_end = trimmed.len() - end_marker_count;
                    if content_start < content_end {
                        let content = trimmed[content_start..content_end].trim();
                        // Check for label [label-name]
                        let (text, _label) = if let Some(bracket_pos) = content.rfind('[') {
                            if content.ends_with(']') {
                                (
                                    content[..bracket_pos].trim(),
                                    Some(&content[bracket_pos + 1..content.len() - 1]),
                                )
                            } else {
                                (content, None)
                            }
                        } else {
                            (content, None)
                        };

                        let inlines = parse_inline(text);
                        return Some(Block::Heading {
                            level: level as u8,
                            numbered,
                            inlines,
                        });
                    }
                }
            }
        }
        None
    }

    fn parse_verbatim_block(&mut self) -> Block {
        let mut content = String::new();
        self.pos += 1; // Skip opening ```

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            if line.trim() == "```" {
                self.pos += 1;
                break;
            }
            if !content.is_empty() {
                content.push('\n');
            }
            content.push_str(line);
            self.pos += 1;
        }

        Block::CodeBlock { content }
    }

    fn parse_raw_block(&mut self) -> Block {
        let mut content = String::new();
        self.pos += 1; // Skip opening """

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            if line.trim() == "\"\"\"" {
                self.pos += 1;
                break;
            }
            if !content.is_empty() {
                content.push('\n');
            }
            content.push_str(line);
            self.pos += 1;
        }

        Block::RawBlock { content }
    }

    fn parse_quote(&mut self) -> Block {
        let mut content = String::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            if !line.starts_with('\t') {
                break;
            }
            if !content.is_empty() {
                content.push(' ');
            }
            content.push_str(line[1..].trim());
            self.pos += 1;
        }

        let inlines = parse_inline(&content);
        Block::Blockquote {
            children: vec![Block::Paragraph { inlines }],
        }
    }

    fn parse_list(&mut self, ordered: bool) -> Block {
        let mut items = Vec::new();
        let marker = if ordered { "+ " } else { "- " };

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            let trimmed = line.trim_start();

            if !trimmed.starts_with(marker) {
                break;
            }

            let content = &trimmed[2..];
            let inlines = parse_inline(content);
            let item = vec![Block::Paragraph { inlines }];
            items.push(item);
            self.pos += 1;
        }

        Block::List { ordered, items }
    }

    fn parse_table(&mut self) -> Block {
        let mut rows = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            let trimmed = line.trim();

            if !trimmed.starts_with('|') {
                break;
            }

            let is_header = trimmed.starts_with("||");
            let row_content = if is_header {
                &trimmed[2..]
            } else {
                &trimmed[1..]
            };

            let mut cells = Vec::new();
            for cell_text in row_content.split('|') {
                let cell_text = cell_text.trim();
                if cell_text.is_empty() && cells.is_empty() {
                    continue; // Skip empty leading cell
                }
                if cell_text.is_empty() {
                    continue; // Skip empty cells
                }
                let inlines = parse_inline(cell_text);
                cells.push(inlines);
            }

            if !cells.is_empty() {
                rows.push(TableRow { cells, is_header });
            }
            self.pos += 1;
        }

        Block::Table { rows }
    }

    fn parse_paragraph(&mut self) -> Block {
        let mut text = String::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            // End paragraph at empty line or block element
            if line.trim().is_empty()
                || line.starts_with('%')
                || line.trim() == "```"
                || line.trim() == "\"\"\""
                || self.try_parse_heading(line).is_some()
                || is_horizontal_rule(line)
                || line.starts_with('\t')
                || line.trim_start().starts_with("- ")
                || line.trim_start().starts_with("+ ")
                || line.trim_start().starts_with('|')
            {
                break;
            }

            if !text.is_empty() {
                text.push(' ');
            }
            text.push_str(line.trim());
            self.pos += 1;
        }

        let inlines = parse_inline(&text);
        Block::Paragraph { inlines }
    }
}

fn is_horizontal_rule(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.len() >= 20 {
        let first_char = trimmed.chars().next().unwrap_or(' ');
        if first_char == '-' || first_char == '=' || first_char == '_' {
            return trimmed.chars().all(|c| c == first_char);
        }
    }
    false
}

fn parse_inline(text: &str) -> Vec<Inline> {
    let mut nodes = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Bold **text**
        if chars[i] == '*'
            && i + 1 < chars.len()
            && chars[i + 1] == '*'
            && let Some((end, content)) = find_double_closing(&chars, i + 2, '*')
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone()));
                current.clear();
            }
            let inner = parse_inline(&content);
            nodes.push(Inline::Bold(inner));
            i = end + 2;
            continue;
        }

        // Italic //text//
        if chars[i] == '/'
            && i + 1 < chars.len()
            && chars[i + 1] == '/'
            && let Some((end, content)) = find_double_closing(&chars, i + 2, '/')
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone()));
                current.clear();
            }
            let inner = parse_inline(&content);
            nodes.push(Inline::Italic(inner));
            i = end + 2;
            continue;
        }

        // Underline __text__
        if chars[i] == '_'
            && i + 1 < chars.len()
            && chars[i + 1] == '_'
            && let Some((end, content)) = find_double_closing(&chars, i + 2, '_')
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone()));
                current.clear();
            }
            let inner = parse_inline(&content);
            nodes.push(Inline::Underline(inner));
            i = end + 2;
            continue;
        }

        // Strikethrough --text--
        if chars[i] == '-'
            && i + 1 < chars.len()
            && chars[i + 1] == '-'
            && let Some((end, content)) = find_double_closing(&chars, i + 2, '-')
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

        // Monospace ``text``
        if chars[i] == '`'
            && i + 1 < chars.len()
            && chars[i + 1] == '`'
            && let Some((end, content)) = find_double_closing(&chars, i + 2, '`')
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone()));
                current.clear();
            }
            nodes.push(Inline::Code(content));
            i = end + 2;
            continue;
        }

        // Link [label url] or image [filename.ext]
        if chars[i] == '['
            && let Some((end, label, url)) = parse_link_or_image(&chars, i)
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone()));
                current.clear();
            }
            if is_image_url(&url) {
                nodes.push(Inline::Image { url });
            } else {
                let text_node = Inline::Text(label);
                nodes.push(Inline::Link {
                    url,
                    children: vec![text_node],
                });
            }
            i = end;
            continue;
        }

        // Auto-detect URLs
        if (chars[i] == 'h' || chars[i] == 'H')
            && let Some((end, url)) = try_parse_url(&chars, i)
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone()));
                current.clear();
            }
            let text_node = Inline::Text(url.clone());
            nodes.push(Inline::Link {
                url,
                children: vec![text_node],
            });
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

fn find_double_closing(chars: &[char], start: usize, marker: char) -> Option<(usize, String)> {
    let mut i = start;
    let mut content = String::new();

    while i + 1 < chars.len() {
        if chars[i] == marker && chars[i + 1] == marker {
            return Some((i, content));
        }
        content.push(chars[i]);
        i += 1;
    }
    None
}

fn parse_link_or_image(chars: &[char], start: usize) -> Option<(usize, String, String)> {
    // [label url] or [filename.ext]
    if start >= chars.len() || chars[start] != '[' {
        return None;
    }

    let mut i = start + 1;
    let mut content = String::new();

    while i < chars.len() && chars[i] != ']' {
        content.push(chars[i]);
        i += 1;
    }

    if i >= chars.len() {
        return None;
    }

    let content = content.trim();

    // Check if it's [label url] format
    if let Some(space_pos) = content.rfind(' ') {
        let label = content[..space_pos].trim();
        let url = content[space_pos + 1..].trim();
        // Ensure URL looks like a URL
        if url.contains('.') || url.starts_with('#') || url.starts_with("http") {
            return Some((i + 1, label.to_string(), url.to_string()));
        }
    }

    // Single item - could be URL or image
    if content.contains('.') {
        return Some((i + 1, content.to_string(), content.to_string()));
    }

    None
}

fn is_image_url(url: &str) -> bool {
    let lower = url.to_lowercase();
    lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".gif")
        || lower.ends_with(".svg")
        || lower.ends_with(".webp")
}

fn try_parse_url(chars: &[char], start: usize) -> Option<(usize, String)> {
    let rest: String = chars[start..].iter().collect();
    if rest.starts_with("http://")
        || rest.starts_with("https://")
        || rest.starts_with("HTTP://")
        || rest.starts_with("HTTPS://")
    {
        let mut end = start;
        while end < chars.len() && !chars[end].is_whitespace() {
            end += 1;
        }
        let url: String = chars[start..end].iter().collect();
        return Some((end, url));
    }
    None
}

// ── Builder ───────────────────────────────────────────────────────────────────

/// Build a txt2tags string from a [`T2tDoc`].
pub fn build(doc: &T2tDoc) -> String {
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

        Block::Heading {
            level,
            numbered,
            inlines,
        } => {
            let marker = if *numbered { '+' } else { '=' };
            let level_capped = (*level as usize).min(5);

            for _ in 0..level_capped {
                ctx.write(&marker.to_string());
            }
            ctx.write(" ");
            build_inlines(inlines, ctx);
            ctx.write(" ");
            for _ in 0..level_capped {
                ctx.write(&marker.to_string());
            }
            ctx.write("\n\n");
        }

        Block::CodeBlock { content } => {
            ctx.write("```\n");
            ctx.write(content);
            if !content.ends_with('\n') {
                ctx.write("\n");
            }
            ctx.write("```\n\n");
        }

        Block::RawBlock { content } => {
            ctx.write("\"\"\"\n");
            ctx.write(content);
            if !content.ends_with('\n') {
                ctx.write("\n");
            }
            ctx.write("\"\"\"\n\n");
        }

        Block::Blockquote { children } => {
            for child in children {
                if let Block::Paragraph { inlines } = child {
                    ctx.write("\t");
                    build_inlines(inlines, ctx);
                    ctx.write("\n");
                } else {
                    build_block(child, ctx);
                }
            }
            ctx.write("\n");
        }

        Block::List { ordered, items } => {
            let marker = if *ordered { "+ " } else { "- " };

            for item_blocks in items {
                ctx.write(marker);
                for block in item_blocks {
                    if let Block::Paragraph { inlines } = block {
                        build_inlines(inlines, ctx);
                    } else {
                        build_block(block, ctx);
                    }
                }
                ctx.write("\n");
            }
            ctx.write("\n");
        }

        Block::Table { rows } => {
            for row in rows {
                if row.is_header {
                    ctx.write("||");
                } else {
                    ctx.write("|");
                }

                for cell in &row.cells {
                    ctx.write(" ");
                    build_inlines(cell, ctx);
                    ctx.write(" |");
                }
                ctx.write("\n");
            }
            ctx.write("\n");
        }

        Block::HorizontalRule => {
            ctx.write("--------------------\n\n");
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

        Inline::Bold(children) => {
            ctx.write("**");
            build_inlines(children, ctx);
            ctx.write("**");
        }

        Inline::Italic(children) => {
            ctx.write("//");
            build_inlines(children, ctx);
            ctx.write("//");
        }

        Inline::Underline(children) => {
            ctx.write("__");
            build_inlines(children, ctx);
            ctx.write("__");
        }

        Inline::Strikethrough(children) => {
            ctx.write("--");
            build_inlines(children, ctx);
            ctx.write("--");
        }

        Inline::Code(s) => {
            ctx.write("``");
            ctx.write(s);
            ctx.write("``");
        }

        Inline::Link { url, children } => {
            ctx.write("[");
            if !children.is_empty() {
                build_inlines(children, ctx);
                ctx.write(" ");
            }
            ctx.write(url);
            ctx.write("]");
        }

        Inline::Image { url } => {
            ctx.write("[");
            ctx.write(url);
            ctx.write("]");
        }

        Inline::LineBreak => {
            ctx.write("\n");
        }

        Inline::SoftBreak => {
            ctx.write(" ");
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_str(input: &str) -> T2tDoc {
        parse(input).unwrap()
    }

    #[test]
    fn test_parse_heading() {
        let doc = parse_str("= Title =\n");
        assert_eq!(doc.blocks.len(), 1);
        match &doc.blocks[0] {
            Block::Heading {
                level,
                numbered,
                inlines: _,
            } => {
                assert_eq!(*level, 1);
                assert!(!*numbered);
            }
            _ => panic!("expected heading"),
        }
    }

    #[test]
    fn test_parse_heading_level2() {
        let doc = parse_str("== Subtitle ==\n");
        match &doc.blocks[0] {
            Block::Heading {
                level,
                numbered: _,
                inlines: _,
            } => {
                assert_eq!(*level, 2);
            }
            _ => panic!("expected heading"),
        }
    }

    #[test]
    fn test_parse_numbered_heading() {
        let doc = parse_str("+ Numbered +\n");
        match &doc.blocks[0] {
            Block::Heading {
                level: _,
                numbered,
                inlines: _,
            } => {
                assert!(*numbered);
            }
            _ => panic!("expected heading"),
        }
    }

    #[test]
    fn test_parse_paragraph() {
        let doc = parse_str("Hello world\n");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Paragraph { .. }));
    }

    #[test]
    fn test_parse_bold() {
        let doc = parse_str("**bold**\n");
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(matches!(inlines[0], Inline::Bold(_)));
    }

    #[test]
    fn test_parse_italic() {
        let doc = parse_str("//italic//\n");
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(matches!(inlines[0], Inline::Italic(_)));
    }

    #[test]
    fn test_parse_underline() {
        let doc = parse_str("__underline__\n");
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(matches!(inlines[0], Inline::Underline(_)));
    }

    #[test]
    fn test_parse_strikethrough() {
        let doc = parse_str("--strike--\n");
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(matches!(inlines[0], Inline::Strikethrough(_)));
    }

    #[test]
    fn test_parse_monospace() {
        let doc = parse_str("``code``\n");
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(matches!(inlines[0], Inline::Code(_)));
    }

    #[test]
    fn test_parse_unordered_list() {
        let doc = parse_str("- item1\n- item2\n");
        assert_eq!(doc.blocks.len(), 1);
        let Block::List { ordered, items } = &doc.blocks[0] else {
            panic!("expected list");
        };
        assert!(!*ordered);
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_parse_ordered_list() {
        let doc = parse_str("+ first\n+ second\n");
        let Block::List { ordered, items: _ } = &doc.blocks[0] else {
            panic!("expected list");
        };
        assert!(*ordered);
    }

    #[test]
    fn test_parse_verbatim_block() {
        let doc = parse_str("```\ncode here\n```\n");
        assert!(matches!(doc.blocks[0], Block::CodeBlock { .. }));
        let Block::CodeBlock { content } = &doc.blocks[0] else {
            panic!("expected code block");
        };
        assert_eq!(content, "code here");
    }

    #[test]
    fn test_parse_link() {
        let doc = parse_str("[click here http://example.com]\n");
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        let link = &inlines[0];
        assert!(matches!(link, Inline::Link { .. }));
        if let Inline::Link { url, .. } = link {
            assert_eq!(url, "http://example.com");
        }
    }

    #[test]
    fn test_parse_quote() {
        let doc = parse_str("\tquoted text\n");
        assert!(matches!(doc.blocks[0], Block::Blockquote { .. }));
    }

    #[test]
    fn test_skip_comments() {
        let doc = parse_str("% comment\ntext\n");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Paragraph { .. }));
    }

    #[test]
    fn test_build_heading() {
        let doc = T2tDoc {
            blocks: vec![Block::Heading {
                level: 1,
                numbered: false,
                inlines: vec![Inline::Text("Title".into())],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("= Title ="));
    }

    #[test]
    fn test_build_paragraph() {
        let doc = T2tDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Hello, world!".into())],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("Hello, world!"));
    }

    #[test]
    fn test_build_bold() {
        let doc = T2tDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Bold(vec![Inline::Text("bold".into())])],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("**bold**"));
    }

    #[test]
    fn test_build_italic() {
        let doc = T2tDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Italic(vec![Inline::Text("italic".into())])],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("//italic//"));
    }

    #[test]
    fn test_build_code_block() {
        let doc = T2tDoc {
            blocks: vec![Block::CodeBlock {
                content: "print hi".into(),
            }],
        };
        let output = build(&doc);
        assert!(output.contains("```"));
        assert!(output.contains("print hi"));
    }
}
