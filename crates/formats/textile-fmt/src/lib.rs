//! Textile markup parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-textile` and `rescribe-write-textile` as thin adapter layers.

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct TextileError(pub String);

impl std::fmt::Display for TextileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Textile error: {}", self.0)
    }
}

impl std::error::Error for TextileError {}

// ── AST ───────────────────────────────────────────────────────────────────────

/// A parsed Textile document.
#[derive(Debug, Clone, Default)]
pub struct TextileDoc {
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
    },
    Blockquote {
        inlines: Vec<Inline>,
    },
    List {
        ordered: bool,
        items: Vec<Vec<Block>>,
    },
    Table {
        rows: Vec<TableRow>,
    },
}

/// A table row.
#[derive(Debug, Clone)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
}

/// A table cell.
#[derive(Debug, Clone)]
pub struct TableCell {
    pub is_header: bool,
    pub inlines: Vec<Inline>,
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
    Image { url: String, alt: Option<String> },
    Superscript(Vec<Inline>),
    Subscript(Vec<Inline>),
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Parse a Textile string into a [`TextileDoc`].
pub fn parse(input: &str) -> Result<TextileDoc, TextileError> {
    let mut p = Parser::new(input);
    let blocks = p.parse();
    Ok(TextileDoc { blocks })
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

            // Block code bc. or bc..
            if line.starts_with("bc.") {
                nodes.push(self.parse_code_block());
                continue;
            }

            // Blockquote bq.
            if line.starts_with("bq.") {
                nodes.push(self.parse_blockquote());
                continue;
            }

            // Pre block pre.
            if line.starts_with("pre.") {
                nodes.push(self.parse_pre_block());
                continue;
            }

            // Heading h1. to h6.
            if let Some(node) = self.try_parse_heading(line) {
                nodes.push(node);
                self.pos += 1;
                continue;
            }

            // Table
            if line.trim_start().starts_with('|') {
                nodes.push(self.parse_table());
                continue;
            }

            // List
            if line.trim_start().starts_with("* ")
                || line.trim_start().starts_with("# ")
                || line.trim_start().starts_with("** ")
                || line.trim_start().starts_with("## ")
            {
                nodes.push(self.parse_list());
                continue;
            }

            // Regular paragraph p. or just text
            nodes.push(self.parse_paragraph());
        }

