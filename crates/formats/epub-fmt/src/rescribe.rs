//! AST<->`rescribe::Document` translation for EPUB.
//!
//! This module only translates between [`crate::EpubDoc`] and rescribe's
//! `Document` IR — no ZIP/XML/XHTML parsing happens here (that all lives
//! in the rest of this crate and in `zip-fmt`/`html-fmt`; see
//! `crate::parse`/`crate::emit`). Enabled by the `rescribe` feature; each
//! direction is additionally gated on the reader/writer mode feature it
//! depends on, so enabling `rescribe` alone (with no mode feature)
//! compiles nothing. Content-document HTML<->IR translation is delegated
//! to `html-fmt`'s own `rescribe` module (round-tripped through
//! `html_fmt::HtmlDoc::emit`/`parse` at the boundary) rather than
//! reimplemented here, per this repo's "adapters translate, they don't
//! parse" rule.
//!
//! # Mapping
//!
//! **Metadata**: the nine cross-format-meaningful Dublin Core fields
//! (title, creator→author, language, publisher, description, date,
//! identifier, subject, rights — using each field's *first* value when
//! the OPF declares several) are projected onto rescribe's generic
//! metadata keys (`title`, `author`, ...). Separately, the *entire*
//! `<metadata>` element (every `dc:*`/`meta`/`link`/unknown child, with
//! every attribute) is raw-preserved verbatim as `epub:metadata_xml`, so
//! `parse` → `emit` reconstructs it exactly rather than re-deriving a
//! lossy subset from the nine flattened keys. `<guide>` is similarly
//! raw-preserved as `epub:guide_xml`. `epub:version`/
//! `epub:unique_identifier`/`epub:opf_dir` round-trip the package
//! document's own attributes and the manifest hrefs' resolution base.
//!
//! **Content**: each spine item's translated HTML content is flattened
//! directly into the document — no wrapper node — matching the previous
//! `rescribe-read-epub` adapter's shape (which existing fixtures assert
//! paths against). For multi-chapter EPUBs, a synthetic level-1 heading
//! whose text is the spine item's manifest id is prepended to each
//! chapter's content (skipped when that id is empty or looks
//! auto-generated, i.e. starts with `"item"`) — also matching the previous
//! adapter, which used this as its only per-chapter label without parsing
//! nav/NCX. The chapter/spine boundary is instead recorded as
//! `epub:manifest_id`/`epub:href`/`epub:spine_linear`/`epub:spine_id`/
//! `epub:spine_properties` properties on that chapter's *first* node (the
//! synthetic heading when present, otherwise the first real content node,
//! or a placeholder empty paragraph if the chapter had no content at all),
//! so the writer reconstructs the manifest/spine exactly — including
//! `linear="no"` items, which still appear in document order (unlike a
//! reading-order-only projection) so no content is dropped. A document
//! built from scratch (not round-tripping an EPUB) has none of these
//! props; the writer falls back to splitting on `h1` headings, mirroring
//! the previous `rescribe-write-epub` adapter's behavior.
//!
//! **Resources**: every non-content manifest item (images, fonts, CSS,
//! ...) becomes a `rescribe_core::Resource` keyed by a fresh `ResourceId`,
//! with `epub:href` recording its manifest-relative href for exact
//! reconstruction. `META-INF/encryption.xml` and any unclassified archive
//! entry are also preserved this way (`epub:kind = "encryption_descriptor"`
//! / `"unclassified"`), so nothing from the archive is silently dropped —
//! though see the gaps below.
//!
//! # Documented gaps
//!
//! - **Navigation is not modeled into the IR.** `nav.xhtml`/NCX structure
//!   (including any custom `landmarks`/`page-list`, and multi-level TOC
//!   nesting) is not preserved through `Document` — a
//!   [`FidelityWarning`] is emitted when present, and the writer always
//!   regenerates a minimal single-level EPUB3 nav from the spine's
//!   chapter titles. This is a real, acknowledged gap (matches the
//!   `[lib]`-tagged gaps in the previous adapter's `fixtures/epub/
//!   COVERAGE.md`, which could not access nav/NCX at all).
//! - **Image/resource references inside content are not relinked** to
//!   `ResourceId`s — an `<img src="...">` inside a chapter round-trips as
//!   a plain `image` node with the original relative URL (via
//!   `rescribe-read-html`/`rescribe-write-html`, unchanged from the
//!   previous adapter's behavior), not as a `prop::RESOURCE_ID` reference
//!   into `Document::resources`.
//! - **SMIL media overlays** and any manifest item this crate's own AST
//!   does not structurally model are preserved as opaque resource bytes
//!   (see `ast.rs`), not decoded.

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
mod read {
    use crate::EpubDoc;
    use crate::ast::{ENCRYPTION_PATH, RootFile};
    use crate::pathutil::dir_of;
    use rescribe_core::{
        ConversionResult, Document, FidelityWarning, Node, ParseError, Properties, Resource,
        ResourceId, ResourceMap, Severity, SourceInfo, WarningKind,
    };
    use rescribe_format_api::{Emit as _, Parse as _};
    use rescribe_std::{node, prop};

