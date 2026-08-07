//! Typst reader for rescribe.
//!
//! Thin AST→IR translator over `typst-fmt`'s `TypstDoc`/`Block`/`Inline` —
//! all Typst parsing lives in the rest of this crate (see its module
//! docs), not here. This module's only job is mapping `typst-fmt`'s
//! domain-typed AST onto rescribe's `Node` tree; the construct mapping
//! (which node kind/property each `Block`/`Inline` variant becomes) is
//! unchanged from the pre-collapse `rescribe-read-typst` crate.

use crate::{Block, Inline, TypstDoc};
use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions};
use rescribe_format_api::Parse as _;
use rescribe_std::{node, prop};

/// Parse Typst source into a document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse Typst source with custom options.
pub fn parse_with_options(
    input: &str,
    _options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let (doc, _diags) = TypstDoc::parse(input.as_bytes());
    let children: Vec<Node> = doc.blocks.iter().map(convert_block).collect();
    let doc_node = Node::new(node::DOCUMENT).children(children);
    let doc = Document::new().with_content(doc_node);
    Ok(ConversionResult::ok(doc))
}

fn convert_blocks(blocks: &[Block]) -> Vec<Node> {
    blocks.iter().map(convert_block).collect()
}

fn convert_inlines(inlines: &[Inline]) -> Vec<Node> {
    inlines.iter().map(convert_inline).collect()
}

fn convert_block(block: &Block) -> Node {
    match block {
        Block::Paragraph(inlines) => Node::new(node::PARAGRAPH).children(convert_inlines(inlines)),
        Block::Heading { level, body } => Node::new(node::HEADING)
            .prop(prop::LEVEL, *level as i64)
            .children(convert_inlines(body)),
        Block::CodeBlock { lang, content } => {
            let mut n = Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone());
            if let Some(lang) = lang
                && !lang.is_empty()
            {
                n = n.prop(prop::LANGUAGE, lang.clone());
            }
            n
        }
        Block::List { ordered, items } => Node::new(node::LIST)
            .prop(prop::ORDERED, *ordered)
            .children(items.iter().map(|item| convert_list_item(item))),
        Block::DefinitionList(entries) => {
            let mut children = Vec::with_capacity(entries.len() * 2);
            for (term, desc) in entries {
                children.push(Node::new(node::DEFINITION_TERM).children(convert_inlines(term)));
                children.push(Node::new(node::DEFINITION_DESC).children(convert_inlines(desc)));
            }
            Node::new(node::DEFINITION_LIST).children(children)
        }
        Block::Quote(body) => Node::new(node::BLOCKQUOTE).children(convert_blocks(body)),
        Block::Table { columns, rows } => {
            let row_nodes: Vec<Node> =
                rows.iter()
                    .map(|row| {
                        Node::new(node::TABLE_ROW).children(row.iter().map(|cell| {
                            Node::new(node::TABLE_CELL).children(convert_inlines(cell))
                        }))
                    })
                    .collect();
            Node::new(node::TABLE)
                .prop("columns", *columns as i64)
                .children(row_nodes)
        }
        Block::Figure { body, caption } => {
            let mut children = Vec::new();
            if let Some(b) = body {
                children.push(convert_block(b));
            }
            if let Some(cap) = caption {
                children.push(Node::new(node::PARAGRAPH).children(convert_inlines(cap)));
            }
            Node::new(node::FIGURE).children(children)
        }
        Block::HorizontalRule => Node::new(node::HORIZONTAL_RULE),
        Block::MathDisplay(source) => Node::new("math_display").prop("math:source", source.clone()),
        Block::Image { url } => Node::new(node::IMAGE).prop(prop::URL, url.clone()),
        Block::Raw(text) => Node::new(node::RAW_BLOCK)
            .prop(prop::FORMAT, "typst")
            .prop(prop::CONTENT, text.clone()),
    }
}

fn convert_list_item(item: &[Block]) -> Node {
    Node::new(node::LIST_ITEM).children(convert_blocks(item))
}

