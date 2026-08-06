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
//! round-trippable: `from_document(to_document(bytes))` reproduces the
//! same LaTeX source, since every construct — resolved or not — is always
//! captured as re-emittable source text, never dropped.
//!
//! No parsing/tokenizing/emitting logic lives in this module — it only
//! calls into `crate::parse::parse` and `crate::emit::emit_one`
//! (`crate::vocab`'s TeX/LaTeX kernel-mechanism recognition, `def`-family
//! definer name matching, and so on all stay in the standalone crate's
//! core, per this codebase's `CLAUDE.md`).

use crate::ast::{LatexDoc, Node as LatexNode, Severity as LatexSeverity};
use crate::emit::emit_one;
use crate::parse::parse as latex_parse;
use rescribe_core::{
    ConversionResult, Document, FidelityWarning, Node, Properties, Severity, WarningKind,
};
use rescribe_std::{node, prop};

/// LaTeX bytes -> `Document`.
pub fn to_document(input: &[u8]) -> ConversionResult<Document> {
    let text = String::from_utf8_lossy(input);
    let (doc, diags) = latex_parse(&text);
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

    let mut children = Vec::new();
    for n in &doc.nodes {
        children.push(node_to_ir(n));
    }

    let content = Node::new(node::DOCUMENT).children(children);
    let document = Document {
        content,
        resources: Default::default(),
        metadata: Properties::new(),
        source: None,
    };
    ConversionResult::with_warnings(document, warnings)
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

/// `Document` -> LaTeX bytes.
///
/// Each top-level IR node produced by [`to_document`] carries enough to
/// reconstruct exactly the source it came from: a `paragraph` wrapping a
/// single `text` child re-emits that text verbatim; anything else re-emits
/// its `latex:source` property verbatim. A `Document` this adapter did not
/// itself produce (e.g. hand-built via `rescribe_std::builder`) is handled
/// on a best-effort basis: a `paragraph` emits its text children
/// concatenated, anything carrying `latex:source` re-emits that raw
/// property, and anything else contributes nothing (rather than guessing
/// at LaTeX markup for IR shapes this narrow adapter doesn't model).
pub fn from_document(document: &Document) -> ConversionResult<Vec<u8>> {
    let mut out = String::new();
    for child in &document.content.children {
        emit_ir_node(child, &mut out);
    }
    ConversionResult::with_warnings(out.into_bytes(), Vec::new())
}

fn emit_ir_node(n: &Node, out: &mut String) {
    if let Some(src) = n.props.get_str("latex:source") {
        out.push_str(src);
        return;
    }
    if n.kind.as_str() == node::PARAGRAPH {
        for c in &n.children {
            if c.kind.as_str() == node::TEXT
                && let Some(t) = c.props.get_str(prop::CONTENT)
            {
                out.push_str(t);
            }
        }
    }
}

/// Convenience: parse LaTeX bytes into a [`LatexDoc`] AST directly (no IR
/// translation) — re-exported here for callers of this feature module that
/// want both entry points from one place.
pub fn parse(input: &[u8]) -> LatexDoc {
    let text = String::from_utf8_lossy(input);
    latex_parse(&text).0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_becomes_paragraph() {
        let result = to_document(b"Hello world");
        let doc = result.value;
        assert_eq!(doc.content.children[0].kind.as_str(), node::PARAGRAPH);
    }

    #[test]
    fn unresolved_command_becomes_raw_block_with_warning() {
        let result = to_document(b"\\section{Intro}");
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
        let src = b"\\begin{itemize}\\item a\\end{itemize}";
        let result = to_document(src);
        let bytes = from_document(&result.value).value;
        assert_eq!(bytes, src);
    }

    #[test]
    fn roundtrip_plain_text() {
        let src = b"Hello world";
        let result = to_document(src);
        let bytes = from_document(&result.value).value;
        assert_eq!(bytes, src);
    }

    #[test]
    fn roundtrip_math() {
        let src = b"$x^2$";
        let result = to_document(src);
        let bytes = from_document(&result.value).value;
        assert_eq!(bytes, src);
    }
}
