//! Creole wiki parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-creole` and `rescribe-write-creole` as thin adapter layers.

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct CreoleError(pub String);

impl std::fmt::Display for CreoleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Creole error: {}", self.0)
    }
}

impl std::error::Error for CreoleError {}

// ── AST ───────────────────────────────────────────────────────────────────────

/// A parsed Creole document.
#[derive(Debug, Clone, Default)]
pub struct CreoleDoc {
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
    Code(String),
    Link { url: String, children: Vec<Inline> },
    Image { url: String, alt: Option<String> },
    LineBreak,
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Parse a Creole string into a [`CreoleDoc`].
pub fn parse(input: &str) -> Result<CreoleDoc, CreoleError> {
    let mut p = Parser::new(input);
    let blocks = p.parse();
    Ok(CreoleDoc { blocks })
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

            // Nowiki block {{{ ... }}}
            if line.trim_start().starts_with("{{{") {
                nodes.push(self.parse_nowiki_block());
                continue;
            }

            // Heading = to ======
            if let Some(node) = self.try_parse_heading(line) {
                nodes.push(node);
                self.pos += 1;
                continue;
            }

            // Horizontal rule ----
            if line.trim().starts_with("----") {
                nodes.push(Block::HorizontalRule);
                self.pos += 1;
                continue;
            }

            // Table
            if line.trim_start().starts_with('|') {
                nodes.push(self.parse_table());
                continue;
            }

            // List - but not bold **text**
            let trimmed = line.trim_start();
            if (trimmed.starts_with('*') && !trimmed.starts_with("**"))
                || (trimmed.starts_with('#') && !trimmed.starts_with("##"))
            {
                nodes.push(self.parse_list());
                continue;
            }

            // Paragraph
            nodes.push(self.parse_paragraph());
        }

