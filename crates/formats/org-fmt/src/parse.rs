//! Org-mode parser.

use crate::ast::{
    Block, Diagnostic, Inline, ListItem, ListItemContent, OrgDoc, Severity, Span,
};

/// Parse an Org-mode string into an [`OrgDoc`].
///
/// Parsing is infallible — unknown constructs produce [`Diagnostic`]s instead
/// of hard errors.
pub fn parse(input: &str) -> (OrgDoc, Vec<Diagnostic>) {
    let mut p = OrgParser::new(input);
    let (blocks, metadata) = p.parse_document();
    let diagnostics = p.diagnostics;
    (OrgDoc { blocks, metadata }, diagnostics)
}

struct OrgParser<'a> {
    lines: Vec<&'a str>,
    pos: usize,
    pub diagnostics: Vec<Diagnostic>,
}

impl<'a> OrgParser<'a> {
    fn new(input: &'a str) -> Self {
        let lines: Vec<&str> = input.lines().collect();
        Self {
            lines,
            pos: 0,
            diagnostics: Vec::new(),
        }
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.lines.len()
    }

    fn current_line(&self) -> Option<&'a str> {
        self.lines.get(self.pos).copied()
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn parse_document(&mut self) -> (Vec<Block>, Vec<(String, String)>) {
        let mut blocks = Vec::new();
        let mut metadata = Vec::new();
        let mut current_para: Vec<String> = Vec::new();

        while !self.is_eof() {
            let line = self.current_line().unwrap();

            // Parse metadata (#+KEY: value) — not #+BEGIN_*
            if line.starts_with("#+") && !line.to_uppercase().starts_with("#+BEGIN") {
                if let Some((key, value)) = parse_metadata_line(line) {
                    metadata.push((key, value));
                }
                self.advance();
                continue;
            }

            // Blank line - end paragraph
            if line.trim().is_empty() {
                if !current_para.is_empty() {
                    let content = current_para.join(" ");
                    blocks.push(Block::Paragraph {
                        inlines: parse_inline_content(&content),
                        span: Span::NONE,
                    });
                    current_para.clear();
                }
                self.advance();
                continue;
            }

            // Heading
            if line.starts_with('*') && line.chars().find(|&c| c != '*') == Some(' ') {
                if !current_para.is_empty() {
                    let content = current_para.join(" ");
                    blocks.push(Block::Paragraph {
                        inlines: parse_inline_content(&content),
                        span: Span::NONE,
                    });
                    current_para.clear();
                }
                blocks.push(self.parse_heading());
                continue;
            }

            // Block elements
            if line.to_uppercase().starts_with("#+BEGIN_") {
                if !current_para.is_empty() {
                    let content = current_para.join(" ");
                    blocks.push(Block::Paragraph {
                        inlines: parse_inline_content(&content),
                        span: Span::NONE,
                    });
                    current_para.clear();
                }
                if let Some(block) = self.parse_block() {
                    blocks.push(block);
                }
                continue;
            }

            // List item
            if is_list_item(line) {
                if !current_para.is_empty() {
                    let content = current_para.join(" ");
                    blocks.push(Block::Paragraph {
                        inlines: parse_inline_content(&content),
                        span: Span::NONE,
                    });
                    current_para.clear();
                }
                blocks.push(self.parse_list());
                continue;
            }

            // Horizontal rule
            if line.trim() == "-----" || (line.chars().all(|c| c == '-') && line.len() >= 5) {
                if !current_para.is_empty() {
                    let content = current_para.join(" ");
                    blocks.push(Block::Paragraph {
                        inlines: parse_inline_content(&content),
                        span: Span::NONE,
                    });
                    current_para.clear();
                }
                blocks.push(Block::HorizontalRule { span: Span::NONE });
                self.advance();
                continue;
            }

            // Regular text - accumulate into current paragraph
            current_para.push(line.to_string());
            self.advance();
        }

        // Flush remaining paragraph
        if !current_para.is_empty() {
            let content = current_para.join(" ");
            blocks.push(Block::Paragraph {
                inlines: parse_inline_content(&content),
                span: Span::NONE,
            });
        }

        (blocks, metadata)
    }

    fn parse_heading(&mut self) -> Block {
        let line = self.current_line().unwrap();
        let level = line.chars().take_while(|&c| c == '*').count();
        let text = &line[level..];
        let text = text.trim();
        let text = strip_heading_metadata(text);
        self.advance();

        Block::Heading {
            level,
            inlines: parse_inline_content(&text),
            span: Span::NONE,
        }
    }

