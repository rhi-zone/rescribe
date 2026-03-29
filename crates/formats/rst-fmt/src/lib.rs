//! reStructuredText (RST) parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-rst` and `rescribe-write-rst` as thin adapter layers.

#![allow(clippy::collapsible_if)]

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct RstError(pub String);

impl std::fmt::Display for RstError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RST error: {}", self.0)
    }
}

impl std::error::Error for RstError {}

// ── AST ───────────────────────────────────────────────────────────────────────

/// A parsed RST document.
#[derive(Debug, Clone, Default)]
pub struct RstDoc {
    pub blocks: Vec<Block>,
}

/// Block-level element.
#[derive(Debug, Clone)]
pub enum Block {
    Paragraph {
        inlines: Vec<Inline>,
    },
    Heading {
        level: i64,
        inlines: Vec<Inline>,
    },
    CodeBlock {
        language: Option<String>,
        content: String,
    },
    Blockquote {
        children: Vec<Block>,
    },
    List {
        ordered: bool,
        items: Vec<Vec<Block>>,
    },
    DefinitionList {
        items: Vec<DefinitionItem>,
    },
    Figure {
        url: String,
        alt: Option<String>,
        caption: Option<Vec<Inline>>,
    },
    Image {
        url: String,
        alt: Option<String>,
        title: Option<String>,
    },
    RawBlock {
        format: String,
        content: String,
    },
    Div {
        class: Option<String>,
        directive: Option<String>,
        children: Vec<Block>,
    },
    HorizontalRule,
    Table {
        rows: Vec<TableRow>,
    },
    FootnoteDef {
        label: String,
        inlines: Vec<Inline>,
    },
    MathDisplay {
        source: String,
    },
    Admonition {
        admonition_type: String,
        children: Vec<Block>,
    },
}

/// A definition list item (term + description pair).
#[derive(Debug, Clone)]
pub struct DefinitionItem {
    pub term: Vec<Inline>,
    pub desc: Vec<Inline>,
}

/// A table row.
#[derive(Debug, Clone)]
pub struct TableRow {
    pub cells: Vec<Vec<Inline>>,
    pub is_header: bool,
}

