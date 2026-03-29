//! AsciiDoc reader for rescribe.
//!
//! Thin adapter over [`asciidoc`]: parses AsciiDoc into the `asciidoc` AST,
//! then maps it to the rescribe document model.

use asciidoc::{AsciiDoc, Block, DefinitionItem, ImageData, Inline, QuoteType, TableRow};
use rescribe_core::{ConversionResult, Document, ParseOptions};
use rescribe_std::{Node, node, prop};

/// Parse AsciiDoc text into a rescribe Document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, rescribe_core::ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse AsciiDoc with custom options.
pub fn parse_with_options(
    input: &str,
    _options: &ParseOptions,
) -> Result<ConversionResult<Document>, rescribe_core::ParseError> {
    let (ast, _diagnostics) = asciidoc::parse(input);
    let children = doc_to_nodes(&ast);
    let root = Node::new(node::DOCUMENT).children(children);
    let doc = Document::new().with_content(root);
    Ok(ConversionResult::ok(doc))
}

fn doc_to_nodes(ast: &AsciiDoc) -> Vec<Node> {
    ast.blocks.iter().map(block_to_node).collect()
}

fn block_to_node(block: &Block) -> Node {
    match block {
        Block::Paragraph { inlines, id, role, .. } => {
            let mut n = Node::new(node::PARAGRAPH).children(inlines_to_nodes(inlines));
            if let Some(id) = id {
                n = n.prop("id", id.clone());
            }
            if let Some(role) = role {
                n = n.prop("role", role.clone());
            }
            n
        }

        Block::Heading {
            level,
            inlines,
            id,
            role,
            ..
        } => {
            let mut n = Node::new(node::HEADING)
                .prop(prop::LEVEL, *level as i64)
                .children(inlines_to_nodes(inlines));
            if let Some(id) = id {
                n = n.prop("id", id.clone());
            }
            if let Some(role) = role {
                n = n.prop("role", role.clone());
            }
            n
        }

        Block::CodeBlock {
            content, language, ..
        } => {
            let mut n = Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone());
            if let Some(lang) = language {
                n = n.prop(prop::LANGUAGE, lang.clone());
            }
            n
        }

        Block::Blockquote {
            children,
            attribution,
            ..
        } => {
            let mut n = Node::new(node::BLOCKQUOTE).children(children.iter().map(block_to_node));
            if let Some(attr) = attribution {
                n = n.prop("attribution", attr.clone());
            }
            n
        }

        Block::List {
            ordered,
            items,
            style,
            ..
        } => {
            let list_items: Vec<Node> = items
                .iter()
                .map(|item_blocks| {
                    let mut li = Node::new(node::LIST_ITEM)
                        .children(item_blocks.iter().map(block_to_node));
                    // Propagate checklist state from first paragraph to the list_item
                    if let Some(Block::Paragraph { checked: Some(c), .. }) = item_blocks.first() {
                        li = li.prop("asciidoc:checked", *c);
                    }
                    li
                })
                .collect();
            let mut n = Node::new(node::LIST)
                .prop(prop::ORDERED, *ordered)
                .children(list_items);
            if let Some(s) = style {
                n = n.prop("list:style", s.clone());
            }
            n
        }

        Block::DefinitionList { items, .. } => {
            let children: Vec<Node> = items.iter().flat_map(definition_item_to_nodes).collect();
            Node::new(node::DEFINITION_LIST).children(children)
        }

        Block::HorizontalRule { .. } => Node::new(node::HORIZONTAL_RULE),

        Block::PageBreak { .. } => Node::new(node::DIV).prop("class", "page-break".to_string()),

        Block::Figure { image, .. } => {
            let img = image_data_to_node(image);
            Node::new(node::FIGURE).children(vec![img])
        }

        Block::Div { class, title, children, .. } => {
            let mut n = Node::new(node::DIV).children(children.iter().map(block_to_node));
            if let Some(cls) = class {
                n = n.prop("class", cls.clone());
            }
            if let Some(t) = title {
                n = n.prop("title", t.clone());
            }
            n
        }

        Block::RawBlock { format, content, .. } => Node::new(node::RAW_BLOCK)
            .prop(prop::CONTENT, content.clone())
            .prop("format", format.clone()),

        Block::MathBlock { content, flavor, .. } => {
            let mut n = Node::new("math_block").prop("math:source", content.clone());
            if let Some(f) = flavor {
                n = n.prop("math:flavor", f.clone());
            }
            n
        }

        Block::Table { rows, .. } => {
            let row_nodes: Vec<Node> = rows.iter().map(table_row_to_node).collect();
            Node::new(node::TABLE).children(row_nodes)
        }
    }
}

