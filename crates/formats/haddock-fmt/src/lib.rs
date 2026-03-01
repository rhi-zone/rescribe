//! Haddock documentation parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-haddock` and `rescribe-write-haddock` as thin adapter layers.

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct HaddockError(pub String);

impl std::fmt::Display for HaddockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Haddock error: {}", self.0)
    }
}

impl std::error::Error for HaddockError {}

// ── AST ───────────────────────────────────────────────────────────────────────

/// A parsed Haddock document.
#[derive(Debug, Clone, Default)]
pub struct HaddockDoc {
    pub blocks: Vec<Block>,
}

/// Block-level element.
#[derive(Debug, Clone)]
pub enum Block {
    Heading {
        level: u8,
        inlines: Vec<Inline>,
    },
    Paragraph {
        inlines: Vec<Inline>,
    },
    CodeBlock {
        content: String,
    },
    UnorderedList {
        items: Vec<Vec<Inline>>,
    },
    OrderedList {
        items: Vec<Vec<Inline>>,
    },
    DefinitionList {
        items: Vec<(Vec<Inline>, Vec<Inline>)>,
    },
}

/// Inline element.
#[derive(Debug, Clone)]
pub enum Inline {
    Text(String),
    Code(String),
    Strong(Vec<Inline>),
    Emphasis(Vec<Inline>),
    Link { url: String, text: String },
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Parse a Haddock string into a [`HaddockDoc`].
pub fn parse(input: &str) -> Result<HaddockDoc, HaddockError> {
    let mut p = Parser::new(input);
    let blocks = p.parse();
    Ok(HaddockDoc { blocks })
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

            if line.trim().is_empty() {
                self.pos += 1;
                continue;
            }

            // Code block (indented with > or @, but not inline @code@)
            if line.starts_with("> ") || (line.starts_with("@ ") && !line.contains("@@")) {
                blocks.push(self.parse_code_block());
                continue;
            }

            // Heading (= to ====)
            if let Some(block) = self.try_parse_heading(line) {
                blocks.push(block);
                self.pos += 1;
                continue;
            }

            // Definition list [term]
            if line.trim_start().starts_with('[')
                && let Some(block) = self.parse_definition_list()
            {
                blocks.push(block);
                continue;
            }

            // Unordered list *
            if line.trim_start().starts_with("* ") {
                blocks.push(self.parse_unordered_list());
                continue;
            }

            // Ordered list (1)
            if self.is_ordered_list_item(line) {
                blocks.push(self.parse_ordered_list());
                continue;
            }

            // Regular paragraph
            blocks.push(self.parse_paragraph());
        }

        blocks
    }

    fn try_parse_heading(&self, line: &str) -> Option<Block> {
        let trimmed = line.trim_start();

        // Count leading = signs
        let level = trimmed.chars().take_while(|&c| c == '=').count();

        if level > 0 && level <= 6 {
            let rest = trimmed[level..].trim();
            // Remove trailing = if present
            let content = rest.trim_end_matches('=').trim();
            let inlines = parse_inline(content);

            return Some(Block::Heading {
                level: level as u8,
                inlines,
            });
        }
        None
    }

    fn parse_code_block(&mut self) -> Block {
        let mut content = String::new();
        let marker = self.lines[self.pos].chars().next().unwrap();
        let marker_with_space = format!("{} ", marker);

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            if !line.starts_with(&marker_with_space) && !line.trim().is_empty() {
                break;
            }

            if line.starts_with(&marker_with_space) {
                // Remove the marker and space
                let code_line = &line[2..];
                content.push_str(code_line);
                content.push('\n');
            }
            self.pos += 1;
        }

        Block::CodeBlock {
            content: content.trim_end().to_string(),
        }
    }

    fn is_ordered_list_item(&self, line: &str) -> bool {
        let trimmed = line.trim_start();
        if trimmed.starts_with('(')
            && let Some(close) = trimmed.find(')')
        {
            let num = &trimmed[1..close];
            return num.chars().all(|c| c.is_ascii_digit());
        }
        false
    }

    fn parse_unordered_list(&mut self) -> Block {
        let mut items = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            let trimmed = line.trim_start();

            if !trimmed.starts_with("* ") {
                break;
            }

            let content = trimmed[2..].trim();
            let inlines = parse_inline(content);
            items.push(inlines);
            self.pos += 1;
        }

        Block::UnorderedList { items }
    }

    fn parse_ordered_list(&mut self) -> Block {
        let mut items = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            if !self.is_ordered_list_item(line) {
                break;
            }

            let trimmed = line.trim_start();
            // Find the closing ) and get content after it
            if let Some(close) = trimmed.find(')') {
                let content = trimmed[close + 1..].trim();
                let inlines = parse_inline(content);
                items.push(inlines);
            }
            self.pos += 1;
        }

        Block::OrderedList { items }
    }

    fn parse_definition_list(&mut self) -> Option<Block> {
        let mut items = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            let trimmed = line.trim_start();

            if !trimmed.starts_with('[') {
                break;
            }

            // Find closing bracket
            if let Some(close) = trimmed.find(']') {
                let term = &trimmed[1..close];
                let desc = trimmed[close + 1..].trim();

                let term_inlines = parse_inline(term);
                let desc_inlines = parse_inline(desc);

                items.push((term_inlines, desc_inlines));
            }
            self.pos += 1;
        }

        if items.is_empty() {
            None
        } else {
            Some(Block::DefinitionList { items })
        }
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
                || trimmed.starts_with("* ")
                || trimmed.starts_with('[')
                || trimmed.starts_with("> ")
                || (trimmed.starts_with("@ ") && !trimmed.contains("@@"))
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

        let inlines = parse_inline(&text);
        Block::Paragraph { inlines }
    }
}