    /// Parse EPUB bytes into a document.
    pub fn parse(input: &[u8]) -> Result<ConversionResult<Document>, ParseError> {
        let (epub, diagnostics) = EpubDoc::parse(input);

        let mut warnings: Vec<FidelityWarning> = diagnostics
            .iter()
            .map(|d| {
                FidelityWarning::new(
                    Severity::Minor,
                    WarningKind::FeatureLost("epub".to_string()),
                    format!("EPUB parse note: {}", d.message),
                )
            })
            .collect();

        if epub.nav.is_some() || epub.ncx.is_some() {
            warnings.push(FidelityWarning::new(
                Severity::Minor,
                WarningKind::FeatureLost("epub-navigation".to_string()),
                "EPUB navigation document (nav.xhtml/NCX) structure is not preserved through \
                 rescribe's IR; a minimal single-level nav is regenerated from spine order on \
                 write.",
            ));
        }

        let opf_path = epub
            .container
            .rootfiles
            .first()
            .map(|r: &RootFile| r.full_path.clone())
            .unwrap_or_else(|| "OEBPS/content.opf".to_string());
        let base_dir = dir_of(&opf_path);

        let mut metadata = Properties::new();
        apply_metadata(&epub, &mut metadata);
        metadata.set("epub:opf_dir", base_dir.clone());

        let mut resources: ResourceMap = ResourceMap::new();
        for res in &epub.resources {
            let href = res
                .path
                .strip_prefix(&format!("{base_dir}/"))
                .unwrap_or(res.path.as_str())
                .to_string();
            let mut resmeta = Properties::new();
            resmeta.set("epub:href", href);
            let id = ResourceId::new();
            resources.insert(
                id,
                Resource {
                    name: Some(res.path.clone()),
                    mime_type: res.media_type.clone(),
                    data: res.content.clone(),
                    metadata: resmeta,
                },
            );
        }
        if let Some(enc) = &epub.encryption_xml {
            let mut resmeta = Properties::new();
            resmeta.set("epub:kind", "encryption_descriptor");
            resources.insert(
                ResourceId::new(),
                Resource {
                    name: Some(ENCRYPTION_PATH.to_string()),
                    mime_type: "application/xml".to_string(),
                    data: enc.clone(),
                    metadata: resmeta,
                },
            );
        }
        for (path, content) in &epub.unclassified {
            let mut resmeta = Properties::new();
            resmeta.set("epub:kind", "unclassified");
            resmeta.set("epub:href", path.clone());
            resources.insert(
                ResourceId::new(),
                Resource {
                    name: Some(path.clone()),
                    mime_type: "application/octet-stream".to_string(),
                    data: content.clone(),
                    metadata: resmeta,
                },
            );
        }

        let num_spine_items = epub.package.spine.items.len();
        let mut children = Vec::new();
        for item in &epub.package.spine.items {
            let Some(mi) = epub.package.manifest.iter().find(|m| m.id == item.idref) else {
                warnings.push(FidelityWarning::new(
                    Severity::Minor,
                    WarningKind::FeatureLost("epub-spine".to_string()),
                    format!(
                        "spine itemref '{}' has no matching manifest item",
                        item.idref
                    ),
                ));
                continue;
            };
            let path = crate::pathutil::resolve_href(&base_dir, &mi.href);
            let Some(cd) = epub.content_documents.iter().find(|c| c.path == path) else {
                warnings.push(FidelityWarning::new(
                    Severity::Minor,
                    WarningKind::FeatureLost("epub-spine".to_string()),
                    format!(
                        "spine item '{}' does not resolve to a content document",
                        mi.id
                    ),
                ));
                continue;
            };

            let html_bytes = cd.doc.emit();
            let html_str = String::from_utf8_lossy(&html_bytes);
            match html_fmt::rescribe::parse(&html_str) {
                Ok(result) => {
                    warnings.extend(result.warnings);
                    // Flatten this chapter's content directly into the
                    // document (no wrapper node) — matches the previous
                    // `rescribe-read-epub` adapter's shape, which existing
                    // fixtures assert paths against. The chapter/spine
                    // boundary is instead recorded as extra properties on
                    // this chapter's *first* node (a placeholder empty
                    // paragraph if the chapter had no content at all), so
                    // `crate::rescribe::emit` can reconstruct manifest/
                    // spine boundaries exactly without introducing a node
                    // no prior fixture expects. Fixture assertions only
                    // check props they name (see `rescribe-fixtures`'s
                    // "care about" philosophy), so these extra `epub:*`
                    // props are invisible to assertions that don't ask for
                    // them.
                    let mut chapter_children = result.value.content.children;
                    // For multi-chapter EPUBs, prepend a synthetic level-1
                    // heading using the spine item's manifest id as a
                    // chapter-title stand-in — matches the previous
                    // `rescribe-read-epub` adapter's behavior (its only
                    // source of a per-chapter label without accessing
                    // nav/NCX). Skipped for ids that look
                    // auto-generated/meaningless ("item1", "item2", ...) or
                    // empty, same heuristic as before.
                    if num_spine_items > 1 && !mi.id.is_empty() && !mi.id.starts_with("item") {
                        let heading = Node::new(node::HEADING)
                            .prop(prop::LEVEL, 1i64)
                            .child(Node::new(node::TEXT).prop(prop::CONTENT, mi.id.clone()));
                        chapter_children.insert(0, heading);
                    }
                    if chapter_children.is_empty() {
                        chapter_children.push(Node::new(node::PARAGRAPH));
                    }
                    let mut first = chapter_children.remove(0);
                    first = first
                        .prop("epub:manifest_id", mi.id.clone())
                        .prop("epub:href", mi.href.clone())
                        .prop("epub:spine_linear", item.linear);
                    if let Some(id) = &item.id {
                        first = first.prop("epub:spine_id", id.clone());
                    }
                    if !item.properties.is_empty() {
                        first = first.prop("epub:spine_properties", item.properties.join(" "));
                    }
                    children.push(first);
                    children.extend(chapter_children);
                }
                Err(e) => warnings.push(FidelityWarning::new(
                    Severity::Minor,
                    WarningKind::FeatureLost("epub-content".to_string()),
                    format!("failed to translate content document '{}': {e}", cd.path),
                )),
            }
        }

        let document = Document {
            content: Node::new(node::DOCUMENT).children(children),
            resources,
            metadata,
            source: Some(SourceInfo {
                format: "epub".to_string(),
                metadata: Properties::new(),
            }),
        };

        Ok(ConversionResult::with_warnings(document, warnings))
    }

