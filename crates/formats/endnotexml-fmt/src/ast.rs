//! AST types for EndNote XML bibliography documents.
//!
//! EndNote's export schema (`<xml><records><record>...</record></records></xml>`)
//! is large and exporter-dependent — EndNote desktop and EndNote Web both add
//! their own extension fields (`custom1`..`custom7`, `research-notes`,
//! `work-type`, ...) on top of a documented core vocabulary (ref-type,
//! contributors, titles, periodical, pages, dates, urls, ...). That core
//! vocabulary is worth modeling directly as a typed AST — [`Record`] with
//! named fields for `ref-type`/`contributors`/`titles`/... — the same
//! rationale `opml-fmt` uses for OPML's small fixed grammar, rather than a
//! fully generic element tree the way `docbook-fmt`/`jats-fmt`/`tei-fmt`
//! model formats with hundreds of document-specific elements.
//!
//! Losslessness for the genuinely open-ended part — any record-level element
//! this crate doesn't give a dedicated field, and any child of a known
//! container element outside that container's documented children — is
//! handled by [`Element`], a small self-contained "name + attrs + inline
//! content" capture, collected into an `extra: Vec<Element>` at every level
//! that has one. Nothing round-trips through a silent drop.
//!
//! EndNote field content (titles, abstracts, notes, author names, ...) can
//! carry `<style face="...">` runs (bold/italic/underline/superscript/
//! subscript) and, in practice, other incidental wrapper elements some
//! exporters emit. [`Inline`] models both: `Style` for the documented markup
//! vocabulary, `Other` for anything else nested in field content, preserving
//! its element name and attributes rather than flattening them away — a
//! strictly more general capture than the pre-existing rescribe adapter,
//! which flattened unrecognized nested elements into their text content
//! (see `rescribe-read-endnotexml`, which preserves that exact flattening
//! behavior when consuming `Inline::Other` for backward compatibility).

/// Byte offset span in the source input.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const NONE: Span = Span { start: 0, end: 0 };
}

/// The XML declaration (`<?xml version="1.0" encoding="UTF-8"?>`), if present.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct XmlDecl {
    pub version: String,
    pub encoding: Option<String>,
    pub standalone: Option<String>,
}

/// An EndNote XML document: `<xml><records>...</records></xml>`.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct EndNoteDoc {
    pub xml_decl: Option<XmlDecl>,
    pub records: Vec<Record>,
    pub span: Span,
}

impl EndNoteDoc {
    pub fn strip_spans(&self) -> EndNoteDoc {
        EndNoteDoc {
            xml_decl: self.xml_decl.clone(),
            records: self.records.iter().map(Record::strip_spans).collect(),
            span: Span::NONE,
        }
    }
}

/// One `<record>`.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Record {
    pub ref_type: RefType,
    /// `<rec-number>` — EndNote's internal record ID. Plain text per the
    /// schema (never observed carrying `<style>` markup).
    pub rec_number: Option<String>,
    /// `<label>` — often used by exporters as a human cite-key.
    pub label: Option<String>,
    pub foreign_keys: Option<ForeignKeys>,
    pub contributors: Option<Contributors>,
    pub titles: Option<Titles>,
    /// `<periodical>` — distinct from `titles/secondary-title`; some
    /// exporters populate both with the same journal name, others only one.
    pub periodical: Option<Periodical>,
    pub volume: Option<Vec<Inline>>,
    pub number: Option<Vec<Inline>>,
    pub pages: Option<Vec<Inline>>,
    pub publisher: Option<Vec<Inline>>,
    pub pub_location: Option<Vec<Inline>>,
    pub isbn: Option<String>,
    pub issn: Option<String>,
    pub electronic_resource_num: Option<String>,
    pub urls: Option<Urls>,
    /// A bare top-level `<url>` (outside `<urls>`) — a non-standard but
    /// observed placement, kept distinct from `urls.related_urls`.
    pub bare_url: Option<String>,
    pub abstract_: Option<Vec<Inline>>,
    pub notes: Option<Vec<Inline>>,
    /// `<keywords>/<keyword>`, in source order.
    pub keywords: Vec<Vec<Inline>>,
    pub dates: Option<Dates>,
    /// Any other `<record>` child this struct doesn't give a dedicated
    /// field — EndNote's schema is large and exporter-dependent
    /// (`custom1`..`custom7`, `research-notes`, `work-type`,
    /// `remote-database-name`, `language`, `section`, ...) — captured
    /// verbatim in source order rather than dropped.
    pub extra: Vec<Element>,
    pub span: Span,
}

impl Record {
    pub fn strip_spans(&self) -> Record {
        Record {
            ref_type: self.ref_type.clone(),
            rec_number: self.rec_number.clone(),
            label: self.label.clone(),
            foreign_keys: self.foreign_keys.clone(),
            contributors: self.contributors.clone(),
            titles: self.titles.clone(),
            periodical: self.periodical.clone(),
            volume: self.volume.clone(),
            number: self.number.clone(),
            pages: self.pages.clone(),
            publisher: self.publisher.clone(),
            pub_location: self.pub_location.clone(),
            isbn: self.isbn.clone(),
            issn: self.issn.clone(),
            electronic_resource_num: self.electronic_resource_num.clone(),
            urls: self.urls.clone(),
            bare_url: self.bare_url.clone(),
            abstract_: self.abstract_.clone(),
            notes: self.notes.clone(),
            keywords: self.keywords.clone(),
            dates: self.dates.clone(),
            extra: self.extra.clone(),
            span: Span::NONE,
        }
    }
}

