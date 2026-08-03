//! reStructuredText (RST) parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-rst` and `rescribe-write-rst` as thin adapter layers.

#![allow(clippy::collapsible_if)]

pub mod batch;
pub mod events;
pub mod writer;

pub use batch::{BatchParser, BatchSink, Handler, StreamingParser};
pub use events::{Event, OwnedEvent};
pub use writer::Writer;

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
///
/// The `'a` lifetime is the input the document was parsed from. Text payloads
/// are [`Cow<'a, str>`], borrowed straight from that input wherever the source
/// span is contiguous and needs no transformation (see the crate-level
/// "Borrowing" notes). Use [`RstDoc::into_owned`] for a `'static` document.
#[derive(Debug, Clone, Default)]
pub struct RstDoc<'a> {
    pub blocks: Vec<Block<'a>>,
}

/// Block-level element.
#[derive(Debug, Clone)]
pub enum Block<'a> {
    Paragraph {
        inlines: Vec<Inline<'a>>,
    },
    Heading {
        level: i64,
        inlines: Vec<Inline<'a>>,
    },
    CodeBlock {
        language: Option<Cow<'a, str>>,
        content: Cow<'a, str>,
    },
    Blockquote {
        children: Vec<Block<'a>>,
    },
    List {
        ordered: bool,
        items: Vec<Vec<Block<'a>>>,
    },
    DefinitionList {
        items: Vec<DefinitionItem<'a>>,
    },
    Figure {
        url: Cow<'a, str>,
        alt: Option<Cow<'a, str>>,
        caption: Option<Vec<Inline<'a>>>,
    },
    Image {
        url: Cow<'a, str>,
        alt: Option<Cow<'a, str>>,
        title: Option<Cow<'a, str>>,
    },
    RawBlock {
        format: Cow<'a, str>,
        content: Cow<'a, str>,
    },
    Div {
        class: Option<Cow<'a, str>>,
        directive: Option<Cow<'a, str>>,
        children: Vec<Block<'a>>,
    },
    HorizontalRule,
    Table {
        rows: Vec<TableRow<'a>>,
    },
    FootnoteDef {
        label: Cow<'a, str>,
        inlines: Vec<Inline<'a>>,
    },
    MathDisplay {
        source: Cow<'a, str>,
    },
    Admonition {
        admonition_type: Cow<'a, str>,
        children: Vec<Block<'a>>,
    },
}

/// A definition list item (term + description pair).
#[derive(Debug, Clone)]
pub struct DefinitionItem<'a> {
    pub term: Vec<Inline<'a>>,
    pub desc: Vec<Inline<'a>>,
}

/// A table row.
#[derive(Debug, Clone)]
pub struct TableRow<'a> {
    pub cells: Vec<Vec<Inline<'a>>>,
    pub is_header: bool,
}

/// Inline element.
#[derive(Debug, Clone)]
pub enum Inline<'a> {
    Text(Cow<'a, str>),
    Emphasis(Vec<Inline<'a>>),
    Strong(Vec<Inline<'a>>),
    Strikeout(Vec<Inline<'a>>),
    Underline(Vec<Inline<'a>>),
    Subscript(Vec<Inline<'a>>),
    Superscript(Vec<Inline<'a>>),
    Code(Cow<'a, str>),
    Link {
        url: Cow<'a, str>,
        children: Vec<Inline<'a>>,
    },
    Image {
        url: Cow<'a, str>,
        alt: Cow<'a, str>,
    },
    LineBreak,
    SoftBreak,
    FootnoteRef {
        label: Cow<'a, str>,
    },
    FootnoteDef {
        label: Cow<'a, str>,
        children: Vec<Inline<'a>>,
    },
    SmallCaps(Vec<Inline<'a>>),
    Quoted {
        quote_type: Cow<'a, str>,
        children: Vec<Inline<'a>>,
    },
    MathInline {
        source: Cow<'a, str>,
    },
    /// RST role-based span with unknown role
    RstSpan {
        role: Cow<'a, str>,
        children: Vec<Inline<'a>>,
    },
}

// ── into_owned ────────────────────────────────────────────────────────────────
//
// Explicit, opt-in conversion to a `'static` tree. Nothing in the parse path
// calls these on the borrowed happy path — a caller that needs to outlive the
// input pays for the copy exactly once, here, rather than every document
// paying for it by default.

fn cow_owned(c: Cow<'_, str>) -> Cow<'static, str> {
    Cow::Owned(c.into_owned())
}

fn opt_cow_owned(c: Option<Cow<'_, str>>) -> Option<Cow<'static, str>> {
    c.map(cow_owned)
}

fn inlines_owned(v: Vec<Inline<'_>>) -> Vec<Inline<'static>> {
    v.into_iter().map(Inline::into_owned).collect()
}

fn blocks_owned(v: Vec<Block<'_>>) -> Vec<Block<'static>> {
    v.into_iter().map(Block::into_owned).collect()
}

impl RstDoc<'_> {
    /// Convert into a document that borrows nothing from the parsed input.
    pub fn into_owned(self) -> RstDoc<'static> {
        RstDoc {
            blocks: blocks_owned(self.blocks),
        }
    }
}

impl DefinitionItem<'_> {
    /// Convert into an item that borrows nothing from the parsed input.
    pub fn into_owned(self) -> DefinitionItem<'static> {
        DefinitionItem {
            term: inlines_owned(self.term),
            desc: inlines_owned(self.desc),
        }
    }
}

impl TableRow<'_> {
    /// Convert into a row that borrows nothing from the parsed input.
    pub fn into_owned(self) -> TableRow<'static> {
        TableRow {
            cells: self.cells.into_iter().map(inlines_owned).collect(),
            is_header: self.is_header,
        }
    }
}

impl Block<'_> {
    /// Convert into a block that borrows nothing from the parsed input.
    pub fn into_owned(self) -> Block<'static> {
        match self {
            Block::Paragraph { inlines } => Block::Paragraph {
                inlines: inlines_owned(inlines),
            },
            Block::Heading { level, inlines } => Block::Heading {
                level,
                inlines: inlines_owned(inlines),
            },
            Block::CodeBlock { language, content } => Block::CodeBlock {
                language: opt_cow_owned(language),
                content: cow_owned(content),
            },
            Block::Blockquote { children } => Block::Blockquote {
                children: blocks_owned(children),
            },
            Block::List { ordered, items } => Block::List {
                ordered,
                items: items.into_iter().map(blocks_owned).collect(),
            },
            Block::DefinitionList { items } => Block::DefinitionList {
                items: items.into_iter().map(DefinitionItem::into_owned).collect(),
            },
            Block::Figure { url, alt, caption } => Block::Figure {
                url: cow_owned(url),
                alt: opt_cow_owned(alt),
                caption: caption.map(inlines_owned),
            },
            Block::Image { url, alt, title } => Block::Image {
                url: cow_owned(url),
                alt: opt_cow_owned(alt),
                title: opt_cow_owned(title),
            },
            Block::RawBlock { format, content } => Block::RawBlock {
                format: cow_owned(format),
                content: cow_owned(content),
            },
            Block::Div {
                class,
                directive,
                children,
            } => Block::Div {
                class: opt_cow_owned(class),
                directive: opt_cow_owned(directive),
                children: blocks_owned(children),
            },
            Block::HorizontalRule => Block::HorizontalRule,
            Block::Table { rows } => Block::Table {
                rows: rows.into_iter().map(TableRow::into_owned).collect(),
            },
            Block::FootnoteDef { label, inlines } => Block::FootnoteDef {
                label: cow_owned(label),
                inlines: inlines_owned(inlines),
            },
            Block::MathDisplay { source } => Block::MathDisplay {
                source: cow_owned(source),
            },
            Block::Admonition {
                admonition_type,
                children,
            } => Block::Admonition {
                admonition_type: cow_owned(admonition_type),
                children: blocks_owned(children),
            },
        }
    }
}

impl Inline<'_> {
    /// Convert into an inline that borrows nothing from the parsed input.
    pub fn into_owned(self) -> Inline<'static> {
        match self {
            Inline::Text(s) => Inline::Text(cow_owned(s)),
            Inline::Emphasis(c) => Inline::Emphasis(inlines_owned(c)),
            Inline::Strong(c) => Inline::Strong(inlines_owned(c)),
            Inline::Strikeout(c) => Inline::Strikeout(inlines_owned(c)),
            Inline::Underline(c) => Inline::Underline(inlines_owned(c)),
            Inline::Subscript(c) => Inline::Subscript(inlines_owned(c)),
            Inline::Superscript(c) => Inline::Superscript(inlines_owned(c)),
            Inline::SmallCaps(c) => Inline::SmallCaps(inlines_owned(c)),
            Inline::Code(s) => Inline::Code(cow_owned(s)),
            Inline::Link { url, children } => Inline::Link {
                url: cow_owned(url),
                children: inlines_owned(children),
            },
            Inline::Image { url, alt } => Inline::Image {
                url: cow_owned(url),
                alt: cow_owned(alt),
            },
            Inline::LineBreak => Inline::LineBreak,
            Inline::SoftBreak => Inline::SoftBreak,
            Inline::FootnoteRef { label } => Inline::FootnoteRef {
                label: cow_owned(label),
            },
            Inline::FootnoteDef { label, children } => Inline::FootnoteDef {
                label: cow_owned(label),
                children: inlines_owned(children),
            },
            Inline::Quoted {
                quote_type,
                children,
            } => Inline::Quoted {
                quote_type: cow_owned(quote_type),
                children: inlines_owned(children),
            },
            Inline::MathInline { source } => Inline::MathInline {
                source: cow_owned(source),
            },
            Inline::RstSpan { role, children } => Inline::RstSpan {
                role: cow_owned(role),
                children: inlines_owned(children),
            },
        }
    }
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// RST heading character priority (lower = higher level).
/// The actual level is determined by order of appearance in the document.
const HEADING_CHARS: &[char] = &['=', '-', '~', '^', '"', '`', '#', '*', '+', '_'];

/// Link-target table: normalised (lowercased) name -> URL slice of the input.
///
/// Keys are owned because RST reference names are matched case-insensitively,
/// so a normalised key is not in general a slice of the input. Values always
/// are, which is what lets a resolved link borrow its URL.
type LinkTargets<'a> = std::collections::HashMap<String, &'a str>;

/// Parse an RST string into an [`RstDoc`] borrowing from `input`.
pub fn parse(input: &str) -> Result<RstDoc<'_>, RstError> {
    let mut p = Parser::new(input);
    let blocks = p.parse_document();
    Ok(RstDoc { blocks })
}

/// Join `parts` with `sep`, borrowing when the result is a single part.
fn join_cow<'a>(parts: &[&'a str], sep: &str) -> Cow<'a, str> {
    match parts {
        [] => Cow::Borrowed(""),
        [one] => Cow::Borrowed(one),
        _ => Cow::Owned(parts.join(sep)),
    }
}

