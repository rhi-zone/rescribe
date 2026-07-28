//! EndNote XML writer for rescribe.
//!
//! Emits documents as EndNote XML bibliography files.

use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use rescribe_core::{
    ConversionResult, Document, EmitError, EmitOptions, FidelityWarning, Node, PropValue,
};
use rescribe_std::{node, prop};
use std::io::Cursor;

/// Legacy flat entry kinds, still accepted for backwards compatibility with
/// documents built by hand or by an older reader version (not produced by
/// `rescribe-read-endnotexml` any more, which now emits `bibliography_entry`).
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
    let mut ctx = EmitContext::new();
    ctx.write_document(doc)?;
    let warnings = std::mem::take(&mut ctx.warnings);
    Ok(ConversionResult::with_warnings(ctx.finish(), warnings))
}

struct EmitContext {
    writer: Writer<Cursor<Vec<u8>>>,
    warnings: Vec<FidelityWarning>,
    record_count: usize,
}

impl EmitContext {
    fn new() -> Self {
        Self {
            writer: Writer::new(Cursor::new(Vec::new())),
            warnings: Vec::new(),
            record_count: 0,
        }
    }

    fn write_document(&mut self, doc: &Document) -> Result<(), EmitError> {
        // XML declaration
        self.writer
            .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
            .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;

        // Root element
        self.writer
            .write_event(Event::Start(BytesStart::new("xml")))
            .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;

        // Records element
        self.writer
            .write_event(Event::Start(BytesStart::new("records")))
            .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;

        // Write all entries
        self.write_nodes(&doc.content.children)?;

        // Close records
        self.writer
            .write_event(Event::End(BytesEnd::new("records")))
            .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;

        // Close root
        self.writer
            .write_event(Event::End(BytesEnd::new("xml")))
            .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;

        Ok(())
    }

    fn write_nodes(&mut self, nodes: &[Node]) -> Result<(), EmitError> {
        for node in nodes {
            self.write_node(node)?;
        }
        Ok(())
    }

    fn write_node(&mut self, node: &Node) -> Result<(), EmitError> {
        match node.kind.as_str() {
            "document" | "definition_list" | node::BIBLIOGRAPHY => {
                self.write_nodes(&node.children)?
            }
            node::BIBLIOGRAPHY_ENTRY => self.write_bibliography_entry(node)?,
            ENDNOTE_ENTRY => self.write_endnote_entry(node)?,
            BIBTEX_ENTRY => self.write_bibtex_entry(node)?,
            RIS_ENTRY => self.write_ris_entry(node)?,
            CITATION_ENTRY => self.write_citation_entry(node)?,
            _ => {
                if is_bibtex_type(node.kind.as_str()) {
                    self.write_typed_entry(node)?;
                } else {
                    self.write_nodes(&node.children)?;
                }
            }
        }
        Ok(())
    }

