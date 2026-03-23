//! Djot reader for rescribe.
//!
//! Parses Djot markup into rescribe's document IR using the `djot-fmt` crate.
//!
//! # Example
//!
//! ```
//! use rescribe_read_djot::parse;
//!
//! let result = parse("# Hello\n\nWorld!").unwrap();
//! let doc = result.value;
//! ```

use djot_fmt::{
    Alignment, Block, DjotDoc, Inline, ListKind,
};
use rescribe_core::{ConversionResult, Document, FidelityWarning, Node, ParseError, Properties};
use rescribe_std::{node, prop};

/// Parse Djot text into a document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    let (djot_doc, _diagnostics) = djot_fmt::parse(input);
    let mut converter = Converter::new();
    let children = converter.convert_doc(&djot_doc);

    let document = Document {
        content: Node::new(node::DOCUMENT).children(children),
        resources: Default::default(),
        metadata: Properties::new(),
        source: None,
    };

    Ok(ConversionResult::with_warnings(document, converter.warnings))
}

struct Converter {
    warnings: Vec<FidelityWarning>,
}

impl Converter {
    fn new() -> Self {
        Self { warnings: Vec::new() }
    }

    fn convert_doc(&mut self, doc: &DjotDoc) -> Vec<Node> {
        let mut nodes = self.convert_blocks(&doc.blocks);
        // Footnote definitions
        for fn_def in &doc.footnotes {
            let children = self.convert_blocks(&fn_def.blocks);
            let node = Node::new(node::FOOTNOTE_DEF)
                .prop(prop::LABEL, fn_def.label.clone())
                .children(children);
            nodes.push(node);
        }
        nodes
    }

    fn convert_blocks(&mut self, blocks: &[Block]) -> Vec<Node> {
        blocks.iter().map(|b| self.convert_block(b)).collect()
    }

