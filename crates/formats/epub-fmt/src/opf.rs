//! OPF package document (`content.opf`) <-> [`Package`] translation.

use crate::ast::{
    DcElement, GuideRef, LinkElement, ManifestItem, MetaElement, Metadata, Package, RawXml, Span,
    Spine, SpineItemRef,
};
use crate::xml::{XmlElement, XmlNode, emit_xml, parse_xml};

/// A Dublin Core element name paired with the field on [`Metadata`] it
/// populates. `dc:*` elements can repeat (multiple `dc:creator`s, ...), so
/// each name maps to a `Vec<DcElement>` accessor rather than a single
/// field.
type DcFieldGetter = fn(&mut Metadata) -> &mut Vec<DcElement>;

const DC_NAMES: &[(&str, DcFieldGetter)] = &[
    ("dc:identifier", |m| &mut m.identifiers),
    ("dc:title", |m| &mut m.titles),
    ("dc:language", |m| &mut m.languages),
    ("dc:creator", |m| &mut m.creators),
    ("dc:contributor", |m| &mut m.contributors),
    ("dc:subject", |m| &mut m.subjects),
    ("dc:description", |m| &mut m.descriptions),
    ("dc:publisher", |m| &mut m.publishers),
    ("dc:date", |m| &mut m.dates),
    ("dc:type", |m| &mut m.types),
    ("dc:format", |m| &mut m.formats),
    ("dc:source", |m| &mut m.sources),
    ("dc:relation", |m| &mut m.relations),
    ("dc:coverage", |m| &mut m.coverages),
    ("dc:rights", |m| &mut m.rights),
];

pub fn parse_package(bytes: &[u8]) -> Result<Package, String> {
    let root = parse_xml(bytes)?;
    if root.name != "package" {
        return Err(format!("expected <package> root, found <{}>", root.name));
    }
    // `xmlns` is dropped rather than round-tripped into `extra_attrs`:
    // `emit_package` always re-adds it unconditionally (the OPF namespace
    // never varies), so preserving a parsed copy in `extra_attrs` would
    // duplicate it on the next `emit` (quick-xml rejects duplicate
    // attributes when reparsing that output).
    let recognized_attrs = [
        "version",
        "unique-identifier",
        "xml:lang",
        "dir",
        "id",
        "xmlns",
    ];
    let extra_attrs = root
        .attrs
        .iter()
        .filter(|(k, _)| !recognized_attrs.contains(&k.as_str()))
        .cloned()
        .collect();

    let metadata = root
        .child_named("metadata")
        .map(parse_metadata)
        .unwrap_or_default();
    let manifest = root
        .child_named("manifest")
        .map(parse_manifest)
        .unwrap_or_default();
    let spine = root
        .child_named("spine")
        .map(parse_spine)
        .unwrap_or_default();
    let guide = root
        .child_named("guide")
        .map(parse_guide)
        .unwrap_or_default();

    let extra_elements = root
        .children
        .iter()
        .filter_map(|n| match n {
            XmlNode::Element(e)
                if !matches!(e.name.as_str(), "metadata" | "manifest" | "spine" | "guide") =>
            {
                Some(RawXml {
                    name: e.name.clone(),
                    attrs: e.attrs.clone(),
                    raw_inner: e.inner_xml(),
                })
            }
            _ => None,
        })
        .collect();

    Ok(Package {
        version: root.attr("version").unwrap_or_default().to_string(),
        unique_identifier: root
            .attr("unique-identifier")
            .unwrap_or_default()
            .to_string(),
        xml_lang: root.attr("xml:lang").map(str::to_string),
        dir: root.attr("dir").map(str::to_string),
        id: root.attr("id").map(str::to_string),
        metadata,
        manifest,
        spine,
        guide,
        extra_elements,
        extra_attrs,
        span: Span::NONE,
    })
}

