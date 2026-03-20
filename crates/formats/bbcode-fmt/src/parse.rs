//! BBCode parser — infallible, returns (BbcodeDoc, Vec<Diagnostic>).

use crate::ast::{BbcodeDoc, Block, Diagnostic, Inline, Span, TableRow};

/// Parse a BBCode string into a [`BbcodeDoc`].
///
/// Always succeeds — malformed markup is tolerated and may produce diagnostics.
pub fn parse(input: &str) -> (BbcodeDoc, Vec<Diagnostic>) {
    let diagnostics = Vec::new();
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

        // Table: [table]...[/table]
        if line.trim().to_lowercase().starts_with("[table]") {
            let (block, end) = parse_table(&lines, i);
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
                span: Span::NONE,
            });
        }
        // Always advance at least one line to avoid infinite loop when
        // collect_paragraph returns end == i (e.g. unrecognised block-like tag).
        i = end.max(i + 1);
    }

    (
        BbcodeDoc {
            blocks: result,
            span: Span::new(0, input.len()),
        },
        diagnostics,
    )
}

fn collect_paragraph<'a>(lines: &[&'a str], start: usize) -> (Vec<&'a str>, usize) {
    let mut para_lines = Vec::new();
    let mut i = start;

    while i < lines.len() {
        let line = lines[i];
        let lower = line.trim().to_lowercase();
        // Break on lines that the main loop handles as block starters.
        // Use the same exact prefixes as the main loop to avoid mismatch:
        // - "[code]" (with closing bracket — partial "[code" alone is not a block)
        // - "[quote" (with or without "=", so prefix match is correct)
        // - "[list" (with or without "=1]/[list=a]", prefix match is correct)
        // - "[table]" (exact)
        if line.trim().is_empty()
            || lower.starts_with("[code]")
            || lower.starts_with("[quote")
            || lower.starts_with("[list")
            || lower.starts_with("[table]")
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
                span: Span::NONE,
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
                    span: Span::NONE,
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
            span: Span::NONE,
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
                        span: Span::NONE,
                    }],
                    span: Span::NONE,
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
                span: Span::NONE,
            }],
            span: Span::NONE,
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
            return (
                Block::List {
                    ordered,
                    items,
                    span: Span::NONE,
                },
                i + 1,
            );
        }
        if lower.trim().starts_with("[*]") {
            let text = line.trim().strip_prefix("[*]").unwrap_or(line).trim();
            items.push(parse_inline(text));
        }
        i += 1;
    }

    (
        Block::List {
            ordered,
            items,
            span: Span::NONE,
        },
        i,
    )
}

fn parse_table(lines: &[&str], start: usize) -> (Block, usize) {
    let mut rows = Vec::new();
    let mut i = start + 1;

    while i < lines.len() {
        let line = lines[i];
        let lower = line.to_lowercase();
        if lower.contains("[/table]") {
            return (
                Block::Table {
                    rows,
                    span: Span::NONE,
                },
                i + 1,
            );
        }
        // Parse a row: [tr][td]...[/td][/tr] or [tr][th]...[/th][/tr]
        if lower.trim().starts_with("[tr]") {
            let cells = parse_table_row(line);
            rows.push(TableRow {
                cells,
                span: Span::NONE,
            });
        }
        i += 1;
    }

    (
        Block::Table {
            rows,
            span: Span::NONE,
        },
        i,
    )
}

fn parse_table_row(line: &str) -> Vec<(bool, Vec<Inline>)> {
    let mut cells = Vec::new();
    let lower = line.to_lowercase();
    let mut search = lower.as_str();
    let mut offset = 0usize;

    // Find [td] or [th] cells
    while !search.is_empty() {
        let (tag_pos, is_header, tag_len) = if let Some(pos) = search.find("[td]") {
            (pos, false, 4usize)
        } else if let Some(pos) = search.find("[th]") {
            (pos, true, 4usize)
        } else {
            break;
        };

        let close_tag = if is_header { "[/th]" } else { "[/td]" };
        let content_start = tag_pos + tag_len;
        if let Some(close_pos) = search[content_start..].find(close_tag) {
            let content = &line[offset + content_start..offset + content_start + close_pos];
            cells.push((is_header, parse_inline(content)));
            let next = content_start + close_pos + close_tag.len();
            offset += next;
            search = &search[next..];
        } else {
            break;
        }
    }
    cells
}

pub(crate) fn parse_inline(text: &str) -> Vec<Inline> {
    let mut nodes = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '['
            && let Some((tag, content, end)) = parse_bbcode_tag(&chars, i)
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }

            let node = match tag.to_lowercase().as_str() {
                "b" => Some(Inline::Bold(parse_inline(&content), Span::NONE)),
                "i" => Some(Inline::Italic(parse_inline(&content), Span::NONE)),
                "u" => Some(Inline::Underline(parse_inline(&content), Span::NONE)),
                "s" | "strike" => {
                    Some(Inline::Strikethrough(parse_inline(&content), Span::NONE))
                }
                "code" => Some(Inline::Code(content, Span::NONE)),
                _ if tag.to_lowercase().starts_with("url=") => {
                    let url = tag[4..].to_string();
                    Some(Inline::Link {
                        url,
                        children: parse_inline(&content),
                        span: Span::NONE,
                    })
                }
                "url" => Some(Inline::Link {
                    url: content.clone(),
                    children: parse_inline(&content),
                    span: Span::NONE,
                }),
                _ if tag.to_lowercase().starts_with("img") => Some(Inline::Image {
                    url: content,
                    span: Span::NONE,
                }),
                "sub" => Some(Inline::Subscript(parse_inline(&content), Span::NONE)),
                "sup" => Some(Inline::Superscript(parse_inline(&content), Span::NONE)),
                _ if tag.to_lowercase().starts_with("color=") => {
                    let value = tag[6..].to_string();
                    Some(Inline::Span {
                        attr: "color".to_string(),
                        value,
                        children: parse_inline(&content),
                        span: Span::NONE,
                    })
                }
                _ if tag.to_lowercase().starts_with("size=") => {
                    let value = tag[5..].to_string();
                    Some(Inline::Span {
                        attr: "size".to_string(),
                        value,
                        children: parse_inline(&content),
                        span: Span::NONE,
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
        nodes.push(Inline::Text(current, Span::NONE));
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

    // Build closing tag chars (lowercase) for comparison without allocating per iteration.
    // e.g. for tag "b" → close_chars = ['[', '/', 'b', ']']
    let tag_base = tag.split('=').next().unwrap_or(&tag).to_lowercase();
    let close_lower: Vec<char> = format!("[/{}]", tag_base).chars().collect();
    let close_len = close_lower.len();

    let content_start = i;

    // Scan for the closing tag by comparing slices, not by allocating a String each time.
    while i + close_len <= chars.len() {
        // Check if chars[i..i+close_len] matches close_lower (case-insensitive)
        let matches = chars[i..i + close_len]
            .iter()
            .zip(close_lower.iter())
            .all(|(a, b)| a.to_lowercase().next() == Some(*b));
        if matches {
            let content: String = chars[content_start..i].iter().collect();
            return Some((tag, content, i + close_len));
        }
        i += 1;
    }

    None
}