fn definition_item_to_nodes(item: &DefinitionItem) -> Vec<Node> {
    vec![
        Node::new(node::DEFINITION_TERM).children(inlines_to_nodes(&item.term)),
        Node::new(node::DEFINITION_DESC).children(inlines_to_nodes(&item.desc)),
    ]
}

fn table_row_to_node(row: &TableRow) -> Node {
    let cells: Vec<Node> = row
        .cells
        .iter()
        .map(|cell| Node::new(node::TABLE_CELL).children(inlines_to_nodes(cell)))
        .collect();
    if row.is_header {
        Node::new(node::TABLE_HEADER).children(cells)
    } else {
        Node::new(node::TABLE_ROW).children(cells)
    }
}

fn image_data_to_node(img: &ImageData) -> Node {
    let mut n = Node::new(node::IMAGE).prop(prop::URL, img.url.clone());
    if let Some(alt) = &img.alt {
        n = n.prop(prop::ALT, alt.clone());
    }
    if let Some(w) = &img.width {
        n = n.prop("width", w.clone());
    }
    if let Some(h) = &img.height {
        n = n.prop("height", h.clone());
    }
    n
}

fn inlines_to_nodes(inlines: &[Inline]) -> Vec<Node> {
    inlines.iter().map(inline_to_node).collect()
}