/// Parse a bare `<metadata>...</metadata>` XML fragment (as produced by
/// [`metadata_inner_xml`]) back into a [`Metadata`]. Used by the
/// `rescribe` feature to round-trip metadata exactly through a raw XML
/// blob rather than re-deriving it from the IR's flattened metadata keys.
pub fn parse_metadata_xml(inner_xml: &str) -> Result<Metadata, String> {
    let wrapped = format!("<metadata>{inner_xml}</metadata>");
    let el = crate::xml::parse_xml(wrapped.as_bytes())?;
    Ok(parse_metadata(&el))
}

/// Serialize `m`'s children as the inner XML of a `<metadata>` element
/// (excluding the `<metadata>` tag itself), for lossless raw-preservation
/// via a single string property.
pub fn metadata_inner_xml(m: &Metadata) -> String {
    emit_metadata(m).inner_xml()
}

/// Parse a bare `<guide>...</guide>` XML fragment back into `Vec<GuideRef>`.
pub fn parse_guide_xml(inner_xml: &str) -> Result<Vec<GuideRef>, String> {
    let wrapped = format!("<guide>{inner_xml}</guide>");
    let el = crate::xml::parse_xml(wrapped.as_bytes())?;
    Ok(parse_guide(&el))
}

/// Serialize `guide`'s `<reference>` elements as the inner XML of a
/// `<guide>` element.
pub fn guide_inner_xml(guide: &[GuideRef]) -> String {
    emit_guide(guide).inner_xml()
}

fn parse_metadata(el: &XmlElement) -> Metadata {
    let mut m = Metadata::default();
    for node in &el.children {
        let XmlNode::Element(e) = node else { continue };
        if let Some((_, getter)) = DC_NAMES.iter().find(|(name, _)| *name == e.name) {
            getter(&mut m).push(DcElement {
                value: e.text(),
                attrs: e.attrs.clone(),
            });
            continue;
        }
        match e.name.as_str() {
            "meta" => m.metas.push(parse_meta(e)),
            "link" => m.links.push(parse_link(e)),
            _ => m.extra_elements.push(RawXml {
                name: e.name.clone(),
                attrs: e.attrs.clone(),
                raw_inner: e.inner_xml(),
            }),
        }
    }
    m
}

fn parse_meta(e: &XmlElement) -> MetaElement {
    let recognized = ["property", "refines", "scheme", "id", "name", "content"];
    MetaElement {
        property: e.attr("property").map(str::to_string),
        refines: e.attr("refines").map(str::to_string),
        scheme: e.attr("scheme").map(str::to_string),
        id: e.attr("id").map(str::to_string),
        value: e.text(),
        name: e.attr("name").map(str::to_string),
        content: e.attr("content").map(str::to_string),
        extra_attrs: e
            .attrs
            .iter()
            .filter(|(k, _)| !recognized.contains(&k.as_str()))
            .cloned()
            .collect(),
    }
}

fn parse_link(e: &XmlElement) -> LinkElement {
    let recognized = ["href", "rel", "media-type", "properties", "refines", "id"];
    LinkElement {
        href: e.attr("href").unwrap_or_default().to_string(),
        rel: e.attr("rel").map(str::to_string),
        media_type: e.attr("media-type").map(str::to_string),
        properties: e.attr("properties").map(str::to_string),
        refines: e.attr("refines").map(str::to_string),
        id: e.attr("id").map(str::to_string),
        extra_attrs: e
            .attrs
            .iter()
            .filter(|(k, _)| !recognized.contains(&k.as_str()))
            .cloned()
            .collect(),
    }
}

fn parse_manifest(el: &XmlElement) -> Vec<ManifestItem> {
    let recognized = [
        "id",
        "href",
        "media-type",
        "properties",
        "fallback",
        "media-overlay",
    ];
    el.children_named("item")
        .into_iter()
        .map(|e| ManifestItem {
            id: e.attr("id").unwrap_or_default().to_string(),
            href: e.attr("href").unwrap_or_default().to_string(),
            media_type: e.attr("media-type").unwrap_or_default().to_string(),
            properties: e
                .attr("properties")
                .map(|p| p.split_whitespace().map(str::to_string).collect())
                .unwrap_or_default(),
            fallback: e.attr("fallback").map(str::to_string),
            media_overlay: e.attr("media-overlay").map(str::to_string),
            extra_attrs: e
                .attrs
                .iter()
                .filter(|(k, _)| !recognized.contains(&k.as_str()))
                .cloned()
                .collect(),
        })
        .collect()
}

