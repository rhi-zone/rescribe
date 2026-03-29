//! AsciiDoc writer for rescribe.
//!
//! Thin adapter over [`asciidoc`]: maps the rescribe document model to
//! the `asciidoc` AST, then builds AsciiDoc output.

use asciidoc::{AsciiDoc, Block, DefinitionItem, ImageData, Inline, QuoteType, TableRow};
use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node};
use rescribe_std::{node, prop};

/// Emit a document as AsciiDoc.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    emit_with_options(doc, &EmitOptions::default())
}

/// Emit a document as AsciiDoc with custom options.
pub fn emit_with_options(
    doc: &Document,
    _options: &EmitOptions,
) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let ast = doc_to_ast(doc);
    let output = asciidoc::build(&ast);
    Ok(ConversionResult::ok(output.into_bytes()))
}

fn doc_to_ast(doc: &Document) -> AsciiDoc {
    AsciiDoc {
        blocks: nodes_to_blocks(&doc.content.children),
        attributes: Default::default(),
        span: asciidoc::Span::NONE,
    }
}

fn nodes_to_blocks(nodes: &[Node]) -> Vec<Block> {
    nodes.iter().flat_map(node_to_blocks).collect()
}

fn node_to_blocks(node: &Node) -> Vec<Block> {
    match node.kind.as_str() {
        node::DOCUMENT => nodes_to_blocks(&node.children),

        node::PARAGRAPH => vec![Block::Paragraph {
            inlines: nodes_to_inlines(&node.children),
            id: None,
            role: None,
            checked: None,
            span: asciidoc::Span::NONE,
        }],

        node::HEADING => {
            let level = node.props.get_int(prop::LEVEL).unwrap_or(1) as usize;
            vec![Block::Heading {
                level,
                inlines: nodes_to_inlines(&node.children),
                id: None,
                role: None,
                span: asciidoc::Span::NONE,
            }]
        }

        node::CODE_BLOCK => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            let language = node.props.get_str(prop::LANGUAGE).map(|s| s.to_string());
            vec![Block::CodeBlock {
                content,
                language,
                span: asciidoc::Span::NONE,
            }]
        }

        node::BLOCKQUOTE => vec![Block::Blockquote {
            children: nodes_to_blocks(&node.children),
            attribution: None,
            span: asciidoc::Span::NONE,
        }],

        node::LIST => {
            let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
            let items: Vec<Vec<Block>> = node
                .children
                .iter()
                .filter(|c| c.kind.as_str() == node::LIST_ITEM)
                .map(|item| nodes_to_blocks(&item.children))
                .collect();
            vec![Block::List {
                ordered,
                items,
                style: None,
                span: asciidoc::Span::NONE,
            }]
        }

        node::LIST_ITEM => nodes_to_blocks(&node.children),

        node::DEFINITION_LIST => {
            // Pair up DEFINITION_TERM and DEFINITION_DESC children
            let mut items = Vec::new();
            let mut i = 0;
            while i < node.children.len() {
                let child = &node.children[i];
                if child.kind.as_str() == node::DEFINITION_TERM {
                    let term = nodes_to_inlines(&child.children);
                    let desc = if i + 1 < node.children.len()
                        && node.children[i + 1].kind.as_str() == node::DEFINITION_DESC
                    {
                        i += 1;
                        nodes_to_inlines(&node.children[i].children)
                    } else {
                        Vec::new()
                    };
                    items.push(DefinitionItem { term, desc });
                }
                i += 1;
            }
            vec![Block::DefinitionList {
                items,
                span: asciidoc::Span::NONE,
            }]
        }

        node::DEFINITION_TERM | node::DEFINITION_DESC => {
            // These are handled inside DEFINITION_LIST; skip if encountered alone
            vec![]
        }

        node::TABLE => {
            let rows: Vec<TableRow> = node.children.iter().flat_map(collect_table_rows).collect();
            vec![Block::Table {
                rows,
                span: asciidoc::Span::NONE,
            }]
        }

        node::FIGURE => {
            // Look for an IMAGE child
            for child in &node.children {
                if child.kind.as_str() == node::IMAGE {
                    return vec![Block::Figure {
                        image: node_to_image_data(child),
                        span: asciidoc::Span::NONE,
                    }];
                }
            }
            nodes_to_blocks(&node.children)
        }

        node::HORIZONTAL_RULE => vec![Block::HorizontalRule {
            span: asciidoc::Span::NONE,
        }],

        node::DIV | node::SPAN => nodes_to_blocks(&node.children),

        node::RAW_BLOCK | node::RAW_INLINE => {
            let format = node
                .props
                .get_str(prop::FORMAT)
                .unwrap_or("asciidoc")
                .to_string();
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            if format == "asciidoc" {
                vec![Block::RawBlock {
                    format,
                    content,
                    span: asciidoc::Span::NONE,
                }]
            } else {
                vec![]
            }
        }

        "math_block" | "math_display" => {
            if let Some(source) = node.props.get_str("math:source") {
                let flavor = node.props.get_str("math:flavor").map(|s| s.to_string());
                vec![Block::MathBlock {
                    content: source.to_string(),
                    flavor,
                    span: asciidoc::Span::NONE,
                }]
            } else {
                vec![]
            }
        }

        "admonition" => {
            let adm_type = node
                .props
                .get_str("admonition_type")
                .unwrap_or("NOTE")
                .to_uppercase();
            vec![Block::Div {
                class: Some(format!("admonition {}", adm_type.to_lowercase())),
                title: None,
                children: nodes_to_blocks(&node.children),
                span: asciidoc::Span::NONE,
            }]
        }

        // Inline nodes at block level: wrap in a paragraph
        node::TEXT
        | node::STRONG
        | node::EMPHASIS
        | node::CODE
        | node::LINK
        | node::STRIKEOUT
        | node::UNDERLINE
        | node::SUPERSCRIPT
        | node::SUBSCRIPT => {
            vec![Block::Paragraph {
                inlines: nodes_to_inlines(std::slice::from_ref(node)),
                id: None,
                role: None,
                checked: None,
                span: asciidoc::Span::NONE,
            }]
        }

        _ => nodes_to_blocks(&node.children),
    }
}

