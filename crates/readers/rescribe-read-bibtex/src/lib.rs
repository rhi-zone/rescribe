//! BibTeX reader for rescribe.
//!
//! Parses BibTeX/BibLaTeX bibliography files into rescribe's document IR,
//! using the `bibliography`/`bibliography_entry`/`bibliography_field` node
//! kinds (see `rescribe_std::node` and ADR 0005 in the rescribe repo).
//!
//! # Example
//!
//! ```
//! use rescribe_read_bibtex::parse;
//!
//! let bibtex = r#"
//! @article{smith2020,
//!   author = {John Smith},
//!   title = {A Great Paper},
//!   journal = {Nature},
//!   year = {2020},
//! }
//! "#;
//!
//! let result = parse(bibtex).unwrap();
//! let doc = result.value;
//! ```

use biblatex::{ChunksExt, DateValue, EntryType, PermissiveType, Person};
use rescribe_core::{ConversionResult, Document, FidelityWarning, Node, ParseError, Properties};
use rescribe_std::{PropValue, node, prop};
use std::collections::HashMap;

/// Parse BibTeX text into a document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    let bibliography = biblatex::Bibliography::parse(input)
        .map_err(|e| ParseError::Invalid(format!("BibTeX parse error: {:?}", e)))?;

    let mut warnings = Vec::new();
    let mut entries = Vec::new();

    for entry in bibliography.iter() {
        entries.push(convert_entry(entry, &mut warnings));
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

/// Fields consumed by dedicated logic below (author/editor persons, the
/// date trio, page splitting, identifier fields). Everything else in
/// `entry.fields` goes through the generic role-lookup loop.
const SPECIAL_FIELDS: &[&str] = &[
    "author", "editor", "date", "year", "month", "day", "pages", "doi", "url", "isbn", "issn",
];

/// Known BibTeX/BibLaTeX field name -> `field:role` mapping for fields with
/// no dedicated handling. Anything not listed here becomes a `misc` field
/// (still round-trippable via the always-present `bibtex:field` property).
fn field_role(name: &str) -> &'static str {
    match name {
        "title" => "title",
        "journal" | "journaltitle" | "booktitle" | "maintitle" => "container_title",
        "publisher" => "publisher",
        "address" | "location" => "publisher_location",
        "edition" => "edition",
        "volume" => "volume",
        "number" | "issue" => "issue",
        _ => "misc",
    }
}

fn convert_entry(entry: &biblatex::Entry, warnings: &mut Vec<FidelityWarning>) -> Node {
    let key = entry.key.clone();
    let entry_type = entry_type_string(&entry.entry_type);

    let mut fields = Vec::new();

    // Authors/editors: each Person becomes its own sibling field, in
    // document order; each Person's given/prefix/name/suffix parts become
    // separate text children (rather than joined into one string) so the
    // structure biblatex already extracted isn't re-flattened.
    if let Ok(authors) = entry.author() {
        for person in &authors {
            fields.push(person_field("author", person));
        }
    }
    if let Ok(editors) = entry.editors() {
        for (persons, _editor_type) in &editors {
            for person in persons {
                fields.push(person_field("editor", person));
            }
        }
    }

    // Date: prefer the structured single-date case; ranges and unparsed
    // dates are demoted to a misc field rather than force-fit or dropped.
    let mut entry_node = Node::new(node::BIBLIOGRAPHY_ENTRY)
        .prop("bibtex:key", key)
        .prop("bibtex:entry-type", entry_type);
    match entry.date() {
        Ok(PermissiveType::Typed(date)) => match date.value {
            DateValue::At(dt) => {
                let mut map = HashMap::new();
                map.insert("year".to_string(), PropValue::Int(dt.year as i64));
                if let Some(m) = dt.month {
                    map.insert("month".to_string(), PropValue::Int(m as i64 + 1));
                }
                if let Some(d) = dt.day {
                    map.insert("day".to_string(), PropValue::Int(d as i64 + 1));
                }
                entry_node = entry_node.prop(prop::DATE, PropValue::Map(map));
                if date.uncertain {
                    entry_node = entry_node.prop("bibtex:date-uncertain", true);
                }
                if date.approximate {
                    entry_node = entry_node.prop("bibtex:date-approximate", true);
                }
                // Raw preservation: the literal `date` field text (only
                // present when the source used a single BibLaTeX-style
                // `date` field rather than separate `year`/`month`/`day`
                // fields), markers and all. Not required for the writer to
                // round-trip `bibtex:date-uncertain`/`bibtex:date-approximate`
                // (those two booleans already fully determine the `?`/`~`/`%`
                // marker `rescribe-write-bibtex` re-emits), but kept for any
                // consumer that wants the exact original text.
                if let Some(chunks) = entry.get("date") {
                    entry_node = entry_node.prop("bibtex:raw-date", chunks.format_verbatim());
                }
            }
            // A date range (`After`/`Before`/`Between`) has no single
            // year/month/day to put in `prop::DATE` — keep it as text
            // instead of guessing which end is "the" date.
            other => fields.push(misc_field("date", &format!("{:?}", other), "date")),
        },
        Ok(PermissiveType::Chunks(chunks)) => {
            fields.push(misc_field("date", &chunks.format_verbatim(), "date"));
        }
        Err(_) => {}
    }

    // Pages: split into page_first/page_last when unambiguous, otherwise
    // keep the whole string as a misc field (mirrors the docbook/jats
    // `<pagenums>` handling).
    if let Some(chunks) = entry.get("pages") {
        let text = chunks.format_verbatim();
        match split_page_range(&text) {
            PageSplit::Single => fields.push(misc_field("pages", &text, "pages")),
            PageSplit::Range(first, last) => {
                fields.push(text_field("page_first", &first, "pages"));
                fields.push(text_field("page_last", &last, "pages"));
            }
            PageSplit::Ambiguous => fields.push(misc_field("pages", &text, "pages")),
        }
    }

    // Identifiers: doi/isbn/issn/url, each an `identifier` field tagged
    // with its scheme via `field:scheme`.
    for (field_name, scheme) in [
        ("doi", "doi"),
        ("isbn", "isbn"),
        ("issn", "issn"),
        ("url", "url"),
    ] {
        if let Some(chunks) = entry.get(field_name) {
            fields.push(identifier_field(
                scheme,
                &chunks.format_verbatim(),
                field_name,
            ));
        }
    }

    // Everything else: generic role lookup, alphabetical (BTreeMap) order.
    for (name, chunks) in &entry.fields {
        if SPECIAL_FIELDS.contains(&name.as_str()) {
            continue;
        }
        let role = field_role(name);
        let text = chunks.format_verbatim();
        fields.push(text_field(role, &text, name));
    }

    let _ = warnings; // no unrepresentable constructs found yet; kept for future use
    entry_node.children(fields)
}

