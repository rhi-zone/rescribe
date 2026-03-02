/// High-level parser: RTF text → [`RtfDoc`] + diagnostics.
use crate::ast::*;

/// Parse raw RTF bytes into an [`RtfDoc`].
///
/// Accepts arbitrary bytes: Windows-1252 high bytes (0x80–0xFF) are decoded
/// lazily via the built-in `windows1252_to_char` table, and `\binN` binary
/// blocks are skipped by advancing the byte position directly.
///
/// Parsing is always infallible: malformed constructs are silently tolerated
/// and may produce entries in the returned [`Diagnostic`] list.
pub fn parse(input: &[u8]) -> (RtfDoc, Vec<Diagnostic>) {
    let mut p = Parser::new(input);
    let blocks = p.run();
    // The parser's internal color_table always has index-0 = auto/default
    // sentinel.  Strip it so that `RtfDoc.color_table` contains only the
    // actual colors (indices 1..N from the RTF `\colortbl`), matching the
    // layout that programmatically-built documents use.
    let color_table = if p.color_table.len() > 1 {
        p.color_table[1..].to_vec()
    } else {
        vec![]
    };
    let doc = RtfDoc {
        blocks,
        color_table,
        span: Span::new(0, input.len()),
    };
    (doc, p.diagnostics)
}

/// Convenience wrapper for callers that already have a `&str`.
pub fn parse_str(input: &str) -> (RtfDoc, Vec<Diagnostic>) {
    parse(input.as_bytes())
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
    all_caps: bool,
    small_caps: bool,
    /// Hidden text (`\v`, `\webhidden`): present in document but not displayed.
    hidden: bool,
    /// Font size in half-points (0 = not explicitly set).
    font_size: u16,
    /// Foreground color table index (0 = auto/default).
    color_idx: u8,
    /// Background color table index (0 = auto/default).  Set by `\cb<N>`.
    bg_color_idx: u8,
    /// Font table index (0 = default font).  Set by `\f<N>`.
    font_idx: u16,
    /// Language LCID (0 = not explicitly set).  Set by `\lang<N>`.
    lang_id: u16,
    /// Raw RTF character-layout control words (e.g. `\dn3\shad`) accumulated
    /// verbatim for re-emission.  Empty string means no char-layout words active.
    char_props: String,
}

// ── Table accumulator ─────────────────────────────────────────────────────────

/// Accumulates table state across `\intbl` / `\cell` / `\row` control words.
#[derive(Default)]
struct TableAccum {
    /// True while we are inside a table paragraph (`\intbl` has been seen for
    /// the current paragraph and `\pard` has not reset it yet).
    in_table: bool,
    /// Inlines accumulated for the cell currently being parsed.
    current_row: Vec<Vec<Inline>>,
    /// Rows accumulated for the table currently being parsed.
    table_rows: Vec<TableRow>,
    /// Byte offset where the current table started (for Span).
    table_start: usize,
    /// Byte offset where the current row started (for Span).
    row_start: usize,
}

impl TableAccum {
    /// Flush any fully-accumulated table into `paragraphs`.  Call this before
    /// pushing a non-table paragraph (on `\pard`).
    fn flush_table(&mut self, paragraphs: &mut Vec<Block>, pos: usize) {
        if !self.table_rows.is_empty() {
            paragraphs.push(Block::Table {
                rows: std::mem::take(&mut self.table_rows),
                span: Span::new(self.table_start, pos),
            });
        }
    }
}

// ── Parser ────────────────────────────────────────────────────────────────────

struct Parser<'a> {
    input: &'a [u8],
    pos: usize,
    pub diagnostics: Vec<Diagnostic>,
    pub color_table: Vec<(u8, u8, u8)>,
    /// Font names from the `\fonttbl` group; index 0 is the default font.
    pub font_table: Vec<String>,
    /// Windows code page declared by `\ansicpg<N>`; defaults to 1252.
    pub codepage: u16,
}

impl<'a> Parser<'a> {
    fn new(input: &'a [u8]) -> Self {
        Self {
            input,
            pos: 0,
            diagnostics: Vec::new(),
            color_table: parse_color_table(input),
            font_table: parse_font_table(input),
            codepage: parse_ansicpg(input),
        }
    }

    /// Construct a sub-parser that inherits the color table, font table, and
    /// code page from the parent document, but parses a different byte slice.
    fn with_tables(
        input: &'a [u8],
        color_table: Vec<(u8, u8, u8)>,
        font_table: Vec<String>,
        codepage: u16,
    ) -> Self {
        Self {
            input,
            pos: 0,
            diagnostics: Vec::new(),
            color_table,
            font_table,
            codepage,
        }
    }

    fn current_byte(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    fn advance(&mut self) {
        self.pos += 1;
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
        let mut current_align = Align::Default;
        // Accumulates raw RTF paragraph-layout control words verbatim so they
        // can be preserved in the AST and re-emitted without loss.
        let mut current_para_props = String::new();
        let mut tbl = TableAccum::default();

        while self.pos < self.input.len() {
            let Some(byte) = self.current_byte() else {
                break;
            };

            match byte {
                b'\\' => {
                    self.advance(); // skip '\'
                    let Some(next) = self.current_byte() else {
                        break;
                    };

                    if next.is_ascii_lowercase() {
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
                            &mut current_align,
                            &mut current_para_props,
                            &mut tbl,
                        );
                    } else if next == b'\'' {
                        // \'XX hex-encoded byte (Windows-1252).
                        self.advance(); // skip '\''
                        let hi = self.current_byte();
                        if hi.is_some() {
                            self.advance();
                        }
                        let lo = self.current_byte();
                        if lo.is_some() {
                            self.advance();
                        }
                        if let (Some(h), Some(l)) = (hi, lo)
                            && h.is_ascii_hexdigit()
                            && l.is_ascii_hexdigit()
                        {
                            let code = ((h as char).to_digit(16).unwrap() * 16
                                + (l as char).to_digit(16).unwrap())
                                as u8;
                            if current_text.is_empty() {
                                text_start = self.pos;
                            }
                            current_text.push(codepage_to_char(self.codepage, code));
                        }
                    } else {
                        // Control symbol
                        match next {
                            b'\\' => current_text.push('\\'),
                            b'{' => current_text.push('{'),
                            b'}' => current_text.push('}'),
                            b'~' => current_text.push('\u{00A0}'), // non-breaking space
                            b'-' => {}                             // optional hyphen — ignore
                            b'_' => current_text.push('\u{2011}'), // non-breaking hyphen
                            b'\n' | b'\r' => {}                    // escaped newline = ignored
                            _ => {} // unknown control symbol — ignore
                        }
                        self.advance();
                    }
                }

                b'{' => {
                    self.advance();
                    if self.is_footnote_group() {
                        // Parse footnote / endnote content into an Inline::Footnote
                        // at the current position, flushing any pending text first.
                        if !current_text.is_empty() {
                            let span = Span::new(text_start, self.pos);
                            current_para.push(make_inline(
                                &current_text,
                                &state,
                                span,
                                &self.color_table,
                                &self.font_table,
                            ));
                            current_text.clear();
                        }
                        let fn_start = self.pos;
                        let footnote_content = self.parse_footnote_group();
                        current_para.push(Inline::Footnote {
                            content: footnote_content,
                            span: Span::new(fn_start, self.pos),
                        });
                        text_start = self.pos;
                    } else if self.is_skip_group() {
                        self.skip_balanced_group();
                    } else {
                        state_stack.push(state.clone());
                    }
                }

                b'}' => {
                    // Flush pending text before restoring state
                    if !current_text.is_empty() {
                        let span = Span::new(text_start, self.pos);
                        current_para.push(make_inline(
                            &current_text,
                            &state,
                            span,
                            &self.color_table,
                            &self.font_table,
                        ));
                        current_text.clear();
                    }
                    text_start = self.pos;
                    // Restore parent group's state
                    if let Some(prev) = state_stack.pop() {
                        state = prev;
                    }
                    self.advance();
                }

                b'\n' | b'\r' => {
                    self.advance();
                }

                _ => {
                    if current_text.is_empty() {
                        text_start = self.pos;
                    }
                    current_text.push(codepage_to_char(self.codepage, byte));
                    self.advance();
                }
            }
        }

        // Flush remaining
        if !current_text.is_empty() {
            let span = Span::new(text_start, self.pos);
            current_para.push(make_inline(
                &current_text,
                &state,
                span,
                &self.color_table,
                &self.font_table,
            ));
        }
        // Flush any pending table row / table before flushing the final paragraph.
        if tbl.in_table && !current_para.is_empty() {
            // Unterminated last cell: treat current_para as the last cell.
            tbl.current_row.push(std::mem::take(&mut current_para));
        }
        if !tbl.current_row.is_empty() {
            tbl.table_rows.push(TableRow {
                cells: std::mem::take(&mut tbl.current_row),
                span: Span::new(tbl.row_start, self.pos),
            });
        }
        tbl.flush_table(&mut paragraphs, self.pos);
        if !current_para.is_empty() {
            paragraphs.push(Block::Paragraph {
                inlines: merge_text_inlines(current_para),
                align: current_align,
                para_props: current_para_props,
                span: Span::new(para_start, self.pos),
            });
        }

        // Normalize: merge adjacent Text nodes in every paragraph so that
        // round-trip (parse → emit → parse) yields a structurally identical
        // AST regardless of how text happened to be flushed during parsing.
        paragraphs
            .into_iter()
            .map(|b| match b {
                Block::Paragraph {
                    inlines,
                    align,
                    para_props,
                    span,
                } => Block::Paragraph {
                    inlines: merge_text_inlines(inlines),
                    align,
                    para_props,
                    span,
                },
                other => other,
            })
            .collect()
    }

