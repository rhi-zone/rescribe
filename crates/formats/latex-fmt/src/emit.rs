//! `emit()` — builder writer: serialize an already-parsed [`LatexDoc`] all
//! at once.

use crate::ast::{LatexDoc, Node};

/// Kept in sync with `parse.rs`'s `COMMAND_DEFINERS`/`TEX_DEFINERS`/
/// `ENV_DEFINERS` — these three sets determine the definer-specific
/// argument ordering `emit_node` special-cases below (see its match arms'
/// comments for why each differs from the generic `Command` shape).
fn is_command_definer(name: &str) -> bool {
    matches!(name, "newcommand" | "renewcommand" | "providecommand")
}
fn is_tex_definer(name: &str) -> bool {
    matches!(name, "def" | "edef" | "gdef" | "xdef")
}
fn is_env_definer(name: &str) -> bool {
    matches!(name, "newenvironment" | "renewenvironment")
}

fn emit_braced(nodes: &[Node], out: &mut String) {
    out.push('{');
    for c in nodes {
        emit_node(c, out);
    }
    out.push('}');
}

fn emit_bracketed(nodes: &[Node], out: &mut String) {
    out.push('[');
    for c in nodes {
        emit_node(c, out);
    }
    out.push(']');
}

pub fn emit(doc: &LatexDoc) -> String {
    let mut out = String::new();
    for n in &doc.nodes {
        emit_node(n, &mut out);
    }
    out
}

/// Emits a single node's LaTeX source in isolation. Used by
/// `crate::rescribe` to re-emit one top-level AST node's raw-preserved
/// source into a standalone `rescribe_core::Node` property value.
pub fn emit_one(n: &Node) -> String {
    let mut out = String::new();
    emit_node(n, &mut out);
    out
}

