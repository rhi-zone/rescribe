//! `META-INF/container.xml` <-> [`Container`] translation.

use crate::ast::{Container, RootFile, Span};
use crate::xml::{XmlElement, emit_xml, parse_xml};

pub fn parse_container(bytes: &[u8]) -> Result<Container, String> {
    let root = parse_xml(bytes)?;
    let rootfiles_el = root
        .child_named("rootfiles")
        .ok_or("container.xml missing <rootfiles>")?;
    let rootfiles = rootfiles_el
        .children_named("rootfile")
        .into_iter()
        .map(|e| RootFile {
            full_path: e.attr("full-path").unwrap_or_default().to_string(),
            media_type: e.attr("media-type").unwrap_or_default().to_string(),
        })
        .collect();
    Ok(Container {
        rootfiles,
        span: Span::NONE,
    })
}

pub fn emit_container(c: &Container) -> Vec<u8> {
    let rootfile_elements = c
        .rootfiles
        .iter()
        .map(|rf| XmlElement {
            name: "rootfile".to_string(),
            attrs: vec![
                ("full-path".to_string(), rf.full_path.clone()),
                ("media-type".to_string(), rf.media_type.clone()),
            ],
            children: Vec::new(),
        })
        .map(crate::xml::XmlNode::Element)
        .collect();
    let root = XmlElement {
        name: "container".to_string(),
        attrs: vec![
            (
                "xmlns".to_string(),
                "urn:oasis:names:tc:opendocument:xmlns:container".to_string(),
            ),
            ("version".to_string(), "1.0".to_string()),
        ],
        children: vec![crate::xml::XmlNode::Element(XmlElement {
            name: "rootfiles".to_string(),
            attrs: Vec::new(),
            children: rootfile_elements,
        })],
    };
    emit_xml(&root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let c = Container {
            rootfiles: vec![RootFile {
                full_path: "OEBPS/content.opf".to_string(),
                media_type: "application/oebps-package+xml".to_string(),
            }],
            span: Span::NONE,
        };
        let bytes = emit_container(&c);
        let c2 = parse_container(&bytes).unwrap();
        assert_eq!(c, c2);
    }
}
