//! Classify one archive entry against an already-parsed OPF [`Package`]'s
//! manifest. Shared by `parse.rs`, `events.rs`, and `batch.rs` — the one
//! piece of real per-entry decision logic every reader API needs, kept as
//! a plain function per `docs/format-library-design.md`'s "share state-
//! transition logic as functions, not a common runtime primitive" rule.

use crate::ast::{ContentDocument, NavList, Navigation, Ncx, Package, ResourceEntry};
use crate::pathutil::resolve_href;
use rescribe_format_api::{Diagnostic, Parse as _};

pub enum Classified {
    Nav(Navigation),
    Ncx(Ncx),
    ContentDocument(ContentDocument),
    Resource(ResourceEntry),
}

/// Returns `None` if `path` does not correspond to any manifest item's
/// resolved `href` (the entry is not part of the manifest at all — a
/// caller falls back to treating it as unclassified).
pub fn classify_entry(
    package: &Package,
    base_dir: &str,
    path: &str,
    content: &[u8],
    diags: &mut Vec<Diagnostic>,
) -> Option<Classified> {
    let item = package
        .manifest
        .iter()
        .find(|item| resolve_href(base_dir, &item.href) == path)?;

    if item.has_property("nav") {
        let (doc, html_diags) = html_fmt::HtmlDoc::parse(content);
        diags.extend(
            html_diags.into_iter().map(|d| {
                Diagnostic::new(d.severity, format!("nav document '{path}': {}", d.message))
            }),
        );
        let navs = crate::nav::find_navs(&doc);
        let mut toc: Option<NavList> = None;
        let mut page_list: Option<NavList> = None;
        let mut landmarks: Option<NavList> = None;
        let mut other = Vec::new();
        for (epub_type, el) in navs {
            let list = crate::nav::extract_nav_list(el);
            match epub_type.as_str() {
                "toc" => toc = Some(list),
                "page-list" => page_list = Some(list),
                "landmarks" => landmarks = Some(list),
                other_type => other.push((other_type.to_string(), list)),
            }
        }
        Some(Classified::Nav(Navigation {
            path: path.to_string(),
            toc,
            page_list,
            landmarks,
            other,
            doc,
        }))
    } else if item.media_type == "application/x-dtbncx+xml" {
        match crate::ncx::parse_ncx(path, content) {
            Ok(ncx) => Some(Classified::Ncx(ncx)),
            Err(msg) => {
                diags.push(Diagnostic::new(
                    rescribe_format_api::Severity::Warning,
                    format!("failed to parse NCX '{path}': {msg}"),
                ));
                None
            }
        }
    } else if item.media_type == "application/xhtml+xml" || item.media_type == "text/html" {
        let (doc, html_diags) = html_fmt::HtmlDoc::parse(content);
        diags.extend(html_diags.into_iter().map(|d| {
            Diagnostic::new(
                d.severity,
                format!("content document '{path}': {}", d.message),
            )
        }));
        Some(Classified::ContentDocument(ContentDocument {
            path: path.to_string(),
            media_type: item.media_type.clone(),
            doc,
        }))
    } else {
        Some(Classified::Resource(ResourceEntry {
            path: path.to_string(),
            media_type: item.media_type.clone(),
            content: content.to_vec(),
        }))
    }
}
