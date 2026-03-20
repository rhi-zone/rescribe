//! Org-mode reader for rescribe.
//!
//! Parses Org-mode source into rescribe's document IR.
//!
//! Delegates all parsing to `org-fmt`, then maps the `OrgDoc` AST into
//! rescribe `Node`/`Document` types.

use org_fmt::{Block, Diagnostic, Inline, ListItem, ListItemContent, OrgDoc, TableRow};
use rescribe_core::{
    ConversionResult, Document, FidelityWarning, ParseError, ParseOptions, Severity, WarningKind,
};
use rescribe_std::{Node, node, prop};

/// Parse Org-mode text into a rescribe Document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse Org-mode with custom options.
pub fn parse_with_options(
    input: &str,
    _options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let (ast, diagnostics) = org_fmt::parse(input);

    let mut warnings: Vec<FidelityWarning> = diagnostics
        .iter()
        .map(|d: &Diagnostic| {
            FidelityWarning::new(
                Severity::Minor,
                WarningKind::UnsupportedNode(d.code.to_string()),
                d.message.clone(),
            )
        })
        .collect();

    let (children, mut more_warnings) = convert_doc(&ast);
    warnings.append(&mut more_warnings);

    let mut metadata = rescribe_core::Properties::new();
    for (key, value) in &ast.metadata {
        metadata.set(key, value.clone());
    }

    let root = Node::new(node::DOCUMENT).children(children);
    let doc = Document::new().with_content(root).with_metadata(metadata);

    Ok(ConversionResult::with_warnings(doc, warnings))

}

fn convert_doc(org_doc: &OrgDoc) -> (Vec<Node>, Vec<FidelityWarning>) {
    let mut warnings = Vec::new();
    let mut nodes = Vec::new();
    for block in &org_doc.blocks {
        match convert_block(block) {
            Ok(Some(n)) => nodes.push(n),
            Ok(None) => {}
            Err(w) => warnings.push(w),
        }
    }
    (nodes, warnings)
}

fn convert_block(block: &Block) -> Result<Option<Node>, FidelityWarning> {
    let node = match block {
        Block::Paragraph { inlines, .. } => {
            Node::new(node::PARAGRAPH).children(convert_inlines(inlines))
        }

        Block::Heading { level, inlines, .. } => Node::new(node::HEADING)
            .prop(prop::LEVEL, *level as i64)
            .children(convert_inlines(inlines)),

        Block::CodeBlock {
            language, content, ..
        } => {
            let mut n = Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone());
            if let Some(lang) = language {
                n = n.prop(prop::LANGUAGE, lang.clone());
            }
            n
        }

        Block::Blockquote { children, .. } => {
            let child_nodes: Vec<Node> = children
                .iter()
                .filter_map(|b| convert_block(b).ok().flatten())
                .collect();
            Node::new(node::BLOCKQUOTE).children(child_nodes)
        }

        Block::List { ordered, items, .. } => {
            let item_nodes: Vec<Node> = items.iter().map(convert_list_item).collect();
            Node::new(node::LIST)
                .prop(prop::ORDERED, *ordered)
                .children(item_nodes)
        }

        Block::Table { rows, .. } => convert_table(rows),

        Block::HorizontalRule { .. } => Node::new(node::HORIZONTAL_RULE),

        Block::DefinitionList { items, .. } => {
            let mut children = Vec::new();
            for item in items {
                children
                    .push(Node::new(node::DEFINITION_TERM).children(convert_inlines(&item.term)));
                children.push(Node::new(node::DEFINITION_DESC).children(vec![
                    Node::new(node::PARAGRAPH).children(convert_inlines(&item.desc)),
                ]));
            }
            Node::new(node::DEFINITION_LIST).children(children)
        }

        Block::Div { inlines, .. } => Node::new(node::DIV).children(convert_inlines(inlines)),

        Block::RawBlock {
            format, content, ..
        } => Node::new(node::RAW_BLOCK)
            .prop(prop::FORMAT, format.clone())
            .prop(prop::CONTENT, content.clone()),

        Block::Figure { children, .. } => {
            let child_nodes: Vec<Node> = children
                .iter()
                .filter_map(|b| convert_block(b).ok().flatten())
                .collect();
            Node::new(node::FIGURE).children(child_nodes)
        }

        Block::Caption { inlines, .. } => {
            Node::new(node::CAPTION).children(convert_inlines(inlines))
        }

        Block::Unknown { kind, .. } => {
            return Err(FidelityWarning::new(
                Severity::Minor,
                WarningKind::UnsupportedNode(kind.clone()),
                format!("Unknown org block: {}", kind),
            ));
        }
    };
    Ok(Some(node))
}

