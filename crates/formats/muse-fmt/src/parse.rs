//! Muse parser.

use crate::ast::{Block, Diagnostic, Inline, MuseDoc, Span, TableRow};

// ── Public API ────────────────────────────────────────────────────────────────

/// Parse a Muse string into a [`MuseDoc`], returning any diagnostics.
///
/// This function is infallible — malformed input produces diagnostics, not errors.
pub(crate) fn parse(input: &str) -> (MuseDoc, Vec<Diagnostic>) {
    let mut p = Parser::new(input);
    let blocks = p.parse();
    let doc = MuseDoc {
        blocks,
        span: Span::new(0, input.len()),
        title: p.title,
        author: p.author,
        date: p.date,
        description: p.description,
        keywords: p.keywords,
    };
    (doc, p.diagnostics)
}

/// Parse `input` as a run of top-level blocks **without** the document-header
/// (`#title`/`#author`/…) phase — i.e. skip straight to
/// [`Parser::parse_block_loop`].
///
/// Used by [`crate::batch::StreamingParser`] to re-parse a single already
/// block-boundary-delimited chunk of the document in isolation. Header
/// directives are only meaningful at the very start of the whole document;
/// [`crate::batch::StreamingParser`] handles that phase itself and must
/// never let a later block whose first line happens to start with `#` be
/// misinterpreted as a header directive by a fresh header-directive scan.
pub(crate) fn parse_blocks(input: &str) -> (Vec<Block>, Vec<Diagnostic>) {
    let mut p = Parser::new(input);
    let blocks = p.parse_block_loop();
    (blocks, p.diagnostics)
}

// ── Parser ────────────────────────────────────────────────────────────────────

