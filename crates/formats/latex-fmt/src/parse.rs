//! `parse()` — direct recursive descent into [`crate::ast::LatexDoc`].
//!
//! # Resolution model
//!
//! The **only** source of "this control sequence has resolved structural
//! meaning" is an in-document `\def`/`\edef`/`\gdef`/`\xdef`/
//! `\newcommand`/`\renewcommand`/`\providecommand`/`\newenvironment`/
//! `\renewenvironment` — tracked with real TeX grouping-scope semantics via
//! [`Parser::command_scopes`]/[`Parser::env_scopes`] (a stack of frames,
//! one per group/environment/mandatory-argument-group nesting level;
//! `\gdef`/`\xdef` write to the outermost/global frame, everything else to
//! the innermost). There is **no built-in table of "standard LaTeX
//! commands"** consulted as a fallback — `\section`, `\textbf`, `\item`,
//! `\includegraphics`, `\cite`, and every other command the LaTeX kernel,
//! document class, or a package would normally define are indistinguishable
//! from an arbitrary unknown command *unless the document itself locally
//! defines them*, which is virtually never the case in practice.
//!
//! Anything not resolved this way is raw-preserved
//! ([`Node::ControlSymbol`] / [`Node::RawEnvironment`]) and gets an
//! `Info`-severity [`Diagnostic`] (code `latex::unresolved-command` /
//! `latex::unresolved-environment`) — the fidelity-tracking record that
//! this construct's meaning was not verified, not an error. This is the
//! resolution-priority-#1-plus-#4 subset of the full design (see
//! `TODO.md`): priority #2 (resolving unknown names against real `.cls`/
//! `.sty` package content, via vendored source / local TeX install /
//! network fetch) and #3 (bounded macro-expansion engine + persistent
//! cache) are explicitly deferred, not implemented, not guessed at here.
//!
//! `\def`'s implied argument count is inferred by counting `#<digit>`
//! parameter tokens in its parameter text (best-effort: TeX's more general
//! delimited-parameter patterns beyond a bare `#1#2...` run are not
//! specially handled — a documented, accepted limitation, not a silent
//! drop, since the raw parameter-text nodes are still captured verbatim
//! for round-tripping even when the inferred count is wrong).
//!
//! Two things are explicitly *not* part of this "no built-in knowledge"
//! rule, because they are tokenizer/catcode-level facts rather than
//! command-name knowledge: `$`/`$$`/`\[`/`\]` math-shift delimiters, and
//! `\verb`/verbatim-environment raw capture (baked into
//! [`crate::tokenize`], untouched by this module).

use crate::ast::{Arg, Diagnostic, LatexDoc, Node, Severity, Span};
use crate::tokenize::{Lexer, Tok};
use std::collections::HashMap;

pub fn parse(input: &str) -> (LatexDoc, Vec<Diagnostic>) {
    let mut p = Parser::new(input);
    let nodes = p.parse_sequence(false);
    (LatexDoc { nodes }, p.diags)
}

/// Commands that define/redefine a macro name via the LaTeX-kernel
/// `\newcommand` family. Recognizing these nine names is knowledge of the
/// TeX/LaTeX *definition mechanism itself* (a kernel primitive / kernel
/// macro, never redefined by a document or package in a way that changes
/// what it fundamentally does), not an assertion about what any
/// document-level command like `\section` means — categorically different
/// from the built-in "standard commands" table this design rejects.
const COMMAND_DEFINERS: &[&str] = &["newcommand", "renewcommand", "providecommand"];
/// Plain-TeX definition primitives: `\def\foo...{body}` (no braces around
/// the name, arbitrary parameter-text before the body). `def`/`edef` are
/// local-scoped; `gdef`/`xdef` write to the global frame.
const TEX_DEFINERS: &[&str] = &["def", "edef", "gdef", "xdef"];
const ENV_DEFINERS: &[&str] = &["newenvironment", "renewenvironment"];

/// What an in-document definition says about how many arguments a use of
/// the name consumes. Derived entirely from the document's own definition
/// (declared `[N]` for `\newcommand`/`\newenvironment`, or inferred `#n`
/// count for `\def`) — never from any external table.
#[derive(Debug, Clone, Copy)]
struct MacroInfo {
    /// Total mandatory `{...}` groups consumed at a use site (already
    /// excluding the leading optional one, if any).
    mandatory: u8,
    has_optional_leading: bool,
}