        nodes
    }

    fn try_parse_heading(&self, line: &str) -> Option<Block> {
        for level in 1..=6 {
            let prefix = format!("h{}.", level);
            if line.starts_with(&prefix) {
                let content = line[prefix.len()..].trim();
                let inline_nodes = parse_inline(content);
                return Some(Block::Heading {
                    level: level as u8,
                    inlines: inline_nodes,
                });
            }
        }
        None
    }

    fn parse_code_block(&mut self) -> Block {
        let first_line = self.lines[self.pos];
        let extended = first_line.starts_with("bc..");

        let content_start = if extended { 4 } else { 3 };
        let mut content = String::new();

        // Get content from first line
        let first_content = first_line[content_start..].trim();
        if !first_content.is_empty() {
            content.push_str(first_content);
            content.push('\n');
        }
        self.pos += 1;

        if extended {
            // Extended block continues until blank line
            while self.pos < self.lines.len() {
                let line = self.lines[self.pos];
                if line.trim().is_empty() {
                    break;
                }
                content.push_str(line);
                content.push('\n');
                self.pos += 1;
            }
        }

        Block::CodeBlock {
            content: content.trim_end().to_string(),
        }
    }

    fn parse_pre_block(&mut self) -> Block {
        let first_line = self.lines[self.pos];
        let extended = first_line.starts_with("pre..");

        let content_start = if extended { 5 } else { 4 };
        let mut content = String::new();

        let first_content = first_line[content_start..].trim();
        if !first_content.is_empty() {
            content.push_str(first_content);
            content.push('\n');
        }
        self.pos += 1;

        if extended {
            while self.pos < self.lines.len() {
                let line = self.lines[self.pos];
                if line.trim().is_empty() {
                    break;
                }
                content.push_str(line);
                content.push('\n');
                self.pos += 1;
            }
        }

        Block::CodeBlock {
            content: content.trim_end().to_string(),
        }
    }

    fn parse_blockquote(&mut self) -> Block {
        let first_line = self.lines[self.pos];
        let extended = first_line.starts_with("bq..");

        let content_start = if extended { 4 } else { 3 };
        let mut text = String::new();

        let first_content = first_line[content_start..].trim();
        if !first_content.is_empty() {
            text.push_str(first_content);
        }
        self.pos += 1;

        if extended {
            while self.pos < self.lines.len() {
                let line = self.lines[self.pos];
                if line.trim().is_empty() {
                    break;
                }
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(line.trim());
                self.pos += 1;
            }
        }

        let inline_nodes = parse_inline(&text);
        Block::Blockquote {
            inlines: inline_nodes,
        }
    }

    fn parse_table(&mut self) -> Block {
        let mut rows = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            if !line.trim_start().starts_with('|') {
                break;
            }

            let row = self.parse_table_row(line);
            rows.push(row);
            self.pos += 1;
        }

        Block::Table { rows }
    }

    fn parse_table_row(&self, line: &str) -> TableRow {
        let mut cells = Vec::new();
        let trimmed = line.trim();

        // Remove leading/trailing |
        let inner = trimmed.trim_start_matches('|').trim_end_matches('|');
        let parts: Vec<&str> = inner.split('|').collect();

        for part in parts {
            let part = part.trim();
            let is_header = part.starts_with("_.");
            let cell_content = if is_header { part[2..].trim() } else { part };

            let inline_nodes = parse_inline(cell_content);
            cells.push(TableCell {
                is_header,
                inlines: inline_nodes,
            });
        }

        TableRow { cells }
    }

    fn parse_list(&mut self) -> Block {
        let first_line = self.lines[self.pos];
        let trimmed = first_line.trim_start();
        let ordered = trimmed.starts_with('#');

        let (items, _) = self.parse_list_at_level(1, ordered);
        Block::List { ordered, items }
    }

    fn parse_list_at_level(&mut self, level: usize, ordered: bool) -> (Vec<Vec<Block>>, bool) {
        let marker = if ordered { '#' } else { '*' };
        let mut items = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            let trimmed = line.trim_start();

            // Count markers
            let marker_count = trimmed.chars().take_while(|&c| c == marker).count();

            if marker_count == 0 {
                // Check for other list type or end
                let other_marker = if ordered { '*' } else { '#' };
                let other_count = trimmed.chars().take_while(|&c| c == other_marker).count();
                if other_count == 0 {
                    break;
                }
                if other_count <= level {
                    break;
                }
            }

            if marker_count < level {
                break;
            }

            if marker_count == level
                && trimmed.len() > marker_count
                && trimmed.chars().nth(marker_count) == Some(' ')
            {
                let content = trimmed[marker_count + 1..].trim();
                let inline_nodes = parse_inline(content);
                let para = Block::Paragraph {
                    inlines: inline_nodes,
                };
                let mut item_children = vec![para];

                self.pos += 1;

                // Check for nested list
                if self.pos < self.lines.len() {
                    let next_line = self.lines[self.pos];
                    let next_trimmed = next_line.trim_start();
                    let next_marker_count =
                        next_trimmed.chars().take_while(|&c| c == marker).count();
                    let other_marker = if ordered { '*' } else { '#' };
                    let next_other_count = next_trimmed
                        .chars()
                        .take_while(|&c| c == other_marker)
                        .count();

                    if next_marker_count > level {
                        let (nested_items, _) =
                            self.parse_list_at_level(next_marker_count, ordered);
                        item_children.push(Block::List {
                            ordered,
                            items: nested_items,
                        });
                    } else if next_other_count > level {
                        let (nested_items, _) =
                            self.parse_list_at_level(next_other_count, !ordered);
                        item_children.push(Block::List {
                            ordered: !ordered,
                            items: nested_items,
                        });
                    }
                }

                items.push(item_children);
            } else if marker_count > level {
                break;
            } else {
                self.pos += 1;
            }
        }

        (items, ordered)
    }

    fn parse_paragraph(&mut self) -> Block {
        let mut text = String::new();
        let first_line = self.lines[self.pos];

        // Check for p. prefix
        let first_content = first_line
            .strip_prefix("p.")
            .map(|s| s.trim())
            .unwrap_or_else(|| first_line.trim());

        text.push_str(first_content);
        self.pos += 1;

        // Continue until empty line or block element
        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            if line.trim().is_empty() {
                break;
            }

            // Check for block elements
            if line.starts_with("h1.")
                || line.starts_with("h2.")
                || line.starts_with("h3.")
                || line.starts_with("h4.")
                || line.starts_with("h5.")
                || line.starts_with("h6.")
                || line.starts_with("bc.")
                || line.starts_with("bq.")
                || line.starts_with("pre.")
                || line.starts_with("p.")
                || line.trim_start().starts_with('|')
                || line.trim_start().starts_with("* ")
                || line.trim_start().starts_with("# ")
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

fn parse_inline(text: &str) -> Vec<Inline> {
    let mut nodes = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Inline code @...@
        if chars[i] == '@' {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone()));
                current.clear();
            }

            i += 1;
            let mut code = String::new();
            while i < chars.len() && chars[i] != '@' {
                code.push(chars[i]);
                i += 1;
            }
            if i < chars.len() {
                i += 1; // skip closing @
            }
            nodes.push(Inline::Code(code));
            continue;
        }

        // Try to parse formatting markers
        if let Some((new_i, node)) = try_parse_formatting(&chars, i, &mut current, &mut nodes) {
            i = new_i;
            nodes.push(node);
            continue;
        }

        // Link "text":url
        if chars[i] == '"'
            && let Some((link_end, link_text, url)) = parse_textile_link(&chars, i)
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone()));
                current.clear();
            }
            let text_node = Inline::Text(link_text);
            nodes.push(Inline::Link {
                url,
                children: vec![text_node],
            });
            i = link_end;
            continue;
        }

        // Image !url!
        if chars[i] == '!'
            && let Some((img_end, url, alt)) = parse_textile_image(&chars, i)
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone()));
                current.clear();
            }
            nodes.push(Inline::Image { url, alt });
            i = img_end;
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

