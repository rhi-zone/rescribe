//! XWiki parser.

use crate::ast::*;

/// Parse an XWiki string into an [`XwikiDoc`].
pub fn parse(input: &str) -> (XwikiDoc, Vec<Diagnostic>) {
    let lines: Vec<&str> = input.lines().collect();
    let mut parser = Parser::new(&lines);
    let blocks = parser.parse().unwrap_or_default();
    (XwikiDoc { blocks, span: Span::NONE }, vec![])
}

struct Parser<'a> {
    lines: &'a [&'a str],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(lines: &'a [&'a str]) -> Self {
        Self { lines, pos: 0 }
    }

    fn parse(&mut self) -> Result<Vec<Block>, String> {
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
                    span: Span::NONE,
                });
                self.pos += 1;
                continue;
            }

            // Horizontal rule: ----
            if line.trim() == "----" {
                result.push(Block::HorizontalRule { span: Span::NONE });
                self.pos += 1;
                continue;
            }

            // Code block: {{code}}...{{/code}}
            if line.trim().starts_with("{{code") {
                let (code_block, end) = self.parse_code_block();
                result.push(code_block);
                self.pos = end.max(self.pos + 1);
                continue;
            }

            // Table
            if line.trim().starts_with('|') {
                let (table_block, end) = self.parse_table();
                result.push(table_block);
                self.pos = end.max(self.pos + 1);
                continue;
            }

            // Lists
            if line.starts_with('*') || line.starts_with("1.") {
                let (list_block, end) = self.parse_list();
                result.push(list_block);
                self.pos = end.max(self.pos + 1);
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
                    span: Span::NONE,
                });
            }
            self.pos = end.max(self.pos + 1);
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
                language: lang,
                span: Span::NONE,
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
                            span: Span::NONE,
                        }
                    } else {
                        TableCell {
                            is_header: false,
                            inlines: parse_inline(cell),
                            span: Span::NONE,
                        }
                    }
                })
                .collect();

            rows.push(TableRow { cells, span: Span::NONE });
            i += 1;
        }

        (Block::Table { rows, span: Span::NONE }, i)
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
                span: Span::NONE,
            }]);
            i += 1;
        }

        (Block::List { ordered, items, span: Span::NONE }, i)
    }
}

pub(crate) fn parse_inline(text: &str) -> Vec<Inline> {
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
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            nodes.push(Inline::Bold(parse_inline(&content), Span::NONE));
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
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            nodes.push(Inline::Italic(parse_inline(&content), Span::NONE));
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
            nodes.push(Inline::Strikeout(parse_inline(&content), Span::NONE));
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
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            nodes.push(Inline::Code(content, Span::NONE));
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
                nodes.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            nodes.push(Inline::Link { url, label, span: Span::NONE });
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
