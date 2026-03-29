//! TWiki parser.

use crate::ast::*;

/// Parse a TWiki string into a [`TwikiDoc`].
pub fn parse(input: &str) -> (TwikiDoc, Vec<Diagnostic>) {
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
                span: Span::NONE,
            });
            i += 1;
            continue;
        }

        // Horizontal rule: ---
        if line.trim() == "---" {
            result.push(Block::HorizontalRule { span: Span::NONE });
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

        // <pre> block
        if line.trim().starts_with("<pre>") || line.trim().starts_with("<pre ") {
            let (code_node, end) = parse_pre_block(&lines, i);
            result.push(code_node);
            i = end;
            continue;
        }

        // Blockquote
        if line.trim().starts_with("<blockquote>") {
            let (bq_node, end) = parse_blockquote(&lines, i);
            result.push(bq_node);
            i = end;
            continue;
        }

        // Macro blocks (e.g. %INCLUDE{}%, %SEARCH{}%)
        if is_macro_block(line) {
            result.push(Block::RawBlock {
                content: line.to_string(),
                span: Span::NONE,
            });
            i += 1;
            continue;
        }

        // Table: |cell|cell|
        if line.trim().starts_with('|') && line.trim().ends_with('|') {
            let (table_node, end) = parse_table(&lines, i);
            result.push(table_node);
            i = end.max(i + 1);
            continue;
        }

        // Definition lists (   $ term: definition)
        if line.starts_with("   ")
            && line.trim().starts_with('$')
            && line.contains(':')
        {
            let (dl_node, end) = parse_definition_list(&lines, i);
            result.push(dl_node);
            i = end.max(i + 1);
            continue;
        }

        // Lists (indented with spaces)
        if line.starts_with("   ")
            && (line.trim().starts_with('*') || line.trim().starts_with("1."))
        {
            let (list_node, end) = parse_list(&lines, i);
            result.push(list_node);
            i = end.max(i + 1);
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
        // Ensure progress even when collect_paragraph returns without consuming anything
        i = end.max(i + 1);
    }

    (TwikiDoc { blocks: result, span: Span::NONE }, vec![])
}

fn is_macro_block(line: &str) -> bool {
    let trimmed = line.trim();
    // Matches %INCLUDE{...}%, %SEARCH{...}%, %TOC%, etc. as standalone block
    if trimmed.starts_with('%') && trimmed.ends_with('%') {
        let inner = &trimmed[1..trimmed.len() - 1];
        // Must be a single macro (uppercase letters, possibly with {})
        if let Some(name_end) = inner.find(['{', '%']) {
            return inner[..name_end].chars().all(|c| c.is_ascii_uppercase() || c == '_');
        }
        return inner.chars().all(|c| c.is_ascii_uppercase() || c == '_');
    }
    false
}

