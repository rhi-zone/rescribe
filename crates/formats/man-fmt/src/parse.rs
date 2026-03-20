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

type ParseElement = (Option<String>, Option<String>, Option<Block>);

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
        let mut blocks = Vec::new();

        while self.pos < self.lines.len() {
            let (t, s, block) = self.parse_element();
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

        ManDoc {
            title,
            section,
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
            None => return (None, None, None),
        };

        // Skip empty lines and comments
        if line.is_empty() {
            self.advance();
            return (None, None, None);
        }
        if line.starts_with(".\\\"") || line.starts_with("'\\\"") {
            self.advance();
            return (None, None, None);
        }

        // Macro lines start with .
        if line.starts_with('.') {
            return self.parse_macro();
        }

        // Plain text paragraph
        let block = self.parse_text_block();
        (None, None, block)
    }

    fn parse_macro(&mut self) -> ParseElement {
        let line = match self.current_line() {
            Some(l) => l,
            None => return (None, None, None),
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
                    inlines: vec![Inline::Text(t.clone(), Span::NONE)],
                    span: Span::NONE,
                });
                (title, section, block)
            }

            // Section heading
            "SH" => {
                let text = args.join(" ");
                let inlines = self.parse_inline_text(&text);
                (
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
                (None, None, block)
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
                    Block::Paragraph {
                        inlines: content_inline,
                        span: Span::NONE,
                    }
                };

                (None, None, Some(block))
            }

            // Relative indent start/end
            "RS" | "RE" => {
                // Skip these for now, they affect indentation
                (None, None, None)
            }

            // No-fill (preformatted)
            "nf" => {
                let block = self.parse_preformatted();
                (None, None, Some(block))
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
                (None, None, Some(block))
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
                (None, None, Some(block))
            }

            // Bold-Roman alternation
            "BR" | "RB" => {
                let block = self.parse_alternating(&args, true);
                (None, None, Some(block))
            }

            // Italic-Roman alternation
            "IR" | "RI" => {
                let block = self.parse_alternating(&args, false);
                (None, None, Some(block))
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
                    Some(Block::Paragraph {
                        inlines,
                        span: Span::NONE,
                    }),
                )
            }

            // URL (groff extension)
            "URL" | "UR" => {
                let url = args.first().cloned().unwrap_or_default();
                let text = args.get(1).cloned().unwrap_or_else(|| url.clone());
                let block = Block::Paragraph {
                    inlines: vec![Inline::Link {
                        url,
                        children: vec![Inline::Text(text, Span::NONE)],
                        span: Span::NONE,
                    }],
                    span: Span::NONE,
                };
                (None, None, Some(block))
            }

            // End URL
            "UE" => (None, None, None),

            // Horizontal rule / break
            "sp" => (
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
                (None, None, None)
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
                        'B' => Inline::Bold(
                            vec![Inline::Text(styled_text, Span::NONE)],
                            Span::NONE,
                        ),
                        'I' => Inline::Italic(
                            vec![Inline::Text(styled_text, Span::NONE)],
                            Span::NONE,
                        ),
                        'R' | 'P' => Inline::Text(styled_text, Span::NONE),
                        _ => Inline::Text(styled_text, Span::NONE),
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
            inlines.push(Inline::Text(current, Span::NONE));
        }

        inlines
    }
}