struct Parser<'a> {
    lines: Vec<&'a str>,
    pos: usize,
    pub diagnostics: Vec<Diagnostic>,
    /// Byte offset of the start of each line.
    line_offsets: Vec<usize>,
    // Document header directives
    title: Option<String>,
    author: Option<String>,
    date: Option<String>,
    description: Option<String>,
    keywords: Option<String>,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        let lines: Vec<&'a str> = input.lines().collect();
        // Compute byte offset of each line start.
        let mut offsets = Vec::with_capacity(lines.len());
        let mut offset = 0usize;
        for line in &lines {
            offsets.push(offset);
            offset += line.len() + 1; // +1 for the newline
        }
        Self {
            lines,
            pos: 0,
            diagnostics: Vec::new(),
            line_offsets: offsets,
            title: None,
            author: None,
            date: None,
            description: None,
            keywords: None,
        }
    }

    fn line_start(&self, idx: usize) -> usize {
        self.line_offsets.get(idx).copied().unwrap_or(0)
    }

    fn line_span(&self, idx: usize) -> Span {
        let start = self.line_start(idx);
        let end = start + self.lines.get(idx).map(|l| l.len()).unwrap_or(0);
        Span::new(start, end)
    }

    fn parse(&mut self) -> Vec<Block> {
        // Parse document header directives at the top
        self.parse_header_directives();
        self.parse_block_loop()
    }

    /// The main block-dispatch loop, **excluding** the document-header
    /// phase. Every `if`/`else if` branch here mirrors a pure predicate
    /// function in the "Boundary predicates" section below
    /// ([`is_table_row`], [`heading_level`], [`is_over_leveled_heading`],
    /// [`is_horizontal_rule`], [`is_unordered_list_start`],
    /// [`is_ordered_list_item`], [`is_definition_list_line`],
    /// [`is_indented_code_start`], [`is_footnote_def_start`],
    /// [`tag_open_close`]) so [`crate::batch::StreamingParser`] can classify
    /// block boundaries without duplicating this dispatch order.
    fn parse_block_loop(&mut self) -> Vec<Block> {
        let mut nodes = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            if line.trim().is_empty() {
                self.pos += 1;
                continue;
            }

            // Comment line: ;; text
            if line.starts_with(";; ") || line == ";;" {
                nodes.push(self.parse_line_comment());
                continue;
            }

            // Footnote definition: [N] text at start of line
            if let Some(node) = self.try_parse_footnote_def() {
                nodes.push(node);
                continue;
            }

            // Table row: | cell | cell |
            if is_table_row(line) {
                nodes.push(self.parse_table());
                continue;
            }

            // Example block <example>...</example>
            if line.trim_start().starts_with("<example>") {
                nodes.push(self.parse_example_block());
                continue;
            }

            // Verse block <verse>...</verse>
            if line.trim_start().starts_with("<verse>") {
                nodes.push(self.parse_verse_block());
                continue;
            }

            // Quote block <quote>...</quote>
            if line.trim_start().starts_with("<quote>") {
                nodes.push(self.parse_quote_block());
                continue;
            }

            // Center block <center>...</center>
            if line.trim_start().starts_with("<center>") {
                nodes.push(self.parse_tag_block("center", 8));
                continue;
            }

            // Right block <right>...</right>
            if line.trim_start().starts_with("<right>") {
                nodes.push(self.parse_tag_block("right", 7));
                continue;
            }

            // Literal block <literal>...</literal>
            if line.trim_start().starts_with("<literal>") {
                nodes.push(self.parse_literal_block());
                continue;
            }

            // Src block <src ...>...</src>
            if line.trim_start().starts_with("<src") {
                nodes.push(self.parse_src_block());
                continue;
            }

            // Comment block <comment>...</comment>
            if line.trim_start().starts_with("<comment>") {
                nodes.push(self.parse_comment_block());
                continue;
            }

            // Unknown block tag — starts with '<' but isn't a known tag.
            // Must advance pos to avoid infinite loop (parse_paragraph would
            // immediately break on '<' without advancing).
            // Exception: inline tags (<anchor, <br, <sub, <sup) should fall
            // through to paragraph parsing.
            if line.trim_start().starts_with('<') && !is_inline_tag_line(line) {
                self.pos += 1;
                continue;
            }

            // Heading * to *****
            if let Some(node) = self.try_parse_heading(line) {
                nodes.push(node);
                self.pos += 1;
                continue;
            }

            // Over-leveled heading (6+ asterisks followed by space): consume
            // without parsing to avoid an infinite loop (parse_paragraph breaks
            // immediately on any `* ` prefix without advancing pos).
            if is_over_leveled_heading(line) {
                self.pos += 1;
                continue;
            }

            // Horizontal rule (4+ dashes)
            if is_horizontal_rule(line) {
                let span = self.line_span(self.pos);
                nodes.push(Block::HorizontalRule { span });
                self.pos += 1;
                continue;
            }

            // Unordered list (space before -)
            if is_unordered_list_start(line) {
                nodes.push(self.parse_unordered_list());
                continue;
            }

            // Ordered list (space before number)
            if is_ordered_list_item(line) {
                nodes.push(self.parse_ordered_list());
                continue;
            }

            // Definition list (term ::)
            if is_definition_list_line(line) {
                nodes.push(self.parse_definition_list());
                continue;
            }

            // Indented code block
            if is_indented_code_start(line) {
                nodes.push(self.parse_indented_code());
                continue;
            }

            // Regular paragraph
            nodes.push(self.parse_paragraph());
        }

        nodes
    }

    /// Parse `#directive value` lines at the top of the document.
    fn parse_header_directives(&mut self) {
        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            // Skip blank lines at the very top
            if line.trim().is_empty() {
                self.pos += 1;
                continue;
            }

            // Must start with #
            if !line.starts_with('#') {
                break;
            }

            if let Some(val) = line.strip_prefix("#title ") {
                self.title = Some(val.trim().to_string());
            } else if let Some(val) = line.strip_prefix("#author ") {
                self.author = Some(val.trim().to_string());
            } else if let Some(val) = line.strip_prefix("#date ") {
                self.date = Some(val.trim().to_string());
            } else if let Some(val) = line.strip_prefix("#desc ") {
                self.description = Some(val.trim().to_string());
            } else if let Some(val) = line.strip_prefix("#keywords ") {
                self.keywords = Some(val.trim().to_string());
            } else {
                // Unknown directive — stop header parsing
                break;
            }
            self.pos += 1;
        }
    }

    fn try_parse_heading(&self, line: &str) -> Option<Block> {
        // Muse headings: * to *****
        let level = heading_level(line)?;
        let level = level as usize;
        let content = line[level + 1..].trim();
        let span = self.line_span(self.pos);
        let inline_nodes = parse_inline(content, span.start + level + 1);

        Some(Block::Heading {
            level: level as u8,
            inlines: inline_nodes,
            span,
        })
    }

    fn parse_example_block(&mut self) -> Block {
        let start = self.line_start(self.pos);
        let mut content = String::new();
        let first_line = self.lines[self.pos];

        // Get content after <example> on same line
        if let Some(pos) = first_line.find("<example>") {
            let after = &first_line[pos + 9..];
            if let Some(end) = after.find("</example>") {
                content.push_str(&after[..end]);
                self.pos += 1;
                let span = Span::new(start, self.line_start(self.pos));
                return Block::CodeBlock { content, span };
            }
            if !after.trim().is_empty() {
                content.push_str(after.trim());
                content.push('\n');
            }
        }
        self.pos += 1;

        // Multi-line
        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            if line.contains("</example>") {
                if let Some(pos) = line.find("</example>") {
                    content.push_str(&line[..pos]);
                }
                self.pos += 1;
                break;
            }
            content.push_str(line);
            content.push('\n');
            self.pos += 1;
        }

        let end = self.line_start(self.pos);
        Block::CodeBlock {
            content: content.trim_end().to_string(),
            span: Span::new(start, end),
        }
    }

    fn parse_verse_block(&mut self) -> Block {
        let start = self.line_start(self.pos);
        let mut content = String::new();
        let first_line = self.lines[self.pos];

        if let Some(pos) = first_line.find("<verse>") {
            let after = &first_line[pos + 7..];
            if let Some(end) = after.find("</verse>") {
                content.push_str(&after[..end]);
                self.pos += 1;
                let span = Span::new(start, self.line_start(self.pos));
                let inline = parse_inline(&content, start);
                return Block::Verse {
                    children: vec![Block::Paragraph {
                        inlines: inline,
                        span,
                    }],
                    span,
                };
            }
            if !after.trim().is_empty() {
                content.push_str(after.trim());
                content.push('\n');
            }
        }
        self.pos += 1;

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            if line.contains("</verse>") {
                if let Some(pos) = line.find("</verse>") {
                    content.push_str(&line[..pos]);
                }
                self.pos += 1;
                break;
            }
            content.push_str(line);
            content.push('\n');
            self.pos += 1;
        }

        let end = self.line_start(self.pos);
        let span = Span::new(start, end);
        let inline = parse_inline(content.trim_end(), start);
        Block::Verse {
            children: vec![Block::Paragraph {
                inlines: inline,
                span,
            }],
            span,
        }
    }

    fn parse_quote_block(&mut self) -> Block {
        let start = self.line_start(self.pos);
        let mut content = String::new();
        let first_line = self.lines[self.pos];

        if let Some(pos) = first_line.find("<quote>") {
            let after = &first_line[pos + 7..];
            if let Some(end) = after.find("</quote>") {
                content.push_str(&after[..end]);
                self.pos += 1;
                let span = Span::new(start, self.line_start(self.pos));
                let inline = parse_inline(&content, start);
                return Block::Blockquote {
                    children: vec![Block::Paragraph {
                        inlines: inline,
                        span,
                    }],
                    span,
                };
            }
            if !after.trim().is_empty() {
                content.push_str(after.trim());
                content.push('\n');
            }
        }
        self.pos += 1;

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            if line.contains("</quote>") {
                if let Some(pos) = line.find("</quote>") {
                    content.push_str(&line[..pos]);
                }
                self.pos += 1;
                break;
            }
            content.push_str(line);
            content.push('\n');
            self.pos += 1;
        }

        let end = self.line_start(self.pos);
        let span = Span::new(start, end);
        let inline = parse_inline(content.trim_end(), start);
        Block::Blockquote {
            children: vec![Block::Paragraph {
                inlines: inline,
                span,
            }],
            span,
        }
    }

    /// Generic parser for `<center>` and `<right>` tag blocks.
    fn parse_tag_block(&mut self, tag: &str, _tag_len: usize) -> Block {
        let start = self.line_start(self.pos);
        let mut content = String::new();
        let first_line = self.lines[self.pos];
        let open_tag = format!("<{}>", tag);
        let close_tag = format!("</{}>", tag);

        if let Some(pos) = first_line.find(&open_tag) {
            let after = &first_line[pos + open_tag.len()..];
            if let Some(end) = after.find(&close_tag) {
                content.push_str(&after[..end]);
                self.pos += 1;
                let span = Span::new(start, self.line_start(self.pos));
                let inline = parse_inline(&content, start);
                let children = vec![Block::Paragraph {
                    inlines: inline,
                    span,
                }];
                return match tag {
                    "center" => Block::CenteredBlock { children, span },
                    "right" => Block::RightBlock { children, span },
                    _ => unreachable!(),
                };
            }
            if !after.trim().is_empty() {
                content.push_str(after.trim());
                content.push('\n');
            }
        }
        self.pos += 1;

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            if line.contains(&close_tag) {
                if let Some(pos) = line.find(&close_tag) {
                    content.push_str(&line[..pos]);
                }
                self.pos += 1;
                break;
            }
            content.push_str(line);
            content.push('\n');
            self.pos += 1;
        }

        let end = self.line_start(self.pos);
        let span = Span::new(start, end);
        let inline = parse_inline(content.trim_end(), start);
        let children = vec![Block::Paragraph {
            inlines: inline,
            span,
        }];
        match tag {
            "center" => Block::CenteredBlock { children, span },
            "right" => Block::RightBlock { children, span },
            _ => unreachable!(),
        }
    }

    fn parse_literal_block(&mut self) -> Block {
        let start = self.line_start(self.pos);
        let mut content = String::new();
        let first_line = self.lines[self.pos];

        if let Some(pos) = first_line.find("<literal>") {
            let after = &first_line[pos + 9..];
            if let Some(end) = after.find("</literal>") {
                content.push_str(&after[..end]);
                self.pos += 1;
                let span = Span::new(start, self.line_start(self.pos));
                return Block::LiteralBlock { content, span };
            }
            if !after.trim().is_empty() {
                content.push_str(after.trim());
                content.push('\n');
            }
        }
        self.pos += 1;

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            if line.contains("</literal>") {
                if let Some(pos) = line.find("</literal>") {
                    content.push_str(&line[..pos]);
                }
                self.pos += 1;
                break;
            }
            content.push_str(line);
            content.push('\n');
            self.pos += 1;
        }

        let end = self.line_start(self.pos);
        Block::LiteralBlock {
            content: content.trim_end().to_string(),
            span: Span::new(start, end),
        }
    }

    fn parse_src_block(&mut self) -> Block {
        let start = self.line_start(self.pos);
        let first_line = self.lines[self.pos];

        // Extract lang attribute from <src lang="...">
        let lang = extract_src_lang(first_line);

        // Find end of opening tag
        let after = if let Some(gt) = first_line.find('>') {
            &first_line[gt + 1..]
        } else {
            ""
        };

        let mut content = String::new();

        // Check for single-line
        if let Some(end) = after.find("</src>") {
            content.push_str(&after[..end]);
            self.pos += 1;
            let span = Span::new(start, self.line_start(self.pos));
            return Block::SrcBlock {
                lang,
                content,
                span,
            };
        }

        if !after.trim().is_empty() {
            content.push_str(after.trim());
            content.push('\n');
        }
        self.pos += 1;

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            if line.contains("</src>") {
                if let Some(pos) = line.find("</src>") {
                    content.push_str(&line[..pos]);
                }
                self.pos += 1;
                break;
            }
            content.push_str(line);
            content.push('\n');
            self.pos += 1;
        }

        let end = self.line_start(self.pos);
        Block::SrcBlock {
            lang,
            content: content.trim_end().to_string(),
            span: Span::new(start, end),
        }
    }

    fn parse_comment_block(&mut self) -> Block {
        let start = self.line_start(self.pos);
        let mut content = String::new();
        let first_line = self.lines[self.pos];

        if let Some(pos) = first_line.find("<comment>") {
            let after = &first_line[pos + 9..];
            if let Some(end) = after.find("</comment>") {
                content.push_str(&after[..end]);
                self.pos += 1;
                let span = Span::new(start, self.line_start(self.pos));
                return Block::Comment { content, span };
            }
            if !after.trim().is_empty() {
                content.push_str(after.trim());
                content.push('\n');
            }
        }
        self.pos += 1;

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            if line.contains("</comment>") {
                if let Some(pos) = line.find("</comment>") {
                    content.push_str(&line[..pos]);
                }
                self.pos += 1;
                break;
            }
            content.push_str(line);
            content.push('\n');
            self.pos += 1;
        }

        let end = self.line_start(self.pos);
        Block::Comment {
            content: content.trim_end().to_string(),
            span: Span::new(start, end),
        }
    }

    fn parse_line_comment(&mut self) -> Block {
        let start = self.line_start(self.pos);
        let line = self.lines[self.pos];
        let content = if line == ";;" {
            String::new()
        } else {
            line[3..].to_string()
        };
        self.pos += 1;
        let end = self.line_start(self.pos);
        Block::Comment {
            content,
            span: Span::new(start, end),
        }
    }

    fn try_parse_footnote_def(&mut self) -> Option<Block> {
        let line = self.lines[self.pos];
        // Pattern: [N] text at start of line
        if !is_footnote_def_start(line) {
            return None;
        }
        let close = line.find(']')?;
        let label = &line[1..close];
        let text = &line[close + 2..];
        let span = self.line_span(self.pos);
        let content = parse_inline(text, span.start + close + 2);
        self.pos += 1;
        Some(Block::FootnoteDef {
            label: label.to_string(),
            content,
            span,
        })
    }

    fn parse_table(&mut self) -> Block {
        let start = self.line_start(self.pos);
        let mut rows = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            if !is_table_row(line) {
                break;
            }
            let row_span = self.line_span(self.pos);
            let row = parse_table_row(line, row_span.start);
            rows.push(row);
            self.pos += 1;
        }

        let end = self.line_start(self.pos);
        Block::Table {
            rows,
            span: Span::new(start, end),
        }
    }

    fn parse_unordered_list(&mut self) -> Block {
        let start = self.line_start(self.pos);
        let mut items = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            if !line.starts_with(" - ") && !line.starts_with("  - ") {
                break;
            }

            let item_span = self.line_span(self.pos);
            let content = line.trim_start()[2..].trim();
            let inline_nodes =
                parse_inline(content, item_span.start + line.find(content).unwrap_or(0));
            items.push(vec![Block::Paragraph {
                inlines: inline_nodes,
                span: item_span,
            }]);
            self.pos += 1;
        }

        let end = self.line_start(self.pos);
        Block::List {
            ordered: false,
            items,
            span: Span::new(start, end),
        }
    }

    fn parse_ordered_list(&mut self) -> Block {
        let start = self.line_start(self.pos);
        let mut items = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            if !is_ordered_list_item(line) {
                break;
            }

            let item_span = self.line_span(self.pos);
            let trimmed = line.trim_start();
            if let Some(dot_pos) = trimmed.find(". ") {
                let content = &trimmed[dot_pos + 2..];
                let inline_nodes =
                    parse_inline(content, item_span.start + line.find(content).unwrap_or(0));
                items.push(vec![Block::Paragraph {
                    inlines: inline_nodes,
                    span: item_span,
                }]);
            }
            self.pos += 1;
        }

        let end = self.line_start(self.pos);
        Block::List {
            ordered: true,
            items,
            span: Span::new(start, end),
        }
    }

    fn parse_definition_list(&mut self) -> Block {
        let start = self.line_start(self.pos);
        let mut items = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            if !line.contains(" :: ") {
                break;
            }

            let item_span = self.line_span(self.pos);
            if let Some(sep_pos) = line.find(" :: ") {
                let term = &line[..sep_pos];
                let desc = &line[sep_pos + 4..];

                let term_inlines = parse_inline(term.trim(), item_span.start);
                let desc_block = Block::Paragraph {
                    inlines: parse_inline(desc.trim(), item_span.start + sep_pos + 4),
                    span: item_span,
                };

                items.push((term_inlines, vec![desc_block]));
            }
            self.pos += 1;
        }

        let end = self.line_start(self.pos);
        Block::DefinitionList {
            items,
            span: Span::new(start, end),
        }
    }

    fn parse_indented_code(&mut self) -> Block {
        let start = self.line_start(self.pos);
        let mut content = String::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            if !line.starts_with("  ") && !line.trim().is_empty() {
                break;
            }

            if let Some(stripped) = line.strip_prefix("  ") {
                content.push_str(stripped);
                content.push('\n');
            }
            self.pos += 1;
        }

        let end = self.line_start(self.pos);
        Block::CodeBlock {
            content: content.trim_end().to_string(),
            span: Span::new(start, end),
        }
    }

    fn parse_paragraph(&mut self) -> Block {
        let start = self.line_start(self.pos);
        let mut text = String::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            if line.trim().is_empty() {
                break;
            }

            // Comment lines break paragraphs
            if line.starts_with(";; ") || line == ";;" {
                break;
            }

            // Footnote definitions break paragraphs
            if is_footnote_def_start(line) {
                break;
            }

            // Table rows break paragraphs
            if is_table_row(line) {
                break;
            }

            // Check for block elements - but not **bold**
            // (any star-run followed by a space, over-leveled or not)
            let is_heading = heading_level(line).is_some() || is_over_leveled_heading(line);
            if is_heading
                // Note: intentionally `line.starts_with` (not `.trim().starts_with`,
                // unlike `is_horizontal_rule`/top-level dispatch) — mirrors the
                // original pre-refactor paragraph-break check exactly, including
                // its (pre-existing) inconsistency with top-level HR detection for
                // an indented "----" line.
                || line.starts_with("----")
                || is_unordered_list_start(line)
                || is_indented_code_start(line)
                || is_definition_list_line(line)
                || (line.trim_start().starts_with('<') && !is_inline_tag_line(line))
                || is_ordered_list_item(line)
            {
                break;
            }

            if !text.is_empty() {
                text.push(' ');
            }
            text.push_str(line.trim());
            self.pos += 1;
        }

        let end = self.line_start(self.pos);
        let span = Span::new(start, end);
        let inline_nodes = parse_inline(&text, start);
        Block::Paragraph {
            inlines: inline_nodes,
            span,
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Check if a line starting with `<` contains only inline tags, not block tags.
pub(crate) fn is_inline_tag_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("<anchor ")
        || trimmed.starts_with("<br>")
        || trimmed.starts_with("<br/>")
        || trimmed.starts_with("<sub>")
        || trimmed.starts_with("<sup>")
}