/// Join the non-empty `parts` with a single space — RST's soft-line-break
/// semantics inside a paragraph, list item, block quote or definition body.
/// Borrows when only one part survives (the single-source-line case).
fn join_words<'a>(parts: &[&'a str]) -> Cow<'a, str> {
    let mut nonempty = parts.iter().copied().filter(|s| !s.is_empty());
    let Some(first) = nonempty.next() else {
        return Cow::Borrowed("");
    };
    let Some(second) = nonempty.next() else {
        return Cow::Borrowed(first);
    };
    let mut out = String::with_capacity(first.len() + second.len() + 1);
    out.push_str(first);
    out.push(' ');
    out.push_str(second);
    for p in nonempty {
        out.push(' ');
        out.push_str(p);
    }
    Cow::Owned(out)
}

struct Parser<'a> {
    lines: Vec<&'a str>,
    line_idx: usize,
    /// Maps underline character to heading level (assigned in order of appearance).
    heading_levels: Vec<char>,
    /// Link targets: normalised name -> url
    link_targets: LinkTargets<'a>,
    /// Substitution definitions: |name| -> replacement text
    substitutions: std::collections::HashMap<&'a str, &'a str>,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        let lines: Vec<&str> = input.lines().collect();
        Self {
            lines,
            line_idx: 0,
            heading_levels: Vec::new(),
            link_targets: std::collections::HashMap::new(),
            substitutions: std::collections::HashMap::new(),
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
                    let url = rest[colon_idx + 1..].trim();
                    self.link_targets.insert(name, url);
                }
            }
            idx += 1;
        }
    }

    /// First pass: collect anonymous link targets (__ url)
    fn collect_anonymous_targets(&mut self) {
        let mut idx = 0;
        let mut anon_counter = 0usize;
        while idx < self.lines.len() {
            let line = self.lines[idx].trim();
            if let Some(url) = line.strip_prefix("__ ") {
                let key = format!("__anon{}", anon_counter);
                self.link_targets.insert(key, url.trim());
                anon_counter += 1;
            }
            idx += 1;
        }
    }

    /// First pass: collect substitution definitions (.. |name| replace:: value)
    fn collect_substitutions(&mut self) {
        let mut idx = 0;
        while idx < self.lines.len() {
            let line = self.lines[idx];
            if let Some(rest) = line.strip_prefix(".. |") {
                if let Some(bar_end) = rest.find('|') {
                    let name = &rest[..bar_end];
                    let after_bar = rest[bar_end + 1..].trim();
                    // Only handle replace:: for now
                    if let Some(value) = after_bar.strip_prefix("replace::") {
                        self.substitutions.insert(name, value.trim());
                    }
                }
            }
            idx += 1;
        }
    }

    fn parse_document(&mut self) -> Vec<Block<'a>> {
        // First pass: collect link targets, anonymous targets, substitutions
        self.collect_link_targets();
        self.collect_anonymous_targets();
        self.collect_substitutions();

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

    fn try_parse_block(&mut self) -> Option<Block<'a>> {
        // Skip link target definitions (already collected)
        if let Some(line) = self.current_line() {
            if line.starts_with(".. _") && line.contains(':') {
                self.advance_line();
                return self.try_parse_block();
            }
            // Skip anonymous link targets (__ url)
            if line.starts_with("__ ") {
                self.advance_line();
                return self.try_parse_block();
            }
        }

        // Skip substitution definitions (.. |name| directive::)
        if let Some(line) = self.current_line() {
            if line.starts_with(".. |") {
                self.advance_line();
                // Skip indented continuation
                while !self.is_eof() {
                    let cont = self.current_line().unwrap_or("");
                    if cont.is_empty() || cont.starts_with(' ') || cont.starts_with('\t') {
                        self.advance_line();
                    } else {
                        break;
                    }
                }
                return self.try_parse_block();
            }
        }

        // Check for transition (RST horizontal rule): 4+ repeated punctuation chars on a line,
        // followed by a blank line (transitions require blank lines on both sides).
        if let Some(line) = self.current_line() {
            let trimmed = line.trim();
            if trimmed.len() >= 4 {
                let first = trimmed.chars().next().unwrap_or(' ');
                if "-=~^\"#*+_".contains(first) && trimmed.chars().all(|c| c == first) {
                    // Only treat as transition if the next line is blank or EOF (not a heading)
                    let next_is_blank = self
                        .peek_line()
                        .map(|l| l.trim().is_empty())
                        .unwrap_or(true);
                    if next_is_blank {
                        self.advance_line();
                        return Some(Block::HorizontalRule);
                    }
                }
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

        // Check for line block (| line text)
        if let Some(lb) = self.try_parse_line_block() {
            return Some(lb);
        }

        // Check for field list (:Name: value)
        if let Some(fieldlist) = self.try_parse_field_list() {
            return Some(fieldlist);
        }

        // Check for definition list
        if let Some(deflist) = self.try_parse_definition_list() {
            return Some(deflist);
        }

        // Check for grid table (+---+---+)
        if let Some(table) = self.try_parse_grid_table() {
            return Some(table);
        }

        // Check for simple table (===  ===)
        if let Some(table) = self.try_parse_simple_table() {
            return Some(table);
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

    fn try_parse_heading(&mut self) -> Option<Block<'a>> {
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

    fn try_parse_directive(&mut self) -> Option<Block<'a>> {
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
                    let mut body_lines: Vec<&'a str> = vec![after_bracket.trim()];
                    self.advance_line();

                    // Collect continuation lines indented by ≥ 3 spaces
                    while !self.is_eof() {
                        let cont = self.current_line().unwrap_or("");
                        if cont.trim().is_empty() {
                            // Blank line ends the footnote body
                            break;
                        }
                        // Continuation line must be indented by at least 3 spaces
                        let indent = cont.chars().take_while(|c| *c == ' ' || *c == '\t').count();
                        if indent < 3 {
                            break;
                        }
                        body_lines.push(cont.trim());
                        self.advance_line();
                    }

                    let body = join_words(&body_lines);
                    let inlines = parse_inline_cow(body, &self.link_targets);
                    return Some(Block::FootnoteDef {
                        label: Cow::Borrowed(label),
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
        let mut content_lines: Vec<&'a str> = Vec::new();
        let mut options: std::collections::HashMap<&'a str, &'a str> =
            std::collections::HashMap::new();

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
                    options.insert(opt_name, opt_value);
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
                    Some(Cow::Borrowed(argument))
                };
                let content = join_cow(&content_lines, "\n");
                Block::CodeBlock { language, content }
            }
            "note" | "warning" | "tip" | "important" | "caution" | "danger" | "error" | "hint"
            | "attention" => {
                let content = join_cow(&content_lines, "\n");
                let inlines = parse_inline_cow(content, &self.link_targets);
                Block::Div {
                    class: Some(Cow::Borrowed(directive_name)),
                    directive: None,
                    children: vec![Block::Paragraph { inlines }],
                }
            }
            "image" => Block::Image {
                url: Cow::Borrowed(argument),
                alt: options.get("alt").copied().map(Cow::Borrowed),
                title: options.get("title").copied().map(Cow::Borrowed),
            },
            "figure" => {
                let caption = if content_lines.is_empty() {
                    None
                } else {
                    let caption_text = join_cow(&content_lines, " ");
                    Some(parse_inline_cow(caption_text, &self.link_targets))
                };
                Block::Figure {
                    url: Cow::Borrowed(argument),
                    alt: options.get("alt").copied().map(Cow::Borrowed),
                    caption,
                }
            }
            "raw" => Block::RawBlock {
                format: Cow::Borrowed(argument),
                content: join_cow(&content_lines, "\n"),
            },
            "contents" | "toc" => Block::Div {
                class: Some(Cow::Borrowed("toc")),
                directive: None,
                children: vec![],
            },
            "math" => Block::MathDisplay {
                source: join_cow(&content_lines, "\n"),
            },
            "admonition" => {
                // Custom admonition with a title argument
                let content = join_cow(&content_lines, "\n");
                let children = if content.is_empty() {
                    vec![]
                } else {
                    let inlines = parse_inline_cow(content, &self.link_targets);
                    vec![Block::Paragraph { inlines }]
                };
                Block::Admonition {
                    admonition_type: Cow::Borrowed(argument),
                    children,
                }
            }
            "container" => {
                let content = join_cow(&content_lines, "\n");
                let children = if content.is_empty() {
                    vec![]
                } else {
                    let inlines = parse_inline_cow(content, &self.link_targets);
                    vec![Block::Paragraph { inlines }]
                };
                Block::Div {
                    class: if argument.is_empty() {
                        None
                    } else {
                        Some(Cow::Borrowed(argument))
                    },
                    directive: Some(Cow::Borrowed("container")),
                    children,
                }
            }
            "rubric" => {
                // rubric directive: argument is the heading text
                let mut children = Vec::new();
                if !argument.is_empty() {
                    let inlines = parse_inline_content(argument, &self.link_targets);
                    children.push(Block::Paragraph { inlines });
                } else {
                    let content = join_cow(&content_lines, "\n");
                    if !content.is_empty() {
                        let inlines = parse_inline_cow(content, &self.link_targets);
                        children.push(Block::Paragraph { inlines });
                    }
                }
                Block::Div {
                    class: None,
                    directive: Some(Cow::Borrowed("rubric")),
                    children,
                }
            }
            _ => {
                // Unknown directive — create generic div (warnings handled by adapter)
                let content = join_cow(&content_lines, "\n");
                let children = if content.is_empty() {
                    vec![]
                } else {
                    let inlines = parse_inline_cow(content, &self.link_targets);
                    vec![Block::Paragraph { inlines }]
                };
                Block::Div {
                    class: None,
                    directive: Some(Cow::Borrowed(directive_name)),
                    children,
                }
            }
        };

        Some(block)
    }

    fn try_parse_list(&mut self) -> Option<Block<'a>> {
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

    fn try_parse_grid_table(&mut self) -> Option<Block<'a>> {
        let line = self.current_line()?;
        if !line.starts_with('+') {
            return None;
        }
        // Must be a border line: +---+---+
        if !line.chars().all(|c| c == '+' || c == '-' || c == '=') {
            return None;
        }
        // Collect the table
        let mut rows: Vec<TableRow<'a>> = Vec::new();

        // First border line — skip
        self.advance_line();

        while !self.is_eof() {
            let row_line = self.current_line().unwrap_or("");

            if row_line.is_empty() {
                break;
            }

            // Border line
            if row_line.starts_with('+')
                && row_line.chars().all(|c| c == '+' || c == '-' || c == '=')
            {
                let has_equals = row_line.contains('=');
                self.advance_line();
                if has_equals && !rows.is_empty() {
                    // Mark last row as header
                    if let Some(last) = rows.last_mut() {
                        last.is_header = true;
                    }
                }
                if row_line.starts_with('+') && !row_line.contains('|') && rows.is_empty() {
                    // Last border line (only borders left)
                    break;
                }
                continue;
            }

            // Data row: | cell | cell |
            if row_line.starts_with('|') {
                let cells: Vec<Vec<Inline<'a>>> = row_line
                    .split('|')
                    .skip(1) // Skip leading |
                    .filter(|s| !s.is_empty() || row_line.ends_with('|'))
                    .filter(|s| !s.is_empty())
                    .map(|cell| {
                        let text = cell.trim();
                        parse_inline_content(text, &self.link_targets)
                    })
                    .collect();
                if !cells.is_empty() {
                    rows.push(TableRow {
                        cells,
                        is_header: false,
                    });
                }
                self.advance_line();
                continue;
            }

            break;
        }

        if rows.is_empty() {
            return None;
        }

        Some(Block::Table { rows })
    }

    fn try_parse_simple_table(&mut self) -> Option<Block<'a>> {
        let line = self.current_line()?;
        let trimmed = line.trim();
        // Simple table border: ===  === (multiple === groups separated by spaces)
        if !trimmed.starts_with('=') {
            return None;
        }
        if !trimmed.chars().all(|c| c == '=' || c == ' ') {
            return None;
        }
        // Parse column widths
        let col_spans: Vec<(usize, usize)> = {
            let mut spans = Vec::new();
            let mut start = None;
            for (i, c) in trimmed.char_indices() {
                match c {
                    '=' => {
                        if start.is_none() {
                            start = Some(i);
                        }
                    }
                    ' ' => {
                        if let Some(s) = start {
                            spans.push((s, i));
                            start = None;
                        }
                    }
                    _ => {}
                }
            }
            if let Some(s) = start {
                spans.push((s, trimmed.len()));
            }
            spans
        };

        if col_spans.is_empty() {
            return None;
        }

        self.advance_line(); // Skip top border

        let mut rows = Vec::new();
        let mut is_header_section = true;

        while !self.is_eof() {
            let row_line = self.current_line().unwrap_or("");

            if row_line.trim().is_empty() {
                self.advance_line();
                continue;
            }

            // Border line
            let trimmed_row = row_line.trim();
            if trimmed_row.starts_with('=') && trimmed_row.chars().all(|c| c == '=' || c == ' ') {
                self.advance_line();
                is_header_section = false;
                // Check if this is the final border
                if self.is_eof()
                    || self
                        .current_line()
                        .map(|l| l.trim().is_empty())
                        .unwrap_or(true)
                {
                    break;
                }
                // Check if next line is also a border (end of table)
                if let Some(next) = self.current_line() {
                    let nt = next.trim();
                    if nt.starts_with('=') && nt.chars().all(|c| c == '=' || c == ' ') {
                        self.advance_line();
                        break;
                    }
                }
                continue;
            }

            // Data row
            let cells: Vec<Vec<Inline<'a>>> = col_spans
                .iter()
                .map(|(start, end)| {
                    let cell_text = if *start < row_line.len() {
                        let end_pos = (*end).min(row_line.len());
                        row_line[*start..end_pos].trim()
                    } else {
                        ""
                    };
                    parse_inline_content(cell_text, &self.link_targets)
                })
                .collect();

            rows.push(TableRow {
                cells,
                is_header: is_header_section,
            });
            self.advance_line();
        }

        if rows.is_empty() {
            return None;
        }

        Some(Block::Table { rows })
    }

    fn parse_bullet_list(&mut self, bullet: char) -> Block<'a> {
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
                let item_inlines = self.parse_list_item(rest);
                let mut item_blocks = vec![Block::Paragraph {
                    inlines: item_inlines,
                }];
                // Check for immediately indented sub-list (no blank line between)
                // Only handle the case where the next non-blank line is a bullet at higher indent
                while !self.is_eof() {
                    let peek = self.current_line().unwrap_or("");
                    // Skip one blank line
                    if peek.trim().is_empty() {
                        let next_idx = self.line_idx + 1;
                        let next_line = self.lines.get(next_idx).copied().unwrap_or("");
                        let next_indent = self.get_line_indent(next_line);
                        let next_trimmed = next_line.trim_start();
                        // Only continue if the next non-blank line is a sub-list bullet at higher indent
                        if next_indent > indent
                            && (next_trimmed.starts_with("- ")
                                || next_trimmed.starts_with("* ")
                                || next_trimmed.starts_with("+ "))
                        {
                            self.advance_line(); // skip blank
                            continue;
                        }
                        break;
                    }
                    let sub_indent = self.get_indent();
                    let sub_trimmed = peek.trim_start();
                    // Only parse sub-lists (bullet lists) at higher indentation
                    if sub_indent > indent
                        && (sub_trimmed.starts_with("- ")
                            || sub_trimmed.starts_with("* ")
                            || sub_trimmed.starts_with("+ "))
                    {
                        if let Some(sub_block) = self.try_parse_list() {
                            item_blocks.push(sub_block);
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                items.push(item_blocks);
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

    fn parse_numbered_list(&mut self) -> Block<'a> {
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

    fn parse_list_item(&mut self, first_line: &'a str) -> Vec<Inline<'a>> {
        self.advance_line();

        let mut content_lines: Vec<&'a str> = vec![first_line];

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
                content_lines.push(trimmed);
                self.advance_line();
            } else {
                break;
            }
        }

        parse_inline_cow(join_words(&content_lines), &self.link_targets)
    }

    fn try_parse_definition_list(&mut self) -> Option<Block<'a>> {
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

    fn try_parse_line_block(&mut self) -> Option<Block<'a>> {
        let line = self.current_line()?;
        if !line.starts_with("| ") && line != "|" {
            return None;
        }
        let mut children = Vec::new();
        while !self.is_eof() {
            let l = self.current_line().unwrap_or("");
            if l.starts_with("| ") || l == "|" {
                let text = l.strip_prefix("| ").unwrap_or("");
                let inlines = parse_inline_content(text, &self.link_targets);
                children.push(Block::Paragraph { inlines });
                self.advance_line();
            } else if l.trim().is_empty() {
                self.advance_line();
                break;
            } else {
                break;
            }
        }
        Some(Block::Div {
            class: Some(Cow::Borrowed("line-block")),
            directive: None,
            children,
        })
    }

    fn try_parse_field_list(&mut self) -> Option<Block<'a>> {
        let line = self.current_line()?;
        // Field list starts with :name: value  (space after closing colon)
        if !line.starts_with(':') {
            return None;
        }
        // Skip standalone :: (literal block marker)
        if line.trim() == "::" {
            return None;
        }
        let rest = &line[1..];
        // Find the closing colon
        let close_colon = rest.find(':')?;
        let after_close = &rest[close_colon + 1..];
        // Must be followed by a space (field list) not a backtick (inline role)
        if !after_close.starts_with(' ') && !after_close.is_empty() {
            return None;
        }
        Some(self.parse_field_list())
    }

    fn parse_field_list(&mut self) -> Block<'a> {
        let mut items = Vec::new();

        while !self.is_eof() {
            let line = self.current_line().unwrap_or("");

            if line.trim().is_empty() {
                self.advance_line();
                continue;
            }

            // Field list item: :name: value
            if let Some(after_colon) = line.strip_prefix(':') {
                if let Some(close_colon) = after_colon.find(':') {
                    let field_name = &after_colon[..close_colon];
                    let field_value = after_colon[close_colon + 1..].trim();
                    let term = parse_inline_content(field_name, &self.link_targets);
                    let desc = parse_inline_content(field_value, &self.link_targets);
                    self.advance_line();
                    items.push(DefinitionItem { term, desc });
                    continue;
                }
            }

            // Non-field-list line — stop
            break;
        }

        Block::DefinitionList { items }
    }

    fn parse_definition_list(&mut self) -> Block<'a> {
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
                        let mut def_lines: Vec<&'a str> = Vec::new();
                        while !self.is_eof() {
                            let def_line = self.current_line().unwrap_or("");
                            if def_line.trim().is_empty() {
                                break;
                            }
                            if def_line.starts_with(' ') || def_line.starts_with('\t') {
                                def_lines.push(def_line.trim());
                                self.advance_line();
                            } else {
                                break;
                            }
                        }

                        let desc = parse_inline_cow(join_words(&def_lines), &self.link_targets);
                        items.push(DefinitionItem { term, desc });
                        continue;
                    }
                }
            }

            break;
        }

        Block::DefinitionList { items }
    }

    fn try_parse_literal_block(&mut self) -> Option<Block<'a>> {
        let line = self.current_line()?;

        // Check for :: at end of line (paragraph ending with ::)
        if line.trim_end().ends_with("::") {
            self.advance_line();
            self.skip_blank_lines();

            // Collect indented content
            let mut content_lines: Vec<&'a str> = Vec::new();
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

            let content = join_cow(&content_lines, "\n");
            return Some(Block::CodeBlock {
                language: None,
                content,
            });
        }

        // Check for standalone :: on its own line
        if line.trim() == "::" {
            self.advance_line();
            self.skip_blank_lines();

            let mut content_lines: Vec<&'a str> = Vec::new();
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

            let content = join_cow(&content_lines, "\n");
            return Some(Block::CodeBlock {
                language: None,
                content,
            });
        }

        None
    }

    fn try_parse_blockquote(&mut self) -> Option<Block<'a>> {
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
            let mut content_lines: Vec<&'a str> = Vec::new();
            while !self.is_eof() {
                let bq_line = self.current_line().unwrap_or("");
                if bq_line.trim().is_empty() {
                    break;
                }
                if bq_line.starts_with(' ') || bq_line.starts_with('\t') {
                    content_lines.push(bq_line.trim());
                    self.advance_line();
                } else {
                    break;
                }
            }

            let inlines = parse_inline_cow(join_words(&content_lines), &self.link_targets);
            return Some(Block::Blockquote {
                children: vec![Block::Paragraph { inlines }],
            });
        }

        None
    }

    fn parse_paragraph(&mut self) -> Option<Block<'a>> {
        // Source lines are collected as borrowed slices and only joined if
        // there is more than one — a single-line paragraph (the common case in
        // hand-written RST and the universal case in generated RST) reaches
        // the tokenizer as a slice of the input and stays borrowed all the way
        // into `Inline::Text`.
        let mut content_lines: Vec<&'a str> = Vec::new();

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

            content_lines.push(line.trim());
            self.advance_line();
        }

        let content = join_words(&content_lines);
        if content.is_empty() {
            return None;
        }

        // Check for trailing :: (literal block indicator)
        let content = if content.ends_with("::") && content.len() > 2 {
            match content {
                Cow::Borrowed(s) => Cow::Borrowed(s[..s.len() - 1].trim_end()),
                Cow::Owned(s) => Cow::Owned(s[..s.len() - 1].trim_end().to_string()),
            }
        } else {
            content
        };

        let expanded = self.expand_substitutions_cow(content);
        let inlines = parse_inline_cow(expanded, &self.link_targets);
        Some(Block::Paragraph { inlines })
    }

    /// [`expand_substitutions`](Self::expand_substitutions) over a `Cow`,
    /// preserving borrowedness when the text was borrowed and no substitution
    /// reference fired.
    fn expand_substitutions_cow(&self, text: Cow<'a, str>) -> Cow<'a, str> {
        match text {
            Cow::Borrowed(s) => self.expand_substitutions(s),
            Cow::Owned(s) => Cow::Owned(self.expand_substitutions(&s).into_owned()),
        }
    }

    /// Expand substitution references |name| in a string.
    fn expand_substitutions<'b>(&self, text: &'b str) -> std::borrow::Cow<'b, str> {
        if !text.contains('|') {
            return std::borrow::Cow::Borrowed(text);
        }
        let mut result = String::new();
        let mut last_end = 0;
        let bytes = text.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'|' {
                // Find closing |
                if let Some(close) = text[i + 1..].find('|') {
                    let name = &text[i + 1..i + 1 + close];
                    if !name.is_empty() && !name.contains(' ') {
                        if let Some(replacement) = self.substitutions.get(name) {
                            result.push_str(&text[last_end..i]);
                            result.push_str(replacement);
                            i = i + 1 + close + 1;
                            last_end = i;
                            continue;
                        }
                    }
                }
            }
            i += 1;
        }
        if result.is_empty() {
            return std::borrow::Cow::Borrowed(text);
        }
        result.push_str(&text[last_end..]);
        std::borrow::Cow::Owned(result)
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

