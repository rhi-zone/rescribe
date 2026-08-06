//! Domain-typed AST for EPUB (2.0.1 and 3.x) publications.
//!
//! An EPUB is a ZIP container (the OCF — Open Container Format) holding:
//! `META-INF/container.xml` (points at the root package document),
//! one OPF package document (`<metadata>`/`<manifest>`/`<spine>`, plus
//! EPUB2's `<guide>`), a navigation document (EPUB3 `nav.xhtml`, and/or
//! an EPUB2 NCX `.ncx` file), XHTML content documents, and arbitrary
//! embedded resources (images, fonts, CSS, audio/video, SMIL overlays).
//!
//! This crate's AST mirrors that structure directly — it is not a
//! reuse of `zip-fmt`'s flat `Archive`/`Entry` shape (a ZIP archive has
//! no notion of "this entry is the package document"; that classification
//! is EPUB-specific, driven by `container.xml` + the OPF manifest) nor of
//! `html-fmt`'s single-document shape (an EPUB embeds many XHTML content
//! documents, each identified by its manifest path). `EpubDoc` composes
//! both: `zip-fmt` supplies the container layer, `html-fmt`'s `HtmlDoc` is
//! reused verbatim as the type for every XHTML content document and for
//! the EPUB3 navigation document (both are XHTML) — reimplementing XHTML
//! parsing here would violate the "no parsing logic outside the owning
//! `-fmt` crate" rule.
//!
//! # Losslessness
//!
//! Every OPF/NCX/container attribute this crate does not specifically
//! model is captured verbatim in an `extra_attrs: Vec<(String, String)>`
//! field (this repo's established convention — see `opml-fmt::Outline`),
//! and every unrecognized child element is captured as [`RawXml`] (its
//! tag name plus the exact inner-XML text) rather than being dropped.
//! `parse(emit(ast)) == ast` is the correctness bar (see `CLAUDE.md`'s
//! roundtrip-direction rule and this crate's fuzz targets).

pub use rescribe_format_api::{Diagnostic, Severity, Span};

/// The archive path of the mandatory first, uncompressed `mimetype` entry.
pub const MIMETYPE_ENTRY: &str = "mimetype";
/// The mandatory `application/epub+zip` mimetype content.
pub const MIMETYPE_CONTENT: &str = "application/epub+zip";
/// The fixed archive path of the OCF container descriptor.
pub const CONTAINER_PATH: &str = "META-INF/container.xml";
/// The fixed archive path of the (optional) encryption descriptor. This
/// crate only detects its presence (see [`EpubDoc::encrypted`]) — it never
/// attempts to decrypt content, per the task scope.
pub const ENCRYPTION_PATH: &str = "META-INF/encryption.xml";

/// A raw, unparsed XML element — this crate's raw-preservation vehicle for
/// OPF/NCX/nav constructs it does not specifically model (future/vendor
/// elements, `<collection>`, `<bindings>`, `<tours>`, ...). `raw` is the
/// element's inner XML (children, serialized), not including the element's
/// own start/end tags — those are reconstructed from `name`/`attrs` on
/// emit.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct RawXml {
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub raw_inner: String,
}

/// A complete parsed EPUB publication.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct EpubDoc {
    pub container: Container,
    pub package: Package,
    /// EPUB3 navigation document (`nav.xhtml`), if the manifest declares
    /// an item with `properties="nav"`.
    pub nav: Option<Navigation>,
    /// EPUB2 NCX document, if the manifest/spine reference one.
    pub ncx: Option<Ncx>,
    /// Every manifest item classified as an XHTML content document
    /// (media-type `application/xhtml+xml`), excluding the nav document
    /// (which is stored on [`EpubDoc::nav`] instead, though it is also a
    /// manifest item of the same media type).
    pub content_documents: Vec<ContentDocument>,
    /// Every other manifest item (images, fonts, CSS, audio/video, SMIL
    /// media overlays, ...), raw bytes plus declared media type. SMIL
    /// overlays and other structured-but-unmodeled formats are preserved
    /// here as raw bytes, not decoded into a domain structure — a
    /// documented gap, not a silent drop.
    pub resources: Vec<ResourceEntry>,
    /// `META-INF/encryption.xml`'s raw bytes, if present. This crate only
    /// detects/preserves it — it never parses or decrypts. Resources it
    /// declares encrypted still appear in [`EpubDoc::resources`] as
    /// opaque (still-encrypted) bytes.
    pub encryption_xml: Option<Vec<u8>>,
    /// Archive entries not classified as any of the above (e.g. the
    /// `mimetype` entry itself is not re-listed here — it is implied and
    /// always re-emitted — but a stray file with no manifest entry, or
    /// `META-INF/encryption.xml`'s own bytes, land here verbatim).
    pub unclassified: Vec<(String, Vec<u8>)>,
    pub span: Span,
}