pub(crate) struct Parser<'a> {
    input: &'a str,
    lex: Lexer<'a>,
    peeked: Option<Option<(Tok<'a>, Span)>>,
    diags: Vec<Diagnostic>,
    /// Scope stack for in-document command definitions. Index 0 is the
    /// global frame (also where top-level/preamble definitions land,
    /// since the document body starts at stack depth 0 already — no
    /// special-casing needed for "looks global because it's top-level").
    command_scopes: Vec<HashMap<String, MacroInfo>>,
    /// Scope stack for in-document `\newenvironment`/`\renewenvironment`.
    env_scopes: Vec<HashMap<String, MacroInfo>>,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Parser {
            input,
            lex: Lexer::new(input),
            peeked: None,
            diags: Vec::new(),
            command_scopes: vec![HashMap::new()],
            env_scopes: vec![HashMap::new()],
        }
    }

    // ---- scope-stack plumbing ---------------------------------------------

    /// Pushed/popped 1:1 around every real TeX grouping boundary this
    /// parser recurses into: a bare `{...}` group, a command/environment
    /// mandatory `{...}` argument group, and a known (locally-defined)
    /// environment's body. `\begingroup`/`\endgroup` are not currently
    /// modeled as scope boundaries (tracked as a known limitation, not a
    /// silent gap — see TODO.md).
    fn push_scope(&mut self) {
        self.command_scopes.push(HashMap::new());
        self.env_scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        // Never pop the global frame (defensive; push/pop are always
        // paired 1:1 with recursion so this should be unreachable, but a
        // stray pop must not panic on malformed/adversarial input).
        if self.command_scopes.len() > 1 {
            self.command_scopes.pop();
        }
        if self.env_scopes.len() > 1 {
            self.env_scopes.pop();
        }
    }

    fn resolve_command(&self, name: &str) -> Option<MacroInfo> {
        self.command_scopes
            .iter()
            .rev()
            .find_map(|frame| frame.get(name))
            .copied()
    }

    fn resolve_environment(&self, name: &str) -> Option<MacroInfo> {
        self.env_scopes
            .iter()
            .rev()
            .find_map(|frame| frame.get(name))
            .copied()
    }

    fn define_command(&mut self, name: String, info: MacroInfo, global: bool) {
        if global {
            self.command_scopes[0].insert(name, info);
        } else {
            self.command_scopes.last_mut().unwrap().insert(name, info);
        }
    }

    fn define_environment(&mut self, name: String, info: MacroInfo) {
        self.env_scopes.last_mut().unwrap().insert(name, info);
    }

    // ---- token-stream plumbing ---------------------------------------------

    fn bump(&mut self) -> Option<(Tok<'a>, Span)> {
        if let Some(t) = self.peeked.take() {
            return t;
        }
        self.lex.next_token()
    }

    fn peek(&mut self) -> Option<&(Tok<'a>, Span)> {
        if self.peeked.is_none() {
            self.peeked = Some(self.lex.next_token());
        }
        self.peeked.as_ref().unwrap().as_ref()
    }

    fn pos(&mut self) -> usize {
        match &self.peeked {
            Some(Some((_, span))) => span.start,
            _ => self.lex.pos(),
        }
    }

    /// Parse a flat sequence of nodes. Stops (without consuming) at
    /// `GroupClose` always, and additionally at `Cs("end")` when
    /// `stop_at_end` is set (used for environment bodies).
    fn parse_sequence(&mut self, stop_at_end: bool) -> Vec<Node> {
        let mut out = Vec::new();
        loop {
            match self.peek() {
                None => break,
                Some((Tok::GroupClose, _)) => break,
                Some((Tok::Cs(name), _)) if stop_at_end && *name == "end" => break,
                _ => {}
            }
            if let Some(node) = self.parse_one() {
                out.push(node);
            }
        }
        out
    }

    /// Like [`Parser::parse_sequence`] but scoped: pushes a new frame
    /// before parsing and pops it after, matching a real TeX grouping
    /// boundary.
    fn parse_scoped_sequence(&mut self, stop_at_end: bool) -> Vec<Node> {
        self.push_scope();
        let out = self.parse_sequence(stop_at_end);
        self.pop_scope();
        out
    }

    fn parse_one(&mut self) -> Option<Node> {
        let (tok, span) = self.bump()?;
        Some(match tok {
            Tok::Text(s) => Node::Text {
                value: s.to_string(),
                span,
            },
            Tok::Comment(s) => Node::Comment {
                value: s.to_string(),
                span,
            },
            Tok::GroupOpen => {
                let body = self.parse_scoped_sequence(false);
                let end = self.expect_group_close(span);
                Node::Group {
                    body,
                    span: Span::new(span.start, end),
                }
            }
            Tok::GroupClose => {
                // Unbalanced close at top level: preserve as literal text
                // rather than dropping it (losslessness).
                Node::Text {
                    value: "}".to_string(),
                    span,
                }
            }
            Tok::MathShift => self.parse_math_inline(span),
            Tok::DisplayMathShift => self.parse_math_display_dollar(span),
            Tok::AlignTab => Node::AlignTab { span },
            Tok::Param(d) => Node::Text {
                value: format!("#{d}"),
                span,
            },
            Tok::Hash => Node::Text {
                value: "#".to_string(),
                span,
            },
            Tok::Verb {
                star,
                delim,
                content,
            } => Node::Verb {
                star,
                delim,
                content: content.to_string(),
                span,
            },
            Tok::VerbatimEnvBody(_) => {
                // Only reachable if a verbatim environment body appears
                // without the `\begin{...}` that normally precedes it
                // (malformed input / desync); preserve as empty text.
                Node::Text {
                    value: String::new(),
                    span,
                }
            }
            Tok::Cs(name) => self.parse_control_sequence(name, span),
        })
    }

    fn expect_group_close(&mut self, open_span: Span) -> usize {
        match self.peek() {
            Some((Tok::GroupClose, _)) => {
                let (_, s) = self.bump().unwrap();
                s.end
            }
            _ => {
                self.diags.push(
                    Diagnostic::new(Severity::Warning, "unterminated group: missing '}'")
                        .with_span(open_span)
                        .with_code("latex::unterminated-group"),
                );
                self.pos()
            }
        }
    }

    // ---- control sequence dispatch -----------------------------------------

    fn parse_control_sequence(&mut self, name: &'a str, span: Span) -> Node {
        if name == "begin" {
            return self.parse_environment(span);
        }
        if name == "\\" {
            return self.parse_row_break(span);
        }
        if COMMAND_DEFINERS.contains(&name) {
            return self.parse_command_definer(name, span);
        }
        if TEX_DEFINERS.contains(&name) {
            return self.parse_tex_definer(name, span);
        }
        if ENV_DEFINERS.contains(&name) {
            return self.parse_env_definer(name, span);
        }
        if let Some(info) = self.resolve_command(name) {
            let mut opt = Vec::new();
            if info.has_optional_leading
                && let Some(arg) = self.try_scan_optional_arg()
            {
                opt.push(arg);
            }
            let mut args = Vec::new();
            for _ in 0..info.mandatory {
                args.push(self.parse_mandatory_arg());
            }
            let end = self.pos();
            Node::Command {
                name: name.to_string(),
                star: false,
                opt,
                args,
                span: Span::new(span.start, end),
            }
        } else {
            self.diags.push(
                Diagnostic::new(
                    Severity::Info,
                    format!("unresolved control sequence '\\{name}': no in-document definition found; raw-preserved"),
                )
                .with_span(span)
                .with_code("latex::unresolved-command"),
            );
            Node::ControlSymbol {
                name: name.to_string(),
                span,
            }
        }
    }

    fn parse_mandatory_arg(&mut self) -> Arg {
        match self.peek() {
            Some((Tok::GroupOpen, _)) => {
                let (_, open_span) = self.bump().unwrap();
                let body = self.parse_scoped_sequence(false);
                self.expect_group_close(open_span);
                body
            }
            _ => {
                self.diags.push(Diagnostic::new(
                    Severity::Warning,
                    "expected mandatory argument group, found none",
                ));
                Vec::new()
            }
        }
    }

    /// Scans a `[...]` optional argument directly over raw source bytes
    /// (bracket-depth aware), recursively re-parsing its content with the
    /// *current* scope stack visible (so a use of a name locally defined
    /// in an enclosing scope resolves correctly inside an optional
    /// argument too). See the crate-level docs' "Known limitations"
    /// section for the accepted bracket-matching edge case (unbalanced
    /// `[`/`]` nested inside a `{...}` group within the optional argument
    /// is not specially handled).
    fn try_scan_optional_arg(&mut self) -> Option<Arg> {
        let start = self.pos();
        if self.input.as_bytes().get(start) != Some(&b'[') {
            return None;
        }
        let bytes = self.input.as_bytes();
        let mut depth = 0i32;
        let mut i = start;
        let end = loop {
            match bytes.get(i) {
                None => {
                    self.diags.push(
                        Diagnostic::new(
                            Severity::Warning,
                            "unterminated optional argument: missing ']'",
                        )
                        .with_span(Span::new(start, self.input.len()))
                        .with_code("latex::unterminated-optional-arg"),
                    );
                    break self.input.len();
                }
                Some(b'[') => {
                    depth += 1;
                    i += 1;
                }
                Some(b']') => {
                    depth -= 1;
                    i += 1;
                    if depth == 0 {
                        break i;
                    }
                }
                Some(_) => i += 1,
            }
        };
        let inner = &self.input[start + 1..(end.saturating_sub(1)).max(start + 1)];
        self.resync_to(end);
        let mut sub = Parser::new(inner);
        sub.command_scopes = self.command_scopes.clone();
        sub.env_scopes = self.env_scopes.clone();
        let mut nodes = sub.parse_sequence(false);
        offset_nodes(&mut nodes, start + 1);
        let mut sub_diags = sub.diags;
        for d in &mut sub_diags {
            d.span = Span::new(d.span.start + start + 1, d.span.end + start + 1);
        }
        self.diags.append(&mut sub_diags);
        Some(nodes)
    }

    /// Move the lexer to byte offset `pos` and drop any stale peeked
    /// token. Uses [`Lexer::seek`] directly (an arbitrary byte-offset
    /// jump) rather than replaying tokens from scratch — replay cannot
    /// land at an arbitrary offset, since the tokenizer's own maximal-munch
    /// text-run scanning can jump straight past a target that falls
    /// mid-run (e.g. two adjacent bracket groups like `[2][Hello]` with no
    /// special byte between them tokenize as one `Text` run; a resync
    /// target between them has no token boundary to replay onto). Every
    /// call site resyncs to a position that is a plain catcode-normal
    /// boundary (just past a `[`/`$`/`\end{name}`-style raw scan) so
    /// `Lexer::seek`'s "always resets to `Mode::Normal`" contract is safe
    /// here (see that method's own doc comment for the general caveat
    /// about verbatim-capture state).
    fn resync_to(&mut self, pos: usize) {
        self.peeked = None;
        self.lex.seek(pos);
    }

    fn parse_math_inline(&mut self, open_span: Span) -> Node {
        let start = open_span.end;
        match self.input[start..].find('$') {
            Some(off) => {
                let end = start + off;
                let source = self.input[start..end].to_string();
                self.resync_to(end + 1);
                Node::MathInline {
                    source,
                    span: Span::new(open_span.start, end + 1),
                }
            }
            None => {
                self.diags.push(
                    Diagnostic::new(Severity::Warning, "unterminated inline math: missing '$'")
                        .with_span(open_span)
                        .with_code("latex::unterminated-math"),
                );
                let source = self.input[start..].to_string();
                self.resync_to(self.input.len());
                Node::MathInline {
                    source,
                    span: Span::new(open_span.start, self.input.len()),
                }
            }
        }
    }

    fn parse_math_display_dollar(&mut self, open_span: Span) -> Node {
        let start = open_span.end;
        match self.input[start..].find("$$") {
            Some(off) => {
                let end = start + off;
                let source = self.input[start..end].to_string();
                self.resync_to(end + 2);
                Node::MathDisplay {
                    source,
                    env: None,
                    span: Span::new(open_span.start, end + 2),
                }
            }
            None => {
                self.diags.push(
                    Diagnostic::new(Severity::Warning, "unterminated display math: missing '$$'")
                        .with_span(open_span)
                        .with_code("latex::unterminated-math"),
                );
                let source = self.input[start..].to_string();
                self.resync_to(self.input.len());
                Node::MathDisplay {
                    source,
                    env: None,
                    span: Span::new(open_span.start, self.input.len()),
                }
            }
        }
    }

    fn parse_row_break(&mut self, span: Span) -> Node {
        // `\\[<spacing>]` — the bracket immediately follows `\\` with no
        // intervening space consumed (control symbols don't swallow
        // trailing whitespace, unlike control words).
        let spacing = self
            .try_scan_optional_arg()
            .map(|nodes| render_plain(&nodes));
        let end = self.pos();
        Node::RowBreak {
            spacing,
            span: Span::new(span.start, end.max(span.end)),
        }
    }

    // ---- environments -------------------------------------------------------

    fn parse_environment(&mut self, begin_span: Span) -> Node {
        let name = match self.expect_name_group() {
            Some(n) => n,
            None => {
                self.diags.push(
                    Diagnostic::new(Severity::Warning, "malformed \\begin: missing {name}")
                        .with_span(begin_span)
                        .with_code("latex::malformed-begin"),
                );
                return Node::ControlSymbol {
                    name: "begin".to_string(),
                    span: begin_span,
                };
            }
        };

        // Tokenizer-level fact, not command-name knowledge: `verbatim`/
        // `verbatim*`/`lstlisting` bodies are raw-captured unconditionally
        // by `tokenize.rs` (untouched by this design). By the time we get
        // here the tokenizer has already produced a single
        // `Tok::VerbatimEnvBody` for the body if `name` was one of those
        // three; we simply consume whatever token comes next structurally
        // rather than checking `name` against a list ourselves.
        if let Some((Tok::VerbatimEnvBody(_), _)) = self.peek() {
            let (tok, _) = self.bump().unwrap();
            let Tok::VerbatimEnvBody(content) = tok else {
                unreachable!()
            };
            let end = self.expect_end(&name, begin_span);
            return Node::RawEnvironment {
                name,
                raw: content.to_string(),
                span: Span::new(begin_span.start, end),
            };
        }

        if let Some(info) = self.resolve_environment(&name) {
            self.push_scope();
            let mut opt = Vec::new();
            if info.has_optional_leading
                && let Some(arg) = self.try_scan_optional_arg()
            {
                opt.push(arg);
            }
            let mut args = Vec::new();
            for _ in 0..info.mandatory {
                args.push(self.parse_mandatory_arg());
            }
            let body = self.parse_sequence(true);
            self.pop_scope();
            let end = self.expect_end(&name, begin_span);
            return Node::Environment {
                name,
                star: false,
                opt,
                args,
                body,
                span: Span::new(begin_span.start, end),
            };
        }

        // Unresolved environment: raw-preserve everything between the
        // opening and matching closing tag — no attempt to guess args or
        // parse the body, since an arbitrary undeclared environment may
        // have arbitrary argument conventions this crate has no way to
        // know. Depth-aware over same-name `\begin{name}`/`\end{name}`
        // pairs.
        self.diags.push(
            Diagnostic::new(
                Severity::Info,
                format!("unresolved environment '{name}': no in-document definition found; raw-preserved"),
            )
            .with_span(begin_span)
            .with_code("latex::unresolved-environment"),
        );
        let start = self.pos();
        let begin_needle = format!("\\begin{{{name}}}");
        let end_needle = format!("\\end{{{name}}}");
        let mut depth = 1i32;
        let mut i = start;
        let raw_end = loop {
            let next_begin = self.input[i..].find(begin_needle.as_str()).map(|o| i + o);
            let next_end = self.input[i..].find(end_needle.as_str()).map(|o| i + o);
            match (next_begin, next_end) {
                (Some(b), Some(e)) if b < e => {
                    depth += 1;
                    i = b + begin_needle.len();
                }
                (_, Some(e)) => {
                    depth -= 1;
                    if depth == 0 {
                        break e;
                    }
                    i = e + end_needle.len();
                }
                _ => break self.input.len(),
            }
        };
        let raw = self.input[start..raw_end].to_string();
        self.resync_to(raw_end);
        let end = self.expect_end(&name, begin_span);
        Node::RawEnvironment {
            name,
            raw,
            span: Span::new(begin_span.start, end),
        }
    }

    /// Consumes `{name}` immediately following `\begin`/`\end`.
    fn expect_name_group(&mut self) -> Option<String> {
        match self.bump() {
            Some((Tok::GroupOpen, _)) => {}
            other => {
                if let Some(t) = other {
                    self.peeked = Some(Some(t));
                }
                return None;
            }
        }
        let mut name = String::new();
        loop {
            match self.bump() {
                Some((Tok::Text(s), _)) => name.push_str(s),
                Some((Tok::GroupClose, _)) => break,
                other => {
                    if let Some(t) = other {
                        self.peeked = Some(Some(t));
                    }
                    break;
                }
            }
        }
        Some(name)
    }

    fn expect_end(&mut self, name: &str, begin_span: Span) -> usize {
        match self.peek() {
            Some((Tok::Cs("end"), _)) => {
                self.bump();
            }
            _ => {
                self.diags.push(
                    Diagnostic::new(
                        Severity::Warning,
                        format!("unterminated environment '{name}': missing \\end"),
                    )
                    .with_span(begin_span)
                    .with_code("latex::unterminated-environment"),
                );
                return self.pos();
            }
        }
        match self.expect_name_group() {
            Some(closing) if closing != name => {
                self.diags.push(
                    Diagnostic::new(
                        Severity::Warning,
                        format!("mismatched \\end{{{closing}}}, expected \\end{{{name}}}"),
                    )
                    .with_span(begin_span)
                    .with_code("latex::mismatched-end"),
                );
            }
            None => {
                self.diags.push(
                    Diagnostic::new(Severity::Warning, "malformed \\end: missing {name}")
                        .with_span(begin_span)
                        .with_code("latex::malformed-end"),
                );
            }
            _ => {}
        }
        self.pos()
    }

    // ---- definers -------------------------------------------------------------

    /// `\newcommand{\foo}[nargs][default]{body}` (or bare `\newcommand\foo...`,
    /// braces around the name are optional in real LaTeX).
    fn parse_command_definer(&mut self, definer: &'a str, span: Span) -> Node {
        let (macro_name, name_arg) = self.parse_definer_target_command();
        let opt1 = self.try_scan_optional_arg();
        let opt2 = self.try_scan_optional_arg();
        let nargs: u8 = opt1
            .as_ref()
            .and_then(|a| parse_u8(&render_plain(a)))
            .unwrap_or(0);
        let has_optional_leading = opt2.is_some();
        let body = self.parse_mandatory_arg();
        let end = self.pos();

        if let Some(name) = &macro_name {
            let mandatory = nargs.saturating_sub(if has_optional_leading { 1 } else { 0 });
            self.define_command(
                name.clone(),
                MacroInfo {
                    mandatory,
                    has_optional_leading,
                },
                false,
            );
        }

        let mut opt = Vec::new();
        if let Some(a) = opt1 {
            opt.push(a);
        }
        if let Some(a) = opt2 {
            opt.push(a);
        }
        Node::Command {
            name: definer.to_string(),
            star: false,
            opt,
            args: vec![name_arg, body],
            span: Span::new(span.start, end),
        }
    }

    /// `\def\foo#1#2{body}` / `\edef`/`\gdef`/`\xdef`. No braces around the
    /// name; arbitrary parameter-text (best-effort `#<digit>` counting)
    /// before the mandatory body group.
    fn parse_tex_definer(&mut self, definer: &'a str, span: Span) -> Node {
        let macro_name = match self.bump() {
            Some((Tok::Cs(n), s)) => Some((n.to_string(), s)),
            other => {
                if let Some(t) = other {
                    self.peeked = Some(Some(t));
                }
                None
            }
        };
        let mut params = Vec::new();
        let mut max_param = 0u8;
        loop {
            match self.peek() {
                Some((Tok::GroupOpen, _)) => break,
                None => break,
                _ => {
                    let (tok, tspan) = self.bump().unwrap();
                    if let Tok::Param(d) = tok {
                        max_param = max_param.max(d);
                    }
                    params.push(node_from_simple_tok(tok, tspan));
                }
            }
        }
        let body = self.parse_mandatory_arg();
        let end = self.pos();

        if let Some((name, _)) = &macro_name {
            let global = matches!(definer, "gdef" | "xdef");
            self.define_command(
                name.clone(),
                MacroInfo {
                    mandatory: max_param,
                    has_optional_leading: false,
                },
                global,
            );
        }

        let name_arg = match &macro_name {
            Some((n, s)) => vec![Node::ControlSymbol {
                name: n.clone(),
                span: *s,
            }],
            None => Vec::new(),
        };
        Node::Command {
            name: definer.to_string(),
            star: false,
            opt: Vec::new(),
            args: vec![name_arg, params, body],
            span: Span::new(span.start, end),
        }
    }

    /// `\newenvironment{name}[nargs][default]{begin-def}{end-def}`.
    fn parse_env_definer(&mut self, definer: &'a str, span: Span) -> Node {
        let name_arg = self.parse_mandatory_arg();
        let env_name = render_plain(&name_arg);
        let opt1 = self.try_scan_optional_arg();
        let opt2 = self.try_scan_optional_arg();
        let nargs: u8 = opt1
            .as_ref()
            .and_then(|a| parse_u8(&render_plain(a)))
            .unwrap_or(0);
        let has_optional_leading = opt2.is_some();
        let begin_def = self.parse_mandatory_arg();
        let end_def = self.parse_mandatory_arg();
        let end = self.pos();

        if !env_name.is_empty() {
            let mandatory = nargs.saturating_sub(if has_optional_leading { 1 } else { 0 });
            self.define_environment(
                env_name,
                MacroInfo {
                    mandatory,
                    has_optional_leading,
                },
            );
        }

        let mut opt = Vec::new();
        if let Some(a) = opt1 {
            opt.push(a);
        }
        if let Some(a) = opt2 {
            opt.push(a);
        }
        Node::Command {
            name: definer.to_string(),
            star: false,
            opt,
            args: vec![name_arg, begin_def, end_def],
            span: Span::new(span.start, end),
        }
    }

    /// Consumes the macro-name target of `\newcommand`/`\renewcommand`/
    /// `\providecommand`: either `{\foo}` or bare `\foo`. Returns the
    /// extracted name (for registration) and the argument's node
    /// representation (for round-trip storage, always brace-wrapped in the
    /// AST regardless of the source form — an accepted normalization per
    /// this codebase's AST-is-ground-truth roundtrip contract).
    fn parse_definer_target_command(&mut self) -> (Option<String>, Arg) {
        match self.peek() {
            Some((Tok::GroupOpen, _)) => {
                // Deliberately does NOT go through `parse_mandatory_arg`
                // (which would recurse through `parse_one` ->
                // `parse_control_sequence`, resolving `\foo` as a *use*
                // against the current scope). The name here is being
                // *declared*, not invoked — resolving it would silently
                // turn a `\renewcommand{\foo}...`/second `\newcommand{\foo}...`
                // (redefining an already-defined `\foo`) into a `Command`
                // node instead of the literal `ControlSymbol` the source
                // actually wrote, breaking round-trip (found by the
                // `fuzz_latex_fmt_roundtrip` target). Consume `{`, a bare
                // `Cs` token, `}` directly instead.
                let (_, open_span) = self.bump().unwrap();
                match self.bump() {
                    Some((Tok::Cs(n), s)) => {
                        let name_node = Node::ControlSymbol {
                            name: n.to_string(),
                            span: s,
                        };
                        self.expect_group_close(open_span);
                        (Some(n.to_string()), vec![name_node])
                    }
                    other => {
                        if let Some(t) = other {
                            self.peeked = Some(Some(t));
                        }
                        self.expect_group_close(open_span);
                        (None, Vec::new())
                    }
                }
            }
            Some((Tok::Cs(_), _)) => {
                let (tok, s) = self.bump().unwrap();
                let Tok::Cs(n) = tok else { unreachable!() };
                (
                    Some(n.to_string()),
                    vec![Node::ControlSymbol {
                        name: n.to_string(),
                        span: s,
                    }],
                )
            }
            _ => (None, Vec::new()),
        }
    }
}

