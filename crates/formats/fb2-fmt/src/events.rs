//! SAX-style event iterator and streaming parser for FictionBook 2.
//!
//! # APIs
//!
//! - [`events`] — pull iterator over `&[u8]` input; `EventIter` holds the
//!   `quick_xml::Reader` state and advances one token at a time.
//! - [`StreamingParser<H>`] — chunk-driven; `feed()` + `finish()` deliver
//!   semantic events via a [`Handler`] callback.
//!
//! # Memory note
//!
//! FB2 is XML; `quick_xml` requires the full document in memory to resolve
//! entity references correctly.  `EventIter` holds a slice reference and
//! `StreamingParser` buffers all input before delivering events — memory is
//! O(full input).  This is an inherent property of the XML format, not an
//! implementation limitation.

use std::collections::HashMap;

use base64::Engine;
use quick_xml::{Reader, events::Event as XmlEvent};

use crate::ast::*;

// ---------------------------------------------------------------------------
// Public event type
// ---------------------------------------------------------------------------

/// A semantic FB2 document event.
///
/// Events are produced in document order; block-level constructs are enclosed
/// by matching `Start*`/`End*` pairs.  Leaf constructs (paragraphs, verse
/// lines, images, …) are emitted as a single variant carrying owned data.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// `<FictionBook>` opened.
    StartFictionBook,
    /// `</FictionBook>` closed.
    EndFictionBook,
    /// Complete `<description>` metadata block.
    Metadata(Box<Description>),
    /// `<body name="…">` opened.
    StartBody { name: Option<String>, lang: Option<String> },
    /// `</body>` closed.
    EndBody,
    /// `<section id="…">` opened.
    StartSection { id: Option<String>, lang: Option<String> },
    /// `</section>` closed.
    EndSection,
    /// `<title>` opened.
    StartTitle,
    /// `</title>` closed.
    EndTitle,
    /// A `<p>` inside a `<title>`.
    TitleParagraph(Vec<InlineElement>),
    /// `<p>` opened (outside title context).
    StartParagraph,
    /// `</p>` closed (outside title context).
    EndParagraph,
    /// Inline content for the enclosing paragraph.
    Inline(Vec<InlineElement>),
    /// `<poem>` opened.
    StartPoem,
    /// `</poem>` closed.
    EndPoem,
    /// `<stanza>` opened.
    StartStanza,
    /// `</stanza>` closed.
    EndStanza,
    /// A verse line (`<v>…</v>`).
    VerseLine(Vec<InlineElement>),
    /// `<empty-line/>` encountered.
    EmptyLine,
    /// `<subtitle>` element.
    Subtitle(Vec<InlineElement>),
    /// `<cite id="…">` opened.
    StartCite { id: Option<String> },
    /// `</cite>` closed.
    EndCite,
    /// `<epigraph id="…">` opened.
    StartEpigraph { id: Option<String> },
    /// `</epigraph>` closed.
    EndEpigraph,
    /// `<text-author>` element.
    TextAuthor(Vec<InlineElement>),
    /// A complete `<table>`.
    Table(Table),
    /// An `<image>` element.
    Image(Image),
    /// A `<binary>` element with decoded data.
    Binary(Binary),
}

// ---------------------------------------------------------------------------
// Pull iterator — true incremental XML parsing
// ---------------------------------------------------------------------------

/// Pull iterator over FB2 semantic events.
///
/// `EventIter` holds a `quick_xml::Reader` positioned on `input`; each call
/// to [`Iterator::next`] advances the XML parser and returns at most one
/// semantic [`Event`].  No pre-materialisation of the AST occurs.
pub struct EventIter<'a> {
    inner: XmlEventIter<'a>,
    done: bool,
}

impl<'a> Iterator for EventIter<'a> {
    type Item = Event;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        match self.inner.next_event() {
            Some(ev) => Some(ev),
            None => {
                self.done = true;
                None
            }
        }
    }
}

/// Create a pull iterator over FB2 semantic events from a byte slice.
///
/// The iterator holds the XML reader; no AST is built upfront.
pub fn events(input: &[u8]) -> EventIter<'_> {
    EventIter {
        inner: XmlEventIter::new(input),
        done: false,
    }
}

// ---------------------------------------------------------------------------
// Handler trait + StreamingParser
// ---------------------------------------------------------------------------

/// Callback trait for chunk-driven FB2 parsing.
pub trait Handler {
    fn handle(&mut self, event: Event);
}

impl<F: FnMut(Event)> Handler for F {
    fn handle(&mut self, event: Event) {
        self(event);
    }
}

/// Chunk-driven FB2 parser delivering semantic events to a [`Handler`].
///
/// # Memory note
///
/// FB2 is XML; this implementation buffers the **full document** before
/// delivering any events (see [module-level docs](self) for details).
pub struct StreamingParser<H: Handler> {
    buf: Vec<u8>,
    handler: H,
}

impl<H: Handler> StreamingParser<H> {
    /// Create a new `StreamingParser` delivering events to `handler`.
    pub fn new(handler: H) -> Self {
        StreamingParser { buf: Vec::new(), handler }
    }

    /// Feed a chunk of input bytes.
    pub fn feed(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);
    }

    /// Finish: parse all buffered input and deliver events to the handler.
    pub fn finish(mut self) {
        for ev in events(&self.buf) {
            self.handler.handle(ev);
        }
    }
}

// ---------------------------------------------------------------------------
// Core XML-driven state machine
// ---------------------------------------------------------------------------

