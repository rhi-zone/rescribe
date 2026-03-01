//! MediaWiki markup parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-mediawiki` and `rescribe-write-mediawiki` as thin adapter layers.

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct MediawikiError(pub String);

impl std::fmt::Display for MediawikiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MediaWiki error: {}", self.0)
    }
}

impl std::error::Error for MediawikiError {}

// ── AST ───────────────────────────────────────────────────────────────────────

/// A parsed MediaWiki document.
#[derive(Debug, Clone, Default)]
pub struct MediawikiDoc {
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
    List {
        ordered: bool,
        items: Vec<Vec<Block>>,
    },
    HorizontalRule,
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
    Code(String),
    Link { url: String, text: String },
    Image { url: String, alt: String },
    LineBreak,
    Strikeout(Vec<Inline>),
    Underline(Vec<Inline>),
    Subscript(Vec<Inline>),
    Superscript(Vec<Inline>),
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Parse a MediaWiki string into a [`MediawikiDoc`].
pub fn parse(input: &str) -> Result<MediawikiDoc, MediawikiError> {
    let mut p = Parser::new(input);
    let blocks = p.parse()?;
    Ok(MediawikiDoc { blocks })
}

struct Parser<'a> {
    input: &'a str,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input }
    }

    fn parse(&mut self) -> Result<Vec<Block>, MediawikiError> {
        let lines: Vec<&str> = self.input.lines().collect();
        let mut i = 0;
        let mut blocks = Vec::new();

        while i < lines.len() {
            let line = lines[i];
            let trimmed = line.trim();

            if trimmed.is_empty() {
                i += 1;
                continue;
            }

            // Heading
            if trimmed.starts_with('=')
                && let Some(heading) = self.parse_heading(trimmed)
            {
                blocks.push(heading);
                i += 1;
                continue;
            }

            // List
            if trimmed.starts_with('*') || trimmed.starts_with('#') {
                let (list, consumed) = self.parse_list(&lines[i..]);
                blocks.push(list);
                i += consumed;
                continue;
            }

            // Horizontal rule
            if trimmed == "----" || (trimmed.chars().all(|c| c == '-') && trimmed.len() >= 4) {
                blocks.push(Block::HorizontalRule);
                i += 1;
                continue;
            }

            // Code block (indented with space)
            if line.starts_with(' ') {
                let (block, consumed) = self.parse_code_block(&lines[i..]);
                blocks.push(block);
                i += consumed;
                continue;
            }

            // Table
            if trimmed.starts_with("{|") {
                let (table, consumed) = self.parse_table(&lines[i..]);
                blocks.push(table);
                i += consumed;
                continue;
            }

            // Regular paragraph
            let (para, consumed) = self.parse_paragraph(&lines[i..]);
            blocks.push(para);
            i += consumed;
        }

        Ok(blocks)
    }

    fn parse_heading(&self, line: &str) -> Option<Block> {
        let trimmed = line.trim();

        // Count leading `=`
        let level = trimmed.chars().take_while(|&c| c == '=').count();
        if level == 0 || level > 6 {
            return None;
        }

        // Check for matching trailing `=`
        let content = trimmed.trim_start_matches('=').trim_end_matches('=').trim();

        let inlines = self.parse_inline(content);
        Some(Block::Heading {
            level: level as u8,
            inlines,
        })
    }

    fn parse_list(&self, lines: &[&str]) -> (Block, usize) {
        let mut items: Vec<Vec<Block>> = Vec::new();
        let mut consumed = 0;
        let first_char = lines[0].trim().chars().next().unwrap_or('*');
        let ordered = first_char == '#';

        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                break;
            }

            // Check if this is a list item with the same marker
            let marker = if ordered { '#' } else { '*' };
            if !trimmed.starts_with(marker) {
                break;
            }

            // For simplicity, flatten nested items
            let content = trimmed.trim_start_matches(marker).trim();
            let inlines = self.parse_inline(content);
            items.push(vec![Block::Paragraph { inlines }]);

            consumed += 1;
        }

        (Block::List { ordered, items }, consumed.max(1))
    }

    fn parse_code_block(&self, lines: &[&str]) -> (Block, usize) {
        let mut content = String::new();
        let mut consumed = 0;

        for line in lines {
            if !line.starts_with(' ') && !line.is_empty() {
                break;
            }
            if !content.is_empty() {
                content.push('\n');
            }
            // Remove one leading space
            content.push_str(line.strip_prefix(' ').unwrap_or(line));
            consumed += 1;
        }

        (Block::CodeBlock { content }, consumed.max(1))
    }

    fn parse_table(&self, lines: &[&str]) -> (Block, usize) {
        let mut rows = Vec::new();
        let mut consumed = 0;

        for line in lines {
            let trimmed = line.trim();

            if trimmed == "|}" {
                consumed += 1;
                break;
            }

            if trimmed.starts_with("|-") {
                // Table row marker
                consumed += 1;
                continue;
            }

            if trimmed.starts_with('|') || trimmed.starts_with('!') {
                // Parse cells in this line
                let is_header = trimmed.starts_with('!');
                let content = trimmed.trim_start_matches(['|', '!']);
                let cells_str: Vec<&str> = content.split("||").collect();
                let mut cells = Vec::new();

                for cell_content in cells_str {
                    let inlines = self.parse_inline(cell_content.trim());
                    cells.push(TableCell { is_header, inlines });
                }

                if !cells.is_empty() {
                    rows.push(TableRow { cells });
                }
            }

            consumed += 1;
        }

        (Block::Table { rows }, consumed.max(1))
    }

    fn parse_paragraph(&self, lines: &[&str]) -> (Block, usize) {
        let mut text = String::new();
        let mut consumed = 0;

        for line in lines {
            let trimmed = line.trim();

            // Stop at empty lines, headings, lists, rules, tables
            if trimmed.is_empty()
                || trimmed.starts_with('=')
                || trimmed.starts_with('*')
                || trimmed.starts_with('#')
                || trimmed == "----"
                || (trimmed.chars().all(|c| c == '-') && trimmed.len() >= 4)
                || trimmed.starts_with("{|")
                || trimmed == "|}"
                || trimmed.starts_with("|-")
                || trimmed.starts_with('|')
                || trimmed.starts_with('!')
            {
                break;
            }

            if !text.is_empty() {
                text.push(' ');
            }
            text.push_str(trimmed);
            consumed += 1;
        }

        let inlines = self.parse_inline(&text);
        (Block::Paragraph { inlines }, consumed.max(1))
    }

    #[allow(clippy::only_used_in_recursion)]
    fn parse_inline(&self, text: &str) -> Vec<Inline> {
        let mut inlines = Vec::new();
        let mut current_text = String::new();
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // Bold: '''text'''
            if i + 2 < chars.len()
                && chars[i] == '\''
                && chars[i + 1] == '\''
                && chars[i + 2] == '\''
            {
                if !current_text.is_empty() {
                    inlines.push(Inline::Text(current_text.clone()));
                    current_text.clear();
                }

                // Find closing '''
                let start = i + 3;
                let mut end = start;
                while end + 2 < chars.len() {
                    if chars[end] == '\'' && chars[end + 1] == '\'' && chars[end + 2] == '\'' {
                        break;
                    }
                    end += 1;
                }

                if end + 2 < chars.len() {
                    let inner: String = chars[start..end].iter().collect();
                    let inner_inlines = self.parse_inline(&inner);
                    inlines.push(Inline::Bold(inner_inlines));
                    i = end + 3;
                    continue;
                }
            }

            // Italic: ''text''
            if i + 1 < chars.len() && chars[i] == '\'' && chars[i + 1] == '\'' {
                // Make sure it's not bold
                if i + 2 < chars.len() && chars[i + 2] == '\'' {
                    // This is bold, handled above
                } else {
                    if !current_text.is_empty() {
                        inlines.push(Inline::Text(current_text.clone()));
                        current_text.clear();
                    }

                    // Find closing ''
                    let start = i + 2;
                    let mut end = start;
                    while end + 1 < chars.len() {
                        if chars[end] == '\'' && chars[end + 1] == '\'' {
                            // Make sure it's not '''
                            if end + 2 < chars.len() && chars[end + 2] == '\'' {
                                end += 1;
                                continue;
                            }
                            break;
                        }
                        end += 1;
                    }

                    if end + 1 < chars.len() {
                        let inner: String = chars[start..end].iter().collect();
                        let inner_inlines = self.parse_inline(&inner);
                        inlines.push(Inline::Italic(inner_inlines));
                        i = end + 2;
                        continue;
                    }
                }
            }

            // Internal link: [[Title]] or [[Title|text]]
            if i + 1 < chars.len() && chars[i] == '[' && chars[i + 1] == '[' {
                if !current_text.is_empty() {
                    inlines.push(Inline::Text(current_text.clone()));
                    current_text.clear();
                }

                // Find closing ]]
                let start = i + 2;
                let mut end = start;
                while end + 1 < chars.len() {
                    if chars[end] == ']' && chars[end + 1] == ']' {
                        break;
                    }
                    end += 1;
                }

                if end + 1 < chars.len() {
                    let inner: String = chars[start..end].iter().collect();
                    let (url, text) = if let Some(pipe_pos) = inner.find('|') {
                        let url = &inner[..pipe_pos];
                        let text = &inner[pipe_pos + 1..];
                        (url.to_string(), text.to_string())
                    } else {
                        (inner.clone(), inner)
                    };

                    inlines.push(Inline::Link { url, text });
                    i = end + 2;
                    continue;
                }
            }

            // External link: [url text]
            if chars[i] == '[' && (i + 1 >= chars.len() || chars[i + 1] != '[') {
                if !current_text.is_empty() {
                    inlines.push(Inline::Text(current_text.clone()));
                    current_text.clear();
                }

                // Find closing ]
                let start = i + 1;
                let mut end = start;
                while end < chars.len() && chars[end] != ']' {
                    end += 1;
                }

                if end < chars.len() {
                    let inner: String = chars[start..end].iter().collect();
                    let parts: Vec<&str> = inner.splitn(2, ' ').collect();
                    let url = parts[0].to_string();
                    let text = if parts.len() > 1 {
                        parts[1].to_string()
                    } else {
                        url.clone()
                    };

                    inlines.push(Inline::Link { url, text });
                    i = end + 1;
                    continue;
                }
            }

            // <code>...</code>
            if i + 5 < chars.len() && &chars[i..i + 6].iter().collect::<String>() == "<code>" {
                if !current_text.is_empty() {
                    inlines.push(Inline::Text(current_text.clone()));
                    current_text.clear();
                }

                let start = i + 6;
                let mut end = start;
                while end + 6 < chars.len() {
                    if &chars[end..end + 7].iter().collect::<String>() == "</code>" {
                        break;
                    }
                    end += 1;
                }

                if end + 6 < chars.len() {
                    let code_text: String = chars[start..end].iter().collect();
                    inlines.push(Inline::Code(code_text));
                    i = end + 7;
                    continue;
                }
            }

            // Regular character
            current_text.push(chars[i]);
            i += 1;
        }

        if !current_text.is_empty() {
            inlines.push(Inline::Text(current_text));
        }

        inlines
    }
}

