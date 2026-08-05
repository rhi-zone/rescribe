//! EndNote XML parser using `quick-xml`'s pull-based `Reader`.
//!
//! Drives `quick_xml::Reader` directly into [`EndNoteDoc`] via a set of
//! mutually-recursive functions, one per container element
//! (`read_record`/`read_contributors`/`read_titles`/`read_urls`/...) — no
//! intermediate generic-XML tree. Each function reads events until it sees
//! its own element's matching `End` tag, dispatching known children to
//! dedicated fields and anything else to that level's `extra: Vec<Element>`
//! bucket (see `ast.rs`'s module docs). This is a true direct recursive
//! descent, which is also why [`crate::events()`] (`events.rs`) can be a
//! genuinely independent implementation: both drive `quick_xml::Reader` from
//! scratch, they just build different things from what it yields.
//!
//! `trim_text` stays off: EndNote field content can be split across several
//! `<style>` runs (`<style face="normal">A </style><style
//! face="italic">Great</style>`), and a global trim would eat the
//! meaningful inter-run space along with harmless inter-element indentation
//! whitespace — the two are indistinguishable to the parser. This never
//! causes a problem for record-structural whitespace since every container
//! reader below dispatches only on recognized element names, silently
//! skipping stray whitespace-only text between them (any structural
//! whitespace source XML has is simply appended to `extra`/`Inline::Text` at
//! whatever level it's nested in, which is the same "don't guess, preserve
//! it" choice `ast.rs` documents for `Element`/`Inline::Other`).

use quick_xml::Reader;
use quick_xml::events::Event as XmlEvent;

use crate::ast::*;

/// Parse an EndNote XML document from bytes (assumed UTF-8).
///
/// Never panics: malformed XML is reported via `Diagnostic`s and parsing
/// stops at the point of failure, returning whatever tree was built so far.
pub(crate) fn parse(input: &[u8]) -> (EndNoteDoc, Vec<Diagnostic>) {
    let mut reader = Reader::from_reader(input);
    reader.config_mut().trim_text(false);

    let mut diagnostics = Vec::new();
    let mut xml_decl = None;
    let mut records = Vec::new();
    let mut doc_start = 0usize;
    let mut doc_end = 0usize;
    let mut buf = Vec::new();

    loop {
        let pos = reader.buffer_position() as usize;
        match reader.read_event_into(&mut buf) {
            Ok(XmlEvent::Decl(decl)) => {
                xml_decl = Some(decode_decl(&decl));
            }
            Ok(XmlEvent::Start(e)) => {
                let name = local_name(&e);
                match name.as_str() {
                    "xml" => doc_start = pos,
                    "record" => {
                        let record = read_record(&mut reader, &mut buf, &mut diagnostics, pos);
                        records.push(record);
                    }
                    // "records" and anything else at the top level: not
                    // modeled beyond entering it (EndNote's schema has no
                    // meaningful content directly under <records> other than
                    // <record>, and no meaningful sibling content of <xml>
                    // in practice — mirrors opml-fmt's documented scope
                    // limit on out-of-grammar top-level content).
                    _ => {}
                }
            }
            Ok(XmlEvent::Eof) => {
                doc_end = reader.buffer_position() as usize;
                break;
            }
            Ok(_) => {}
            Err(e) => {
                diagnostics.push(Diagnostic {
                    message: format!("XML parse error: {e}"),
                    span: Span {
                        start: pos,
                        end: pos,
                    },
                    severity: rescribe_format_api::Severity::Warning,
                    code: "",
                });
                break;
            }
        }
        buf.clear();
    }

    (
        EndNoteDoc {
            xml_decl,
            records,
            span: Span {
                start: doc_start,
                end: doc_end,
            },
        },
        diagnostics,
    )
}