fn parse_spine(el: &XmlElement) -> Spine {
    let recognized = ["id", "toc", "page-progression-direction"];
    let item_recognized = ["idref", "linear", "id", "properties"];
    let items = el
        .children_named("itemref")
        .into_iter()
        .map(|e| SpineItemRef {
            idref: e.attr("idref").unwrap_or_default().to_string(),
            linear: e.attr("linear") != Some("no"),
            id: e.attr("id").map(str::to_string),
            properties: e
                .attr("properties")
                .map(|p| p.split_whitespace().map(str::to_string).collect())
                .unwrap_or_default(),
            extra_attrs: e
                .attrs
                .iter()
                .filter(|(k, _)| !item_recognized.contains(&k.as_str()))
                .cloned()
                .collect(),
        })
        .collect();
    Spine {
        id: el.attr("id").map(str::to_string),
        toc: el.attr("toc").map(str::to_string),
        page_progression_direction: el.attr("page-progression-direction").map(str::to_string),
        items,
        extra_attrs: el
            .attrs
            .iter()
            .filter(|(k, _)| !recognized.contains(&k.as_str()))
            .cloned()
            .collect(),
    }
}

fn parse_guide(el: &XmlElement) -> Vec<GuideRef> {
    el.children_named("reference")
        .into_iter()
        .map(|e| GuideRef {
            type_: e.attr("type").unwrap_or_default().to_string(),
            title: e.attr("title").map(str::to_string),
            href: e.attr("href").unwrap_or_default().to_string(),
        })
        .collect()
}

// ── Emit ─────────────────────────────────────────────────────────────────

pub fn emit_package(p: &Package) -> Vec<u8> {
    let mut attrs = vec![
        ("version".to_string(), p.version.clone()),
        ("unique-identifier".to_string(), p.unique_identifier.clone()),
        (
            "xmlns".to_string(),
            "http://www.idpf.org/2007/opf".to_string(),
        ),
    ];
    if let Some(v) = &p.xml_lang {
        attrs.push(("xml:lang".to_string(), v.clone()));
    }
    if let Some(v) = &p.dir {
        attrs.push(("dir".to_string(), v.clone()));
    }
    if let Some(v) = &p.id {
        attrs.push(("id".to_string(), v.clone()));
    }
    attrs.extend(p.extra_attrs.clone());

    let mut children = vec![
        XmlNode::Element(emit_metadata(&p.metadata)),
        XmlNode::Element(emit_manifest(&p.manifest)),
        XmlNode::Element(emit_spine(&p.spine)),
    ];
    if !p.guide.is_empty() {
        children.push(XmlNode::Element(emit_guide(&p.guide)));
    }
    for extra in &p.extra_elements {
        children.push(XmlNode::Element(raw_to_element(extra)));
    }

    let root = XmlElement {
        name: "package".to_string(),
        attrs,
        children,
    };
    emit_xml(&root)
}

fn raw_to_element(raw: &RawXml) -> XmlElement {
    let inner = crate::xml::parse_xml(
        format!(
            "<{name}>{inner}</{name}>",
            name = raw.name,
            inner = raw.raw_inner
        )
        .as_bytes(),
    )
    .unwrap_or_else(|_| XmlElement {
        name: raw.name.clone(),
        attrs: Vec::new(),
        children: Vec::new(),
    });
    XmlElement {
        name: raw.name.clone(),
        attrs: raw.attrs.clone(),
        children: inner.children,
    }
}

fn emit_metadata(m: &Metadata) -> XmlElement {
    let mut children = Vec::new();
    for (name, _) in DC_NAMES {
        for dc in dc_elements_for(m, name) {
            children.push(XmlNode::Element(XmlElement {
                name: name.to_string(),
                attrs: dc.attrs.clone(),
                children: vec![XmlNode::Text(dc.value.clone())],
            }));
        }
    }
    for meta in &m.metas {
        children.push(XmlNode::Element(emit_meta(meta)));
    }
    for link in &m.links {
        children.push(XmlNode::Element(emit_link(link)));
    }
    for extra in &m.extra_elements {
        children.push(XmlNode::Element(raw_to_element(extra)));
    }
    XmlElement {
        name: "metadata".to_string(),
        attrs: vec![(
            "xmlns:dc".to_string(),
            "http://purl.org/dc/elements/1.1/".to_string(),
        )],
        children,
    }
}

