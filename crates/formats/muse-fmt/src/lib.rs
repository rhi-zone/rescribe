//! Muse markup parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-muse` and `rescribe-write-muse` as thin adapter layers.

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct MuseError(pub String);

impl std::fmt::Display for MuseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Muse error: {}", self.0)
    }
}

impl std::error::Error for MuseError {}

// ── AST ───────────────────────────────────────────────────────────────────────

/// A parsed Muse document.
#[derive(Debug, Clone, Default)]
pub struct MuseDoc {
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
    DefinitionList {
        items: Vec<(Vec<Inline>, Vec<Block>)>,
    },
    HorizontalRule,
}

/// Inline element.
#[derive(Debug, Clone)]
pub enum Inline {
    Text(String),
    Bold(Vec<Inline>),
    Italic(Vec<Inline>),
    Code(String),
    Link { url: String, children: Vec<Inline> },
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Parse a Muse string into a [`MuseDoc`].
pub fn parse(input: &str) -> Result<MuseDoc, MuseError> {
    let mut p = Parser::new(input);
    let blocks = p.parse();
    Ok(MuseDoc { blocks })
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

            // Example block <example>...</example>
            if line.trim_start().starts_with("<example>") {
                nodes.push(self.parse_example_block());
                continue;
            }

            // Verse block <verse>...</verse>
            if line.trim_start().starts_with("<verse>") {
                nodes.push(self.parse_verse_block());
                continue;
            }

            // Quote block <quote>...</quote>
            if line.trim_start().starts_with("<quote>") {
                nodes.push(self.parse_quote_block());
                continue;
            }

            // Heading * to *****
            if let Some(node) = self.try_parse_heading(line) {
                nodes.push(node);
                self.pos += 1;
                continue;
            }

            // Horizontal rule (4+ dashes)
            if line.trim().starts_with("----") {
                nodes.push(Block::HorizontalRule);
                self.pos += 1;
                continue;
            }

            // Unordered list (space before -)
            if line.starts_with(" - ") || line.starts_with("  - ") {
                nodes.push(self.parse_unordered_list());
                continue;
            }

            // Ordered list (space before number)
            if self.is_ordered_list_item(line) {
                nodes.push(self.parse_ordered_list());
                continue;
            }

            // Definition list (term ::)
            if line.contains(" :: ") {
                nodes.push(self.parse_definition_list());
                continue;
            }

            // Indented code block
            if line.starts_with("  ") && !line.trim().is_empty() {
                nodes.push(self.parse_indented_code());
                continue;
            }

            // Regular paragraph
            nodes.push(self.parse_paragraph());
        }

