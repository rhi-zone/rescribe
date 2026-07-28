//! Markdown reader for rescribe.
//!
//! Parses CommonMark (with GFM strikethrough extension) into rescribe's document IR
//! using the `commonmark-fmt` crate as the default backend.
//!
//! An optional `pulldown` feature enables the legacy pulldown-cmark backend
//! (with YAML/TOML frontmatter, tables, footnotes, etc.) under `backend_pulldown`.

use rescribe_core::{ConversionResult, Document, ParseError, ParseOptions};

mod commonmark;

#[cfg(feature = "pulldown")]
mod pulldown;

/// Parse markdown text into a rescribe Document.
///
/// Uses the `commonmark-fmt` backend (CommonMark + GFM strikethrough).
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse markdown with custom options.
pub fn parse_with_options(
    input: &str,
    opts: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    Ok(commonmark::parse_with_options(input.as_bytes(), opts))
}

/// Parse using specifically the pulldown-cmark backend (legacy; requires the `pulldown` feature).
///
/// The pulldown backend supports additional extensions: YAML/TOML frontmatter, GFM tables,
/// footnotes, task lists, and math. Enable with `features = ["pulldown"]`.
#[cfg(feature = "pulldown")]
pub mod backend_pulldown {
    pub use crate::pulldown::{parse, parse_with_options};
}

#[cfg(test)]
mod tests {
    use super::*;
    use rescribe_std::{node, prop};

    fn root_children(doc: &Document) -> &[rescribe_std::Node] {
        &doc.content.children
    }

