//! CSL JSON writer half of `rescribe-fmt-csl-json`.
//!
//! Serializes `bibliography`/`bibliography_entry`/`bibliography_field` IR
//! nodes (see `rescribe_std::node` and ADR 0005 in the rescribe repo) to
//! CSL JSON (Citation Style Language JSON).

use rescribe_core::{ConversionResult, Document, EmitError, Node};
use rescribe_std::{node, prop};
use serde::Serialize;
use serde_json::Value;

/// Emit a document to CSL JSON.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let mut items = Vec::new();
    let warnings = Vec::new();

    collect_items(&doc.content, &mut items);

    let json = serde_json::to_string_pretty(&items)
        .map_err(|e| EmitError::Io(std::io::Error::other(format!("CSL JSON error: {}", e))))?;

    Ok(ConversionResult::with_warnings(json.into_bytes(), warnings))
}

fn collect_items(node: &Node, items: &mut Vec<CslItem>) {
    if node.kind.as_str() == node::BIBLIOGRAPHY_ENTRY {
        items.push(extract_from_entry(node));
        return;
    }

    // Legacy shapes, kept for backwards compatibility with documents built
    // by hand or by an older reader version (not produced by
    // this crate's `read` module/`rescribe-fmt-bibtex` any more).
    if node.kind.as_str() == "csl:item"
        && let Some(item) = extract_csl_item(node)
    {
        items.push(item);
        return;
    }
    if node.kind.as_str() == "bibtex:entry"
        && let Some(item) = extract_bibtex_item(node)
    {
        items.push(item);
        return;
    }
    if node.kind.as_str() == node::DEFINITION_DESC {
        if let Some(id) = node.props.get_str("csl:id") {
            items.push(extract_from_definition(node, id));
            return;
        }
        if let Some(key) = node.props.get_str("bibtex:key") {
            items.push(extract_from_definition(node, key));
            return;
        }
    }

    for child in &node.children {
        collect_items(child, items);
    }
}

