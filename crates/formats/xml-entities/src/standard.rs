//! The standard named-character-reference table.
//!
//! Sourced from the `entities` crate, which republishes the WHATWG HTML5
//! "named character references" table
//! (<https://html.spec.whatwg.org/multipage/named-characters.html>, itself
//! derived from <https://www.w3.org/TR/html5/entities.json>). That table is a
//! superset of the ISO 8879 (SGML) and ISO 9573 entity sets used by DocBook,
//! JATS, TEI and friends (`ISOlat1`, `ISOnum`, `ISOpub`, `ISOtech`, `mmlalias`,
//! and so on) — HTML5 absorbed nearly all of them when it standardized named
//! character references. It is not a perfect superset: a handful of
//! DTD-specific entities have no HTML5 equivalent, and every DTD also defines
//! its own custom entities that only a copy of that DTD would know about.
//! Those cases are exactly what [`crate::DtdEntities`] is for.
//!
//! HTML5's table includes legacy names without a trailing `;` (e.g. `&amp`),
//! kept for compatibility with old HTML parsing quirks. XML has no such
//! quirks mode — every entity reference requires the trailing `;` — so this
//! module only indexes the semicolon-terminated forms.

use std::collections::HashMap;
use std::sync::OnceLock;

/// Look up a standard (non-document-declared) named entity, e.g. `"amp"` or
/// `"eacute"` (without the surrounding `&` and `;`).
///
/// Returns the replacement text (which may be more than one character, e.g.
/// `"NotEqualTilde"` expands to a two-codepoint sequence) or `None` if the
/// name is not part of the WHATWG HTML5 named character reference table.
pub fn resolve_standard(name: &str) -> Option<&'static str> {
    table().get(name).copied()
}

/// The number of entries in the standard table (semicolon-terminated forms
/// only). Exposed mainly for tests and diagnostics.
pub fn standard_table_len() -> usize {
    table().len()
}

fn table() -> &'static HashMap<&'static str, &'static str> {
    static TABLE: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut map = HashMap::with_capacity(entities::ENTITIES.len());
        for entity in entities::ENTITIES.iter() {
            // `entity.entity` is like "&amp;" or, for legacy forms, "&amp"
            // (no trailing semicolon). XML requires the semicolon, so skip
            // anything that doesn't have one.
            let Some(name) = entity
                .entity
                .strip_prefix('&')
                .and_then(|s| s.strip_suffix(';'))
            else {
                continue;
            };
            map.insert(name, entity.characters);
        }
        map
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_common_entities() {
        assert_eq!(resolve_standard("amp"), Some("&"));
        assert_eq!(resolve_standard("lt"), Some("<"));
        assert_eq!(resolve_standard("gt"), Some(">"));
        assert_eq!(resolve_standard("quot"), Some("\""));
        assert_eq!(resolve_standard("apos"), Some("'"));
    }

    #[test]
    fn resolves_iso_latin1_style_entities() {
        // ISOlat1 entities absorbed into HTML5's table.
        assert_eq!(resolve_standard("eacute"), Some("\u{e9}"));
        assert_eq!(resolve_standard("aring"), Some("\u{e5}"));
        assert_eq!(resolve_standard("ntilde"), Some("\u{f1}"));
    }

    #[test]
    fn resolves_isonum_style_entities() {
        // ISOnum-style entities absorbed into HTML5's table.
        assert_eq!(resolve_standard("half"), Some("\u{bd}"));
        assert_eq!(resolve_standard("plusmn"), Some("\u{b1}"));
    }

    #[test]
    fn unknown_name_is_none() {
        assert_eq!(resolve_standard("this-is-not-a-real-entity"), None);
    }

    #[test]
    fn legacy_no_semicolon_forms_are_excluded() {
        // "AMP" without a semicolon is a legacy HTML4 quirks-mode form; it
        // must not leak into the table since XML always requires `;`.
        // (We can't test a *negative* name reliably since the map only has
        // semicolon-stripped names anyway, so this just documents the
        // invariant via the count check below.)
        assert!(standard_table_len() > 0);
        assert!(standard_table_len() < entities::ENTITIES.len());
    }
}