    /// Skip past the `\rtf1` header word so we start processing at the
    /// document body.
    fn skip_rtf_header(&mut self) {
        let pattern = b"\\rtf";
        if let Some(pos) = self.input.windows(pattern.len()).position(|w| w == pattern) {
            self.pos = pos;
            // Skip `\rtf1` (or `\rtf`) word
            while self.pos < self.input.len() {
                match self.current_byte() {
                    Some(b' ') | Some(b'\\') | Some(b'{') => break,
                    _ => self.advance(),
                }
            }
            // Skip the trailing space delimiter if present
            if self.current_byte() == Some(b' ') {
                self.advance();
            }
        }
    }

    /// Return `true` if the current position is the start of a group that
    /// should be skipped wholesale (font table, color table, picture data, etc.)
    fn is_skip_group(&self) -> bool {
        let rest = &self.input[self.pos..];

        // `{\*\...}` destination groups
        if rest.starts_with(b"\\*") {
            return true;
        }

        const SKIP_PREFIXES: &[&[u8]] = &[
            b"\\fonttbl",
            b"\\colortbl",
            b"\\stylesheet",
            b"\\info",
            b"\\pict",
            b"\\object",
            b"\\header",
            b"\\footer",
            b"\\headerl",
            b"\\headerr",
            b"\\footerl",
            b"\\footerr",
            b"\\fldinst",
            // Annotations: skip so they don't bleed into main text.
            // (Footnotes/endnotes are now handled by is_footnote_group().)
            b"\\annotation",
            // Table of contents / index entry markers
            b"\\tc",
            b"\\xe",
            // List override / numbering tables
            b"\\listoverridetable",
            b"\\listtable",
        ];

        SKIP_PREFIXES.iter().any(|p| rest.starts_with(p))
    }

    /// Skip a balanced `{...}` group.  Called when `pos` is just past the
    /// opening `{` (i.e. we have already consumed the `{`).
    fn skip_balanced_group(&mut self) {
        let mut depth = 1usize;
        while self.pos < self.input.len() && depth > 0 {
            match self.current_byte() {
                Some(b'{') => {
                    depth += 1;
                    self.advance();
                }
                Some(b'}') => {
                    depth -= 1;
                    self.advance();
                }
                Some(b'\\') => {
                    self.advance(); // skip '\'
                    if self.pos < self.input.len() {
                        self.advance(); // skip next byte (control symbol or start of word)
                    }
                }
                _ => self.advance(),
            }
        }
    }

    /// Return `true` if the current position (already past `{`) begins a
    /// `\footnote` or `\endnote` group that we should parse rather than skip.
    fn is_footnote_group(&self) -> bool {
        let rest = &self.input[self.pos..];
        // Must start with \footnote or \endnote followed by a word boundary
        for prefix in [b"\\footnote".as_slice(), b"\\endnote".as_slice()] {
            if rest.starts_with(prefix) {
                let after = rest.get(prefix.len()).copied();
                // word boundary: space, \, {, }, digit, or end
                if matches!(
                    after,
                    None | Some(b' ') | Some(b'\\') | Some(b'{') | Some(b'}')
                ) || after.map(|b| b.is_ascii_digit()).unwrap_or(false)
                {
                    return true;
                }
            }
        }
        false
    }

