//! TikiWiki parser.

use crate::ast::*;

/// Parse a TikiWiki string into a [`TikiwikiDoc`].
pub fn parse(input: &str) -> (TikiwikiDoc, Vec<Diagnostic>) {
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
                span: Span::NONE,
            });
            i += 1;
            continue;
        }

        // Horizontal rule
        if line.trim() == "---" {
            result.push(Block::HorizontalRule { span: Span::NONE });
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

        // Blockquote: {QUOTE()}...{QUOTE}
        if line.trim().starts_with("{QUOTE") {
            let (bq_node, end) = parse_blockquote(&lines, i);
            result.push(bq_node);
            i = end;
            continue;
        }

        // Table: ||cell|cell||
        if line.trim().starts_with("||") {
            let (table_node, end) = parse_table(&lines, i);
            result.push(table_node);
            i = end.max(i + 1);
            continue;
        }

        // Lists
        if line.starts_with('*') || line.starts_with('#') {
            let (list_node, end) = parse_list(&lines, i, 1);
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
        i = end.max(i + 1);
    }

    (TikiwikiDoc { blocks: result, span: Span::NONE }, vec![])
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
            || line.trim().starts_with("{QUOTE")
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
        if let Some(rel_end) = first_line[paren_start + 1..].find(')') {
            let paren_end = paren_start + 1 + rel_end;
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
                    span: Span::NONE,
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
            span: Span::NONE,
        },
        i,
    )
}

fn parse_blockquote(lines: &[&str], start: usize) -> (Block, usize) {
    let mut content_lines = Vec::new();
    let mut i = start + 1;

    while i < lines.len() {
        let line = lines[i];
        if line.trim() == "{QUOTE}" || line.trim().starts_with("{QUOTE}") {
            let text = content_lines.join("\n");
            let (inner_doc, _) = parse(&text);
            return (
                Block::Blockquote {
                    blocks: inner_doc.blocks,
                    span: Span::NONE,
                },
                i + 1,
            );
        }
        content_lines.push(line);
        i += 1;
    }

    // Unclosed blockquote
    let text = content_lines.join("\n");
    let (inner_doc, _) = parse(&text);
    (
        Block::Blockquote {
            blocks: inner_doc.blocks,
            span: Span::NONE,
        },
        i,
    )
}

fn parse_table(lines: &[&str], start: usize) -> (Block, usize) {
    let mut rows = Vec::new();
    let mut i = start;

    // Check first line for table params (e.g. ||border=1)
    let first_line = lines[start].trim();
    let first_inner = first_line
        .trim_start_matches("||")
        .trim_end_matches("||");
    // If first line looks like a parameter line (no pipe separators in content)
    let is_param_line = !first_inner.is_empty()
        && !first_inner.contains('|')
        && first_inner.contains('=');
    if is_param_line {
        i += 1;
    }

    while i < lines.len() {
        let line = lines[i].trim();
        if line.is_empty() {
            break;
        }
        // End of table block: line that is just || (closing)
        if line == "||" {
            i += 1;
            break;
        }
        if !line.contains('|') {
            break;
        }

        // Parse row
        let inner = line.trim_start_matches("||").trim_end_matches("||");
        if inner.is_empty() {
            i += 1;
            continue;
        }

        let cell_strs: Vec<&str> = inner.split('|').collect();
        let mut is_header = false;
        let cells: Vec<TableCell> = cell_strs
            .iter()
            .map(|cell_str| {
                let trimmed = cell_str.trim();
                // Header cells use ^content^ syntax
                if trimmed.starts_with('^') && trimmed.ends_with('^') && trimmed.len() > 1 {
                    is_header = true;
                    let header_content = &trimmed[1..trimmed.len() - 1];
                    TableCell {
                        inlines: parse_inline(header_content),
                        span: Span::NONE,
                    }
                } else {
                    TableCell {
                        inlines: parse_inline(trimmed),
                        span: Span::NONE,
                    }
                }
            })
            .collect();

        rows.push(TableRow { cells, is_header, span: Span::NONE });
        i += 1;
    }

    (Block::Table { rows, span: Span::NONE }, i)
}

fn parse_list(lines: &[&str], start: usize, depth: usize) -> (Block, usize) {
    let first_char = lines[start].chars().next().unwrap_or(' ');
    let ordered = first_char == '#';
    let marker = if ordered { '#' } else { '*' };
    let mut items = Vec::new();
    let mut i = start;

    while i < lines.len() {
        let line = lines[i];

        // Count the marker depth
        let line_depth = line.chars().take_while(|&c| c == marker).count();
        if line_depth < depth {
            break;
        }

        if line_depth == depth {
            let text = line[line_depth..].trim();
            let mut item = ListItem {
                inlines: parse_inline(text),
                children: Vec::new(),
                span: Span::NONE,
            };
            i += 1;

            // Check for nested list items at deeper levels
            while i < lines.len() {
                let next_line = lines[i];
                let next_depth = next_line.chars().take_while(|&c| c == marker).count();
                if next_depth > depth {
                    let (nested, end) = parse_list(lines, i, next_depth);
                    item.children.push(nested);
                    i = end;
                } else {
                    break;
                }
            }

            items.push(item);
        } else {
            // line_depth > depth — handled by nesting above
            break;
        }
    }

    (Block::List { ordered, items, span: Span::NONE }, i)
}

