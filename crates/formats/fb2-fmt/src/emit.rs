use crate::ast::*;
use base64::Engine;
use quick_xml::{
    Writer,
    events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event},
};
use std::io::Cursor;

pub fn emit(fb: &FictionBook) -> Vec<u8> {
    let mut w = Writer::new(Cursor::new(Vec::new()));

    // XML declaration
    w.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .unwrap();

    // <FictionBook>
    let mut root = BytesStart::new("FictionBook");
    root.push_attribute(("xmlns", "http://www.gribuser.ru/xml/fictionbook/2.0"));
    root.push_attribute(("xmlns:l", "http://www.w3.org/1999/xlink"));
    w.write_event(Event::Start(root)).unwrap();

    write_description(&mut w, &fb.description);

    for body in &fb.bodies {
        write_body(&mut w, body);
    }

    for binary in &fb.binaries {
        write_binary(&mut w, binary);
    }

    w.write_event(Event::End(BytesEnd::new("FictionBook"))).unwrap();

    w.into_inner().into_inner()
}

fn start(w: &mut Writer<Cursor<Vec<u8>>>, tag: &str) {
    w.write_event(Event::Start(BytesStart::new(tag))).unwrap();
}

fn end(w: &mut Writer<Cursor<Vec<u8>>>, tag: &str) {
    w.write_event(Event::End(BytesEnd::new(tag))).unwrap();
}

fn text_elem(w: &mut Writer<Cursor<Vec<u8>>>, tag: &str, content: &str) {
    start(w, tag);
    w.write_event(Event::Text(BytesText::new(content))).unwrap();
    end(w, tag);
}

fn text_elem_opt(w: &mut Writer<Cursor<Vec<u8>>>, tag: &str, content: Option<&str>) {
    if let Some(s) = content {
        text_elem(w, tag, s);
    }
}

fn write_description(w: &mut Writer<Cursor<Vec<u8>>>, desc: &Description) {
    start(w, "description");
    write_title_info(w, &desc.title_info);
    if let Some(di) = &desc.document_info {
        write_document_info(w, di);
    }
    if let Some(pi) = &desc.publish_info {
        write_publish_info(w, pi);
    }
    for ci in &desc.custom_info {
        let mut e = BytesStart::new("custom-info");
        e.push_attribute(("info-type", ci.info_type.as_str()));
        w.write_event(Event::Start(e)).unwrap();
        w.write_event(Event::Text(BytesText::new(&ci.content))).unwrap();
        end(w, "custom-info");
    }
    end(w, "description");
}

fn write_title_info(w: &mut Writer<Cursor<Vec<u8>>>, ti: &TitleInfo) {
    start(w, "title-info");
    for genre in &ti.genre {
        text_elem(w, "genre", genre);
    }
    for author in &ti.author {
        write_author(w, author, "author");
    }
    text_elem(w, "book-title", &ti.book_title);
    if let Some(ann) = &ti.annotation {
        write_annotation(w, ann);
    }
    text_elem_opt(w, "keywords", ti.keywords.as_deref());
    text_elem_opt(w, "date", ti.date.as_deref());
    if !ti.lang.is_empty() {
        text_elem(w, "lang", &ti.lang);
    }
    text_elem_opt(w, "src-lang", ti.src_lang.as_deref());
    for tr in &ti.translator {
        write_author(w, tr, "translator");
    }
    for seq in &ti.sequence {
        write_sequence(w, seq);
    }
    end(w, "title-info");
}

fn write_author(w: &mut Writer<Cursor<Vec<u8>>>, a: &Author, tag: &str) {
    start(w, tag);
    text_elem_opt(w, "first-name", a.first_name.as_deref());
    text_elem_opt(w, "middle-name", a.middle_name.as_deref());
    text_elem_opt(w, "last-name", a.last_name.as_deref());
    text_elem_opt(w, "nickname", a.nickname.as_deref());
    for email in &a.email {
        text_elem(w, "email", email);
    }
    text_elem_opt(w, "id", a.id.as_deref());
    end(w, tag);
}