fn collect_table_rows(node: &Node) -> Vec<TableRow> {
    match node.kind.as_str() {
        node::TABLE_HEAD | node::TABLE_BODY | node::TABLE_FOOT => {
            node.children.iter().flat_map(collect_table_rows).collect()
        }
        node::TABLE_ROW => {
            let cells: Vec<Vec<Inline>> = node
                .children
                .iter()
                .map(|cell| nodes_to_inlines(&cell.children))
                .collect();
            vec![TableRow {
                cells,
                is_header: false,
            }]
        }
        node::TABLE_HEADER => {
            let cells: Vec<Vec<Inline>> = node
                .children
                .iter()
                .map(|cell| nodes_to_inlines(&cell.children))
                .collect();
            vec![TableRow {
                cells,
                is_header: true,
            }]
        }
        _ => vec![],
    }
}

fn node_to_image_data(node: &Node) -> ImageData {
    ImageData {
        url: node.props.get_str(prop::URL).unwrap_or("").to_string(),
        alt: node.props.get_str(prop::ALT).map(|s| s.to_string()),
        width: node.props.get_str("width").map(|s| s.to_string()),
        height: node.props.get_str("height").map(|s| s.to_string()),
    }
}

fn nodes_to_inlines(nodes: &[Node]) -> Vec<Inline> {
    nodes.iter().map(node_to_inline).collect()
}