/// Converts a subset of `Tok` (used for `\def`'s raw parameter-text
/// capture, where groups/environments cannot meaningfully appear) into a
/// `Node`. Falls back to an empty text node for token kinds that can't
/// occur in a bare parameter-text run.
fn node_from_simple_tok(tok: Tok<'_>, span: Span) -> Node {
    match tok {
        Tok::Text(s) => Node::Text {
            value: s.to_string(),
            span,
        },
        Tok::Param(d) => Node::Text {
            value: format!("#{d}"),
            span,
        },
        Tok::Hash => Node::Text {
            value: "#".to_string(),
            span,
        },
        Tok::Cs(n) => Node::ControlSymbol {
            name: n.to_string(),
            span,
        },
        _ => Node::Text {
            value: String::new(),
            span,
        },
    }
}

fn parse_u8(s: &str) -> Option<u8> {
    s.trim().parse().ok()
}

/// Adds `base` to every span in `nodes`, recursively. Used when a raw
/// substring (e.g. an optional argument's content) is re-parsed on its own
/// and the resulting spans need to be relative to the outer document.
fn offset_nodes(nodes: &mut [Node], base: usize) {
    for n in nodes {
        offset_span(n, base);
    }
}

fn offset_span(n: &mut Node, base: usize) {
    fn shift(s: &mut Span, base: usize) {
        *s = Span::new(s.start + base, s.end + base);
    }
    match n {
        Node::Text { span, .. }
        | Node::Comment { span, .. }
        | Node::ControlSymbol { span, .. }
        | Node::MathInline { span, .. }
        | Node::AlignTab { span }
        | Node::RowBreak { span, .. }
        | Node::Verb { span, .. }
        | Node::RawEnvironment { span, .. }
        | Node::MathDisplay { span, .. } => shift(span, base),
        Node::Group { body, span } => {
            shift(span, base);
            offset_nodes(body, base);
        }
        Node::Command {
            opt, args, span, ..
        } => {
            shift(span, base);
            for a in opt.iter_mut().chain(args.iter_mut()) {
                offset_nodes(a, base);
            }
        }
        Node::Environment {
            opt,
            args,
            body,
            span,
            ..
        } => {
            shift(span, base);
            for a in opt.iter_mut().chain(args.iter_mut()) {
                offset_nodes(a, base);
            }
            offset_nodes(body, base);
        }
    }
}

