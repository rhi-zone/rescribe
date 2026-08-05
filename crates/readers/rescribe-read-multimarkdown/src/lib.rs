//! MultiMarkdown reader for rescribe.
//!
//! A thin AST→IR translator: all MultiMarkdown parsing (metadata blocks,
//! citations, cross-references, and everything CommonMark/GFM defines) lives
//! in the standalone `multimarkdown-fmt` crate. This crate only converts
//! `multimarkdown_fmt::MmdDoc` into a rescribe `Document`.

use multimarkdown_fmt::{MmdBlock, MmdDoc, MmdInline};
use rescribe_core::{ConversionResult, Document, ParseError, ParseOptions, PropValue, Properties};
use rescribe_format_api::Parse as _;
use rescribe_std::{Node, node, prop};

/// Parse MultiMarkdown input into a document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse MultiMarkdown input into a document with options.
pub fn parse_with_options(
    input: &str,
    _options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let (doc, diags) = MmdDoc::parse(input.as_bytes());

    let mut metadata = Properties::new();
    for entry in &doc.metadata {
        metadata.set(entry.key.clone(), PropValue::String(entry.value.clone()));
    }

    let content = Node::new(node::DOCUMENT).children(blocks_to_nodes(&doc.blocks));
    let document = Document {
        content,
        resources: Default::default(),
        metadata,
        source: None,
    };

    let warnings = diags
        .into_iter()
        .map(|d| {
            rescribe_core::FidelityWarning::new(
                rescribe_core::Severity::Minor,
                rescribe_core::WarningKind::FeatureLost(d.code.to_string()),
                d.message,
            )
        })
        .collect();
    Ok(ConversionResult::with_warnings(document, warnings))
}

fn blocks_to_nodes(blocks: &[MmdBlock]) -> Vec<Node> {
    blocks.iter().map(block_to_node).collect()
}

fn block_to_node(block: &MmdBlock) -> Node {
    match block {
        MmdBlock::Paragraph { inlines, .. } => {
            Node::new(node::PARAGRAPH).children(inlines_to_nodes(inlines))
        }
        MmdBlock::Heading {
            level,
            inlines,
            anchor,
            ..
        } => {
            let mut heading = Node::new(node::HEADING)
                .prop(prop::LEVEL, *level as i64)
                .children(inlines_to_nodes(inlines));
            if let Some(anchor) = anchor {
                heading = heading.prop(prop::ID, anchor.clone());
            }
            heading
        }
        MmdBlock::CodeBlock {
            language, content, ..
        } => {
            let mut code = Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone());
            if let Some(lang) = language {
                code = code.prop(prop::LANGUAGE, lang.clone());
            }
            code
        }
        MmdBlock::HtmlBlock { content, .. } => Node::new(node::RAW_BLOCK)
            .prop(prop::FORMAT, "html")
            .prop(prop::CONTENT, content.clone()),
        MmdBlock::Blockquote { blocks, .. } => {
            Node::new(node::BLOCKQUOTE).children(blocks_to_nodes(blocks))
        }
        MmdBlock::List { items, tight, .. } => {
            let ordered = matches!(items_ordered(block), Some(true));
            let mut list = Node::new(node::LIST)
                .prop(prop::ORDERED, ordered)
                .prop(prop::TIGHT, *tight);
            for item in items {
                let mut item_node =
                    Node::new(node::LIST_ITEM).children(blocks_to_nodes(&item.blocks));
                if let Some(checked) = item.checked {
                    item_node = item_node.prop(prop::CHECKED, checked);
                }
                list = list.child(item_node);
            }
            list
        }
        MmdBlock::ThematicBreak { .. } => Node::new(node::HORIZONTAL_RULE),
        MmdBlock::Table { head, rows, .. } => {
            let mut table = Node::new(node::TABLE);
            let mut head_row = Node::new(node::TABLE_ROW);
            for cell in &head.cells {
                head_row = head_row
                    .child(Node::new(node::TABLE_HEADER).children(inlines_to_nodes(&cell.inlines)));
            }
            table = table.child(head_row);
            for row in rows {
                let mut row_node = Node::new(node::TABLE_ROW);
                for cell in &row.cells {
                    row_node = row_node.child(
                        Node::new(node::TABLE_CELL).children(inlines_to_nodes(&cell.inlines)),
                    );
                }
                table = table.child(row_node);
            }
            table
        }
        MmdBlock::FootnoteDefinition { label, blocks, .. } => Node::new(node::FOOTNOTE_DEF)
            .prop(prop::ID, label.clone())
            .children(blocks_to_nodes(blocks)),
        MmdBlock::DefinitionList { items, .. } => {
            let mut dl = Node::new(node::DEFINITION_LIST);
            for item in items {
                dl = dl
                    .child(Node::new(node::DEFINITION_TERM).children(inlines_to_nodes(&item.term)));
                for def in &item.definitions {
                    dl = dl.child(Node::new(node::DEFINITION_DESC).children(blocks_to_nodes(def)));
                }
            }
            dl
        }
        MmdBlock::CitationDefinition { label, content, .. } => Node::new("mmd:citation_def")
            .prop(prop::ID, label.clone())
            .children(inlines_to_nodes(content)),
    }
}