    fn apply_metadata(epub: &EpubDoc, metadata: &mut Properties) {
        let m = &epub.package.metadata;
        macro_rules! first {
            ($list:expr, $key:literal) => {
                if let Some(v) = $list.first() {
                    metadata.set($key, v.value.clone());
                }
            };
        }
        first!(m.titles, "title");
        first!(m.creators, "author");
        first!(m.languages, "language");
        first!(m.publishers, "publisher");
        first!(m.descriptions, "description");
        first!(m.dates, "date");
        first!(m.identifiers, "identifier");
        first!(m.rights, "rights");
        if !m.subjects.is_empty() {
            let joined = m
                .subjects
                .iter()
                .map(|s| s.value.as_str())
                .collect::<Vec<_>>()
                .join("; ");
            metadata.set("subject", joined);
        }

        metadata.set("epub:version", epub.package.version.clone());
        metadata.set(
            "epub:unique_identifier",
            epub.package.unique_identifier.clone(),
        );
        metadata.set("epub:metadata_xml", crate::opf::metadata_inner_xml(m));
        if !epub.package.guide.is_empty() {
            metadata.set(
                "epub:guide_xml",
                crate::opf::guide_inner_xml(&epub.package.guide),
            );
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn parses_metadata_and_spine() {
            let bytes = crate::testutil::sample_epub();
            let result = parse(&bytes).unwrap();
            let doc = result.value;
            assert_eq!(doc.metadata.get_str("title"), Some("Sample Book"));
            assert_eq!(doc.metadata.get_str("author"), Some("Sample Author"));
            assert_eq!(doc.metadata.get_str("language"), Some("en"));
            // chapter1.xhtml is `<h1>Chapter 1</h1><p>Hello.</p>`; chapter2
            // similarly — each chapter contributes 2 top-level nodes from its
            // own XHTML, plus a synthetic level-1 heading (spine id "ch1"/
            // "ch2") prepended since this is a multi-chapter book — 3 nodes
            // per chapter, 6 top-level document children total.
            assert_eq!(doc.content.children.len(), 6);
            assert_eq!(doc.content.children[0].kind.as_str(), node::HEADING);
            assert_eq!(
                doc.content.children[0].props.get_str("epub:manifest_id"),
                Some("ch1")
            );
            assert_eq!(doc.resources.len(), 1);
        }
    }
}

#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
mod write {
    use crate::ast::{
        Container, ContentDocument, EpubDoc as EpubAst, GuideRef, ManifestItem, Metadata, NavList,
        NavPoint, Navigation, Package, ResourceEntry, RootFile, Span, Spine, SpineItemRef,
    };
    use rescribe_core::{
        ConversionResult, Document, EmitError, FidelityWarning, Node, Severity, WarningKind,
    };
    use rescribe_format_api::Parse as _;
    use rescribe_std::{node, prop};

