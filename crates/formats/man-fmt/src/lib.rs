//! Man page (roff/troff) parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-man` and `rescribe-write-man` as thin adapter layers.

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct ManError(pub String);

impl std::fmt::Display for ManError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "man error: {}", self.0)
    }
}

impl std::error::Error for ManError {}

// ── AST ───────────────────────────────────────────────────────────────────────

/// A parsed man page document.
#[derive(Debug, Clone, Default)]
pub struct ManDoc {
    pub title: Option<String>,
    pub section: Option<String>,
    pub blocks: Vec<Block>,
}

/// Block-level element.
#[derive(Debug, Clone)]
pub enum Block {
    Heading {
        level: u8,
        inlines: Vec<Inline>,
    },
    Paragraph {
        inlines: Vec<Inline>,
    },
    CodeBlock {
        content: String,
    },
    List {
        ordered: bool,
        items: Vec<Vec<Block>>,
    },
    DefinitionList {
        items: Vec<(Vec<Inline>, Vec<Block>)>,
    },
    HorizontalRule,
}

/// Inline element.
#[derive(Debug, Clone)]
pub enum Inline {
    Text(String),
    Bold(Vec<Inline>),
    Italic(Vec<Inline>),
    Link { url: String, children: Vec<Inline> },
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Parse a man page string into a [`ManDoc`].
pub fn parse(input: &str) -> Result<ManDoc, ManError> {
    let mut p = Parser::new(input);
    p.parse_document()
}

type ParseResult = (Option<String>, Option<String>, Option<Block>);

struct Parser<'a> {
    lines: Vec<&'a str>,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        let lines: Vec<&str> = input.lines().collect();
        Self { lines, pos: 0 }
    }

    fn parse_document(&mut self) -> Result<ManDoc, ManError> {
        let mut title = None;
        let mut section = None;
        let mut blocks = Vec::new();

        while self.pos < self.lines.len() {
            let (t, s, block) = self.parse_element()?;
            if let Some(new_title) = t {
                title = Some(new_title);
            }
            if let Some(new_section) = s {
                section = Some(new_section);
            }
            if let Some(block) = block {
                blocks.push(block);
            }
        }

        Ok(ManDoc {
            title,
            section,
            blocks,
        })
    }

    fn current_line(&self) -> Option<&'a str> {
        self.lines.get(self.pos).copied()
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn parse_element(&mut self) -> Result<ParseResult, ManError> {
        let line = match self.current_line() {
            Some(l) => l,
            None => return Ok((None, None, None)),
        };

        // Skip empty lines and comments
        if line.is_empty() {
            self.advance();
            return Ok((None, None, None));
        }
        if line.starts_with(".\\\"") || line.starts_with("'\\\"") {
            self.advance();
            return Ok((None, None, None));
        }

        // Macro lines start with .
        if line.starts_with('.') {
            return self.parse_macro();
        }

        // Plain text paragraph
        let block = self.parse_text_block()?;
        Ok((None, None, block))
    }

    fn parse_macro(&mut self) -> Result<ParseResult, ManError> {
        let line = match self.current_line() {
            Some(l) => l,
            None => return Ok((None, None, None)),
        };
        self.advance();

        let (macro_name, args) = self.parse_macro_line(line);

        match macro_name.as_str() {
            // Title header
            "TH" => {
                let title = args.first().cloned();
                let section = args.get(1).cloned();
                let block = title.as_ref().map(|t| Block::Heading {
                    level: 1,
                    inlines: vec![Inline::Text(t.clone())],
                });
                Ok((title, section, block))
            }

            // Section heading
            "SH" => {
                let text = args.join(" ");
                let inlines = self.parse_inline_text(&text);
                Ok((None, None, Some(Block::Heading { level: 2, inlines })))
            }

            // Subsection heading
            "SS" => {
                let text = args.join(" ");
                let inlines = self.parse_inline_text(&text);
                Ok((None, None, Some(Block::Heading { level: 3, inlines })))
            }

            // Paragraph break
            "PP" | "P" | "LP" => {
                let block = self.parse_paragraph()?;
                Ok((None, None, block))
            }

            // Indented paragraph / definition list
            "IP" | "TP" => {
                let tag = if macro_name == "TP" {
                    // Tag is on the next line
                    self.current_line().map(|l| {
                        self.advance();
                        l.to_string()
                    })
                } else {
                    args.first().cloned()
                };

                // Content follows
                let content = self.collect_paragraph_text();
                let content_inline = self.parse_inline_text(&content);

                let block = if let Some(tag) = tag {
                    let tag_inlines = self.parse_inline_text(&tag);
                    let content_block = Block::Paragraph {
                        inlines: content_inline,
                    };
                    Block::DefinitionList {
                        items: vec![(tag_inlines, vec![content_block])],
                    }
                } else {
                    Block::Paragraph {
                        inlines: content_inline,
                    }
                };

                Ok((None, None, Some(block)))
            }

            // Relative indent start/end
            "RS" | "RE" => {
                // Skip these for now, they affect indentation
                Ok((None, None, None))
            }

            // No-fill (preformatted)
            "nf" => {
                let block = self.parse_preformatted()?;
                Ok((None, None, Some(block)))
            }

            // Bold text
            "B" => {
                let text = args.join(" ");
                let block = Block::Paragraph {
                    inlines: vec![Inline::Bold(vec![Inline::Text(text)])],
                };
                Ok((None, None, Some(block)))
            }

            // Italic text
            "I" => {
                let text = args.join(" ");
                let block = Block::Paragraph {
                    inlines: vec![Inline::Italic(vec![Inline::Text(text)])],
                };
                Ok((None, None, Some(block)))
            }

            // Bold-Roman alternation
            "BR" => {
                let block = self.parse_alternating(&args, true)?;
                Ok((None, None, Some(block)))
            }

            // Italic-Roman alternation
            "IR" => {
                let block = self.parse_alternating(&args, false)?;
                Ok((None, None, Some(block)))
            }

            // Roman-Bold alternation
            "RB" => {
                let block = self.parse_alternating(&args, true)?;
                Ok((None, None, Some(block)))
            }

            // Roman-Italic alternation
            "RI" => {
                let block = self.parse_alternating(&args, false)?;
                Ok((None, None, Some(block)))
            }

            // Bold-Italic alternation
            "BI" => {
                let mut inlines = Vec::new();
                let mut is_bold = true;
                for arg in &args {
                    if is_bold {
                        inlines.push(Inline::Bold(vec![Inline::Text(arg.clone())]));
                    } else {
                        inlines.push(Inline::Italic(vec![Inline::Text(arg.clone())]));
                    }
                    is_bold = !is_bold;
                }
                Ok((None, None, Some(Block::Paragraph { inlines })))
            }

            // Italic-Bold alternation
            "IB" => {
                let mut inlines = Vec::new();
                let mut is_italic = true;
                for arg in &args {
                    if is_italic {
                        inlines.push(Inline::Italic(vec![Inline::Text(arg.clone())]));
                    } else {
                        inlines.push(Inline::Bold(vec![Inline::Text(arg.clone())]));
                    }
                    is_italic = !is_italic;
                }
                Ok((None, None, Some(Block::Paragraph { inlines })))
            }

            // URL (groff extension)
            "URL" | "UR" => {
                let url = args.first().cloned().unwrap_or_default();
                let text = args.get(1).cloned().unwrap_or_else(|| url.clone());
                let block = Block::Paragraph {
                    inlines: vec![Inline::Link {
                        url,
                        children: vec![Inline::Text(text)],
                    }],
                };
                Ok((None, None, Some(block)))
            }

            // End URL
            "UE" => Ok((None, None, None)),

            // Horizontal rule / break
            "sp" => Ok((None, None, Some(Block::HorizontalRule))),

            // Other macros - ignore
            _ => Ok((None, None, None)),
        }
    }

    fn parse_macro_line(&self, line: &str) -> (String, Vec<String>) {
        let line = &line[1..]; // Skip leading .
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut in_quote = false;

        for c in line.chars() {
            match c {
                '"' => {
                    in_quote = !in_quote;
                }
                ' ' | '\t' if !in_quote => {
                    if !current.is_empty() {
                        parts.push(current);
                        current = String::new();
                    }
                }
                _ => {
                    current.push(c);
                }
            }
        }
        if !current.is_empty() {
            parts.push(current);
        }

        let macro_name = parts.first().cloned().unwrap_or_default();
        let args = parts.into_iter().skip(1).collect();

        (macro_name, args)
    }

    fn parse_paragraph(&mut self) -> Result<Option<Block>, ManError> {
        let text = self.collect_paragraph_text();
        if text.is_empty() {
            return Ok(None);
        }
        let inlines = self.parse_inline_text(&text);
        Ok(Some(Block::Paragraph { inlines }))
    }

    fn collect_paragraph_text(&mut self) -> String {
        let mut lines = Vec::new();

        while let Some(line) = self.current_line() {
            // Stop at macro lines or empty lines
            if line.is_empty() || line.starts_with('.') {
                break;
            }
            lines.push(line);
            self.advance();
        }

        lines.join(" ")
    }

    fn parse_text_block(&mut self) -> Result<Option<Block>, ManError> {
        let text = self.collect_paragraph_text();
        if text.is_empty() {
            return Ok(None);
        }
        let inlines = self.parse_inline_text(&text);
        Ok(Some(Block::Paragraph { inlines }))
    }

    fn parse_preformatted(&mut self) -> Result<Block, ManError> {
        let mut lines = Vec::new();

        while let Some(line) = self.current_line() {
            if line == ".fi" {
                self.advance();
                break;
            }
            // Skip macro lines inside preformatted
            if !line.starts_with('.') {
                lines.push(line);
            }
            self.advance();
        }

        let content = lines.join("\n");
        Ok(Block::CodeBlock { content })
    }

    fn parse_alternating(&mut self, args: &[String], bold_first: bool) -> Result<Block, ManError> {
        let mut inlines = Vec::new();
        let mut use_style = bold_first;

        for arg in args {
            if use_style {
                let inline = if bold_first {
                    Inline::Bold(vec![Inline::Text(arg.clone())])
                } else {
                    Inline::Italic(vec![Inline::Text(arg.clone())])
                };
                inlines.push(inline);
            } else {
                inlines.push(Inline::Text(arg.clone()));
            }
            use_style = !use_style;
        }

        Ok(Block::Paragraph { inlines })
    }

    fn parse_inline_text(&self, text: &str) -> Vec<Inline> {
        let mut inlines = Vec::new();
        let mut current = String::new();
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // Font escape: \fX or \f(XX
            if i + 2 < chars.len() && chars[i] == '\\' && chars[i + 1] == 'f' {
                if !current.is_empty() {
                    inlines.push(Inline::Text(current.clone()));
                    current.clear();
                }

                let font_char = chars[i + 2];
                i += 3;

                // Find the text until next font change or end
                let mut styled_text = String::new();
                while i < chars.len() {
                    if i + 2 < chars.len() && chars[i] == '\\' && chars[i + 1] == 'f' {
                        break;
                    }
                    styled_text.push(chars[i]);
                    i += 1;
                }

                if !styled_text.is_empty() {
                    let styled_inline = match font_char {
                        'B' => Inline::Bold(vec![Inline::Text(styled_text)]),
                        'I' => Inline::Italic(vec![Inline::Text(styled_text)]),
                        'R' | 'P' => Inline::Text(styled_text),
                        _ => Inline::Text(styled_text),
                    };
                    inlines.push(styled_inline);
                }
                continue;
            }

            // Other escapes
            if i + 1 < chars.len() && chars[i] == '\\' {
                match chars[i + 1] {
                    '-' => current.push('-'),
                    '\\' => current.push('\\'),
                    'e' => current.push('\\'),
                    '&' => {} // Zero-width space, ignore
                    _ => {
                        current.push(chars[i]);
                        current.push(chars[i + 1]);
                    }
                }
                i += 2;
                continue;
            }

            current.push(chars[i]);
            i += 1;
        }

        if !current.is_empty() {
            inlines.push(Inline::Text(current));
        }

        inlines
    }
}