// ── Streaming event iterator ───────────────────────────────────────────────────
//
// `EventIter` is the parser: it holds a `Parser<'a>` (the same recursive-descent
// engine `parse()` uses — there is exactly one implementation of RST grammar,
// not two parallel copies) plus a lazy `frame_stack` that expands one
// already-parsed top-level `Block` into events at a time. `next()` never
// materializes a `Vec<Block>` for the whole document: only the current
// top-level block's subtree (bounded by that construct's own size) plus
// `frame_stack` (O(nesting depth)) are live at once. This mirrors the same
// memory tradeoff `StreamingParser` (batch.rs) makes for chunked input.
//
// `parse()` is NOT implemented as `events().collect()` — it drives `Parser`
// directly with no event-dispatch overhead (see `parse()` above). `events()`
// is a wholly separate, genuinely lazy pull iterator over the same grammar.

/// Lazy-traversal frame for the event iterator.
/// Frames are pushed in reverse-emission order, so `pop()` yields the next event.
enum Frame<'a> {
    Event(Event<'a>),
    Blocks(std::vec::IntoIter<Block<'a>>),
    Inlines(std::vec::IntoIter<Inline<'a>>),
    /// List items are `Vec<Block>`.
    ListItems(std::vec::IntoIter<Vec<Block<'a>>>),
    TableRows(std::vec::IntoIter<TableRow<'a>>),
    /// Table cells are `Vec<Inline>`.
    TableCells(std::vec::IntoIter<Vec<Inline<'a>>>),
    DefinitionItems(std::vec::IntoIter<DefinitionItem<'a>>),
}

/// Streaming pull iterator over an RST document.
///
/// `next()` advances an internal [`Parser`] one top-level block at a time and
/// lazily expands that block's subtree into a sequence of [`Event`]s. This is
/// the same grammar `parse()` uses — not a re-implementation — so `events()`
/// and `parse()` can never disagree on what constitutes a block.
///
/// Text fields on [`Event`] are `Cow<'a, str>` slices of `input` wherever the
/// span is contiguous in the source and needs no transformation — the shared
/// inline tokenizer (`parse_inline_content`) is byte-indexed over the input
/// precisely so both `parse()` and `events()` can borrow. `Cow::Owned` appears
/// only where the format forces it: text runs containing backslash escapes
/// (which must be resolved), content joined from non-contiguous source lines
/// (multi-line paragraphs, list items, block quotes, definition bodies), and
/// synthesised `:ref:`/`:doc:` URLs.
pub struct EventIter<'a> {
    parser: Parser<'a>,
    frame_stack: Vec<Frame<'a>>,
    iter_done: bool,
}

impl<'a> EventIter<'a> {
    /// Construct an `EventIter` ready to be used as an `Iterator`.
    ///
    /// Runs the same link-target/substitution/anonymous-target pre-scan
    /// `parse()` runs, so link resolution matches exactly.
    pub fn new(input: &'a str) -> Self {
        let mut parser = Parser::new(input);
        parser.collect_link_targets();
        parser.collect_anonymous_targets();
        parser.collect_substitutions();
        Self {
            parser,
            frame_stack: Vec::new(),
            iter_done: false,
        }
    }

    /// Like [`EventIter::new`], but seeds the underline-character-to-level
    /// assignment (normally built up fresh, in order of first appearance,
    /// over the whole input) from `heading_levels` instead of starting empty.
    ///
    /// For a single call over a whole document, [`EventIter::new`] is always
    /// correct and this constructor is unnecessary. It exists for callers
    /// that parse one document in successive, independently-scoped chunks
    /// (e.g. a chunk-driven streaming parser that re-parses each accumulated
    /// block via a fresh `EventIter`) and need heading levels to stay
    /// consistent across those chunk boundaries — without it, every chunk's
    /// first heading would be renumbered level 1 regardless of its true
    /// position in the document. Call [`EventIter::heading_levels`] after
    /// exhausting one chunk's iterator to get the state to carry into the
    /// next.
    pub fn with_heading_levels(input: &'a str, heading_levels: Vec<char>) -> Self {
        let mut parser = Parser::new(input);
        parser.heading_levels = heading_levels;
        parser.collect_link_targets();
        parser.collect_anonymous_targets();
        parser.collect_substitutions();
        Self {
            parser,
            frame_stack: Vec::new(),
            iter_done: false,
        }
    }

    /// The underline-character-to-level assignment accumulated so far.
    /// Meaningful once the iterator has been exhausted (see
    /// [`EventIter::with_heading_levels`]); mid-iteration it reflects only
    /// the headings parsed up to whatever point `next()` has reached.
    pub fn heading_levels(&self) -> &[char] {
        &self.parser.heading_levels
    }

    fn expand_block(&mut self, block: Block<'a>) {
        match block {
            Block::Paragraph { inlines } => {
                self.frame_stack.push(Frame::Event(Event::EndParagraph));
                if !inlines.is_empty() {
                    self.frame_stack.push(Frame::Inlines(inlines.into_iter()));
                }
                self.frame_stack.push(Frame::Event(Event::StartParagraph));
            }
            Block::Heading { level, inlines } => {
                self.frame_stack.push(Frame::Event(Event::EndHeading));
                if !inlines.is_empty() {
                    self.frame_stack.push(Frame::Inlines(inlines.into_iter()));
                }
                self.frame_stack
                    .push(Frame::Event(Event::StartHeading { level }));
            }
            Block::CodeBlock { language, content } => {
                self.frame_stack.push(Frame::Event(Event::EndCodeBlock));
                self.frame_stack
                    .push(Frame::Event(Event::CodeBlockContent(content)));
                self.frame_stack
                    .push(Frame::Event(Event::StartCodeBlock { language }));
            }
            Block::Blockquote { children } => {
                self.frame_stack.push(Frame::Event(Event::EndBlockquote));
                if !children.is_empty() {
                    self.frame_stack.push(Frame::Blocks(children.into_iter()));
                }
                self.frame_stack.push(Frame::Event(Event::StartBlockquote));
            }
            Block::List { ordered, items } => {
                self.frame_stack.push(Frame::Event(Event::EndList));
                if !items.is_empty() {
                    self.frame_stack.push(Frame::ListItems(items.into_iter()));
                }
                self.frame_stack
                    .push(Frame::Event(Event::StartList { ordered }));
            }
            Block::DefinitionList { items } => {
                self.frame_stack
                    .push(Frame::Event(Event::EndDefinitionList));
                if !items.is_empty() {
                    self.frame_stack
                        .push(Frame::DefinitionItems(items.into_iter()));
                }
                self.frame_stack
                    .push(Frame::Event(Event::StartDefinitionList));
            }
            Block::Figure { url, alt, caption } => {
                self.frame_stack.push(Frame::Event(Event::EndFigure));
                if let Some(cap_inlines) = caption {
                    if !cap_inlines.is_empty() {
                        self.frame_stack
                            .push(Frame::Inlines(cap_inlines.into_iter()));
                    }
                }
                self.frame_stack
                    .push(Frame::Event(Event::StartFigure { url, alt }));
            }
            Block::Image { url, alt, title } => {
                self.frame_stack
                    .push(Frame::Event(Event::ImageBlock { url, alt, title }));
            }
            Block::RawBlock { format, content } => {
                self.frame_stack
                    .push(Frame::Event(Event::RawBlock { format, content }));
            }
            Block::Div {
                class,
                directive,
                children,
            } => {
                self.frame_stack.push(Frame::Event(Event::EndDiv));
                if !children.is_empty() {
                    self.frame_stack.push(Frame::Blocks(children.into_iter()));
                }
                self.frame_stack
                    .push(Frame::Event(Event::StartDiv { class, directive }));
            }
            Block::HorizontalRule => {
                self.frame_stack.push(Frame::Event(Event::HorizontalRule));
            }
            Block::Table { rows } => {
                self.frame_stack.push(Frame::Event(Event::EndTable));
                if !rows.is_empty() {
                    self.frame_stack.push(Frame::TableRows(rows.into_iter()));
                }
                self.frame_stack.push(Frame::Event(Event::StartTable));
            }
            Block::FootnoteDef { label, inlines } => {
                self.frame_stack.push(Frame::Event(Event::EndFootnoteDef));
                if !inlines.is_empty() {
                    self.frame_stack.push(Frame::Inlines(inlines.into_iter()));
                }
                self.frame_stack
                    .push(Frame::Event(Event::StartFootnoteDef { label }));
            }
            Block::MathDisplay { source } => {
                self.frame_stack
                    .push(Frame::Event(Event::MathDisplay { source }));
            }
            Block::Admonition {
                admonition_type,
                children,
            } => {
                self.frame_stack.push(Frame::Event(Event::EndAdmonition));
                if !children.is_empty() {
                    self.frame_stack.push(Frame::Blocks(children.into_iter()));
                }
                self.frame_stack
                    .push(Frame::Event(Event::StartAdmonition { admonition_type }));
            }
        }
    }

    fn expand_inline(&mut self, inline: Inline<'a>) {
        match inline {
            Inline::Text(s) => {
                self.frame_stack.push(Frame::Event(Event::Text(s)));
            }
            Inline::SoftBreak => {
                self.frame_stack.push(Frame::Event(Event::SoftBreak));
            }
            Inline::LineBreak => {
                self.frame_stack.push(Frame::Event(Event::LineBreak));
            }
            Inline::Emphasis(children) => {
                self.frame_stack.push(Frame::Event(Event::EndEmphasis));
                if !children.is_empty() {
                    self.frame_stack.push(Frame::Inlines(children.into_iter()));
                }
                self.frame_stack.push(Frame::Event(Event::StartEmphasis));
            }
            Inline::Strong(children) => {
                self.frame_stack.push(Frame::Event(Event::EndStrong));
                if !children.is_empty() {
                    self.frame_stack.push(Frame::Inlines(children.into_iter()));
                }
                self.frame_stack.push(Frame::Event(Event::StartStrong));
            }
            Inline::Strikeout(children) => {
                self.frame_stack.push(Frame::Event(Event::EndStrikeout));
                if !children.is_empty() {
                    self.frame_stack.push(Frame::Inlines(children.into_iter()));
                }
                self.frame_stack.push(Frame::Event(Event::StartStrikeout));
            }
            Inline::Underline(children) => {
                self.frame_stack.push(Frame::Event(Event::EndUnderline));
                if !children.is_empty() {
                    self.frame_stack.push(Frame::Inlines(children.into_iter()));
                }
                self.frame_stack.push(Frame::Event(Event::StartUnderline));
            }
            Inline::Subscript(children) => {
                self.frame_stack.push(Frame::Event(Event::EndSubscript));
                if !children.is_empty() {
                    self.frame_stack.push(Frame::Inlines(children.into_iter()));
                }
                self.frame_stack.push(Frame::Event(Event::StartSubscript));
            }
            Inline::Superscript(children) => {
                self.frame_stack.push(Frame::Event(Event::EndSuperscript));
                if !children.is_empty() {
                    self.frame_stack.push(Frame::Inlines(children.into_iter()));
                }
                self.frame_stack.push(Frame::Event(Event::StartSuperscript));
            }
            Inline::SmallCaps(children) => {
                self.frame_stack.push(Frame::Event(Event::EndSmallCaps));
                if !children.is_empty() {
                    self.frame_stack.push(Frame::Inlines(children.into_iter()));
                }
                self.frame_stack.push(Frame::Event(Event::StartSmallCaps));
            }
            Inline::Code(s) => {
                self.frame_stack.push(Frame::Event(Event::Code(s)));
            }
            Inline::Link { url, children } => {
                self.frame_stack.push(Frame::Event(Event::EndLink));
                if !children.is_empty() {
                    self.frame_stack.push(Frame::Inlines(children.into_iter()));
                }
                self.frame_stack
                    .push(Frame::Event(Event::StartLink { url }));
            }
            Inline::Image { url, alt } => {
                self.frame_stack
                    .push(Frame::Event(Event::InlineImage { url, alt }));
            }
            Inline::FootnoteRef { label } => {
                self.frame_stack
                    .push(Frame::Event(Event::FootnoteRef { label }));
            }
            Inline::FootnoteDef { label, children } => {
                self.frame_stack
                    .push(Frame::Event(Event::EndFootnoteDefInline));
                if !children.is_empty() {
                    self.frame_stack.push(Frame::Inlines(children.into_iter()));
                }
                self.frame_stack
                    .push(Frame::Event(Event::StartFootnoteDefInline { label }));
            }
            Inline::Quoted {
                quote_type,
                children,
            } => {
                self.frame_stack.push(Frame::Event(Event::EndQuoted));
                if !children.is_empty() {
                    self.frame_stack.push(Frame::Inlines(children.into_iter()));
                }
                self.frame_stack
                    .push(Frame::Event(Event::StartQuoted { quote_type }));
            }
            Inline::MathInline { source } => {
                self.frame_stack
                    .push(Frame::Event(Event::MathInline { source }));
            }
            Inline::RstSpan { role, children } => {
                self.frame_stack.push(Frame::Event(Event::EndRstSpan));
                if !children.is_empty() {
                    self.frame_stack.push(Frame::Inlines(children.into_iter()));
                }
                self.frame_stack
                    .push(Frame::Event(Event::StartRstSpan { role }));
            }
        }
    }
}

impl<'a> Iterator for EventIter<'a> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Event<'a>> {
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
                        self.frame_stack.push(Frame::Event(Event::EndListItem));
                        if !item_blocks.is_empty() {
                            self.frame_stack
                                .push(Frame::Blocks(item_blocks.into_iter()));
                        }
                        self.frame_stack.push(Frame::Event(Event::StartListItem));
                    }
                    continue;
                }
                Some(Frame::TableRows(mut iter)) => {
                    if let Some(row) = iter.next() {
                        let is_header = row.is_header;
                        self.frame_stack.push(Frame::TableRows(iter));
                        self.frame_stack.push(Frame::Event(Event::EndTableRow));
                        if !row.cells.is_empty() {
                            self.frame_stack
                                .push(Frame::TableCells(row.cells.into_iter()));
                        }
                        self.frame_stack
                            .push(Frame::Event(Event::StartTableRow { is_header }));
                    }
                    continue;
                }
                Some(Frame::TableCells(mut iter)) => {
                    if let Some(cell_inlines) = iter.next() {
                        self.frame_stack.push(Frame::TableCells(iter));
                        self.frame_stack.push(Frame::Event(Event::EndTableCell));
                        if !cell_inlines.is_empty() {
                            self.frame_stack
                                .push(Frame::Inlines(cell_inlines.into_iter()));
                        }
                        self.frame_stack.push(Frame::Event(Event::StartTableCell));
                    }
                    continue;
                }
                Some(Frame::DefinitionItems(mut iter)) => {
                    if let Some(item) = iter.next() {
                        self.frame_stack.push(Frame::DefinitionItems(iter));
                        self.frame_stack
                            .push(Frame::Event(Event::EndDefinitionDesc));
                        if !item.desc.is_empty() {
                            self.frame_stack.push(Frame::Inlines(item.desc.into_iter()));
                        }
                        self.frame_stack
                            .push(Frame::Event(Event::StartDefinitionDesc));
                        self.frame_stack
                            .push(Frame::Event(Event::EndDefinitionTerm));
                        if !item.term.is_empty() {
                            self.frame_stack.push(Frame::Inlines(item.term.into_iter()));
                        }
                        self.frame_stack
                            .push(Frame::Event(Event::StartDefinitionTerm));
                    }
                    continue;
                }
                None => {
                    if self.iter_done {
                        return None;
                    }
                    self.parser.skip_blank_lines();
                    if self.parser.is_eof() {
                        self.iter_done = true;
                        return None;
                    }
                    if let Some(block) = self.parser.try_parse_block() {
                        self.expand_block(block);
                    } else {
                        self.parser.advance_line();
                    }
                    continue;
                }
            }
        }
    }
}