fn parse_inline(text: &str) -> Vec<Inline> {
    let mut inlines = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Inline code @...@
        if chars[i] == '@'
            && i + 1 < chars.len()
            && let Some((end, content)) = find_closing(&chars, i + 1, '@')
        {
            if !current.is_empty() {
                inlines.push(Inline::Text(current.clone()));
                current.clear();
            }
            inlines.push(Inline::Code(content));
            i = end + 1;
            continue;
        }

        // Bold __...__
        if chars[i] == '_'
            && i + 1 < chars.len()
            && chars[i + 1] == '_'
            && i + 2 < chars.len()
            && let Some((end, content)) = find_double_closing(&chars, i + 2, '_')
        {
            if !current.is_empty() {
                inlines.push(Inline::Text(current.clone()));
                current.clear();
            }
            let inner = parse_inline(&content);
            inlines.push(Inline::Strong(inner));
            i = end + 2;
            continue;
        }

        // Italic /.../ (but not //)
        if chars[i] == '/'
            && i + 1 < chars.len()
            && chars[i + 1] != '/'
            && (i == 0 || !chars[i - 1].is_alphanumeric())
            && let Some((end, content)) = find_closing(&chars, i + 1, '/')
        {
            if !current.is_empty() {
                inlines.push(Inline::Text(current.clone()));
                current.clear();
            }
            let inner = parse_inline(&content);
            inlines.push(Inline::Emphasis(inner));
            i = end + 1;
            continue;
        }

        // Identifier reference '...'
        if chars[i] == '\''
            && i + 1 < chars.len()
            && let Some((end, content)) = find_closing(&chars, i + 1, '\'')
        {
            if !current.is_empty() {
                inlines.push(Inline::Text(current.clone()));
                current.clear();
            }
            inlines.push(Inline::Code(content));
            i = end + 1;
            continue;
        }

        // Link "text"<url> or raw URL <url>
        if chars[i] == '"'
            && let Some((end, link_text, url)) = parse_haddock_link(&chars, i)
        {
            if !current.is_empty() {
                inlines.push(Inline::Text(current.clone()));
                current.clear();
            }
            inlines.push(Inline::Link {
                url,
                text: link_text,
            });
            i = end;
            continue;
        }

        // Raw URL <url>
        if chars[i] == '<'
            && let Some((end, url)) = parse_raw_url(&chars, i)
        {
            if !current.is_empty() {
                inlines.push(Inline::Text(current.clone()));
                current.clear();
            }
            inlines.push(Inline::Link {
                url: url.clone(),
                text: url,
            });
            i = end;
            continue;
        }

        current.push(chars[i]);
        i += 1;
    }

    if !current.is_empty() {
        inlines.push(Inline::Text(current));
    }

    inlines
}

