//! BBCode parser — infallible, returns (BbcodeDoc, Vec<Diagnostic>).

use crate::ast::{AlignKind, BbcodeDoc, Block, Diagnostic, Inline, Span, TableRow};

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
        let lower = line.trim().to_lowercase();

        // Code block: [code]...[/code] or [code=lang]...[/code]
        if lower.starts_with("[code]") || lower.starts_with("[code=") {
            let (block, end) = parse_code_block(&lines, i);
            result.push(block);
            i = end;
            continue;
        }

        // Pre block: [pre]...[/pre]
        if lower.starts_with("[pre]") {
            let (block, end) = parse_pre_block(&lines, i);
            result.push(block);
            i = end;
            continue;
        }

        // Quote: [quote]...[/quote] or [quote=Author]...[/quote]
        if lower.starts_with("[quote") {
            let (block, end) = parse_quote(&lines, i);
            result.push(block);
            i = end;
            continue;
        }

        // List: [list]...[/list]
        if lower.starts_with("[list") {
            let (block, end) = parse_list(&lines, i);
            result.push(block);
            i = end;
            continue;
        }

        // Table: [table]...[/table]
        if lower.starts_with("[table]") {
            let (block, end) = parse_table(&lines, i);
            result.push(block);
            i = end;
            continue;
        }

        // Spoiler: [spoiler]...[/spoiler]
        if lower.starts_with("[spoiler]") {
            let (block, end) = parse_wrapped_block(&lines, i, "spoiler");
            result.push(Block::Spoiler {
                children: block,
                span: Span::NONE,
            });
            i = end;
            continue;
        }

        // Indent: [indent]...[/indent]
        if lower.starts_with("[indent]") {
            let (block, end) = parse_wrapped_block(&lines, i, "indent");
            result.push(Block::Indent {
                children: block,
                span: Span::NONE,
            });
            i = end;
            continue;
        }

        // Alignment blocks: [center], [left], [right]
        if lower.starts_with("[center]") {
            let (block, end) = parse_wrapped_block(&lines, i, "center");
            result.push(Block::Alignment {
                kind: AlignKind::Center,
                children: block,
                span: Span::NONE,
            });
            i = end;
            continue;
        }
        if lower.starts_with("[left]") {
            let (block, end) = parse_wrapped_block(&lines, i, "left");
            result.push(Block::Alignment {
                kind: AlignKind::Left,
                children: block,
                span: Span::NONE,
            });
            i = end;
            continue;
        }
        if lower.starts_with("[right]") {
            let (block, end) = parse_wrapped_block(&lines, i, "right");
            result.push(Block::Alignment {
                kind: AlignKind::Right,
                children: block,
                span: Span::NONE,
            });
            i = end;
            continue;
        }

        // Heading: [h1]...[/h1] through [h6]...[/h6]
        if let Some(level) = parse_heading_level(&lower) {
            let (block, end) = parse_heading(&lines, i, level);
            result.push(block);
            i = end;
            continue;
        }

        // Horizontal rule: [hr] (self-closing)
        if lower.starts_with("[hr]") || lower == "[hr/]" || lower == "[hr /]" {
            result.push(Block::HorizontalRule { span: Span::NONE });
            i += 1;
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

fn parse_heading_level(lower: &str) -> Option<u8> {
    for level in 1..=6u8 {
        let tag = format!("[h{}]", level);
        if lower.starts_with(&tag) {
            return Some(level);
        }
    }
    None
}

fn parse_heading(lines: &[&str], start: usize, level: u8) -> (Block, usize) {
    let open_tag = format!("[h{}]", level);
    let close_tag = format!("[/h{}]", level);

    // Combine all lines until we find close tag
    let mut content_parts = Vec::new();
    let first_line = lines[start].trim();
    // Extract content after opening tag on first line
    let lower_first = first_line.to_lowercase();
    if let Some(pos) = lower_first.find(&open_tag) {
        let after = &first_line[pos + open_tag.len()..];
        // Check if close tag is on same line
        let lower_after = after.to_lowercase();
        if let Some(close_pos) = lower_after.find(&close_tag) {
            let content = &after[..close_pos];
            return (
                Block::Heading {
                    level,
                    children: parse_inline(content),
                    span: Span::NONE,
                },
                start + 1,
            );
        }
        content_parts.push(after.to_string());
    }

    let mut i = start + 1;
    while i < lines.len() {
        let lower = lines[i].to_lowercase();
        if let Some(close_pos) = lower.find(&close_tag) {
            let before = &lines[i][..close_pos];
            if !before.trim().is_empty() {
                content_parts.push(before.trim().to_string());
            }
            let text = content_parts.join(" ");
            return (
                Block::Heading {
                    level,
                    children: parse_inline(&text),
                    span: Span::NONE,
                },
                i + 1,
            );
        }
        if !lines[i].trim().is_empty() {
            content_parts.push(lines[i].trim().to_string());
        }
        i += 1;
    }

    let text = content_parts.join(" ");
    (
        Block::Heading {
            level,
            children: parse_inline(&text),
            span: Span::NONE,
        },
        i,
    )
}

fn is_block_start(lower: &str) -> bool {
    lower.starts_with("[code]")
        || lower.starts_with("[code=")
        || lower.starts_with("[pre]")
        || lower.starts_with("[quote")
        || lower.starts_with("[list")
        || lower.starts_with("[table]")
        || lower.starts_with("[spoiler]")
        || lower.starts_with("[indent]")
        || lower.starts_with("[center]")
        || lower.starts_with("[left]")
        || lower.starts_with("[right]")
        || lower.starts_with("[hr]")
        || lower == "[hr/]"
        || lower == "[hr /]"
        || parse_heading_level(lower).is_some()
}

fn collect_paragraph<'a>(lines: &[&'a str], start: usize) -> (Vec<&'a str>, usize) {
    let mut para_lines = Vec::new();
    let mut i = start;

    while i < lines.len() {
        let line = lines[i];
        let lower = line.trim().to_lowercase();
        if line.trim().is_empty() || is_block_start(&lower) {
            break;
        }
        para_lines.push(line.trim());
        i += 1;
    }

    (para_lines, i)
}

