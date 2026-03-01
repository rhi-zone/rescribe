//! DokuWiki parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-dokuwiki` and `rescribe-write-dokuwiki` as thin adapter layers.

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct DokuwikiError(pub String);

impl std::fmt::Display for DokuwikiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DokuWiki error: {}", self.0)
    }
}

impl std::error::Error for DokuwikiError {}

// ── AST ───────────────────────────────────────────────────────────────────────

/// A parsed DokuWiki document.
#[derive(Debug, Clone, Default)]
pub struct DokuwikiDoc {
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
        language: Option<String>,
        content: String,
    },
    Blockquote {
        children: Vec<Block>,
    },
    List {
        ordered: bool,
        items: Vec<Vec<Block>>,
    },
    HorizontalRule,
}

/// Inline element.
#[derive(Debug, Clone)]
pub enum Inline {
    Text(String),
    Bold(Vec<Inline>),
    Italic(Vec<Inline>),
    Underline(Vec<Inline>),
    Code(String),
    Link { url: String, children: Vec<Inline> },
    Image { url: String, alt: Option<String> },
    LineBreak,
    SoftBreak,
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Parse a DokuWiki string into a [`DokuwikiDoc`].
pub fn parse(input: &str) -> Result<DokuwikiDoc, DokuwikiError> {
    let mut p = Parser::new(input);
    let blocks = p.parse()?;
    Ok(DokuwikiDoc { blocks })
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

    fn parse(&mut self) -> Result<Vec<Block>, DokuwikiError> {
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

    fn parse_block(&mut self) -> Result<Option<Block>, DokuwikiError> {
        let line = match self.current_line() {
            Some(l) => l,
            None => return Ok(None),
        };

        // Skip blank lines
        if line.trim().is_empty() {
            self.advance();
            return Ok(None);
        }

        let trimmed = line.trim();

        // Heading: ====== H1 ====== (6 =), ===== H2 ===== (5 =), etc.
        if trimmed.starts_with('=') && trimmed.ends_with('=') {
            return Ok(Some(self.parse_heading()));
        }

        // Code block: <code> or <code lang>
        if trimmed.starts_with("<code") {
            return Ok(Some(self.parse_code_block()?));
        }

        // File block: <file>
        if trimmed.starts_with("<file") {
            return Ok(Some(self.parse_code_block()?));
        }

        // List: starts with spaces and * or -
        if line.starts_with("  ")
            && (line.trim_start().starts_with('*') || line.trim_start().starts_with('-'))
        {
            return Ok(Some(self.parse_list()?));
        }

        // Blockquote: > text
        if trimmed.starts_with('>') {
            return Ok(Some(self.parse_blockquote()?));
        }

        // Horizontal rule: ----
        if trimmed == "----" {
            self.advance();
            return Ok(Some(Block::HorizontalRule));
        }

        // Default: paragraph
        Ok(Some(self.parse_paragraph()?))
    }

    fn parse_heading(&mut self) -> Block {
        let line = self.current_line().unwrap();
        self.advance();

        let trimmed = line.trim();

        // Count leading = signs
        let leading = trimmed.chars().take_while(|c| *c == '=').count();
        // DokuWiki uses 6 = for H1, 5 for H2, etc.
        let level = (7 - leading.min(6)) as u8;

        // Extract content between = signs
        let content = trimmed.trim_start_matches('=').trim_end_matches('=').trim();

        Block::Heading {
            level,
            inlines: self.parse_inline(content),
        }
    }

    fn parse_code_block(&mut self) -> Result<Block, DokuwikiError> {
        let line = self.current_line().unwrap();
        self.advance();

        // Extract language from <code lang> or <file lang>
        let lang = if let Some(start) = line.find('<') {
            let after = &line[start..];
            if let Some(end) = after.find('>') {
                let tag = &after[1..end];
                let parts: Vec<&str> = tag.split_whitespace().collect();
                if parts.len() > 1 {
                    Some(parts[1].to_string())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let end_tag = if line.contains("<file") {
            "</file>"
        } else {
            "</code>"
        };

        let mut content = String::new();
        while let Some(line) = self.current_line() {
            if line.contains(end_tag) {
                self.advance();
                break;
            }
            if !content.is_empty() {
                content.push('\n');
            }
            content.push_str(line);
            self.advance();
        }

        Ok(Block::CodeBlock {
            language: lang,
            content,
        })
    }

    fn parse_list(&mut self) -> Result<Block, DokuwikiError> {
        let mut items = Vec::new();
        let first_char = self
            .current_line()
            .and_then(|l| l.trim_start().chars().next());
        let ordered = first_char == Some('-');

        while let Some(line) = self.current_line() {
            if !line.starts_with("  ") {
                break;
            }
            let trimmed = line.trim_start();
            if !trimmed.starts_with('*') && !trimmed.starts_with('-') {
                break;
            }

            // Get content after marker
            let content = trimmed[1..].trim_start();
            let item_inlines = self.parse_inline(content);
            let item = Block::Paragraph {
                inlines: item_inlines,
            };
            items.push(vec![item]);
            self.advance();
        }

        Ok(Block::List { ordered, items })
    }

    fn parse_blockquote(&mut self) -> Result<Block, DokuwikiError> {
        let mut lines = Vec::new();

        while let Some(line) = self.current_line() {
            let trimmed = line.trim();
            if !trimmed.starts_with('>') {
                break;
            }
            let content = trimmed[1..].trim_start();
            lines.push(content);
            self.advance();
        }

        let text = lines.join("\n");
        Ok(Block::Blockquote {
            children: vec![Block::Paragraph {
                inlines: vec![Inline::Text(text)],
            }],
        })
    }

    fn parse_paragraph(&mut self) -> Result<Block, DokuwikiError> {
        let mut lines = Vec::new();

        while let Some(line) = self.current_line() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                break;
            }
            // Stop at block-level elements
            if (trimmed.starts_with('=') && trimmed.ends_with('='))
                || trimmed.starts_with("<code")
                || trimmed.starts_with("<file")
                || line.starts_with("  ")
                || trimmed.starts_with('>')
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
        let mut nodes = Vec::new();
        let mut current = String::new();
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // Bold: **text**
            if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '*' {
                if !current.is_empty() {
                    nodes.push(Inline::Text(current.clone()));
                    current.clear();
                }
                if let Some((content, end)) = self.find_double_delim(&chars, i + 2, '*') {
                    nodes.push(Inline::Bold(vec![Inline::Text(content)]));
                    i = end + 2;
                    continue;
                }
            }

            // Italic: //text//
            if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '/' {
                if !current.is_empty() {
                    nodes.push(Inline::Text(current.clone()));
                    current.clear();
                }
                if let Some((content, end)) = self.find_double_delim(&chars, i + 2, '/') {
                    nodes.push(Inline::Italic(vec![Inline::Text(content)]));
                    i = end + 2;
                    continue;
                }
            }

            // Underline: __text__
            if i + 1 < chars.len() && chars[i] == '_' && chars[i + 1] == '_' {
                if !current.is_empty() {
                    nodes.push(Inline::Text(current.clone()));
                    current.clear();
                }
                if let Some((content, end)) = self.find_double_delim(&chars, i + 2, '_') {
                    nodes.push(Inline::Underline(vec![Inline::Text(content)]));
                    i = end + 2;
                    continue;
                }
            }

            // Monospace: ''text''
            if i + 1 < chars.len() && chars[i] == '\'' && chars[i + 1] == '\'' {
                if !current.is_empty() {
                    nodes.push(Inline::Text(current.clone()));
                    current.clear();
                }
                if let Some((content, end)) = self.find_double_delim(&chars, i + 2, '\'') {
                    nodes.push(Inline::Code(content));
                    i = end + 2;
                    continue;
                }
            }

            // Link: [[url|text]]
            if i + 1 < chars.len() && chars[i] == '[' && chars[i + 1] == '[' {
                if !current.is_empty() {
                    nodes.push(Inline::Text(current.clone()));
                    current.clear();
                }
                if let Some((link_content, end)) = self.find_double_bracket(&chars, i + 2) {
                    let (url, link_text) = if let Some(pipe_pos) = link_content.find('|') {
                        (&link_content[..pipe_pos], &link_content[pipe_pos + 1..])
                    } else {
                        (link_content.as_str(), link_content.as_str())
                    };
                    nodes.push(Inline::Link {
                        url: url.to_string(),
                        children: vec![Inline::Text(link_text.to_string())],
                    });
                    i = end + 2;
                    continue;
                }
            }

            // Image: {{url|alt}}
            if i + 1 < chars.len() && chars[i] == '{' && chars[i + 1] == '{' {
                if !current.is_empty() {
                    nodes.push(Inline::Text(current.clone()));
                    current.clear();
                }
                if let Some((img_content, end)) = self.find_double_brace(&chars, i + 2) {
                    let (url, alt) = if let Some(pipe_pos) = img_content.find('|') {
                        (&img_content[..pipe_pos], Some(&img_content[pipe_pos + 1..]))
                    } else {
                        (img_content.as_str(), None)
                    };
                    nodes.push(Inline::Image {
                        url: url.to_string(),
                        alt: alt.map(|s| s.to_string()),
                    });
                    i = end + 2;
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

    fn find_double_delim(
        &self,
        chars: &[char],
        start: usize,
        delim: char,
    ) -> Option<(String, usize)> {
        let mut i = start;
        let mut content = String::new();

        while i + 1 < chars.len() {
            if chars[i] == delim && chars[i + 1] == delim {
                return Some((content, i));
            }
            content.push(chars[i]);
            i += 1;
        }

        None
    }

    fn find_double_bracket(&self, chars: &[char], start: usize) -> Option<(String, usize)> {
        let mut i = start;
        let mut content = String::new();

        while i + 1 < chars.len() {
            if chars[i] == ']' && chars[i + 1] == ']' {
                return Some((content, i));
            }
            content.push(chars[i]);
            i += 1;
        }

        None
    }

    fn find_double_brace(&self, chars: &[char], start: usize) -> Option<(String, usize)> {
        let mut i = start;
        let mut content = String::new();

        while i + 1 < chars.len() {
            if chars[i] == '}' && chars[i + 1] == '}' {
                return Some((content, i));
            }
            content.push(chars[i]);
            i += 1;
        }

        None
    }
}

// ── Builder ───────────────────────────────────────────────────────────────────

/// Build a DokuWiki string from a [`DokuwikiDoc`].
pub fn build(doc: &DokuwikiDoc) -> String {
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
            let equals_count = 7 - (*level as usize).min(6);
            for _ in 0..equals_count {
                ctx.write("=");
            }
            ctx.write(" ");
            build_inlines(inlines, ctx);
            ctx.write(" ");
            for _ in 0..equals_count {
                ctx.write("=");
            }
            ctx.write("\n\n");
        }

        Block::CodeBlock { language, content } => {
            ctx.write("<code");
            if let Some(lang) = language {
                ctx.write(" ");
                ctx.write(lang);
            }
            ctx.write(">\n");
            ctx.write(content);
            if !content.ends_with('\n') {
                ctx.write("\n");
            }
            ctx.write("</code>\n\n");
        }

        Block::Blockquote { children } => {
            for child in children {
                match child {
                    Block::Paragraph { inlines } => {
                        ctx.write("> ");
                        build_inlines(inlines, ctx);
                        ctx.write("\n");
                    }
                    _ => build_block(child, ctx),
                }
            }
            ctx.write("\n");
        }

        Block::List { ordered, items } => {
            ctx.list_depth += 1;
            for item_blocks in items {
                for _ in 0..ctx.list_depth {
                    ctx.write("  ");
                }
                if *ordered {
                    ctx.write("- ");
                } else {
                    ctx.write("* ");
                }
                for block in item_blocks {
                    match block {
                        Block::Paragraph { inlines } => {
                            build_inlines(inlines, ctx);
                            ctx.write("\n");
                        }
                        _ => build_block(block, ctx),
                    }
                }
            }
            ctx.list_depth -= 1;
            if ctx.list_depth == 0 {
                ctx.write("\n");
            }
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

        Inline::Underline(children) => {
            ctx.write("__");
            build_inlines(children, ctx);
            ctx.write("__");
        }

        Inline::Code(s) => {
            ctx.write("''");
            ctx.write(s);
            ctx.write("''");
        }

        Inline::Link { url, children } => {
            ctx.write("[[");
            ctx.write(url);
            ctx.write("|");
            build_inlines(children, ctx);
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

        Inline::LineBreak => ctx.write("\\\\\n"),
        Inline::SoftBreak => ctx.write(" "),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_heading() {
        let doc = parse("====== Title ======").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Heading { level: 1, .. }));
    }

    #[test]
    fn test_parse_heading_levels() {
        let doc = parse("====== H1 ======\n===== H2 =====\n==== H3 ====").unwrap();
        assert_eq!(doc.blocks.len(), 3);
        let Block::Heading { level: l1, .. } = &doc.blocks[0] else {
            panic!("expected heading");
        };
        assert_eq!(*l1, 1);
        let Block::Heading { level: l2, .. } = &doc.blocks[1] else {
            panic!("expected heading");
        };
        assert_eq!(*l2, 2);
        let Block::Heading { level: l3, .. } = &doc.blocks[2] else {
            panic!("expected heading");
        };
        assert_eq!(*l3, 3);
    }

    #[test]
    fn test_parse_paragraph() {
        let doc = parse("Hello world!").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Paragraph { .. }));
    }

    #[test]
    fn test_parse_bold() {
        let doc = parse("This is **bold** text.").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Bold(_))));
    }

    #[test]
    fn test_parse_italic() {
        let doc = parse("This is //italic// text.").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Italic(_))));
    }

    #[test]
    fn test_parse_code() {
        let doc = parse("Use ''code'' here.").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Code(_))));
    }

    #[test]
    fn test_parse_link() {
        let doc = parse("Click [[https://example.com|here]].").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(
            inlines
                .iter()
                .any(|i| matches!(i, Inline::Link { url, .. } if url == "https://example.com"))
        );
    }

    #[test]
    fn test_parse_list() {
        let doc = parse("  * Item 1\n  * Item 2").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        let Block::List { ordered, items } = &doc.blocks[0] else {
            panic!("expected list");
        };
        assert!(!ordered);
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_parse_code_block() {
        let doc = parse("<code rust>\nfn main() {}\n</code>").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        let Block::CodeBlock { language, content } = &doc.blocks[0] else {
            panic!("expected code block");
        };
        assert_eq!(language.as_deref(), Some("rust"));
        assert!(content.contains("fn main()"));
    }

    #[test]
    fn test_build_paragraph() {
        let doc = DokuwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Hello, world!".into())],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("Hello, world!"));
    }

    #[test]
    fn test_build_heading() {
        let doc = DokuwikiDoc {
            blocks: vec![Block::Heading {
                level: 1,
                inlines: vec![Inline::Text("Title".into())],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("====== Title ======"));
    }

    #[test]
    fn test_build_bold() {
        let doc = DokuwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Bold(vec![Inline::Text("bold".into())])],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("**bold**"));
    }

    #[test]
    fn test_build_italic() {
        let doc = DokuwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Italic(vec![Inline::Text("italic".into())])],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("//italic//"));
    }

    #[test]
    fn test_build_code() {
        let doc = DokuwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Code("code".into())],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("''code''"));
    }

    #[test]
    fn test_build_link() {
        let doc = DokuwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "https://example.com".into(),
                    children: vec![Inline::Text("click".into())],
                }],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("[[https://example.com|click]]"));
    }

    #[test]
    fn test_build_code_block() {
        let doc = DokuwikiDoc {
            blocks: vec![Block::CodeBlock {
                language: Some("python".into()),
                content: "print('hi')".into(),
            }],
        };
        let out = build(&doc);
        assert!(out.contains("<code python>"));
        assert!(out.contains("print('hi')"));
        assert!(out.contains("</code>"));
    }
}
