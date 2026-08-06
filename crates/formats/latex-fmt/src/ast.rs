//! LaTeX AST: `Ast`, node variants, `Span`, `Diagnostic`, `strip_spans()`.

pub use rescribe_format_api::{Diagnostic, Severity, Span};

/// One optional or mandatory argument group's content, as a sequence of
/// sibling nodes (matching how the tokenizer's group content is structured
/// everywhere else in the AST).
pub type Arg = Vec<Node>;

/// A LaTeX document, as a flat top-level sequence of nodes — mirrors the
/// tokenizer's own output shape (no implicit top-level grouping).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LatexDoc {
    pub nodes: Vec<Node>,
}

/// Alias kept for symmetry with other crates' `Ast` naming; `LatexDoc` is
/// both the AST type and the `rescribe-format-api` `Parse`/`Emit` impl
/// target (same pattern as `rtf_fmt::RtfDoc`).
pub type Ast = LatexDoc;

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// A run of ordinary text (including whitespace/newlines).
    Text { value: String, span: Span },
    /// `%...` line comment content (not including `%` or the newline).
    Comment { value: String, span: Span },
    /// A `{...}` group not consumed as a known command's argument (either
    /// standalone, e.g. `{\itshape text}`, or an unrecognized command's
    /// trailing group).
    Group { body: Vec<Node>, span: Span },
    /// An unrecognized control sequence — no argument groups are consumed;
    /// any groups that follow in the source remain sibling `Group` nodes.
    /// This is the raw-preservation path for anything outside the known
    /// vocabulary (`crate::vocab`).
    ControlSymbol { name: String, span: Span },
    /// A recognized-vocabulary command with resolved argument structure.
    Command {
        name: String,
        star: bool,
        opt: Vec<Arg>,
        args: Vec<Arg>,
        span: Span,
    },
    /// A recognized-vocabulary environment (`\begin{name}...\end{name}`)
    /// with resolved argument structure and a structurally-parsed body.
    Environment {
        name: String,
        star: bool,
        opt: Vec<Arg>,
        args: Vec<Arg>,
        body: Vec<Node>,
        span: Span,
    },
    /// An *unrecognized* `\begin{name}...\end{name}` environment: the name
    /// plus everything between the delimiters (including any args) is
    /// captured as raw source text — no attempt to parse its internals,
    /// since arbitrary environments may have arbitrary argument
    /// conventions the tokenizer/semantic layer has no way to know.
    RawEnvironment {
        name: String,
        raw: String,
        span: Span,
    },
    /// Raw math-mode source, `$...$`.
    MathInline { source: String, span: Span },
    /// Raw math-mode source: `$$...$$`, `\[...\]`, or a known math
    /// environment (`equation`, `align`, ...). `env` is `Some(name)` for
    /// the environment form, `None` for `$$...$$`/`\[...\]`.
    MathDisplay {
        source: String,
        env: Option<String>,
        span: Span,
    },
    /// `&` — column separator, meaningful inside `tabular`.
    AlignTab { span: Span },
    /// `\\` (optionally `\\[<spacing>]`) — row break.
    RowBreak { spacing: Option<String>, span: Span },
    /// `\verb`/`\verb*<delim>...<delim>`.
    Verb {
        star: bool,
        delim: char,
        content: String,
        span: Span,
    },
}

impl Node {
    pub fn span(&self) -> Span {
        match self {
            Node::Text { span, .. }
            | Node::Comment { span, .. }
            | Node::Group { span, .. }
            | Node::ControlSymbol { span, .. }
            | Node::Command { span, .. }
            | Node::Environment { span, .. }
            | Node::RawEnvironment { span, .. }
            | Node::MathInline { span, .. }
            | Node::MathDisplay { span, .. }
            | Node::AlignTab { span }
            | Node::RowBreak { span, .. }
            | Node::Verb { span, .. } => *span,
        }
    }

    fn strip_span(&mut self) {
        match self {
            Node::Text { span, .. }
            | Node::Comment { span, .. }
            | Node::Group { span, .. }
            | Node::ControlSymbol { span, .. }
            | Node::Command { span, .. }
            | Node::Environment { span, .. }
            | Node::RawEnvironment { span, .. }
            | Node::MathInline { span, .. }
            | Node::MathDisplay { span, .. }
            | Node::AlignTab { span }
            | Node::RowBreak { span, .. }
            | Node::Verb { span, .. } => *span = Span::NONE,
        }
        match self {
            Node::Group { body, .. } | Node::Environment { body, .. } => {
                for n in body {
                    n.strip_span();
                }
            }
            _ => {}
        }
        match self {
            Node::Command { opt, args, .. } | Node::Environment { opt, args, .. } => {
                for a in opt.iter_mut().chain(args.iter_mut()) {
                    for n in a.iter_mut() {
                        n.strip_span();
                    }
                }
            }
            _ => {}
        }
    }
}

impl LatexDoc {
    pub fn strip_spans(&mut self) -> &mut Self {
        for n in &mut self.nodes {
            n.strip_span();
        }
        self
    }

    /// Puts a programmatically-built document into the canonical shape
    /// `parse()` always produces, before round-tripping (same contract as
    /// `rtf_fmt::RtfDoc::normalize` — see that crate for precedent).
    ///
    /// Concretely: merges adjacent `Text` siblings into one. `parse()`'s
    /// tokenizer produces maximal-munch `Text` runs, so two back-to-back
    /// `Text` nodes with nothing tokenizer-special between them can never
    /// come out of `parse()` — but the `Node` type itself doesn't forbid
    /// constructing that shape by hand (e.g. a fuzz/property-test
    /// generator that emits two independent `Text` leaves in a row).
    /// `emit(doc)` then `parse()` naturally re-merges them, which is
    /// correct behavior, not a bug — `normalize()` makes a hand-built
    /// `LatexDoc` match what `parse()` would have produced for the same
    /// rendered text, so `parse(emit(doc)).strip_spans() ==
    /// doc.normalize().strip_spans()` holds for any `doc`, not only ones
    /// already in canonical form.
    pub fn normalize(&mut self) -> &mut Self {
        merge_adjacent_text(&mut self.nodes);
        for n in &mut self.nodes {
            n.normalize();
        }
        self
    }
}

fn merge_adjacent_text(nodes: &mut Vec<Node>) {
    let mut i = 0;
    while i + 1 < nodes.len() {
        let can_merge = matches!(
            (&nodes[i], &nodes[i + 1]),
            (Node::Text { .. }, Node::Text { .. })
        );
        if can_merge {
            let next_value = match nodes.remove(i + 1) {
                Node::Text { value, .. } => value,
                _ => unreachable!(),
            };
            if let Node::Text { value, .. } = &mut nodes[i] {
                value.push_str(&next_value);
            }
        } else {
            i += 1;
        }
    }
}

impl Node {
    fn normalize(&mut self) {
        match self {
            Node::Group { body, .. } | Node::Environment { body, .. } => {
                merge_adjacent_text(body);
                for n in body.iter_mut() {
                    n.normalize();
                }
            }
            _ => {}
        }
        match self {
            Node::Command { opt, args, .. } | Node::Environment { opt, args, .. } => {
                for a in opt.iter_mut().chain(args.iter_mut()) {
                    merge_adjacent_text(a);
                    for n in a.iter_mut() {
                        n.normalize();
                    }
                }
            }
            _ => {}
        }
    }
}