/// Inline element.
#[derive(Debug, Clone)]
pub enum Inline {
    Text(String),
    Emphasis(Vec<Inline>),
    Strong(Vec<Inline>),
    Strikeout(Vec<Inline>),
    Underline(Vec<Inline>),
    Subscript(Vec<Inline>),
    Superscript(Vec<Inline>),
    Code(String),
    Link {
        url: String,
        children: Vec<Inline>,
    },
    Image {
        url: String,
        alt: String,
    },
    LineBreak,
    SoftBreak,
    FootnoteRef {
        label: String,
    },
    FootnoteDef {
        label: String,
        children: Vec<Inline>,
    },
    SmallCaps(Vec<Inline>),
    Quoted {
        quote_type: String,
        children: Vec<Inline>,
    },
    MathInline {
        source: String,
    },
    /// RST role-based span with unknown role
    RstSpan {
        role: String,
        children: Vec<Inline>,
    },
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// RST heading character priority (lower = higher level).
/// The actual level is determined by order of appearance in the document.
const HEADING_CHARS: &[char] = &['=', '-', '~', '^', '"', '`', '#', '*', '+', '_'];

/// Parse an RST string into an [`RstDoc`].
pub fn parse(input: &str) -> Result<RstDoc, RstError> {
    let mut p = Parser::new(input);
    let blocks = p.parse_document();
    Ok(RstDoc { blocks })
}

struct Parser<'a> {
    lines: Vec<&'a str>,
    line_idx: usize,
    /// Maps underline character to heading level (assigned in order of appearance).
    heading_levels: Vec<char>,
    /// Link targets: name -> url
    link_targets: std::collections::HashMap<String, String>,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        let lines: Vec<&str> = input.lines().collect();
        Self {
            lines,
            line_idx: 0,
            heading_levels: Vec::new(),
            link_targets: std::collections::HashMap::new(),
        }
    }

    fn current_line(&self) -> Option<&'a str> {
        self.lines.get(self.line_idx).copied()
    }

    fn peek_line(&self) -> Option<&'a str> {
        self.lines.get(self.line_idx + 1).copied()
    }

    fn advance_line(&mut self) {
        self.line_idx += 1;
    }

    fn is_eof(&self) -> bool {
        self.line_idx >= self.lines.len()
    }

    fn is_blank_line(&self) -> bool {
        self.current_line()
            .map(|l| l.trim().is_empty())
            .unwrap_or(true)
    }

    fn skip_blank_lines(&mut self) {
        while !self.is_eof() && self.is_blank_line() {
            self.advance_line();
        }
    }

    /// First pass: collect link targets (.. _name: url)
    fn collect_link_targets(&mut self) {
        let mut idx = 0;
        while idx < self.lines.len() {
            let line = self.lines[idx];
            if let Some(rest) = line.strip_prefix(".. _") {
                if let Some(colon_idx) = rest.find(':') {
                    let name = rest[..colon_idx].trim().to_lowercase();
                    let url = rest[colon_idx + 1..].trim().to_string();
                    self.link_targets.insert(name, url);
                }
            }
            idx += 1;
        }
    }

    fn parse_document(&mut self) -> Vec<Block> {
        // First pass: collect link targets
        self.collect_link_targets();

        let mut blocks = Vec::new();

        while !self.is_eof() {
            self.skip_blank_lines();
            if self.is_eof() {
                break;
            }

            if let Some(block) = self.try_parse_block() {
                blocks.push(block);
            } else {
                // Fallback: skip line to prevent infinite loop
                self.advance_line();
            }
        }

        blocks
    }

    fn try_parse_block(&mut self) -> Option<Block> {
        // Skip link target definitions (already collected)
        if let Some(line) = self.current_line() {
            if line.starts_with(".. _") && line.contains(':') {
                self.advance_line();
                return self.try_parse_block();
            }
        }

        // Check for heading (text followed by underline)
        if let Some(heading) = self.try_parse_heading() {
            return Some(heading);
        }

        // Check for directive
        if let Some(directive) = self.try_parse_directive() {
            return Some(directive);
        }

        // Check for list
        if let Some(list) = self.try_parse_list() {
            return Some(list);
        }

        // Check for definition list
        if let Some(deflist) = self.try_parse_definition_list() {
            return Some(deflist);
        }

        // Check for literal block (ends with ::)
        if let Some(literal) = self.try_parse_literal_block() {
            return Some(literal);
        }

        // Check for block quote (indented paragraph)
        if let Some(bq) = self.try_parse_blockquote() {
            return Some(bq);
        }

        // Regular paragraph
        self.parse_paragraph()
    }

    fn try_parse_heading(&mut self) -> Option<Block> {
        let line = self.current_line()?;

        // Check if this line is all underline chars (possible overline)
        if self.is_underline(line) && !line.is_empty() {
            // Overlined heading: === then title then ===
            let overline_char = line.chars().next()?;
            let next_line = self.peek_line()?;
            if !next_line.trim().is_empty() && !self.is_underline(next_line) {
                // Check for underline after title
                let title = next_line.trim();
                if let Some(underline) = self.lines.get(self.line_idx + 2) {
                    if self.is_underline(underline) && underline.starts_with(overline_char) {
                        self.advance_line(); // skip overline
                        self.advance_line(); // skip title
                        self.advance_line(); // skip underline
                        let level = self.get_heading_level(overline_char);
                        let inlines = parse_inline_content(title, &self.link_targets);
                        return Some(Block::Heading { level, inlines });
                    }
                }
            }
        }

        // Underlined heading: title then ===
        if !line.trim().is_empty() && !self.is_underline(line) {
            if let Some(underline) = self.peek_line() {
                if self.is_underline(underline) && underline.len() >= line.trim().len() {
                    let title = line.trim();
                    let underline_char = underline.chars().next()?;
                    self.advance_line(); // skip title
                    self.advance_line(); // skip underline
                    let level = self.get_heading_level(underline_char);
                    let inlines = parse_inline_content(title, &self.link_targets);
                    return Some(Block::Heading { level, inlines });
                }
            }
        }

        None
    }

    fn is_underline(&self, line: &str) -> bool {
        if line.is_empty() {
            return false;
        }
        let first = line.chars().next().unwrap();
        HEADING_CHARS.contains(&first) && line.chars().all(|c| c == first)
    }

    fn get_heading_level(&mut self, ch: char) -> i64 {
        if let Some(pos) = self.heading_levels.iter().position(|&c| c == ch) {
            (pos + 1) as i64
        } else {
            self.heading_levels.push(ch);
            self.heading_levels.len() as i64
        }
    }

    fn try_parse_directive(&mut self) -> Option<Block> {
        let line = self.current_line()?;

        if !line.starts_with(".. ") {
            return None;
        }

        let rest = &line[3..];

        // Check for footnote definition: .. [label] text
        // Label is digits, *, or # (numbered, auto-symbol, auto-numbered).
        if rest.starts_with('[') {
            if let Some(close_bracket) = rest.find(']') {
                let label = rest[1..close_bracket].trim();
                let after_bracket = &rest[close_bracket + 1..];
                // Must be followed by a space (and optional text) — not `[label]_` which is an inline ref
                if after_bracket.starts_with(' ') || after_bracket.is_empty() {
                    let first_line_text = after_bracket.trim().to_string();
                    self.advance_line();

                    // Collect continuation lines indented by ≥ 3 spaces
                    let mut body = first_line_text;
                    while !self.is_eof() {
                        let cont = self.current_line().unwrap_or("");
                        if cont.trim().is_empty() {
                            // Blank line ends the footnote body
                            break;
                        }
                        // Continuation line must be indented by at least 3 spaces
                        let indent = cont
                            .chars()
                            .take_while(|c| *c == ' ' || *c == '\t')
                            .count();
                        if indent < 3 {
                            break;
                        }
                        if !body.is_empty() {
                            body.push(' ');
                        }
                        body.push_str(cont.trim());
                        self.advance_line();
                    }

                    let inlines =
                        parse_inline_content(&body, &self.link_targets);
                    return Some(Block::FootnoteDef {
                        label: label.to_string(),
                        inlines,
                    });
                }
            }
        }

        // Check for comment (just .. with optional text but no ::)
        if !rest.contains("::") {
            // It's a comment, skip it
            self.advance_line();
            // Skip indented continuation
            while !self.is_eof() {
                let content_line = self.current_line().unwrap_or("");
                if content_line.is_empty()
                    || content_line.starts_with(' ')
                    || content_line.starts_with('\t')
                {
                    self.advance_line();
                } else {
                    break;
                }
            }
            return self.try_parse_block();
        }

        // Parse directive: .. name:: argument
        let colon_idx = rest.find("::")?;
        let directive_name = rest[..colon_idx].trim();
        let argument = rest[colon_idx + 2..].trim();

        self.advance_line();

        // Collect directive content (indented lines)
        let mut content_lines = Vec::new();
        let mut options = std::collections::HashMap::new();

        // First, collect field list options (:option: value)
        while !self.is_eof() {
            let Some(content_line) = self.current_line() else {
                break;
            };
            let trimmed = content_line.trim();
            if trimmed.is_empty() {
                self.advance_line();
                continue;
            }
            if (content_line.starts_with(' ') || content_line.starts_with('\t'))
                && trimmed.starts_with(':')
                && trimmed.len() > 1
            {
                // Option line
                if let Some(end_colon) = trimmed[1..].find(':') {
                    let opt_name = &trimmed[1..end_colon + 1];
                    let opt_value = trimmed[end_colon + 2..].trim();
                    options.insert(opt_name.to_string(), opt_value.to_string());
                    self.advance_line();
                    continue;
                }
            }
            break;
        }

        // Then collect content
        while !self.is_eof() {
            let Some(content_line) = self.current_line() else {
                break;
            };
            if content_line.is_empty() {
                content_lines.push("");
                self.advance_line();
            } else if content_line.starts_with(' ') || content_line.starts_with('\t') {
                content_lines.push(content_line.trim());
                self.advance_line();
            } else {
                break;
            }
        }

        // Handle specific directives
        let block = match directive_name {
            "code" | "code-block" | "sourcecode" => {
                let language = if argument.is_empty() {
                    None
                } else {
                    Some(argument.to_string())
                };
                let content = content_lines.join("\n");
                Block::CodeBlock { language, content }
            }
            "note" | "warning" | "tip" | "important" | "caution" | "danger" | "error" | "hint"
            | "attention" => {
                let content = content_lines.join("\n");
                let inlines = parse_inline_content(&content, &self.link_targets);
                Block::Div {
                    class: Some(directive_name.to_string()),
                    directive: None,
                    children: vec![Block::Paragraph { inlines }],
                }
            }
            "image" => Block::Image {
                url: argument.to_string(),
                alt: options.get("alt").cloned(),
                title: options.get("title").cloned(),
            },
            "figure" => {
                let caption = if content_lines.is_empty() {
                    None
                } else {
                    let caption_text = content_lines.join(" ");
                    Some(parse_inline_content(&caption_text, &self.link_targets))
                };
                Block::Figure {
                    url: argument.to_string(),
                    alt: options.get("alt").cloned(),
                    caption,
                }
            }
            "raw" => Block::RawBlock {
                format: argument.to_string(),
                content: content_lines.join("\n"),
            },
            "contents" | "toc" => Block::Div {
                class: Some("toc".to_string()),
                directive: None,
                children: vec![],
            },
            "math" => Block::MathDisplay {
                source: content_lines.join("\n"),
            },
            _ => {
                // Unknown directive — create generic div (warnings handled by adapter)
                let content = content_lines.join("\n");
                let children = if content.is_empty() {
                    vec![]
                } else {
                    let inlines = parse_inline_content(&content, &self.link_targets);
                    vec![Block::Paragraph { inlines }]
                };
                Block::Div {
                    class: None,
                    directive: Some(directive_name.to_string()),
                    children,
                }
            }
        };

        Some(block)
    }

    fn try_parse_list(&mut self) -> Option<Block> {
        let line = self.current_line()?;
        let trimmed = line.trim_start();

        // Bullet list: *, -, +
        if trimmed
            .strip_prefix("* ")
            .or_else(|| trimmed.strip_prefix("- "))
            .or_else(|| trimmed.strip_prefix("+ "))
            .is_some()
        {
            let bullet_char = trimmed.chars().next().unwrap();
            return Some(self.parse_bullet_list(bullet_char));
        }

        // Numbered list: 1. or #.
        if let Some(idx) = trimmed.find(". ") {
            let prefix = &trimmed[..idx];
            if prefix.chars().all(|c| c.is_ascii_digit()) || prefix == "#" {
                return Some(self.parse_numbered_list());
            }
        }

        None
    }

    fn parse_bullet_list(&mut self, bullet: char) -> Block {
        let mut items = Vec::new();
        let indent = self.get_indent();

        while !self.is_eof() {
            let line = self.current_line().unwrap_or("");
            let current_indent = self.get_line_indent(line);
            let trimmed = line.trim_start();

            if trimmed.is_empty() {
                // Blank line - check if list continues
                let next_idx = self.line_idx + 1;
                if next_idx < self.lines.len() {
                    let next_line = self.lines[next_idx];
                    let next_trimmed = next_line.trim_start();
                    if !next_trimmed.starts_with(&format!("{} ", bullet)) {
                        break;
                    }
                }
                self.advance_line();
                continue;
            }

            if current_indent < indent && indent > 0 {
                break;
            }

            if let Some(rest) = trimmed.strip_prefix(&format!("{} ", bullet)) {
                let item = self.parse_list_item(rest);
                items.push(vec![Block::Paragraph { inlines: item }]);
            } else if current_indent > indent {
                // Continuation of previous item - skip for now
                self.advance_line();
            } else {
                break;
            }
        }

        Block::List {
            ordered: false,
            items,
        }
    }

    fn parse_numbered_list(&mut self) -> Block {
        let mut items = Vec::new();
        let indent = self.get_indent();

        while !self.is_eof() {
            let line = self.current_line().unwrap_or("");
            let current_indent = self.get_line_indent(line);
            let trimmed = line.trim_start();

            if trimmed.is_empty() {
                self.advance_line();
                continue;
            }

            if current_indent < indent && indent > 0 {
                break;
            }

            // Check for numbered item
            if let Some(idx) = trimmed.find(". ") {
                let prefix = &trimmed[..idx];
                if prefix.chars().all(|c| c.is_ascii_digit()) || prefix == "#" {
                    let rest = &trimmed[idx + 2..];
                    let item = self.parse_list_item(rest);
                    items.push(vec![Block::Paragraph { inlines: item }]);
                    continue;
                }
            }

            if current_indent > indent {
                // Continuation
                self.advance_line();
            } else {
                break;
            }
        }

        Block::List {
            ordered: true,
            items,
        }
    }

    fn parse_list_item(&mut self, first_line: &str) -> Vec<Inline> {
        self.advance_line();

        let mut content = first_line.to_string();

        // Collect continuation lines
        while !self.is_eof() {
            let line = self.current_line().unwrap_or("");
            if line.trim().is_empty() {
                break;
            }
            // Check if it's a new list item
            let trimmed = line.trim_start();
            if trimmed.starts_with("* ") || trimmed.starts_with("- ") || trimmed.starts_with("+ ") {
                break;
            }
            if let Some(idx) = trimmed.find(". ") {
                let prefix = &trimmed[..idx];
                if prefix.chars().all(|c| c.is_ascii_digit()) || prefix == "#" {
                    break;
                }
            }
            // Check if indented (continuation)
            if line.starts_with(' ') || line.starts_with('\t') {
                content.push(' ');
                content.push_str(trimmed);
                self.advance_line();
            } else {
                break;
            }
        }

        parse_inline_content(&content, &self.link_targets)
    }

    fn try_parse_definition_list(&mut self) -> Option<Block> {
        let line = self.current_line()?;

        // Definition list: term at start of line, definition indented on next line
        if !line.is_empty()
            && !line.starts_with(' ')
            && !line.starts_with('\t')
            && !line.starts_with(".. ")
        {
            if let Some(next_line) = self.peek_line() {
                if (next_line.starts_with(' ') || next_line.starts_with('\t'))
                    && !next_line.trim().is_empty()
                {
                    return Some(self.parse_definition_list());
                }
            }
        }

        None
    }

    fn parse_definition_list(&mut self) -> Block {
        let mut items = Vec::new();

        while !self.is_eof() {
            let line = self.current_line().unwrap_or("");

            // Skip blank lines
            if line.trim().is_empty() {
                self.advance_line();
                continue;
            }

            // Check if it's a term (non-indented)
            if !line.starts_with(' ') && !line.starts_with('\t') && !line.starts_with(".. ") {
                // Check if next line is definition (indented)
                if let Some(next_line) = self.peek_line() {
                    if (next_line.starts_with(' ') || next_line.starts_with('\t'))
                        && !next_line.trim().is_empty()
                    {
                        let term_str = line.trim();
                        let term = parse_inline_content(term_str, &self.link_targets);

                        self.advance_line();

                        // Collect definition
                        let mut def_content = String::new();
                        while !self.is_eof() {
                            let def_line = self.current_line().unwrap_or("");
                            if def_line.trim().is_empty() {
                                break;
                            }
                            if def_line.starts_with(' ') || def_line.starts_with('\t') {
                                if !def_content.is_empty() {
                                    def_content.push(' ');
                                }
                                def_content.push_str(def_line.trim());
                                self.advance_line();
                            } else {
                                break;
                            }
                        }

                        let desc = parse_inline_content(&def_content, &self.link_targets);
                        items.push(DefinitionItem { term, desc });
                        continue;
                    }
                }
            }

            break;
        }

        Block::DefinitionList { items }
    }

    fn try_parse_literal_block(&mut self) -> Option<Block> {
        let line = self.current_line()?;

        // Check for :: at end of line (paragraph ending with ::)
        if line.trim_end().ends_with("::") {
            self.advance_line();
            self.skip_blank_lines();

            // Collect indented content
            let mut content_lines: Vec<&str> = Vec::new();
            let base_indent = self.get_indent();

            while !self.is_eof() {
                let content_line = self.current_line().unwrap_or("");
                if content_line.trim().is_empty() {
                    content_lines.push("");
                    self.advance_line();
                } else if self.get_line_indent(content_line) >= base_indent {
                    // Remove base indentation
                    let dedented = if content_line.len() > base_indent {
                        &content_line[base_indent..]
                    } else {
                        ""
                    };
                    content_lines.push(dedented);
                    self.advance_line();
                } else {
                    break;
                }
            }

            // Trim trailing empty lines
            while content_lines.last() == Some(&"") {
                content_lines.pop();
            }

            let content = content_lines.join("\n");
            return Some(Block::CodeBlock {
                language: None,
                content,
            });
        }

        // Check for standalone :: on its own line
        if line.trim() == "::" {
            self.advance_line();
            self.skip_blank_lines();

            let mut content_lines: Vec<&str> = Vec::new();
            let base_indent = self.get_indent();

            while !self.is_eof() {
                let content_line = self.current_line().unwrap_or("");
                if content_line.trim().is_empty() {
                    content_lines.push("");
                    self.advance_line();
                } else if self.get_line_indent(content_line) >= base_indent {
                    let dedented = if content_line.len() > base_indent {
                        &content_line[base_indent..]
                    } else {
                        ""
                    };
                    content_lines.push(dedented);
                    self.advance_line();
                } else {
                    break;
                }
            }

            while content_lines.last() == Some(&"") {
                content_lines.pop();
            }

            let content = content_lines.join("\n");
            return Some(Block::CodeBlock {
                language: None,
                content,
            });
        }

        None
    }

    fn try_parse_blockquote(&mut self) -> Option<Block> {
        let line = self.current_line()?;

        // Block quote: indented text that's not a list or literal block
        if (line.starts_with(' ') || line.starts_with('\t')) && !line.trim().is_empty() {
            let trimmed = line.trim();
            // Make sure it's not a list item
            if trimmed.starts_with("* ") || trimmed.starts_with("- ") || trimmed.starts_with("+ ") {
                return None;
            }
            if let Some(idx) = trimmed.find(". ") {
                let prefix = &trimmed[..idx];
                if prefix.chars().all(|c| c.is_ascii_digit()) || prefix == "#" {
                    return None;
                }
            }

            // Collect block quote content
            let mut content = String::new();
            while !self.is_eof() {
                let bq_line = self.current_line().unwrap_or("");
                if bq_line.trim().is_empty() {
                    break;
                }
                if bq_line.starts_with(' ') || bq_line.starts_with('\t') {
                    if !content.is_empty() {
                        content.push(' ');
                    }
                    content.push_str(bq_line.trim());
                    self.advance_line();
                } else {
                    break;
                }
            }

            let inlines = parse_inline_content(&content, &self.link_targets);
            return Some(Block::Blockquote {
                children: vec![Block::Paragraph { inlines }],
            });
        }

        None
    }

    fn parse_paragraph(&mut self) -> Option<Block> {
        let mut content = String::new();

        while !self.is_eof() {
            let line = self.current_line().unwrap_or("");

            if line.trim().is_empty() {
                break;
            }

            // Check if next line is an underline (making this a heading)
            if let Some(next) = self.peek_line() {
                if self.is_underline(next) {
                    break;
                }
            }

            // Check for start of block elements
            if line.starts_with(".. ") {
                break;
            }

            if !content.is_empty() {
                content.push(' ');
            }
            content.push_str(line.trim());
            self.advance_line();
        }

        if content.is_empty() {
            return None;
        }

        // Check for trailing :: (literal block indicator)
        let content = if content.ends_with("::") && content.len() > 2 {
            content[..content.len() - 1].trim_end().to_string()
        } else {
            content
        };

        let inlines = parse_inline_content(&content, &self.link_targets);
        Some(Block::Paragraph { inlines })
    }

    fn get_indent(&self) -> usize {
        self.current_line()
            .map(|l| self.get_line_indent(l))
            .unwrap_or(0)
    }

    fn get_line_indent(&self, line: &str) -> usize {
        line.chars().take_while(|c| *c == ' ' || *c == '\t').count()
    }
}

