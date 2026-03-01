//! Jira wiki markup parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-jira` and `rescribe-write-jira` as thin adapter layers.

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct JiraError(pub String);

impl std::fmt::Display for JiraError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Jira error: {}", self.0)
    }
}

impl std::error::Error for JiraError {}

// ── AST ───────────────────────────────────────────────────────────────────────

/// A parsed Jira document.
#[derive(Debug, Clone, Default)]
pub struct JiraDoc {
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
    Panel {
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
    Underline(Vec<Inline>),
    Strikethrough(Vec<Inline>),
    Code(String),
    Link { url: String, children: Vec<Inline> },
    Image { url: String, alt: Option<String> },
    Superscript(Vec<Inline>),
    Subscript(Vec<Inline>),
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Parse a Jira string into a [`JiraDoc`].
pub fn parse(input: &str) -> Result<JiraDoc, JiraError> {
    let mut p = Parser::new(input);
    let blocks = p.parse()?;
    Ok(JiraDoc { blocks })
}

struct Parser<'a> {
    lines: Vec<&'a str>,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        let lines: Vec<&str> = input.lines().collect();
        Self { lines, pos: 0 }
    }

    fn parse(&mut self) -> Result<Vec<Block>, JiraError> {
        let mut blocks = Vec::new();

        while self.pos < self.lines.len() {
            if let Some(block) = self.parse_block()? {
                blocks.push(block);
            }
        }

        Ok(blocks)
    }

    fn current_line(&self) -> Option<&'a str> {
        self.lines.get(self.pos).copied()
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn parse_block(&mut self) -> Result<Option<Block>, JiraError> {
        let line = match self.current_line() {
            Some(l) => l,
            None => return Ok(None),
        };

        // Skip blank lines
        if line.trim().is_empty() {
            self.advance();
            return Ok(None);
        }

        // Heading: h1. to h6.
        if let Some(rest) = line.strip_prefix("h1. ") {
            self.advance();
            return Ok(Some(Block::Heading {
                level: 1,
                inlines: self.parse_inline(rest),
            }));
        }
        if let Some(rest) = line.strip_prefix("h2. ") {
            self.advance();
            return Ok(Some(Block::Heading {
                level: 2,
                inlines: self.parse_inline(rest),
            }));
        }
        if let Some(rest) = line.strip_prefix("h3. ") {
            self.advance();
            return Ok(Some(Block::Heading {
                level: 3,
                inlines: self.parse_inline(rest),
            }));
        }
        if let Some(rest) = line.strip_prefix("h4. ") {
            self.advance();
            return Ok(Some(Block::Heading {
                level: 4,
                inlines: self.parse_inline(rest),
            }));
        }
        if let Some(rest) = line.strip_prefix("h5. ") {
            self.advance();
            return Ok(Some(Block::Heading {
                level: 5,
                inlines: self.parse_inline(rest),
            }));
        }
        if let Some(rest) = line.strip_prefix("h6. ") {
            self.advance();
            return Ok(Some(Block::Heading {
                level: 6,
                inlines: self.parse_inline(rest),
            }));
        }

        // Code block: {code} or {code:lang}
        if line.starts_with("{code") {
            return Ok(Some(self.parse_code_block()?));
        }

        // Quote block: {quote}
        if line.trim() == "{quote}" {
            return Ok(Some(self.parse_quote_block()?));
        }

        // Panel: {panel}
        if line.starts_with("{panel") {
            return Ok(Some(self.parse_panel_block()?));
        }

        // Lists: * or #
        if line.starts_with('*') || line.starts_with('#') {
            return Ok(Some(self.parse_list()?));
        }

        // Table: starts with |
        if line.starts_with('|') {
            return Ok(Some(self.parse_table()?));
        }

        // Horizontal rule: ----
        if line.trim() == "----" {
            self.advance();
            return Ok(Some(Block::HorizontalRule));
        }

        // Default: paragraph
        Ok(Some(self.parse_paragraph()?))
    }

    fn parse_code_block(&mut self) -> Result<Block, JiraError> {
        let line = self.current_line().unwrap();
        self.advance();

        // Extract language from {code:lang}
        let language = if let Some(rest) = line.strip_prefix("{code:") {
            rest.strip_suffix('}').map(|s| s.to_string())
        } else {
            None
        };

        let mut content = String::new();
        while let Some(line) = self.current_line() {
            if line.trim() == "{code}" {
                self.advance();
                break;
            }
            if !content.is_empty() {
                content.push('\n');
            }
            content.push_str(line);
            self.advance();
        }

        Ok(Block::CodeBlock { content, language })
    }