fn collect_paragraph<'a>(lines: &[&'a str], start: usize) -> (Vec<&'a str>, usize) {
    let mut para_lines = Vec::new();
    let mut i = start;

    while i < lines.len() {
        let line = lines[i];
        if line.trim().is_empty()
            || line.starts_with("---")
            || (line.starts_with("   ")
                && (line.trim().starts_with('*')
                    || line.trim().starts_with("1.")
                    || line.trim().starts_with('$')))
            || (line.trim().starts_with('|') && line.trim().ends_with('|'))
            || line.trim().starts_with("<verbatim>")
            || line.trim().starts_with("<pre>")
            || line.trim().starts_with("<pre ")
            || line.trim().starts_with("<blockquote>")
            || is_macro_block(line)
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
                span: Span::NONE,
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

fn parse_pre_block(lines: &[&str], start: usize) -> (Block, usize) {
    let mut code_lines = Vec::new();
    let first_line = lines[start].trim();
    let mut i = start;

    // Handle single-line
    if first_line.contains("</pre>") {
        let content = first_line
            .strip_prefix("<pre>")
            .or_else(|| {
                // Handle <pre ...> with attributes
                let idx = first_line.find('>')?;
                Some(&first_line[idx + 1..])
            })
            .unwrap_or(first_line)
            .strip_suffix("</pre>")
            .unwrap_or(first_line);
        return (
            Block::CodeBlock {
                content: content.to_string(),
                span: Span::NONE,
            },
            start + 1,
        );
    }

    // Skip opening tag
    let after_tag = if let Some(rest) = first_line.strip_prefix("<pre>") {
        rest.to_string()
    } else if let Some(idx) = first_line.find('>') {
        first_line[idx + 1..].to_string()
    } else {
        String::new()
    };
    if !after_tag.is_empty() {
        code_lines.push(after_tag);
    }
    i += 1;

    while i < lines.len() {
        let line = lines[i];
        if line.contains("</pre>") {
            let before = line.split("</pre>").next().unwrap_or("");
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

fn parse_blockquote(lines: &[&str], start: usize) -> (Block, usize) {
    let mut content_lines = Vec::new();
    let first_line = lines[start].trim();
    let mut i = start;

    // Handle single-line
    if first_line.contains("</blockquote>") {
        let content = first_line
            .strip_prefix("<blockquote>")
            .unwrap_or(first_line)
            .strip_suffix("</blockquote>")
            .unwrap_or(first_line);
        let children = if content.trim().is_empty() {
            vec![]
        } else {
            vec![Block::Paragraph {
                inlines: parse_inline(content.trim()),
                span: Span::NONE,
            }]
        };
        return (
            Block::Blockquote { children, span: Span::NONE },
            start + 1,
        );
    }

    // Skip opening tag
    let after_tag = first_line
        .strip_prefix("<blockquote>")
        .unwrap_or("")
        .to_string();
    if !after_tag.trim().is_empty() {
        content_lines.push(after_tag.trim().to_string());
    }
    i += 1;

    while i < lines.len() {
        let line = lines[i];
        if line.contains("</blockquote>") {
            let before = line.split("</blockquote>").next().unwrap_or("");
            if !before.trim().is_empty() {
                content_lines.push(before.trim().to_string());
            }
            let children = if content_lines.is_empty() {
                vec![]
            } else {
                let text = content_lines.join(" ");
                vec![Block::Paragraph {
                    inlines: parse_inline(&text),
                    span: Span::NONE,
                }]
            };
            return (
                Block::Blockquote { children, span: Span::NONE },
                i + 1,
            );
        }
        if !line.trim().is_empty() {
            content_lines.push(line.trim().to_string());
        }
        i += 1;
    }

    let children = if content_lines.is_empty() {
        vec![]
    } else {
        let text = content_lines.join(" ");
        vec![Block::Paragraph {
            inlines: parse_inline(&text),
            span: Span::NONE,
        }]
    };
    (Block::Blockquote { children, span: Span::NONE }, i)
}

fn parse_table(lines: &[&str], start: usize) -> (Block, usize) {
    let mut rows = Vec::new();
    let mut i = start;

    while i < lines.len() {
        let line = lines[i].trim();
        if !line.starts_with('|') || !line.ends_with('|') {
            break;
        }

        // A valid table row needs at least "||" (2 chars).
        if line.len() < 2 {
            break;
        }

        let inner = &line[1..line.len() - 1];
        let cells: Vec<TableCell> = inner
            .split('|')
            .map(|cell| {
                let cell = cell.trim();
                // Header cells start and end with * (need at least 2 chars)
                let is_header =
                    cell.len() >= 2 && cell.starts_with('*') && cell.ends_with('*');
                let content = if is_header {
                    &cell[1..cell.len() - 1]
                } else {
                    cell
                };
                TableCell {
                    inlines: parse_inline(content),
                    is_header,
                    span: Span::NONE,
                }
            })
            .collect();

        rows.push(TableRow { cells, span: Span::NONE });
        i += 1;
    }

    (Block::Table { rows, span: Span::NONE }, i)
}

fn parse_definition_list(lines: &[&str], start: usize) -> (Block, usize) {
    let mut items = Vec::new();
    let mut i = start;

    while i < lines.len() {
        let line = lines[i];
        if !line.starts_with("   ") {
            break;
        }
        let trimmed = line.trim();
        if !trimmed.starts_with('$') {
            break;
        }

        let rest = trimmed[1..].trim();
        if let Some(colon_pos) = rest.find(':') {
            let term = rest[..colon_pos].trim();
            let desc = rest[colon_pos + 1..].trim();
            items.push(DefinitionItem {
                term: parse_inline(term),
                desc: parse_inline(desc),
                span: Span::NONE,
            });
        }
        i += 1;
    }

    (Block::DefinitionList { items, span: Span::NONE }, i)
}

fn parse_list(lines: &[&str], start: usize) -> (Block, usize) {
    let first_line = lines[start].trim();
    let ordered = first_line.starts_with("1.");
    let mut items: Vec<ListItem> = Vec::new();
    let mut i = start;

    while i < lines.len() {
        let line = lines[i];
        if !line.starts_with("   ") {
            break;
        }

        let trimmed = line.trim();
        // Detect nested lists (more indentation)
        let indent_level = line.len() - line.trim_start().len();
        if indent_level > 3 && !items.is_empty() {
            // Nested list item — collect and attach as child
            let (nested, end) = parse_nested_list(lines, i, indent_level);
            if let Some(last) = items.last_mut() {
                last.children.push(nested);
            }
            i = end;
            continue;
        }

        let text = if ordered {
            trimmed.strip_prefix("1.").unwrap_or(trimmed).trim()
        } else {
            trimmed.strip_prefix('*').unwrap_or(trimmed).trim()
        };

        if trimmed.starts_with('*') || trimmed.starts_with("1.") {
            items.push(ListItem {
                inlines: parse_inline(text),
                children: vec![],
                span: Span::NONE,
            });
        }
        i += 1;
    }

    (Block::List { ordered, items, span: Span::NONE }, i)
}

fn parse_nested_list(lines: &[&str], start: usize, min_indent: usize) -> (Block, usize) {
    let first_trimmed = lines[start].trim();
    let ordered = first_trimmed.starts_with("1.");
    let mut items: Vec<ListItem> = Vec::new();
    let mut i = start;

    while i < lines.len() {
        let line = lines[i];
        let indent = line.len() - line.trim_start().len();
        if indent < min_indent {
            break;
        }

        let trimmed = line.trim();
        // Even deeper nesting
        if indent > min_indent && !items.is_empty() {
            let (nested, end) = parse_nested_list(lines, i, indent);
            if let Some(last) = items.last_mut() {
                last.children.push(nested);
            }
            i = end;
            continue;
        }

        let text = if ordered {
            trimmed.strip_prefix("1.").unwrap_or(trimmed).trim()
        } else {
            trimmed.strip_prefix('*').unwrap_or(trimmed).trim()
        };

        if trimmed.starts_with('*') || trimmed.starts_with("1.") {
            items.push(ListItem {
                inlines: parse_inline(text),
                children: vec![],
                span: Span::NONE,
            });
        }
        i += 1;
    }

    (Block::List { ordered, items, span: Span::NONE }, i)
}

pub(crate) fn parse_inline(text: &str) -> Vec<Inline> {
    let mut nodes = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // HTML inline tags: <del>, <ins>, <sup>, <sub>, <u>
        if chars[i] == '<' {
            if let Some((tag, content, end)) = try_html_inline_tag(&chars, i) {
                if !current.is_empty() {
                    nodes.push(Inline::Text(current.clone(), Span::NONE));
                    current.clear();
                }
                let children = parse_inline(&content);
                match tag.as_str() {
                    "del" => nodes.push(Inline::Strikethrough(children, Span::NONE)),
                    "ins" | "u" => nodes.push(Inline::Underline(children, Span::NONE)),
                    "sup" => nodes.push(Inline::Superscript(children, Span::NONE)),
                    "sub" => nodes.push(Inline::Subscript(children, Span::NONE)),
                    "img" => {
                        // Already handled by try_html_inline_tag
                        let url = content.clone();
                        nodes.push(Inline::Image { url, alt: String::new(), span: Span::NONE });
                    }
                    _ => nodes.push(Inline::RawInline { content: format!("<{tag}>{content}</{tag}>"), span: Span::NONE }),
                }
                i = end;
                continue;
            }
            // <img> tag (self-closing or with attributes)
            if let Some((url, alt, end)) = try_img_tag(&chars, i) {
                if !current.is_empty() {
                    nodes.push(Inline::Text(current.clone(), Span::NONE));
                    current.clear();
                }
                nodes.push(Inline::Image { url, alt, span: Span::NONE });
                i = end;
                continue;
            }
            // <sticky>, <noautolink> and their closing forms — skip as raw
            if let Some((raw, end)) = try_control_tag(&chars, i) {
                if !current.is_empty() {
                    nodes.push(Inline::Text(current.clone(), Span::NONE));
                    current.clear();
                }
                nodes.push(Inline::RawInline { content: raw, span: Span::NONE });
                i = end;
                continue;
            }
        }

        // %MACRO{...}% or %VAR% inline macros
        if chars[i] == '%'
            && let Some((raw, end)) = try_inline_macro(&chars, i)
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            // Special case: %BR% is a line break
            if raw == "%BR%" {
                nodes.push(Inline::LineBreak { span: Span::NONE });
            } else if raw.starts_with("%ATTACHURL%") || raw.starts_with("%PUBURL%") {
                // Image reference
                nodes.push(Inline::Image { url: raw, alt: String::new(), span: Span::NONE });
            } else {
                nodes.push(Inline::RawInline { content: raw, span: Span::NONE });
            }
            i = end;
            continue;
        }

        // Bold italic: __text__
        if i + 1 < chars.len()
            && chars[i] == '_'
            && chars[i + 1] == '_'
            && let Some((content, end)) = find_delimited(&chars, i + 2, "__")
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            nodes.push(Inline::BoldItalic(parse_inline(&content), Span::NONE));
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
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            nodes.push(Inline::Bold(parse_inline(&content), Span::NONE));
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
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            nodes.push(Inline::Italic(parse_inline(&content), Span::NONE));
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
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            nodes.push(Inline::BoldCode(parse_inline(&content), Span::NONE));
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
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            nodes.push(Inline::Code(content, Span::NONE));
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
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            nodes.push(Inline::Link { url, label, span: Span::NONE });
            i = end;
            continue;
        }

        // Escaped WikiWord: !WikiWord — emit as plain text without the !
        if chars[i] == '!' && i + 1 < chars.len() && is_wikiword_start(&chars, i + 1) {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            let (word, end) = collect_wikiword(&chars, i + 1);
            current.push_str(&word);
            i = end;
            continue;
        }

        // WikiWord auto-link
        if is_wikiword_start(&chars, i)
            && (i == 0 || !chars[i - 1].is_alphanumeric())
            && let Some((word, end)) = try_wikiword(&chars, i)
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            nodes.push(Inline::WikiWord { word, span: Span::NONE });
            i = end;
            continue;
        }

        current.push(chars[i]);
        i += 1;
    }

    if !current.is_empty() {
        nodes.push(Inline::Text(current, Span::NONE));
    }

    nodes
}

fn try_html_inline_tag(chars: &[char], start: usize) -> Option<(String, String, usize)> {
    if chars[start] != '<' {
        return None;
    }
    // Collect tag name
    let mut i = start + 1;
    let mut tag = String::new();
    while i < chars.len() && chars[i].is_ascii_alphanumeric() {
        tag.push(chars[i]);
        i += 1;
    }
    if tag.is_empty() || i >= chars.len() || chars[i] != '>' {
        return None;
    }
    let tag_lower = tag.to_lowercase();
    match tag_lower.as_str() {
        "del" | "ins" | "sup" | "sub" | "u" => {}
        _ => return None,
    }
    i += 1; // skip >

    // Find closing tag
    let close_tag = format!("</{}>", tag_lower);
    let close_chars: Vec<char> = close_tag.chars().collect();
    let mut content = String::new();
    while i + close_chars.len() <= chars.len() {
        let mut matched = true;
        for (j, c) in close_chars.iter().enumerate() {
            if chars[i + j].to_lowercase().next() != c.to_lowercase().next() {
                matched = false;
                break;
            }
        }
        if matched {
            return Some((tag_lower, content, i + close_chars.len()));
        }
        content.push(chars[i]);
        i += 1;
    }
    None
}

fn try_img_tag(chars: &[char], start: usize) -> Option<(String, String, usize)> {
    if chars[start] != '<' {
        return None;
    }
    let rest: String = chars[start..].iter().collect();
    let lower = rest.to_lowercase();
    if !lower.starts_with("<img ") {
        return None;
    }
    // Find the end of the tag (> or />)
    let end_pos = rest.find('>')?;
    let tag_str = &rest[..end_pos + 1];

    let mut url = String::new();
    let mut alt = String::new();

    // Extract src attribute
    if let Some(src_start) = tag_str.to_lowercase().find("src=") {
        let after_src = &tag_str[src_start + 4..];
        if let Some(val) = extract_attr_value(after_src) {
            url = val;
        }
    }
    // Extract alt attribute
    if let Some(alt_start) = tag_str.to_lowercase().find("alt=") {
        let after_alt = &tag_str[alt_start + 4..];
        if let Some(val) = extract_attr_value(after_alt) {
            alt = val;
        }
    }

    Some((url, alt, start + end_pos + 1))
}

fn extract_attr_value(s: &str) -> Option<String> {
    let s = s.trim();
    if let Some(rest) = s.strip_prefix('"') {
        let end = rest.find('"')?;
        Some(rest[..end].to_string())
    } else if let Some(rest) = s.strip_prefix('\'') {
        let end = rest.find('\'')?;
        Some(rest[..end].to_string())
    } else {
        let end = s.find(|c: char| c.is_whitespace() || c == '>' || c == '/').unwrap_or(s.len());
        Some(s[..end].to_string())
    }
}

fn try_control_tag(chars: &[char], start: usize) -> Option<(String, usize)> {
    let rest: String = chars[start..].iter().collect();
    let lower = rest.to_lowercase();
    let tags = ["<sticky>", "</sticky>", "<noautolink>", "</noautolink>"];
    for tag in &tags {
        if lower.starts_with(tag) {
            let actual: String = chars[start..start + tag.len()].iter().collect();
            return Some((actual, start + tag.len()));
        }
    }
    None
}

fn try_inline_macro(chars: &[char], start: usize) -> Option<(String, usize)> {
    if chars[start] != '%' {
        return None;
    }
    let mut i = start + 1;
    // Collect macro name (uppercase letters + underscore)
    while i < chars.len() && (chars[i].is_ascii_uppercase() || chars[i] == '_') {
        i += 1;
    }
    if i == start + 1 {
        return None;
    }
    // Check for {args}
    if i < chars.len() && chars[i] == '{' {
        let mut depth = 1;
        i += 1;
        while i < chars.len() && depth > 0 {
            if chars[i] == '{' {
                depth += 1;
            } else if chars[i] == '}' {
                depth -= 1;
            }
            i += 1;
        }
    }
    // Must end with %
    if i < chars.len() && chars[i] == '%' {
        let raw: String = chars[start..=i].iter().collect();
        return Some((raw, i + 1));
    }
    None
}

fn is_wikiword_start(chars: &[char], pos: usize) -> bool {
    pos < chars.len() && chars[pos].is_ascii_uppercase()
}

fn try_wikiword(chars: &[char], start: usize) -> Option<(String, usize)> {
    // WikiWord: two+ uppercase letters each followed by lowercase letters
    // e.g. WikiWord, MyPage, ABCDef
    let (word, end) = collect_wikiword(chars, start);
    // Must have at least two uppercase transitions
    let mut upper_count = 0;
    let mut prev_lower = false;
    for ch in word.chars() {
        if ch.is_ascii_uppercase() {
            if prev_lower || upper_count == 0 {
                upper_count += 1;
            }
            prev_lower = false;
        } else if ch.is_ascii_lowercase() {
            prev_lower = true;
        } else {
            return None; // Contains non-alpha
        }
    }
    if upper_count >= 2 && word.len() >= 3 {
        Some((word, end))
    } else {
        None
    }
}

fn collect_wikiword(chars: &[char], start: usize) -> (String, usize) {
    let mut i = start;
    let mut word = String::new();
    while i < chars.len() && chars[i].is_ascii_alphanumeric() {
        word.push(chars[i]);
        i += 1;
    }
    (word, i)
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
