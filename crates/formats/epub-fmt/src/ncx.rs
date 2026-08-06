//! EPUB2 NCX (`.ncx`, `application/x-dtbncx+xml`) <-> [`Ncx`] translation.

use crate::ast::{NavList, NavPoint, Ncx};
use crate::xml::{XmlElement, XmlNode, emit_xml, parse_xml};

pub fn parse_ncx(path: &str, bytes: &[u8]) -> Result<Ncx, String> {
    let root = parse_xml(bytes)?;
    let head = root.child_named("head");
    let mut head_metas = Vec::new();
    let mut uid = None;
    if let Some(head) = head {
        for meta in head.children_named("meta") {
            let name = meta.attr("name").unwrap_or_default().to_string();
            let content = meta.attr("content").unwrap_or_default().to_string();
            if name == "dtb:uid" {
                uid = Some(content.clone());
            }
            head_metas.push((name, content));
        }
    }

    let doc_title = root
        .child_named("docTitle")
        .and_then(|t| t.child_named("text"))
        .map(|t| t.text());
    let doc_authors = root
        .children_named("docAuthor")
        .into_iter()
        .filter_map(|a| a.child_named("text"))
        .map(|t| t.text())
        .collect();

    let nav_map = root
        .child_named("navMap")
        .map(|nm| {
            nm.children_named("navPoint")
                .into_iter()
                .map(parse_nav_point)
                .collect()
        })
        .unwrap_or_default();

    let page_list = root.child_named("pageList").map(parse_nav_list_container);
    let nav_lists = root
        .children_named("navList")
        .into_iter()
        .map(parse_nav_list_container)
        .collect();

    Ok(Ncx {
        path: path.to_string(),
        uid,
        head_metas,
        doc_title,
        doc_authors,
        nav_map,
        page_list,
        nav_lists,
    })
}

fn parse_nav_point(e: &XmlElement) -> NavPoint {
    let label = e
        .child_named("navLabel")
        .and_then(|l| l.child_named("text"))
        .map(|t| t.text())
        .unwrap_or_default();
    let href = e
        .child_named("content")
        .and_then(|c| c.attr("src"))
        .map(str::to_string);
    let children = e
        .children_named("navPoint")
        .into_iter()
        .map(parse_nav_point)
        .collect();
    NavPoint {
        label,
        href,
        children,
    }
}

/// `<pageList>`/`<navList>` share the same `navLabel` + `<pageTarget>`/
/// `<navTarget>` shape (both carry a label + a `<content src="...">`); this
/// crate's [`NavList`]/[`NavPoint`] shape already generalizes over both.
fn parse_nav_list_container(e: &XmlElement) -> NavList {
    let heading = e
        .child_named("navLabel")
        .and_then(|l| l.child_named("text"))
        .map(|t| t.text());
    let items = e
        .children
        .iter()
        .filter_map(|n| match n {
            XmlNode::Element(el) if el.name == "pageTarget" || el.name == "navTarget" => {
                Some(parse_target(el))
            }
            _ => None,
        })
        .collect();
    NavList { heading, items }
}

fn parse_target(e: &XmlElement) -> NavPoint {
    let label = e
        .child_named("navLabel")
        .and_then(|l| l.child_named("text"))
        .map(|t| t.text())
        .unwrap_or_default();
    let href = e
        .child_named("content")
        .and_then(|c| c.attr("src"))
        .map(str::to_string);
    NavPoint {
        label,
        href,
        children: Vec::new(),
    }
}

