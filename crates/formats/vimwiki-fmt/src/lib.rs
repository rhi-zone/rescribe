//! VimWiki parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-vimwiki` and `rescribe-write-vimwiki` as thin adapter layers.

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct VimwikiError(pub String);

impl std::fmt::Display for VimwikiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "VimWiki error: {}", self.0)
    }
}

impl std::error::Error for VimwikiError {}

// ── AST ───────────────────────────────────────────────────────────────────────

/// A parsed VimWiki document.
#[derive(Debug, Clone, Default)]
pub struct VimwikiDoc {
    pub blocks: Vec<Block>,
}

/// Block-level element.
#[derive(Debug, Clone)]
pub enum Block {
    Paragraph {
        inlines: Vec<Inline>,
    },
    Heading {
        level: usize,
        inlines: Vec<Inline>,
    },
    CodeBlock {
        language: Option<String>,
        content: String,
    },
    Blockquote {
        inlines: Vec<Inline>,
    },
    List {
        ordered: bool,
        items: Vec<ListItem>,
    },
    Table {
        rows: Vec<TableRow>,
    },
    HorizontalRule,
}

/// A list item.
#[derive(Debug, Clone)]
pub struct ListItem {
    pub checked: Option<bool>,
    pub inlines: Vec<Inline>,
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
    Strikethrough(Vec<Inline>),
    Code(String),
    Link { url: String, label: String },
    Image { url: String, alt: Option<String> },
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Parse a VimWiki string into a [`VimwikiDoc`].
pub fn parse(input: &str) -> Result<VimwikiDoc, VimwikiError> {
    let mut p = Parser::new(input);
    let blocks = p.parse();
    Ok(VimwikiDoc { blocks })
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

            // Heading = Title = to ====== Title ======
            if let Some(block) = self.try_parse_heading(line) {
                blocks.push(block);
                self.pos += 1;
                continue;
            }

            // Horizontal rule (4+ dashes)
            if line.trim().chars().all(|c| c == '-') && line.trim().len() >= 4 {
                blocks.push(Block::HorizontalRule);
                self.pos += 1;
                continue;
            }

            // Preformatted block {{{ ... }}}
            if line.trim_start().starts_with("{{{") {
                blocks.push(self.parse_preformatted());
                continue;
            }

            // Unordered list (* or -)
            let trimmed = line.trim_start();
            if (trimmed.starts_with("* ") && !trimmed.starts_with("**"))
                || trimmed.starts_with("- ")
            {
                blocks.push(self.parse_list(false));
                continue;
            }

            // Ordered list (1. or a))
            if self.is_ordered_list_item(line) {
                blocks.push(self.parse_list(true));
                continue;
            }

            // Blockquote (lines starting with >)
            if trimmed.starts_with("> ") || trimmed == ">" {
                blocks.push(self.parse_blockquote());
                continue;
            }

            // Table
            if trimmed.starts_with('|') {
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

        // Count leading =
        let level = trimmed.chars().take_while(|&c| c == '=').count();
        if level == 0 || level > 6 {
            return None;
        }

        // Check for matching trailing =
        let trailing = trimmed.chars().rev().take_while(|&c| c == '=').count();
        if trailing < level {
            return None;
        }

        // Guard against degenerate input like a lone "=" (level + trailing > len).
        if level + trailing > trimmed.len() {
            return None;
        }

        // Extract content
        let content = &trimmed[level..trimmed.len() - trailing].trim();
        if content.is_empty() {
            return None;
        }

        let inlines = parse_inline(content);
        Some(Block::Heading { level, inlines })
    }

    fn parse_preformatted(&mut self) -> Block {
        let mut content = String::new();
        let first_line = self.lines[self.pos].trim_start();

        // Check for language after {{{
        let language = if first_line.len() > 3 {
            let after = first_line[3..].trim();
            if !after.is_empty() {
                Some(after.to_string())
            } else {
                None
            }
        } else {
            None
        };

        self.pos += 1;

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            if line.trim() == "}}}" {
                self.pos += 1;
                break;
            }
            if !content.is_empty() {
                content.push('\n');
            }
            content.push_str(line);
            self.pos += 1;
        }

        Block::CodeBlock { language, content }
    }