fn try_parse_formatting(
    chars: &[char],
    i: usize,
    current: &mut String,
    nodes: &mut Vec<Inline>,
) -> Option<(usize, Inline)> {
    // Define formatting markers: (marker, doubled_marker, variant_fn, check_prev)
    let markers: &[(char, char, bool)] = &[
        ('*', '*', true),  // Bold
        ('_', '_', true),  // Emphasis
        ('-', '-', true),  // Strikeout
        ('+', '+', true),  // Underline
        ('^', ' ', false), // Superscript (^ has no doubled version)
        ('~', ' ', false), // Subscript (~ has no doubled version)
    ];

    for &(marker, doubled, check_prev) in markers {
        if chars[i] != marker {
            continue;
        }

        // Check previous char if needed
        if check_prev && i > 0 && chars[i - 1].is_alphanumeric() {
            continue;
        }

        // Check next char exists and is valid
        if i + 1 >= chars.len() || chars[i + 1] == ' ' {
            continue;
        }

        // Skip if doubled marker
        if doubled != ' ' && chars[i + 1] == doubled {
            continue;
        }

        if let Some((end, content)) = find_closing_marker(chars, i + 1, marker) {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone()));
                current.clear();
            }
            let inner = parse_inline(&content);

            let result = match marker {
                '*' => Inline::Bold(inner),
                '_' => Inline::Italic(inner),
                '-' => Inline::Strikethrough(inner),
                '+' => Inline::Underline(inner),
                '^' => Inline::Superscript(inner),
                '~' => Inline::Subscript(inner),
                _ => return None,
            };

            return Some((end + 1, result));
        }
    }

    None
}

fn find_closing_marker(chars: &[char], start: usize, marker: char) -> Option<(usize, String)> {
    let mut i = start;
    let mut content = String::new();

    while i < chars.len() {
        if chars[i] == marker && (i + 1 >= chars.len() || !chars[i + 1].is_alphanumeric()) {
            return Some((i, content));
        }
        content.push(chars[i]);
        i += 1;
    }
    None
}

fn parse_textile_link(chars: &[char], start: usize) -> Option<(usize, String, String)> {
    // "text":url
    if chars[start] != '"' {
        return None;
    }

    let mut i = start + 1;
    let mut link_text = String::new();

    // Find closing "
    while i < chars.len() && chars[i] != '"' {
        link_text.push(chars[i]);
        i += 1;
    }

    if i >= chars.len() || chars[i] != '"' {
        return None;
    }
    i += 1; // skip "

    // Must be followed by :
    if i >= chars.len() || chars[i] != ':' {
        return None;
    }
    i += 1; // skip :

    // Collect URL until whitespace or end
    let mut url = String::new();
    while i < chars.len() && !chars[i].is_whitespace() {
        url.push(chars[i]);
        i += 1;
    }

    if url.is_empty() {
        return None;
    }

    Some((i, link_text, url))
}