/// `MmdBlock::List`'s `kind` field distinguishes ordered/unordered; pulled
/// out to a helper only to keep `block_to_node`'s `List` arm's destructuring
/// simple (the `kind` variant carries different fields per case).
fn items_ordered(block: &MmdBlock) -> Option<bool> {
    match block {
        MmdBlock::List { kind, .. } => {
            Some(matches!(kind, multimarkdown_fmt::ListKind::Ordered { .. }))
        }
        _ => None,
    }
}

fn inlines_to_nodes(inlines: &[MmdInline]) -> Vec<Node> {
    inlines.iter().map(inline_to_node).collect()
}

fn inline_to_node(inline: &MmdInline) -> Node {
    match inline {
        MmdInline::Text { content, .. } => {
            Node::new(node::TEXT).prop(prop::CONTENT, content.clone())
        }
        MmdInline::SoftBreak { .. } => Node::new(node::SOFT_BREAK),
        MmdInline::HardBreak { .. } => Node::new(node::LINE_BREAK),
        MmdInline::Emphasis { inlines, .. } => {
            Node::new(node::EMPHASIS).children(inlines_to_nodes(inlines))
        }
        MmdInline::Strong { inlines, .. } => {
            Node::new(node::STRONG).children(inlines_to_nodes(inlines))
        }
        MmdInline::Strikethrough { inlines, .. } => {
            Node::new(node::STRIKEOUT).children(inlines_to_nodes(inlines))
        }
        MmdInline::Code { content, .. } => {
            Node::new(node::CODE).prop(prop::CONTENT, content.clone())
        }
        MmdInline::HtmlInline { content, .. } => Node::new(node::RAW_INLINE)
            .prop(prop::FORMAT, "html")
            .prop(prop::CONTENT, content.clone()),
        MmdInline::Link {
            inlines,
            url,
            title,
            ..
        } => {
            let mut link = Node::new(node::LINK)
                .prop(prop::URL, url.clone())
                .children(inlines_to_nodes(inlines));
            if let Some(title) = title {
                link = link.prop(prop::TITLE, title.clone());
            }
            link
        }
        MmdInline::Image {
            alt, url, title, ..
        } => {
            let mut img = Node::new(node::IMAGE)
                .prop(prop::URL, url.clone())
                .prop(prop::ALT, alt.clone());
            if let Some(title) = title {
                img = img.prop(prop::TITLE, title.clone());
            }
            img
        }
        MmdInline::FootnoteReference { label, .. } => {
            Node::new(node::FOOTNOTE_REF).prop(prop::ID, label.clone())
        }
        MmdInline::InlineMath { source, .. } => {
            Node::new("math_inline").prop(prop::CONTENT, source.clone())
        }
        MmdInline::DisplayMath { source, .. } => {
            Node::new("math_block").prop(prop::CONTENT, source.clone())
        }
        MmdInline::Citation { locator, label, .. } => {
            let mut n = Node::new("mmd:citation").prop("label", label.clone());
            if let Some(locator) = locator {
                n = n.prop("locator", locator.clone());
            }
            n
        }
        MmdInline::CrossReference {
            target, collapsed, ..
        } => Node::new("mmd:cross_reference")
            .prop("target", target.clone())
            .prop("collapsed", *collapsed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic() {
        let md = "# Hello\n\nThis is a paragraph.";
        let result = parse(md).unwrap();
        let doc = result.value;
        assert_eq!(doc.content.children.len(), 2);
    }

    #[test]
    fn test_parse_footnote() {
        let md = "Here is a footnote[^1].\n\n[^1]: This is the footnote.";
        let result = parse(md).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_definition_list() {
        let md = "Term\n: Definition";
        let result = parse(md).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_table() {
        let md = "| A | B |\n|---|---|\n| 1 | 2 |";
        let result = parse(md).unwrap();
        let doc = result.value;
        let table = &doc.content.children[0];
        assert_eq!(table.kind.as_str(), node::TABLE);
    }

    #[test]
    fn test_parse_math() {
        let md = "Inline $x^2$ math and display $$y = mx + b$$ math.";
        let result = parse(md).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_metadata() {
        let md = "Title: My Document\nAuthor: Jane Doe\n\n# Heading\n";
        let result = parse(md).unwrap();
        let doc = result.value;
        assert_eq!(doc.metadata.get_str("Title"), Some("My Document"));
        assert_eq!(doc.content.children.len(), 1);
    }

    #[test]
    fn test_parse_citation() {
        let md = "See[p. 23][#Doe:2006] for details.\n\n[#Doe:2006]: John Doe.\n";
        let result = parse(md).unwrap();
        let doc = result.value;
        let para = &doc.content.children[0];
        assert!(
            para.children
                .iter()
                .any(|n| n.kind.as_str() == "mmd:citation")
        );
        assert_eq!(doc.content.children[1].kind.as_str(), "mmd:citation_def");
    }

    #[test]
    fn test_parse_cross_reference() {
        let md = "### Overview [MultiMarkdownOverview] ###\n\nSee [MultiMarkdownOverview] above.\n";
        let result = parse(md).unwrap();
        let doc = result.value;
        assert_eq!(
            doc.content.children[0].props.get_str(prop::ID),
            Some("MultiMarkdownOverview")
        );
        let para = &doc.content.children[1];
        assert!(
            para.children
                .iter()
                .any(|n| n.kind.as_str() == "mmd:cross_reference")
        );
    }
}