fn parse_code_block(lines: &[&str], start: usize) -> (Block, usize) {
    let first_line = lines[start].trim();
    let lower_first = first_line.to_lowercase();
    let mut code_lines = Vec::new();

    // Extract language if present: [code=lang]
    let language = if lower_first.starts_with("[code=") {
        let after_eq = &first_line[6..]; // skip "[code="
        after_eq
            .find(']')
            .map(|end| first_line[6..6 + end].to_string())
    } else {
        None
    };

    // Find the end of the opening tag
    let open_end = first_line
        .find(']')
        .map(|p| p + 1)
        .unwrap_or(first_line.len());
    let after_open = &first_line[open_end..];

    // Single line code block
    let lower_after = after_open.to_lowercase();
    if lower_after.contains("[/code]") {
        let content = if let Some(pos) = lower_after.find("[/code]") {
            &after_open[..pos]
        } else {
            after_open
        };
        return (
            Block::CodeBlock {
                language,
                content: content.to_string(),
                span: Span::NONE,
            },
            start + 1,
        );
    }

    if !after_open.is_empty() {
        code_lines.push(after_open.to_string());
    }
    let mut i = start + 1;

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
                    language,
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
            language,
            content: code_lines.join("\n"),
            span: Span::NONE,
        },
        i,
    )
}

fn parse_pre_block(lines: &[&str], start: usize) -> (Block, usize) {
    let first_line = lines[start].trim();
    let lower_first = first_line.to_lowercase();

    // Single-line: [pre]content[/pre]
    if lower_first.contains("[/pre]") {
        let after_open = &first_line[5..]; // skip "[pre]"
        let content = if let Some(pos) = after_open.to_lowercase().find("[/pre]") {
            &after_open[..pos]
        } else {
            after_open
        };
        return (
            Block::Preformatted {
                content: content.to_string(),
                span: Span::NONE,
            },
            start + 1,
        );
    }

    let after_open = &first_line[5..]; // skip "[pre]"
    let mut content_lines = Vec::new();
    if !after_open.is_empty() {
        content_lines.push(after_open.to_string());
    }
    let mut i = start + 1;

    while i < lines.len() {
        let lower = lines[i].to_lowercase();
        if lower.contains("[/pre]") {
            let before = lines[i].split("[/pre]").next().unwrap_or("");
            let before = before.split("[/PRE]").next().unwrap_or(before);
            if !before.is_empty() {
                content_lines.push(before.to_string());
            }
            return (
                Block::Preformatted {
                    content: content_lines.join("\n"),
                    span: Span::NONE,
                },
                i + 1,
            );
        }
        content_lines.push(lines[i].to_string());
        i += 1;
    }

    (
        Block::Preformatted {
            content: content_lines.join("\n"),
            span: Span::NONE,
        },
        i,
    )
}