fn parse_textile_image(chars: &[char], start: usize) -> Option<(usize, String, Option<String>)> {
    // !url! or !url(alt)!
    if chars[start] != '!' {
        return None;
    }

    let mut i = start + 1;
    let mut url = String::new();
    let mut alt = None;

    while i < chars.len() && chars[i] != '!' && chars[i] != '(' {
        url.push(chars[i]);
        i += 1;
    }

    if i >= chars.len() {
        return None;
    }

    // Alt text in parentheses
    if chars[i] == '(' {
        i += 1;
        let mut alt_text = String::new();
        while i < chars.len() && chars[i] != ')' {
            alt_text.push(chars[i]);
            i += 1;
        }
        if i < chars.len() && chars[i] == ')' {
            alt = Some(alt_text);
            i += 1;
        }
    }

    // Must end with !
    if i >= chars.len() || chars[i] != '!' {
        return None;
    }
    i += 1;

    if url.is_empty() {
        return None;
    }

    Some((i, url, alt))
}

// ── Builder ───────────────────────────────────────────────────────────────────

/// Build a Textile string from a [`TextileDoc`].
pub fn build(doc: &TextileDoc) -> String {
    let mut ctx = BuildContext::new();
    for block in &doc.blocks {
        build_block(block, &mut ctx);
    }
    ctx.output
}

struct BuildContext {
    output: String,
    list_depth: usize,
}

impl BuildContext {
    fn new() -> Self {
        Self {
            output: String::new(),
            list_depth: 0,
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
            ctx.write(&format!("h{}. ", level));
            build_inlines(inlines, ctx);
            ctx.write("\n\n");
        }

        Block::CodeBlock { content } => {
            ctx.write("bc. ");
            ctx.write(content);
            ctx.write("\n\n");
        }

        Block::Blockquote { inlines } => {
            ctx.write("bq. ");
            build_inlines(inlines, ctx);
            ctx.write("\n\n");
        }

        Block::List { ordered, items } => {
            let marker = if *ordered { "#" } else { "*" };
            ctx.list_depth += 1;

            for item_blocks in items {
                for _ in 0..ctx.list_depth {
                    ctx.write(marker);
                }
                ctx.write(" ");

                for item_child in item_blocks {
                    match item_child {
                        Block::Paragraph { inlines } => {
                            build_inlines(inlines, ctx);
                        }
                        Block::List { .. } => {
                            ctx.write("\n");
                            build_block(item_child, ctx);
                            continue;
                        }
                        other => build_block(other, ctx),
                    }
                }
                ctx.write("\n");
            }

            ctx.list_depth -= 1;
            if ctx.list_depth == 0 {
                ctx.write("\n");
            }
        }

        Block::Table { rows } => {
            for row in rows {
                for cell in &row.cells {
                    ctx.write("|");
                    if cell.is_header {
                        ctx.write("_. ");
                    }
                    build_inlines(&cell.inlines, ctx);
                }
                ctx.write("|\n");
            }
            ctx.write("\n");
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
            ctx.write("*");
            build_inlines(children, ctx);
            ctx.write("*");
        }

        Inline::Italic(children) => {
            ctx.write("_");
            build_inlines(children, ctx);
            ctx.write("_");
        }

        Inline::Strikethrough(children) => {
            ctx.write("-");
            build_inlines(children, ctx);
            ctx.write("-");
        }

        Inline::Underline(children) => {
            ctx.write("+");
            build_inlines(children, ctx);
            ctx.write("+");
        }

        Inline::Superscript(children) => {
            ctx.write("^");
            build_inlines(children, ctx);
            ctx.write("^");
        }

        Inline::Subscript(children) => {
            ctx.write("~");
            build_inlines(children, ctx);
            ctx.write("~");
        }

        Inline::Code(s) => {
            ctx.write("@");
            ctx.write(s);
            ctx.write("@");
        }

        Inline::Link { url, children } => {
            ctx.write("\"");
            build_inlines(children, ctx);
            ctx.write("\":");
            ctx.write(url);
        }

        Inline::Image { url, alt } => {
            ctx.write("!");
            ctx.write(url);
            if let Some(alt_text) = alt {
                ctx.write("(");
                ctx.write(alt_text);
                ctx.write(")");
            }
            ctx.write("!");
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_heading() {
        let doc = parse("h1. Title\n").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Heading { level: 1, .. }));
    }

