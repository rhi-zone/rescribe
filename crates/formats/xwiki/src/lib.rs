//! XWiki 2.0 format parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-xwiki` and `rescribe-write-xwiki` as thin adapter layers.

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct XwikiError(pub String);

impl std::fmt::Display for XwikiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "XWiki error: {}", self.0)
    }
}

impl std::error::Error for XwikiError {}

// ── AST ───────────────────────────────────────────────────────────────────────

/// A parsed XWiki document.
#[derive(Debug, Clone, Default)]
pub struct XwikiDoc {
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
        language: Option<String>,
    },
    Table {
        rows: Vec<TableRow>,
    },
    List {
        ordered: bool,
        items: Vec<Vec<Block>>,
    },
    HorizontalRule,
}

/// A table row.
#[derive(Debug, Clone)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
}

/// A table cell (can be header or regular).
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
    Strikeout(Vec<Inline>),
    Code(String),
    Link { url: String, label: String },
    Image { url: String },
    LineBreak,
    SoftBreak,
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Parse an XWiki string into an [`XwikiDoc`].
pub fn parse(input: &str) -> Result<XwikiDoc, XwikiError> {
    let lines: Vec<&str> = input.lines().collect();
    let mut parser = Parser::new(&lines);
    let blocks = parser.parse()?;
    Ok(XwikiDoc { blocks })
}

struct Parser<'a> {
    lines: &'a [&'a str],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(lines: &'a [&'a str]) -> Self {
        Self { lines, pos: 0 }
    }

    fn parse(&mut self) -> Result<Vec<Block>, XwikiError> {
        let mut result = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            // Headings: = text = or == text == etc
            if line.starts_with('=') && line.trim().ends_with('=') {
                let level = line.chars().take_while(|&c| c == '=').count();
                let text = line.trim().trim_matches('=').trim();
                result.push(Block::Heading {
                    level: level.min(6) as u8,
                    inlines: parse_inline(text),
                });
                self.pos += 1;
                continue;
            }

            // Horizontal rule: ----
            if line.trim() == "----" {
                result.push(Block::HorizontalRule);
                self.pos += 1;
                continue;
            }

            // Code block: {{code}}...{{/code}}
            if line.trim().starts_with("{{code") {
                let (code_block, end) = self.parse_code_block();
                result.push(code_block);
                self.pos = end;
                continue;
            }

            // Table
            if line.trim().starts_with('|') {
                let (table_block, end) = self.parse_table();
                result.push(table_block);
                self.pos = end;
                continue;
            }

            // Lists
            if line.starts_with('*') || line.starts_with("1.") {
                let (list_block, end) = self.parse_list();
                result.push(list_block);
                self.pos = end;
                continue;
            }

            // Empty line
            if line.trim().is_empty() {
                self.pos += 1;
                continue;
            }

            // Regular paragraph
            let (para_lines, end) = self.collect_paragraph();
            if !para_lines.is_empty() {
                let text = para_lines.join(" ");
                result.push(Block::Paragraph {
                    inlines: parse_inline(&text),
                });
            }
            self.pos = end;
        }

        Ok(result)
    }

    fn collect_paragraph(&self) -> (Vec<String>, usize) {
        let mut para_lines = Vec::new();
        let mut i = self.pos;

        while i < self.lines.len() {
            let line = self.lines[i];
            if line.trim().is_empty()
                || line.starts_with('=')
                || line.starts_with('*')
                || line.starts_with("1.")
                || line.trim().starts_with('|')
                || line.trim().starts_with("{{code")
                || line.trim() == "----"
            {
                break;
            }
            para_lines.push(line.trim().to_string());
            i += 1;
        }

        (para_lines, i)
    }

    fn parse_code_block(&mut self) -> (Block, usize) {
        let first_line = self.lines[self.pos].trim();

        // Extract language if present: {{code language="python"}}
        let lang = if let Some(lang_start) = first_line.find("language=\"") {
            let rest = &first_line[lang_start + 10..];
            rest.find('"').map(|end| rest[..end].to_string())
        } else {
            None
        };

        let mut code_lines = Vec::new();
        let mut i = self.pos + 1;

        while i < self.lines.len() {
            let line = self.lines[i];
            if line.trim() == "{{/code}}" || line.trim().contains("{{/code}}") {
                return (
                    Block::CodeBlock {
                        content: code_lines.join("\n"),
                        language: lang,
                    },
                    i + 1,
                );
            }
            code_lines.push(line.to_string());
            i += 1;
        }

        (
            Block::CodeBlock {
                content: code_lines.join("\n"),
                language: lang,
            },
            i,
        )
    }

    fn parse_table(&mut self) -> (Block, usize) {
        let mut rows = Vec::new();
        let mut i = self.pos;

        while i < self.lines.len() {
            let line = self.lines[i].trim();
            if !line.starts_with('|') {
                break;
            }

            let inner = line.trim_matches('|');
            let cells: Vec<TableCell> = inner
                .split('|')
                .map(|cell| {
                    let cell = cell.trim();
                    if cell.starts_with('=') {
                        TableCell {
                            is_header: true,
                            inlines: parse_inline(cell.trim_start_matches('=')),
                        }
                    } else {
                        TableCell {
                            is_header: false,
                            inlines: parse_inline(cell),
                        }
                    }
                })
                .collect();

            rows.push(TableRow { cells });
            i += 1;
        }

        (Block::Table { rows }, i)
    }

    fn parse_list(&mut self) -> (Block, usize) {
        let ordered = self.lines[self.pos].starts_with("1.");
        let mut items = Vec::new();
        let mut i = self.pos;

        while i < self.lines.len() {
            let line = self.lines[i];
            let marker = if ordered { "1." } else { "*" };

            if !line.starts_with(marker) {
                break;
            }

            let text = line.strip_prefix(marker).unwrap_or(line).trim();
            items.push(vec![Block::Paragraph {
                inlines: parse_inline(text),
            }]);
            i += 1;
        }

        (Block::List { ordered, items }, i)
    }
}

