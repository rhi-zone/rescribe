//! BBCode parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-bbcode` and `rescribe-write-bbcode` as thin adapter layers.

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct BbcodeError(pub String);

impl std::fmt::Display for BbcodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BBCode error: {}", self.0)
    }
}

impl std::error::Error for BbcodeError {}

// ── AST ───────────────────────────────────────────────────────────────────────

/// A parsed BBCode document.
#[derive(Debug, Clone, Default)]
pub struct BbcodeDoc {
    pub blocks: Vec<Block>,
}

/// Block-level element.
#[derive(Debug, Clone)]
pub enum Block {
    Paragraph {
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
        items: Vec<Vec<Inline>>,
    },
    Table {
        rows: Vec<TableRow>,
    },
}

/// A table row.
#[derive(Debug, Clone)]
pub struct TableRow {
    pub cells: Vec<(bool, Vec<Inline>)>, // (is_header, inlines)
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
    Link {
        url: String,
        children: Vec<Inline>,
    },
    Image {
        url: String,
    },
    Subscript(Vec<Inline>),
    Superscript(Vec<Inline>),
    Span {
        attr: String,
        value: String,
        children: Vec<Inline>,
    },
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Parse a BBCode string into a [`BbcodeDoc`].
pub fn parse(input: &str) -> Result<BbcodeDoc, BbcodeError> {
    let mut result = Vec::new();
    let lines: Vec<&str> = input.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // Code block: [code]...[/code]
        if line.trim().to_lowercase().starts_with("[code]") {
            let (block, end) = parse_code_block(&lines, i);
            result.push(block);
            i = end;
            continue;
        }

        // Quote: [quote]...[/quote]
        if line.trim().to_lowercase().starts_with("[quote") {
            let (block, end) = parse_quote(&lines, i);
            result.push(block);
            i = end;
            continue;
        }

        // List: [list]...[/list]
        if line.trim().to_lowercase().starts_with("[list") {
            let (block, end) = parse_list(&lines, i);
            result.push(block);
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

    Ok(BbcodeDoc { blocks: result })
}

fn collect_paragraph<'a>(lines: &[&'a str], start: usize) -> (Vec<&'a str>, usize) {
    let mut para_lines = Vec::new();
    let mut i = start;

    while i < lines.len() {
        let line = lines[i];
        let lower = line.trim().to_lowercase();
        if line.trim().is_empty()
            || lower.starts_with("[code")
            || lower.starts_with("[quote")
            || lower.starts_with("[list")
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
    let mut code_lines = Vec::new();
    let mut i = start;

    // Single line code block
    if first_line.to_lowercase().contains("[/code]") {
        let content = first_line
            .strip_prefix("[code]")
            .or_else(|| first_line.strip_prefix("[CODE]"))
            .unwrap_or(first_line)
            .strip_suffix("[/code]")
            .or_else(|| first_line.strip_suffix("[/CODE]"))
            .unwrap_or(first_line);
        return (
            Block::CodeBlock {
                content: content.to_string(),
            },
            start + 1,
        );
    }

    // Multi-line
    let after_tag = first_line
        .strip_prefix("[code]")
        .or_else(|| first_line.strip_prefix("[CODE]"))
        .unwrap_or("");
    if !after_tag.is_empty() {
        code_lines.push(after_tag.to_string());
    }
    i += 1;

    while i < lines.len() {
        let line = lines[i];
        let lower = line.to_lowercase();
        if lower.contains("[/code]") {
            let before = line.split("[/code]").next().unwrap_or("");
            let before = before.split("[/CODE]").next().unwrap_or(before);
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

fn parse_quote(lines: &[&str], start: usize) -> (Block, usize) {
    let mut quote_lines = Vec::new();
    let mut i = start + 1;

    while i < lines.len() {
        let line = lines[i];
        if line.to_lowercase().contains("[/quote]") {
            let text = quote_lines.join(" ");
            return (
                Block::Blockquote {
                    children: vec![Block::Paragraph {
                        inlines: parse_inline(&text),
                    }],
                },
                i + 1,
            );
        }
        if !line.trim().is_empty() {
            quote_lines.push(line.trim());
        }
        i += 1;
    }

    let text = quote_lines.join(" ");
    (
        Block::Blockquote {
            children: vec![Block::Paragraph {
                inlines: parse_inline(&text),
            }],
        },
        i,
    )
}

fn parse_list(lines: &[&str], start: usize) -> (Block, usize) {
    let first_line = lines[start].trim().to_lowercase();
    let ordered = first_line.contains("[list=1]") || first_line.contains("[list=a]");
    let mut items = Vec::new();
    let mut i = start + 1;

    while i < lines.len() {
        let line = lines[i];
        let lower = line.to_lowercase();
        if lower.contains("[/list]") {
            return (Block::List { ordered, items }, i + 1);
        }
        if lower.trim().starts_with("[*]") {
            let text = line
                .trim()
                .strip_prefix("[*]")
                .or_else(|| line.trim().strip_prefix("[*]"))
                .unwrap_or(line)
                .trim();
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
        if chars[i] == '['
            && let Some((tag, content, end)) = parse_bbcode_tag(&chars, i)
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone()));
                current.clear();
            }

            let node = match tag.to_lowercase().as_str() {
                "b" => Some(Inline::Bold(parse_inline(&content))),
                "i" => Some(Inline::Italic(parse_inline(&content))),
                "u" => Some(Inline::Underline(parse_inline(&content))),
                "s" | "strike" => Some(Inline::Strikethrough(parse_inline(&content))),
                "code" => Some(Inline::Code(content)),
                _ if tag.to_lowercase().starts_with("url=") => {
                    let url = tag[4..].to_string();
                    Some(Inline::Link {
                        url,
                        children: parse_inline(&content),
                    })
                }
                "url" => Some(Inline::Link {
                    url: content.clone(),
                    children: parse_inline(&content),
                }),
                _ if tag.to_lowercase().starts_with("img") => Some(Inline::Image { url: content }),
                "sub" => Some(Inline::Subscript(parse_inline(&content))),
                "sup" => Some(Inline::Superscript(parse_inline(&content))),
                _ if tag.to_lowercase().starts_with("color=") => {
                    let value = tag[6..].to_string();
                    Some(Inline::Span {
                        attr: "color".to_string(),
                        value,
                        children: parse_inline(&content),
                    })
                }
                _ if tag.to_lowercase().starts_with("size=") => {
                    let value = tag[5..].to_string();
                    Some(Inline::Span {
                        attr: "size".to_string(),
                        value,
                        children: parse_inline(&content),
                    })
                }
                _ => None,
            };

            if let Some(n) = node {
                nodes.push(n);
                i = end;
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

fn parse_bbcode_tag(chars: &[char], start: usize) -> Option<(String, String, usize)> {
    if chars[start] != '[' {
        return None;
    }

    // Find tag name
    let mut tag = String::new();
    let mut i = start + 1;
    while i < chars.len() && chars[i] != ']' {
        tag.push(chars[i]);
        i += 1;
    }
    if i >= chars.len() {
        return None;
    }
    i += 1; // Skip ]

    // Find closing tag
    let close_tag = format!("[/{}]", tag.split('=').next().unwrap_or(&tag));
    let close_lower = close_tag.to_lowercase();

    let content_start = i;

    while i < chars.len() {
        // Check for closing tag
        let remaining: String = chars[i..].iter().collect();
        if remaining.to_lowercase().starts_with(&close_lower) {
            let content: String = chars[content_start..i].iter().collect();
            return Some((tag, content, i + close_tag.len()));
        }
        i += 1;
    }

    None
}

// ── Builder ───────────────────────────────────────────────────────────────────

/// Build a BBCode string from a [`BbcodeDoc`].
pub fn build(doc: &BbcodeDoc) -> String {
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

        Block::CodeBlock { content } => {
            output.push_str("[code]\n");
            output.push_str(content);
            if !content.ends_with('\n') {
                output.push('\n');
            }
            output.push_str("[/code]\n\n");
        }

        Block::Blockquote { children } => {
            output.push_str("[quote]\n");
            for child in children {
                if let Block::Paragraph { inlines } = child {
                    build_inlines(inlines, output);
                    output.push('\n');
                } else {
                    build_block(child, output);
                }
            }
            output.push_str("[/quote]\n\n");
        }

        Block::List { ordered, items } => {
            if *ordered {
                output.push_str("[list=1]\n");
            } else {
                output.push_str("[list]\n");
            }

            for item_inlines in items {
                output.push_str("[*]");
                build_inlines(item_inlines, output);
                output.push('\n');
            }

            output.push_str("[/list]\n\n");
        }

        Block::Table { rows } => {
            output.push_str("[table]\n");
            for row in rows {
                output.push_str("[tr]");
                for (is_header, inlines) in &row.cells {
                    let tag = if *is_header { "th" } else { "td" };
                    output.push_str(&format!("[{}]", tag));
                    build_inlines(inlines, output);
                    output.push_str(&format!("[/{}]", tag));
                }
                output.push_str("[/tr]\n");
            }
            output.push_str("[/table]\n\n");
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
            output.push_str("[b]");
            build_inlines(children, output);
            output.push_str("[/b]");
        }

        Inline::Italic(children) => {
            output.push_str("[i]");
            build_inlines(children, output);
            output.push_str("[/i]");
        }

        Inline::Underline(children) => {
            output.push_str("[u]");
            build_inlines(children, output);
            output.push_str("[/u]");
        }

        Inline::Strikethrough(children) => {
            output.push_str("[s]");
            build_inlines(children, output);
            output.push_str("[/s]");
        }

        Inline::Code(s) => {
            output.push_str("[code]");
            output.push_str(s);
            output.push_str("[/code]");
        }

        Inline::Link { url, children } => {
            output.push_str(&format!("[url={}]", url));
            build_inlines(children, output);
            output.push_str("[/url]");
        }

        Inline::Image { url } => {
            output.push_str("[img]");
            output.push_str(url);
            output.push_str("[/img]");
        }

        Inline::Subscript(children) => {
            output.push_str("[sub]");
            build_inlines(children, output);
            output.push_str("[/sub]");
        }

        Inline::Superscript(children) => {
            output.push_str("[sup]");
            build_inlines(children, output);
            output.push_str("[/sup]");
        }

        Inline::Span {
            attr,
            value,
            children,
        } => {
            output.push_str(&format!("[{}={}]", attr, value));
            build_inlines(children, output);
            output.push_str(&format!("[/{}]", attr));
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bold() {
        let doc = parse("This is [b]bold[/b] text").unwrap();
        assert!(!doc.blocks.is_empty());
    }

    #[test]
    fn test_parse_italic() {
        let doc = parse("This is [i]italic[/i] text").unwrap();
        assert!(!doc.blocks.is_empty());
    }

    #[test]
    fn test_parse_link() {
        let doc = parse("[url=http://example.com]Example[/url]").unwrap();
        assert!(!doc.blocks.is_empty());
    }

    #[test]
    fn test_parse_list() {
        let doc = parse("[list]\n[*]Item 1\n[*]Item 2\n[/list]").unwrap();
        assert!(!doc.blocks.is_empty());
    }

    #[test]
    fn test_parse_code() {
        let doc = parse("[code]print('hello')[/code]").unwrap();
        assert!(!doc.blocks.is_empty());
    }

    #[test]
    fn test_build_paragraph() {
        let doc = BbcodeDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Hello, world!".into())],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("Hello, world!"));
    }

    #[test]
    fn test_build_bold() {
        let doc = BbcodeDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Bold(vec![Inline::Text("bold".into())])],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("[b]bold[/b]"));
    }

    #[test]
    fn test_build_link() {
        let doc = BbcodeDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "http://example.com".into(),
                    children: vec![Inline::Text("Example".into())],
                }],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("[url=http://example.com]"));
        assert!(out.contains("Example"));
        assert!(out.contains("[/url]"));
    }

    #[test]
    fn test_build_code_block() {
        let doc = BbcodeDoc {
            blocks: vec![Block::CodeBlock {
                content: "print('hello')".into(),
            }],
        };
        let out = build(&doc);
        assert!(out.contains("[code]"));
        assert!(out.contains("print('hello')"));
        assert!(out.contains("[/code]"));
    }

    #[test]
    fn test_build_list() {
        let doc = BbcodeDoc {
            blocks: vec![Block::List {
                ordered: false,
                items: vec![
                    vec![Inline::Text("Item 1".into())],
                    vec![Inline::Text("Item 2".into())],
                ],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("[list]"));
        assert!(out.contains("[*]"));
        assert!(out.contains("[/list]"));
    }

    #[test]
    fn test_roundtrip_bold() {
        let input = "Text with [b]bold[/b] word";
        let doc = parse(input).unwrap();
        let output = build(&doc);
        assert!(output.contains("[b]"));
        assert!(output.contains("bold"));
        assert!(output.contains("[/b]"));
    }
}
