//! TWiki format parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-twiki` and `rescribe-write-twiki` as thin adapter layers.

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct TwikiError(pub String);

impl std::fmt::Display for TwikiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TWiki error: {}", self.0)
    }
}

impl std::error::Error for TwikiError {}

// ── AST ───────────────────────────────────────────────────────────────────────

/// A parsed TWiki document.
#[derive(Debug, Clone, Default)]
pub struct TwikiDoc {
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
    pub cells: Vec<TableCell>,
}

/// A table cell.
#[derive(Debug, Clone)]
pub struct TableCell {
    pub inlines: Vec<Inline>,
    pub is_header: bool,
}

/// Inline element.
#[derive(Debug, Clone)]
pub enum Inline {
    Text(String),
    Bold(Vec<Inline>),
    Italic(Vec<Inline>),
    BoldItalic(Vec<Inline>),
    Code(String),
    BoldCode(Vec<Inline>),
    Link { url: String, label: String },
    LineBreak,
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Parse a TWiki string into a [`TwikiDoc`].
pub fn parse(input: &str) -> Result<TwikiDoc, TwikiError> {
    let mut result = Vec::new();
    let lines: Vec<&str> = input.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // Headings: ---+ ---++ ---+++ etc
        if line.starts_with("---+") {
            let level = line.chars().skip(3).take_while(|&c| c == '+').count();
            let text = line.trim_start_matches('-').trim_start_matches('+').trim();
            result.push(Block::Heading {
                level: level.min(6) as u8,
                inlines: parse_inline(text),
            });
            i += 1;
            continue;
        }

        // Horizontal rule: ---
        if line.trim() == "---" {
            result.push(Block::HorizontalRule);
            i += 1;
            continue;
        }

        // Verbatim block
        if line.trim().starts_with("<verbatim>") {
            let (code_node, end) = parse_verbatim(&lines, i);
            result.push(code_node);
            i = end;
            continue;
        }

        // Table: |cell|cell|
        if line.trim().starts_with('|') && line.trim().ends_with('|') {
            let (table_node, end) = parse_table(&lines, i);
            result.push(table_node);
            i = end;
            continue;
        }

        // Lists (indented with spaces)
        if line.starts_with("   ")
            && (line.trim().starts_with('*') || line.trim().starts_with("1."))
        {
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
        // Ensure progress even when collect_paragraph returns without consuming anything
        // (e.g. definition-list lines "   $ item:" that don't match list syntax).
        i = end.max(i + 1);
    }

    Ok(TwikiDoc { blocks: result })
}

fn collect_paragraph<'a>(lines: &[&'a str], start: usize) -> (Vec<&'a str>, usize) {
    let mut para_lines = Vec::new();
    let mut i = start;

    while i < lines.len() {
        let line = lines[i];
        if line.trim().is_empty()
            || line.starts_with("---")
            || line.starts_with("   ")
            || line.trim().starts_with('|')
            || line.trim().starts_with("<verbatim>")
        {
            break;
        }
        para_lines.push(line.trim());
        i += 1;
    }

    (para_lines, i)
}