/// `<ref-type>` — numeric code as text content, with an optional `name`
/// attribute (e.g. `<ref-type name="Journal Article">17</ref-type>`).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct RefType {
    pub code: String,
    pub name: Option<String>,
}

/// `<foreign-keys>`.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ForeignKeys {
    pub keys: Vec<ForeignKey>,
    pub extra: Vec<Element>,
}

/// One `<foreign-keys>/<key>`: an accession number plus `app`/`db-id`
/// attributes.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ForeignKey {
    pub app: Option<String>,
    pub db_id: Option<String>,
    pub text: String,
}

/// `<contributors>`: the four documented author-role lists, each a list of
/// per-author inline content (an author name field may itself carry
/// `<style>` runs, rare but valid).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Contributors {
    pub authors: Vec<Vec<Inline>>,
    /// `<secondary-authors>` — conventionally editors.
    pub secondary_authors: Vec<Vec<Inline>>,
    pub tertiary_authors: Vec<Vec<Inline>>,
    pub subsidiary_authors: Vec<Vec<Inline>>,
    /// Any other `<contributors>` child (e.g. `<translated-authors>`),
    /// captured verbatim rather than dropped.
    pub extra: Vec<Element>,
}

impl Contributors {
    pub fn is_empty(&self) -> bool {
        self.authors.is_empty()
            && self.secondary_authors.is_empty()
            && self.tertiary_authors.is_empty()
            && self.subsidiary_authors.is_empty()
            && self.extra.is_empty()
    }
}

/// `<titles>`.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Titles {
    pub title: Option<Vec<Inline>>,
    /// `<secondary-title>` — journal/container name for articles, book
    /// title for a book section.
    pub secondary_title: Option<Vec<Inline>>,
    /// `<tertiary-title>` — series title.
    pub tertiary_title: Option<Vec<Inline>>,
    /// Any other `<titles>` child (`short-title`, `translated-title`,
    /// `alt-title`, ...), captured verbatim.
    pub extra: Vec<Element>,
}

impl Titles {
    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.secondary_title.is_none()
            && self.tertiary_title.is_none()
            && self.extra.is_empty()
    }
}

/// `<periodical>`.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Periodical {
    pub full_title: Option<Vec<Inline>>,
    /// Any other `<periodical>` child (`abbr-1`, `abbr-2`, `abbr-3`, ...).
    pub extra: Vec<Element>,
}

impl Periodical {
    pub fn is_empty(&self) -> bool {
        self.full_title.is_none() && self.extra.is_empty()
    }
}

/// `<urls>`.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Urls {
    pub related_urls: Vec<String>,
    pub pdf_urls: Vec<String>,
    /// Any other `<urls>` child (`web-urls` as its own wrapper distinct from
    /// `related-urls`, etc.), captured verbatim.
    pub extra: Vec<Element>,
}

impl Urls {
    pub fn is_empty(&self) -> bool {
        self.related_urls.is_empty() && self.pdf_urls.is_empty() && self.extra.is_empty()
    }
}

/// `<dates>`.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Dates {
    pub year: Option<Vec<Inline>>,
    /// `<pub-dates>/<date>` — a free-text publication date (e.g. "Jan 15")
    /// with no unambiguous year/month/day parse without guessing a locale's
    /// month-name convention. Only the first `<date>` is modeled (matches
    /// the pre-existing rescribe adapter behavior); the schema permits
    /// exactly one in practice.
    pub pub_date: Option<Vec<Inline>>,
    /// Any other `<dates>` child (`access-date`, ...), captured verbatim.
    pub extra: Vec<Element>,
}

impl Dates {
    pub fn is_empty(&self) -> bool {
        self.year.is_none() && self.pub_date.is_none() && self.extra.is_empty()
    }
}

/// An element this crate doesn't give a dedicated typed field, captured
/// verbatim: tag name, attributes, and inline content (recursively —
/// `<style>` runs and further-nested elements inside an unknown field are
/// still modeled, not collapsed to a text blob).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Element {
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Inline>,
}

/// Inline content of a field that can carry EndNote `<style>` markup:
/// titles, abstract, notes, author names, keywords, pages, volume/number,
/// publisher fields, and any [`Element`]'s content.
#[derive(Clone, Debug, PartialEq)]
pub enum Inline {
    Text(String),
    /// `<style face="bold|italic|underline|superscript|subscript|normal|...">`.
    /// `face` is preserved verbatim (not validated against that list) —
    /// interpreting it into semantic markup is a consumer decision (see
    /// `rescribe-read-endnotexml`'s `face`-to-node-kind mapping).
    Style {
        face: String,
        children: Vec<Inline>,
    },
    /// An element nested in field content that is not `<style>` — outside
    /// EndNote's documented markup vocabulary but seen from some exporters.
    /// Preserved as its own node (name, attributes, children) rather than
    /// flattened.
    Other {
        name: String,
        attrs: Vec<(String, String)>,
        children: Vec<Inline>,
    },
}

/// Which `<contributors>` role list an author belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AuthorRole {
    Authors,
    /// `<secondary-authors>` — conventionally editors.
    SecondaryAuthors,
    TertiaryAuthors,
    SubsidiaryAuthors,
}

/// Which `<urls>` role list a URL belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UrlRole {
    RelatedUrls,
    PdfUrls,
}

/// Diagnostic message from parsing.
#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    pub message: String,
    pub span: Span,
}