        nodes
    }

    fn try_parse_heading(&self, line: &str) -> Option<Block> {
        // Muse headings: * to *****
        let level = line.chars().take_while(|&c| c == '*').count();

        if level > 0 && level <= 5 && line.len() > level && line.chars().nth(level) == Some(' ') {
            let content = line[level + 1..].trim();
            let inline_nodes = parse_inline(content);

            return Some(Block::Heading {
                level: level as u8,
                inlines: inline_nodes,
            });
        }
        None
    }

    fn parse_example_block(&mut self) -> Block {
        let mut content = String::new();
        let first_line = self.lines[self.pos];

        // Get content after <example> on same line
        if let Some(pos) = first_line.find("<example>") {
            let after = &first_line[pos + 9..];
            if let Some(end) = after.find("</example>") {
                content.push_str(&after[..end]);
                self.pos += 1;
                return Block::CodeBlock { content };
            }
            if !after.trim().is_empty() {
                content.push_str(after.trim());
                content.push('\n');
            }
        }
        self.pos += 1;

        // Multi-line
        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            if line.contains("</example>") {
                if let Some(pos) = line.find("</example>") {
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

    fn parse_verse_block(&mut self) -> Block {
        let mut content = String::new();
        let first_line = self.lines[self.pos];

        if let Some(pos) = first_line.find("<verse>") {
            let after = &first_line[pos + 7..];
            if let Some(end) = after.find("</verse>") {
                content.push_str(&after[..end]);
                self.pos += 1;
                let inline = parse_inline(&content);
                return Block::Blockquote {
                    children: vec![Block::Paragraph { inlines: inline }],
                };
            }
            if !after.trim().is_empty() {
                content.push_str(after.trim());
                content.push('\n');
            }
        }
        self.pos += 1;

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            if line.contains("</verse>") {
                if let Some(pos) = line.find("</verse>") {
                    content.push_str(&line[..pos]);
                }
                self.pos += 1;
                break;
            }
            content.push_str(line);
            content.push('\n');
            self.pos += 1;
        }

        let inline = parse_inline(content.trim_end());
        Block::Blockquote {
            children: vec![Block::Paragraph { inlines: inline }],
        }
    }

    fn parse_quote_block(&mut self) -> Block {
        let mut content = String::new();
        let first_line = self.lines[self.pos];

        if let Some(pos) = first_line.find("<quote>") {
            let after = &first_line[pos + 7..];
            if let Some(end) = after.find("</quote>") {
                content.push_str(&after[..end]);
                self.pos += 1;
                let inline = parse_inline(&content);
                return Block::Blockquote {
                    children: vec![Block::Paragraph { inlines: inline }],
                };
            }
            if !after.trim().is_empty() {
                content.push_str(after.trim());
                content.push('\n');
            }
        }
        self.pos += 1;

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            if line.contains("</quote>") {
                if let Some(pos) = line.find("</quote>") {
                    content.push_str(&line[..pos]);
                }
                self.pos += 1;
                break;
            }
            content.push_str(line);
            content.push('\n');
            self.pos += 1;
        }

        let inline = parse_inline(content.trim_end());
        Block::Blockquote {
            children: vec![Block::Paragraph { inlines: inline }],
        }
    }

    fn is_ordered_list_item(&self, line: &str) -> bool {
        if line.starts_with(' ') {
            let trimmed = line.trim_start();
            if let Some(dot_pos) = trimmed.find(". ") {
                let num = &trimmed[..dot_pos];
                return num.chars().all(|c| c.is_ascii_digit());
            }
        }
        false
    }

    fn parse_unordered_list(&mut self) -> Block {
        let mut items = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            if !line.starts_with(" - ") && !line.starts_with("  - ") {
                break;
            }

            let content = line.trim_start()[2..].trim();
            let inline_nodes = parse_inline(content);
            items.push(vec![Block::Paragraph {
                inlines: inline_nodes,
            }]);
            self.pos += 1;
        }

        Block::List {
            ordered: false,
            items,
        }
    }

    fn parse_ordered_list(&mut self) -> Block {
        let mut items = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            if !self.is_ordered_list_item(line) {
                break;
            }

            let trimmed = line.trim_start();
            if let Some(dot_pos) = trimmed.find(". ") {
                let content = &trimmed[dot_pos + 2..];
                let inline_nodes = parse_inline(content);
                items.push(vec![Block::Paragraph {
                    inlines: inline_nodes,
                }]);
            }
            self.pos += 1;
        }

        Block::List {
            ordered: true,
            items,
        }
    }

    fn parse_definition_list(&mut self) -> Block {
        let mut items = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            if !line.contains(" :: ") {
                break;
            }

            if let Some(sep_pos) = line.find(" :: ") {
                let term = &line[..sep_pos];
                let desc = &line[sep_pos + 4..];

                let term_inlines = parse_inline(term.trim());
                let desc_block = Block::Paragraph {
                    inlines: parse_inline(desc.trim()),
                };

                items.push((term_inlines, vec![desc_block]));
            }
            self.pos += 1;
        }

        Block::DefinitionList { items }
    }

    fn parse_indented_code(&mut self) -> Block {
        let mut content = String::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            if !line.starts_with("  ") && !line.trim().is_empty() {
                break;
            }

            if let Some(stripped) = line.strip_prefix("  ") {
                content.push_str(stripped);
                content.push('\n');
            }
            self.pos += 1;
        }

        Block::CodeBlock {
            content: content.trim_end().to_string(),
        }
    }

    fn parse_paragraph(&mut self) -> Block {
        let mut text = String::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            if line.trim().is_empty() {
                break;
            }

            // Check for block elements - but not **bold**
            let is_heading = line.chars().take_while(|&c| c == '*').count() > 0
                && line.chars().find(|&c| c != '*') == Some(' ');
            if is_heading
                || line.starts_with("----")
                || line.starts_with(" - ")
                || (line.starts_with("  ") && !line.trim().is_empty())
                || line.contains(" :: ")
                || line.trim_start().starts_with('<')
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

fn parse_inline(text: &str) -> Vec<Inline> {
    let mut nodes = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Inline code =...=
        if chars[i] == '='
            && i + 1 < chars.len()
            && let Some((end, content)) = find_closing(&chars, i + 1, '=')
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone()));
                current.clear();
            }
            nodes.push(Inline::Code(content));
            i = end + 1;
            continue;
        }

        // Bold **...** (doubled asterisks)
        if chars[i] == '*'
            && i + 1 < chars.len()
            && chars[i + 1] == '*'
            && i + 2 < chars.len()
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

        // Emphasis *...*
        if chars[i] == '*'
            && i + 1 < chars.len()
            && chars[i + 1] != '*'
            && (i == 0 || !chars[i - 1].is_alphanumeric())
            && let Some((end, content)) = find_closing(&chars, i + 1, '*')
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone()));
                current.clear();
            }
            let inner = parse_inline(&content);
            nodes.push(Inline::Italic(inner));
            i = end + 1;
            continue;
        }

        // Link [[url][text]] or [[url]]
        if chars[i] == '['
            && i + 1 < chars.len()
            && chars[i + 1] == '['
            && let Some((end, url, link_text)) = parse_muse_link(&chars, i)
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone()));
                current.clear();
            }
            nodes.push(Inline::Link {
                url,
                children: vec![Inline::Text(link_text)],
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

fn parse_muse_link(chars: &[char], start: usize) -> Option<(usize, String, String)> {
    // [[url][text]] or [[url]]
    if start + 1 >= chars.len() || chars[start] != '[' || chars[start + 1] != '[' {
        return None;
    }

    let mut i = start + 2;
    let mut url = String::new();

    // Collect URL until ] or [
    while i < chars.len() && chars[i] != ']' && chars[i] != '[' {
        url.push(chars[i]);
        i += 1;
    }

    if i >= chars.len() {
        return None;
    }

    // Check for link text
    if chars[i] == ']' && i + 1 < chars.len() && chars[i + 1] == '[' {
        i += 2;
        let mut text = String::new();
        while i < chars.len() && chars[i] != ']' {
            text.push(chars[i]);
            i += 1;
        }
        if i + 1 < chars.len() && chars[i] == ']' && chars[i + 1] == ']' {
            return Some((i + 2, url, text));
        }
    } else if chars[i] == ']' && i + 1 < chars.len() && chars[i + 1] == ']' {
        // No link text, use URL
        return Some((i + 2, url.clone(), url));
    }

    None
}

// ── Builder ───────────────────────────────────────────────────────────────────

/// Build a Muse string from a [`MuseDoc`].
pub fn build(doc: &MuseDoc) -> String {
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
            let level_capped = (*level as usize).min(5);
            for _ in 0..level_capped {
                ctx.write("*");
            }
            ctx.write(" ");
            build_inlines(inlines, ctx);
            ctx.write("\n\n");
        }

        Block::CodeBlock { content } => {
            ctx.write("<example>\n");
            ctx.write(content);
            if !content.ends_with('\n') {
                ctx.write("\n");
            }
            ctx.write("</example>\n\n");
        }

        Block::Blockquote { children } => {
            ctx.write("<quote>\n");
            for child in children {
                match child {
                    Block::Paragraph { inlines } => {
                        build_inlines(inlines, ctx);
                        ctx.write("\n");
                    }
                    other => build_block(other, ctx),
                }
            }
            ctx.write("</quote>\n\n");
        }

        Block::List { ordered, items } => {
            let mut num = 1;
            for item_blocks in items {
                if *ordered {
                    ctx.write(&format!(" {}. ", num));
                    num += 1;
                } else {
                    ctx.write(" - ");
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

        Block::DefinitionList { items } => {
            for (term_inlines, desc_blocks) in items {
                build_inlines(term_inlines, ctx);
                ctx.write(" :: ");
                for block in desc_blocks {
                    match block {
                        Block::Paragraph { inlines } => build_inlines(inlines, ctx),
                        other => build_block(other, ctx),
                    }
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
            ctx.write("*");
            build_inlines(children, ctx);
            ctx.write("*");
        }

        Inline::Code(s) => {
            ctx.write("=");
            ctx.write(s);
            ctx.write("=");
        }

        Inline::Link { url, children } => {
            ctx.write("[[");
            ctx.write(url);
            ctx.write("][");
            build_inlines(children, ctx);
            ctx.write("]]");
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_heading() {
        let doc = parse("* Title\n").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Heading { level: 1, .. }));
    }

    #[test]
    fn test_parse_heading_levels() {
        let doc = parse("** Level 2\n*** Level 3\n").unwrap();
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
        let doc = parse("**bold**\n").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert_eq!(inlines.len(), 1);
        assert!(matches!(inlines[0], Inline::Bold(_)));
    }

    #[test]
    fn test_parse_emphasis() {
        let doc = parse("text with *emphasis*\n").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|n| matches!(n, Inline::Italic(_))));
    }

    #[test]
    fn test_parse_code() {
        let doc = parse("=code=\n").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert_eq!(inlines.len(), 1);
        assert!(matches!(inlines[0], Inline::Code(_)));
    }

    #[test]
    fn test_parse_link() {
        let doc = parse("[[https://example.com][Example]]\n").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        let link = &inlines[0];
        assert!(matches!(link, Inline::Link { .. }));
    }

    #[test]
    fn test_parse_unordered_list() {
        let doc = parse(" - item1\n - item2\n").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::List { ordered: false, .. }));
        if let Block::List { items, .. } = &doc.blocks[0] {
            assert_eq!(items.len(), 2);
        }
    }

    #[test]
    fn test_parse_ordered_list() {
        let doc = parse(" 1. item1\n 2. item2\n").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::List { ordered: true, .. }));
        if let Block::List { items, .. } = &doc.blocks[0] {
            assert_eq!(items.len(), 2);
        }
    }

    #[test]
    fn test_parse_example_block() {
        let doc = parse("<example>\ncode here\n</example>\n").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::CodeBlock { .. }));
    }

    #[test]
    fn test_build_heading() {
        let doc = MuseDoc {
            blocks: vec![Block::Heading {
                level: 1,
                inlines: vec![Inline::Text("Title".into())],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("* Title"));
    }

    #[test]
    fn test_build_paragraph() {
        let doc = MuseDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Hello, world!".into())],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("Hello, world!"));
    }

    #[test]
    fn test_build_bold() {
        let doc = MuseDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Bold(vec![Inline::Text("bold".into())])],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("**bold**"));
    }

    #[test]
    fn test_build_emphasis() {
        let doc = MuseDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Italic(vec![Inline::Text("emphasis".into())])],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("*emphasis*"));
    }

    #[test]
    fn test_build_code() {
        let doc = MuseDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Code("code".into())],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("=code="));
    }

    #[test]
    fn test_build_link() {
        let doc = MuseDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "https://example.com".into(),
                    children: vec![Inline::Text("click".into())],
                }],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("[[https://example.com][click]]"));
    }

    #[test]
    fn test_build_unordered_list() {
        let doc = MuseDoc {
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
        let out = build(&doc);
        assert!(out.contains(" - one"));
        assert!(out.contains(" - two"));
    }

    #[test]
    fn test_build_ordered_list() {
        let doc = MuseDoc {
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
        let out = build(&doc);
        assert!(out.contains(" 1. first"));
        assert!(out.contains(" 2. second"));
    }

    #[test]
    fn test_build_code_block() {
        let doc = MuseDoc {
            blocks: vec![Block::CodeBlock {
                content: "print hi".into(),
            }],
        };
        let out = build(&doc);
        assert!(out.contains("<example>"));
        assert!(out.contains("print hi"));
        assert!(out.contains("</example>"));
    }
}