/// Parse `input` as RST and return a streaming [`EventIter`].
pub fn events(input: &str) -> EventIter<'_> {
    EventIter::new(input)
}

// ── Inline parser (free function) ─────────────────────────────────────────────

/// Byte length of the character starting at byte offset `i`.
///
/// `i` is always a character boundary: every advance in the tokenizer moves by
/// a whole character, and every delimiter it indexes past is ASCII.
fn char_len_at(s: &str, i: usize) -> usize {
    s[i..].chars().next().map_or(1, char::len_utf8)
}

/// End (byte offset) of the RST reference-name run starting at `from`.
fn ref_name_end(s: &str, from: usize) -> usize {
    let mut end = from;
    for c in s[from..].chars() {
        if c.is_alphanumeric() || c == '_' || c == '-' {
            end += c.len_utf8();
        } else {
            break;
        }
    }
    end
}

/// Whether lowercasing `s` would leave it unchanged — the check that lets a
/// reference-name lookup skip `to_lowercase`'s allocation for the (dominant)
/// already-lowercase case without ever disagreeing with it.
fn is_already_lowercase(s: &str) -> bool {
    s.chars().all(|c| {
        let mut lower = c.to_lowercase();
        lower.next() == Some(c) && lower.next().is_none()
    })
}

/// Case-insensitive link-target lookup, allocating a normalised key only when
/// the name is not already normalised.
fn lookup_target<'m, 'a>(targets: &'m LinkTargets<'a>, name: &str) -> Option<&'m &'a str> {
    if is_already_lowercase(name) {
        targets.get(name)
    } else {
        targets.get(&name.to_lowercase())
    }
}