/// Check if a line is a table row (starts with `|` or `||`).
pub(crate) fn is_table_row(line: &str) -> bool {
    let trimmed = line.trim();
    (trimmed.starts_with('|') || trimmed.starts_with("||")) && (trimmed.ends_with('|'))
}

// ── Boundary predicates ─────────────────────────────────────────────────────
//
// Pure, `self`-free "does this line start/continue a block of kind X"
// predicates. `Parser::parse_block_loop` and `Parser::parse_paragraph` use
// these as the single source of truth for block-boundary decisions, and
// [`crate::batch::StreamingParser`] reuses the exact same functions to
// classify block boundaries incrementally, line by line, without
// duplicating (and risking drifting from) this parser's own dispatch logic.

/// Muse heading level (1..=5) if `line` is a well-formed heading start
/// (`*` to `*****` followed by a space), else `None`. Levels above 5 are
/// [`is_over_leveled_heading`], not this.
pub(crate) fn heading_level(line: &str) -> Option<u8> {
    let level = line.chars().take_while(|&c| c == '*').count();
    if level > 0 && level <= 5 && line.len() > level && line.chars().nth(level) == Some(' ') {
        Some(level as u8)
    } else {
        None
    }
}

/// True if `line` is a 6+ `*` heading-like line (over-leveled — consumed
/// without producing a heading node, to avoid an infinite loop rather than
/// falling through to paragraph parsing).
pub(crate) fn is_over_leveled_heading(line: &str) -> bool {
    let star_count = line.chars().take_while(|&c| c == '*').count();
    star_count > 5 && line.len() > star_count && line.chars().nth(star_count) == Some(' ')
}

