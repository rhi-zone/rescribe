//! Minimal generic XML element-tree parse/emit, shared by
//! `container.rs`/`opf.rs`/`ncx.rs`. `container.xml`, the OPF package
//! document, and the NCX document are all simple, shallow, non-mixed-
//! content XML — nothing close to the recovery/tree-construction
//! complexity HTML5 has, so a generic recursive-descent tree (rather than
//! delegating to `html-fmt`, which is XHTML-specific and applies HTML5
//! parsing rules) is the right, small, self-contained tool. This is
//! genuinely `epub-fmt`'s own parsing logic (not something belonging in
//! another `-fmt` crate) — OPF/NCX/OCF container XML are EPUB-specific
//! sub-formats with no independent ecosystem crate to delegate to.
//!
//! Element/attribute names are kept exactly as written (including any
//! namespace prefix, e.g. `dc:title`, `opf:role`) — this crate resolves
//! meaning by prefix convention (`dc:`, `opf:`) rather than doing real XML
//! namespace resolution via `xmlns` declarations, which is sufficient for
//! every real-world EPUB (the prefixes are conventional/fixed in the OPF
//! spec) and keeps this module simple. `xmlns*` attributes on the root
//! element are preserved verbatim like any other attribute so they still
//! round-trip.

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event as XmlEvent};

#[derive(Clone, Debug, PartialEq, Default)]
pub struct XmlElement {
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<XmlNode>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum XmlNode {
    Element(XmlElement),
    Text(String),
    Comment(String),
}

impl XmlElement {
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    pub fn children_named(&self, name: &str) -> Vec<&XmlElement> {
        self.children
            .iter()
            .filter_map(|n| match n {
                XmlNode::Element(e) if e.name == name => Some(e),
                _ => None,
            })
            .collect()
    }

    pub fn child_named(&self, name: &str) -> Option<&XmlElement> {
        self.children.iter().find_map(|n| match n {
            XmlNode::Element(e) if e.name == name => Some(e),
            _ => None,
        })
    }