fn parse_verbatim(lines: &[&str], start: usize) -> (Block, usize) {
    let mut code_lines = Vec::new();
    let mut i = start;

    // Handle single-line verbatim
    let first_line = lines[start].trim();
    if first_line.contains("</verbatim>") {
        let content = first_line
            .strip_prefix("<verbatim>")
            .unwrap_or(first_line)
            .strip_suffix("</verbatim>")
            .unwrap_or(first_line);
        return (
            Block::CodeBlock {
                content: content.to_string(),
            },
            start + 1,
        );
    }

    // Skip opening tag
    let after_tag = first_line
        .strip_prefix("<verbatim>")
        .unwrap_or("")
        .to_string();
    if !after_tag.is_empty() {
        code_lines.push(after_tag);
    }
    i += 1;

    while i < lines.len() {
        let line = lines[i];
        if line.contains("</verbatim>") {
            let before = line.split("</verbatim>").next().unwrap_or("");
            if !before.is_empty() {
                code_lines.push(before.to_string());
            }
            return (
                Block::CodeBlock {
                    content: code_lines.join("\n"),
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
        },
        i,
    )
}

fn parse_table(lines: &[&str], start: usize) -> (Block, usize) {
    let mut rows = Vec::new();
    let mut i = start;

    while i < lines.len() {
        let line = lines[i].trim();
        if !line.starts_with('|') || !line.ends_with('|') {
            break;
        }

        let inner = &line[1..line.len() - 1];
        let cells: Vec<TableCell> = inner
            .split('|')
            .map(|cell| {
                let cell = cell.trim();
                // Header cells start and end with *
                let is_header = cell.starts_with('*') && cell.ends_with('*');
                let content = if is_header {
                    &cell[1..cell.len() - 1]
                } else {
                    cell
                };
                TableCell {
                    inlines: parse_inline(content),
                    is_header,
                }
            })
            .collect();

        rows.push(TableRow { cells });
        i += 1;
    }

    (Block::Table { rows }, i)
}

fn parse_list(lines: &[&str], start: usize) -> (Block, usize) {
    let first_line = lines[start].trim();
    let ordered = first_line.starts_with("1.");
    let mut items = Vec::new();
    let mut i = start;

    while i < lines.len() {
        let line = lines[i];
        if !line.starts_with("   ") {
            break;
        }

        let trimmed = line.trim();
        let text = if ordered {
            trimmed.strip_prefix("1.").unwrap_or(trimmed).trim()
        } else {
            trimmed.strip_prefix('*').unwrap_or(trimmed).trim()
        };

        if trimmed.starts_with('*') || trimmed.starts_with("1.") {
            items.push(parse_inline(text));
        }
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
        // Bold italic: __text__
        if i + 1 < chars.len()
            && chars[i] == '_'
            && chars[i + 1] == '_'
            && let Some((content, end)) = find_delimited(&chars, i + 2, "__")
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone()));
                current.clear();
            }
            nodes.push(Inline::BoldItalic(parse_inline(&content)));
            i = end;
            continue;
        }

        // Bold: *text*
        if chars[i] == '*'
            && let Some((content, end)) = find_delimited(&chars, i + 1, "*")
            && !content.is_empty()
            && !content.starts_with(' ')
            && !content.ends_with(' ')
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone()));
                current.clear();
            }
            nodes.push(Inline::Bold(parse_inline(&content)));
            i = end;
            continue;
        }

        // Italic: _text_
        if chars[i] == '_'
            && let Some((content, end)) = find_delimited(&chars, i + 1, "_")
            && !content.is_empty()
            && !content.starts_with(' ')
            && !content.ends_with(' ')
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone()));
                current.clear();
            }
            nodes.push(Inline::Italic(parse_inline(&content)));
            i = end;
            continue;
        }

        // Bold fixed: ==text==
        if i + 1 < chars.len()
            && chars[i] == '='
            && chars[i + 1] == '='
            && let Some((content, end)) = find_delimited(&chars, i + 2, "==")
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone()));
                current.clear();
            }
            nodes.push(Inline::BoldCode(parse_inline(&content)));
            i = end;
            continue;
        }

        // Fixed: =text=
        if chars[i] == '='
            && let Some((content, end)) = find_delimited(&chars, i + 1, "=")
            && !content.is_empty()
            && !content.starts_with(' ')
            && !content.ends_with(' ')
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone()));
                current.clear();
            }
            nodes.push(Inline::Code(content));
            i = end;
            continue;
        }

        // Link: [[url][label]] or [[url]]
        if i + 1 < chars.len()
            && chars[i] == '['
            && chars[i + 1] == '['
            && let Some((url, label, end)) = parse_twiki_link(&chars, i + 2)
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

fn parse_twiki_link(chars: &[char], start: usize) -> Option<(String, String, usize)> {
    let mut url = String::new();
    let mut i = start;

    // Get URL part
    while i < chars.len() {
        if chars[i] == ']' {
            break;
        }
        url.push(chars[i]);
        i += 1;
    }

    if i >= chars.len() {
        return None;
    }

    // Check for [label] part
    if i + 1 < chars.len() && chars[i] == ']' && chars[i + 1] == '[' {
        let mut label = String::new();
        i += 2;
        while i < chars.len() && chars[i] != ']' {
            label.push(chars[i]);
            i += 1;
        }
        if i + 1 < chars.len() && chars[i] == ']' && chars[i + 1] == ']' {
            return Some((url, label, i + 2));
        }
    }

    // Just [[url]]
    if i + 1 < chars.len() && chars[i] == ']' && chars[i + 1] == ']' {
        let label = url.clone();
        return Some((url, label, i + 2));
    }

    None
}

// ── Builder ───────────────────────────────────────────────────────────────────

