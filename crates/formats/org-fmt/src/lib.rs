//! Org-mode parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-org` and `rescribe-write-org` as thin adapter layers.

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct OrgError(pub String);

impl std::fmt::Display for OrgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Org error: {}", self.0)
    }
}

impl std::error::Error for OrgError {}

// ── AST ───────────────────────────────────────────────────────────────────────

/// A parsed Org-mode document.
#[derive(Debug, Clone, Default)]
pub struct OrgDoc {
    pub blocks: Vec<Block>,
    /// Document-level metadata (e.g. title, author from #+TITLE: etc.)
    pub metadata: Vec<(String, String)>,
}

/// Block-level element.
#[derive(Debug, Clone)]
pub enum Block {
    Paragraph {
        inlines: Vec<Inline>,
    },
    Heading {
        level: usize,
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
        items: Vec<ListItem>,
    },
    Table {
        rows: Vec<TableRow>,
    },
    HorizontalRule,
    DefinitionList {
        items: Vec<DefinitionItem>,
    },
    Div {
        inlines: Vec<Inline>,
    },
    /// Raw block (format, content)
    RawBlock {
        format: String,
        content: String,
    },
    Figure {
        children: Vec<Block>,
    },
    Caption {
        inlines: Vec<Inline>,
    },
    /// Unknown block type logged as warning
    Unknown {
        kind: String,
    },
}

/// A list item (may contain inline or block content).
#[derive(Debug, Clone)]
pub struct ListItem {
    pub children: Vec<ListItemContent>,
}

/// Content within a list item.
#[derive(Debug, Clone)]
pub enum ListItemContent {
    Inline(Vec<Inline>),
    Block(Block),
}

/// A definition list item with term and description.
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
    Bold(Vec<Inline>),
    Italic(Vec<Inline>),
    Underline(Vec<Inline>),
    Strikethrough(Vec<Inline>),
    Code(String),
    Link {
        url: String,
        children: Vec<Inline>,
    },
    Image {
        url: String,
    },
    LineBreak,
    SoftBreak,
    Superscript(Vec<Inline>),
    Subscript(Vec<Inline>),
    FootnoteRef {
        label: String,
    },
    FootnoteDefinition {
        label: String,
        children: Vec<Inline>,
    },
    MathInline {
        source: String,
    },
}

// ── Unknown block kinds encountered during parsing ────────────────────────────

/// A warning emitted during parse or build.
#[derive(Debug, Clone)]
pub struct OrgWarning {
    pub kind: String,
    pub message: String,
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Parse an Org-mode string into an [`OrgDoc`].
///
/// Unknown block types are included in the result as `Block::Unknown` and
/// also listed in [`ParseResult::warnings`].
pub fn parse(input: &str) -> Result<ParseResult, OrgError> {
    let mut p = OrgParser::new(input);
    let (blocks, metadata, warnings) = p.parse_document();
    Ok(ParseResult {
        doc: OrgDoc { blocks, metadata },
        warnings,
    })
}

/// Result of [`parse`].
pub struct ParseResult {
    pub doc: OrgDoc,
    pub warnings: Vec<OrgWarning>,
}

struct OrgParser<'a> {
    lines: Vec<&'a str>,
    pos: usize,
    warnings: Vec<OrgWarning>,
}