/// True if `line` is a horizontal rule (4+ dashes, top-level dispatch
/// variant: leading whitespace is trimmed first).
pub(crate) fn is_horizontal_rule(line: &str) -> bool {
    line.trim().starts_with("----")
}

/// True if `line` starts (or continues) an unordered list item.
pub(crate) fn is_unordered_list_start(line: &str) -> bool {
    line.starts_with(" - ") || line.starts_with("  - ")
}

/// True if `line` starts (or continues) an ordered list item (`" 1. "`-style).
pub(crate) fn is_ordered_list_item(line: &str) -> bool {
    if line.starts_with(' ') {
        let trimmed = line.trim_start();
        if let Some(dot_pos) = trimmed.find(". ") {
            let num = &trimmed[..dot_pos];
            // Note: `all()` on an empty `num` is vacuously true (matches the
            // original `Parser::is_ordered_list_item` method exactly — not
            // fixing that pre-existing edge case here).
            return num.chars().all(|c| c.is_ascii_digit());
        }
    }
    false
}

/// True if `line` is a definition-list line (`term :: description`).
pub(crate) fn is_definition_list_line(line: &str) -> bool {
    line.contains(" :: ")
}

/// True if `line` starts (or continues) an indented code block (2+ leading
/// spaces, non-blank).
pub(crate) fn is_indented_code_start(line: &str) -> bool {
    line.starts_with("  ") && !line.trim().is_empty()
}

