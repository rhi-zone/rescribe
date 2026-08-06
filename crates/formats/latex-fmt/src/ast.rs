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
}