    fn convert_block(&mut self, block: &Block) -> Node {
        match block {
            Block::Paragraph { inlines, .. } => {
                Node::new(node::PARAGRAPH).children(self.convert_inlines(inlines))
            }
            Block::Heading { level, inlines, .. } => Node::new(node::HEADING)
                .prop(prop::LEVEL, *level as i64)
                .children(self.convert_inlines(inlines)),
            Block::Blockquote { blocks, .. } => {
                Node::new(node::BLOCKQUOTE).children(self.convert_blocks(blocks))
            }
            Block::List { kind, items, tight, .. } => {
                let (ordered, start) = match kind {
                    ListKind::Bullet(_) | ListKind::Task => (false, 1i64),
                    ListKind::Ordered { start, .. } => (true, *start as i64),
                };
                let mut list = Node::new(node::LIST)
                    .prop(prop::ORDERED, ordered)
                    .prop("tight", *tight);
                if ordered && start != 1 {
                    list = list.prop(prop::START, start);
                }
                let item_nodes: Vec<Node> = items
                    .iter()
                    .map(|item| {
                        let mut li = Node::new(node::LIST_ITEM)
                            .children(self.convert_blocks(&item.blocks));
                        if let Some(checked) = item.checked {
                            li = li.prop(prop::CHECKED, checked);
                        }
                        li
                    })
                    .collect();
                list.children(item_nodes)
            }
            Block::CodeBlock { language, content, .. } => {
                let mut cb = Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone());
                if let Some(lang) = language {
                    cb = cb.prop(prop::LANGUAGE, lang.clone());
                }
                cb
            }
            Block::RawBlock { format, content, .. } => Node::new(node::RAW_BLOCK)
                .prop(prop::FORMAT, format.clone())
                .prop(prop::CONTENT, content.clone()),
            Block::Div { class, blocks, .. } => {
                let mut div = Node::new(node::DIV).children(self.convert_blocks(blocks));
                if let Some(c) = class {
                    div = div.prop("html:class", c.clone());
                }
                div
            }
            Block::Table { caption, rows, .. } => {
                let mut table_nodes = Vec::new();
                if let Some(cap_inlines) = caption {
                    // Caption as a paragraph with a caption marker
                    let cap = Node::new(node::PARAGRAPH)
                        .prop("role", "caption")
                        .children(self.convert_inlines(cap_inlines));
                    table_nodes.push(cap);
                }
                for row in rows {
                    let cell_nodes: Vec<Node> = row
                        .cells
                        .iter()
                        .map(|cell| {
                            let kind = if row.is_header {
                                node::TABLE_HEADER
                            } else {
                                node::TABLE_CELL
                            };
                            let mut cn = Node::new(kind)
                                .children(self.convert_inlines(&cell.inlines));
                            let align_str = match cell.alignment {
                                Alignment::Left => "left",
                                Alignment::Right => "right",
                                Alignment::Center => "center",
                                Alignment::Default => "",
                            };
                            if !align_str.is_empty() {
                                cn = cn.prop("style:align", align_str);
                            }
                            cn
                        })
                        .collect();
                    table_nodes.push(Node::new(node::TABLE_ROW).children(cell_nodes));
                }
                Node::new(node::TABLE).children(table_nodes)
            }
            Block::ThematicBreak { .. } => Node::new(node::HORIZONTAL_RULE),
            Block::DefinitionList { items, .. } => {
                let mut dl_children = Vec::new();
                for item in items {
                    dl_children.push(
                        Node::new(node::DEFINITION_TERM)
                            .children(self.convert_inlines(&item.term)),
                    );
                    for def_block in &item.definitions {
                        dl_children.push(
                            Node::new(node::DEFINITION_DESC)
                                .children(vec![self.convert_block(def_block)]),
                        );
                    }
                }
                Node::new(node::DEFINITION_LIST).children(dl_children)
            }
        }
    }

    fn convert_inlines(&mut self, inlines: &[Inline]) -> Vec<Node> {
        inlines.iter().map(|i| self.convert_inline(i)).collect()
    }

    fn convert_inline(&mut self, inline: &Inline) -> Node {
        match inline {
            Inline::Text { content, .. } => {
                Node::new(node::TEXT).prop(prop::CONTENT, content.clone())
            }
            Inline::SoftBreak { .. } => Node::new(node::SOFT_BREAK),
            Inline::HardBreak { .. } => Node::new(node::LINE_BREAK),
            Inline::Emphasis { inlines, .. } => {
                Node::new(node::EMPHASIS).children(self.convert_inlines(inlines))
            }
            Inline::Strong { inlines, .. } => {
                Node::new(node::STRONG).children(self.convert_inlines(inlines))
            }
            Inline::Delete { inlines, .. } => {
                Node::new(node::STRIKEOUT).children(self.convert_inlines(inlines))
            }
            Inline::Insert { inlines, .. } => {
                Node::new(node::UNDERLINE).children(self.convert_inlines(inlines))
            }
            Inline::Highlight { inlines, .. } => Node::new(node::SPAN)
                .prop("html:class", "mark")
                .children(self.convert_inlines(inlines)),
            Inline::Subscript { inlines, .. } => {
                Node::new(node::SUBSCRIPT).children(self.convert_inlines(inlines))
            }
            Inline::Superscript { inlines, .. } => {
                Node::new(node::SUPERSCRIPT).children(self.convert_inlines(inlines))
            }
            Inline::Verbatim { content, .. } => {
                Node::new(node::CODE).prop(prop::CONTENT, content.clone())
            }
            Inline::MathInline { content, .. } => {
                Node::new("math:inline").prop(prop::CONTENT, content.clone())
            }
            Inline::MathDisplay { content, .. } => {
                Node::new("math:display").prop(prop::CONTENT, content.clone())
            }
            Inline::RawInline { format, content, .. } => Node::new(node::RAW_INLINE)
                .prop(prop::FORMAT, format.clone())
                .prop(prop::CONTENT, content.clone()),
            Inline::Link { inlines, url, title, .. } => {
                let mut link = Node::new(node::LINK)
                    .prop(prop::URL, url.clone())
                    .children(self.convert_inlines(inlines));
                if let Some(t) = title {
                    link = link.prop(prop::TITLE, t.clone());
                }
                link
            }
            Inline::Image { inlines, url, title, .. } => {
                let alt = collect_text(inlines);
                let mut img = Node::new(node::IMAGE).prop(prop::URL, url.clone());
                if !alt.is_empty() {
                    img = img.prop(prop::ALT, alt);
                }
                if let Some(t) = title {
                    img = img.prop(prop::TITLE, t.clone());
                }
                img
            }
            Inline::Span { inlines, attr, .. } => {
                let mut span = Node::new(node::SPAN).children(self.convert_inlines(inlines));
                if let Some(id) = &attr.id {
                    span = span.prop("html:id", id.clone());
                }
                if !attr.classes.is_empty() {
                    span = span.prop("html:class", attr.classes.join(" "));
                }
                span
            }
            Inline::FootnoteRef { label, .. } => {
                Node::new(node::FOOTNOTE_REF).prop(prop::LABEL, label.clone())
            }
            Inline::Symbol { name, .. } => {
                Node::new(node::TEXT).prop(prop::CONTENT, format!(":{name}:"))
            }
            Inline::Autolink { url, .. } => {
                let text = Node::new(node::TEXT).prop(prop::CONTENT, url.clone());
                Node::new(node::LINK)
                    .prop(prop::URL, url.clone())
                    .children(vec![text])
            }
        }
    }
}