/// Extract a `CslItem` from a `bibliography_entry` node (see
/// this crate's `read` module's `convert_item`). `csl:field` on each
/// `bibliography_field` child names the exact source CSL-JSON variable;
/// `field:role` is the fallback for a field built by a non-CSL-JSON
/// producer (a cross-format conversion into CSL JSON). A `page_first`/
/// `page_last` pair recombines into one `page` field; `prop::DATE` becomes
/// `issued.date-parts`; a `misc` field whose `csl:field` is `"issued"` (the
/// unparsed-literal-date fallback, see this crate's `read` module's
/// `convert_date`) reconstructs `issued.literal` instead of leaking into
/// the catch-all `extra` bucket, since `issued` must stay an object per the
/// CSL-JSON schema. Every other unrecognized field name/value round-trips
/// through `extra` (a flattened JSON object) exactly as read.
fn extract_from_entry(node: &Node) -> CslItem {
    let id = node
        .props
        .get_str("csl:id")
        .unwrap_or("unknown")
        .to_string();
    let item_type = node.props.get_str("csl:type").map(String::from);

    let mut title = None;
    let mut authors = Vec::new();
    let mut editors = Vec::new();
    let mut container_title = None;
    let mut publisher = None;
    let mut publisher_place = None;
    let mut edition = None;
    let mut volume = None;
    let mut issue = None;
    let mut page = None;
    let mut doi = None;
    let mut url = None;
    let mut isbn = None;
    let mut issn = None;
    let mut issued_literal = None;
    let mut extra = serde_json::Map::new();

    let mut iter = node.children.iter().peekable();
    while let Some(child) = iter.next() {
        if child.kind.as_str() != node::BIBLIOGRAPHY_FIELD {
            continue;
        }
        let role = child.props.get_str(prop::FIELD_ROLE).unwrap_or("misc");
        let field_name = child.props.get_str("csl:field").unwrap_or(role).to_string();
        match role {
            "author" => authors.push(name_from_field(child)),
            "editor" => editors.push(name_from_field(child)),
            "title" => title = Some(flatten_field_text(child)),
            "container_title" => container_title = Some(flatten_field_text(child)),
            "publisher" => publisher = Some(flatten_field_text(child)),
            "publisher_location" => publisher_place = Some(flatten_field_text(child)),
            "edition" => edition = Some(flatten_field_text(child)),
            "volume" => volume = Some(flatten_field_text(child)),
            "issue" => issue = Some(flatten_field_text(child)),
            "page_first"
                if iter
                    .peek()
                    .and_then(|next| next.props.get_str(prop::FIELD_ROLE))
                    == Some("page_last") =>
            {
                let last = iter.next().unwrap();
                let first_text = flatten_field_text(child);
                let last_text = flatten_field_text(last);
                page = Some(format!("{first_text}-{last_text}"));
            }
            "identifier" => {
                let text = flatten_field_text(child);
                match child.props.get_str(prop::FIELD_SCHEME) {
                    Some("doi") => doi = Some(text),
                    Some("isbn") => isbn = Some(text),
                    Some("issn") => issn = Some(text),
                    _ => url = Some(text),
                }
            }
            _ if field_name == "issued" => {
                issued_literal = Some(flatten_field_text(child));
            }
            _ => {
                let text = flatten_field_text(child);
                if !text.is_empty() {
                    extra.insert(field_name, Value::String(text));
                }
            }
        }
    }

    let issued = match node.props.get(prop::DATE) {
        Some(rescribe_core::PropValue::Map(map)) => Some(CslDate {
            date_parts: Some(vec![date_parts_from_map(map)]),
            literal: None,
        }),
        _ => issued_literal.map(|literal| CslDate {
            date_parts: None,
            literal: Some(literal),
        }),
    };

    CslItem {
        id,
        item_type,
        title,
        author: (!authors.is_empty()).then_some(authors),
        editor: (!editors.is_empty()).then_some(editors),
        issued,
        container_title,
        publisher,
        publisher_place,
        edition,
        volume,
        issue,
        page,
        doi,
        url,
        isbn,
        issn,
        extra,
    }
}

fn date_parts_from_map(
    map: &std::collections::HashMap<String, rescribe_core::PropValue>,
) -> Vec<i64> {
    let as_int = |key: &str| match map.get(key) {
        Some(rescribe_core::PropValue::Int(i)) => Some(*i),
        _ => None,
    };
    let mut parts = Vec::new();
    if let Some(y) = as_int("year") {
        parts.push(y);
    }
    if let Some(m) = as_int("month") {
        parts.push(m);
    }
    if let Some(d) = as_int("day") {
        parts.push(d);
    }
    parts
}

/// Reconstruct a `CslName` from a `bibliography_field`'s TEXT children,
/// using `csl:name-part` (set by this crate's `read` module's `name_field`)
/// to tell `literal`/`given`/`family` apart — a bare child count is
/// otherwise ambiguous.
fn name_from_field(node: &Node) -> CslName {
    let mut given = None;
    let mut family = None;
    let mut literal = None;
    for child in &node.children {
        if child.kind.as_str() != node::TEXT {
            continue;
        }
        let Some(text) = child.props.get_str(prop::CONTENT) else {
            continue;
        };
        match child.props.get_str("csl:name-part") {
            Some("given") => given = Some(text.to_string()),
            Some("family") => family = Some(text.to_string()),
            Some("literal") => literal = Some(text.to_string()),
            // No marker — a field built by a non-CSL-JSON producer.
            // Fall back to positional "given family" like the old writer.
            _ => match (&given, &family) {
                (None, _) => given = Some(text.to_string()),
                (Some(_), None) => family = Some(text.to_string()),
                _ => {}
            },
        }
    }
    CslName {
        family,
        given,
        literal,
    }
}

/// Concatenate a field's descendant `TEXT` node content (depth-first).
fn flatten_field_text(node: &Node) -> String {
    let mut out = String::new();
    flatten_field_text_into(node, &mut out);
    out
}

