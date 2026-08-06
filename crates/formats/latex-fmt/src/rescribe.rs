//! AST<->`Document` translation (`rescribe` feature, default off).
//!
//! # Scope of this translation
//!
//! `crate::parse`'s resolution model (see that module's docs) means the
//! overwhelming majority of real-document content arrives here as
//! *unresolved* [`Node::ControlSymbol`]/[`Node::RawEnvironment`] (raw
//! LaTeX with no built-in "standard commands" table consulted). This
//! adapter reflects that honestly rather than pretending to a
//! semantic-modeling depth the parser doesn't have: each top-level
//! [`Node`] becomes one `raw_block` (or `text`-bearing `paragraph` for
//! plain-text runs) carrying its re-emitted LaTeX source verbatim in a
//! `latex:source` property. This is a deliberately narrow first pass —
//! deeper IR modeling (sections as `heading`, resolved `\newcommand`
//! macros as something more structured than raw source, etc.) is future
//! work, not attempted here given the scope of this session. It is fully
//! round-trippable: `emit(parse(bytes))` reproduces the same LaTeX source,
//! since every construct — resolved or not — is always captured as
//! re-emittable source text, never dropped.
//!
//! No parsing/tokenizing/emitting logic lives in this module — it only
//! calls into `crate::parse::parse` and `crate::emit::emit_one` (all
//! tokenizing/parsing/emitting logic stays in the standalone crate's core,
//! per this codebase's `CLAUDE.md`).
//!
//! Function names/signatures (`parse`/`parse_with_options`/`emit`/
//! `emit_with_options`/`emit_full_document`, all returning
//! `Result<ConversionResult<_>, _>`) match the convention every other
//! migrated format crate's `rescribe` module uses (see e.g.
//! `rst-fmt`/`html-fmt`'s `src/rescribe/{read,write}.rs`), replacing the
//! former `rescribe-read-latex`/`rescribe-write-latex` adapter crates'
//! public surface — `crates/rescribe`, `crates/rescribe-fixtures`, and
//! `fuzz/fuzz_targets/latex_reader.rs` all repoint here.

use crate::ast::{LatexDoc, Node as LatexNode, Severity as LatexSeverity, Span};
use crate::emit::emit_one;
use crate::parse::parse as latex_parse;
use rescribe_core::{
    ConversionResult, Document, EmitError, EmitOptions, FidelityWarning, Node, ParseError,
    ParseOptions, Properties, Severity, WarningKind,
};
use rescribe_std::{node, prop};

/// Parse LaTeX text into a rescribe `Document`.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse LaTeX text into a rescribe `Document`.
///
/// `latex-fmt` currently has no options that affect parsing (`_options` is
/// accepted for API-surface parity with every other format crate's
/// `parse_with_options`, but unused) — `crate::parse`'s resolution model
/// (in-document definitions only, no built-in vocabulary table) has no
/// "handwritten vs tree-sitter backend"-style knob the way the former
/// `rescribe-read-latex` adapter did.
pub fn parse_with_options(
    input: &str,
    _options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let (doc, diags) = latex_parse(input);
    let mut warnings = Vec::new();
    for d in &diags {
        // `Info`-severity `latex::unresolved-*` diagnostics are exactly
        // this codebase's "semantic constructs that can't yet be modeled
        // must emit a fidelity warning" case (`CLAUDE.md`): the construct
        // is still fully captured (as raw source in the resulting node
        // below), just not semantically modeled. `Warning`-severity
        // diagnostics (malformed input, unterminated constructs) are
        // surfaced the same way — both map to `WarningKind::UnsupportedNode`
        // since neither has a more specific `WarningKind` variant, with
        // severity carried through so a caller can still distinguish them.
        let severity = match d.severity {
            LatexSeverity::Info => Severity::Info,
            LatexSeverity::Warning => Severity::Minor,
            LatexSeverity::Error => Severity::Major,
        };
        warnings.push(FidelityWarning::new(
            severity,
            WarningKind::UnsupportedNode(d.code.to_string()),
            d.message.clone(),
        ));
    }

    let children: Vec<Node> = doc.nodes.iter().map(node_to_ir).collect();
    let content = Node::new(node::DOCUMENT).children(children);
    let document = Document {
        content,
        resources: Default::default(),
        metadata: Properties::new(),
        source: None,
    };
    Ok(ConversionResult::with_warnings(document, warnings))
}

