//! RTF (Rich Text Format) tokenizer, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-rtf` and `rescribe-write-rtf` as thin adapter layers.

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct RtfError(pub String);

impl std::fmt::Display for RtfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RTF error: {}", self.0)
    }
}

impl std::error::Error for RtfError {}

// ── AST ───────────────────────────────────────────────────────────────────────

/// A parsed RTF document.
#[derive(Debug, Clone, Default)]
pub struct RtfDoc {
    pub blocks: Vec<Block>,
}

/// Block-level element.
#[derive(Debug, Clone)]
pub enum Block {
    Paragraph {
        inlines: Vec<Inline>,
    },
    Heading {
        level: u8,
        inlines: Vec<Inline>,
    },
    CodeBlock {
        content: String,
    },
    Blockquote {
        children: Vec<Block>,
    },
    List {
        ordered: bool,
        items: Vec<Vec<Block>>,
    },
    Table {
        rows: Vec<TableRow>,
    },
    HorizontalRule,
}

/// A table row.
#[derive(Debug, Clone)]
pub struct TableRow {
    pub cells: Vec<Vec<Inline>>,
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
    Link { url: String, children: Vec<Inline> },
    Image { url: String, alt: String },
    LineBreak,
    SoftBreak,
    Superscript(Vec<Inline>),
    Subscript(Vec<Inline>),
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Parse an RTF string into an [`RtfDoc`].
pub fn parse(input: &str) -> Result<RtfDoc, RtfError> {
    let mut p = Parser::new(input);
    let blocks = p.parse()?;
    Ok(RtfDoc { blocks })
}

#[derive(Default, Clone)]
struct TextState {
    bold: bool,
    italic: bool,
    underline: bool,
    strikethrough: bool,
}

struct Parser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn parse(&mut self) -> Result<Vec<Block>, RtfError> {
        self.skip_header();

        let mut state = TextState::default();
        let mut paragraphs: Vec<Block> = Vec::new();
        let mut current_para: Vec<Inline> = Vec::new();
        let mut current_text = String::new();

        while self.pos < self.input.len() {
            let ch = self.current_char();

            match ch {
                '\\' => {
                    self.pos += 1;
                    if self.pos >= self.input.len() {
                        break;
                    }

                    let next = self.current_char();
                    if next.is_alphabetic() {
                        let (word, param) = self.read_control_word();
                        self.handle_control_word(
                            &word,
                            param,
                            &mut state,
                            &mut current_text,
                            &mut current_para,
                            &mut paragraphs,
                        );
                    } else if next == '\'' {
                        // Hex character
                        self.pos += 1;
                        if self.pos + 2 <= self.input.len() {
                            let hex = &self.input[self.pos..self.pos + 2];
                            if let Ok(code) = u8::from_str_radix(hex, 16) {
                                current_text.push(code as char);
                            }
                            self.pos += 2;
                        }
                    } else {
                        match next {
                            '\\' => current_text.push('\\'),
                            '{' => current_text.push('{'),
                            '}' => current_text.push('}'),
                            '~' => current_text.push('\u{00A0}'),
                            '-' => {}
                            '_' => current_text.push('\u{2011}'),
                            '\n' | '\r' => {}
                            _ => {}
                        }
                        self.pos += 1;
                    }
                }
                '{' => {
                    self.pos += 1;
                    self.skip_special_groups();
                }
                '}' => {
                    self.pos += 1;
                }
                '\n' | '\r' => {
                    self.pos += 1;
                }
                _ => {
                    current_text.push(ch);
                    self.pos += 1;
                }
            }
        }

        // Flush remaining text
        if !current_text.is_empty() {
            current_para.push(make_inline(&current_text, &state));
        }

        // Flush remaining paragraph
        if !current_para.is_empty() {
            paragraphs.push(Block::Paragraph {
                inlines: current_para,
            });
        }

        Ok(paragraphs)
    }

    fn current_char(&self) -> char {
        self.input[self.pos..].chars().next().unwrap_or('\0')
    }

    fn skip_header(&mut self) {
        if let Some(pos) = self.input.find("\\rtf") {
            self.pos = pos;
            while self.pos < self.input.len() {
                let ch = self.current_char();
                if ch == ' ' || ch == '\\' || ch == '{' {
                    break;
                }
                self.pos += 1;
            }
        }
    }

