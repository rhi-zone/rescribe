//! Spec-derived construct registry: the machine-readable catalog of every
//! construct the JATS Archiving and Interchange Tag Set defines.
//!
//! # What this is for
//!
//! "Does this library cover the whole format?" is only answerable against a
//! trustworthy *denominator* — the full list of constructs the format
//! actually defines. Hand-written checklists cannot supply that: they grow
//! by incidental discovery, so a "101/105 covered" ratio measures the list
//! someone wrote, not the format. This module supplies the denominator
//! mechanically, derived from the format's own published schema.
//!
//! The registry is **spec-pure**. It records what JATS defines, never what
//! this crate or any downstream consumer supports. Support status is a
//! *join* the consumer performs — see [`Registry::contains_element`] and the
//! `in_normative_slice` / `elements` iterators — so the registry never churns when
//! implementation work lands.
//!
//! # Slices: normative vs. pragmatic
//!
//! Every construct is annotated with the **slice**(s) it belongs to, in two
//! independent, separately-provenanced collections (per
//! `docs/adr/0013-per-format-construct-registry.md`'s 2026-07-28 amendment):
//!
//! - [`Construct::normative_slices`] — a partition the format itself
//!   publishes. For JATS this is the DTD Suite's own module files
//!   (`JATS-section1-3.ent`, `JATS-phrase1-3.ent`, …); a construct can belong
//!   to more than one when several modules declare it, listed in the driver
//!   schema's `<include>` order so `normative_slices[0]` is a stable primary.
//!   A downstream implementer can use these to decide how to decompose work —
//!   "implement the section and para modules first" is a statement the format
//!   supports, unlike "implement the easy elements first." This list may be
//!   empty for a format whose normative schema publishes no modularization at
//!   all (DocBook is the motivating case); JATS's is never empty.
//! - [`Construct::pragmatic_slices`] — a partition invented by whoever
//!   maintains this registry (e.g. ooxml's `core`/`styling`/`charts` feature
//!   groupings), explicitly non-normative. Always permitted, since it makes
//!   no claim to reflect the format's own structure. The JATS pilot leaves
//!   this empty for every construct: JATS's normative modularization already
//!   does the decomposition job, so inventing a second grouping nobody asked
//!   for would just be noise.
//!
//! # Citations survive an absent schema
//!
//! Citations are **external** references (canonical URLs and a spec
//! identifier), never `file:line` into a vendored schema copy. The schema is
//! not vendored in this repository, and for some formats (notably OOXML) it
//! legally cannot be — so a citation form that only resolves when the schema
//! is present would be useless exactly where it is needed most. See
//! `docs/adr/0013-per-format-construct-registry.md`.
//!
//! # Availability
//!
//! Behind the `registry` Cargo feature, off by default: a consumer that only
//! wants to parse XML should not pay for the catalog.
//!
//! ```no_run
//! # #[cfg(feature = "registry")] {
//! let reg = jats_fmt::registry::registry();
//! assert!(reg.contains_element("sec"));
//! for c in reg.in_normative_slice("JATS-section1-3.ent") {
//!     println!("{} ({})", c.name, reg.citation_url(c).unwrap_or_default());
//! }
//! # }
//! ```

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

/// The committed registry document, verbatim.
///
/// Exposed so consumers that already have a YAML or serde pipeline (e.g. a
/// `rescribe query`-style jaq filter) can feed the raw document straight in
/// without going through the typed API.
pub const REGISTRY_YAML: &str = include_str!("../registry/jats-1.3-archiving.yaml");

/// What kind of thing a construct is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstructKind {
    /// An XML element the format defines.
    Element,
    /// An XML attribute the format defines.
    Attribute,
}

impl ConstructKind {
    /// The prefix used in a construct's stable id (`element:sec`).
    pub fn id_prefix(self) -> &'static str {
        match self {
            ConstructKind::Element => "element",
            ConstructKind::Attribute => "attribute",
        }
    }
}

/// The form of the authoritative schema a registry was derived from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceKind {
    /// RELAX NG, XML syntax.
    Relaxng,
    /// RELAX NG, compact syntax.
    Rnc,
    /// XML DTD.
    Dtd,
    /// W3C XML Schema.
    Xsd,
    /// TEI ODD (literate schema source).
    Odd,
    /// No machine-readable schema exists; entries were curated by hand.
    ///
    /// A registry carrying this kind does **not** deliver the guarantee the
    /// rest of the design exists to provide — it is as fallible as the
    /// hand-written checklist it replaces, and must say so.
    HandCurated,
}

/// Which format, version, and profile this registry describes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormatInfo {
    /// Short machine id, e.g. `jats`.
    pub id: String,
    /// Human-readable format name.
    pub name: String,
    /// Format version the registry describes, e.g. `1.3`.
    pub version: String,
    /// Sub-profile / tag set id, where the format has several, e.g. `archiving`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    /// Human-readable profile name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_name: Option<String>,
}

