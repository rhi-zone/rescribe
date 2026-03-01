//! TikiWiki parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-tikiwiki` and `rescribe-write-tikiwiki` as thin adapter layers.

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct TikiwikiError(pub String);

impl std::fmt::Display for TikiwikiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TikiWiki error: {}", self.0)
    }
}

impl std::error::Error for TikiwikiError {}

// ── AST ───────────────────────────────────────────────────────────────────────

/// A parsed TikiWiki document.
#[derive(Debug, Clone, Default)]
pub struct TikiwikiDoc {
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
        inlines: Vec<Inline>,
    },
    List {
        ordered: bool,
        items: Vec<Vec<Inline>>,
    },
    Table {
        rows: Vec<TableRow>,
    },
    HorizontalRule,
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
    Code(String),
    Link { url: String, children: Vec<Inline> },
    Image { url: String, alt: String },
    LineBreak,
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Parse a TikiWiki string into a [`TikiwikiDoc`].
pub fn parse(input: &str) -> Result<TikiwikiDoc, TikiwikiError> {
    let mut result = Vec::new();
    let lines: Vec<&str> = input.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // Headings: ! !! !!! etc
        if let Some(rest) = line.strip_prefix('!') {
            let level = 1 + rest.chars().take_while(|&c| c == '!').count();
            let text = rest.trim_start_matches('!').trim();
            result.push(Block::Heading {
                level: level.min(6) as u8,
                inlines: parse_inline(text),
            });
            i += 1;
            continue;
        }

        // Horizontal rule
        if line.trim() == "---" {
            result.push(Block::HorizontalRule);
            i += 1;
            continue;
        }

        // Code block: {CODE()}...{CODE}
        if line.trim().starts_with("{CODE") {
            let (code_node, end) = parse_code_block(&lines, i);
            result.push(code_node);
            i = end;
            continue;
        }

        // Table: ||cell|cell||
        if line.trim().starts_with("||") {
            let (table_node, end) = parse_table(&lines, i);
            result.push(table_node);
            i = end;
            continue;
        }

        // Lists
        if line.starts_with('*') || line.starts_with('#') {
            let (list_node, end) = parse_list(&lines, i);
            result.push(list_node);
            i = end;
            continue;
        }

        // Empty line
        if line.trim().is_empty() {
            i += 1;
            continue;
        }

        // Regular paragraph
        let (para_lines, end) = collect_paragraph(&lines, i);
        if !para_lines.is_empty() {
            let text = para_lines.join(" ");
            result.push(Block::Paragraph {
                inlines: parse_inline(&text),
            });
        }
        i = end;
    }

    Ok(TikiwikiDoc { blocks: result })
}

fn collect_paragraph<'a>(lines: &[&'a str], start: usize) -> (Vec<&'a str>, usize) {
    let mut para_lines = Vec::new();
    let mut i = start;

    while i < lines.len() {
        let line = lines[i];
        if line.trim().is_empty()
            || line.starts_with('!')
            || line.starts_with('*')
            || line.starts_with('#')
            || line.trim().starts_with("||")
            || line.trim().starts_with("{CODE")
            || line.trim() == "---"
        {
            break;
        }
        para_lines.push(line.trim());
        i += 1;
    }

    (para_lines, i)
}

