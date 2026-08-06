//! Catcode-driven LaTeX tokenizer.
//!
//! This is the shared low-level primitive that `parse()`, `events()`, and
//! `StreamingParser` each drive independently to their own optimal output
//! shape (per `docs/format-library-design.md` — NOT `parse() =
//! events().collect()`).
//!
//! # Design
//!
//! The tokenizer mirrors real TeX's own tokenizer: it has **no built-in
//! knowledge of specific command names or their argument arity**. A control
//! sequence is always an atomic [`Tok::Cs`] token; any `{...}` groups that
//! structurally follow it are ordinary sibling/child tokens in the stream,
//! never "consumed as arguments" here — arity resolution lives only in the
//! semantic layer (`parse.rs`, `events.rs`, `batch.rs`).
//!
//! It recognizes only the constructs that genuinely change tokenization:
//! control-sequence starts (`\`), group delimiters (`{`/`}`), comments
//! (`%`), math-mode shifts (`$`/`$$`), alignment tabs (`&`), macro
//! parameters (`#<digit>`), and a narrow closed set of mode-changing
//! constructs:
//!
//! - `\verb<delim>...<delim>` / `\verb*<delim>...<delim>` — the delimiter
//!   is chosen at run time (the byte immediately following `\verb`), so the
//!   tokenizer holds it as explicit pending state while scanning for the
//!   matching close.
//! - `verbatim` / `verbatim*` / `lstlisting` environments — once the
//!   tokenizer has emitted the literal 4-token sequence
//!   `Cs("begin") GroupOpen Text(name) GroupClose` for one of those three
//!   names, backslash/braces/etc. stop being special until the literal
//!   substring `\end{<name>}` is found; that entire span becomes one
//!   [`Tok::Text`] token, and normal tokenization resumes exactly at the
//!   `\end{...}` (which is then tokenized normally, producing the
//!   `Cs("end") GroupOpen Text(name) GroupClose` the semantic layer
//!   expects — symmetric with how the environment was opened).
//!
//! **Out of scope, by design decision:** runtime `\catcode` redefinition.
//! Catcodes are the fixed, standard LaTeX defaults for the lifetime of a
//! parse. A document that reassigns catcodes mid-stream (rare outside
//! package internals) will have the reassigned region's tokenization not
//! reflect the new catcodes; the tokens produced are still well-formed and
//! round-trip losslessly as raw text, they're simply not re-interpreted
//! under the new catcode regime. Full runtime catcode tracking is a
//! meaningfully larger feature (a catcode table threaded through every
//! decision point) that this crate does not need for its target vocabulary
//! (document structure, formatting, math, tables, citations, footnotes) —
//! see TODO.md for the explicit tracking of this as a known scope
//! boundary, not a silent gap.
//!
//! Similarly, `~` (TeX catcode 13, "active") and `^`/`_` (catcodes 7/8,
//! meaningful only inside math mode) are tokenized as ordinary [`Tok::Text`]
//! characters rather than modeled as distinct catcode classes: math content
//! is captured as a raw source span by the semantic layer (matching this
//! codebase's existing `rescribe-read-latex` precedent — see
//! `crates/readers/rescribe-read-latex`), so no consumer needs the
//! tokenizer to resolve math-internal catcodes.
//!
//! # Zero-copy contract
//!
//! Every text-bearing token borrows a `&'a str` slice of the input. No
//! escape decoding happens at this layer: LaTeX source needs none for
//! tokenization purposes (`\%`, `\{`, `\$`, ... are just single-character
//! control symbols — themselves atomic tokens, not escapes to resolve).
//!
//! # Single-pass, zero-lookahead
//!
//! Every catcode decision is made from the current byte plus carried state
//! (group-nesting depth is tracked by the *caller* via balanced
//! `GroupOpen`/`GroupClose` tokens, not the tokenizer; the tokenizer itself
//! only carries the narrow `Mode` state below, plus the small
//! begin/environment-name detector needed for the verbatim-environment
//! special case). Bounded buffering of an in-progress token (a
//! control-sequence name not yet terminated by a non-letter byte, or a
//! verbatim body not yet terminated by its delimiter) across chunk
//! boundaries is ordinary token accumulation, not a lookahead violation —
//! this is exactly the O(largest token) bound `StreamingParser` documents.