/// A checksum of one source schema file, so staleness is detectable even
/// when the schema itself is not present in the checkout.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceDigest {
    /// File name as published, e.g. `JATS-section1-3.ent.rng`.
    pub file: String,
    /// Size in bytes at derivation time.
    pub bytes: u64,
    /// Lowercase hex SHA-256 of the file's bytes at derivation time.
    pub sha256: String,
}

/// Where the registry came from and how, so a reader can judge staleness
/// without holding the source schema.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    /// The standard, cited as it names itself, e.g. `ANSI/NISO Z39.96-2021`.
    pub spec: String,
    /// Form of the schema the registry was derived from.
    pub source_kind: SourceKind,
    /// Driver/entry-point schema file the derivation started from.
    pub source_driver: String,
    /// Canonical base URL the source files are published at. Joined with a
    /// file name this yields a stable, resolvable citation.
    pub source_base_url: String,
    /// The source schema's license, quoted or named.
    pub source_license: String,
    /// Whether that license permits redistributing the schema verbatim.
    pub source_redistributable: bool,
    /// Whether this repository actually vendors a copy of the schema.
    ///
    /// Distinct from `source_redistributable`: a schema may be legally
    /// vendorable and still not vendored. When false, re-derivation requires
    /// fetching the source first.
    pub source_vendored: bool,
    /// ISO-8601 date the registry was derived.
    pub derived_on: String,
    /// Tool and version that performed the derivation.
    pub derived_by: String,
    /// Per-file checksums of every source consumed.
    #[serde(default)]
    pub source_digests: Vec<SourceDigest>,
}

/// How to build a citation URL for a construct.
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Citation {
    /// URL template for elements; `{name}` is replaced with the local name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub element_url_template: Option<String>,
    /// URL template for attributes; `{name}` is replaced with the local name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribute_url_template: Option<String>,
}

/// One partition of the format — either the format's own published
/// modularization, or a hand-curated grouping. Which one a given `Slice`
/// belongs to is determined by *which list it lives in*
/// (`Registry::normative_slices` vs. `Registry::pragmatic_slices`), not by a
/// field on this type — see the module docs and
/// `docs/adr/0013-per-format-construct-registry.md`'s 2026-07-28 amendment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Slice {
    /// Stable id. For a normative slice, the format's own module identifier;
    /// for a pragmatic slice, whatever id its curator chose.
    pub id: String,
    /// Declared name. For a normative slice, taken from the schema file; for
    /// a pragmatic slice, curator-chosen.
    pub name: String,
    /// Source schema file that declares this slice's constructs. Empty for a
    /// pragmatic slice with no backing schema file.
    #[serde(default)]
    pub source_file: String,
    /// Resolvable URL for that file, or for whatever explains the pragmatic
    /// grouping's rationale. Empty if none exists.
    #[serde(default)]
    pub url: String,
}

/// One construct the format defines.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Construct {
    /// Stable id, `{kind}:{name}`, e.g. `element:sec`.
    pub id: String,
    /// Local name as it appears in a document.
    pub name: String,
    /// Element, attribute, …
    pub kind: ConstructKind,
    /// Ids into `Registry::normative_slices` that declare this construct, in
    /// driver `<include>` order. Empty only when the format's normative
    /// schema publishes no modularization at all; JATS's is never empty.
    /// `normative_slices[0]`, when non-empty, is the stable primary.
    #[serde(default)]
    pub normative_slices: Vec<String>,
    /// Ids into `Registry::pragmatic_slices` this construct has been
    /// hand-assigned to. Always legitimately empty — no format is required
    /// to have a pragmatic partition, and JATS's pilot leaves this empty for
    /// every construct.
    #[serde(default)]
    pub pragmatic_slices: Vec<String>,
}

impl Construct {
    /// The primary normative slice id — the first module to declare this
    /// construct in the driver schema's include order, if the format
    /// publishes a normative modularization at all.
    pub fn primary_normative_slice(&self) -> Option<&str> {
        self.normative_slices.first().map(String::as_str)
    }

    /// The primary pragmatic slice id, if this construct has been assigned
    /// to any pragmatic grouping.
    pub fn primary_pragmatic_slice(&self) -> Option<&str> {
        self.pragmatic_slices.first().map(String::as_str)
    }
}