fn convert_inline(inline: &Inline) -> Node {
    match inline {
        Inline::Text(t) => Node::new(node::TEXT).prop(prop::CONTENT, t.clone()),
        Inline::Strong(body) => Node::new(node::STRONG).children(convert_inlines(body)),
        Inline::Emph(body) => Node::new(node::EMPHASIS).children(convert_inlines(body)),
        Inline::Underline(body) => Node::new(node::UNDERLINE).children(convert_inlines(body)),
        Inline::Strike(body) => Node::new(node::STRIKEOUT).children(convert_inlines(body)),
        Inline::Subscript(body) => Node::new(node::SUBSCRIPT).children(convert_inlines(body)),
        Inline::Superscript(body) => Node::new(node::SUPERSCRIPT).children(convert_inlines(body)),
        Inline::Code(content) => Node::new(node::CODE).prop(prop::CONTENT, content.clone()),
        Inline::Link { url, body } => Node::new(node::LINK)
            .prop(prop::URL, url.clone())
            .children(convert_inlines(body)),
        Inline::Image { url } => Node::new(node::IMAGE).prop(prop::URL, url.clone()),
        Inline::LineBreak => Node::new(node::LINE_BREAK),
        Inline::MathInline(source) => Node::new("math_inline").prop("math:source", source.clone()),
        Inline::MathDisplay(source) => Node::new("math_block").prop("math:source", source.clone()),
        Inline::Footnote(body) => Node::new(node::FOOTNOTE_DEF).children(convert_inlines(body)),
        Inline::SmallCaps(body) => Node::new(node::SMALL_CAPS).children(convert_inlines(body)),
        Inline::Quoted { double, body } => Node::new(node::QUOTED)
            .prop(prop::QUOTE_TYPE, if *double { "double" } else { "single" })
            .children(convert_inlines(body)),
        Inline::Raw(text) => Node::new(node::RAW_BLOCK)
            .prop(prop::FORMAT, "typst")
            .prop(prop::CONTENT, text.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_str(input: &str) -> Document {
        parse(input).unwrap().value
    }

    #[test]
    fn test_parse_heading() {
        let doc = parse_str("= Title");
        let heading = &doc.content.children[0];
        assert_eq!(heading.kind.as_str(), node::HEADING);
        assert_eq!(heading.props.get_int(prop::LEVEL), Some(1));
    }

    #[test]
    fn test_parse_heading_levels() {
        let doc = parse_str("= Level 1\n\n== Level 2\n\n=== Level 3");
        assert_eq!(doc.content.children[0].props.get_int(prop::LEVEL), Some(1));
        assert_eq!(doc.content.children[1].props.get_int(prop::LEVEL), Some(2));
        assert_eq!(doc.content.children[2].props.get_int(prop::LEVEL), Some(3));
    }

    #[test]
    fn test_parse_paragraph() {
        let doc = parse_str("Hello world!");
        let para = &doc.content.children[0];
        assert_eq!(para.kind.as_str(), node::PARAGRAPH);
    }

    #[test]
    fn test_parse_bold() {
        let doc = parse_str("This is *bold* text.");
        let para = &doc.content.children[0];
        let strong = para
            .children
            .iter()
            .find(|c| c.kind.as_str() == node::STRONG);
        assert!(strong.is_some(), "Expected a strong node in paragraph");
    }

    #[test]
    fn test_parse_italic() {
        let doc = parse_str("This is _italic_ text.");
        let para = &doc.content.children[0];
        let emph = para
            .children
            .iter()
            .find(|c| c.kind.as_str() == node::EMPHASIS);
        assert!(emph.is_some(), "Expected an emphasis node in paragraph");
    }

    #[test]
    fn test_parse_code() {
        let doc = parse_str("Use `code` here.");
        let para = &doc.content.children[0];
        let code = para.children.iter().find(|c| c.kind.as_str() == node::CODE);
        assert!(code.is_some(), "Expected a code node in paragraph");
    }

    #[test]
    fn test_parse_code_block() {
        let doc = parse_str("```rust\nfn main() {}\n```");
        let code = &doc.content.children[0];
        assert_eq!(code.kind.as_str(), node::CODE_BLOCK);
        assert_eq!(code.props.get_str(prop::LANGUAGE), Some("rust"));
    }

    #[test]
    fn test_parse_list() {
        let doc = parse_str("- Item 1\n- Item 2");
        let list = &doc.content.children[0];
        assert_eq!(list.kind.as_str(), node::LIST);
        assert_eq!(list.props.get_bool(prop::ORDERED), Some(false));
        assert_eq!(list.children.len(), 2);
    }

    #[test]
    fn test_parse_ordered_list() {
        let doc = parse_str("+ First\n+ Second");
        let list = &doc.content.children[0];
        assert_eq!(list.kind.as_str(), node::LIST);
        assert_eq!(list.props.get_bool(prop::ORDERED), Some(true));
    }

    #[test]
    fn test_parse_image() {
        let doc = parse_str("#image(\"photo.png\")");
        let img = &doc.content.children[0];
        assert_eq!(img.kind.as_str(), node::IMAGE);
        assert_eq!(img.props.get_str(prop::URL), Some("photo.png"));
    }

    #[test]
    fn test_parse_math_inline() {
        let doc = parse_str("Here is $x^2$ math.");
        let para = &doc.content.children[0];
        let math = para
            .children
            .iter()
            .find(|c| c.kind.as_str() == "math_inline");
        assert!(math.is_some(), "Expected a math_inline node");
    }

    #[test]
    fn test_parse_link() {
        let doc = parse_str("Visit https://typst.app for info.");
        let para = &doc.content.children[0];
        let link = para.children.iter().find(|c| c.kind.as_str() == node::LINK);
        assert!(link.is_some(), "Expected a link node");
    }

    #[test]
    fn test_parse_term_list() {
        let doc = parse_str("/ Rust: A systems language\n/ Python: A scripting language");
        assert_eq!(
            doc.content.children.len(),
            1,
            "Adjacent term items should merge"
        );
        let dl = &doc.content.children[0];
        assert_eq!(dl.kind.as_str(), node::DEFINITION_LIST);
        // Two terms merged: 4 children total (term+desc, term+desc).
        assert_eq!(dl.children.len(), 4);
        assert_eq!(dl.children[0].kind.as_str(), node::DEFINITION_TERM);
        assert_eq!(dl.children[1].kind.as_str(), node::DEFINITION_DESC);
    }

    #[test]
    fn test_parse_footnote() {
        let doc = parse_str("#footnote[A note here]");
        // Footnotes are inline; they end up inside a paragraph.
        let para = &doc.content.children[0];
        assert_eq!(para.kind.as_str(), node::PARAGRAPH);
        let footnote = para
            .children
            .iter()
            .find(|c| c.kind.as_str() == node::FOOTNOTE_DEF);
        let Some(footnote) = footnote else {
            panic!("Expected a footnote_def node in paragraph");
        };
        assert!(!footnote.children.is_empty());
    }

    #[test]
    fn test_parse_sub_super() {
        let doc = parse_str("#sub[2] and #super[3]");
        let para = &doc.content.children[0];
        let sub = para
            .children
            .iter()
            .find(|c| c.kind.as_str() == node::SUBSCRIPT);
        assert!(sub.is_some(), "Expected subscript node");
        let sup = para
            .children
            .iter()
            .find(|c| c.kind.as_str() == node::SUPERSCRIPT);
        assert!(sup.is_some(), "Expected superscript node");
    }

    #[test]
    fn test_parse_underline_strike() {
        let doc = parse_str("#underline[hello] and #strike[world]");
        let para = &doc.content.children[0];
        let u = para
            .children
            .iter()
            .find(|c| c.kind.as_str() == node::UNDERLINE);
        assert!(u.is_some(), "Expected underline node");
        let s = para
            .children
            .iter()
            .find(|c| c.kind.as_str() == node::STRIKEOUT);
        assert!(s.is_some(), "Expected strikeout node");
    }

    #[test]
    fn test_parse_table() {
        let doc = parse_str("#table(columns: 2, [A], [B], [C], [D])");
        let table = &doc.content.children[0];
        assert_eq!(table.kind.as_str(), node::TABLE);
        // 4 cells / 2 columns = 2 rows
        assert_eq!(table.children.len(), 2, "Expected 2 rows");
        assert_eq!(table.children[0].kind.as_str(), node::TABLE_ROW);
        assert_eq!(
            table.children[0].children.len(),
            2,
            "Expected 2 cells per row"
        );
    }

    #[test]
    fn test_parse_figure() {
        let doc = parse_str("#figure(image(\"cat.png\"), caption: [A cat])");
        let figure = &doc.content.children[0];
        assert_eq!(figure.kind.as_str(), node::FIGURE);
        // First child should be an image.
        assert_eq!(figure.children[0].kind.as_str(), node::IMAGE);
        // Second child should be a paragraph (caption).
        assert_eq!(figure.children[1].kind.as_str(), node::PARAGRAPH);
    }
}
