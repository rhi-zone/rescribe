//! Streaming FB2 writer — serializes FB2 semantic events to XML bytes.
//!
//! [`Writer`] accepts [`Event`] items (the high-level FB2 event stream) and
//! writes the corresponding XML bytes to the underlying `Write` sink.
//!
//! This is the streaming inverse of the [`events`](crate::events) iterator:
//! collecting the output of `events(input)` through a `Writer` should
//! reproduce a semantically equivalent FB2 document.
//!
//! # Example
//! ```no_run
//! use fb2_fmt::writer::Writer;
//! use fb2_fmt::Event;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! w.write_event(Event::StartFictionBook).unwrap();
//! w.write_event(Event::EndFictionBook).unwrap();
//! let bytes = w.finish().unwrap();
//! ```

use std::io::Write;

use base64::Engine;
use quick_xml::{
    Writer as XmlWriter,
    events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event as XmlEvent},
};

use crate::ast::*;
use crate::events::Event;

/// Streaming FB2 writer.
///
/// Feed semantic events with [`write_event`](Writer::write_event), then call
/// [`finish`](Writer::finish) to recover the sink.
pub struct Writer<W: Write> {
    inner: XmlWriter<W>,
}

impl<W: Write> Writer<W> {
    /// Create a new writer wrapping the given sink.
    pub fn new(sink: W) -> Self {
        Writer { inner: XmlWriter::new(sink) }
    }