    const DEFAULT_OPF_DIR: &str = "OEBPS";

    /// Emit a document as an EPUB.
    pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        let mut warnings = Vec::new();
        let base_dir = doc
            .metadata
            .get_str("epub:opf_dir")
            .map(str::to_string)
            .unwrap_or_else(|| DEFAULT_OPF_DIR.to_string());

        let metadata = build_metadata(doc);
        let guide: Vec<GuideRef> = doc
            .metadata
            .get_str("epub:guide_xml")
            .and_then(|s| crate::opf::parse_guide_xml(s).ok())
            .unwrap_or_default();

        let (mut manifest, spine, content_documents, nav_points) =
            build_content(doc, &mut warnings);
        let (resources, encryption_xml, unclassified) =
            build_resources(doc, &base_dir, &mut manifest);

        let nav_path = format!("{base_dir}/nav.xhtml");
        manifest.push(ManifestItem {
            id: "nav".to_string(),
            href: "nav.xhtml".to_string(),
            media_type: "application/xhtml+xml".to_string(),
            properties: vec!["nav".to_string()],
            ..Default::default()
        });
        let nav = build_nav(&nav_path, &nav_points);

        let package = Package {
            version: doc
                .metadata
                .get_str("epub:version")
                .map(str::to_string)
                .unwrap_or_else(|| "3.0".to_string()),
            unique_identifier: doc
                .metadata
                .get_str("epub:unique_identifier")
                .map(str::to_string)
                .unwrap_or_else(|| "pub-id".to_string()),
            metadata,
            manifest,
            spine,
            guide,
            ..Default::default()
        };

        let epub = EpubAst {
            container: Container {
                rootfiles: vec![RootFile {
                    full_path: format!("{base_dir}/content.opf"),
                    media_type: "application/oebps-package+xml".to_string(),
                }],
                span: Span::NONE,
            },
            package,
            nav: Some(nav),
            ncx: None,
            content_documents,
            resources,
            encryption_xml,
            unclassified,
            span: Span::NONE,
        };

        Ok(ConversionResult::with_warnings(
            crate::emit::emit(&epub),
            warnings,
        ))
    }