    fn parse_quote_block(&mut self) -> Result<Block, JiraError> {
        self.advance(); // Skip {quote}
        let mut children = Vec::new();

        while let Some(line) = self.current_line() {
            if line.trim() == "{quote}" {
                self.advance();
                break;
            }
            if line.trim().is_empty() {
                self.advance();
                continue;
            }
            children.push(Block::Paragraph {
                inlines: self.parse_inline(line),
            });
            self.advance();
        }

        Ok(Block::Blockquote { children })
    }

    fn parse_panel_block(&mut self) -> Result<Block, JiraError> {
        self.advance(); // Skip {panel...}
        let mut children = Vec::new();

        while let Some(line) = self.current_line() {
            if line.trim() == "{panel}" {
                self.advance();
                break;
            }
            if line.trim().is_empty() {
                self.advance();
                continue;
            }
            children.push(Block::Paragraph {
                inlines: self.parse_inline(line),
            });
            self.advance();
        }

        Ok(Block::Panel { children })
    }

    fn parse_list(&mut self) -> Result<Block, JiraError> {
        let first_line = self.current_line().unwrap();
        let ordered = first_line.starts_with('#');
        let mut items = Vec::new();

        while let Some(line) = self.current_line() {
            let marker = if ordered { '#' } else { '*' };
            if !line.starts_with(marker) {
                break;
            }

            // Count depth
            let depth = line.chars().take_while(|&c| c == marker).count();
            let content = line[depth..].trim_start();

            // For now, just handle single-level lists
            if depth == 1 {
                let item = Block::Paragraph {
                    inlines: self.parse_inline(content),
                };
                items.push(vec![item]);
            }
            self.advance();
        }

        Ok(Block::List { ordered, items })
    }

    fn parse_table(&mut self) -> Result<Block, JiraError> {
        let mut rows = Vec::new();

        while let Some(line) = self.current_line() {
            if !line.starts_with('|') {
                break;
            }

            // Check if header row (starts with ||)
            let is_header = line.starts_with("||");
            let cells: Vec<TableCell> = if is_header {
                line.split("||")
                    .filter(|s| !s.is_empty())
                    .map(|cell| TableCell {
                        is_header: true,
                        inlines: self.parse_inline(cell.trim()),
                    })
                    .collect()
            } else {
                line.split('|')
                    .filter(|s| !s.is_empty())
                    .map(|cell| TableCell {
                        is_header: false,
                        inlines: self.parse_inline(cell.trim()),
                    })
                    .collect()
            };

            rows.push(TableRow { cells });
            self.advance();
        }

        Ok(Block::Table { rows })
    }

