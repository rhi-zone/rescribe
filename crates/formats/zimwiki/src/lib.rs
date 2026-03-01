//! ZimWiki parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-zimwiki` and `rescribe-write-zimwiki` as thin adapter layers.

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct ZimwikiError(pub String);

impl std::fmt::Display for ZimwikiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ZimWiki error: {}", self.0)
    }
}

impl std::error::Error for ZimwikiError {}

// ── AST ───────────────────────────────────────────────────────────────────────

/// A parsed ZimWiki document.
#[derive(Debug, Clone, Default)]
pub struct ZimwikiDoc {
    pub blocks: Vec<Block>,
}

/// Block-level element.
#[derive(Debug, Clone)]
pub enum Block {
    Paragraph { inlines: Vec<Inline> },
    Heading { level: u8, inlines: Vec<Inline> },
    CodeBlock { content: String },
    Blockquote { children: Vec<Block> },
    List { ordered: bool, items: Vec<ListItem> },
    Table { rows: Vec<TableRow> },
    HorizontalRule,
}

/// A list item.
#[derive(Debug, Clone)]
pub struct ListItem {
    pub checked: Option<bool>,
    pub children: Vec<Block>,
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
    Bold(Vec<Inline>),
    Italic(Vec<Inline>),
    Underline(Vec<Inline>),
    Strikethrough(Vec<Inline>),
    Subscript(Vec<Inline>),
    Superscript(Vec<Inline>),
    Code(String),
    Link { url: String, children: Vec<Inline> },
    Image { url: String },
    LineBreak,
    SoftBreak,
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Parse a ZimWiki string into a [`ZimwikiDoc`].
pub fn parse(input: &str) -> Result<ZimwikiDoc, ZimwikiError> {
    let mut p = Parser::new(input);
    let blocks = p.parse();
    Ok(ZimwikiDoc { blocks })
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

            // Heading ====== Title ====== (more = = lower level, inverted)
            if let Some(block) = self.try_parse_heading(line) {
                blocks.push(block);
                self.pos += 1;
                continue;
            }

            // Horizontal rule (line of only dashes, at least 4)
            if line.trim().chars().all(|c| c == '-') && line.trim().len() >= 4 {
                blocks.push(Block::HorizontalRule);
                self.pos += 1;
                continue;
            }

            // Verbatim block '''...'''
            if line.trim_start().starts_with("'''") {
                blocks.push(self.parse_verbatim_block());
                continue;
            }

            // Unordered list (*)
            let trimmed = line.trim_start();
            if trimmed.starts_with("* ") && !trimmed.starts_with("**") {
                blocks.push(self.parse_list(false));
                continue;
            }

            // Ordered list (1. or a.)
            if self.is_ordered_list_item(line) {
                blocks.push(self.parse_list(true));
                continue;
            }

            // Checkbox list [ ] or [*] or [x]
            if trimmed.starts_with("[ ] ")
                || trimmed.starts_with("[*] ")
                || trimmed.starts_with("[x] ")
            {
                blocks.push(self.parse_checkbox_list());
                continue;
            }

            // Regular paragraph
            blocks.push(self.parse_paragraph());
        }