fn dc_elements_for<'a>(m: &'a Metadata, name: &str) -> &'a [DcElement] {
    match name {
        "dc:identifier" => &m.identifiers,
        "dc:title" => &m.titles,
        "dc:language" => &m.languages,
        "dc:creator" => &m.creators,
        "dc:contributor" => &m.contributors,
        "dc:subject" => &m.subjects,
        "dc:description" => &m.descriptions,
        "dc:publisher" => &m.publishers,
        "dc:date" => &m.dates,
        "dc:type" => &m.types,
        "dc:format" => &m.formats,
        "dc:source" => &m.sources,
        "dc:relation" => &m.relations,
        "dc:coverage" => &m.coverages,
        "dc:rights" => &m.rights,
        _ => &[],
    }
}

fn emit_meta(meta: &MetaElement) -> XmlElement {
    let mut attrs = Vec::new();
    if let Some(v) = &meta.property {
        attrs.push(("property".to_string(), v.clone()));
    }
    if let Some(v) = &meta.refines {
        attrs.push(("refines".to_string(), v.clone()));
    }
    if let Some(v) = &meta.scheme {
        attrs.push(("scheme".to_string(), v.clone()));
    }
    if let Some(v) = &meta.id {
        attrs.push(("id".to_string(), v.clone()));
    }
    if let Some(v) = &meta.name {
        attrs.push(("name".to_string(), v.clone()));
    }
    if let Some(v) = &meta.content {
        attrs.push(("content".to_string(), v.clone()));
    }
    attrs.extend(meta.extra_attrs.clone());
    let children = if meta.value.is_empty() {
        Vec::new()
    } else {
        vec![XmlNode::Text(meta.value.clone())]
    };
    XmlElement {
        name: "meta".to_string(),
        attrs,
        children,
    }
}

fn emit_link(link: &LinkElement) -> XmlElement {
    let mut attrs = vec![("href".to_string(), link.href.clone())];
    if let Some(v) = &link.rel {
        attrs.push(("rel".to_string(), v.clone()));
    }
    if let Some(v) = &link.media_type {
        attrs.push(("media-type".to_string(), v.clone()));
    }
    if let Some(v) = &link.properties {
        attrs.push(("properties".to_string(), v.clone()));
    }
    if let Some(v) = &link.refines {
        attrs.push(("refines".to_string(), v.clone()));
    }
    if let Some(v) = &link.id {
        attrs.push(("id".to_string(), v.clone()));
    }
    attrs.extend(link.extra_attrs.clone());
    XmlElement {
        name: "link".to_string(),
        attrs,
        children: Vec::new(),
    }
}

fn emit_manifest(items: &[ManifestItem]) -> XmlElement {
    let children = items
        .iter()
        .map(|item| {
            let mut attrs = vec![
                ("id".to_string(), item.id.clone()),
                ("href".to_string(), item.href.clone()),
                ("media-type".to_string(), item.media_type.clone()),
            ];
            if !item.properties.is_empty() {
                attrs.push(("properties".to_string(), item.properties.join(" ")));
            }
            if let Some(v) = &item.fallback {
                attrs.push(("fallback".to_string(), v.clone()));
            }
            if let Some(v) = &item.media_overlay {
                attrs.push(("media-overlay".to_string(), v.clone()));
            }
            attrs.extend(item.extra_attrs.clone());
            XmlNode::Element(XmlElement {
                name: "item".to_string(),
                attrs,
                children: Vec::new(),
            })
        })
        .collect();
    XmlElement {
        name: "manifest".to_string(),
        attrs: Vec::new(),
        children,
    }
}

