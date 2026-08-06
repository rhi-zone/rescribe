//! Test-only helper: build a small but structurally complete sample EPUB
//! (container + OPF with metadata/manifest/spine + nav + two content
//! documents + one CSS resource) for use across this crate's unit tests.

use crate::ast::*;
use rescribe_format_api::Parse as _;

pub fn sample_epub() -> Vec<u8> {
    let (nav_doc, _) = html_fmt::HtmlDoc::parse(
        br#"<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
<head><title>Nav</title></head>
<body>
<nav epub:type="toc"><h1>Table of Contents</h1><ol>
<li><a href="chapter1.xhtml">Chapter 1</a></li>
<li><a href="chapter2.xhtml">Chapter 2</a></li>
</ol></nav>
</body></html>"#,
    );
    let (ch1_doc, _) = html_fmt::HtmlDoc::parse(
        br#"<!DOCTYPE html><html><head><title>Ch1</title></head><body><h1>Chapter 1</h1><p>Hello.</p></body></html>"#,
    );
    let (ch2_doc, _) = html_fmt::HtmlDoc::parse(
        br#"<!DOCTYPE html><html><head><title>Ch2</title></head><body><h1>Chapter 2</h1><p>World.</p></body></html>"#,
    );

    let doc = EpubDoc {
        container: Container {
            rootfiles: vec![RootFile {
                full_path: "OEBPS/content.opf".to_string(),
                media_type: "application/oebps-package+xml".to_string(),
            }],
            span: Span::NONE,
        },
        package: Package {
            version: "3.0".to_string(),
            unique_identifier: "pub-id".to_string(),
            metadata: Metadata {
                identifiers: vec![DcElement {
                    value: "urn:uuid:sample-1234".to_string(),
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
                    value: "Sample Author".to_string(),
                    attrs: Vec::new(),
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
                ManifestItem {
                    id: "ch2".to_string(),
                    href: "chapter2.xhtml".to_string(),
                    media_type: "application/xhtml+xml".to_string(),
                    ..Default::default()
                },
                ManifestItem {
                    id: "style".to_string(),
                    href: "style.css".to_string(),
                    media_type: "text/css".to_string(),
                    ..Default::default()
                },
            ],
            spine: Spine {
                items: vec![
                    SpineItemRef {
                        idref: "ch1".to_string(),
                        ..Default::default()
                    },
                    SpineItemRef {
                        idref: "ch2".to_string(),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            },
            ..Default::default()
        },
        nav: Some(Navigation {
            path: "OEBPS/nav.xhtml".to_string(),
            toc: Some(NavList {
                heading: Some("Table of Contents".to_string()),
                items: vec![
                    NavPoint {
                        label: "Chapter 1".to_string(),
                        href: Some("chapter1.xhtml".to_string()),
                        children: Vec::new(),
                    },
                    NavPoint {
                        label: "Chapter 2".to_string(),
                        href: Some("chapter2.xhtml".to_string()),
                        children: Vec::new(),
                    },
                ],
            }),
            page_list: None,
            landmarks: None,
            other: Vec::new(),
            doc: nav_doc,
        }),
        ncx: None,
        content_documents: vec![
            ContentDocument {
                path: "OEBPS/chapter1.xhtml".to_string(),
                media_type: "application/xhtml+xml".to_string(),
                doc: ch1_doc,
            },
            ContentDocument {
                path: "OEBPS/chapter2.xhtml".to_string(),
                media_type: "application/xhtml+xml".to_string(),
                doc: ch2_doc,
            },
        ],
        resources: vec![ResourceEntry {
            path: "OEBPS/style.css".to_string(),
            media_type: "text/css".to_string(),
            content: b"body { font-family: serif; }".to_vec(),
        }],
        encryption_xml: None,
        unclassified: Vec::new(),
        span: Span::NONE,
    };

    crate::emit::emit(&doc)
}
