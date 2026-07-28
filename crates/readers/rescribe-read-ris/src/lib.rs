//! RIS (Research Information Systems) reader for rescribe.
//!
//! Parses RIS bibliography files into rescribe's document IR, using the
//! `bibliography`/`bibliography_entry`/`bibliography_field` node kinds (see
//! `rescribe_std::node` and ADR 0005 in the rescribe repo). RIS is a
//! standardized tag format for bibliographic citations; all tag parsing
//! lives in the standalone `ris` crate (`crates/formats/ris`) — this crate
//! is a thin IR translator only.
//!
//! # Example
//!
//! ```
//! use rescribe_read_ris::parse;
//!
//! let ris = "TY  - JOUR\nAU  - Smith, John\nTI  - A Great Paper\nER  -";
//! let result = parse(ris).unwrap();
//! let doc = result.value;
//! ```

use rescribe_core::{ConversionResult, Document, Node, ParseError, Properties};
use rescribe_std::{PropValue, node, prop};
use std::collections::HashMap;

/// Parse RIS text into a document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &Default::default())
}

/// Parse RIS text with options.
pub fn parse_with_options(
    input: &str,
    _options: &rescribe_core::ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let (ris_doc, _diags) = ris::parse(input);

    let entries: Vec<Node> = ris_doc.entries.iter().map(entry_to_node).collect();

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

/// Tags consumed by dedicated logic below (author/editor, title/container,
/// publisher, volume/issue, pages, identifiers, dates). Everything else in
/// `entry.fields` goes through the generic role-lookup loop.
const SPECIAL_TAGS: &[&str] = &[
    "AU", "A2", "TI", "T1", "JO", "JF", "JA", "J1", "J2", "T2", "PB", "CY", "ET", "VL", "IS", "SP",
    "EP", "DO", "UR", "SN", "PY", "Y1", "DA",
];

/// Known RIS tag -> `field:role` mapping for tags with no dedicated
/// handling. Anything not listed here becomes a `misc` field (still
/// round-trippable via the always-present `ris:field` property).
fn field_role(tag: &str) -> &'static str {
    match tag {
        "TI" | "T1" => "title",
        "JO" | "JF" | "JA" | "J1" | "J2" | "T2" => "container_title",
        "PB" => "publisher",
        "CY" => "publisher_location",
        "ET" => "edition",
        "VL" => "volume",
        "IS" => "issue",
        _ => "misc",
    }
}