    fn parse_paragraph(&mut self) -> Result<Block, JiraError> {
        let mut lines = Vec::new();

        while let Some(line) = self.current_line() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                break;
            }
            // Stop at block-level elements
            if trimmed.starts_with("h1. ")
                || trimmed.starts_with("h2. ")
                || trimmed.starts_with("h3. ")
                || trimmed.starts_with("h4. ")
                || trimmed.starts_with("h5. ")
                || trimmed.starts_with("h6. ")
                || trimmed.starts_with("{code")
                || trimmed == "{quote}"
                || trimmed.starts_with("{panel")
                || trimmed.starts_with('*')
                || trimmed.starts_with('#')
                || trimmed.starts_with('|')
                || trimmed == "----"
            {
                break;
            }
            lines.push(trimmed);
            self.advance();
        }

        let text = lines.join(" ");
        Ok(Block::Paragraph {
            inlines: self.parse_inline(&text),
        })
    }

    fn parse_inline(&self, text: &str) -> Vec<Inline> {
        let mut inlines = Vec::new();
        let mut current = String::new();
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // Bold: *text*
            if chars[i] == '*' {
                if !current.is_empty() {
                    inlines.push(Inline::Text(current.clone()));
                    current.clear();
                }
                if let Some((content, end)) = self.find_delim(&chars, i + 1, '*') {
                    inlines.push(Inline::Bold(vec![Inline::Text(content)]));
                    i = end + 1;
                    continue;
                }
            }

            // Italic: _text_
            if chars[i] == '_' {
                if !current.is_empty() {
                    inlines.push(Inline::Text(current.clone()));
                    current.clear();
                }
                if let Some((content, end)) = self.find_delim(&chars, i + 1, '_') {
                    inlines.push(Inline::Italic(vec![Inline::Text(content)]));
                    i = end + 1;
                    continue;
                }
            }

            // Strikethrough: -text-
            if chars[i] == '-' {
                if !current.is_empty() {
                    inlines.push(Inline::Text(current.clone()));
                    current.clear();
                }
                if let Some((content, end)) = self.find_delim(&chars, i + 1, '-') {
                    inlines.push(Inline::Strikethrough(vec![Inline::Text(content)]));
                    i = end + 1;
                    continue;
                }
            }

            // Underline: +text+
            if chars[i] == '+' {
                if !current.is_empty() {
                    inlines.push(Inline::Text(current.clone()));
                    current.clear();
                }
                if let Some((content, end)) = self.find_delim(&chars, i + 1, '+') {
                    inlines.push(Inline::Underline(vec![Inline::Text(content)]));
                    i = end + 1;
                    continue;
                }
            }

            // Superscript: ^text^
            if chars[i] == '^' {
                if !current.is_empty() {
                    inlines.push(Inline::Text(current.clone()));
                    current.clear();
                }
                if let Some((content, end)) = self.find_delim(&chars, i + 1, '^') {
                    inlines.push(Inline::Superscript(vec![Inline::Text(content)]));
                    i = end + 1;
                    continue;
                }
            }

            // Subscript: ~text~
            if chars[i] == '~' {
                if !current.is_empty() {
                    inlines.push(Inline::Text(current.clone()));
                    current.clear();
                }
                if let Some((content, end)) = self.find_delim(&chars, i + 1, '~') {
                    inlines.push(Inline::Subscript(vec![Inline::Text(content)]));
                    i = end + 1;
                    continue;
                }
            }

            // Monospace: {{text}}
            if i + 1 < chars.len() && chars[i] == '{' && chars[i + 1] == '{' {
                if !current.is_empty() {
                    inlines.push(Inline::Text(current.clone()));
                    current.clear();
                }
                if let Some((content, end)) = self.find_double_brace(&chars, i + 2) {
                    inlines.push(Inline::Code(content));
                    i = end + 2;
                    continue;
                }
            }

            // Link: [text|url] or [url]
            if chars[i] == '[' {
                if !current.is_empty() {
                    inlines.push(Inline::Text(current.clone()));
                    current.clear();
                }
                if let Some((link_content, end)) = self.find_bracket(&chars, i + 1) {
                    let (text, url) = if let Some(pipe_pos) = link_content.find('|') {
                        (&link_content[..pipe_pos], &link_content[pipe_pos + 1..])
                    } else {
                        (link_content.as_str(), link_content.as_str())
                    };
                    inlines.push(Inline::Link {
                        url: url.to_string(),
                        children: vec![Inline::Text(text.to_string())],
                    });
                    i = end + 1;
                    continue;
                }
            }

            // Image: !url! or !url|alt!
            if chars[i] == '!' {
                if !current.is_empty() {
                    inlines.push(Inline::Text(current.clone()));
                    current.clear();
                }
                if let Some((img_content, end)) = self.find_delim(&chars, i + 1, '!') {
                    let (url, alt) = if let Some(pipe_pos) = img_content.find('|') {
                        (
                            &img_content[..pipe_pos],
                            Some(img_content[pipe_pos + 1..].to_string()),
                        )
                    } else {
                        (img_content.as_str(), None)
                    };
                    inlines.push(Inline::Image {
                        url: url.to_string(),
                        alt,
                    });
                    i = end + 1;
                    continue;
                }
            }

            current.push(chars[i]);
            i += 1;
        }

        if !current.is_empty() {
            inlines.push(Inline::Text(current));
        }

        inlines
    }

    fn find_delim(&self, chars: &[char], start: usize, delim: char) -> Option<(String, usize)> {
        let mut content = String::new();
        let mut i = start;

        while i < chars.len() {
            if chars[i] == delim {
                return Some((content, i));
            }
            content.push(chars[i]);
            i += 1;
        }

        None
    }

    fn find_double_brace(&self, chars: &[char], start: usize) -> Option<(String, usize)> {
        let mut content = String::new();
        let mut i = start;

        while i + 1 < chars.len() {
            if chars[i] == '}' && chars[i + 1] == '}' {
                return Some((content, i));
            }
            content.push(chars[i]);
            i += 1;
        }

        None
    }

    fn find_bracket(&self, chars: &[char], start: usize) -> Option<(String, usize)> {
        let mut content = String::new();
        let mut i = start;

        while i < chars.len() {
            if chars[i] == ']' {
                return Some((content, i));
            }
            content.push(chars[i]);
            i += 1;
        }

        None
    }
}

// ── Builder ───────────────────────────────────────────────────────────────────