fn emit_spine(spine: &Spine) -> XmlElement {
    let mut attrs = Vec::new();
    if let Some(v) = &spine.id {
        attrs.push(("id".to_string(), v.clone()));
    }
    if let Some(v) = &spine.toc {
        attrs.push(("toc".to_string(), v.clone()));
    }
    if let Some(v) = &spine.page_progression_direction {
        attrs.push(("page-progression-direction".to_string(), v.clone()));
    }
    attrs.extend(spine.extra_attrs.clone());

    let children = spine
        .items
        .iter()
        .map(|item| {
            let mut attrs = vec![("idref".to_string(), item.idref.clone())];
            if !item.linear {
                attrs.push(("linear".to_string(), "no".to_string()));
            }
            if let Some(v) = &item.id {
                attrs.push(("id".to_string(), v.clone()));
            }
            if !item.properties.is_empty() {
                attrs.push(("properties".to_string(), item.properties.join(" ")));
            }
            attrs.extend(item.extra_attrs.clone());
            XmlNode::Element(XmlElement {
                name: "itemref".to_string(),
                attrs,
                children: Vec::new(),
            })
        })
        .collect();
    XmlElement {
        name: "spine".to_string(),
        attrs,
        children,
    }
}

fn emit_guide(guide: &[GuideRef]) -> XmlElement {
    let children = guide
        .iter()
        .map(|g| {
            let mut attrs = vec![("type".to_string(), g.type_.clone())];
            if let Some(v) = &g.title {
                attrs.push(("title".to_string(), v.clone()));
            }
            attrs.push(("href".to_string(), g.href.clone()));
            XmlNode::Element(XmlElement {
                name: "reference".to_string(),
                attrs,
                children: Vec::new(),
            })
        })
        .collect();
    XmlElement {
        name: "guide".to_string(),
        attrs: Vec::new(),
        children,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Package {
        Package {
            version: "3.0".to_string(),
            unique_identifier: "pub-id".to_string(),
            xml_lang: None,
            dir: None,
            id: None,
            metadata: Metadata {
                identifiers: vec![DcElement {
                    value: "urn:uuid:1234".to_string(),
                    attrs: vec![("id".to_string(), "pub-id".to_string())],
                }],
                titles: vec![DcElement {
                    value: "Sample Book".to_string(),
                    attrs: Vec::new(),
                }],
                languages: vec![DcElement {
                    value: "en".to_string(),
                    attrs: Vec::new(),
                }],
                creators: vec![DcElement {
                    value: "Author Name".to_string(),
                    attrs: vec![("opf:role".to_string(), "aut".to_string())],
                }],
                metas: vec![MetaElement {
                    property: Some("dcterms:modified".to_string()),
                    value: "2026-01-01T00:00:00Z".to_string(),
                    ..Default::default()
                }],
                ..Default::default()
            },
            manifest: vec![
                ManifestItem {
                    id: "nav".to_string(),
                    href: "nav.xhtml".to_string(),
                    media_type: "application/xhtml+xml".to_string(),
                    properties: vec!["nav".to_string()],
                    ..Default::default()
                },
                ManifestItem {
                    id: "ch1".to_string(),
                    href: "chapter1.xhtml".to_string(),
                    media_type: "application/xhtml+xml".to_string(),
                    ..Default::default()
                },
            ],
            spine: Spine {
                items: vec![SpineItemRef {
                    idref: "ch1".to_string(),
                    ..Default::default()
                }],
                ..Default::default()
            },
            guide: Vec::new(),
            extra_elements: Vec::new(),
            extra_attrs: Vec::new(),
            span: Span::NONE,
        }
    }

    #[test]
    fn roundtrip() {
        let p = sample();
        let bytes = emit_package(&p);
        let p2 = parse_package(&bytes).unwrap();
        assert_eq!(p, p2);
    }

    #[test]
    fn linear_no_roundtrips() {
        let mut p = sample();
        p.spine.items.push(SpineItemRef {
            idref: "notes".to_string(),
            linear: false,
            ..Default::default()
        });
        let bytes = emit_package(&p);
        let p2 = parse_package(&bytes).unwrap();
        assert!(!p2.spine.items[1].linear);
    }
}