/// True if `line` is a well-formed footnote-definition start: `[N] text`
/// where `N` is one or more digits, immediately followed by `] ` at the
/// start of the line.
pub(crate) fn is_footnote_def_start(line: &str) -> bool {
    if !line.starts_with('[') {
        return false;
    }
    let Some(close) = line.find(']') else {
        return false;
    };
    let label = &line[1..close];
    if label.is_empty() || !label.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    close + 1 < line.len() && line.as_bytes()[close + 1] == b' '
}

/// If `line` (after trimming leading whitespace) opens one of Muse's
/// tag-delimited block kinds (`<example>`, `<verse>`, `<quote>`, `<center>`,
/// `<right>`, `<literal>`, `<src ...>`, `<comment>`), return that kind's
/// closing tag. Order mirrors `Parser::parse_block_loop`'s own dispatch
/// order exactly.
pub(crate) fn tag_open_close(line: &str) -> Option<&'static str> {
    let t = line.trim_start();
    if t.starts_with("<example>") {
        Some("</example>")
    } else if t.starts_with("<verse>") {
        Some("</verse>")
    } else if t.starts_with("<quote>") {
        Some("</quote>")
    } else if t.starts_with("<center>") {
        Some("</center>")
    } else if t.starts_with("<right>") {
        Some("</right>")
    } else if t.starts_with("<literal>") {
        Some("</literal>")
    } else if t.starts_with("<src") {
        Some("</src>")
    } else if t.starts_with("<comment>") {
        Some("</comment>")
    } else {
        None
    }
}