/// Tokenize inline content that may or may not be a slice of the original
/// input.
///
/// A `Cow::Borrowed` span (single-source-line paragraph, heading title, table
/// cell, line-block line, …) tokenizes straight into borrowed `Inline`s. A
/// `Cow::Owned` span (a multi-line paragraph, whose source lines are not
/// contiguous in the input, or a substitution-expanded one) is tokenized the
/// same way and then deep-copied — the same cost the all-owned implementation
/// paid unconditionally, now confined to the case that genuinely needs it.
fn parse_inline_cow<'a>(content: Cow<'a, str>, link_targets: &LinkTargets<'a>) -> Vec<Inline<'a>> {
    match content {
        Cow::Borrowed(s) => parse_inline_content(s, link_targets),
        Cow::Owned(s) => inlines_owned(parse_inline_content(&s, link_targets)),
    }
}

/// Byte range of the `Inline::Text` node currently at the end of `nodes`, if
/// any, plus whether that node is still exactly `content[start..end]` (and so
/// can absorb a following adjacent run by widening the borrowed slice rather
/// than by copying).
struct TextRun {
    start: usize,
    end: usize,
    exact_slice: bool,
}

/// Append a resolved text run covering `content[start..end]`, merging it into
/// the immediately preceding text node when there is one — the job the old
/// `merge_text_nodes` post-pass did, done here so that two adjacent borrowed
/// runs merge into one *borrowed* slice instead of an owned concatenation.
fn push_text<'a>(
    nodes: &mut Vec<Inline<'a>>,
    last: &mut Option<TextRun>,
    content: &'a str,
    start: usize,
    end: usize,
    resolved: Cow<'a, str>,
) {
    if let Some(run) = last.as_mut() {
        if run.end == start {
            if resolved.is_empty() {
                // Escaped whitespace: contributes no text but does consume
                // source, so the previous node is no longer an exact slice of
                // `content[run.start..run.end]`.
                run.end = end;
                run.exact_slice = false;
                return;
            }
            if let Some(Inline::Text(prev)) = nodes.last_mut() {
                if run.exact_slice && matches!(resolved, Cow::Borrowed(_)) {
                    *prev = Cow::Borrowed(&content[run.start..end]);
                } else {
                    prev.to_mut().push_str(&resolved);
                    run.exact_slice = false;
                }
                run.end = end;
                return;
            }
        }
    }
    if resolved.is_empty() {
        *last = None;
        return;
    }
    let exact_slice = matches!(resolved, Cow::Borrowed(_));
    nodes.push(Inline::Text(resolved));
    *last = Some(TextRun {
        start,
        end,
        exact_slice,
    });
}