fn parse_inline(text: &str) -> Vec<Inline> {
    let mut nodes = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Bold: **text**
        if i + 1 < chars.len()
            && chars[i] == '*'
            && chars[i + 1] == '*'
            && let Some((content, end)) = find_delimited(&chars, i + 2, "**")
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone()));
                current.clear();
            }
            nodes.push(Inline::Bold(parse_inline(&content)));
            i = end;
            continue;
        }

        // Italic: //text//
        if i + 1 < chars.len()
            && chars[i] == '/'
            && chars[i + 1] == '/'
            && let Some((content, end)) = find_delimited(&chars, i + 2, "//")
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone()));
                current.clear();
            }
            nodes.push(Inline::Italic(parse_inline(&content)));
            i = end;
            continue;
        }

        // Underline: __text__
        if i + 1 < chars.len()
            && chars[i] == '_'
            && chars[i + 1] == '_'
            && let Some((content, end)) = find_delimited(&chars, i + 2, "__")
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone()));
                current.clear();
            }
            nodes.push(Inline::Underline(parse_inline(&content)));
            i = end;
            continue;
        }

        // Strikethrough: --text--
        if i + 1 < chars.len()
            && chars[i] == '-'
            && chars[i + 1] == '-'
            && let Some((content, end)) = find_delimited(&chars, i + 2, "--")
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone()));
                current.clear();
            }
            nodes.push(Inline::Strikeout(parse_inline(&content)));
            i = end;
            continue;
        }

        // Monospace: ##text##
        if i + 1 < chars.len()
            && chars[i] == '#'
            && chars[i + 1] == '#'
            && let Some((content, end)) = find_delimited(&chars, i + 2, "##")
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone()));
                current.clear();
            }
            nodes.push(Inline::Code(content));
            i = end;
            continue;
        }

        // Link: [[label>>url]] or [[url]]
        if i + 1 < chars.len()
            && chars[i] == '['
            && chars[i + 1] == '['
            && let Some((url, label, end)) = parse_xwiki_link(&chars, i + 2)
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone()));
                current.clear();
            }
            nodes.push(Inline::Link { url, label });
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

fn find_delimited(chars: &[char], start: usize, delim: &str) -> Option<(String, usize)> {
    let delim_chars: Vec<char> = delim.chars().collect();
    let mut i = start;

    while i + delim_chars.len() <= chars.len() {
        let mut matches = true;
        for (j, dc) in delim_chars.iter().enumerate() {
            if chars[i + j] != *dc {
                matches = false;
                break;
            }
        }
        if matches {
            let content: String = chars[start..i].iter().collect();
            return Some((content, i + delim_chars.len()));
        }
        i += 1;
    }
    None
}

fn parse_xwiki_link(chars: &[char], start: usize) -> Option<(String, String, usize)> {
    let mut content = String::new();
    let mut i = start;

    while i + 1 < chars.len() {
        if chars[i] == ']' && chars[i + 1] == ']' {
            // Parse content: "label>>url" or just "url"
            if let Some(sep) = content.find(">>") {
                let label = content[..sep].to_string();
                let url = content[sep + 2..].to_string();
                return Some((url, label, i + 2));
            } else {
                let url = content.clone();
                return Some((url.clone(), url, i + 2));
            }
        }
        content.push(chars[i]);
        i += 1;
    }
    None
}

// ── Builder ────────────────────────────────────────────────────────────────────

/// Build an XWiki string from an [`XwikiDoc`].
pub fn build(doc: &XwikiDoc) -> String {
    let mut output = String::new();
    for block in &doc.blocks {
        build_block(block, &mut output);
    }
    output
}