    /// Concatenated text content of direct `Text` children (element
    /// content is assumed non-mixed for the sub-formats this module
    /// serves; any nested element's text is not included).
    pub fn text(&self) -> String {
        self.children
            .iter()
            .filter_map(|n| match n {
                XmlNode::Text(t) => Some(t.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }

    /// Recursively re-serialize `raw_inner` (this element's children,
    /// serialized as XML) — used to raw-preserve elements this crate
    /// doesn't specially model.
    pub fn inner_xml(&self) -> String {
        let mut out = String::new();
        for child in &self.children {
            write_node(&mut out, child);
        }
        out
    }
}

fn write_node(out: &mut String, node: &XmlNode) {
    match node {
        XmlNode::Text(t) => out.push_str(&escape_text(t)),
        XmlNode::Comment(c) => {
            out.push_str("<!--");
            out.push_str(c);
            out.push_str("-->");
        }
        XmlNode::Element(e) => write_element(out, e),
    }
}

fn write_element(out: &mut String, e: &XmlElement) {
    out.push('<');
    out.push_str(&e.name);
    for (k, v) in &e.attrs {
        out.push(' ');
        out.push_str(k);
        out.push_str("=\"");
        out.push_str(&escape_attr(v));
        out.push('"');
    }
    if e.children.is_empty() {
        out.push_str("/>");
        return;
    }
    out.push('>');
    for child in &e.children {
        write_node(out, child);
    }
    out.push_str("</");
    out.push_str(&e.name);
    out.push('>');
}

fn escape_text(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_attr(s: &str) -> String {
    escape_text(s).replace('"', "&quot;")
}

/// Parse `input` into its single root element, ignoring the XML
/// declaration/DOCTYPE (both are fixed/uninteresting for these
/// sub-formats — none of `container.xml`/OPF/NCX carry a meaningful
/// DOCTYPE, and this crate always re-emits a fixed `<?xml version="1.0"
/// encoding="UTF-8"?>` declaration on write).
pub fn parse_xml(input: &[u8]) -> Result<XmlElement, String> {
    let mut reader = Reader::from_reader(input);
    reader.config_mut().trim_text(false);
    let mut stack: Vec<XmlElement> = Vec::new();
    let mut root: Option<XmlElement> = None;
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match reader.read_event_into(&mut buf) {
            Ok(XmlEvent::Start(e)) => {
                stack.push(element_from_start(&e)?);
            }
            Ok(XmlEvent::Empty(e)) => {
                let el = element_from_start(&e)?;
                push_finished(&mut stack, &mut root, el);
            }
            Ok(XmlEvent::End(_)) => {
                let el = stack.pop().ok_or("unbalanced closing tag")?;
                push_finished(&mut stack, &mut root, el);
            }
            Ok(XmlEvent::Text(t)) => {
                // `BytesText::decode` both decodes the source encoding and
                // unescapes predefined XML entities (`&amp;`, `&lt;`, ...)
                // — no separate unescape pass is needed (confirmed by a
                // roundtrip test; an earlier version of this code that
                // additionally called `quick_xml::escape::unescape` on the
                // already-unescaped result double-unescaped and corrupted
                // any text containing `&`).
                let text = t.decode().map_err(|e| e.to_string())?.into_owned();
                if let Some(top) = stack.last_mut() {
                    top.children.push(XmlNode::Text(text));
                }
            }
            Ok(XmlEvent::CData(t)) => {
                let text = String::from_utf8_lossy(t.as_ref()).into_owned();
                if let Some(top) = stack.last_mut() {
                    top.children.push(XmlNode::Text(text));
                }
            }
            Ok(XmlEvent::Comment(c)) => {
                let text = String::from_utf8_lossy(c.as_ref()).into_owned();
                if let Some(top) = stack.last_mut() {
                    top.children.push(XmlNode::Comment(text));
                }
            }
            Ok(XmlEvent::GeneralRef(r)) => {
                // quick-xml 0.39+ tokenizes `&amp;`/`&#65;`/etc. as a
                // separate `GeneralRef` event rather than leaving it
                // embedded in the surrounding `Text` event's raw bytes —
                // verified via this module's own roundtrip test (an
                // earlier version of this code assumed `Text::decode()`
                // unescaped entities inline and silently dropped these
                // events, corrupting any text containing `&`). Numeric
                // character references resolve directly; the five
                // predefined XML named entities are mapped explicitly;
                // any other named entity (this crate does not resolve a
                // DTD) is re-emitted as the literal `&name;` so it still
                // round-trips instead of being silently dropped.
                let ch = r.resolve_char_ref().map_err(|e| e.to_string())?;
                let text = match ch {
                    Some(c) => c.to_string(),
                    None => {
                        let name = r.decode().map_err(|e| e.to_string())?;
                        match name.as_ref() {
                            "amp" => "&".to_string(),
                            "lt" => "<".to_string(),
                            "gt" => ">".to_string(),
                            "apos" => "'".to_string(),
                            "quot" => "\"".to_string(),
                            other => format!("&{other};"),
                        }
                    }
                };
                if let Some(top) = stack.last_mut() {
                    top.children.push(XmlNode::Text(text));
                }
            }
            Ok(XmlEvent::Eof) => break,
            // Declaration/DOCTYPE/processing-instruction: no structural
            // content for the sub-formats this module serves (OCF
            // container.xml, OPF, NCX).
            Ok(_) => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    root.ok_or_else(|| "no root element found".to_string())
}

fn push_finished(stack: &mut [XmlElement], root: &mut Option<XmlElement>, el: XmlElement) {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(XmlNode::Element(el));
    } else {
        *root = Some(el);
    }
}

fn element_from_start(e: &BytesStart) -> Result<XmlElement, String> {
    let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
    let mut attrs = Vec::new();
    for a in e.attributes() {
        let a = a.map_err(|e| e.to_string())?;
        let key = String::from_utf8_lossy(a.key.as_ref()).into_owned();
        let value = a.unescape_value().map_err(|e| e.to_string())?.into_owned();
        attrs.push((key, value));
    }
    Ok(XmlElement {
        name,
        attrs,
        children: Vec::new(),
    })
}

/// Serialize `root` as a complete XML document with a fixed UTF-8
/// declaration.
pub fn emit_xml(root: &XmlElement) -> Vec<u8> {
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    write_element(&mut out, root);
    out.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_simple() {
        let input =
            br#"<?xml version="1.0"?><root a="1"><child>text &amp; more</child><!--c--></root>"#;
        let el = parse_xml(input).unwrap();
        assert_eq!(el.name, "root");
        assert_eq!(el.attr("a"), Some("1"));
        let child = el.child_named("child").unwrap();
        assert_eq!(child.text(), "text & more");
        let emitted = emit_xml(&el);
        let el2 = parse_xml(&emitted).unwrap();
        assert_eq!(el, el2);
    }
}