/// Parse a single table row line.
fn parse_table_row(line: &str, base_offset: usize) -> TableRow {
    let trimmed = line.trim();
    let header = trimmed.starts_with("||");

    // Strip outer delimiters
    let inner = if header {
        // || cell || cell ||  ->  strip leading || and trailing ||
        let s = trimmed.strip_prefix("||").unwrap_or(trimmed);
        let s = s.strip_suffix("||").unwrap_or(s);
        (s, "||")
    } else {
        let s = trimmed.strip_prefix('|').unwrap_or(trimmed);
        let s = s.strip_suffix('|').unwrap_or(s);
        (s, "|")
    };

    let (inner_str, delim) = inner;
    let cells: Vec<Vec<Inline>> = inner_str
        .split(delim)
        .map(|cell| parse_inline(cell.trim(), base_offset))
        .collect();

    TableRow { cells, header }
}

/// Extract `lang` from `<src lang="...">`.
fn extract_src_lang(line: &str) -> Option<String> {
    // Look for lang="..." or lang='...'
    let lang_pos = line.find("lang=")?;
    let after = &line[lang_pos + 5..];
    let quote = after.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let rest = &after[1..];
    let end = rest.find(quote)?;
    Some(rest[..end].to_string())
}

// ── Image detection ──────────────────────────────────────────────────────────

/// Check if a URL looks like an image path (by extension).
fn is_image_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".gif")
        || lower.ends_with(".svg")
        || lower.ends_with(".bmp")
        || lower.ends_with(".webp")
        || lower.ends_with(".ico")
}

// ── Inline parser ─────────────────────────────────────────────────────────────