    fn build_metadata(doc: &Document) -> Metadata {
        if let Some(xml) = doc.metadata.get_str("epub:metadata_xml")
            && let Ok(m) = crate::opf::parse_metadata_xml(xml)
        {
            return m;
        }
        let mut m = Metadata::default();
        macro_rules! push {
            ($field:expr, $key:literal) => {
                if let Some(v) = doc.metadata.get_str($key) {
                    $field.push(crate::ast::DcElement {
                        value: v.to_string(),
                        attrs: Vec::new(),
                    });
                }
            };
        }
        push!(m.titles, "title");
        push!(m.creators, "author");
        push!(m.languages, "language");
        push!(m.publishers, "publisher");
        push!(m.descriptions, "description");
        push!(m.dates, "date");
        push!(m.identifiers, "identifier");
        push!(m.rights, "rights");
        push!(m.subjects, "subject");
        if m.identifiers.is_empty() {
            m.identifiers.push(crate::ast::DcElement {
                value: "urn:uuid:generated".to_string(),
                attrs: vec![("id".to_string(), "pub-id".to_string())],
            });
        }
        if m.languages.is_empty() {
            m.languages.push(crate::ast::DcElement {
                value: "en".to_string(),
                attrs: Vec::new(),
            });
        }
        m
    }

    fn is_chapter_boundary(n: &Node) -> bool {
        n.props.contains("epub:manifest_id")
    }

    /// Group a flat list of top-level nodes into chapters, where each
    /// chapter starts at a node tagged `epub:manifest_id` (by
    /// `crate::rescribe::parse`) and runs until the next tagged node or
    /// the end of the list. Mirrors how `parse` flattens each chapter's
    /// content directly into the document, tagging only that chapter's
    /// first node — see this module's doc comment.
    fn group_by_chapter_boundary(children: &[Node]) -> Vec<(&Node, &[Node])> {
        let mut chapters = Vec::new();
        let mut i = 0;
        while i < children.len() {
            if is_chapter_boundary(&children[i]) {
                let start = i;
                i += 1;
                while i < children.len() && !is_chapter_boundary(&children[i]) {
                    i += 1;
                }
                chapters.push((&children[start], &children[start + 1..i]));
            } else {
                i += 1;
            }
        }
        chapters
    }