// ── Inline parser (free function) ─────────────────────────────────────────────

fn parse_inline_content(
    content: &str,
    link_targets: &std::collections::HashMap<String, String>,
) -> Vec<Inline> {
    let mut nodes = Vec::new();
    let mut pos = 0;
    let chars: Vec<char> = content.chars().collect();

    while pos < chars.len() {
        // Strong: **text**
        if pos + 1 < chars.len() && chars[pos] == '*' && chars[pos + 1] == '*' {
            if let Some((end, text)) = find_closing(&chars, pos + 2, "**") {
                let children = parse_inline_content(&text, link_targets);
                nodes.push(Inline::Strong(children));
                pos = end + 2;
                continue;
            }
        }

        // Emphasis: *text*
        if chars[pos] == '*' {
            if let Some((end, text)) = find_closing_char(&chars, pos + 1, '*') {
                if !text.is_empty() && !text.starts_with('*') {
                    let children = parse_inline_content(&text, link_targets);
                    nodes.push(Inline::Emphasis(children));
                    pos = end + 1;
                    continue;
                }
            }
        }

        // Inline literal: ``text``
        if pos + 1 < chars.len() && chars[pos] == '`' && chars[pos + 1] == '`' {
            if let Some((end, text)) = find_closing(&chars, pos + 2, "``") {
                nodes.push(Inline::Code(text));
                pos = end + 2;
                continue;
            }
        }

        // Interpreted text with role: :role:`text`
        if chars[pos] == ':' {
            if let Some((role_end, role)) = find_closing_char(&chars, pos + 1, ':') {
                if role_end + 1 < chars.len() && chars[role_end + 1] == '`' {
                    if let Some((text_end, text)) = find_closing_char(&chars, role_end + 2, '`') {
                        let inline = match role.as_str() {
                            "emphasis" | "em" => {
                                let ch = parse_inline_content(&text, link_targets);
                                Inline::Emphasis(ch)
                            }
                            "strong" => {
                                let ch = parse_inline_content(&text, link_targets);
                                Inline::Strong(ch)
                            }
                            "code" | "literal" => Inline::Code(text),
                            "subscript" | "sub" => {
                                let ch = parse_inline_content(&text, link_targets);
                                Inline::Subscript(ch)
                            }
                            "superscript" | "sup" => {
                                let ch = parse_inline_content(&text, link_targets);
                                Inline::Superscript(ch)
                            }
                            "title-reference" | "title" | "t" => {
                                let ch = parse_inline_content(&text, link_targets);
                                Inline::Emphasis(ch)
                            }
                            "ref" | "doc" => Inline::Link {
                                url: format!("#{}", text),
                                children: vec![Inline::Text(text.clone())],
                            },
                            "math" => Inline::MathInline { source: text },
                            _ => {
                                let ch = parse_inline_content(&text, link_targets);
                                Inline::RstSpan { role, children: ch }
                            }
                        };
                        nodes.push(inline);
                        pos = text_end + 1;
                        continue;
                    }
                }
            }
        }

        // Inline link: `text <url>`_
        if chars[pos] == '`' {
            if let Some((end, text)) = find_closing_char(&chars, pos + 1, '`') {
                // Check for trailing _
                if end + 1 < chars.len() && chars[end + 1] == '_' {
                    // Check if it's an inline link with URL
                    if let Some(angle_start) = text.rfind('<') {
                        if text.ends_with('>') {
                            let link_text = text[..angle_start].trim();
                            let url = &text[angle_start + 1..text.len() - 1];
                            nodes.push(Inline::Link {
                                url: url.to_string(),
                                children: vec![Inline::Text(link_text.to_string())],
                            });
                            pos = end + 2;
                            continue;
                        }
                    }

                    // Reference link - look up in link_targets
                    let ref_name = text.to_lowercase();
                    if let Some(url) = link_targets.get(&ref_name) {
                        nodes.push(Inline::Link {
                            url: url.clone(),
                            children: vec![Inline::Text(text.to_string())],
                        });
                        pos = end + 2;
                        continue;
                    }
                }

                // Plain interpreted text (default role, usually emphasis)
                nodes.push(Inline::Emphasis(vec![Inline::Text(text.to_string())]));
                pos = end + 1;
                continue;
            }
        }

        // Simple reference link: word_
        if chars[pos].is_alphanumeric() {
            let mut word_end = pos;
            while word_end < chars.len()
                && (chars[word_end].is_alphanumeric()
                    || chars[word_end] == '_'
                    || chars[word_end] == '-')
            {
                word_end += 1;
            }
            if word_end < chars.len() && chars[word_end] == '_' {
                // Check it's not __ (anonymous reference)
                if word_end + 1 >= chars.len() || chars[word_end + 1] != '_' {
                    let word: String = chars[pos..word_end].iter().collect();
                    let ref_name = word.to_lowercase();
                    if let Some(url) = link_targets.get(&ref_name) {
                        nodes.push(Inline::Link {
                            url: url.clone(),
                            children: vec![Inline::Text(word)],
                        });
                        pos = word_end + 1;
                        continue;
                    }
                }
            }
        }

        // Footnote reference: [label]_
        if chars[pos] == '[' {
            if let Some(close) = chars[pos + 1..].iter().position(|&c| c == ']') {
                let close_abs = pos + 1 + close;
                // Check for trailing _
                if close_abs + 1 < chars.len() && chars[close_abs + 1] == '_' {
                    let label: String = chars[pos + 1..close_abs].iter().collect();
                    nodes.push(Inline::FootnoteRef { label });
                    pos = close_abs + 2;
                    continue;
                }
            }
        }

        // Regular text
        let pos_before = pos;
        let mut text = String::new();
        while pos < chars.len() {
            let c = chars[pos];
            // Stop at potential inline markup starts
            if c == '*' || c == '`' || c == ':' || c == '[' {
                break;
            }
            // Stop at potential reference (word followed by _)
            if c.is_alphanumeric() {
                let mut word_end = pos;
                while word_end < chars.len()
                    && (chars[word_end].is_alphanumeric()
                        || chars[word_end] == '_'
                        || chars[word_end] == '-')
                {
                    word_end += 1;
                }
                if word_end < chars.len()
                    && chars[word_end] == '_'
                    && (word_end + 1 >= chars.len() || chars[word_end + 1] != '_')
                {
                    let word: String = chars[pos..word_end].iter().collect();
                    if link_targets.contains_key(&word.to_lowercase()) {
                        break;
                    }
                }
            }
            text.push(c);
            pos += 1;
        }

        if !text.is_empty() {
            nodes.push(Inline::Text(text));
        } else if pos == pos_before {
            // No markup matched and the text loop didn't advance — consume
            // current character literally to guarantee forward progress.
            nodes.push(Inline::Text(chars[pos].to_string()));
            pos += 1;
        }
    }

    // Merge adjacent text nodes
    merge_text_nodes(&mut nodes);

    nodes
}