fn collect_text(inlines: &[Inline]) -> String {
    inlines
        .iter()
        .map(|i| match i {
            Inline::Text { content, .. } => content.as_str(),
            _ => "",
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_paragraph() {
        let result = parse("Hello, world!").unwrap();
        let doc = result.value;
        assert_eq!(doc.content.children.len(), 1);
        assert_eq!(doc.content.children[0].kind.as_str(), node::PARAGRAPH);
    }

    #[test]
    fn test_parse_heading() {
        let result = parse("# Heading 1\n\n## Heading 2").unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_emphasis() {
        let result = parse("_emphasis_ and *strong*").unwrap();
        let doc = result.value;
        assert_eq!(doc.content.children.len(), 1);
        let para = &doc.content.children[0];
        let has_emphasis = para.children.iter().any(|n| n.kind.as_str() == node::EMPHASIS);
        let has_strong = para.children.iter().any(|n| n.kind.as_str() == node::STRONG);
        assert!(has_emphasis);
        assert!(has_strong);
    }

    #[test]
    fn test_parse_link() {
        let result = parse("[link](https://example.com)").unwrap();
        let doc = result.value;
        let para = &doc.content.children[0];
        let link = para.children.iter().find(|n| n.kind.as_str() == node::LINK);
        assert!(link.is_some());
        assert_eq!(link.unwrap().props.get_str(prop::URL), Some("https://example.com"));
    }

    #[test]
    fn test_parse_code_block() {
        let result = parse("```rust\nfn main() {}\n```").unwrap();
        let doc = result.value;
        let cb = doc.content.children.iter().find(|n| n.kind.as_str() == node::CODE_BLOCK);
        assert!(cb.is_some());
        assert_eq!(cb.unwrap().props.get_str(prop::LANGUAGE), Some("rust"));
    }

    #[test]
    fn test_parse_list() {
        let result = parse("- item 1\n- item 2").unwrap();
        let doc = result.value;
        let list = doc.content.children.iter().find(|n| n.kind.as_str() == node::LIST);
        assert!(list.is_some());
        assert_eq!(list.unwrap().props.get_bool(prop::ORDERED), Some(false));
        assert_eq!(list.unwrap().children.len(), 2);
    }
}
