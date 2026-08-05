//! EndNote XML emitter: converts an [`EndNoteDoc`] AST back to bytes using
//! `quick_xml::Writer` directly (handles attribute/text escaping) — an
//! independent implementation from [`crate::writer::Writer`], not routed
//! through it (mirrors `opml-fmt`'s `emit.rs` vs. `writer.rs` split).

use std::io::Cursor;

use quick_xml::Writer as XmlWriter;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event as XmlEvent};

use crate::ast::*;

/// Emit an `EndNoteDoc` as XML bytes.
///
/// Deliberately **not** pretty-printed (no `new_with_indent`): EndNote field
/// content can carry meaningful whitespace between `<style>` runs (`"A "`
/// then `"Great"` then `" Paper"`), and a pretty-printer's automatic
/// indentation would inject additional whitespace around every tag,
/// corrupting exactly that content. Mirrors the pre-existing
/// `rescribe-write-endnotexml` writer's identical choice.
pub(crate) fn emit(doc: &EndNoteDoc) -> Vec<u8> {
    let mut writer = XmlWriter::new(Cursor::new(Vec::new()));

    if let Some(decl) = &doc.xml_decl {
        let _ = writer.write_event(XmlEvent::Decl(BytesDecl::new(
            &decl.version,
            decl.encoding.as_deref(),
            decl.standalone.as_deref(),
        )));
    }

    let _ = writer.write_event(XmlEvent::Start(BytesStart::new("xml")));
    // A zero-record document is indistinguishable, once parsed, from a
    // source with no `<records>` element at all (the AST has no "was
    // `<records>` present" bit — same convention `opml-fmt` uses for
    // `Head::is_empty()`) — so the `<records>` wrapper is omitted here to
    // match what `events()` produces when streaming real bytes that never
    // had the tag (fixture `adv-empty`: `<xml></xml>`).
    if !doc.records.is_empty() {
        let _ = writer.write_event(XmlEvent::Start(BytesStart::new("records")));
        for record in &doc.records {
            write_record(&mut writer, record);
        }
        let _ = writer.write_event(XmlEvent::End(BytesEnd::new("records")));
    }
    let _ = writer.write_event(XmlEvent::End(BytesEnd::new("xml")));

    writer.into_inner().into_inner()
}

