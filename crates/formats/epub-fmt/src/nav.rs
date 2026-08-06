//! EPUB3 navigation document (`nav.xhtml`) <-> [`Navigation`] projection.
//!
//! `nav.xhtml` is itself an XHTML content document, so it is parsed via
//! `html-fmt::HtmlDoc` like any other content document (no bespoke XHTML
//! parsing here). This module only *projects* the useful navigation
//! structure (`<nav epub:type="toc|page-list|landmarks|...">` -> nested
//! `<ol>`/`<li>` lists) out of that already-parsed tree — `emit()` always
//! re-serializes [`Navigation::doc`] (the full parsed document), so the
//! projection is read-only convenience, not the source of truth for
//! writing.

use crate::ast::{NavList, NavPoint};
use html_fmt::Node;

/// Find every `<nav epub:type="...">` element anywhere in the document
/// tree, returning `(epub:type value, element)` pairs.
pub fn find_navs(doc: &html_fmt::HtmlDoc) -> Vec<(String, &Node)> {
    let mut out = Vec::new();
    for node in &doc.nodes {
        collect_navs(node, &mut out);
    }
    out
}

fn collect_navs<'a>(node: &'a Node, out: &mut Vec<(String, &'a Node)>) {
    if node.tag() == Some("nav") {
        let epub_type = node.get_attr("epub:type").unwrap_or("").to_string();
        out.push((epub_type, node));
    }
    if let Some(children) = node.children() {
        for child in children {
            collect_navs(child, out);
        }
    }
}

/// Extract a [`NavList`] from a `<nav>` element: its heading (first
/// `<h1>`-`<h6>`, if present) and the items of its first top-level `<ol>`.
pub fn extract_nav_list(nav: &Node) -> NavList {
    let children = nav.children().unwrap_or(&[]);
    let heading = children
        .iter()
        .find(
            |c| matches!(c.tag(), Some(t) if matches!(t, "h1" | "h2" | "h3" | "h4" | "h5" | "h6")),
        )
        .map(text_content);
    let items = children
        .iter()
        .find(|c| c.tag() == Some("ol"))
        .map(extract_ol)
        .unwrap_or_default();
    NavList { heading, items }
}

fn extract_ol(ol: &Node) -> Vec<NavPoint> {
    ol.children()
        .unwrap_or(&[])
        .iter()
        .filter(|c| c.tag() == Some("li"))
        .map(extract_li)
        .collect()
}

fn extract_li(li: &Node) -> NavPoint {
    let children = li.children().unwrap_or(&[]);
    let anchor = children.iter().find(|c| c.tag() == Some("a"));
    let (label, href) = match anchor {
        Some(a) => (text_content(a), a.get_attr("href").map(str::to_string)),
        None => {
            let span = children.iter().find(|c| c.tag() == Some("span"));
            match span {
                Some(s) => (text_content(s), None),
                None => (String::new(), None),
            }
        }
    };
    let nested = children
        .iter()
        .find(|c| c.tag() == Some("ol"))
        .map(extract_ol)
        .unwrap_or_default();
    NavPoint {
        label,
        href,
        children: nested,
    }
}

fn text_content(node: &Node) -> String {
    let mut out = String::new();
    collect_text(node, &mut out);
    out
}

fn collect_text(node: &Node, out: &mut String) {
    if let Node::Text { content, .. } = node {
        out.push_str(content);
    }
    if let Some(children) = node.children() {
        for child in children {
            collect_text(child, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rescribe_format_api::Parse as _;

    #[test]
    fn extracts_toc() {
        let html = br#"<!DOCTYPE html><html xmlns:epub="http://www.idpf.org/2007/ops"><body>
            <nav epub:type="toc"><h1>Contents</h1><ol>
                <li><a href="ch1.xhtml">Chapter 1</a></li>
                <li><a href="ch2.xhtml">Chapter 2</a><ol>
                    <li><a href="ch2.xhtml#s1">Section 1</a></li>
                </ol></li>
            </ol></nav>
        </body></html>"#;
        let (doc, _) = html_fmt::HtmlDoc::parse(html);
        let navs = find_navs(&doc);
        assert_eq!(navs.len(), 1);
        assert_eq!(navs[0].0, "toc");
        let list = extract_nav_list(navs[0].1);
        assert_eq!(list.heading.as_deref(), Some("Contents"));
        assert_eq!(list.items.len(), 2);
        assert_eq!(list.items[1].children.len(), 1);
    }
}