fn find_closing(chars: &[char], start: usize, marker: char) -> Option<(usize, String)> {
    let mut i = start;
    let mut content = String::new();

    while i < chars.len() {
        if chars[i] == marker {
            return Some((i, content));
        }
        content.push(chars[i]);
        i += 1;
    }
    None
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

fn parse_haddock_link(chars: &[char], start: usize) -> Option<(usize, String, String)> {
    // "text"<url>
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

    // Must be followed by <
    if i >= chars.len() || chars[i] != '<' {
        return None;
    }
    i += 1; // skip <

    // Collect URL until >
    let mut url = String::new();
    while i < chars.len() && chars[i] != '>' {
        url.push(chars[i]);
        i += 1;
    }

    if i >= chars.len() || chars[i] != '>' {
        return None;
    }
    i += 1; // skip >

    Some((i, link_text, url))
}

fn parse_raw_url(chars: &[char], start: usize) -> Option<(usize, String)> {
    // <url>
    if chars[start] != '<' {
        return None;
    }

    let mut i = start + 1;
    let mut url = String::new();

    while i < chars.len() && chars[i] != '>' {
        url.push(chars[i]);
        i += 1;
    }

    if i >= chars.len() || chars[i] != '>' {
        return None;
    }
    i += 1;

    // Basic URL validation
    if url.starts_with("http://") || url.starts_with("https://") || url.contains('@') {
        Some((i, url))
    } else {
        None
    }
}

// ── Builder ───────────────────────────────────────────────────────────────────

/// Build a Haddock string from a [`HaddockDoc`].
pub fn build(doc: &HaddockDoc) -> String {
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
        Block::Heading { level, inlines } => {
            for _ in 0..*level {
                ctx.write("=");
            }
            ctx.write(" ");
            build_inlines(inlines, ctx);
            ctx.write("\n\n");
        }

        Block::Paragraph { inlines } => {
            build_inlines(inlines, ctx);
            ctx.write("\n\n");
        }

        Block::CodeBlock { content } => {
            for line in content.lines() {
                ctx.write("> ");
                ctx.write(line);
                ctx.write("\n");
            }
            ctx.write("\n");
        }

        Block::UnorderedList { items } => {
            for item_inlines in items {
                ctx.write("* ");
                build_inlines(item_inlines, ctx);
                ctx.write("\n");
            }
            ctx.write("\n");
        }

        Block::OrderedList { items } => {
            for (i, item_inlines) in items.iter().enumerate() {
                ctx.write(&format!("({}) ", i + 1));
                build_inlines(item_inlines, ctx);
                ctx.write("\n");
            }
            ctx.write("\n");
        }

        Block::DefinitionList { items } => {
            for (term_inlines, desc_inlines) in items {
                ctx.write("[");
                build_inlines(term_inlines, ctx);
                ctx.write("] ");
                build_inlines(desc_inlines, ctx);
                ctx.write("\n");
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

        Inline::Code(s) => {
            ctx.write("@");
            ctx.write(s);
            ctx.write("@");
        }

        Inline::Strong(children) => {
            ctx.write("__");
            build_inlines(children, ctx);
            ctx.write("__");
        }

        Inline::Emphasis(children) => {
            ctx.write("/");
            build_inlines(children, ctx);
            ctx.write("/");
        }

        Inline::Link { url, text } => {
            ctx.write("\"");
            ctx.write(text);
            ctx.write("\"<");
            ctx.write(url);
            ctx.write(">");
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
        assert!(matches!(doc.blocks[0], Block::Heading { level: 1, .. }));
    }

    #[test]
    fn test_parse_heading_levels() {
        let doc = parse("== Level 2\n=== Level 3\n").unwrap();
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
        let doc = parse("__bold__\n").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert_eq!(inlines.len(), 1);
        assert!(matches!(inlines[0], Inline::Strong(_)));
    }

    #[test]
    fn test_parse_italic() {
        let doc = parse("/italic/\n").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert_eq!(inlines.len(), 1);
        assert!(matches!(inlines[0], Inline::Emphasis(_)));
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
        let doc = parse("\"Example\"<https://example.com>\n").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        let link = &inlines[0];
        assert!(matches!(link, Inline::Link { .. }));
        if let Inline::Link { url, text } = link {
            assert_eq!(url, "https://example.com");
            assert_eq!(text, "Example");
        }
    }

    #[test]
    fn test_parse_unordered_list() {
        let doc = parse("* item1\n* item2\n").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::UnorderedList { .. }));
        if let Block::UnorderedList { items } = &doc.blocks[0] {
            assert_eq!(items.len(), 2);
        }
    }

    #[test]
    fn test_parse_code_block() {
        let doc = parse("> code here\n").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::CodeBlock { .. }));
    }

    #[test]
    fn test_build_heading() {
        let doc = HaddockDoc {
            blocks: vec![Block::Heading {
                level: 1,
                inlines: vec![Inline::Text("Title".into())],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("= Title"));
    }

    #[test]
    fn test_build_paragraph() {
        let doc = HaddockDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Hello, world!".into())],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("Hello, world!"));
    }

    #[test]
    fn test_build_bold() {
        let doc = HaddockDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Strong(vec![Inline::Text("bold".into())])],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("__bold__"));
    }

    #[test]
    fn test_build_italic() {
        let doc = HaddockDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Emphasis(vec![Inline::Text("italic".into())])],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("/italic/"));
    }

    #[test]
    fn test_build_code() {
        let doc = HaddockDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Code("code".into())],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("@code@"));
    }

    #[test]
    fn test_build_link() {
        let doc = HaddockDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "https://example.com".into(),
                    text: "click".into(),
                }],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("\"click\"<https://example.com>"));
    }

    #[test]
    fn test_build_unordered_list() {
        let doc = HaddockDoc {
            blocks: vec![Block::UnorderedList {
                items: vec![
                    vec![Inline::Text("one".into())],
                    vec![Inline::Text("two".into())],
                ],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("* one"));
        assert!(out.contains("* two"));
    }

    #[test]
    fn test_build_ordered_list() {
        let doc = HaddockDoc {
            blocks: vec![Block::OrderedList {
                items: vec![
                    vec![Inline::Text("first".into())],
                    vec![Inline::Text("second".into())],
                ],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("(1) first"));
        assert!(out.contains("(2) second"));
    }

    #[test]
    fn test_build_code_block() {
        let doc = HaddockDoc {
            blocks: vec![Block::CodeBlock {
                content: "print hi".into(),
            }],
        };
        let out = build(&doc);
        assert!(out.contains("> print hi"));
    }
}