    fn parse_block(&mut self) -> Option<Block> {
        let orig_line = self.current_line()?;
        let line_upper = orig_line.to_uppercase();
        // Always advance past the BEGIN line, even on early return, to prevent
        // infinite loops when the block_type is missing (e.g. "#+BEGIN_" alone).
        let block_type_opt = line_upper
            .strip_prefix("#+BEGIN_")
            .and_then(|rest| rest.split_whitespace().next())
            .map(|s| s.to_uppercase());

        let block_type = match block_type_opt {
            Some(bt) => bt,
            None => {
                self.advance();
                return None;
            }
        };

        // Get language for SRC blocks
        let lang = if block_type == "SRC" {
            orig_line
                .to_uppercase()
                .strip_prefix("#+BEGIN_SRC")
                .and_then(|s| s.split_whitespace().next())
                .map(|s| {
                    // Use original case from original line
                    let upper_offset = "#+BEGIN_SRC".len();
                    let rest = &orig_line[upper_offset..];
                    rest.split_whitespace().next().unwrap_or(s).to_lowercase()
                })
        } else {
            None
        };

        self.advance();

        let end_marker = format!("#+END_{}", block_type);
        let mut content_lines = Vec::new();

        while !self.is_eof() {
            let line = self.current_line().unwrap();
            if line.to_uppercase().starts_with(&end_marker) {
                self.advance();
                break;
            }
            content_lines.push(line);
            self.advance();
        }

        let content_str = content_lines.join("\n");

        match block_type.as_str() {
            "SRC" => Some(Block::CodeBlock {
                language: lang.filter(|l| !l.is_empty()),
                content: content_str,
                span: Span::NONE,
            }),
            "QUOTE" => {
                let inlines = parse_inline_content(&content_str);
                Some(Block::Blockquote {
                    children: vec![Block::Paragraph {
                        inlines,
                        span: Span::NONE,
                    }],
                    span: Span::NONE,
                })
            }
            "EXAMPLE" | "VERSE" => Some(Block::CodeBlock {
                language: None,
                content: content_str,
                span: Span::NONE,
            }),
            "CENTER" => Some(Block::Div {
                inlines: parse_inline_content(&content_str),
                span: Span::NONE,
            }),
            _ => {
                self.diagnostics.push(Diagnostic {
                    span: Span::NONE,
                    severity: Severity::Warning,
                    message: format!("Unknown block type: {}", block_type),
                    code: "org:unknown-block",
                });
                None
            }
        }
    }

    fn parse_list(&mut self) -> Block {
        let first_line = self.current_line().unwrap();
        let indent = first_line.len() - first_line.trim_start().len();
        let ordered = is_ordered_list_item(first_line);

        let mut items = Vec::new();

        while !self.is_eof() {
            let line = self.current_line().unwrap();
            let line_indent = line.len() - line.trim_start().len();

            // Check if still part of list
            if line.trim().is_empty() {
                // Blank line might end the list or be between items
                self.advance();
                if self.is_eof() {
                    break;
                }
                let next = self.current_line().unwrap();
                let next_indent = next.len() - next.trim_start().len();
                if !is_list_item(next) || next_indent < indent {
                    break;
                }
                continue;
            }

            if line_indent < indent && !line.trim().is_empty() {
                break;
            }

            if is_list_item(line) && line_indent == indent {
                items.push(self.parse_list_item(indent));
            } else {
                break;
            }
        }

        Block::List {
            ordered,
            items,
            span: Span::NONE,
        }
    }

    fn parse_list_item(&mut self, base_indent: usize) -> ListItem {
        let line = self.current_line().unwrap();
        let trimmed = line.trim_start();

        // Extract item content (skip marker)
        let content = if trimmed.starts_with("- ") || trimmed.starts_with("+ ") {
            &trimmed[2..]
        } else {
            // Ordered list: skip "1. " or "1) "
            let idx = trimmed.find(['.', ')']).map(|i| i + 2).unwrap_or(0);
            if idx < trimmed.len() {
                &trimmed[idx..]
            } else {
                trimmed
            }
        };

        self.advance();

        // Collect continuation lines
        let mut full_content = content.to_string();
        while !self.is_eof() {
            let line = self.current_line().unwrap();
            if line.trim().is_empty() {
                break;
            }
            let line_indent = line.len() - line.trim_start().len();
            if line_indent <= base_indent && is_list_item(line) {
                break;
            }
            if line_indent > base_indent {
                full_content.push(' ');
                full_content.push_str(line.trim());
                self.advance();
            } else {
                break;
            }
        }

        let inlines = parse_inline_content(&full_content);
        ListItem {
            children: vec![ListItemContent::Inline(inlines)],
        }
    }
}

pub(crate) fn parse_metadata_line(line: &str) -> Option<(String, String)> {
    let line = line.strip_prefix("#+")?.trim();
    let (key, value) = line.split_once(':')?;
    Some((key.trim().to_lowercase(), value.trim().to_string()))
}

pub(crate) fn strip_heading_metadata(text: &str) -> String {
    let text = text.trim();

    // Remove TODO keywords
    let text = if text.starts_with("TODO ") || text.starts_with("DONE ") {
        &text[5..]
    } else {
        text
    };

    // Remove tags (like :tag1:tag2:)
    if let Some(idx) = text.rfind(" :")
        && text.ends_with(':')
    {
        return text[..idx].trim().to_string();
    }

    text.trim().to_string()
}

