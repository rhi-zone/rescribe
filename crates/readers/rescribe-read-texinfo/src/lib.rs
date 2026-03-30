//! Texinfo reader for rescribe.
//!
//! Parses GNU Texinfo documentation format into rescribe's document IR.
//!
//! # Example
//!
//! ```
//! use rescribe_read_texinfo::parse;
//!
//! let texinfo = r#"@chapter Introduction
//! This is the introduction.
//!
//! @section Getting Started
//! Here is how to get started."#;
//!
//! let result = parse(texinfo).unwrap();
//! let doc = result.value;
//! ```

use rescribe_core::{ConversionResult, Document, FidelityWarning, Node, ParseError, ParseOptions};
use rescribe_std::{node, prop};
use texinfo::{self, Block, Inline};

/// Parse Texinfo into a document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse Texinfo with options.
pub fn parse_with_options(
    input: &str,
    _options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let (texinfo_doc, _parse_diags) = texinfo::parse(input);

    let mut warnings: Vec<FidelityWarning> = Vec::new();
    let mut result_nodes = Vec::new();

    for block in texinfo_doc.blocks {
        result_nodes.push(block_to_node(&block, &mut warnings));
    }

    let mut metadata = rescribe_core::Properties::new();
    if let Some(title) = texinfo_doc.title {
        metadata.set("title", title);
    }

    let document = Document {
        content: Node::new(node::DOCUMENT).children(result_nodes),
        resources: Default::default(),
        metadata,
        source: None,
    };

    Ok(ConversionResult::with_warnings(document, warnings))
}

fn block_to_node(block: &Block, _warnings: &mut Vec<FidelityWarning>) -> Node {
    match block {
        Block::Heading { level, inlines, .. } => {
            let inline_nodes = inlines_to_nodes(inlines);
            Node::new(node::HEADING)
                .prop(prop::LEVEL, *level as i64)
                .children(inline_nodes)
        }

        Block::Paragraph { inlines, .. } => {
            let inline_nodes = inlines_to_nodes(inlines);
            Node::new(node::PARAGRAPH).children(inline_nodes)
        }

        Block::CodeBlock { content, .. } => {
            Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone())
        }

        Block::Blockquote { children, .. } => {
            let block_nodes: Vec<_> = children
                .iter()
                .map(|b| block_to_node(b, _warnings))
                .collect();
            Node::new(node::BLOCKQUOTE).children(block_nodes)
        }

        Block::List { ordered, items, .. } => {
            let list_items: Vec<_> = items
                .iter()
                .map(|item_inlines| {
                    let inline_nodes = inlines_to_nodes(item_inlines);
                    Node::new(node::LIST_ITEM).children(inline_nodes)
                })
                .collect();

            Node::new(node::LIST)
                .prop(prop::ORDERED, *ordered)
                .children(list_items)
        }

        Block::DefinitionList { items, .. } => {
            let mut def_nodes = Vec::new();
            for (term_inlines, desc_blocks) in items {
                let term_inline_nodes = inlines_to_nodes(term_inlines);
                def_nodes.push(Node::new(node::DEFINITION_TERM).children(term_inline_nodes));

                let desc_block_nodes: Vec<_> = desc_blocks
                    .iter()
                    .map(|b| block_to_node(b, _warnings))
                    .collect();
                def_nodes.push(Node::new(node::DEFINITION_DESC).children(desc_block_nodes));
            }
            Node::new(node::DEFINITION_LIST).children(def_nodes)
        }

        Block::HorizontalRule { .. } => Node::new(node::HORIZONTAL_RULE),

        Block::Table { rows, .. } => {
            let row_nodes: Vec<_> = rows
                .iter()
                .map(|row| {
                    let cells: Vec<_> = row
                        .cells
                        .iter()
                        .map(|cell| {
                            let cell_kind = if row.is_header {
                                node::TABLE_HEADER
                            } else {
                                node::TABLE_CELL
                            };
                            Node::new(cell_kind).children(inlines_to_nodes(cell))
                        })
                        .collect();
                    Node::new(node::TABLE_ROW).children(cells)
                })
                .collect();
            Node::new(node::TABLE).children(row_nodes)
        }

        Block::Menu { entries, .. } => {
            let mut def_nodes = Vec::new();
            for entry in entries {
                def_nodes.push(
                    Node::new(node::DEFINITION_TERM)
                        .child(Node::new(node::TEXT).prop(prop::CONTENT, entry.node.clone())),
                );
                let desc_text = entry.description.clone().unwrap_or_default();
                def_nodes.push(
                    Node::new(node::DEFINITION_DESC)
                        .child(Node::new(node::PARAGRAPH).child(
                            Node::new(node::TEXT).prop(prop::CONTENT, desc_text),
                        )),
                );
            }
            Node::new("menu").children(def_nodes)
        }

        Block::RawBlock { environment, content, .. } => Node::new(node::RAW_BLOCK)
            .prop(prop::FORMAT, environment.clone())
            .prop(prop::CONTENT, content.clone()),

        Block::Float { float_type, label, children, .. } => {
            let mut n = Node::new("float");
            if let Some(ft) = float_type {
                n = n.prop("texinfo:float-type", ft.clone());
            }
            if let Some(lbl) = label {
                n = n.prop(prop::LABEL, lbl.clone());
            }
            let block_nodes: Vec<_> = children
                .iter()
                .map(|b| block_to_node(b, _warnings))
                .collect();
            n.children(block_nodes)
        }

        Block::NoIndent { .. } => Node::new("noindent"),
    }
}