use rescribe_format_api::Span;

/// One tokenizer output. Borrows from the input string wherever possible.
#[derive(Debug, Clone, PartialEq)]
pub enum Tok<'a> {
    /// Control sequence name, without the leading backslash. Empty only for
    /// the pathological case of a backslash at end-of-input.
    Cs(&'a str),
    /// `{`
    GroupOpen,
    /// `}`
    GroupClose,
    /// `$`
    MathShift,
    /// `$$` (legacy TeX display-math shift, recognized as a single token
    /// because splitting it into two `MathShift`s would make display vs.
    /// inline math indistinguishable to the semantic layer without extra
    /// state).
    DisplayMathShift,
    /// `&`
    AlignTab,
    /// `#<digit>` macro parameter reference.
    Param(u8),
    /// A bare `#` not followed by a digit.
    Hash,
    /// `%...` line comment content, not including the leading `%` or the
    /// terminating newline.
    Comment(&'a str),
    /// A run of ordinary (non-special) characters, including whitespace and
    /// newlines. Never empty.
    Text(&'a str),
    /// `\verb<delim>...<delim>` or `\verb*<delim>...<delim>`.
    Verb {
        star: bool,
        delim: char,
        content: &'a str,
    },
    /// The raw, unprocessed body of a `verbatim`/`verbatim*`/`lstlisting`
    /// environment — everything between `\begin{name}` and the following
    /// `\end{name}`, exclusive of both. Backslashes and braces inside are
    /// *not* tokenized; the semantic layer stores this verbatim.
    VerbatimEnvBody(&'a str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Normal,
    /// Just emitted `Cs("begin")`; watching for `GroupOpen`.
    AfterBegin,
    /// Just emitted `Cs("begin") GroupOpen`; capturing the env name.
    AfterBeginGroupOpen,
}

/// The three verbatim-like environment names the tokenizer special-cases.
const VERBATIM_ENV_NAMES: [&str; 3] = ["verbatim", "verbatim*", "lstlisting"];

/// Stateful, single-pass tokenizer over a `&'a str`.
#[derive(Debug, Clone)]
pub struct Lexer<'a> {
    input: &'a str,
    pos: usize,
    mode: Mode,
    /// Captured env name while in `AfterBeginGroupOpen`.
    pending_name_start: usize,
    /// Set when the previous token completed a `\begin{verbatim}`-style
    /// opening sequence; consumed (and cleared) by the very next call to
    /// [`Lexer::next_token`], which performs the raw-body scan and returns
    /// a [`Tok::VerbatimEnvBody`] before resuming normal tokenization.
    verbatim_pending: Option<String>,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Lexer {
            input,
            pos: 0,
            mode: Mode::Normal,
            pending_name_start: 0,
            verbatim_pending: None,
        }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    /// The unconsumed remainder of the input, from the current position.
    /// Public so callers that need to do a bounded raw-text scan for a
    /// known, narrow terminator (e.g. the matching `$` of an inline math
    /// span, or a `\end{name}` marker) can do so without the tokenizer
    /// itself needing to understand those constructs — mirrors the same
    /// pattern `parse.rs` already uses via direct `&str` slicing.
    pub fn rest(&self) -> &'a str {
        &self.input[self.pos..]
    }

    /// Jumps the cursor directly to `pos` and resets any in-progress
    /// tokenizer mode to `Normal`. Only safe to call at a position that is
    /// a valid **catcode-normal** boundary — never in the middle of a
    /// `\verb` delimiter scan or a verbatim-environment raw capture, since
    /// those carry state (`verbatim_pending`, the delimiter byte) that a
    /// blind jump would silently discard. Callers that only ever seek
    /// across plain text / math-shift spans (as `events.rs`'s inline-math
    /// handling does) satisfy this by construction.
    pub fn seek(&mut self, pos: usize) {
        self.pos = pos;
        self.mode = Mode::Normal;
        self.verbatim_pending = None;
    }

    fn peek_byte(&self) -> Option<u8> {
        self.input.as_bytes().get(self.pos).copied()
    }

    /// Advance past one full UTF-8 char at `pos`, returning it.
    fn bump_char(&mut self) -> char {
        let ch = self.rest().chars().next().expect("bump_char at EOF");
        self.pos += ch.len_utf8();
        ch
    }

    fn is_special(b: u8) -> bool {
        matches!(b, b'\\' | b'{' | b'}' | b'$' | b'&' | b'%' | b'#')
    }

    /// Produce the next token, or `None` at end of input.
    pub fn next_token(&mut self) -> Option<(Tok<'a>, Span)> {
        if let Some(needle) = self.verbatim_pending.take() {
            let start = self.pos;
            let end = self
                .rest()
                .find(needle.as_str())
                .map_or(self.input.len(), |off| start + off);
            self.pos = end;
            return Some((
                Tok::VerbatimEnvBody(&self.input[start..end]),
                Span::new(start, end),
            ));
        }
        let start = self.pos;
        let b = self.peek_byte()?;
        let tok = match b {
            b'\\' => self.scan_control_sequence(),
            b'{' => {
                self.pos += 1;
                self.note_group_open(start);
                Tok::GroupOpen
            }
            b'}' => {
                self.pos += 1;
                Tok::GroupClose
            }
            b'$' => {
                self.pos += 1;
                if self.peek_byte() == Some(b'$') {
                    self.pos += 1;
                    Tok::DisplayMathShift
                } else {
                    Tok::MathShift
                }
            }
            b'&' => {
                self.pos += 1;
                Tok::AlignTab
            }
            b'#' => {
                self.pos += 1;
                match self.peek_byte() {
                    Some(d) if d.is_ascii_digit() => {
                        self.pos += 1;
                        Tok::Param(d - b'0')
                    }
                    _ => Tok::Hash,
                }
            }
            b'%' => {
                self.pos += 1;
                let content_start = self.pos;
                while let Some(c) = self.peek_byte() {
                    if c == b'\n' {
                        break;
                    }
                    self.pos += 1;
                }
                let comment = &self.input[content_start..self.pos];
                // Consume the terminating newline as part of the comment
                // token's span (but not its content) so span coverage of
                // the source is contiguous; the newline itself carries no
                // semantic content beyond "comment ended".
                if self.peek_byte() == Some(b'\n') {
                    self.pos += 1;
                }
                Tok::Comment(comment)
            }
            _ => self.scan_text(),
        };
        // Track the begin{name} detector for verbatim-environment entry.
        self.observe_for_verbatim_env(&tok, start);
        Some((tok, Span::new(start, self.pos)))
    }

    fn scan_control_sequence(&mut self) -> Tok<'a> {
        // consume '\'
        self.pos += 1;
        let name_start = self.pos;
        match self.rest().chars().next() {
            None => Tok::Cs(&self.input[name_start..self.pos]),
            Some(c) if c.is_ascii_alphabetic() => {
                while let Some(c) = self.rest().chars().next() {
                    if c.is_ascii_alphabetic() {
                        self.pos += 1;
                    } else {
                        break;
                    }
                }
                let name = &self.input[name_start..self.pos];
                // TeX control words swallow immediately-following
                // whitespace/newlines. That whitespace is not semantically
                // meaningful (it never survives into any AST field), so it
                // is consumed here and intentionally not preserved — real
                // TeX itself discards it; re-emitting a single space after
                // a control word is standard and always valid.
                while matches!(self.peek_byte(), Some(b' ') | Some(b'\t')) {
                    self.pos += 1;
                }
                if name == "verb" {
                    return self.scan_verb();
                }
                Tok::Cs(name)
            }
            Some(_) => {
                // Control symbol: exactly one non-letter char (digit or
                // punctuation), e.g. `\\`, `\%`, `\{`, `\$`, `\[`, `\]`.
                self.bump_char();
                Tok::Cs(&self.input[name_start..self.pos])
            }
        }
    }

    /// `\verb`/`\verb*<delim>...<delim>`. `star` is currently always passed
    /// `false` at the call site and detected below by peeking the very next
    /// byte (a `*` immediately after the control word `verb`, before the
    /// delimiter) — this is why `\verb` is in the tokenizer's closed set:
    /// the delimiter itself is runtime-chosen and cannot be modeled as
    /// ordinary structure.
    fn scan_verb(&mut self) -> Tok<'a> {
        let star = if self.peek_byte() == Some(b'*') {
            self.pos += 1;
            true
        } else {
            false
        };
        let Some(delim_byte) = self.peek_byte() else {
            // Malformed (`\verb` at EOF with no delimiter): degrade to a
            // bare Cs token so the caller sees *something* well-formed.
            return Tok::Cs(if star { "verb*" } else { "verb" });
        };
        // The delimiter is a single byte per the classic `\verb` contract
        // (any non-letter, non-`*`, non-whitespace character; typically
        // ASCII punctuation). Multi-byte UTF-8 delimiters are not part of
        // the standard `\verb` contract and are not supported here.
        let delim = delim_byte as char;
        self.pos += 1;
        let content_start = self.pos;
        loop {
            match self.peek_byte() {
                None => break, // unterminated \verb at EOF: best-effort
                Some(b) if b == delim_byte => break,
                Some(_) => {
                    self.bump_char();
                }
            }
        }
        let content = &self.input[content_start..self.pos];
        if self.peek_byte() == Some(delim_byte) {
            self.pos += 1;
        }
        Tok::Verb {
            star,
            delim,
            content,
        }
    }

    fn scan_text(&mut self) -> Tok<'a> {
        let start = self.pos;
        while let Some(b) = self.peek_byte() {
            if Self::is_special(b) {
                break;
            }
            // ASCII fast path; multi-byte UTF-8 continuation bytes are all
            // > 0x7F and never equal a special byte, so stepping one byte
            // at a time here is safe as long as we only *stop* at char
            // boundaries — which `is_special` guarantees since specials
            // are all ASCII.
            self.pos += 1;
        }
        Tok::Text(&self.input[start..self.pos])
    }

    // ---- verbatim-environment detector -----------------------------------

    /// Tracks the literal 4-token sequence
    /// `Cs("begin") GroupOpen Text(name) GroupClose` and, on completion
    /// with `name` in [`VERBATIM_ENV_NAMES`], switches into raw-capture
    /// mode for the environment body.
    fn observe_for_verbatim_env(&mut self, tok: &Tok<'a>, _tok_start: usize) {
        self.mode = match (self.mode, tok) {
            (Mode::Normal, Tok::Cs("begin")) => Mode::AfterBegin,
            (Mode::AfterBegin, Tok::GroupOpen) => {
                self.pending_name_start = self.pos;
                Mode::AfterBeginGroupOpen
            }
            (Mode::AfterBeginGroupOpen, Tok::Text(_)) => Mode::AfterBeginGroupOpen,
            (Mode::AfterBeginGroupOpen, Tok::GroupClose) => {
                // pos is just past the '}'; name is between
                // pending_name_start and pos-1.
                let name = &self.input[self.pending_name_start..self.pos - 1];
                if VERBATIM_ENV_NAMES.contains(&name) {
                    self.enter_verbatim_capture(name);
                }
                Mode::Normal
            }
            _ => Mode::Normal,
        };
    }

    fn note_group_open(&mut self, _start: usize) {
        // Reserved hook (kept symmetrical with `observe_for_verbatim_env`'s
        // call site); no additional bookkeeping needed beyond that
        // function today.
    }

    /// Marks that the *next* [`Lexer::next_token`] call should perform the
    /// raw-body scan for a verbatim-like environment and return a single
    /// [`Tok::VerbatimEnvBody`] token before resuming normal tokenization.
    /// The scan itself is deferred (rather than done here) so that this
    /// call — made from inside `observe_for_verbatim_env`, itself called
    /// while still producing the `GroupClose` token — returns cleanly
    /// without trying to produce two tokens from one `next_token()` call.
    fn enter_verbatim_capture(&mut self, name: &str) {
        self.verbatim_pending = Some(format!("\\end{{{name}}}"));
    }
}