    /// Extract the current balanced group (caller has already consumed `{`)
    /// as a byte slice, advancing past the closing `}`.
    /// Returns the group content (between `{` and `}`), not including the braces.
    fn extract_balanced_group(&mut self) -> &'a [u8] {
        let content_start = self.pos;
        let mut depth = 1usize;
        while self.pos < self.input.len() && depth > 0 {
            match self.current_byte() {
                Some(b'{') => {
                    depth += 1;
                    self.advance();
                }
                Some(b'}') => {
                    depth -= 1;
                    if depth == 0 {
                        let content_end = self.pos;
                        self.advance(); // consume '}'
                        return &self.input[content_start..content_end];
                    }
                    self.advance();
                }
                Some(b'\\') => {
                    self.advance();
                    if self.pos < self.input.len() {
                        self.advance();
                    }
                }
                _ => self.advance(),
            }
        }
        &self.input[content_start..self.pos]
    }

    /// Parse a `{\footnote ...}` group (caller has already consumed `{`).
    /// Returns the parsed blocks and advances past the closing `}`.
    fn parse_footnote_group(&mut self) -> Vec<Block> {
        let group_bytes = self.extract_balanced_group();
        // group_bytes starts with `\footnote` or `\endnote`; skip that word
        let mut sub = Parser::with_tables(
            group_bytes,
            self.color_table.clone(),
            self.font_table.clone(),
            self.codepage,
        );
        // Skip the opening control word (\footnote or \endnote)
        if sub.current_byte() == Some(b'\\') {
            sub.advance(); // skip '\'
            // skip the word itself
            while sub
                .current_byte()
                .map(|b| b.is_ascii_alphabetic())
                .unwrap_or(false)
            {
                sub.advance();
            }
            // skip optional trailing space
            if sub.current_byte() == Some(b' ') {
                sub.advance();
            }
        }
        sub.run()
    }

    fn read_control_word(&mut self) -> (String, Option<i32>) {
        let mut word = String::new();
        while self.pos < self.input.len() {
            match self.current_byte() {
                Some(b) if b.is_ascii_alphabetic() => {
                    word.push(b as char);
                    self.advance();
                }
                _ => break,
            }
        }

        let mut negative = false;
        if self.current_byte() == Some(b'-') {
            negative = true;
            self.advance();
        }

        let mut num = String::new();
        while self.pos < self.input.len() {
            match self.current_byte() {
                Some(b) if b.is_ascii_digit() => {
                    num.push(b as char);
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
        if self.current_byte() == Some(b' ') {
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
        current_align: &mut Align,
        current_para_props: &mut String,
        tbl: &mut TableAccum,
    ) {
        match word {
            // \binN — N raw binary bytes follow; skip them entirely.
            // Without this, the parser reads binary image/OLE data as RTF text,
            // producing spurious control words from stray 0x5C bytes in the payload.
            "bin" => {
                // \binN — N raw binary bytes follow; skip them entirely.
                // With the byte-level parser this is trivially correct.
                let n = param.unwrap_or(0).max(0) as usize;
                self.pos = (self.pos + n).min(self.input.len());
            }

            "par" | "pard" => {
                if !current_text.is_empty() {
                    let span = Span::new(*text_start, self.pos);
                    current_para.push(make_inline(current_text, state, span, &self.color_table, &self.font_table));
                    current_text.clear();
                }
                if tbl.in_table {
                    // Inside a table: \par is just a soft paragraph break within
                    // the cell — don't push a Block::Paragraph.  Leave current_para
                    // in place so its inlines become part of the cell on \cell.
                    if word == "pard" {
                        // \pard exits the table context for this paragraph.
                        tbl.in_table = false;
                        // Flush accumulated table rows as a Block::Table.
                        tbl.flush_table(paragraphs, self.pos);
                        // current_para was cell content — discard it (table is already
                        // pushed; any text was not part of a complete cell).
                        current_para.clear();
                        *state = TextState::default();
                        *current_align = Align::Default;
                    }
                } else {
                    if !current_para.is_empty() {
                        paragraphs.push(Block::Paragraph {
                            inlines: merge_text_inlines(std::mem::take(current_para)),
                            align: *current_align,
                            para_props: std::mem::take(current_para_props),
                            span: Span::new(*para_start, self.pos),
                        });
                    } else {
                        current_para_props.clear();
                    }
                    if word == "pard" {
                        *state = TextState::default();
                        *current_align = Align::Default;
                    }
                }
                *text_start = self.pos;
                *para_start = self.pos;
            }

            // ── Table control words ──────────────────────────────────────────
            "intbl" => {
                if !tbl.in_table {
                    tbl.in_table = true;
                    if tbl.table_rows.is_empty() && tbl.current_row.is_empty() {
                        tbl.table_start = self.pos;
                    }
                    tbl.row_start = self.pos;
                }
            }

            "cell" => {
                // End of a table cell: flush current text + inlines into the cell.
                if !current_text.is_empty() {
                    let span = Span::new(*text_start, self.pos);
                    current_para.push(make_inline(current_text, state, span, &self.color_table, &self.font_table));
                    current_text.clear();
                }
                tbl.current_row.push(std::mem::take(current_para));
                *text_start = self.pos;
                *para_start = self.pos;
            }

            "row" => {
                // End of a table row: push the accumulated cells as a TableRow.
                if !tbl.current_row.is_empty() || !current_para.is_empty() {
                    // Handle case where last cell wasn't explicitly closed.
                    if !current_para.is_empty() {
                        tbl.current_row.push(std::mem::take(current_para));
                    }
                    tbl.table_rows.push(TableRow {
                        cells: std::mem::take(&mut tbl.current_row),
                        span: Span::new(tbl.row_start, self.pos),
                    });
                    tbl.row_start = self.pos;
                }
            }

            "line" => {
                if !current_text.is_empty() {
                    let span = Span::new(*text_start, self.pos);
                    current_para.push(make_inline(current_text, state, span, &self.color_table, &self.font_table));
                    current_text.clear();
                }
                *text_start = self.pos;
                current_para.push(Inline::LineBreak {
                    span: Span::new(self.pos, self.pos),
                });
            }

            "b" => {
                flush_text(
                    current_text,
                    text_start,
                    self.pos,
                    state,
                    current_para,
                    &self.color_table,
                    &self.font_table,
                );
                state.bold = param.unwrap_or(1) != 0;
            }
            "i" => {
                flush_text(
                    current_text,
                    text_start,
                    self.pos,
                    state,
                    current_para,
                    &self.color_table,
                    &self.font_table,
                );
                state.italic = param.unwrap_or(1) != 0;
            }
            "ul" | "uld" | "uldb" | "ulw" => {
                flush_text(
                    current_text,
                    text_start,
                    self.pos,
                    state,
                    current_para,
                    &self.color_table,
                    &self.font_table,
                );
                state.underline = param.unwrap_or(1) != 0;
            }
            "ulnone" => {
                flush_text(
                    current_text,
                    text_start,
                    self.pos,
                    state,
                    current_para,
                    &self.color_table,
                    &self.font_table,
                );
                state.underline = false;
            }
            "strike" => {
                flush_text(
                    current_text,
                    text_start,
                    self.pos,
                    state,
                    current_para,
                    &self.color_table,
                    &self.font_table,
                );
                state.strikethrough = param.unwrap_or(1) != 0;
            }
            "super" => {
                flush_text(
                    current_text,
                    text_start,
                    self.pos,
                    state,
                    current_para,
                    &self.color_table,
                    &self.font_table,
                );
                state.superscript = true;
                state.subscript = false;
            }
            "sub" => {
                flush_text(
                    current_text,
                    text_start,
                    self.pos,
                    state,
                    current_para,
                    &self.color_table,
                    &self.font_table,
                );
                state.subscript = true;
                state.superscript = false;
            }
            "nosupersub" => {
                flush_text(
                    current_text,
                    text_start,
                    self.pos,
                    state,
                    current_para,
                    &self.color_table,
                    &self.font_table,
                );
                state.superscript = false;
                state.subscript = false;
            }

            // Alignment
            "ql" => {
                flush_text(
                    current_text,
                    text_start,
                    self.pos,
                    state,
                    current_para,
                    &self.color_table,
                    &self.font_table,
                );
                *current_align = Align::Left;
            }
            "qr" => {
                flush_text(
                    current_text,
                    text_start,
                    self.pos,
                    state,
                    current_para,
                    &self.color_table,
                    &self.font_table,
                );
                *current_align = Align::Right;
            }
            "qc" => {
                flush_text(
                    current_text,
                    text_start,
                    self.pos,
                    state,
                    current_para,
                    &self.color_table,
                    &self.font_table,
                );
                *current_align = Align::Center;
            }
            "qj" | "qd" => {
                flush_text(
                    current_text,
                    text_start,
                    self.pos,
                    state,
                    current_para,
                    &self.color_table,
                    &self.font_table,
                );
                *current_align = Align::Justify;
            }

            // Font size (half-points)
            "fs" => {
                flush_text(
                    current_text,
                    text_start,
                    self.pos,
                    state,
                    current_para,
                    &self.color_table,
                    &self.font_table,
                );
                state.font_size = param.unwrap_or(0).max(0) as u16;
            }

            // Foreground color index
            "cf" => {
                flush_text(
                    current_text,
                    text_start,
                    self.pos,
                    state,
                    current_para,
                    &self.color_table,
                    &self.font_table,
                );
                state.color_idx = param.unwrap_or(0).max(0) as u8;
            }

            "caps" => {
                flush_text(
                    current_text,
                    text_start,
                    self.pos,
                    state,
                    current_para,
                    &self.color_table,
                    &self.font_table,
                );
                state.all_caps = param.unwrap_or(1) != 0;
            }
            "scaps" => {
                flush_text(
                    current_text,
                    text_start,
                    self.pos,
                    state,
                    current_para,
                    &self.color_table,
                    &self.font_table,
                );
                state.small_caps = param.unwrap_or(1) != 0;
            }
            // \v = hidden text; \webhidden = hidden in web view — treat both as hidden.
            "v" | "webhidden" => {
                flush_text(
                    current_text,
                    text_start,
                    self.pos,
                    state,
                    current_para,
                    &self.color_table,
                    &self.font_table,
                );
                state.hidden = param.unwrap_or(1) != 0;
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
                    if self.current_byte() == Some(b'?') {
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

            // \plain resets all character formatting to default.
            "plain" => {
                flush_text(
                    current_text,
                    text_start,
                    self.pos,
                    state,
                    current_para,
                    &self.color_table,
                    &self.font_table,
                );
                *state = TextState::default();
            }

            // Paragraph-layout words: captured verbatim into para_props for
            // raw preservation so the emitter can re-emit them without loss.
            // These are paragraph-scoped (appear between \pard and \par).
            "li" | "fi" | "ri" | "rin" | "lin"      // left/first-line/right indent
            | "sa" | "sb" | "sl" | "slmult"          // space after/before/line-spacing
            // Tab stops and their alignment modifiers
            | "tx" | "tqr" | "tqc" | "tqdec" | "tbin" | "tbpos"
            // Keep-with-next / orphan control
            | "keep" | "keepn" | "widctlpar" | "nowidctlpar"
            // Paragraph direction
            | "ltrpar" | "rtlpar"
            // Layout hints (rendering only, but still paragraph-scoped)
            | "adjustright" | "wrapdefault" | "faauto"
            | "nooverflow" | "noline" | "sbys" | "hyphpar"
            // Structure
            | "outlinelevel" | "itap"
            // Paragraph borders
            | "brdrb" | "brdrs" | "brdrw" | "brsp" | "brdrt" | "brdrl" | "brdrr"
            | "brdrth" | "brdrdot" | "brdrdash" | "brdrnone" | "brdrcf"
            | "brdrhair" | "brdrnil" | "brdroutset" | "brdrdb" | "brdrtriple" | "box"
            // List spacing / indent
            | "lisa" | "lisb" | "ipgp"
            // Page / paragraph flow
            | "pagebb" | "notabind"
            // Tab stop leaders and alignment variants
            | "tl" | "tql" | "tlul" | "tlhyph"
            // Paragraph direction / widow control
            | "pardirnatural" | "widowctl" => {
                current_para_props.push_str(&format_para_word(word, param));
            }

            // Character-layout words: accumulated verbatim in char_props for lossless
            // re-emission.  These have no cross-format semantic equivalent.
            // Words with params: skip if param == 0 (means "off").
            "dn" | "up" | "shading" | "expnd" | "expndtw" | "kerning" | "charscalex"
            | "chcfpat" | "chcbpat" | "chshdng" | "highlight" | "cfpat" | "chbrdr" => {
                if param.is_some_and(|n| n != 0) {
                    flush_text(
                        current_text,
                        text_start,
                        self.pos,
                        state,
                        current_para,
                        &self.color_table,
                        &self.font_table,
                    );
                    state.char_props.push_str(&format_para_word(word, param));
                }
            }
            // Binary char-layout flags (no param — always accumulate when seen).
            "shad" | "jcompress" | "jexpand" => {
                flush_text(
                    current_text,
                    text_start,
                    self.pos,
                    state,
                    current_para,
                    &self.color_table,
                    &self.font_table,
                );
                state.char_props.push_str(&format_para_word(word, None));
            }

            // Font index: \f<N> selects a font from the font table
            "f" => {
                flush_text(
                    current_text,
                    text_start,
                    self.pos,
                    state,
                    current_para,
                    &self.color_table,
                    &self.font_table,
                );
                state.font_idx = param.unwrap_or(0).max(0) as u16;
            }

            // Background (highlight) color index: \cb<N>
            "cb" => {
                flush_text(
                    current_text,
                    text_start,
                    self.pos,
                    state,
                    current_para,
                    &self.color_table,
                    &self.font_table,
                );
                state.bg_color_idx = param.unwrap_or(0).max(0) as u8;
            }
            // Language tag: \lang<N> and \langfe<N> (complex-script language)
            "lang" | "langfe" => {
                flush_text(
                    current_text,
                    text_start,
                    self.pos,
                    state,
                    current_para,
                    &self.color_table,
                    &self.font_table,
                );
                state.lang_id = param.unwrap_or(0).max(0) as u16;
            }


            // Ignored formatting / sizing controls
            "pntext" | "pn" | "pnlvlblt" | "rtf" | "ansi" | "mac" | "pc" | "pca" | "deff"
            | "deflang" | "widowctrl" | "hyphauto" | "hyphconsec" | "hyphcaps" | "paperw"
            | "paperh" | "margl" | "margr" | "margt" | "margb" | "cols" | "colsx"
            | "endhere" | "headerl" | "headerr" | "header" | "footer"
            | "footerl" | "footerr" | "trowd" | "cellx" | "trgaph"
            | "trql" | "trqr" | "trqc" | "b0"
            | "i0"
            // Revision tracking / session IDs (no semantic content)
            | "insrsid" | "charrsid" | "delrsid" | "rsidroot" | "rsid" | "pararsid"
            // Font color space / associated font / complex script (character-level)
            | "fcs" | "af" | "afs" | "afcs" | "ab" | "ai" | "loch" | "hich" | "dbch"
            | "ltrch" | "rtlch" | "alang" | "faroman" | "cs" | "ds" | "ts"
            // Row/section direction (not paragraph-level)
            | "ltrrow" | "rtlrow" | "ltrmark" | "rtlmark"
            // Paragraph style / misc (kept in ignored: complex or unclear scope)
            | "s" | "aspnum" | "aspalpha"
            | "snext" | "styrsid" | "qnatural" | "noproof"
            // Border controls (cell-level, not paragraph-level)
            | "clbrdrl" | "clbrdrt" | "clbrdrr" | "clbrdrb"
            | "clftsWidth" | "clwWidth" | "clvertalt" | "clvertalb" | "clvertalc"
            | "clshdrawnil" | "clpadl" | "clpadr" | "clpadt" | "clpadb" | "brdrtbl"
            | "clpadfl" | "clpadfr" | "clpadft" | "clpadfb" | "clcbpat" | "clcbpatraw"
            | "cltxlrtb" | "cltxtbrl" | "cltxbtlr" | "cltxlrtbv" | "cltxtbrlv" | "clNoWrap"
            // Cell / table
            | "clvmgf" | "clvmrg" | "tcelld"
            | "trpadl" | "trpadr" | "trpadt" | "trpadb" | "trrh" | "trleft" | "tblind"
            | "tblindtype" | "trautofit" | "irow" | "irowband"
            | "trpadfl" | "trpadfr" | "trpadft" | "trpadfb"
            | "trspfl" | "trspfr" | "trspft" | "trspfb"
            | "trspdl" | "trspdr" | "trspdt" | "trspdb"
            | "trftsWidthB" | "trftsWidth" | "trwWidthB" | "trwWidth"
            | "trftsWidthA" | "trwWidthA" | "trkeep" | "trkeepfollow"
            | "trhdr" | "lastrow" | "taprtl"
            // Page / section properties
            | "pgwsxn" | "pghsxn" | "sect" | "sectd" | "sbknone" | "sbkcol"
            | "sbkeven" | "sbkodd" | "sbkpage" | "pgncont" | "pgnstarts" | "pgdec"
            | "cgrid" | "langnp" | "langfenp" | "page"
            // Absolute positioning
            | "posx" | "posy" | "absw" | "absh" | "phpg" | "pvpg" | "phcol" | "phmrg"
            | "pvmrg" | "pvpara" | "posxc" | "posxl" | "posxr" | "posyb" | "posyt"
            | "posyin" | "posyc" | "dxfrtext" | "dfrmtxtx" | "dfrmtxty" | "dfrmtxtl"
            | "dfrmtxtr" | "nowrap"
            // Paragraph numbering
            | "pnrpnbr" | "pnrnfc" | "pnrstart" | "pnrindent" | "pnrhang"
            | "pnrrgb" | "pnrxst" | "pnrstop" | "ls" | "ilvl" | "jclisttab"
            | "listtext" | "stextflow"
            // Table row/border
            | "trpaddfl" | "trpaddfr" | "trpaddft" | "trpaddfb"
            | "trpaddl" | "trpaddr" | "trpaddt" | "trpaddb"
            | "trbrdrb" | "trbrdrt" | "trbrdrr" | "trbrdrl" | "trbrdrv" | "trbrdrh"
            // Misc layout / section
            | "linex" | "linemod" | "sectdefaultcl" | "endnhere" | "rtlgutter"
            | "sftnbj"
            | "shp" | "shprslt" | "nonesttables" | "tblrsid" | "sectrsid" | "tldot"
            | "footery" | "headery" | "pnrdate" | "pnrauth"
            | "softline" | "clshdng" | "clcfpat" | "clmrg" | "yts"
            // NOTE: \caps, \scaps, \v (hidden), \webhidden, \up, \dn are SEMANTIC
            // (all-caps, small-caps, hidden text, baseline offset) and intentionally
            // NOT in this list — they fall through to the diagnostic arm so the caller
            // knows fidelity was lost.
            | "cchs" | "flddirty" | "fldedit" | "ftnbj"
            | "sectlinegrid" | "psz" | "margbsxn" | "margtsxn" | "margrsxn" | "marglsxn"
            | "nestcell" | "clshdngraw" | "ansicpg" | "aenddoc"
            | "ltrsect" | "deflangfe" | "revised" | "ppscheme"
            | "tbllkhdrrows" | "tbllkhdrcols" | "tbllklastcol" | "tbllklastrow" | "saftnnar"
            | "pgnhn" | "pgnlcrm" | "lndscpsxn" | "sbauto" | "saauto"
            | "pgnrestart" | "cbpat" | "tcf" | "tcl"
            // Drawing grid
            | "dghshow" | "dghorigin" | "dghspace" | "dgvshow" | "dgvorigin" | "dgvspace"
            | "horzdoc" | "vertdoc"
            // Typography hints (no semantic content — affect spacing/rendering only)
            // NOTE: \up/\dn are SEMANTIC (baseline offset) and not in this list.
            // Underline color / char properties
            | "ulc" | "nospaceforul" | "noultrlspc" | "expshrtn" | "noxlattoyen"
            | "nolnhtadjtbl" | "titlepg" | "cufi"
            // Style sheet font refs
            | "stshfloch" | "stshfdbch" | "stshfbi" | "stshfhich"
            // Footnote / misc refs (skip group handles content; word itself is ignored)
            | "chftn" | "fldpriv" | "nonshppict"
            // Section / column layout
            | "trspdfr" | "trspdfl" | "trspdfb" | "trspdft" | "colno" | "colw" | "column"
            // More page numbering / annotation
            | "aftnnar" | "pgbrdrfoot" | "pgbrdrhead"
            // Noextrasprl / misc
            | "noextrasprl" | "htmautsp" | "dntblnsbdb" | "useltbaln" | "fet"
            | "formshade" | "viewkind" | "viewscale" | "viewzk"
            | "uc" | "ud" | "field"
            // Table cell structure (cell-level, not paragraph-level)
            | "clmgf" | "clcfpatraw"
            // Section / footnote structure
            | "ftnrestart" | "endnrestart" | "aftnrestart"
            // Page number absolute positioning (document-level layout)
            | "pgnx" | "pgny"
            // Asian typography document settings
            | "adeflang" | "adeff" | "colsr" | "ilfomacatclnup" | "dgmargin"
            // Word compatibility / document-level flags
            | "lnbrkrule" | "alntblind" | "lyttblrtgr" | "lytcalctblwd"
            | "splytwnine" | "ftnlytwnine" | "grfdocevents"
            | "ignoremixedcontent" | "saveinvalidxml" | "donotembedlingdata"
            | "showplaceholdtext" | "showxmlerrors"
            // Page numbering format
            | "pgndec"
            // Layout / print compat flags
            | "lytprtmet" | "subfontbysize" | "donotembedsysfont" | "validatexml"
            | "allowfieldendsel" | "nobrkwrptbl"
            // Document geometry
            | "gutter"
            // Hyphenation
            | "hyphhotz"
            // Field flags
            | "fldlock"
            // Table look flags
            | "tbllkfont" | "tbllkborder" | "tbllkshading" | "tbllkbestfit" | "tbllkcolor"
            // Text wrap (floating objects)
            | "wraptrsp" | "wraparound"
            // Shape object properties (name/value pairs inside \shp groups)
            | "sp" | "sn" | "sv"
            // Equation / EQ field keywords (single lowercase letter — inside \field groups)
            // Note: uppercase-letter EQ keywords (T, Y, V, etc.) never reach here;
            // they are silently consumed by the control-symbol handler (\+uppercase → ignored).
            | "c" | "r" | "m" | "o" | "k" | "d" | "n" | "q" | "j" | "z"
            | "x" | "y" | "h" | "t" | "a"
            // Asian typography compat flags
            | "wrppunct" | "asianbrkrule" | "snaptogridincell" | "sprslnsp" | "nojkernpunct"
            // Section / document properties
            | "nocolbal" | "pgbrdropt" | "deftab" | "facingp" | "ftnrstpg"
            | "binsxn" | "binfsxn" | "newtblstyruls" | "transmf" | "brkfrm"
            | "truncatefontheight" | "abslock" | "pnrnot" | "pgnstart" | "chatn"
            | "vertalc" | "rempersonalinfo" | "revisions" | "fracwidth" | "makebackup"
            | "prcolbl" | "cvmme" | "wpjst"
            // Footnote / endnote restart and numbering
            | "sftnrestart" | "ftnrstcont" | "ftnnrlc"
            // Compat / document flags
            | "oldas" | "otblrul" | "truncex" | "linkstyles" | "enddoc"
            | "donotshowmarkup" | "margmirror" | "noqfpromote" | "viewnobound"
            | "relyonvml" | "enforceprot" | "protlevel" | "formprot"
            // Track changes flags
            | "trackformatting" | "trackmoves" | "revbar"
            // Author / date info (info group words)
            | "prdate" | "prauth" | "crdate"
            // Shape object geometry and positioning
            | "shpgrp" | "shpwrk" | "shpleft" | "shpwr" | "shpbottom" | "shpright"
            | "shptop" | "shpinst" | "shpbxmargin" | "msmcap"
            // Extended text-box / absolute positioning
            | "tposxr" | "tposyc" | "absnoovrlp"
            // Theme / language
            | "themelang"
            // Table frame distances (floating table text wrap offsets)
            | "tdfrmtxtLeft" | "tdfrmtxtRight" | "tdfrmtxtTop" | "tdfrmtxtBottom"
            | "nogrowautofit"
            // Extended absolute positioning
            | "tposy" | "posnegy" | "tposx" | "posnegx" | "tphmrg" | "tpvpara" | "tpvpg"
            // Revision tracking (we don't model change-tracking; words appear in tracked groups)
            | "deleted" | "revdttmdel" | "revauthdel" | "revauth" | "revdttm"
            // Unicode representation group (ANSI fallback — complex; skip word, content flows through)
            | "upr"
            // Border styles not yet in para_props list
            | "brdrthtnsg" | "brdrthtnmg" | "brdrthtnlg" | "brdrtnthmg" | "brdrtnthsg"
            | "brdrtnthtnsg" | "brdrtnthtnmg" | "brdrtnthtnlg" | "brdrtnthlg"
            | "brdrdashd" | "brdrdashdd" | "brdrdashsm" | "brdrwavydb" | "brdrwavy"
            | "brdremboss" | "brdrengrave" | "brdrframe" | "bdrrlswsix"
            | "brdrsh" | "brdrbtw" | "swpbdr"
            // Paragraph spacing / flow suppressions
            | "contextualspace" | "sprstsp" | "sprsspbf" | "sprsbsp" | "nolead" | "sprstsm"
            // Tab / absolute positioning para properties
            | "tabsnoovrlp"
            // Theme language variants
            | "themelangcs" | "themelangfe"
            // Page borders (all sides)
            | "pgbrdrl" | "pgbrdrr" | "pgbrdrt" | "pgbrdrb"
            // Section / document protection flags
            | "annotprot" | "sectunlocked" | "revprot" | "readprot" | "stylelockenforced"
            // Page / section geometry
            | "landscape" | "guttersxn" | "marg"
            // Text box / shape absolute positioning
            | "tposxc" | "tposnegy" | "tposnegx" | "shpbymargin" | "shpbxpage"
            | "shpbypage" | "shpbyignore" | "shpfblwtxt" | "shplid"
            // Document view settings
            | "vieww" | "viewh" | "viewbksp"
            // Footnote continuation / separator (destination groups handled structurally)
            | "ftncn" | "ftnsep" | "ftnsepc" | "chftnsep" | "ftntj" | "ftnstart"
            | "sftntj" | "aendnotes"
            // Cocoa (macOS) RTF extensions — version markers
            | "cocoartf" | "cocoasubrtf"
            // Revision author (change-tracking metadata)
            | "crauth"
            // Drawing object primitives (inside \do / \dposp groups)
            | "do" | "dpptx" | "dppty" | "dpxsize" | "dpy" | "dpline"
            | "dplinecob" | "dplinecor" | "dplinecog" | "dpx" | "dodhgt" | "dobxpage"
            // Drawing object fill/line properties (\do group, no semantic content)
            | "dpfillfgcb" | "dpfillfgcr" | "dpfillfgcg" | "dpfillbgcb" | "dpfillbgcr" | "dpfillbgcg"
            | "dplinehollow" | "dropcapt" | "dropcapli"
            // Document flags
            | "donotshowprops"
            // Remaining rare shape / draw / section words
            | "shpbypara" | "shpbxignore" | "shpz" | "shpfhdr"
            | "dpysize" | "dplinew" | "dobypage"
            // Asian typography control words (CJK-specific, no cross-format semantic equivalent)
            | "mm" | "chm" | "duj" | "ffc" | "jdc" | "iov" | "qx" | "qcy"
            | "iy" | "kh" | "kf" | "hf" | "of" | "nc" | "vv" | "fq" | "yoq"
            | "ixr" | "jr" | "hui" | "xi" | "kuz" | "yk" | "zyf" | "cj" | "rj"
            | "dd" | "mc" | "xv" | "juc" | "ommu"
            // Single-letter Asian/CJK layout words (\g, \e, \l, \w, \p — different from EQ field letters)
            // Note: these overlap with the EQ field single-letter words below; both are no-ops here.
            | "g" | "e" | "l" | "w" | "p"
            // Additional Asian/CJK typography (less common, found in govdocs1 corpus)
            | "embo" | "qe" | "lw" | "jl" | "np" | "owy" | "ya" | "jx" | "yd" | "jhu"
            | "cd" | "su" | "emjk" | "etz" | "gp" | "mte" | "aiu" | "ids" | "qdl" | "kb"
            | "bbr" | "lm" | "my" | "fm" | "ka" | "ytkn" | "ma" | "ejo" | "rdlnjs" | "wg"
            | "utr" | "dmzq" | "vc" | "icpm" | "vn" | "biyypj" | "te" | "fz"
            // Drawing object fill pattern
            | "dpfillpat"
            // \tc as bare word (content handled as skip group when in braces)
            | "tc"
            // Additional rare Asian/CJK words found in corpus
            | "cky" | "mv" | "clols" | "ns" | "az" | "elqc" | "htw" | "ed" | "dh" | "pv"
            | "qt" | "zf" | "rqn" | "lh" | "je" | "dk" | "hp" | "pi" | "qb" | "yi"
            | "hyti" | "em" | "lqt" | "yw"
            // Form display control (document-level, no semantic content)
            | "formdisp"
            // Page numbering variants
            | "pgnid" | "pgnhnsm"
            // Footnote / endnote numbering
            | "aftnnrlc"
            // Style lock
            | "stylelock"
            // Language tags
            | "ksulang"
            // Revision metadata
            | "srauth"
            // Inline text / Unicode flags
            | "utinl"
            // From-text marker (document-level)
            | "fromtext"
            => {
                // Most toggle-off words are redundant after state flush above,
                // but b0/i0 specifically turn off formatting
                if word == "b0" {
                    flush_text(
                        current_text,
                        text_start,
                        self.pos,
                        state,
                        current_para,
                        &self.color_table,
                        &self.font_table,
                    );
                    state.bold = false;
                } else if word == "i0" {
                    flush_text(
                        current_text,
                        text_start,
                        self.pos,
                        state,
                        current_para,
                        &self.color_table,
                        &self.font_table,
                    );
                    state.italic = false;
                }
            }

            _ => {
                // Only emit a diagnostic for all-lowercase unknown words — those
                // might be real RTF words we haven't categorised yet.  Words with
                // uppercase letters in the middle (e.g. \kR, \dOx) are binary data
                // bleeding through the parser, not real control words.
                if word.bytes().all(|b| b.is_ascii_lowercase()) {
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
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Format a paragraph-layout control word for raw accumulation in `para_props`.
///
/// Produces `\word` (no param) or `\wordN` (with integer param).
fn format_para_word(word: &str, param: Option<i32>) -> String {
    match param {
        Some(n) => format!("\\{word}{n}"),
        None => format!("\\{word}"),
    }
}

fn flush_text(
    current_text: &mut String,
    text_start: &mut usize,
    pos: usize,
    state: &TextState,
    current_para: &mut Vec<Inline>,
    color_table: &[(u8, u8, u8)],
    font_table: &[String],
) {
    if !current_text.is_empty() {
        let span = Span::new(*text_start, pos);
        current_para.push(make_inline(
            current_text,
            state,
            span,
            color_table,
            font_table,
        ));
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

fn make_inline(
    text: &str,
    state: &TextState,
    span: Span,
    color_table: &[(u8, u8, u8)],
    font_table: &[String],
) -> Inline {
    let mut inline = Inline::Text {
        text: text.to_string(),
        span,
    };
    if state.small_caps {
        inline = Inline::SmallCaps {
            children: vec![inline],
            span,
        };
    }
    if state.all_caps {
        inline = Inline::AllCaps {
            children: vec![inline],
            span,
        };
    }
    if state.hidden {
        inline = Inline::Hidden {
            children: vec![inline],
            span,
        };
    }
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
    if state.font_size != 0 {
        inline = Inline::FontSize {
            size: state.font_size,
            children: vec![inline],
            span,
        };
    }
    if state.color_idx != 0 {
        let idx = state.color_idx as usize;
        if idx < color_table.len() {
            let (r, g, b) = color_table[idx];
            inline = Inline::Color {
                r,
                g,
                b,
                children: vec![inline],
                span,
            };
        }
    }
    if !state.char_props.is_empty() {
        inline = Inline::CharSpan {
            char_props: state.char_props.clone(),
            children: vec![inline],
            span,
        };
    }
    if state.bg_color_idx != 0 {
        let idx = state.bg_color_idx as usize;
        if idx < color_table.len() {
            let (r, g, b) = color_table[idx];
            inline = Inline::BgColor {
                r,
                g,
                b,
                children: vec![inline],
                span,
            };
        }
    }
    if state.font_idx != 0 {
        let idx = state.font_idx as usize;
        if idx < font_table.len() {
            let name = font_table[idx].clone();
            inline = Inline::Font {
                name,
                children: vec![inline],
                span,
            };
        }
    }
    if state.lang_id != 0 {
        inline = Inline::Lang {
            lcid: state.lang_id,
            children: vec![inline],
            span,
        };
    }
    inline
}

/// Pre-scan the input for a `\colortbl` group and parse its RGB entries.
/// Index 0 is the auto color (no RGB), subsequent entries are RGB triples.
fn parse_color_table(input: &[u8]) -> Vec<(u8, u8, u8)> {
    let mut colors = vec![(0u8, 0u8, 0u8)]; // index 0 = auto/default
    let pattern = b"{\\colortbl";
    let Some(start) = input.windows(pattern.len()).position(|w| w == pattern) else {
        return colors;
    };
    // Find the content of the group (just scan bytes, counting braces)
    let rest = &input[start + 1..]; // skip the opening '{'
    let mut depth = 1usize;
    let mut content: Vec<u8> = Vec::new();
    for &b in rest {
        match b {
            b'{' => {
                depth += 1;
                content.push(b);
            }
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
                content.push(b);
            }
            _ => content.push(b),
        }
    }
    // The colortbl body is always pure ASCII RTF; lossy conversion is safe.
    let content_str = String::from_utf8_lossy(&content);
    // Parse semicolon-delimited entries; first entry is index 0 (auto)
    for entry in content_str.split(';').skip(1) {
        // skip before first ';' = index 0
        // Skip empty/whitespace entries (e.g. trailing ';' at end of colortbl)
        if entry.split('\\').all(|t| t.trim().is_empty()) {
            continue;
        }
        let mut r = 0u8;
        let mut g = 0u8;
        let mut bv = 0u8;
        for token in entry.split('\\').filter(|s| !s.is_empty()) {
            if let Some(n) = token.strip_prefix("red") {
                r = n.trim().parse().unwrap_or(0);
            } else if let Some(n) = token.strip_prefix("green") {
                g = n.trim().parse().unwrap_or(0);
            } else if let Some(n) = token.strip_prefix("blue") {
                bv = n.trim().parse().unwrap_or(0);
            }
        }
        colors.push((r, g, bv));
    }
    colors
}

/// Pre-scan the input for a `\fonttbl` group and parse font names.
///
/// Returns a `Vec<String>` where index N is the name of `\fN`.  Index 0 is
/// always present (default font).  Fonts not declared are left as empty strings.
fn parse_font_table(input: &[u8]) -> Vec<String> {
    let pattern = b"{\\fonttbl";
    let Some(start) = input.windows(pattern.len()).position(|w| w == pattern) else {
        return vec![String::new()]; // index 0 = default (empty)
    };
    let rest = &input[start + 1..]; // skip opening '{'
    let mut depth = 1usize;
    let mut content: Vec<u8> = Vec::new();
    for &b in rest {
        match b {
            b'{' => {
                depth += 1;
                content.push(b);
            }
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
                content.push(b);
            }
            _ => content.push(b),
        }
    }
    let content_str = String::from_utf8_lossy(&content);
    let mut fonts: Vec<(usize, String)> = Vec::new();
    let mut i = 0usize;
    let bytes = content_str.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'{' {
            let entry_start = i + 1;
            let mut depth2 = 1usize;
            let mut j = entry_start;
            while j < bytes.len() && depth2 > 0 {
                match bytes[j] {
                    b'{' => depth2 += 1,
                    b'}' => depth2 -= 1,
                    _ => {}
                }
                j += 1;
            }
            let entry = &content_str[entry_start..j.saturating_sub(1)];
            // Find the font index from \f<N>
            let mut font_idx: Option<usize> = None;
            let mut pos = 0usize;
            let eb = entry.as_bytes();
            while pos < eb.len() {
                if eb[pos] == b'\\' {
                    pos += 1;
                    let word_start = pos;
                    while pos < eb.len() && eb[pos].is_ascii_alphabetic() {
                        pos += 1;
                    }
                    let word = &entry[word_start..pos];
                    // Read optional integer param
                    let num_start = pos;
                    if pos < eb.len() && eb[pos] == b'-' {
                        pos += 1;
                    }
                    while pos < eb.len() && eb[pos].is_ascii_digit() {
                        pos += 1;
                    }
                    if word == "f" && pos > num_start {
                        font_idx = entry[num_start..pos].parse::<usize>().ok();
                    }
                    // skip space delimiter
                    if pos < eb.len() && eb[pos] == b' ' {
                        pos += 1;
                    }
                } else {
                    pos += 1;
                }
            }
            if let Some(idx) = font_idx {
                fonts.push((idx, extract_font_name(entry)));
            }
            i = j;
        } else {
            i += 1;
        }
    }
    if fonts.is_empty() {
        return vec![String::new()];
    }
    let max_idx = fonts.iter().map(|(idx, _)| *idx).max().unwrap_or(0);
    let mut table = vec![String::new(); max_idx + 1];
    for (idx, name) in fonts {
        table[idx] = name;
    }
    table
}

/// Extract the font name from a font table entry like `\f0\froman Times New Roman;`.
///
/// Returns text after the last control word, stripped of trailing `;` and whitespace.
fn extract_font_name(entry: &str) -> String {
    let mut last_word_end = 0usize;
    let mut i = 0usize;
    let b = entry.as_bytes();
    while i < b.len() {
        if b[i] == b'\\' {
            i += 1;
            while i < b.len() && (b[i].is_ascii_alphabetic() || b[i].is_ascii_digit()) {
                i += 1;
            }
            // skip space delimiter
            if i < b.len() && b[i] == b' ' {
                i += 1;
            }
            last_word_end = i;
        } else {
            i += 1;
        }
    }
    entry[last_word_end..]
        .trim()
        .trim_end_matches(';')
        .trim()
        .to_string()
}

/// Merge consecutive `Inline::Text` nodes and recursively normalise children.
///
/// Delegates flat merging to `ast::merge_text_inlines` and recurses into
/// container nodes so every level of the tree is normalised.
fn merge_text_inlines(inlines: Vec<Inline>) -> Vec<Inline> {
    let normalised = inlines
        .into_iter()
        .map(|inline| inline.normalize())
        .collect();
    crate::ast::merge_text_inlines(normalised)
}

/// Decode a Windows-1252 byte to a Unicode char.
///
/// For bytes 0x00–0x7F it is identical to ASCII.  The range 0x80–0x9F is
/// remapped per the Windows-1252 code page.  0xA0–0xFF map to Latin-1.
/// Pre-scan the input for `\ansicpg<N>` and return the declared code page.
/// Returns 1252 (Windows Western) if not found.
fn parse_ansicpg(input: &[u8]) -> u16 {
    let needle = b"\\ansicpg";
    let Some(start) = input.windows(needle.len()).position(|w| w == needle) else {
        return 1252;
    };
    let rest = &input[start + needle.len()..];
    let digits: Vec<u8> = rest
        .iter()
        .copied()
        .take_while(|b| b.is_ascii_digit())
        .collect();
    if digits.is_empty() {
        return 1252;
    }
    std::str::from_utf8(&digits)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1252)
}

/// Decode a single byte using the given Windows code page.
///
/// Supported: 1250, 1251, 1252, 1253, 1254.
/// Unknown code pages fall back to CP1252 with a best-effort mapping.
pub(crate) fn codepage_to_char(cp: u16, byte: u8) -> char {
    match cp {
        1250 => windows1250_to_char(byte),
        1251 => windows1251_to_char(byte),
        1253 => windows1253_to_char(byte),
        1254 => windows1254_to_char(byte),
        _ => windows1252_to_char(byte),
    }
}

/// CP1250 (Central European / Polish etc.): 0x80–0x9F differ from Latin-1.
fn windows1250_to_char(byte: u8) -> char {
    #[rustfmt::skip]
    const W1250: [char; 32] = [
        '\u{20AC}', '\u{FFFD}', '\u{201A}', '\u{FFFD}',
        '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}',
        '\u{FFFD}', '\u{2030}', '\u{0160}', '\u{2039}',
        '\u{015A}', '\u{0164}', '\u{017D}', '\u{0179}',
        '\u{FFFD}', '\u{2018}', '\u{2019}', '\u{201C}',
        '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}',
        '\u{FFFD}', '\u{2122}', '\u{0161}', '\u{203A}',
        '\u{015B}', '\u{0165}', '\u{017E}', '\u{017A}',
    ];
    if (0x80..=0x9F).contains(&byte) {
        W1250[(byte - 0x80) as usize]
    } else {
        byte as char
    }
}

/// CP1251 (Cyrillic).
fn windows1251_to_char(byte: u8) -> char {
    #[rustfmt::skip]
    const W1251: [char; 128] = [
        // 0x80–0xBF
        '\u{0402}', '\u{0403}', '\u{201A}', '\u{0453}',
        '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}',
        '\u{20AC}', '\u{2030}', '\u{0409}', '\u{2039}',
        '\u{040A}', '\u{040C}', '\u{040B}', '\u{040F}',
        '\u{0452}', '\u{2018}', '\u{2019}', '\u{201C}',
        '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}',
        '\u{FFFD}', '\u{2122}', '\u{0459}', '\u{203A}',
        '\u{045A}', '\u{045C}', '\u{045B}', '\u{045F}',
        '\u{00A0}', '\u{040E}', '\u{045E}', '\u{0408}',
        '\u{00A4}', '\u{0490}', '\u{00A6}', '\u{00A7}',
        '\u{0401}', '\u{00A9}', '\u{0404}', '\u{00AB}',
        '\u{00AC}', '\u{00AD}', '\u{00AE}', '\u{0407}',
        '\u{00B0}', '\u{00B1}', '\u{0406}', '\u{0456}',
        '\u{0491}', '\u{00B5}', '\u{00B6}', '\u{00B7}',
        '\u{0451}', '\u{2116}', '\u{0454}', '\u{00BB}',
        '\u{0458}', '\u{0405}', '\u{0455}', '\u{0457}',
        // 0xC0–0xFF: Cyrillic block
        '\u{0410}', '\u{0411}', '\u{0412}', '\u{0413}',
        '\u{0414}', '\u{0415}', '\u{0416}', '\u{0417}',
        '\u{0418}', '\u{0419}', '\u{041A}', '\u{041B}',
        '\u{041C}', '\u{041D}', '\u{041E}', '\u{041F}',
        '\u{0420}', '\u{0421}', '\u{0422}', '\u{0423}',
        '\u{0424}', '\u{0425}', '\u{0426}', '\u{0427}',
        '\u{0428}', '\u{0429}', '\u{042A}', '\u{042B}',
        '\u{042C}', '\u{042D}', '\u{042E}', '\u{042F}',
        '\u{0430}', '\u{0431}', '\u{0432}', '\u{0433}',
        '\u{0434}', '\u{0435}', '\u{0436}', '\u{0437}',
        '\u{0438}', '\u{0439}', '\u{043A}', '\u{043B}',
        '\u{043C}', '\u{043D}', '\u{043E}', '\u{043F}',
        '\u{0440}', '\u{0441}', '\u{0442}', '\u{0443}',
        '\u{0444}', '\u{0445}', '\u{0446}', '\u{0447}',
        '\u{0448}', '\u{0449}', '\u{044A}', '\u{044B}',
        '\u{044C}', '\u{044D}', '\u{044E}', '\u{044F}',
    ];
    if byte >= 0x80 {
        W1251[(byte - 0x80) as usize]
    } else {
        byte as char
    }
}

/// CP1253 (Greek).
fn windows1253_to_char(byte: u8) -> char {
    #[rustfmt::skip]
    const W1253: [char; 32] = [
        '\u{20AC}', '\u{FFFD}', '\u{201A}', '\u{0192}',
        '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}',
        '\u{FFFD}', '\u{2030}', '\u{FFFD}', '\u{2039}',
        '\u{FFFD}', '\u{FFFD}', '\u{FFFD}', '\u{FFFD}',
        '\u{FFFD}', '\u{2018}', '\u{2019}', '\u{201C}',
        '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}',
        '\u{FFFD}', '\u{2122}', '\u{FFFD}', '\u{203A}',
        '\u{FFFD}', '\u{FFFD}', '\u{FFFD}', '\u{FFFD}',
    ];
    if (0x80..=0x9F).contains(&byte) {
        W1253[(byte - 0x80) as usize]
    } else {
        byte as char
    }
}

/// CP1254 (Turkish).  The 0x80–0x9F range differs from W1252 in 4 positions.
fn windows1254_to_char(byte: u8) -> char {
    #[rustfmt::skip]
    const W1254: [char; 32] = [
        '\u{20AC}', '\u{FFFD}', '\u{201A}', '\u{0192}',
        '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}',
        '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}',
        '\u{0152}', '\u{FFFD}', '\u{FFFD}', '\u{FFFD}',
        '\u{FFFD}', '\u{2018}', '\u{2019}', '\u{201C}',
        '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}',
        '\u{02DC}', '\u{2122}', '\u{0161}', '\u{203A}',
        '\u{0153}', '\u{FFFD}', '\u{FFFD}', '\u{0178}',
    ];
    if (0x80..=0x9F).contains(&byte) {
        W1254[(byte - 0x80) as usize]
    } else {
        byte as char
    }
}

pub(crate) fn windows1252_to_char(byte: u8) -> char {
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
        parse_str(input).0
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

    /// Regression: `\'XX` where XX spans a multi-byte UTF-8 char boundary
    /// used to panic with a byte-level slice.
    #[test]
    fn test_hex_escape_multibyte_boundary() {
        // bytes: \ ' p 0xD9 0x9B \  — the hex digits are 'p' + non-ASCII byte
        let s = "\\'p\u{066B}\\";
        let (doc, _) = parse_str(s); // must not panic
        // 'p' is not a valid hex digit pair so no char is emitted; that's fine
        let _ = doc;
    }

    /// Regression: raw `\t}\t` caused a roundtrip assertion failure because
    /// the `}` flushed a Text node and the re-parse merged the two tabs.
    #[test]
    fn test_roundtrip_tab_group_close() {
        let s = "\t}\t";
        let (ast1, _) = parse_str(s);
        let emitted = crate::emit::emit(&ast1);
        let (ast2, _) = parse(emitted.as_bytes());
        assert_eq!(ast1.strip_spans(), ast2.strip_spans());
    }

    #[test]
    fn test_parse_alignment() {
        let doc = p(r"{\rtf1 \qc centered text\par}");
        let Block::Paragraph { align, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert_eq!(*align, Align::Center);
    }

    #[test]
    fn test_parse_font_size() {
        let doc = p(r"{\rtf1 \fs48 big text\par}");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(
            inlines
                .iter()
                .any(|i| matches!(i, Inline::FontSize { size: 48, .. }))
        );
    }

    #[test]
    fn test_parse_color_table() {
        let doc = p(r"{\rtf1{\colortbl ;\red255\green0\blue0;}\cf1 red text\par}");
        // color_table stores only actual colors (index-0 auto is stripped)
        assert!(!doc.color_table.is_empty());
        assert_eq!(doc.color_table[0], (255, 0, 0));
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(
            i,
            Inline::Color {
                r: 255,
                g: 0,
                b: 0,
                ..
            }
        )));
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
                | Inline::Subscript { children, .. }
                | Inline::FontSize { children, .. }
                | Inline::Color { children, .. }
                | Inline::AllCaps { children, .. }
                | Inline::SmallCaps { children, .. }
                | Inline::Hidden { children, .. }
                | Inline::CharSpan { children, .. }
                | Inline::Font { children, .. }
                | Inline::BgColor { children, .. }
                | Inline::Lang { children, .. } => out.push_str(&collect_text(children)),
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
                Inline::Footnote { .. } => {} // footnote body is separate
            }
        }
        out
    }
}

#[cfg(test)]
mod font_bg_tests {
    use super::*;

    fn p(input: &str) -> RtfDoc {
        parse_str(input).0
    }

    #[test]
    fn test_parse_font_face() {
        let doc = p(r"{\rtf1{\fonttbl{\f0 Times New Roman;}{\f1 Arial;}}{\f1 Arial text}\par}");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(
            inlines
                .iter()
                .any(|i| matches!(i, Inline::Font { name, .. } if name == "Arial")),
            "expected Font{{Arial}} inline, got: {:?}",
            inlines
        );
    }

    #[test]
    fn test_parse_bg_color() {
        let doc = p(r"{\rtf1{\colortbl ;\red255\green255\blue0;}\cb1 yellow bg\par}");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(
            inlines.iter().any(|i| matches!(
                i,
                Inline::BgColor {
                    r: 255,
                    g: 255,
                    b: 0,
                    ..
                }
            )),
            "expected BgColor{{255,255,0}} inline, got: {:?}",
            inlines
        );
    }

    #[test]
    fn test_roundtrip_font_face() {
        let input = r"{\rtf1{\fonttbl{\f0 Times New Roman;}{\f1 Arial;}}{\f1 Arial text}\par}";
        let (doc1, diags) = parse_str(input);
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
        let emitted = crate::emit::emit(&doc1);
        let (doc2, _) = parse_str(&emitted);
        assert_eq!(doc1.strip_spans(), doc2.strip_spans());
    }

    #[test]
    fn test_roundtrip_bg_color() {
        let input = r"{\rtf1{\colortbl ;\red255\green255\blue0;}\cb1 yellow bg\par}";
        let (doc1, diags) = parse_str(input);
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
        let emitted = crate::emit::emit(&doc1);
        let (doc2, _) = parse_str(&emitted);
        assert_eq!(doc1.strip_spans(), doc2.strip_spans());
    }

    fn collect_text(inlines: &[Inline]) -> String {
        inlines
            .iter()
            .map(|i| match i {
                Inline::Text { text, .. } => text.clone(),
                _ => String::new(),
            })
            .collect()
    }

    #[test]
    fn test_parse_table_simple() {
        let doc = p(
            r"{\rtf1\trowd\cellx2000\cellx4000\intbl Cell 1\cell Cell 2\cell\row\pard After\par}",
        );
        assert_eq!(
            doc.blocks.len(),
            2,
            "expected table + paragraph, got: {:?}",
            doc.blocks
        );
        let Block::Table { rows, .. } = &doc.blocks[0] else {
            panic!("expected table as first block, got: {:?}", &doc.blocks[0]);
        };
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].cells.len(), 2);
        // Check cell content
        let text = collect_text(&rows[0].cells[0]);
        assert!(text.contains("Cell 1"), "cell 0: {text:?}");
        let text = collect_text(&rows[0].cells[1]);
        assert!(text.contains("Cell 2"), "cell 1: {text:?}");
    }

    #[test]
    fn test_parse_table_multi_row() {
        let doc = p(
            r"{\rtf1\trowd\cellx2000\intbl A\cell\row\trowd\cellx2000\intbl B\cell\row\pard\par}",
        );
        let Block::Table { rows, .. } = &doc.blocks[0] else {
            panic!("expected table");
        };
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn test_roundtrip_table() {
        let input = r"{\rtf1\trowd\cellx2000\cellx4000\intbl Cell 1\cell Cell 2\cell\row\pard\par}";
        let (doc1, diags) = parse_str(input);
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
        let emitted = crate::emit::emit(&doc1);
        let (doc2, _) = parse_str(&emitted);
        assert_eq!(doc1.strip_spans(), doc2.strip_spans());
    }
}
