//! DocBook/XML parser using `quick-xml`'s pull-based `Reader`.
//!
//! `quick_xml::Reader` is already a SAX-style pull parser over a `&[u8]`
//! slice — there is no intermediate DOM inside quick-xml itself. `parse()`
//! drives that reader directly into our AST (single pass, no event-layer
//! indirection), which is why `events()` (see `events.rs`) can *also* drive
//! the same reader directly and be a genuinely independent, zero-copy-where-
//! possible implementation rather than a walk over this AST.
//!
//! quick-xml 0.39 tokenizes entity references (`&name;`, `&#65;`) as their
//! own `GeneralRef` events rather than folding them into `Text`. The five
//! predefined XML entities and numeric character references are resolved
//! here and merged into the surrounding text run; any other named entity
//! (DTD-defined, unresolvable without the DTD) is preserved verbatim as
//! [`Node::EntityRef`] per the raw-preservation rule — never silently
//! dropped.

use quick_xml::Reader;
use quick_xml::events::Event as XmlEvent;

use crate::ast::*;

/// Parse a DocBook/XML document from bytes (assumed UTF-8).
///
/// Never panics: malformed XML is reported via `Diagnostic`s and parsing
/// stops at the point of failure, returning whatever tree was built so far.
pub fn parse(input: &[u8]) -> (DocBookDoc, Vec<Diagnostic>) {
    let mut reader = Reader::from_reader(input);
    reader.config_mut().trim_text(false);

    let mut diagnostics = Vec::new();
    let mut xml_decl = None;
    let mut roots: Vec<Node> = Vec::new();
    let mut stack: Vec<ElementFrame> = Vec::new();
    let mut current_text = String::new();
    let mut text_start = 0usize;
    let mut buf = Vec::new();

    macro_rules! flush_text {
        ($end:expr) => {
            if !current_text.is_empty() {
                let content = std::mem::take(&mut current_text);
                let node = Node::Text {
                    content,
                    span: Span {
                        start: text_start,
                        end: $end,
                    },
                };
                push_node(node, &mut stack, &mut roots);
            }
        };
    }

    loop {
        let pos = reader.buffer_position() as usize;
        if current_text.is_empty() {
            text_start = pos;
        }
        match reader.read_event_into(&mut buf) {
            Ok(XmlEvent::Decl(decl)) => {
                flush_text!(pos);
                let version = decl
                    .version()
                    .map(|v| String::from_utf8_lossy(&v).into_owned())
                    .unwrap_or_else(|_| "1.0".to_string());
                let encoding = decl
                    .encoding()
                    .and_then(|e| e.ok())
                    .map(|e| String::from_utf8_lossy(&e).into_owned());
                let standalone = decl
                    .standalone()
                    .and_then(|s| s.ok())
                    .map(|s| String::from_utf8_lossy(&s).into_owned());
                xml_decl = Some(XmlDecl {
                    version,
                    encoding,
                    standalone,
                });
            }
            Ok(XmlEvent::DocType(dt)) => {
                flush_text!(pos);
                let content = String::from_utf8_lossy(dt.as_ref()).into_owned();
                let node = Node::Doctype {
                    content,
                    span: Span {
                        start: pos,
                        end: reader.buffer_position() as usize,
                    },
                };
                push_node(node, &mut stack, &mut roots);
            }
            Ok(XmlEvent::PI(pi)) => {
                flush_text!(pos);
                let raw = String::from_utf8_lossy(pi.as_ref()).into_owned();
                let (target, data) = crate::ast::split_pi(&raw);
                let node = Node::ProcessingInstruction {
                    target,
                    data,
                    span: Span {
                        start: pos,
                        end: reader.buffer_position() as usize,
                    },
                };
                push_node(node, &mut stack, &mut roots);
            }
            Ok(XmlEvent::Start(e)) => {
                flush_text!(pos);
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                let attrs = read_attrs(&e, &mut diagnostics, pos);
                stack.push(ElementFrame {
                    name,
                    attrs,
                    children: Vec::new(),
                    start: pos,
                });
            }
            Ok(XmlEvent::Empty(e)) => {
                flush_text!(pos);
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                let attrs = read_attrs(&e, &mut diagnostics, pos);
                let node = Node::Element {
                    name,
                    attrs,
                    children: Vec::new(),
                    span: Span {
                        start: pos,
                        end: reader.buffer_position() as usize,
                    },
                };
                push_node(node, &mut stack, &mut roots);
            }
            Ok(XmlEvent::End(e)) => {
                flush_text!(pos);
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                match stack.pop() {
                    Some(frame) if frame.name == name => {
                        let node = Node::Element {
                            name: frame.name,
                            attrs: frame.attrs,
                            children: frame.children,
                            span: Span {
                                start: frame.start,
                                end: reader.buffer_position() as usize,
                            },
                        };
                        push_node(node, &mut stack, &mut roots);
                    }
                    Some(frame) => {
                        diagnostics.push(Diagnostic {
                            message: format!(
                                "mismatched closing tag: expected </{}>, found </{}>",
                                frame.name, name
                            ),
                            span: Span {
                                start: pos,
                                end: reader.buffer_position() as usize,
                            },
                        });
                        stack.push(frame);
                    }
                    None => {
                        diagnostics.push(Diagnostic {
                            message: format!("unexpected closing tag </{}>", name),
                            span: Span {
                                start: pos,
                                end: reader.buffer_position() as usize,
                            },
                        });
                    }
                }
            }
            Ok(XmlEvent::Text(t)) => {
                let content = t
                    .decode()
                    .map(|c| c.into_owned())
                    .unwrap_or_else(|_| String::from_utf8_lossy(t.as_ref()).into_owned());
                current_text.push_str(&content);
            }
            Ok(XmlEvent::GeneralRef(r)) => {
                if let Ok(Some(ch)) = r.resolve_char_ref() {
                    current_text.push(ch);
                } else {
                    let name = r
                        .decode()
                        .map(|c| c.into_owned())
                        .unwrap_or_else(|_| String::from_utf8_lossy(r.as_ref()).into_owned());
                    if let Some(ch) = resolve_predefined_entity(&name) {
                        current_text.push(ch);
                    } else {
                        flush_text!(pos);
                        let node = Node::EntityRef {
                            name,
                            span: Span {
                                start: pos,
                                end: reader.buffer_position() as usize,
                            },
                        };
                        push_node(node, &mut stack, &mut roots);
                    }
                }
            }
            Ok(XmlEvent::CData(c)) => {
                flush_text!(pos);
                let content = String::from_utf8_lossy(c.as_ref()).into_owned();
                let node = Node::Cdata {
                    content,
                    span: Span {
                        start: pos,
                        end: reader.buffer_position() as usize,
                    },
                };
                push_node(node, &mut stack, &mut roots);
            }
            Ok(XmlEvent::Comment(c)) => {
                flush_text!(pos);
                let content = String::from_utf8_lossy(c.as_ref()).into_owned();
                let node = Node::Comment {
                    content,
                    span: Span {
                        start: pos,
                        end: reader.buffer_position() as usize,
                    },
                };
                push_node(node, &mut stack, &mut roots);
            }
            Ok(XmlEvent::Eof) => {
                flush_text!(pos);
                break;
            }
            Err(e) => {
                flush_text!(pos);
                diagnostics.push(Diagnostic {
                    message: format!("XML parse error: {e}"),
                    span: Span {
                        start: pos,
                        end: pos,
                    },
                });
                break;
            }
        }
        buf.clear();
    }

    // Close any unclosed elements (best-effort recovery for truncated input).
    while let Some(frame) = stack.pop() {
        diagnostics.push(Diagnostic {
            message: format!("unclosed element <{}>", frame.name),
            span: Span::NONE,
        });
        let node = Node::Element {
            name: frame.name,
            attrs: frame.attrs,
            children: frame.children,
            span: Span::NONE,
        };
        push_node(node, &mut stack, &mut roots);
    }

    (
        DocBookDoc {
            xml_decl,
            nodes: roots,
        },
        diagnostics,
    )
}

