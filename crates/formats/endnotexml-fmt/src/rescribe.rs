//! AST↔`rescribe::Document` translation for EndNote XML.
//!
//! This module only translates between [`EndNoteDoc`](crate::EndNoteDoc)
//! and rescribe's `Document` IR — no XML tokenizing/parsing/emitting
//! happens here (that all lives in the rest of this crate; see
//! `crate::parse` and `crate::emit`). Enabled by the `rescribe` feature;
//! each direction is additionally gated on the reader/writer mode feature
//! it depends on, so enabling `rescribe` alone (with no mode feature)
//! compiles nothing.
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
//! losslessly even where the pre-relocation reader this module replaces did
//! not (see `endnotexml_fmt::ast` module docs).
//!
//! `<style face="...">` runs in field content become real
//! `emphasis`/`strong`/`underline`/`superscript`/`subscript` inline nodes
//! (recursively) rather than flattened text, since `bibliography_field`
//! children are ordinary inline nodes — deciding what a given `face` value
//! *means* is this adapter's job (see `read::inline_to_ir`); `endnotexml-fmt`
//! itself only records the `face` string verbatim.

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
mod read {
    use crate::{Dates, EndNoteDoc, Inline, Record};
    use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions, Properties};
    use rescribe_format_api::Parse as _;
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
        let (doc, _diagnostics) = EndNoteDoc::parse(input.as_bytes());

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
                let text = crate::parse::flatten_inline_text(year);
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
            .map(|a| crate::parse::flatten_inline_text(a));
        let year = record
            .dates
            .as_ref()
            .and_then(|d: &Dates| d.year.as_ref())
            .map(|y| crate::parse::flatten_inline_text(y));

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
}

#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
mod write {
    use crate::{
        Contributors, Dates, Element, EndNoteDoc, ForeignKey, ForeignKeys, Inline, Periodical,
        Record, RefType, Titles, Urls,
    };
    use rescribe_core::{
        ConversionResult, Document, EmitError, EmitOptions, FidelityWarning, Node, PropValue,
    };
    use rescribe_format_api::Emit as _;
    use rescribe_std::{node, prop};

    /// Legacy flat entry kinds, still accepted for backwards compatibility with
    /// documents built by hand or by an older reader version (not produced by
    /// [`super::read::parse`] any more, which now emits `bibliography_entry`).
    const ENDNOTE_ENTRY: &str = "endnote:entry";
    const BIBTEX_ENTRY: &str = "bibtex:entry";
    const RIS_ENTRY: &str = "ris:entry";
    const CITATION_ENTRY: &str = "citation_entry";