fn write_sequence(w: &mut Writer<Cursor<Vec<u8>>>, seq: &Sequence) {
    let mut e = BytesStart::new("sequence");
    e.push_attribute(("name", seq.name.as_str()));
    if let Some(n) = seq.number {
        e.push_attribute(("number", n.to_string().as_str()));
    }
    w.write_event(Event::Empty(e)).unwrap();
}

fn write_document_info(w: &mut Writer<Cursor<Vec<u8>>>, di: &DocumentInfo) {
    start(w, "document-info");
    for author in &di.author {
        write_author(w, author, "author");
    }
    text_elem_opt(w, "program-used", di.program_used.as_deref());
    text_elem_opt(w, "date", di.date.as_deref());
    for url in &di.src_url {
        text_elem(w, "src-url", url);
    }
    text_elem_opt(w, "src-ocr", di.src_ocr.as_deref());
    text_elem_opt(w, "id", di.id.as_deref());
    text_elem_opt(w, "version", di.version.as_deref());
    end(w, "document-info");
}

fn write_publish_info(w: &mut Writer<Cursor<Vec<u8>>>, pi: &PublishInfo) {
    start(w, "publish-info");
    text_elem_opt(w, "book-name", pi.book_name.as_deref());
    text_elem_opt(w, "publisher", pi.publisher.as_deref());
    text_elem_opt(w, "city", pi.city.as_deref());
    text_elem_opt(w, "year", pi.year.as_deref());
    text_elem_opt(w, "isbn", pi.isbn.as_deref());
    for seq in &pi.sequence {
        write_sequence(w, seq);
    }
    end(w, "publish-info");
}

fn write_body(w: &mut Writer<Cursor<Vec<u8>>>, body: &Body) {
    let mut e = BytesStart::new("body");
    if let Some(name) = &body.name {
        e.push_attribute(("name", name.as_str()));
    }
    if let Some(lang) = &body.lang {
        e.push_attribute(("xml:lang", lang.as_str()));
    }
    w.write_event(Event::Start(e)).unwrap();

    if let Some(title) = &body.title {
        write_title(w, title);
    }
    for epigraph in &body.epigraph {
        write_epigraph(w, epigraph);
    }
    for section in &body.section {
        write_section(w, section);
    }
    end(w, "body");
}

fn write_section(w: &mut Writer<Cursor<Vec<u8>>>, section: &Section) {
    let mut e = BytesStart::new("section");
    if let Some(id) = &section.id {
        e.push_attribute(("id", id.as_str()));
    }
    if let Some(lang) = &section.lang {
        e.push_attribute(("xml:lang", lang.as_str()));
    }
    w.write_event(Event::Start(e)).unwrap();

    if let Some(title) = &section.title {
        write_title(w, title);
    }
    for epigraph in &section.epigraph {
        write_epigraph(w, epigraph);
    }
    if let Some(img) = &section.image {
        write_image_elem(w, img);
    }
    if let Some(ann) = &section.annotation {
        write_annotation(w, ann);
    }
    for content in &section.content {
        write_section_content(w, content);
    }
    for nested in &section.section {
        write_section(w, nested);
    }
    end(w, "section");
}

fn write_title(w: &mut Writer<Cursor<Vec<u8>>>, title: &Title) {
    let mut e = BytesStart::new("title");
    if let Some(lang) = &title.lang {
        e.push_attribute(("xml:lang", lang.as_str()));
    }
    w.write_event(Event::Start(e)).unwrap();
    for para in &title.para {
        match para {
            TitlePara::Para(inlines) => {
                start(w, "p");
                write_inlines(w, inlines);
                end(w, "p");
            }
            TitlePara::EmptyLine => {
                w.write_event(Event::Empty(BytesStart::new("empty-line"))).unwrap();
            }
        }
    }
    end(w, "title");
}

