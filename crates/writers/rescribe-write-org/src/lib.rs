//! Org-mode writer for rescribe.
//!
//! Emits documents in Emacs Org-mode format.
//!
//! Maps rescribe `Document`/`Node` to the `org-fmt` AST, then calls
//! `org_fmt::build()` to produce the final string.

pub mod builder;

use org_fmt::{Block, DefinitionItem, Inline, ListItem, ListItemContent, OrgDoc, TableRow};
use rescribe_core::{
    ConversionResult, Document, EmitError, EmitOptions, FidelityWarning, Node, Severity,
    WarningKind,
};
use rescribe_std::{node, prop};

/// Emit a document as Org-mode.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    emit_with_options(doc, &EmitOptions::default())
}

/// Emit a document as Org-mode with custom options.
pub fn emit_with_options(
    doc: &Document,
    _options: &EmitOptions,
) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let mut warnings: Vec<FidelityWarning> = Vec::new();

    // Convert metadata
    let metadata: Vec<(String, String)> = doc
        .metadata
        .iter()
        .filter_map(|(k, v)| {
            use rescribe_core::PropValue;
            let s = match v {
                PropValue::String(s) => s.clone(),
                PropValue::Int(i) => i.to_string(),
                PropValue::Float(f) => f.to_string(),
                PropValue::Bool(b) => b.to_string(),
                _ => return None,
            };
            Some((k.to_string(), s))
        })
        .collect();

    // Convert blocks
    let blocks = convert_nodes(&doc.content.children, &mut warnings);

    let org_doc = OrgDoc { blocks, metadata };
    let result = org_fmt::build(&org_doc);

    // Map build warnings to fidelity warnings
    for w in result.warnings {
        warnings.push(FidelityWarning::new(
            Severity::Minor,
            WarningKind::UnsupportedNode(w.kind),
            w.message,
        ));
    }

    Ok(ConversionResult::with_warnings(
        result.output.into_bytes(),
        warnings,
    ))
}

fn convert_nodes(nodes: &[Node], warnings: &mut Vec<FidelityWarning>) -> Vec<Block> {
    nodes
        .iter()
        .filter_map(|n| convert_node(n, warnings))
        .collect()
}

fn convert_node(n: &Node, warnings: &mut Vec<FidelityWarning>) -> Option<Block> {
    match n.kind.as_str() {
        node::DOCUMENT => {
            // Flatten document children as blocks
            // This shouldn't normally appear but handle gracefully
            Some(Block::Div {
                inlines: convert_nodes_to_inlines(&n.children, warnings),
            })
        }

        node::PARAGRAPH => Some(Block::Paragraph {
            inlines: convert_nodes_to_inlines(&n.children, warnings),
        }),

        node::HEADING => {
            let level = n.props.get_int(prop::LEVEL).unwrap_or(1) as usize;
            Some(Block::Heading {
                level,
                inlines: convert_nodes_to_inlines(&n.children, warnings),
            })
        }

        node::CODE_BLOCK => {
            let content = n.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            let language = n.props.get_str(prop::LANGUAGE).map(str::to_string);
            Some(Block::CodeBlock { language, content })
        }

        node::BLOCKQUOTE => Some(Block::Blockquote {
            children: convert_nodes(&n.children, warnings),
        }),

        node::LIST => {
            let ordered = n.props.get_bool(prop::ORDERED).unwrap_or(false);
            let mut counter = 1i32;
            let items: Vec<ListItem> = n
                .children
                .iter()
                .filter_map(|child| {
                    if child.kind.as_str() == node::LIST_ITEM {
                        Some(convert_list_item(child, &mut counter, warnings))
                    } else {
                        convert_node(child, warnings).map(|b| ListItem {
                            children: vec![ListItemContent::Block(b)],
                        })
                    }
                })
                .collect();
            Some(Block::List { ordered, items })
        }

        node::LIST_ITEM => {
            let mut counter = 1i32;
            Some(Block::List {
                ordered: false,
                items: vec![convert_list_item(n, &mut counter, warnings)],
            })
        }

        node::TABLE => Some(convert_table(n, warnings)),

        node::FIGURE => Some(Block::Figure {
            children: convert_nodes(&n.children, warnings),
        }),

        node::CAPTION => Some(Block::Caption {
            inlines: convert_nodes_to_inlines(&n.children, warnings),
        }),

        node::HORIZONTAL_RULE => Some(Block::HorizontalRule),

        node::DIV | node::SPAN => Some(Block::Div {
            inlines: convert_nodes_to_inlines(&n.children, warnings),
        }),

        node::RAW_BLOCK => {
            let format = n.props.get_str(prop::FORMAT).unwrap_or("").to_string();
            let content = n.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            Some(Block::RawBlock { format, content })
        }

        node::DEFINITION_LIST => {
            let items = convert_definition_list(&n.children, warnings);
            Some(Block::DefinitionList { items })
        }

        node::DEFINITION_TERM | node::DEFINITION_DESC => {
            // These appear inside DEFINITION_LIST and are handled there.
            // If encountered standalone, treat as paragraph.
            Some(Block::Paragraph {
                inlines: convert_nodes_to_inlines(&n.children, warnings),
            })
        }

        // Inline elements appearing at block level — wrap in paragraph
        node::TEXT
        | node::EMPHASIS
        | node::STRONG
        | node::STRIKEOUT
        | node::UNDERLINE
        | node::SUBSCRIPT
        | node::SUPERSCRIPT
        | node::CODE
        | node::LINK
        | node::IMAGE
        | node::LINE_BREAK
        | node::SOFT_BREAK
        | node::FOOTNOTE_REF
        | node::FOOTNOTE_DEF
        | node::SMALL_CAPS
        | node::QUOTED
        | node::RAW_INLINE => Some(Block::Paragraph {
            inlines: convert_nodes_to_inlines(std::slice::from_ref(n), warnings),
        }),

        "math_display" => n
            .props
            .get_str("math:source")
            .map(|source| Block::RawBlock {
                format: "org".into(),
                content: format!("\\[\n{}\n\\]\n", source),
            }),

        "math_inline" => Some(Block::Paragraph {
            inlines: vec![Inline::MathInline {
                source: n.props.get_str("math:source").unwrap_or("").to_string(),
            }],
        }),

        _ => {
            warnings.push(FidelityWarning::new(
                Severity::Minor,
                WarningKind::UnsupportedNode(n.kind.as_str().to_string()),
                format!("Unknown node type for Org: {}", n.kind.as_str()),
            ));
            // Recurse into children to not lose content
            let children = convert_nodes(&n.children, warnings);
            if children.is_empty() {
                None
            } else if children.len() == 1 {
                Some(children.into_iter().next().unwrap())
            } else {
                Some(Block::Figure { children })
            }
        }
    }
}