pub(crate) fn is_list_item(line: &str) -> bool {
    let trimmed = line.trim_start();
    // Unordered: - item, + item
    if trimmed.starts_with("- ") || trimmed.starts_with("+ ") {
        return true;
    }
    // Ordered: 1. item, 1) item
    if let Some(rest) = trimmed.strip_prefix(|c: char| c.is_ascii_digit()) {
        let rest = rest.trim_start_matches(|c: char| c.is_ascii_digit());
        return rest.starts_with(". ") || rest.starts_with(") ");
    }
    false
}

pub(crate) fn is_ordered_list_item(line: &str) -> bool {
    let trimmed = line.trim_start();
    if let Some(rest) = trimmed.strip_prefix(|c: char| c.is_ascii_digit()) {
        let rest = rest.trim_start_matches(|c: char| c.is_ascii_digit());
        rest.starts_with(". ") || rest.starts_with(") ")
    } else {
        false
    }
}

pub(crate) fn parse_inline_content(text: &str) -> Vec<Inline> {
    let mut nodes = Vec::new();
    let mut pos = 0;
    let chars: Vec<char> = text.chars().collect();

    while pos < chars.len() {
        let c = chars[pos];

        match c {
            // Bold: *text*
            '*' => {
                if let Some((content, end)) = find_inline_span(&chars, pos, '*') {
                    nodes.push(Inline::Bold(parse_inline_content(&content), Span::NONE));
                    pos = end + 1;
                    continue;
                }
            }
            // Italic: /text/
            '/' => {
                if let Some((content, end)) = find_inline_span(&chars, pos, '/') {
                    nodes.push(Inline::Italic(parse_inline_content(&content), Span::NONE));
                    pos = end + 1;
                    continue;
                }
            }
            // Underline: _text_
            '_' => {
                if let Some((content, end)) = find_inline_span(&chars, pos, '_') {
                    nodes.push(Inline::Underline(
                        parse_inline_content(&content),
                        Span::NONE,
                    ));
                    pos = end + 1;
                    continue;
                }
            }
            // Strikethrough: +text+
            '+' => {
                if let Some((content, end)) = find_inline_span(&chars, pos, '+') {
                    nodes.push(Inline::Strikethrough(
                        parse_inline_content(&content),
                        Span::NONE,
                    ));
                    pos = end + 1;
                    continue;
                }
            }
            // Code: ~text~ or =text=
            '~' | '=' => {
                if let Some((content, end)) = find_inline_span(&chars, pos, c) {
                    nodes.push(Inline::Code(content, Span::NONE));
                    pos = end + 1;
                    continue;
                }
            }
            // Link: [[url]] or [[url][description]]
            '[' => {
                if pos + 1 < chars.len()
                    && chars[pos + 1] == '['
                    && let Some((link_inline, end)) = parse_link(&chars, pos)
                {
                    nodes.push(link_inline);
                    pos = end;
                    continue;
                }
            }
            _ => {}
        }

        // Regular character - append to last text node or create new one
        match nodes.last_mut() {
            Some(Inline::Text { text, .. }) => {
                text.push(c);
            }
            _ => {
                nodes.push(Inline::Text {
                    text: c.to_string(),
                    span: Span::NONE,
                });
            }
        }
        pos += 1;
    }

    // Merge adjacent text nodes (already handled by appending to last)
    nodes
}

pub(crate) fn find_inline_span(
    chars: &[char],
    start: usize,
    marker: char,
) -> Option<(String, usize)> {
    if start + 2 >= chars.len() {
        return None;
    }

    // Opening marker must not be followed by whitespace
    if chars[start + 1].is_whitespace() {
        return None;
    }

    // Find closing marker
    for i in (start + 2)..chars.len() {
        if chars[i] == marker {
            // Closing marker must not be preceded by whitespace
            if !chars[i - 1].is_whitespace() {
                let content: String = chars[(start + 1)..i].iter().collect();
                return Some((content, i));
            }
        }
    }

    None
}

pub(crate) fn parse_link(chars: &[char], start: usize) -> Option<(Inline, usize)> {
    // Skip [[
    let mut pos = start + 2;
    let mut url = String::new();
    let mut description = String::new();
    let mut in_description = false;

    while pos < chars.len() {
        let c = chars[pos];
        if c == ']' {
            if pos + 1 < chars.len() && chars[pos + 1] == ']' {
                // End of link
                let children = if description.is_empty() {
                    vec![Inline::Text {
                        text: url.clone(),
                        span: Span::NONE,
                    }]
                } else {
                    parse_inline_content(&description)
                };
                return Some((
                    Inline::Link {
                        url,
                        children,
                        span: Span::NONE,
                    },
                    pos + 2,
                ));
            } else if pos + 1 < chars.len() && chars[pos + 1] == '[' {
                // Start of description
                in_description = true;
                pos += 2;
                continue;
            }
        }

        if in_description {
            description.push(c);
        } else {
            url.push(c);
        }
        pos += 1;
    }

    None
}