    /// Write one FB2 semantic event to the sink.
    pub fn write_event(&mut self, event: Event) -> std::io::Result<()> {
        match event {
            Event::StartFictionBook => {
                self.inner.write_event(XmlEvent::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;
                let mut root = BytesStart::new("FictionBook");
                root.push_attribute(("xmlns", "http://www.gribuser.ru/xml/fictionbook/2.0"));
                root.push_attribute(("xmlns:l", "http://www.w3.org/1999/xlink"));
                self.inner.write_event(XmlEvent::Start(root))?;
            }
            Event::EndFictionBook => {
                self.inner.write_event(XmlEvent::End(BytesEnd::new("FictionBook")))?;
            }
            Event::Metadata(desc) => {
                write_description(&mut self.inner, &desc)?;
            }
            Event::StartBody { name, lang } => {
                let mut e = BytesStart::new("body");
                if let Some(n) = &name {
                    e.push_attribute(("name", n.as_str()));
                }
                if let Some(l) = &lang {
                    e.push_attribute(("xml:lang", l.as_str()));
                }
                self.inner.write_event(XmlEvent::Start(e))?;
            }
            Event::EndBody => {
                self.inner.write_event(XmlEvent::End(BytesEnd::new("body")))?;
            }
            Event::StartSection { id, lang } => {
                let mut e = BytesStart::new("section");
                if let Some(id) = &id {
                    e.push_attribute(("id", id.as_str()));
                }
                if let Some(l) = &lang {
                    e.push_attribute(("xml:lang", l.as_str()));
                }
                self.inner.write_event(XmlEvent::Start(e))?;
            }
            Event::EndSection => {
                self.inner.write_event(XmlEvent::End(BytesEnd::new("section")))?;
            }
            Event::StartTitle => {
                self.inner.write_event(XmlEvent::Start(BytesStart::new("title")))?;
            }
            Event::EndTitle => {
                self.inner.write_event(XmlEvent::End(BytesEnd::new("title")))?;
            }
            Event::TitleParagraph(inlines) => {
                write_para(&mut self.inner, &inlines)?;
            }
            Event::StartParagraph => {
                self.inner.write_event(XmlEvent::Start(BytesStart::new("p")))?;
            }
            Event::EndParagraph => {
                self.inner.write_event(XmlEvent::End(BytesEnd::new("p")))?;
            }
            Event::Inline(inlines) => {
                write_inlines(&mut self.inner, &inlines)?;
            }
            Event::EmptyLine => {
                self.inner.write_event(XmlEvent::Empty(BytesStart::new("empty-line")))?;
            }
            Event::Subtitle(inlines) => {
                self.inner.write_event(XmlEvent::Start(BytesStart::new("subtitle")))?;
                write_inlines(&mut self.inner, &inlines)?;
                self.inner.write_event(XmlEvent::End(BytesEnd::new("subtitle")))?;
            }
            Event::StartPoem => {
                self.inner.write_event(XmlEvent::Start(BytesStart::new("poem")))?;
            }
            Event::EndPoem => {
                self.inner.write_event(XmlEvent::End(BytesEnd::new("poem")))?;
            }
            Event::StartStanza => {
                self.inner.write_event(XmlEvent::Start(BytesStart::new("stanza")))?;
            }
            Event::EndStanza => {
                self.inner.write_event(XmlEvent::End(BytesEnd::new("stanza")))?;
            }
            Event::VerseLine(inlines) => {
                self.inner.write_event(XmlEvent::Start(BytesStart::new("v")))?;
                write_inlines(&mut self.inner, &inlines)?;
                self.inner.write_event(XmlEvent::End(BytesEnd::new("v")))?;
            }
            Event::StartCite { id } => {
                let mut e = BytesStart::new("cite");
                if let Some(id) = &id {
                    e.push_attribute(("id", id.as_str()));
                }
                self.inner.write_event(XmlEvent::Start(e))?;
            }
            Event::EndCite => {
                self.inner.write_event(XmlEvent::End(BytesEnd::new("cite")))?;
            }
            Event::StartEpigraph { id } => {
                let mut e = BytesStart::new("epigraph");
                if let Some(id) = &id {
                    e.push_attribute(("id", id.as_str()));
                }
                self.inner.write_event(XmlEvent::Start(e))?;
            }
            Event::EndEpigraph => {
                self.inner.write_event(XmlEvent::End(BytesEnd::new("epigraph")))?;
            }
            Event::TextAuthor(inlines) => {
                self.inner.write_event(XmlEvent::Start(BytesStart::new("text-author")))?;
                write_inlines(&mut self.inner, &inlines)?;
                self.inner.write_event(XmlEvent::End(BytesEnd::new("text-author")))?;
            }
            Event::Table(table) => {
                write_table(&mut self.inner, &table)?;
            }
            Event::Image(img) => {
                write_image_elem(&mut self.inner, &img)?;
            }
            Event::Binary(binary) => {
                write_binary(&mut self.inner, &binary)?;
            }
        }
        Ok(())
    }

    /// Flush and return the underlying sink.
    pub fn finish(self) -> std::io::Result<W> {
        let mut sink = self.inner.into_inner();
        sink.flush()?;
        Ok(sink)
    }
}

// ---------------------------------------------------------------------------
// Helpers (generic over any Write, mirroring emit.rs but returning Result)
// ---------------------------------------------------------------------------

fn xw_start(w: &mut XmlWriter<impl Write>, tag: &str) -> std::io::Result<()> {
    w.write_event(XmlEvent::Start(BytesStart::new(tag)))
}

fn xw_end(w: &mut XmlWriter<impl Write>, tag: &str) -> std::io::Result<()> {
    w.write_event(XmlEvent::End(BytesEnd::new(tag)))
}

fn xw_text_elem(w: &mut XmlWriter<impl Write>, tag: &str, content: &str) -> std::io::Result<()> {
    xw_start(w, tag)?;
    w.write_event(XmlEvent::Text(BytesText::new(content)))?;
    xw_end(w, tag)
}

fn xw_text_elem_opt(
    w: &mut XmlWriter<impl Write>,
    tag: &str,
    content: Option<&str>,
) -> std::io::Result<()> {
    if let Some(s) = content {
        xw_text_elem(w, tag, s)?;
    }
    Ok(())
}

fn write_description(w: &mut XmlWriter<impl Write>, desc: &Description) -> std::io::Result<()> {
    xw_start(w, "description")?;
    write_title_info(w, &desc.title_info)?;
    if let Some(di) = &desc.document_info {
        write_document_info(w, di)?;
    }
    if let Some(pi) = &desc.publish_info {
        write_publish_info(w, pi)?;
    }
    for ci in &desc.custom_info {
        let mut e = BytesStart::new("custom-info");
        e.push_attribute(("info-type", ci.info_type.as_str()));
        w.write_event(XmlEvent::Start(e))?;
        w.write_event(XmlEvent::Text(BytesText::new(&ci.content)))?;
        xw_end(w, "custom-info")?;
    }
    xw_end(w, "description")
}

fn write_title_info(w: &mut XmlWriter<impl Write>, ti: &TitleInfo) -> std::io::Result<()> {
    xw_start(w, "title-info")?;
    for genre in &ti.genre {
        xw_text_elem(w, "genre", genre)?;
    }
    for author in &ti.author {
        write_author(w, author, "author")?;
    }
    xw_text_elem(w, "book-title", &ti.book_title)?;
    if let Some(ann) = &ti.annotation {
        write_annotation(w, ann)?;
    }
    xw_text_elem_opt(w, "keywords", ti.keywords.as_deref())?;
    xw_text_elem_opt(w, "date", ti.date.as_deref())?;
    if !ti.lang.is_empty() {
        xw_text_elem(w, "lang", &ti.lang)?;
    }
    xw_text_elem_opt(w, "src-lang", ti.src_lang.as_deref())?;
    for tr in &ti.translator {
        write_author(w, tr, "translator")?;
    }
    for seq in &ti.sequence {
        write_sequence(w, seq)?;
    }
    xw_end(w, "title-info")
}

fn write_author(
    w: &mut XmlWriter<impl Write>,
    a: &Author,
    tag: &str,
) -> std::io::Result<()> {
    xw_start(w, tag)?;
    xw_text_elem_opt(w, "first-name", a.first_name.as_deref())?;
    xw_text_elem_opt(w, "middle-name", a.middle_name.as_deref())?;
    xw_text_elem_opt(w, "last-name", a.last_name.as_deref())?;
    xw_text_elem_opt(w, "nickname", a.nickname.as_deref())?;
    for email in &a.email {
        xw_text_elem(w, "email", email)?;
    }
    xw_text_elem_opt(w, "id", a.id.as_deref())?;
    xw_end(w, tag)
}

fn write_sequence(w: &mut XmlWriter<impl Write>, seq: &Sequence) -> std::io::Result<()> {
    let mut e = BytesStart::new("sequence");
    e.push_attribute(("name", seq.name.as_str()));
    if let Some(n) = seq.number {
        e.push_attribute(("number", n.to_string().as_str()));
    }
    w.write_event(XmlEvent::Empty(e))
}

fn write_document_info(w: &mut XmlWriter<impl Write>, di: &DocumentInfo) -> std::io::Result<()> {
    xw_start(w, "document-info")?;
    for author in &di.author {
        write_author(w, author, "author")?;
    }
    xw_text_elem_opt(w, "program-used", di.program_used.as_deref())?;
    xw_text_elem_opt(w, "date", di.date.as_deref())?;
    for url in &di.src_url {
        xw_text_elem(w, "src-url", url)?;
    }
    xw_text_elem_opt(w, "src-ocr", di.src_ocr.as_deref())?;
    xw_text_elem_opt(w, "id", di.id.as_deref())?;
    xw_text_elem_opt(w, "version", di.version.as_deref())?;
    xw_end(w, "document-info")
}

fn write_publish_info(w: &mut XmlWriter<impl Write>, pi: &PublishInfo) -> std::io::Result<()> {
    xw_start(w, "publish-info")?;
    xw_text_elem_opt(w, "book-name", pi.book_name.as_deref())?;
    xw_text_elem_opt(w, "publisher", pi.publisher.as_deref())?;
    xw_text_elem_opt(w, "city", pi.city.as_deref())?;
    xw_text_elem_opt(w, "year", pi.year.as_deref())?;
    xw_text_elem_opt(w, "isbn", pi.isbn.as_deref())?;
    for seq in &pi.sequence {
        write_sequence(w, seq)?;
    }
    xw_end(w, "publish-info")
}

fn write_annotation(w: &mut XmlWriter<impl Write>, ann: &Annotation) -> std::io::Result<()> {
    let mut e = BytesStart::new("annotation");
    if let Some(id) = &ann.id {
        e.push_attribute(("id", id.as_str()));
    }
    if let Some(lang) = &ann.lang {
        e.push_attribute(("xml:lang", lang.as_str()));
    }
    w.write_event(XmlEvent::Start(e))?;
    for content in &ann.content {
        match content {
            AnnotationContent::Para(il) => write_para(w, il)?,
            AnnotationContent::Poem(p) => write_poem(w, p)?,
            AnnotationContent::Cite(c) => write_cite(w, c)?,
            AnnotationContent::Subtitle(il) => {
                xw_start(w, "subtitle")?;
                write_inlines(w, il)?;
                xw_end(w, "subtitle")?;
            }
            AnnotationContent::EmptyLine => {
                w.write_event(XmlEvent::Empty(BytesStart::new("empty-line")))?;
            }
            AnnotationContent::Table(t) => write_table(w, t)?,
        }
    }
    xw_end(w, "annotation")
}

fn write_para(w: &mut XmlWriter<impl Write>, inlines: &[InlineElement]) -> std::io::Result<()> {
    xw_start(w, "p")?;
    write_inlines(w, inlines)?;
    xw_end(w, "p")
}

fn write_poem(w: &mut XmlWriter<impl Write>, poem: &Poem) -> std::io::Result<()> {
    let mut e = BytesStart::new("poem");
    if let Some(id) = &poem.id {
        e.push_attribute(("id", id.as_str()));
    }
    w.write_event(XmlEvent::Start(e))?;
    if let Some(title) = &poem.title {
        write_title(w, title)?;
    }
    for epigraph in &poem.epigraph {
        write_epigraph(w, epigraph)?;
    }
    for stanza in &poem.stanza {
        write_stanza(w, stanza)?;
    }
    for ta in &poem.text_author {
        xw_start(w, "text-author")?;
        write_inlines(w, ta)?;
        xw_end(w, "text-author")?;
    }
    if let Some(date) = &poem.date {
        xw_text_elem(w, "date", date)?;
    }
    xw_end(w, "poem")
}

fn write_stanza(w: &mut XmlWriter<impl Write>, stanza: &Stanza) -> std::io::Result<()> {
    xw_start(w, "stanza")?;
    if let Some(title) = &stanza.title {
        write_title(w, title)?;
    }
    if let Some(sub) = &stanza.subtitle {
        xw_start(w, "subtitle")?;
        write_inlines(w, sub)?;
        xw_end(w, "subtitle")?;
    }
    for v in &stanza.v {
        xw_start(w, "v")?;
        write_inlines(w, v)?;
        xw_end(w, "v")?;
    }
    xw_end(w, "stanza")
}

fn write_title(w: &mut XmlWriter<impl Write>, title: &Title) -> std::io::Result<()> {
    let mut e = BytesStart::new("title");
    if let Some(lang) = &title.lang {
        e.push_attribute(("xml:lang", lang.as_str()));
    }
    w.write_event(XmlEvent::Start(e))?;
    for para in &title.para {
        match para {
            TitlePara::Para(inlines) => write_para(w, inlines)?,
            TitlePara::EmptyLine => {
                w.write_event(XmlEvent::Empty(BytesStart::new("empty-line")))?;
            }
        }
    }
    xw_end(w, "title")
}

fn write_epigraph(w: &mut XmlWriter<impl Write>, epigraph: &Epigraph) -> std::io::Result<()> {
    let mut e = BytesStart::new("epigraph");
    if let Some(id) = &epigraph.id {
        e.push_attribute(("id", id.as_str()));
    }
    w.write_event(XmlEvent::Start(e))?;
    for content in &epigraph.content {
        match content {
            EpigraphContent::Para(il) => write_para(w, il)?,
            EpigraphContent::Poem(p) => write_poem(w, p)?,
            EpigraphContent::Cite(c) => write_cite(w, c)?,
            EpigraphContent::EmptyLine => {
                w.write_event(XmlEvent::Empty(BytesStart::new("empty-line")))?;
            }
        }
    }
    for ta in &epigraph.text_author {
        xw_start(w, "text-author")?;
        write_inlines(w, ta)?;
        xw_end(w, "text-author")?;
    }
    xw_end(w, "epigraph")
}

fn write_cite(w: &mut XmlWriter<impl Write>, cite: &Cite) -> std::io::Result<()> {
    let mut e = BytesStart::new("cite");
    if let Some(id) = &cite.id {
        e.push_attribute(("id", id.as_str()));
    }
    w.write_event(XmlEvent::Start(e))?;
    for content in &cite.content {
        match content {
            CiteContent::Para(il) => write_para(w, il)?,
            CiteContent::Poem(p) => write_poem(w, p)?,
            CiteContent::EmptyLine => {
                w.write_event(XmlEvent::Empty(BytesStart::new("empty-line")))?;
            }
            CiteContent::Table(t) => write_table(w, t)?,
        }
    }
    for ta in &cite.text_author {
        xw_start(w, "text-author")?;
        write_inlines(w, ta)?;
        xw_end(w, "text-author")?;
    }
    xw_end(w, "cite")
}

fn write_table(w: &mut XmlWriter<impl Write>, table: &Table) -> std::io::Result<()> {
    let mut e = BytesStart::new("table");
    if let Some(id) = &table.id {
        e.push_attribute(("id", id.as_str()));
    }
    if let Some(style) = &table.style {
        e.push_attribute(("style", style.as_str()));
    }
    w.write_event(XmlEvent::Start(e))?;
    for row in &table.row {
        let mut re = BytesStart::new("tr");
        if let Some(align) = &row.align {
            re.push_attribute(("align", align.as_str()));
        }
        w.write_event(XmlEvent::Start(re))?;
        for cell in &row.cell {
            let cell_tag = if cell.is_header { "th" } else { "td" };
            let mut ce = BytesStart::new(cell_tag);
            if let Some(id) = &cell.id {
                ce.push_attribute(("id", id.as_str()));
            }
            if let Some(style) = &cell.style {
                ce.push_attribute(("style", style.as_str()));
            }
            if let Some(colspan) = cell.colspan {
                ce.push_attribute(("colspan", colspan.to_string().as_str()));
            }
            if let Some(rowspan) = cell.rowspan {
                ce.push_attribute(("rowspan", rowspan.to_string().as_str()));
            }
            if let Some(align) = &cell.align {
                ce.push_attribute(("align", align.as_str()));
            }
            if let Some(valign) = &cell.valign {
                ce.push_attribute(("valign", valign.as_str()));
            }
            w.write_event(XmlEvent::Start(ce))?;
            write_inlines(w, &cell.content)?;
            xw_end(w, cell_tag)?;
        }
        xw_end(w, "tr")?;
    }
    xw_end(w, "table")
}

fn write_image_elem(w: &mut XmlWriter<impl Write>, img: &Image) -> std::io::Result<()> {
    let mut e = BytesStart::new("image");
    e.push_attribute(("l:href", img.href.as_str()));
    if let Some(alt) = &img.alt {
        e.push_attribute(("alt", alt.as_str()));
    }
    if let Some(title) = &img.title {
        e.push_attribute(("title", title.as_str()));
    }
    if let Some(id) = &img.id {
        e.push_attribute(("id", id.as_str()));
    }
    w.write_event(XmlEvent::Empty(e))
}

fn write_inlines(w: &mut XmlWriter<impl Write>, inlines: &[InlineElement]) -> std::io::Result<()> {
    for el in inlines {
        write_inline(w, el)?;
    }
    Ok(())
}

fn write_inline(w: &mut XmlWriter<impl Write>, el: &InlineElement) -> std::io::Result<()> {
    match el {
        InlineElement::Text(s) => {
            w.write_event(XmlEvent::Text(BytesText::new(s)))?;
        }
        InlineElement::Strong(ch) => {
            xw_start(w, "strong")?;
            write_inlines(w, ch)?;
            xw_end(w, "strong")?;
        }
        InlineElement::Emphasis(ch) => {
            xw_start(w, "emphasis")?;
            write_inlines(w, ch)?;
            xw_end(w, "emphasis")?;
        }
        InlineElement::Strikethrough(ch) => {
            xw_start(w, "strikethrough")?;
            write_inlines(w, ch)?;
            xw_end(w, "strikethrough")?;
        }
        InlineElement::Sub(ch) => {
            xw_start(w, "sub")?;
            write_inlines(w, ch)?;
            xw_end(w, "sub")?;
        }
        InlineElement::Sup(ch) => {
            xw_start(w, "sup")?;
            write_inlines(w, ch)?;
            xw_end(w, "sup")?;
        }
        InlineElement::Code(s) => {
            xw_start(w, "code")?;
            w.write_event(XmlEvent::Text(BytesText::new(s)))?;
            xw_end(w, "code")?;
        }
        InlineElement::Image(img) => {
            write_image_elem(w, img)?;
        }
        InlineElement::Link { href, children, .. } => {
            let mut e = BytesStart::new("a");
            e.push_attribute(("l:href", href.as_str()));
            w.write_event(XmlEvent::Start(e))?;
            write_inlines(w, children)?;
            xw_end(w, "a")?;
        }
        InlineElement::FootnoteRef { href, children } => {
            let mut e = BytesStart::new("a");
            e.push_attribute(("l:href", href.as_str()));
            e.push_attribute(("type", "note"));
            w.write_event(XmlEvent::Start(e))?;
            write_inlines(w, children)?;
            xw_end(w, "a")?;
        }
    }
    Ok(())
}

fn write_binary(w: &mut XmlWriter<impl Write>, binary: &Binary) -> std::io::Result<()> {
    let mut e = BytesStart::new("binary");
    e.push_attribute(("id", binary.id.as_str()));
    e.push_attribute(("content-type", binary.content_type.as_str()));
    w.write_event(XmlEvent::Start(e))?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&binary.data);
    w.write_event(XmlEvent::Text(BytesText::new(&encoded)))?;
    xw_end(w, "binary")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::events;