fn convert_list_item(item: &ListItem) -> Node {
    let mut children = Vec::new();
    for content in &item.children {
        match content {
            ListItemContent::Inline(inlines) => {
                children.push(Node::new(node::PARAGRAPH).children(convert_inlines(inlines)));
            }
            ListItemContent::Block(block) => {
                if let Ok(Some(n)) = convert_block(block) {
                    children.push(n);
                }
            }
        }
    }
    // If we ended up with just one paragraph and it's what the parser produced,
    // unwrap it so list items have inline children directly (matching original behavior)
    if children.len() == 1 && children[0].kind.as_str() == node::PARAGRAPH {
        let para = children.remove(0);
        Node::new(node::LIST_ITEM).children(para.children)
    } else {
        Node::new(node::LIST_ITEM).children(children)
    }
}

fn convert_table(rows: &[TableRow]) -> Node {
    // For now, flatten all rows into a simple table with table_row + table_cell children
    let row_nodes: Vec<Node> = rows
        .iter()
        .map(|row| {
            let cell_kind = if row.is_header {
                node::TABLE_HEADER
            } else {
                node::TABLE_CELL
            };
            let cells: Vec<Node> = row
                .cells
                .iter()
                .map(|cell| Node::new(cell_kind).children(convert_inlines(cell)))
                .collect();
            Node::new(node::TABLE_ROW).children(cells)
        })
        .collect();
    Node::new(node::TABLE).children(row_nodes)
}

fn convert_inlines(inlines: &[Inline]) -> Vec<Node> {
    inlines.iter().map(convert_inline).collect()
}

fn convert_inline(inline: &Inline) -> Node {
    match inline {
        Inline::Text { text: s, .. } => Node::new(node::TEXT).prop(prop::CONTENT, s.clone()),

        Inline::Bold(children, _) => {
            Node::new(node::STRONG).children(convert_inlines(children))
        }

        Inline::Italic(children, _) => {
            Node::new(node::EMPHASIS).children(convert_inlines(children))
        }

        Inline::Underline(children, _) => {
            Node::new(node::UNDERLINE).children(convert_inlines(children))
        }

        Inline::Strikethrough(children, _) => {
            Node::new(node::STRIKEOUT).children(convert_inlines(children))
        }

        Inline::Code(s, _) => Node::new(node::CODE).prop(prop::CONTENT, s.clone()),

        Inline::Link { url, children, .. } => Node::new(node::LINK)
            .prop(prop::URL, url.clone())
            .children(convert_inlines(children)),

        Inline::Image { url, .. } => Node::new(node::IMAGE).prop(prop::URL, url.clone()),

        Inline::LineBreak { .. } => Node::new(node::LINE_BREAK),

        Inline::SoftBreak { .. } => Node::new(node::SOFT_BREAK),

        Inline::Superscript(children, _) => {
            Node::new(node::SUPERSCRIPT).children(convert_inlines(children))
        }

        Inline::Subscript(children, _) => {
            Node::new(node::SUBSCRIPT).children(convert_inlines(children))
        }

        Inline::FootnoteRef { label, .. } => {
            Node::new(node::FOOTNOTE_REF).prop(prop::LABEL, label.clone())
        }

        Inline::FootnoteDefinition {
            label, children, ..
        } => Node::new(node::FOOTNOTE_DEF)
            .prop(prop::LABEL, label.clone())
            .children(convert_inlines(children)),

        Inline::MathInline { source, .. } => {
            Node::new("math_inline").prop("math:source", source.clone())
        }
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
        let input = "* Hello World\n** Subheading";
        let result = parse(input).unwrap();
        let doc = result.value;
        let children = root_children(&doc);

        assert_eq!(children.len(), 2);
        assert_eq!(children[0].kind.as_str(), node::HEADING);
        assert_eq!(children[0].props.get_int(prop::LEVEL), Some(1));
        assert_eq!(children[1].props.get_int(prop::LEVEL), Some(2));
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
    fn test_parse_emphasis() {
        let input = "/italic/ and *bold*";
        let result = parse(input).unwrap();
        let doc = result.value;
        let para = &root_children(&doc)[0];

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
    fn test_parse_list() {
        let input = "- First item\n- Second item";
        let result = parse(input).unwrap();
        let doc = result.value;
        let children = root_children(&doc);

        assert!(!children.is_empty());
        let list = &children[0];
        assert_eq!(list.kind.as_str(), node::LIST);
        assert_eq!(list.props.get_bool(prop::ORDERED), Some(false));
        assert_eq!(list.children.len(), 2);
    }

    #[test]
    fn test_parse_code_block() {
        let input = "#+BEGIN_SRC rust\nfn main() {}\n#+END_SRC";
        let result = parse(input).unwrap();
        let doc = result.value;
        let children = root_children(&doc);

        assert!(!children.is_empty());
        let code = &children[0];
        assert_eq!(code.kind.as_str(), node::CODE_BLOCK);
        assert_eq!(code.props.get_str(prop::LANGUAGE), Some("rust"));
    }

    #[test]
    fn test_parse_metadata() {
        let input = "#+TITLE: My Document\n#+AUTHOR: Jane Doe\n\nContent here.";
        let result = parse(input).unwrap();
        let doc = result.value;

        assert_eq!(doc.metadata.get_str("title"), Some("My Document"));
        assert_eq!(doc.metadata.get_str("author"), Some("Jane Doe"));
    }
}