fn find_closing(chars: &[char], start: usize, pattern: &str) -> Option<(usize, String)> {
    let pat_chars: Vec<char> = pattern.chars().collect();
    let mut pos = start;
    let mut text = String::new();

    while pos + pat_chars.len() <= chars.len() {
        let mut matches = true;
        for (i, pc) in pat_chars.iter().enumerate() {
            if chars[pos + i] != *pc {
                matches = false;
                break;
            }
        }
        if matches {
            return Some((pos, text));
        }
        text.push(chars[pos]);
        pos += 1;
    }

    None
}

fn find_closing_char(chars: &[char], start: usize, close: char) -> Option<(usize, String)> {
    let mut pos = start;
    let mut text = String::new();

    while pos < chars.len() {
        if chars[pos] == close {
            return Some((pos, text));
        }
        text.push(chars[pos]);
        pos += 1;
    }

    None
}

fn merge_text_nodes(nodes: &mut Vec<Inline>) {
    let mut i = 0;
    while i + 1 < nodes.len() {
        let both_text =
            matches!(&nodes[i], Inline::Text(_)) && matches!(&nodes[i + 1], Inline::Text(_));
        if both_text {
            let next_text = match nodes.remove(i + 1) {
                Inline::Text(s) => s,
                _ => unreachable!(),
            };
            if let Inline::Text(current) = &mut nodes[i] {
                current.push_str(&next_text);
            }
        } else {
            i += 1;
        }
    }
}