/// The full spec-derived catalog for one format/version/profile.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Registry {
    /// Schema version of the *registry document format* itself, so a
    /// consumer can tell v1 from v2. `2` introduced the normative/pragmatic
    /// slice split (`docs/adr/0013-...`'s 2026-07-28 amendment); `1` had a
    /// single `slices` field.
    pub registry_version: u32,
    /// Which format this describes.
    pub format: FormatInfo,
    /// Where it came from.
    pub provenance: Provenance,
    /// How to cite an individual construct.
    #[serde(default)]
    pub citation: Citation,
    /// The format's own published modularization. May be empty for a format
    /// whose normative schema publishes no modularization (e.g. DocBook);
    /// when empty, `normative_slices_absent_reason` should say why.
    #[serde(default)]
    pub normative_slices: Vec<Slice>,
    /// Why `normative_slices` is empty, when it is. `None` when
    /// `normative_slices` is non-empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normative_slices_absent_reason: Option<String>,
    /// A hand-curated, explicitly non-normative partition. Always
    /// legitimately empty — a format gains nothing from a pragmatic grouping
    /// nobody asked for. JATS's pilot leaves this empty.
    #[serde(default)]
    pub pragmatic_slices: Vec<Slice>,
    /// Every construct, sorted by id.
    pub constructs: Vec<Construct>,
}

impl Registry {
    /// Parse a registry document from YAML.
    pub fn from_yaml(src: &str) -> Result<Registry, serde_yaml::Error> {
        serde_yaml::from_str(src)
    }

    /// All constructs, sorted by id.
    pub fn constructs(&self) -> &[Construct] {
        &self.constructs
    }

    /// The format's own published modules.
    pub fn normative_slices(&self) -> &[Slice] {
        &self.normative_slices
    }

    /// This registry's hand-curated, explicitly non-normative groupings.
    pub fn pragmatic_slices(&self) -> &[Slice] {
        &self.pragmatic_slices
    }

    /// Look up a normative slice by id.
    pub fn normative_slice(&self, id: &str) -> Option<&Slice> {
        self.normative_slices.iter().find(|s| s.id == id)
    }

    /// Look up a pragmatic slice by id.
    pub fn pragmatic_slice(&self, id: &str) -> Option<&Slice> {
        self.pragmatic_slices.iter().find(|s| s.id == id)
    }

    /// Look up a construct by its stable id, e.g. `element:sec`.
    pub fn get(&self, id: &str) -> Option<&Construct> {
        self.constructs
            .binary_search_by(|c| c.id.as_str().cmp(id))
            .ok()
            .map(|i| &self.constructs[i])
    }

    /// Look up a construct by kind and local name.
    pub fn lookup(&self, kind: ConstructKind, name: &str) -> Option<&Construct> {
        self.get(&format!("{}:{}", kind.id_prefix(), name))
    }

    /// Does the format define an element with this name?
    pub fn contains_element(&self, name: &str) -> bool {
        self.lookup(ConstructKind::Element, name).is_some()
    }

    /// Does the format define an attribute with this name?
    pub fn contains_attribute(&self, name: &str) -> bool {
        self.lookup(ConstructKind::Attribute, name).is_some()
    }

    /// Every element the format defines.
    pub fn elements(&self) -> impl Iterator<Item = &Construct> {
        self.of_kind(ConstructKind::Element)
    }

    /// Every attribute the format defines.
    pub fn attributes(&self) -> impl Iterator<Item = &Construct> {
        self.of_kind(ConstructKind::Attribute)
    }

    /// Every construct of one kind.
    pub fn of_kind(&self, kind: ConstructKind) -> impl Iterator<Item = &Construct> {
        self.constructs.iter().filter(move |c| c.kind == kind)
    }

    /// Every construct declared by a given normative slice.
    pub fn in_normative_slice<'a>(
        &'a self,
        slice_id: &'a str,
    ) -> impl Iterator<Item = &'a Construct> {
        self.constructs
            .iter()
            .filter(move |c| c.normative_slices.iter().any(|s| s == slice_id))
    }

    /// Every construct assigned to a given pragmatic slice.
    pub fn in_pragmatic_slice<'a>(
        &'a self,
        slice_id: &'a str,
    ) -> impl Iterator<Item = &'a Construct> {
        self.constructs
            .iter()
            .filter(move |c| c.pragmatic_slices.iter().any(|s| s == slice_id))
    }

    /// A resolvable citation URL for a construct, if a template is defined.
    pub fn citation_url(&self, c: &Construct) -> Option<String> {
        let tpl = match c.kind {
            ConstructKind::Element => self.citation.element_url_template.as_ref()?,
            ConstructKind::Attribute => self.citation.attribute_url_template.as_ref()?,
        };
        Some(tpl.replace("{name}", &c.name))
    }

    /// Constructs of `kind` whose names are **not** in `handled` — the
    /// coverage-gap join.
    ///
    /// This is the query the registry exists for: the caller supplies what it
    /// actually handles, the registry supplies what the format defines, and
    /// the difference is the honest gap. Support status deliberately does not
    /// live in the registry, so this join is the caller's to make.
    pub fn not_handled<I, S>(
        &self,
        kind: ConstructKind,
        handled: I,
    ) -> impl Iterator<Item = &Construct>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let handled: std::collections::BTreeSet<String> = handled
            .into_iter()
            .map(|s| s.as_ref().to_string())
            .collect();
        self.of_kind(kind)
            .filter(move |c| !handled.contains(&c.name))
    }

    /// Count of constructs per normative slice, for a coverage-report-style
    /// summary.
    pub fn counts_by_normative_slice(&self, kind: ConstructKind) -> BTreeMap<&str, usize> {
        let mut out = BTreeMap::new();
        for c in self.of_kind(kind) {
            for s in &c.normative_slices {
                *out.entry(s.as_str()).or_insert(0) += 1;
            }
        }
        out
    }

    /// Count of constructs per pragmatic slice, for a coverage-report-style
    /// summary.
    pub fn counts_by_pragmatic_slice(&self, kind: ConstructKind) -> BTreeMap<&str, usize> {
        let mut out = BTreeMap::new();
        for c in self.of_kind(kind) {
            for s in &c.pragmatic_slices {
                *out.entry(s.as_str()).or_insert(0) += 1;
            }
        }
        out
    }
}

