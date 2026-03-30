//! Man page parser.

use crate::ast::{Block, Diagnostic, Inline, ManDoc, Severity, Span};

// ── Public entry point ────────────────────────────────────────────────────────

/// Parse a man page string into a [`ManDoc`].  Infallible — errors become [`Diagnostic`]s.
pub fn parse(input: &str) -> (ManDoc, Vec<Diagnostic>) {
    let mut p = Parser::new(input);
    let doc = p.parse_document();
    (doc, p.diagnostics)
}

// ── Internal ──────────────────────────────────────────────────────────────────

type ParseElement = (Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<Block>);

struct Parser<'a> {
    lines: Vec<&'a str>,
    pos: usize,
    pub diagnostics: Vec<Diagnostic>,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        let lines: Vec<&str> = input.lines().collect();
        Self {
            lines,
            pos: 0,
            diagnostics: Vec::new(),
        }
    }

    fn parse_document(&mut self) -> ManDoc {
        let mut title = None;
        let mut section = None;
        let mut date = None;
        let mut source = None;
        let mut manual = None;
        let mut blocks = Vec::new();

        while self.pos < self.lines.len() {
            let (t, s, d, src, man, block) = self.parse_element();
            if let Some(new_title) = t {
                title = Some(new_title);
            }
            if let Some(new_section) = s {
                section = Some(new_section);
            }
            if let Some(new_date) = d {
                date = Some(new_date);
            }
            if let Some(new_source) = src {
                source = Some(new_source);
            }
            if let Some(new_manual) = man {
                manual = Some(new_manual);
            }
            if let Some(block) = block {
                blocks.push(block);
            }
        }

        ManDoc {
            title,
            section,
            date,
            source,
            manual,
            blocks,
            span: Span::NONE,
        }
    }

    fn current_line(&self) -> Option<&'a str> {
        self.lines.get(self.pos).copied()
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn parse_element(&mut self) -> ParseElement {
        let line = match self.current_line() {
            Some(l) => l,
            None => return (None, None, None, None, None, None),
        };

        // Skip empty lines
        if line.is_empty() {
            self.advance();
            return (None, None, None, None, None, None);
        }

        // Comments: .\" or '\"
        if line.starts_with(".\\\"") || line.starts_with("'\\\"") {
            let text = line[3..].trim().to_string();
            self.advance();
            return (None, None, None, None, None, Some(Block::Comment { text, span: Span::NONE }));
        }

        // Macro lines start with .
        if line.starts_with('.') {
            return self.parse_macro();
        }

        // Plain text paragraph
        let block = self.parse_text_block();
        (None, None, None, None, None, block)
    }

    fn parse_macro(&mut self) -> ParseElement {
        let line = match self.current_line() {
            Some(l) => l,
            None => return (None, None, None, None, None, None),
        };
        self.advance();

        let (macro_name, args) = self.parse_macro_line(line);

        match macro_name.as_str() {
            // Title header
            "TH" => {
                let title = args.first().cloned();
                let section = args.get(1).cloned();
                let date = args.get(2).cloned();
                let source = args.get(3).cloned();
                let manual = args.get(4).cloned();
                let block = title.as_ref().map(|t| Block::Heading {
                    level: 1,
                    inlines: vec![Inline::Text(t.clone(), Span::NONE)],
                    span: Span::NONE,
                });
                (title, section, date, source, manual, block)
            }

            // Section heading
            "SH" => {
                let text = args.join(" ");
                let inlines = self.parse_inline_text(&text);
                (
                    None,
                    None,
                    None,
                    None,
                    None,
                    Some(Block::Heading {
                        level: 2,
                        inlines,
                        span: Span::NONE,
                    }),
                )
            }

            // Subsection heading
            "SS" => {
                let text = args.join(" ");
                let inlines = self.parse_inline_text(&text);
                (
                    None,
                    None,
                    None,
                    None,
                    None,
                    Some(Block::Heading {
                        level: 3,
                        inlines,
                        span: Span::NONE,
                    }),
                )
            }

            // Paragraph break
            "PP" | "P" | "LP" => {
                let block = self.parse_paragraph();
                (None, None, None, None, None, block)
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
                        span: Span::NONE,
                    };
                    Block::DefinitionList {
                        items: vec![(tag_inlines, vec![content_block])],
                        span: Span::NONE,
                    }
                } else {
                    // .IP without a tag → IndentedParagraph
                    if content_inline.is_empty() {
                        return (None, None, None, None, None, None);
                    }
                    Block::IndentedParagraph {
                        inlines: content_inline,
                        span: Span::NONE,
                    }
                };

                (None, None, None, None, None, Some(block))
            }

            // Relative indent start/end
            "RS" | "RE" => {
                // Skip these for now, they affect indentation
                (None, None, None, None, None, None)
            }

            // No-fill (preformatted)
            "nf" => {
                let block = self.parse_preformatted();
                (None, None, None, None, None, Some(block))
            }

            // Example block (groff extension)
            "EX" => {
                let block = self.parse_example_block();
                (None, None, None, None, None, Some(block))
            }

            // Bold text
            "B" => {
                let text = args.join(" ");
                let block = Block::Paragraph {
                    inlines: vec![Inline::Bold(
                        vec![Inline::Text(text, Span::NONE)],
                        Span::NONE,
                    )],
                    span: Span::NONE,
                };
                (None, None, None, None, None, Some(block))
            }

            // Italic text
            "I" => {
                let text = args.join(" ");
                let block = Block::Paragraph {
                    inlines: vec![Inline::Italic(
                        vec![Inline::Text(text, Span::NONE)],
                        Span::NONE,
                    )],
                    span: Span::NONE,
                };
                (None, None, None, None, None, Some(block))
            }

            // Bold-Roman alternation
            "BR" | "RB" => {
                let block = self.parse_alternating(&args, true);
                (None, None, None, None, None, Some(block))
            }

            // Italic-Roman alternation
            "IR" | "RI" => {
                let block = self.parse_alternating(&args, false);
                (None, None, None, None, None, Some(block))
            }

            // Bold-Italic alternation
            "BI" => {
                let mut inlines = Vec::new();
                let mut is_bold = true;
                for arg in &args {
                    if is_bold {
                        inlines.push(Inline::Bold(
                            vec![Inline::Text(arg.clone(), Span::NONE)],
                            Span::NONE,
                        ));
                    } else {
                        inlines.push(Inline::Italic(
                            vec![Inline::Text(arg.clone(), Span::NONE)],
                            Span::NONE,
                        ));
                    }
                    is_bold = !is_bold;
                }
                (
                    None,
                    None,
                    None,
                    None,
                    None,
                    Some(Block::Paragraph {
                        inlines,
                        span: Span::NONE,
                    }),
                )
            }

            // Italic-Bold alternation
            "IB" => {
                let mut inlines = Vec::new();
                let mut is_italic = true;
                for arg in &args {
                    if is_italic {
                        inlines.push(Inline::Italic(
                            vec![Inline::Text(arg.clone(), Span::NONE)],
                            Span::NONE,
                        ));
                    } else {
                        inlines.push(Inline::Bold(
                            vec![Inline::Text(arg.clone(), Span::NONE)],
                            Span::NONE,
                        ));
                    }
                    is_italic = !is_italic;
                }
                (
                    None,
                    None,
                    None,
                    None,
                    None,
                    Some(Block::Paragraph {
                        inlines,
                        span: Span::NONE,
                    }),
                )
            }

            // URL (groff extension)
            // .UR url [text]   or   .UR url \n text-lines \n .UE
            "URL" | "UR" => {
                let url = args.first().cloned().unwrap_or_default();
                // Collect text lines between .UR and .UE (if no inline text arg)
                let text = if let Some(inline_text) = args.get(1) {
                    inline_text.clone()
                } else {
                    // Consume lines until .UE or end of input
                    let mut text_lines: Vec<&str> = Vec::new();
                    while let Some(next) = self.current_line() {
                        if next.trim_start().starts_with(".UE")
                            || next.trim_start().starts_with(".UR")
                        {
                            break;
                        }
                        if next.starts_with('.') {
                            // Any other macro stops the URL text
                            break;
                        }
                        let trimmed = next.trim();
                        if !trimmed.is_empty() {
                            text_lines.push(trimmed);
                        }
                        self.advance();
                    }
                    if text_lines.is_empty() {
                        url.clone()
                    } else {
                        text_lines.join(" ")
                    }
                };
                let block = Block::Paragraph {
                    inlines: vec![Inline::Link {
                        url,
                        children: vec![Inline::Text(text, Span::NONE)],
                        span: Span::NONE,
                    }],
                    span: Span::NONE,
                };
                (None, None, None, None, None, Some(block))
            }

            // End URL
            "UE" => (None, None, None, None, None, None),

            // Horizontal rule / break
            "sp" => (
                None,
                None,
                None,
                None,
                None,
                Some(Block::HorizontalRule { span: Span::NONE }),
            ),

            // Other macros - emit diagnostic and ignore
            other => {
                self.diagnostics.push(Diagnostic {
                    span: Span::NONE,
                    severity: Severity::Info,
                    message: format!("unknown macro: .{other}"),
                    code: "man:unknown-macro",
                });
                (None, None, None, None, None, None)
            }
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

    fn parse_paragraph(&mut self) -> Option<Block> {
        let text = self.collect_paragraph_text();
        if text.is_empty() {
            return None;
        }
        let inlines = self.parse_inline_text(&text);
        Some(Block::Paragraph {
            inlines,
            span: Span::NONE,
        })
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

    fn parse_text_block(&mut self) -> Option<Block> {
        let text = self.collect_paragraph_text();
        if text.is_empty() {
            return None;
        }
        let inlines = self.parse_inline_text(&text);
        Some(Block::Paragraph {
            inlines,
            span: Span::NONE,
        })
    }

    fn parse_preformatted(&mut self) -> Block {
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
        Block::CodeBlock {
            content,
            span: Span::NONE,
        }
    }

    fn parse_example_block(&mut self) -> Block {
        let mut lines = Vec::new();

        while let Some(line) = self.current_line() {
            if line == ".EE" {
                self.advance();
                break;
            }
            lines.push(line);
            self.advance();
        }

        let content = lines.join("\n");
        Block::ExampleBlock {
            content,
            span: Span::NONE,
        }
    }

    fn parse_alternating(&mut self, args: &[String], bold_first: bool) -> Block {
        let mut inlines = Vec::new();
        let mut use_style = bold_first;

        for arg in args {
            if use_style {
                let inline = if bold_first {
                    Inline::Bold(vec![Inline::Text(arg.clone(), Span::NONE)], Span::NONE)
                } else {
                    Inline::Italic(vec![Inline::Text(arg.clone(), Span::NONE)], Span::NONE)
                };
                inlines.push(inline);
            } else {
                inlines.push(Inline::Text(arg.clone(), Span::NONE));
            }
            use_style = !use_style;
        }

        Block::Paragraph {
            inlines,
            span: Span::NONE,
        }
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
                    inlines.push(Inline::Text(current.clone(), Span::NONE));
                    current.clear();
                }

                let font_code;
                if chars[i + 2] == '(' && i + 4 < chars.len() {
                    // \f(CW — two-character font name
                    font_code = format!("{}{}", chars[i + 3], chars[i + 4]);
                    i += 5;
                } else {
                    font_code = chars[i + 2].to_string();
                    i += 3;
                }

                // Find the text until next font change or end
                let mut styled_text = String::new();
                while i < chars.len() {
                    if i + 2 < chars.len() && chars[i] == '\\' && chars[i + 1] == 'f' {
                        break;
                    }
                    // Handle escapes inside styled text
                    if i + 1 < chars.len()
                        && chars[i] == '\\'
                        && let Some(esc) = self.try_escape(&chars, &mut i)
                    {
                        styled_text.push_str(&esc);
                        continue;
                    }
                    styled_text.push(chars[i]);
                    i += 1;
                }

                if !styled_text.is_empty() {
                    let styled_inline = match font_code.as_str() {
                        "B" => Inline::Bold(
                            vec![Inline::Text(styled_text, Span::NONE)],
                            Span::NONE,
                        ),
                        "I" => Inline::Italic(
                            vec![Inline::Text(styled_text, Span::NONE)],
                            Span::NONE,
                        ),
                        "CW" | "CR" => Inline::Code(styled_text, Span::NONE),
                        "R" | "P" => Inline::Text(styled_text, Span::NONE),
                        _ => Inline::Text(styled_text, Span::NONE),
                    };
                    inlines.push(styled_inline);
                }
                continue;
            }

            // Special character escapes
            if i + 1 < chars.len() && chars[i] == '\\' {
                if let Some(esc) = self.try_escape(&chars, &mut i) {
                    current.push_str(&esc);
                    continue;
                }
                // Fallback: push both chars
                current.push(chars[i]);
                current.push(chars[i + 1]);
                i += 2;
                continue;
            }

            current.push(chars[i]);
            i += 1;
        }

        if !current.is_empty() {
            inlines.push(Inline::Text(current, Span::NONE));
        }

        inlines
    }

    /// Try to parse an escape sequence starting at position `i`.
    /// On success, advances `i` past the escape and returns the replacement string.
    /// On failure, returns None and does not advance `i`.
    fn try_escape(&self, chars: &[char], i: &mut usize) -> Option<String> {
        if *i + 1 >= chars.len() || chars[*i] != '\\' {
            return None;
        }
        match chars[*i + 1] {
            '-' => { *i += 2; Some("-".into()) }
            '\\' => { *i += 2; Some("\\".into()) }
            'e' => { *i += 2; Some("\\".into()) }
            '~' => { *i += 2; Some("\u{00a0}".into()) } // non-breaking space
            '&' => { *i += 2; Some(String::new()) } // zero-width, skip
            '(' if *i + 3 < chars.len() => {
                let code = format!("{}{}", chars[*i + 2], chars[*i + 3]);
                let replacement = match code.as_str() {
                    "em" => "\u{2014}", // em dash
                    "en" => "\u{2013}", // en dash
                    "co" => "\u{00a9}", // copyright
                    "rg" => "\u{00ae}", // registered
                    "bu" => "\u{2022}", // bullet
                    "sq" => "\u{25a1}", // white square
                    "lq" => "\u{201c}", // left double quote
                    "rq" => "\u{201d}", // right double quote
                    _ => return None,
                };
                *i += 4;
                Some(replacement.into())
            }
            _ => None,
        }
    }
}
