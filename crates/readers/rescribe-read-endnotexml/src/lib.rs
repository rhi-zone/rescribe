//! EndNote XML reader for rescribe.
//!
//! Parses EndNote XML bibliography files into rescribe's document IR,
//! using the `bibliography`/`bibliography_entry`/`bibliography_field`
//! node kinds (see `rescribe_std::node` and ADR 0005 in the rescribe
//! repo). EndNote XML field content can carry `<style>` markup elements
//! (bold/italic/underline/sub/superscript runs) — those are converted to
//! real inline nodes rather than flattened to plain text, since
//! `bibliography_field` children are ordinary inline nodes.
//!
//! # Example
//!
//! ```ignore
//! use rescribe_read_endnotexml::parse;
//!
//! let xml = r#"<?xml version="1.0"?>
//! <xml><records><record>...</record></records></xml>"#;
//! let result = parse(xml).unwrap();
//! ```

use quick_xml::Reader;
use quick_xml::events::Event;
use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions, Properties};
use rescribe_std::{PropValue, node, prop};
use std::collections::HashMap;

/// Parse EndNote XML into a document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse EndNote XML with options.
pub fn parse_with_options(
    input: &str,
    _options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let roots = parse_xml_tree(input);

    let mut records = Vec::new();
    find_all(&roots, "record", &mut records);

    let entries: Vec<Node> = records
        .iter()
        .map(|record| convert_record(children_of(record)))
        .collect();

    let content = if entries.is_empty() {
        Node::new(node::DOCUMENT)
    } else {
        Node::new(node::DOCUMENT).child(Node::new(node::BIBLIOGRAPHY).children(entries))
    };

    Ok(ConversionResult::ok(Document {
        content,
        resources: Default::default(),
        metadata: Properties::new(),
        source: None,
    }))
}

// ---------------------------------------------------------------------------
// Minimal generic XML tree (this adapter's own concern is EndNote's element
// vocabulary -> IR mapping below; the tree itself is just enough structure
// to walk that vocabulary without a fragile flat state machine, since
// EndNote's schema nests to varying depths and field content can itself
// contain child elements (`<style>`)).
// ---------------------------------------------------------------------------

enum XmlNode {
    Element {
        name: String,
        attrs: Vec<(String, String)>,
        children: Vec<XmlNode>,
    },
    Text(String),
}

/// (element name, attrs, children-so-far) for one level of open-element
/// nesting during tree construction.
type OpenElement = (String, Vec<(String, String)>, Vec<XmlNode>);

fn parse_xml_tree(input: &str) -> Vec<XmlNode> {
    let mut reader = Reader::from_str(input);
    // `trim_text` stays off: EndNote field content can be split across
    // several `<style>` runs (`<style face="normal">A </style><style
    // face="italic">Great</style>`), and a global trim would eat the
    // meaningful inter-run space along with real exporters' harmless
    // inter-element indentation whitespace — the two are indistinguishable
    // to the parser. Structural indentation between record-level elements
    // is never a problem in practice since every lookup below selects
    // specific named children (`find_child`/`find_children`), silently
    // skipping any stray whitespace-only `Text` siblings.
    reader.config_mut().trim_text(false);

    let mut stack: Vec<OpenElement> = Vec::new();
    let mut roots: Vec<XmlNode> = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let attrs = read_attrs(&e);
                stack.push((name, attrs, Vec::new()));
            }
            Ok(Event::Empty(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let attrs = read_attrs(&e);
                push_node(
                    &mut stack,
                    &mut roots,
                    XmlNode::Element {
                        name,
                        attrs,
                        children: Vec::new(),
                    },
                );
            }
            Ok(Event::End(_)) => {
                if let Some((name, attrs, children)) = stack.pop() {
                    push_node(
                        &mut stack,
                        &mut roots,
                        XmlNode::Element {
                            name,
                            attrs,
                            children,
                        },
                    );
                }
            }
            Ok(Event::Text(e)) => {
                let text = e.xml_content().map(|c| c.to_string()).unwrap_or_default();
                if !text.is_empty() {
                    push_node(&mut stack, &mut roots, XmlNode::Text(text));
                }
            }
            Ok(Event::CData(e)) => {
                let text = String::from_utf8_lossy(e.as_ref()).to_string();
                if !text.is_empty() {
                    push_node(&mut stack, &mut roots, XmlNode::Text(text));
                }
            }
            Ok(Event::Eof) => break,
            // Malformed input: stop rather than panic (adversarial safety);
            // whatever was parsed so far is still returned.
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    roots
}

