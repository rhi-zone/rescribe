#![no_main]

//! epub-fmt native AST roundtrip fuzz target.
//!
//! Constructs an arbitrary `EpubDoc` from fuzz data, emits it via
//! `epub_fmt::emit`, parses it back via `epub_fmt::parse`, and asserts
//! structural equality (after `strip_spans`).
//!
//! Direction: arbitrary_epub_ast → emit → parse → assert equality. Per
//! `CLAUDE.md`'s roundtrip-direction rule, this starts from the format
//! crate's own `EpubDoc` type, not from `parse(bytes)`.
//!
//! # Why `nav`'s `toc`/`page_list`/`landmarks`/`other` are left empty
//!
//! Those fields are a read-only *projection* derived from `Navigation::doc`
//! at parse time (see `nav.rs`'s module docs) — `emit()` only ever
//! serializes `doc`, never those fields directly. To keep this target's
//! equality assertion meaningful without duplicating `epub-fmt`'s private
//! nav-extraction logic here, the generated `nav.doc` deliberately
//! contains no `<nav epub:type="...">` elements, so the reparsed
//! projection is `None`/empty on both sides — matching what was
//! generated, and covering the OPF/manifest/spine/resource/encryption/
//! unclassified-entry round-trip this target exists for.

use epub_fmt::EpubDoc;
use libfuzzer_sys::fuzz_target;
use rescribe_format_api::{Emit as _, Parse as _};

struct Gen<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Gen<'a> {
    fn new(data: &'a [u8]) -> Self {
        Gen { data, pos: 0 }
    }

    fn byte(&mut self) -> u8 {
        if self.pos < self.data.len() {
            let b = self.data[self.pos];
            self.pos += 1;
            b
        } else {
            0
        }
    }

    fn bytes(&mut self, n: usize) -> Vec<u8> {
        let start = self.pos;
        let end = (self.pos + n).min(self.data.len());
        self.pos = end;
        self.data[start..end].to_vec()
    }

    /// Lowercase ASCII letters only — never empty, never contains any
    /// character that requires XML/HTML escaping.
    fn safe_text(&mut self, n: usize) -> String {
        let raw: String = self
            .bytes(n)
            .iter()
            .map(|b| ((*b % 26) + b'a') as char)
            .collect();
        if raw.is_empty() { "x".to_string() } else { raw }
    }

    fn bool(&mut self) -> bool {
        self.byte() % 2 == 0
    }
}

fn html_doc(text: &str) -> html_fmt::HtmlDoc {
    let src = format!("<!DOCTYPE html><html><body><p>{text}</p></body></html>");
    let (doc, _diags) = html_fmt::HtmlDoc::parse(src.as_bytes());
    doc
}

fuzz_target!(|data: &[u8]| {
    if data.len() < 8 {
        return;
    }

    let mut g = Gen::new(data);

    let title = g.safe_text(6);
    let author = g.safe_text(6);
    let n_chapters = (g.byte() % 3) + 1;
    let has_resource = g.bool();
    let has_encryption = g.bool();
    let n_unclassified = g.byte() % 3;

    let mut manifest = vec![epub_fmt::ManifestItem {
        id: "nav".to_string(),
        href: "nav.xhtml".to_string(),
        media_type: "application/xhtml+xml".to_string(),
        properties: vec!["nav".to_string()],
        ..Default::default()
    }];
    let mut spine_items = Vec::new();
    let mut content_documents = Vec::new();

    for i in 0..n_chapters {
        let id = format!("ch{i}");
        let href = format!("ch{i}.xhtml");
        manifest.push(epub_fmt::ManifestItem {
            id: id.clone(),
            href: href.clone(),
            media_type: "application/xhtml+xml".to_string(),
            ..Default::default()
        });
        spine_items.push(epub_fmt::SpineItemRef {
            idref: id,
            linear: g.bool(),
            ..Default::default()
        });
        content_documents.push(epub_fmt::ContentDocument {
            path: format!("OEBPS/{href}"),
            media_type: "application/xhtml+xml".to_string(),
            doc: html_doc(&g.safe_text(8)),
        });
    }

    let mut resources = Vec::new();
    if has_resource {
        manifest.push(epub_fmt::ManifestItem {
            id: "style".to_string(),
            href: "style.css".to_string(),
            media_type: "text/css".to_string(),
            ..Default::default()
        });
        resources.push(epub_fmt::ResourceEntry {
            path: "OEBPS/style.css".to_string(),
            media_type: "text/css".to_string(),
            content: g.bytes(16),
        });
    }

    let encryption_xml = if has_encryption {
        Some(g.bytes(12))
    } else {
        None
    };

    let unclassified: Vec<(String, Vec<u8>)> = (0..n_unclassified)
        .map(|i| (format!("extra/{}{i}", g.safe_text(4)), g.bytes(8)))
        .collect();

    let doc = EpubDoc {
        container: epub_fmt::Container {
            rootfiles: vec![epub_fmt::RootFile {
                full_path: "OEBPS/content.opf".to_string(),
                media_type: "application/oebps-package+xml".to_string(),
            }],
            span: Default::default(),
        },
        package: epub_fmt::Package {
            version: "3.0".to_string(),
            unique_identifier: "pub-id".to_string(),
            metadata: epub_fmt::Metadata {
                identifiers: vec![epub_fmt::DcElement {
                    value: "urn:uuid:fuzz".to_string(),
                    attrs: vec![("id".to_string(), "pub-id".to_string())],
                }],
                titles: vec![epub_fmt::DcElement {
                    value: title,
                    attrs: Vec::new(),
                }],
                languages: vec![epub_fmt::DcElement {
                    value: "en".to_string(),
                    attrs: Vec::new(),
                }],
                creators: vec![epub_fmt::DcElement {
                    value: author,
                    attrs: Vec::new(),
                }],
                ..Default::default()
            },
            manifest,
            spine: epub_fmt::Spine {
                items: spine_items,
                ..Default::default()
            },
            ..Default::default()
        },
        nav: Some(epub_fmt::Navigation {
            path: "OEBPS/nav.xhtml".to_string(),
            toc: None,
            page_list: None,
            landmarks: None,
            other: Vec::new(),
            doc: html_doc("nav"),
        }),
        ncx: None,
        content_documents,
        resources,
        encryption_xml,
        unclassified,
        span: Default::default(),
    };

    let emitted = doc.emit();
    let (doc2, diags) = EpubDoc::parse(&emitted);
    assert!(
        diags.is_empty(),
        "unexpected diagnostics reparsing generated EPUB: {diags:?}"
    );
    assert_eq!(
        doc.strip_spans(),
        doc2.strip_spans(),
        "epub-fmt roundtrip mismatch"
    );
});
