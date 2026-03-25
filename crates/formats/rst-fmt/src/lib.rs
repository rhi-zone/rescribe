//! reStructuredText (RST) parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-rst` and `rescribe-write-rst` as thin adapter layers.

#![allow(clippy::collapsible_if)]

pub mod events;
pub mod writer;
pub mod batch;
pub use events::{Event, OwnedEvent};
pub use writer::Writer;
pub use batch::{BatchParser, BatchSink, Handler, StreamingParser};

use std::borrow::Cow;

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
    /// RST line block (lines starting with `| `)
    LineBlock {
        lines: Vec<Vec<Inline>>,
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
    let mut iter = EventIter::new(input);
    let blocks = events::collect_doc_from_iter(&mut iter);
    Ok(RstDoc { blocks })
}

/// Lazy-traversal frame for the event iterator.
/// Frames pushed in reverse-emission order, so pop() yields the next event.
/// Memory is O(nesting depth).
enum Frame {
    Event(events::OwnedEvent),
    Blocks(std::vec::IntoIter<Block>),
    Inlines(std::vec::IntoIter<Inline>),
    /// List items are Vec<Block>
    ListItems(std::vec::IntoIter<Vec<Block>>),
    TableRows(std::vec::IntoIter<TableRow>),
    /// Table cells are Vec<Inline>
    TableCells(std::vec::IntoIter<Vec<Inline>>),
    DefinitionItems(std::vec::IntoIter<DefinitionItem>),
    LineBlockLines(std::vec::IntoIter<Vec<Inline>>),
}

pub struct EventIter<'a> {
    pub(crate) lines: Vec<&'a str>,
    pub(crate) line_idx: usize,
    /// Maps underline character to heading level (assigned in order of appearance).
    pub(crate) heading_levels: Vec<char>,
    /// Link targets: name -> url
    pub(crate) link_targets: std::collections::HashMap<String, String>,
    /// Substitution definitions: |name| -> replacement text
    pub(crate) substitutions: std::collections::HashMap<String, String>,
    /// Anonymous link targets (__ url), in order of definition
    pub(crate) anon_targets: Vec<String>,
    /// Holds a code block to be emitted immediately after the current block.
    /// Used when "text::" emits a paragraph and defers the code block.
    pub(crate) pending_block: Option<Block>,
    // ── Iterator state (used when EventIter is driven as an Iterator) ────────────
    /// Lazy frame stack for event traversal. Memory is O(nesting depth).
    pub(crate) frame_stack: Vec<Frame>,
    /// Set to true once the parser has reached EOF during iteration.
    pub(crate) iter_done: bool,
}

impl<'a> EventIter<'a> {
    fn new_uninit(input: &'a str) -> Self {
        let lines: Vec<&str> = input.lines().collect();
        Self {
            lines,
            line_idx: 0,
            heading_levels: Vec::new(),
            link_targets: std::collections::HashMap::new(),
            substitutions: std::collections::HashMap::new(),
            anon_targets: Vec::new(),
            pending_block: None,
            frame_stack: Vec::new(),
            iter_done: false,
        }
    }

    /// Construct an `EventIter` ready to be used as an `Iterator`.
    ///
    /// Runs the link-target pre-scan so that `next()` calls produce correct events.
    pub fn new(input: &'a str) -> Self {
        let mut p = Self::new_uninit(input);
        p.collect_link_targets();
        p
    }