fn convert_list_item(
    n: &Node,
    _counter: &mut i32,
    warnings: &mut Vec<FidelityWarning>,
) -> ListItem {
    let mut children = Vec::new();

    for child in &n.children {
        if child.kind.as_str() == node::LIST {
            // Nested list
            if let Some(block) = convert_node(child, warnings) {
                children.push(ListItemContent::Block(block));
            }
        } else if child.kind.as_str() == node::PARAGRAPH {
            children.push(ListItemContent::Block(Block::Paragraph {
                inlines: convert_nodes_to_inlines(&child.children, warnings),
            }));
        } else {
            // Inline children or other block types
            let inline = try_convert_inline(child, warnings);
            if let Some(i) = inline {
                // Collect into inline content
                if let Some(ListItemContent::Inline(inlines)) = children.last_mut() {
                    inlines.push(i);
                } else {
                    children.push(ListItemContent::Inline(vec![i]));
                }
            } else if let Some(block) = convert_node(child, warnings) {
                children.push(ListItemContent::Block(block));
            }
        }
    }

    // If empty, produce an empty inline
    if children.is_empty() {
        children.push(ListItemContent::Inline(vec![]));
    }

    ListItem { children }
}

fn convert_table(n: &Node, warnings: &mut Vec<FidelityWarning>) -> Block {
    let mut rows = Vec::new();
    collect_table_rows(&n.children, &mut rows, false, warnings);
    Block::Table { rows }
}

fn collect_table_rows(
    nodes: &[Node],
    rows: &mut Vec<TableRow>,
    is_header: bool,
    warnings: &mut Vec<FidelityWarning>,
) {
    for n in nodes {
        match n.kind.as_str() {
            node::TABLE_HEAD => collect_table_rows(&n.children, rows, true, warnings),
            node::TABLE_BODY | node::TABLE_FOOT => {
                collect_table_rows(&n.children, rows, false, warnings)
            }
            node::TABLE_ROW => {
                let mut cells = Vec::new();
                for cell in &n.children {
                    cells.push(convert_nodes_to_inlines(&cell.children, warnings));
                }
                rows.push(TableRow { cells, is_header });
            }
            _ => {}
        }
    }
}