    /// Emit a document as EndNote XML.
    pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        emit_with_options(doc, &EmitOptions::default())
    }

    /// Emit a document as EndNote XML with options.
    pub fn emit_with_options(
        doc: &Document,
        _options: &EmitOptions,
    ) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        let mut ctx = EmitContext::default();
        ctx.write_document(doc);
        let records = std::mem::take(&mut ctx.records);
        let warnings = std::mem::take(&mut ctx.warnings);

        let ast = EndNoteDoc {
            xml_decl: Some(crate::XmlDecl {
                version: "1.0".to_string(),
                encoding: Some("UTF-8".to_string()),
                standalone: None,
            }),
            records,
            span: crate::Span::NONE,
        };
        let bytes = ast.emit();
        Ok(ConversionResult::with_warnings(bytes, warnings))
    }

    #[derive(Default)]
    struct EmitContext {
        records: Vec<Record>,
        warnings: Vec<FidelityWarning>,
        record_count: usize,
    }

    impl EmitContext {
        fn write_document(&mut self, doc: &Document) {
            self.write_nodes(&doc.content.children);
        }

        fn write_nodes(&mut self, nodes: &[Node]) {
            for node in nodes {
                self.write_node(node);
            }
        }

        fn write_node(&mut self, node: &Node) {
            match node.kind.as_str() {
                "document" | "definition_list" | node::BIBLIOGRAPHY => {
                    self.write_nodes(&node.children)
                }
                node::BIBLIOGRAPHY_ENTRY => self.records.push(bibliography_entry_to_record(node)),
                ENDNOTE_ENTRY => {
                    let rec = self.next_rec();
                    self.records.push(endnote_entry_to_record(node, rec));
                }
                BIBTEX_ENTRY => {
                    let rec = self.next_rec();
                    self.records.push(bibtex_entry_to_record(node, rec));
                }
                RIS_ENTRY => {
                    let rec = self.next_rec();
                    self.records.push(ris_entry_to_record(node, rec));
                }
                CITATION_ENTRY => {
                    let rec = self.next_rec();
                    self.records.push(citation_entry_to_record(node, rec));
                }
                _ => {
                    if is_bibtex_type(node.kind.as_str()) {
                        let rec = self.next_rec();
                        self.records.push(typed_entry_to_record(node, rec));
                    } else {
                        self.write_nodes(&node.children);
                    }
                }
            }
        }

        fn next_rec(&mut self) -> usize {
            self.record_count += 1;
            self.record_count
        }
    }

    /// Build a `Record` from a `bibliography_entry` node (see
    /// [`super::read::convert_record`]) — the inverse of that reader, grouping
    /// fields back by their `endnote:field` tag into the nested wrappers
    /// `endnotexml_fmt::emit` expects.
    fn bibliography_entry_to_record(node: &Node) -> Record {
        let mut authors = Vec::new();
        let mut secondary_authors = Vec::new();
        let mut tertiary_authors = Vec::new();
        let mut subsidiary_authors = Vec::new();
        let mut foreign_keys = Vec::new();
        let mut title = None;
        let mut secondary_title = None;
        let mut tertiary_title = None;
        let mut periodical_full_title = None;
        let mut volume = None;
        let mut number = None;
        let mut page_first = None;
        let mut page_last = None;
        let mut pages_misc = None;
        let mut publisher = None;
        let mut pub_location = None;
        let mut isbn = None;
        let mut issn = None;
        let mut doi = None;
        let mut related_urls = Vec::new();
        let mut pdf_urls = Vec::new();
        let mut bare_url = None;
        let mut abstract_ = None;
        let mut notes = None;
        let mut keywords = Vec::new();
        let mut dates_year = None;
        let mut dates_pub_date = None;
        let mut extra: Vec<(&str, &Node)> = Vec::new();

        for child in &node.children {
            if child.kind.as_str() != node::BIBLIOGRAPHY_FIELD {
                continue;
            }
            let role = child.props.get_str(prop::FIELD_ROLE).unwrap_or("misc");
            let tag = child.props.get_str("endnote:field").unwrap_or(role);
            match tag {
                "authors/author" => authors.push(child),
                "secondary-authors/author" => secondary_authors.push(child),
                "tertiary-authors/author" => tertiary_authors.push(child),
                "subsidiary-authors/author" => subsidiary_authors.push(child),
                "titles/title" => title = Some(child),
                "titles/secondary-title" => secondary_title = Some(child),
                "titles/tertiary-title" => tertiary_title = Some(child),
                "periodical/full-title" => periodical_full_title = Some(child),
                "volume" => volume = Some(child),
                "number" => number = Some(child),
                "pages" if role == "page_first" => page_first = Some(child),
                "pages" if role == "page_last" => page_last = Some(child),
                "pages" => pages_misc = Some(child),
                "publisher" => publisher = Some(child),
                "pub-location" => pub_location = Some(child),
                "isbn" => isbn = Some(child),
                "issn" => issn = Some(child),
                "electronic-resource-num" => doi = Some(child),
                "urls/related-urls/url" => related_urls.push(child),
                "urls/pdf-urls/url" => pdf_urls.push(child),
                "url" => bare_url = Some(child),
                "foreign-keys/key" => foreign_keys.push(child),
                "abstract" => abstract_ = Some(child),
                "notes" => notes = Some(child),
                "keywords/keyword" => keywords.push(child),
                "dates/year" => dates_year = Some(child),
                "dates/pub-dates/date" => dates_pub_date = Some(child),
                other => extra.push((other, child)),
            }
        }

        let contributors = if authors.is_empty()
            && secondary_authors.is_empty()
            && tertiary_authors.is_empty()
            && subsidiary_authors.is_empty()
        {
            None
        } else {
            Some(Contributors {
                authors: authors.into_iter().map(field_to_inline).collect(),
                secondary_authors: secondary_authors.into_iter().map(field_to_inline).collect(),
                tertiary_authors: tertiary_authors.into_iter().map(field_to_inline).collect(),
                subsidiary_authors: subsidiary_authors
                    .into_iter()
                    .map(field_to_inline)
                    .collect(),
                extra: Vec::new(),
            })
        };

        let titles = if title.is_some() || secondary_title.is_some() || tertiary_title.is_some() {
            Some(Titles {
                title: title.map(field_to_inline),
                secondary_title: secondary_title.map(field_to_inline),
                tertiary_title: tertiary_title.map(field_to_inline),
                extra: Vec::new(),
            })
        } else {
            None
        };

        let periodical = periodical_full_title.map(|f| Periodical {
            full_title: Some(field_to_inline(f)),
            extra: Vec::new(),
        });

        let pages = match (page_first, page_last) {
            (Some(first), Some(last)) => Some(vec![Inline::Text(format!(
                "{}-{}",
                flatten_field_text(first),
                flatten_field_text(last)
            ))]),
            (Some(first), None) => Some(field_to_inline(first)),
            (None, Some(last)) => Some(field_to_inline(last)),
            (None, None) => pages_misc.map(field_to_inline),
        };

        let urls = if related_urls.is_empty() && pdf_urls.is_empty() {
            None
        } else {
            Some(Urls {
                related_urls: related_urls.into_iter().map(flatten_field_text).collect(),
                pdf_urls: pdf_urls.into_iter().map(flatten_field_text).collect(),
                extra: Vec::new(),
            })
        };

        let foreign_keys = if foreign_keys.is_empty() {
            None
        } else {
            Some(ForeignKeys {
                keys: foreign_keys
                    .into_iter()
                    .map(|key| ForeignKey {
                        app: key.props.get_str("endnote:app").map(str::to_string),
                        db_id: key.props.get_str("endnote:db-id").map(str::to_string),
                        text: flatten_field_text(key),
                    })
                    .collect(),
                extra: Vec::new(),
            })
        };

        let dates = if dates_year.is_some()
            || dates_pub_date.is_some()
            || node.props.get(prop::DATE).is_some()
        {
            let year = if let Some(PropValue::Map(date)) = node.props.get(prop::DATE) {
                match date.get("year") {
                    Some(PropValue::Int(y)) => Some(vec![Inline::Text(y.to_string())]),
                    _ => dates_year.map(field_to_inline),
                }
            } else {
                dates_year.map(field_to_inline)
            };
            Some(Dates {
                year,
                pub_date: dates_pub_date.map(field_to_inline),
                extra: Vec::new(),
            })
        } else {
            None
        };

        // Everything else (a plain top-level tag name with no "/", from the
        // reader's record-level generic fallback, or a `container/leaf` path
        // from one of the newer nested-container fallbacks) round-trips as its
        // own element: a `/`-free tag stays at the record level; a
        // `container/leaf` tag round-trips as a record-level element too
        // (endnotexml-fmt's own AST has no slot for re-nesting an arbitrary
        // unknown container's unknown child, so this preserves the *content*
        // exactly while flattening it back to the record level rather than
        // guessing which container to reconstruct).
        let extra: Vec<Element> = extra
            .into_iter()
            .map(|(tag, field)| {
                let name = tag.rsplit('/').next().unwrap_or(tag).to_string();
                Element {
                    name,
                    attrs: Vec::new(),
                    children: field_to_inline(field),
                }
            })
            .collect();

        Record {
            ref_type: RefType {
                code: node.props.get_str("endnote:type").unwrap_or("").to_string(),
                name: node
                    .props
                    .get_str("endnote:ref-type-name")
                    .map(str::to_string),
            },
            rec_number: node.props.get_str("endnote:rec-number").map(str::to_string),
            label: node.props.get_str("endnote:label").map(str::to_string),
            foreign_keys,
            contributors,
            titles,
            periodical,
            volume: volume.map(field_to_inline),
            number: number.map(field_to_inline),
            pages,
            publisher: publisher.map(field_to_inline),
            pub_location: pub_location.map(field_to_inline),
            isbn: isbn.map(flatten_field_text),
            issn: issn.map(flatten_field_text),
            electronic_resource_num: doi.map(flatten_field_text),
            urls,
            bare_url: bare_url.map(flatten_field_text),
            abstract_: abstract_.map(field_to_inline),
            notes: notes.map(field_to_inline),
            keywords: keywords.into_iter().map(field_to_inline).collect(),
            dates,
            extra,
            span: crate::Span::NONE,
        }
    }

    /// Inverse of [`super::read::inline_to_ir`]: `TEXT` nodes become
    /// `Inline::Text`; `emphasis`/`strong`/`underline`/`superscript`/
    /// `subscript` become `Inline::Style { face, .. }`. Any other inline node
    /// kind (from a non-EndNote producer) has its content flattened in rather
    /// than dropped.
    fn field_to_inline(field: &Node) -> Vec<Inline> {
        ir_to_inline(&field.children)
    }

    fn ir_to_inline(children: &[Node]) -> Vec<Inline> {
        let mut out = Vec::new();
        for child in children {
            let face = match child.kind.as_str() {
                k if k == node::TEXT => {
                    if let Some(content) = child.props.get_str(prop::CONTENT) {
                        out.push(Inline::Text(content.to_string()));
                    }
                    continue;
                }
                k if k == node::EMPHASIS => Some("italic"),
                k if k == node::STRONG => Some("bold"),
                k if k == node::UNDERLINE => Some("underline"),
                k if k == node::SUPERSCRIPT => Some("superscript"),
                k if k == node::SUBSCRIPT => Some("subscript"),
                _ => None,
            };
            match face {
                Some(face) => out.push(Inline::Style {
                    face: face.to_string(),
                    children: ir_to_inline(&child.children),
                }),
                None => out.extend(ir_to_inline(&child.children)),
            }
        }
        out
    }

    /// Concatenate a field's descendant `TEXT` node content (depth-first) —
    /// used for fields where `endnotexml-fmt`'s AST stores a flat `String`
    /// (ISBN/ISSN/DOI/URLs/record numbers/foreign-key text), which are never
    /// expected to carry `<style>` markup.
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

    fn endnote_entry_to_record(node: &Node, rec_number: usize) -> Record {
        let mut record = Record {
            ref_type: RefType {
                code: node.props.get_str("endnote:type").unwrap_or("").to_string(),
                name: None,
            },
            rec_number: Some(rec_number.to_string()),
            ..Record::default()
        };
        for (key, value) in node.props.iter() {
            if let Some(field) = key.strip_prefix("endnote:")
                && field != "type"
                && field != "key"
                && let rescribe_core::PropValue::String(s) = value
            {
                record.extra.push(Element {
                    name: field.to_string(),
                    attrs: Vec::new(),
                    children: vec![Inline::Text(s.clone())],
                });
            }
        }
        record
    }

    fn bibtex_entry_to_record(node: &Node, rec_number: usize) -> Record {
        let mut record = Record {
            rec_number: Some(rec_number.to_string()),
            ..Record::default()
        };
        if let Some(bibtex_type) = node.props.get_str("bibtex:type") {
            record.ref_type.code = bibtex_to_endnote_type(bibtex_type).to_string();
        }
        if let Some(title) = node.props.get_str("bibtex:title") {
            record.titles = Some(Titles {
                title: Some(vec![Inline::Text(title.to_string())]),
                secondary_title: None,
                tertiary_title: None,
                extra: Vec::new(),
            });
        }
        if let Some(author) = node.props.get_str("bibtex:author") {
            record.contributors = Some(authors_from_str(author));
        }
        if let Some(year) = node.props.get_str("bibtex:year") {
            record.dates = Some(Dates {
                year: Some(vec![Inline::Text(year.to_string())]),
                pub_date: None,
                extra: Vec::new(),
            });
        }
        if let Some(journal) = node.props.get_str("bibtex:journal") {
            record.periodical = Some(Periodical {
                full_title: Some(vec![Inline::Text(journal.to_string())]),
                extra: Vec::new(),
            });
        }
        if let Some(volume) = node.props.get_str("bibtex:volume") {
            record.volume = Some(vec![Inline::Text(volume.to_string())]);
        }
        if let Some(number) = node.props.get_str("bibtex:number") {
            record.number = Some(vec![Inline::Text(number.to_string())]);
        }
        if let Some(pages) = node.props.get_str("bibtex:pages") {
            record.pages = Some(vec![Inline::Text(pages.to_string())]);
        }
        if let Some(publisher) = node.props.get_str("bibtex:publisher") {
            record.publisher = Some(vec![Inline::Text(publisher.to_string())]);
        }
        if let Some(doi) = node.props.get_str("bibtex:doi") {
            record.electronic_resource_num = Some(doi.to_string());
        }
        if let Some(url) = node.props.get_str("bibtex:url") {
            record.urls = Some(Urls {
                related_urls: vec![url.to_string()],
                pdf_urls: Vec::new(),
                extra: Vec::new(),
            });
        }
        if let Some(abs) = node.props.get_str("bibtex:abstract") {
            record.abstract_ = Some(vec![Inline::Text(abs.to_string())]);
        }
        record
    }

    fn ris_entry_to_record(node: &Node, rec_number: usize) -> Record {
        let mut record = Record {
            rec_number: Some(rec_number.to_string()),
            ..Record::default()
        };
        if let Some(ris_type) = node.props.get_str("ris:type") {
            record.ref_type.code = ris_to_endnote_type(ris_type).to_string();
        }
        for (key, value) in node.props.iter() {
            if let Some(tag) = key.strip_prefix("ris:")
                && tag != "type"
                && tag != "key"
                && let rescribe_core::PropValue::String(s) = value
                && let Some(endnote_field) = ris_tag_to_endnote(tag)
            {
                apply_generic_field(&mut record, endnote_field, s);
            }
        }
        record
    }

    fn citation_entry_to_record(node: &Node, rec_number: usize) -> Record {
        let mut record = Record {
            rec_number: Some(rec_number.to_string()),
            ..Record::default()
        };
        if let Some(csl_type) = node.props.get_str("type") {
            record.ref_type.code = csl_to_endnote_type(csl_type).to_string();
        }
        if let Some(title) = node.props.get_str("title") {
            record.titles = Some(Titles {
                title: Some(vec![Inline::Text(title.to_string())]),
                secondary_title: None,
                tertiary_title: None,
                extra: Vec::new(),
            });
        }
        if let Some(author) = node.props.get_str("author") {
            record.contributors = Some(authors_from_str(author));
        }
        if let Some(issued) = node.props.get_str("issued") {
            record.dates = Some(Dates {
                year: Some(vec![Inline::Text(issued.to_string())]),
                pub_date: None,
                extra: Vec::new(),
            });
        }
        if let Some(container) = node.props.get_str("container-title") {
            record.periodical = Some(Periodical {
                full_title: Some(vec![Inline::Text(container.to_string())]),
                extra: Vec::new(),
            });
        }
        if let Some(volume) = node.props.get_str("volume") {
            record.volume = Some(vec![Inline::Text(volume.to_string())]);
        }
        if let Some(page) = node.props.get_str("page") {
            record.pages = Some(vec![Inline::Text(page.to_string())]);
        }
        if let Some(doi) = node.props.get_str("DOI") {
            record.electronic_resource_num = Some(doi.to_string());
        }
        if let Some(url) = node.props.get_str("URL") {
            record.urls = Some(Urls {
                related_urls: vec![url.to_string()],
                pdf_urls: Vec::new(),
                extra: Vec::new(),
            });
        }
        record
    }

    fn typed_entry_to_record(node: &Node, rec_number: usize) -> Record {
        let mut record = Record {
            rec_number: Some(rec_number.to_string()),
            ..Record::default()
        };
        record.ref_type.code = bibtex_to_endnote_type(node.kind.as_str()).to_string();
        if let Some(title) = node.props.get_str("title") {
            record.titles = Some(Titles {
                title: Some(vec![Inline::Text(title.to_string())]),
                secondary_title: None,
                tertiary_title: None,
                extra: Vec::new(),
            });
        }
        if let Some(author) = node.props.get_str("author") {
            record.contributors = Some(authors_from_str(author));
        }
        if let Some(year) = node.props.get_str("year") {
            record.dates = Some(Dates {
                year: Some(vec![Inline::Text(year.to_string())]),
                pub_date: None,
                extra: Vec::new(),
            });
        }
        if let Some(journal) = node.props.get_str("journal") {
            record.periodical = Some(Periodical {
                full_title: Some(vec![Inline::Text(journal.to_string())]),
                extra: Vec::new(),
            });
        }
        if let Some(volume) = node.props.get_str("volume") {
            record.volume = Some(vec![Inline::Text(volume.to_string())]);
        }
        if let Some(number) = node.props.get_str("number") {
            record.number = Some(vec![Inline::Text(number.to_string())]);
        }
        if let Some(pages) = node.props.get_str("pages") {
            record.pages = Some(vec![Inline::Text(pages.to_string())]);
        }
        if let Some(publisher) = node.props.get_str("publisher") {
            record.publisher = Some(vec![Inline::Text(publisher.to_string())]);
        }
        if let Some(doi) = node.props.get_str("doi") {
            record.electronic_resource_num = Some(doi.to_string());
        }
        if let Some(url) = node.props.get_str("url") {
            record.urls = Some(Urls {
                related_urls: vec![url.to_string()],
                pdf_urls: Vec::new(),
                extra: Vec::new(),
            });
        }
        if let Some(abs) = node.props.get_str("abstract") {
            record.abstract_ = Some(vec![Inline::Text(abs.to_string())]);
        }
        record
    }

    /// Apply a single already-mapped EndNote field name (from
    /// `ris_tag_to_endnote`) to a `Record` under construction. Kept generic
    /// (matched by tag name) since `ris:` properties are looped over
    /// dynamically rather than named individually.
    fn apply_generic_field(record: &mut Record, field: &str, value: &str) {
        match field {
            "title" => {
                record.titles = Some(Titles {
                    title: Some(vec![Inline::Text(value.to_string())]),
                    secondary_title: None,
                    tertiary_title: None,
                    extra: Vec::new(),
                });
            }
            "author" => record.contributors = Some(authors_from_str(value)),
            "year" => {
                record.dates = Some(Dates {
                    year: Some(vec![Inline::Text(value.to_string())]),
                    pub_date: None,
                    extra: Vec::new(),
                });
            }
            "secondary-title" => {
                let titles = record.titles.get_or_insert_with(|| Titles {
                    title: None,
                    secondary_title: None,
                    tertiary_title: None,
                    extra: Vec::new(),
                });
                titles.secondary_title = Some(vec![Inline::Text(value.to_string())]);
            }
            "volume" => record.volume = Some(vec![Inline::Text(value.to_string())]),
            "number" => record.number = Some(vec![Inline::Text(value.to_string())]),
            "pages" => record.pages = Some(vec![Inline::Text(value.to_string())]),
            "publisher" => record.publisher = Some(vec![Inline::Text(value.to_string())]),
            "electronic-resource-num" => record.electronic_resource_num = Some(value.to_string()),
            "url" => {
                record.urls = Some(Urls {
                    related_urls: vec![value.to_string()],
                    pdf_urls: Vec::new(),
                    extra: Vec::new(),
                });
            }
            "abstract" => record.abstract_ = Some(vec![Inline::Text(value.to_string())]),
            "keyword" => record.keywords.push(vec![Inline::Text(value.to_string())]),
            "isbn" => record.isbn = Some(value.to_string()),
            _ => {}
        }
    }

    /// Split an author string on `" and "` (BibTeX/CSL convention) or `;`
    /// (RIS/other convention) into individual `authors/author` entries.
    fn authors_from_str(authors: &str) -> Contributors {
        let author_list: Vec<&str> = if authors.contains(" and ") {
            authors.split(" and ").collect()
        } else {
            authors.split(';').collect()
        };
        Contributors {
            authors: author_list
                .into_iter()
                .map(|a| vec![Inline::Text(a.trim().to_string())])
                .collect(),
            secondary_authors: Vec::new(),
            tertiary_authors: Vec::new(),
            subsidiary_authors: Vec::new(),
            extra: Vec::new(),
        }
    }

    fn is_bibtex_type(s: &str) -> bool {
        matches!(
            s.to_lowercase().as_str(),
            "article"
                | "book"
                | "booklet"
                | "conference"
                | "inbook"
                | "incollection"
                | "inproceedings"
                | "manual"
                | "mastersthesis"
                | "misc"
                | "phdthesis"
                | "proceedings"
                | "techreport"
                | "unpublished"
                | "online"
                | "software"
                | "dataset"
        )
    }

    fn bibtex_to_endnote_type(bibtex: &str) -> &'static str {
        match bibtex.to_lowercase().as_str() {
            "article" => "Journal Article",
            "book" => "Book",
            "incollection" | "inbook" => "Book Section",
            "inproceedings" | "conference" => "Conference Paper",
            "phdthesis" => "Thesis",
            "mastersthesis" => "Thesis",
            "techreport" => "Report",
            "online" => "Web Page",
            "software" => "Computer Program",
            "dataset" => "Dataset",
            "unpublished" => "Manuscript",
            "booklet" => "Book",
            "proceedings" => "Conference Proceedings",
            "manual" => "Book",
            _ => "Generic",
        }
    }

    fn ris_to_endnote_type(ris: &str) -> &'static str {
        match ris {
            "JOUR" => "Journal Article",
            "BOOK" => "Book",
            "CHAP" | "SECT" => "Book Section",
            "CONF" | "CPAPER" => "Conference Paper",
            "THES" => "Thesis",
            "RPRT" => "Report",
            "ELEC" | "WEB" => "Web Page",
            "COMP" => "Computer Program",
            "DATA" => "Dataset",
            "MGZN" | "NEWS" => "Magazine Article",
            "UNPB" => "Manuscript",
            _ => "Generic",
        }
    }

    fn csl_to_endnote_type(csl: &str) -> &'static str {
        match csl {
            "article-journal" | "article-magazine" | "article-newspaper" => "Journal Article",
            "book" => "Book",
            "chapter" => "Book Section",
            "paper-conference" => "Conference Paper",
            "thesis" => "Thesis",
            "report" => "Report",
            "webpage" | "post-weblog" => "Web Page",
            "software" => "Computer Program",
            "dataset" => "Dataset",
            _ => "Generic",
        }
    }

    fn ris_tag_to_endnote(tag: &str) -> Option<&'static str> {
        match tag.to_uppercase().as_str() {
            "TI" | "T1" => Some("title"),
            "AU" | "A1" => Some("author"),
            "PY" | "Y1" => Some("year"),
            "JO" | "JF" | "T2" => Some("secondary-title"),
            "VL" => Some("volume"),
            "IS" => Some("number"),
            "SP" | "EP" => Some("pages"),
            "PB" => Some("publisher"),
            "DO" => Some("electronic-resource-num"),
            "UR" => Some("url"),
            "AB" => Some("abstract"),
            "KW" => Some("keyword"),
            "SN" => Some("isbn"),
            _ => None,
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use rescribe_core::NodeKind;

        fn emit_str(doc: &Document) -> String {
            String::from_utf8(emit(doc).unwrap().value).unwrap()
        }

        #[test]
        fn test_emit_empty_document() {
            // A zero-record document omits the `<records>` wrapper — see
            // endnotexml-fmt's `emit()` doc comment: an empty document is
            // indistinguishable, once parsed, from a source with no `<records>`
            // element at all, so this is the same "canonicalize the empty case"
            // convention opml-fmt uses for `Head::is_empty()`. This is a
            // deliberate, documented change from the pre-relocation writer
            // (which always emitted an empty `<records></records>` pair);
            // records themselves are unaffected.
            let doc = Document::new();
            let output = emit_str(&doc);
            assert!(output.contains("<?xml"));
            assert!(output.contains("<xml"));
            assert!(!output.contains("<records"));
        }

        #[test]
        fn test_emit_typed_entry() {
            let entry = Node::new(NodeKind::from("article"))
                .prop("author", "Smith, John")
                .prop("title", "A Great Paper")
                .prop("journal", "Nature")
                .prop("year", "2020");

            let doc =
                Document::new().with_content(Node::new(NodeKind::from("document")).child(entry));

            let output = emit_str(&doc);
            assert!(output.contains("<record>"));
            assert!(output.contains("<ref-type>Journal Article</ref-type>"));
            assert!(output.contains("<author>Smith, John</author>"));
            assert!(output.contains("<title>A Great Paper</title>"));
            assert!(output.contains("<year>2020</year>"));
        }

        #[test]
        fn test_emit_multiple_authors() {
            let entry = Node::new(NodeKind::from("article"))
                .prop("author", "Smith, John and Doe, Jane")
                .prop("title", "Collaborative Work");

            let doc =
                Document::new().with_content(Node::new(NodeKind::from("document")).child(entry));

            let output = emit_str(&doc);
            assert!(output.contains("<author>Smith, John</author>"));
            assert!(output.contains("<author>Doe, Jane</author>"));
        }

        #[test]
        fn test_roundtrip_through_reader() {
            let xml = r#"<xml><records><record>
  <ref-type name="Journal Article">17</ref-type>
  <contributors><authors><author>Smith, John</author></authors></contributors>
  <titles><title>A Great Paper</title></titles>
  <dates><year>2020</year></dates>
</record></records></xml>"#;
            let parsed = super::super::read::parse(xml).unwrap();
            let emitted = emit(&parsed.value).unwrap();
            let xml2 = String::from_utf8(emitted.value).unwrap();
            assert!(xml2.contains("<author>Smith, John</author>"));
            assert!(xml2.contains("<title>A Great Paper</title>"));
            assert!(xml2.contains("<year>2020</year>"));
        }
    }
}

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
pub use read::{parse, parse_with_options};
#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
pub use write::{emit, emit_with_options};
