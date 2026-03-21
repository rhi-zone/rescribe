//! Texinfo writer for rescribe.
//!
//! Serializes rescribe's document IR to GNU Texinfo format.
//!
//! # Example
//!
//! ```
//! use rescribe_write_texinfo::emit;
//! use rescribe_core::{Document, Node, Properties};
//!
//! let doc = Document {
//!     content: Node::new("document"),
//!     resources: Default::default(),
//!     metadata: Properties::new(),
//!     source: None,
//! };
//!
//! let result = emit(&doc).unwrap();
//! let texinfo = String::from_utf8(result.value).unwrap();
//! ```

use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node};
use rescribe_std::{node, prop};
use texinfo::{Block, Inline, Span, TexinfoDoc};

/// Emit a document to Texinfo format.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    emit_with_options(doc, &EmitOptions::default())
}

/// Emit a document to Texinfo format with options.
pub fn emit_with_options(
    doc: &Document,
    _options: &EmitOptions,
) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let mut blocks = Vec::new();

    for child in &doc.content.children {
        if let Some(block) = node_to_block(child) {
            blocks.push(block);
        }
    }

    let title = doc.metadata.get_str("title").map(|s| s.to_string());

    let texinfo_doc = TexinfoDoc { title, blocks, span: Span::NONE };
    let output = texinfo::emit(&texinfo_doc);

    Ok(ConversionResult::ok(output.into_bytes()))
}

fn node_to_block(node: &Node) -> Option<Block> {
    match node.kind.as_str() {
        node::HEADING => {
            let level = node.props.get_int(prop::LEVEL).unwrap_or(1) as u8;
            let inlines = node.children.iter().map(node_to_inline).collect();
            Some(Block::Heading { level, inlines, span: Span::NONE })
        }

        node::PARAGRAPH => {
            let inlines = node.children.iter().map(node_to_inline).collect();
            Some(Block::Paragraph { inlines, span: Span::NONE })
        }

        node::CODE_BLOCK => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            Some(Block::CodeBlock { content, span: Span::NONE })
        }

        node::BLOCKQUOTE => {
            let children = node.children.iter().filter_map(node_to_block).collect();
            Some(Block::Blockquote { children, span: Span::NONE })
        }

        node::LIST => {
            let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
            let items = node
                .children
                .iter()
                .filter_map(|child| {
                    if child.kind.as_str() == node::LIST_ITEM {
                        // Extract inlines from the list item
                        // If the item contains paragraphs, extract inlines from those
                        let inlines = if child.children.len() == 1
                            && child.children[0].kind.as_str() == node::PARAGRAPH
                        {
                            child.children[0]
                                .children
                                .iter()
                                .map(node_to_inline)
                                .collect()
                        } else {
                            child.children.iter().map(node_to_inline).collect()
                        };
                        Some(inlines)
                    } else {
                        None
                    }
                })
                .collect();
            Some(Block::List { ordered, items, span: Span::NONE })
        }

        node::DEFINITION_LIST => {
            let mut items = Vec::new();
            let mut i = 0;
            while i < node.children.len() {
                let child = &node.children[i];
                if child.kind.as_str() == node::DEFINITION_TERM {
                    let term = child.children.iter().map(node_to_inline).collect();
                    let mut desc_blocks = Vec::new();

                    if i + 1 < node.children.len() {
                        let next = &node.children[i + 1];
                        if next.kind.as_str() == node::DEFINITION_DESC {
                            desc_blocks = next.children.iter().filter_map(node_to_block).collect();
                            i += 1;
                        }
                    }

                    items.push((term, desc_blocks));
                }
                i += 1;
            }
            Some(Block::DefinitionList { items, span: Span::NONE })
        }

        node::HORIZONTAL_RULE => Some(Block::HorizontalRule { span: Span::NONE }),

        node::DOCUMENT => {
            let children: Vec<_> = node.children.iter().filter_map(node_to_block).collect();
            if children.len() == 1 {
                children.into_iter().next()
            } else {
                None
            }
        }

        _ => None,
    }
}

