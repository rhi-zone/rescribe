/// A parsed FictionBook 2 document.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FictionBook {
    pub description: Description,
    pub bodies: Vec<Body>,
    pub binaries: Vec<Binary>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Description {
    pub title_info: TitleInfo,
    pub document_info: Option<DocumentInfo>,
    pub publish_info: Option<PublishInfo>,
    pub custom_info: Vec<CustomInfo>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TitleInfo {
    pub genre: Vec<String>,
    pub author: Vec<Author>,
    pub book_title: String,
    pub annotation: Option<Annotation>,
    pub keywords: Option<String>,
    pub date: Option<String>,
    pub coverpage: Option<Vec<InlineElement>>,
    pub lang: String,
    pub src_lang: Option<String>,
    pub translator: Vec<Author>,
    pub sequence: Vec<Sequence>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Author {
    pub first_name: Option<String>,
    pub middle_name: Option<String>,
    pub last_name: Option<String>,
    pub nickname: Option<String>,
    pub email: Vec<String>,
    pub id: Option<String>,
}

impl Author {
    /// Format as a single display name string.
    pub fn display_name(&self) -> String {
        let mut parts = Vec::new();
        if let Some(f) = &self.first_name {
            parts.push(f.as_str());
        }
        if let Some(m) = &self.middle_name {
            parts.push(m.as_str());
        }
        if let Some(l) = &self.last_name {
            parts.push(l.as_str());
        }
        if parts.is_empty()
            && let Some(n) = &self.nickname
        {
            return n.clone();
        }
        parts.join(" ")
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Sequence {
    pub name: String,
    pub number: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DocumentInfo {
    pub author: Vec<Author>,
    pub program_used: Option<String>,
    pub date: Option<String>,
    pub src_url: Vec<String>,
    pub src_ocr: Option<String>,
    pub id: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PublishInfo {
    pub book_name: Option<String>,
    pub publisher: Option<String>,
    pub city: Option<String>,
    pub year: Option<String>,
    pub isbn: Option<String>,
    pub sequence: Vec<Sequence>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CustomInfo {
    pub info_type: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Body {
    pub name: Option<String>,
    pub lang: Option<String>,
    pub epigraph: Vec<Epigraph>,
    pub title: Option<Title>,
    pub section: Vec<Section>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Section {
    pub id: Option<String>,
    pub lang: Option<String>,
    pub title: Option<Title>,
    pub epigraph: Vec<Epigraph>,
    pub image: Option<Image>,
    pub annotation: Option<Annotation>,
    pub content: Vec<SectionContent>,
    pub section: Vec<Section>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SectionContent {
    Para(Vec<InlineElement>),
    Image(Image),
    Poem(Poem),
    Subtitle(Vec<InlineElement>),
    Cite(Cite),
    EmptyLine,
    Table(Table),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Title {
    pub lang: Option<String>,
    pub para: Vec<TitlePara>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TitlePara {
    Para(Vec<InlineElement>),
    EmptyLine,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Epigraph {
    pub id: Option<String>,
    pub content: Vec<EpigraphContent>,
    pub text_author: Vec<Vec<InlineElement>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EpigraphContent {
    Para(Vec<InlineElement>),
    Poem(Poem),
    Cite(Cite),
    EmptyLine,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Annotation {
    pub id: Option<String>,
    pub lang: Option<String>,
    pub content: Vec<AnnotationContent>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AnnotationContent {
    Para(Vec<InlineElement>),
    Poem(Poem),
    Cite(Cite),
    Subtitle(Vec<InlineElement>),
    EmptyLine,
    Table(Table),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Poem {
    pub id: Option<String>,
    pub lang: Option<String>,
    pub title: Option<Title>,
    pub epigraph: Vec<Epigraph>,
    pub stanza: Vec<Stanza>,
    pub text_author: Vec<Vec<InlineElement>>,
    pub date: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Stanza {
    pub lang: Option<String>,
    pub title: Option<Title>,
    pub subtitle: Option<Vec<InlineElement>>,
    pub v: Vec<Vec<InlineElement>>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Cite {
    pub id: Option<String>,
    pub lang: Option<String>,
    pub content: Vec<CiteContent>,
    pub text_author: Vec<Vec<InlineElement>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CiteContent {
    Para(Vec<InlineElement>),
    Poem(Poem),
    EmptyLine,
    Table(Table),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Table {
    pub id: Option<String>,
    pub style: Option<String>,
    pub row: Vec<TableRow>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TableRow {
    pub align: Option<String>,
    pub cell: Vec<TableCell>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TableCell {
    pub id: Option<String>,
    pub style: Option<String>,
    pub colspan: Option<u32>,
    pub rowspan: Option<u32>,
    pub align: Option<String>,
    pub valign: Option<String>,
    pub is_header: bool,
    pub content: Vec<InlineElement>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Image {
    pub href: String,
    pub alt: Option<String>,
    pub title: Option<String>,
    pub id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InlineElement {
    Text(String),
    Strong(Vec<InlineElement>),
    Emphasis(Vec<InlineElement>),
    Strikethrough(Vec<InlineElement>),
    Sub(Vec<InlineElement>),
    Sup(Vec<InlineElement>),
    Code(String),
    Image(Image),
    Link {
        href: String,
        kind: Option<String>,
        children: Vec<InlineElement>,
    },
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Binary {
    pub id: String,
    pub content_type: String,
    pub data: Vec<u8>,
}

/// A diagnostic from parsing.
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Warning,
    Error,
}