fn parse_quote(lines: &[&str], start: usize) -> (Block, usize) {
    let first_line = lines[start].trim();
    let lower_first = first_line.to_lowercase();

    // Extract author if present: [quote=Author] or [quote="Author"]
    let author = if lower_first.starts_with("[quote=") {
        let after_eq = &first_line[7..]; // skip "[quote="
        after_eq.find(']').map(|end| {
            let raw = &first_line[7..7 + end];
            // Strip quotes if present
            raw.trim_matches('"').trim_matches('\'').to_string()
        })
    } else {
        None
    };

    let mut quote_lines = Vec::new();
    let mut i = start + 1;

    while i < lines.len() {
        let line = lines[i];
        if line.to_lowercase().contains("[/quote]") {
            let text = quote_lines.join(" ");
            return (
                Block::Blockquote {
                    author,
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
            author,
            children: vec![Block::Paragraph {
                inlines: parse_inline(&text),
                span: Span::NONE,
            }],
            span: Span::NONE,
        },
        i,
    )
}

/// Parse a generic wrapped block ([spoiler], [center], [left], [right], [indent]).
fn parse_wrapped_block(lines: &[&str], start: usize, tag: &str) -> (Vec<Block>, usize) {
    let close_tag = format!("[/{}]", tag);
    let mut inner_lines = Vec::new();
    let mut i = start + 1;

    while i < lines.len() {
        let lower = lines[i].to_lowercase();
        if lower.contains(&close_tag) {
            break;
        }
        inner_lines.push(lines[i]);
        i += 1;
    }
    // Skip close tag line
    if i < lines.len() {
        i += 1;
    }

    // Parse inner lines as block content
    let inner_text = inner_lines.join("\n");
    let (inner_doc, _) = parse(&inner_text);
    (inner_doc.blocks, i)
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
            if let Some(th_pos) = search.find("[th]") {
                if th_pos < pos {
                    (th_pos, true, 4usize)
                } else {
                    (pos, false, 4usize)
                }
            } else {
                (pos, false, 4usize)
            }
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

            let tag_lower = tag.to_lowercase();
            let node = match tag_lower.as_str() {
                "b" => Some(Inline::Bold(parse_inline(&content), Span::NONE)),
                "i" => Some(Inline::Italic(parse_inline(&content), Span::NONE)),
                "u" => Some(Inline::Underline(parse_inline(&content), Span::NONE)),
                "s" | "strike" => {
                    Some(Inline::Strikethrough(parse_inline(&content), Span::NONE))
                }
                "code" | "icode" | "inlinecode" => Some(Inline::Code(content, Span::NONE)),
                "sub" => Some(Inline::Subscript(parse_inline(&content), Span::NONE)),
                "sup" => Some(Inline::Superscript(parse_inline(&content), Span::NONE)),
                "noparse" | "nobbc" => Some(Inline::Noparse(content, Span::NONE)),
                "url" => Some(Inline::Link {
                    url: content.clone(),
                    children: parse_inline(&content),
                    span: Span::NONE,
                }),
                "email" => Some(Inline::Email {
                    addr: content.clone(),
                    children: parse_inline(&content),
                    span: Span::NONE,
                }),
                "img" => Some(Inline::Image {
                    url: content,
                    width: None,
                    height: None,
                    span: Span::NONE,
                }),
                _ if tag_lower.starts_with("url=") => {
                    let url = tag[4..].to_string();
                    Some(Inline::Link {
                        url,
                        children: parse_inline(&content),
                        span: Span::NONE,
                    })
                }
                _ if tag_lower.starts_with("email=") => {
                    let addr = tag[6..].to_string();
                    Some(Inline::Email {
                        addr,
                        children: parse_inline(&content),
                        span: Span::NONE,
                    })
                }
                _ if tag_lower.starts_with("img=") => {
                    let dims = &tag[4..];
                    let (w, h) = parse_dimensions(dims);
                    Some(Inline::Image {
                        url: content,
                        width: w,
                        height: h,
                        span: Span::NONE,
                    })
                }
                _ if tag_lower.starts_with("color=") => {
                    let value = tag[6..].to_string();
                    Some(Inline::Color {
                        value,
                        children: parse_inline(&content),
                        span: Span::NONE,
                    })
                }
                _ if tag_lower.starts_with("size=") => {
                    let value = tag[5..].to_string();
                    Some(Inline::Size {
                        value,
                        children: parse_inline(&content),
                        span: Span::NONE,
                    })
                }
                _ if tag_lower.starts_with("font=") => {
                    let name = tag[5..].to_string();
                    Some(Inline::Font {
                        name,
                        children: parse_inline(&content),
                        span: Span::NONE,
                    })
                }
                // YouTube / video → link
                "youtube" => Some(Inline::Link {
                    url: format!("https://www.youtube.com/watch?v={}", content),
                    children: vec![Inline::Text(content, Span::NONE)],
                    span: Span::NONE,
                }),
                "video" => Some(Inline::Link {
                    url: content.clone(),
                    children: vec![Inline::Text(content, Span::NONE)],
                    span: Span::NONE,
                }),
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

/// Parse dimensions string like "100x50" into (width, height).
fn parse_dimensions(s: &str) -> (Option<u32>, Option<u32>) {
    let lower = s.to_lowercase();
    if let Some(pos) = lower.find('x') {
        let w = lower[..pos].parse().ok();
        let h = lower[pos + 1..].parse().ok();
        (w, h)
    } else {
        (None, None)
    }
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