// ── Builder ───────────────────────────────────────────────────────────────────

/// Build an RST string from an [`RstDoc`].
pub fn build(doc: &RstDoc) -> String {
    let mut ctx = BuildContext::new();
    build_blocks(&doc.blocks, &mut ctx);
    ctx.output
}

struct BuildContext {
    output: String,
    list_depth: usize,
}

impl BuildContext {
    fn new() -> Self {
        Self {
            output: String::new(),
            list_depth: 0,
        }
    }

    fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn write_indent(&mut self) {
        for _ in 0..self.list_depth {
            self.write("   ");
        }
    }
}

fn build_blocks(blocks: &[Block], ctx: &mut BuildContext) {
    for block in blocks {
        build_block(block, ctx);
    }
}

fn build_block(block: &Block, ctx: &mut BuildContext) {
    match block {
        Block::Paragraph { inlines } => {
            build_inlines(inlines, ctx);
            ctx.write("\n\n");
        }

        Block::Heading { level, inlines } => build_heading(*level, inlines, ctx),

        Block::CodeBlock { language, content } => {
            build_code_block(language.as_deref(), content, ctx)
        }

        Block::Blockquote { children } => build_blockquote(children, ctx),

        Block::List { ordered, items } => build_list(*ordered, items, ctx),

        Block::DefinitionList { items } => build_definition_list(items, ctx),

        Block::Figure { url, alt, caption } => {
            build_figure(url, alt.as_deref(), caption.as_deref(), ctx)
        }

        Block::Image { url, alt, title: _ } => build_image(url, alt.as_deref(), ctx),

        Block::RawBlock { format, content } => {
            if format == "rst" {
                ctx.write(content);
            }
        }

        Block::Div { children, .. } => build_blocks(children, ctx),

        Block::HorizontalRule => {
            ctx.write("----\n\n");
        }

        Block::Table { rows } => build_table(rows, ctx),

        Block::FootnoteDef { label, inlines } => {
            ctx.write(".. [");
            ctx.write(label);
            ctx.write("] ");
            build_inlines(inlines, ctx);
            ctx.write("\n");
        }

        Block::MathDisplay { source } => {
            ctx.write(".. math::\n\n   ");
            ctx.write(&source.replace('\n', "\n   "));
            ctx.write("\n\n");
        }

        Block::Admonition {
            admonition_type,
            children,
        } => build_admonition(admonition_type, children, ctx),
    }
}

