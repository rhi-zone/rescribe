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
                span: Span::NONE,
            });
        }
        i = end;
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

        rows.push(TableRow { cells, span: Span::NONE });
        i += 1;
    }

    (Block::Table { rows, span: Span::NONE }, i)
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

    (Block::List { ordered, items, span: Span::NONE }, i)
}

pub(crate) fn parse_inline(text: &str) -> Vec<Inline> {
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

        current.push(chars[i]);
        i += 1;
    }

    if !current.is_empty() {
        nodes.push(Inline::Text(current, Span::NONE));
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