    fn skip_special_groups(&mut self) {
        let start = self.pos;
        if self.pos < self.input.len() && self.current_char() == '\\' {
            let temp_pos = self.pos + 1;
            let rest = &self.input[temp_pos..];

            let skip_groups = [
                "fonttbl",
                "colortbl",
                "stylesheet",
                "info",
                "pict",
                "object",
                "header",
                "footer",
                "headerl",
                "headerr",
                "footerl",
                "footerr",
                "*",
            ];

            for group in skip_groups {
                if rest.starts_with(group) {
                    let mut depth = 1;
                    self.pos = start;
                    while self.pos < self.input.len() && depth > 0 {
                        match self.current_char() {
                            '{' => depth += 1,
                            '}' => depth -= 1,
                            '\\' => {
                                self.pos += 1;
                                if self.pos < self.input.len() {
                                    self.pos += 1;
                                }
                                continue;
                            }
                            _ => {}
                        }
                        self.pos += 1;
                    }
                    return;
                }
            }
        }
    }

    fn read_control_word(&mut self) -> (String, Option<i32>) {
        let mut word = String::new();

        while self.pos < self.input.len() {
            let ch = self.current_char();
            if ch.is_ascii_alphabetic() {
                word.push(ch);
                self.pos += 1;
            } else {
                break;
            }
        }

        let mut param = None;
        let mut negative = false;

        if self.pos < self.input.len() && self.current_char() == '-' {
            negative = true;
            self.pos += 1;
        }

        if self.pos < self.input.len() && self.current_char().is_ascii_digit() {
            let mut num = String::new();
            while self.pos < self.input.len() && self.current_char().is_ascii_digit() {
                num.push(self.current_char());
                self.pos += 1;
            }
            if let Ok(n) = num.parse::<i32>() {
                param = Some(if negative { -n } else { n });
            }
        }

        if self.pos < self.input.len() && self.current_char() == ' ' {
            self.pos += 1;
        }

        (word, param)
    }

    fn handle_control_word(
        &mut self,
        word: &str,
        param: Option<i32>,
        state: &mut TextState,
        current_text: &mut String,
        current_para: &mut Vec<Inline>,
        paragraphs: &mut Vec<Block>,
    ) {
        match word {
            "par" | "pard" => {
                if !current_text.is_empty() {
                    current_para.push(make_inline(current_text, state));
                    current_text.clear();
                }
                if !current_para.is_empty() {
                    paragraphs.push(Block::Paragraph {
                        inlines: std::mem::take(current_para),
                    });
                }
                if word == "pard" {
                    *state = TextState::default();
                }
            }

            "line" => {
                if !current_text.is_empty() {
                    current_para.push(make_inline(current_text, state));
                    current_text.clear();
                }
                current_para.push(Inline::LineBreak);
            }

            "b" => {
                flush_text(current_text, state, current_para);
                state.bold = param.unwrap_or(1) != 0;
            }
            "i" => {
                flush_text(current_text, state, current_para);
                state.italic = param.unwrap_or(1) != 0;
            }
            "ul" | "uld" | "uldb" | "ulw" => {
                flush_text(current_text, state, current_para);
                state.underline = param.unwrap_or(1) != 0;
            }
            "ulnone" => {
                flush_text(current_text, state, current_para);
                state.underline = false;
            }
            "strike" => {
                flush_text(current_text, state, current_para);
                state.strikethrough = param.unwrap_or(1) != 0;
            }

            "tab" => current_text.push('\t'),

            "emdash" => current_text.push('\u{2014}'),
            "endash" => current_text.push('\u{2013}'),
            "lquote" => current_text.push('\u{2018}'),
            "rquote" => current_text.push('\u{2019}'),
            "ldblquote" => current_text.push('\u{201C}'),
            "rdblquote" => current_text.push('\u{201D}'),
            "bullet" => current_text.push('\u{2022}'),

            _ => {}
        }
    }
}

fn flush_text(current_text: &mut String, state: &TextState, current_para: &mut Vec<Inline>) {
    if !current_text.is_empty() {
        current_para.push(make_inline(current_text, state));
        current_text.clear();
    }
}