fn write_record<W: std::io::Write>(writer: &mut XmlWriter<W>, r: &Record) {
    let _ = writer.write_event(XmlEvent::Start(BytesStart::new("record")));

    let mut ref_type = BytesStart::new("ref-type");
    if let Some(name) = &r.ref_type.name {
        ref_type.push_attribute(("name", name.as_str()));
    }
    write_leaf(writer, ref_type, &r.ref_type.code_as_inline());

    if let Some(c) = &r.contributors {
        let _ = writer.write_event(XmlEvent::Start(BytesStart::new("contributors")));
        write_author_role(writer, "authors", &c.authors);
        write_author_role(writer, "secondary-authors", &c.secondary_authors);
        write_author_role(writer, "tertiary-authors", &c.tertiary_authors);
        write_author_role(writer, "subsidiary-authors", &c.subsidiary_authors);
        for el in &c.extra {
            write_element(writer, el);
        }
        let _ = writer.write_event(XmlEvent::End(BytesEnd::new("contributors")));
    }

    if let Some(t) = &r.titles {
        let _ = writer.write_event(XmlEvent::Start(BytesStart::new("titles")));
        write_field(writer, "title", &t.title);
        write_field(writer, "secondary-title", &t.secondary_title);
        write_field(writer, "tertiary-title", &t.tertiary_title);
        for el in &t.extra {
            write_element(writer, el);
        }
        let _ = writer.write_event(XmlEvent::End(BytesEnd::new("titles")));
    }

    if let Some(p) = &r.periodical {
        let _ = writer.write_event(XmlEvent::Start(BytesStart::new("periodical")));
        write_field(writer, "full-title", &p.full_title);
        for el in &p.extra {
            write_element(writer, el);
        }
        let _ = writer.write_event(XmlEvent::End(BytesEnd::new("periodical")));
    }

    if let Some(d) = &r.dates {
        let _ = writer.write_event(XmlEvent::Start(BytesStart::new("dates")));
        write_field(writer, "year", &d.year);
        if d.pub_date.is_some() {
            let _ = writer.write_event(XmlEvent::Start(BytesStart::new("pub-dates")));
            write_field(writer, "date", &d.pub_date);
            let _ = writer.write_event(XmlEvent::End(BytesEnd::new("pub-dates")));
        }
        for el in &d.extra {
            write_element(writer, el);
        }
        let _ = writer.write_event(XmlEvent::End(BytesEnd::new("dates")));
    }

    write_field(writer, "volume", &r.volume);
    write_field(writer, "number", &r.number);
    write_field(writer, "pages", &r.pages);
    write_field(writer, "publisher", &r.publisher);
    write_field(writer, "pub-location", &r.pub_location);
    write_text_field(writer, "isbn", &r.isbn);
    write_text_field(writer, "issn", &r.issn);
    write_text_field(
        writer,
        "electronic-resource-num",
        &r.electronic_resource_num,
    );

    if let Some(u) = &r.urls {
        let _ = writer.write_event(XmlEvent::Start(BytesStart::new("urls")));
        write_url_role(writer, "related-urls", &u.related_urls);
        write_url_role(writer, "pdf-urls", &u.pdf_urls);
        for el in &u.extra {
            write_element(writer, el);
        }
        let _ = writer.write_event(XmlEvent::End(BytesEnd::new("urls")));
    }
    write_text_field(writer, "url", &r.bare_url);

    write_field(writer, "abstract", &r.abstract_);
    write_field(writer, "notes", &r.notes);

    if !r.keywords.is_empty() {
        let _ = writer.write_event(XmlEvent::Start(BytesStart::new("keywords")));
        for kw in &r.keywords {
            write_leaf(writer, BytesStart::new("keyword"), kw);
        }
        let _ = writer.write_event(XmlEvent::End(BytesEnd::new("keywords")));
    }

    write_text_field(writer, "rec-number", &r.rec_number);
    write_text_field(writer, "label", &r.label);

    if let Some(fk) = &r.foreign_keys {
        let _ = writer.write_event(XmlEvent::Start(BytesStart::new("foreign-keys")));
        for key in &fk.keys {
            let mut start = BytesStart::new("key");
            if let Some(app) = &key.app {
                start.push_attribute(("app", app.as_str()));
            }
            if let Some(db_id) = &key.db_id {
                start.push_attribute(("db-id", db_id.as_str()));
            }
            write_leaf(writer, start, &text_as_inline(&key.text));
        }
        for el in &fk.extra {
            write_element(writer, el);
        }
        let _ = writer.write_event(XmlEvent::End(BytesEnd::new("foreign-keys")));
    }

    for el in &r.extra {
        write_element(writer, el);
    }

    let _ = writer.write_event(XmlEvent::End(BytesEnd::new("record")));
}

fn write_author_role<W: std::io::Write>(
    writer: &mut XmlWriter<W>,
    tag: &str,
    people: &[Vec<Inline>],
) {
    if people.is_empty() {
        return;
    }
    let _ = writer.write_event(XmlEvent::Start(BytesStart::new(tag)));
    for person in people {
        write_leaf(writer, BytesStart::new("author"), person);
    }
    let _ = writer.write_event(XmlEvent::End(BytesEnd::new(tag)));
}

fn write_url_role<W: std::io::Write>(writer: &mut XmlWriter<W>, tag: &str, urls: &[String]) {
    if urls.is_empty() {
        return;
    }
    let _ = writer.write_event(XmlEvent::Start(BytesStart::new(tag)));
    for url in urls {
        write_leaf(writer, BytesStart::new("url"), &text_as_inline(url));
    }
    let _ = writer.write_event(XmlEvent::End(BytesEnd::new(tag)));
}

fn write_field<W: std::io::Write>(
    writer: &mut XmlWriter<W>,
    tag: &str,
    content: &Option<Vec<Inline>>,
) {
    if let Some(inline) = content {
        write_leaf(writer, BytesStart::new(tag), inline);
    }
}

