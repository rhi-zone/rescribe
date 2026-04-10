//! FictionBook 2 (FB2) writer for rescribe.
//!
//! Thin adapter over `fb2-fmt`. Converts rescribe IR → native FB2 AST → XML bytes.

use fb2_fmt::{
    Author, Binary, Body, Cite, CiteContent, Description, DocumentInfo, FictionBook, Image,
    InlineElement, Poem, Section, SectionContent, Stanza, Table, TableCell, TableRow, Title,
    TitleInfo, TitlePara,
};
use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node};
use rescribe_std::{node, prop};

/// Emit a document to FB2 XML.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    emit_with_options(doc, &EmitOptions::default())
}

/// Emit a document to FB2 XML with options.
pub fn emit_with_options(
    doc: &Document,
    _options: &EmitOptions,
) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let fb = doc_to_fb(doc);
    let bytes = fb2_fmt::emit(&fb);
    Ok(ConversionResult::ok(bytes))
}

fn doc_to_fb(doc: &Document) -> FictionBook {
    let genre = doc.metadata.get_str("genre").unwrap_or("prose").to_string();
    let author_str = doc.metadata.get_str("author").unwrap_or("").to_string();
    let book_title = doc.metadata.get_str("title").unwrap_or("Untitled").to_string();
    let lang = doc.metadata.get_str("lang").unwrap_or("en").to_string();

    let author = if !author_str.is_empty() {
        let parts: Vec<&str> = author_str.splitn(2, ' ').collect();
        vec![Author {
            first_name: parts.first().map(|s| s.to_string()),
            last_name: if parts.len() >= 2 {
                Some(parts[1].to_string())
            } else {
                None
            },
            ..Default::default()
        }]
    } else {
        vec![]
    };

    let title_info = TitleInfo {
        genre: vec![genre],
        author,
        book_title,
        lang,
        ..Default::default()
    };

    let description = Description {
        title_info,
        document_info: Some(DocumentInfo {
            program_used: Some("rescribe".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };

    let body_section = nodes_to_section(&doc.content.children);
    let body = Body {
        section: vec![body_section],
        ..Default::default()
    };

    let binaries: Vec<Binary> = doc
        .resources
        .iter()
        .map(|(id, res)| Binary {
            id: id.as_str().to_string(),
            content_type: res.mime_type.clone(),
            data: res.data.clone(),
        })
        .collect();

    FictionBook {
        description,
        bodies: vec![body],
        binaries,
    }
}

/// Convert a flat list of rescribe nodes into a single FB2 section.
fn nodes_to_section(nodes: &[Node]) -> Section {
    let mut section_title: Option<Title> = None;
    let mut content = Vec::new();
    for node in nodes {
        match node.kind.as_str() {
            node::DOCUMENT => {
                let inner = nodes_to_section(&node.children);
                return inner;
            }
            node::DIV => {
                // Recursively convert nested divs as sub-sections
                // (this mirrors write_node's DIV handling)
                let class = node.props.get_str("html:class");
                match class {
                    Some("poem") => {
                        content.push(SectionContent::Poem(div_to_poem(node)));
                    }
                    Some("stanza") => {} // handled inside poem
                    _ => {
                        // A div becomes a nested section content — flatten
                        let inner = nodes_to_section(&node.children);
                        if inner.title.is_some() && section_title.is_none() {
                            section_title = inner.title;
                        }
                        content.extend(inner.content);
                    }
                }
            }
            node::HEADING => {
                // First heading becomes section title; subsequent ones become subtitle
                let inlines = nodes_to_inlines(&node.children);
                if section_title.is_none() {
                    section_title = Some(Title {
                        para: vec![TitlePara::Para(inlines)],
                        ..Default::default()
                    });
                } else {
                    content.push(SectionContent::Subtitle(inlines));
                }
            }
            node::PARAGRAPH => {
                if node.children.is_empty() {
                    content.push(SectionContent::EmptyLine);
                } else {
                    content.push(SectionContent::Para(nodes_to_inlines(&node.children)));
                }
            }
            node::BLOCKQUOTE => {
                content.push(SectionContent::Cite(blockquote_to_cite(node)));
            }
            node::IMAGE => {
                let href = node.props.get_str(prop::URL).unwrap_or("").to_string();
                let href = if href.starts_with('#') || href.contains("://") {
                    href
                } else {
                    format!("#{href}")
                };
                content.push(SectionContent::Image(Image {
                    href,
                    ..Default::default()
                }));
            }
            node::TABLE => {
                content.push(SectionContent::Table(node_to_table(node)));
            }
            node::LIST => {
                // FB2 has no native lists — render as paragraphs with markers
                let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
                let mut num = 1u32;
                for child in &node.children {
                    if child.kind.as_str() == node::LIST_ITEM {
                        let mut inlines = Vec::new();
                        let marker = if ordered {
                            let m = format!("{num}. ");
                            num += 1;
                            m
                        } else {
                            "• ".to_string()
                        };
                        inlines.push(InlineElement::Text(marker));
                        for item_child in &child.children {
                            if item_child.kind.as_str() == node::PARAGRAPH {
                                inlines.extend(nodes_to_inlines(&item_child.children));
                            } else {
                                inlines.push(node_to_inline(item_child));
                            }
                        }
                        content.push(SectionContent::Para(inlines));
                    }
                }
            }
            node::CODE_BLOCK => {
                // No code blocks in FB2 — wrap in p > code
                let code_text = node
                    .props
                    .get_str(prop::CONTENT)
                    .unwrap_or("")
                    .to_string();
                content.push(SectionContent::Para(vec![InlineElement::Code(code_text)]));
            }
            node::HORIZONTAL_RULE => {
                content.push(SectionContent::EmptyLine);
            }
            node::FIGURE => {
                // Recurse
                let inner = nodes_to_section(&node.children);
                content.extend(inner.content);
            }
            // Inline nodes at block level — wrap in p
            node::TEXT | node::EMPHASIS | node::STRONG | node::CODE | node::LINK => {
                content.push(SectionContent::Para(vec![node_to_inline(node)]));
            }
            _ => {}
        }
    }
    Section {
        title: section_title,
        content,
        ..Default::default()
    }
}

fn blockquote_to_cite(node: &Node) -> Cite {
    let mut content = Vec::new();
    let mut text_author = Vec::new();
    for child in &node.children {
        if child.props.get_str("html:class") == Some("text-author") {
            text_author.push(nodes_to_inlines(&child.children));
            continue;
        }
        match child.kind.as_str() {
            node::PARAGRAPH => {
                if child.children.is_empty() {
                    content.push(CiteContent::EmptyLine);
                } else {
                    content.push(CiteContent::Para(nodes_to_inlines(&child.children)));
                }
            }
            node::TABLE => {
                content.push(CiteContent::Table(node_to_table(child)));
            }
            node::DIV if child.props.get_str("html:class") == Some("poem") => {
                content.push(CiteContent::Poem(div_to_poem(child)));
            }
            _ => {}
        }
    }
    Cite {
        content,
        text_author,
        ..Default::default()
    }
}

fn div_to_poem(node: &Node) -> Poem {
    let stanzas = node
        .children
        .iter()
        .filter_map(|child| {
            if child.kind.as_str() == node::DIV
                && child.props.get_str("html:class") == Some("stanza")
            {
                let v: Vec<Vec<InlineElement>> = child
                    .children
                    .iter()
                    .filter_map(|span| {
                        if span.kind.as_str() == node::SPAN {
                            // Verse line: SPAN with LINE_BREAK at end
                            let inlines: Vec<InlineElement> = span
                                .children
                                .iter()
                                .filter(|n| n.kind.as_str() != node::LINE_BREAK)
                                .map(node_to_inline)
                                .collect();
                            Some(inlines)
                        } else {
                            None
                        }
                    })
                    .collect();
                Some(Stanza { v, ..Default::default() })
            } else {
                None
            }
        })
        .collect();
    Poem {
        stanza: stanzas,
        ..Default::default()
    }
}

fn node_to_table(node: &Node) -> Table {
    let rows = node
        .children
        .iter()
        .filter_map(|row| {
            if row.kind.as_str() == node::TABLE_ROW {
                let cells = row
                    .children
                    .iter()
                    .map(|cell| TableCell {
                        is_header: cell.kind.as_str() == node::TABLE_HEADER,
                        content: nodes_to_inlines(&cell.children),
                        ..Default::default()
                    })
                    .collect();
                Some(TableRow {
                    cell: cells,
                    ..Default::default()
                })
            } else {
                None
            }
        })
        .collect();
    Table {
        row: rows,
        ..Default::default()
    }
}

fn nodes_to_inlines(nodes: &[Node]) -> Vec<InlineElement> {
    nodes.iter().map(node_to_inline).collect()
}

fn node_to_inline(node: &Node) -> InlineElement {
    match node.kind.as_str() {
        node::TEXT => {
            InlineElement::Text(node.props.get_str(prop::CONTENT).unwrap_or("").to_string())
        }
        node::STRONG => InlineElement::Strong(nodes_to_inlines(&node.children)),
        node::EMPHASIS => InlineElement::Emphasis(nodes_to_inlines(&node.children)),
        node::STRIKEOUT => InlineElement::Strikethrough(nodes_to_inlines(&node.children)),
        node::SUBSCRIPT => InlineElement::Sub(nodes_to_inlines(&node.children)),
        node::SUPERSCRIPT => InlineElement::Sup(nodes_to_inlines(&node.children)),
        node::CODE => InlineElement::Code(
            node.props.get_str(prop::CONTENT).unwrap_or("").to_string(),
        ),
        node::IMAGE => {
            let href = node.props.get_str(prop::URL).unwrap_or("").to_string();
            let href = if href.starts_with('#') || href.contains("://") {
                href
            } else {
                format!("#{href}")
            };
            InlineElement::Image(Image {
                href,
                ..Default::default()
            })
        }
        node::LINK => {
            let href = node.props.get_str(prop::URL).unwrap_or("").to_string();
            InlineElement::Link {
                href,
                kind: None,
                children: nodes_to_inlines(&node.children),
            }
        }
        node::LINE_BREAK => InlineElement::Text("\n".to_string()),
        node::SOFT_BREAK => InlineElement::Text(" ".to_string()),
        _ => InlineElement::Text(String::new()),
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

        let xml = emit_str(&doc);
        assert!(xml.contains("<FictionBook"));
        assert!(xml.contains("</FictionBook>"));
        assert!(xml.contains("<book-title>Untitled</book-title>"));
    }

    #[test]
    fn test_emit_with_title() {
        let mut metadata = Properties::new();
        metadata.set("title", "Test Book".to_string());

        let doc = Document {
            content: Node::new(node::DOCUMENT),
            resources: Default::default(),
            metadata,
            source: None,
        };

        let xml = emit_str(&doc);
        assert!(xml.contains("<book-title>Test Book</book-title>"));
    }

    #[test]
    fn test_emit_paragraph() {
        let doc = doc(|d| d.para(|p| p.text("Hello, world!")));
        let xml = emit_str(&doc);
        assert!(xml.contains("<p>Hello, world!</p>"));
    }

    #[test]
    fn test_emit_heading() {
        let doc = doc(|d| d.heading(1, |h| h.text("Chapter Title")));
        let xml = emit_str(&doc);
        assert!(xml.contains("<title><p>Chapter Title</p></title>"));
    }

    #[test]
    fn test_emit_emphasis() {
        let doc = doc(|d| d.para(|p| p.em(|e| e.text("italic"))));
        let xml = emit_str(&doc);
        assert!(xml.contains("<emphasis>italic</emphasis>"));
    }

    #[test]
    fn test_emit_strong() {
        let doc = doc(|d| d.para(|p| p.strong(|s| s.text("bold"))));
        let xml = emit_str(&doc);
        assert!(xml.contains("<strong>bold</strong>"));
    }

    #[test]
    fn test_emit_link() {
        let doc = doc(|d| d.para(|p| p.link("http://example.com", |l| l.text("click"))));
        let xml = emit_str(&doc);
        assert!(xml.contains("<a l:href=\"http://example.com\">click</a>"));
    }

    #[test]
    fn test_emit_blockquote() {
        let doc = doc(|d| d.blockquote(|b| b.para(|p| p.text("Quoted text"))));
        let xml = emit_str(&doc);
        assert!(xml.contains("<cite>"));
        assert!(xml.contains("Quoted text"));
        assert!(xml.contains("</cite>"));
    }

    #[test]
    fn test_emit_list() {
        let doc = doc(|d| d.bullet_list(|l| l.item(|i| i.text("one")).item(|i| i.text("two"))));
        let xml = emit_str(&doc);
        assert!(xml.contains("• one"));
        assert!(xml.contains("• two"));
    }

    #[test]
    fn test_emit_ordered_list() {
        let doc =
            doc(|d| d.ordered_list(|l| l.item(|i| i.text("first")).item(|i| i.text("second"))));
        let xml = emit_str(&doc);
        assert!(xml.contains("1. first"));
        assert!(xml.contains("2. second"));
    }
}