/// The RST inline tokenizer, shared by `parse()` and `events()`.
///
/// Indexes `content` by byte offset so every span it recognises can be handed
/// back as a `Cow::Borrowed` slice of the input. Only two things force an
/// owned payload: a text run containing a backslash escape (which must be
/// resolved, so the emitted text is not a slice of the source) and a
/// `:ref:`/`:doc:` URL (synthesised as `#` + text).
#[allow(clippy::too_many_lines)]
fn parse_inline_content<'a>(content: &'a str, link_targets: &LinkTargets<'a>) -> Vec<Inline<'a>> {
    let mut nodes: Vec<Inline<'a>> = Vec::new();
    let mut last: Option<TextRun> = None;
    let b = content.as_bytes();
    let mut pos = 0usize;

    while pos < b.len() {
        // Strong: **text**
        if pos + 1 < b.len() && b[pos] == b'*' && b[pos + 1] == b'*' {
            if let Some(end) = find_closing(content, pos + 2, "**", true) {
                let children = parse_inline_content(&content[pos + 2..end], link_targets);
                last = None;
                nodes.push(Inline::Strong(children));
                pos = end + 2;
                continue;
            }
        }

        // Emphasis: *text*
        if b[pos] == b'*' {
            if let Some(end) = find_closing_char(content, pos + 1, b'*') {
                let text = &content[pos + 1..end];
                if !text.is_empty() && !text.starts_with('*') {
                    let children = parse_inline_content(text, link_targets);
                    last = None;
                    nodes.push(Inline::Emphasis(children));
                    pos = end + 1;
                    continue;
                }
            }
        }

        // Inline literal: ``text``
        if pos + 1 < b.len() && b[pos] == b'`' && b[pos + 1] == b'`' {
            // Inline literals are the one span the RST spec exempts from
            // escape processing: a backslash inside ``…`` is a literal
            // backslash, and cannot hide the closing delimiter.
            if let Some(end) = find_closing(content, pos + 2, "``", false) {
                last = None;
                nodes.push(Inline::Code(Cow::Borrowed(&content[pos + 2..end])));
                pos = end + 2;
                continue;
            }
        }

        // Interpreted text with role: :role:`text`
        if b[pos] == b':' {
            if let Some(role_end) = find_closing_char(content, pos + 1, b':') {
                if role_end + 1 < b.len() && b[role_end + 1] == b'`' {
                    if let Some(text_end) = find_closing_char(content, role_end + 2, b'`') {
                        let role = &content[pos + 1..role_end];
                        let text = &content[role_end + 2..text_end];
                        let inline = match role {
                            "emphasis" | "em" => {
                                Inline::Emphasis(parse_inline_content(text, link_targets))
                            }
                            "strong" => Inline::Strong(parse_inline_content(text, link_targets)),
                            "code" | "literal" => Inline::Code(Cow::Borrowed(text)),
                            "subscript" | "sub" => {
                                Inline::Subscript(parse_inline_content(text, link_targets))
                            }
                            "superscript" | "sup" => {
                                Inline::Superscript(parse_inline_content(text, link_targets))
                            }
                            "title-reference" | "title" | "t" => {
                                Inline::Emphasis(parse_inline_content(text, link_targets))
                            }
                            "ref" | "doc" => Inline::Link {
                                url: Cow::Owned(format!("#{}", text)),
                                children: vec![Inline::Text(Cow::Borrowed(text))],
                            },
                            "math" => Inline::MathInline {
                                source: Cow::Borrowed(text),
                            },
                            _ => Inline::RstSpan {
                                role: Cow::Borrowed(role),
                                children: parse_inline_content(text, link_targets),
                            },
                        };
                        last = None;
                        nodes.push(inline);
                        pos = text_end + 1;
                        continue;
                    }
                }
            }
        }

        // Inline link: `text <url>`_  or  `text`__  (anonymous)
        if b[pos] == b'`' {
            if let Some(end) = find_closing_char(content, pos + 1, b'`') {
                let text = &content[pos + 1..end];
                // Check for trailing __ (anonymous reference)
                if end + 2 < b.len() && b[end + 1] == b'_' && b[end + 2] == b'_' {
                    // Anonymous link — look up next anon target
                    let counter_key = "__anon_counter";
                    let counter: usize = link_targets
                        .get(counter_key)
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);
                    let anon_key = format!("__anon{}", counter);
                    if let Some(url) = link_targets.get(&anon_key).copied() {
                        // We need a mutable reference to increment counter, but link_targets is &
                        // For now: don't increment (single-use anonymous links work correctly)
                        last = None;
                        nodes.push(Inline::Link {
                            url: Cow::Borrowed(url),
                            children: vec![Inline::Text(Cow::Borrowed(text))],
                        });
                        pos = end + 3;
                        continue;
                    }
                }

                // Check for trailing _
                if end + 1 < b.len() && b[end + 1] == b'_' {
                    // Check if it's an inline link with URL
                    if let Some(angle_start) = text.rfind('<') {
                        if text.ends_with('>') {
                            let link_text = text[..angle_start].trim();
                            let url = &text[angle_start + 1..text.len() - 1];
                            last = None;
                            nodes.push(Inline::Link {
                                url: Cow::Borrowed(url),
                                children: vec![Inline::Text(Cow::Borrowed(link_text))],
                            });
                            pos = end + 2;
                            continue;
                        }
                    }

                    // Reference link - look up in link_targets
                    if let Some(url) = lookup_target(link_targets, text).copied() {
                        last = None;
                        nodes.push(Inline::Link {
                            url: Cow::Borrowed(url),
                            children: vec![Inline::Text(Cow::Borrowed(text))],
                        });
                        pos = end + 2;
                        continue;
                    }
                }

                // Plain interpreted text (default role, usually emphasis)
                last = None;
                nodes.push(Inline::Emphasis(vec![Inline::Text(Cow::Borrowed(text))]));
                pos = end + 1;
                continue;
            }
        }

        // Simple reference link: word_
        if content[pos..]
            .chars()
            .next()
            .is_some_and(char::is_alphanumeric)
        {
            let word_end = ref_name_end(content, pos);
            if word_end < b.len() && b[word_end] == b'_' {
                // Check it's not __ (anonymous reference)
                if word_end + 1 >= b.len() || b[word_end + 1] != b'_' {
                    let word = &content[pos..word_end];
                    if let Some(url) = lookup_target(link_targets, word).copied() {
                        last = None;
                        nodes.push(Inline::Link {
                            url: Cow::Borrowed(url),
                            children: vec![Inline::Text(Cow::Borrowed(word))],
                        });
                        pos = word_end + 1;
                        continue;
                    }
                }
            }
        }

        // Footnote reference: [label]_
        if b[pos] == b'[' {
            if let Some(close) = content[pos + 1..].find(']') {
                let close_abs = pos + 1 + close;
                // Check for trailing _
                if close_abs + 1 < b.len() && b[close_abs + 1] == b'_' {
                    last = None;
                    nodes.push(Inline::FootnoteRef {
                        label: Cow::Borrowed(&content[pos + 1..close_abs]),
                    });
                    pos = close_abs + 2;
                    continue;
                }
            }
        }

        // Regular text. Scanned as one contiguous run; an owned buffer is
        // materialized lazily, only if the run actually contains an escape.
        let run_start = pos;
        let mut owned: Option<String> = None;
        while pos < b.len() {
            let c = content[pos..].chars().next().unwrap();
            // RST escaping mechanism: a backslash strips the special meaning
            // of the character that follows, so the escaped character is
            // emitted as literal text and never inspected as markup. Escaped
            // whitespace disappears entirely (it exists only to make markup
            // adjacent to a word). This is the one place escapes are
            // resolved — `find_closing`/`find_closing_char` deliberately pass
            // them through so a nested span's content resolves them here too.
            if c == '\\' {
                let buf = owned.get_or_insert_with(|| content[run_start..pos].to_string());
                match content[pos + 1..].chars().next() {
                    Some(next) => {
                        if !next.is_whitespace() {
                            buf.push(next);
                        }
                        pos += 1 + next.len_utf8();
                    }
                    None => {
                        buf.push('\\');
                        pos += 1;
                    }
                }
                continue;
            }
            // Stop at potential inline markup starts
            if matches!(c, '*' | '`' | ':' | '[') {
                break;
            }
            // Stop at potential reference (word followed by _)
            if c.is_alphanumeric() {
                let word_end = ref_name_end(content, pos);
                if word_end < b.len()
                    && b[word_end] == b'_'
                    && (word_end + 1 >= b.len() || b[word_end + 1] != b'_')
                    && lookup_target(link_targets, &content[pos..word_end]).is_some()
                {
                    break;
                }
            }
            if let Some(buf) = owned.as_mut() {
                buf.push(c);
            }
            pos += c.len_utf8();
        }

        if pos == run_start {
            // No markup matched and the text loop didn't advance — consume
            // the current character literally to guarantee forward progress.
            let clen = char_len_at(content, pos);
            push_text(
                &mut nodes,
                &mut last,
                content,
                pos,
                pos + clen,
                Cow::Borrowed(&content[pos..pos + clen]),
            );
            pos += clen;
        } else {
            let resolved = match owned {
                Some(s) => Cow::Owned(s),
                None => Cow::Borrowed(&content[run_start..pos]),
            };
            push_text(&mut nodes, &mut last, content, run_start, pos, resolved);
        }
    }

    nodes
}