fn node_to_inline(node: &Node) -> Inline {
    match node.kind.as_str() {
        node::TEXT => {
            let s = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            Inline::Text {
                text: s,
                span: asciidoc::Span::NONE,
            }
        }

        node::EMPHASIS => Inline::Emphasis(nodes_to_inlines(&node.children), asciidoc::Span::NONE),

        node::STRONG => Inline::Strong(nodes_to_inlines(&node.children), asciidoc::Span::NONE),

        node::STRIKEOUT => {
            Inline::Strikeout(nodes_to_inlines(&node.children), asciidoc::Span::NONE)
        }

        node::UNDERLINE => {
            Inline::Underline(nodes_to_inlines(&node.children), asciidoc::Span::NONE)
        }

        node::SUBSCRIPT => {
            Inline::Subscript(nodes_to_inlines(&node.children), asciidoc::Span::NONE)
        }

        node::SUPERSCRIPT => {
            Inline::Superscript(nodes_to_inlines(&node.children), asciidoc::Span::NONE)
        }

        node::CODE => {
            let s = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            Inline::Code(s, asciidoc::Span::NONE)
        }

        node::LINK => {
            let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
            Inline::Link {
                url,
                children: nodes_to_inlines(&node.children),
                target: None,
                span: asciidoc::Span::NONE,
            }
        }

        node::IMAGE => Inline::Image(node_to_image_data(node), asciidoc::Span::NONE),

        node::LINE_BREAK => Inline::LineBreak {
            span: asciidoc::Span::NONE,
        },

        node::SOFT_BREAK => Inline::SoftBreak {
            span: asciidoc::Span::NONE,
        },

        node::FOOTNOTE_REF => {
            let label = node.props.get_str(prop::LABEL).unwrap_or("").to_string();
            Inline::FootnoteRef {
                label,
                span: asciidoc::Span::NONE,
            }
        }

        node::FOOTNOTE_DEF => {
            let label = node.props.get_str(prop::LABEL).unwrap_or("").to_string();
            Inline::FootnoteDef {
                label,
                children: nodes_to_inlines(&node.children),
                span: asciidoc::Span::NONE,
            }
        }

        node::SMALL_CAPS => {
            Inline::SmallCaps(nodes_to_inlines(&node.children), asciidoc::Span::NONE)
        }

        node::QUOTED => {
            let qt = match node.props.get_str(prop::QUOTE_TYPE).unwrap_or("double") {
                "single" => QuoteType::Single,
                _ => QuoteType::Double,
            };
            Inline::Quoted {
                quote_type: qt,
                children: nodes_to_inlines(&node.children),
                span: asciidoc::Span::NONE,
            }
        }

        node::SPAN => {
            // Passthrough span: just recurse
            let children = nodes_to_inlines(&node.children);
            if children.len() == 1 {
                children.into_iter().next().unwrap()
            } else {
                // Wrap in a highlight to group
                Inline::Highlight(children, asciidoc::Span::NONE)
            }
        }

        node::RAW_INLINE => {
            let format = node
                .props
                .get_str(prop::FORMAT)
                .unwrap_or("asciidoc")
                .to_string();
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            Inline::RawInline {
                format,
                content,
                span: asciidoc::Span::NONE,
            }
        }

        "math_inline" => {
            let content = node.props.get_str("math:source").unwrap_or("").to_string();
            let flavor = node.props.get_str("math:flavor").map(|s| s.to_string());
            Inline::MathInline {
                content,
                flavor,
                span: asciidoc::Span::NONE,
            }
        }

        _ => {
            // Unknown inline: recurse into children, or emit empty text
            let children = nodes_to_inlines(&node.children);
            if children.is_empty() {
                Inline::Text {
                    text: String::new(),
                    span: asciidoc::Span::NONE,
                }
            } else if children.len() == 1 {
                children.into_iter().next().unwrap()
            } else {
                Inline::Highlight(children, asciidoc::Span::NONE)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rescribe_std::builder::*;

    fn emit_str(doc: &Document) -> String {
        let result = emit(doc).unwrap();
        String::from_utf8(result.value).unwrap()
    }

    #[test]
    fn test_emit_paragraph() {
        let doc = doc(|d| d.para(|p| p.text("Hello, world!")));
        let output = emit_str(&doc);
        assert!(output.contains("Hello, world!"));
    }

    #[test]
    fn test_emit_heading() {
        let doc = doc(|d| d.heading(1, |h| h.text("Title")));
        let output = emit_str(&doc);
        assert!(output.contains("== Title"));
    }

    #[test]
    fn test_emit_heading_level2() {
        let doc = doc(|d| d.heading(2, |h| h.text("Subtitle")));
        let output = emit_str(&doc);
        assert!(output.contains("=== Subtitle"));
    }

    #[test]
    fn test_emit_emphasis() {
        let doc = doc(|d| d.para(|p| p.em(|e| e.text("italic"))));
        let output = emit_str(&doc);
        assert!(output.contains("_italic_"));
    }

    #[test]
    fn test_emit_strong() {
        let doc = doc(|d| d.para(|p| p.strong(|s| s.text("bold"))));
        let output = emit_str(&doc);
        assert!(output.contains("*bold*"));
    }

    #[test]
    fn test_emit_code() {
        let doc = doc(|d| d.para(|p| p.code("code")));
        let output = emit_str(&doc);
        assert!(output.contains("`code`"));
    }

    #[test]
    fn test_emit_link() {
        let doc = doc(|d| d.para(|p| p.link("https://example.com", |l| l.text("click"))));
        let output = emit_str(&doc);
        assert!(output.contains("https://example.com[click]"));
    }

    #[test]
    fn test_emit_code_block() {
        let doc = doc(|d| d.code_block_lang("print('hi')", "python"));
        let output = emit_str(&doc);
        assert!(output.contains("[source,python]"));
        assert!(output.contains("----"));
        assert!(output.contains("print('hi')"));
    }

    #[test]
    fn test_emit_list() {
        let doc = doc(|d| d.bullet_list(|l| l.item(|i| i.text("one")).item(|i| i.text("two"))));
        let output = emit_str(&doc);
        assert!(output.contains("* one"));
        assert!(output.contains("* two"));
    }
}