    #[test]
    fn test_parse_heading_levels() {
        let doc = parse("h2. Level 2\nh3. Level 3\n").unwrap();
        assert_eq!(doc.blocks.len(), 2);
        assert!(matches!(doc.blocks[0], Block::Heading { level: 2, .. }));
        assert!(matches!(doc.blocks[1], Block::Heading { level: 3, .. }));
    }

    #[test]
    fn test_parse_paragraph() {
        let doc = parse("Hello world\n").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Paragraph { .. }));
    }

    #[test]
    fn test_parse_bold() {
        let doc = parse("*bold*\n").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert_eq!(inlines.len(), 1);
        assert!(matches!(inlines[0], Inline::Bold(_)));
    }

    #[test]
    fn test_parse_italic() {
        let doc = parse("_italic_\n").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert_eq!(inlines.len(), 1);
        assert!(matches!(inlines[0], Inline::Italic(_)));
    }

    #[test]
    fn test_parse_code() {
        let doc = parse("@code@\n").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert_eq!(inlines.len(), 1);
        assert!(matches!(inlines[0], Inline::Code(_)));
    }

    #[test]
    fn test_parse_link() {
        let doc = parse("\"Example\":https://example.com\n").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        let link = &inlines[0];
        assert!(matches!(link, Inline::Link { .. }));
        if let Inline::Link { url, .. } = link {
            assert_eq!(url, "https://example.com");
        }
    }

    #[test]
    fn test_parse_list() {
        let doc = parse("* item1\n* item2\n").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::List { .. }));
        if let Block::List { items, .. } = &doc.blocks[0] {
            assert_eq!(items.len(), 2);
        }
    }

    #[test]
    fn test_parse_code_block() {
        let doc = parse("bc. code here\n").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::CodeBlock { .. }));
    }

    #[test]
    fn test_build_heading() {
        let doc = TextileDoc {
            blocks: vec![Block::Heading {
                level: 1,
                inlines: vec![Inline::Text("Title".to_string())],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("h1. Title"));
    }

    #[test]
    fn test_build_paragraph() {
        let doc = TextileDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Hello, world!".to_string())],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("Hello, world!"));
    }

    #[test]
    fn test_build_bold() {
        let doc = TextileDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Bold(vec![Inline::Text("bold".to_string())])],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("*bold*"));
    }

    #[test]
    fn test_build_italic() {
        let doc = TextileDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Italic(vec![Inline::Text("italic".to_string())])],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("_italic_"));
    }

    #[test]
    fn test_build_code() {
        let doc = TextileDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Code("code".to_string())],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("@code@"));
    }

    #[test]
    fn test_build_link() {
        let doc = TextileDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "https://example.com".to_string(),
                    children: vec![Inline::Text("click".to_string())],
                }],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("\"click\":https://example.com"));
    }

    #[test]
    fn test_build_list() {
        let doc = TextileDoc {
            blocks: vec![Block::List {
                ordered: false,
                items: vec![
                    vec![Block::Paragraph {
                        inlines: vec![Inline::Text("one".to_string())],
                    }],
                    vec![Block::Paragraph {
                        inlines: vec![Inline::Text("two".to_string())],
                    }],
                ],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("* one"));
        assert!(output.contains("* two"));
    }

    #[test]
    fn test_build_ordered_list() {
        let doc = TextileDoc {
            blocks: vec![Block::List {
                ordered: true,
                items: vec![
                    vec![Block::Paragraph {
                        inlines: vec![Inline::Text("first".to_string())],
                    }],
                    vec![Block::Paragraph {
                        inlines: vec![Inline::Text("second".to_string())],
                    }],
                ],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("# first"));
        assert!(output.contains("# second"));
    }

    #[test]
    fn test_build_code_block() {
        let doc = TextileDoc {
            blocks: vec![Block::CodeBlock {
                content: "print('hi')".to_string(),
            }],
        };
        let output = build(&doc);
        assert!(output.contains("bc. print('hi')"));
    }

    #[test]
    fn test_parse_image() {
        let doc = parse("!image.png!\n").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(matches!(inlines[0], Inline::Image { .. }));
    }

    #[test]
    fn test_build_image() {
        let doc = TextileDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Image {
                    url: "image.png".to_string(),
                    alt: None,
                }],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("!image.png!"));
    }

    #[test]
    fn test_build_image_with_alt() {
        let doc = TextileDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Image {
                    url: "image.png".to_string(),
                    alt: Some("alt text".to_string()),
                }],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("!image.png(alt text)!"));
    }
}
