//! `emit()` — full-tree EPUB writer: builds a `zip-fmt::Archive` from an
//! [`EpubDoc`] and serializes it via `zip-fmt::Archive::emit`.

use crate::ast::{CONTAINER_PATH, ENCRYPTION_PATH, EpubDoc, MIMETYPE_CONTENT, MIMETYPE_ENTRY};
use rescribe_format_api::Emit as _;
use zip_fmt::{Archive, CompressionMethod, Entry};

pub fn emit(doc: &EpubDoc) -> Vec<u8> {
    let mut entries = Vec::new();

    // The mimetype entry must be first and stored (uncompressed), per the
    // OCF spec — this lets a generic ZIP/file-magic sniffer identify an
    // EPUB without inflating anything.
    entries.push(Entry {
        name: MIMETYPE_ENTRY.to_string(),
        is_utf8_name: true,
        compression: CompressionMethod::Store,
        content: MIMETYPE_CONTENT.as_bytes().to_vec(),
        ..Entry::default()
    });

    entries.push(deflated(
        CONTAINER_PATH,
        crate::container::emit_container(&doc.container),
    ));

    let opf_path = doc
        .container
        .rootfiles
        .first()
        .map(|r| r.full_path.clone())
        .unwrap_or_else(|| "OEBPS/content.opf".to_string());
    entries.push(deflated(&opf_path, crate::opf::emit_package(&doc.package)));

    if let Some(nav) = &doc.nav {
        entries.push(deflated(&nav.path, nav.doc.emit()));
    }
    if let Some(ncx) = &doc.ncx {
        entries.push(deflated(&ncx.path, crate::ncx::emit_ncx(ncx)));
    }
    for cd in &doc.content_documents {
        entries.push(deflated(&cd.path, cd.doc.emit()));
    }
    for res in &doc.resources {
        entries.push(deflated(&res.path, res.content.clone()));
    }
    if let Some(enc) = &doc.encryption_xml {
        entries.push(deflated(ENCRYPTION_PATH, enc.clone()));
    }
    for (name, content) in &doc.unclassified {
        entries.push(deflated(name, content.clone()));
    }

    let archive = Archive {
        entries,
        comment: Vec::new(),
        span: Default::default(),
    };
    archive.emit()
}

fn deflated(name: &str, content: Vec<u8>) -> Entry {
    Entry {
        name: name.to_string(),
        is_utf8_name: true,
        compression: CompressionMethod::Deflate,
        content,
        ..Entry::default()
    }
}