    pub(crate) fn current_line(&self) -> Option<&'a str> {
        self.lines.get(self.line_idx).copied()
    }

    pub(crate) fn peek_line(&self) -> Option<&'a str> {
        self.lines.get(self.line_idx + 1).copied()
    }

    pub(crate) fn advance_line(&mut self) {
        self.line_idx += 1;
    }

    pub(crate) fn is_eof(&self) -> bool {
        self.line_idx >= self.lines.len()
    }

    pub(crate) fn is_blank_line(&self) -> bool {
        self.current_line()
            .map(|l| l.trim().is_empty())
            .unwrap_or(true)
    }

    pub(crate) fn skip_blank_lines(&mut self) {
        while !self.is_eof() && self.is_blank_line() {
            self.advance_line();
        }
    }

    /// First pass: collect link targets (.. _name: url), substitution defs, and anonymous targets
    pub(crate) fn collect_link_targets(&mut self) {
        let mut idx = 0;
        while idx < self.lines.len() {
            let line = self.lines[idx];
            // Named link target: .. _name: url
            if let Some(rest) = line.strip_prefix(".. _") {
                if let Some(colon_idx) = rest.find(':') {
                    let name = rest[..colon_idx].trim().to_lowercase();
                    let url = rest[colon_idx + 1..].trim().to_string();
                    self.link_targets.insert(name, url);
                }
            }
            // Substitution definition: .. |name| replace:: text
            if let Some(rest) = line.strip_prefix(".. |") {
                if let Some(pipe_idx) = rest.find('|') {
                    let sub_name = rest[..pipe_idx].to_lowercase();
                    let after_pipe = rest[pipe_idx + 1..].trim();
                    if let Some(replacement) = after_pipe.strip_prefix("replace::") {
                        let replacement = replacement.trim().to_string();
                        self.substitutions.insert(sub_name, replacement);
                    }
                }
            }
            // Anonymous target: __ url (line starting with exactly "__ ")
            if let Some(url) = line.strip_prefix("__ ") {
                let url = url.trim();
                if !url.is_empty() {
                    self.anon_targets.push(url.to_string());
                }
            }
            idx += 1;
        }
    }

    pub(crate) fn try_parse_block(&mut self) -> Option<Block> {
        // Drain any block deferred by the previous call (e.g. code block after "text::").
        if let Some(pending) = self.pending_block.take() {
            return Some(pending);
        }

        // Skip preamble lines (link targets, substitution defs, anonymous targets)
        // using a loop instead of recursion to avoid stack overflow on long sequences.
        loop {
            // Skip link target definitions (already collected)
            if let Some(line) = self.current_line() {
                if line.starts_with(".. _") && line.contains(':') {
                    self.advance_line();
                    continue;
                }
            }
            // Skip substitution definitions (already collected in first pass)
            if let Some(line) = self.current_line() {
                if line.starts_with(".. |") && line.contains("::") {
                    self.advance_line();
                    // Skip indented continuation lines
                    while !self.is_eof() {
                        let cont = self.current_line().unwrap_or("");
                        if cont.starts_with(' ') || cont.starts_with('\t') {
                            self.advance_line();
                        } else {
                            break;
                        }
                    }
                    continue;
                }
            }
            // Skip anonymous target lines (__ url)
            if let Some(line) = self.current_line() {
                if line.starts_with("__ ") {
                    self.advance_line();
                    continue;
                }
            }
            break;
        }

        // Check for horizontal rule (transition: 4+ underline chars, no following text)
        if let Some(hr) = self.try_parse_horizontal_rule() {
            return Some(hr);
        }

        // Check for heading (text followed by underline)
        if let Some(heading) = self.try_parse_heading() {
            return Some(heading);
        }

        // Check for directive
        if let Some(directive) = self.try_parse_directive() {
            return Some(directive);
        }

        // Check for grid table (+---+)
        if let Some(table) = self.try_parse_grid_table() {
            return Some(table);
        }

        // Check for simple table (=== ===)
        if let Some(table) = self.try_parse_simple_table() {
            return Some(table);
        }

        // Check for line block (| prefix)
        if let Some(lb) = self.try_parse_line_block() {
            return Some(lb);
        }

        // Check for list
        if let Some(list) = self.try_parse_list() {
            return Some(list);
        }

        // Check for field list (:Name: value lines at column 0)
        if let Some(fieldlist) = self.try_parse_field_list() {
            return Some(fieldlist);
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

    fn try_parse_horizontal_rule(&mut self) -> Option<Block> {
        let line = self.current_line()?;
        // RST transitions: 4+ identical punctuation chars, next line blank or EOF
        if line.len() >= 4 && self.is_underline(line) {
            let next = self.peek_line();
            if next.is_none() || next.unwrap().trim().is_empty() {
                self.advance_line();
                return Some(Block::HorizontalRule);
            }
        }
        None
    }

    fn try_parse_heading(&mut self) -> Option<Block> {
        let line = self.current_line()?;

        // Check if this line is all underline chars (possible overline)
        if self.is_underline(line) && !line.is_empty() {
            // Overlined heading: === then title then ===
            let overline_char = line.chars().next()?;
            let next_line = self.peek_line()?;
            // The title may itself look like an adornment line (e.g., "^" for a
            // level-1 heading whose text happens to be "^").  Allow this as long
            // as the title char is DIFFERENT from the overline char — otherwise
            // it's three identical adornment lines and not a valid heading.
            if !next_line.trim().is_empty() {
                let title_first = next_line.trim().chars().next().unwrap_or('\0');
                let title_same_as_overline =
                    self.is_underline(next_line.trim()) && title_first == overline_char;
                if !title_same_as_overline {
                    // Check for underline after title
                    let title = next_line.trim();
                    if let Some(underline) = self.lines.get(self.line_idx + 2) {
                        if self.is_underline(underline) && underline.starts_with(overline_char) {
                            self.advance_line(); // skip overline
                            self.advance_line(); // skip title
                            self.advance_line(); // skip underline
                            let level = self.get_heading_level(overline_char);
                            let inlines = self.inline_from(title);
                            return Some(Block::Heading { level, inlines });
                        }
                    }
                }
            }
        }

        // Underlined heading: title then ===
        // The title may itself look like a line of adornment chars (e.g., "+").
        // Distinguish from the overline case: if the title and underline use the
        // SAME char, it's not a plain-underline heading (that is handled by the
        // overline path above or is ambiguous RST). If they use DIFFERENT chars,
        // treat the first line as the title text.
        if !line.trim().is_empty() {
            if let Some(underline) = self.peek_line() {
                if self.is_underline(underline) && underline.len() >= line.trim().len() {
                    let title_first = line.trim().chars().next().unwrap_or('\0');
                    let underline_first = underline.chars().next().unwrap_or('\0');
                    let title_looks_like_adornment = self.is_underline(line.trim());
                    // Only accept when title doesn't look like adornment, OR it does
                    // but uses a different char than the underline (unambiguous).
                    if !title_looks_like_adornment || title_first != underline_first {
                        let title = line.trim();
                        self.advance_line(); // skip title
                        self.advance_line(); // skip underline
                        let level = self.get_heading_level(underline_first);
                        let inlines = self.inline_from(title);
                        return Some(Block::Heading { level, inlines });
                    }
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

    fn try_parse_footnote_def(&mut self) -> Option<Block> {
        let line = self.current_line()?;

        // Pattern: .. [label] content
        // label is alphanumeric (and may contain hyphens/underscores for citations)
        if !line.starts_with(".. [") {
            return None;
        }
        let rest = &line[4..]; // after ".. ["
        let close_bracket = rest.find(']')?;
        let label = &rest[..close_bracket];
        // Label must be non-empty and followed by space+content or just end of line
        if label.is_empty() {
            return None;
        }
        // Must not be a directive like .. [label]:: (but that's unusual; safe to parse)
        let after_bracket = rest[close_bracket + 1..].trim();

        self.advance_line();

        // Collect continuation lines (indented)
        let mut content = after_bracket.to_string();
        while !self.is_eof() {
            let cont_line = self.current_line().unwrap_or("");
            if cont_line.trim().is_empty() {
                break;
            }
            if cont_line.starts_with(' ') || cont_line.starts_with('\t') {
                if !content.is_empty() {
                    content.push(' ');
                }
                content.push_str(cont_line.trim());
                self.advance_line();
            } else {
                break;
            }
        }

        let inlines = self.inline_from(&content);
        Some(Block::FootnoteDef {
            label: label.to_string(),
            inlines,
        })
    }

    fn try_parse_directive(&mut self) -> Option<Block> {
        let line = self.current_line()?;

        if !line.starts_with(".. ") {
            return None;
        }

        let rest = &line[3..];

        // Check for footnote/citation definition: .. [label] content
        // (before checking for ::, since these have no ::)
        if rest.starts_with('[') {
            if let Some(block) = self.try_parse_footnote_def() {
                return Some(block);
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
                let inlines = self.inline_from(&content);
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
                    Some(self.inline_from(&caption_text))
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
            "admonition" => {
                // Custom admonition with a title: .. admonition:: My Title
                let content = content_lines.join("\n");
                let body_inlines = self.inline_from(&content);
                Block::Admonition {
                    admonition_type: argument.to_string(),
                    children: if content.is_empty() {
                        vec![]
                    } else {
                        vec![Block::Paragraph {
                            inlines: body_inlines,
                        }]
                    },
                }
            }
            "rubric" => {
                // rubric: argument is the heading text; no body
                let inlines = self.inline_from(argument);
                Block::Div {
                    class: None,
                    directive: Some("rubric".to_string()),
                    children: vec![Block::Paragraph { inlines }],
                }
            }
            "container" => {
                // container: argument is the CSS class name
                let content = content_lines.join("\n");
                let children = if content.is_empty() {
                    vec![]
                } else {
                    let inlines = self.inline_from(&content);
                    vec![Block::Paragraph { inlines }]
                };
                Block::Div {
                    class: if argument.is_empty() { None } else { Some(argument.to_string()) },
                    directive: Some("container".to_string()),
                    children,
                }
            }
            _ => {
                // Unknown directive — create generic div (warnings handled by adapter)
                let content = content_lines.join("\n");
                let children = if content.is_empty() {
                    vec![]
                } else {
                    let inlines = self.inline_from(&content);
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

    /// Detect and parse an RST grid table (+---+---+ borders).
    fn try_parse_grid_table(&mut self) -> Option<Block> {
        let line = self.current_line()?;

        // Grid table starts with a border line: +...+
        if !line.starts_with('+') || !line.ends_with('+') {
            return None;
        }
        // Must be a valid border: only +, -, = characters
        if !line.chars().all(|c| c == '+' || c == '-' || c == '=') {
            return None;
        }
        if line.len() < 3 {
            return None;
        }

        // Determine column boundaries from this border line
        let col_boundaries = parse_grid_border(line);
        if col_boundaries.len() < 2 {
            return None;
        }

        // Consume the first border line
        self.advance_line();

        let mut rows: Vec<TableRow> = Vec::new();
        // Track whether the *next* border will be the header separator
        let mut pending_header = true; // first row is a header candidate
        let mut current_row_cells: Vec<Vec<Inline>> = vec![Vec::new(); col_boundaries.len() - 1];
        let num_cols = col_boundaries.len() - 1;

        while !self.is_eof() {
            let cur = self.current_line().unwrap_or("");

            if cur.starts_with('+') && cur.ends_with('+') && cur.chars().all(|c| c == '+' || c == '-' || c == '=') {
                // Border line: uses = for header separator
                let header_sep = cur.contains('=');
                // Finalize current row
                let cells: Vec<Vec<Inline>> = current_row_cells
                    .iter()
                    .map(|c| {
                        let text: String = c.iter().map(|i| match i {
                            Inline::Text(s) => s.clone(),
                            _ => String::new(),
                        }).collect::<Vec<_>>().join("").trim().to_string();
                        self.inline_from(&text)
                    })
                    .collect();
                if cells.iter().any(|c| !c.is_empty()) {
                    let row_is_header = pending_header && header_sep;
                    rows.push(TableRow { cells, is_header: row_is_header });
                }
                if header_sep {
                    pending_header = false; // rows after header separator are body
                }
                current_row_cells = vec![Vec::new(); num_cols];
                self.advance_line();
                // Check if table ended (next line doesn't start with | or +)
                if let Some(next) = self.current_line() {
                    if !next.starts_with('|') && !next.starts_with('+') {
                        break;
                    }
                } else {
                    break;
                }
            } else if cur.starts_with('|') {
                // Content row: extract cell content based on column boundaries.
                // Column positions are byte offsets from the ASCII border line; content
                // rows may contain multi-byte UTF-8 chars, so use .get() to avoid
                // panicking when positions fall inside a multi-byte char.
                for col in 0..num_cols {
                    let start = col_boundaries[col] + 1;
                    let end = col_boundaries[col + 1];
                    if start < cur.len() && end <= cur.len() {
                        let cell_text = cur.get(start..end).unwrap_or("").trim();
                        if !cell_text.is_empty() {
                            if !current_row_cells[col].is_empty() {
                                current_row_cells[col].push(Inline::Text(" ".to_string()));
                            }
                            current_row_cells[col].push(Inline::Text(cell_text.to_string()));
                        }
                    }
                }
                self.advance_line();
            } else {
                break;
            }
        }

        // If we accumulated any rows, return the table
        if rows.is_empty() {
            return None;
        }

        Some(Block::Table { rows })
    }

    /// Detect and parse an RST simple table (=== === borders).
    fn try_parse_simple_table(&mut self) -> Option<Block> {
        let line = self.current_line()?;

        // Simple table starts with a line of `=` separated by spaces
        if !is_simple_table_border(line) {
            return None;
        }

        // Parse column positions from the border line
        let col_spans = parse_simple_table_cols(line);
        if col_spans.len() < 2 {
            return None;
        }

        // Consume the opening border
        self.advance_line();

        let mut rows: Vec<TableRow> = Vec::new();
        let mut header_done = false;

        while !self.is_eof() {
            let cur = self.current_line().unwrap_or("");

            if cur.trim().is_empty() {
                self.advance_line();
                continue;
            }

            if is_simple_table_border(cur) {
                // If we haven't seen the header separator yet, mark it
                if !header_done && !rows.is_empty() {
                    header_done = true;
                }
                self.advance_line();
                // Check if the table ended (two borders in a row, or this was the closing border)
                if let Some(next) = self.current_line() {
                    if next.trim().is_empty() || is_simple_table_border(next) {
                        // Consume closing border if present
                        if is_simple_table_border(next) {
                            self.advance_line();
                        }
                        break;
                    }
                } else {
                    break;
                }
                continue;
            }

            // Content row
            let mut cells = Vec::new();
            for &(start, end) in &col_spans {
                // Column positions come from the ASCII border line; content rows may
                // contain multi-byte UTF-8 characters, so byte positions may not
                // align with char boundaries. Use .get() to avoid panics.
                let cell_text = if end <= cur.len() {
                    cur.get(start..end).unwrap_or("").trim().to_string()
                } else if start < cur.len() {
                    cur.get(start..).unwrap_or("").trim().to_string()
                } else {
                    String::new()
                };
                cells.push(self.inline_from(&cell_text));
            }
            rows.push(TableRow {
                cells,
                is_header: !header_done,
            });
            self.advance_line();
        }

        if rows.is_empty() {
            return None;
        }

        Some(Block::Table { rows })
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
            // Require non-empty prefix: an empty prefix (line starting with ". ")
            // is NOT a numbered list — it's a paragraph beginning with a period.
            if !prefix.is_empty() && (prefix.chars().all(|c| c.is_ascii_digit()) || prefix == "#") {
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
                    let next_indent = self.get_line_indent(next_line);
                    let next_trimmed = next_line.trim_start();
                    // Continue if next item has same indent and same bullet
                    if next_indent == indent
                        && next_trimmed.starts_with(&format!("{} ", bullet))
                    {
                        self.advance_line();
                        continue;
                    }
                }
                break;
            }

            if current_indent < indent && indent > 0 {
                break;
            }

            if let Some(rest) = trimmed.strip_prefix(&format!("{} ", bullet)) {
                let item_blocks = self.parse_list_item_blocks(rest, indent);
                items.push(item_blocks);
            } else if current_indent > indent {
                // Continuation of previous item (shouldn't normally happen here)
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
                    let item_blocks = self.parse_list_item_blocks(rest, indent);
                    items.push(item_blocks);
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

    /// Parse a list item's content into a Vec<Block>, supporting nested sublists.
    /// `first_line` is the text after the bullet/number marker.
    /// `list_indent` is the column of the parent list's bullet character.
    fn parse_list_item_blocks(&mut self, first_line: &str, list_indent: usize) -> Vec<Block> {
        self.advance_line();

        let mut content = first_line.to_string();
        let mut extra_blocks: Vec<Block> = Vec::new();

        // Collect continuation lines and detect nested sublists
        loop {
            if self.is_eof() {
                break;
            }
            let line = self.current_line().unwrap_or("");

            if line.trim().is_empty() {
                // Blank line: check if followed by a nested list
                let next_idx = self.line_idx + 1;
                if next_idx < self.lines.len() {
                    let next_line = self.lines[next_idx];
                    let next_indent = self.get_line_indent(next_line);
                    let next_trimmed = next_line.trim_start();
                    // If the next line is indented more than the list bullet and is a sublist
                    if next_indent > list_indent
                        && (next_trimmed.starts_with("* ")
                            || next_trimmed.starts_with("- ")
                            || next_trimmed.starts_with("+ "))
                    {
                        self.advance_line(); // skip blank
                        // Parse the nested sublist
                        if let Some(sublist) = self.try_parse_list() {
                            extra_blocks.push(sublist);
                        }
                        continue;
                    }
                }
                break;
            }

            let current_indent = self.get_line_indent(line);
            let trimmed = line.trim_start();

            // If same or less indent than list bullet, stop
            if current_indent <= list_indent && !line.trim().is_empty() {
                break;
            }

            // If indented more and it's a sublist, parse as nested list
            if current_indent > list_indent
                && (trimmed.starts_with("* ")
                    || trimmed.starts_with("- ")
                    || trimmed.starts_with("+ "))
            {
                if let Some(sublist) = self.try_parse_list() {
                    extra_blocks.push(sublist);
                }
                continue;
            }

            // Numbered nested list
            if current_indent > list_indent {
                if let Some(idx) = trimmed.find(". ") {
                    let prefix = &trimmed[..idx];
                    if prefix.chars().all(|c| c.is_ascii_digit()) || prefix == "#" {
                        if let Some(sublist) = self.try_parse_list() {
                            extra_blocks.push(sublist);
                            continue;
                        }
                    }
                }
            }

            // Regular continuation text (indented)
            if current_indent > list_indent {
                content.push(' ');
                content.push_str(trimmed);
                self.advance_line();
            } else {
                break;
            }
        }

        let mut blocks = vec![Block::Paragraph {
            inlines: self.inline_from(&content),
        }];
        blocks.extend(extra_blocks);
        blocks
    }

    fn try_parse_field_list(&mut self) -> Option<Block> {
        let line = self.current_line()?;

        // Field list: starts with `:FieldName:` at column 0
        if !line.starts_with(':') {
            return None;
        }
        // Find the closing colon of the field name
        let rest = &line[1..];
        let close_colon = rest.find(':')?;
        let field_name = &rest[..close_colon];
        // Field name must be non-empty and not contain spaces (avoid matching ::)
        if field_name.is_empty() || field_name.contains(' ') {
            return None;
        }
        // Make sure this isn't a role-based inline like :role:`text`
        // A field list line at column 0 must have `:name: value` structure
        let after_name_colon = &rest[close_colon + 1..];
        if !after_name_colon.starts_with(' ') && !after_name_colon.is_empty() {
            return None;
        }

        let mut items = Vec::new();

        while !self.is_eof() {
            let cur_line = self.current_line().unwrap_or("");

            if cur_line.trim().is_empty() {
                self.advance_line();
                continue;
            }

            if !cur_line.starts_with(':') {
                break;
            }
            let r = &cur_line[1..];
            let Some(cc) = r.find(':') else { break };
            let fname = &r[..cc];
            if fname.is_empty() || fname.contains(' ') {
                break;
            }
            let fvalue = r[cc + 1..].trim();

            self.advance_line();

            // Collect continuation lines (indented)
            let mut value = fvalue.to_string();
            while !self.is_eof() {
                let cont = self.current_line().unwrap_or("");
                if cont.trim().is_empty() {
                    break;
                }
                if cont.starts_with(' ') || cont.starts_with('\t') {
                    if !value.is_empty() {
                        value.push(' ');
                    }
                    value.push_str(cont.trim());
                    self.advance_line();
                } else {
                    break;
                }
            }

            let term = vec![Inline::Text(fname.to_string())];
            let desc = self.inline_from(&value);
            items.push(DefinitionItem { term, desc });
        }

        if items.is_empty() {
            return None;
        }

        Some(Block::DefinitionList { items })
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
                        let term = self.inline_from(term_str);

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

                        let desc = self.inline_from(&def_content);
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

        // "text::" — emit the introductory paragraph, defer the code block.
        // In RST, "Some text::" means: emit "Some text:" as a paragraph, then
        // emit the following indented block as a literal code block.
        if line.trim_end().ends_with("::") && line.trim_end().len() > 2 {
            // Extract the paragraph text (strip trailing :: → single :, or remove
            // entirely if text is just "::").
            let trimmed = line.trim_end();
            let intro_text = {
                let without_dcolon = &trimmed[..trimmed.len() - 1]; // strip one ':'
                without_dcolon.trim_end()
            };
            let inlines = self.inline_from(intro_text);
            self.advance_line();
            self.skip_blank_lines();

            // Collect the following indented literal block.
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

            let code_content = content_lines.join("\n");
            // Defer the code block; return the paragraph first.
            self.pending_block = Some(Block::CodeBlock {
                language: None,
                content: code_content,
            });
            return Some(Block::Paragraph { inlines });
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

            let inlines = self.inline_from(&content);
            return Some(Block::Blockquote {
                children: vec![Block::Paragraph { inlines }],
            });
        }

        None
    }

    fn try_parse_line_block(&mut self) -> Option<Block> {
        let line = self.current_line()?;
        // Line block: lines starting with "| " or the bare "|" (empty line in block)
        if !line.starts_with("| ") && line != "|" {
            return None;
        }
        let mut lines: Vec<Vec<Inline>> = Vec::new();
        while !self.is_eof() {
            let cur = self.current_line().unwrap_or("");
            if let Some(text) = cur.strip_prefix("| ") {
                lines.push(self.inline_from(text));
                self.advance_line();
            } else if cur == "|" {
                lines.push(vec![]);
                self.advance_line();
            } else {
                break;
            }
        }
        if lines.is_empty() {
            return None;
        }
        Some(Block::LineBlock { lines })
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

        let inlines = self.inline_from(&content);
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

    /// Parse inline content with substitution expansion and anonymous link resolution.
    fn inline_from(&self, content: &str) -> Vec<Inline> {
        // Expand substitutions: replace |name| with the defined replacement text
        let expanded = expand_substitutions(content, &self.substitutions);
        let mut nodes = parse_inline_content(&expanded, &self.link_targets);
        // Resolve anonymous link placeholders in order
        resolve_anon_links(&mut nodes, &self.anon_targets);
        nodes
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
                            "strike" | "strikethrough" | "del" => {
                                let ch = parse_inline_content(&text, link_targets);
                                Inline::Strikeout(ch)
                            }
                            "underline" | "u" => {
                                let ch = parse_inline_content(&text, link_targets);
                                Inline::Underline(ch)
                            }
                            "small-caps" | "smallcaps" | "sc" => {
                                let ch = parse_inline_content(&text, link_targets);
                                Inline::SmallCaps(ch)
                            }
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

        // Inline link: `text <url>`_ or `text <url>`__ (anonymous) or `text`__ (anon ref)
        if chars[pos] == '`' {
            if let Some((end, text)) = find_closing_char(&chars, pos + 1, '`') {
                // Check for trailing __ (anonymous link — must check before single _)
                if end + 2 < chars.len() && chars[end + 1] == '_' && chars[end + 2] == '_' {
                    if let Some(angle_start) = text.rfind('<') {
                        if text.ends_with('>') {
                            let link_text = text[..angle_start].trim();
                            let url = &text[angle_start + 1..text.len() - 1];
                            // When no explicit text, RST uses the URL as display text.
                            let display = if link_text.is_empty() { url } else { link_text };
                            nodes.push(Inline::Link {
                                url: url.to_string(),
                                children: vec![Inline::Text(display.to_string())],
                            });
                            pos = end + 3;
                            continue;
                        }
                    }
                    // Anonymous reference: `text`__ — placeholder resolved later
                    nodes.push(Inline::Link {
                        url: "rst:anon".to_string(),
                        children: vec![Inline::Text(text.to_string())],
                    });
                    pos = end + 3;
                    continue;
                }

                // Check for trailing _ (named link)
                if end + 1 < chars.len() && chars[end + 1] == '_' {
                    // Check if it's an inline link with URL
                    if let Some(angle_start) = text.rfind('<') {
                        if text.ends_with('>') {
                            let link_text = text[..angle_start].trim();
                            let url = &text[angle_start + 1..text.len() - 1];
                            // When no explicit text, RST uses the URL as display text.
                            let display = if link_text.is_empty() { url } else { link_text };
                            nodes.push(Inline::Link {
                                url: url.to_string(),
                                children: vec![Inline::Text(display.to_string())],
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

        // Footnote/citation reference: [label]_
        if chars[pos] == '[' {
            let mut end = pos + 1;
            while end < chars.len() && chars[end] != ']' && chars[end] != '\n' {
                end += 1;
            }
            if end < chars.len()
                && chars[end] == ']'
                && end + 1 < chars.len()
                && chars[end + 1] == '_'
            {
                let label: String = chars[pos + 1..end].iter().collect();
                if !label.is_empty()
                    && label
                        .chars()
                        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
                {
                    nodes.push(Inline::FootnoteRef { label });
                    pos = end + 2;
                    continue;
                }
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

/// Return the column positions of `+` characters in a grid table border line.
fn parse_grid_border(line: &str) -> Vec<usize> {
    line.char_indices()
        .filter(|(_, c)| *c == '+')
        .map(|(i, _)| i)
        .collect()
}

/// Return true if `line` is a simple-table border (one or more runs of `=` separated by spaces).
fn is_simple_table_border(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }
    // Must start with '=' and consist only of '=' and ' '
    trimmed.starts_with('=') && trimmed.chars().all(|c| c == '=' || c == ' ')
}

/// Parse column (start, end) byte offsets from a simple-table border line.
fn parse_simple_table_cols(line: &str) -> Vec<(usize, usize)> {
    let mut cols = Vec::new();
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'=' {
            let start = i;
            while i < bytes.len() && bytes[i] == b'=' {
                i += 1;
            }
            cols.push((start, i));
        } else {
            i += 1;
        }
    }
    cols
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

/// Expand substitution references `|name|` in content using the given map.
fn expand_substitutions(
    content: &str,
    subs: &std::collections::HashMap<String, String>,
) -> String {
    if subs.is_empty() || !content.contains('|') {
        return content.to_string();
    }
    let mut result = String::with_capacity(content.len());
    let chars: Vec<char> = content.chars().collect();
    let mut pos = 0;
    while pos < chars.len() {
        if chars[pos] == '|' {
            // Find closing |
            let mut end = pos + 1;
            while end < chars.len() && chars[end] != '|' {
                end += 1;
            }
            if end < chars.len() && chars[end] == '|' {
                let name: String = chars[pos + 1..end].iter().collect();
                let key = name.to_lowercase();
                if let Some(replacement) = subs.get(&key) {
                    result.push_str(replacement);
                    pos = end + 1;
                    continue;
                }
            }
        }
        result.push(chars[pos]);
        pos += 1;
    }
    result
}

/// Resolve anonymous link placeholders ("rst:anon") in order using the anon_targets list.
fn resolve_anon_links(nodes: &mut [Inline], anon_targets: &[String]) {
    let mut idx = 0;
    resolve_anon_links_in(nodes, anon_targets, &mut idx);
}

fn resolve_anon_links_in(nodes: &mut [Inline], anon_targets: &[String], idx: &mut usize) {
    for node in nodes.iter_mut() {
        match node {
            Inline::Link { url, children } if url == "rst:anon" => {
                if *idx < anon_targets.len() {
                    *url = anon_targets[*idx].clone();
                    *idx += 1;
                }
                resolve_anon_links_in(children, anon_targets, idx);
            }
            Inline::Link { children, .. }
            | Inline::Emphasis(children)
            | Inline::Strong(children)
            | Inline::Strikeout(children)
            | Inline::Underline(children)
            | Inline::Subscript(children)
            | Inline::Superscript(children)
            | Inline::SmallCaps(children)
            | Inline::RstSpan { children, .. }
            | Inline::Quoted { children, .. }
            | Inline::FootnoteDef { children, .. } => {
                resolve_anon_links_in(children, anon_targets, idx);
            }
            _ => {}
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

        Block::LineBlock { lines } => {
            for line_inlines in lines {
                ctx.write("| ");
                build_inlines(line_inlines, ctx);
                ctx.write("\n");
            }
            ctx.write("\n");
        }
    }
}

fn build_heading(level: i64, inlines: &[Inline], ctx: &mut BuildContext) {
    let mut plain_text = String::new();
    collect_text_from_inlines(inlines, &mut plain_text);

    // Render the inline markup to a temporary buffer to measure the RST
    // source length (e.g., "**e**" is 5 chars, not 1).  The underline must be
    // at least as long as the RST source line, not the plain-text length.
    let mut tmp = BuildContext::new();
    build_inlines(inlines, &mut tmp);
    let rendered = tmp.output;
    let render_len = rendered.len().max(1);

    let preferred_char = match level {
        1 => '=',
        2 => '-',
        3 => '~',
        4 => '^',
        5 => '"',
        _ => '\'',
    };

    // If the title itself looks like an adornment line (all chars equal the
    // preferred underline char), the emitted RST would be ambiguous: e.g.,
    // "-\n-\n" can't be parsed as a heading.  Use a different underline char.
    let title_clashes = !plain_text.is_empty() && plain_text.chars().all(|c| c == preferred_char);
    let underline_char = if title_clashes {
        // Pick the first HEADING_CHAR that differs from preferred_char.
        HEADING_CHARS
            .iter()
            .find(|&&c| c != preferred_char)
            .copied()
            .unwrap_or('=')
    } else {
        preferred_char
    };

    // For level 1, add overline (only when using the preferred char and no clash)
    if level == 1 && !title_clashes {
        let line: String = std::iter::repeat_n(underline_char, render_len).collect();
        ctx.write(&line);
        ctx.write("\n");
    }

    ctx.write(&rendered);
    ctx.write("\n");

    let line: String = std::iter::repeat_n(underline_char, render_len).collect();
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

// ── Iterator implementation ───────────────────────────────────────────────────

impl<'a> EventIter<'a> {
    fn expand_block(&mut self, block: Block) {
        use events::{Event as Ev, OwnedEvent};
        match block {
            Block::Paragraph { inlines } => {
                self.frame_stack.push(Frame::Event(OwnedEvent::EndParagraph));
                if !inlines.is_empty() {
                    self.frame_stack.push(Frame::Inlines(inlines.into_iter()));
                }
                self.frame_stack.push(Frame::Event(OwnedEvent::StartParagraph));
            }
            Block::Heading { level, inlines } => {
                self.frame_stack.push(Frame::Event(OwnedEvent::EndHeading));
                if !inlines.is_empty() {
                    self.frame_stack.push(Frame::Inlines(inlines.into_iter()));
                }
                self.frame_stack.push(Frame::Event(OwnedEvent::StartHeading { level }));
            }
            Block::CodeBlock { language, content } => {
                self.frame_stack.push(Frame::Event(OwnedEvent::EndCodeBlock));
                self.frame_stack.push(Frame::Event(Ev::CodeBlockContent(Cow::Owned(content))));
                self.frame_stack.push(Frame::Event(OwnedEvent::StartCodeBlock { language }));
            }
            Block::Blockquote { children } => {
                self.frame_stack.push(Frame::Event(OwnedEvent::EndBlockquote));
                if !children.is_empty() {
                    self.frame_stack.push(Frame::Blocks(children.into_iter()));
                }
                self.frame_stack.push(Frame::Event(OwnedEvent::StartBlockquote));
            }
            Block::List { ordered, items } => {
                self.frame_stack.push(Frame::Event(OwnedEvent::EndList));
                if !items.is_empty() {
                    self.frame_stack.push(Frame::ListItems(items.into_iter()));
                }
                self.frame_stack.push(Frame::Event(OwnedEvent::StartList { ordered }));
            }
            Block::DefinitionList { items } => {
                self.frame_stack.push(Frame::Event(OwnedEvent::EndDefinitionList));
                if !items.is_empty() {
                    self.frame_stack.push(Frame::DefinitionItems(items.into_iter()));
                }
                self.frame_stack.push(Frame::Event(OwnedEvent::StartDefinitionList));
            }
            Block::Figure { url, alt, caption } => {
                self.frame_stack.push(Frame::Event(OwnedEvent::EndFigure));
                if let Some(cap_inlines) = caption {
                    if !cap_inlines.is_empty() {
                        self.frame_stack.push(Frame::Inlines(cap_inlines.into_iter()));
                    }
                }
                self.frame_stack.push(Frame::Event(OwnedEvent::StartFigure { url, alt }));
            }
            Block::Image { url, alt, title } => {
                self.frame_stack.push(Frame::Event(OwnedEvent::ImageBlock { url, alt, title }));
            }
            Block::RawBlock { format, content } => {
                self.frame_stack.push(Frame::Event(OwnedEvent::RawBlock { format, content }));
            }
            Block::Div { class, directive, children } => {
                self.frame_stack.push(Frame::Event(OwnedEvent::EndDiv));
                if !children.is_empty() {
                    self.frame_stack.push(Frame::Blocks(children.into_iter()));
                }
                self.frame_stack.push(Frame::Event(OwnedEvent::StartDiv { class, directive }));
            }
            Block::HorizontalRule => {
                self.frame_stack.push(Frame::Event(OwnedEvent::HorizontalRule));
            }
            Block::Table { rows } => {
                self.frame_stack.push(Frame::Event(OwnedEvent::EndTable));
                if !rows.is_empty() {
                    self.frame_stack.push(Frame::TableRows(rows.into_iter()));
                }
                self.frame_stack.push(Frame::Event(OwnedEvent::StartTable));
            }
            Block::FootnoteDef { label, inlines } => {
                self.frame_stack.push(Frame::Event(OwnedEvent::EndFootnoteDef));
                if !inlines.is_empty() {
                    self.frame_stack.push(Frame::Inlines(inlines.into_iter()));
                }
                self.frame_stack.push(Frame::Event(OwnedEvent::StartFootnoteDef { label }));
            }
            Block::MathDisplay { source } => {
                self.frame_stack.push(Frame::Event(OwnedEvent::MathDisplay { source }));
            }
            Block::Admonition { admonition_type, children } => {
                self.frame_stack.push(Frame::Event(OwnedEvent::EndAdmonition));
                if !children.is_empty() {
                    self.frame_stack.push(Frame::Blocks(children.into_iter()));
                }
                self.frame_stack.push(Frame::Event(OwnedEvent::StartAdmonition { admonition_type }));
            }
            Block::LineBlock { lines } => {
                self.frame_stack.push(Frame::Event(OwnedEvent::EndLineBlock));
                if !lines.is_empty() {
                    self.frame_stack.push(Frame::LineBlockLines(lines.into_iter()));
                }
                self.frame_stack.push(Frame::Event(OwnedEvent::StartLineBlock));
            }
        }
    }

    fn expand_inline(&mut self, inline: Inline) {
        use events::{Event as Ev, OwnedEvent};
        match inline {
            Inline::Text(s) => {
                self.frame_stack.push(Frame::Event(Ev::Text(Cow::Owned(s))));
            }
            Inline::SoftBreak => {
                self.frame_stack.push(Frame::Event(OwnedEvent::SoftBreak));
            }
            Inline::LineBreak => {
                self.frame_stack.push(Frame::Event(OwnedEvent::LineBreak));
            }
            Inline::Emphasis(children) => {
                self.frame_stack.push(Frame::Event(OwnedEvent::EndEmphasis));
                if !children.is_empty() {
                    self.frame_stack.push(Frame::Inlines(children.into_iter()));
                }
                self.frame_stack.push(Frame::Event(OwnedEvent::StartEmphasis));
            }
            Inline::Strong(children) => {
                self.frame_stack.push(Frame::Event(OwnedEvent::EndStrong));
                if !children.is_empty() {
                    self.frame_stack.push(Frame::Inlines(children.into_iter()));
                }
                self.frame_stack.push(Frame::Event(OwnedEvent::StartStrong));
            }
            Inline::Strikeout(children) => {
                self.frame_stack.push(Frame::Event(OwnedEvent::EndStrikeout));
                if !children.is_empty() {
                    self.frame_stack.push(Frame::Inlines(children.into_iter()));
                }
                self.frame_stack.push(Frame::Event(OwnedEvent::StartStrikeout));
            }
            Inline::Underline(children) => {
                self.frame_stack.push(Frame::Event(OwnedEvent::EndUnderline));
                if !children.is_empty() {
                    self.frame_stack.push(Frame::Inlines(children.into_iter()));
                }
                self.frame_stack.push(Frame::Event(OwnedEvent::StartUnderline));
            }
            Inline::Subscript(children) => {
                self.frame_stack.push(Frame::Event(OwnedEvent::EndSubscript));
                if !children.is_empty() {
                    self.frame_stack.push(Frame::Inlines(children.into_iter()));
                }
                self.frame_stack.push(Frame::Event(OwnedEvent::StartSubscript));
            }
            Inline::Superscript(children) => {
                self.frame_stack.push(Frame::Event(OwnedEvent::EndSuperscript));
                if !children.is_empty() {
                    self.frame_stack.push(Frame::Inlines(children.into_iter()));
                }
                self.frame_stack.push(Frame::Event(OwnedEvent::StartSuperscript));
            }
            Inline::SmallCaps(children) => {
                self.frame_stack.push(Frame::Event(OwnedEvent::EndSmallCaps));
                if !children.is_empty() {
                    self.frame_stack.push(Frame::Inlines(children.into_iter()));
                }
                self.frame_stack.push(Frame::Event(OwnedEvent::StartSmallCaps));
            }
            Inline::Code(s) => {
                self.frame_stack.push(Frame::Event(Ev::Code(Cow::Owned(s))));
            }
            Inline::Link { url, children } => {
                self.frame_stack.push(Frame::Event(OwnedEvent::EndLink));
                if !children.is_empty() {
                    self.frame_stack.push(Frame::Inlines(children.into_iter()));
                }
                self.frame_stack.push(Frame::Event(OwnedEvent::StartLink { url }));
            }
            Inline::Image { url, alt } => {
                self.frame_stack.push(Frame::Event(OwnedEvent::InlineImage { url, alt }));
            }
            Inline::FootnoteRef { label } => {
                self.frame_stack.push(Frame::Event(OwnedEvent::FootnoteRef { label }));
            }
            Inline::FootnoteDef { label, children } => {
                self.frame_stack.push(Frame::Event(OwnedEvent::EndFootnoteDefInline));
                if !children.is_empty() {
                    self.frame_stack.push(Frame::Inlines(children.into_iter()));
                }
                self.frame_stack.push(Frame::Event(OwnedEvent::StartFootnoteDefInline { label }));
            }
            Inline::Quoted { quote_type, children } => {
                self.frame_stack.push(Frame::Event(OwnedEvent::EndQuoted));
                if !children.is_empty() {
                    self.frame_stack.push(Frame::Inlines(children.into_iter()));
                }
                self.frame_stack.push(Frame::Event(OwnedEvent::StartQuoted { quote_type }));
            }
            Inline::MathInline { source } => {
                self.frame_stack.push(Frame::Event(OwnedEvent::MathInline { source }));
            }
            Inline::RstSpan { role, children } => {
                self.frame_stack.push(Frame::Event(OwnedEvent::EndRstSpan));
                if !children.is_empty() {
                    self.frame_stack.push(Frame::Inlines(children.into_iter()));
                }
                self.frame_stack.push(Frame::Event(OwnedEvent::StartRstSpan { role }));
            }
        }
    }
}

impl<'a> Iterator for EventIter<'a> {
    type Item = events::Event<'a>;

    fn next(&mut self) -> Option<events::Event<'a>> {
        use events::OwnedEvent;
        loop {
            match self.frame_stack.pop() {
                Some(Frame::Event(ev)) => return Some(ev),
                Some(Frame::Blocks(mut iter)) => {
                    if let Some(block) = iter.next() {
                        self.frame_stack.push(Frame::Blocks(iter));
                        self.expand_block(block);
                    }
                    continue;
                }
                Some(Frame::Inlines(mut iter)) => {
                    if let Some(inline) = iter.next() {
                        self.frame_stack.push(Frame::Inlines(iter));
                        self.expand_inline(inline);
                    }
                    continue;
                }
                Some(Frame::ListItems(mut iter)) => {
                    if let Some(item_blocks) = iter.next() {
                        self.frame_stack.push(Frame::ListItems(iter));
                        self.frame_stack.push(Frame::Event(OwnedEvent::EndListItem));
                        if !item_blocks.is_empty() {
                            self.frame_stack.push(Frame::Blocks(item_blocks.into_iter()));
                        }
                        self.frame_stack.push(Frame::Event(OwnedEvent::StartListItem));
                    }
                    continue;
                }
                Some(Frame::TableRows(mut iter)) => {
                    if let Some(row) = iter.next() {
                        let is_header = row.is_header;
                        self.frame_stack.push(Frame::TableRows(iter));
                        self.frame_stack.push(Frame::Event(OwnedEvent::EndTableRow));
                        if !row.cells.is_empty() {
                            self.frame_stack.push(Frame::TableCells(row.cells.into_iter()));
                        }
                        self.frame_stack.push(Frame::Event(OwnedEvent::StartTableRow { is_header }));
                    }
                    continue;
                }
                Some(Frame::TableCells(mut iter)) => {
                    if let Some(cell_inlines) = iter.next() {
                        self.frame_stack.push(Frame::TableCells(iter));
                        self.frame_stack.push(Frame::Event(OwnedEvent::EndTableCell));
                        if !cell_inlines.is_empty() {
                            self.frame_stack.push(Frame::Inlines(cell_inlines.into_iter()));
                        }
                        self.frame_stack.push(Frame::Event(OwnedEvent::StartTableCell));
                    }
                    continue;
                }
                Some(Frame::DefinitionItems(mut iter)) => {
                    if let Some(item) = iter.next() {
                        self.frame_stack.push(Frame::DefinitionItems(iter));
                        self.frame_stack.push(Frame::Event(OwnedEvent::EndDefinitionDesc));
                        if !item.desc.is_empty() {
                            self.frame_stack.push(Frame::Inlines(item.desc.into_iter()));
                        }
                        self.frame_stack.push(Frame::Event(OwnedEvent::StartDefinitionDesc));
                        self.frame_stack.push(Frame::Event(OwnedEvent::EndDefinitionTerm));
                        if !item.term.is_empty() {
                            self.frame_stack.push(Frame::Inlines(item.term.into_iter()));
                        }
                        self.frame_stack.push(Frame::Event(OwnedEvent::StartDefinitionTerm));
                    }
                    continue;
                }
                Some(Frame::LineBlockLines(mut iter)) => {
                    if let Some(line_inlines) = iter.next() {
                        self.frame_stack.push(Frame::LineBlockLines(iter));
                        self.frame_stack.push(Frame::Event(OwnedEvent::EndLineBlockLine));
                        if !line_inlines.is_empty() {
                            self.frame_stack.push(Frame::Inlines(line_inlines.into_iter()));
                        }
                        self.frame_stack.push(Frame::Event(OwnedEvent::StartLineBlockLine));
                    }
                    continue;
                }
                None => {
                    // Check pending_block first (may have been set by the previous try_parse_block).
                    if let Some(pending) = self.pending_block.take() {
                        self.expand_block(pending);
                        continue;
                    }
                    if self.iter_done {
                        return None;
                    }
                    self.skip_blank_lines();
                    if self.is_eof() {
                        self.iter_done = true;
                        return None;
                    }
                    if let Some(block) = self.try_parse_block() {
                        self.expand_block(block);
                    } else {
                        self.advance_line();
                    }
                    continue;
                }
            }
        }
    }
}

// ── Public streaming API ──────────────────────────────────────────────────────

/// Parse `input` as RST and return a streaming [`EventIter`].
pub fn events(input: &str) -> EventIter<'_> {
    EventIter::new(input)
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

    /// Heading whose title text looks like an RST adornment line (a single
    /// repeated HEADING_CHARS character) must survive a build→parse roundtrip.
    #[test]
    fn test_heading_adornment_title_roundtrip() {
        // "+" is a HEADING_CHAR, so a heading with text "+" historically
        // caused the parser to mistake the title line for an overline.
        for title in &["+", "~", "=", "-", "^", "\""] {
            let doc = RstDoc {
                blocks: vec![Block::Heading {
                    level: 2,
                    inlines: vec![Inline::Text((*title).into())],
                }],
            };
            let rst_output = build(&doc);
            let reparsed = parse(&rst_output).expect("parse should not fail");
            assert_eq!(
                reparsed.blocks.len(),
                1,
                "heading with title {title:?} should round-trip; got: {reparsed:?}\nRST:\n{rst_output}"
            );
            if let Block::Heading { inlines, .. } = &reparsed.blocks[0] {
                let mut text = String::new();
                collect_text_from_inlines(inlines, &mut text);
                assert_eq!(
                    text, *title,
                    "heading title {title:?} changed after roundtrip"
                );
            } else {
                panic!("expected Heading block, got {:?}", reparsed.blocks[0]);
            }
        }
    }

    /// Long heading title consisting of grid-table border chars (4+ '+' chars)
    /// must survive a build→parse roundtrip at all heading levels.
    #[test]
    fn test_heading_long_adornment_title_roundtrip() {
        let title = "+".repeat(103);
        for level in 1i64..=5 {
            let doc = RstDoc {
                blocks: vec![Block::Heading {
                    level,
                    inlines: vec![Inline::Text(title.clone())],
                }],
            };
            let rst_output = build(&doc);
            let reparsed = parse(&rst_output).expect("parse should not fail");
            let mut text = String::new();
            for block in &reparsed.blocks {
                if let Block::Heading { inlines, .. } = block {
                    collect_text_from_inlines(inlines, &mut text);
                }
            }
            assert_eq!(
                text, title,
                "level {level} heading with 103-char '+' title lost after roundtrip\nRST:\n{rst_output}"
            );
        }
    }
}