fn build_heading(level: i64, inlines: &[Inline], ctx: &mut BuildContext) {
    let mut text = String::new();
    collect_text_from_inlines(inlines, &mut text);

    let underline_char = match level {
        1 => '=',
        2 => '-',
        3 => '~',
        4 => '^',
        5 => '"',
        _ => '\'',
    };

    // For level 1, add overline
    if level == 1 {
        let line: String = std::iter::repeat_n(underline_char, text.len()).collect();
        ctx.write(&line);
        ctx.write("\n");
    }

    build_inlines(inlines, ctx);
    ctx.write("\n");

    let line: String = std::iter::repeat_n(underline_char, text.len()).collect();
    ctx.write(&line);
    ctx.write("\n\n");
}

fn build_code_block(language: Option<&str>, content: &str, ctx: &mut BuildContext) {
    if let Some(lang) = language {
        ctx.write(".. code-block:: ");
        ctx.write(lang);
        ctx.write("\n\n");
    } else {
        ctx.write("::\n\n");
    }

    for line in content.lines() {
        ctx.write("   ");
        ctx.write(line);
        ctx.write("\n");
    }
    ctx.write("\n");
}

fn build_blockquote(children: &[Block], ctx: &mut BuildContext) {
    let mut inner = BuildContext::new();
    build_blocks(children, &mut inner);

    for line in inner.output.lines() {
        ctx.write("   ");
        ctx.write(line);
        ctx.write("\n");
    }
    ctx.write("\n");
}

fn build_list(ordered: bool, items: &[Vec<Block>], ctx: &mut BuildContext) {
    ctx.list_depth += 1;
    for item_blocks in items {
        build_list_item(ordered, item_blocks, ctx);
    }
    ctx.list_depth -= 1;
    ctx.write("\n");
}

