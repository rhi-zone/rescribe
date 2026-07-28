//! CSL JSON reader for rescribe.
//!
//! Parses CSL JSON (Citation Style Language JSON) into rescribe's document
//! IR, using the `bibliography`/`bibliography_entry`/`bibliography_field`
//! node kinds (see `rescribe_std::node` and ADR 0005 in the rescribe repo).
//!
//! # Example
//!
//! ```
//! use rescribe_read_csl_json::parse;
//!
//! let csl = r#"[{
//!   "id": "smith2020",
//!   "type": "article-journal",
//!   "title": "A Great Paper",
//!   "author": [{"family": "Smith", "given": "John"}],
//!   "issued": {"date-parts": [[2020]]}
//! }]"#;
//!
//! let result = parse(csl).unwrap();
//! let doc = result.value;
//! ```

use rescribe_core::{ConversionResult, Document, FidelityWarning, Node, ParseError, Properties};
use rescribe_std::{PropValue, node, prop};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

/// Parse CSL JSON text into a document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    let items: Vec<CslItem> = serde_json::from_str(input)
        .map_err(|e| ParseError::Invalid(format!("CSL JSON parse error: {}", e)))?;

    let mut warnings = Vec::new();
    let mut entries = Vec::new();

    for item in &items {
        entries.push(convert_item(item, &mut warnings));
    }

    let content = if entries.is_empty() {
        Node::new(node::DOCUMENT)
    } else {
        Node::new(node::DOCUMENT).child(Node::new(node::BIBLIOGRAPHY).children(entries))
    };

    let document = Document {
        content,
        resources: Default::default(),
        metadata: Properties::new(),
        source: None,
    };

    Ok(ConversionResult::with_warnings(document, warnings))
}

#[derive(Debug, Deserialize)]
struct CslItem {
    id: String,
    #[serde(rename = "type")]
    item_type: Option<String>,
    title: Option<String>,
    author: Option<Vec<CslName>>,
    editor: Option<Vec<CslName>>,
    issued: Option<CslDate>,
    #[serde(rename = "container-title")]
    container_title: Option<String>,
    publisher: Option<String>,
    #[serde(rename = "publisher-place")]
    publisher_place: Option<String>,
    edition: Option<String>,
    volume: Option<StringOrInt>,
    issue: Option<StringOrInt>,
    page: Option<String>,
    #[serde(rename = "DOI")]
    doi: Option<String>,
    #[serde(rename = "URL")]
    url: Option<String>,
    #[serde(rename = "ISBN")]
    isbn: Option<String>,
    #[serde(rename = "ISSN")]
    issn: Option<String>,
    /// Every other CSL-JSON variable (`abstract`, `note`, `genre`,
    /// `collection-title`, `language`, ... — the CSL variable list is large
    /// and growing) lands here instead of being silently dropped.
    #[serde(flatten)]
    extra: serde_json::Map<String, Value>,
}