        nodes
    }

    fn try_parse_heading(&self, line: &str) -> Option<Block> {
        let trimmed = line.trim_start();
        let level = trimmed.chars().take_while(|&c| c == '=').count();

        if level > 0 && level <= 6 {
            let rest = trimmed[level..].trim();
            // Remove trailing = if present
            let content = rest.trim_end_matches('=').trim();
            let inline_nodes = Self::parse_inline(content);

            return Some(Block::Heading {
                level: level as u8,
                inlines: inline_nodes,
            });
        }
        None
    }

    fn parse_nowiki_block(&mut self) -> Block {
        let first_line = self.lines[self.pos];
        let content_start = first_line.find("{{{").unwrap() + 3;
        let mut content = String::new();

        // Check if it ends on the same line
        if let Some(end_pos) = first_line[content_start..].find("}}}") {
            content.push_str(&first_line[content_start..content_start + end_pos]);
            self.pos += 1;
        } else {
            // Multi-line nowiki
            if content_start < first_line.len() {
                content.push_str(&first_line[content_start..]);
                content.push('\n');
            }
            self.pos += 1;

            while self.pos < self.lines.len() {
                let line = self.lines[self.pos];
                if let Some(end_pos) = line.find("}}}") {
                    content.push_str(&line[..end_pos]);
                    self.pos += 1;
                    break;
                } else {
                    content.push_str(line);
                    content.push('\n');
                    self.pos += 1;
                }
            }
        }

        Block::CodeBlock { content }
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

        // Split by | but skip empty first/last
        let parts: Vec<&str> = trimmed.split('|').collect();

        for part in parts {
            if part.is_empty() {
                continue;
            }

            let is_header = part.starts_with('=');
            let cell_content = if is_header {
                part[1..].trim()
            } else {
                part.trim()
            };

            let inline_nodes = Self::parse_inline(cell_content);
            cells.push(TableCell {
                is_header,
                inlines: inline_nodes,
            });
        }

        TableRow { cells }
    }

    fn parse_list(&mut self) -> Block {
        let first_line = self.lines[self.pos];
        let first_char = first_line.trim_start().chars().next().unwrap();
        let ordered = first_char == '#';

        let (items, _) = self.parse_list_at_level(1, ordered);
        Block::List { ordered, items }
    }

    fn parse_list_at_level(&mut self, level: usize, ordered: bool) -> (Vec<Vec<Block>>, usize) {
        let marker = if ordered { '#' } else { '*' };
        let mut items = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            let trimmed = line.trim_start();

            // Count markers
            let marker_count = trimmed.chars().take_while(|&c| c == marker).count();

            if marker_count == 0 {
                // Check for other list type
                let other_marker = if ordered { '*' } else { '#' };
                let other_count = trimmed.chars().take_while(|&c| c == other_marker).count();
                if other_count == 0 {
                    break;
                }
                // Different list type at same level - break
                if other_count == level {
                    break;
                }
            }

            if marker_count < level {
                break;
            }

            if marker_count == level {
                let content = trimmed[marker_count..].trim();
                let inline_nodes = Self::parse_inline(content);
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
                        let (nested, _) = self.parse_list_at_level(next_marker_count, ordered);
                        item_children.push(Block::List {
                            ordered,
                            items: nested,
                        });
                    } else if next_other_count > 0 {
                        let (nested, _) = self.parse_list_at_level(next_other_count, !ordered);
                        item_children.push(Block::List {
                            ordered: !ordered,
                            items: nested,
                        });
                    }
                }

                items.push(item_children);
            } else if marker_count > level {
                // Nested list - handled by item above
                break;
            }
        }

        (items, level)
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
            if trimmed.starts_with('=')
                || trimmed.starts_with("----")
                || trimmed.starts_with('|')
                || (trimmed.starts_with('*') && !trimmed.starts_with("**"))
                || (trimmed.starts_with('#') && !trimmed.starts_with("##"))
                || trimmed.starts_with("{{{")
            {
                break;
            }

            if !text.is_empty() {
                text.push(' ');
            }
            text.push_str(line.trim());
            self.pos += 1;
        }

        let inline_nodes = Self::parse_inline(&text);
        Block::Paragraph {
            inlines: inline_nodes,
        }
    }

    fn parse_inline(text: &str) -> Vec<Inline> {
        let mut nodes = Vec::new();
        let mut current = String::new();
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // Line break \\
            if i + 1 < chars.len() && chars[i] == '\\' && chars[i + 1] == '\\' {
                if !current.is_empty() {
                    nodes.push(Inline::Text(current.clone()));
                    current.clear();
                }
                nodes.push(Inline::LineBreak);
                i += 2;
                continue;
            }

            // Inline nowiki {{{...}}}
            if i + 2 < chars.len() && chars[i] == '{' && chars[i + 1] == '{' && chars[i + 2] == '{'
            {
                if !current.is_empty() {
                    nodes.push(Inline::Text(current.clone()));
                    current.clear();
                }

                i += 3;
                let mut code = String::new();
                while i + 2 < chars.len() {
                    if chars[i] == '}' && chars[i + 1] == '}' && chars[i + 2] == '}' {
                        i += 3;
                        break;
                    }
                    code.push(chars[i]);
                    i += 1;
                }
                nodes.push(Inline::Code(code));
                continue;
            }

            // Bold **...**
            if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '*' {
                if !current.is_empty() {
                    nodes.push(Inline::Text(current.clone()));
                    current.clear();
                }

                i += 2;
                let mut bold_text = String::new();
                while i + 1 < chars.len() {
                    if chars[i] == '*' && chars[i + 1] == '*' {
                        i += 2;
                        break;
                    }
                    bold_text.push(chars[i]);
                    i += 1;
                }
                let inner = Self::parse_inline(&bold_text);
                nodes.push(Inline::Bold(inner));
                continue;
            }

            // Italic //...//
            if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '/' {
                // Make sure we're not in a URL (preceded by :)
                let preceded_by_colon = i > 0 && chars[i - 1] == ':';
                if !preceded_by_colon {
                    if !current.is_empty() {
                        nodes.push(Inline::Text(current.clone()));
                        current.clear();
                    }

                    i += 2;
                    let mut italic_text = String::new();
                    while i + 1 < chars.len() {
                        if chars[i] == '/' && chars[i + 1] == '/' {
                            i += 2;
                            break;
                        }
                        italic_text.push(chars[i]);
                        i += 1;
                    }
                    let inner = Self::parse_inline(&italic_text);
                    nodes.push(Inline::Italic(inner));
                    continue;
                }
            }

            // Link [[url|text]] or [[url]]
            if i + 1 < chars.len() && chars[i] == '[' && chars[i + 1] == '[' {
                if !current.is_empty() {
                    nodes.push(Inline::Text(current.clone()));
                    current.clear();
                }

                i += 2;
                let mut link_content = String::new();
                while i + 1 < chars.len() {
                    if chars[i] == ']' && chars[i + 1] == ']' {
                        i += 2;
                        break;
                    }
                    link_content.push(chars[i]);
                    i += 1;
                }

                let (url, link_text) = if let Some(pipe_pos) = link_content.find('|') {
                    (
                        &link_content[..pipe_pos],
                        link_content[pipe_pos + 1..].to_string(),
                    )
                } else {
                    (link_content.as_str(), link_content.clone())
                };

                let text_node = Inline::Text(link_text);
                nodes.push(Inline::Link {
                    url: url.to_string(),
                    children: vec![text_node],
                });
                continue;
            }

            // Image {{url|alt}} or {{url}}
            if i + 1 < chars.len() && chars[i] == '{' && chars[i + 1] == '{' {
                // Not inline nowiki (checked above)
                if i + 2 < chars.len() && chars[i + 2] != '{' {
                    if !current.is_empty() {
                        nodes.push(Inline::Text(current.clone()));
                        current.clear();
                    }

                    i += 2;
                    let mut img_content = String::new();
                    while i + 1 < chars.len() {
                        if chars[i] == '}' && chars[i + 1] == '}' {
                            i += 2;
                            break;
                        }
                        img_content.push(chars[i]);
                        i += 1;
                    }

                    let (url, alt) = if let Some(pipe_pos) = img_content.find('|') {
                        (
                            &img_content[..pipe_pos],
                            Some(img_content[pipe_pos + 1..].to_string()),
                        )
                    } else {
                        (img_content.as_str(), None)
                    };

                    nodes.push(Inline::Image {
                        url: url.to_string(),
                        alt,
                    });
                    continue;
                }
            }

            current.push(chars[i]);
            i += 1;
        }

        if !current.is_empty() {
            nodes.push(Inline::Text(current));
        }

        nodes
    }
}