fn build_block(block: &Block, output: &mut String) {
    match block {
        Block::Heading { level, inlines } => {
            for _ in 0..*level {
                output.push('=');
            }
            output.push(' ');
            build_inlines(inlines, output);
            output.push(' ');
            for _ in 0..*level {
                output.push('=');
            }
            output.push('\n');
        }

        Block::Paragraph { inlines } => {
            build_inlines(inlines, output);
            output.push_str("\n\n");
        }

        Block::CodeBlock { content, language } => {
            if let Some(lang) = language {
                output.push_str(&format!("{{{{code language=\"{}\"}}}}\n", lang));
            } else {
                output.push_str("{{code}}\n");
            }
            output.push_str(content);
            if !content.ends_with('\n') {
                output.push('\n');
            }
            output.push_str("{{/code}}\n\n");
        }

        Block::Table { rows } => {
            for row in rows {
                output.push('|');
                for cell in &row.cells {
                    if cell.is_header {
                        output.push('=');
                    }
                    build_inlines(&cell.inlines, output);
                    output.push('|');
                }
                output.push('\n');
            }
            output.push('\n');
        }

        Block::List { ordered, items } => {
            for item_blocks in items {
                if *ordered {
                    output.push_str("1. ");
                } else {
                    output.push_str("* ");
                }
                for block in item_blocks {
                    match block {
                        Block::Paragraph { inlines } => build_inlines(inlines, output),
                        other => build_block(other, output),
                    }
                }
                output.push('\n');
            }
            output.push('\n');
        }

        Block::HorizontalRule => {
            output.push_str("----\n\n");
        }
    }
}

fn build_inlines(inlines: &[Inline], output: &mut String) {
    for inline in inlines {
        build_inline(inline, output);
    }
}

fn build_inline(inline: &Inline, output: &mut String) {
    match inline {
        Inline::Text(s) => output.push_str(s),

        Inline::Bold(children) => {
            output.push_str("**");
            build_inlines(children, output);
            output.push_str("**");
        }

        Inline::Italic(children) => {
            output.push_str("//");
            build_inlines(children, output);
            output.push_str("//");
        }

        Inline::Underline(children) => {
            output.push_str("__");
            build_inlines(children, output);
            output.push_str("__");
        }

        Inline::Strikeout(children) => {
            output.push_str("--");
            build_inlines(children, output);
            output.push_str("--");
        }

        Inline::Code(s) => {
            output.push_str("##");
            output.push_str(s);
            output.push_str("##");
        }

        Inline::Link { url, label } => {
            output.push_str("[[");
            output.push_str(label);
            output.push_str(">>");
            output.push_str(url);
            output.push_str("]]");
        }

        Inline::Image { url } => {
            output.push_str("[[image:");
            output.push_str(url);
            output.push_str("]]");
        }

        Inline::LineBreak => output.push('\n'),
        Inline::SoftBreak => output.push(' '),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_heading() {
        let result = parse("= Heading 1 =\n== Heading 2 ==").unwrap();
        assert_eq!(result.blocks.len(), 2);
    }

    #[test]
    fn test_parse_bold() {
        let result = parse("This is **bold** text").unwrap();
        assert!(!result.blocks.is_empty());
    }

    #[test]
    fn test_parse_italic() {
        let result = parse("This is //italic// text").unwrap();
        assert!(!result.blocks.is_empty());
    }

    #[test]
    fn test_parse_link() {
        let result = parse("[[Example>>http://example.com]]").unwrap();
        assert!(!result.blocks.is_empty());
    }

    #[test]
    fn test_parse_list() {
        let result = parse("* Item 1\n* Item 2").unwrap();
        assert_eq!(result.blocks.len(), 1);
    }

    #[test]
    fn test_parse_code_block() {
        let result = parse("{{code language=\"rust\"}}\nfn main() {}\n{{/code}}").unwrap();
        assert_eq!(result.blocks.len(), 1);
    }

    #[test]
    fn test_parse_table() {
        let result = parse("|=Header|Cell|").unwrap();
        assert_eq!(result.blocks.len(), 1);
    }

    #[test]
    fn test_build_heading() {
        let doc = XwikiDoc {
            blocks: vec![Block::Heading {
                level: 1,
                inlines: vec![Inline::Text("Title".into())],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("= Title ="));
    }

    #[test]
    fn test_build_bold() {
        let doc = XwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Bold(vec![Inline::Text("bold".into())])],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("**bold**"));
    }

    #[test]
    fn test_build_link() {
        let doc = XwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "http://example.com".into(),
                    label: "Example".into(),
                }],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("[[Example>>http://example.com]]"));
    }

    #[test]
    fn test_roundtrip_simple() {
        let input = "= Heading =\n\nSimple paragraph with **bold** text.";
        let doc = parse(input).unwrap();
        let output = build(&doc);
        let doc2 = parse(&output).unwrap();
        assert_eq!(doc.blocks.len(), doc2.blocks.len());
    }
}