fn write_epigraph(w: &mut Writer<Cursor<Vec<u8>>>, epigraph: &Epigraph) {
    let mut e = BytesStart::new("epigraph");
    if let Some(id) = &epigraph.id {
        e.push_attribute(("id", id.as_str()));
    }
    w.write_event(Event::Start(e)).unwrap();
    for content in &epigraph.content {
        match content {
            EpigraphContent::Para(il) => write_para(w, il),
            EpigraphContent::Poem(p) => write_poem(w, p),
            EpigraphContent::Cite(c) => write_cite(w, c),
            EpigraphContent::EmptyLine => {
                w.write_event(Event::Empty(BytesStart::new("empty-line"))).unwrap();
            }
        }
    }
    for ta in &epigraph.text_author {
        start(w, "text-author");
        write_inlines(w, ta);
        end(w, "text-author");
    }
    end(w, "epigraph");
}

fn write_annotation(w: &mut Writer<Cursor<Vec<u8>>>, ann: &Annotation) {
    let mut e = BytesStart::new("annotation");
    if let Some(id) = &ann.id {
        e.push_attribute(("id", id.as_str()));
    }
    if let Some(lang) = &ann.lang {
        e.push_attribute(("xml:lang", lang.as_str()));
    }
    w.write_event(Event::Start(e)).unwrap();
    for content in &ann.content {
        match content {
            AnnotationContent::Para(il) => write_para(w, il),
            AnnotationContent::Poem(p) => write_poem(w, p),
            AnnotationContent::Cite(c) => write_cite(w, c),
            AnnotationContent::Subtitle(il) => {
                start(w, "subtitle");
                write_inlines(w, il);
                end(w, "subtitle");
            }
            AnnotationContent::EmptyLine => {
                w.write_event(Event::Empty(BytesStart::new("empty-line"))).unwrap();
            }
            AnnotationContent::Table(t) => write_table(w, t),
        }
    }
    end(w, "annotation");
}

fn write_section_content(w: &mut Writer<Cursor<Vec<u8>>>, content: &SectionContent) {
    match content {
        SectionContent::Para(il) => write_para(w, il),
        SectionContent::Image(img) => write_image_elem(w, img),
        SectionContent::Poem(p) => write_poem(w, p),
        SectionContent::Subtitle(il) => {
            start(w, "subtitle");
            write_inlines(w, il);
            end(w, "subtitle");
        }
        SectionContent::Cite(c) => write_cite(w, c),
        SectionContent::EmptyLine => {
            w.write_event(Event::Empty(BytesStart::new("empty-line"))).unwrap();
        }
        SectionContent::Table(t) => write_table(w, t),
    }
}

fn write_para(w: &mut Writer<Cursor<Vec<u8>>>, inlines: &[InlineElement]) {
    start(w, "p");
    write_inlines(w, inlines);
    end(w, "p");
}

fn write_poem(w: &mut Writer<Cursor<Vec<u8>>>, poem: &Poem) {
    let mut e = BytesStart::new("poem");
    if let Some(id) = &poem.id {
        e.push_attribute(("id", id.as_str()));
    }
    w.write_event(Event::Start(e)).unwrap();
    if let Some(title) = &poem.title {
        write_title(w, title);
    }
    for epigraph in &poem.epigraph {
        write_epigraph(w, epigraph);
    }
    for stanza in &poem.stanza {
        write_stanza(w, stanza);
    }
    for ta in &poem.text_author {
        start(w, "text-author");
        write_inlines(w, ta);
        end(w, "text-author");
    }
    if let Some(date) = &poem.date {
        text_elem(w, "date", date);
    }
    end(w, "poem");
}

fn write_stanza(w: &mut Writer<Cursor<Vec<u8>>>, stanza: &Stanza) {
    start(w, "stanza");
    if let Some(title) = &stanza.title {
        write_title(w, title);
    }
    if let Some(sub) = &stanza.subtitle {
        start(w, "subtitle");
        write_inlines(w, sub);
        end(w, "subtitle");
    }
    for v in &stanza.v {
        start(w, "v");
        write_inlines(w, v);
        end(w, "v");
    }
    end(w, "stanza");
}