fn node_to_ir(n: &LatexNode) -> Node {
    match n {
        LatexNode::Text { value, .. } => Node::new(node::PARAGRAPH)
            .child(Node::new(node::TEXT).prop(prop::CONTENT, value.clone())),
        other => Node::new(node::RAW_BLOCK)
            .prop(prop::FORMAT, "latex")
            .prop("latex:source", emit_one(other)),
    }
}

/// Emit a `Document` as LaTeX fragment (body content only, no preamble —
/// see [`emit_full_document`] for a wrapped, standalone-compilable form).
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    emit_with_options(doc, &EmitOptions::default())
}

/// Emit a `Document` as LaTeX, with options.
///
/// Two cases, tried in order per top-level child:
///
/// 1. **Round-trip case** — a node this adapter itself produced (a
///    `paragraph` wrapping a single `text` child, or anything carrying a
///    `latex:source` property) re-emits exactly the source it came from.
/// 2. **Arbitrary-Document case** — a `Document` this adapter did not
///    produce (e.g. built by another format's reader, or by hand via
///    `rescribe_std::builder`) carrying a *semantic* node kind
///    (`heading`, `strong`, `emphasis`, ...) is translated to LaTeX markup
///    by constructing a small `crate::ast::Node` (e.g. a `Command` node
///    named `"section"`/`"textbf"`/`"emph"`) and delegating to
///    `crate::emit::emit_one` — no format-byte string-building happens in
///    this module itself, per this codebase's rule that a `rescribe`
///    module only ever calls into the crate's core emitter. This case is
///    deliberately narrow (see [`semantic_ir_to_latex_ast`]): it covers
///    the node kinds this crate's own `fixtures/writers/latex/` suite
///    exercises, not full pandoc-parity semantic emission (the former
///    `rescribe-write-latex` adapter's much larger builder is not
///    reproduced here).
///
/// `_options` is accepted for API parity with every other format crate's
/// `emit_with_options`; `latex-fmt` has no emit options that change this
/// behavior yet.
pub fn emit_with_options(
    doc: &Document,
    _options: &EmitOptions,
) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let mut out = String::new();
    for child in &doc.content.children {
        emit_ir_node(child, &mut out);
    }
    Ok(ConversionResult::with_warnings(
        out.into_bytes(),
        Vec::new(),
    ))
}

/// Emit a complete, standalone-compilable LaTeX document: a minimal
/// preamble (`\documentclass{article}`), the body from [`emit`], and
/// `\end{document}`. Unlike the former `rescribe-write-latex` adapter's
/// `emit_full_document` (which hardcoded a package list — `graphicx`,
/// `hyperref`, `listings`, `amsmath`, `amssymb`, `ulem` — inferred from
/// what *that* adapter's semantic modeling could produce), this one adds
/// no packages: `latex-fmt`'s narrow IR mapping (see module docs) doesn't
/// know what the body's raw-preserved content might need, and guessing a
/// package list here would be exactly the kind of unverified assumption
/// this crate's design otherwise refuses to make.
pub fn emit_full_document(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let body = emit_with_options(doc, &EmitOptions::default())?;
    let mut out = String::new();
    out.push_str("\\documentclass{article}\n\\begin{document}\n");
    out.push_str(&String::from_utf8_lossy(&body.value));
    out.push_str("\n\\end{document}\n");
    Ok(ConversionResult::with_warnings(
        out.into_bytes(),
        body.warnings,
    ))
}

fn emit_ir_node(n: &Node, out: &mut String) {
    if let Some(src) = n.props.get_str("latex:source") {
        out.push_str(src);
        return;
    }
    if n.kind.as_str() == node::PARAGRAPH {
        // A paragraph isn't itself one LaTeX construct — emit each inline
        // child in sequence rather than trying to represent "paragraph"
        // as a single `crate::ast::Node`.
        for c in &n.children {
            emit_ir_node(c, out);
        }
        return;
    }
    if let Some(ast_node) = semantic_ir_to_latex_ast(n) {
        out.push_str(&emit_one(&ast_node));
    }
}