fn read_attrs(e: &quick_xml::events::BytesStart) -> Vec<(String, String)> {
    e.attributes()
        .filter_map(|a| a.ok())
        .map(|a| {
            let key = String::from_utf8_lossy(a.key.as_ref()).to_string();
            let value = a
                .unescape_value()
                .map(|c| c.to_string())
                .unwrap_or_default();
            (key, value)
        })
        .collect()
}

fn push_node(stack: &mut [OpenElement], roots: &mut Vec<XmlNode>, node: XmlNode) {
    if let Some(top) = stack.last_mut() {
        top.2.push(node);
    } else {
        roots.push(node);
    }
}

fn children_of(node: &XmlNode) -> &[XmlNode] {
    match node {
        XmlNode::Element { children, .. } => children,
        XmlNode::Text(_) => &[],
    }
}

fn attr<'a>(node: &'a XmlNode, key: &str) -> Option<&'a str> {
    match node {
        XmlNode::Element { attrs, .. } => attrs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str()),
        XmlNode::Text(_) => None,
    }
}

fn find_all<'a>(nodes: &'a [XmlNode], name: &str, out: &mut Vec<&'a XmlNode>) {
    for n in nodes {
        if let XmlNode::Element {
            name: n_name,
            children,
            ..
        } = n
        {
            if n_name == name {
                out.push(n);
            }
            find_all(children, name, out);
        }
    }
}

fn find_child<'a>(nodes: &'a [XmlNode], name: &str) -> Option<&'a XmlNode> {
    nodes
        .iter()
        .find(|n| matches!(n, XmlNode::Element { name: n_name, .. } if n_name == name))
}

fn find_children<'a>(nodes: &'a [XmlNode], name: &str) -> Vec<&'a XmlNode> {
    nodes
        .iter()
        .filter(|n| matches!(n, XmlNode::Element { name: n_name, .. } if n_name == name))
        .collect()
}

/// Flatten all descendant text (ignoring element boundaries) — used only
/// for fields that are never expected to carry markup (numeric codes,
/// years, record numbers).
fn text_of(node: &XmlNode) -> String {
    let mut out = String::new();
    text_of_into(node, &mut out);
    out
}

