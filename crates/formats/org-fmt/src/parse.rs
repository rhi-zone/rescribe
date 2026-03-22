//! Org-mode parser.

use crate::ast::{
    Block, CheckboxState, DefinitionItem, Diagnostic, Inline, ListItem, ListItemContent, OrgDoc,
    Severity, Span, TableRow,
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
        // Affiliated keyword #+NAME: carries over to the next block
        let mut pending_name: Option<String> = None;

        while !self.is_eof() {
            let line = self.current_line().unwrap();

            // #+CAPTION: text followed by [[file:image]] → Figure block
            if line.to_uppercase().starts_with("#+CAPTION:") {
                if !current_para.is_empty() {
                    blocks.push(Block::Paragraph {
                        inlines: parse_para_lines(&current_para),
                        span: Span::NONE,
                    });
                    current_para.clear();
                }
                let caption_text = line["#+CAPTION:".len()..].trim().to_string();
                self.advance();
                // Skip blank lines between caption and image
                while !self.is_eof() && self.current_line().unwrap().trim().is_empty() {
                    self.advance();
                }
                // Check if next line is an image link
                let mut made_figure = false;
                if let Some(img_line) = self.current_line() {
                    let trimmed = img_line.trim();
                    if trimmed.starts_with("[[") {
                        let img_chars: Vec<char> = trimmed.chars().collect();
                        if let Some((Inline::Image { url, .. }, _)) = parse_link(&img_chars, 0) {
                            let caption_inlines = parse_inline_content(&caption_text);
                            let img_inline = Inline::Image { url, span: Span::NONE };
                            blocks.push(Block::Figure {
                                children: vec![
                                    Block::Caption {
                                        inlines: caption_inlines,
                                        span: Span::NONE,
                                    },
                                    Block::Paragraph {
                                        inlines: vec![img_inline],
                                        span: Span::NONE,
                                    },
                                ],
                                span: Span::NONE,
                            });
                            self.advance();
                            made_figure = true;
                        }
                    }
                }
                if !made_figure {
                    // No image follows — emit standalone caption
                    blocks.push(Block::Caption {
                        inlines: parse_inline_content(&caption_text),
                        span: Span::NONE,
                    });
                }
                continue;
            }

            // Parse metadata (#+KEY: value) — not #+BEGIN_*
            if line.starts_with("#+") && !line.to_uppercase().starts_with("#+BEGIN") {
                if let Some((key, value)) = parse_metadata_line(line) {
                    if key == "name" {
                        // #+NAME: is an affiliated keyword; carry it to the next block
                        pending_name = Some(value);
                    } else {
                        metadata.push((key, value));
                    }
                }
                self.advance();
                continue;
            }

            // Blank line - end paragraph
            if line.trim().is_empty() {
                if !current_para.is_empty() {
                    blocks.push(Block::Paragraph {
                        inlines: parse_para_lines(&current_para),
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
                    blocks.push(Block::Paragraph {
                        inlines: parse_para_lines(&current_para),
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
                    blocks.push(Block::Paragraph {
                        inlines: parse_para_lines(&current_para),
                        span: Span::NONE,
                    });
                    current_para.clear();
                }
                if let Some(mut block) = self.parse_block() {
                    // Attach #+NAME: affiliated keyword to the block
                    if let Some(name) = pending_name.take()
                        && let Block::CodeBlock { name: ref mut n, .. } = block
                    {
                        *n = Some(name);
                    }
                    blocks.push(block);
                } else {
                    pending_name = None;
                }
                continue;
            }

            // Definition list: - TERM :: DESCRIPTION
            if is_definition_list_item(line) {
                if !current_para.is_empty() {
                    blocks.push(Block::Paragraph {
                        inlines: parse_para_lines(&current_para),
                        span: Span::NONE,
                    });
                    current_para.clear();
                }
                blocks.push(self.parse_definition_list());
                continue;
            }

            // List item
            if is_list_item(line) {
                if !current_para.is_empty() {
                    blocks.push(Block::Paragraph {
                        inlines: parse_para_lines(&current_para),
                        span: Span::NONE,
                    });
                    current_para.clear();
                }
                blocks.push(self.parse_list());
                continue;
            }

            // Table: lines starting with |
            if line.starts_with('|') {
                if !current_para.is_empty() {
                    blocks.push(Block::Paragraph {
                        inlines: parse_para_lines(&current_para),
                        span: Span::NONE,
                    });
                    current_para.clear();
                }
                blocks.push(self.parse_table());
                continue;
            }

            // Horizontal rule
            if line.trim() == "-----" || (line.chars().all(|c| c == '-') && line.len() >= 5) {
                if !current_para.is_empty() {
                    blocks.push(Block::Paragraph {
                        inlines: parse_para_lines(&current_para),
                        span: Span::NONE,
                    });
                    current_para.clear();
                }
                blocks.push(Block::HorizontalRule { span: Span::NONE });
                self.advance();
                continue;
            }

            // Comment line: "# " (single hash + space, not "#+")
            if (line.starts_with("# ") || line == "#") && !line.starts_with("#+") {
                // Org comment lines are silently dropped
                self.advance();
                continue;
            }

            // Fixed-width area: ": " prefix (colon + space) or lone ":"
            if line.starts_with(": ") || line == ":" {
                if !current_para.is_empty() {
                    blocks.push(Block::Paragraph {
                        inlines: parse_para_lines(&current_para),
                        span: Span::NONE,
                    });
                    current_para.clear();
                }
                let block = self.parse_fixed_width();
                blocks.push(block);
                continue;
            }

            // Drawer: ":NAME:" on its own line (colon-identifier-colon)
            if is_drawer_start(line) {
                if !current_para.is_empty() {
                    blocks.push(Block::Paragraph {
                        inlines: parse_para_lines(&current_para),
                        span: Span::NONE,
                    });
                    current_para.clear();
                }
                let drawer_name = line.trim().trim_matches(':').to_string();
                self.skip_drawer();
                self.diagnostics.push(Diagnostic {
                    span: Span::NONE,
                    severity: Severity::Info,
                    message: format!("Drawer :{}: dropped", drawer_name),
                    code: "org:drawer-dropped",
                });
                continue;
            }

            // Regular text - accumulate into current paragraph
            current_para.push(line.to_string());
            self.advance();
        }

        // Flush remaining paragraph
        if !current_para.is_empty() {
            blocks.push(Block::Paragraph {
                inlines: parse_para_lines(&current_para),
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
        let (todo, priority, tags, title) = parse_heading_metadata(text);
        self.advance();

        // Look ahead: immediately after a heading, consume optional PROPERTIES drawer
        // and/or SCHEDULED:/DEADLINE: planning lines.
        let mut properties: Vec<(String, String)> = Vec::new();
        let mut scheduled: Option<String> = None;
        let mut deadline: Option<String> = None;

        // Parse :PROPERTIES: drawer if present
        if let Some(next) = self.current_line()
            && next.trim().eq_ignore_ascii_case(":PROPERTIES:")
        {
            self.advance();
            while !self.is_eof() {
                let prop_line = self.current_line().unwrap();
                if prop_line.trim().eq_ignore_ascii_case(":END:") {
                    self.advance();
                    break;
                }
                let prop_trimmed = prop_line.trim();
                // :KEY: value
                if let Some(rest) = prop_trimmed.strip_prefix(':')
                    && let Some(colon2) = rest.find(':')
                {
                    let key = rest[..colon2].trim().to_string();
                    let val = rest[colon2 + 1..].trim().to_string();
                    if !key.is_empty() {
                        properties.push((key, val));
                    }
                }
                self.advance();
            }
        }

        // Parse SCHEDULED: and DEADLINE: planning lines (may appear in any order)
        loop {
            match self.current_line() {
                Some(line) if line.trim_start().starts_with("SCHEDULED:") || line.trim_start().starts_with("DEADLINE:") => {
                    let trimmed = line.trim();
                    if let Some(rest) = trimmed.strip_prefix("SCHEDULED:") {
                        scheduled = Some(rest.trim().to_string());
                    } else if let Some(rest) = trimmed.strip_prefix("DEADLINE:") {
                        deadline = Some(rest.trim().to_string());
                    }
                    self.advance();
                }
                _ => break,
            }
        }

        Block::Heading {
            level,
            todo,
            priority,
            tags,
            properties,
            scheduled,
            deadline,
            inlines: parse_inline_content(&title),
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

        // Get language and header args for SRC blocks
        let (lang, header_args) = if block_type == "SRC" {
            let upper_offset = "#+BEGIN_SRC".len();
            let rest = &orig_line[upper_offset..].trim_start();
            let mut parts = rest.splitn(2, |c: char| c.is_whitespace());
            let lang_str = parts.next().filter(|s| !s.is_empty()).map(|s| s.to_lowercase());
            let args = parts.next().and_then(|s| {
                let s = s.trim();
                if s.is_empty() { None } else { Some(s.to_string()) }
            });
            (lang_str, args)
        } else {
            (None, None)
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
                header_args,
                name: None, // filled by caller if #+NAME: was present
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
                header_args: None,
                name: None,
                content: content_str,
                span: Span::NONE,
            }),
            "CENTER" => Some(Block::Div {
                inlines: parse_inline_content(&content_str),
                span: Span::NONE,
            }),
            "COMMENT" => {
                // Comment blocks are silently dropped (no diagnostic)
                None
            }
            "EXPORT" => {
                // #+BEGIN_EXPORT format ... #+END_EXPORT → raw_block with format prop
                let format = orig_line
                    .to_uppercase()
                    .strip_prefix("#+BEGIN_EXPORT")
                    .and_then(|s| s.split_whitespace().next())
                    .map(|s| {
                        // Use original case
                        let upper_offset = "#+BEGIN_EXPORT".len();
                        let rest = &orig_line[upper_offset..];
                        rest.split_whitespace().next().unwrap_or(s).to_lowercase()
                    })
                    .unwrap_or_else(|| "unknown".to_string());
                Some(Block::RawBlock {
                    format,
                    content: content_str,
                    span: Span::NONE,
                })
            }
            _ => {
                // Special block: #+BEGIN_NAME...#+END_NAME — emit as Div
                // (unknown custom containers)
                Some(Block::Div {
                    inlines: parse_inline_content(&content_str),
                    span: Span::NONE,
                })
            }
        }
    }

    fn parse_list(&mut self) -> Block {
        let first_line = self.current_line().unwrap();
        let indent = first_line.len() - first_line.trim_start().len();
        let ordered = is_ordered_list_item(first_line);

        let mut items = Vec::new();
        let mut start: Option<u64> = None;
        let mut first = true;

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
                let (item, counter) = self.parse_list_item_with_counter(indent);
                // The counter cookie [@N] on the first item sets the list start
                if first {
                    start = counter;
                    first = false;
                }
                items.push(item);
            } else {
                break;
            }
        }

        Block::List {
            ordered,
            start,
            items,
            span: Span::NONE,
        }
    }

    /// Parse a list item, returning `(item, counter_start)` where `counter_start`
    /// is `Some(n)` if the item had a `[@n]` counter cookie.
    fn parse_list_item_with_counter(&mut self, base_indent: usize) -> (ListItem, Option<u64>) {
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

        // Strip counter cookie [@N] from ordered list items
        let (counter, content) = parse_counter_cookie(content);

        // Detect checkbox prefix: "[ ] ", "[X] ", "[-] "
        let (checkbox, content) = parse_checkbox(content);

        self.advance();

        // Collect content lines and sub-lists
        let mut children: Vec<ListItemContent> = Vec::new();
        let mut inline_lines: Vec<&str> = vec![content];

        while !self.is_eof() {
            let line = self.current_line().unwrap();
            if line.trim().is_empty() {
                break;
            }
            let line_indent = line.len() - line.trim_start().len();
            if line_indent <= base_indent {
                break;
            }
            // Indented sub-list
            if is_list_item(line) && line_indent > base_indent {
                // Flush accumulated inline text first
                if !inline_lines.is_empty() {
                    let full = inline_lines.join(" ");
                    let inlines = parse_inline_content(&full);
                    children.push(ListItemContent::Inline(inlines));
                    inline_lines.clear();
                }
                children.push(ListItemContent::Block(self.parse_list()));
            } else if line_indent > base_indent {
                inline_lines.push(line.trim());
                self.advance();
            } else {
                break;
            }
        }

        // Flush any remaining inline text
        if !inline_lines.is_empty() {
            let full = inline_lines.join(" ");
            let inlines = parse_inline_content(&full);
            children.push(ListItemContent::Inline(inlines));
        }

        (ListItem { children, checkbox }, counter)
    }

    fn parse_table(&mut self) -> Block {
        let mut rows = Vec::new();

        while !self.is_eof() {
            let line = self.current_line().unwrap();
            if !line.starts_with('|') {
                break;
            }

            // Separator line (|---+---|): mark previous row as header
            let inner = line.trim_start_matches('|').trim_end_matches('|');
            if !inner.is_empty() && inner.chars().all(|c| matches!(c, '-' | '+' | ' ')) {
                if let Some(last) = rows.last_mut() {
                    let last: &mut TableRow = last;
                    last.is_header = true;
                }
                self.advance();
                continue;
            }

            // Data row
            let cells = parse_table_row(line);
            rows.push(TableRow { cells, is_header: false });
            self.advance();
        }

        Block::Table { rows, span: Span::NONE }
    }

    fn parse_definition_list(&mut self) -> Block {
        let mut items = Vec::new();

        while !self.is_eof() {
            let line = self.current_line().unwrap();
            if !is_definition_list_item(line) {
                break;
            }

            let trimmed = line.trim_start();
            // Strip leading "- "
            let content = &trimmed[2..];
            let (term_str, desc_str) = content.split_once(" :: ").unwrap_or((content, ""));
            items.push(DefinitionItem {
                term: parse_inline_content(term_str.trim()),
                desc: parse_inline_content(desc_str.trim()),
            });
            self.advance();
        }

        Block::DefinitionList { items, span: Span::NONE }
    }

    /// Parse consecutive fixed-width lines (`: text`) into a CodeBlock.
    fn parse_fixed_width(&mut self) -> Block {
        let mut content_lines: Vec<&str> = Vec::new();
        while !self.is_eof() {
            let line = self.current_line().unwrap();
            if let Some(stripped) = line.strip_prefix(": ") {
                content_lines.push(stripped);
                self.advance();
            } else if line == ":" {
                content_lines.push("");
                self.advance();
            } else {
                break;
            }
        }
        let content = content_lines.join("\n");
        Block::CodeBlock { language: None, header_args: None, name: None, content, span: Span::NONE }
    }

    /// Skip drawer content up to and including the `:END:` line.
    fn skip_drawer(&mut self) {
        // Advance past the `:NAME:` line
        self.advance();
        while !self.is_eof() {
            let line = self.current_line().unwrap();
            self.advance();
            if line.trim().eq_ignore_ascii_case(":END:") {
                break;
            }
        }
    }
}

pub(crate) fn parse_metadata_line(line: &str) -> Option<(String, String)> {
    let line = line.strip_prefix("#+")?.trim();
    let (key, value) = line.split_once(':')?;
    Some((key.trim().to_lowercase(), value.trim().to_string()))
}

/// Parse heading metadata: TODO keyword, priority cookie, tags, and remaining title.
/// Returns `(todo, priority, tags, title)`.
pub(crate) fn parse_heading_metadata(
    text: &str,
) -> (Option<String>, Option<String>, Vec<String>, String) {
    let text = text.trim();

    // Extract TODO/DONE keyword (known keywords followed by space, at start)
    let (todo, text) = {
        let words: Vec<&str> = text.splitn(2, ' ').collect();
        if words.len() == 2 {
            let w = words[0];
            if w == "TODO" || w == "DONE" || w == "NEXT" || w == "WAITING"
                || w == "CANCELLED" || w == "HOLD" || w == "STARTED" || w == "COMMENT"
            {
                (Some(w.to_string()), words[1])
            } else {
                (None, text)
            }
        } else {
            (None, text)
        }
    };

    // Extract priority cookie [#A], [#B], etc.
    let (priority, text) = if text.starts_with("[#") {
        if let Some(end) = text.find(']') {
            let p = text[2..end].to_string();
            let rest = text[end + 1..].trim_start();
            (Some(p), rest)
        } else {
            (None, text)
        }
    } else {
        (None, text)
    };

    // Extract tags at end of line: "  :tag1:tag2:"
    let (tags, text) = if text.ends_with(':') {
        if let Some(idx) = text.rfind(" :") {
            // Guard: idx + 2 must not exceed text.len() - 1 (the trailing ':').
            // If the ' :' found by rfind IS the trailing ':', tag_str would be
            // a backwards slice — treat as no tags in that case.
            let tag_end = text.len() - 1;
            if idx + 2 <= tag_end {
                let tag_str = &text[idx + 2..tag_end];
                let tags: Vec<String> = tag_str.split(':').map(|t| t.to_string()).collect();
                let title = text[..idx].trim();
                (tags, title)
            } else {
                (vec![], text)
            }
        } else {
            (vec![], text)
        }
    } else {
        (vec![], text)
    };

    (todo, priority, tags, text.trim().to_string())
}

/// Parse a sequence of paragraph lines into inlines, inserting LineBreak nodes
/// where lines end with `\\`.
pub(crate) fn parse_para_lines(lines: &[String]) -> Vec<Inline> {
    let mut result = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let last = i == lines.len() - 1;
        if line.ends_with("\\\\") {
            let content = &line[..line.len() - 2];
            result.extend(parse_inline_content(content));
            result.push(Inline::LineBreak { span: Span::NONE });
        } else {
            result.extend(parse_inline_content(line));
            if !last {
                result.push(Inline::SoftBreak { span: Span::NONE });
            }
        }
    }
    result
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

pub(crate) fn is_definition_list_item(line: &str) -> bool {
    let trimmed = line.trim_start();
    (trimmed.starts_with("- ") || trimmed.starts_with("+ ")) && trimmed.contains(" :: ")
}

/// Returns true if `line` looks like a drawer start: `:NAME:` (colon-word-colon).
/// Must not match `:END:` (end marker) or fixed-width lines (`: text`).
fn is_drawer_start(line: &str) -> bool {
    let trimmed = line.trim();
    if !trimmed.starts_with(':') || !trimmed.ends_with(':') || trimmed.len() < 3 {
        return false;
    }
    if trimmed.eq_ignore_ascii_case(":END:") {
        return false;
    }
    let inner = &trimmed[1..trimmed.len() - 1];
    !inner.is_empty() && !inner.contains(' ') && !inner.contains(':')
}

/// Parse an optional `[@N]` counter cookie from the beginning of `content`.
/// Returns `(counter_value, remaining_content)`.
fn parse_counter_cookie(content: &str) -> (Option<u64>, &str) {
    if let Some(rest) = content.strip_prefix("[@")
        && let Some(close) = rest.find(']')
    {
        let num_str = &rest[..close];
        if let Ok(n) = num_str.parse::<u64>() {
            let after = rest[close + 1..].trim_start();
            return (Some(n), after);
        }
    }
    (None, content)
}

/// Parse a `[ ]` / `[X]` / `[-]` checkbox prefix from the beginning of content.
/// Returns `(checkbox_state, remaining_content)`.
fn parse_checkbox(content: &str) -> (Option<CheckboxState>, &str) {
    if let Some(rest) = content.strip_prefix("[ ] ") {
        return (Some(CheckboxState::Unchecked), rest);
    }
    if let Some(rest) = content.strip_prefix("[X] ") {
        return (Some(CheckboxState::Checked), rest);
    }
    if let Some(rest) = content.strip_prefix("[-] ") {
        return (Some(CheckboxState::Partial), rest);
    }
    (None, content)
}

fn parse_table_row(line: &str) -> Vec<Vec<Inline>> {
    let line = line.trim();
    let line = line.strip_prefix('|').unwrap_or(line);
    let line = line.strip_suffix('|').unwrap_or(line);
    line.split('|')
        .map(|cell| parse_inline_content(cell.trim()))
        .collect()
}

/// Find the content inside `{...}` starting at `open` where `chars[open] == '{'`.
/// Returns (content, index_of_closing_brace).
fn find_brace_span(chars: &[char], open: usize) -> Option<(String, usize)> {
    debug_assert!(chars.get(open) == Some(&'{'));
    let mut depth = 1usize;
    let mut i = open + 1;
    while i < chars.len() {
        match chars[i] {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    let content: String = chars[(open + 1)..i].iter().collect();
                    return Some((content, i));
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
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
            // Subscript: _{text}  /  Underline: _text_
            '_' => {
                if pos + 1 < chars.len()
                    && chars[pos + 1] == '{'
                    && let Some((content, end)) = find_brace_span(&chars, pos + 1)
                {
                    nodes.push(Inline::Subscript(
                        parse_inline_content(&content),
                        Span::NONE,
                    ));
                    pos = end + 1;
                    continue;
                }
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
            // Superscript: ^{text}
            '^' => {
                if pos + 1 < chars.len()
                    && chars[pos + 1] == '{'
                    && let Some((content, end)) = find_brace_span(&chars, pos + 1)
                {
                    nodes.push(Inline::Superscript(
                        parse_inline_content(&content),
                        Span::NONE,
                    ));
                    pos = end + 1;
                    continue;
                }
            }
            // Math inline: $source$
            // Per Org-mode spec: opening $ must not be immediately followed by a
            // digit (that would be a currency sign like $20, not math).
            '$' => {
                let next_is_digit = chars
                    .get(pos + 1)
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false);
                if !next_is_digit
                    && let Some((content, end)) = find_inline_span(&chars, pos, '$')
                {
                    nodes.push(Inline::MathInline {
                        source: content,
                        span: Span::NONE,
                    });
                    pos = end + 1;
                    continue;
                }
            }
            // Footnote ref: [fn:label]  /  Link: [[url]] or [[url][description]]
            // Inactive timestamp: [YYYY-MM-DD ...]
            '[' => {
                // Footnote reference: [fn:LABEL]
                if chars[pos..].starts_with(&['[', 'f', 'n', ':'])
                    && let Some((footnote_inline, end)) = parse_footnote_ref(&chars, pos)
                {
                    nodes.push(footnote_inline);
                    pos = end;
                    continue;
                }
                // Link: [[url]] or [[url][description]]
                if pos + 1 < chars.len()
                    && chars[pos + 1] == '['
                    && let Some((link_inline, end)) = parse_link(&chars, pos)
                {
                    nodes.push(link_inline);
                    pos = end;
                    continue;
                }
                // Inactive timestamp: [YYYY-MM-DD ...]
                if let Some((ts, end)) = parse_timestamp_inactive(&chars, pos) {
                    nodes.push(ts);
                    pos = end;
                    continue;
                }
            }
            // Active timestamp: <YYYY-MM-DD ...>
            '<' => {
                if let Some((ts, end)) = parse_timestamp_active(&chars, pos) {
                    nodes.push(ts);
                    pos = end;
                    continue;
                }
            }
            // Export snippet: @@backend:content@@
            '@' => {
                if pos + 1 < chars.len()
                    && chars[pos + 1] == '@'
                    && let Some((snippet, end)) = parse_export_snippet(&chars, pos)
                {
                    nodes.push(snippet);
                    pos = end;
                    continue;
                }
            }
            // LaTeX fragment: \(...\) or entity: \word
            '\\' => {
                if pos + 1 < chars.len() {
                    // \(...\) inline math
                    if chars[pos + 1] == '('
                        && let Some((math, end)) = parse_latex_fragment(&chars, pos)
                    {
                        nodes.push(math);
                        pos = end;
                        continue;
                    }
                    // \word entity (letter start)
                    if chars[pos + 1].is_alphabetic()
                        && let Some((entity, end)) = parse_entity(&chars, pos)
                    {
                        nodes.push(entity);
                        pos = end;
                        continue;
                    }
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
                // End of link — check if URL is an image (no description)
                if description.is_empty() && is_image_url(&url) {
                    return Some((Inline::Image { url, span: Span::NONE }, pos + 2));
                }
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

/// Returns true if `url` looks like an image file reference.
fn is_image_url(url: &str) -> bool {
    let path = url.strip_prefix("file:").unwrap_or(url);
    let lower = path.to_lowercase();
    matches!(
        std::path::Path::new(&lower)
            .extension()
            .and_then(|e| e.to_str()),
        Some("png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "bmp" | "tiff")
    )
}

/// Parse `[fn:LABEL]` or `[fn:LABEL: content]` at `start`.
///
/// - `[fn:label]` → `FootnoteRef`
/// - `[fn:label: content]` or `[fn:: content]` → `FootnoteDefinition`
fn parse_footnote_ref(chars: &[char], start: usize) -> Option<(Inline, usize)> {
    // chars[start] == '[', chars[start+1..start+4] == "fn:"
    debug_assert!(chars[start..].starts_with(&['[', 'f', 'n', ':']));
    let inner_start = start + 4; // after "[fn:"
    let close = chars[inner_start..].iter().position(|&c| c == ']')?;
    let inner: String = chars[inner_start..(inner_start + close)].iter().collect();
    let end_pos = inner_start + close + 1;

    // Inline definition: [fn:label: content] or [fn:: content] (anonymous)
    if let Some(colon_pos) = inner.find(':') {
        let label = inner[..colon_pos].to_string();
        let content = inner[colon_pos + 1..].trim_start().to_string();
        return Some((
            Inline::FootnoteDefinition {
                label,
                children: parse_inline_content(&content),
                span: Span::NONE,
            },
            end_pos,
        ));
    }

    // Regular reference: [fn:label]
    if inner.is_empty() {
        return None;
    }
    Some((Inline::FootnoteRef { label: inner, span: Span::NONE }, end_pos))
}

/// Parse active timestamp `<YYYY-MM-DD ...>` at `start`.
fn parse_timestamp_active(chars: &[char], start: usize) -> Option<(Inline, usize)> {
    debug_assert_eq!(chars[start], '<');
    if start + 10 >= chars.len() {
        return None;
    }
    // Must start with a digit (year)
    if !chars[start + 1].is_ascii_digit() {
        return None;
    }
    let close = chars[start + 1..].iter().position(|&c| c == '>')?;
    let value: String = chars[start + 1..start + 1 + close].iter().collect();
    if value.len() < 10 {
        return None;
    }
    Some((
        Inline::Timestamp { active: true, value, span: Span::NONE },
        start + 1 + close + 1,
    ))
}

/// Parse inactive timestamp `[YYYY-MM-DD ...]` at `start`.
fn parse_timestamp_inactive(chars: &[char], start: usize) -> Option<(Inline, usize)> {
    debug_assert_eq!(chars[start], '[');
    // Must start with a digit after [
    if start + 1 >= chars.len() || !chars[start + 1].is_ascii_digit() {
        return None;
    }
    let close = chars[start + 1..].iter().position(|&c| c == ']')?;
    let value: String = chars[start + 1..start + 1 + close].iter().collect();
    if value.len() < 10 {
        return None;
    }
    Some((
        Inline::Timestamp { active: false, value, span: Span::NONE },
        start + 1 + close + 1,
    ))
}

/// Parse export snippet `@@backend:content@@` at `start`.
fn parse_export_snippet(chars: &[char], start: usize) -> Option<(Inline, usize)> {
    debug_assert!(chars[start] == '@' && chars.get(start + 1) == Some(&'@'));
    let inner_start = start + 2;
    let mut i = inner_start;
    while i + 1 < chars.len() {
        if chars[i] == '@' && chars[i + 1] == '@' {
            let inner: String = chars[inner_start..i].iter().collect();
            if let Some(colon) = inner.find(':') {
                let backend = inner[..colon].to_string();
                let value = inner[colon + 1..].to_string();
                return Some((
                    Inline::ExportSnippet { backend, value, span: Span::NONE },
                    i + 2,
                ));
            }
            return None;
        }
        i += 1;
    }
    None
}

/// Parse LaTeX fragment `\(...\)` at `start`.
fn parse_latex_fragment(chars: &[char], start: usize) -> Option<(Inline, usize)> {
    debug_assert!(chars[start] == '\\' && chars.get(start + 1) == Some(&'('));
    let inner_start = start + 2;
    let mut i = inner_start;
    while i + 1 < chars.len() {
        if chars[i] == '\\' && chars[i + 1] == ')' {
            let source: String = chars[inner_start..i].iter().collect();
            return Some((Inline::MathInline { source, span: Span::NONE }, i + 2));
        }
        i += 1;
    }
    None
}

/// Parse Org entity `\name` or `\name{}` at `start`. Returns a Text node with the Unicode expansion.
fn parse_entity(chars: &[char], start: usize) -> Option<(Inline, usize)> {
    debug_assert_eq!(chars[start], '\\');
    let name_start = start + 1;
    let mut end = name_start;
    while end < chars.len() && chars[end].is_alphabetic() {
        end += 1;
    }
    if end == name_start {
        return None;
    }
    let name: String = chars[name_start..end].iter().collect();
    // Optional trailing {} (e.g. \alpha{} to prevent letter-joining)
    let end = if end + 1 < chars.len() && chars[end] == '{' && chars[end + 1] == '}' {
        end + 2
    } else {
        end
    };
    let text = org_entity_to_unicode(&name).unwrap_or_else(|| format!("\\{}", name));
    Some((Inline::Text { text, span: Span::NONE }, end))
}

/// Map common Org entity names to their Unicode representations.
fn org_entity_to_unicode(name: &str) -> Option<String> {
    Some(
        match name {
            "alpha" => "\u{03B1}",
            "beta" => "\u{03B2}",
            "gamma" => "\u{03B3}",
            "delta" => "\u{03B4}",
            "epsilon" => "\u{03B5}",
            "zeta" => "\u{03B6}",
            "eta" => "\u{03B7}",
            "theta" => "\u{03B8}",
            "iota" => "\u{03B9}",
            "kappa" => "\u{03BA}",
            "lambda" => "\u{03BB}",
            "mu" => "\u{03BC}",
            "nu" => "\u{03BD}",
            "xi" => "\u{03BE}",
            "pi" => "\u{03C0}",
            "rho" => "\u{03C1}",
            "sigma" => "\u{03C3}",
            "tau" => "\u{03C4}",
            "upsilon" => "\u{03C5}",
            "phi" => "\u{03C6}",
            "chi" => "\u{03C7}",
            "psi" => "\u{03C8}",
            "omega" => "\u{03C9}",
            "Alpha" => "\u{0391}",
            "Beta" => "\u{0392}",
            "Gamma" => "\u{0393}",
            "Delta" => "\u{0394}",
            "Theta" => "\u{0398}",
            "Lambda" => "\u{039B}",
            "Pi" => "\u{03A0}",
            "Sigma" => "\u{03A3}",
            "Omega" => "\u{03A9}",
            "nbsp" => "\u{00A0}",
            "shy" => "\u{00AD}",
            "copy" => "\u{00A9}",
            "reg" => "\u{00AE}",
            "trade" => "\u{2122}",
            "mdash" => "\u{2014}",
            "ndash" => "\u{2013}",
            "laquo" => "\u{00AB}",
            "raquo" => "\u{00BB}",
            "ldquo" => "\u{201C}",
            "rdquo" => "\u{201D}",
            "lsquo" => "\u{2018}",
            "rsquo" => "\u{2019}",
            "hellip" => "\u{2026}",
            "pound" => "\u{00A3}",
            "euro" => "\u{20AC}",
            "yen" => "\u{00A5}",
            "deg" => "\u{00B0}",
            "pm" => "\u{00B1}",
            "times" => "\u{00D7}",
            "div" => "\u{00F7}",
            "infin" => "\u{221E}",
            "rarr" => "\u{2192}",
            "larr" => "\u{2190}",
            "harr" => "\u{2194}",
            "uarr" => "\u{2191}",
            "darr" => "\u{2193}",
            _ => return None,
        }
        .to_string(),
    )
}