    /// Write a `bibliography_entry` node (see `rescribe-read-endnotexml`'s
    /// `convert_record`) back to a `<record>`. `endnote:field` on each
    /// `bibliography_field` child (set by every field-producing arm of the
    /// reader) names the exact source element path (`"titles/title"`,
    /// `"urls/related-urls/url"`, ...), so fields are grouped back into
    /// their original nested wrappers instead of guessing a shape from
    /// `field:role` alone. A `page_first`/`page_last` pair recombines into
    /// one `<pages>`; each field's inline children go through
    /// `write_inline_children`, which is the inverse of the reader's
    /// `convert_inline_children` — `emphasis`/`strong`/`underline`/
    /// `superscript`/`subscript` become `<style face="...">` runs.
    fn write_bibliography_entry(&mut self, node: &Node) -> Result<(), EmitError> {
        self.record_count += 1;

        self.writer
            .write_event(Event::Start(BytesStart::new("record")))
            .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;

        let rec_number = node
            .props
            .get_str("endnote:rec-number")
            .map(str::to_string)
            .unwrap_or_else(|| self.record_count.to_string());
        self.write_element("rec-number", &rec_number)?;

        if let Some(label) = node.props.get_str("endnote:label") {
            self.write_element("label", label)?;
        }

        let ref_type = node.props.get_str("endnote:type").unwrap_or("");
        let mut start = BytesStart::new("ref-type");
        if let Some(name) = node.props.get_str("endnote:ref-type-name") {
            start.push_attribute(("name", name));
        }
        self.writer
            .write_event(Event::Start(start))
            .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;
        self.writer
            .write_event(Event::Text(BytesText::new(ref_type)))
            .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;
        self.writer
            .write_event(Event::End(BytesEnd::new("ref-type")))
            .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;

        let mut authors = Vec::new();
        let mut secondary_authors = Vec::new();
        let mut foreign_keys = Vec::new();
        let mut title = None;
        let mut secondary_title = None;
        let mut tertiary_title = None;
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
                "titles/title" => title = Some(child),
                "titles/secondary-title" => secondary_title = Some(child),
                "titles/tertiary-title" => tertiary_title = Some(child),
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
                other => extra.push((other, child)),
            }
        }

        if !authors.is_empty() || !secondary_authors.is_empty() {
            self.writer
                .write_event(Event::Start(BytesStart::new("contributors")))
                .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;
            if !authors.is_empty() {
                self.writer
                    .write_event(Event::Start(BytesStart::new("authors")))
                    .map_err(|e| {
                        EmitError::Io(std::io::Error::other(format!("XML error: {}", e)))
                    })?;
                for author in authors {
                    self.write_field_element("author", author)?;
                }
                self.writer
                    .write_event(Event::End(BytesEnd::new("authors")))
                    .map_err(|e| {
                        EmitError::Io(std::io::Error::other(format!("XML error: {}", e)))
                    })?;
            }
            if !secondary_authors.is_empty() {
                self.writer
                    .write_event(Event::Start(BytesStart::new("secondary-authors")))
                    .map_err(|e| {
                        EmitError::Io(std::io::Error::other(format!("XML error: {}", e)))
                    })?;
                for author in secondary_authors {
                    self.write_field_element("author", author)?;
                }
                self.writer
                    .write_event(Event::End(BytesEnd::new("secondary-authors")))
                    .map_err(|e| {
                        EmitError::Io(std::io::Error::other(format!("XML error: {}", e)))
                    })?;
            }
            self.writer
                .write_event(Event::End(BytesEnd::new("contributors")))
                .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;
        }

        if title.is_some() || secondary_title.is_some() || tertiary_title.is_some() {
            self.writer
                .write_event(Event::Start(BytesStart::new("titles")))
                .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;
            if let Some(t) = title {
                self.write_field_element("title", t)?;
            }
            if let Some(t) = secondary_title {
                self.write_field_element("secondary-title", t)?;
            }
            if let Some(t) = tertiary_title {
                self.write_field_element("tertiary-title", t)?;
            }
            self.writer
                .write_event(Event::End(BytesEnd::new("titles")))
                .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;
        }

        if let Some(PropValue::Map(date)) = node.props.get(prop::DATE) {
            let year = match date.get("year") {
                Some(PropValue::Int(y)) => Some(y.to_string()),
                _ => None,
            };
            if let Some(year) = year {
                self.writer
                    .write_event(Event::Start(BytesStart::new("dates")))
                    .map_err(|e| {
                        EmitError::Io(std::io::Error::other(format!("XML error: {}", e)))
                    })?;
                self.write_element("year", &year)?;
                self.writer
                    .write_event(Event::End(BytesEnd::new("dates")))
                    .map_err(|e| {
                        EmitError::Io(std::io::Error::other(format!("XML error: {}", e)))
                    })?;
            }
        }

        if let Some(v) = volume {
            self.write_field_element("volume", v)?;
        }
        if let Some(n) = number {
            self.write_field_element("number", n)?;
        }
        match (page_first, page_last) {
            (Some(first), Some(last)) => {
                let combined =
                    format!("{}-{}", flatten_field_text(first), flatten_field_text(last));
                self.write_element("pages", &combined)?;
            }
            (Some(first), None) => self.write_field_element("pages", first)?,
            (None, Some(last)) => self.write_field_element("pages", last)?,
            (None, None) => {
                if let Some(p) = pages_misc {
                    self.write_field_element("pages", p)?;
                }
            }
        }
        if let Some(p) = publisher {
            self.write_field_element("publisher", p)?;
        }
        if let Some(p) = pub_location {
            self.write_field_element("pub-location", p)?;
        }
        if let Some(i) = isbn {
            self.write_field_element("isbn", i)?;
        }
        if let Some(i) = issn {
            self.write_field_element("issn", i)?;
        }
        if let Some(d) = doi {
            self.write_field_element("electronic-resource-num", d)?;
        }

        if !related_urls.is_empty() || !pdf_urls.is_empty() {
            self.writer
                .write_event(Event::Start(BytesStart::new("urls")))
                .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;
            if !related_urls.is_empty() {
                self.writer
                    .write_event(Event::Start(BytesStart::new("related-urls")))
                    .map_err(|e| {
                        EmitError::Io(std::io::Error::other(format!("XML error: {}", e)))
                    })?;
                for u in related_urls {
                    self.write_field_element("url", u)?;
                }
                self.writer
                    .write_event(Event::End(BytesEnd::new("related-urls")))
                    .map_err(|e| {
                        EmitError::Io(std::io::Error::other(format!("XML error: {}", e)))
                    })?;
            }
            if !pdf_urls.is_empty() {
                self.writer
                    .write_event(Event::Start(BytesStart::new("pdf-urls")))
                    .map_err(|e| {
                        EmitError::Io(std::io::Error::other(format!("XML error: {}", e)))
                    })?;
                for u in pdf_urls {
                    self.write_field_element("url", u)?;
                }
                self.writer
                    .write_event(Event::End(BytesEnd::new("pdf-urls")))
                    .map_err(|e| {
                        EmitError::Io(std::io::Error::other(format!("XML error: {}", e)))
                    })?;
            }
            self.writer
                .write_event(Event::End(BytesEnd::new("urls")))
                .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;
        }
        if let Some(u) = bare_url {
            self.write_field_element("url", u)?;
        }

        if !foreign_keys.is_empty() {
            self.writer
                .write_event(Event::Start(BytesStart::new("foreign-keys")))
                .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;
            for key in foreign_keys {
                let mut start = BytesStart::new("key");
                if let Some(app) = key.props.get_str("endnote:app") {
                    start.push_attribute(("app", app));
                }
                if let Some(db_id) = key.props.get_str("endnote:db-id") {
                    start.push_attribute(("db-id", db_id));
                }
                self.writer.write_event(Event::Start(start)).map_err(|e| {
                    EmitError::Io(std::io::Error::other(format!("XML error: {}", e)))
                })?;
                self.write_inline_children(&key.children)?;
                self.writer
                    .write_event(Event::End(BytesEnd::new("key")))
                    .map_err(|e| {
                        EmitError::Io(std::io::Error::other(format!("XML error: {}", e)))
                    })?;
            }
            self.writer
                .write_event(Event::End(BytesEnd::new("foreign-keys")))
                .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;
        }

        if let Some(a) = abstract_ {
            self.write_field_element("abstract", a)?;
        }
        if let Some(n) = notes {
            self.write_field_element("notes", n)?;
        }
        if !keywords.is_empty() {
            self.writer
                .write_event(Event::Start(BytesStart::new("keywords")))
                .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;
            for k in keywords {
                self.write_field_element("keyword", k)?;
            }
            self.writer
                .write_event(Event::End(BytesEnd::new("keywords")))
                .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;
        }

        // Everything else (a plain top-level tag name with no "/", from the
        // reader's record-level generic fallback) round-trips as its own
        // bare element.
        for (tag, field) in extra {
            if !tag.contains('/') {
                self.write_field_element(tag, field)?;
            }
        }

        self.writer
            .write_event(Event::End(BytesEnd::new("record")))
            .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;

        Ok(())
    }

    /// Write `field`'s inline children (see `write_inline_children`) inside
    /// a `<name>...</name>` wrapper.
    fn write_field_element(&mut self, name: &str, field: &Node) -> Result<(), EmitError> {
        self.writer
            .write_event(Event::Start(BytesStart::new(name)))
            .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;
        self.write_inline_children(&field.children)?;
        self.writer
            .write_event(Event::End(BytesEnd::new(name)))
            .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;
        Ok(())
    }

    /// Inverse of `rescribe-read-endnotexml`'s `convert_inline_children`:
    /// `TEXT` nodes are written as-is; `emphasis`/`strong`/`underline`/
    /// `superscript`/`subscript` become `<style face="...">` runs around
    /// their (recursively written) content. Any other inline node kind
    /// (from a non-EndNote producer) has its content flattened in rather
    /// than dropped.
    fn write_inline_children(&mut self, children: &[Node]) -> Result<(), EmitError> {
        for child in children {
            let face = match child.kind.as_str() {
                k if k == node::TEXT => {
                    if let Some(content) = child.props.get_str(prop::CONTENT) {
                        self.writer
                            .write_event(Event::Text(BytesText::new(content)))
                            .map_err(|e| {
                                EmitError::Io(std::io::Error::other(format!("XML error: {}", e)))
                            })?;
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
                Some(face) => {
                    let mut start = BytesStart::new("style");
                    start.push_attribute(("face", face));
                    self.writer.write_event(Event::Start(start)).map_err(|e| {
                        EmitError::Io(std::io::Error::other(format!("XML error: {}", e)))
                    })?;
                    self.write_inline_children(&child.children)?;
                    self.writer
                        .write_event(Event::End(BytesEnd::new("style")))
                        .map_err(|e| {
                            EmitError::Io(std::io::Error::other(format!("XML error: {}", e)))
                        })?;
                }
                None => self.write_inline_children(&child.children)?,
            }
        }
        Ok(())
    }

    fn write_endnote_entry(&mut self, node: &Node) -> Result<(), EmitError> {
        self.record_count += 1;

        self.writer
            .write_event(Event::Start(BytesStart::new("record")))
            .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;

        // Record number
        self.write_element("rec-number", &self.record_count.to_string())?;

        // Reference type
        if let Some(ref_type) = node.props.get_str("endnote:type") {
            self.write_element("ref-type", ref_type)?;
        }

        // Write all endnote: prefixed properties
        for (key, value) in node.props.iter() {
            if let Some(field) = key.strip_prefix("endnote:")
                && field != "type"
                && field != "key"
                && let rescribe_core::PropValue::String(s) = value
            {
                self.write_element(field, s)?;
            }
        }

        self.writer
            .write_event(Event::End(BytesEnd::new("record")))
            .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;

        Ok(())
    }

    fn write_bibtex_entry(&mut self, node: &Node) -> Result<(), EmitError> {
        self.record_count += 1;

        self.writer
            .write_event(Event::Start(BytesStart::new("record")))
            .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;

        self.write_element("rec-number", &self.record_count.to_string())?;

        // Convert bibtex type to EndNote type
        if let Some(bibtex_type) = node.props.get_str("bibtex:type") {
            let endnote_type = bibtex_to_endnote_type(bibtex_type);
            self.write_element("ref-type", endnote_type)?;
        }

        // Map bibtex fields
        self.write_bibtex_fields(node)?;

        self.writer
            .write_event(Event::End(BytesEnd::new("record")))
            .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;

        Ok(())
    }

    fn write_bibtex_fields(&mut self, node: &Node) -> Result<(), EmitError> {
        // Titles
        if let Some(title) = node.props.get_str("bibtex:title") {
            self.write_nested_element("titles", "title", title)?;
        }

        // Authors
        if let Some(author) = node.props.get_str("bibtex:author") {
            self.write_authors(author)?;
        }

        // Year
        if let Some(year) = node.props.get_str("bibtex:year") {
            self.write_nested_element("dates", "year", year)?;
        }

        // Journal -> secondary-title
        if let Some(journal) = node.props.get_str("bibtex:journal") {
            self.write_nested_element("periodical", "full-title", journal)?;
        }

        // Volume
        if let Some(volume) = node.props.get_str("bibtex:volume") {
            self.write_element("volume", volume)?;
        }

        // Number
        if let Some(number) = node.props.get_str("bibtex:number") {
            self.write_element("number", number)?;
        }

        // Pages
        if let Some(pages) = node.props.get_str("bibtex:pages") {
            self.write_element("pages", pages)?;
        }

        // Publisher
        if let Some(publisher) = node.props.get_str("bibtex:publisher") {
            self.write_element("publisher", publisher)?;
        }

        // DOI
        if let Some(doi) = node.props.get_str("bibtex:doi") {
            self.write_element("electronic-resource-num", doi)?;
        }

        // URL
        if let Some(url) = node.props.get_str("bibtex:url") {
            self.write_nested_element("urls", "web-urls", url)?;
        }

        // Abstract
        if let Some(abs) = node.props.get_str("bibtex:abstract") {
            self.write_element("abstract", abs)?;
        }

        Ok(())
    }

    fn write_ris_entry(&mut self, node: &Node) -> Result<(), EmitError> {
        self.record_count += 1;

        self.writer
            .write_event(Event::Start(BytesStart::new("record")))
            .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;

        self.write_element("rec-number", &self.record_count.to_string())?;

        // Convert RIS type to EndNote type
        if let Some(ris_type) = node.props.get_str("ris:type") {
            let endnote_type = ris_to_endnote_type(ris_type);
            self.write_element("ref-type", endnote_type)?;
        }

        // Map RIS fields
        for (key, value) in node.props.iter() {
            if let Some(tag) = key.strip_prefix("ris:")
                && tag != "type"
                && tag != "key"
                && let rescribe_core::PropValue::String(s) = value
                && let Some(endnote_field) = ris_tag_to_endnote(tag)
            {
                self.write_element(endnote_field, s)?;
            }
        }

        self.writer
            .write_event(Event::End(BytesEnd::new("record")))
            .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;

        Ok(())
    }

    fn write_citation_entry(&mut self, node: &Node) -> Result<(), EmitError> {
        self.record_count += 1;

        self.writer
            .write_event(Event::Start(BytesStart::new("record")))
            .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;

        self.write_element("rec-number", &self.record_count.to_string())?;

        // Convert CSL type to EndNote type
        if let Some(csl_type) = node.props.get_str("type") {
            let endnote_type = csl_to_endnote_type(csl_type);
            self.write_element("ref-type", endnote_type)?;
        }

        // Map CSL fields
        if let Some(title) = node.props.get_str("title") {
            self.write_nested_element("titles", "title", title)?;
        }
        if let Some(author) = node.props.get_str("author") {
            self.write_authors(author)?;
        }
        if let Some(issued) = node.props.get_str("issued") {
            self.write_nested_element("dates", "year", issued)?;
        }
        if let Some(container) = node.props.get_str("container-title") {
            self.write_nested_element("periodical", "full-title", container)?;
        }
        if let Some(volume) = node.props.get_str("volume") {
            self.write_element("volume", volume)?;
        }
        if let Some(page) = node.props.get_str("page") {
            self.write_element("pages", page)?;
        }
        if let Some(doi) = node.props.get_str("DOI") {
            self.write_element("electronic-resource-num", doi)?;
        }
        if let Some(url) = node.props.get_str("URL") {
            self.write_nested_element("urls", "web-urls", url)?;
        }

        self.writer
            .write_event(Event::End(BytesEnd::new("record")))
            .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;

        Ok(())
    }

    fn write_typed_entry(&mut self, node: &Node) -> Result<(), EmitError> {
        self.record_count += 1;

        self.writer
            .write_event(Event::Start(BytesStart::new("record")))
            .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;

        self.write_element("rec-number", &self.record_count.to_string())?;

        let endnote_type = bibtex_to_endnote_type(node.kind.as_str());
        self.write_element("ref-type", endnote_type)?;

        // Standard fields
        if let Some(title) = node.props.get_str("title") {
            self.write_nested_element("titles", "title", title)?;
        }
        if let Some(author) = node.props.get_str("author") {
            self.write_authors(author)?;
        }
        if let Some(year) = node.props.get_str("year") {
            self.write_nested_element("dates", "year", year)?;
        }
        if let Some(journal) = node.props.get_str("journal") {
            self.write_nested_element("periodical", "full-title", journal)?;
        }
        if let Some(volume) = node.props.get_str("volume") {
            self.write_element("volume", volume)?;
        }
        if let Some(number) = node.props.get_str("number") {
            self.write_element("number", number)?;
        }
        if let Some(pages) = node.props.get_str("pages") {
            self.write_element("pages", pages)?;
        }
        if let Some(publisher) = node.props.get_str("publisher") {
            self.write_element("publisher", publisher)?;
        }
        if let Some(doi) = node.props.get_str("doi") {
            self.write_element("electronic-resource-num", doi)?;
        }
        if let Some(url) = node.props.get_str("url") {
            self.write_nested_element("urls", "web-urls", url)?;
        }
        if let Some(abs) = node.props.get_str("abstract") {
            self.write_element("abstract", abs)?;
        }

        self.writer
            .write_event(Event::End(BytesEnd::new("record")))
            .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;

        Ok(())
    }

    fn write_element(&mut self, name: &str, value: &str) -> Result<(), EmitError> {
        self.writer
            .write_event(Event::Start(BytesStart::new(name)))
            .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;
        self.writer
            .write_event(Event::Text(BytesText::new(value)))
            .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;
        self.writer
            .write_event(Event::End(BytesEnd::new(name)))
            .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;
        Ok(())
    }

    fn write_nested_element(
        &mut self,
        parent: &str,
        child: &str,
        value: &str,
    ) -> Result<(), EmitError> {
        self.writer
            .write_event(Event::Start(BytesStart::new(parent)))
            .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;
        self.write_element(child, value)?;
        self.writer
            .write_event(Event::End(BytesEnd::new(parent)))
            .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;
        Ok(())
    }

    fn write_authors(&mut self, authors: &str) -> Result<(), EmitError> {
        self.writer
            .write_event(Event::Start(BytesStart::new("contributors")))
            .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;
        self.writer
            .write_event(Event::Start(BytesStart::new("authors")))
            .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;

        // Split by " and " or ";"
        let author_list: Vec<&str> = if authors.contains(" and ") {
            authors.split(" and ").collect()
        } else {
            authors.split(';').collect()
        };

        for author in author_list {
            self.write_element("author", author.trim())?;
        }

        self.writer
            .write_event(Event::End(BytesEnd::new("authors")))
            .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;
        self.writer
            .write_event(Event::End(BytesEnd::new("contributors")))
            .map_err(|e| EmitError::Io(std::io::Error::other(format!("XML error: {}", e))))?;

        Ok(())
    }

    fn finish(self) -> Vec<u8> {
        self.writer.into_inner().into_inner()
    }
}

/// Concatenate a field's descendant `TEXT` node content (depth-first) —
/// used only for recombining a `page_first`/`page_last` pair back into one
/// `<pages>first-last</pages>` string, where page numbers are never
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
        let doc = Document::new();
        let output = emit_str(&doc);
        assert!(output.contains("<?xml"));
        assert!(output.contains("<records"));
        assert!(output.contains("</records>"));
    }

    #[test]
    fn test_emit_typed_entry() {
        let entry = Node::new(NodeKind::from("article"))
            .prop("author", "Smith, John")
            .prop("title", "A Great Paper")
            .prop("journal", "Nature")
            .prop("year", "2020");

        let doc = Document::new().with_content(Node::new(NodeKind::from("document")).child(entry));

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

        let doc = Document::new().with_content(Node::new(NodeKind::from("document")).child(entry));

        let output = emit_str(&doc);
        assert!(output.contains("<author>Smith, John</author>"));
        assert!(output.contains("<author>Doe, Jane</author>"));
    }
}
