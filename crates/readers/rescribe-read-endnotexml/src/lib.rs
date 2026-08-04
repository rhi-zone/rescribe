//! EndNote XML reader for rescribe.
//!
//! Thin AST→IR adapter over [`endnotexml_fmt`] (the standalone EndNote XML
//! parser/AST/emitter crate — see its docs for the format grammar). All XML
//! parsing lives in `endnotexml-fmt`; this crate only translates
//! `endnotexml_fmt::EndNoteDoc` into rescribe's `Document`, using the
//! `bibliography`/`bibliography_entry`/`bibliography_field` node kinds (see
//! `rescribe_std::node` and ADR 0005). Per CLAUDE.md's adapter-layer rule,
//! no `quick_xml` (or any XML tokenizer) appears in this crate's production
//! code.
//!
//! # Mapping
//!
//! Each `Record` becomes one `bibliography_entry`, carrying `endnote:type`
//! (the `ref-type` code), `endnote:key` (a generated or `label`/
//! `rec-number`-derived cite key), and — when present — `endnote:ref-type-name`,
//! `endnote:rec-number`, `endnote:label`.
//!
//! Each recognized field becomes a `bibliography_field` child tagged with
//! both `field:role` (the semantic vocabulary other bibliography formats
//! share) and `endnote:field` (the exact source element path, e.g.
//! `titles/secondary-title`, `urls/related-urls/url`), so the writer can
//! reconstruct the original nested wrapper without guessing a shape from
//! `field:role` alone. Any field this reader doesn't have a dedicated
//! mapping for (EndNote's schema is exporter-dependent — `custom1`..
//! `custom7`, `research-notes`, `work-type`, ...) becomes a `misc` field
//! instead of being dropped — this includes both `Record::extra` (unknown
//! top-level elements) and every container's own `extra` bucket (unknown
//! children of `<contributors>`/`<titles>`/`<periodical>`/`<urls>`/
//! `<dates>`/`<foreign-keys>`), which `endnotexml-fmt`'s AST captures
//! losslessly even where the pre-existing reader this crate replaces did
//! not (see `endnotexml_fmt::ast` module docs).
//!
//! `<style face="...">` runs in field content become real
//! `emphasis`/`strong`/`underline`/`superscript`/`subscript` inline nodes
//! (recursively) rather than flattened text, since `bibliography_field`
//! children are ordinary inline nodes — deciding what a given `face` value
//! *means* is this adapter's job (see [`inline_to_ir`]); `endnotexml-fmt`
//! itself only records the `face` string verbatim.

use endnotexml_fmt::{Dates, EndNoteDoc, Inline, Record};
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
    let (doc, _diagnostics): (EndNoteDoc, _) = endnotexml_fmt::parse(input.as_bytes());

    let entries: Vec<Node> = doc.records.iter().map(convert_record).collect();

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