// ── Builder ───────────────────────────────────────────────────────────────────

/// Build a MediaWiki string from a [`MediawikiDoc`].
pub fn build(doc: &MediawikiDoc) -> String {
    let mut ctx = BuildContext::new();
    for block in &doc.blocks {
        build_block(block, &mut ctx);
    }
    ctx.output.trim_end().to_string() + "\n"
}

struct BuildContext {
    output: String,
    list_depth: usize,
    list_markers: Vec<char>,
}

impl BuildContext {
    fn new() -> Self {
        Self {
            output: String::new(),
            list_depth: 0,
            list_markers: Vec::new(),
        }
    }

    fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn writeln(&mut self, s: &str) {
        self.output.push_str(s);
        self.output.push('\n');
    }

    fn newline(&mut self) {
        self.output.push('\n');
    }
}

fn build_block(block: &Block, ctx: &mut BuildContext) {
    match block {
        Block::Paragraph { inlines } => {
            build_inlines(inlines, ctx);
            ctx.newline();
            ctx.newline();
        }

        Block::Heading { level, inlines } => {
            let markers = "=".repeat(*level as usize);
            ctx.write(&markers);
            ctx.write(" ");
            build_inlines(inlines, ctx);
            ctx.write(" ");
            ctx.writeln(&markers);
            ctx.newline();
        }

        Block::CodeBlock { content } => {
            for line in content.lines() {
                ctx.write(" ");
                ctx.writeln(line);
            }
            ctx.newline();
        }

        Block::List { ordered, items } => {
            let marker = if *ordered { '#' } else { '*' };
            ctx.list_markers.push(marker);
            ctx.list_depth += 1;

            for item_blocks in items {
                for block in item_blocks {
                    match block {
                        Block::Paragraph { inlines } => {
                            let markers: String = ctx.list_markers.iter().collect();
                            ctx.write(&markers);
                            ctx.write(" ");
                            build_inlines(inlines, ctx);
                            ctx.newline();
                        }
                        other => build_block(other, ctx),
                    }
                }
            }

            ctx.list_depth -= 1;
            ctx.list_markers.pop();

            if ctx.list_depth == 0 {
                ctx.newline();
            }
        }

        Block::HorizontalRule => {
            ctx.writeln("----");
            ctx.newline();
        }

        Block::Table { rows } => {
            ctx.writeln("{|");
            for (i, row) in rows.iter().enumerate() {
                if i > 0 {
                    ctx.writeln("|-");
                }
                for cell in &row.cells {
                    let marker = if cell.is_header { "!" } else { "|" };
                    ctx.write(marker);
                    ctx.write(" ");
                    build_inlines(&cell.inlines, ctx);
                    ctx.newline();
                }
            }
            ctx.writeln("|}");
            ctx.newline();
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
            ctx.write("'''");
            build_inlines(children, ctx);
            ctx.write("'''");
        }

        Inline::Italic(children) => {
            ctx.write("''");
            build_inlines(children, ctx);
            ctx.write("''");
        }

        Inline::Code(s) => {
            ctx.write("<code>");
            ctx.write(s);
            ctx.write("</code>");
        }

        Inline::Link { url, text } => {
            if url.starts_with("http://") || url.starts_with("https://") {
                // External link
                if text == url {
                    ctx.write(&format!("[{}]", url));
                } else {
                    ctx.write(&format!("[{} {}]", url, text));
                }
            } else {
                // Internal link
                if text == url {
                    ctx.write(&format!("[[{}]]", url));
                } else {
                    ctx.write(&format!("[[{}|{}]]", url, text));
                }
            }
        }

        Inline::Image { url, alt } => {
            if alt.is_empty() {
                ctx.write(&format!("[[File:{}]]", url));
            } else {
                ctx.write(&format!("[[File:{}|{}]]", url, alt));
            }
        }

        Inline::LineBreak => {
            ctx.write("<br/>");
        }

        Inline::Strikeout(children) => {
            ctx.write("<s>");
            build_inlines(children, ctx);
            ctx.write("</s>");
        }

        Inline::Underline(children) => {
            ctx.write("<u>");
            build_inlines(children, ctx);
            ctx.write("</u>");
        }

        Inline::Subscript(children) => {
            ctx.write("<sub>");
            build_inlines(children, ctx);
            ctx.write("</sub>");
        }

        Inline::Superscript(children) => {
            ctx.write("<sup>");
            build_inlines(children, ctx);
            ctx.write("</sup>");
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_text() {
        let doc = parse("Some simple text").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Paragraph { .. }));
    }

    #[test]
    fn test_parse_heading() {
        let doc = parse("== Heading ==").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        if let Block::Heading { level, .. } = &doc.blocks[0] {
            assert_eq!(*level, 2);
        } else {
            panic!("expected heading");
        }
    }

    #[test]
    fn test_parse_bold() {
        let doc = parse("'''bold'''").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Bold(_))));
    }

    #[test]
    fn test_parse_italic() {
        let doc = parse("''italic''").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Italic(_))));
    }

    #[test]
    fn test_parse_list() {
        let doc = parse("* Item 1\n* Item 2").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        let Block::List { ordered, items } = &doc.blocks[0] else {
            panic!("expected list");
        };
        assert!(!ordered);
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_parse_internal_link() {
        let doc = parse("[[Title|Link text]]").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        let has_link = inlines.iter().any(|i| {
            if let Inline::Link { url, .. } = i {
                url == "Title"
            } else {
                false
            }
        });
        assert!(has_link);
    }

    #[test]
    fn test_parse_external_link() {
        let doc = parse("[https://example.com Example]").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        let has_link = inlines.iter().any(|i| {
            if let Inline::Link { url, .. } = i {
                url == "https://example.com"
            } else {
                false
            }
        });
        assert!(has_link);
    }

    #[test]
    fn test_parse_horizontal_rule() {
        let doc = parse("----").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::HorizontalRule));
    }

    #[test]
    fn test_parse_code_block() {
        let doc = parse(" code line 1\n code line 2").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        if let Block::CodeBlock { content } = &doc.blocks[0] {
            assert!(content.contains("code line"));
        } else {
            panic!("expected code block");
        }
    }

    #[test]
    fn test_build_heading() {
        let doc = MediawikiDoc {
            blocks: vec![Block::Heading {
                level: 2,
                inlines: vec![Inline::Text("Title".into())],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("== Title =="));
    }

    #[test]
    fn test_build_bold() {
        let doc = MediawikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Bold(vec![Inline::Text("bold".into())])],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("'''bold'''"));
    }

    #[test]
    fn test_build_italic() {
        let doc = MediawikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Italic(vec![Inline::Text("italic".into())])],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("''italic''"));
    }

    #[test]
    fn test_build_list() {
        let doc = MediawikiDoc {
            blocks: vec![Block::List {
                ordered: false,
                items: vec![
                    vec![Block::Paragraph {
                        inlines: vec![Inline::Text("Item 1".into())],
                    }],
                    vec![Block::Paragraph {
                        inlines: vec![Inline::Text("Item 2".into())],
                    }],
                ],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("* Item 1"));
        assert!(out.contains("* Item 2"));
    }

    #[test]
    fn test_build_internal_link() {
        let doc = MediawikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "Title".to_string(),
                    text: "Link text".to_string(),
                }],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("[[Title|Link text]]"));
    }

    #[test]
    fn test_build_external_link() {
        let doc = MediawikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "https://example.com".to_string(),
                    text: "Example".to_string(),
                }],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("[https://example.com Example]"));
    }

    #[test]
    fn test_roundtrip_simple() {
        let input = "== Heading ==\n\nSome '''bold''' text.";
        let doc = parse(input).unwrap();
        let output = build(&doc);
        // Output should be parseable again
        let doc2 = parse(&output).unwrap();
        assert!(!doc2.blocks.is_empty());
    }
}