// ── Builder ───────────────────────────────────────────────────────────────────

/// Build a Creole string from a [`CreoleDoc`].
pub fn build(doc: &CreoleDoc) -> String {
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
            let level = (*level as usize).min(6);
            for _ in 0..level {
                ctx.write("=");
            }
            ctx.write(" ");
            build_inlines(inlines, ctx);
            ctx.write(" ");
            for _ in 0..level {
                ctx.write("=");
            }
            ctx.write("\n\n");
        }

        Block::CodeBlock { content } => {
            ctx.write("{{{\n");
            ctx.write(content);
            if !content.ends_with('\n') {
                ctx.write("\n");
            }
            ctx.write("}}}\n\n");
        }

        Block::Blockquote { children } => {
            for child in children {
                if matches!(child, Block::Paragraph { .. }) {
                    ctx.write("> ");
                    if let Block::Paragraph { inlines } = child {
                        build_inlines(inlines, ctx);
                    }
                    ctx.write("\n");
                } else {
                    build_block(child, ctx);
                }
            }
            ctx.write("\n");
        }

        Block::List { ordered, items } => {
            let marker = if *ordered { "#" } else { "*" };
            ctx.list_depth += 1;

            for item_blocks in items {
                for _ in 0..ctx.list_depth {
                    ctx.write(marker);
                }
                ctx.write(" ");

                for (i, item_child) in item_blocks.iter().enumerate() {
                    if i > 0 {
                        ctx.write("\n");
                    }
                    match item_child {
                        Block::Paragraph { inlines } => {
                            build_inlines(inlines, ctx);
                        }
                        Block::List { .. } => {
                            build_block(item_child, ctx);
                        }
                        _ => {
                            build_block(item_child, ctx);
                        }
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
                    if cell.is_header {
                        ctx.write("|=");
                    } else {
                        ctx.write("|");
                    }
                    build_inlines(&cell.inlines, ctx);
                }
                ctx.write("|\n");
            }
            ctx.write("\n");
        }

        Block::HorizontalRule => {
            ctx.write("----\n\n");
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

        Inline::Code(s) => {
            ctx.write("{{{");
            ctx.write(s);
            ctx.write("}}}");
        }

        Inline::Link { url, children } => {
            ctx.write("[[");
            ctx.write(url);
            if !children.is_empty() {
                ctx.write("|");
                build_inlines(children, ctx);
            }
            ctx.write("]]");
        }

        Inline::Image { url, alt } => {
            ctx.write("{{");
            ctx.write(url);
            if let Some(alt_text) = alt {
                ctx.write("|");
                ctx.write(alt_text);
            }
            ctx.write("}}");
        }

        Inline::LineBreak => {
            ctx.write("\\\\");
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_heading() {
        let doc = parse("= Title\n").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Heading { .. }));
        if let Block::Heading { level, .. } = &doc.blocks[0] {
            assert_eq!(*level, 1);
        }
    }

    #[test]
    fn test_parse_heading_levels() {
        let doc = parse("== Level 2\n=== Level 3\n").unwrap();
        assert_eq!(doc.blocks.len(), 2);
        if let Block::Heading { level, .. } = &doc.blocks[0] {
            assert_eq!(*level, 2);
        }
        if let Block::Heading { level, .. } = &doc.blocks[1] {
            assert_eq!(*level, 3);
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
        assert_eq!(inlines.len(), 1);
        assert!(matches!(inlines[0], Inline::Bold(_)));
    }

    #[test]
    fn test_parse_italic() {
        let doc = parse("//italic//\n").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert_eq!(inlines.len(), 1);
        assert!(matches!(inlines[0], Inline::Italic(_)));
    }

    #[test]
    fn test_parse_link() {
        let doc = parse("[[https://example.com|Example]]\n").unwrap();
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
        let Block::List { items, .. } = &doc.blocks[0] else {
            panic!("expected list");
        };
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_parse_nowiki() {
        let doc = parse("{{{code}}}\n").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::CodeBlock { .. }));
    }

    #[test]
    fn test_build_heading() {
        let doc = CreoleDoc {
            blocks: vec![Block::Heading {
                level: 1,
                inlines: vec![Inline::Text("Title".into())],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("= Title ="));
    }

    #[test]
    fn test_build_paragraph() {
        let doc = CreoleDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Hello, world!".into())],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("Hello, world!"));
    }

    #[test]
    fn test_build_bold() {
        let doc = CreoleDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Bold(vec![Inline::Text("bold".into())])],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("**bold**"));
    }

    #[test]
    fn test_build_italic() {
        let doc = CreoleDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Italic(vec![Inline::Text("italic".into())])],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("//italic//"));
    }

    #[test]
    fn test_build_code() {
        let doc = CreoleDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Code("code".into())],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("{{{code}}}"));
    }

    #[test]
    fn test_build_link() {
        let doc = CreoleDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "https://example.com".into(),
                    children: vec![Inline::Text("click".into())],
                }],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("[[https://example.com|click]]"));
    }

    #[test]
    fn test_build_list() {
        let doc = CreoleDoc {
            blocks: vec![Block::List {
                ordered: false,
                items: vec![
                    vec![Block::Paragraph {
                        inlines: vec![Inline::Text("one".into())],
                    }],
                    vec![Block::Paragraph {
                        inlines: vec![Inline::Text("two".into())],
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
        let doc = CreoleDoc {
            blocks: vec![Block::List {
                ordered: true,
                items: vec![
                    vec![Block::Paragraph {
                        inlines: vec![Inline::Text("first".into())],
                    }],
                    vec![Block::Paragraph {
                        inlines: vec![Inline::Text("second".into())],
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
        let doc = CreoleDoc {
            blocks: vec![Block::CodeBlock {
                content: "print('hi')".into(),
            }],
        };
        let output = build(&doc);
        assert!(output.contains("{{{\n"));
        assert!(output.contains("print('hi')"));
        assert!(output.contains("}}}\n"));
    }
}