fn read_record(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    diagnostics: &mut Vec<Diagnostic>,
    start: usize,
) -> Record {
    let mut rec = Record::default();
    loop {
        match reader.read_event_into(buf) {
            Ok(XmlEvent::Start(e)) => {
                let name = local_name(&e);
                match name.as_str() {
                    "ref-type" => {
                        let ref_name = attr_value(&e, "name");
                        let inline = read_inline_until(reader, buf, "ref-type", diagnostics);
                        rec.ref_type = RefType {
                            code: flatten_inline_text(&inline),
                            name: ref_name,
                        };
                    }
                    "rec-number" => {
                        let inline = read_inline_until(reader, buf, "rec-number", diagnostics);
                        rec.rec_number = Some(flatten_inline_text(&inline));
                    }
                    "label" => {
                        let inline = read_inline_until(reader, buf, "label", diagnostics);
                        rec.label = Some(flatten_inline_text(&inline));
                    }
                    "foreign-keys" => {
                        rec.foreign_keys = Some(read_foreign_keys(reader, buf, diagnostics));
                    }
                    "contributors" => {
                        rec.contributors = Some(read_contributors(reader, buf, diagnostics));
                    }
                    "titles" => {
                        rec.titles = Some(read_titles(reader, buf, diagnostics));
                    }
                    "periodical" => {
                        rec.periodical = Some(read_periodical(reader, buf, diagnostics));
                    }
                    "volume" => {
                        rec.volume = Some(read_inline_until(reader, buf, "volume", diagnostics));
                    }
                    "number" => {
                        rec.number = Some(read_inline_until(reader, buf, "number", diagnostics));
                    }
                    "pages" => {
                        rec.pages = Some(read_inline_until(reader, buf, "pages", diagnostics));
                    }
                    "publisher" => {
                        rec.publisher =
                            Some(read_inline_until(reader, buf, "publisher", diagnostics));
                    }
                    "pub-location" => {
                        rec.pub_location =
                            Some(read_inline_until(reader, buf, "pub-location", diagnostics));
                    }
                    "isbn" => {
                        let inline = read_inline_until(reader, buf, "isbn", diagnostics);
                        rec.isbn = Some(flatten_inline_text(&inline));
                    }
                    "issn" => {
                        let inline = read_inline_until(reader, buf, "issn", diagnostics);
                        rec.issn = Some(flatten_inline_text(&inline));
                    }
                    "electronic-resource-num" => {
                        let inline =
                            read_inline_until(reader, buf, "electronic-resource-num", diagnostics);
                        rec.electronic_resource_num = Some(flatten_inline_text(&inline));
                    }
                    "urls" => {
                        rec.urls = Some(read_urls(reader, buf, diagnostics));
                    }
                    "url" => {
                        let inline = read_inline_until(reader, buf, "url", diagnostics);
                        rec.bare_url = Some(flatten_inline_text(&inline));
                    }
                    "abstract" => {
                        rec.abstract_ =
                            Some(read_inline_until(reader, buf, "abstract", diagnostics));
                    }
                    "notes" => {
                        rec.notes = Some(read_inline_until(reader, buf, "notes", diagnostics));
                    }
                    "keywords" => {
                        rec.keywords = read_keywords(reader, buf, diagnostics, &mut rec.extra);
                    }
                    "dates" => {
                        rec.dates = Some(read_dates(reader, buf, diagnostics));
                    }
                    other => {
                        let attrs = read_attrs(&e);
                        let children = read_inline_until(reader, buf, other, diagnostics);
                        rec.extra.push(Element {
                            name: other.to_string(),
                            attrs,
                            children,
                        });
                    }
                }
            }
            Ok(XmlEvent::Empty(e)) => {
                let name = local_name(&e);
                match name.as_str() {
                    "ref-type" => {
                        rec.ref_type = RefType {
                            code: String::new(),
                            name: attr_value(&e, "name"),
                        };
                    }
                    "rec-number" => rec.rec_number = Some(String::new()),
                    "label" => rec.label = Some(String::new()),
                    "foreign-keys" => rec.foreign_keys = Some(ForeignKeys::default()),
                    "contributors" => rec.contributors = Some(Contributors::default()),
                    "titles" => rec.titles = Some(Titles::default()),
                    "periodical" => rec.periodical = Some(Periodical::default()),
                    "volume" => rec.volume = Some(Vec::new()),
                    "number" => rec.number = Some(Vec::new()),
                    "pages" => rec.pages = Some(Vec::new()),
                    "publisher" => rec.publisher = Some(Vec::new()),
                    "pub-location" => rec.pub_location = Some(Vec::new()),
                    "isbn" => rec.isbn = Some(String::new()),
                    "issn" => rec.issn = Some(String::new()),
                    "electronic-resource-num" => rec.electronic_resource_num = Some(String::new()),
                    "urls" => rec.urls = Some(Urls::default()),
                    "url" => rec.bare_url = Some(String::new()),
                    "abstract" => rec.abstract_ = Some(Vec::new()),
                    "notes" => rec.notes = Some(Vec::new()),
                    "keywords" => {}
                    "dates" => rec.dates = Some(Dates::default()),
                    other => {
                        let attrs = read_attrs(&e);
                        rec.extra.push(Element {
                            name: other.to_string(),
                            attrs,
                            children: Vec::new(),
                        });
                    }
                }
            }
            Ok(XmlEvent::End(e)) => {
                if end_local_name(&e) == "record" {
                    rec.span = Span {
                        start,
                        end: reader.buffer_position() as usize,
                    };
                    return rec;
                }
            }
            Ok(XmlEvent::Eof) => {
                diagnostics.push(Diagnostic {
                    message: "unclosed element <record>".to_string(),
                    span: Span::NONE,
                    severity: rescribe_format_api::Severity::Warning,
                    code: "",
                });
                rec.span = Span {
                    start,
                    end: reader.buffer_position() as usize,
                };
                return rec;
            }
            Err(e) => {
                diagnostics.push(Diagnostic {
                    message: format!("XML parse error: {e}"),
                    span: Span::NONE,
                    severity: rescribe_format_api::Severity::Warning,
                    code: "",
                });
                return rec;
            }
            Ok(_) => {}
        }
        buf.clear();
    }
}