fn entry_to_node(entry: &ris::RisEntry) -> Node {
    let cite_key = entry.generate_cite_key();

    let mut fields = Vec::new();

    for author in entry.get_all("AU") {
        fields.push(text_field("author", author, "AU"));
    }
    for editor in entry.get_all("A2") {
        fields.push(text_field("editor", editor, "A2"));
    }

    // Title: first of TI/T1 (RIS uses either interchangeably); the other,
    // if also present and different, is kept as a misc field rather than
    // silently dropped.
    push_primary_then_alt(&mut fields, entry, "TI", "title", "T1");

    // Container title: JO/JF/JA/J1/J2 are full/abbreviated journal name
    // variants, T2 is the secondary title (book title for a chapter,
    // conference name for a paper) — RIS gives no way to tell which of
    // several present variants is "the" container title, so every one
    // present becomes its own sibling `container_title` field rather than
    // guessing which single one wins.
    for tag in ["JO", "JF", "JA", "J1", "J2", "T2"] {
        if let Some(v) = entry.get_first(tag) {
            fields.push(text_field("container_title", v, tag));
        }
    }

    if let Some(pb) = entry.get_first("PB") {
        fields.push(text_field("publisher", pb, "PB"));
    }
    if let Some(cy) = entry.get_first("CY") {
        fields.push(text_field("publisher_location", cy, "CY"));
    }
    if let Some(et) = entry.get_first("ET") {
        fields.push(text_field("edition", et, "ET"));
    }
    if let Some(vl) = entry.get_first("VL") {
        fields.push(text_field("volume", vl, "VL"));
    }
    if let Some(is) = entry.get_first("IS") {
        fields.push(text_field("issue", is, "IS"));
    }

    // SP/EP (start/end page) are already isolated by RIS itself, unlike
    // BibTeX/CSL-JSON's combined `pages`/`page` string — no splitting
    // needed, just a direct role mapping each.
    if let Some(sp) = entry.get_first("SP") {
        fields.push(text_field("page_first", sp, "SP"));
    }
    if let Some(ep) = entry.get_first("EP") {
        fields.push(text_field("page_last", ep, "EP"));
    }

    if let Some(doi) = entry.get_first("DO") {
        fields.push(identifier_field("doi", doi, "DO"));
    }
    if let Some(url) = entry.get_first("UR") {
        fields.push(identifier_field("url", url, "UR"));
    }
    if let Some(sn) = entry.get_first("SN") {
        // RIS's SN tag is used for both ISBN and ISSN depending on entry
        // type, with no way to tell which from the tag alone — naming the
        // scheme after the RIS tag itself avoids guessing which one it is.
        fields.push(identifier_field("sn", sn, "SN"));
    }

    let mut entry_node = Node::new(node::BIBLIOGRAPHY_ENTRY)
        .prop("ris:type", entry.entry_type.clone())
        .prop("ris:key", cite_key);

    // Date: PY (Publication Year) is preferred as the canonical date; Y1
    // (Primary Date, used interchangeably with PY by some exporters) is
    // folded in only when PY is absent, and kept as a misc field instead
    // when both are present and might differ. DA (a distinct "Date" tag,
    // e.g. an access/created date in some RIS dialects) always stays a
    // misc field since it isn't necessarily the publication date.
    let py = entry.get_first("PY");
    let y1 = entry.get_first("Y1");
    match (py, y1) {
        (Some(text), other) => {
            entry_node = apply_date(entry_node, &mut fields, text, "PY");
            if let Some(other) = other
                && other != text
            {
                fields.push(text_field("misc", other, "Y1"));
            }
        }
        (None, Some(text)) => entry_node = apply_date(entry_node, &mut fields, text, "Y1"),
        (None, None) => {}
    }
    if let Some(da) = entry.get_first("DA") {
        fields.push(text_field("misc", da, "DA"));
    }

    // Everything else: generic role lookup. `entry.fields` is a HashMap,
    // so iterate tag names sorted for deterministic output.
    let mut tags: Vec<&String> = entry.fields.keys().collect();
    tags.sort();
    for tag in tags {
        if SPECIAL_TAGS.contains(&tag.as_str()) {
            continue;
        }
        let role = field_role(tag);
        for value in entry.get_all(tag) {
            fields.push(text_field(role, value, tag));
        }
    }

    entry_node.children(fields)
}

/// Push a field for `primary_tag` if present; if `alt_tag` is also present
/// with a *different* value, keep it too (as a misc field) instead of
/// letting the alternate silently disappear.
fn push_primary_then_alt(
    fields: &mut Vec<Node>,
    entry: &ris::RisEntry,
    primary_tag: &str,
    role: &str,
    alt_tag: &str,
) {
    let primary = entry.get_first(primary_tag);
    let alt = entry.get_first(alt_tag);
    match (primary, alt) {
        (Some(p), a) => {
            fields.push(text_field(role, p, primary_tag));
            if let Some(a) = a
                && a != p
            {
                fields.push(text_field("misc", a, alt_tag));
            }
        }
        (None, Some(a)) => fields.push(text_field(role, a, alt_tag)),
        (None, None) => {}
    }
}

fn text_field(role: &str, text: &str, tag: &str) -> Node {
    let mut node = Node::new(node::BIBLIOGRAPHY_FIELD)
        .prop(prop::FIELD_ROLE, role)
        .prop("ris:field", tag);
    if !text.is_empty() {
        node = node.child(Node::new(node::TEXT).prop(prop::CONTENT, text.to_string()));
    }
    node
}

fn identifier_field(scheme: &str, text: &str, tag: &str) -> Node {
    text_field("identifier", text, tag).prop(prop::FIELD_SCHEME, scheme)
}