pub(crate) fn parse_inline(text: &str, base_offset: usize) -> Vec<Inline> {
    let mut nodes = Vec::new();
    let mut current = String::new();
    let mut current_start = base_offset;
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Inline code =...=
        if chars[i] == '='
            && i + 1 < chars.len()
            && let Some((end, content)) = find_closing(&chars, i + 1, '=')
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(
                    current.clone(),
                    Span::new(current_start, base_offset + char_byte_offset(text, i)),
                ));
                current.clear();
            }
            let code_start = base_offset + char_byte_offset(text, i);
            let code_end = base_offset + char_byte_offset(text, end + 1);
            nodes.push(Inline::Code(content, Span::new(code_start, code_end)));
            i = end + 1;
            current_start = base_offset + char_byte_offset(text, i.min(chars.len()));
            continue;
        }

        // Strikethrough ~~...~~
        if chars[i] == '~'
            && i + 1 < chars.len()
            && chars[i + 1] == '~'
            && i + 2 < chars.len()
            && let Some((end, content)) = find_double_closing(&chars, i + 2, '~')
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(
                    current.clone(),
                    Span::new(current_start, base_offset + char_byte_offset(text, i)),
                ));
                current.clear();
            }
            let st_start = base_offset + char_byte_offset(text, i);
            let st_end = base_offset + char_byte_offset(text, (end + 2).min(chars.len()));
            let inner = parse_inline(&content, st_start + 2);
            nodes.push(Inline::Strikethrough(inner, Span::new(st_start, st_end)));
            i = end + 2;
            current_start = base_offset + char_byte_offset(text, i.min(chars.len()));
            continue;
        }

        // Bold **...** (doubled asterisks)
        if chars[i] == '*'
            && i + 1 < chars.len()
            && chars[i + 1] == '*'
            && i + 2 < chars.len()
            && let Some((end, content)) = find_double_closing(&chars, i + 2, '*')
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(
                    current.clone(),
                    Span::new(current_start, base_offset + char_byte_offset(text, i)),
                ));
                current.clear();
            }
            let bold_start = base_offset + char_byte_offset(text, i);
            let bold_end = base_offset + char_byte_offset(text, (end + 2).min(chars.len()));
            let inner = parse_inline(&content, bold_start + 2);
            nodes.push(Inline::Bold(inner, Span::new(bold_start, bold_end)));
            i = end + 2;
            current_start = base_offset + char_byte_offset(text, i.min(chars.len()));
            continue;
        }

        // Emphasis *...*
        if chars[i] == '*'
            && i + 1 < chars.len()
            && chars[i + 1] != '*'
            && (i == 0 || !chars[i - 1].is_alphanumeric())
            && let Some((end, content)) = find_closing(&chars, i + 1, '*')
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(
                    current.clone(),
                    Span::new(current_start, base_offset + char_byte_offset(text, i)),
                ));
                current.clear();
            }
            let em_start = base_offset + char_byte_offset(text, i);
            let em_end = base_offset + char_byte_offset(text, (end + 1).min(chars.len()));
            let inner = parse_inline(&content, em_start + 1);
            nodes.push(Inline::Italic(inner, Span::new(em_start, em_end)));
            i = end + 1;
            current_start = base_offset + char_byte_offset(text, i.min(chars.len()));
            continue;
        }

        // Underline _..._
        if chars[i] == '_'
            && i + 1 < chars.len()
            && chars[i + 1] != '_'
            && (i == 0 || !chars[i - 1].is_alphanumeric())
            && let Some((end, content)) = find_closing(&chars, i + 1, '_')
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(
                    current.clone(),
                    Span::new(current_start, base_offset + char_byte_offset(text, i)),
                ));
                current.clear();
            }
            let u_start = base_offset + char_byte_offset(text, i);
            let u_end = base_offset + char_byte_offset(text, (end + 1).min(chars.len()));
            let inner = parse_inline(&content, u_start + 1);
            nodes.push(Inline::Underline(inner, Span::new(u_start, u_end)));
            i = end + 1;
            current_start = base_offset + char_byte_offset(text, i.min(chars.len()));
            continue;
        }

        // Superscript ^...^
        if chars[i] == '^'
            && i + 1 < chars.len()
            && let Some((end, content)) = find_closing(&chars, i + 1, '^')
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(
                    current.clone(),
                    Span::new(current_start, base_offset + char_byte_offset(text, i)),
                ));
                current.clear();
            }
            let s_start = base_offset + char_byte_offset(text, i);
            let s_end = base_offset + char_byte_offset(text, (end + 1).min(chars.len()));
            let inner = parse_inline(&content, s_start + 1);
            nodes.push(Inline::Superscript(inner, Span::new(s_start, s_end)));
            i = end + 1;
            current_start = base_offset + char_byte_offset(text, i.min(chars.len()));
            continue;
        }

        // Inline HTML-like tags: <sub>...</sub>, <br>, <anchor name>
        if chars[i] == '<' {
            // <br> or <br/>
            if i + 3 < chars.len()
                && chars[i + 1] == 'b'
                && chars[i + 2] == 'r'
                && (chars[i + 3] == '>'
                    || (i + 4 < chars.len() && chars[i + 3] == '/' && chars[i + 4] == '>'))
            {
                if !current.is_empty() {
                    nodes.push(Inline::Text(
                        current.clone(),
                        Span::new(current_start, base_offset + char_byte_offset(text, i)),
                    ));
                    current.clear();
                }
                let br_start = base_offset + char_byte_offset(text, i);
                let advance = if i + 4 < chars.len() && chars[i + 3] == '/' {
                    5
                } else {
                    4
                };
                let br_end = base_offset + char_byte_offset(text, (i + advance).min(chars.len()));
                nodes.push(Inline::LineBreak(Span::new(br_start, br_end)));
                i += advance;
                current_start = base_offset + char_byte_offset(text, i.min(chars.len()));
                continue;
            }

            // <sub>...</sub>
            if text[char_byte_offset(text, i)..].starts_with("<sub>")
                && let Some(close) = text[char_byte_offset(text, i)..].find("</sub>")
            {
                if !current.is_empty() {
                    nodes.push(Inline::Text(
                        current.clone(),
                        Span::new(current_start, base_offset + char_byte_offset(text, i)),
                    ));
                    current.clear();
                }
                let sub_start = base_offset + char_byte_offset(text, i);
                let content_str =
                    &text[char_byte_offset(text, i) + 5..char_byte_offset(text, i) + close];
                let sub_end = base_offset + char_byte_offset(text, i) + close + 6;
                let inner = parse_inline(content_str, sub_start + 5);
                nodes.push(Inline::Subscript(inner, Span::new(sub_start, sub_end)));
                let bytes_consumed = close + 6;
                let consumed_text =
                    &text[char_byte_offset(text, i)..char_byte_offset(text, i) + bytes_consumed];
                let chars_consumed = consumed_text.chars().count();
                i += chars_consumed;
                current_start = base_offset + char_byte_offset(text, i.min(chars.len()));
                continue;
            }

            // <sup>...</sup>
            if text[char_byte_offset(text, i)..].starts_with("<sup>")
                && let Some(close) = text[char_byte_offset(text, i)..].find("</sup>")
            {
                if !current.is_empty() {
                    nodes.push(Inline::Text(
                        current.clone(),
                        Span::new(current_start, base_offset + char_byte_offset(text, i)),
                    ));
                    current.clear();
                }
                let sup_start = base_offset + char_byte_offset(text, i);
                let content_str =
                    &text[char_byte_offset(text, i) + 5..char_byte_offset(text, i) + close];
                let sup_end = base_offset + char_byte_offset(text, i) + close + 6;
                let inner = parse_inline(content_str, sup_start + 5);
                nodes.push(Inline::Superscript(inner, Span::new(sup_start, sup_end)));
                let bytes_consumed = close + 6;
                let consumed_text =
                    &text[char_byte_offset(text, i)..char_byte_offset(text, i) + bytes_consumed];
                let chars_consumed = consumed_text.chars().count();
                i += chars_consumed;
                current_start = base_offset + char_byte_offset(text, i.min(chars.len()));
                continue;
            }

            // <anchor name>
            if text[char_byte_offset(text, i)..].starts_with("<anchor ")
                && let Some(close) = text[char_byte_offset(text, i)..].find('>')
            {
                if !current.is_empty() {
                    nodes.push(Inline::Text(
                        current.clone(),
                        Span::new(current_start, base_offset + char_byte_offset(text, i)),
                    ));
                    current.clear();
                }
                let anchor_start = base_offset + char_byte_offset(text, i);
                let name = &text[char_byte_offset(text, i) + 8..char_byte_offset(text, i) + close];
                let anchor_end = base_offset + char_byte_offset(text, i) + close + 1;
                nodes.push(Inline::Anchor {
                    name: name.trim().to_string(),
                    span: Span::new(anchor_start, anchor_end),
                });
                let bytes_consumed = close + 1;
                let consumed_text =
                    &text[char_byte_offset(text, i)..char_byte_offset(text, i) + bytes_consumed];
                let chars_consumed = consumed_text.chars().count();
                i += chars_consumed;
                current_start = base_offset + char_byte_offset(text, i.min(chars.len()));
                continue;
            }
        }

        // Footnote reference [N] — must be a standalone [digits]
        if chars[i] == '['
            && let Some((end, label)) = try_parse_footnote_ref(&chars, i)
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(
                    current.clone(),
                    Span::new(current_start, base_offset + char_byte_offset(text, i)),
                ));
                current.clear();
            }
            let ref_start = base_offset + char_byte_offset(text, i);
            let ref_end = base_offset + char_byte_offset(text, end);
            nodes.push(Inline::FootnoteRef {
                label,
                span: Span::new(ref_start, ref_end),
            });
            i = end;
            current_start = base_offset + char_byte_offset(text, i.min(chars.len()));
            continue;
        }

        // Link [[url][text]] or [[url]]
        if chars[i] == '['
            && i + 1 < chars.len()
            && chars[i + 1] == '['
            && let Some((end, url, link_text)) = parse_muse_link(&chars, i)
        {
            if !current.is_empty() {
                nodes.push(Inline::Text(
                    current.clone(),
                    Span::new(current_start, base_offset + char_byte_offset(text, i)),
                ));
                current.clear();
            }
            let link_start = base_offset + char_byte_offset(text, i);
            let link_end = base_offset + char_byte_offset(text, end.min(chars.len()));

            // Check if URL is an image
            if is_image_url(&url) {
                let alt = if link_text == url {
                    None
                } else {
                    Some(link_text)
                };
                nodes.push(Inline::Image {
                    src: url,
                    alt,
                    span: Span::new(link_start, link_end),
                });
            } else {
                nodes.push(Inline::Link {
                    url,
                    children: vec![Inline::Text(link_text, Span::new(link_start, link_end))],
                    span: Span::new(link_start, link_end),
                });
            }
            i = end;
            current_start = base_offset + char_byte_offset(text, i.min(chars.len()));
            continue;
        }

        current.push(chars[i]);
        i += 1;
    }

    if !current.is_empty() {
        let end = base_offset + text.len();
        nodes.push(Inline::Text(current, Span::new(current_start, end)));
    }

    nodes
}