static REGISTRY: OnceLock<Registry> = OnceLock::new();

/// The JATS 1.3 Archiving registry.
///
/// Parsed once from the committed YAML document on first access.
///
/// # Panics
///
/// Only if the committed registry document is malformed, which a crate test
/// rules out at build time.
pub fn registry() -> &'static Registry {
    REGISTRY.get_or_init(|| {
        Registry::from_yaml(REGISTRY_YAML).expect("committed registry document must parse")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn committed_document_parses() {
        let r = registry();
        assert_eq!(r.registry_version, 2);
        assert_eq!(r.format.id, "jats");
        assert_eq!(r.format.version, "1.3");
        assert_eq!(r.format.profile.as_deref(), Some("archiving"));
    }

    #[test]
    fn constructs_are_sorted_and_unique_so_binary_search_is_valid() {
        let r = registry();
        for w in r.constructs.windows(2) {
            assert!(w[0].id < w[1].id, "unsorted or duplicate: {:?}", w[0].id);
        }
    }

    #[test]
    fn every_construct_id_resolves() {
        let r = registry();
        for c in r.constructs() {
            assert_eq!(r.get(&c.id).map(|x| &x.id), Some(&c.id));
        }
    }

    #[test]
    fn every_construct_slice_is_declared() {
        let r = registry();
        for c in r.constructs() {
            // JATS has a normative modularization, so every construct must
            // cite at least one; pragmatic slices remain unused by the pilot.
            assert!(
                !c.normative_slices.is_empty(),
                "{} has no normative slice",
                c.id
            );
            for s in &c.normative_slices {
                assert!(
                    r.normative_slice(s).is_some(),
                    "{} cites undeclared normative slice {s}",
                    c.id
                );
            }
            for s in &c.pragmatic_slices {
                assert!(
                    r.pragmatic_slice(s).is_some(),
                    "{} cites undeclared pragmatic slice {s}",
                    c.id
                );
            }
        }
    }

    #[test]
    fn known_elements_are_present_with_the_right_slice() {
        let r = registry();
        // Spot-check against the JATS DTD Suite's own module assignments.
        assert!(r.contains_element("sec"));
        assert_eq!(
            r.lookup(ConstructKind::Element, "sec")
                .unwrap()
                .primary_normative_slice(),
            Some("JATS-section1-3.ent")
        );
        assert!(r.contains_element("article"));
        assert!(r.contains_element("italic"));
        assert!(!r.contains_element("this-element-does-not-exist"));
    }

    #[test]
    fn citations_resolve_to_the_tag_library() {
        let r = registry();
        let sec = r.lookup(ConstructKind::Element, "sec").unwrap();
        assert_eq!(
            r.citation_url(sec).as_deref(),
            Some("https://jats.nlm.nih.gov/archiving/tag-library/1.3/element/sec.html")
        );
    }

    #[test]
    fn provenance_is_complete() {
        let p = &registry().provenance;
        assert!(!p.spec.is_empty());
        assert!(!p.derived_on.is_empty());
        assert!(!p.derived_by.is_empty());
        assert!(!p.source_digests.is_empty(), "no source checksums recorded");
        for d in &p.source_digests {
            assert_eq!(d.sha256.len(), 64, "{} digest is not sha-256", d.file);
        }
    }

    #[test]
    fn not_handled_is_the_gap_join() {
        let r = registry();
        let gaps: Vec<_> = r
            .not_handled(ConstructKind::Element, ["sec", "p"])
            .map(|c| c.name.as_str())
            .collect();
        assert!(!gaps.contains(&"sec"));
        assert!(!gaps.contains(&"p"));
        assert!(gaps.contains(&"article"));
    }
}