fn write_text_field<W: std::io::Write>(
    writer: &mut XmlWriter<W>,
    tag: &str,
    content: &Option<String>,
) {
    if let Some(text) = content {
        write_leaf(writer, BytesStart::new(tag), &text_as_inline(text));
    }
}

fn write_element<W: std::io::Write>(writer: &mut XmlWriter<W>, el: &Element) {
    let mut start = BytesStart::new(el.name.as_str());
    for (k, v) in &el.attrs {
        start.push_attribute((k.as_str(), v.as_str()));
    }
    write_leaf(writer, start, &el.children);
}

/// Write `<start>...(inline content)...</start>` (or a self-closing tag if
/// `inline` is empty), sharing the same shape every leaf field/element
/// uses.
fn write_leaf<W: std::io::Write>(
    writer: &mut XmlWriter<W>,
    start: BytesStart<'_>,
    inline: &[Inline],
) {
    if inline.is_empty() {
        let _ = writer.write_event(XmlEvent::Empty(start));
        return;
    }
    let name = String::from_utf8_lossy(start.name().as_ref()).into_owned();
    let _ = writer.write_event(XmlEvent::Start(start));
    write_inline(writer, inline);
    let _ = writer.write_event(XmlEvent::End(BytesEnd::new(name)));
}

fn write_inline<W: std::io::Write>(writer: &mut XmlWriter<W>, inline: &[Inline]) {
    for item in inline {
        match item {
            Inline::Text(t) => {
                let _ = writer.write_event(XmlEvent::Text(BytesText::new(t)));
            }
            Inline::Style { face, children } => {
                let mut start = BytesStart::new("style");
                start.push_attribute(("face", face.as_str()));
                write_leaf(writer, start, children);
            }
            Inline::Other {
                name,
                attrs,
                children,
            } => {
                let mut start = BytesStart::new(name.as_str());
                for (k, v) in attrs {
                    start.push_attribute((k.as_str(), v.as_str()));
                }
                write_leaf(writer, start, children);
            }
        }
    }
}

fn text_as_inline(text: &str) -> Vec<Inline> {
    if text.is_empty() {
        Vec::new()
    } else {
        vec![Inline::Text(text.to_string())]
    }
}

impl RefType {
    fn code_as_inline(&self) -> Vec<Inline> {
        text_as_inline(&self.code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_and_roundtrips() {
        let doc = EndNoteDoc {
            xml_decl: Some(XmlDecl {
                version: "1.0".into(),
                encoding: Some("UTF-8".into()),
                standalone: None,
            }),
            records: vec![Record {
                ref_type: RefType {
                    code: "17".into(),
                    name: Some("Journal Article".into()),
                },
                contributors: Some(Contributors {
                    authors: vec![vec![Inline::Text("Smith, John".into())]],
                    ..Default::default()
                }),
                titles: Some(Titles {
                    title: Some(vec![Inline::Text("A Great Paper".into())]),
                    ..Default::default()
                }),
                ..Default::default()
            }],
            span: Span::NONE,
        };

        let bytes = emit(&doc);
        let xml = String::from_utf8(bytes.clone()).unwrap();
        assert!(xml.contains(r#"<ref-type name="Journal Article">17</ref-type>"#));
        assert!(xml.contains("<author>Smith, John</author>"));
        assert!(xml.contains("<title>A Great Paper</title>"));

        let (doc2, diags) = crate::parse::parse(&bytes);
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert_eq!(doc.strip_spans(), doc2.strip_spans());
    }

    #[test]
    fn emits_style_runs() {
        let doc = EndNoteDoc {
            xml_decl: None,
            records: vec![Record {
                ref_type: RefType {
                    code: "17".into(),
                    name: None,
                },
                titles: Some(Titles {
                    title: Some(vec![
                        Inline::Text("A ".into()),
                        Inline::Style {
                            face: "italic".into(),
                            children: vec![Inline::Text("Great".into())],
                        },
                        Inline::Text(" Paper".into()),
                    ]),
                    ..Default::default()
                }),
                ..Default::default()
            }],
            span: Span::NONE,
        };
        let bytes = emit(&doc);
        let (doc2, diags) = crate::parse::parse(&bytes);
        assert!(diags.is_empty());
        assert_eq!(doc.strip_spans(), doc2.strip_spans());
    }
}