    fn is_ordered_list_item(&self, line: &str) -> bool {
        let trimmed = line.trim_start();
        // Check for 1. or a) style
        if let Some(dot_pos) = trimmed.find(". ") {
            let prefix = &trimmed[..dot_pos];
            return prefix.chars().all(|c| c.is_ascii_digit());
        }
        if let Some(paren_pos) = trimmed.find(") ") {
            let prefix = &trimmed[..paren_pos];
            return prefix.len() == 1 && prefix.chars().all(|c| c.is_ascii_lowercase());
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

            // Check if still in list at same or deeper level
            if indent < base_indent {
                break;
            }

            let is_bullet = (trimmed.starts_with("* ") && !trimmed.starts_with("**"))
                || trimmed.starts_with("- ");
            let is_numbered = self.is_ordered_list_item(line);

            if !is_bullet && !is_numbered {
                break;
            }

            // Extract item content
            let content = if is_bullet {
                &trimmed[2..]
            } else if let Some(pos) = trimmed.find(". ") {
                &trimmed[pos + 2..]
            } else if let Some(pos) = trimmed.find(") ") {
                &trimmed[pos + 2..]
            } else {
                break;
            };

            // Check for checkbox [ ] or [X]
            let (checkbox_state, actual_content) = if let Some(rest) = content.strip_prefix("[ ] ")
            {
                (Some(false), rest)
            } else if let Some(rest) = content
                .strip_prefix("[X] ")
                .or_else(|| content.strip_prefix("[x] "))
            {
                (Some(true), rest)
            } else {
                (None, content)
            };

            let inlines = parse_inline(actual_content);
            items.push(ListItem {
                checked: checkbox_state,
                inlines,
            });
            self.pos += 1;
        }

        Block::List { ordered, items }
    }

    fn parse_blockquote(&mut self) -> Block {
        let mut content = String::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            let trimmed = line.trim_start();

            if !trimmed.starts_with('>') {
                break;
            }

            let text = if let Some(rest) = trimmed.strip_prefix("> ") {
                rest
            } else if trimmed == ">" {
                ""
            } else {
                break;
            };

            if !content.is_empty() {
                content.push(' ');
            }
            content.push_str(text);
            self.pos += 1;
        }