fn inlines_to_nodes(inlines: &[Inline]) -> Vec<Node> {
    inlines.iter().map(inline_to_node).collect()
}

fn inline_to_node(inline: &Inline) -> Node {
    match inline {
        Inline::Text(s, _) => Node::new(node::TEXT).prop(prop::CONTENT, s.clone()),

        Inline::Strong(children, _) => {
            let inline_nodes = inlines_to_nodes(children);
            Node::new(node::STRONG).children(inline_nodes)
        }

        Inline::Emphasis(children, _) => {
            let inline_nodes = inlines_to_nodes(children);
            Node::new(node::EMPHASIS).children(inline_nodes)
        }

        Inline::Code(s, _) => Node::new(node::CODE).prop(prop::CONTENT, s.clone()),

        Inline::Link { url, children, .. } => {
            let inline_nodes = inlines_to_nodes(children);
            Node::new(node::LINK)
                .prop(prop::URL, url.clone())
                .children(inline_nodes)
        }

        Inline::Superscript(children, _) => {
            let inline_nodes = inlines_to_nodes(children);
            Node::new(node::SUPERSCRIPT).children(inline_nodes)
        }

        Inline::Subscript(children, _) => {
            let inline_nodes = inlines_to_nodes(children);
            Node::new(node::SUBSCRIPT).children(inline_nodes)
        }

        Inline::LineBreak { .. } => Node::new(node::LINE_BREAK),

        Inline::SoftBreak { .. } => Node::new(node::SOFT_BREAK),

        Inline::FootnoteDef { content, .. } => {
            let inline_nodes = inlines_to_nodes(content);
            Node::new(node::FOOTNOTE_DEF).children(inline_nodes)
        }

        // Texinfo semantic inlines: format-specific node kinds
        Inline::Var(children, _) => {
            let inline_nodes = inlines_to_nodes(children);
            Node::new("var").children(inline_nodes)
        }

        Inline::File(s, _) => Node::new("file").prop(prop::CONTENT, s.clone()),

        Inline::Command(s, _) => Node::new("command").prop(prop::CONTENT, s.clone()),

        Inline::Option(s, _) => Node::new("option").prop(prop::CONTENT, s.clone()),

        Inline::Env(s, _) => Node::new("env").prop(prop::CONTENT, s.clone()),

        Inline::Samp(s, _) => Node::new("samp").prop(prop::CONTENT, s.clone()),

        Inline::Kbd(s, _) => Node::new("kbd").prop(prop::CONTENT, s.clone()),

        Inline::Key(s, _) => Node::new("key").prop(prop::CONTENT, s.clone()),

        Inline::Dfn(children, _) => {
            let inline_nodes = inlines_to_nodes(children);
            Node::new("dfn").children(inline_nodes)
        }

        Inline::Cite(s, _) => Node::new("cite").prop(prop::CONTENT, s.clone()),

        Inline::Acronym { abbrev, expansion, .. } => {
            let mut n = Node::new("acronym").prop("texinfo:abbrev", abbrev.clone());
            if let Some(exp) = expansion {
                n = n.prop("texinfo:expansion", exp.clone());
            }
            n.prop(prop::CONTENT, abbrev.clone())
        }

        Inline::Abbr { abbrev, expansion, .. } => {
            let mut n = Node::new("abbr").prop("texinfo:abbrev", abbrev.clone());
            if let Some(exp) = expansion {
                n = n.prop("texinfo:expansion", exp.clone());
            }
            n.prop(prop::CONTENT, abbrev.clone())
        }

        Inline::Roman(s, _) => Node::new("roman")
            .child(Node::new(node::TEXT).prop(prop::CONTENT, s.clone())),

        Inline::SmallCaps(s, _) => Node::new("small_caps")
            .child(Node::new(node::TEXT).prop(prop::CONTENT, s.clone())),

        Inline::DirectItalic(children, _) => {
            let inline_nodes = inlines_to_nodes(children);
            Node::new("direct_italic").children(inline_nodes)
        }

        Inline::DirectBold(children, _) => {
            let inline_nodes = inlines_to_nodes(children);
            Node::new("direct_bold").children(inline_nodes)
        }

        Inline::DirectTypewriter(s, _) => Node::new("direct_typewriter").prop(prop::CONTENT, s.clone()),

        Inline::Image { file, alt, .. } => {
            let mut n = Node::new(node::IMAGE).prop(prop::URL, file.clone());
            if let Some(a) = alt {
                n = n.prop(prop::ALT, a.clone());
            }
            n
        }

        Inline::CrossRef { node: ref_node, text, .. } => {
            let mut n = Node::new("cross_ref").prop("texinfo:node", ref_node.clone());
            if let Some(t) = text {
                n = n.prop(prop::CONTENT, t.clone());
            }
            n
        }

        Inline::Anchor { name, .. } => Node::new("anchor").prop(prop::ID, name.clone()),

        Inline::NoBreak(s, _) => Node::new("no_break").prop(prop::CONTENT, s.clone()),

        Inline::Email { address, text, .. } => {
            let mut n = Node::new("email").prop("texinfo:address", address.clone());
            if let Some(t) = text {
                n = n.prop(prop::CONTENT, t.clone());
            }
            n
        }

        Inline::Symbol(kind, _) => {
            use texinfo::SymbolKind;
            let sym = match kind {
                SymbolKind::Dots => "dots",
                SymbolKind::EndDots => "enddots",
                SymbolKind::Minus => "minus",
                SymbolKind::Copyright => "copyright",
                SymbolKind::Registered => "registered",
                SymbolKind::LaTeX => "latex",
                SymbolKind::TeX => "tex",
                SymbolKind::Tie => "tie",
            };
            Node::new("symbol").prop("symbol", sym.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let input = r#"@chapter Introduction
This is the introduction paragraph.

@section Getting Started
Here is how to get started."#;

        let result = parse(input).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_headings() {
        let input = r#"@chapter Chapter One
@section Section One
@subsection Subsection One
@subsubsection Sub-subsection"#;

        let result = parse(input).unwrap();
        let doc = result.value;
        assert_eq!(doc.content.children.len(), 4);
    }

    #[test]
    fn test_parse_emphasis() {
        let input = r#"This is @emph{emphasized} and @strong{bold} text."#;

        let result = parse(input).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_code() {
        let input = r#"Use @code{printf} to print."#;

        let result = parse(input).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_list() {
        let input = r#"@itemize
@item First item
@item Second item
@end itemize"#;

        let result = parse(input).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
        assert_eq!(doc.content.children[0].kind.as_str(), node::LIST);
    }

    #[test]
    fn test_parse_enumerate() {
        let input = r#"@enumerate
@item First
@item Second
@end enumerate"#;

        let result = parse(input).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
        let list = &doc.content.children[0];
        assert_eq!(list.props.get_bool(prop::ORDERED), Some(true));
    }

    #[test]
    fn test_parse_example() {
        let input = r#"@example
int main() {
    return 0;
}
@end example"#;

        let result = parse(input).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
        assert_eq!(doc.content.children[0].kind.as_str(), node::CODE_BLOCK);
    }

    #[test]
    fn test_parse_url() {
        let input = r#"Visit @uref{https://example.com, Example Site}."#;

        let result = parse(input).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_quotation() {
        let input = r#"@quotation
This is a quoted passage.
@end quotation"#;

        let result = parse(input).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
        assert_eq!(doc.content.children[0].kind.as_str(), node::BLOCKQUOTE);
    }

    #[test]
    fn test_skip_comments() {
        let input = r#"@c This is a comment
This is visible.
@comment Another comment
Still visible."#;

        let result = parse(input).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }
}