impl<'a> OrgParser<'a> {
    fn new(input: &'a str) -> Self {
        let lines: Vec<&str> = input.lines().collect();
        Self {
            lines,
            pos: 0,
            warnings: Vec::new(),
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

    fn parse_document(&mut self) -> (Vec<Block>, Vec<(String, String)>, Vec<OrgWarning>) {
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
                    });
                    current_para.clear();
                }
                blocks.push(Block::HorizontalRule);
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
            });
        }

        (blocks, metadata, std::mem::take(&mut self.warnings))
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
        }
    }

    fn parse_block(&mut self) -> Option<Block> {
        let orig_line = self.current_line()?;
        let line_upper = orig_line.to_uppercase();
        let block_type = line_upper
            .strip_prefix("#+BEGIN_")?
            .split_whitespace()
            .next()?
            .to_uppercase();

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
            }),
            "QUOTE" => {
                let inlines = parse_inline_content(&content_str);
                Some(Block::Blockquote {
                    children: vec![Block::Paragraph { inlines }],
                })
            }
            "EXAMPLE" | "VERSE" => Some(Block::CodeBlock {
                language: None,
                content: content_str,
            }),
            "CENTER" => Some(Block::Div {
                inlines: parse_inline_content(&content_str),
            }),
            _ => {
                self.warnings.push(OrgWarning {
                    kind: format!("org:{}", block_type),
                    message: format!("Unknown block type: {}", block_type),
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

        Block::List { ordered, items }
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

fn parse_metadata_line(line: &str) -> Option<(String, String)> {
    let line = line.strip_prefix("#+")?.trim();
    let (key, value) = line.split_once(':')?;
    Some((key.trim().to_lowercase(), value.trim().to_string()))
}

fn strip_heading_metadata(text: &str) -> String {
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

fn is_list_item(line: &str) -> bool {
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

fn is_ordered_list_item(line: &str) -> bool {
    let trimmed = line.trim_start();
    if let Some(rest) = trimmed.strip_prefix(|c: char| c.is_ascii_digit()) {
        let rest = rest.trim_start_matches(|c: char| c.is_ascii_digit());
        rest.starts_with(". ") || rest.starts_with(") ")
    } else {
        false
    }
}

fn parse_inline_content(text: &str) -> Vec<Inline> {
    let mut nodes = Vec::new();
    let mut pos = 0;
    let chars: Vec<char> = text.chars().collect();

    while pos < chars.len() {
        let c = chars[pos];

        match c {
            // Bold: *text*
            '*' => {
                if let Some((content, end)) = find_inline_span(&chars, pos, '*') {
                    nodes.push(Inline::Bold(parse_inline_content(&content)));
                    pos = end + 1;
                    continue;
                }
            }
            // Italic: /text/
            '/' => {
                if let Some((content, end)) = find_inline_span(&chars, pos, '/') {
                    nodes.push(Inline::Italic(parse_inline_content(&content)));
                    pos = end + 1;
                    continue;
                }
            }
            // Underline: _text_
            '_' => {
                if let Some((content, end)) = find_inline_span(&chars, pos, '_') {
                    nodes.push(Inline::Underline(parse_inline_content(&content)));
                    pos = end + 1;
                    continue;
                }
            }
            // Strikethrough: +text+
            '+' => {
                if let Some((content, end)) = find_inline_span(&chars, pos, '+') {
                    nodes.push(Inline::Strikethrough(parse_inline_content(&content)));
                    pos = end + 1;
                    continue;
                }
            }
            // Code: ~text~ or =text=
            '~' | '=' => {
                if let Some((content, end)) = find_inline_span(&chars, pos, c) {
                    nodes.push(Inline::Code(content));
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
            Some(Inline::Text(s)) => {
                s.push(c);
            }
            _ => {
                nodes.push(Inline::Text(c.to_string()));
            }
        }
        pos += 1;
    }

    // Merge adjacent text nodes (already handled by appending to last)
    nodes
}

fn find_inline_span(chars: &[char], start: usize, marker: char) -> Option<(String, usize)> {
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

fn parse_link(chars: &[char], start: usize) -> Option<(Inline, usize)> {
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
                    vec![Inline::Text(url.clone())]
                } else {
                    parse_inline_content(&description)
                };
                return Some((Inline::Link { url, children }, pos + 2));
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

// ── Builder ───────────────────────────────────────────────────────────────────

/// Build an Org-mode string from an [`OrgDoc`].
pub fn build(doc: &OrgDoc) -> BuildResult {
    let mut ctx = BuildContext::new();

    // Emit metadata as #+KEY: value lines
    for (key, value) in &doc.metadata {
        ctx.write("#+");
        ctx.write(&key.to_uppercase());
        ctx.write(": ");
        ctx.write(value);
        ctx.write("\n");
    }
    if !doc.metadata.is_empty() && !doc.blocks.is_empty() {
        ctx.write("\n");
    }

    for block in &doc.blocks {
        build_block(block, &mut ctx);
    }

    // Ensure trailing newline
    if !ctx.output.ends_with('\n') {
        ctx.output.push('\n');
    }

    BuildResult {
        output: ctx.output,
        warnings: ctx.warnings,
    }
}

/// Result of [`build`].
pub struct BuildResult {
    pub output: String,
    pub warnings: Vec<OrgWarning>,
}

struct BuildContext {
    output: String,
    warnings: Vec<OrgWarning>,
    list_depth: usize,
}

impl BuildContext {
    fn new() -> Self {
        Self {
            output: String::new(),
            warnings: Vec::new(),
            list_depth: 0,
        }
    }

    fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn ensure_blank_line(&mut self) {
        let trimmed = self.output.trim_end();
        let len = trimmed.len();
        self.output.truncate(len);
        self.output.push_str("\n\n");
    }

    fn ensure_newline(&mut self) {
        if !self.output.is_empty() && !self.output.ends_with('\n') {
            self.output.push('\n');
        }
    }
}

fn build_block(block: &Block, ctx: &mut BuildContext) {
    match block {
        Block::Paragraph { inlines } => {
            build_inlines(inlines, ctx);
            ctx.ensure_blank_line();
        }

        Block::Heading { level, inlines } => {
            ctx.ensure_newline();
            for _ in 0..*level {
                ctx.write("*");
            }
            ctx.write(" ");
            build_inlines(inlines, ctx);
            ctx.ensure_blank_line();
        }

        Block::CodeBlock { language, content } => {
            ctx.ensure_newline();
            if let Some(lang) = language {
                ctx.write("#+BEGIN_SRC ");
                ctx.write(lang);
                ctx.write("\n");
            } else {
                ctx.write("#+BEGIN_SRC\n");
            }
            ctx.write(content);
            if !content.ends_with('\n') {
                ctx.write("\n");
            }
            ctx.write("#+END_SRC\n\n");
        }

        Block::Blockquote { children } => {
            ctx.ensure_newline();
            ctx.write("#+BEGIN_QUOTE\n");
            for child in children {
                build_block(child, ctx);
            }
            ctx.write("#+END_QUOTE\n\n");
        }

        Block::List { ordered, items } => {
            ctx.list_depth += 1;
            let mut counter = 1i32;
            for item in items {
                build_list_item(item, *ordered, &mut counter, ctx);
            }
            ctx.list_depth -= 1;
            if ctx.list_depth == 0 {
                ctx.ensure_newline();
            }
        }

        Block::Table { rows } => {
            build_table(rows, ctx);
        }

        Block::HorizontalRule => {
            ctx.ensure_newline();
            ctx.write("-----\n\n");
        }

        Block::DefinitionList { items } => {
            for item in items {
                ctx.write("- ");
                build_inlines(&item.term, ctx);
                ctx.write(" :: ");
                build_inlines(&item.desc, ctx);
                ctx.ensure_newline();
            }
            ctx.ensure_newline();
        }

        Block::Div { inlines } => {
            build_inlines(inlines, ctx);
        }

        Block::RawBlock { format, content } => {
            if format == "org" {
                ctx.write(content);
            }
        }

        Block::Figure { children } => {
            for child in children {
                build_block(child, ctx);
            }
        }

        Block::Caption { inlines } => {
            ctx.write("#+CAPTION: ");
            build_inlines(inlines, ctx);
            ctx.ensure_newline();
        }

        Block::Unknown { kind } => {
            ctx.warnings.push(OrgWarning {
                kind: kind.clone(),
                message: format!("Unknown block type for Org: {}", kind),
            });
        }
    }
}

fn build_list_item(item: &ListItem, ordered: bool, counter: &mut i32, ctx: &mut BuildContext) {
    let indent = "  ".repeat(ctx.list_depth - 1);
    ctx.write(&indent);

    if ordered {
        ctx.write(&format!("{}. ", counter));
        *counter += 1;
    } else {
        ctx.write("- ");
    }

    let mut first = true;
    for child in &item.children {
        match child {
            ListItemContent::Inline(inlines) => {
                build_inlines(inlines, ctx);
                ctx.ensure_newline();
            }
            ListItemContent::Block(block) => {
                if first {
                    // If first child is a paragraph-like block, emit inline
                    if let Block::Paragraph { inlines } = block {
                        build_inlines(inlines, ctx);
                        ctx.ensure_newline();
                    } else {
                        ctx.ensure_newline();
                        build_block(block, ctx);
                    }
                } else if let Block::Paragraph { inlines } = block {
                    let content_indent = "  ".repeat(ctx.list_depth);
                    ctx.write(&content_indent);
                    build_inlines(inlines, ctx);
                    ctx.ensure_newline();
                } else if let Block::List { .. } = block {
                    ctx.ensure_newline();
                    build_block(block, ctx);
                } else {
                    build_block(block, ctx);
                }
            }
        }
        first = false;
    }
}

fn build_table(rows: &[TableRow], ctx: &mut BuildContext) {
    ctx.ensure_newline();

    if rows.is_empty() {
        ctx.ensure_blank_line();
        return;
    }

    // Build string representations of all cells
    let string_rows: Vec<(Vec<String>, bool)> = rows
        .iter()
        .map(|row| {
            let cells: Vec<String> = row
                .cells
                .iter()
                .map(|cell| {
                    let mut cell_ctx = BuildContext::new();
                    build_inlines(cell, &mut cell_ctx);
                    cell_ctx.output.trim().to_string()
                })
                .collect();
            (cells, row.is_header)
        })
        .collect();

    // Calculate column widths
    let num_cols = string_rows.iter().map(|(r, _)| r.len()).max().unwrap_or(0);
    let mut col_widths = vec![0usize; num_cols];
    for (row, _) in &string_rows {
        for (i, cell) in row.iter().enumerate() {
            col_widths[i] = col_widths[i].max(cell.len());
        }
    }

    // Find where the first header row ends (for separator placement)
    let first_header_idx = string_rows.iter().position(|(_, is_hdr)| *is_hdr);
    let separator_after = first_header_idx.map(|_| 0usize); // separator after row 0 if there's a header

    let total_rows = string_rows.len();
    for (idx, (row, _is_header)) in string_rows.iter().enumerate() {
        ctx.write("|");
        for (i, cell) in row.iter().enumerate() {
            ctx.write(" ");
            ctx.write(cell);
            let padding = col_widths[i].saturating_sub(cell.len());
            ctx.write(&" ".repeat(padding));
            ctx.write(" |");
        }
        ctx.write("\n");

        // Add separator after first row if there are multiple rows
        if separator_after.is_some() && idx == 0 && total_rows > 1 {
            ctx.write("|");
            for width in &col_widths {
                ctx.write("-");
                ctx.write(&"-".repeat(*width));
                ctx.write("-+");
            }
            // Fix the last + to |
            if !col_widths.is_empty() {
                ctx.output.pop();
                ctx.write("|");
            }
            ctx.write("\n");
        }
    }

    ctx.ensure_blank_line();
}

fn build_inlines(inlines: &[Inline], ctx: &mut BuildContext) {
    for inline in inlines {
        build_inline(inline, ctx);
    }
}

fn build_inline(inline: &Inline, ctx: &mut BuildContext) {
    match inline {
        Inline::Text(s) => ctx.write(s),

        Inline::Bold(children) => {
            ctx.write("*");
            build_inlines(children, ctx);
            ctx.write("*");
        }

        Inline::Italic(children) => {
            ctx.write("/");
            build_inlines(children, ctx);
            ctx.write("/");
        }

        Inline::Underline(children) => {
            ctx.write("_");
            build_inlines(children, ctx);
            ctx.write("_");
        }

        Inline::Strikethrough(children) => {
            ctx.write("+");
            build_inlines(children, ctx);
            ctx.write("+");
        }

        Inline::Code(s) => {
            ctx.write("=");
            ctx.write(s);
            ctx.write("=");
        }

        Inline::Link { url, children } => {
            ctx.write("[[");
            ctx.write(url);
            ctx.write("][");
            build_inlines(children, ctx);
            ctx.write("]]");
        }

        Inline::Image { url } => {
            ctx.write("[[");
            if !url.starts_with("file:") && !url.starts_with("http") {
                ctx.write("file:");
            }
            ctx.write(url);
            ctx.write("]]");
        }

        Inline::LineBreak => ctx.write("\\\\\n"),

        Inline::SoftBreak => ctx.write("\n"),

        Inline::Superscript(children) => {
            ctx.write("^{");
            build_inlines(children, ctx);
            ctx.write("}");
        }

        Inline::Subscript(children) => {
            ctx.write("_{");
            build_inlines(children, ctx);
            ctx.write("}");
        }

        Inline::FootnoteRef { label } => {
            ctx.write("[fn:");
            ctx.write(label);
            ctx.write("]");
        }

        Inline::FootnoteDefinition { label, children } => {
            ctx.write("[fn:");
            ctx.write(label);
            ctx.write("] ");
            build_inlines(children, ctx);
        }

        Inline::MathInline { source } => {
            ctx.write("$");
            ctx.write(source);
            ctx.write("$");
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(input: &str) -> OrgDoc {
        parse(input).unwrap().doc
    }

    // ── Parser tests ──────────────────────────────────────────────────────────

    #[test]
    fn test_parse_heading() {
        let doc = parse_ok("* Hello World\n** Subheading");
        assert_eq!(doc.blocks.len(), 2);
        assert!(matches!(doc.blocks[0], Block::Heading { level: 1, .. }));
        assert!(matches!(doc.blocks[1], Block::Heading { level: 2, .. }));
    }

    #[test]
    fn test_parse_paragraph() {
        let doc = parse_ok("This is a paragraph.\n\nThis is another.");
        assert_eq!(doc.blocks.len(), 2);
        assert!(matches!(doc.blocks[0], Block::Paragraph { .. }));
        assert!(matches!(doc.blocks[1], Block::Paragraph { .. }));
    }

    #[test]
    fn test_parse_emphasis() {
        let doc = parse_ok("/italic/ and *bold*");
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Italic(_))));
        assert!(inlines.iter().any(|i| matches!(i, Inline::Bold(_))));
    }

    #[test]
    fn test_parse_list() {
        let doc = parse_ok("- First item\n- Second item");
        assert!(!doc.blocks.is_empty());
        let Block::List { ordered, items } = &doc.blocks[0] else {
            panic!("expected list");
        };
        assert!(!ordered);
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_parse_ordered_list() {
        let doc = parse_ok("1. First\n2. Second");
        let Block::List { ordered, .. } = &doc.blocks[0] else {
            panic!("expected list");
        };
        assert!(ordered);
    }

    #[test]
    fn test_parse_code_block() {
        let doc = parse_ok("#+BEGIN_SRC rust\nfn main() {}\n#+END_SRC");
        let Block::CodeBlock { language, content } = &doc.blocks[0] else {
            panic!("expected code block");
        };
        assert_eq!(language.as_deref(), Some("rust"));
        assert_eq!(content, "fn main() {}");
    }

    #[test]
    fn test_parse_metadata() {
        let doc = parse_ok("#+TITLE: My Document\n#+AUTHOR: Jane Doe\n\nContent here.");
        assert!(
            doc.metadata
                .iter()
                .any(|(k, v)| k == "title" && v == "My Document")
        );
        assert!(
            doc.metadata
                .iter()
                .any(|(k, v)| k == "author" && v == "Jane Doe")
        );
    }

    #[test]
    fn test_parse_blockquote() {
        let doc = parse_ok("#+BEGIN_QUOTE\nSome quoted text\n#+END_QUOTE");
        assert!(matches!(doc.blocks[0], Block::Blockquote { .. }));
    }

    #[test]
    fn test_parse_horizontal_rule() {
        let doc = parse_ok("-----");
        assert!(matches!(doc.blocks[0], Block::HorizontalRule));
    }

    #[test]
    fn test_parse_link() {
        let doc = parse_ok("[[https://example.com][click here]]");
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        let Inline::Link { url, .. } = &inlines[0] else {
            panic!("expected link");
        };
        assert_eq!(url, "https://example.com");
    }

    #[test]
    fn test_parse_code_inline() {
        let doc = parse_ok("Some =verbatim= text");
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Code(_))));
    }

    // ── Builder tests ─────────────────────────────────────────────────────────

    fn build_str(doc: &OrgDoc) -> String {
        build(doc).output
    }

    fn simple_doc(block: Block) -> OrgDoc {
        OrgDoc {
            blocks: vec![block],
            metadata: vec![],
        }
    }

    #[test]
    fn test_build_paragraph() {
        let doc = simple_doc(Block::Paragraph {
            inlines: vec![Inline::Text("Hello, world!".into())],
        });
        assert!(build_str(&doc).contains("Hello, world!"));
    }

    #[test]
    fn test_build_heading() {
        let doc = simple_doc(Block::Heading {
            level: 1,
            inlines: vec![Inline::Text("Main Title".into())],
        });
        assert!(build_str(&doc).contains("* Main Title"));
    }

    #[test]
    fn test_build_heading_levels() {
        let doc = OrgDoc {
            blocks: vec![
                Block::Heading {
                    level: 1,
                    inlines: vec![Inline::Text("Level 1".into())],
                },
                Block::Heading {
                    level: 2,
                    inlines: vec![Inline::Text("Level 2".into())],
                },
                Block::Heading {
                    level: 3,
                    inlines: vec![Inline::Text("Level 3".into())],
                },
            ],
            metadata: vec![],
        };
        let out = build_str(&doc);
        assert!(out.contains("* Level 1"));
        assert!(out.contains("** Level 2"));
        assert!(out.contains("*** Level 3"));
    }

    #[test]
    fn test_build_emphasis() {
        let doc = simple_doc(Block::Paragraph {
            inlines: vec![Inline::Italic(vec![Inline::Text("italic".into())])],
        });
        assert!(build_str(&doc).contains("/italic/"));
    }

    #[test]
    fn test_build_strong() {
        let doc = simple_doc(Block::Paragraph {
            inlines: vec![Inline::Bold(vec![Inline::Text("bold".into())])],
        });
        assert!(build_str(&doc).contains("*bold*"));
    }

    #[test]
    fn test_build_link() {
        let doc = simple_doc(Block::Paragraph {
            inlines: vec![Inline::Link {
                url: "https://example.com".into(),
                children: vec![Inline::Text("click".into())],
            }],
        });
        assert!(build_str(&doc).contains("[[https://example.com][click]]"));
    }

    #[test]
    fn test_build_unordered_list() {
        let doc = simple_doc(Block::List {
            ordered: false,
            items: vec![
                ListItem {
                    children: vec![ListItemContent::Inline(vec![Inline::Text("item 1".into())])],
                },
                ListItem {
                    children: vec![ListItemContent::Inline(vec![Inline::Text("item 2".into())])],
                },
            ],
        });
        let out = build_str(&doc);
        assert!(out.contains("- item 1"));
        assert!(out.contains("- item 2"));
    }

    #[test]
    fn test_build_ordered_list() {
        let doc = simple_doc(Block::List {
            ordered: true,
            items: vec![
                ListItem {
                    children: vec![ListItemContent::Inline(vec![Inline::Text("first".into())])],
                },
                ListItem {
                    children: vec![ListItemContent::Inline(vec![Inline::Text("second".into())])],
                },
            ],
        });
        let out = build_str(&doc);
        assert!(out.contains("1. first"));
        assert!(out.contains("2. second"));
    }

    #[test]
    fn test_build_code_block() {
        let doc = simple_doc(Block::CodeBlock {
            language: Some("rust".into()),
            content: "fn main() {}".into(),
        });
        let out = build_str(&doc);
        assert!(out.contains("#+BEGIN_SRC rust"));
        assert!(out.contains("fn main() {}"));
        assert!(out.contains("#+END_SRC"));
    }

    #[test]
    fn test_build_blockquote() {
        let doc = simple_doc(Block::Blockquote {
            children: vec![Block::Paragraph {
                inlines: vec![Inline::Text("A quote".into())],
            }],
        });
        let out = build_str(&doc);
        assert!(out.contains("#+BEGIN_QUOTE"));
        assert!(out.contains("A quote"));
        assert!(out.contains("#+END_QUOTE"));
    }

    #[test]
    fn test_build_image() {
        let doc = simple_doc(Block::Paragraph {
            inlines: vec![Inline::Image {
                url: "test.png".into(),
            }],
        });
        assert!(build_str(&doc).contains("[[file:test.png]]"));
    }

    #[test]
    fn test_build_inline_code() {
        let doc = simple_doc(Block::Paragraph {
            inlines: vec![Inline::Code("inline code".into())],
        });
        assert!(build_str(&doc).contains("=inline code="));
    }

    #[test]
    fn test_build_metadata() {
        let doc = OrgDoc {
            blocks: vec![],
            metadata: vec![("title".into(), "My Doc".into())],
        };
        assert!(build_str(&doc).contains("#+TITLE: My Doc"));
    }

    #[test]
    fn test_build_horizontal_rule() {
        let doc = simple_doc(Block::HorizontalRule);
        assert!(build_str(&doc).contains("-----"));
    }

    #[test]
    fn test_parse_unknown_block_warning() {
        let result = parse("#+BEGIN_FOOBAR\ncontent\n#+END_FOOBAR").unwrap();
        assert!(!result.warnings.is_empty());
        assert!(result.warnings[0].kind.contains("FOOBAR"));
    }
}