/// Build a TWiki string from a [`TwikiDoc`].
pub fn build(doc: &TwikiDoc) -> String {
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
            output.push_str("---");
            for _ in 0..(*level as usize).min(6) {
                output.push('+');
            }
            output.push(' ');
            build_inlines(inlines, output);
            output.push('\n');
        }

        Block::CodeBlock { content } => {
            output.push_str("<verbatim>\n");
            output.push_str(content);
            if !content.ends_with('\n') {
                output.push('\n');
            }
            output.push_str("</verbatim>\n\n");
        }

        Block::List { ordered, items } => {
            for item_inlines in items {
                output.push_str("   ");
                if *ordered {
                    output.push_str("1. ");
                } else {
                    output.push_str("* ");
                }
                build_inlines(item_inlines, output);
                output.push('\n');
            }
            output.push('\n');
        }

        Block::Table { rows } => {
            for row in rows {
                output.push('|');
                for cell in &row.cells {
                    output.push(' ');
                    if cell.is_header {
                        output.push('*');
                        build_inlines(&cell.inlines, output);
                        output.push('*');
                    } else {
                        build_inlines(&cell.inlines, output);
                    }
                    output.push_str(" |");
                }
                output.push('\n');
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
            output.push('*');
            build_inlines(children, output);
            output.push('*');
        }

        Inline::Italic(children) => {
            output.push('_');
            build_inlines(children, output);
            output.push('_');
        }

        Inline::BoldItalic(children) => {
            output.push_str("__");
            build_inlines(children, output);
            output.push_str("__");
        }

        Inline::Code(s) => {
            output.push('=');
            output.push_str(s);
            output.push('=');
        }

        Inline::BoldCode(children) => {
            output.push_str("==");
            build_inlines(children, output);
            output.push_str("==");
        }

        Inline::Link { url, label } => {
            output.push_str("[[");
            output.push_str(url);
            output.push_str("][");
            output.push_str(label);
            output.push_str("]]");
        }

        Inline::LineBreak => output.push_str("%BR%"),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_heading() {
        let result = parse("---+ Heading 1\n---++ Heading 2").unwrap();
        assert_eq!(result.blocks.len(), 2);
        assert!(matches!(result.blocks[0], Block::Heading { .. }));
    }

    #[test]
    fn test_parse_bold() {
        let result = parse("This is *bold* text").unwrap();
        assert_eq!(result.blocks.len(), 1);
        assert!(matches!(result.blocks[0], Block::Paragraph { .. }));
    }

    #[test]
    fn test_parse_italic() {
        let result = parse("This is _italic_ text").unwrap();
        assert_eq!(result.blocks.len(), 1);
        assert!(matches!(result.blocks[0], Block::Paragraph { .. }));
    }

    #[test]
    fn test_parse_code() {
        let result = parse("Use =code= here").unwrap();
        assert_eq!(result.blocks.len(), 1);
        assert!(matches!(result.blocks[0], Block::Paragraph { .. }));
    }

    #[test]
    fn test_parse_link() {
        let result = parse("Visit [[http://example.com][Example]]").unwrap();
        assert_eq!(result.blocks.len(), 1);
        assert!(matches!(result.blocks[0], Block::Paragraph { .. }));
    }

    #[test]
    fn test_parse_table() {
        let result = parse("| A | B |\n| C | D |").unwrap();
        assert_eq!(result.blocks.len(), 1);
        assert!(matches!(result.blocks[0], Block::Table { .. }));
    }

    #[test]
    fn test_parse_heading_level() {
        let result = parse("---+ Level 1\n---++ Level 2\n---+++ Level 3").unwrap();
        assert_eq!(result.blocks.len(), 3);
        if let Block::Heading { level, .. } = &result.blocks[0] {
            assert_eq!(*level, 1);
        } else {
            panic!("expected heading");
        }
        if let Block::Heading { level, .. } = &result.blocks[1] {
            assert_eq!(*level, 2);
        } else {
            panic!("expected heading");
        }
        if let Block::Heading { level, .. } = &result.blocks[2] {
            assert_eq!(*level, 3);
        } else {
            panic!("expected heading");
        }
    }

    #[test]
    fn test_build_heading() {
        let doc = TwikiDoc {
            blocks: vec![Block::Heading {
                level: 1,
                inlines: vec![Inline::Text("Title".into())],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("---+ Title"));
    }

    #[test]
    fn test_build_bold() {
        let doc = TwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Bold(vec![Inline::Text("bold".into())])],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("*bold*"));
    }

    #[test]
    fn test_build_italic() {
        let doc = TwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Italic(vec![Inline::Text("italic".into())])],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("_italic_"));
    }

    #[test]
    fn test_build_link() {
        let doc = TwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "http://example.com".into(),
                    label: "Example".into(),
                }],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("[[http://example.com][Example]]"));
    }

    #[test]
    fn test_build_list() {
        let doc = TwikiDoc {
            blocks: vec![Block::List {
                ordered: false,
                items: vec![
                    vec![Inline::Text("one".into())],
                    vec![Inline::Text("two".into())],
                ],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("   * one"));
        assert!(out.contains("   * two"));
    }

    #[test]
    fn test_build_paragraph() {
        let doc = TwikiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Hello, world!".into())],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("Hello, world!"));
    }

    #[test]
    fn test_roundtrip_simple() {
        let original = "---+ Hello\n\nThis is a paragraph.\n\n";
        let doc = parse(original).unwrap();
        let rebuilt = build(&doc);
        let doc2 = parse(&rebuilt).unwrap();
        assert_eq!(doc.blocks.len(), doc2.blocks.len());
    }
}
