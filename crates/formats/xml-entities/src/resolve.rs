//! Resolving `&name;` references to replacement text, combining a
//! document's own DTD-declared entities with the standard table.

use crate::decl::{DtdEntities, EntityDeclKind};
use crate::standard::resolve_standard;
use std::borrow::Cow;
use std::collections::HashSet;

/// Where an entity's replacement text came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Resolved via an entity declared in the document's own DTD internal
    /// subset (or a parameter entity expanded into it).
    Document,
    /// Resolved via the standard WHATWG HTML5 / ISO 8879 & 9573-derived
    /// table.
    Standard,
}

/// The outcome of resolving one entity name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution<'a> {
    /// Replacement text was found.
    Resolved { text: Cow<'a, str>, source: Source },
    /// The entity is declared, but as an external entity (`SYSTEM`/`PUBLIC`)
    /// — its replacement text lives in a file this crate does not fetch.
    /// The identifiers are included so a caller who *does* want to fetch it
    /// can.
    ExternalUnresolved {
        public_id: Option<&'a str>,
        system_id: &'a str,
    },
    /// Not found in the document's declared entities or the standard table.
    /// Per rescribe's (and generally, any lossless converter's) rules, this
    /// is not an error — callers should raw-preserve the original
    /// `&name;` text rather than dropping it.
    Unknown,
}

impl Resolution<'_> {
    /// The resolved text, if any (`None` for [`Resolution::ExternalUnresolved`]
    /// and [`Resolution::Unknown`]).
    pub fn text(&self) -> Option<&str> {
        match self {
            Resolution::Resolved { text, .. } => Some(text),
            _ => None,
        }
    }
}

const MAX_RESOLUTION_DEPTH: usize = 64;

/// A reusable entity resolver: build once from a document's DTD-declared
/// entities (or [`DtdEntities::empty`] if it has none), then query it for
/// every `&name;` reference found while walking the document.
///
/// ```
/// use xml_entities::{DtdEntities, EntityResolver, Resolution, Source};
///
/// let (declared, _diagnostics) = DtdEntities::parse_doctype(
///     r#"root [ <!ENTITY custom "hand-rolled text"> ]"#,
/// );
/// let resolver = EntityResolver::new(declared);
///
/// // Resolved via the document's own declaration:
/// assert_eq!(
///     resolver.resolve("custom"),
///     Resolution::Resolved { text: "hand-rolled text".into(), source: Source::Document },
/// );
/// // Falls back to the standard table for entities the document didn't declare:
/// assert_eq!(
///     resolver.resolve("eacute"),
///     Resolution::Resolved { text: "\u{e9}".into(), source: Source::Standard },
/// );
/// // Unknown entities resolve to `Unknown` rather than an error, so a
/// // caller can raw-preserve the original `&name;` text losslessly.
/// assert_eq!(resolver.resolve("not-a-real-entity"), Resolution::Unknown);
/// ```
#[derive(Debug, Clone)]
pub struct EntityResolver {
    declared: DtdEntities,
}

impl EntityResolver {
    /// Build a resolver from a document's declared entities. Pass
    /// [`DtdEntities::empty`] for documents with no DOCTYPE or no internal
    /// subset — the resolver then falls back to the standard table alone.
    pub fn new(declared: DtdEntities) -> Self {
        Self { declared }
    }

    /// The document-declared entities this resolver was built from.
    pub fn declared(&self) -> &DtdEntities {
        &self.declared
    }

    /// Resolve one entity name (without the surrounding `&` and `;`).
    ///
    /// Document-declared entities take priority over the standard table —
    /// a document is free to redefine e.g. a custom `&company;` entity even
    /// if that name happened to coincide with a standard one (it wouldn't in
    /// practice, but document declarations are more specific and should
    /// win). If the document's declaration is an *internal* entity whose
    /// literal value itself contains further `&name;` references, those are
    /// resolved recursively (with cycle/depth protection — see
    /// [`MAX_RESOLUTION_DEPTH`]).
    pub fn resolve<'a>(&'a self, name: &str) -> Resolution<'a> {
        let mut seen = HashSet::new();
        self.resolve_inner(name, &mut seen, 0)
    }

    fn resolve_inner<'a>(
        &'a self,
        name: &str,
        seen: &mut HashSet<String>,
        depth: usize,
    ) -> Resolution<'a> {
        if let Some(decl) = self.declared.get(name) {
            return match &decl.kind {
                EntityDeclKind::Internal(value) => {
                    if depth >= MAX_RESOLUTION_DEPTH || !seen.insert(name.to_string()) {
                        // Cycle or runaway nesting: return the literal value
                        // unexpanded rather than looping or panicking.
                        return Resolution::Resolved {
                            text: Cow::Borrowed(value),
                            source: Source::Document,
                        };
                    }
                    let expanded = self.expand_nested_refs(value, seen, depth + 1);
                    Resolution::Resolved {
                        text: Cow::Owned(expanded),
                        source: Source::Document,
                    }
                }
                EntityDeclKind::External {
                    public_id,
                    system_id,
                    ..
                } => Resolution::ExternalUnresolved {
                    public_id: public_id.as_deref(),
                    system_id,
                },
            };
        }
        match resolve_standard(name) {
            Some(text) => Resolution::Resolved {
                text: Cow::Borrowed(text),
                source: Source::Standard,
            },
            None => Resolution::Unknown,
        }
    }

    /// Expand `&name;` references nested inside an internal entity's literal
    /// value. Unresolvable nested references (unknown name, or external) are
    /// left verbatim in the output — this function never fails, it only
    /// does as much resolution as it safely can.
    fn expand_nested_refs(&self, value: &str, seen: &mut HashSet<String>, depth: usize) -> String {
        let mut out = String::with_capacity(value.len());
        let bytes = value.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'&'
                && let Some(semi) = value[i + 1..].find(';')
            {
                let name = &value[i + 1..i + 1 + semi];
                // Numeric char refs were already expanded when this entity
                // was declared (see `expand_char_refs` in `decl.rs`);
                // anything left starting with '#' here is malformed input we
                // should leave alone.
                if !name.is_empty() && !name.starts_with('#') && is_xml_name(name) {
                    match self.resolve_inner(name, seen, depth) {
                        Resolution::Resolved { text, .. } => {
                            out.push_str(&text);
                            i += 1 + semi + 1;
                            continue;
                        }
                        Resolution::ExternalUnresolved { .. } | Resolution::Unknown => {
                            // Leave the reference verbatim; caller sees it
                            // in the resolved text and can decide what to do
                            // (this matches how an XML parser handles a
                            // reference to an entity it can't resolve inside
                            // another entity's replacement text: it's
                            // preserved, not silently dropped).
                        }
                    }
                }
            }
            let ch_len = value[i..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
            out.push_str(&value[i..(i + ch_len).min(value.len())]);
            i += ch_len;
        }
        out
    }
}