fn inline_to_node(inline: &Inline) -> Node {
    match inline {
        Inline::Text { text: s, .. } => Node::new(node::TEXT).prop(prop::CONTENT, s.clone()),

        Inline::Strong(children, _) => {
            Node::new(node::STRONG).children(inlines_to_nodes(children))
        }

        Inline::Emphasis(children, _) => {
            Node::new(node::EMPHASIS).children(inlines_to_nodes(children))
        }

        Inline::Code(s, _) => Node::new(node::CODE).prop(prop::CONTENT, s.clone()),

        Inline::Superscript(children, _) => {
            Node::new(node::SUPERSCRIPT).children(inlines_to_nodes(children))
        }

        Inline::Subscript(children, _) => {
            Node::new(node::SUBSCRIPT).children(inlines_to_nodes(children))
        }

        Inline::Highlight(children, _) => Node::new(node::SPAN)
            .prop("class", "highlight".to_string())
            .children(inlines_to_nodes(children)),

        Inline::Strikeout(children, _) => {
            Node::new(node::STRIKEOUT).children(inlines_to_nodes(children))
        }

        Inline::Underline(children, _) => {
            Node::new(node::UNDERLINE).children(inlines_to_nodes(children))
        }

        Inline::SmallCaps(children, _) => {
            Node::new(node::SMALL_CAPS).children(inlines_to_nodes(children))
        }

        Inline::Quoted {
            quote_type,
            children,
            ..
        } => {
            let qt = match quote_type {
                QuoteType::Single => "single",
                QuoteType::Double => "double",
            };
            Node::new(node::QUOTED)
                .prop(prop::QUOTE_TYPE, qt.to_string())
                .children(inlines_to_nodes(children))
        }

        Inline::Link {
            url, children, target, ..
        } => {
            let mut n = Node::new(node::LINK)
                .prop(prop::URL, url.clone())
                .children(inlines_to_nodes(children));
            if let Some(t) = target {
                n = n.prop("target", t.clone());
            }
            n
        }

        Inline::Image(img, _) => image_data_to_node(img),

        Inline::LineBreak { .. } => Node::new(node::LINE_BREAK),

        Inline::SoftBreak { .. } => Node::new(node::SOFT_BREAK),

        Inline::FootnoteRef { label, .. } => {
            Node::new(node::FOOTNOTE_REF).prop(prop::LABEL, label.clone())
        }

        Inline::FootnoteDef {
            label, children, ..
        } => Node::new(node::FOOTNOTE_DEF)
            .prop(prop::LABEL, label.clone())
            .children(inlines_to_nodes(children)),

        Inline::MathInline { content, flavor, .. } => {
            let mut n = Node::new("math_inline").prop("math:source", content.clone());
            if let Some(f) = flavor {
                n = n.prop("math:flavor", f.clone());
            }
            n
        }

        Inline::RawInline {
            format, content, ..
        } => Node::new(node::RAW_INLINE)
            .prop("format", format.clone())
            .prop(prop::CONTENT, content.clone()),

        Inline::Anchor { id, .. } => Node::new(node::SPAN).prop("id", id.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root_children(doc: &Document) -> &[Node] {
        &doc.content.children
    }

    #[test]
    fn test_parse_heading() {
        let input = "== Hello World\n\nSome text.";
        let result = parse(input).unwrap();
        let doc = result.value;
        let children = root_children(&doc);

        assert_eq!(children.len(), 2);
        assert_eq!(children[0].kind.as_str(), node::HEADING);
        assert_eq!(children[0].props.get_int(prop::LEVEL), Some(2));
    }

    #[test]
    fn test_parse_paragraph() {
        let input = "This is a paragraph.\n\nThis is another.";
        let result = parse(input).unwrap();
        let doc = result.value;
        let children = root_children(&doc);

        assert_eq!(children.len(), 2);
        assert_eq!(children[0].kind.as_str(), node::PARAGRAPH);
        assert_eq!(children[1].kind.as_str(), node::PARAGRAPH);
    }

    #[test]
    fn test_parse_strong() {
        let input = "This is *strong* text.";
        let result = parse(input).unwrap();
        let doc = result.value;
        let para = &root_children(&doc)[0];

        assert!(
            para.children
                .iter()
                .any(|n| n.kind.as_str() == node::STRONG)
        );
    }

    #[test]
    fn test_parse_emphasis() {
        let input = "This is _emphasized_ text.";
        let result = parse(input).unwrap();
        let doc = result.value;
        let para = &root_children(&doc)[0];

        assert!(
            para.children
                .iter()
                .any(|n| n.kind.as_str() == node::EMPHASIS)
        );
    }

    #[test]
    fn test_parse_bullet_list() {
        let input = "* First item\n* Second item\n* Third item";
        let result = parse(input).unwrap();
        let doc = result.value;
        let children = root_children(&doc);

        assert_eq!(children.len(), 1);
        assert_eq!(children[0].kind.as_str(), node::LIST);
        assert_eq!(children[0].props.get_bool(prop::ORDERED), Some(false));
        assert_eq!(children[0].children.len(), 3);
    }

    #[test]
    fn test_parse_numbered_list() {
        let input = ". First item\n. Second item\n. Third item";
        let result = parse(input).unwrap();
        let doc = result.value;
        let children = root_children(&doc);

        assert_eq!(children.len(), 1);
        assert_eq!(children[0].kind.as_str(), node::LIST);
        assert_eq!(children[0].props.get_bool(prop::ORDERED), Some(true));
        assert_eq!(children[0].children.len(), 3);
    }

    #[test]
    fn test_parse_code_block() {
        let input = "[source,python]\n----\nprint('hello')\n----";
        let result = parse(input).unwrap();
        let doc = result.value;
        let children = root_children(&doc);

        assert_eq!(children.len(), 1);
        assert_eq!(children[0].kind.as_str(), node::CODE_BLOCK);
        assert_eq!(children[0].props.get_str(prop::LANGUAGE), Some("python"));
    }

    #[test]
    fn test_parse_inline_code() {
        let input = "Use `code here` in text.";
        let result = parse(input).unwrap();
        let doc = result.value;
        let para = &root_children(&doc)[0];

        assert!(para.children.iter().any(|n| n.kind.as_str() == node::CODE));
    }

    #[test]
    fn test_parse_link() {
        let input = "Visit https://example.com[Example Site] for more.";
        let result = parse(input).unwrap();
        let doc = result.value;
        let para = &root_children(&doc)[0];

        let link = para.children.iter().find(|n| n.kind.as_str() == node::LINK);
        assert!(link.is_some());
        assert_eq!(
            link.unwrap().props.get_str(prop::URL),
            Some("https://example.com")
        );
    }

    #[test]
    fn test_parse_block_image() {
        let input = "image::path/to/image.png[Alt text]";
        let result = parse(input).unwrap();
        let doc = result.value;
        let children = root_children(&doc);

        assert_eq!(children.len(), 1);
        assert_eq!(children[0].kind.as_str(), node::FIGURE);

        let img = &children[0].children[0];
        assert_eq!(img.kind.as_str(), node::IMAGE);
        assert_eq!(img.props.get_str(prop::URL), Some("path/to/image.png"));
    }
}
