//! EndNote XML writer for rescribe.
//!
//! Thin IR→AST adapter over [`endnotexml_fmt`] (the standalone EndNote XML
//! parser/AST/emitter crate). All XML emission lives in `endnotexml-fmt`;
//! this crate only translates rescribe's `Document` into
//! `endnotexml_fmt::EndNoteDoc`, then calls `endnotexml_fmt::emit`. Per
//! CLAUDE.md's adapter-layer rule, no `quick_xml` appears in this crate's
//! production code.
//!
//! # Mapping (inverse of `rescribe-read-endnotexml`)
//!
//! `endnote:field` on each `bibliography_field` child (set by every
//! field-producing arm of the reader) names the exact source element path
//! (`"titles/title"`, `"urls/related-urls/url"`, ...), so fields are
//! grouped back into their original nested wrappers instead of guessing a
//! shape from `field:role` alone. A `page_first`/`page_last` pair
//! recombines into one `<pages>`; each field's inline children go through
//! [`ir_to_inline`], the inverse of the reader's `inline_to_ir` —
//! `emphasis`/`strong`/`underline`/`superscript`/`subscript` become
//! `<style face="...">` runs.
//!
//! The legacy flat `endnote:entry`/`bibtex:entry`/`ris:entry`/
//! `citation_entry` fallback paths are kept for documents built by other
//! producers (not `rescribe-read-endnotexml`, which emits
//! `bibliography_entry`); only the primary `bibliography_entry` path
//! changed when this crate was relocated onto `endnotexml-fmt`.

use endnotexml_fmt::{
    Contributors, Dates, Element, EndNoteDoc, ForeignKey, ForeignKeys, Inline, Periodical, Record,
    RefType, Titles, Urls,
};
use rescribe_core::{
    ConversionResult, Document, EmitError, EmitOptions, FidelityWarning, Node, PropValue,
};
use rescribe_std::{node, prop};

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
    let mut ctx = EmitContext::default();
    ctx.write_document(doc);
    let records = std::mem::take(&mut ctx.records);
    let warnings = std::mem::take(&mut ctx.warnings);

    let ast = EndNoteDoc {
        xml_decl: Some(endnotexml_fmt::XmlDecl {
            version: "1.0".to_string(),
            encoding: Some("UTF-8".to_string()),
            standalone: None,
        }),
        records,
        span: endnotexml_fmt::Span::NONE,
    };
    let bytes = endnotexml_fmt::emit(&ast);
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
            "document" | "definition_list" | node::BIBLIOGRAPHY => self.write_nodes(&node.children),
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
/// `rescribe-read-endnotexml`'s `convert_record`) — the inverse of that
/// reader, grouping fields back by their `endnote:field` tag into the
/// nested wrappers `endnotexml_fmt::emit` expects.
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

    let dates =
        if dates_year.is_some() || dates_pub_date.is_some() || node.props.get(prop::DATE).is_some()
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
        span: endnotexml_fmt::Span::NONE,
    }
}

/// Inverse of `rescribe-read-endnotexml`'s `inline_to_ir`: `TEXT` nodes
/// become `Inline::Text`; `emphasis`/`strong`/`underline`/`superscript`/
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

    #[test]
    fn test_roundtrip_through_reader() {
        let xml = r#"<xml><records><record>
  <ref-type name="Journal Article">17</ref-type>
  <contributors><authors><author>Smith, John</author></authors></contributors>
  <titles><title>A Great Paper</title></titles>
  <dates><year>2020</year></dates>
</record></records></xml>"#;
        let parsed = rescribe_read_endnotexml::parse(xml).unwrap();
        let emitted = emit(&parsed.value).unwrap();
        let xml2 = String::from_utf8(emitted.value).unwrap();
        assert!(xml2.contains("<author>Smith, John</author>"));
        assert!(xml2.contains("<title>A Great Paper</title>"));
        assert!(xml2.contains("<year>2020</year>"));
    }
}