pub fn emit_ncx(ncx: &Ncx) -> Vec<u8> {
    let head_children = ncx
        .head_metas
        .iter()
        .map(|(name, content)| {
            XmlNode::Element(XmlElement {
                name: "meta".to_string(),
                attrs: vec![
                    ("name".to_string(), name.clone()),
                    ("content".to_string(), content.clone()),
                ],
                children: Vec::new(),
            })
        })
        .collect();

    let mut children = vec![XmlNode::Element(XmlElement {
        name: "head".to_string(),
        attrs: Vec::new(),
        children: head_children,
    })];

    children.push(XmlNode::Element(XmlElement {
        name: "docTitle".to_string(),
        attrs: Vec::new(),
        children: vec![XmlNode::Element(text_element(
            ncx.doc_title.as_deref().unwrap_or_default(),
        ))],
    }));
    for author in &ncx.doc_authors {
        children.push(XmlNode::Element(XmlElement {
            name: "docAuthor".to_string(),
            attrs: Vec::new(),
            children: vec![XmlNode::Element(text_element(author))],
        }));
    }

    children.push(XmlNode::Element(XmlElement {
        name: "navMap".to_string(),
        attrs: Vec::new(),
        children: ncx
            .nav_map
            .iter()
            .map(|np| XmlNode::Element(emit_nav_point(np)))
            .collect(),
    }));

    if let Some(pl) = &ncx.page_list {
        children.push(XmlNode::Element(emit_nav_list_container(
            "pageList",
            "pageTarget",
            pl,
        )));
    }
    for nl in &ncx.nav_lists {
        children.push(XmlNode::Element(emit_nav_list_container(
            "navList",
            "navTarget",
            nl,
        )));
    }

    let root = XmlElement {
        name: "ncx".to_string(),
        attrs: vec![
            (
                "xmlns".to_string(),
                "http://www.daisy.org/z3986/2005/ncx/".to_string(),
            ),
            ("version".to_string(), "2005-1".to_string()),
        ],
        children,
    };
    emit_xml(&root)
}

fn text_element(text: &str) -> XmlElement {
    XmlElement {
        name: "text".to_string(),
        attrs: Vec::new(),
        children: vec![XmlNode::Text(text.to_string())],
    }
}

fn emit_nav_point(np: &NavPoint) -> XmlElement {
    let mut children = vec![
        XmlNode::Element(XmlElement {
            name: "navLabel".to_string(),
            attrs: Vec::new(),
            children: vec![XmlNode::Element(text_element(&np.label))],
        }),
        XmlNode::Element(XmlElement {
            name: "content".to_string(),
            attrs: vec![("src".to_string(), np.href.clone().unwrap_or_default())],
            children: Vec::new(),
        }),
    ];
    for child in &np.children {
        children.push(XmlNode::Element(emit_nav_point(child)));
    }
    XmlElement {
        name: "navPoint".to_string(),
        attrs: Vec::new(),
        children,
    }
}

fn emit_nav_list_container(container_name: &str, target_name: &str, list: &NavList) -> XmlElement {
    let mut children = Vec::new();
    if let Some(h) = &list.heading {
        children.push(XmlNode::Element(XmlElement {
            name: "navLabel".to_string(),
            attrs: Vec::new(),
            children: vec![XmlNode::Element(text_element(h))],
        }));
    }
    for item in &list.items {
        children.push(XmlNode::Element(XmlElement {
            name: target_name.to_string(),
            attrs: Vec::new(),
            children: vec![
                XmlNode::Element(XmlElement {
                    name: "navLabel".to_string(),
                    attrs: Vec::new(),
                    children: vec![XmlNode::Element(text_element(&item.label))],
                }),
                XmlNode::Element(XmlElement {
                    name: "content".to_string(),
                    attrs: vec![("src".to_string(), item.href.clone().unwrap_or_default())],
                    children: Vec::new(),
                }),
            ],
        }));
    }
    XmlElement {
        name: container_name.to_string(),
        attrs: Vec::new(),
        children,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let ncx = Ncx {
            path: "toc.ncx".to_string(),
            uid: Some("urn:uuid:1234".to_string()),
            head_metas: vec![("dtb:uid".to_string(), "urn:uuid:1234".to_string())],
            doc_title: Some("Sample".to_string()),
            doc_authors: vec!["Author".to_string()],
            nav_map: vec![NavPoint {
                label: "Chapter 1".to_string(),
                href: Some("chapter1.xhtml".to_string()),
                children: vec![NavPoint {
                    label: "Section 1.1".to_string(),
                    href: Some("chapter1.xhtml#sec1".to_string()),
                    children: Vec::new(),
                }],
            }],
            page_list: None,
            nav_lists: Vec::new(),
        };
        let bytes = emit_ncx(&ncx);
        let ncx2 = parse_ncx("toc.ncx", &bytes).unwrap();
        assert_eq!(ncx, ncx2);
    }
}