fn make_inline(text: &str, state: &TextState) -> Inline {
    let mut inline = Inline::Text(text.to_string());
    if state.strikethrough {
        inline = Inline::Strikethrough(vec![inline]);
    }
    if state.underline {
        inline = Inline::Underline(vec![inline]);
    }
    if state.italic {
        inline = Inline::Italic(vec![inline]);
    }
    if state.bold {
        inline = Inline::Bold(vec![inline]);
    }
    inline
}

// ── Builder ───────────────────────────────────────────────────────────────────

/// Build an RTF string from an [`RtfDoc`].
pub fn build(doc: &RtfDoc) -> String {
    let mut ctx = BuildContext::new();
    ctx.write(r"{\rtf1\ansi\deff0");
    ctx.write(r"{\fonttbl{\f0 Times New Roman;}}");
    ctx.write("\n");
    for block in &doc.blocks {
        build_block(block, &mut ctx);
    }
    ctx.write("}");
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

    fn write_escaped(&mut self, s: &str) {
        for ch in s.chars() {
            match ch {
                '\\' => self.write("\\\\"),
                '{' => self.write("\\{"),
                '}' => self.write("\\}"),
                '\t' => self.write("\\tab "),
                '\n' => self.write("\\line "),
                '\u{00A0}' => self.write("\\~"),
                '\u{2014}' => self.write("\\emdash "),
                '\u{2013}' => self.write("\\endash "),
                '\u{2018}' => self.write("\\lquote "),
                '\u{2019}' => self.write("\\rquote "),
                '\u{201C}' => self.write("\\ldblquote "),
                '\u{201D}' => self.write("\\rdblquote "),
                '\u{2022}' => self.write("\\bullet "),
                c if c.is_ascii() => self.output.push(c),
                c => {
                    let code = c as i16;
                    self.write(&format!("\\u{}?", code));
                }
            }
        }
    }
}