/// Build a Jira string from a [`JiraDoc`].
pub fn build(doc: &JiraDoc) -> String {
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

        Block::CodeBlock { content, language } => {
            if let Some(lang) = language {
                ctx.write(&format!("{{code:{}}}\n", lang));
            } else {
                ctx.write("{code}\n");
            }
            ctx.write(content);
            if !content.ends_with('\n') {
                ctx.write("\n");
            }
            ctx.write("{code}\n\n");
        }

        Block::Blockquote { children } => {
            ctx.write("{quote}\n");
            for child in children {
                build_block(child, ctx);
            }
            ctx.write("{quote}\n\n");
        }

        Block::Panel { children } => {
            ctx.write("{panel}\n");
            for child in children {
                build_block(child, ctx);
            }
            ctx.write("{panel}\n\n");
        }

        Block::List { ordered, items } => {
            ctx.list_depth += 1;
            for item_blocks in items {
                let marker = if *ordered { "#" } else { "*" };
                for _ in 0..ctx.list_depth {
                    ctx.write(marker);
                }
                ctx.write(" ");
                for block in item_blocks {
                    match block {
                        Block::Paragraph { inlines } => build_inlines(inlines, ctx),
                        _ => build_block(block, ctx),
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
                        ctx.write("||");
                    } else {
                        ctx.write("|");
                    }
                    build_inlines(&cell.inlines, ctx);
                }
                if rows
                    .first()
                    .and_then(|r| r.cells.first().map(|c| c.is_header))
                    == Some(true)
                {
                    ctx.write("||\n");
                } else {
                    ctx.write("|\n");
                }
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
            ctx.write("*");
            build_inlines(children, ctx);
            ctx.write("*");
        }

        Inline::Italic(children) => {
            ctx.write("_");
            build_inlines(children, ctx);
            ctx.write("_");
        }

        Inline::Underline(children) => {
            ctx.write("+");
            build_inlines(children, ctx);
            ctx.write("+");
        }

        Inline::Strikethrough(children) => {
            ctx.write("-");
            build_inlines(children, ctx);
            ctx.write("-");
        }

        Inline::Code(s) => {
            ctx.write("{{");
            ctx.write(s);
            ctx.write("}}");
        }

        Inline::Link { url, children } => {
            ctx.write("[");
            build_inlines(children, ctx);
            ctx.write("|");
            ctx.write(url);
            ctx.write("]");
        }

        Inline::Image { url, alt } => {
            ctx.write("!");
            ctx.write(url);
            if let Some(alt) = alt {
                ctx.write("|");
                ctx.write(alt);
            }
            ctx.write("!");
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
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_text() {
        let doc = parse("Hello world").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Paragraph { .. }));
    }

    #[test]
    fn test_parse_heading() {
        let doc = parse("h1. Title").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Heading { level: 1, .. }));
    }

    #[test]
    fn test_parse_bold() {
        let doc = parse("This is *bold* text.").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Bold(_))));
    }

    #[test]
    fn test_parse_italic() {
        let doc = parse("This is _italic_ text.").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Italic(_))));
    }

    #[test]
    fn test_parse_code() {
        let doc = parse("Use {{code}} here.").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Code(_))));
    }

    #[test]
    fn test_parse_link() {
        let doc = parse("Click [here|https://example.com].").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Link { .. })));
    }

    #[test]
    fn test_parse_list() {
        let doc = parse("* Item 1\n* Item 2").unwrap();
        let block = &doc.blocks[0];
        assert!(matches!(block, Block::List { ordered: false, .. }));
    }

    #[test]
    fn test_parse_code_block() {
        let doc = parse("{code:java}\npublic class Test {}\n{code}").unwrap();
        let code = &doc.blocks[0];
        assert!(matches!(code, Block::CodeBlock { .. }));
        if let Block::CodeBlock { language, .. } = code {
            assert_eq!(language.as_deref(), Some("java"));
        }
    }

    #[test]
    fn test_build_paragraph() {
        let doc = JiraDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Hello".into())],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("Hello"));
    }

    #[test]
    fn test_build_bold() {
        let doc = JiraDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Bold(vec![Inline::Text("bold".into())])],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("*bold*"));
    }

    #[test]
    fn test_build_heading() {
        let doc = JiraDoc {
            blocks: vec![Block::Heading {
                level: 1,
                inlines: vec![Inline::Text("Title".into())],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("h1. Title"));
    }

    #[test]
    fn test_build_code_block() {
        let doc = JiraDoc {
            blocks: vec![Block::CodeBlock {
                content: "print('hi')".into(),
                language: Some("python".into()),
            }],
        };
        let output = build(&doc);
        assert!(output.contains("{code:python}"));
        assert!(output.contains("print('hi')"));
        assert!(output.contains("{code}"));
    }

    #[test]
    fn test_roundtrip_heading() {
        let original = "h1. Title";
        let doc = parse(original).unwrap();
        let rebuilt = build(&doc);
        assert!(rebuilt.contains("h1. Title"));
    }

    #[test]
    fn test_roundtrip_bold() {
        let original = "This is *bold* text.";
        let doc = parse(original).unwrap();
        let rebuilt = build(&doc);
        assert!(rebuilt.contains("*bold*"));
    }
}