fn text_of_into(node: &XmlNode, out: &mut String) {
    match node {
        XmlNode::Text(t) => out.push_str(t),
        XmlNode::Element { children, .. } => {
            for c in children {
                text_of_into(c, out);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// EndNote element vocabulary -> IR
// ---------------------------------------------------------------------------

/// Record-level children handled by dedicated logic below. Everything else
/// becomes a generic `misc` field rather than being silently dropped.
const SPECIAL_ELEMENTS: &[&str] = &[
    "ref-type",
    "rec-number",
    "label",
    "foreign-keys",
    "contributors",
    "titles",
    "dates",
    "volume",
    "number",
    "pages",
    "publisher",
    "pub-location",
    "isbn",
    "issn",
    "electronic-resource-num",
    "urls",
    "url",
    "abstract",
    "keywords",
    "notes",
];

fn convert_record(children: &[XmlNode]) -> Node {
    let mut fields = Vec::new();

    let ref_type_code = find_child(children, "ref-type")
        .map(text_of)
        .unwrap_or_default();
    let ref_type_name = find_child(children, "ref-type").and_then(|n| attr(n, "name"));

    let rec_number = find_child(children, "rec-number").map(text_of);
    let label = find_child(children, "label").map(text_of);

    if let Some(fk) = find_child(children, "foreign-keys") {
        for key in find_children(children_of(fk), "key") {
            let mut field = Node::new(node::BIBLIOGRAPHY_FIELD)
                .prop(prop::FIELD_ROLE, "misc")
                .prop("endnote:field", "foreign-keys/key")
                .child(Node::new(node::TEXT).prop(prop::CONTENT, text_of(key)));
            if let Some(app) = attr(key, "app") {
                field = field.prop("endnote:app", app.to_string());
            }
            if let Some(db_id) = attr(key, "db-id") {
                field = field.prop("endnote:db-id", db_id.to_string());
            }
            fields.push(field);
        }
    }

    if let Some(contributors) = find_child(children, "contributors") {
        let cc = children_of(contributors);
        if let Some(authors) = find_child(cc, "authors") {
            for author in find_children(children_of(authors), "author") {
                fields.push(person_field("author", author, "authors/author"));
            }
        }
        if let Some(authors) = find_child(cc, "secondary-authors") {
            for author in find_children(children_of(authors), "author") {
                fields.push(person_field("editor", author, "secondary-authors/author"));
            }
        }
        // Tertiary/subsidiary authors have no `field:role` equivalent —
        // raw-preserved as misc rather than dropped.
        for (wrapper, tag) in [
            ("tertiary-authors", "tertiary-authors/author"),
            ("subsidiary-authors", "subsidiary-authors/author"),
        ] {
            if let Some(authors) = find_child(cc, wrapper) {
                for author in find_children(children_of(authors), "author") {
                    fields.push(inline_field("misc", author, tag));
                }
            }
        }
    }

    if let Some(titles) = find_child(children, "titles") {
        let tc = children_of(titles);
        if let Some(title) = find_child(tc, "title") {
            fields.push(inline_field("title", title, "titles/title"));
        }
        if let Some(secondary) = find_child(tc, "secondary-title") {
            fields.push(inline_field(
                "container_title",
                secondary,
                "titles/secondary-title",
            ));
        }
        if let Some(tertiary) = find_child(tc, "tertiary-title") {
            fields.push(inline_field("misc", tertiary, "titles/tertiary-title"));
        }
    }

    if let Some(volume) = find_child(children, "volume") {
        fields.push(inline_field("volume", volume, "volume"));
    }
    if let Some(number) = find_child(children, "number") {
        fields.push(inline_field("issue", number, "number"));
    }

    if let Some(pages) = find_child(children, "pages") {
        push_pages(&mut fields, pages);
    }

    if let Some(publisher) = find_child(children, "publisher") {
        fields.push(inline_field("publisher", publisher, "publisher"));
    }
    if let Some(loc) = find_child(children, "pub-location") {
        fields.push(inline_field("publisher_location", loc, "pub-location"));
    }
    if let Some(isbn) = find_child(children, "isbn") {
        fields.push(identifier_field("isbn", &text_of(isbn), "isbn"));
    }
    if let Some(issn) = find_child(children, "issn") {
        fields.push(identifier_field("issn", &text_of(issn), "issn"));
    }
    if let Some(doi) = find_child(children, "electronic-resource-num") {
        fields.push(identifier_field(
            "doi",
            &text_of(doi),
            "electronic-resource-num",
        ));
    }

    if let Some(urls) = find_child(children, "urls") {
        let uc = children_of(urls);
        for (wrapper, tag) in [
            ("related-urls", "urls/related-urls/url"),
            ("pdf-urls", "urls/pdf-urls/url"),
        ] {
            if let Some(w) = find_child(uc, wrapper) {
                for url in find_children(children_of(w), "url") {
                    fields.push(identifier_field("url", &text_of(url), tag));
                }
            }
        }
    }
    // A bare top-level `<url>` (outside `<urls>`) also occurs in practice —
    // handled the same as `related-urls/url` rather than dropped.
    if let Some(url) = find_child(children, "url") {
        fields.push(identifier_field("url", &text_of(url), "url"));
    }

    if let Some(abstract_) = find_child(children, "abstract") {
        fields.push(inline_field("misc", abstract_, "abstract"));
    }
    if let Some(notes) = find_child(children, "notes") {
        fields.push(inline_field("misc", notes, "notes"));
    }
    if let Some(keywords) = find_child(children, "keywords") {
        for keyword in find_children(children_of(keywords), "keyword") {
            fields.push(inline_field("misc", keyword, "keywords/keyword"));
        }
    }

    // Everything else at the record level: generic fallback, nothing
    // silently dropped even for EndNote fields this reader doesn't know
    // about by name (EndNote's field vocabulary is large and
    // exporter-dependent — `custom1`..`custom7`, `research-notes`,
    // `remote-database-name`, `language`, `work-type`, `section`, ...).
    for child in children {
        if let XmlNode::Element { name, .. } = child
            && !SPECIAL_ELEMENTS.contains(&name.as_str())
        {
            fields.push(inline_field("misc", child, name));
        }
    }

    let cite_key = label
        .clone()
        .filter(|s| !s.is_empty())
        .or_else(|| rec_number.clone().filter(|s| !s.is_empty()))
        .unwrap_or_else(|| generate_cite_key(children));

    let mut entry_node = Node::new(node::BIBLIOGRAPHY_ENTRY)
        .prop("endnote:type", ref_type_code)
        .prop("endnote:key", cite_key);
    if let Some(name) = ref_type_name {
        entry_node = entry_node.prop("endnote:ref-type-name", name.to_string());
    }
    if let Some(rec_number) = rec_number.filter(|s| !s.is_empty()) {
        entry_node = entry_node.prop("endnote:rec-number", rec_number);
    }
    if let Some(label) = label.filter(|s| !s.is_empty()) {
        entry_node = entry_node.prop("endnote:label", label);
    }

    if let Some(dates) = find_child(children, "dates") {
        let dc = children_of(dates);
        if let Some(year) = find_child(dc, "year") {
            let text = text_of(year);
            if let Ok(y) = text.trim().parse::<i64>() {
                let mut map = HashMap::new();
                map.insert("year".to_string(), PropValue::Int(y));
                entry_node = entry_node.prop(prop::DATE, PropValue::Map(map));
            } else if !text.is_empty() {
                fields.push(inline_field("misc", year, "dates/year"));
            }
        }
        if let Some(pub_dates) = find_child(dc, "pub-dates")
            && let Some(date) = find_child(children_of(pub_dates), "date")
        {
            // A free-text pub-date (e.g. "Jan 15") has no unambiguous
            // year/month/day parse without guessing a locale's month-name
            // convention — kept as a misc field instead.
            fields.push(inline_field("misc", date, "dates/pub-dates/date"));
        }
    }

    entry_node.children(fields)
}

fn generate_cite_key(children: &[XmlNode]) -> String {
    let author = find_child(children, "contributors")
        .and_then(|c| find_child(children_of(c), "authors"))
        .and_then(|a| find_children(children_of(a), "author").into_iter().next())
        .map(text_of);
    let year = find_child(children, "dates")
        .and_then(|d| find_child(children_of(d), "year"))
        .map(text_of);

    let author_part = author
        .map(|a| {
            a.split(',')
                .next()
                .unwrap_or(&a)
                .chars()
                .filter(|c| c.is_alphanumeric())
                .take(8)
                .collect::<String>()
                .to_lowercase()
        })
        .unwrap_or_else(|| "unknown".to_string());

    format!("{}{}", author_part, year.unwrap_or_default())
}

/// Build a `bibliography_field` from an author `XmlNode`, converting
/// `<style>` markup (if any — rare but valid inside `<author>`) the same
/// way as any other field.
fn person_field(role: &str, node: &XmlNode, tag: &str) -> Node {
    inline_field(role, node, tag)
}

/// Build a `bibliography_field` whose children are `node`'s content
/// converted through `convert_inline_children` — this is where `<style>`
/// markup becomes real `emphasis`/`strong`/... nodes instead of being
/// flattened to plain text, the one place among these four formats where
/// the field-node design (vs. a flat string) actually earns its keep.
fn inline_field(role: &str, node: &XmlNode, tag: &str) -> Node {
    Node::new(node::BIBLIOGRAPHY_FIELD)
        .prop(prop::FIELD_ROLE, role)
        .prop("endnote:field", tag)
        .children(convert_inline_children(children_of(node)))
}

fn identifier_field(scheme: &str, text: &str, tag: &str) -> Node {
    let mut node = Node::new(node::BIBLIOGRAPHY_FIELD)
        .prop(prop::FIELD_ROLE, "identifier")
        .prop(prop::FIELD_SCHEME, scheme)
        .prop("endnote:field", tag);
    if !text.is_empty() {
        node = node.child(Node::new(node::TEXT).prop(prop::CONTENT, text.to_string()));
    }
    node
}

/// Split `<pages>` into `page_first`/`page_last` when the content is plain,
/// unambiguous digits on both sides of a separator (mirrors the
/// bibtex/csl-json/docbook page-splitting logic); anything else (markup,
/// non-numeric labels, multiple separators) is kept whole as a `misc`
/// field, `<style>` markup included, rather than guessing.
fn push_pages(fields: &mut Vec<Node>, pages: &XmlNode) {
    let inline = convert_inline_children(children_of(pages));
    if let [single] = inline.as_slice()
        && single.kind.as_str() == node::TEXT
        && let Some(text) = single.props.get_str(prop::CONTENT)
    {
        let t = text.trim();
        if !t.is_empty() && t.chars().all(|c| c.is_ascii_digit()) {
            fields.push(text_field("page_first", t, "pages"));
            return;
        }
        for sep in ["--", "\u{2013}", "\u{2014}", "-"] {
            if let Some((first, last)) = t.split_once(sep) {
                let first = first.trim();
                let last = last.trim();
                if !first.is_empty()
                    && !last.is_empty()
                    && first.chars().all(|c| c.is_ascii_digit())
                    && last.chars().all(|c| c.is_ascii_digit())
                {
                    fields.push(text_field("page_first", first, "pages"));
                    fields.push(text_field("page_last", last, "pages"));
                    return;
                }
            }
        }
    }
    fields.push(
        Node::new(node::BIBLIOGRAPHY_FIELD)
            .prop(prop::FIELD_ROLE, "misc")
            .prop("endnote:field", "pages")
            .children(inline),
    );
}

fn text_field(role: &str, text: &str, tag: &str) -> Node {
    Node::new(node::BIBLIOGRAPHY_FIELD)
        .prop(prop::FIELD_ROLE, role)
        .prop("endnote:field", tag)
        .child(Node::new(node::TEXT).prop(prop::CONTENT, text.to_string()))
}

/// Convert a field's XML children to inline IR nodes. Plain text becomes a
/// `TEXT` node; `<style face="...">` becomes the matching inline markup
/// node (`emphasis`/`strong`/`underline`/`superscript`/`subscript`) wrapped
/// around its own (recursively converted) content — `face="normal"` (or
/// any unrecognized/absent face) passes its content through unwrapped.
/// Any other unexpected element is also passed through transparently
/// (its content flattened in, rather than dropped) since EndNote's actual
/// exporters occasionally nest other presentation wrappers here.
fn convert_inline_children(children: &[XmlNode]) -> Vec<Node> {
    let mut out = Vec::new();
    for child in children {
        match child {
            XmlNode::Text(t) => {
                if !t.is_empty() {
                    out.push(Node::new(node::TEXT).prop(prop::CONTENT, t.clone()));
                }
            }
            XmlNode::Element { name, .. } if name == "style" => {
                out.extend(convert_style(child));
            }
            XmlNode::Element { children, .. } => {
                out.extend(convert_inline_children(children));
            }
        }
    }
    out
}

fn convert_style(style: &XmlNode) -> Vec<Node> {
    let inner = convert_inline_children(children_of(style));
    let face = attr(style, "face").unwrap_or("normal").to_lowercase();
    let wrap = |kind: &str, children: Vec<Node>| vec![Node::new(kind).children(children)];
    if face.contains("bold") && face.contains("italic") {
        wrap(
            node::STRONG,
            wrap(node::EMPHASIS, inner).into_iter().collect(),
        )
    } else if face.contains("bold") {
        wrap(node::STRONG, inner)
    } else if face.contains("italic") {
        wrap(node::EMPHASIS, inner)
    } else if face.contains("underline") {
        wrap(node::UNDERLINE, inner)
    } else if face.contains("superscript") {
        wrap(node::SUPERSCRIPT, inner)
    } else if face.contains("subscript") {
        wrap(node::SUBSCRIPT, inner)
    } else {
        inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<xml>
  <records>
    <record>
      <ref-type>Journal Article</ref-type>
      <contributors>
        <authors>
          <author>Smith, John</author>
        </authors>
      </contributors>
      <titles>
        <title>A Great Paper</title>
        <secondary-title>Nature</secondary-title>
      </titles>
      <dates>
        <year>2020</year>
      </dates>
    </record>
  </records>
</xml>"#;

        let result = parse(xml).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
        let entry = &doc.content.children[0].children[0];
        assert_eq!(entry.kind.as_str(), node::BIBLIOGRAPHY_ENTRY);
    }

    #[test]
    fn test_parse_empty() {
        let xml = r#"<?xml version="1.0"?><xml><records></records></xml>"#;
        let result = parse(xml).unwrap();
        let doc = result.value;
        assert!(doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_multiple_authors() {
        let xml = r#"<?xml version="1.0"?>
<xml>
  <records>
    <record>
      <ref-type name="Journal Article">17</ref-type>
      <contributors>
        <authors>
          <author>Smith, John</author>
          <author>Doe, Jane</author>
        </authors>
      </contributors>
      <titles>
        <title>Collaborative Work</title>
      </titles>
      <dates>
        <year>2021</year>
      </dates>
    </record>
  </records>
</xml>"#;

        let result = parse(xml).unwrap();
        let doc = result.value;
        let entry = &doc.content.children[0].children[0];
        let authors: Vec<_> = entry
            .children
            .iter()
            .filter(|c| c.props.get_str(prop::FIELD_ROLE) == Some("author"))
            .collect();
        assert_eq!(authors.len(), 2);
    }

    #[test]
    fn test_style_markup() {
        let xml = r#"<?xml version="1.0"?>
<xml>
  <records>
    <record>
      <ref-type name="Journal Article">17</ref-type>
      <titles>
        <title><style face="normal">A </style><style face="italic">Great</style><style face="normal"> Paper</style></title>
      </titles>
    </record>
  </records>
</xml>"#;
        let result = parse(xml).unwrap();
        let doc = result.value;
        let entry = &doc.content.children[0].children[0];
        let title = entry
            .children
            .iter()
            .find(|c| c.props.get_str(prop::FIELD_ROLE) == Some("title"))
            .unwrap();
        assert_eq!(title.children.len(), 3);
        assert_eq!(title.children[0].kind.as_str(), node::TEXT);
        assert_eq!(title.children[1].kind.as_str(), node::EMPHASIS);
        assert_eq!(
            title.children[1].children[0].props.get_str(prop::CONTENT),
            Some("Great")
        );
        assert_eq!(title.children[2].kind.as_str(), node::TEXT);
    }
}