fn emit_node(n: &Node, out: &mut String) {
    match n {
        Node::Text { value, .. } => out.push_str(value),
        Node::Comment { value, .. } => {
            out.push('%');
            out.push_str(value);
            out.push('\n');
        }
        Node::Group { body, .. } => {
            out.push('{');
            for c in body {
                emit_node(c, out);
            }
            out.push('}');
        }
        Node::ControlSymbol { name, .. } => {
            out.push('\\');
            out.push_str(name);
            // A control *word* (all-letters) needs a delimiting space if
            // immediately followed at emit time by another letter (or
            // this crate would misparse `\foo` `bar` re-concatenated as
            // `\foobar` on re-parse). A control *symbol* (single
            // non-letter char) never needs this, since the tokenizer's
            // maximal-munch rule for control symbols only ever consumes
            // exactly one char after the backslash regardless of what
            // follows.
            if name.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
                out.push(' ');
            }
        }
        Node::Command {
            name,
            star,
            opt,
            args,
            ..
        } if is_command_definer(name) => {
            // `\newcommand{\foo}[nargs][default]{body}` — the mandatory
            // name argument comes *before* the optional arguments, unlike
            // ordinary commands (`\section[short]{long}`, optionals
            // first). `args = [name_arg, body]`.
            out.push('\\');
            out.push_str(name);
            emit_braced(&args[0], out);
            for a in opt {
                emit_bracketed(a, out);
            }
            emit_braced(args.get(1).map(Vec::as_slice).unwrap_or(&[]), out);
        }
        Node::Command { name, args, .. } if is_tex_definer(name) => {
            // `\def\foo#1#2{body}` — bare control-sequence name and bare
            // parameter text, neither brace-wrapped; only the body is.
            // `args = [name_arg (bare Cs), params (bare), body (braced)]`.
            out.push('\\');
            out.push_str(name);
            out.push(' ');
            for c in args.first().map(Vec::as_slice).unwrap_or(&[]) {
                emit_node(c, out);
            }
            for c in args.get(1).map(Vec::as_slice).unwrap_or(&[]) {
                emit_node(c, out);
            }
            emit_braced(args.get(2).map(Vec::as_slice).unwrap_or(&[]), out);
        }
        Node::Command {
            name, opt, args, ..
        } if is_env_definer(name) => {
            // `\newenvironment{name}[nargs][default]{begin-def}{end-def}`.
            out.push('\\');
            out.push_str(name);
            emit_braced(&args[0], out);
            for a in opt {
                emit_bracketed(a, out);
            }
            emit_braced(args.get(1).map(Vec::as_slice).unwrap_or(&[]), out);
            emit_braced(args.get(2).map(Vec::as_slice).unwrap_or(&[]), out);
        }
        Node::Command {
            name,
            star,
            opt,
            args,
            ..
        } => {
            out.push('\\');
            out.push_str(name);
            if *star {
                out.push('*');
            }
            for a in opt {
                emit_bracketed(a, out);
            }
            for a in args {
                emit_braced(a, out);
            }
            // A zero-arg, non-starred resolved command (e.g. a
            // `\newcommand`-defined macro taking no arguments) ends with
            // nothing but the bare control word — a following node that
            // starts with a letter would otherwise re-tokenize as one
            // longer, different control word on re-parse. Same guard as
            // `ControlSymbol` above.
            if !*star
                && opt.is_empty()
                && args.is_empty()
                && name.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
            {
                out.push(' ');
            }
        }
        Node::Environment {
            name,
            opt,
            args,
            body,
            ..
        } => {
            out.push_str("\\begin{");
            out.push_str(name);
            out.push('}');
            for a in opt {
                out.push('[');
                for c in a {
                    emit_node(c, out);
                }
                out.push(']');
            }
            for a in args {
                out.push('{');
                for c in a {
                    emit_node(c, out);
                }
                out.push('}');
            }
            for c in body {
                emit_node(c, out);
            }
            out.push_str("\\end{");
            out.push_str(name);
            out.push('}');
        }
        Node::RawEnvironment { name, raw, .. } => {
            out.push_str("\\begin{");
            out.push_str(name);
            out.push('}');
            out.push_str(raw);
            out.push_str("\\end{");
            out.push_str(name);
            out.push('}');
        }
        Node::MathInline { source, .. } => {
            out.push('$');
            out.push_str(source);
            out.push('$');
        }
        Node::MathDisplay { source, env, .. } => match env {
            Some(name) => {
                out.push_str("\\begin{");
                out.push_str(name);
                out.push('}');
                out.push_str(source);
                out.push_str("\\end{");
                out.push_str(name);
                out.push('}');
            }
            None => {
                out.push_str("$$");
                out.push_str(source);
                out.push_str("$$");
            }
        },
        Node::AlignTab { .. } => out.push('&'),
        Node::RowBreak { spacing, .. } => {
            out.push_str("\\\\");
            if let Some(s) = spacing {
                out.push('[');
                out.push_str(s);
                out.push(']');
            }
        }
        Node::Verb {
            star,
            delim,
            content,
            ..
        } => {
            out.push_str("\\verb");
            if *star {
                out.push('*');
            }
            out.push(*delim);
            out.push_str(content);
            out.push(*delim);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;

    fn roundtrip(src: &str) {
        let (mut doc, _) = parse(src);
        let out = emit(&doc);
        let (mut doc2, _) = parse(&out);
        doc.strip_spans();
        doc2.strip_spans();
        assert_eq!(
            doc.nodes, doc2.nodes,
            "roundtrip mismatch for {src:?}: emitted {out:?}"
        );
    }

    #[test]
    fn roundtrip_plain_text() {
        roundtrip("Hello world");
    }

    #[test]
    fn roundtrip_unresolved_command_and_group() {
        roundtrip("\\section{Intro}");
    }

    #[test]
    fn roundtrip_resolved_command() {
        roundtrip("\\newcommand{\\norm}[1]{\\lVert #1 \\rVert}\\norm{x}");
    }

    #[test]
    fn roundtrip_environment() {
        roundtrip("\\newenvironment{myenv}{[}{]}\\begin{myenv}x\\end{myenv}");
    }

    #[test]
    fn roundtrip_unresolved_environment() {
        roundtrip("\\begin{itemize}\\item a\\item b\\end{itemize}");
    }

    #[test]
    fn roundtrip_math() {
        roundtrip("$x^2$ and $$y=1$$");
    }

    #[test]
    fn roundtrip_verb_and_verbatim_env() {
        roundtrip("\\verb|a{b}|");
        roundtrip("\\begin{verbatim}a\\b{c}\\end{verbatim}");
    }

    #[test]
    fn roundtrip_tabular_align_and_rowbreak() {
        roundtrip("\\begin{tabular}a & b\\\\ c & d\\end{tabular}");
    }

    #[test]
    fn roundtrip_control_word_followed_by_letter_reinserts_space() {
        // `\foo` followed directly by the letter `b` in source (`\foobar`)
        // would be a single control word "foobar" on re-tokenization;
        // emit() must guard against accidentally producing that shape
        // when the AST says these are two separate nodes.
        roundtrip("\\def\\foo{a}\\foo bar");
    }
}