fn convert_definition_list(
    nodes: &[Node],
    warnings: &mut Vec<FidelityWarning>,
) -> Vec<DefinitionItem> {
    let mut items = Vec::new();
    let mut i = 0;
    while i < nodes.len() {
        if nodes[i].kind.as_str() == node::DEFINITION_TERM {
            let term = convert_nodes_to_inlines(&nodes[i].children, warnings);
            i += 1;
            let desc = if i < nodes.len() && nodes[i].kind.as_str() == node::DEFINITION_DESC {
                let d = &nodes[i];
                // desc may contain a paragraph child
                let desc_inlines =
                    if d.children.len() == 1 && d.children[0].kind.as_str() == node::PARAGRAPH {
                        convert_nodes_to_inlines(&d.children[0].children, warnings)
                    } else {
                        convert_nodes_to_inlines(&d.children, warnings)
                    };
                i += 1;
                desc_inlines
            } else {
                vec![]
            };
            items.push(DefinitionItem { term, desc });
        } else {
            i += 1;
        }
    }
    items
}

fn convert_nodes_to_inlines(nodes: &[Node], warnings: &mut Vec<FidelityWarning>) -> Vec<Inline> {
    nodes
        .iter()
        .filter_map(|n| try_convert_inline(n, warnings))
        .collect()
}

fn try_convert_inline(n: &Node, warnings: &mut Vec<FidelityWarning>) -> Option<Inline> {
    match n.kind.as_str() {
        node::TEXT => Some(Inline::Text(
            n.props.get_str(prop::CONTENT).unwrap_or("").to_string(),
        )),

        node::EMPHASIS => Some(Inline::Italic(convert_nodes_to_inlines(
            &n.children,
            warnings,
        ))),

        node::STRONG => Some(Inline::Bold(convert_nodes_to_inlines(
            &n.children,
            warnings,
        ))),

        node::STRIKEOUT => Some(Inline::Strikethrough(convert_nodes_to_inlines(
            &n.children,
            warnings,
        ))),

        node::UNDERLINE => Some(Inline::Underline(convert_nodes_to_inlines(
            &n.children,
            warnings,
        ))),

        node::SUBSCRIPT => Some(Inline::Subscript(convert_nodes_to_inlines(
            &n.children,
            warnings,
        ))),

        node::SUPERSCRIPT => Some(Inline::Superscript(convert_nodes_to_inlines(
            &n.children,
            warnings,
        ))),

        node::CODE => Some(Inline::Code(
            n.props.get_str(prop::CONTENT).unwrap_or("").to_string(),
        )),

        node::LINK => {
            let url = n.props.get_str(prop::URL).unwrap_or("").to_string();
            let children = convert_nodes_to_inlines(&n.children, warnings);
            Some(Inline::Link { url, children })
        }

        node::IMAGE => {
            let url = n.props.get_str(prop::URL).unwrap_or("").to_string();
            Some(Inline::Image { url })
        }

        node::LINE_BREAK => Some(Inline::LineBreak),

        node::SOFT_BREAK => Some(Inline::SoftBreak),

        node::FOOTNOTE_REF => {
            let label = n.props.get_str(prop::LABEL).unwrap_or("").to_string();
            Some(Inline::FootnoteRef { label })
        }

        node::FOOTNOTE_DEF => {
            let label = n.props.get_str(prop::LABEL).unwrap_or("").to_string();
            let children = convert_nodes_to_inlines(&n.children, warnings);
            Some(Inline::FootnoteDefinition { label, children })
        }

        node::SMALL_CAPS => {
            // Org doesn't have native small caps, emit children as-is
            let children = convert_nodes_to_inlines(&n.children, warnings);
            if children.len() == 1 {
                Some(children.into_iter().next().unwrap())
            } else if children.is_empty() {
                None
            } else {
                // Wrap in a text that concatenates — just return the first
                // or we could concatenate all text, but returning a Bold is wrong.
                // Best: return all as separate, but we can only return one Inline.
                // Return the children by wrapping them inside a no-op container.
                // We don't have such a container in OrgFmt, so just return first.
                Some(Inline::Text(collect_text(&children)))
            }
        }

        node::QUOTED => {
            let quote_type = n.props.get_str(prop::QUOTE_TYPE).unwrap_or("double");
            let children = convert_nodes_to_inlines(&n.children, warnings);
            let inner_text = collect_text(&children);
            if quote_type == "single" {
                Some(Inline::Text(format!("'{}'", inner_text)))
            } else {
                Some(Inline::Text(format!("\"{}\"", inner_text)))
            }
        }

        node::RAW_INLINE => {
            let format = n.props.get_str(prop::FORMAT).unwrap_or("");
            if format == "org" {
                let content = n.props.get_str(prop::CONTENT).unwrap_or("").to_string();
                Some(Inline::Text(content))
            } else {
                None
            }
        }

        "math_inline" => {
            let source = n.props.get_str("math:source").unwrap_or("").to_string();
            Some(Inline::MathInline { source })
        }

        _ => {
            // For unknown inlines, try to emit children
            if n.children.is_empty() {
                warnings.push(FidelityWarning::new(
                    Severity::Minor,
                    WarningKind::UnsupportedNode(n.kind.as_str().to_string()),
                    format!("Unknown inline node for Org: {}", n.kind.as_str()),
                ));
                None
            } else {
                let children = convert_nodes_to_inlines(&n.children, warnings);
                let text = collect_text(&children);
                Some(Inline::Text(text))
            }
        }
    }
}

