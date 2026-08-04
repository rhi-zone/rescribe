//! OPML emitter: converts an [`OpmlDoc`] AST back to bytes using
//! `quick_xml::Writer` directly (handles attribute/text escaping) — an
//! independent implementation from [`crate::writer::Writer`], not routed
//! through it (mirrors `tei-fmt`/`docbook-fmt`'s `emit.rs` vs. `writer.rs`
//! split).

use std::io::Cursor;

use quick_xml::Writer as XmlWriter;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event as XmlEvent};

use crate::ast::*;

/// Emit an `OpmlDoc` as XML bytes.
pub fn emit(doc: &OpmlDoc) -> Vec<u8> {
    let mut writer = XmlWriter::new_with_indent(Cursor::new(Vec::new()), b' ', 2);

    if let Some(decl) = &doc.xml_decl {
        let _ = writer.write_event(XmlEvent::Decl(BytesDecl::new(
            &decl.version,
            decl.encoding.as_deref(),
            decl.standalone.as_deref(),
        )));
    }

    let mut opml = BytesStart::new("opml");
    let version = if doc.version.is_empty() {
        "2.0"
    } else {
        &doc.version
    };
    opml.push_attribute(("version", version));
    let _ = writer.write_event(XmlEvent::Start(opml));

    emit_head(&mut writer, &doc.head);
    emit_body(&mut writer, &doc.body);

    let _ = writer.write_event(XmlEvent::End(BytesEnd::new("opml")));

    writer.into_inner().into_inner()
}

fn emit_head<W: std::io::Write>(writer: &mut XmlWriter<W>, head: &Head) {
    let _ = writer.write_event(XmlEvent::Start(BytesStart::new("head")));
    macro_rules! field {
        ($opt:expr, $tag:literal) => {
            if let Some(v) = &$opt {
                emit_text_element(writer, $tag, v);
            }
        };
    }
    field!(head.title, "title");
    field!(head.date_created, "dateCreated");
    field!(head.date_modified, "dateModified");
    field!(head.owner_name, "ownerName");
    field!(head.owner_email, "ownerEmail");
    field!(head.owner_id, "ownerId");
    field!(head.docs, "docs");
    field!(head.expansion_state, "expansionState");
    field!(head.vert_scroll_state, "vertScrollState");
    field!(head.window_top, "windowTop");
    field!(head.window_left, "windowLeft");
    field!(head.window_bottom, "windowBottom");
    field!(head.window_right, "windowRight");
    for (name, value) in &head.extra {
        emit_text_element(writer, name, value);
    }
    let _ = writer.write_event(XmlEvent::End(BytesEnd::new("head")));
}

fn emit_text_element<W: std::io::Write>(writer: &mut XmlWriter<W>, tag: &str, value: &str) {
    if value.is_empty() {
        let _ = writer.write_event(XmlEvent::Empty(BytesStart::new(tag)));
        return;
    }
    let _ = writer.write_event(XmlEvent::Start(BytesStart::new(tag)));
    let _ = writer.write_event(XmlEvent::Text(BytesText::new(value)));
    let _ = writer.write_event(XmlEvent::End(BytesEnd::new(tag)));
}

fn emit_body<W: std::io::Write>(writer: &mut XmlWriter<W>, body: &Body) {
    let _ = writer.write_event(XmlEvent::Start(BytesStart::new("body")));
    for o in &body.outlines {
        emit_outline(writer, o);
    }
    let _ = writer.write_event(XmlEvent::End(BytesEnd::new("body")));
}

fn emit_outline<W: std::io::Write>(writer: &mut XmlWriter<W>, o: &Outline) {
    let mut start = BytesStart::new("outline");
    for (k, v) in &o.attrs {
        start.push_attribute((k.as_str(), v.as_str()));
    }
    if o.children.is_empty() && o.self_closing {
        let _ = writer.write_event(XmlEvent::Empty(start));
    } else {
        let _ = writer.write_event(XmlEvent::Start(start));
        for child in &o.children {
            emit_outline(writer, child);
        }
        let _ = writer.write_event(XmlEvent::End(BytesEnd::new("outline")));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_and_roundtrips() {
        let doc = OpmlDoc {
            xml_decl: Some(XmlDecl {
                version: "1.0".into(),
                encoding: Some("UTF-8".into()),
                standalone: None,
            }),
            version: "2.0".into(),
            head: Head {
                title: Some("My List".into()),
                ..Default::default()
            },
            body: Body {
                outlines: vec![Outline {
                    attrs: vec![("text".into(), "Item 1".into())],
                    children: vec![],
                    self_closing: true,
                    span: Span::NONE,
                }],
                span: Span::NONE,
            },
            span: Span::NONE,
        };

        let bytes = emit(&doc);
        let xml = String::from_utf8(bytes.clone()).unwrap();
        assert!(xml.contains("<title>My List</title>"));
        assert!(xml.contains(r#"<outline text="Item 1"/>"#));

        let (doc2, diags) = crate::parse::parse(&bytes);
        assert!(diags.is_empty());
        assert_eq!(doc.strip_spans(), doc2.strip_spans());
    }
}