/// Byte offset of `pattern`'s next occurrence at or after `start`, or `None`.
///
/// The span's text is exactly `content[start..offset]` — when `escapes` is
/// set, a backslash-escaped character cannot close the span (RST's escaping
/// mechanism) but the escape is *passed through* rather than resolved, because
/// the span's content is re-parsed by `parse_inline_content`, which resolves
/// escapes exactly once, at the level that actually emits the text. Inline
/// literals pass `false` — the spec exempts them from escape processing
/// entirely. Returning an offset rather than a rebuilt `String` is what lets
/// nested spans keep borrowing from the input.
fn find_closing(content: &str, start: usize, pattern: &str, escapes: bool) -> Option<usize> {
    let b = content.as_bytes();
    let pat = pattern.as_bytes();
    let mut pos = start;

    while pos + pat.len() <= b.len() {
        if escapes && b[pos] == b'\\' && pos + 1 < b.len() {
            pos += 1;
            pos += char_len_at(content, pos);
            continue;
        }
        if b[pos..pos + pat.len()] == *pat {
            return Some(pos);
        }
        pos += char_len_at(content, pos);
    }

    None
}

/// Byte offset of the next unescaped `close` (an ASCII delimiter) at or after
/// `start`. As in [`find_closing`], escapes are passed through rather than
/// resolved — every span this is used for (emphasis, interpreted text, roles)
/// has its content re-parsed, and resolving here would let `\*` re-enter the
/// tokenizer as live markup.
fn find_closing_char(content: &str, start: usize, close: u8) -> Option<usize> {
    let b = content.as_bytes();
    let mut pos = start;

    while pos < b.len() {
        if b[pos] == b'\\' && pos + 1 < b.len() {
            pos += 1;
            pos += char_len_at(content, pos);
            continue;
        }
        if b[pos] == close {
            return Some(pos);
        }
        pos += char_len_at(content, pos);
    }

    None
}

// ── Builder ───────────────────────────────────────────────────────────────────

/// Build an RST string from an [`RstDoc`].
pub fn build(doc: &RstDoc) -> String {
    let mut ctx = BuildContext::new();
    build_blocks(&doc.blocks, &mut ctx);
    ctx.output
}

pub(crate) struct BuildContext {
    pub(crate) output: String,
    list_depth: usize,
}

impl BuildContext {
    pub(crate) fn new() -> Self {
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

pub(crate) fn build_block(block: &Block, ctx: &mut BuildContext) {
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

    let col_widths = calculate_column_widths(text_rows.iter().map(Vec::as_slice));

    emit_table_border(&col_widths, &mut ctx.output);

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
            emit_table_border(&col_widths, &mut ctx.output);
        }
        is_first = false;
    }

    emit_table_border(&col_widths, &mut ctx.output);
    ctx.write("\n");
}

/// Shared by `build_table` and the streaming writer's `render_table` — the two
/// emission paths are independent, but the *border geometry* is one function,
/// not two copies that could drift. Takes a plain `&mut String` rather than a
/// `BuildContext` so the streaming writer can emit straight into its own
/// shared output buffer.
pub(crate) fn emit_table_border(widths: &[usize], out: &mut String) {
    out.push('+');
    for w in widths {
        for _ in 0..(*w + 2) {
            out.push('-');
        }
        out.push('+');
    }
    out.push('\n');
}

/// Column widths for a table, as an iterator over rows of already-collected
/// cell text. Iterator-shaped (rather than `&[Vec<String>]`) so the streaming
/// writer can pass a projection of its `(cells, is_header)` rows without
/// cloning every cell into a parallel `Vec<Vec<String>>` first.
pub(crate) fn calculate_column_widths<'a, I: Iterator<Item = &'a [String]>>(rows: I) -> Vec<usize> {
    let mut widths: Vec<usize> = Vec::new();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i == widths.len() {
                widths.push(1);
            }
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

/// Re-apply RST's escaping mechanism to literal text on emit.
///
/// The reader resolves `\*` to a literal `*`, so the writer must put the
/// backslash back — otherwise `parse(emit(parse(x))) != parse(x)` for any
/// document containing a literal asterisk, backtick, or backslash: the bare
/// character would be re-read as live markup. Borrows unless an escape is
/// actually needed, which is the overwhelmingly common case.
pub(crate) fn escape_text(s: &str) -> Cow<'_, str> {
    if !s.bytes().any(|b| b == b'\\' || b == b'*' || b == b'`') {
        return Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        if c == '\\' || c == '*' || c == '`' {
            out.push('\\');
        }
        out.push(c);
    }
    Cow::Owned(out)
}