fn flatten_field_text_into(node: &Node, out: &mut String) {
    if node.kind.as_str() == node::TEXT
        && let Some(content) = node.props.get_str(prop::CONTENT)
    {
        out.push_str(content);
    }
    for child in &node.children {
        flatten_field_text_into(child, out);
    }
}

// ---------------------------------------------------------------------------
// Legacy shapes (backwards compatibility)
// ---------------------------------------------------------------------------

fn extract_csl_item(node: &Node) -> Option<CslItem> {
    for child in &node.children {
        if child.kind.as_str() == node::DEFINITION_DESC
            && let Some(id) = child.props.get_str("csl:id")
        {
            return Some(extract_from_definition(child, id));
        }
    }
    None
}

fn extract_bibtex_item(node: &Node) -> Option<CslItem> {
    for child in &node.children {
        if child.kind.as_str() == node::DEFINITION_DESC
            && let Some(key) = child.props.get_str("bibtex:key")
        {
            return Some(extract_from_definition(child, key));
        }
    }
    None
}

fn extract_from_definition(node: &Node, id: &str) -> CslItem {
    let item_type = node
        .props
        .get_str("csl:type")
        .or_else(|| node.props.get_str("bibtex:type"))
        .map(map_type_to_csl)
        .unwrap_or_else(|| "article".to_string());

    let mut title = None;
    let mut authors = Vec::new();
    let mut container_title = None;
    let mut doi = None;
    let mut url = None;

    extract_content(
        node,
        &mut title,
        &mut authors,
        &mut container_title,
        &mut doi,
        &mut url,
    );

    CslItem {
        id: id.to_string(),
        item_type: Some(item_type),
        title,
        author: if authors.is_empty() {
            None
        } else {
            Some(authors)
        },
        container_title,
        doi,
        url,
        editor: None,
        issued: None,
        publisher: None,
        publisher_place: None,
        edition: None,
        volume: None,
        issue: None,
        page: None,
        isbn: None,
        issn: None,
        extra: serde_json::Map::new(),
    }
}

fn extract_content(
    node: &Node,
    title: &mut Option<String>,
    authors: &mut Vec<CslName>,
    _container_title: &mut Option<String>,
    doi: &mut Option<String>,
    url: &mut Option<String>,
) {
    if node.kind.as_str() == node::EMPHASIS && title.is_none() {
        *title = Some(collect_text(node));
    }

    if node.kind.as_str() == node::STRONG {
        let text = collect_text(node);
        for author in text.split(';') {
            let author = author.trim();
            if !author.is_empty() {
                authors.push(CslName::from_string(author));
            }
        }
    }

    if node.kind.as_str() == node::LINK
        && let Some(link_url) = node.props.get_str(prop::URL)
    {
        if link_url.contains("doi.org") {
            if let Some(d) = link_url.strip_prefix("https://doi.org/") {
                *doi = Some(d.to_string());
            }
        } else if url.is_none() {
            *url = Some(link_url.to_string());
        }
    }

    for child in &node.children {
        extract_content(child, title, authors, _container_title, doi, url);
    }
}

fn collect_text(node: &Node) -> String {
    let mut text = String::new();
    collect_text_recursive(node, &mut text);
    text
}

fn collect_text_recursive(node: &Node, text: &mut String) {
    if node.kind.as_str() == node::TEXT
        && let Some(content) = node.props.get_str(prop::CONTENT)
    {
        text.push_str(content);
    }
    for child in &node.children {
        collect_text_recursive(child, text);
    }
}

fn map_type_to_csl(bibtex_type: &str) -> String {
    match bibtex_type {
        "article" => "article-journal",
        "book" => "book",
        "inbook" => "chapter",
        "incollection" => "chapter",
        "inproceedings" => "paper-conference",
        "conference" => "paper-conference",
        "phdthesis" => "thesis",
        "mastersthesis" => "thesis",
        "techreport" => "report",
        "manual" => "book",
        "misc" => "article",
        "unpublished" => "manuscript",
        "online" => "webpage",
        other => other,
    }
    .to_string()
}

