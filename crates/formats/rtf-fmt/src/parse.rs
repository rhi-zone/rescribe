/// High-level parser: RTF text → [`RtfDoc`] + diagnostics.
use crate::ast::*;

/// Parse an RTF string into an [`RtfDoc`].
///
/// Parsing is always infallible: malformed constructs are silently tolerated
/// and may produce entries in the returned [`Diagnostic`] list.
pub fn parse(input: &str) -> (RtfDoc, Vec<Diagnostic>) {
    let mut p = Parser::new(input);
    let blocks = p.run();
    let doc = RtfDoc {
        blocks,
        span: Span::new(0, input.len()),
    };
    (doc, p.diagnostics)
}

// ── State ─────────────────────────────────────────────────────────────────────

/// Text formatting state.  A copy is pushed onto the stack at each `{` and
/// restored on `}`, matching RTF's group semantics.
#[derive(Default, Clone)]
struct TextState {
    bold: bool,
    italic: bool,
    underline: bool,
    strikethrough: bool,
    superscript: bool,
    subscript: bool,
}

// ── Parser ────────────────────────────────────────────────────────────────────

struct Parser<'a> {
    input: &'a str,
    pos: usize,
    pub diagnostics: Vec<Diagnostic>,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            pos: 0,
            diagnostics: Vec::new(),
        }
    }

    fn current_char(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn advance(&mut self) {
        if let Some(ch) = self.current_char() {
            self.pos += ch.len_utf8();
        }
    }

    fn run(&mut self) -> Vec<Block> {
        self.skip_rtf_header();

        let mut state = TextState::default();
        let mut state_stack: Vec<TextState> = Vec::new();
        let mut paragraphs: Vec<Block> = Vec::new();
        let mut current_para: Vec<Inline> = Vec::new();
        let mut current_text = String::new();
        let mut text_start = self.pos;
        let mut para_start = self.pos;

        while self.pos < self.input.len() {
            let Some(ch) = self.current_char() else {
                break;
            };

            match ch {
                '\\' => {
                    self.advance(); // skip '\'
                    let Some(next) = self.current_char() else {
                        break;
                    };

                    if next.is_ascii_alphabetic() {
                        let word_start = self.pos - 1; // position of '\'
                        let (word, param) = self.read_control_word();
                        self.handle_control_word(
                            &word,
                            param,
                            word_start,
                            &mut state,
                            &mut current_text,
                            &mut text_start,
                            &mut current_para,
                            &mut paragraphs,
                            &mut para_start,
                        );
                    } else if next == '\'' {
                        // \'XX hex-encoded byte (Windows-1252)
                        self.advance(); // skip '\''
                        if self.pos + 2 <= self.input.len() {
                            let hex = &self.input[self.pos..self.pos + 2];
                            if let Ok(code) = u8::from_str_radix(hex, 16) {
                                // Treat as Windows-1252 if in non-ASCII range
                                let decoded = windows1252_to_char(code);
                                current_text.push(decoded);
                            }
                            self.pos += 2;
                        }
                    } else {
                        // Control symbol
                        match next {
                            '\\' => current_text.push('\\'),
                            '{' => current_text.push('{'),
                            '}' => current_text.push('}'),
                            '~' => current_text.push('\u{00A0}'), // non-breaking space
                            '-' => {}                             // optional hyphen — ignore
                            '_' => current_text.push('\u{2011}'), // non-breaking hyphen
                            '\n' | '\r' => {}                     // escaped newline = ignored
                            _ => {}                               // unknown control symbol — ignore
                        }
                        self.advance();
                    }
                }

                '{' => {
                    self.advance();
                    if self.is_skip_group() {
                        self.skip_balanced_group();
                    } else {
                        state_stack.push(state.clone());
                    }
                }

                '}' => {
                    // Flush pending text before restoring state
                    if !current_text.is_empty() {
                        let span = Span::new(text_start, self.pos);
                        current_para.push(make_inline(&current_text, &state, span));
                        current_text.clear();
                    }
                    text_start = self.pos;
                    // Restore parent group's state
                    if let Some(prev) = state_stack.pop() {
                        state = prev;
                    }
                    self.advance();
                }

                '\n' | '\r' => {
                    self.advance();
                }

                _ => {
                    if current_text.is_empty() {
                        text_start = self.pos;
                    }
                    current_text.push(ch);
                    self.advance();
                }
            }
        }

        // Flush remaining
        if !current_text.is_empty() {
            let span = Span::new(text_start, self.pos);
            current_para.push(make_inline(&current_text, &state, span));
        }
        if !current_para.is_empty() {
            paragraphs.push(Block::Paragraph {
                inlines: current_para,
                span: Span::new(para_start, self.pos),
            });
        }

        paragraphs
    }

    /// Skip past the `\rtf1` header word so we start processing at the
    /// document body.
    fn skip_rtf_header(&mut self) {
        if let Some(pos) = self.input.find("\\rtf") {
            self.pos = pos;
            // Skip `\rtf1` (or `\rtf`) word
            while self.pos < self.input.len() {
                match self.current_char() {
                    Some(' ') | Some('\\') | Some('{') => break,
                    _ => self.advance(),
                }
            }
            // Skip the trailing space delimiter if present
            if self.current_char() == Some(' ') {
                self.advance();
            }
        }
    }

    /// Return `true` if the current position is the start of a group that
    /// should be skipped wholesale (font table, color table, picture data, etc.)
    fn is_skip_group(&self) -> bool {
        let rest = &self.input[self.pos..];

        // `{\*\...}` destination groups
        if rest.starts_with("\\*") {
            return true;
        }

        const SKIP_PREFIXES: &[&str] = &[
            "\\fonttbl",
            "\\colortbl",
            "\\stylesheet",
            "\\info",
            "\\pict",
            "\\object",
            "\\header",
            "\\footer",
            "\\headerl",
            "\\headerr",
            "\\footerl",
            "\\footerr",
            "\\fldinst",
        ];

        SKIP_PREFIXES.iter().any(|p| rest.starts_with(p))
    }

    /// Skip a balanced `{...}` group.  Called when `pos` is just past the
    /// opening `{` (i.e. we have already consumed the `{`).
    fn skip_balanced_group(&mut self) {
        let mut depth = 1usize;
        while self.pos < self.input.len() && depth > 0 {
            match self.current_char() {
                Some('{') => {
                    depth += 1;
                    self.advance();
                }
                Some('}') => {
                    depth -= 1;
                    self.advance();
                }
                Some('\\') => {
                    self.advance(); // skip '\'
                    if self.pos < self.input.len() {
                        self.advance(); // skip next char (control symbol or start of word)
                    }
                }
                _ => self.advance(),
            }
        }
    }

    fn read_control_word(&mut self) -> (String, Option<i32>) {
        let mut word = String::new();
        while self.pos < self.input.len() {
            match self.current_char() {
                Some(c) if c.is_ascii_alphabetic() => {
                    word.push(c);
                    self.advance();
                }
                _ => break,
            }
        }

        let mut negative = false;
        if self.current_char() == Some('-') {
            negative = true;
            self.advance();
        }

        let mut num = String::new();
        while self.pos < self.input.len() {
            match self.current_char() {
                Some(c) if c.is_ascii_digit() => {
                    num.push(c);
                    self.advance();
                }
                _ => break,
            }
        }

        let param = if num.is_empty() {
            None
        } else {
            num.parse::<i32>()
                .ok()
                .map(|n| if negative { -n } else { n })
        };

        // Consume optional trailing space delimiter
        if self.current_char() == Some(' ') {
            self.advance();
        }

        (word, param)
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_control_word(
        &mut self,
        word: &str,
        param: Option<i32>,
        _word_start: usize,
        state: &mut TextState,
        current_text: &mut String,
        text_start: &mut usize,
        current_para: &mut Vec<Inline>,
        paragraphs: &mut Vec<Block>,
        para_start: &mut usize,
    ) {
        match word {
            "par" | "pard" => {
                if !current_text.is_empty() {
                    let span = Span::new(*text_start, self.pos);
                    current_para.push(make_inline(current_text, state, span));
                    current_text.clear();
                }
                if !current_para.is_empty() {
                    paragraphs.push(Block::Paragraph {
                        inlines: std::mem::take(current_para),
                        span: Span::new(*para_start, self.pos),
                    });
                }
                *text_start = self.pos;
                *para_start = self.pos;
                if word == "pard" {
                    *state = TextState::default();
                }
            }

            "line" => {
                if !current_text.is_empty() {
                    let span = Span::new(*text_start, self.pos);
                    current_para.push(make_inline(current_text, state, span));
                    current_text.clear();
                }
                *text_start = self.pos;
                current_para.push(Inline::LineBreak {
                    span: Span::new(self.pos, self.pos),
                });
            }

            "b" => {
                flush_text(current_text, text_start, self.pos, state, current_para);
                state.bold = param.unwrap_or(1) != 0;
            }
            "i" => {
                flush_text(current_text, text_start, self.pos, state, current_para);
                state.italic = param.unwrap_or(1) != 0;
            }
            "ul" | "uld" | "uldb" | "ulw" => {
                flush_text(current_text, text_start, self.pos, state, current_para);
                state.underline = param.unwrap_or(1) != 0;
            }
            "ulnone" => {
                flush_text(current_text, text_start, self.pos, state, current_para);
                state.underline = false;
            }
            "strike" => {
                flush_text(current_text, text_start, self.pos, state, current_para);
                state.strikethrough = param.unwrap_or(1) != 0;
            }
            "super" => {
                flush_text(current_text, text_start, self.pos, state, current_para);
                state.superscript = true;
                state.subscript = false;
            }
            "sub" => {
                flush_text(current_text, text_start, self.pos, state, current_para);
                state.subscript = true;
                state.superscript = false;
            }
            "nosupersub" => {
                flush_text(current_text, text_start, self.pos, state, current_para);
                state.superscript = false;
                state.subscript = false;
            }

            "tab" => {
                if current_text.is_empty() {
                    *text_start = self.pos;
                }
                current_text.push('\t');
            }

            // Unicode character: \uN?
            "u" => {
                if let Some(n) = param {
                    let ch = char::from_u32(n as u32).unwrap_or('\u{FFFD}');
                    if current_text.is_empty() {
                        *text_start = self.pos;
                    }
                    current_text.push(ch);
                    // Skip the ANSI fallback character that follows \uN
                    if self.current_char() == Some('?') {
                        self.advance();
                    }
                }
            }

            // Named special characters
            "emdash" => push_char(current_text, text_start, self.pos, '\u{2014}'),
            "endash" => push_char(current_text, text_start, self.pos, '\u{2013}'),
            "lquote" => push_char(current_text, text_start, self.pos, '\u{2018}'),
            "rquote" => push_char(current_text, text_start, self.pos, '\u{2019}'),
            "ldblquote" => push_char(current_text, text_start, self.pos, '\u{201C}'),
            "rdblquote" => push_char(current_text, text_start, self.pos, '\u{201D}'),
            "bullet" => push_char(current_text, text_start, self.pos, '\u{2022}'),
            "enspace" => push_char(current_text, text_start, self.pos, '\u{2002}'),
            "emspace" => push_char(current_text, text_start, self.pos, '\u{2003}'),

            // Field result (hyperlink text) — descend into group
            "fldrslt" => {
                // The fldrslt group contains the visible link text.
                // We have already entered the group (state_stack has been
                // pushed), so just continue parsing normally.
            }

            // Ignored formatting / sizing controls
            "fs" | "f" | "cf" | "cb" | "li" | "fi" | "ri" | "sa" | "sb" | "sl" | "slmult"
            | "ql" | "qr" | "qc" | "qj" | "qd" | "outlinelevel" | "pntext" | "pn" | "pnlvlblt"
            | "rtf" | "ansi" | "mac" | "pc" | "pca" | "deff" | "deflang" | "widowctrl"
            | "hyphauto" | "hyphconsec" | "hyphcaps" | "paperw" | "paperh" | "margl" | "margr"
            | "margt" | "margb" | "sectd" | "cols" | "colsx" | "endhere" | "pgwsxn" | "pghsxn"
            | "headerl" | "headerr" | "header" | "footer" | "footerl" | "footerr" | "trowd"
            | "cellx" | "intbl" | "cell" | "row" | "trgaph" | "trql" | "trqr" | "trqc"
            | "clmgf" | "clmrg" | "brdrb" | "brdrs" | "brdrw" | "brsp" | "brdrt" | "brdrl"
            | "brdrr" | "brdrth" | "brdrdot" | "brdrdash" | "b0" | "i0" => {
                // Most toggle-off words are redundant after state flush above,
                // but b0/i0 specifically turn off formatting
                if word == "b0" {
                    flush_text(current_text, text_start, self.pos, state, current_para);
                    state.bold = false;
                } else if word == "i0" {
                    flush_text(current_text, text_start, self.pos, state, current_para);
                    state.italic = false;
                }
            }

            _ => {
                // Unknown control word — emit a diagnostic
                self.diagnostics.push(Diagnostic {
                    span: Span::new(self.pos, self.pos),
                    severity: Severity::Info,
                    message: format!("unknown control word: \\{word}"),
                    code: "rtf::unknown-control-word",
                });
            }
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn flush_text(
    current_text: &mut String,
    text_start: &mut usize,
    pos: usize,
    state: &TextState,
    current_para: &mut Vec<Inline>,
) {
    if !current_text.is_empty() {
        let span = Span::new(*text_start, pos);
        current_para.push(make_inline(current_text, state, span));
        current_text.clear();
        *text_start = pos;
    }
}

fn push_char(current_text: &mut String, text_start: &mut usize, pos: usize, ch: char) {
    if current_text.is_empty() {
        *text_start = pos;
    }
    current_text.push(ch);
}

fn make_inline(text: &str, state: &TextState, span: Span) -> Inline {
    let mut inline = Inline::Text {
        text: text.to_string(),
        span,
    };
    if state.strikethrough {
        inline = Inline::Strikethrough {
            children: vec![inline],
            span,
        };
    }
    if state.underline {
        inline = Inline::Underline {
            children: vec![inline],
            span,
        };
    }
    if state.italic {
        inline = Inline::Italic {
            children: vec![inline],
            span,
        };
    }
    if state.bold {
        inline = Inline::Bold {
            children: vec![inline],
            span,
        };
    }
    if state.superscript {
        inline = Inline::Superscript {
            children: vec![inline],
            span,
        };
    }
    if state.subscript {
        inline = Inline::Subscript {
            children: vec![inline],
            span,
        };
    }
    inline
}

/// Decode a Windows-1252 byte to a Unicode char.
///
/// For bytes 0x00–0x7F it is identical to ASCII.  The range 0x80–0x9F is
/// remapped per the Windows-1252 code page.  0xA0–0xFF map to Latin-1.
fn windows1252_to_char(byte: u8) -> char {
    // Only the 0x80–0x9F range differs from Latin-1
    #[rustfmt::skip]
    const W1252: [char; 32] = [
        '\u{20AC}', '\u{FFFD}', '\u{201A}', '\u{0192}',
        '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}',
        '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}',
        '\u{0152}', '\u{FFFD}', '\u{017D}', '\u{FFFD}',
        '\u{FFFD}', '\u{2018}', '\u{2019}', '\u{201C}',
        '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}',
        '\u{02DC}', '\u{2122}', '\u{0161}', '\u{203A}',
        '\u{0153}', '\u{FFFD}', '\u{017E}', '\u{0178}',
    ];
    if (0x80..=0x9F).contains(&byte) {
        W1252[(byte - 0x80) as usize]
    } else {
        byte as char
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn p(input: &str) -> RtfDoc {
        parse(input).0
    }

    #[test]
    fn test_parse_simple_text() {
        let doc = p(r"{\rtf1 Hello world\par}");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Paragraph { .. }));
    }

    #[test]
    fn test_parse_bold() {
        let doc = p(r"{\rtf1 \b bold text\b0 normal\par}");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Bold { .. })));
    }

    #[test]
    fn test_parse_italic() {
        let doc = p(r"{\rtf1 \i italic\i0\par}");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Italic { .. })));
    }

    #[test]
    fn test_parse_underline() {
        let doc = p(r"{\rtf1 \ul underlined\ulnone\par}");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(
            inlines
                .iter()
                .any(|i| matches!(i, Inline::Underline { .. }))
        );
    }

    #[test]
    fn test_parse_multiple_paragraphs() {
        let doc = p(r"{\rtf1 First paragraph\par Second paragraph\par}");
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_parse_escaped_chars() {
        let doc = p(r"{\rtf1 Open \{ and close \}\par}");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        let text = collect_text(inlines);
        assert!(text.contains('{'));
        assert!(text.contains('}'));
    }

    #[test]
    fn test_parse_special_chars() {
        let doc = p(r"{\rtf1 Em\emdash dash\par}");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        let text = collect_text(inlines);
        assert!(text.contains('\u{2014}'));
    }

    #[test]
    fn test_group_state_restoration() {
        // After leaving a group, bold should be off again
        let doc = p(r"{\rtf1 normal {\b bold} still normal\par}");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        // "still normal" should be plain Text, not Bold
        let last = inlines.last().unwrap();
        assert!(
            matches!(last, Inline::Text { .. }),
            "expected Text after group close, got {last:?}"
        );
    }

    #[test]
    fn test_spans_present() {
        let doc = p(r"{\rtf1 Hello\par}");
        assert_ne!(doc.span, Span::NONE);
        let Block::Paragraph { span, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert_ne!(*span, Span::NONE);
    }

    fn collect_text(inlines: &[Inline]) -> String {
        let mut out = String::new();
        for inline in inlines {
            match inline {
                Inline::Text { text, .. } => out.push_str(text),
                Inline::Bold { children, .. }
                | Inline::Italic { children, .. }
                | Inline::Underline { children, .. }
                | Inline::Strikethrough { children, .. }
                | Inline::Superscript { children, .. }
                | Inline::Subscript { children, .. } => out.push_str(&collect_text(children)),
                Inline::Code { text, .. } => out.push_str(text),
                Inline::Link { children, url, .. } => {
                    if children.is_empty() {
                        out.push_str(url);
                    } else {
                        out.push_str(&collect_text(children));
                    }
                }
                Inline::Image { alt, .. } => out.push_str(alt),
                Inline::LineBreak { .. } | Inline::SoftBreak { .. } => out.push(' '),
            }
        }
        out
    }
}