/// Collect all tokens from `input` (test/debug helper).
#[cfg(test)]
pub fn tokenize_all(input: &str) -> Vec<Tok<'_>> {
    let mut lex = Lexer::new(input);
    let mut out = Vec::new();
    while let Some((tok, _)) = lex.next_token() {
        out.push(tok);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text() {
        assert_eq!(tokenize_all("hello world"), vec![Tok::Text("hello world")]);
    }

    #[test]
    fn control_word_swallows_trailing_space() {
        let toks = tokenize_all("\\textbf hello");
        assert_eq!(toks, vec![Tok::Cs("textbf"), Tok::Text("hello")]);
    }

    #[test]
    fn control_symbol() {
        assert_eq!(
            tokenize_all("\\%\\{\\}"),
            vec![Tok::Cs("%"), Tok::Cs("{"), Tok::Cs("}")]
        );
    }

    #[test]
    fn groups() {
        assert_eq!(
            tokenize_all("\\textbf{hi}"),
            vec![
                Tok::Cs("textbf"),
                Tok::GroupOpen,
                Tok::Text("hi"),
                Tok::GroupClose
            ]
        );
    }

    #[test]
    fn math_shift_single_and_double() {
        assert_eq!(
            tokenize_all("$x$"),
            vec![Tok::MathShift, Tok::Text("x"), Tok::MathShift]
        );
        assert_eq!(
            tokenize_all("$$x$$"),
            vec![Tok::DisplayMathShift, Tok::Text("x"), Tok::DisplayMathShift]
        );
    }

    #[test]
    fn comment() {
        assert_eq!(
            tokenize_all("a%comment\nb"),
            vec![Tok::Text("a"), Tok::Comment("comment"), Tok::Text("b")]
        );
    }

    #[test]
    fn align_tab_and_row_break() {
        assert_eq!(
            tokenize_all("a & b \\\\ c"),
            vec![
                Tok::Text("a "),
                Tok::AlignTab,
                Tok::Text(" b "),
                Tok::Cs("\\"),
                Tok::Text(" c"),
            ]
        );
    }

    #[test]
    fn param_and_hash() {
        assert_eq!(
            tokenize_all("#1#x#"),
            vec![Tok::Param(1), Tok::Hash, Tok::Text("x"), Tok::Hash]
        );
    }

    #[test]
    fn verb_inline() {
        assert_eq!(
            tokenize_all("\\verb|a{b}c|"),
            vec![Tok::Verb {
                star: false,
                delim: '|',
                content: "a{b}c"
            }]
        );
        assert_eq!(
            tokenize_all("\\verb*!x!"),
            vec![Tok::Verb {
                star: true,
                delim: '!',
                content: "x"
            }]
        );
    }

    #[test]
    fn verbatim_environment_raw_body() {
        let toks = tokenize_all("\\begin{verbatim}a\\b{c}%d\n\\end{verbatim}");
        assert_eq!(
            toks,
            vec![
                Tok::Cs("begin"),
                Tok::GroupOpen,
                Tok::Text("verbatim"),
                Tok::GroupClose,
                Tok::VerbatimEnvBody("a\\b{c}%d\n"),
                Tok::Cs("end"),
                Tok::GroupOpen,
                Tok::Text("verbatim"),
                Tok::GroupClose,
            ]
        );
    }

    #[test]
    fn lstlisting_environment_raw_body() {
        let toks = tokenize_all("\\begin{lstlisting}\\foo\\end{lstlisting}");
        assert_eq!(
            toks,
            vec![
                Tok::Cs("begin"),
                Tok::GroupOpen,
                Tok::Text("lstlisting"),
                Tok::GroupClose,
                Tok::VerbatimEnvBody("\\foo"),
                Tok::Cs("end"),
                Tok::GroupOpen,
                Tok::Text("lstlisting"),
                Tok::GroupClose,
            ]
        );
    }

    #[test]
    fn non_verbatim_environment_tokenizes_normally() {
        let toks = tokenize_all("\\begin{itemize}\\item a\\end{itemize}");
        assert_eq!(
            toks,
            vec![
                Tok::Cs("begin"),
                Tok::GroupOpen,
                Tok::Text("itemize"),
                Tok::GroupClose,
                Tok::Cs("item"),
                Tok::Text("a"),
                Tok::Cs("end"),
                Tok::GroupOpen,
                Tok::Text("itemize"),
                Tok::GroupClose,
            ]
        );
    }

    #[test]
    fn unicode_text() {
        assert_eq!(
            tokenize_all("caf\u{e9} \\'e"),
            vec![Tok::Text("caf\u{e9} "), Tok::Cs("'"), Tok::Text("e")]
        );
    }
}