fn write_cite(w: &mut Writer<Cursor<Vec<u8>>>, cite: &Cite) {
    let mut e = BytesStart::new("cite");
    if let Some(id) = &cite.id {
        e.push_attribute(("id", id.as_str()));
    }
    w.write_event(Event::Start(e)).unwrap();
    for content in &cite.content {
        match content {
            CiteContent::Para(il) => write_para(w, il),
            CiteContent::Poem(p) => write_poem(w, p),
            CiteContent::EmptyLine => {
                w.write_event(Event::Empty(BytesStart::new("empty-line"))).unwrap();
            }
            CiteContent::Table(t) => write_table(w, t),
        }
    }
    for ta in &cite.text_author {
        start(w, "text-author");
        write_inlines(w, ta);
        end(w, "text-author");
    }
    end(w, "cite");
}

fn write_table(w: &mut Writer<Cursor<Vec<u8>>>, table: &Table) {
    let mut e = BytesStart::new("table");
    if let Some(id) = &table.id {
        e.push_attribute(("id", id.as_str()));
    }
    if let Some(style) = &table.style {
        e.push_attribute(("style", style.as_str()));
    }
    w.write_event(Event::Start(e)).unwrap();
    for row in &table.row {
        let mut re = BytesStart::new("tr");
        if let Some(align) = &row.align {
            re.push_attribute(("align", align.as_str()));
        }
        w.write_event(Event::Start(re)).unwrap();
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
            w.write_event(Event::Start(ce)).unwrap();
            write_inlines(w, &cell.content);
            end(w, cell_tag);
        }
        end(w, "tr");
    }
    end(w, "table");
}

fn write_image_elem(w: &mut Writer<Cursor<Vec<u8>>>, img: &Image) {
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
    w.write_event(Event::Empty(e)).unwrap();
}

fn write_inlines(w: &mut Writer<Cursor<Vec<u8>>>, inlines: &[InlineElement]) {
    for el in inlines {
        write_inline(w, el);
    }
}

fn write_inline(w: &mut Writer<Cursor<Vec<u8>>>, el: &InlineElement) {
    match el {
        InlineElement::Text(s) => {
            w.write_event(Event::Text(BytesText::new(s))).unwrap();
        }
        InlineElement::Strong(ch) => {
            start(w, "strong");
            write_inlines(w, ch);
            end(w, "strong");
        }
        InlineElement::Emphasis(ch) => {
            start(w, "emphasis");
            write_inlines(w, ch);
            end(w, "emphasis");
        }
        InlineElement::Strikethrough(ch) => {
            start(w, "strikethrough");
            write_inlines(w, ch);
            end(w, "strikethrough");
        }
        InlineElement::Sub(ch) => {
            start(w, "sub");
            write_inlines(w, ch);
            end(w, "sub");
        }
        InlineElement::Sup(ch) => {
            start(w, "sup");
            write_inlines(w, ch);
            end(w, "sup");
        }
        InlineElement::Code(s) => {
            start(w, "code");
            w.write_event(Event::Text(BytesText::new(s))).unwrap();
            end(w, "code");
        }
        InlineElement::Image(img) => {
            write_image_elem(w, img);
        }
        InlineElement::Link { href, children, .. } => {
            let mut e = BytesStart::new("a");
            e.push_attribute(("l:href", href.as_str()));
            w.write_event(Event::Start(e)).unwrap();
            write_inlines(w, children);
            end(w, "a");
        }
    }
}

fn write_binary(w: &mut Writer<Cursor<Vec<u8>>>, binary: &Binary) {
    let mut e = BytesStart::new("binary");
    e.push_attribute(("id", binary.id.as_str()));
    e.push_attribute(("content-type", binary.content_type.as_str()));
    w.write_event(Event::Start(e)).unwrap();
    let encoded = base64::engine::general_purpose::STANDARD.encode(&binary.data);
    w.write_event(Event::Text(BytesText::new(&encoded))).unwrap();
    end(w, "binary");
}