pub(crate) fn parse_inline(text: &str) -> Vec<Inline> {
    let mut nodes = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Nowiki: ~np~...~/np~
        if i + 3 < chars.len()
            && chars[i] == '~'
            && chars[i + 1] == 'n'
            && chars[i + 2] == 'p'
            && chars[i + 3] == '~'
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            if let Some((content, end)) = find_delimited(&chars, i + 4, "~/np~") {
                nodes.push(Inline::Nowiki(content, Span::NONE));
                i = end;
            } else {
                current.push('~');
                i += 1;
            }
            continue;
        }

        // Bold: __text__
        if i + 1 < chars.len()
            && chars[i] == '_'
            && chars[i + 1] == '_'
            && let Some((content, end)) = find_delimited(&chars, i + 2, "__")
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            nodes.push(Inline::Bold(parse_inline(&content), Span::NONE));
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
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            nodes.push(Inline::Italic(parse_inline(&content), Span::NONE));
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
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            nodes.push(Inline::Underline(parse_inline(&content), Span::NONE));
            i = end;
            continue;
        }

        // Strikethrough: --text--
        if i + 1 < chars.len()
            && chars[i] == '-'
            && chars[i + 1] == '-'
            // Don't match --- (horizontal rule marker in inline context)
            && !(i + 2 < chars.len() && chars[i + 2] == '-')
            && let Some((content, end)) = find_delimited(&chars, i + 2, "--")
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            nodes.push(Inline::Strikethrough(parse_inline(&content), Span::NONE));
            i = end;
            continue;
        }

        // Superscript: ^text^
        if chars[i] == '^'
            && let Some((content, end)) = find_delimited(&chars, i + 1, "^")
        {
            // Only if content is non-empty and doesn't contain newline
            if !content.is_empty() {
                if !current.is_empty() {
                    nodes.push(Inline::Text(current.clone(), Span::NONE));
                    current.clear();
                }
                nodes.push(Inline::Superscript(parse_inline(&content), Span::NONE));
                i = end;
                continue;
            }
        }

        // Subscript: ,,text,,
        if i + 1 < chars.len()
            && chars[i] == ','
            && chars[i + 1] == ','
            && let Some((content, end)) = find_delimited(&chars, i + 2, ",,")
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            nodes.push(Inline::Subscript(parse_inline(&content), Span::NONE));
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
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            nodes.push(Inline::Code(content, Span::NONE));
            i = end;
            continue;
        }

        // Image: {img src=... }
        if chars[i] == '{'
            && i + 4 < chars.len()
            && chars[i + 1] == 'i'
            && chars[i + 2] == 'm'
            && chars[i + 3] == 'g'
            && chars[i + 4] == ' '
            && let Some((content, end)) = find_bracket_content(&chars, i + 1, '{', '}')
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            let img_params = &content[3..].trim(); // skip "img"
            let url = extract_img_param(img_params, "src")
                .unwrap_or_default();
            let alt = extract_img_param(img_params, "alt")
                .unwrap_or_default();
            nodes.push(Inline::Image { url, alt, span: Span::NONE });
            i = end;
            continue;
        }

        // WikiLink: ((page)) or ((page|label))
        if i + 1 < chars.len()
            && chars[i] == '('
            && chars[i + 1] == '('
            && let Some((content, end)) = find_delimited(&chars, i + 2, "))")
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            let parts: Vec<&str> = content.splitn(2, '|').collect();
            let page = parts[0].trim().to_string();
            let label = if parts.len() > 1 {
                parts[1].trim()
            } else {
                parts[0].trim()
            };
            nodes.push(Inline::WikiLink {
                page,
                children: vec![Inline::Text(label.to_string(), Span::NONE)],
                span: Span::NONE,
            });
            i = end;
            continue;
        }

        // Link: [url|label] or [url]
        if chars[i] == '['
            && let Some((content, end)) = find_bracket_content(&chars, i + 1, '[', ']')
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
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
                children: vec![Inline::Text(label.to_string(), Span::NONE)],
                span: Span::NONE,
            });
            i = end;
            continue;
        }

        // Line break: %%%
        if i + 2 < chars.len()
            && chars[i] == '%'
            && chars[i + 1] == '%'
            && chars[i + 2] == '%'
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            nodes.push(Inline::LineBreak { span: Span::NONE });
            i += 3;
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

fn extract_img_param(params: &str, key: &str) -> Option<String> {
    // Handle both src=value and src="value" forms
    let prefix = format!("{}=", key);
    for part in params.split_whitespace() {
        if let Some(val) = part.strip_prefix(&prefix) {
            let val = val.trim_matches('"');
            return Some(val.to_string());
        }
    }
    // Also try key= with spaces after =
    if let Some(pos) = params.find(&prefix) {
        let rest = &params[pos + prefix.len()..];
        if let Some(stripped) = rest.strip_prefix('"')
            && let Some(end) = stripped.find('"')
        {
            return Some(stripped[..end].to_string());
        }
    }
    None
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