/// Renders a small node sequence back to plain text (used for `\\[<spacing>]`'s
/// spacing value and for extracting a definer's plain-text name argument).
fn render_plain(nodes: &[Node]) -> String {
    let mut s = String::new();
    for n in nodes {
        if let Node::Text { value, .. } = n {
            s.push_str(value);
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(src: &str) -> LatexDoc {
        let (doc, diags) = parse(src);
        let hard_errors: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .collect();
        assert!(
            hard_errors.is_empty(),
            "unexpected warning diagnostics: {hard_errors:?}"
        );
        doc
    }

    #[test]
    fn plain_paragraph() {
        let doc = parse_ok("Hello world");
        assert_eq!(
            doc.nodes,
            vec![Node::Text {
                value: "Hello world".to_string(),
                span: Span::new(0, 11)
            }]
        );
    }

    #[test]
    fn undefined_command_raw_preserved_with_info_diag() {
        let (doc, diags) = parse("\\section{Intro}");
        assert!(matches!(&doc.nodes[0], Node::ControlSymbol { name, .. } if name == "section"));
        assert!(matches!(&doc.nodes[1], Node::Group { .. }));
        assert!(
            diags
                .iter()
                .any(|d| d.code == "latex::unresolved-command" && d.severity == Severity::Info)
        );
    }

    #[test]
    fn newcommand_resolves_later_use() {
        let doc = parse_ok("\\newcommand{\\R}{\\mathbb{R}}Let $x \\in \\R$.");
        // First node: the \newcommand definition itself.
        assert!(matches!(&doc.nodes[0], Node::Command { name, .. } if name == "newcommand"));
        // \R inside the math span is raw math source (unaffected by
        // command resolution — math content is a raw span), so exercise
        // resolution outside math instead.
        let (doc2, diags2) = parse("\\newcommand{\\R}[0]{\\mathbb{R}} \\R");
        let hard: Vec<_> = diags2
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .collect();
        assert!(hard.is_empty(), "{hard:?}");
        assert!(matches!(&doc2.nodes[2], Node::Command { name, .. } if name == "R"));
    }

    #[test]
    fn newcommand_with_args_consumes_declared_arity() {
        let doc = parse_ok("\\newcommand{\\norm}[1]{\\lVert #1 \\rVert}\\norm{x}");
        match &doc.nodes[1] {
            Node::Command { name, args, .. } => {
                assert_eq!(name, "norm");
                assert_eq!(args.len(), 1);
                assert_eq!(
                    args[0],
                    vec![Node::Text {
                        value: "x".to_string(),
                        span: Span::new(45, 46)
                    }]
                );
            }
            other => panic!("expected resolved Command, got {other:?}"),
        }
    }

    #[test]
    fn newcommand_optional_leading_arg() {
        let doc = parse_ok("\\newcommand{\\greet}[2][Hello]{#1, #2!}\\greet[Hi]{World}");
        match &doc.nodes[1] {
            Node::Command {
                name, opt, args, ..
            } => {
                assert_eq!(name, "greet");
                assert_eq!(opt.len(), 1);
                assert_eq!(args.len(), 1);
            }
            other => panic!("expected resolved Command, got {other:?}"),
        }
    }

    #[test]
    fn def_infers_arity_from_param_count() {
        let doc = parse_ok("\\def\\add#1#2{#1+#2}\\add{a}{b}");
        match &doc.nodes[1] {
            Node::Command { name, args, .. } => {
                assert_eq!(name, "add");
                assert_eq!(args.len(), 2);
            }
            other => panic!("expected resolved Command, got {other:?}"),
        }
    }

    #[test]
    fn local_definition_is_scoped_to_its_group() {
        let (doc, _diags) = parse("{\\def\\x{a}\\x}\\x");
        // Inside the group: \x resolves (defined in that scope).
        match &doc.nodes[0] {
            Node::Group { body, .. } => {
                assert!(matches!(&body[1], Node::Command { name, .. } if name == "x"));
            }
            other => panic!("expected Group, got {other:?}"),
        }
        // Outside the group: \x is unresolved again (scope popped).
        assert!(matches!(&doc.nodes[1], Node::ControlSymbol { name, .. } if name == "x"));
    }

    #[test]
    fn gdef_escapes_to_global_scope() {
        let (doc, _diags) = parse("{\\gdef\\x{a}}\\x");
        // Outside the group: \x still resolves, because \gdef wrote to the
        // global frame rather than the group's local frame.
        assert!(matches!(&doc.nodes[1], Node::Command { name, .. } if name == "x"));
    }

    #[test]
    fn shadowing_in_nested_scope() {
        let src = "\\def\\x{outer}{\\def\\x{inner}\\x}\\x";
        let (doc, _diags) = parse(src);
        // doc.nodes: [Command(def outer), Group{ [Command(def inner), Command(x resolved)] }, Command(x resolved, still outer def visible)]
        match &doc.nodes[1] {
            Node::Group { body, .. } => {
                assert!(matches!(&body[1], Node::Command { name, .. } if name == "x"));
            }
            other => panic!("expected Group, got {other:?}"),
        }
        assert!(matches!(&doc.nodes[2], Node::Command { name, .. } if name == "x"));
    }

    #[test]
    fn unresolved_environment_raw_preserved_with_info_diag() {
        let (doc, diags) = parse("\\begin{itemize}\\item a\\end{itemize}");
        match &doc.nodes[0] {
            Node::RawEnvironment { name, raw, .. } => {
                assert_eq!(name, "itemize");
                assert_eq!(raw, "\\item a");
            }
            other => panic!("expected RawEnvironment, got {other:?}"),
        }
        assert!(
            diags
                .iter()
                .any(|d| d.code == "latex::unresolved-environment")
        );
    }

    #[test]
    fn newenvironment_resolves_later_use() {
        let src = "\\newenvironment{myenv}{[}{]}\\begin{myenv}x\\end{myenv}";
        let doc = parse_ok(src);
        match &doc.nodes[1] {
            Node::Environment { name, body, .. } => {
                assert_eq!(name, "myenv");
                assert_eq!(
                    body,
                    &vec![Node::Text {
                        value: "x".to_string(),
                        span: Span::new(41, 42)
                    }]
                );
            }
            other => panic!("expected resolved Environment, got {other:?}"),
        }
    }

    #[test]
    fn verbatim_environment_raw_body_independent_of_resolution() {
        let doc = parse_ok("\\begin{verbatim}a\\b{c}\\end{verbatim}");
        match &doc.nodes[0] {
            Node::RawEnvironment { name, raw, .. } => {
                assert_eq!(name, "verbatim");
                assert_eq!(raw, "a\\b{c}");
            }
            other => panic!("expected RawEnvironment, got {other:?}"),
        }
    }

    #[test]
    fn verb_inline() {
        let doc = parse_ok("\\verb|a{b}|");
        assert!(matches!(&doc.nodes[0], Node::Verb { content, .. } if content == "a{b}"));
    }

    #[test]
    fn comment_preserved() {
        let doc = parse_ok("a%note\nb");
        assert!(
            doc.nodes
                .iter()
                .any(|n| matches!(n, Node::Comment { value, .. } if value == "note"))
        );
    }

    #[test]
    fn inline_and_display_math_raw_capture() {
        let doc = parse_ok("$x^2$ and $$y = 1$$");
        assert!(matches!(&doc.nodes[0], Node::MathInline { source, .. } if source == "x^2"));
        let display = doc
            .nodes
            .iter()
            .find(|n| matches!(n, Node::MathDisplay { .. }))
            .unwrap();
        assert!(
            matches!(display, Node::MathDisplay { source, env: None, .. } if source == "y = 1")
        );
    }

    #[test]
    fn equation_environment_is_generic_raw_preserve_not_math_shaped() {
        // No special "this is math" table consulted: an undeclared
        // `equation` environment raw-preserves exactly like any other
        // undeclared environment.
        let (doc, diags) = parse("\\begin{equation}E = mc^2\\end{equation}");
        match &doc.nodes[0] {
            Node::RawEnvironment { name, raw, .. } => {
                assert_eq!(name, "equation");
                assert_eq!(raw, "E = mc^2");
            }
            other => panic!("expected RawEnvironment, got {other:?}"),
        }
        assert!(
            diags
                .iter()
                .any(|d| d.code == "latex::unresolved-environment")
        );
    }

    #[test]
    fn optional_arg_sees_enclosing_scope() {
        // \x is locally defined in the outer scope; an optional-argument
        // sub-parse must still see it (regression test for the
        // scope-blindness gap flagged and fixed in this design).
        let src = "\\def\\x{y}\\newcommand{\\cmd}[1][\\x]{#1}\\cmd";
        let (doc, diags) = parse(src);
        let hard: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .collect();
        assert!(hard.is_empty(), "{hard:?}");
        // The default-value optional arg of \newcommand's own definition
        // (opt[1]) should contain a *resolved* \x, not raw-preserved.
        match &doc.nodes[1] {
            Node::Command { opt, .. } => match &opt[1][0] {
                Node::Command { name, .. } => assert_eq!(name, "x"),
                other => {
                    panic!("expected resolved \\x inside default-value optional arg, got {other:?}")
                }
            },
            other => panic!("expected newcommand Command node, got {other:?}"),
        }
    }

    #[test]
    fn strip_spans_zeroes_all_spans() {
        let mut doc = parse_ok("\\def\\x{a}\\x $y$");
        doc.strip_spans();
        for n in &doc.nodes {
            assert_eq!(n.span(), Span::NONE);
        }
    }
}