fn build_list_item(ordered: bool, item_blocks: &[Block], ctx: &mut BuildContext) {
    if ordered {
        ctx.write("#. ");
    } else {
        ctx.write("- ");
    }

    let mut first = true;
    for child in item_blocks {
        match child {
            Block::Paragraph { inlines } => {
                if !first {
                    ctx.write_indent();
                    ctx.write("   ");
                }
                build_inlines(inlines, ctx);
                ctx.write("\n");
            }
            Block::List { ordered, items } => {
                ctx.write("\n");
                ctx.write_indent();
                build_list(*ordered, items, ctx);
            }
            other => build_block(other, ctx),
        }
        first = false;
    }
}

fn build_definition_list(items: &[DefinitionItem], ctx: &mut BuildContext) {
    for item in items {
        build_inlines(&item.term, ctx);
        ctx.write("\n");
        ctx.write("   ");
        build_inlines(&item.desc, ctx);
        ctx.write("\n\n");
    }
}

fn build_figure(url: &str, alt: Option<&str>, caption: Option<&[Inline]>, ctx: &mut BuildContext) {
    ctx.write(".. figure:: ");
    ctx.write(url);
    ctx.write("\n");

    if let Some(alt_text) = alt {
        ctx.write("   :alt: ");
        ctx.write(alt_text);
        ctx.write("\n");
    }

    if let Some(cap) = caption {
        ctx.write("\n   ");
        build_inlines(cap, ctx);
        ctx.write("\n");
    }

    ctx.write("\n");
}

fn build_image(url: &str, alt: Option<&str>, ctx: &mut BuildContext) {
    ctx.write(".. image:: ");
    ctx.write(url);
    ctx.write("\n");

    if let Some(alt_text) = alt {
        ctx.write("   :alt: ");
        ctx.write(alt_text);
        ctx.write("\n");
    }
    ctx.write("\n");
}

fn build_table(rows: &[TableRow], ctx: &mut BuildContext) {
    if rows.is_empty() {
        return;
    }

    // Collect cell text for width calculation
    let text_rows: Vec<Vec<String>> = rows
        .iter()
        .map(|r| {
            r.cells
                .iter()
                .map(|cell| {
                    let mut s = String::new();
                    collect_text_from_inlines(cell, &mut s);
                    s
                })
                .collect()
        })
        .collect();

    let col_widths = calculate_column_widths(&text_rows);

    emit_table_border(&col_widths, ctx);

    let mut is_first = true;
    for (row, text_row) in rows.iter().zip(text_rows.iter()) {
        ctx.write("|");
        for (i, cell) in text_row.iter().enumerate() {
            let width = col_widths.get(i).copied().unwrap_or(1);
            ctx.write(" ");
            ctx.write(cell);
            for _ in cell.len()..width {
                ctx.write(" ");
            }
            ctx.write(" |");
        }
        ctx.write("\n");

        // Header separator after first row if it's a header
        if is_first && row.is_header && rows.len() > 1 {
            emit_table_border(&col_widths, ctx);
        }
        is_first = false;
    }

    emit_table_border(&col_widths, ctx);
    ctx.write("\n");
}

fn emit_table_border(widths: &[usize], ctx: &mut BuildContext) {
    ctx.write("+");
    for w in widths {
        for _ in 0..(*w + 2) {
            ctx.write("-");
        }
        ctx.write("+");
    }
    ctx.write("\n");
}

fn calculate_column_widths(rows: &[Vec<String>]) -> Vec<usize> {
    let num_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut widths = vec![1usize; num_cols];

    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if cell.len() > widths[i] {
                widths[i] = cell.len();
            }
        }
    }
    widths
}

fn build_admonition(admonition_type: &str, children: &[Block], ctx: &mut BuildContext) {
    ctx.write(".. ");
    ctx.write(admonition_type);
    ctx.write("::\n\n");

    let mut inner = BuildContext::new();
    build_blocks(children, &mut inner);

    for line in inner.output.lines() {
        ctx.write("   ");
        ctx.write(line);
        ctx.write("\n");
    }
    ctx.write("\n");
}

fn build_inlines(inlines: &[Inline], ctx: &mut BuildContext) {
    for inline in inlines {
        build_inline(inline, ctx);
    }
}

fn build_inline(inline: &Inline, ctx: &mut BuildContext) {
    match inline {
        Inline::Text(s) => ctx.write(s),

        Inline::Emphasis(children) => {
            ctx.write("*");
            build_inlines(children, ctx);
            ctx.write("*");
        }

        Inline::Strong(children) => {
            ctx.write("**");
            build_inlines(children, ctx);
            ctx.write("**");
        }

        Inline::Strikeout(children) => {
            ctx.write(":strike:`");
            build_inlines(children, ctx);
            ctx.write("`");
        }

        Inline::Underline(children) => {
            ctx.write(":underline:`");
            build_inlines(children, ctx);
            ctx.write("`");
        }

        Inline::Subscript(children) => {
            ctx.write(":sub:`");
            build_inlines(children, ctx);
            ctx.write("`");
        }

        Inline::Superscript(children) => {
            ctx.write(":sup:`");
            build_inlines(children, ctx);
            ctx.write("`");
        }

        Inline::Code(s) => {
            ctx.write("``");
            ctx.write(s);
            ctx.write("``");
        }

        Inline::Link { url, children } => {
            ctx.write("`");
            build_inlines(children, ctx);
            ctx.write(" <");
            ctx.write(url);
            ctx.write(">`_");
        }

        Inline::Image { url, alt } => {
            ctx.write(".. image:: ");
            ctx.write(url);
            if !alt.is_empty() {
                ctx.write("\n   :alt: ");
                ctx.write(alt);
            }
            ctx.write("\n");
        }

        Inline::LineBreak | Inline::SoftBreak => ctx.write("\n"),

        Inline::FootnoteRef { label } => {
            ctx.write("[");
            ctx.write(label);
            ctx.write("]_");
        }

        Inline::FootnoteDef { label, children } => {
            ctx.write(".. [");
            ctx.write(label);
            ctx.write("] ");
            build_inlines(children, ctx);
        }

        Inline::SmallCaps(children) => {
            ctx.write(":sc:`");
            build_inlines(children, ctx);
            ctx.write("`");
        }

        Inline::Quoted {
            quote_type,
            children,
        } => {
            if quote_type == "single" {
                ctx.write("'");
                build_inlines(children, ctx);
                ctx.write("'");
            } else {
                ctx.write("\"");
                build_inlines(children, ctx);
                ctx.write("\"");
            }
        }

        Inline::MathInline { source } => {
            ctx.write(":math:`");
            ctx.write(source);
            ctx.write("`");
        }

        Inline::RstSpan { role, children } => {
            ctx.write(":");
            ctx.write(role);
            ctx.write(":`");
            build_inlines(children, ctx);
            ctx.write("`");
        }
    }
}