#[derive(Debug, Deserialize)]
struct CslName {
    family: Option<String>,
    given: Option<String>,
    literal: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CslDate {
    #[serde(rename = "date-parts")]
    date_parts: Option<Vec<Vec<Value>>>,
    literal: Option<String>,
    raw: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum StringOrInt {
    String(String),
    Int(i64),
}

impl StringOrInt {
    fn as_str_owned(&self) -> String {
        match self {
            StringOrInt::String(s) => s.clone(),
            StringOrInt::Int(i) => i.to_string(),
        }
    }
}

/// CSL-JSON variable names with no `field:role` equivalent, mapped to a
/// `misc` field tagged with the original variable name via `csl:field` (the
/// same round-trip mechanism as `rescribe-read-bibtex`'s `bibtex:field`).
/// Everything not explicitly modeled (author/editor/title/container-
/// title/publisher/publisher-place/edition/volume/issue/page/DOI/URL/ISBN/
/// ISSN/issued) goes through this generic path.
fn convert_item(item: &CslItem, warnings: &mut Vec<FidelityWarning>) -> Node {
    let mut entry = Node::new(node::BIBLIOGRAPHY_ENTRY).prop("csl:id", item.id.clone());
    if let Some(item_type) = &item.item_type {
        entry = entry.prop("csl:type", item_type.clone());
    }

    let mut fields = Vec::new();

    if let Some(authors) = &item.author {
        for name in authors {
            fields.push(name_field("author", name));
        }
    }
    if let Some(editors) = &item.editor {
        for name in editors {
            fields.push(name_field("editor", name));
        }
    }

    if let Some(title) = &item.title {
        fields.push(text_field("title", title, "title"));
    }
    if let Some(container_title) = &item.container_title {
        fields.push(text_field(
            "container_title",
            container_title,
            "container-title",
        ));
    }
    if let Some(publisher) = &item.publisher {
        fields.push(text_field("publisher", publisher, "publisher"));
    }
    if let Some(place) = &item.publisher_place {
        fields.push(text_field("publisher_location", place, "publisher-place"));
    }
    if let Some(edition) = &item.edition {
        fields.push(text_field("edition", edition, "edition"));
    }
    if let Some(volume) = &item.volume {
        fields.push(text_field("volume", &volume.as_str_owned(), "volume"));
    }
    if let Some(issue) = &item.issue {
        fields.push(text_field("issue", &issue.as_str_owned(), "issue"));
    }
    if let Some(page) = &item.page {
        match split_page_range(page) {
            PageSplit::Single => fields.push(text_field("misc", page, "page")),
            PageSplit::Range(first, last) => {
                fields.push(text_field("page_first", &first, "page"));
                fields.push(text_field("page_last", &last, "page"));
            }
            PageSplit::Ambiguous => fields.push(text_field("misc", page, "page")),
        }
    }
    for (field_name, scheme, value) in [
        ("DOI", "doi", &item.doi),
        ("URL", "url", &item.url),
        ("ISBN", "isbn", &item.isbn),
        ("ISSN", "issn", &item.issn),
    ] {
        if let Some(v) = value {
            fields.push(identifier_field(scheme, v, field_name));
        }
    }

    if let Some(issued) = &item.issued {
        match convert_date(issued) {
            DateResult::Structured(map) => entry = entry.prop(prop::DATE, PropValue::Map(map)),
            DateResult::Text(text) => fields.push(text_field("misc", &text, "issued")),
            DateResult::None => {}
        }
    }

    // Every remaining CSL-JSON variable, in the (deterministic) order
    // serde_json's Map preserves — insertion order from the source JSON.
    for (key, value) in &item.extra {
        match value {
            Value::String(s) => fields.push(text_field("misc", s, key)),
            Value::Number(n) => fields.push(text_field("misc", &n.to_string(), key)),
            Value::Bool(b) => fields.push(text_field("misc", &b.to_string(), key)),
            Value::Null => {}
            // Nested objects/arrays (e.g. a contributor-role variable this
            // reader doesn't special-case, like `translator`) have no
            // lossless flat-text representation here — tracked as a
            // fidelity gap rather than silently dropped or guessed at.
            Value::Array(_) | Value::Object(_) => {
                warnings.push(FidelityWarning::new(
                    rescribe_core::Severity::Minor,
                    rescribe_core::WarningKind::UnsupportedNode(format!("csl-json:{key}")),
                    format!(
                        "CSL-JSON variable '{key}' has a nested array/object value with no \
                         modeled representation; dropped"
                    ),
                ));
            }
        }
    }

    entry.children(fields)
}

fn name_field(role: &str, name: &CslName) -> Node {
    // `csl:name-part` disambiguates which `CslName` field each TEXT child
    // came from — a bare "one child" case is otherwise ambiguous between
    // `literal`, a given-only name, and a family-only name, and the writer
    // must not guess which.
    let mut children = Vec::new();
    if let Some(literal) = &name.literal {
        if !literal.is_empty() {
            children.push(
                Node::new(node::TEXT)
                    .prop(prop::CONTENT, literal.clone())
                    .prop("csl:name-part", "literal"),
            );
        }
    } else {
        if let Some(given) = &name.given
            && !given.is_empty()
        {
            children.push(
                Node::new(node::TEXT)
                    .prop(prop::CONTENT, given.clone())
                    .prop("csl:name-part", "given"),
            );
        }
        if let Some(family) = &name.family
            && !family.is_empty()
        {
            children.push(
                Node::new(node::TEXT)
                    .prop(prop::CONTENT, family.clone())
                    .prop("csl:name-part", "family"),
            );
        }
    }
    Node::new(node::BIBLIOGRAPHY_FIELD)
        .prop(prop::FIELD_ROLE, role)
        .prop("csl:field", role)
        .children(children)
}

fn text_field(role: &str, text: &str, field_name: &str) -> Node {
    let mut node = Node::new(node::BIBLIOGRAPHY_FIELD)
        .prop(prop::FIELD_ROLE, role)
        .prop("csl:field", field_name);
    if !text.is_empty() {
        node = node.child(Node::new(node::TEXT).prop(prop::CONTENT, text.to_string()));
    }
    node
}

fn identifier_field(scheme: &str, text: &str, field_name: &str) -> Node {
    text_field("identifier", text, field_name).prop(prop::FIELD_SCHEME, scheme)
}

enum DateResult {
    Structured(HashMap<String, PropValue>),
    Text(String),
    None,
}

/// Convert CSL-JSON's `issued` date object into `prop::DATE`'s structured
/// map. `date-parts` is `[[year, month, day]]` for a single date (a second
/// inner array makes it a range, with no single date to extract — kept as
/// text instead of guessing which end is "the" date, same as
/// `rescribe-read-bibtex`'s date-range handling); `literal`/`raw` free-text
/// dates (e.g. `"circa 1850"`) have no structured parse and stay as text.
fn convert_date(date: &CslDate) -> DateResult {
    if let Some(parts) = &date.date_parts {
        if parts.len() == 1
            && let Some(single) = parts.first()
        {
            let as_i64 = |v: &Value| v.as_i64();
            if let Some(year) = single.first().and_then(as_i64) {
                let mut map = HashMap::new();
                map.insert("year".to_string(), PropValue::Int(year));
                if let Some(month) = single.get(1).and_then(as_i64) {
                    map.insert("month".to_string(), PropValue::Int(month));
                }
                if let Some(day) = single.get(2).and_then(as_i64) {
                    map.insert("day".to_string(), PropValue::Int(day));
                }
                return DateResult::Structured(map);
            }
        } else if !parts.is_empty() {
            let text = parts
                .iter()
                .map(|p| {
                    p.iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<_>>()
                        .join("-")
                })
                .collect::<Vec<_>>()
                .join("/");
            return DateResult::Text(text);
        }
    }
    if let Some(literal) = &date.literal {
        return DateResult::Text(literal.clone());
    }
    if let Some(raw) = &date.raw {
        return DateResult::Text(raw.clone());
    }
    DateResult::None
}

enum PageSplit {
    Single,
    Range(String, String),
    Ambiguous,
}

fn split_page_range(text: &str) -> PageSplit {
    let t = text.trim();
    if t.is_empty() {
        return PageSplit::Ambiguous;
    }
    if t.chars().all(|c| c.is_ascii_digit()) {
        return PageSplit::Single;
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
                return PageSplit::Range(first.to_string(), last.to_string());
            }
        }
    }
    PageSplit::Ambiguous
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_article() {
        let csl = r#"[{
            "id": "smith2020",
            "type": "article-journal",
            "title": "A Great Paper",
            "author": [{"family": "Smith", "given": "John"}],
            "container-title": "Nature",
            "issued": {"date-parts": [[2020]]}
        }]"#;

        let result = parse(csl).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
        let entry = &doc.content.children[0].children[0];
        assert_eq!(entry.props.get_str("csl:id"), Some("smith2020"));
        assert_eq!(entry.props.get_str("csl:type"), Some("article-journal"));
    }

    #[test]
    fn test_parse_book() {
        let csl = r#"[{
            "id": "knuth1984",
            "type": "book",
            "title": "The TeXbook",
            "author": [{"family": "Knuth", "given": "Donald E."}],
            "publisher": "Addison-Wesley",
            "issued": {"date-parts": [[1984]]}
        }]"#;

        let result = parse(csl).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_empty() {
        let csl = "[]";
        let result = parse(csl).unwrap();
        let doc = result.value;
        assert!(doc.content.children.is_empty());
    }

    #[test]
    fn test_multi_author_siblings() {
        let csl = r#"[{
            "id": "x",
            "author": [
                {"family": "Brown", "given": "Carol"},
                {"family": "Davis", "given": "Eve"}
            ]
        }]"#;
        let result = parse(csl).unwrap();
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
    fn test_literal_date() {
        let csl = r#"[{"id": "old1850", "issued": {"literal": "circa 1850"}}]"#;
        let result = parse(csl).unwrap();
        let doc = result.value;
        let entry = &doc.content.children[0].children[0];
        assert!(entry.props.get(prop::DATE).is_none());
        let misc = entry
            .children
            .iter()
            .find(|c| c.props.get_str("csl:field") == Some("issued"))
            .unwrap();
        assert_eq!(
            misc.children[0].props.get_str(prop::CONTENT),
            Some("circa 1850")
        );
    }
}