/// Parse RIS's `YYYY/MM/DD/other` date format (only the unambiguous
/// `YYYY`, `YYYY/MM`, `YYYY/MM/DD` prefixes; the trailing free-text
/// `other` field, if present, is dropped from the structured date — it
/// isn't part of year/month/day) into `prop::DATE` on `entry_node`. Falls
/// back to pushing a `misc` field holding the raw text (tagged `source_tag`
/// — `"PY"` or `"Y1"`, whichever supplied `text`) when the year isn't a
/// clean 4-digit number, rather than guessing.
fn apply_date(entry_node: Node, fields: &mut Vec<Node>, text: &str, source_tag: &str) -> Node {
    let parts: Vec<&str> = text.split('/').collect();
    let year = parts
        .first()
        .filter(|y| y.len() == 4 && y.chars().all(|c| c.is_ascii_digit()))
        .and_then(|y| y.parse::<i64>().ok());
    let Some(year) = year else {
        fields.push(text_field("misc", text, source_tag));
        return entry_node;
    };
    let month = parts
        .get(1)
        .filter(|m| !m.is_empty())
        .and_then(|m| m.parse::<i64>().ok())
        .filter(|m| (1..=12).contains(m));
    let day = month.and_then(|_| {
        parts
            .get(2)
            .filter(|d| !d.is_empty())
            .and_then(|d| d.parse::<i64>().ok())
            .filter(|d| (1..=31).contains(d))
    });
    let mut map = HashMap::new();
    map.insert("year".to_string(), PropValue::Int(year));
    if let Some(month) = month {
        map.insert("month".to_string(), PropValue::Int(month));
    }
    if let Some(day) = day {
        map.insert("day".to_string(), PropValue::Int(day));
    }
    entry_node.prop(prop::DATE, PropValue::Map(map))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_article() {
        let ris = r#"TY  - JOUR
AU  - Smith, John
AU  - Doe, Jane
TI  - A Great Paper
JO  - Nature
PY  - 2020
VL  - 123
SP  - 45
EP  - 67
DO  - 10.1234/nature.2020
ER  -"#;

        let result = parse(ris).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
        let entry = &doc.content.children[0].children[0];
        assert_eq!(entry.props.get_str("ris:type"), Some("JOUR"));
        let authors: Vec<_> = entry
            .children
            .iter()
            .filter(|c| c.props.get_str(prop::FIELD_ROLE) == Some("author"))
            .collect();
        assert_eq!(authors.len(), 2);
    }

    #[test]
    fn test_parse_book() {
        let ris = r#"TY  - BOOK
AU  - Knuth, Donald E.
TI  - The Art of Computer Programming
PY  - 1997
ER  -"#;

        let result = parse(ris).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_multiple() {
        let ris = r#"TY  - JOUR
AU  - First, Author
TI  - First Paper
ER  -
TY  - JOUR
AU  - Second, Author
TI  - Second Paper
ER  -"#;

        let result = parse(ris).unwrap();
        let doc = result.value;
        let bibliography = &doc.content.children[0];
        assert_eq!(bibliography.children.len(), 2);
    }

    #[test]
    fn test_parse_empty() {
        let ris = "";
        let result = parse(ris).unwrap();
        let doc = result.value;
        assert!(doc.content.children.is_empty());
    }

    #[test]
    fn test_date_structured() {
        let ris = "TY  - JOUR\nPY  - 2020/03/15/\nER  -";
        let result = parse(ris).unwrap();
        let doc = result.value;
        let entry = &doc.content.children[0].children[0];
        let Some(rescribe_core::PropValue::Map(map)) = entry.props.get(prop::DATE) else {
            panic!("expected structured date");
        };
        assert_eq!(map.get("year"), Some(&rescribe_core::PropValue::Int(2020)));
        assert_eq!(map.get("month"), Some(&rescribe_core::PropValue::Int(3)));
        assert_eq!(map.get("day"), Some(&rescribe_core::PropValue::Int(15)));
    }
}