fn read_foreign_keys(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    diagnostics: &mut Vec<Diagnostic>,
) -> ForeignKeys {
    let mut fk = ForeignKeys::default();
    loop {
        match reader.read_event_into(buf) {
            Ok(XmlEvent::Start(e)) => {
                let name = local_name(&e);
                if name == "key" {
                    let app = attr_value(&e, "app");
                    let db_id = attr_value(&e, "db-id");
                    let inline = read_inline_until(reader, buf, "key", diagnostics);
                    fk.keys.push(ForeignKey {
                        app,
                        db_id,
                        text: flatten_inline_text(&inline),
                    });
                } else {
                    let attrs = read_attrs(&e);
                    let children = read_inline_until(reader, buf, &name, diagnostics);
                    fk.extra.push(Element {
                        name,
                        attrs,
                        children,
                    });
                }
            }
            Ok(XmlEvent::Empty(e)) => {
                let name = local_name(&e);
                if name == "key" {
                    fk.keys.push(ForeignKey {
                        app: attr_value(&e, "app"),
                        db_id: attr_value(&e, "db-id"),
                        text: String::new(),
                    });
                } else {
                    let attrs = read_attrs(&e);
                    fk.extra.push(Element {
                        name,
                        attrs,
                        children: Vec::new(),
                    });
                }
            }
            Ok(XmlEvent::End(e)) => {
                if end_local_name(&e) == "foreign-keys" {
                    return fk;
                }
            }
            Ok(XmlEvent::Eof) => return fk,
            Err(_) => return fk,
            Ok(_) => {}
        }
        buf.clear();
    }
}

fn read_author_role_list(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    diagnostics: &mut Vec<Diagnostic>,
    end_name: &str,
    extra: &mut Vec<Element>,
) -> Vec<Vec<Inline>> {
    let mut out = Vec::new();
    loop {
        match reader.read_event_into(buf) {
            Ok(XmlEvent::Start(e)) => {
                let name = local_name(&e);
                if name == "author" {
                    out.push(read_inline_until(reader, buf, "author", diagnostics));
                } else {
                    let attrs = read_attrs(&e);
                    let children = read_inline_until(reader, buf, &name, diagnostics);
                    extra.push(Element {
                        name,
                        attrs,
                        children,
                    });
                }
            }
            Ok(XmlEvent::Empty(e)) => {
                let name = local_name(&e);
                if name == "author" {
                    out.push(Vec::new());
                } else {
                    let attrs = read_attrs(&e);
                    extra.push(Element {
                        name,
                        attrs,
                        children: Vec::new(),
                    });
                }
            }
            Ok(XmlEvent::End(e)) => {
                if end_local_name(&e) == end_name {
                    return out;
                }
            }
            Ok(XmlEvent::Eof) => return out,
            Err(_) => return out,
            Ok(_) => {}
        }
        buf.clear();
    }
}