    #[test]
    fn test_writer_empty_fictionbook() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(Event::StartFictionBook).unwrap();
        w.write_event(Event::EndFictionBook).unwrap();
        let bytes = w.finish().unwrap();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("FictionBook"));
        assert!(s.contains("xmlns"));
    }

    #[test]
    fn test_writer_roundtrip() {
        // Collect events from input, write them, re-parse, re-collect, compare.
        let input = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
<FictionBook xmlns=\"http://www.gribuser.ru/xml/fictionbook/2.0\" \
xmlns:l=\"http://www.w3.org/1999/xlink\">\
<description><title-info><genre>prose</genre>\
<book-title>Test Book</book-title><lang>en</lang></title-info></description>\
<body><section><title><p>Chapter 1</p></title>\
<p>Hello, <emphasis>world</emphasis>!</p>\
<empty-line/></section></body></FictionBook>";

        let evs: Vec<Event> = events(input).collect();

        let mut w = Writer::new(Vec::<u8>::new());
        for ev in evs.clone() {
            w.write_event(ev).unwrap();
        }
        let output = w.finish().unwrap();

        let evs2: Vec<Event> = events(&output).collect();

        assert_eq!(evs, evs2, "event roundtrip mismatch");
    }

    #[test]
    fn test_writer_roundtrip_with_binary() {
        let data = b"iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";
        let input = [
            b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>".as_ref(),
            b"<FictionBook xmlns=\"http://www.gribuser.ru/xml/fictionbook/2.0\" xmlns:l=\"http://www.w3.org/1999/xlink\">".as_ref(),
            b"<description><title-info><genre>prose</genre><book-title>Binary Test</book-title><lang>en</lang></title-info></description>".as_ref(),
            b"<body><section><p>See <image l:href=\"#cover\"/>.</p></section></body>".as_ref(),
            b"<binary id=\"cover\" content-type=\"image/png\">".as_ref(),
            data,
            b"</binary></FictionBook>".as_ref(),
        ].concat();

        let evs: Vec<Event> = events(&input).collect();

        let mut w = Writer::new(Vec::<u8>::new());
        for ev in evs.clone() {
            w.write_event(ev).unwrap();
        }
        let output = w.finish().unwrap();

        let evs2: Vec<Event> = events(&output).collect();

        assert_eq!(evs, evs2, "binary roundtrip mismatch");
    }
}