/// Try to parse `[digits]` as a footnote reference. Returns (end_index, label).
fn try_parse_footnote_ref(chars: &[char], start: usize) -> Option<(usize, String)> {
    if chars[start] != '[' {
        return None;
    }
    let mut i = start + 1;
    let mut label = String::new();
    while i < chars.len() && chars[i].is_ascii_digit() {
        label.push(chars[i]);
        i += 1;
    }
    if label.is_empty() || i >= chars.len() || chars[i] != ']' {
        return None;
    }
    // Make sure this isn't followed by another [ (which would be a link)
    if i + 1 < chars.len() && chars[i + 1] == '[' {
        return None;
    }
    Some((i + 1, label))
}

/// Get byte offset of the i-th character in `text`.
fn char_byte_offset(text: &str, char_idx: usize) -> usize {
    text.char_indices()
        .nth(char_idx)
        .map(|(b, _)| b)
        .unwrap_or(text.len())
}

fn find_closing(chars: &[char], start: usize, marker: char) -> Option<(usize, String)> {
    let mut i = start;
    let mut content = String::new();

    while i < chars.len() {
        if chars[i] == marker {
            return Some((i, content));
        }
        content.push(chars[i]);
        i += 1;
    }
    None
}

fn find_double_closing(chars: &[char], start: usize, marker: char) -> Option<(usize, String)> {
    let mut i = start;
    let mut content = String::new();

    while i + 1 < chars.len() {
        if chars[i] == marker && chars[i + 1] == marker {
            return Some((i, content));
        }
        content.push(chars[i]);
        i += 1;
    }
    None
}

fn parse_muse_link(chars: &[char], start: usize) -> Option<(usize, String, String)> {
    // [[url][text]] or [[url]]
    if start + 1 >= chars.len() || chars[start] != '[' || chars[start + 1] != '[' {
        return None;
    }

    let mut i = start + 2;
    let mut url = String::new();

    // Collect URL until ] or [
    while i < chars.len() && chars[i] != ']' && chars[i] != '[' {
        url.push(chars[i]);
        i += 1;
    }

    if i >= chars.len() {
        return None;
    }

    // Check for link text
    if chars[i] == ']' && i + 1 < chars.len() && chars[i + 1] == '[' {
        i += 2;
        let mut text = String::new();
        while i < chars.len() && chars[i] != ']' {
            text.push(chars[i]);
            i += 1;
        }
        if i + 1 < chars.len() && chars[i] == ']' && chars[i + 1] == ']' {
            return Some((i + 2, url, text));
        }
    } else if chars[i] == ']' && i + 1 < chars.len() && chars[i + 1] == ']' {
        // No link text, use URL
        return Some((i + 2, url.clone(), url));
    }

    None
}