impl EpubDoc {
    pub fn encrypted(&self) -> bool {
        self.encryption_xml.is_some()
    }

    pub fn strip_spans(&self) -> EpubDoc {
        EpubDoc {
            container: self.container.strip_spans(),
            package: self.package.strip_spans(),
            nav: self.nav.as_ref().map(Navigation::strip_spans),
            ncx: self.ncx.clone(),
            content_documents: self
                .content_documents
                .iter()
                .map(ContentDocument::strip_spans)
                .collect(),
            resources: self.resources.clone(),
            encryption_xml: self.encryption_xml.clone(),
            unclassified: self.unclassified.clone(),
            span: Span::NONE,
        }
    }
}

// ── META-INF/container.xml ──────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Container {
    pub rootfiles: Vec<RootFile>,
    pub span: Span,
}

impl Container {
    pub fn strip_spans(&self) -> Container {
        Container {
            rootfiles: self.rootfiles.clone(),
            span: Span::NONE,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct RootFile {
    pub full_path: String,
    pub media_type: String,
}

// ── OPF package document ────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Package {
    pub version: String,
    pub unique_identifier: String,
    pub xml_lang: Option<String>,
    pub dir: Option<String>,
    pub id: Option<String>,
    pub metadata: Metadata,
    pub manifest: Vec<ManifestItem>,
    pub spine: Spine,
    /// EPUB2 `<guide>` (superseded by EPUB3 nav landmarks, but still
    /// widely emitted for backwards compatibility).
    pub guide: Vec<GuideRef>,
    /// Top-level `<package>` children this crate does not specially model
    /// (`<bindings>`, `<collection>`, `<tours>`), raw-preserved.
    pub extra_elements: Vec<RawXml>,
    pub extra_attrs: Vec<(String, String)>,
    pub span: Span,
}

impl Package {
    pub fn strip_spans(&self) -> Package {
        Package {
            span: Span::NONE,
            ..self.clone()
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Metadata {
    pub identifiers: Vec<DcElement>,
    pub titles: Vec<DcElement>,
    pub languages: Vec<DcElement>,
    pub creators: Vec<DcElement>,
    pub contributors: Vec<DcElement>,
    pub subjects: Vec<DcElement>,
    pub descriptions: Vec<DcElement>,
    pub publishers: Vec<DcElement>,
    pub dates: Vec<DcElement>,
    pub types: Vec<DcElement>,
    pub formats: Vec<DcElement>,
    pub sources: Vec<DcElement>,
    pub relations: Vec<DcElement>,
    pub coverages: Vec<DcElement>,
    pub rights: Vec<DcElement>,
    /// EPUB3 `<meta property="..." refines="...">value</meta>` and legacy
    /// EPUB2 `<meta name="..." content="..."/>` elements alike (the two
    /// forms are distinguished by which of `property`/`name` is set).
    pub metas: Vec<MetaElement>,
    pub links: Vec<LinkElement>,
    /// Unrecognized `<metadata>` children — any Dublin Core element this
    /// crate doesn't have a dedicated field for, plus any non-DC,
    /// non-`meta`, non-`link` element.
    pub extra_elements: Vec<RawXml>,
}

/// One Dublin Core metadata element (`<dc:title>`, `<dc:creator>`, ...).
/// The element's text content plus every attribute it carried — including
/// `id`, `xml:lang`, and the OPF-namespace refinement attributes
/// (`opf:role`, `opf:file-as`, `opf:event`, `opf:scheme`) used pre-EPUB3 —
/// stored verbatim in `attrs` rather than decoded into dedicated fields,
/// so an attribute this crate has no special knowledge of still
/// round-trips.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct DcElement {
    pub value: String,
    pub attrs: Vec<(String, String)>,
}

impl DcElement {
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}

/// A `<meta>` element in either its EPUB3 (`property`/`refines`/`scheme`
/// + text content) or legacy EPUB2 (`name`/`content` attributes) form.
///
/// This crate does not interpret specific `property` values (for example
/// `rendition:layout` for EPUB3 fixed-layout metadata, or
/// `belongs-to-collection` for series metadata); they are modeled
/// generically here and pass through unchanged, which is sufficient for
/// lossless roundtrip but does not give callers a dedicated
/// fixed-layout/series API.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct MetaElement {
    pub property: Option<String>,
    pub refines: Option<String>,
    pub scheme: Option<String>,
    pub id: Option<String>,
    /// Element text content (EPUB3 form).
    pub value: String,
    /// Legacy EPUB2 `name` attribute.
    pub name: Option<String>,
    /// Legacy EPUB2 `content` attribute.
    pub content: Option<String>,
    pub extra_attrs: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct LinkElement {
    pub href: String,
    pub rel: Option<String>,
    pub media_type: Option<String>,
    pub properties: Option<String>,
    pub refines: Option<String>,
    pub id: Option<String>,
    pub extra_attrs: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ManifestItem {
    pub id: String,
    pub href: String,
    pub media_type: String,
    /// Space-separated `properties` attribute, split (e.g. `nav`,
    /// `cover-image`, `mathml`, `scripted`, `svg`, `remote-resources`,
    /// `switch`).
    pub properties: Vec<String>,
    pub fallback: Option<String>,
    pub media_overlay: Option<String>,
    pub extra_attrs: Vec<(String, String)>,
}

impl ManifestItem {
    pub fn has_property(&self, p: &str) -> bool {
        self.properties.iter().any(|x| x == p)
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Spine {
    pub id: Option<String>,
    /// Manifest id-ref of the EPUB2 NCX document. EPUB3 readers ignore
    /// this in favor of the nav document but authoring tools still emit
    /// it for EPUB2 compatibility.
    pub toc: Option<String>,
    pub page_progression_direction: Option<String>,
    pub items: Vec<SpineItemRef>,
    pub extra_attrs: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpineItemRef {
    pub idref: String,
    /// `linear="no"` marks an item excluded from the primary reading
    /// order (e.g. a pop-up footnote document). Defaults to `true`.
    pub linear: bool,
    pub id: Option<String>,
    pub properties: Vec<String>,
    pub extra_attrs: Vec<(String, String)>,
}

impl Default for SpineItemRef {
    fn default() -> Self {
        SpineItemRef {
            idref: String::new(),
            linear: true,
            id: None,
            properties: Vec::new(),
            extra_attrs: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct GuideRef {
    pub type_: String,
    pub title: Option<String>,
    pub href: String,
}

// ── Navigation (EPUB3 nav.xhtml) ────────────────────────────────────────

/// The EPUB3 navigation document. `doc` is the full parsed XHTML content
/// document (via `html-fmt`) — the source of truth for `emit()`. `toc`/
/// `page_list`/`landmarks`/`other` are a convenience projection extracted
/// by walking `doc` for `<nav epub:type="...">` elements, so a caller that
/// only wants the table of contents doesn't have to walk raw HTML nodes
/// itself.
#[derive(Clone, Debug, PartialEq)]
pub struct Navigation {
    pub path: String,
    pub toc: Option<NavList>,
    pub page_list: Option<NavList>,
    pub landmarks: Option<NavList>,
    /// Any other `<nav epub:type="...">` list this crate doesn't have a
    /// dedicated field for, keyed by its `epub:type` value (or `""` if
    /// absent).
    pub other: Vec<(String, NavList)>,
    pub doc: html_fmt::HtmlDoc,
}

impl Navigation {
    pub fn strip_spans(&self) -> Navigation {
        Navigation {
            doc: self.doc.strip_spans(),
            ..self.clone()
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct NavList {
    /// The `<h1>`..`<h6>` (or `<span>`) heading text inside the `<nav>`,
    /// if present.
    pub heading: Option<String>,
    pub items: Vec<NavPoint>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct NavPoint {
    pub label: String,
    pub href: Option<String>,
    pub children: Vec<NavPoint>,
}

// ── NCX (EPUB2 navigation) ──────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Ncx {
    pub path: String,
    /// `<meta name="dtb:uid" content="..."/>`.
    pub uid: Option<String>,
    /// Every `<head><meta name="..." content="..."/></head>` entry,
    /// verbatim (`dtb:uid`, `dtb:depth`, `dtb:totalPageCount`,
    /// `dtb:maxPageNumber`, and any future name), so none are lost even
    /// though only `uid` gets a dedicated field.
    pub head_metas: Vec<(String, String)>,
    pub doc_title: Option<String>,
    pub doc_authors: Vec<String>,
    pub nav_map: Vec<NavPoint>,
    pub page_list: Option<NavList>,
    pub nav_lists: Vec<NavList>,
}

// ── Content documents and resources ─────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
pub struct ContentDocument {
    pub path: String,
    pub media_type: String,
    pub doc: html_fmt::HtmlDoc,
}

impl ContentDocument {
    pub fn strip_spans(&self) -> ContentDocument {
        ContentDocument {
            path: self.path.clone(),
            media_type: self.media_type.clone(),
            doc: self.doc.strip_spans(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ResourceEntry {
    pub path: String,
    pub media_type: String,
    pub content: Vec<u8>,
}