        let inlines = parse_inline(&content);
        Block::Blockquote { inlines }
    }

    fn parse_table(&mut self) -> Block {
        let mut rows = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            let trimmed = line.trim();

            if !trimmed.starts_with('|') {
                break;
            }

            // Check for separator row (|---|---|)
            if trimmed.contains("---") {
                self.pos += 1;
                continue;
            }

            let mut cells = Vec::new();
            let parts: Vec<&str> = trimmed.split('|').collect();

            for part in &parts[1..] {
                // Skip empty trailing part
                if part.trim().is_empty() && parts.last() == Some(part) {
                    continue;
                }
                let inlines = parse_inline(part.trim());
                cells.push(inlines);
            }

            if !cells.is_empty() {
                rows.push(TableRow { cells });
            }
            self.pos += 1;
        }

        Block::Table { rows }
    }

    fn parse_paragraph(&mut self) -> Block {
        let mut text = String::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            if line.trim().is_empty() {
                break;
            }

            // Check for block elements
            if self.try_parse_heading(line).is_some()
                || (line.trim().chars().all(|c| c == '-') && line.trim().len() >= 4)
                || line.trim_start().starts_with("{{{")
                || line.trim_start().starts_with("* ")
                || line.trim_start().starts_with("- ")
                || self.is_ordered_list_item(line)
                || line.trim_start().starts_with("> ")
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

fn parse_inline(text: &str) -> Vec<Inline> {
    let mut inlines = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Bold *text*
        if chars[i] == '*'
            && i + 1 < chars.len()
            && chars[i + 1] != '*'
            && chars[i + 1] != ' '
            && let Some((end, content)) = find_closing(&chars, i + 1, '*')
        {
            if !current.is_empty() {
                inlines.push(Inline::Text(current.clone()));
                current.clear();
            }
            let inner = parse_inline(&content);
            inlines.push(Inline::Bold(inner));
            i = end + 1;
            continue;
        }

        // Italic _text_
        if chars[i] == '_'
            && i + 1 < chars.len()
            && chars[i + 1] != ' '
            && let Some((end, content)) = find_closing(&chars, i + 1, '_')
        {
            if !current.is_empty() {
                inlines.push(Inline::Text(current.clone()));
                current.clear();
            }
            let inner = parse_inline(&content);
            inlines.push(Inline::Italic(inner));
            i = end + 1;
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

        // Code `text`
        if chars[i] == '`'
            && let Some((end, content)) = find_closing(&chars, i + 1, '`')
        {
            if !current.is_empty() {
                inlines.push(Inline::Text(current.clone()));
                current.clear();
            }
            inlines.push(Inline::Code(content));
            i = end + 1;
            continue;
        }

        // Wiki link [[link]] or [[link|description]]
        if chars[i] == '['
            && i + 1 < chars.len()
            && chars[i + 1] == '['
            && let Some((end, url, label)) = parse_wiki_link(&chars, i)
        {
            if !current.is_empty() {
                inlines.push(Inline::Text(current.clone()));
                current.clear();
            }
            inlines.push(Inline::Link { url, label });
            i = end;
            continue;
        }

        // Image {{image.png}} or {{image.png|alt}}
        if chars[i] == '{'
            && i + 1 < chars.len()
            && chars[i + 1] == '{'
            && let Some((end, url, alt)) = parse_image(&chars, i)
        {
            if !current.is_empty() {
                inlines.push(Inline::Text(current.clone()));
                current.clear();
            }
            inlines.push(Inline::Image { url, alt });
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
        if chars[i] == marker
            && (i + 1 >= chars.len() || chars[i + 1] == ' ' || !chars[i + 1].is_alphanumeric())
        {
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

fn parse_wiki_link(chars: &[char], start: usize) -> Option<(usize, String, String)> {
    // [[link]] or [[link|description]]
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

fn parse_image(chars: &[char], start: usize) -> Option<(usize, String, Option<String>)> {
    // {{image.png}} or {{image.png|alt}}
    if start + 1 >= chars.len() || chars[start] != '{' || chars[start + 1] != '{' {
        return None;
    }

    let mut i = start + 2;
    let mut content = String::new();

    while i < chars.len() {
        if chars[i] == '}' && i + 1 < chars.len() && chars[i + 1] == '}' {
            let (url, alt) = if let Some(pipe_pos) = content.find('|') {
                (
                    content[..pipe_pos].to_string(),
                    Some(content[pipe_pos + 1..].to_string()),
                )
            } else {
                (content, None)
            };
            return Some((i + 2, url, alt));
        }
        content.push(chars[i]);
        i += 1;
    }
    None
}

// ── Builder ───────────────────────────────────────────────────────────────────

/// Build a VimWiki string from a [`VimwikiDoc`].
pub fn build(doc: &VimwikiDoc) -> String {
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
            let marker: String = "=".repeat(*level);
            ctx.write(&marker);
            ctx.write(" ");
            build_inlines(inlines, ctx);
            ctx.write(" ");
            ctx.write(&marker);
            ctx.write("\n\n");
        }

        Block::CodeBlock { language, content } => {
            ctx.write("{{{");
            if let Some(lang) = language {
                ctx.write(lang);
            }
            ctx.write("\n");
            ctx.write(content);
            if !content.ends_with('\n') {
                ctx.write("\n");
            }
            ctx.write("}}}\n\n");
        }

        Block::Blockquote { inlines } => {
            ctx.write("> ");
            build_inlines(inlines, ctx);
            ctx.write("\n\n");
        }

        Block::List { ordered, items } => {
            let mut num = 1;
            for item in items {
                if *ordered {
                    ctx.write(&format!("{}. ", num));
                    num += 1;
                } else {
                    ctx.write("* ");
                }

                // Check for checkbox
                if let Some(checked) = item.checked {
                    if checked {
                        ctx.write("[X] ");
                    } else {
                        ctx.write("[ ] ");
                    }
                }

                build_inlines(&item.inlines, ctx);
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
            ctx.write("~~");
            build_inlines(children, ctx);
            ctx.write("~~");
        }

        Inline::Code(s) => {
            ctx.write("`");
            ctx.write(s);
            ctx.write("`");
        }

        Inline::Link { url, label } => {
            ctx.write("[[");
            ctx.write(url);
            if url != label {
                ctx.write("|");
                ctx.write(label);
            }
            ctx.write("]]");
        }

        Inline::Image { url, alt } => {
            ctx.write("{{");
            ctx.write(url);
            if let Some(a) = alt {
                ctx.write("|");
                ctx.write(a);
            }
            ctx.write("}}");
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_heading() {
        let doc = parse("= Title =\n").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Heading { level: 1, .. }));
    }

    #[test]
    fn test_parse_heading_level2() {
        let doc = parse("== Subtitle ==\n").unwrap();
        assert!(matches!(doc.blocks[0], Block::Heading { level: 2, .. }));
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
        assert!(inlines.iter().any(|i| matches!(i, Inline::Bold(_))));
    }

    #[test]
    fn test_parse_italic() {
        let doc = parse("_italic_\n").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Italic(_))));
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
    fn test_parse_wiki_link() {
        let doc = parse("[[MyPage]]\n").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Link { .. })));
    }

    #[test]
    fn test_parse_wiki_link_with_description() {
        let doc = parse("[[MyPage|click here]]\n").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        let link = inlines
            .iter()
            .find(|i| matches!(i, Inline::Link { .. }))
            .unwrap();
        assert!(
            matches!(link, Inline::Link { url, label } if url == "MyPage" && label == "click here")
        );
    }

    #[test]
    fn test_parse_unordered_list() {
        let doc = parse("* item1\n* item2\n").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        let Block::List { ordered, items } = &doc.blocks[0] else {
            panic!("expected list");
        };
        assert!(!ordered);
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_parse_ordered_list() {
        let doc = parse("1. first\n2. second\n").unwrap();
        let Block::List { ordered, items } = &doc.blocks[0] else {
            panic!("expected list");
        };
        assert!(*ordered);
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_parse_preformatted() {
        let doc = parse("{{{\ncode here\n}}}\n").unwrap();
        assert!(matches!(doc.blocks[0], Block::CodeBlock { .. }));
    }

    #[test]
    fn test_parse_checkbox() {
        let doc = parse("* [ ] unchecked\n* [X] checked\n").unwrap();
        let Block::List { items, .. } = &doc.blocks[0] else {
            panic!("expected list");
        };
        assert_eq!(items[0].checked, Some(false));
        assert_eq!(items[1].checked, Some(true));
    }

    #[test]
    fn test_build_heading() {
        let doc = VimwikiDoc {
            blocks: vec![Block::Heading {
                level: 1,
                inlines: vec![Inline::Text("Title".to_string())],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("= Title ="));
    }

    #[test]
    fn test_build_paragraph() {
        let doc = VimwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Hello, world!".to_string())],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("Hello, world!"));
    }

    #[test]
    fn test_build_bold() {
        let doc = VimwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Bold(vec![Inline::Text("bold".to_string())])],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("*bold*"));
    }

    #[test]
    fn test_build_italic() {
        let doc = VimwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Italic(vec![Inline::Text("italic".to_string())])],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("_italic_"));
    }

    #[test]
    fn test_build_code() {
        let doc = VimwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Code("code".to_string())],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("`code`"));
    }

    #[test]
    fn test_build_link() {
        let doc = VimwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "MyPage".to_string(),
                    label: "click".to_string(),
                }],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("[[MyPage|click]]"));
    }

    #[test]
    fn test_build_unordered_list() {
        let doc = VimwikiDoc {
            blocks: vec![Block::List {
                ordered: false,
                items: vec![
                    ListItem {
                        checked: None,
                        inlines: vec![Inline::Text("one".to_string())],
                    },
                    ListItem {
                        checked: None,
                        inlines: vec![Inline::Text("two".to_string())],
                    },
                ],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("* one"));
        assert!(output.contains("* two"));
    }

    #[test]
    fn test_build_ordered_list() {
        let doc = VimwikiDoc {
            blocks: vec![Block::List {
                ordered: true,
                items: vec![
                    ListItem {
                        checked: None,
                        inlines: vec![Inline::Text("first".to_string())],
                    },
                    ListItem {
                        checked: None,
                        inlines: vec![Inline::Text("second".to_string())],
                    },
                ],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("1. first"));
        assert!(output.contains("2. second"));
    }

    #[test]
    fn test_build_code_block() {
        let doc = VimwikiDoc {
            blocks: vec![Block::CodeBlock {
                language: None,
                content: "print hi".to_string(),
            }],
        };
        let output = build(&doc);
        assert!(output.contains("{{{"));
        assert!(output.contains("print hi"));
        assert!(output.contains("}}}"));
    }

    #[test]
    fn test_build_horizontal_rule() {
        let doc = VimwikiDoc {
            blocks: vec![Block::HorizontalRule],
        };
        let output = build(&doc);
        assert!(output.contains("----"));
    }
}