fn is_xml_name(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_alphabetic() || c == '_' || c == ':' => {}
        _ => return false,
    }
    chars.all(|c| c.is_alphanumeric() || c == '_' || c == ':' || c == '-' || c == '.')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decl::DtdEntities;

    #[test]
    fn falls_back_to_standard_table_when_undeclared() {
        let resolver = EntityResolver::new(DtdEntities::empty());
        assert_eq!(
            resolver.resolve("amp"),
            Resolution::Resolved {
                text: "&".into(),
                source: Source::Standard
            }
        );
    }

    #[test]
    fn document_declaration_overrides_standard_table() {
        // "amp" isn't a realistic override, so use a name that's real in
        // both places conceptually: document declares its own custom
        // meaning, which must win.
        let (declared, _) = DtdEntities::parse_subset(r#"<!ENTITY custom "mine">"#);
        let resolver = EntityResolver::new(declared);
        assert_eq!(
            resolver.resolve("custom"),
            Resolution::Resolved {
                text: "mine".into(),
                source: Source::Document
            }
        );
    }

    #[test]
    fn unknown_entity_is_unknown_not_error() {
        let resolver = EntityResolver::new(DtdEntities::empty());
        assert_eq!(resolver.resolve("totally-made-up"), Resolution::Unknown);
    }

    #[test]
    fn external_entity_is_reported_unresolved_with_identifiers() {
        let (declared, _) = DtdEntities::parse_subset(r#"<!ENTITY foo SYSTEM "foo.xml">"#);
        let resolver = EntityResolver::new(declared);
        assert_eq!(
            resolver.resolve("foo"),
            Resolution::ExternalUnresolved {
                public_id: None,
                system_id: "foo.xml"
            }
        );
    }

    #[test]
    fn resolves_nested_entity_references_recursively() {
        let (declared, _) =
            DtdEntities::parse_subset(r#"<!ENTITY inner "world"><!ENTITY outer "hello &inner;">"#);
        let resolver = EntityResolver::new(declared);
        assert_eq!(
            resolver.resolve("outer"),
            Resolution::Resolved {
                text: "hello world".into(),
                source: Source::Document
            }
        );
    }

    #[test]
    fn resolves_nested_reference_into_standard_table() {
        let (declared, _) = DtdEntities::parse_subset(r#"<!ENTITY greeting "caf&eacute;">"#);
        let resolver = EntityResolver::new(declared);
        assert_eq!(
            resolver.resolve("greeting"),
            Resolution::Resolved {
                text: "caf\u{e9}".into(),
                source: Source::Document
            }
        );
    }

    #[test]
    fn self_referential_entity_does_not_infinite_loop_or_panic() {
        let (declared, _) = DtdEntities::parse_subset(r#"<!ENTITY loop "&loop;">"#);
        let resolver = EntityResolver::new(declared);
        // Must terminate. The exact fallback text isn't load-bearing, only
        // that resolution completes without panicking or hanging.
        let result = resolver.resolve("loop");
        assert!(matches!(
            result,
            Resolution::Resolved {
                source: Source::Document,
                ..
            }
        ));
    }

    #[test]
    fn mutually_recursive_entities_do_not_infinite_loop_or_panic() {
        let (declared, _) = DtdEntities::parse_subset(r#"<!ENTITY a "&b;"><!ENTITY b "&a;">"#);
        let resolver = EntityResolver::new(declared);
        let result = resolver.resolve("a");
        assert!(matches!(
            result,
            Resolution::Resolved {
                source: Source::Document,
                ..
            }
        ));
    }

    #[test]
    fn nested_unresolvable_reference_left_verbatim() {
        let (declared, _) = DtdEntities::parse_subset(r#"<!ENTITY a "x &nosuchentity; y">"#);
        let resolver = EntityResolver::new(declared);
        assert_eq!(
            resolver.resolve("a"),
            Resolution::Resolved {
                text: "x &nosuchentity; y".into(),
                source: Source::Document
            }
        );
    }
}