fn read_contributors(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Contributors {
    let mut c = Contributors::default();
    loop {
        match reader.read_event_into(buf) {
            Ok(XmlEvent::Start(e)) => {
                let name = local_name(&e);
                match name.as_str() {
                    "authors" => {
                        c.authors = read_author_role_list(
                            reader,
                            buf,
                            diagnostics,
                            "authors",
                            &mut c.extra,
                        );
                    }
                    "secondary-authors" => {
                        c.secondary_authors = read_author_role_list(
                            reader,
                            buf,
                            diagnostics,
                            "secondary-authors",
                            &mut c.extra,
                        );
                    }
                    "tertiary-authors" => {
                        c.tertiary_authors = read_author_role_list(
                            reader,
                            buf,
                            diagnostics,
                            "tertiary-authors",
                            &mut c.extra,
                        );
                    }
                    "subsidiary-authors" => {
                        c.subsidiary_authors = read_author_role_list(
                            reader,
                            buf,
                            diagnostics,
                            "subsidiary-authors",
                            &mut c.extra,
                        );
                    }
                    other => {
                        let attrs = read_attrs(&e);
                        let children = read_inline_until(reader, buf, other, diagnostics);
                        c.extra.push(Element {
                            name: other.to_string(),
                            attrs,
                            children,
                        });
                    }
                }
            }
            Ok(XmlEvent::Empty(e)) => {
                let name = local_name(&e);
                match name.as_str() {
                    "authors" | "secondary-authors" | "tertiary-authors" | "subsidiary-authors" => {
                    }
                    other => {
                        let attrs = read_attrs(&e);
                        c.extra.push(Element {
                            name: other.to_string(),
                            attrs,
                            children: Vec::new(),
                        });
                    }
                }
            }
            Ok(XmlEvent::End(e)) => {
                if end_local_name(&e) == "contributors" {
                    return c;
                }
            }
            Ok(XmlEvent::Eof) => return c,
            Err(_) => return c,
            Ok(_) => {}
        }
        buf.clear();
    }
}

fn read_titles(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Titles {
    let mut t = Titles::default();
    loop {
        match reader.read_event_into(buf) {
            Ok(XmlEvent::Start(e)) => {
                let name = local_name(&e);
                match name.as_str() {
                    "title" => {
                        t.title = Some(read_inline_until(reader, buf, "title", diagnostics));
                    }
                    "secondary-title" => {
                        t.secondary_title = Some(read_inline_until(
                            reader,
                            buf,
                            "secondary-title",
                            diagnostics,
                        ));
                    }
                    "tertiary-title" => {
                        t.tertiary_title = Some(read_inline_until(
                            reader,
                            buf,
                            "tertiary-title",
                            diagnostics,
                        ));
                    }
                    other => {
                        let attrs = read_attrs(&e);
                        let children = read_inline_until(reader, buf, other, diagnostics);
                        t.extra.push(Element {
                            name: other.to_string(),
                            attrs,
                            children,
                        });
                    }
                }
            }
            Ok(XmlEvent::Empty(e)) => {
                let name = local_name(&e);
                match name.as_str() {
                    "title" => t.title = Some(Vec::new()),
                    "secondary-title" => t.secondary_title = Some(Vec::new()),
                    "tertiary-title" => t.tertiary_title = Some(Vec::new()),
                    other => {
                        let attrs = read_attrs(&e);
                        t.extra.push(Element {
                            name: other.to_string(),
                            attrs,
                            children: Vec::new(),
                        });
                    }
                }
            }
            Ok(XmlEvent::End(e)) => {
                if end_local_name(&e) == "titles" {
                    return t;
                }
            }
            Ok(XmlEvent::Eof) => return t,
            Err(_) => return t,
            Ok(_) => {}
        }
        buf.clear();
    }
}

fn read_periodical(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Periodical {
    let mut p = Periodical::default();
    loop {
        match reader.read_event_into(buf) {
            Ok(XmlEvent::Start(e)) => {
                let name = local_name(&e);
                if name == "full-title" {
                    p.full_title = Some(read_inline_until(reader, buf, "full-title", diagnostics));
                } else {
                    let attrs = read_attrs(&e);
                    let children = read_inline_until(reader, buf, &name, diagnostics);
                    p.extra.push(Element {
                        name,
                        attrs,
                        children,
                    });
                }
            }
            Ok(XmlEvent::Empty(e)) => {
                let name = local_name(&e);
                if name == "full-title" {
                    p.full_title = Some(Vec::new());
                } else {
                    let attrs = read_attrs(&e);
                    p.extra.push(Element {
                        name,
                        attrs,
                        children: Vec::new(),
                    });
                }
            }
            Ok(XmlEvent::End(e)) => {
                if end_local_name(&e) == "periodical" {
                    return p;
                }
            }
            Ok(XmlEvent::Eof) => return p,
            Err(_) => return p,
            Ok(_) => {}
        }
        buf.clear();
    }
}

fn read_url_list(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    diagnostics: &mut Vec<Diagnostic>,
    end_name: &str,
    extra: &mut Vec<Element>,
) -> Vec<String> {
    let mut out = Vec::new();
    loop {
        match reader.read_event_into(buf) {
            Ok(XmlEvent::Start(e)) => {
                let name = local_name(&e);
                if name == "url" {
                    let inline = read_inline_until(reader, buf, "url", diagnostics);
                    out.push(flatten_inline_text(&inline));
                } else {
                    let attrs = read_attrs(&e);
                    let children = read_inline_until(reader, buf, &name, diagnostics);
                    extra.push(Element {
                        name,
                        attrs,
                        children,
                    });
                }
            }
            Ok(XmlEvent::Empty(e)) => {
                let name = local_name(&e);
                if name == "url" {
                    out.push(String::new());
                } else {
                    let attrs = read_attrs(&e);
                    extra.push(Element {
                        name,
                        attrs,
                        children: Vec::new(),
                    });
                }
            }
            Ok(XmlEvent::End(e)) => {
                if end_local_name(&e) == end_name {
                    return out;
                }
            }
            Ok(XmlEvent::Eof) => return out,
            Err(_) => return out,
            Ok(_) => {}
        }
        buf.clear();
    }
}

fn read_urls(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Urls {
    let mut u = Urls::default();
    loop {
        match reader.read_event_into(buf) {
            Ok(XmlEvent::Start(e)) => {
                let name = local_name(&e);
                match name.as_str() {
                    "related-urls" => {
                        u.related_urls =
                            read_url_list(reader, buf, diagnostics, "related-urls", &mut u.extra);
                    }
                    "pdf-urls" => {
                        u.pdf_urls =
                            read_url_list(reader, buf, diagnostics, "pdf-urls", &mut u.extra);
                    }
                    other => {
                        let attrs = read_attrs(&e);
                        let children = read_inline_until(reader, buf, other, diagnostics);
                        u.extra.push(Element {
                            name: other.to_string(),
                            attrs,
                            children,
                        });
                    }
                }
            }
            Ok(XmlEvent::Empty(e)) => {
                let name = local_name(&e);
                match name.as_str() {
                    "related-urls" | "pdf-urls" => {}
                    other => {
                        let attrs = read_attrs(&e);
                        u.extra.push(Element {
                            name: other.to_string(),
                            attrs,
                            children: Vec::new(),
                        });
                    }
                }
            }
            Ok(XmlEvent::End(e)) => {
                if end_local_name(&e) == "urls" {
                    return u;
                }
            }
            Ok(XmlEvent::Eof) => return u,
            Err(_) => return u,
            Ok(_) => {}
        }
        buf.clear();
    }
}

fn read_keywords(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    diagnostics: &mut Vec<Diagnostic>,
    extra: &mut Vec<Element>,
) -> Vec<Vec<Inline>> {
    let mut out = Vec::new();
    loop {
        match reader.read_event_into(buf) {
            Ok(XmlEvent::Start(e)) => {
                let name = local_name(&e);
                if name == "keyword" {
                    out.push(read_inline_until(reader, buf, "keyword", diagnostics));
                } else {
                    let attrs = read_attrs(&e);
                    let children = read_inline_until(reader, buf, &name, diagnostics);
                    extra.push(Element {
                        name,
                        attrs,
                        children,
                    });
                }
            }
            Ok(XmlEvent::Empty(e)) => {
                let name = local_name(&e);
                if name == "keyword" {
                    out.push(Vec::new());
                } else {
                    let attrs = read_attrs(&e);
                    extra.push(Element {
                        name,
                        attrs,
                        children: Vec::new(),
                    });
                }
            }
            Ok(XmlEvent::End(e)) => {
                if end_local_name(&e) == "keywords" {
                    return out;
                }
            }
            Ok(XmlEvent::Eof) => return out,
            Err(_) => return out,
            Ok(_) => {}
        }
        buf.clear();
    }
}

fn read_pub_dates(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    diagnostics: &mut Vec<Diagnostic>,
    extra: &mut Vec<Element>,
) -> Option<Vec<Inline>> {
    let mut date = None;
    loop {
        match reader.read_event_into(buf) {
            Ok(XmlEvent::Start(e)) => {
                let name = local_name(&e);
                if name == "date" {
                    let inline = read_inline_until(reader, buf, "date", diagnostics);
                    if date.is_none() {
                        date = Some(inline);
                    } else {
                        // The schema allows exactly one <date> in practice;
                        // a second one is preserved rather than dropped.
                        extra.push(Element {
                            name,
                            attrs: Vec::new(),
                            children: inline,
                        });
                    }
                } else {
                    let attrs = read_attrs(&e);
                    let children = read_inline_until(reader, buf, &name, diagnostics);
                    extra.push(Element {
                        name,
                        attrs,
                        children,
                    });
                }
            }
            Ok(XmlEvent::Empty(e)) => {
                let name = local_name(&e);
                if name == "date" {
                    if date.is_none() {
                        date = Some(Vec::new());
                    }
                } else {
                    let attrs = read_attrs(&e);
                    extra.push(Element {
                        name,
                        attrs,
                        children: Vec::new(),
                    });
                }
            }
            Ok(XmlEvent::End(e)) => {
                if end_local_name(&e) == "pub-dates" {
                    return date;
                }
            }
            Ok(XmlEvent::Eof) => return date,
            Err(_) => return date,
            Ok(_) => {}
        }
        buf.clear();
    }
}

fn read_dates(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Dates {
    let mut d = Dates::default();
    loop {
        match reader.read_event_into(buf) {
            Ok(XmlEvent::Start(e)) => {
                let name = local_name(&e);
                match name.as_str() {
                    "year" => {
                        d.year = Some(read_inline_until(reader, buf, "year", diagnostics));
                    }
                    "pub-dates" => {
                        d.pub_date = read_pub_dates(reader, buf, diagnostics, &mut d.extra);
                    }
                    other => {
                        let attrs = read_attrs(&e);
                        let children = read_inline_until(reader, buf, other, diagnostics);
                        d.extra.push(Element {
                            name: other.to_string(),
                            attrs,
                            children,
                        });
                    }
                }
            }
            Ok(XmlEvent::Empty(e)) => {
                let name = local_name(&e);
                match name.as_str() {
                    "year" => d.year = Some(Vec::new()),
                    "pub-dates" => {}
                    other => {
                        let attrs = read_attrs(&e);
                        d.extra.push(Element {
                            name: other.to_string(),
                            attrs,
                            children: Vec::new(),
                        });
                    }
                }
            }
            Ok(XmlEvent::End(e)) => {
                if end_local_name(&e) == "dates" {
                    return d;
                }
            }
            Ok(XmlEvent::Eof) => return d,
            Err(_) => return d,
            Ok(_) => {}
        }
        buf.clear();
    }
}

/// Read a field's inline content (text interleaved with `<style>` runs and
/// any other nested elements) until `end_name`'s closing tag. Used for
/// every leaf field and for building each level's `extra: Vec<Element>`
/// entries.
fn read_inline_until(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    end_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<Inline> {
    let mut out = Vec::new();
    loop {
        match reader.read_event_into(buf) {
            Ok(XmlEvent::Text(t)) => {
                let content = t
                    .decode()
                    .map(|c| c.into_owned())
                    .unwrap_or_else(|_| String::from_utf8_lossy(t.as_ref()).into_owned());
                if !content.is_empty() {
                    out.push(Inline::Text(content));
                }
            }
            Ok(XmlEvent::CData(t)) => {
                let content = String::from_utf8_lossy(t.as_ref()).into_owned();
                if !content.is_empty() {
                    out.push(Inline::Text(content));
                }
            }
            Ok(XmlEvent::Start(e)) => {
                let name = local_name(&e);
                if name == "style" {
                    let face = attr_value(&e, "face").unwrap_or_default();
                    let children = read_inline_until(reader, buf, "style", diagnostics);
                    out.push(Inline::Style { face, children });
                } else {
                    let attrs = read_attrs(&e);
                    let children = read_inline_until(reader, buf, &name, diagnostics);
                    out.push(Inline::Other {
                        name,
                        attrs,
                        children,
                    });
                }
            }
            Ok(XmlEvent::Empty(e)) => {
                let name = local_name(&e);
                if name == "style" {
                    out.push(Inline::Style {
                        face: attr_value(&e, "face").unwrap_or_default(),
                        children: Vec::new(),
                    });
                } else {
                    let attrs = read_attrs(&e);
                    out.push(Inline::Other {
                        name,
                        attrs,
                        children: Vec::new(),
                    });
                }
            }
            Ok(XmlEvent::End(e)) => {
                if end_local_name(&e) == end_name {
                    return out;
                }
                // A mismatched close tag at this depth means malformed
                // input; recover by treating it as this field's end rather
                // than looping forever.
                return out;
            }
            Ok(XmlEvent::Eof) => {
                diagnostics.push(Diagnostic {
                    message: format!("unclosed element <{end_name}>"),
                    span: Span::NONE,
                    severity: rescribe_format_api::Severity::Warning,
                    code: "",
                });
                return out;
            }
            Err(e) => {
                diagnostics.push(Diagnostic {
                    message: format!("XML parse error: {e}"),
                    span: Span::NONE,
                    severity: rescribe_format_api::Severity::Warning,
                    code: "",
                });
                return out;
            }
            Ok(_) => {}
        }
        buf.clear();
    }
}

/// Flatten all descendant text (ignoring element boundaries), used for
/// fields never expected to carry markup (record numbers, ISBN/ISSN, URLs,
/// DOIs, ref-type codes). Public within the crate: also used by
/// `events.rs`'s `collect_doc` for the same fields, and by
/// `rescribe-read-endnotexml` for e.g. cite-key generation.
pub fn flatten_inline_text(inline: &[Inline]) -> String {
    let mut out = String::new();
    for i in inline {
        flatten_inline_text_into(i, &mut out);
    }
    out
}

fn flatten_inline_text_into(inline: &Inline, out: &mut String) {
    match inline {
        Inline::Text(t) => out.push_str(t),
        Inline::Style { children, .. } | Inline::Other { children, .. } => {
            for c in children {
                flatten_inline_text_into(c, out);
            }
        }
    }
}

fn decode_decl(decl: &quick_xml::events::BytesDecl<'_>) -> XmlDecl {
    let version = decl
        .version()
        .map(|v| String::from_utf8_lossy(&v).into_owned())
        .unwrap_or_else(|_| "1.0".to_string());
    let encoding = decl
        .encoding()
        .and_then(|e| e.ok())
        .map(|e| String::from_utf8_lossy(&e).into_owned());
    let standalone = decl
        .standalone()
        .and_then(|s| s.ok())
        .map(|s| String::from_utf8_lossy(&s).into_owned());
    XmlDecl {
        version,
        encoding,
        standalone,
    }
}

fn local_name(e: &quick_xml::events::BytesStart<'_>) -> String {
    String::from_utf8_lossy(e.local_name().as_ref()).into_owned()
}

fn end_local_name(e: &quick_xml::events::BytesEnd<'_>) -> String {
    String::from_utf8_lossy(e.local_name().as_ref()).into_owned()
}

fn attr_value(e: &quick_xml::events::BytesStart<'_>, name: &str) -> Option<String> {
    e.attributes().flatten().find_map(|a| {
        if a.key.local_name().as_ref() == name.as_bytes() {
            Some(
                a.unescape_value()
                    .map(|v| v.into_owned())
                    .unwrap_or_else(|_| String::from_utf8_lossy(&a.value).into_owned()),
            )
        } else {
            None
        }
    })
}

fn read_attrs(e: &quick_xml::events::BytesStart<'_>) -> Vec<(String, String)> {
    let mut attrs = Vec::new();
    for attr in e.attributes().flatten() {
        let key = String::from_utf8_lossy(attr.key.as_ref()).into_owned();
        let value = attr
            .unescape_value()
            .map(|v| v.into_owned())
            .unwrap_or_else(|_| String::from_utf8_lossy(&attr.value).into_owned());
        attrs.push((key, value));
    }
    attrs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_record() {
        let (doc, diags) = parse(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<xml><records><record>
  <ref-type name="Journal Article">17</ref-type>
  <contributors><authors><author>Smith, John</author></authors></contributors>
  <titles><title>A Great Paper</title><secondary-title>Nature</secondary-title></titles>
  <dates><year>2020</year></dates>
</record></records></xml>"#,
        );
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert_eq!(doc.records.len(), 1);
        let r = &doc.records[0];
        assert_eq!(r.ref_type.code, "17");
        assert_eq!(r.ref_type.name.as_deref(), Some("Journal Article"));
        assert_eq!(r.contributors.as_ref().unwrap().authors.len(), 1);
        assert_eq!(
            flatten_inline_text(&r.contributors.as_ref().unwrap().authors[0]),
            "Smith, John"
        );
        assert_eq!(
            flatten_inline_text(r.titles.as_ref().unwrap().title.as_ref().unwrap()),
            "A Great Paper"
        );
        assert_eq!(
            flatten_inline_text(r.titles.as_ref().unwrap().secondary_title.as_ref().unwrap()),
            "Nature"
        );
        assert_eq!(
            flatten_inline_text(r.dates.as_ref().unwrap().year.as_ref().unwrap()),
            "2020"
        );
    }

    #[test]
    fn parses_style_runs() {
        let (doc, diags) = parse(
            br#"<xml><records><record>
  <ref-type name="Journal Article">17</ref-type>
  <titles><title><style face="normal">A </style><style face="italic">Great</style><style face="normal"> Paper</style></title></titles>
</record></records></xml>"#,
        );
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        let title = doc.records[0]
            .titles
            .as_ref()
            .unwrap()
            .title
            .as_ref()
            .unwrap();
        assert_eq!(title.len(), 3);
        assert!(matches!(&title[0], Inline::Style { face, .. } if face == "normal"));
        assert!(matches!(&title[1], Inline::Style { face, .. } if face == "italic"));
        assert_eq!(flatten_inline_text(title), "A Great Paper");
    }

    #[test]
    fn preserves_unknown_record_elements() {
        let (doc, diags) = parse(
            br#"<xml><records><record>
  <ref-type>13</ref-type>
  <custom1>foo</custom1>
</record></records></xml>"#,
        );
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert_eq!(doc.records[0].extra.len(), 1);
        assert_eq!(doc.records[0].extra[0].name, "custom1");
        assert_eq!(
            flatten_inline_text(&doc.records[0].extra[0].children),
            "foo"
        );
    }

    #[test]
    fn parses_foreign_keys() {
        let (doc, diags) = parse(
            br#"<xml><records><record>
  <ref-type>17</ref-type>
  <foreign-keys><key app="EN" db-id="abc123">42</key></foreign-keys>
</record></records></xml>"#,
        );
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        let keys = &doc.records[0].foreign_keys.as_ref().unwrap().keys;
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].app.as_deref(), Some("EN"));
        assert_eq!(keys[0].db_id.as_deref(), Some("abc123"));
        assert_eq!(keys[0].text, "42");
    }

    #[test]
    fn recovers_from_truncated_input() {
        let (doc, diags) =
            parse(br#"<xml><records><record><ref-type>17</ref-type><titles><title>X"#);
        assert!(!diags.is_empty());
        assert_eq!(doc.records.len(), 1);
    }
}