// ── Builder ───────────────────────────────────────────────────────────────────

/// Build a man page string from a [`ManDoc`].
pub fn build(doc: &ManDoc) -> String {
    let mut ctx = BuildContext::new();

    // Write title header
    let title = doc.title.as_deref().unwrap_or("UNTITLED");
    let section = doc.section.as_deref().unwrap_or("1");
    ctx.write(&format!(".TH {} {} ", title.to_uppercase(), section));
    ctx.write("\"\" \"\" \"\"\n");

    for block in &doc.blocks {
        build_block(block, &mut ctx);
    }

    ctx.output
}

struct BuildContext {
    output: String,
}

impl BuildContext {
    fn new() -> Self {
        Self {
            output: String::new(),
        }
    }

    fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn newline(&mut self) {
        if !self.output.ends_with('\n') {
            self.write("\n");
        }
    }
}

fn build_block(block: &Block, ctx: &mut BuildContext) {
    match block {
        Block::Heading { level, inlines } => {
            ctx.newline();

            // Level 1 is document title (already handled), 2 is .SH, 3+ is .SS
            let macro_name = if *level <= 2 { ".SH" } else { ".SS" };
            ctx.write(macro_name);
            ctx.write(" ");

            // Emit text in uppercase for sections
            let text = extract_text(inlines);
            ctx.write(&text.to_uppercase());
            ctx.write("\n");
        }

        Block::Paragraph { inlines } => {
            ctx.newline();
            ctx.write(".PP\n");
            build_inlines(inlines, ctx);
            ctx.write("\n");
        }

        Block::CodeBlock { content } => {
            ctx.newline();
            ctx.write(".nf\n");
            for line in content.lines() {
                // Lines starting with . need escaping
                if line.starts_with('.') {
                    ctx.write("\\&");
                }
                ctx.write(line);
                ctx.write("\n");
            }
            ctx.write(".fi\n");
        }

        Block::List { ordered, items } => {
            ctx.newline();
            for (i, item_blocks) in items.iter().enumerate() {
                if *ordered {
                    ctx.write(&format!(".IP {}.\n", i + 1));
                } else {
                    ctx.write(".IP \\(bu\n");
                }
                for block in item_blocks {
                    if let Block::Paragraph { inlines } = block {
                        build_inlines(inlines, ctx);
                        ctx.write("\n");
                    } else {
                        build_block(block, ctx);
                    }
                }
            }
        }

        Block::DefinitionList { items } => {
            for (term_inlines, content_blocks) in items {
                ctx.newline();
                ctx.write(".TP\n");
                build_inlines(term_inlines, ctx);
                ctx.write("\n");
                for content_block in content_blocks {
                    if let Block::Paragraph { inlines } = content_block {
                        build_inlines(inlines, ctx);
                        ctx.write("\n");
                    } else {
                        build_block(content_block, ctx);
                    }
                }
            }
        }

        Block::HorizontalRule => {
            ctx.newline();
            ctx.write(".sp\n");
        }
    }
}