fn convert_record(record: &Record) -> Node {
    let mut fields = Vec::new();

    if let Some(fk) = &record.foreign_keys {
        for key in &fk.keys {
            let mut field = Node::new(node::BIBLIOGRAPHY_FIELD)
                .prop(prop::FIELD_ROLE, "misc")
                .prop("endnote:field", "foreign-keys/key")
                .child(Node::new(node::TEXT).prop(prop::CONTENT, key.text.clone()));
            if let Some(app) = &key.app {
                field = field.prop("endnote:app", app.clone());
            }
            if let Some(db_id) = &key.db_id {
                field = field.prop("endnote:db-id", db_id.clone());
            }
            fields.push(field);
        }
        for el in &fk.extra {
            fields.push(misc_field(
                &format!("foreign-keys/{}", el.name),
                &el.children,
            ));
        }
    }

    if let Some(c) = &record.contributors {
        for person in &c.authors {
            fields.push(inline_field("author", "authors/author", person));
        }
        for person in &c.secondary_authors {
            fields.push(inline_field("editor", "secondary-authors/author", person));
        }
        // Tertiary/subsidiary authors have no `field:role` equivalent —
        // raw-preserved as misc rather than dropped.
        for person in &c.tertiary_authors {
            fields.push(inline_field("misc", "tertiary-authors/author", person));
        }
        for person in &c.subsidiary_authors {
            fields.push(inline_field("misc", "subsidiary-authors/author", person));
        }
        for el in &c.extra {
            fields.push(misc_field(
                &format!("contributors/{}", el.name),
                &el.children,
            ));
        }
    }

    if let Some(t) = &record.titles {
        if let Some(title) = &t.title {
            fields.push(inline_field("title", "titles/title", title));
        }
        if let Some(secondary) = &t.secondary_title {
            fields.push(inline_field(
                "container_title",
                "titles/secondary-title",
                secondary,
            ));
        }
        if let Some(tertiary) = &t.tertiary_title {
            fields.push(inline_field("misc", "titles/tertiary-title", tertiary));
        }
        for el in &t.extra {
            fields.push(misc_field(&format!("titles/{}", el.name), &el.children));
        }
    }

    if let Some(p) = &record.periodical {
        if let Some(full_title) = &p.full_title {
            fields.push(inline_field("misc", "periodical/full-title", full_title));
        }
        for el in &p.extra {
            fields.push(misc_field(&format!("periodical/{}", el.name), &el.children));
        }
    }

    if let Some(volume) = &record.volume {
        fields.push(inline_field("volume", "volume", volume));
    }
    if let Some(number) = &record.number {
        fields.push(inline_field("issue", "number", number));
    }
    if let Some(pages) = &record.pages {
        push_pages(&mut fields, pages);
    }
    if let Some(publisher) = &record.publisher {
        fields.push(inline_field("publisher", "publisher", publisher));
    }
    if let Some(loc) = &record.pub_location {
        fields.push(inline_field("publisher_location", "pub-location", loc));
    }
    if let Some(isbn) = &record.isbn {
        fields.push(identifier_field("isbn", isbn, "isbn"));
    }
    if let Some(issn) = &record.issn {
        fields.push(identifier_field("issn", issn, "issn"));
    }
    if let Some(doi) = &record.electronic_resource_num {
        fields.push(identifier_field("doi", doi, "electronic-resource-num"));
    }

    if let Some(u) = &record.urls {
        for url in &u.related_urls {
            fields.push(identifier_field("url", url, "urls/related-urls/url"));
        }
        for url in &u.pdf_urls {
            fields.push(identifier_field("url", url, "urls/pdf-urls/url"));
        }
        for el in &u.extra {
            fields.push(misc_field(&format!("urls/{}", el.name), &el.children));
        }
    }
    // A bare top-level `<url>` (outside `<urls>`) also occurs in practice —
    // handled the same as `related-urls/url` rather than dropped.
    if let Some(url) = &record.bare_url {
        fields.push(identifier_field("url", url, "url"));
    }

    if let Some(abstract_) = &record.abstract_ {
        fields.push(inline_field("misc", "abstract", abstract_));
    }
    if let Some(notes) = &record.notes {
        fields.push(inline_field("misc", "notes", notes));
    }
    for keyword in &record.keywords {
        fields.push(inline_field("misc", "keywords/keyword", keyword));
    }

    // Any other record-level element this reader doesn't know by name:
    // generic fallback, nothing silently dropped even for EndNote fields
    // this reader doesn't recognize (EndNote's field vocabulary is large
    // and exporter-dependent — `custom1`..`custom7`, `research-notes`,
    // `remote-database-name`, `language`, `work-type`, ...).
    for el in &record.extra {
        fields.push(misc_field(&el.name, &el.children));
    }

    let cite_key = record
        .label
        .clone()
        .filter(|s| !s.is_empty())
        .or_else(|| record.rec_number.clone().filter(|s| !s.is_empty()))
        .unwrap_or_else(|| generate_cite_key(record));

    let mut entry_node = Node::new(node::BIBLIOGRAPHY_ENTRY)
        .prop("endnote:type", record.ref_type.code.clone())
        .prop("endnote:key", cite_key);
    if let Some(name) = &record.ref_type.name {
        entry_node = entry_node.prop("endnote:ref-type-name", name.clone());
    }
    if let Some(rec_number) = record.rec_number.clone().filter(|s| !s.is_empty()) {
        entry_node = entry_node.prop("endnote:rec-number", rec_number);
    }
    if let Some(label) = record.label.clone().filter(|s| !s.is_empty()) {
        entry_node = entry_node.prop("endnote:label", label);
    }

    if let Some(dates) = &record.dates {
        if let Some(year) = &dates.year {
            let text = endnotexml_fmt::parse::flatten_inline_text(year);
            if let Ok(y) = text.trim().parse::<i64>() {
                let mut map = HashMap::new();
                map.insert("year".to_string(), PropValue::Int(y));
                entry_node = entry_node.prop(prop::DATE, PropValue::Map(map));
            } else if !text.is_empty() {
                fields.push(inline_field("misc", "dates/year", year));
            }
        }
        if let Some(pub_date) = &dates.pub_date {
            // A free-text pub-date (e.g. "Jan 15") has no unambiguous
            // year/month/day parse without guessing a locale's month-name
            // convention — kept as a misc field instead.
            fields.push(inline_field("misc", "dates/pub-dates/date", pub_date));
        }
        for el in &dates.extra {
            fields.push(misc_field(&format!("dates/{}", el.name), &el.children));
        }
    }

    entry_node.children(fields)
}