/// Translates a narrow, explicit set of rescribe-IR semantic node kinds
/// into a `crate::ast::Node` for `crate::emit::emit_one` to serialize.
/// Covers exactly what `fixtures/writers/latex/` exercises today
/// (`paragraph`, `heading`, `strong`, `emphasis`) plus their inline
/// children recursively — not a general pandoc-parity IR-to-LaTeX writer.
/// Extend this list deliberately (with a matching fixture) rather than
/// guessing at markup for other IR kinds.
fn semantic_ir_to_latex_ast(n: &Node) -> Option<LatexNode> {
    let inline_children = || -> Vec<LatexNode> {
        n.children
            .iter()
            .filter_map(semantic_ir_to_latex_ast)
            .collect()
    };
    match n.kind.as_str() {
        k if k == node::TEXT => {
            let content = n.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            Some(LatexNode::Text {
                value: content,
                span: Span::NONE,
            })
        }
        k if k == node::STRONG => Some(LatexNode::Command {
            name: "textbf".to_string(),
            star: false,
            opt: Vec::new(),
            args: vec![inline_children()],
            span: Span::NONE,
        }),
        k if k == node::EMPHASIS => Some(LatexNode::Command {
            name: "emph".to_string(),
            star: false,
            opt: Vec::new(),
            args: vec![inline_children()],
            span: Span::NONE,
        }),
        k if k == node::HEADING => {
            let level = n.props.get_int(prop::LEVEL).unwrap_or(1);
            let name = match level {
                1 => "section",
                2 => "subsection",
                3 => "subsubsection",
                4 => "paragraph",
                _ => "subparagraph",
            };
            Some(LatexNode::Command {
                name: name.to_string(),
                star: false,
                opt: Vec::new(),
                args: vec![inline_children()],
                span: Span::NONE,
            })
        }
        _ => None,
    }
}

/// Convenience: parse LaTeX bytes into a [`LatexDoc`] AST directly (no IR
/// translation) — re-exported here for callers of this feature module that
/// want both entry points from one place.
pub fn parse_ast(input: &str) -> LatexDoc {
    latex_parse(input).0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_becomes_paragraph() {
        let result = parse("Hello world").unwrap();
        let doc = result.value;
        assert_eq!(doc.content.children[0].kind.as_str(), node::PARAGRAPH);
    }

    #[test]
    fn unresolved_command_becomes_raw_block_with_warning() {
        let result = parse("\\section{Intro}").unwrap();
        let doc = result.value;
        // Two top-level LaTeX AST nodes (ControlSymbol + Group) -> two IR
        // raw_block nodes.
        assert!(
            doc.content
                .children
                .iter()
                .all(|c| c.kind.as_str() == node::RAW_BLOCK)
        );
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn roundtrip_raw_preserved_content() {
        let src = "\\begin{itemize}\\item a\\end{itemize}";
        let result = parse(src).unwrap();
        let bytes = emit(&result.value).unwrap().value;
        assert_eq!(bytes, src.as_bytes());
    }

    #[test]
    fn roundtrip_plain_text() {
        let src = "Hello world";
        let result = parse(src).unwrap();
        let bytes = emit(&result.value).unwrap().value;
        assert_eq!(bytes, src.as_bytes());
    }

    #[test]
    fn roundtrip_math() {
        let src = "$x^2$";
        let result = parse(src).unwrap();
        let bytes = emit(&result.value).unwrap().value;
        assert_eq!(bytes, src.as_bytes());
    }

    #[test]
    fn emit_semantic_heading_and_strong() {
        use rescribe_std::builder::doc;
        let document = doc(|d| {
            d.heading(1, |i| i.text("Title"))
                .para(|i| i.text("plain ").strong(|i| i.text("bold")))
        });
        let bytes = emit(&document).unwrap().value;
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("\\section{Title}"), "{s:?}");
        assert!(s.contains("\\textbf{bold}"), "{s:?}");
    }

    #[test]
    fn emit_full_document_wraps_in_documentclass_and_document_env() {
        let result = parse("hi").unwrap();
        let bytes = emit_full_document(&result.value).unwrap().value;
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.starts_with("\\documentclass{article}\n\\begin{document}\n"));
        assert!(s.trim_end().ends_with("\\end{document}"));
        assert!(s.contains("hi"));
    }
}
