//! MultiMarkdown AST reader.
//!
//! All CommonMark tokenizing is delegated to
//! `commonmark_fmt::parse::parse_str`; this module only extracts the leading
//! metadata block (`crate::metadata`, genuine MMD grammar with no
//! commonmark-fmt equivalent) and post-processes the resulting `CmDoc`
//! (`crate::transform::cm_to_mmd_blocks`) to recognize citations,
//! cross-references, and citation definitions.

use crate::ast::{Diagnostic, MetadataStyle, MmdDoc, Span};
use crate::{metadata, transform};

/// Parse MultiMarkdown input into an [`MmdDoc`].
///
/// Not part of the crate's public free-function surface — use
/// [`rescribe_format_api::Parse::parse`] on [`MmdDoc`] instead. Kept
/// `pub(crate)` because it's the implementation the trait impl and this
/// crate's own tests call directly.
pub(crate) fn parse(input: &[u8]) -> (MmdDoc, Vec<Diagnostic>) {
    match std::str::from_utf8(input) {
        Ok(s) => parse_str(s),
        Err(_) => (
            MmdDoc {
                metadata: Vec::new(),
                metadata_style: MetadataStyle::None,
                blocks: Vec::new(),
                link_defs: Vec::new(),
            },
            vec![Diagnostic {
                span: Span::NONE,
                severity: crate::ast::Severity::Error,
                message: "input is not valid UTF-8".to_string(),
                code: "multimarkdown::invalid-utf8",
            }],
        ),
    }
}

/// Parse a MultiMarkdown `&str` into an [`MmdDoc`].
pub fn parse_str(input: &str) -> (MmdDoc, Vec<Diagnostic>) {
    let (meta_entries, style, rest) = metadata::extract(input);
    let (cmdoc, cm_diags) = commonmark_fmt::parse::parse_str(rest);
    let blocks = transform::cm_to_mmd_blocks(&cmdoc.blocks);
    let diags = cm_diags
        .into_iter()
        .map(|d| Diagnostic {
            span: d.span,
            severity: d.severity,
            message: d.message,
            code: d.code,
        })
        .collect();
    (
        MmdDoc {
            metadata: meta_entries,
            metadata_style: style,
            blocks,
            link_defs: cmdoc.link_defs,
        },
        diags,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::MmdBlock;

    #[test]
    fn parses_metadata_and_body() {
        let (doc, diags) = parse(b"Title: My Document\n\n# Heading\n\nParagraph.\n");
        assert!(diags.is_empty());
        assert_eq!(doc.metadata_style, MetadataStyle::Bare);
        assert_eq!(doc.metadata[0].key, "Title");
        assert_eq!(doc.blocks.len(), 2);
        assert!(matches!(doc.blocks[0], MmdBlock::Heading { .. }));
    }

    #[test]
    fn parses_citation_definition_block() {
        let (doc, _diags) = parse(b"[#Doe:2006]: John Doe. *A Book*.\n");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(
            &doc.blocks[0],
            MmdBlock::CitationDefinition { label, .. } if label == "Doe:2006"
        ));
    }

    #[test]
    fn parses_heading_anchor() {
        let (doc, _diags) = parse(b"### Overview [MultiMarkdownOverview] ###\n");
        assert!(matches!(
            &doc.blocks[0],
            MmdBlock::Heading { anchor: Some(a), .. } if a == "MultiMarkdownOverview"
        ));
    }
}
