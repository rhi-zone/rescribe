use crate::ast::*;
use base64::Engine;
use quick_xml::{Reader, events::Event};

pub fn parse(input: &[u8]) -> (FictionBook, Vec<Diagnostic>) {
    let mut reader = Reader::from_reader(input);
    // Do not use trim_text(true) — it strips spaces adjacent to entity refs

    let mut parser = Parser::new();
    parser.run(&mut reader);
    (parser.fb, parser.diags)
}

pub fn parse_str(input: &str) -> (FictionBook, Vec<Diagnostic>) {
    parse(input.as_bytes())
}

struct Parser {
    fb: FictionBook,
    diags: Vec<Diagnostic>,
    stack: Vec<StackItem>,
    current_text: String,
}

/// Stack items represent open elements.
enum StackItem {
    FictionBook,
    Description,
    TitleInfo { ti: TitleInfo },
    DocumentInfo { di: DocumentInfo },
    PublishInfo { pi: PublishInfo },
    CustomInfo { info_type: String },
    AuthorInTitleInfo { a: Author },
    AuthorInDocInfo { a: Author },
    Translator { a: Author },
    /// A text-only leaf element in metadata (e.g. <genre>, <book-title>, <first-name>)
    /// Text accumulates in current_text; the tag name tells commit_leaf_text what to do.
    LeafText,
    Body { b: Body },
    BodyTitle { title: Title },
    Section { s: Section },
    SectionTitle { title: Title },
    TitlePara { inlines: Vec<InlineElement> },
    Paragraph { inlines: Vec<InlineElement> },
    Subtitle { inlines: Vec<InlineElement> },
    Poem { p: Poem },
    PoemTitle { title: Title },
    Stanza { s: Stanza },
    VerseLine { inlines: Vec<InlineElement> },
    Cite { c: Cite },
    Epigraph { e: Epigraph },
    TextAuthor { inlines: Vec<InlineElement> },
    Annotation { ann: Annotation },
    Table { t: Table },
    TableRow { row: TableRow },
    TableCell { cell: TableCell },
    Binary { id: String, content_type: String },
    InlineWrapper { kind: InlineKind, children: Vec<InlineElement> },
    Link { href: String, link_kind: Option<String>, children: Vec<InlineElement> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InlineKind {
    Strong,
    Emphasis,
    Strikethrough,
    Sub,
    Sup,
}

/// Returns true if the top of the stack is a title context
fn is_title_context(stack: &[StackItem]) -> bool {
    matches!(
        stack.last(),
        Some(
            StackItem::SectionTitle { .. }
                | StackItem::BodyTitle { .. }
                | StackItem::PoemTitle { .. }
        )
    )
}

impl Parser {
    fn new() -> Self {
        Self {
            fb: FictionBook::default(),
            diags: Vec::new(),
            stack: Vec::new(),
            current_text: String::new(),
        }
    }

    fn run(&mut self, reader: &mut Reader<&[u8]>) {
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    self.flush_text();
                    let name = local_name_str(e.local_name().as_ref());
                    let attrs = collect_attrs(&e);
                    self.handle_start(&name, attrs);
                }
                Ok(Event::Empty(e)) => {
                    self.flush_text();
                    let name = local_name_str(e.local_name().as_ref());
                    let attrs = collect_attrs(&e);
                    self.handle_empty(&name, attrs);
                }
                Ok(Event::End(e)) => {
                    let name = local_name_str(e.local_name().as_ref());
                    // For leaf text elements and binary, don't flush (preserve current_text)
                    let is_leaf = is_leaf_tag(&name);
                    let is_binary = name == "binary";
                    if !is_leaf && !is_binary {
                        self.flush_text();
                    }
                    self.handle_end(&name);
                }
                Ok(Event::Text(e)) => {
                    self.current_text
                        .push_str(&String::from_utf8_lossy(e.as_ref()));
                }
                Ok(Event::CData(e)) => {
                    self.current_text
                        .push_str(&String::from_utf8_lossy(e.as_ref()));
                }
                Ok(Event::GeneralRef(e)) => {
                    let name = String::from_utf8_lossy(&e);
                    match name.as_ref() {
                        "amp" => self.current_text.push('&'),
                        "lt" => self.current_text.push('<'),
                        "gt" => self.current_text.push('>'),
                        "apos" => self.current_text.push('\''),
                        "quot" => self.current_text.push('"'),
                        s if s.starts_with('#') => {
                            let digits = &s[1..];
                            let code = if let Some(hex) = digits.strip_prefix('x') {
                                u32::from_str_radix(hex, 16).ok()
                            } else {
                                digits.parse::<u32>().ok()
                            };
                            if let Some(c) = code.and_then(char::from_u32) {
                                self.current_text.push(c);
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::Eof) => break,
                Ok(_) => {}
                Err(e) => {
                    self.diags.push(Diagnostic {
                        severity: Severity::Error,
                        message: format!("XML parse error: {e}"),
                    });
                    break;
                }
            }
            buf.clear();
        }
    }

    fn flush_text(&mut self) {
        if self.current_text.is_empty() {
            return;
        }
        let text = std::mem::take(&mut self.current_text);

        // LeafText context: text stays in current_text to be consumed by handle_end
        if matches!(self.stack.last(), Some(StackItem::LeafText)) {
            self.current_text = text;
            return;
        }

        // Whitespace-only text is dropped unless we're inside an inline context
        if text.trim().is_empty() && !self.in_inline_context() {
            return;
        }

        self.push_text_to_inline_context(text);
    }

    fn in_inline_context(&self) -> bool {
        for item in self.stack.iter().rev() {
            match item {
                StackItem::Paragraph { .. }
                | StackItem::TitlePara { .. }
                | StackItem::Subtitle { .. }
                | StackItem::VerseLine { .. }
                | StackItem::TextAuthor { .. }
                | StackItem::InlineWrapper { .. }
                | StackItem::Link { .. }
                | StackItem::TableCell { .. } => return true,
                // Block contexts stop the search
                StackItem::Section { .. }
                | StackItem::Body { .. }
                | StackItem::Cite { .. }
                | StackItem::Epigraph { .. }
                | StackItem::Poem { .. }
                | StackItem::Stanza { .. }
                | StackItem::SectionTitle { .. }
                | StackItem::BodyTitle { .. }
                | StackItem::PoemTitle { .. }
                | StackItem::LeafText => return false,
                _ => continue,
            }
        }
        false
    }

    fn push_text_to_inline_context(&mut self, text: String) {
        if text.is_empty() {
            return;
        }
        for item in self.stack.iter_mut().rev() {
            match item {
                StackItem::Paragraph { inlines }
                | StackItem::TitlePara { inlines }
                | StackItem::Subtitle { inlines }
                | StackItem::VerseLine { inlines }
                | StackItem::TextAuthor { inlines } => {
                    inlines.push(InlineElement::Text(text));
                    return;
                }
                StackItem::InlineWrapper { children, .. } | StackItem::Link { children, .. } => {
                    children.push(InlineElement::Text(text));
                    return;
                }
                StackItem::TableCell { cell } => {
                    cell.content.push(InlineElement::Text(text));
                    return;
                }
                _ => {}
            }
        }
        // No inline context — drop
    }

    fn handle_start(&mut self, name: &str, attrs: AttrMap) {
        match name {
            "FictionBook" => self.stack.push(StackItem::FictionBook),
            "description" => self.stack.push(StackItem::Description),
            "title-info" => self.stack.push(StackItem::TitleInfo { ti: TitleInfo::default() }),
            "document-info" => {
                self.stack.push(StackItem::DocumentInfo { di: DocumentInfo::default() });
            }
            "publish-info" => {
                self.stack.push(StackItem::PublishInfo { pi: PublishInfo::default() });
            }
            "custom-info" => {
                self.stack.push(StackItem::CustomInfo {
                    info_type: attrs.get("info-type").cloned().unwrap_or_default(),
                });
            }
            "author" => {
                let in_doc_info = self
                    .stack
                    .iter()
                    .any(|s| matches!(s, StackItem::DocumentInfo { .. }));
                if in_doc_info {
                    self.stack.push(StackItem::AuthorInDocInfo { a: Author::default() });
                } else {
                    self.stack.push(StackItem::AuthorInTitleInfo { a: Author::default() });
                }
            }
            "translator" => self.stack.push(StackItem::Translator { a: Author::default() }),
            // Leaf text elements in metadata / inline code
            "genre" | "book-title" | "lang" | "src-lang" | "keywords" | "date" | "version"
            | "program-used" | "src-url" | "src-ocr" | "id" | "book-name" | "publisher"
            | "city" | "year" | "isbn" | "first-name" | "middle-name" | "last-name"
            | "nickname" | "email" | "code" | "sequence" => {
                self.stack.push(StackItem::LeafText);
            }
            "annotation" => self.stack.push(StackItem::Annotation {
                ann: Annotation {
                    id: attrs.get("id").cloned(),
                    ..Default::default()
                },
            }),
            "body" => self.stack.push(StackItem::Body {
                b: Body {
                    name: attrs.get("name").cloned(),
                    lang: attrs.get("lang").cloned(),
                    ..Default::default()
                },
            }),
            "title" => {
                match self.stack.last() {
                    Some(StackItem::Body { .. }) => {
                        self.stack.push(StackItem::BodyTitle {
                            title: Title { lang: attrs.get("lang").cloned(), ..Default::default() },
                        });
                    }
                    Some(StackItem::Poem { .. }) => {
                        self.stack.push(StackItem::PoemTitle {
                            title: Title { lang: attrs.get("lang").cloned(), ..Default::default() },
                        });
                    }
                    _ => {
                        self.stack.push(StackItem::SectionTitle {
                            title: Title { lang: attrs.get("lang").cloned(), ..Default::default() },
                        });
                    }
                }
            }
            "section" => self.stack.push(StackItem::Section {
                s: Section {
                    id: attrs.get("id").cloned(),
                    lang: attrs.get("lang").cloned(),
                    ..Default::default()
                },
            }),
            "p" => {
                if is_title_context(&self.stack) {
                    self.stack.push(StackItem::TitlePara { inlines: Vec::new() });
                } else {
                    self.stack.push(StackItem::Paragraph { inlines: Vec::new() });
                }
            }
            "subtitle" => self.stack.push(StackItem::Subtitle { inlines: Vec::new() }),
            "epigraph" => self.stack.push(StackItem::Epigraph {
                e: Epigraph {
                    id: attrs.get("id").cloned(),
                    ..Default::default()
                },
            }),
            "poem" => self.stack.push(StackItem::Poem {
                p: Poem {
                    id: attrs.get("id").cloned(),
                    lang: attrs.get("lang").cloned(),
                    ..Default::default()
                },
            }),
            "stanza" => self.stack.push(StackItem::Stanza {
                s: Stanza {
                    lang: attrs.get("lang").cloned(),
                    ..Default::default()
                },
            }),
            "v" => self.stack.push(StackItem::VerseLine { inlines: Vec::new() }),
            "cite" => self.stack.push(StackItem::Cite {
                c: Cite {
                    id: attrs.get("id").cloned(),
                    lang: attrs.get("lang").cloned(),
                    ..Default::default()
                },
            }),
            "text-author" => self.stack.push(StackItem::TextAuthor { inlines: Vec::new() }),
            "table" => self.stack.push(StackItem::Table {
                t: Table {
                    id: attrs.get("id").cloned(),
                    style: attrs.get("style").cloned(),
                    ..Default::default()
                },
            }),
            "tr" => self.stack.push(StackItem::TableRow {
                row: TableRow {
                    align: attrs.get("align").cloned(),
                    ..Default::default()
                },
            }),
            "td" => self.stack.push(StackItem::TableCell {
                cell: TableCell {
                    id: attrs.get("id").cloned(),
                    style: attrs.get("style").cloned(),
                    colspan: attrs.get("colspan").and_then(|v| v.parse().ok()),
                    rowspan: attrs.get("rowspan").and_then(|v| v.parse().ok()),
                    align: attrs.get("align").cloned(),
                    valign: attrs.get("valign").cloned(),
                    is_header: false,
                    ..Default::default()
                },
            }),
            "th" => self.stack.push(StackItem::TableCell {
                cell: TableCell {
                    id: attrs.get("id").cloned(),
                    style: attrs.get("style").cloned(),
                    colspan: attrs.get("colspan").and_then(|v| v.parse().ok()),
                    rowspan: attrs.get("rowspan").and_then(|v| v.parse().ok()),
                    align: attrs.get("align").cloned(),
                    valign: attrs.get("valign").cloned(),
                    is_header: true,
                    ..Default::default()
                },
            }),
            "emphasis" => self.stack.push(StackItem::InlineWrapper {
                kind: InlineKind::Emphasis,
                children: Vec::new(),
            }),
            "strong" => self.stack.push(StackItem::InlineWrapper {
                kind: InlineKind::Strong,
                children: Vec::new(),
            }),
            "strikethrough" => self.stack.push(StackItem::InlineWrapper {
                kind: InlineKind::Strikethrough,
                children: Vec::new(),
            }),
            "sub" => self.stack.push(StackItem::InlineWrapper {
                kind: InlineKind::Sub,
                children: Vec::new(),
            }),
            "sup" => self.stack.push(StackItem::InlineWrapper {
                kind: InlineKind::Sup,
                children: Vec::new(),
            }),
            "a" => {
                let href = attrs.get("href").cloned().unwrap_or_default();
                let kind = attrs.get("type").cloned();
                self.stack.push(StackItem::Link {
                    href,
                    link_kind: kind,
                    children: Vec::new(),
                });
            }
            "binary" => {
                let id = attrs.get("id").cloned().unwrap_or_default();
                let content_type = attrs
                    .get("content-type")
                    .cloned()
                    .unwrap_or_else(|| "application/octet-stream".to_string());
                self.stack.push(StackItem::Binary { id, content_type });
            }
            _ => {}
        }
    }

    fn handle_empty(&mut self, name: &str, attrs: AttrMap) {
        match name {
            "empty-line" => {
                self.push_block_content(SectionContent::EmptyLine);
            }
            "image" => {
                let href = attrs.get("href").cloned().unwrap_or_default();
                let img = Image {
                    href,
                    alt: attrs.get("alt").cloned(),
                    title: attrs.get("title").cloned(),
                    id: attrs.get("id").cloned(),
                };
                if self.in_inline_context() {
                    self.push_inline(InlineElement::Image(img));
                } else {
                    self.push_block_content(SectionContent::Image(img));
                }
            }
            "sequence" => {
                let seq = Sequence {
                    name: attrs.get("name").cloned().unwrap_or_default(),
                    number: attrs.get("number").and_then(|v| v.parse().ok()),
                };
                self.push_sequence(seq);
            }
            _ => {}
        }
    }

    fn push_inline(&mut self, el: InlineElement) {
        for item in self.stack.iter_mut().rev() {
            match item {
                StackItem::Paragraph { inlines }
                | StackItem::TitlePara { inlines }
                | StackItem::Subtitle { inlines }
                | StackItem::VerseLine { inlines }
                | StackItem::TextAuthor { inlines } => {
                    inlines.push(el);
                    return;
                }
                StackItem::InlineWrapper { children, .. } | StackItem::Link { children, .. } => {
                    children.push(el);
                    return;
                }
                StackItem::TableCell { cell } => {
                    cell.content.push(el);
                    return;
                }
                _ => {}
            }
        }
    }

    fn push_sequence(&mut self, seq: Sequence) {
        for item in self.stack.iter_mut().rev() {
            match item {
                StackItem::TitleInfo { ti } => { ti.sequence.push(seq); return; }
                StackItem::PublishInfo { pi } => { pi.sequence.push(seq); return; }
                _ => {}
            }
        }
    }

    fn push_block_content(&mut self, content: SectionContent) {
        for item in self.stack.iter_mut().rev() {
            match item {
                StackItem::Section { s } => {
                    s.content.push(content);
                    return;
                }
                StackItem::Cite { c } => {
                    let cc = match content {
                        SectionContent::Para(il) => CiteContent::Para(il),
                        SectionContent::EmptyLine => CiteContent::EmptyLine,
                        SectionContent::Poem(p) => CiteContent::Poem(p),
                        SectionContent::Table(t) => CiteContent::Table(t),
                        other => { let _ = other; return; }
                    };
                    c.content.push(cc);
                    return;
                }
                StackItem::Epigraph { e } => {
                    let ec = match content {
                        SectionContent::Para(il) => EpigraphContent::Para(il),
                        SectionContent::EmptyLine => EpigraphContent::EmptyLine,
                        SectionContent::Poem(p) => EpigraphContent::Poem(p),
                        SectionContent::Cite(c) => EpigraphContent::Cite(c),
                        other => { let _ = other; return; }
                    };
                    e.content.push(ec);
                    return;
                }
                StackItem::Annotation { ann } => {
                    let ac = match content {
                        SectionContent::Para(il) => AnnotationContent::Para(il),
                        SectionContent::EmptyLine => AnnotationContent::EmptyLine,
                        SectionContent::Poem(p) => AnnotationContent::Poem(p),
                        SectionContent::Cite(c) => AnnotationContent::Cite(c),
                        SectionContent::Subtitle(il) => AnnotationContent::Subtitle(il),
                        SectionContent::Table(t) => AnnotationContent::Table(t),
                        SectionContent::Image(_) => return,
                    };
                    ann.content.push(ac);
                    return;
                }
                _ => {}
            }
        }
    }

    fn handle_end(&mut self, name: &str) {
        // Binary: consume current_text as base64
        if name == "binary" {
            if let Some(StackItem::Binary { id, content_type }) = self.stack.pop() {
                let text = std::mem::take(&mut self.current_text);
                let text = text.trim();
                match base64::engine::general_purpose::STANDARD.decode(text) {
                    Ok(data) => self.fb.binaries.push(Binary { id, content_type, data }),
                    Err(_) => self.diags.push(Diagnostic {
                        severity: Severity::Warning,
                        message: "Failed to decode binary data".to_string(),
                    }),
                }
            }
            return;
        }

        // Leaf text elements: consume current_text
        if is_leaf_tag(name) {
            if let Some(StackItem::LeafText) = self.stack.pop() {
                let text = std::mem::take(&mut self.current_text).trim().to_string();
                if name == "code" {
                    self.push_inline(InlineElement::Code(text));
                } else {
                    self.commit_leaf_text(name, text);
                }
            }
            return;
        }

        let item = match self.stack.pop() {
            Some(i) => i,
            None => return,
        };

        match item {
            StackItem::FictionBook => {}
            StackItem::Description => {}
            StackItem::LeafText => {} // handled above

            StackItem::TitleInfo { ti } => {
                self.fb.description.title_info = ti;
            }
            StackItem::DocumentInfo { di } => {
                self.fb.description.document_info = Some(di);
            }
            StackItem::PublishInfo { pi } => {
                self.fb.description.publish_info = Some(pi);
            }
            StackItem::CustomInfo { info_type } => {
                let text = std::mem::take(&mut self.current_text).trim().to_string();
                self.fb.description.custom_info.push(CustomInfo { info_type, content: text });
            }

            StackItem::AuthorInTitleInfo { a } => {
                for item in self.stack.iter_mut().rev() {
                    if let StackItem::TitleInfo { ti } = item {
                        ti.author.push(a);
                        return;
                    }
                }
            }
            StackItem::AuthorInDocInfo { a } => {
                for item in self.stack.iter_mut().rev() {
                    if let StackItem::DocumentInfo { di } = item {
                        di.author.push(a);
                        return;
                    }
                }
            }
            StackItem::Translator { a } => {
                for item in self.stack.iter_mut().rev() {
                    if let StackItem::TitleInfo { ti } = item {
                        ti.translator.push(a);
                        return;
                    }
                }
            }

            StackItem::Annotation { ann } => {
                for item in self.stack.iter_mut().rev() {
                    match item {
                        StackItem::TitleInfo { ti } => { ti.annotation = Some(ann); return; }
                        StackItem::Section { s } => { s.annotation = Some(ann); return; }
                        _ => {}
                    }
                }
            }

            StackItem::Body { b } => {
                self.fb.bodies.push(b);
            }
            StackItem::BodyTitle { title } => {
                for item in self.stack.iter_mut().rev() {
                    if let StackItem::Body { b } = item {
                        b.title = Some(title);
                        return;
                    }
                }
            }

            StackItem::Section { s } => {
                for item in self.stack.iter_mut().rev() {
                    match item {
                        StackItem::Section { s: parent } => { parent.section.push(s); return; }
                        StackItem::Body { b } => { b.section.push(s); return; }
                        _ => {}
                    }
                }
            }
            StackItem::SectionTitle { title } => {
                for item in self.stack.iter_mut().rev() {
                    if let StackItem::Section { s } = item {
                        s.title = Some(title);
                        return;
                    }
                }
            }

            StackItem::TitlePara { inlines } => {
                let tp = TitlePara::Para(inlines);
                for item in self.stack.iter_mut().rev() {
                    match item {
                        StackItem::SectionTitle { title }
                        | StackItem::BodyTitle { title }
                        | StackItem::PoemTitle { title } => {
                            title.para.push(tp);
                            return;
                        }
                        _ => {}
                    }
                }
            }

            StackItem::Paragraph { inlines } => {
                self.push_block_content(SectionContent::Para(inlines));
            }

            StackItem::Subtitle { inlines } => {
                self.push_block_content(SectionContent::Subtitle(inlines));
            }

            StackItem::Poem { p } => {
                self.push_block_content(SectionContent::Poem(p));
            }
            StackItem::PoemTitle { title } => {
                for item in self.stack.iter_mut().rev() {
                    if let StackItem::Poem { p } = item {
                        p.title = Some(title);
                        return;
                    }
                }
            }

            StackItem::Stanza { s } => {
                for item in self.stack.iter_mut().rev() {
                    if let StackItem::Poem { p } = item {
                        p.stanza.push(s);
                        return;
                    }
                }
            }
            StackItem::VerseLine { inlines } => {
                for item in self.stack.iter_mut().rev() {
                    if let StackItem::Stanza { s } = item {
                        s.v.push(inlines);
                        return;
                    }
                }
            }

            StackItem::Cite { c } => {
                let cite = c;
                for item in self.stack.iter_mut().rev() {
                    match item {
                        StackItem::Section { s } => { s.content.push(SectionContent::Cite(cite)); return; }
                        StackItem::Epigraph { e } => { e.content.push(EpigraphContent::Cite(cite)); return; }
                        StackItem::Annotation { ann } => { ann.content.push(AnnotationContent::Cite(cite)); return; }
                        _ => {}
                    }
                }
            }
            StackItem::Epigraph { e } => {
                for item in self.stack.iter_mut().rev() {
                    match item {
                        StackItem::Section { s } => { s.epigraph.push(e); return; }
                        StackItem::Body { b } => { b.epigraph.push(e); return; }
                        StackItem::Poem { p } => { p.epigraph.push(e); return; }
                        _ => {}
                    }
                }
            }
            StackItem::TextAuthor { inlines } => {
                for item in self.stack.iter_mut().rev() {
                    match item {
                        StackItem::Cite { c } => { c.text_author.push(inlines); return; }
                        StackItem::Epigraph { e } => { e.text_author.push(inlines); return; }
                        StackItem::Poem { p } => { p.text_author.push(inlines); return; }
                        _ => {}
                    }
                }
            }

            StackItem::Table { t } => {
                self.push_block_content(SectionContent::Table(t));
            }
            StackItem::TableRow { row } => {
                for item in self.stack.iter_mut().rev() {
                    if let StackItem::Table { t } = item {
                        t.row.push(row);
                        return;
                    }
                }
            }
            StackItem::TableCell { cell } => {
                for item in self.stack.iter_mut().rev() {
                    if let StackItem::TableRow { row } = item {
                        row.cell.push(cell);
                        return;
                    }
                }
            }

            StackItem::InlineWrapper { kind, children } => {
                let el = match kind {
                    InlineKind::Strong => InlineElement::Strong(children),
                    InlineKind::Emphasis => InlineElement::Emphasis(children),
                    InlineKind::Strikethrough => InlineElement::Strikethrough(children),
                    InlineKind::Sub => InlineElement::Sub(children),
                    InlineKind::Sup => InlineElement::Sup(children),
                };
                self.push_inline(el);
            }
            StackItem::Link { href, link_kind, children } => {
                self.push_inline(InlineElement::Link { href, kind: link_kind, children });
            }

            StackItem::Binary { id, content_type } => {
                // Handled at top but just in case
                let text = std::mem::take(&mut self.current_text);
                let text = text.trim();
                if let Ok(data) = base64::engine::general_purpose::STANDARD.decode(text) {
                    self.fb.binaries.push(Binary { id, content_type, data });
                }
            }
        }
    }

    fn commit_leaf_text(&mut self, tag: &str, text: String) {
        for item in self.stack.iter_mut().rev() {
            match item {
                StackItem::AuthorInTitleInfo { a }
                | StackItem::AuthorInDocInfo { a }
                | StackItem::Translator { a } => {
                    match tag {
                        "first-name" => { a.first_name = Some(text); return; }
                        "middle-name" => { a.middle_name = Some(text); return; }
                        "last-name" => { a.last_name = Some(text); return; }
                        "nickname" => { a.nickname = Some(text); return; }
                        "email" => { a.email.push(text); return; }
                        "id" => { a.id = Some(text); return; }
                        _ => {}
                    }
                }
                StackItem::TitleInfo { ti } => {
                    match tag {
                        "genre" => { ti.genre.push(text); return; }
                        "book-title" => { ti.book_title = text; return; }
                        "lang" => { ti.lang = text; return; }
                        "src-lang" => { ti.src_lang = Some(text); return; }
                        "keywords" => { ti.keywords = Some(text); return; }
                        "date" => { ti.date = Some(text); return; }
                        _ => {}
                    }
                }
                StackItem::DocumentInfo { di } => {
                    match tag {
                        "program-used" => { di.program_used = Some(text); return; }
                        "date" => { di.date = Some(text); return; }
                        "src-url" => { di.src_url.push(text); return; }
                        "src-ocr" => { di.src_ocr = Some(text); return; }
                        "id" => { di.id = Some(text); return; }
                        "version" => { di.version = Some(text); return; }
                        _ => {}
                    }
                }
                StackItem::PublishInfo { pi } => {
                    match tag {
                        "book-name" => { pi.book_name = Some(text); return; }
                        "publisher" => { pi.publisher = Some(text); return; }
                        "city" => { pi.city = Some(text); return; }
                        "year" => { pi.year = Some(text); return; }
                        "isbn" => { pi.isbn = Some(text); return; }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }
}

fn is_leaf_tag(name: &str) -> bool {
    matches!(
        name,
        "genre" | "book-title" | "lang" | "src-lang" | "keywords" | "date" | "version"
        | "program-used" | "src-url" | "src-ocr" | "id" | "book-name" | "publisher"
        | "city" | "year" | "isbn" | "first-name" | "middle-name" | "last-name"
        | "nickname" | "email" | "code" | "sequence"
    )
}

type AttrMap = std::collections::HashMap<String, String>;

fn local_name_str(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn collect_attrs(e: &quick_xml::events::BytesStart<'_>) -> AttrMap {
    let mut map = AttrMap::new();
    for attr in e.attributes().flatten() {
        let key = local_name_str(attr.key.local_name().as_ref());
        let value = String::from_utf8_lossy(&attr.value).into_owned();
        map.insert(key, value);
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic() {
        let xml = r#"<?xml version="1.0"?>
<FictionBook xmlns="http://www.gribuser.ru/xml/fictionbook/2.0">
  <description>
    <title-info>
      <genre>prose</genre>
      <author><first-name>John</first-name><last-name>Doe</last-name></author>
      <book-title>Test Book</book-title>
      <lang>en</lang>
    </title-info>
  </description>
  <body>
    <section>
      <title><p>Chapter 1</p></title>
      <p>Hello, world!</p>
    </section>
  </body>
</FictionBook>"#;
        let (fb, diags) = parse(xml.as_bytes());
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
        assert_eq!(fb.description.title_info.book_title, "Test Book");
        assert_eq!(fb.description.title_info.lang, "en");
        assert_eq!(fb.description.title_info.genre, vec!["prose"]);
        assert_eq!(
            fb.description.title_info.author[0].first_name.as_deref(),
            Some("John")
        );
        assert_eq!(fb.bodies.len(), 1);
        assert_eq!(fb.bodies[0].section.len(), 1);
    }

    #[test]
    fn test_roundtrip() {
        use crate::emit::emit;
        let xml = r#"<?xml version="1.0"?>
<FictionBook xmlns="http://www.gribuser.ru/xml/fictionbook/2.0">
  <description>
    <title-info>
      <genre>prose</genre>
      <book-title>Roundtrip Test</book-title>
      <lang>en</lang>
    </title-info>
  </description>
  <body>
    <section>
      <p>Hello <emphasis>world</emphasis>!</p>
    </section>
  </body>
</FictionBook>"#;
        let (fb1, _) = parse(xml.as_bytes());
        let out = emit(&fb1);
        let (fb2, _) = parse(&out);
        assert_eq!(fb1, fb2);
    }
}