    #[test]
    fn test_parse_paragraph() {
        let result = parse("Hello, world!").unwrap();
        let doc = result.value;
        let children = root_children(&doc);
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].kind.as_str(), node::PARAGRAPH);
    }

    #[test]
    fn test_parse_heading() {
        let result = parse("# Heading 1\n\n## Heading 2").unwrap();
        let doc = result.value;
        let children = root_children(&doc);
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].kind.as_str(), node::HEADING);
        assert_eq!(children[0].props.get_int(prop::LEVEL), Some(1));
        assert_eq!(children[1].props.get_int(prop::LEVEL), Some(2));
    }

    #[test]
    fn test_parse_emphasis() {
        let result = parse("*italic* and **bold**").unwrap();
        let doc = result.value;
        let children = root_children(&doc);
        let para = &children[0];
        assert!(
            para.children
                .iter()
                .any(|n| n.kind.as_str() == node::EMPHASIS)
        );
        assert!(
            para.children
                .iter()
                .any(|n| n.kind.as_str() == node::STRONG)
        );
    }

    #[test]
    fn test_parse_link() {
        let result = parse("[example](https://example.com)").unwrap();
        let doc = result.value;
        let children = root_children(&doc);
        let para = &children[0];
        let link = &para.children[0];
        assert_eq!(link.kind.as_str(), node::LINK);
        assert_eq!(link.props.get_str(prop::URL), Some("https://example.com"));
    }

    #[test]
    fn test_parse_code_block() {
        let result = parse("```rust\nfn main() {}\n```").unwrap();
        let doc = result.value;
        let children = root_children(&doc);
        assert_eq!(children[0].kind.as_str(), node::CODE_BLOCK);
        assert_eq!(children[0].props.get_str(prop::LANGUAGE), Some("rust"));
    }

    #[test]
    fn test_parse_list() {
        let result = parse("- item 1\n- item 2").unwrap();
        let doc = result.value;
        let children = root_children(&doc);
        assert_eq!(children[0].kind.as_str(), node::LIST);
        assert_eq!(children[0].props.get_bool(prop::ORDERED), Some(false));
        assert_eq!(children[0].children.len(), 2);
    }

    #[test]
    fn test_parse_ordered_list() {
        let result = parse("1. first\n2. second").unwrap();
        let doc = result.value;
        let children = root_children(&doc);
        assert_eq!(children[0].kind.as_str(), node::LIST);
        assert_eq!(children[0].props.get_bool(prop::ORDERED), Some(true));
    }

    #[test]
    #[cfg(feature = "pulldown")]
    fn test_parse_yaml_frontmatter() {
        let input = r#"---
title: My Document
author: John Doe
date: 2024-01-15
draft: true
tags:
  - rust
  - markdown
---

# Hello

Content here."#;
        let result = backend_pulldown::parse(input).unwrap();
        let doc = result.value;

        assert_eq!(doc.metadata.get_str("title"), Some("My Document"));
        assert_eq!(doc.metadata.get_str("author"), Some("John Doe"));
        assert_eq!(doc.metadata.get_str("date"), Some("2024-01-15"));
        assert_eq!(doc.metadata.get_bool("draft"), Some(true));
        let tags = doc.metadata.get("tags");
        assert!(tags.is_some());
        if let Some(rescribe_core::PropValue::List(items)) = tags {
            assert_eq!(items.len(), 2);
        }

        let children = root_children(&doc);
        assert!(!children.is_empty());
        assert_eq!(children[0].kind.as_str(), node::HEADING);
    }

    #[test]
    #[cfg(feature = "pulldown")]
    fn test_parse_nested_yaml_frontmatter() {
        let input = r#"---
title: My Document
author:
  name: John Doe
  email: john@example.com
---

# Hello"#;
        let result = backend_pulldown::parse(input).unwrap();
        let doc = result.value;

        assert_eq!(doc.metadata.get_str("title"), Some("My Document"));
        assert_eq!(doc.metadata.get_str("author.name"), Some("John Doe"));
        assert_eq!(
            doc.metadata.get_str("author.email"),
            Some("john@example.com")
        );
    }

    #[test]
    fn test_preserve_source_info() {
        // commonmark-fmt always records spans; check that spans are present.
        let input = "# Hello\n\nWorld!";
        let result = parse(input).unwrap();
        let doc = result.value;
        let children = root_children(&doc);

        let heading = &children[0];
        assert!(heading.span.is_some());
        let span = heading.span.unwrap();
        assert_eq!(span.start, 0);
        assert!(span.end > span.start);

        let para = &children[1];
        assert!(para.span.is_some());
    }

    #[test]
    fn test_default_backend_yaml_frontmatter_no_bogus_nodes() {
        // Regression test for the headline bug this vertical fixes: on the
        // DEFAULT path (no backend override), `---\ntitle: X\n---` used to
        // silently misparse as a horizontal_rule + a setext heading("title:
        // X") in the document body, with doc.metadata left empty and no
        // fidelity warning. It must now populate metadata and emit no bogus
        // structural nodes.
        let input = "---\ntitle: X\n---\n\nbody\n";
        let result = parse(input).unwrap();
        let doc = result.value;

        assert_eq!(doc.metadata.get_str("title"), Some("X"));

        let children = root_children(&doc);
        assert!(
            !children
                .iter()
                .any(|n| n.kind.as_str() == node::HORIZONTAL_RULE
                    || n.kind.as_str() == node::HEADING),
            "body should not contain a bogus horizontal_rule/heading from misparsed \
             frontmatter, got: {:?}",
            children.iter().map(|n| n.kind.as_str()).collect::<Vec<_>>()
        );
        assert!(children.iter().any(|n| n.kind.as_str() == node::PARAGRAPH));
    }

    #[test]
    fn test_default_backend_toml_frontmatter() {
        let input = "+++\ntitle = \"X\"\n+++\n\nbody\n";
        let result = parse(input).unwrap();
        let doc = result.value;
        assert_eq!(doc.metadata.get_str("title"), Some("X"));
    }

    #[test]
    #[cfg(feature = "pulldown")]
    fn test_backends_agree_on_representative_documents() {
        // Parse the same representative documents through both the default
        // (commonmark-fmt) backend and the pulldown backend, and assert the
        // resulting Documents agree structurally (span-free comparison via
        // node kind + key props), for constructs both backends model.
        let docs = [
            "# Title\n\nA paragraph with *em* and **strong**.\n",
            "- one\n- two\n- three\n",
            "1. first\n2. second\n",
            "> quoted\n",
            "```rust\nfn main() {}\n```\n",
            "[text](https://example.com)\n",
            "---\ntitle: X\nauthor: Y\n---\n\nbody\n",
            "+++\ntitle = \"X\"\n+++\n\nbody\n",
            "| a | b |\n|---|---|\n| 1 | 2 |\n",
            "- [ ] todo\n- [x] done\n",
            "~~deleted~~\n",
        ];
        for input in docs {
            let default_doc = parse(input).unwrap().value;
            let pulldown_doc = backend_pulldown::parse(input).unwrap().value;
            assert_eq!(
                default_doc.metadata, pulldown_doc.metadata,
                "metadata mismatch for input: {input:?}"
            );
            assert_eq!(
                node_kind_shape(&default_doc.content),
                node_kind_shape(&pulldown_doc.content),
                "node-kind shape mismatch for input: {input:?}"
            );
        }
    }

    /// Reduce a node tree to just its kind names (ignoring spans/most props),
    /// for a structural-shape comparison between the two backends.
    #[cfg(feature = "pulldown")]
    fn node_kind_shape(node: &rescribe_std::Node) -> Vec<String> {
        let mut out = vec![node.kind.as_str().to_string()];
        for child in &node.children {
            out.extend(node_kind_shape(child));
        }
        out
    }

    #[test]
    fn test_parse_task_list() {
        // Task list markers are not modeled by commonmark-fmt (they're a GFM extension).
        // This test just checks that task-list syntax parses without panic.
        let result = parse("- [ ] unchecked\n- [x] checked").unwrap();
        let doc = result.value;
        let children = root_children(&doc);
        let list = &children[0];
        assert_eq!(list.kind.as_str(), node::LIST);
        assert_eq!(list.children.len(), 2);
    }
}