struct ElementFrame {
    name: String,
    attrs: Vec<(String, String)>,
    children: Vec<Node>,
    start: usize,
}

fn push_node(node: Node, stack: &mut [ElementFrame], roots: &mut Vec<Node>) {
    if let Some(frame) = stack.last_mut() {
        frame.children.push(node);
    } else {
        roots.push(node);
    }
}

fn read_attrs(
    e: &quick_xml::events::BytesStart<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    pos: usize,
) -> Vec<(String, String)> {
    let mut attrs = Vec::new();
    for attr in e.attributes() {
        match attr {
            Ok(attr) => {
                let key = String::from_utf8_lossy(attr.key.as_ref()).into_owned();
                let value = attr
                    .unescape_value()
                    .map(|v| v.into_owned())
                    .unwrap_or_else(|_| String::from_utf8_lossy(&attr.value).into_owned());
                attrs.push((key, value));
            }
            Err(e) => diagnostics.push(Diagnostic {
                message: format!("attribute parse error: {e}"),
                span: Span {
                    start: pos,
                    end: pos,
                },
            }),
        }
    }
    attrs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_article() {
        let (doc, diags) = parse(
            br#"<?xml version="1.0"?>
<article xmlns="http://docbook.org/ns/docbook">
  <title>Example</title>
  <para>Hello, world!</para>
</article>"#,
        );
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert!(doc.xml_decl.is_some());
        let root = doc.root().unwrap();
        assert_eq!(root.name(), Some("article"));
        // Whitespace between elements is preserved as Text nodes (raw
        // preservation — see CLAUDE.md losslessness rule), so count only
        // the element children.
        let elements: Vec<_> = root
            .children()
            .unwrap()
            .iter()
            .filter(|n| matches!(n, Node::Element { .. }))
            .collect();
        assert_eq!(elements.len(), 2);
    }

    #[test]
    fn parses_attributes_and_entities() {
        let (doc, _) = parse(br#"<link url="a&amp;b">text</link>"#);
        let root = doc.root().unwrap();
        assert_eq!(root.get_attr("url"), Some("a&b"));
        assert_eq!(root.text_content(), "text");
    }

    #[test]
    fn merges_text_around_predefined_entities() {
        let (doc, diags) = parse(b"<para>a &amp; b &lt; c</para>");
        assert!(diags.is_empty());
        let root = doc.root().unwrap();
        // Should be a single merged Text child, not split across the entity.
        assert_eq!(root.children().unwrap().len(), 1);
        assert_eq!(root.text_content(), "a & b < c");
    }

    #[test]
    fn preserves_unresolvable_named_entity() {
        let (doc, diags) = parse(b"<para>a &custom; b</para>");
        assert!(diags.is_empty());
        let root = doc.root().unwrap();
        let children = root.children().unwrap();
        assert_eq!(children.len(), 3);
        assert!(matches!(&children[1], Node::EntityRef { name, .. } if name == "custom"));
    }

    #[test]
    fn resolves_numeric_char_ref() {
        let (doc, _) = parse(b"<para>&#65;</para>");
        let root = doc.root().unwrap();
        assert_eq!(root.text_content(), "A");
    }

    #[test]
    fn recovers_from_truncated_input() {
        let (doc, diags) = parse(b"<article><para>unterminated");
        assert!(!diags.is_empty());
        assert!(doc.root().is_some());
    }
}
