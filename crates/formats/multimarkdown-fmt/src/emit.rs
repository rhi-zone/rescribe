//! MultiMarkdown builder writer: converts an [`MmdDoc`] back to
//! MultiMarkdown bytes.
//!
//! Metadata is written directly (`crate::metadata::emit` — again, genuine
//! MMD-only grammar). MMD-unique inline/block nodes are spelled back out as
//! the literal bracket text CommonMark itself would produce for an
//! unresolved reference
//! (`crate::transform::mmd_to_cm_blocks`); actual CommonMark writing is then
//! fully delegated to `commonmark_fmt::emit::emit` — this crate never
//! hand-writes Markdown syntax.

use crate::ast::MmdDoc;
use crate::{metadata, transform};

/// Emit an [`MmdDoc`] as MultiMarkdown bytes.
pub fn emit(doc: &MmdDoc) -> Vec<u8> {
    let mut out = String::new();
    metadata::emit(&doc.metadata, doc.metadata_style, &mut out);

    let cm_blocks = transform::mmd_to_cm_blocks(&doc.blocks);
    let cmdoc = commonmark_fmt::CmDoc {
        blocks: cm_blocks,
        link_defs: doc.link_defs.clone(),
        // MMD's own `---`-delimited metadata (crate::metadata) is stripped
        // before the body ever reaches commonmark-fmt, so CommonMark-level
        // frontmatter never applies here. Always requested (Cargo.toml)
        // rather than left feature-conditional so this struct literal stays
        // correct regardless of what other crates in a shared build graph
        // enable via Cargo feature unification.
        frontmatter: None,
    };
    let body = commonmark_fmt::emit::emit(&cmdoc);
    out.push_str(std::str::from_utf8(&body).expect("commonmark-fmt emits valid UTF-8"));
    out.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::*;

    #[test]
    fn emits_metadata_and_body() {
        let doc = MmdDoc {
            metadata: vec![MetadataEntry {
                key: "Title".to_string(),
                value: "My Document".to_string(),
            }],
            metadata_style: MetadataStyle::Bare,
            blocks: vec![MmdBlock::Paragraph {
                inlines: vec![MmdInline::Text {
                    content: "Hello.".to_string(),
                    span: Span::NONE,
                }],
                span: Span::NONE,
            }],
            link_defs: Vec::new(),
        };
        let out = String::from_utf8(emit(&doc)).unwrap();
        assert!(out.starts_with("Title: My Document\n\n"));
        assert!(out.contains("Hello."));
    }

    #[test]
    fn emits_citation() {
        let doc = MmdDoc {
            metadata: Vec::new(),
            metadata_style: MetadataStyle::None,
            blocks: vec![MmdBlock::Paragraph {
                inlines: vec![MmdInline::Citation {
                    locator: Some("p. 23".to_string()),
                    label: "Doe:2006".to_string(),
                    span: Span::NONE,
                }],
                span: Span::NONE,
            }],
            link_defs: Vec::new(),
        };
        let out = String::from_utf8(emit(&doc)).unwrap();
        // commonmark-fmt's plain-text escaper backslash-escapes literal `[`
        // (needed so it can't be mistaken for real link syntax on reparse) —
        // that's invisible after reparsing, which is what actually matters.
        let (doc2, diags2) = crate::parse::parse(out.as_bytes());
        assert!(diags2.is_empty());
        assert_eq!(doc.strip_spans(), doc2.strip_spans());
    }

    #[test]
    fn emits_citation_definition() {
        let doc = MmdDoc {
            metadata: Vec::new(),
            metadata_style: MetadataStyle::None,
            blocks: vec![MmdBlock::CitationDefinition {
                label: "Doe:2006".to_string(),
                content: vec![MmdInline::Text {
                    content: "John Doe.".to_string(),
                    span: Span::NONE,
                }],
                span: Span::NONE,
            }],
            link_defs: Vec::new(),
        };
        let out = String::from_utf8(emit(&doc)).unwrap();
        assert!(out.contains("[#Doe:2006]: John Doe."));
    }
}