fn collect_text_from_inlines(inlines: &[Inline], out: &mut String) {
    for inline in inlines {
        match inline {
            Inline::Text(s) => out.push_str(s),
            Inline::Code(s) => out.push_str(s),
            Inline::MathInline { source } => out.push_str(source),
            Inline::Emphasis(ch)
            | Inline::Strong(ch)
            | Inline::Strikeout(ch)
            | Inline::Underline(ch)
            | Inline::Subscript(ch)
            | Inline::Superscript(ch)
            | Inline::SmallCaps(ch)
            | Inline::RstSpan { children: ch, .. }
            | Inline::Quoted { children: ch, .. }
            | Inline::FootnoteDef { children: ch, .. } => collect_text_from_inlines(ch, out),
            Inline::Link { children, .. } => collect_text_from_inlines(children, out),
            Inline::Image { alt, .. } => out.push_str(alt),
            Inline::LineBreak | Inline::SoftBreak => out.push(' '),
            Inline::FootnoteRef { label } => out.push_str(label),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_heading() {
        let input = "Hello World\n===========\n\nSome text.";
        let doc = parse(input).unwrap();

        assert_eq!(doc.blocks.len(), 2);
        assert!(matches!(doc.blocks[0], Block::Heading { level: 1, .. }));
    }

    #[test]
    fn test_parse_paragraph() {
        let input = "This is a paragraph.\n\nThis is another.";
        let doc = parse(input).unwrap();

        assert_eq!(doc.blocks.len(), 2);
        assert!(matches!(doc.blocks[0], Block::Paragraph { .. }));
        assert!(matches!(doc.blocks[1], Block::Paragraph { .. }));
    }

    #[test]
    fn test_parse_emphasis() {
        let input = "This is *emphasized* text.";
        let doc = parse(input).unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Emphasis(_))));
    }

    #[test]
    fn test_parse_strong() {
        let input = "This is **strong** text.";
        let doc = parse(input).unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Strong(_))));
    }

    #[test]
    fn test_parse_bullet_list() {
        let input = "* First item\n* Second item\n* Third item";
        let doc = parse(input).unwrap();

        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::List { ordered: false, .. }));
        if let Block::List { items, .. } = &doc.blocks[0] {
            assert_eq!(items.len(), 3);
        }
    }

    #[test]
    fn test_parse_numbered_list() {
        let input = "1. First item\n2. Second item\n3. Third item";
        let doc = parse(input).unwrap();

        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::List { ordered: true, .. }));
        if let Block::List { items, .. } = &doc.blocks[0] {
            assert_eq!(items.len(), 3);
        }
    }

    #[test]
    fn test_parse_code_block() {
        let input = "Example::\n\n    def hello():\n        print('Hello')";
        let doc = parse(input).unwrap();
        assert!(
            doc.blocks
                .iter()
                .any(|b| matches!(b, Block::CodeBlock { .. }))
        );
    }

    #[test]
    fn test_parse_inline_code() {
        let input = "Use ``code here`` in text.";
        let doc = parse(input).unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Code(_))));
    }

    #[test]
    fn test_parse_link() {
        let input = "Click `here <https://example.com>`_ for more.";
        let doc = parse(input).unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        let link = inlines.iter().find(|i| matches!(i, Inline::Link { .. }));
        assert!(link.is_some());
        if let Some(Inline::Link { url, .. }) = link {
            assert_eq!(url, "https://example.com");
        }
    }

    #[test]
    fn test_parse_directive() {
        let input = ".. code-block:: python\n\n   print('hello')";
        let doc = parse(input).unwrap();

        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(
            doc.blocks[0],
            Block::CodeBlock {
                language: Some(_),
                ..
            }
        ));
        if let Block::CodeBlock { language, .. } = &doc.blocks[0] {
            assert_eq!(language.as_deref(), Some("python"));
        }
    }

    #[test]
    fn test_build_paragraph() {
        let doc = RstDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Hello, world!".into())],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("Hello, world!"));
    }

    #[test]
    fn test_build_heading() {
        let doc = RstDoc {
            blocks: vec![Block::Heading {
                level: 1,
                inlines: vec![Inline::Text("Title".into())],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("====="));
        assert!(output.contains("Title"));
    }

    #[test]
    fn test_build_heading_level2() {
        let doc = RstDoc {
            blocks: vec![Block::Heading {
                level: 2,
                inlines: vec![Inline::Text("Subtitle".into())],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("--------"));
        assert!(output.contains("Subtitle"));
    }

    #[test]
    fn test_build_emphasis() {
        let doc = RstDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Emphasis(vec![Inline::Text("italic".into())])],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("*italic*"));
    }

    #[test]
    fn test_build_strong() {
        let doc = RstDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Strong(vec![Inline::Text("bold".into())])],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("**bold**"));
    }

    #[test]
    fn test_build_code() {
        let doc = RstDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Code("code".into())],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("``code``"));
    }

    #[test]
    fn test_build_link() {
        let doc = RstDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "https://example.com".into(),
                    children: vec![Inline::Text("click".into())],
                }],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("`click <https://example.com>`_"));
    }

    #[test]
    fn test_build_code_block() {
        let doc = RstDoc {
            blocks: vec![Block::CodeBlock {
                language: Some("python".into()),
                content: "print('hi')".into(),
            }],
        };
        let output = build(&doc);
        assert!(output.contains(".. code-block:: python"));
        assert!(output.contains("   print('hi')"));
    }

    #[test]
    fn test_build_list() {
        let doc = RstDoc {
            blocks: vec![Block::List {
                ordered: false,
                items: vec![
                    vec![Block::Paragraph {
                        inlines: vec![Inline::Text("one".into())],
                    }],
                    vec![Block::Paragraph {
                        inlines: vec![Inline::Text("two".into())],
                    }],
                ],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("- one"));
        assert!(output.contains("- two"));
    }
}