/// Internal state machine that drives incremental XML → Event conversion.
struct XmlEventIter<'a> {
    reader: Reader<&'a [u8]>,
    buf: Vec<u8>,
    /// Pending semantic events to yield before reading more XML.
    pending: std::collections::VecDeque<Event>,
    /// Parser state stack.
    stack: Vec<ParseState>,
    /// Accumulated character data for leaf/inline contexts.
    current_text: String,
    /// Accumulated description state (built up before Metadata event).
    desc: DescState,
    /// Accumulated table (built up before Table event).
    current_table: Option<Table>,
    current_table_row: Option<TableRow>,
    current_table_cell: Option<TableCell>,
    done: bool,
}

/// The description-building sub-state (analogous to the full parse.rs TitleInfo etc.)
#[derive(Default)]
struct DescState {
    desc: Description,
    ti: TitleInfo,
    di: Option<DocumentInfo>,
    pi: Option<PublishInfo>,
    current_author_in_ti: Option<Author>,
    current_author_in_di: Option<Author>,
    current_translator: Option<Author>,
    current_custom_info_type: Option<String>,
    in_description: bool,
    in_title_info: bool,
    in_doc_info: bool,
    in_pub_info: bool,
    // pending leaf-text target
    leaf_target: Option<LeafTarget>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum LeafTarget {
    Genre,
    BookTitle,
    Lang,
    SrcLang,
    Keywords,
    Date,
    Version,
    ProgramUsed,
    SrcUrl,
    SrcOcr,
    Id,
    BookName,
    Publisher,
    City,
    Year,
    Isbn,
    FirstName,
    MiddleName,
    LastName,
    Nickname,
    Email,
    CustomInfo,
}

/// Parse-stack items (mirrors the parse.rs StackItem but uses owned Strings
/// so the iterator can live independently of the input slice lifetime).
enum ParseState {
    FictionBook,
    Description,
    /// Marker for an open `<body>` element.
    Body,
    /// Marker for an open `<title>` inside a body.
    BodyTitle,
    /// Marker for an open `<section>` element.
    Section,
    /// Marker for an open `<title>` inside a section.
    SectionTitle,
    TitlePara { inlines: Vec<InlineElement> },
    Paragraph { inlines: Vec<InlineElement> },
    Subtitle { inlines: Vec<InlineElement> },
    /// Marker for an open `<poem>` element.
    Poem,
    /// Marker for an open `<title>` inside a poem.
    PoemTitle,
    /// Marker for an open `<stanza>` element.
    Stanza,
    VerseLine { inlines: Vec<InlineElement> },
    /// Marker for an open `<cite>` element.
    Cite,
    /// Marker for an open `<epigraph>` element.
    Epigraph,
    TextAuthor { inlines: Vec<InlineElement> },
    Annotation,
    InlineWrapper { kind: InlineKind, children: Vec<InlineElement> },
    Link { href: String, link_kind: Option<String>, children: Vec<InlineElement> },
    Binary { id: String, content_type: String },
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum InlineKind {
    Strong,
    Emphasis,
    Strikethrough,
    Sub,
    Sup,
}

impl<'a> XmlEventIter<'a> {
    fn new(input: &'a [u8]) -> Self {
        XmlEventIter {
            reader: Reader::from_reader(input),
            buf: Vec::new(),
            pending: std::collections::VecDeque::new(),
            stack: Vec::new(),
            current_text: String::new(),
            desc: DescState::default(),
            current_table: None,
            current_table_row: None,
            current_table_cell: None,
            done: false,
        }
    }

    /// Advance the XML reader until one or more semantic events are queued,
    /// then return the first queued event.
    fn next_event(&mut self) -> Option<Event> {
        while self.pending.is_empty() && !self.done {
            self.step();
        }
        self.pending.pop_front()
    }

