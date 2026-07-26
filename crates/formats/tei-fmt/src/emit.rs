//! TEI/XML emitter: converts a `TeiDoc` AST back to bytes using
//! `quick_xml::Writer` (handles attribute/text escaping).

use std::io::Cursor;

use quick_xml::Writer as XmlWriter;
use quick_xml::events::{
    BytesCData, BytesDecl, BytesPI, BytesRef, BytesStart, BytesText, Event as XmlEvent,
};

use crate::ast::*;

/// Emit a `TeiDoc` as XML bytes.
pub fn emit(doc: &TeiDoc) -> Vec<u8> {
    let mut writer = XmlWriter::new(Cursor::new(Vec::new()));

    if let Some(decl) = &doc.xml_decl {
        let _ = writer.write_event(XmlEvent::Decl(BytesDecl::new(
            &decl.version,
            decl.encoding.as_deref(),
            decl.standalone.as_deref(),
        )));
    }

    for node in &doc.nodes {
        emit_node(&mut writer, node);
    }

    writer.into_inner().into_inner()
}

fn emit_node(writer: &mut XmlWriter<Cursor<Vec<u8>>>, node: &Node) {
    match node {
        Node::Element {
            name,
            attrs,
            children,
            ..
        } => {
            if children.is_empty() {
                let mut start = BytesStart::new(name.as_str());
                for (k, v) in attrs {
                    start.push_attribute((k.as_str(), v.as_str()));
                }
                let _ = writer.write_event(XmlEvent::Empty(start));
            } else {
                let mut start = BytesStart::new(name.as_str());
                for (k, v) in attrs {
                    start.push_attribute((k.as_str(), v.as_str()));
                }
                let _ = writer.write_event(XmlEvent::Start(start));
                for child in children {
                    emit_node(writer, child);
                }
                let _ = writer.write_event(XmlEvent::End(quick_xml::events::BytesEnd::new(
                    name.as_str(),
                )));
            }
        }
        Node::Text { content, .. } => {
            let _ = writer.write_event(XmlEvent::Text(BytesText::new(content)));
        }
        Node::Cdata { content, .. } => {
            let _ = writer.write_event(XmlEvent::CData(BytesCData::new(content)));
        }
        Node::Comment { content, .. } => {
            let _ = writer.write_event(XmlEvent::Comment(quick_xml::events::BytesText::new(
                content,
            )));
        }
        Node::ProcessingInstruction { target, data, .. } => {
            let content = if data.is_empty() {
                target.clone()
            } else {
                format!("{target} {data}")
            };
            let _ = writer.write_event(XmlEvent::PI(BytesPI::new(content)));
        }
        Node::Doctype { content, .. } => {
            let _ = writer.write_event(XmlEvent::DocType(quick_xml::events::BytesText::new(
                content,
            )));
        }
        Node::EntityRef { name, .. } => {
            let _ = writer.write_event(XmlEvent::GeneralRef(BytesRef::new(name.as_str())));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_element_with_attrs_and_text() {
        let doc = TeiDoc {
            xml_decl: Some(XmlDecl {
                version: "1.0".into(),
                encoding: Some("UTF-8".into()),
                standalone: None,
            }),
            nodes: vec![Node::Element {
                name: "p".into(),
                attrs: vec![("rend".into(), "italic".into())],
                children: vec![Node::Text {
                    content: "Hello & <world>".into(),
                    span: Span::NONE,
                }],
                span: Span::NONE,
            }],
        };

        let bytes = emit(&doc);
        let xml = String::from_utf8(bytes).unwrap();
        assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(xml.contains("<p rend=\"italic\">Hello &amp; &lt;world&gt;</p>"));
    }

    #[test]
    fn roundtrips_through_parse() {
        let input = br#"<?xml version="1.0"?><TEI><text><body><p>Body</p></body></text></TEI>"#;
        let (doc, _) = crate::parse::parse(input);
        let out = emit(&doc);
        let (doc2, diags) = crate::parse::parse(&out);
        assert!(diags.is_empty());
        assert_eq!(doc.strip_spans(), doc2.strip_spans());
    }
}