#[derive(Debug, Serialize)]
struct CslItem {
    id: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    item_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    author: Option<Vec<CslName>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    editor: Option<Vec<CslName>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    issued: Option<CslDate>,
    #[serde(rename = "container-title", skip_serializing_if = "Option::is_none")]
    container_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    publisher: Option<String>,
    #[serde(rename = "publisher-place", skip_serializing_if = "Option::is_none")]
    publisher_place: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    edition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    volume: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    issue: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    page: Option<String>,
    #[serde(rename = "DOI", skip_serializing_if = "Option::is_none")]
    doi: Option<String>,
    #[serde(rename = "URL", skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(rename = "ISBN", skip_serializing_if = "Option::is_none")]
    isbn: Option<String>,
    #[serde(rename = "ISSN", skip_serializing_if = "Option::is_none")]
    issn: Option<String>,
    #[serde(flatten)]
    extra: serde_json::Map<String, Value>,
}

#[derive(Debug, Serialize)]
struct CslName {
    #[serde(skip_serializing_if = "Option::is_none")]
    family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    given: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    literal: Option<String>,
}

impl CslName {
    fn from_string(s: &str) -> Self {
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.len() >= 2 {
            let (given, family) = parts.split_at(parts.len() - 1);
            CslName {
                given: Some(given.join(" ")),
                family: Some(family[0].to_string()),
                literal: None,
            }
        } else if parts.len() == 1 {
            CslName {
                family: Some(parts[0].to_string()),
                given: None,
                literal: None,
            }
        } else {
            CslName {
                literal: Some(s.to_string()),
                family: None,
                given: None,
            }
        }
    }
}

#[derive(Debug, Serialize)]
struct CslDate {
    #[serde(rename = "date-parts", skip_serializing_if = "Option::is_none")]
    date_parts: Option<Vec<Vec<i64>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    literal: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rescribe_core::{PropValue, Properties};
    use rescribe_std::node as std_node;

    #[test]
    fn test_emit_empty() {
        let doc = Document {
            content: Node::new(node::DOCUMENT),
            resources: Default::default(),
            metadata: Properties::new(),
            source: None,
        };

        let result = emit(&doc).unwrap();
        let json = String::from_utf8(result.value).unwrap();
        assert_eq!(json.trim(), "[]");
    }

    fn name_field(role: &str, given: &str, family: &str) -> Node {
        Node::new(std_node::BIBLIOGRAPHY_FIELD)
            .prop(prop::FIELD_ROLE, role)
            .prop("csl:field", role)
            .child(
                Node::new(std_node::TEXT)
                    .prop(prop::CONTENT, given)
                    .prop("csl:name-part", "given"),
            )
            .child(
                Node::new(std_node::TEXT)
                    .prop(prop::CONTENT, family)
                    .prop("csl:name-part", "family"),
            )
    }

    #[test]
    fn test_emit_entry() {
        let mut map = std::collections::HashMap::new();
        map.insert("year".to_string(), PropValue::Int(2020));
        let entry = Node::new(std_node::BIBLIOGRAPHY_ENTRY)
            .prop("csl:id", "smith2020")
            .prop("csl:type", "article-journal")
            .prop(prop::DATE, PropValue::Map(map))
            .child(name_field("author", "John", "Smith"));
        let doc = Document {
            content: Node::new(node::DOCUMENT).child(entry),
            resources: Default::default(),
            metadata: Properties::new(),
            source: None,
        };
        let result = emit(&doc).unwrap();
        let json = String::from_utf8(result.value).unwrap();
        assert!(json.contains("\"id\": \"smith2020\""));
        assert!(json.contains("\"type\": \"article-journal\""));
        assert!(json.contains("\"family\": \"Smith\""));
        assert!(json.contains("\"date-parts\""));
    }
}