fn parse_code_block(lines: &[&str], start: usize) -> (Block, usize) {
    let first_line = lines[start].trim();

    // Extract language if present: {CODE(lang=python)}
    let lang = if let Some(paren_start) = first_line.find('(') {
        if let Some(paren_end) = first_line.find(')') {
            let params = &first_line[paren_start + 1..paren_end];
            params
                .split(',')
                .find_map(|p| {
                    p.strip_prefix("lang=")
                        .or_else(|| p.strip_prefix("language="))
                })
                .map(|s| s.trim().to_string())
        } else {
            None
        }
    } else {
        None
    };

    let mut code_lines = Vec::new();
    let mut i = start + 1;

    while i < lines.len() {
        let line = lines[i];
        if line.trim() == "{CODE}" || line.trim().starts_with("{CODE}") {
            return (
                Block::CodeBlock {
                    content: code_lines.join("\n"),
                    language: lang,
                },
                i + 1,
            );
        }
        code_lines.push(line);
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

fn parse_table(lines: &[&str], start: usize) -> (Block, usize) {
    let mut rows = Vec::new();
    let mut i = start;

    while i < lines.len() {
        let line = lines[i].trim();
        if !line.starts_with("||") {
            break;
        }

        // Parse row: ||cell|cell||
        let inner = line.trim_start_matches("||").trim_end_matches("||");
        let cells: Vec<Vec<Inline>> = inner
            .split('|')
            .map(|cell| parse_inline(cell.trim()))
            .collect();

        rows.push(TableRow { cells });
        i += 1;
    }

    (Block::Table { rows }, i)
}

fn parse_list(lines: &[&str], start: usize) -> (Block, usize) {
    let first_char = lines[start].chars().next().unwrap_or(' ');
    let ordered = first_char == '#';
    let mut items = Vec::new();
    let mut i = start;

    while i < lines.len() {
        let line = lines[i];
        let marker = if ordered { '#' } else { '*' };

        if !line.starts_with(marker) {
            break;
        }

        let text = line.trim_start_matches(marker).trim();
        items.push(parse_inline(text));
        i += 1;
    }

    (Block::List { ordered, items }, i)
}

fn parse_inline(text: &str) -> Vec<Inline> {
    let mut nodes = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Bold: __text__
        if i + 1 < chars.len()
            && chars[i] == '_'
            && chars[i + 1] == '_'
            && let Some((content, end)) = find_delimited(&chars, i + 2, "__")
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone()));
                current.clear();
            }
            nodes.push(Inline::Bold(parse_inline(&content)));
            i = end;
            continue;
        }

        // Italic: ''text''
        if i + 1 < chars.len()
            && chars[i] == '\''
            && chars[i + 1] == '\''
            && let Some((content, end)) = find_delimited(&chars, i + 2, "''")
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone()));
                current.clear();
            }
            nodes.push(Inline::Italic(parse_inline(&content)));
            i = end;
            continue;
        }

        // Underline: ===text===
        if i + 2 < chars.len()
            && chars[i] == '='
            && chars[i + 1] == '='
            && chars[i + 2] == '='
            && let Some((content, end)) = find_delimited(&chars, i + 3, "===")
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
            nodes.push(Inline::Strikethrough(parse_inline(&content)));
            i = end;
            continue;
        }

        // Inline code: -+text+-
        if i + 1 < chars.len()
            && chars[i] == '-'
            && chars[i + 1] == '+'
            && let Some((content, end)) = find_delimited(&chars, i + 2, "+-")
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone()));
                current.clear();
            }
            nodes.push(Inline::Code(content));
            i = end;
            continue;
        }

        // Link: [url|label] or [url]
        if chars[i] == '['
            && let Some((content, end)) = find_bracket_content(&chars, i + 1, '[', ']')
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone()));
                current.clear();
            }
            let parts: Vec<&str> = content.splitn(2, '|').collect();
            let url = parts[0].trim();
            let label = if parts.len() > 1 {
                parts[1].trim()
            } else {
                url
            };
            nodes.push(Inline::Link {
                url: url.to_string(),
                children: vec![Inline::Text(label.to_string())],
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

fn find_bracket_content(
    chars: &[char],
    start: usize,
    _open: char,
    close: char,
) -> Option<(String, usize)> {
    let mut i = start;
    while i < chars.len() {
        if chars[i] == close {
            let content: String = chars[start..i].iter().collect();
            return Some((content, i + 1));
        }
        i += 1;
    }
    None
}

// ── Builder ───────────────────────────────────────────────────────────────────

/// Build a TikiWiki string from a [`TikiwikiDoc`].
pub fn build(doc: &TikiwikiDoc) -> String {
    let mut output = String::new();
    for block in &doc.blocks {
        build_block(block, &mut output);
    }
    output
}

fn build_block(block: &Block, output: &mut String) {
    match block {
        Block::Paragraph { inlines } => {
            build_inlines(inlines, output);
            output.push_str("\n\n");
        }

        Block::Heading { level, inlines } => {
            for _ in 0..(*level as usize).min(6) {
                output.push('!');
            }
            build_inlines(inlines, output);
            output.push('\n');
        }

        Block::CodeBlock { content, language } => {
            if let Some(lang) = language {
                output.push_str(&format!("{{CODE(lang={})}}\n", lang));
            } else {
                output.push_str("{CODE()}\n");
            }
            output.push_str(content);
            if !content.ends_with('\n') {
                output.push('\n');
            }
            output.push_str("{CODE}\n\n");
        }

        Block::Blockquote { inlines } => {
            output.push('^');
            build_inlines(inlines, output);
            output.push_str("^\n\n");
        }

        Block::List { ordered, items } => {
            let marker = if *ordered { '#' } else { '*' };
            for item_inlines in items {
                output.push(marker);
                build_inlines(item_inlines, output);
                output.push('\n');
            }
            output.push('\n');
        }

        Block::Table { rows } => {
            for row in rows {
                output.push_str("||");
                for (i, cell) in row.cells.iter().enumerate() {
                    if i > 0 {
                        output.push('|');
                    }
                    build_inlines(cell, output);
                }
                output.push_str("||\n");
            }
            output.push('\n');
        }

        Block::HorizontalRule => {
            output.push_str("---\n\n");
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
            output.push_str("__");
            build_inlines(children, output);
            output.push_str("__");
        }

        Inline::Italic(children) => {
            output.push_str("''");
            build_inlines(children, output);
            output.push_str("''");
        }

        Inline::Underline(children) => {
            output.push_str("===");
            build_inlines(children, output);
            output.push_str("===");
        }

        Inline::Strikethrough(children) => {
            output.push_str("--");
            build_inlines(children, output);
            output.push_str("--");
        }

        Inline::Code(s) => {
            output.push_str("-+");
            output.push_str(s);
            output.push_str("+-");
        }

        Inline::Link { url, children } => {
            output.push('[');
            output.push_str(url);
            if !children.is_empty() {
                output.push('|');
                build_inlines(children, output);
            }
            output.push(']');
        }

        Inline::Image { url, alt } => {
            output.push_str("{img src=\"");
            output.push_str(url);
            if !alt.is_empty() {
                output.push_str("\" alt=\"");
                output.push_str(alt);
            }
            output.push_str("\"}");
        }

        Inline::LineBreak => output.push_str("%%%"),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_heading() {
        let doc = parse("!Heading 1\n!!Heading 2").unwrap();
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_parse_bold() {
        let doc = parse("This is __bold__ text").unwrap();
        assert_eq!(doc.blocks.len(), 1);
    }

    #[test]
    fn test_parse_italic() {
        let doc = parse("This is ''italic'' text").unwrap();
        assert_eq!(doc.blocks.len(), 1);
    }

    #[test]
    fn test_parse_link() {
        let doc = parse("[http://example.com|Example]").unwrap();
        assert_eq!(doc.blocks.len(), 1);
    }

    #[test]
    fn test_parse_list() {
        let doc = parse("*Item 1\n*Item 2").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        if let Block::List { .. } = &doc.blocks[0] {
            // OK
        } else {
            panic!("Expected list block");
        }
    }

    #[test]
    fn test_parse_table() {
        let doc = parse("||A|B||\n||C|D||").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        if let Block::Table { rows } = &doc.blocks[0] {
            assert_eq!(rows.len(), 2);
        } else {
            panic!("Expected table block");
        }
    }

    #[test]
    fn test_build_heading() {
        let doc = TikiwikiDoc {
            blocks: vec![Block::Heading {
                level: 1,
                inlines: vec![Inline::Text("Title".into())],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("!Title"));
    }

    #[test]
    fn test_build_bold() {
        let doc = TikiwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Bold(vec![Inline::Text("bold".into())])],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("__bold__"));
    }

    #[test]
    fn test_build_italic() {
        let doc = TikiwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Italic(vec![Inline::Text("italic".into())])],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("''italic''"));
    }

    #[test]
    fn test_build_link() {
        let doc = TikiwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "http://example.com".into(),
                    children: vec![Inline::Text("Example".into())],
                }],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("[http://example.com|Example]"));
    }

    #[test]
    fn test_build_code_block() {
        let doc = TikiwikiDoc {
            blocks: vec![Block::CodeBlock {
                content: "let x = 5;".into(),
                language: Some("rust".into()),
            }],
        };
        let out = build(&doc);
        assert!(out.contains("{CODE(lang=rust)}"));
        assert!(out.contains("let x = 5;"));
    }

    #[test]
    fn test_roundtrip_simple() {
        let input = "!Heading\n\nParagraph text\n\n__bold__";
        let doc = parse(input).unwrap();
        let output = build(&doc);
        // Parse again to verify consistency
        let doc2 = parse(&output).unwrap();
        assert_eq!(doc.blocks.len(), doc2.blocks.len());
    }
}