fn build_inlines(inlines: &[Inline], ctx: &mut BuildContext) {
    for inline in inlines {
        build_inline(inline, ctx);
    }
}

fn build_inline(inline: &Inline, ctx: &mut BuildContext) {
    match inline {
        Inline::Text(s) => {
            let escaped = escape_man(s);
            ctx.write(&escaped);
        }

        Inline::Bold(children) => {
            ctx.write("\\fB");
            build_inlines(children, ctx);
            ctx.write("\\fR");
        }

        Inline::Italic(children) => {
            ctx.write("\\fI");
            build_inlines(children, ctx);
            ctx.write("\\fR");
        }

        Inline::Link { url, children } => {
            build_inlines(children, ctx);
            ctx.write(" (");
            ctx.write(&escape_man(url));
            ctx.write(")");
        }
    }
}

fn escape_man(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => result.push_str("\\\\"),
            '-' => result.push_str("\\-"),
            _ => result.push(c),
        }
    }
    result
}

fn extract_text(inlines: &[Inline]) -> String {
    let mut text = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(s) => text.push_str(s),
            Inline::Bold(children) | Inline::Italic(children) | Inline::Link { children, .. } => {
                text.push_str(&extract_text(children));
            }
        }
    }
    text
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_title() {
        let doc = parse(".TH TEST 1 \"2024-01-01\" \"Version 1.0\"").unwrap();
        assert_eq!(doc.title, Some("TEST".to_string()));
        assert_eq!(doc.section, Some("1".to_string()));
    }

    #[test]
    fn test_parse_sections() {
        let doc = parse(".SH NAME\ntest \\- a test program\n.SH SYNOPSIS\ntest [options]").unwrap();
        assert_eq!(doc.blocks.len(), 4); // 2 headings + 2 paragraphs
    }

    #[test]
    fn test_parse_bold() {
        let doc = parse(".B bold text").unwrap();
        let block = &doc.blocks[0];
        assert!(matches!(block, Block::Paragraph { .. }));
        if let Block::Paragraph { inlines } = block {
            assert!(inlines.iter().any(|i| matches!(i, Inline::Bold(_))));
        }
    }

    #[test]
    fn test_parse_italic() {
        let doc = parse(".I italic text").unwrap();
        let block = &doc.blocks[0];
        assert!(matches!(block, Block::Paragraph { .. }));
        if let Block::Paragraph { inlines } = block {
            assert!(inlines.iter().any(|i| matches!(i, Inline::Italic(_))));
        }
    }

    #[test]
    fn test_parse_preformatted() {
        let doc = parse(".nf\ncode line 1\ncode line 2\n.fi").unwrap();
        let block = &doc.blocks[0];
        assert!(matches!(block, Block::CodeBlock { .. }));
    }

    #[test]
    fn test_parse_inline_font() {
        let doc = parse("This is \\fBbold\\fR text").unwrap();
        let block = &doc.blocks[0];
        if let Block::Paragraph { inlines } = block {
            // Should have multiple inlines
            assert!(inlines.len() >= 2);
        }
    }

    #[test]
    fn test_build_basic() {
        let doc = ManDoc {
            title: Some("TEST".to_string()),
            section: Some("1".to_string()),
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Hello, world!".to_string())],
            }],
        };
        let output = build(&doc);
        assert!(output.contains(".TH"));
        assert!(output.contains(".PP"));
        assert!(output.contains("Hello, world!"));
    }

    #[test]
    fn test_build_heading() {
        let doc = ManDoc {
            title: None,
            section: None,
            blocks: vec![Block::Heading {
                level: 2,
                inlines: vec![Inline::Text("Section Title".to_string())],
            }],
        };
        let output = build(&doc);
        assert!(output.contains(".SH SECTION TITLE"));
    }

    #[test]
    fn test_build_bold() {
        let doc = ManDoc {
            title: None,
            section: None,
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Bold(vec![Inline::Text("bold".to_string())])],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("\\fBbold\\fR"));
    }

    #[test]
    fn test_build_italic() {
        let doc = ManDoc {
            title: None,
            section: None,
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Italic(vec![Inline::Text("italic".to_string())])],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("\\fIitalic\\fR"));
    }

    #[test]
    fn test_build_link() {
        let doc = ManDoc {
            title: None,
            section: None,
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "https://example.com".to_string(),
                    children: vec![Inline::Text("Example".to_string())],
                }],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("Example"));
        assert!(output.contains("https://example.com"));
    }
}