    #[allow(clippy::type_complexity)]
    fn build_content(
        doc: &Document,
        warnings: &mut Vec<FidelityWarning>,
    ) -> (
        Vec<ManifestItem>,
        Spine,
        Vec<ContentDocument>,
        Vec<NavPoint>,
    ) {
        let mut manifest = Vec::new();
        let mut spine_items = Vec::new();
        let mut content_documents = Vec::new();
        let mut nav_points = Vec::new();

        if doc.content.children.iter().any(is_chapter_boundary) {
            let leading_untagged = doc
                .content
                .children
                .iter()
                .take_while(|n| !is_chapter_boundary(n))
                .count();
            if leading_untagged > 0 {
                warnings.push(FidelityWarning::new(
                    Severity::Minor,
                    WarningKind::Simplified("epub-spine".to_string()),
                    format!(
                        "{leading_untagged} top-level node(s) before the first epub chapter \
                         boundary were dropped"
                    ),
                ));
            }
            for (i, (head, rest)) in group_by_chapter_boundary(&doc.content.children)
                .into_iter()
                .enumerate()
            {
                let id = head
                    .props
                    .get_str("epub:manifest_id")
                    .unwrap_or("chapter")
                    .to_string();
                let href = head
                    .props
                    .get_str("epub:href")
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("chapter{}.xhtml", i + 1));
                let linear = head.props.get_bool("epub:spine_linear").unwrap_or(true);
                let spine_id = head.props.get_str("epub:spine_id").map(str::to_string);
                let properties = head
                    .props
                    .get_str("epub:spine_properties")
                    .map(|s| s.split_whitespace().map(str::to_string).collect())
                    .unwrap_or_default();

                let mut chapter_nodes = Vec::with_capacity(rest.len() + 1);
                chapter_nodes.push(head.clone());
                chapter_nodes.extend(rest.iter().cloned());
                let title = chapter_nodes
                    .iter()
                    .find_map(|n| {
                        if n.kind.as_str() == node::HEADING {
                            Some(extract_text(n))
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| format!("Chapter {}", i + 1));
                let html_doc = build_html(&chapter_nodes, warnings);
                manifest.push(ManifestItem {
                    id: id.clone(),
                    href: href.clone(),
                    media_type: "application/xhtml+xml".to_string(),
                    ..Default::default()
                });
                spine_items.push(SpineItemRef {
                    idref: id,
                    linear,
                    id: spine_id,
                    properties,
                    ..Default::default()
                });
                nav_points.push(NavPoint {
                    label: title,
                    href: Some(href.clone()),
                    children: Vec::new(),
                });
                content_documents.push(ContentDocument {
                    path: format!("{}/{href}", opf_dir(doc)),
                    media_type: "application/xhtml+xml".to_string(),
                    doc: html_doc,
                });
            }
        } else {
            for (i, chapter) in split_into_chapters(&doc.content).iter().enumerate() {
                let id = format!("chapter{}", i + 1);
                let href = format!("chapter{}.xhtml", i + 1);
                let title = chapter
                    .title
                    .clone()
                    .unwrap_or_else(|| format!("Chapter {}", i + 1));
                let html_doc = build_html(&chapter.nodes, warnings);
                manifest.push(ManifestItem {
                    id: id.clone(),
                    href: href.clone(),
                    media_type: "application/xhtml+xml".to_string(),
                    ..Default::default()
                });
                spine_items.push(SpineItemRef {
                    idref: id,
                    ..Default::default()
                });
                nav_points.push(NavPoint {
                    label: title,
                    href: Some(href.clone()),
                    children: Vec::new(),
                });
                content_documents.push(ContentDocument {
                    path: format!("{}/{href}", opf_dir(doc)),
                    media_type: "application/xhtml+xml".to_string(),
                    doc: html_doc,
                });
            }
        }

        (
            manifest,
            Spine {
                items: spine_items,
                ..Default::default()
            },
            content_documents,
            nav_points,
        )
    }

    fn opf_dir(doc: &Document) -> String {
        doc.metadata
            .get_str("epub:opf_dir")
            .map(str::to_string)
            .unwrap_or_else(|| DEFAULT_OPF_DIR.to_string())
    }

    fn build_html(children: &[Node], warnings: &mut Vec<FidelityWarning>) -> html_fmt::HtmlDoc {
        let temp_doc =
            Document::new().with_content(Node::new(node::DOCUMENT).children(children.to_vec()));
        match html_fmt::rescribe::emit_full_document(&temp_doc) {
            Ok(result) => {
                warnings.extend(result.warnings);
                let (html_doc, _diags) = html_fmt::HtmlDoc::parse(&result.value);
                html_doc
            }
            Err(e) => {
                warnings.push(FidelityWarning::new(
                    Severity::Major,
                    WarningKind::FeatureLost("epub-content".to_string()),
                    format!("failed to render chapter content to HTML: {e}"),
                ));
                html_fmt::HtmlDoc { nodes: Vec::new() }
            }
        }
    }

    struct Chapter {
        title: Option<String>,
        nodes: Vec<Node>,
    }

    fn split_into_chapters(root: &Node) -> Vec<Chapter> {
        let mut chapters = Vec::new();
        let mut current_nodes = Vec::new();
        let mut current_title: Option<String> = None;

        for n in &root.children {
            if n.kind.as_str() == node::HEADING {
                let level = n.props.get_int(prop::LEVEL).unwrap_or(1);
                if level == 1 {
                    if !current_nodes.is_empty() || current_title.is_some() {
                        chapters.push(Chapter {
                            title: current_title.take(),
                            nodes: std::mem::take(&mut current_nodes),
                        });
                    }
                    current_title = Some(extract_text(n));
                    continue;
                }
            }
            current_nodes.push(n.clone());
        }
        if !current_nodes.is_empty() || current_title.is_some() {
            chapters.push(Chapter {
                title: current_title,
                nodes: current_nodes,
            });
        }
        if chapters.is_empty() {
            chapters.push(Chapter {
                title: None,
                nodes: root.children.clone(),
            });
        }
        chapters
    }

    fn extract_text(node: &Node) -> String {
        let mut out = String::new();
        extract_text_recursive(node, &mut out);
        out
    }

    fn extract_text_recursive(node: &Node, out: &mut String) {
        if node.kind.as_str() == node::TEXT
            && let Some(content) = node.props.get_str(prop::CONTENT)
        {
            out.push_str(content);
        }
        for child in &node.children {
            extract_text_recursive(child, out);
        }
    }

    #[allow(clippy::type_complexity)]
    fn build_resources(
        doc: &Document,
        base_dir: &str,
        manifest: &mut Vec<ManifestItem>,
    ) -> (Vec<ResourceEntry>, Option<Vec<u8>>, Vec<(String, Vec<u8>)>) {
        let mut resources = Vec::new();
        let mut encryption_xml = None;
        let mut unclassified = Vec::new();

        for (i, res) in doc.resources.values().enumerate() {
            match res.metadata.get_str("epub:kind") {
                Some("encryption_descriptor") => encryption_xml = Some(res.data.clone()),
                Some("unclassified") => {
                    let path = res
                        .metadata
                        .get_str("epub:href")
                        .map(str::to_string)
                        .unwrap_or_else(|| format!("unclassified/res{i}"));
                    unclassified.push((path, res.data.clone()));
                }
                _ => {
                    let href = res
                        .metadata
                        .get_str("epub:href")
                        .map(str::to_string)
                        .unwrap_or_else(|| format!("resources/res{i}"));
                    let path = crate::pathutil::resolve_href(base_dir, &href);
                    let id = format!("res{i}");
                    manifest.push(ManifestItem {
                        id,
                        href,
                        media_type: res.mime_type.clone(),
                        ..Default::default()
                    });
                    resources.push(ResourceEntry {
                        path,
                        media_type: res.mime_type.clone(),
                        content: res.data.clone(),
                    });
                }
            }
        }
        (resources, encryption_xml, unclassified)
    }

    fn build_nav(path: &str, points: &[NavPoint]) -> Navigation {
        let items_html: String = points
            .iter()
            .map(|p| {
                format!(
                    r#"<li><a href="{}">{}</a></li>"#,
                    p.href.as_deref().unwrap_or(""),
                    escape(&p.label)
                )
            })
            .collect();
        let html = format!(
            r#"<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
<head><title>Table of Contents</title></head>
<body>
<nav epub:type="toc"><h1>Table of Contents</h1><ol>{items_html}</ol></nav>
</body></html>"#
        );
        let (doc, _diags) = html_fmt::HtmlDoc::parse(html.as_bytes());
        Navigation {
            path: path.to_string(),
            toc: Some(NavList {
                heading: Some("Table of Contents".to_string()),
                items: points.to_vec(),
            }),
            page_list: None,
            landmarks: None,
            other: Vec::new(),
            doc,
        }
    }

    fn escape(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use rescribe_std::builder::doc;

        #[test]
        fn emits_simple_epub() {
            let document = doc(|d| {
                d.heading(1, |i| i.text("Chapter 1"))
                    .para(|i| i.text("Hello, world!"))
            });
            let result = emit(&document).unwrap();
            assert!(result.value.starts_with(b"PK"));
            let (epub, diags) = crate::EpubDoc::parse(&result.value);
            assert!(diags.is_empty(), "diagnostics: {diags:?}");
            assert_eq!(epub.content_documents.len(), 1);
            assert!(epub.nav.is_some());
        }

        #[test]
        #[cfg(feature = "reader-ast")]
        fn roundtrips_through_rescribe() {
            let bytes = crate::testutil::sample_epub();
            let parsed = super::super::read::parse(&bytes).unwrap();
            let emitted = emit(&parsed.value).unwrap();
            let (epub, diags) = crate::EpubDoc::parse(&emitted.value);
            assert!(diags.is_empty(), "diagnostics: {diags:?}");
            assert_eq!(epub.package.metadata.titles[0].value, "Sample Book");
            assert_eq!(epub.content_documents.len(), 2);
        }
    }
}

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
pub use read::parse;
#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
pub use write::emit;
