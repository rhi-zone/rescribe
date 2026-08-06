//! `parse()` — full-tree EPUB reader: `zip-fmt::Archive::parse` for the
//! container, this crate's own OPF/NCX/`container.xml` XML translation
//! (`opf.rs`/`ncx.rs`/`container.rs`), and `html-fmt::HtmlDoc::parse` for
//! every XHTML content document (including `nav.xhtml`) — via the shared
//! `classify::classify_entry` per-entry logic.

use crate::ast::{CONTAINER_PATH, ENCRYPTION_PATH, EpubDoc, MIMETYPE_ENTRY, Span};
use crate::classify::{Classified, classify_entry};
use crate::pathutil::dir_of;
use rescribe_format_api::{Diagnostic, Parse as _, Severity};
use zip_fmt::Archive;

pub fn parse(input: &[u8]) -> (EpubDoc, Vec<Diagnostic>) {
    let mut diags = Vec::new();
    let (archive, zip_diags) = Archive::parse(input);
    diags.extend(zip_diags);

    let find = |path: &str| archive.entries.iter().find(|e| e.name == path);

    let container = match find(CONTAINER_PATH) {
        Some(e) => match crate::container::parse_container(&e.content) {
            Ok(c) => c,
            Err(msg) => {
                diags.push(warn(format!("failed to parse container.xml: {msg}")));
                Default::default()
            }
        },
        None => {
            diags.push(warn("missing META-INF/container.xml"));
            Default::default()
        }
    };

    let opf_path = container.rootfiles.first().map(|r| r.full_path.clone());
    let mut package = Default::default();
    if let Some(opf_path) = &opf_path {
        match find(opf_path) {
            Some(e) => match crate::opf::parse_package(&e.content) {
                Ok(p) => package = p,
                Err(msg) => {
                    diags.push(warn(format!("failed to parse OPF package document: {msg}")))
                }
            },
            None => diags.push(warn(format!("OPF package document not found: {opf_path}"))),
        }
    } else {
        diags.push(warn("container.xml has no <rootfile>"));
    }

    let base_dir = opf_path.as_deref().map(dir_of).unwrap_or_default();

    let mut classified_paths = std::collections::HashSet::new();
    classified_paths.insert(MIMETYPE_ENTRY.to_string());
    classified_paths.insert(CONTAINER_PATH.to_string());
    if let Some(p) = &opf_path {
        classified_paths.insert(p.clone());
    }

    let mut nav = None;
    let mut ncx = None;
    let mut content_documents = Vec::new();
    let mut resources = Vec::new();

    for item in &package.manifest {
        let path = crate::pathutil::resolve_href(&base_dir, &item.href);
        classified_paths.insert(path.clone());
        let Some(entry) = find(&path) else {
            diags.push(warn(format!(
                "manifest item '{}' references missing archive entry: {path}",
                item.id
            )));
            continue;
        };
        match classify_entry(&package, &base_dir, &path, &entry.content, &mut diags) {
            Some(Classified::Nav(n)) => nav = Some(n),
            Some(Classified::Ncx(n)) => ncx = Some(n),
            Some(Classified::ContentDocument(d)) => content_documents.push(d),
            Some(Classified::Resource(r)) => resources.push(r),
            None => {}
        }
    }

    let encryption_xml = find(ENCRYPTION_PATH).map(|e| {
        classified_paths.insert(ENCRYPTION_PATH.to_string());
        e.content.clone()
    });

    let unclassified = archive
        .entries
        .iter()
        .filter(|e| !classified_paths.contains(&e.name) && !e.is_dir())
        .map(|e| (e.name.clone(), e.content.clone()))
        .collect();

    (
        EpubDoc {
            container,
            package,
            nav,
            ncx,
            content_documents,
            resources,
            encryption_xml,
            unclassified,
            span: Span::NONE,
        },
        diags,
    )
}

pub(crate) fn warn(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(Severity::Warning, message.into())
}