    fn step(&mut self) {
        self.buf.clear();
        match self.reader.read_event_into(&mut self.buf) {
            Ok(XmlEvent::Start(ref e)) => {
                let name = local_name_str(e.local_name().as_ref());
                let attrs = collect_attrs(e);
                self.flush_text_if_needed(&name, false);
                self.handle_start(&name, attrs);
            }
            Ok(XmlEvent::Empty(ref e)) => {
                let name = local_name_str(e.local_name().as_ref());
                let attrs = collect_attrs(e);
                self.flush_text_if_needed(&name, false);
                self.handle_empty(&name, attrs);
            }
            Ok(XmlEvent::End(ref e)) => {
                let name = local_name_str(e.local_name().as_ref());
                self.flush_text_if_needed(&name, true);
                self.handle_end(&name);
            }
            Ok(XmlEvent::Text(ref e)) => {
                self.current_text.push_str(&String::from_utf8_lossy(e.as_ref()));
            }
            Ok(XmlEvent::CData(ref e)) => {
                self.current_text.push_str(&String::from_utf8_lossy(e.as_ref()));
            }
            Ok(XmlEvent::GeneralRef(ref e)) => {
                let n = String::from_utf8_lossy(e);
                match n.as_ref() {
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
            Ok(XmlEvent::Eof) => {
                self.done = true;
            }
            Ok(_) => {}
            Err(_) => {
                self.done = true;
            }
        }
    }

    /// Flush accumulated text to the appropriate context, unless we're at a
    /// leaf-element boundary (where the text is consumed differently).
    fn flush_text_if_needed(&mut self, tag: &str, is_end: bool) {
        // Binary text is consumed raw in handle_end
        if tag == "binary" {
            return;
        }
        // Leaf tags: flush only on End (the text is consumed by handle_end itself)
        if is_leaf_tag(tag) {
            if !is_end {
                // on Start: nothing to flush yet
            }
            return;
        }
        // Custom-info closing: consumed in handle_end
        if tag == "custom-info" && is_end {
            return;
        }
        // For non-leaf, non-binary tags: flush pending text to inline context
        self.flush_inline_text();
    }

    fn flush_inline_text(&mut self) {
        if self.current_text.is_empty() {
            return;
        }
        let text = std::mem::take(&mut self.current_text);

        // In description leaf context (handled separately)
        if let Some(_target) = self.desc.leaf_target {
            // keep it for the leaf handler
            self.current_text = text;
            return;
        }

        // Check if we're in description mode (not yet emitted Metadata)
        if self.desc.in_description {
            // whitespace-only text in metadata is discarded
            return;
        }

        // Whitespace-only outside inline context
        if text.trim().is_empty() && !self.in_inline_context() {
            return;
        }

        self.push_text_to_inline_context(text);
    }

    fn in_inline_context(&self) -> bool {
        for item in self.stack.iter().rev() {
            match item {
                ParseState::Paragraph { .. }
                | ParseState::TitlePara { .. }
                | ParseState::Subtitle { .. }
                | ParseState::VerseLine { .. }
                | ParseState::TextAuthor { .. }
                | ParseState::InlineWrapper { .. }
                | ParseState::Link { .. } => return true,
                ParseState::Section
                | ParseState::Body
                | ParseState::Cite
                | ParseState::Epigraph
                | ParseState::Poem
                | ParseState::Stanza
                | ParseState::SectionTitle
                | ParseState::BodyTitle
                | ParseState::PoemTitle => return false,
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
                ParseState::Paragraph { inlines }
                | ParseState::TitlePara { inlines }
                | ParseState::Subtitle { inlines }
                | ParseState::VerseLine { inlines }
                | ParseState::TextAuthor { inlines } => {
                    inlines.push(InlineElement::Text(text));
                    return;
                }
                ParseState::InlineWrapper { children, .. }
                | ParseState::Link { children, .. } => {
                    children.push(InlineElement::Text(text));
                    return;
                }
                _ => {}
            }
        }
        // Also check table cell
        if let Some(cell) = self.current_table_cell.as_mut() {
            cell.content.push(InlineElement::Text(text));
        }
    }

    fn push_inline(&mut self, el: InlineElement) {
        for item in self.stack.iter_mut().rev() {
            match item {
                ParseState::Paragraph { inlines }
                | ParseState::TitlePara { inlines }
                | ParseState::Subtitle { inlines }
                | ParseState::VerseLine { inlines }
                | ParseState::TextAuthor { inlines } => {
                    inlines.push(el);
                    return;
                }
                ParseState::InlineWrapper { children, .. }
                | ParseState::Link { children, .. } => {
                    children.push(el);
                    return;
                }
                _ => {}
            }
        }
        if let Some(cell) = self.current_table_cell.as_mut() {
            cell.content.push(el);
        }
    }

    fn is_title_context(&self) -> bool {
        matches!(
            self.stack.last(),
            Some(
                ParseState::SectionTitle
                    | ParseState::BodyTitle
                    | ParseState::PoemTitle
            )
        )
    }

    fn handle_start(&mut self, name: &str, attrs: AttrMap) {
        // Description sub-state
        if self.desc.in_description || name == "description" {
            self.handle_desc_start(name, attrs);
            return;
        }

        match name {
            "FictionBook" => {
                self.stack.push(ParseState::FictionBook);
                self.pending.push_back(Event::StartFictionBook);
            }
            "body" => {
                let body_name = attrs.get("name").cloned();
                let lang = attrs.get("lang").cloned();
                self.stack.push(ParseState::Body);
                self.pending.push_back(Event::StartBody { name: body_name, lang });
            }
            "title" => {
                match self.stack.last() {
                    Some(ParseState::Body) => self.stack.push(ParseState::BodyTitle),
                    Some(ParseState::Poem) => self.stack.push(ParseState::PoemTitle),
                    _ => self.stack.push(ParseState::SectionTitle),
                }
                self.pending.push_back(Event::StartTitle);
            }
            "section" => {
                let id = attrs.get("id").cloned();
                let lang = attrs.get("lang").cloned();
                self.stack.push(ParseState::Section);
                self.pending.push_back(Event::StartSection { id, lang });
            }
            "p" => {
                if self.is_title_context() {
                    self.stack.push(ParseState::TitlePara { inlines: Vec::new() });
                } else {
                    self.stack.push(ParseState::Paragraph { inlines: Vec::new() });
                    self.pending.push_back(Event::StartParagraph);
                }
            }
            "subtitle" => self.stack.push(ParseState::Subtitle { inlines: Vec::new() }),
            "epigraph" => {
                let id = attrs.get("id").cloned();
                self.stack.push(ParseState::Epigraph);
                self.pending.push_back(Event::StartEpigraph { id });
            }
            "poem" => {
                self.stack.push(ParseState::Poem);
                self.pending.push_back(Event::StartPoem);
            }
            "stanza" => {
                self.stack.push(ParseState::Stanza);
                self.pending.push_back(Event::StartStanza);
            }
            "v" => self.stack.push(ParseState::VerseLine { inlines: Vec::new() }),
            "cite" => {
                let id = attrs.get("id").cloned();
                self.stack.push(ParseState::Cite);
                self.pending.push_back(Event::StartCite { id });
            }
            "text-author" => self.stack.push(ParseState::TextAuthor { inlines: Vec::new() }),
            "table" => {
                self.current_table = Some(Table {
                    id: attrs.get("id").cloned(),
                    style: attrs.get("style").cloned(),
                    ..Default::default()
                });
            }
            "tr" => {
                self.current_table_row = Some(TableRow {
                    align: attrs.get("align").cloned(),
                    ..Default::default()
                });
            }
            "td" | "th" => {
                self.current_table_cell = Some(TableCell {
                    id: attrs.get("id").cloned(),
                    style: attrs.get("style").cloned(),
                    colspan: attrs.get("colspan").and_then(|v| v.parse().ok()),
                    rowspan: attrs.get("rowspan").and_then(|v| v.parse().ok()),
                    align: attrs.get("align").cloned(),
                    valign: attrs.get("valign").cloned(),
                    is_header: name == "th",
                    ..Default::default()
                });
            }
            "emphasis" => self.stack.push(ParseState::InlineWrapper {
                kind: InlineKind::Emphasis,
                children: Vec::new(),
            }),
            "strong" => self.stack.push(ParseState::InlineWrapper {
                kind: InlineKind::Strong,
                children: Vec::new(),
            }),
            "strikethrough" => self.stack.push(ParseState::InlineWrapper {
                kind: InlineKind::Strikethrough,
                children: Vec::new(),
            }),
            "sub" => self.stack.push(ParseState::InlineWrapper {
                kind: InlineKind::Sub,
                children: Vec::new(),
            }),
            "sup" => self.stack.push(ParseState::InlineWrapper {
                kind: InlineKind::Sup,
                children: Vec::new(),
            }),
            "a" => {
                let href = attrs.get("href").cloned().unwrap_or_default();
                let kind = attrs.get("type").cloned();
                self.stack.push(ParseState::Link { href, link_kind: kind, children: Vec::new() });
            }
            "binary" => {
                let id = attrs.get("id").cloned().unwrap_or_default();
                let content_type = attrs
                    .get("content-type")
                    .cloned()
                    .unwrap_or_else(|| "application/octet-stream".to_string());
                self.stack.push(ParseState::Binary { id, content_type });
            }
            "annotation" => {
                self.stack.push(ParseState::Annotation);
            }
            _ => {}
        }
    }

    fn handle_empty(&mut self, name: &str, attrs: AttrMap) {
        match name {
            "empty-line" => {
                self.pending.push_back(Event::EmptyLine);
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
                    self.pending.push_back(Event::Image(img));
                }
            }
            "sequence" => {
                // handled in description state only
                if self.desc.in_description {
                    let seq = Sequence {
                        name: attrs.get("name").cloned().unwrap_or_default(),
                        number: attrs.get("number").and_then(|v| v.parse().ok()),
                    };
                    if self.desc.in_title_info {
                        self.desc.ti.sequence.push(seq);
                    } else if self.desc.in_pub_info && let Some(pi) = self.desc.pi.as_mut() {
                        pi.sequence.push(seq);
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_end(&mut self, name: &str) {
        // Binary: decode base64
        if name == "binary" {
            if let Some(ParseState::Binary { id, content_type }) = self.stack.pop() {
                let text = std::mem::take(&mut self.current_text);
                let text = text.trim();
                if let Ok(data) = base64::engine::general_purpose::STANDARD.decode(text) {
                    self.pending.push_back(Event::Binary(Binary { id, content_type, data }));
                }
            }
            return;
        }

        // Description
        if name == "description" {
            self.finalize_description();
            return;
        }
        if self.desc.in_description {
            self.handle_desc_end(name);
            return;
        }

        // Leaf text in doc context (code element in inline context)
        if name == "code" {
            let text = std::mem::take(&mut self.current_text).trim().to_string();
            self.push_inline(InlineElement::Code(text));
            return;
        }

        let item = match self.stack.pop() {
            Some(i) => i,
            None => return,
        };

        match item {
            ParseState::FictionBook => {
                self.pending.push_back(Event::EndFictionBook);
            }
            ParseState::Body => {
                self.pending.push_back(Event::EndBody);
            }
            ParseState::BodyTitle => {
                // title content already emitted as TitleParagraph events
                self.pending.push_back(Event::EndTitle);
            }
            ParseState::Section => {
                self.pending.push_back(Event::EndSection);
            }
            ParseState::SectionTitle => {
                self.pending.push_back(Event::EndTitle);
            }
            ParseState::TitlePara { inlines } => {
                self.pending.push_back(Event::TitleParagraph(inlines));
            }
            ParseState::Paragraph { inlines } => {
                self.pending.push_back(Event::Inline(inlines));
                self.pending.push_back(Event::EndParagraph);
            }
            ParseState::Subtitle { inlines } => {
                self.pending.push_back(Event::Subtitle(inlines));
            }
            ParseState::VerseLine { inlines } => {
                self.pending.push_back(Event::VerseLine(inlines));
            }
            ParseState::TextAuthor { inlines } => {
                self.pending.push_back(Event::TextAuthor(inlines));
            }
            ParseState::Poem => {
                self.pending.push_back(Event::EndPoem);
            }
            ParseState::PoemTitle => {
                self.pending.push_back(Event::EndTitle);
            }
            ParseState::Stanza => {
                self.pending.push_back(Event::EndStanza);
            }
            ParseState::Cite => {
                self.pending.push_back(Event::EndCite);
            }
            ParseState::Epigraph => {
                self.pending.push_back(Event::EndEpigraph);
            }
            ParseState::InlineWrapper { kind, children } => {
                let el = match kind {
                    InlineKind::Strong => InlineElement::Strong(children),
                    InlineKind::Emphasis => InlineElement::Emphasis(children),
                    InlineKind::Strikethrough => InlineElement::Strikethrough(children),
                    InlineKind::Sub => InlineElement::Sub(children),
                    InlineKind::Sup => InlineElement::Sup(children),
                };
                self.push_inline(el);
            }
            ParseState::Link { href, link_kind, children } => {
                self.push_inline(InlineElement::Link { href, kind: link_kind, children });
            }
            ParseState::Binary { id, content_type } => {
                // fallback — normally handled at top
                let text = std::mem::take(&mut self.current_text);
                let text = text.trim();
                if let Ok(data) = base64::engine::general_purpose::STANDARD.decode(text) {
                    self.pending.push_back(Event::Binary(Binary { id, content_type, data }));
                }
            }
            ParseState::Description | ParseState::Annotation => {}
        }

        // Handle table element endings outside the stack
        match name {
            "table" => {
                if let Some(t) = self.current_table.take() {
                    self.pending.push_back(Event::Table(t));
                }
            }
            "tr" => {
                if let (Some(row), Some(t)) =
                    (self.current_table_row.take(), self.current_table.as_mut())
                {
                    t.row.push(row);
                }
            }
            "td" | "th" => {
                if let (Some(cell), Some(row)) =
                    (self.current_table_cell.take(), self.current_table_row.as_mut())
                {
                    row.cell.push(cell);
                }
            }
            _ => {}
        }
    }

    // ---- Description state handlers ----------------------------------------

    fn handle_desc_start(&mut self, name: &str, attrs: AttrMap) {
        match name {
            "description" => {
                self.desc.in_description = true;
                self.stack.push(ParseState::Description);
            }
            "title-info" => self.desc.in_title_info = true,
            "document-info" => {
                self.desc.in_doc_info = true;
                self.desc.di = Some(DocumentInfo::default());
            }
            "publish-info" => {
                self.desc.in_pub_info = true;
                self.desc.pi = Some(PublishInfo::default());
            }
            "custom-info" => {
                self.desc.current_custom_info_type =
                    Some(attrs.get("info-type").cloned().unwrap_or_default());
                self.desc.leaf_target = Some(LeafTarget::CustomInfo);
            }
            "author" => {
                if self.desc.in_doc_info {
                    self.desc.current_author_in_di = Some(Author::default());
                } else {
                    self.desc.current_author_in_ti = Some(Author::default());
                }
            }
            "translator" => self.desc.current_translator = Some(Author::default()),
            "genre" => self.desc.leaf_target = Some(LeafTarget::Genre),
            "book-title" => self.desc.leaf_target = Some(LeafTarget::BookTitle),
            "lang" => self.desc.leaf_target = Some(LeafTarget::Lang),
            "src-lang" => self.desc.leaf_target = Some(LeafTarget::SrcLang),
            "keywords" => self.desc.leaf_target = Some(LeafTarget::Keywords),
            "date" => self.desc.leaf_target = Some(LeafTarget::Date),
            "version" => self.desc.leaf_target = Some(LeafTarget::Version),
            "program-used" => self.desc.leaf_target = Some(LeafTarget::ProgramUsed),
            "src-url" => self.desc.leaf_target = Some(LeafTarget::SrcUrl),
            "src-ocr" => self.desc.leaf_target = Some(LeafTarget::SrcOcr),
            "id" => self.desc.leaf_target = Some(LeafTarget::Id),
            "book-name" => self.desc.leaf_target = Some(LeafTarget::BookName),
            "publisher" => self.desc.leaf_target = Some(LeafTarget::Publisher),
            "city" => self.desc.leaf_target = Some(LeafTarget::City),
            "year" => self.desc.leaf_target = Some(LeafTarget::Year),
            "isbn" => self.desc.leaf_target = Some(LeafTarget::Isbn),
            "first-name" => self.desc.leaf_target = Some(LeafTarget::FirstName),
            "middle-name" => self.desc.leaf_target = Some(LeafTarget::MiddleName),
            "last-name" => self.desc.leaf_target = Some(LeafTarget::LastName),
            "nickname" => self.desc.leaf_target = Some(LeafTarget::Nickname),
            "email" => self.desc.leaf_target = Some(LeafTarget::Email),
            "sequence" => {} // handled as empty element
            _ => {}
        }
    }

    fn handle_desc_end(&mut self, name: &str) {
        let text = std::mem::take(&mut self.current_text).trim().to_string();
        let target = self.desc.leaf_target.take();

        match name {
            "author" => {
                if self.desc.in_doc_info {
                    if let (Some(a), Some(di)) =
                        (self.desc.current_author_in_di.take(), self.desc.di.as_mut())
                    {
                        di.author.push(a);
                    }
                } else if let Some(a) = self.desc.current_author_in_ti.take() {
                    self.desc.ti.author.push(a);
                }
            }
            "translator" => {
                if let Some(t) = self.desc.current_translator.take() {
                    self.desc.ti.translator.push(t);
                }
            }
            "title-info" => self.desc.in_title_info = false,
            "document-info" => self.desc.in_doc_info = false,
            "publish-info" => self.desc.in_pub_info = false,
            "custom-info" => {
                if let Some(info_type) = self.desc.current_custom_info_type.take() {
                    self.desc.desc.custom_info.push(CustomInfo { info_type, content: text });
                }
            }
            _ => {
                if let Some(target) = target {
                    self.commit_desc_leaf(target, text);
                }
            }
        }
    }

    fn commit_desc_leaf(&mut self, target: LeafTarget, text: String) {
        // Try author context first
        if let Some(a) = self.desc.current_author_in_ti.as_mut() {
            match target {
                LeafTarget::FirstName => { a.first_name = Some(text); return; }
                LeafTarget::MiddleName => { a.middle_name = Some(text); return; }
                LeafTarget::LastName => { a.last_name = Some(text); return; }
                LeafTarget::Nickname => { a.nickname = Some(text); return; }
                LeafTarget::Email => { a.email.push(text); return; }
                LeafTarget::Id => { a.id = Some(text); return; }
                _ => {}
            }
        }
        if let Some(a) = self.desc.current_author_in_di.as_mut() {
            match target {
                LeafTarget::FirstName => { a.first_name = Some(text); return; }
                LeafTarget::MiddleName => { a.middle_name = Some(text); return; }
                LeafTarget::LastName => { a.last_name = Some(text); return; }
                LeafTarget::Nickname => { a.nickname = Some(text); return; }
                LeafTarget::Email => { a.email.push(text); return; }
                LeafTarget::Id => { a.id = Some(text); return; }
                _ => {}
            }
        }
        if let Some(a) = self.desc.current_translator.as_mut() {
            match target {
                LeafTarget::FirstName => { a.first_name = Some(text); return; }
                LeafTarget::MiddleName => { a.middle_name = Some(text); return; }
                LeafTarget::LastName => { a.last_name = Some(text); return; }
                LeafTarget::Nickname => { a.nickname = Some(text); return; }
                LeafTarget::Email => { a.email.push(text); return; }
                LeafTarget::Id => { a.id = Some(text); return; }
                _ => {}
            }
        }

        // Title-info fields
        if self.desc.in_title_info {
            match target {
                LeafTarget::Genre => { self.desc.ti.genre.push(text); }
                LeafTarget::BookTitle => { self.desc.ti.book_title = text; }
                LeafTarget::Lang => { self.desc.ti.lang = text; }
                LeafTarget::SrcLang => { self.desc.ti.src_lang = Some(text); }
                LeafTarget::Keywords => { self.desc.ti.keywords = Some(text); }
                LeafTarget::Date => { self.desc.ti.date = Some(text); }
                _ => {}
            }
            return;
        }

        // Doc-info fields
        if self.desc.in_doc_info {
            if let Some(di) = self.desc.di.as_mut() {
                match target {
                    LeafTarget::ProgramUsed => { di.program_used = Some(text); }
                    LeafTarget::Date => { di.date = Some(text); }
                    LeafTarget::SrcUrl => { di.src_url.push(text); }
                    LeafTarget::SrcOcr => { di.src_ocr = Some(text); }
                    LeafTarget::Id => { di.id = Some(text); }
                    LeafTarget::Version => { di.version = Some(text); }
                    _ => {}
                }
            }
            return;
        }

        // Pub-info fields
        if self.desc.in_pub_info && let Some(pi) = self.desc.pi.as_mut() {
            match target {
                LeafTarget::BookName => { pi.book_name = Some(text); }
                LeafTarget::Publisher => { pi.publisher = Some(text); }
                LeafTarget::City => { pi.city = Some(text); }
                LeafTarget::Year => { pi.year = Some(text); }
                LeafTarget::Isbn => { pi.isbn = Some(text); }
                _ => {}
            }
        }
    }

    fn finalize_description(&mut self) {
        // Pop Description from stack
        if let Some(ParseState::Description) = self.stack.last() {
            self.stack.pop();
        }
        // Build final Description
        let mut desc = std::mem::take(&mut self.desc.desc);
        desc.title_info = std::mem::take(&mut self.desc.ti);
        desc.document_info = self.desc.di.take();
        desc.publish_info = self.desc.pi.take();
        self.desc.in_description = false;
        self.desc.in_title_info = false;
        self.desc.in_doc_info = false;
        self.desc.in_pub_info = false;
        self.desc.leaf_target = None;
        self.pending.push_back(Event::Metadata(Box::new(desc)));
    }
}

// ---------------------------------------------------------------------------
// Helpers (mirrors parse.rs helpers)
// ---------------------------------------------------------------------------

fn is_leaf_tag(name: &str) -> bool {
    matches!(
        name,
        "genre" | "book-title" | "lang" | "src-lang" | "keywords" | "date" | "version"
        | "program-used" | "src-url" | "src-ocr" | "id" | "book-name" | "publisher"
        | "city" | "year" | "isbn" | "first-name" | "middle-name" | "last-name"
        | "nickname" | "email" | "code" | "sequence"
    )
}

type AttrMap = HashMap<String, String>;

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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parse, emit};

    const BASIC_FB2: &str = r#"<?xml version="1.0"?>
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
    <section id="s1">
      <title><p>Chapter 1</p></title>
      <p>Hello, <emphasis>world</emphasis>!</p>
      <empty-line/>
    </section>
  </body>
  <binary id="cover.jpg" content-type="image/jpeg">AAAA</binary>
</FictionBook>"#;

    /// Collect events from the pull iterator.
    fn collect_events(input: &[u8]) -> Vec<Event> {
        events(input).collect()
    }

    /// Collect events by walking parse() output the old way (using old collect_* logic),
    /// so we can cross-check the two implementations.
    fn events_from_ast(fb: &FictionBook) -> Vec<Event> {
        let mut out = Vec::new();
        collect_ast_events(fb, &mut out);
        out
    }

    fn collect_ast_events(fb: &FictionBook, out: &mut Vec<Event>) {
        out.push(Event::StartFictionBook);
        out.push(Event::Metadata(Box::new(fb.description.clone())));

        for body in &fb.bodies {
            out.push(Event::StartBody {
                name: body.name.clone(),
                lang: body.lang.clone(),
            });
            if let Some(title) = &body.title {
                collect_title_events(title, out);
            }
            for epigraph in &body.epigraph {
                collect_epigraph_events(epigraph, out);
            }
            for section in &body.section {
                collect_section_events(section, out);
            }
            out.push(Event::EndBody);
        }

        for binary in &fb.binaries {
            out.push(Event::Binary(binary.clone()));
        }

        out.push(Event::EndFictionBook);
    }

    fn collect_section_events(section: &Section, out: &mut Vec<Event>) {
        out.push(Event::StartSection {
            id: section.id.clone(),
            lang: section.lang.clone(),
        });
        if let Some(title) = &section.title {
            collect_title_events(title, out);
        }
        for epigraph in &section.epigraph {
            collect_epigraph_events(epigraph, out);
        }
        for content in &section.content {
            collect_section_content_events(content, out);
        }
        for nested in &section.section {
            collect_section_events(nested, out);
        }
        out.push(Event::EndSection);
    }

    fn collect_title_events(title: &Title, out: &mut Vec<Event>) {
        out.push(Event::StartTitle);
        for para in &title.para {
            match para {
                TitlePara::Para(il) => out.push(Event::TitleParagraph(il.clone())),
                TitlePara::EmptyLine => out.push(Event::EmptyLine),
            }
        }
        out.push(Event::EndTitle);
    }

    fn collect_section_content_events(content: &SectionContent, out: &mut Vec<Event>) {
        match content {
            SectionContent::Para(il) => {
                out.push(Event::StartParagraph);
                out.push(Event::Inline(il.clone()));
                out.push(Event::EndParagraph);
            }
            SectionContent::EmptyLine => out.push(Event::EmptyLine),
            SectionContent::Subtitle(il) => out.push(Event::Subtitle(il.clone())),
            SectionContent::Image(img) => out.push(Event::Image(img.clone())),
            SectionContent::Poem(p) => collect_poem_events(p, out),
            SectionContent::Cite(c) => collect_cite_events(c, out),
            SectionContent::Table(t) => out.push(Event::Table(t.clone())),
        }
    }

    fn collect_poem_events(poem: &Poem, out: &mut Vec<Event>) {
        out.push(Event::StartPoem);
        if let Some(title) = &poem.title {
            collect_title_events(title, out);
        }
        for epigraph in &poem.epigraph {
            collect_epigraph_events(epigraph, out);
        }
        for stanza in &poem.stanza {
            out.push(Event::StartStanza);
            for v in &stanza.v {
                out.push(Event::VerseLine(v.clone()));
            }
            out.push(Event::EndStanza);
        }
        for ta in &poem.text_author {
            out.push(Event::TextAuthor(ta.clone()));
        }
        out.push(Event::EndPoem);
    }

    fn collect_cite_events(cite: &Cite, out: &mut Vec<Event>) {
        out.push(Event::StartCite { id: cite.id.clone() });
        for content in &cite.content {
            match content {
                CiteContent::Para(il) => {
                    out.push(Event::StartParagraph);
                    out.push(Event::Inline(il.clone()));
                    out.push(Event::EndParagraph);
                }
                CiteContent::EmptyLine => out.push(Event::EmptyLine),
                CiteContent::Poem(p) => collect_poem_events(p, out),
                CiteContent::Table(t) => out.push(Event::Table(t.clone())),
            }
        }
        for ta in &cite.text_author {
            out.push(Event::TextAuthor(ta.clone()));
        }
        out.push(Event::EndCite);
    }

    fn collect_epigraph_events(epigraph: &Epigraph, out: &mut Vec<Event>) {
        out.push(Event::StartEpigraph { id: epigraph.id.clone() });
        for content in &epigraph.content {
            match content {
                EpigraphContent::Para(il) => {
                    out.push(Event::StartParagraph);
                    out.push(Event::Inline(il.clone()));
                    out.push(Event::EndParagraph);
                }
                EpigraphContent::EmptyLine => out.push(Event::EmptyLine),
                EpigraphContent::Poem(p) => collect_poem_events(p, out),
                EpigraphContent::Cite(c) => collect_cite_events(c, out),
            }
        }
        for ta in &epigraph.text_author {
            out.push(Event::TextAuthor(ta.clone()));
        }
        out.push(Event::EndEpigraph);
    }

    // --- actual tests ---

    #[test]
    fn test_events_basic_sequence() {
        let evts = collect_events(BASIC_FB2.as_bytes());
        assert!(evts.iter().any(|e| matches!(e, Event::StartFictionBook)));
        assert!(evts.iter().any(|e| matches!(e, Event::EndFictionBook)));
        assert!(evts.iter().any(|e| matches!(e, Event::Metadata(_))));
        assert!(evts.iter().any(|e| matches!(e, Event::StartBody { .. })));
        assert!(evts.iter().any(|e| matches!(e, Event::EndBody)));
        assert!(evts.iter().any(|e| matches!(e, Event::StartSection { id: Some(s), .. } if s == "s1")));
        assert!(evts.iter().any(|e| matches!(e, Event::EndSection)));
        assert!(evts.iter().any(|e| matches!(e, Event::EmptyLine)));
    }

    #[test]
    fn test_events_metadata() {
        let evts = collect_events(BASIC_FB2.as_bytes());
        let meta = evts.iter().find_map(|e| {
            if let Event::Metadata(d) = e { Some(d) } else { None }
        });
        let meta = meta.expect("should have Metadata event");
        assert_eq!(meta.title_info.book_title, "Test Book");
        assert_eq!(meta.title_info.lang, "en");
        assert_eq!(meta.title_info.genre, vec!["prose"]);
        assert_eq!(meta.title_info.author[0].first_name.as_deref(), Some("John"));
        assert_eq!(meta.title_info.author[0].last_name.as_deref(), Some("Doe"));
    }

    #[test]
    fn test_events_binary() {
        let evts = collect_events(BASIC_FB2.as_bytes());
        let bin = evts.iter().find_map(|e| {
            if let Event::Binary(b) = e { Some(b) } else { None }
        });
        let bin = bin.expect("should have Binary event");
        assert_eq!(bin.id, "cover.jpg");
        assert_eq!(bin.content_type, "image/jpeg");
    }

    #[test]
    fn test_events_inline_content() {
        let evts = collect_events(BASIC_FB2.as_bytes());
        let inline = evts.iter().find_map(|e| {
            if let Event::Inline(il) = e { Some(il) } else { None }
        });
        let inline = inline.expect("should have Inline event");
        assert!(inline.iter().any(|el| matches!(el, InlineElement::Text(t) if t.contains("Hello"))));
        assert!(inline.iter().any(|el| matches!(el, InlineElement::Emphasis(_))));
    }

    #[test]
    fn test_events_matches_ast_walk() {
        let (fb, _) = parse(BASIC_FB2.as_bytes());
        let ast_events = events_from_ast(&fb);
        let pull_events = collect_events(BASIC_FB2.as_bytes());
        assert_eq!(
            pull_events, ast_events,
            "pull iterator events must match AST-walk events"
        );
    }

    #[test]
    fn test_streaming_parser_same_as_events() {
        // Verify StreamingParser produces the same events regardless of chunk boundaries
        let input = BASIC_FB2.as_bytes();

        let expected: Vec<Event> = events(input).collect();

        // Feed byte-by-byte
        let mut collected = Vec::new();
        let mut sp = StreamingParser::new(|ev: Event| collected.push(ev));
        for b in input {
            sp.feed(std::slice::from_ref(b));
        }
        sp.finish();

        assert_eq!(collected, expected, "StreamingParser byte-by-byte must equal events()");

        // Feed in two halves
        let mid = input.len() / 2;
        let mut collected2 = Vec::new();
        let mut sp2 = StreamingParser::new(|ev: Event| collected2.push(ev));
        sp2.feed(&input[..mid]);
        sp2.feed(&input[mid..]);
        sp2.finish();

        assert_eq!(collected2, expected, "StreamingParser split-chunk must equal events()");
    }

    #[test]
    fn test_streaming_parser_poem() {
        let xml = r#"<?xml version="1.0"?>
<FictionBook xmlns="http://www.gribuser.ru/xml/fictionbook/2.0">
  <description><title-info><book-title>P</book-title><lang>en</lang></title-info></description>
  <body>
    <section>
      <poem>
        <stanza>
          <v>First line</v>
          <v>Second line</v>
        </stanza>
        <text-author>The Poet</text-author>
      </poem>
    </section>
  </body>
</FictionBook>"#;
        let evts: Vec<Event> = events(xml.as_bytes()).collect();
        assert!(evts.iter().any(|e| matches!(e, Event::StartPoem)));
        assert!(evts.iter().any(|e| matches!(e, Event::EndPoem)));
        assert!(evts.iter().any(|e| matches!(e, Event::StartStanza)));
        assert!(evts.iter().any(|e| matches!(e, Event::EndStanza)));
        assert_eq!(
            evts.iter()
                .filter(|e| matches!(e, Event::VerseLine(_)))
                .count(),
            2
        );
        assert!(evts.iter().any(|e| matches!(e, Event::TextAuthor(_))));
    }

    #[test]
    fn test_events_cite_epigraph() {
        let xml = r#"<?xml version="1.0"?>
<FictionBook xmlns="http://www.gribuser.ru/xml/fictionbook/2.0">
  <description><title-info><book-title>C</book-title><lang>en</lang></title-info></description>
  <body>
    <section>
      <epigraph id="e1">
        <p>An epigraph</p>
        <text-author>Author</text-author>
      </epigraph>
      <cite id="c1">
        <p>A citation</p>
      </cite>
    </section>
  </body>
</FictionBook>"#;
        let evts: Vec<Event> = events(xml.as_bytes()).collect();
        assert!(evts.iter().any(|e| matches!(e, Event::StartEpigraph { id: Some(s), .. } if s == "e1")));
        assert!(evts.iter().any(|e| matches!(e, Event::EndEpigraph)));
        assert!(evts.iter().any(|e| matches!(e, Event::StartCite { id: Some(s), .. } if s == "c1")));
        assert!(evts.iter().any(|e| matches!(e, Event::EndCite)));
    }

    #[test]
    fn test_events_roundtrip_via_emit() {
        // parse → emit → events should produce the same content events as events on original
        let input = BASIC_FB2.as_bytes();
        let (fb, _) = parse(input);
        let emitted = emit(&fb);
        let evts1: Vec<Event> = events(input).collect();
        let evts2: Vec<Event> = events(&emitted).collect();
        assert_eq!(evts1, evts2, "events on original and re-emitted must match");
    }
}