/// Collect plain text from a slice of inlines (for fallback rendering).
fn collect_text(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(s) => out.push_str(s),
            Inline::Bold(c)
            | Inline::Italic(c)
            | Inline::Underline(c)
            | Inline::Strikethrough(c)
            | Inline::Superscript(c)
            | Inline::Subscript(c) => out.push_str(&collect_text(c)),
            Inline::Code(s) => out.push_str(s),
            Inline::Link { url, children } => {
                let t = collect_text(children);
                if t.is_empty() {
                    out.push_str(url);
                } else {
                    out.push_str(&t);
                }
            }
            Inline::Image { url } => out.push_str(url),
            Inline::LineBreak | Inline::SoftBreak => out.push(' '),
            Inline::FootnoteRef { label } | Inline::FootnoteDefinition { label, .. } => {
                out.push_str(label)
            }
            Inline::MathInline { source } => {
                out.push('$');
                out.push_str(source);
                out.push('$');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::org;

    fn emit_str(doc: &Document) -> String {
        let result = emit(doc).unwrap();
        String::from_utf8(result.value).unwrap()
    }

    #[test]
    fn test_emit_paragraph() {
        let doc = org(|d| d.para(|i| i.text("Hello, world!")));
        let output = emit_str(&doc);
        assert!(output.contains("Hello, world!"));
    }

    #[test]
    fn test_emit_heading() {
        let doc = org(|d| d.heading(1, |i| i.text("Main Title")));
        let output = emit_str(&doc);
        assert!(output.contains("* Main Title"));
    }

    #[test]
    fn test_emit_heading_levels() {
        let doc = org(|d| {
            d.heading(1, |i| i.text("Level 1"))
                .heading(2, |i| i.text("Level 2"))
                .heading(3, |i| i.text("Level 3"))
        });
        let output = emit_str(&doc);
        assert!(output.contains("* Level 1"));
        assert!(output.contains("** Level 2"));
        assert!(output.contains("*** Level 3"));
    }

    #[test]
    fn test_emit_emphasis() {
        let doc = org(|d| d.para(|i| i.italic(|i| i.text("italic"))));
        let output = emit_str(&doc);
        assert!(output.contains("/italic/"));
    }

    #[test]
    fn test_emit_strong() {
        let doc = org(|d| d.para(|i| i.bold(|i| i.text("bold"))));
        let output = emit_str(&doc);
        assert!(output.contains("*bold*"));
    }

    #[test]
    fn test_emit_link() {
        let doc = org(|d| d.para(|i| i.link("https://example.com", |i| i.text("click"))));
        let output = emit_str(&doc);
        assert!(output.contains("[[https://example.com][click]]"));
    }

    #[test]
    fn test_emit_list() {
        let doc =
            org(|d| d.unordered_list(|l| l.item(|i| i.text("item 1")).item(|i| i.text("item 2"))));
        let output = emit_str(&doc);
        assert!(output.contains("- item 1"));
        assert!(output.contains("- item 2"));
    }

    #[test]
    fn test_emit_ordered_list() {
        let doc =
            org(|d| d.ordered_list(|l| l.item(|i| i.text("first")).item(|i| i.text("second"))));
        let output = emit_str(&doc);
        assert!(output.contains("1. first"));
        assert!(output.contains("2. second"));
    }

    #[test]
    fn test_emit_code_block() {
        let doc = org(|d| d.src_block("rust", "fn main() {}"));
        let output = emit_str(&doc);
        assert!(output.contains("#+BEGIN_SRC rust"));
        assert!(output.contains("fn main() {}"));
        assert!(output.contains("#+END_SRC"));
    }

    #[test]
    fn test_emit_blockquote() {
        let doc = org(|d| d.quote(|b| b.para(|i| i.text("A quote"))));
        let output = emit_str(&doc);
        assert!(output.contains("#+BEGIN_QUOTE"));
        assert!(output.contains("A quote"));
        assert!(output.contains("#+END_QUOTE"));
    }

    #[test]
    fn test_emit_image() {
        let doc = org(|d| d.para(|i| i.image("test.png")));
        let output = emit_str(&doc);
        assert!(output.contains("[[file:test.png]]"));
    }

    #[test]
    fn test_emit_inline_code() {
        let doc = org(|d| d.para(|i| i.verbatim("inline code")));
        let output = emit_str(&doc);
        assert!(output.contains("=inline code="));
    }
}