fn build_block(block: &Block, ctx: &mut BuildContext) {
    match block {
        Block::Paragraph { inlines } => {
            ctx.write("\\pard\\fs24 ");
            build_inlines(inlines, ctx);
            ctx.write("\\par\n");
        }

        Block::Heading { level, inlines } => {
            let size = match level {
                1 => 48,
                2 => 40,
                3 => 32,
                4 => 28,
                _ => 24,
            };
            ctx.write(&format!("\\pard\\fs{} \\b ", size));
            build_inlines(inlines, ctx);
            ctx.write("\\b0\\par\n");
        }

        Block::CodeBlock { content } => {
            ctx.write("\\pard\\f1\\fs20 ");
            for line in content.lines() {
                ctx.write_escaped(line);
                ctx.write("\\line ");
            }
            ctx.write("\\f0\\par\n");
        }

        Block::Blockquote { children } => {
            ctx.write("\\pard\\li720 ");
            for child in children {
                match child {
                    Block::Paragraph { inlines } => build_inlines(inlines, ctx),
                    other => build_block(other, ctx),
                }
            }
            ctx.write("\\par\n");
        }

        Block::List { ordered, items } => {
            let mut num = 1u32;
            for item_blocks in items {
                ctx.write("\\pard\\li720\\fi-360 ");
                if *ordered {
                    ctx.write(&format!("{}. ", num));
                    num += 1;
                } else {
                    ctx.write("\\bullet  ");
                }
                for block in item_blocks {
                    match block {
                        Block::Paragraph { inlines } => build_inlines(inlines, ctx),
                        other => build_block(other, ctx),
                    }
                }
                ctx.write("\\par\n");
            }
        }

        Block::Table { rows } => {
            for row in rows {
                ctx.write("\\trowd ");
                for (i, _) in row.cells.iter().enumerate() {
                    let right = (i + 1) * 2000;
                    ctx.write(&format!("\\cellx{}", right));
                }
                for cell in &row.cells {
                    ctx.write("\\pard\\intbl ");
                    build_inlines(cell, ctx);
                    ctx.write("\\cell ");
                }
                ctx.write("\\row\n");
            }
        }

        Block::HorizontalRule => {
            ctx.write("\\pard\\brdrb\\brdrs\\brdrw10\\brsp20 \\par\n");
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
        Inline::Text(s) => ctx.write_escaped(s),

        Inline::Bold(children) => {
            ctx.write("{\\b ");
            build_inlines(children, ctx);
            ctx.write("}");
        }

        Inline::Italic(children) => {
            ctx.write("{\\i ");
            build_inlines(children, ctx);
            ctx.write("}");
        }

        Inline::Underline(children) => {
            ctx.write("{\\ul ");
            build_inlines(children, ctx);
            ctx.write("}");
        }

        Inline::Strikethrough(children) => {
            ctx.write("{\\strike ");
            build_inlines(children, ctx);
            ctx.write("}");
        }

        Inline::Code(s) => {
            ctx.write("{\\f1 ");
            ctx.write_escaped(s);
            ctx.write("}");
        }

        Inline::Link { url, children } => {
            ctx.write("{\\field{\\*\\fldinst HYPERLINK \"");
            ctx.write(url);
            ctx.write("\"}{\\fldrslt ");
            if children.is_empty() {
                ctx.write_escaped(url);
            } else {
                build_inlines(children, ctx);
            }
            ctx.write("}}");
        }

        Inline::Image { url, alt } => {
            let label = if !alt.is_empty() { alt } else { url };
            ctx.write("[Image: ");
            ctx.write_escaped(label);
            ctx.write("]");
        }

        Inline::LineBreak => ctx.write("\\line "),

        Inline::SoftBreak => ctx.write(" "),

        Inline::Superscript(children) => {
            ctx.write("{\\super ");
            build_inlines(children, ctx);
            ctx.write("}");
        }

        Inline::Subscript(children) => {
            ctx.write("{\\sub ");
            build_inlines(children, ctx);
            ctx.write("}");
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_text() {
        let doc = parse(r"{\rtf1 Hello world\par}").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Paragraph { .. }));
    }

    #[test]
    fn test_parse_bold() {
        let doc = parse(r"{\rtf1 \b bold text\b0  normal\par}").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Bold(_))));
    }

    #[test]
    fn test_parse_italic() {
        let doc = parse(r"{\rtf1 \i italic\i0\par}").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Italic(_))));
    }

    #[test]
    fn test_parse_underline() {
        let doc = parse(r"{\rtf1 \ul underlined\ulnone\par}").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Underline(_))));
    }

    #[test]
    fn test_parse_multiple_paragraphs() {
        let doc = parse(r"{\rtf1 First paragraph\par Second paragraph\par}").unwrap();
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_parse_escaped_chars() {
        let doc = parse(r"{\rtf1 Open \{ and close \}\par}").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        let text = collect_text(inlines);
        assert!(text.contains('{'));
        assert!(text.contains('}'));
    }

    #[test]
    fn test_parse_special_chars() {
        let doc = parse(r"{\rtf1 Em\emdash dash\par}").unwrap();
        let Block::Paragraph { inlines } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        let text = collect_text(inlines);
        assert!(text.contains('\u{2014}'));
    }

    #[test]
    fn test_build_header() {
        let doc = RtfDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Hello".into())],
            }],
        };
        let out = build(&doc);
        assert!(out.starts_with("{\\rtf1"));
        assert!(out.ends_with('}'));
    }

    #[test]
    fn test_build_paragraph() {
        let doc = RtfDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Hello, world!".into())],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("Hello, world!"));
        assert!(out.contains("\\par"));
    }

    #[test]
    fn test_build_bold() {
        let doc = RtfDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Bold(vec![Inline::Text("bold".into())])],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("{\\b bold}"));
    }

    fn collect_text(inlines: &[Inline]) -> String {
        let mut out = String::new();
        for inline in inlines {
            match inline {
                Inline::Text(s) => out.push_str(s),
                Inline::Bold(c)
                | Inline::Italic(c)
                | Inline::Underline(c)
                | Inline::Strikethrough(c)
                | Inline::Superscript(c)
                | Inline::Subscript(c) => out.push_str(&collect_text(c)),
                Inline::Code(s) => out.push_str(s),
                Inline::Link { url, children } => {
                    out.push_str(&collect_text(children));
                    if children.is_empty() {
                        out.push_str(url);
                    }
                }
                Inline::Image { alt, .. } => out.push_str(alt),
                Inline::LineBreak | Inline::SoftBreak => out.push(' '),
            }
        }
        out
    }
}