/// BibLaTeX's `EntryType` serializes to lowercase via `Display` for every
/// known variant; `Unknown(name)` carries the original (already-lowercased
/// by biblatex) custom type name, which `Display` would otherwise discard.
fn entry_type_string(entry_type: &EntryType) -> String {
    match entry_type {
        EntryType::Unknown(name) => name.to_lowercase(),
        other => other.to_string(),
    }
}

fn person_field(role: &str, person: &Person) -> Node {
    let mut children = Vec::new();
    for part in [
        &person.given_name,
        &person.prefix,
        &person.name,
        &person.suffix,
    ] {
        if !part.is_empty() {
            children.push(Node::new(node::TEXT).prop(prop::CONTENT, part.clone()));
        }
    }
    Node::new(node::BIBLIOGRAPHY_FIELD)
        .prop(prop::FIELD_ROLE, role)
        .prop("bibtex:field", role)
        .children(children)
}

fn text_field(role: &str, text: &str, field_name: &str) -> Node {
    let mut node = Node::new(node::BIBLIOGRAPHY_FIELD)
        .prop(prop::FIELD_ROLE, role)
        .prop("bibtex:field", field_name);
    if !text.is_empty() {
        node = node.child(Node::new(node::TEXT).prop(prop::CONTENT, text.to_string()));
    }
    node
}

fn misc_field(field_name: &str, text: &str, tag: &str) -> Node {
    text_field("misc", text, field_name).prop("bibtex:field", tag)
}

fn identifier_field(scheme: &str, text: &str, field_name: &str) -> Node {
    text_field("identifier", text, field_name).prop(prop::FIELD_SCHEME, scheme)
}

/// The result of attempting to split a `pages` field into a page range.
enum PageSplit {
    /// A single page number (no separator) — kept whole.
    Single,
    /// A clean `first`-`last` numeric range.
    Range(String, String),
    /// Anything else (discontiguous list, roman numerals, `42ff`, multiple
    /// separators, ...) — splitting would be a guess, kept whole instead.
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
        let bibtex = r#"
@article{smith2020,
  author = {John Smith},
  title = {A Great Paper},
  journal = {Nature},
  year = {2020},
}
"#;

        let result = parse(bibtex).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
        let bibliography = &doc.content.children[0];
        assert_eq!(bibliography.kind.as_str(), node::BIBLIOGRAPHY);
        let entry = &bibliography.children[0];
        assert_eq!(entry.kind.as_str(), node::BIBLIOGRAPHY_ENTRY);
        assert_eq!(entry.props.get_str("bibtex:key"), Some("smith2020"));
        assert_eq!(entry.props.get_str("bibtex:entry-type"), Some("article"));
    }

    #[test]
    fn test_parse_book() {
        let bibtex = r#"
@book{knuth1984,
  author = {Donald E. Knuth},
  title = {The TeXbook},
  publisher = {Addison-Wesley},
  year = {1984},
}
"#;

        let result = parse(bibtex).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_empty() {
        let bibtex = "";
        let result = parse(bibtex).unwrap();
        let doc = result.value;
        assert!(doc.content.children.is_empty());
    }

    #[test]
    fn test_multiple_authors_are_sibling_fields() {
        let bibtex = r#"
@article{smith2021,
  author = {Smith, J. and Jones, A.},
  title = {A Joint Study},
  year = {2021},
}
"#;
        let result = parse(bibtex).unwrap();
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
    fn test_date_year_month_day() {
        let bibtex = r#"
@article{x,
  author = {A},
  title = {T},
  year = {2020},
  month = {mar},
}
"#;
        let result = parse(bibtex).unwrap();
        let doc = result.value;
        let entry = &doc.content.children[0].children[0];
        let Some(rescribe_core::PropValue::Map(map)) = entry.props.get(prop::DATE) else {
            panic!("expected structured date");
        };
        assert_eq!(map.get("year"), Some(&rescribe_core::PropValue::Int(2020)));
        assert_eq!(map.get("month"), Some(&rescribe_core::PropValue::Int(3)));
    }

    #[test]
    fn test_date_uncertain_marker_and_raw_capture() {
        let bibtex = r#"
@article{x,
  author = {A},
  title = {T},
  date = {2020-03-15?},
}
"#;
        let result = parse(bibtex).unwrap();
        let doc = result.value;
        let entry = &doc.content.children[0].children[0];
        assert_eq!(entry.props.get_bool("bibtex:date-uncertain"), Some(true));
        assert_eq!(entry.props.get_str("bibtex:raw-date"), Some("2020-03-15?"));
    }
}
