//! MediaWiki parser — infallible, returns (MediawikiDoc, Vec<Diagnostic>).

use crate::ast::{Block, Diagnostic, Inline, MediawikiDoc, Span, TableCell, TableRow};

/// Parse a MediaWiki string into a [`MediawikiDoc`].
///
/// The parser is infallible: any unrecognised input is treated as a paragraph.
/// Diagnostics (warnings/errors) are returned alongside the document.
pub fn parse(input: &str) -> (MediawikiDoc, Vec<Diagnostic>) {
    let mut p = Parser::new(input);
    let (blocks, diags) = p.parse();
    (MediawikiDoc { blocks, span: Span::NONE }, diags)
}

struct Parser<'a> {
    input: &'a str,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input }
    }

    fn parse(&mut self) -> (Vec<Block>, Vec<Diagnostic>) {
        let lines: Vec<&str> = self.input.lines().collect();
        let mut i = 0;
        let mut blocks = Vec::new();
        let diags = Vec::new();

        while i < lines.len() {
            let line = lines[i];
            let trimmed = line.trim();

            if trimmed.is_empty() {
                i += 1;
                continue;
            }

            // Heading
            if trimmed.starts_with('=')
                && let Some(heading) = self.parse_heading(trimmed)
            {
                blocks.push(heading);
                i += 1;
                continue;
            }

            // List
            if trimmed.starts_with('*') || trimmed.starts_with('#') {
                let (list, consumed) = self.parse_list(&lines[i..]);
                blocks.push(list);
                i += consumed;
                continue;
            }

            // Horizontal rule
            if trimmed == "----" || (trimmed.chars().all(|c| c == '-') && trimmed.len() >= 4) {
                blocks.push(Block::HorizontalRule);
                i += 1;
                continue;
            }

            // Code block (indented with space)
            if line.starts_with(' ') {
                let (block, consumed) = self.parse_code_block(&lines[i..]);
                blocks.push(block);
                i += consumed;
                continue;
            }

            // Table
            if trimmed.starts_with("{|") {
                let (table, consumed) = self.parse_table(&lines[i..]);
                blocks.push(table);
                i += consumed;
                continue;
            }

            // Regular paragraph
            let (para, consumed) = self.parse_paragraph(&lines[i..]);
            blocks.push(para);
            i += consumed;
        }

        (blocks, diags)
    }

    fn parse_heading(&self, line: &str) -> Option<Block> {
        let trimmed = line.trim();

        // Count leading `=`
        let level = trimmed.chars().take_while(|&c| c == '=').count();
        if level == 0 || level > 6 {
            return None;
        }

        // Check for matching trailing `=`
        let content = trimmed.trim_start_matches('=').trim_end_matches('=').trim();

        let inlines = self.parse_inline(content);
        Some(Block::Heading { level: level as u8, inlines, span: Span::NONE })
    }

    fn parse_list(&self, lines: &[&str]) -> (Block, usize) {
        let mut items: Vec<Vec<Block>> = Vec::new();
        let mut consumed = 0;
        let first_char = lines[0].trim().chars().next().unwrap_or('*');
        let ordered = first_char == '#';

        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                break;
            }

            // Check if this is a list item with the same marker
            let marker = if ordered { '#' } else { '*' };
            if !trimmed.starts_with(marker) {
                break;
            }

            // For simplicity, flatten nested items
            let content = trimmed.trim_start_matches(marker).trim();
            let inlines = self.parse_inline(content);
            items.push(vec![Block::Paragraph { inlines, span: Span::NONE }]);

            consumed += 1;
        }

        (Block::List { ordered, items, span: Span::NONE }, consumed.max(1))
    }

    fn parse_code_block(&self, lines: &[&str]) -> (Block, usize) {
        let mut content = String::new();
        let mut consumed = 0;

        for line in lines {
            if !line.starts_with(' ') && !line.is_empty() {
                break;
            }
            if !content.is_empty() {
                content.push('\n');
            }
            // Remove one leading space
            content.push_str(line.strip_prefix(' ').unwrap_or(line));
            consumed += 1;
        }

        (Block::CodeBlock { content, span: Span::NONE }, consumed.max(1))
    }

    fn parse_table(&self, lines: &[&str]) -> (Block, usize) {
        let mut rows = Vec::new();
        let mut consumed = 0;

        for line in lines {
            let trimmed = line.trim();

            if trimmed == "|}" {
                consumed += 1;
                break;
            }

            if trimmed.starts_with("|-") {
                // Table row marker
                consumed += 1;
                continue;
            }

            if trimmed.starts_with('|') || trimmed.starts_with('!') {
                // Parse cells in this line
                let is_header = trimmed.starts_with('!');
                let content = trimmed.trim_start_matches(['|', '!']);
                let cells_str: Vec<&str> = content.split("||").collect();
                let mut cells = Vec::new();

                for cell_content in cells_str {
                    let inlines = self.parse_inline(cell_content.trim());
                    cells.push(TableCell { is_header, inlines, span: Span::NONE });
                }

                if !cells.is_empty() {
                    rows.push(TableRow { cells, span: Span::NONE });
                }
            }

            consumed += 1;
        }

        (Block::Table { rows, span: Span::NONE }, consumed.max(1))
    }

    fn parse_paragraph(&self, lines: &[&str]) -> (Block, usize) {
        let mut text = String::new();
        let mut consumed = 0;

        for line in lines {
            let trimmed = line.trim();

            // Stop at empty lines, headings, lists, rules, tables
            if trimmed.is_empty()
                || trimmed.starts_with('=')
                || trimmed.starts_with('*')
                || trimmed.starts_with('#')
                || trimmed == "----"
                || (trimmed.chars().all(|c| c == '-') && trimmed.len() >= 4)
                || trimmed.starts_with("{|")
                || trimmed == "|}"
                || trimmed.starts_with("|-")
                || trimmed.starts_with('|')
                || trimmed.starts_with('!')
            {
                break;
            }

            if !text.is_empty() {
                text.push(' ');
            }
            text.push_str(trimmed);
            consumed += 1;
        }

        let inlines = self.parse_inline(&text);
        (Block::Paragraph { inlines, span: Span::NONE }, consumed.max(1))
    }

    #[allow(clippy::only_used_in_recursion)]
    fn parse_inline(&self, text: &str) -> Vec<Inline> {
        let mut inlines = Vec::new();
        let mut current_text = String::new();
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // Bold: '''text'''
            if i + 2 < chars.len()
                && chars[i] == '\''
                && chars[i + 1] == '\''
                && chars[i + 2] == '\''
            {
                if !current_text.is_empty() {
                    inlines.push(Inline::Text(current_text.clone()));
                    current_text.clear();
                }

                // Find closing '''
                let start = i + 3;
                let mut end = start;
                while end + 2 < chars.len() {
                    if chars[end] == '\'' && chars[end + 1] == '\'' && chars[end + 2] == '\'' {
                        break;
                    }
                    end += 1;
                }

                if end + 2 < chars.len() {
                    let inner: String = chars[start..end].iter().collect();
                    let inner_inlines = self.parse_inline(&inner);
                    inlines.push(Inline::Bold(inner_inlines));
                    i = end + 3;
                    continue;
                }
            }

            // Italic: ''text''
            if i + 1 < chars.len() && chars[i] == '\'' && chars[i + 1] == '\'' {
                // Make sure it's not bold
                if i + 2 < chars.len() && chars[i + 2] == '\'' {
                    // This is bold, handled above
                } else {
                    if !current_text.is_empty() {
                        inlines.push(Inline::Text(current_text.clone()));
                        current_text.clear();
                    }

                    // Find closing ''
                    let start = i + 2;
                    let mut end = start;
                    while end + 1 < chars.len() {
                        if chars[end] == '\'' && chars[end + 1] == '\'' {
                            // Make sure it's not '''
                            if end + 2 < chars.len() && chars[end + 2] == '\'' {
                                end += 1;
                                continue;
                            }
                            break;
                        }
                        end += 1;
                    }

                    if end + 1 < chars.len() {
                        let inner: String = chars[start..end].iter().collect();
                        let inner_inlines = self.parse_inline(&inner);
                        inlines.push(Inline::Italic(inner_inlines));
                        i = end + 2;
                        continue;
                    }
                }
            }

            // Internal link: [[Title]] or [[Title|text]]
            if i + 1 < chars.len() && chars[i] == '[' && chars[i + 1] == '[' {
                if !current_text.is_empty() {
                    inlines.push(Inline::Text(current_text.clone()));
                    current_text.clear();
                }

                // Find closing ]]
                let start = i + 2;
                let mut end = start;
                while end + 1 < chars.len() {
                    if chars[end] == ']' && chars[end + 1] == ']' {
                        break;
                    }
                    end += 1;
                }

                if end + 1 < chars.len() {
                    let inner: String = chars[start..end].iter().collect();
                    let (url, text) = if let Some(pipe_pos) = inner.find('|') {
                        let url = &inner[..pipe_pos];
                        let text = &inner[pipe_pos + 1..];
                        (url.to_string(), text.to_string())
                    } else {
                        (inner.clone(), inner)
                    };

                    inlines.push(Inline::Link { url, text });
                    i = end + 2;
                    continue;
                }
            }

            // External link: [url text]
            if chars[i] == '[' && (i + 1 >= chars.len() || chars[i + 1] != '[') {
                if !current_text.is_empty() {
                    inlines.push(Inline::Text(current_text.clone()));
                    current_text.clear();
                }

                // Find closing ]
                let start = i + 1;
                let mut end = start;
                while end < chars.len() && chars[end] != ']' {
                    end += 1;
                }

                if end < chars.len() {
                    let inner: String = chars[start..end].iter().collect();
                    let parts: Vec<&str> = inner.splitn(2, ' ').collect();
                    let url = parts[0].to_string();
                    let text = if parts.len() > 1 {
                        parts[1].to_string()
                    } else {
                        url.clone()
                    };

                    inlines.push(Inline::Link { url, text });
                    i = end + 1;
                    continue;
                }
            }

            // <code>...</code>
            if i + 5 < chars.len() && &chars[i..i + 6].iter().collect::<String>() == "<code>" {
                if !current_text.is_empty() {
                    inlines.push(Inline::Text(current_text.clone()));
                    current_text.clear();
                }

                let start = i + 6;
                let mut end = start;
                while end + 6 < chars.len() {
                    if &chars[end..end + 7].iter().collect::<String>() == "</code>" {
                        break;
                    }
                    end += 1;
                }

                if end + 6 < chars.len() {
                    let code_text: String = chars[start..end].iter().collect();
                    inlines.push(Inline::Code(code_text));
                    i = end + 7;
                    continue;
                }
            }

            // Regular character
            current_text.push(chars[i]);
            i += 1;
        }

        if !current_text.is_empty() {
            inlines.push(Inline::Text(current_text));
        }

        inlines
    }
}