fn node_to_inline(node: &Node) -> Inline {
    match node.kind.as_str() {
        node::TEXT => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            Inline::Text(content, Span::NONE)
        }

        node::STRONG => {
            let children = node.children.iter().map(node_to_inline).collect();
            Inline::Strong(children, Span::NONE)
        }

        node::EMPHASIS => {
            let children = node.children.iter().map(node_to_inline).collect();
            Inline::Emphasis(children, Span::NONE)
        }

        node::CODE => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            Inline::Code(content, Span::NONE)
        }

        node::LINK => {
            let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
            let children = node.children.iter().map(node_to_inline).collect();
            Inline::Link { url, children, span: Span::NONE }
        }

        node::SUPERSCRIPT => {
            let children = node.children.iter().map(node_to_inline).collect();
            Inline::Superscript(children, Span::NONE)
        }

        node::SUBSCRIPT => {
            let children = node.children.iter().map(node_to_inline).collect();
            Inline::Subscript(children, Span::NONE)
        }

        node::LINE_BREAK => Inline::LineBreak { span: Span::NONE },

        node::SOFT_BREAK => Inline::SoftBreak { span: Span::NONE },

        node::FOOTNOTE_DEF => {
            let content = node.children.iter().map(node_to_inline).collect();
            Inline::FootnoteDef { content, span: Span::NONE }
        }

        _ => Inline::Text(String::new(), Span::NONE),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rescribe_core::Properties;
    use rescribe_std::builder::*;

    fn emit_str(doc: &Document) -> String {
        let result = emit(doc).unwrap();
        String::from_utf8(result.value).unwrap()
    }

    #[test]
    fn test_emit_empty() {
        let doc = Document {
            content: Node::new(node::DOCUMENT),
            resources: Default::default(),
            metadata: Properties::new(),
            source: None,
        };

        let output = emit_str(&doc);
        assert!(output.contains("\\input texinfo"));
        assert!(output.contains("@bye"));
    }

    #[test]
    fn test_emit_with_title() {
        let mut metadata = Properties::new();
        metadata.set("title", "Test Document".to_string());

        let doc = Document {
            content: Node::new(node::DOCUMENT),
            resources: Default::default(),
            metadata,
            source: None,
        };

        let output = emit_str(&doc);
        assert!(output.contains("@settitle Test Document"));
    }

    #[test]
    fn test_emit_heading() {
        let doc = doc(|d| d.heading(1, |h| h.text("Chapter Title")));
        let output = emit_str(&doc);
        assert!(output.contains("@chapter Chapter Title"));
    }

    #[test]
    fn test_emit_section() {
        let doc = doc(|d| d.heading(2, |h| h.text("Section Title")));
        let output = emit_str(&doc);
        assert!(output.contains("@section Section Title"));
    }

    #[test]
    fn test_emit_paragraph() {
        let doc = doc(|d| d.para(|p| p.text("Hello, world!")));
        let output = emit_str(&doc);
        assert!(output.contains("Hello, world!"));
    }

    #[test]
    fn test_emit_emphasis() {
        let doc = doc(|d| d.para(|p| p.em(|e| e.text("italic"))));
        let output = emit_str(&doc);
        assert!(output.contains("@emph{italic}"));
    }

    #[test]
    fn test_emit_strong() {
        let doc = doc(|d| d.para(|p| p.strong(|s| s.text("bold"))));
        let output = emit_str(&doc);
        assert!(output.contains("@strong{bold}"));
    }

    #[test]
    fn test_emit_code() {
        let doc = doc(|d| d.para(|p| p.code("printf")));
        let output = emit_str(&doc);
        assert!(output.contains("@code{printf}"));
    }

    #[test]
    fn test_emit_link() {
        let doc = doc(|d| d.para(|p| p.link("https://example.com", |l| l.text("Example"))));
        let output = emit_str(&doc);
        assert!(output.contains("@uref{https://example.com, Example}"));
    }

    #[test]
    fn test_emit_list() {
        let doc = doc(|d| d.bullet_list(|l| l.item(|i| i.text("one")).item(|i| i.text("two"))));
        let output = emit_str(&doc);
        assert!(output.contains("@itemize @bullet"));
        assert!(output.contains("@item one"));
        assert!(output.contains("@item two"));
        assert!(output.contains("@end itemize"));
    }

    #[test]
    fn test_emit_enumerate() {
        let doc =
            doc(|d| d.ordered_list(|l| l.item(|i| i.text("first")).item(|i| i.text("second"))));
        let output = emit_str(&doc);
        assert!(output.contains("@enumerate"));
        assert!(output.contains("@item first"));
        assert!(output.contains("@end enumerate"));
    }

    #[test]
    fn test_emit_code_block() {
        let doc = doc(|d| d.code_block("int main() {}"));
        let output = emit_str(&doc);
        assert!(output.contains("@example"));
        assert!(output.contains("int main() {}"));
        assert!(output.contains("@end example"));
    }

    #[test]
    fn test_emit_blockquote() {
        let doc = doc(|d| d.blockquote(|b| b.para(|p| p.text("Quoted text"))));
        let output = emit_str(&doc);
        assert!(output.contains("@quotation"));
        assert!(output.contains("Quoted text"));
        assert!(output.contains("@end quotation"));
    }

    #[test]
    fn test_escape_special_chars() {
        let doc = doc(|d| d.para(|p| p.text("Use @{braces}")));
        let output = emit_str(&doc);
        // @ -> @@, { -> @{, } -> @}
        assert!(output.contains("Use @@@{braces@}"));
    }
}