fn build_inline(inline: &Inline, ctx: &mut BuildContext) {
    match inline {
        Inline::Text(s) => {
            let escaped = escape_text(s);
            ctx.write(&escaped);
        }

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
            // Escaped, because this text is what actually gets *emitted* into
            // a heading line or a table cell: underline widths and column
            // widths have to count the bytes that will be written.
            Inline::Text(s) => out.push_str(&escape_text(s)),
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

    /// RST's escaping mechanism: a backslash strips the special meaning of
    /// the next character, so `\*not emphasis\*` is literal text. Before this
    /// was implemented the parser saw live emphasis markup there and silently
    /// misread the document.
    #[test]
    fn test_parse_escaped_markup_is_literal_text() {
        let doc = parse(r"This is \*not emphasis\* here.").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(
            !inlines.iter().any(|i| matches!(i, Inline::Emphasis(_))),
            "escaped asterisks must not open emphasis: {inlines:?}"
        );
        assert!(
            matches!(&inlines[0], Inline::Text(s) if s == "This is *not emphasis* here."),
            "escape must resolve to a literal asterisk: {inlines:?}"
        );
    }

    /// Escaped whitespace is removed entirely — that is the whole point of
    /// `word\ *markup*`, which makes markup adjacent to a word.
    #[test]
    fn test_parse_escaped_whitespace_is_removed() {
        let doc = parse(r"star\ *adjacent*").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(
            matches!(&inlines[0], Inline::Text(s) if s == "star"),
            "{inlines:?}"
        );
        assert!(matches!(&inlines[1], Inline::Emphasis(_)), "{inlines:?}");
    }

    /// Inline literals are exempt from escape processing per the RST spec:
    /// a backslash inside ``…`` is a literal backslash.
    #[test]
    fn test_parse_inline_literal_keeps_backslashes() {
        let doc = parse(r"``literal \* stays``").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(
            matches!(&inlines[0], Inline::Code(s) if s == r"literal \* stays"),
            "{inlines:?}"
        );
    }

    /// The escape has to survive emission, or the round trip silently turns
    /// literal text back into markup: `parse(emit(parse(x))) == parse(x)`.
    #[test]
    fn test_escaped_markup_roundtrips() {
        for input in [
            r"This is \*not emphasis\* here.",
            r"A backslash \\ and \`not literal\`.",
            r"Real *emphasis* and \*escaped\* together.",
        ] {
            let doc = parse(input).unwrap();
            let emitted = build(&doc);
            let doc2 = parse(&emitted).unwrap();
            assert_eq!(
                format!("{:?}", doc.blocks),
                format!("{:?}", doc2.blocks),
                "escape round-trip lost information for {input:?} (emitted {emitted:?})"
            );
        }
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

    #[test]
    fn test_parse_grid_table() {
        let input = "+--------+--------+\n| A      | B      |\n+========+========+\n| Cell 1 | Cell 2 |\n+--------+--------+\n";
        let doc = parse(input).unwrap();
        assert!(
            matches!(doc.blocks[0], Block::Table { .. }),
            "expected Table, got {:?}",
            doc.blocks[0]
        );
        let Block::Table { rows } = &doc.blocks[0] else {
            panic!()
        };
        assert_eq!(rows.len(), 2);
        assert!(rows[0].is_header);
        assert!(!rows[1].is_header);
    }

    #[test]
    fn test_parse_simple_table() {
        let input = "=====  =====\nA      B\n=====  =====\n1      2\n=====  =====\n";
        let doc = parse(input).unwrap();
        assert!(
            matches!(doc.blocks[0], Block::Table { .. }),
            "expected Table, got {:?}",
            doc.blocks[0]
        );
        let Block::Table { rows } = &doc.blocks[0] else {
            panic!()
        };
        assert_eq!(rows.len(), 2);
        assert!(rows[0].is_header);
        assert!(!rows[1].is_header);
    }

    // ── events() vs parse() equivalence ───────────────────────────────────────
    //
    // `events()` and `parse()` are independent implementations sharing only
    // `Parser` (see the module doc comment on `EventIter`). This test proves
    // they agree on which constructs are produced and in what order, without
    // requiring identical Rust types — by reducing each side to a "shape
    // signature" of discriminant tags.

    /// Reduce a parsed `Block`/`Inline` tree to a flat sequence of
    /// Start/End/Leaf tag names, in emission order — the same shape an
    /// event stream produces.
    fn block_shape(block: &Block, out: &mut Vec<&'static str>) {
        match block {
            Block::Paragraph { inlines } => {
                out.push("StartParagraph");
                inlines.iter().for_each(|i| inline_shape(i, out));
                out.push("EndParagraph");
            }
            Block::Heading { inlines, .. } => {
                out.push("StartHeading");
                inlines.iter().for_each(|i| inline_shape(i, out));
                out.push("EndHeading");
            }
            Block::CodeBlock { .. } => {
                out.push("StartCodeBlock");
                out.push("CodeBlockContent");
                out.push("EndCodeBlock");
            }
            Block::Blockquote { children } => {
                out.push("StartBlockquote");
                children.iter().for_each(|b| block_shape(b, out));
                out.push("EndBlockquote");
            }
            Block::List { items, .. } => {
                out.push("StartList");
                for item in items {
                    out.push("StartListItem");
                    item.iter().for_each(|b| block_shape(b, out));
                    out.push("EndListItem");
                }
                out.push("EndList");
            }
            Block::DefinitionList { items } => {
                out.push("StartDefinitionList");
                for item in items {
                    out.push("StartDefinitionTerm");
                    item.term.iter().for_each(|i| inline_shape(i, out));
                    out.push("EndDefinitionTerm");
                    out.push("StartDefinitionDesc");
                    item.desc.iter().for_each(|i| inline_shape(i, out));
                    out.push("EndDefinitionDesc");
                }
                out.push("EndDefinitionList");
            }
            Block::Figure { caption, .. } => {
                out.push("StartFigure");
                if let Some(cap) = caption {
                    cap.iter().for_each(|i| inline_shape(i, out));
                }
                out.push("EndFigure");
            }
            Block::Image { .. } => out.push("ImageBlock"),
            Block::RawBlock { .. } => out.push("RawBlock"),
            Block::Div { children, .. } => {
                out.push("StartDiv");
                children.iter().for_each(|b| block_shape(b, out));
                out.push("EndDiv");
            }
            Block::HorizontalRule => out.push("HorizontalRule"),
            Block::Table { rows } => {
                out.push("StartTable");
                for row in rows {
                    out.push("StartTableRow");
                    for cell in &row.cells {
                        out.push("StartTableCell");
                        cell.iter().for_each(|i| inline_shape(i, out));
                        out.push("EndTableCell");
                    }
                    out.push("EndTableRow");
                }
                out.push("EndTable");
            }
            Block::FootnoteDef { inlines, .. } => {
                out.push("StartFootnoteDef");
                inlines.iter().for_each(|i| inline_shape(i, out));
                out.push("EndFootnoteDef");
            }
            Block::MathDisplay { .. } => out.push("MathDisplay"),
            Block::Admonition { children, .. } => {
                out.push("StartAdmonition");
                children.iter().for_each(|b| block_shape(b, out));
                out.push("EndAdmonition");
            }
        }
    }

    fn inline_shape(inline: &Inline, out: &mut Vec<&'static str>) {
        match inline {
            Inline::Text(_) => out.push("Text"),
            Inline::Emphasis(c) => wrap(out, "Emphasis", c),
            Inline::Strong(c) => wrap(out, "Strong", c),
            Inline::Strikeout(c) => wrap(out, "Strikeout", c),
            Inline::Underline(c) => wrap(out, "Underline", c),
            Inline::Subscript(c) => wrap(out, "Subscript", c),
            Inline::Superscript(c) => wrap(out, "Superscript", c),
            Inline::SmallCaps(c) => wrap(out, "SmallCaps", c),
            Inline::Code(_) => out.push("Code"),
            Inline::Link { children, .. } => wrap(out, "Link", children),
            Inline::Image { .. } => out.push("InlineImage"),
            Inline::LineBreak => out.push("LineBreak"),
            Inline::SoftBreak => out.push("SoftBreak"),
            Inline::FootnoteRef { .. } => out.push("FootnoteRef"),
            Inline::FootnoteDef { children, .. } => wrap(out, "FootnoteDefInline", children),
            Inline::Quoted { children, .. } => wrap(out, "Quoted", children),
            Inline::MathInline { .. } => out.push("MathInline"),
            Inline::RstSpan { children, .. } => wrap(out, "RstSpan", children),
        }

        fn wrap(out: &mut Vec<&'static str>, name: &'static str, children: &[Inline]) {
            out.push(name);
            children.iter().for_each(|i| inline_shape(i, out));
            // Close tags aren't disambiguated per-name here since the open tag
            // already anchors nesting; sufficient for a shape comparison.
        }
    }

    /// Reduce an `Event` stream to the same tag vocabulary as `block_shape`/
    /// `inline_shape`, dropping payload data.
    fn event_shape(input: &str) -> Vec<&'static str> {
        events(input)
            .map(|e| match e {
                Event::StartParagraph => "StartParagraph",
                Event::EndParagraph => "EndParagraph",
                Event::StartHeading { .. } => "StartHeading",
                Event::EndHeading => "EndHeading",
                Event::StartBlockquote => "StartBlockquote",
                Event::EndBlockquote => "EndBlockquote",
                Event::StartList { .. } => "StartList",
                Event::EndList => "EndList",
                Event::StartListItem => "StartListItem",
                Event::EndListItem => "EndListItem",
                Event::StartCodeBlock { .. } => "StartCodeBlock",
                Event::CodeBlockContent(_) => "CodeBlockContent",
                Event::EndCodeBlock => "EndCodeBlock",
                Event::RawBlock { .. } => "RawBlock",
                Event::StartDiv { .. } => "StartDiv",
                Event::EndDiv => "EndDiv",
                Event::HorizontalRule => "HorizontalRule",
                Event::StartTable => "StartTable",
                Event::EndTable => "EndTable",
                Event::StartTableRow { .. } => "StartTableRow",
                Event::EndTableRow => "EndTableRow",
                Event::StartTableCell => "StartTableCell",
                Event::EndTableCell => "EndTableCell",
                Event::StartDefinitionList => "StartDefinitionList",
                Event::EndDefinitionList => "EndDefinitionList",
                Event::StartDefinitionTerm => "StartDefinitionTerm",
                Event::EndDefinitionTerm => "EndDefinitionTerm",
                Event::StartDefinitionDesc => "StartDefinitionDesc",
                Event::EndDefinitionDesc => "EndDefinitionDesc",
                Event::StartFootnoteDef { .. } => "StartFootnoteDef",
                Event::EndFootnoteDef => "EndFootnoteDef",
                Event::MathDisplay { .. } => "MathDisplay",
                Event::StartAdmonition { .. } => "StartAdmonition",
                Event::EndAdmonition => "EndAdmonition",
                Event::StartFigure { .. } => "StartFigure",
                Event::EndFigure => "EndFigure",
                Event::ImageBlock { .. } => "ImageBlock",
                Event::Text(_) => "Text",
                Event::SoftBreak => "SoftBreak",
                Event::LineBreak => "LineBreak",
                Event::StartEmphasis => "Emphasis",
                Event::EndEmphasis => "__skip",
                Event::StartStrong => "Strong",
                Event::EndStrong => "__skip",
                Event::StartStrikeout => "Strikeout",
                Event::EndStrikeout => "__skip",
                Event::StartUnderline => "Underline",
                Event::EndUnderline => "__skip",
                Event::StartSubscript => "Subscript",
                Event::EndSubscript => "__skip",
                Event::StartSuperscript => "Superscript",
                Event::EndSuperscript => "__skip",
                Event::StartSmallCaps => "SmallCaps",
                Event::EndSmallCaps => "__skip",
                Event::Code(_) => "Code",
                Event::StartLink { .. } => "Link",
                Event::EndLink => "__skip",
                Event::InlineImage { .. } => "InlineImage",
                Event::FootnoteRef { .. } => "FootnoteRef",
                Event::StartFootnoteDefInline { .. } => "FootnoteDefInline",
                Event::EndFootnoteDefInline => "__skip",
                Event::StartQuoted { .. } => "Quoted",
                Event::EndQuoted => "__skip",
                Event::MathInline { .. } => "MathInline",
                Event::StartRstSpan { .. } => "RstSpan",
                Event::EndRstSpan => "__skip",
            })
            .filter(|s| *s != "__skip")
            .collect()
    }

    fn parse_shape(input: &str) -> Vec<&'static str> {
        let doc = parse(input).unwrap();
        let mut out = Vec::new();
        doc.blocks.iter().for_each(|b| block_shape(b, &mut out));
        out
    }

    #[test]
    fn test_events_matches_parse_shape() {
        let inputs = [
            "Section\n=======\n\nHello *world* and **strong** and ``code``.\n",
            "- item one\n- item two\n  - nested\n",
            "1. first\n2. second\n",
            ".. code-block:: rust\n\n   let x = 1;\n",
            ".. note::\n\n   A note.\n",
            "term\n    definition text\n",
            "+--------+--------+\n| A      | B      |\n+========+========+\n| Cell 1 | Cell 2 |\n+--------+--------+\n",
            "=====  =====\nA      B\n=====  =====\n1      2\n=====  =====\n",
            "See [1]_ for details.\n\n.. [1] A footnote body that continues\n   on a second line.\n",
            "    indented block quote text\n\nAfter.\n",
            "----\n\nAfter the rule.\n",
        ];
        for input in inputs {
            let via_parse = parse_shape(input);
            let via_events = event_shape(input);
            assert_eq!(
                via_parse, via_events,
                "shape mismatch for input {input:?}\nparse: {via_parse:?}\nevents: {via_events:?}"
            );
        }
    }
}