        blocks
    }

    fn try_parse_heading(&self, line: &str) -> Option<Block> {
        let trimmed = line.trim();

        // Count leading =
        let eq_count = trimmed.chars().take_while(|&c| c == '=').count();
        if !(2..=6).contains(&eq_count) {
            return None;
        }

        // Check for matching trailing =
        let trailing = trimmed.chars().rev().take_while(|&c| c == '=').count();
        if trailing < eq_count {
            return None;
        }

        // Extract content
        let content = &trimmed[eq_count..trimmed.len() - trailing].trim();
        if content.is_empty() {
            return None;
        }

        // ZimWiki heading levels are inverted: ====== = level 1, ===== = level 2, etc.
        let level = 7 - eq_count; // 6 = signs -> level 1, 5 -> level 2, etc.

        let inlines = parse_inline(content);
        Some(Block::Heading {
            level: level as u8,
            inlines,
        })
    }

    fn parse_verbatim_block(&mut self) -> Block {
        let mut content = String::new();
        let first_line = self.lines[self.pos].trim_start();

        // Get content after ''' on same line
        if first_line.len() > 3 {
            let after = &first_line[3..];
            if let Some(end_pos) = after.find("'''") {
                content.push_str(&after[..end_pos]);
                self.pos += 1;
                return Block::CodeBlock { content };
            }
            content.push_str(after);
            content.push('\n');
        }
        self.pos += 1;

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            if line.contains("'''") {
                if let Some(pos) = line.find("'''") {
                    content.push_str(&line[..pos]);
                }
                self.pos += 1;
                break;
            }
            content.push_str(line);
            content.push('\n');
            self.pos += 1;
        }

        Block::CodeBlock {
            content: content.trim_end().to_string(),
        }
    }

    fn is_ordered_list_item(&self, line: &str) -> bool {
        let trimmed = line.trim_start();
        if let Some(dot_pos) = trimmed.find(". ") {
            let prefix = &trimmed[..dot_pos];
            return prefix.chars().all(|c| c.is_ascii_digit())
                || (prefix.len() == 1 && prefix.chars().all(|c| c.is_ascii_lowercase()));
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

            let is_bullet = trimmed.starts_with("* ") && !trimmed.starts_with("**");
            let is_numbered = self.is_ordered_list_item(line);

            if !is_bullet && !is_numbered {
                break;
            }

            let content = if is_bullet {
                &trimmed[2..]
            } else if let Some(pos) = trimmed.find(". ") {
                &trimmed[pos + 2..]
            } else {
                break;
            };

            let inlines = parse_inline(content);
            let para = Block::Paragraph { inlines };
            items.push(ListItem {
                checked: None,
                children: vec![para],
            });
            self.pos += 1;
        }

        Block::List { ordered, items }
    }

    fn parse_checkbox_list(&mut self) -> Block {
        let mut items = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            let trimmed = line.trim_start();

            let (checked, content) = if let Some(rest) = trimmed.strip_prefix("[ ] ") {
                (Some(false), rest)
            } else if let Some(rest) = trimmed.strip_prefix("[*] ") {
                (Some(true), rest)
            } else if let Some(rest) = trimmed.strip_prefix("[x] ") {
                (Some(true), rest)
            } else {
                break;
            };

            let inlines = parse_inline(content);
            let para = Block::Paragraph { inlines };
            items.push(ListItem {
                checked,
                children: vec![para],
            });
            self.pos += 1;
        }

        Block::List {
            ordered: false,
            items,
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
            if self.try_parse_heading(line).is_some()
                || (line.trim().chars().all(|c| c == '-') && line.trim().len() >= 4)
                || trimmed.starts_with("'''")
                || (trimmed.starts_with("* ") && !trimmed.starts_with("**"))
                || self.is_ordered_list_item(line)
                || trimmed.starts_with("[ ] ")
                || trimmed.starts_with("[*] ")
                || trimmed.starts_with("[x] ")
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
        // Bold **text** or __text__
        if (chars[i] == '*' || chars[i] == '_')
            && i + 1 < chars.len()
            && chars[i + 1] == chars[i]
            && let Some((end, content)) = find_double_closing(&chars, i + 2, chars[i])
        {
            if !current.is_empty() {
                inlines.push(Inline::Text(current.clone()));
                current.clear();
            }
            let inner = parse_inline(&content);
            inlines.push(Inline::Bold(inner));
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
                inlines.push(Inline::Text(current.clone()));
                current.clear();
            }
            let inner = parse_inline(&content);
            inlines.push(Inline::Italic(inner));
            i = end + 2;
            continue;
        }

        // Strikethrough ~~text~~
        if chars[i] == '~'
            && i + 1 < chars.len()
            && chars[i + 1] == '~'
            && let Some((end, content)) = find_double_closing(&chars, i + 2, '~')
        {
            if !current.is_empty() {
                inlines.push(Inline::Text(current.clone()));
                current.clear();
            }
            let inner = parse_inline(&content);
            inlines.push(Inline::Strikethrough(inner));
            i = end + 2;
            continue;
        }

        // Subscript _{text}
        if chars[i] == '_'
            && i + 1 < chars.len()
            && chars[i + 1] == '{'
            && let Some((end, content)) = find_brace_closing(&chars, i + 2)
        {
            if !current.is_empty() {
                inlines.push(Inline::Text(current.clone()));
                current.clear();
            }
            let inner = parse_inline(&content);
            inlines.push(Inline::Subscript(inner));
            i = end + 1;
            continue;
        }

        // Superscript ^{text}
        if chars[i] == '^'
            && i + 1 < chars.len()
            && chars[i + 1] == '{'
            && let Some((end, content)) = find_brace_closing(&chars, i + 2)
        {
            if !current.is_empty() {
                inlines.push(Inline::Text(current.clone()));
                current.clear();
            }
            let inner = parse_inline(&content);
            inlines.push(Inline::Superscript(inner));
            i = end + 1;
            continue;
        }

        // Inline code ''text''
        if chars[i] == '\''
            && i + 1 < chars.len()
            && chars[i + 1] == '\''
            && let Some((end, content)) = find_double_closing(&chars, i + 2, '\'')
        {
            if !current.is_empty() {
                inlines.push(Inline::Text(current.clone()));
                current.clear();
            }
            inlines.push(Inline::Code(content));
            i = end + 2;
            continue;
        }

        // Link [[target]] or [[target|label]]
        if chars[i] == '['
            && i + 1 < chars.len()
            && chars[i + 1] == '['
            && let Some((end, url, label)) = parse_link(&chars, i)
        {
            if !current.is_empty() {
                inlines.push(Inline::Text(current.clone()));
                current.clear();
            }
            let text_inline = Inline::Text(label);
            inlines.push(Inline::Link {
                url,
                children: vec![text_inline],
            });
            i = end;
            continue;
        }

        // Image {{image.png}}
        if chars[i] == '{'
            && i + 1 < chars.len()
            && chars[i + 1] == '{'
            && let Some((end, url)) = parse_image(&chars, i)
        {
            if !current.is_empty() {
                inlines.push(Inline::Text(current.clone()));
                current.clear();
            }
            inlines.push(Inline::Image { url });
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

fn find_brace_closing(chars: &[char], start: usize) -> Option<(usize, String)> {
    let mut i = start;
    let mut content = String::new();

    while i < chars.len() {
        if chars[i] == '}' {
            return Some((i, content));
        }
        content.push(chars[i]);
        i += 1;
    }
    None
}

fn parse_link(chars: &[char], start: usize) -> Option<(usize, String, String)> {
    if start + 1 >= chars.len() || chars[start] != '[' || chars[start + 1] != '[' {
        return None;
    }

    let mut i = start + 2;
    let mut content = String::new();

    while i < chars.len() {
        if chars[i] == ']' && i + 1 < chars.len() && chars[i + 1] == ']' {
            let (url, label) = if let Some(pipe_pos) = content.find('|') {
                (
                    content[..pipe_pos].to_string(),
                    content[pipe_pos + 1..].to_string(),
                )
            } else {
                (content.clone(), content)
            };
            return Some((i + 2, url, label));
        }
        content.push(chars[i]);
        i += 1;
    }
    None
}

fn parse_image(chars: &[char], start: usize) -> Option<(usize, String)> {
    if start + 1 >= chars.len() || chars[start] != '{' || chars[start + 1] != '{' {
        return None;
    }

    let mut i = start + 2;
    let mut content = String::new();

    while i < chars.len() {
        if chars[i] == '}' && i + 1 < chars.len() && chars[i + 1] == '}' {
            return Some((i + 2, content));
        }
        content.push(chars[i]);
        i += 1;
    }
    None
}

// ── Builder ───────────────────────────────────────────────────────────────────

/// Build a ZimWiki string from a [`ZimwikiDoc`].
pub fn build(doc: &ZimwikiDoc) -> String {
    let mut ctx = BuildContext::new();
    build_blocks(&doc.blocks, &mut ctx, 0);
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

fn build_blocks(blocks: &[Block], ctx: &mut BuildContext, _depth: usize) {
    for block in blocks {
        build_block(block, ctx);
    }
}

fn build_block(block: &Block, ctx: &mut BuildContext) {
    match block {
        Block::Paragraph { inlines } => {
            build_inlines(inlines, ctx);
            ctx.write("\n\n");
        }

        Block::Heading { level, inlines } => {
            let level = (*level as usize).clamp(1, 5);
            // ZimWiki uses inverted levels: level 1 = 6 equals signs, level 2 = 5, etc.
            let eq_count = 7 - level;
            let marker: String = "=".repeat(eq_count);
            ctx.write(&marker);
            ctx.write(" ");
            build_inlines(inlines, ctx);
            ctx.write(" ");
            ctx.write(&marker);
            ctx.write("\n\n");
        }

        Block::CodeBlock { content } => {
            ctx.write("'''\n");
            ctx.write(content);
            if !content.ends_with('\n') {
                ctx.write("\n");
            }
            ctx.write("'''\n\n");
        }

        Block::Blockquote { children } => {
            for child in children {
                if let Block::Paragraph { inlines } = child {
                    ctx.write("> ");
                    build_inlines(inlines, ctx);
                    ctx.write("\n");
                } else {
                    build_block(child, ctx);
                }
            }
            ctx.write("\n");
        }

        Block::List { ordered, items } => {
            let mut num = 1;

            for item in items {
                // Check for checkbox first
                if let Some(checked) = item.checked {
                    if checked {
                        ctx.write("[*] ");
                    } else {
                        ctx.write("[ ] ");
                    }
                } else if *ordered {
                    ctx.write(&format!("{}. ", num));
                    num += 1;
                } else {
                    ctx.write("* ");
                }

                for child in &item.children {
                    if let Block::Paragraph { inlines } = child {
                        build_inlines(inlines, ctx);
                    } else {
                        build_block(child, ctx);
                    }
                }
                ctx.write("\n");
            }
            ctx.write("\n");
        }

        Block::Table { rows } => {
            for row in rows {
                ctx.write("|");
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

        Inline::Strikethrough(children) => {
            ctx.write("~~");
            build_inlines(children, ctx);
            ctx.write("~~");
        }

        Inline::Code(s) => {
            ctx.write("''");
            ctx.write(s);
            ctx.write("''");
        }

        Inline::Subscript(children) => {
            ctx.write("_{");
            build_inlines(children, ctx);
            ctx.write("}");
        }

        Inline::Superscript(children) => {
            ctx.write("^{");
            build_inlines(children, ctx);
            ctx.write("}");
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

        Inline::Image { url } => {
            ctx.write("{{");
            ctx.write(url);
            ctx.write("}}");
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

    #[test]
    fn test_parse_heading_level1() {
        let doc = parse("====== Title ======\n").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        match &doc.blocks[0] {
            Block::Heading { level, .. } => assert_eq!(*level, 1),
            _ => panic!("expected heading"),
        }
    }

    #[test]
    fn test_parse_heading_level2() {
        let doc = parse("===== Subtitle =====\n").unwrap();
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
        assert!(inlines.iter().any(|i| matches!(i, Inline::Bold(_))));
    }

    #[test]
    fn test_parse_italic() {
        let doc = parse("//italic//\n").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Italic(_))));
    }

    #[test]
    fn test_parse_strikethrough() {
        let doc = parse("~~strike~~\n").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(
            inlines
                .iter()
                .any(|i| matches!(i, Inline::Strikethrough(_)))
        );
    }

    #[test]
    fn test_parse_code() {
        let doc = parse("''code''\n").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Code(_))));
    }

    #[test]
    fn test_parse_link() {
        let doc = parse("[[MyPage]]\n").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        let link = inlines.iter().find(|i| matches!(i, Inline::Link { .. }));
        assert!(link.is_some());
    }

    #[test]
    fn test_parse_link_with_label() {
        let doc = parse("[[MyPage|click here]]\n").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        if let Some(Inline::Link { url, .. }) =
            inlines.iter().find(|i| matches!(i, Inline::Link { .. }))
        {
            assert_eq!(url, "MyPage");
        } else {
            panic!("expected link");
        }
    }

    #[test]
    fn test_parse_unordered_list() {
        let doc = parse("* item1\n* item2\n").unwrap();
        let Block::List { items, .. } = &doc.blocks[0] else {
            panic!("expected list");
        };
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_parse_checkbox_list() {
        let doc = parse("[ ] unchecked\n[*] checked\n").unwrap();
        let Block::List { items, .. } = &doc.blocks[0] else {
            panic!("expected list");
        };
        assert_eq!(items[0].checked, Some(false));
        assert_eq!(items[1].checked, Some(true));
    }

    #[test]
    fn test_parse_verbatim() {
        let doc = parse("'''\ncode here\n'''\n").unwrap();
        assert!(matches!(doc.blocks[0], Block::CodeBlock { .. }));
    }

    #[test]
    fn test_build_heading_level1() {
        let doc = ZimwikiDoc {
            blocks: vec![Block::Heading {
                level: 1,
                inlines: vec![Inline::Text("Title".into())],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("====== Title ======"));
    }

    #[test]
    fn test_build_heading_level2() {
        let doc = ZimwikiDoc {
            blocks: vec![Block::Heading {
                level: 2,
                inlines: vec![Inline::Text("Subtitle".into())],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("===== Subtitle ====="));
    }

    #[test]
    fn test_build_paragraph() {
        let doc = ZimwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Hello, world!".into())],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("Hello, world!"));
    }

    #[test]
    fn test_build_bold() {
        let doc = ZimwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Bold(vec![Inline::Text("bold".into())])],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("**bold**"));
    }

    #[test]
    fn test_build_italic() {
        let doc = ZimwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Italic(vec![Inline::Text("italic".into())])],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("//italic//"));
    }

    #[test]
    fn test_build_strikethrough() {
        let doc = ZimwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Strikethrough(vec![Inline::Text("deleted".into())])],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("~~deleted~~"));
    }

    #[test]
    fn test_build_code() {
        let doc = ZimwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Code("code".into())],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("''code''"));
    }

    #[test]
    fn test_build_link() {
        let doc = ZimwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "MyPage".into(),
                    children: vec![Inline::Text("click".into())],
                }],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("[[MyPage|click]]"));
    }

    #[test]
    fn test_build_unordered_list() {
        let doc = ZimwikiDoc {
            blocks: vec![Block::List {
                ordered: false,
                items: vec![
                    ListItem {
                        checked: None,
                        children: vec![Block::Paragraph {
                            inlines: vec![Inline::Text("one".into())],
                        }],
                    },
                    ListItem {
                        checked: None,
                        children: vec![Block::Paragraph {
                            inlines: vec![Inline::Text("two".into())],
                        }],
                    },
                ],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("* one"));
        assert!(out.contains("* two"));
    }

    #[test]
    fn test_build_code_block() {
        let doc = ZimwikiDoc {
            blocks: vec![Block::CodeBlock {
                content: "print hi".into(),
            }],
        };
        let out = build(&doc);
        assert!(out.contains("'''"));
        assert!(out.contains("print hi"));
    }

    #[test]
    fn test_build_horizontal_rule() {
        let doc = ZimwikiDoc {
            blocks: vec![Block::HorizontalRule],
        };
        let out = build(&doc);
        assert!(out.contains("----"));
    }

    #[test]
    fn test_build_image() {
        let doc = ZimwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Image {
                    url: "image.png".into(),
                }],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("{{image.png}}"));
    }
}
