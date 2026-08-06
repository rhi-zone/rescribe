//! Archive-path helpers shared by `parse.rs`/`classify.rs`/`events.rs`/
//! `batch.rs`.

/// The directory portion of an archive path (everything before the last
/// `/`, or `""` if there is none).
pub fn dir_of(path: &str) -> String {
    match path.rfind('/') {
        Some(i) => path[..i].to_string(),
        None => String::new(),
    }
}

/// Resolve a manifest `href` (a relative-reference URI, per the OPF spec)
/// against the OPF's own directory, producing an archive-entry path.
/// Handles `.`/`..` segments and strips a trailing `#fragment`, but does
/// not attempt full percent-decoding beyond the common `%20`
/// (space)/`%23` (`#`) cases real-world EPUBs use for filenames — a
/// scoped simplification, not a silent structural drop (an href using
/// other percent-escapes still resolves, just without decoding those
/// particular bytes, so lookups against unusually-escaped archive entry
/// names could fail to match; this has not been observed in real EPUBs).
pub fn resolve_href(base_dir: &str, href: &str) -> String {
    let href = href.split('#').next().unwrap_or(href);
    let href = href.replace("%20", " ").replace("%23", "#");
    let mut segments: Vec<&str> = if base_dir.is_empty() {
        Vec::new()
    } else {
        base_dir.split('/').collect()
    };
    for seg in href.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            other => segments.push(other),
        }
    }
    segments.join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_href_relative() {
        assert_eq!(
            resolve_href("OEBPS", "chapter1.xhtml"),
            "OEBPS/chapter1.xhtml"
        );
        assert_eq!(resolve_href("", "chapter1.xhtml"), "chapter1.xhtml");
        assert_eq!(
            resolve_href("OEBPS/text", "../images/a.png"),
            "OEBPS/images/a.png"
        );
    }
}