fn generate_cite_key(record: &Record) -> String {
    let author = record
        .contributors
        .as_ref()
        .and_then(|c| c.authors.first())
        .map(|a| endnotexml_fmt::parse::flatten_inline_text(a));
    let year = record
        .dates
        .as_ref()
        .and_then(|d: &Dates| d.year.as_ref())
        .map(|y| endnotexml_fmt::parse::flatten_inline_text(y));

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

/// Build a `bibliography_field` whose children are `inline`'s content
/// converted through [`inline_to_ir`] — this is where `<style>` markup
/// becomes real `emphasis`/`strong`/... nodes instead of being flattened to
/// plain text.
fn inline_field(role: &str, tag: &str, inline: &[Inline]) -> Node {
    Node::new(node::BIBLIOGRAPHY_FIELD)
        .prop(prop::FIELD_ROLE, role)
        .prop("endnote:field", tag)
        .children(inline_to_ir(inline))
}

/// A raw-preserved field for content this reader doesn't have a dedicated
/// `field:role` mapping for — used both for `Record::extra` (unknown
/// top-level elements) and every container's own `extra` bucket.
fn misc_field(tag: &str, inline: &[Inline]) -> Node {
    inline_field("misc", tag, inline)
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
fn push_pages(fields: &mut Vec<Node>, pages: &[Inline]) {
    let inline_ir = inline_to_ir(pages);
    if let [single] = inline_ir.as_slice()
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
            .children(inline_ir),
    );
}

fn text_field(role: &str, text: &str, tag: &str) -> Node {
    Node::new(node::BIBLIOGRAPHY_FIELD)
        .prop(prop::FIELD_ROLE, role)
        .prop("endnote:field", tag)
        .child(Node::new(node::TEXT).prop(prop::CONTENT, text.to_string()))
}

/// Convert a field's `endnotexml-fmt` inline content to IR inline nodes.
/// Plain text becomes a `TEXT` node; `Style { face, .. }` becomes the
/// matching inline markup node (`emphasis`/`strong`/`underline`/
/// `superscript`/`subscript`) wrapped around its own (recursively
/// converted) content — `face == "normal"` (or any unrecognized/absent
/// face) passes its content through unwrapped. `Other` (an element
/// `endnotexml-fmt` doesn't recognize as `<style>`) is also passed through
/// transparently (its content flattened in, rather than dropped), since
/// EndNote's actual exporters occasionally nest other presentation wrappers
/// here — deciding *how* to interpret `face`/unrecognized elements is this
/// adapter's job; `endnotexml-fmt` itself only records them verbatim.
fn inline_to_ir(inline: &[Inline]) -> Vec<Node> {
    let mut out = Vec::new();
    for item in inline {
        match item {
            Inline::Text(t) => {
                if !t.is_empty() {
                    out.push(Node::new(node::TEXT).prop(prop::CONTENT, t.clone()));
                }
            }
            Inline::Style { face, children } => {
                out.extend(convert_style(face, children));
            }
            Inline::Other { children, .. } => {
                out.extend(inline_to_ir(children));
            }
        }
    }
    out
}

fn convert_style(face: &str, children: &[Inline]) -> Vec<Node> {
    let inner = inline_to_ir(children);
    let face = face.to_lowercase();
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

    #[test]
    fn test_unknown_record_element_preserved() {
        let xml = r#"<xml><records><record>
  <ref-type>13</ref-type>
  <custom1>foo</custom1>
</record></records></xml>"#;
        let result = parse(xml).unwrap();
        let doc = result.value;
        let entry = &doc.content.children[0].children[0];
        let misc = entry
            .children
            .iter()
            .find(|c| c.props.get_str("endnote:field") == Some("custom1"))
            .unwrap();
        assert_eq!(misc.props.get_str(prop::FIELD_ROLE), Some("misc"));
    }

    #[test]
    fn test_foreign_keys() {
        let xml = r#"<xml><records><record>
  <ref-type>17</ref-type>
  <foreign-keys><key app="EN" db-id="abc123">42</key></foreign-keys>
</record></records></xml>"#;
        let result = parse(xml).unwrap();
        let doc = result.value;
        let entry = &doc.content.children[0].children[0];
        let key_field = entry
            .children
            .iter()
            .find(|c| c.props.get_str("endnote:field") == Some("foreign-keys/key"))
            .unwrap();
        assert_eq!(key_field.props.get_str("endnote:app"), Some("EN"));
        assert_eq!(key_field.props.get_str("endnote:db-id"), Some("abc123"));
    }
}
