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
//! # Runtime representation: committed generated Rust statics, not parsed data
//!
//! Every field in this module's types is `&'static str` / `&'static [T]`,
//! backed by [`generated::REGISTRY`] — a single `static` value compiled
//! directly into the binary's read-only data, committed as
//! `src/registry_generated.rs` the same way `ooxml-wml`'s `src/generated.rs`
//! is committed. [`registry()`] is a plain reference return, not a parse: no
//! serialization crate, no runtime deserialization step, no heap allocation
//! for any of this module's data. A consumer who enables the `registry`
//! feature pays only for the static data itself being linked in — nothing
//! else.
//!
//! The human-readable, language-agnostic **source** this generated file is
//! derived from lives alongside it, at `registry/jats-1.3-archiving.json`,
//! and is not read by this crate at runtime at all — it exists for review
//! diffs, for non-Rust consumers, and as the input `derive-registry`
//! regenerates `registry_generated.rs` from. See `src/registry_derive.rs`
//! (behind the `registry-derive` feature) for the derivation/regeneration/
//! drift-check tooling, and its module docs for why JSON (via `serde_json`)
//! replaced YAML (via the now-archived `serde_yaml`) as that source format.
//!
//! # Slices: normative vs. pragmatic
//!
//! Every construct is annotated with the **slice**(s) it belongs to, in two
//! independent, separately-provenanced collections:
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
//! # Content models: flattened, not full grammar
//!
//! [`Construct::content_model`] records, per element, the *set* of permitted
//! direct children and attributes — plus whether each child can repeat, each
//! attribute is required, and whether character data may appear directly —
//! but **not** the source schema's ordering, choice, group, or interleave
//! structure. RELAX NG can express "an optional `title`, then one or more
//! `p`, in that order, interleaved with `fn-group`"; this registry records
//! only "`title` (not repeatable), `p` (repeatable), `fn-group` (not
//! repeatable) are permitted children," with no claim about sequence or
//! grouping.
//!
//! This is a deliberate scope decision, not an oversight. The two questions
//! a registry consumer asks are different in kind: "can `sec` contain `fig`?"
//! (a set-membership question, answered by the flattened form) versus "is
//! *this specific* `sec` element valid?" (a validation question, which needs
//! the full grammar — order, choice exclusivity, interleave). This design
//! serves the first and explicitly does not attempt the second: a linter or
//! validator that needs true schema validation should run the source schema
//! through a RELAX NG validator, not this registry.
//!
//! Many elements share an identical content model (e.g. every element built
//! from the same reusable schema pattern). [`generated::REGISTRY`] stores
//! each *distinct* content model once, as a named `static`, and every
//! construct with that model references it by pointer
//! (`Option<&'static ContentModel>`) — so the committed generated file's size
//! tracks the number of distinct shapes, not the number of constructs.
//!
//! # Citations survive an absent schema
//!
//! Citations are **external** references (canonical URLs and a spec
//! identifier), never `file:line` into a vendored schema copy. The schema is
//! not vendored in this repository, and for some formats (notably OOXML) it
//! legally cannot be — so a citation form that only resolves when the schema
//! is present would be useless exactly where it is needed most.
//!
//! # Availability
//!
//! Behind the `registry` Cargo feature, off by default: a consumer that only
//! wants to parse XML should not compile, or pay for, the catalog.
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

#[path = "registry_generated.rs"]
pub(crate) mod generated;

use std::collections::BTreeMap;

/// What kind of thing a construct is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "registry", derive(serde::Serialize))]
#[cfg_attr(feature = "registry-derive", derive(serde::Deserialize))]
#[cfg_attr(
    any(feature = "registry", feature = "registry-derive"),
    serde(rename_all = "snake_case")
)]
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

/// The form of the authoritative schema (or, for [`SourceKind::ScriptedExtraction`],
/// the published artifact) a registry was derived from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "registry", derive(serde::Serialize))]
#[cfg_attr(feature = "registry-derive", derive(serde::Deserialize))]
#[cfg_attr(
    any(feature = "registry", feature = "registry-derive"),
    serde(rename_all = "kebab-case")
)]
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
    /// The format has no machine-readable schema, so the construct list was
    /// produced by a script that mechanically extracts it from a published
    /// prose artifact (e.g. an HTML element index), rather than by parsing a
    /// grammar. First-class, fully permitted — not a marked-down fallback —
    /// because the property this design needs is *reproducibility*, not
    /// "came from a schema."
    ScriptedExtraction,
}

/// Which format, version, and profile this registry describes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "registry", derive(serde::Serialize))]
pub struct FormatInfo {
    /// Short machine id, e.g. `jats`.
    pub id: &'static str,
    /// Human-readable format name.
    pub name: &'static str,
    /// Format version the registry describes, e.g. `1.3`.
    pub version: &'static str,
    /// Sub-profile / tag set id, where the format has several, e.g. `archiving`.
    pub profile: Option<&'static str>,
    /// Human-readable profile name.
    pub profile_name: Option<&'static str>,
}

/// A checksum of one source file (a schema module, or, for
/// [`SourceKind::ScriptedExtraction`], one fetched prose page) so staleness
/// is detectable even when the source itself is not present in the checkout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "registry", derive(serde::Serialize))]
pub struct SourceDigest {
    /// File name as published, e.g. `JATS-section1-3.ent.rng`.
    pub file: &'static str,
    /// Size in bytes at derivation time.
    pub bytes: u64,
    /// Lowercase hex SHA-256 of the file's bytes at derivation time.
    pub sha256: &'static str,
    /// The exact URL this entry was fetched from, when it differs per entry
    /// rather than sharing `Provenance::source_base_url`. Empty when
    /// `source_base_url` plus `file` already resolves it.
    pub url: &'static str,
}

/// Where the registry came from and how, so a reader can judge staleness
/// without holding the source schema.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "registry", derive(serde::Serialize))]
pub struct Provenance {
    /// The standard, cited as it names itself, e.g. `ANSI/NISO Z39.96-2021`.
    pub spec: &'static str,
    /// Form of the schema the registry was derived from.
    pub source_kind: SourceKind,
    /// Driver/entry-point schema file the derivation started from.
    pub source_driver: &'static str,
    /// Canonical base URL the source files are published at.
    pub source_base_url: &'static str,
    /// The source schema's license, quoted or named.
    pub source_license: &'static str,
    /// Whether that license permits redistributing the schema verbatim.
    pub source_redistributable: bool,
    /// Whether this repository actually vendors a copy of the schema.
    pub source_vendored: bool,
    /// ISO-8601 date the registry was derived.
    pub derived_on: &'static str,
    /// Tool and version that performed the derivation.
    pub derived_by: &'static str,
    /// Per-file checksums of every source consumed.
    pub source_digests: &'static [SourceDigest],
}

/// How to build a citation URL for a construct.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "registry", derive(serde::Serialize))]
pub struct Citation {
    /// URL template for elements; `{name}` is replaced with the local name.
    pub element_url_template: Option<&'static str>,
    /// URL template for attributes; `{name}` is replaced with the local name.
    pub attribute_url_template: Option<&'static str>,
}

/// One partition of the format — either the format's own published
/// modularization, or a hand-curated grouping. Which one a given `Slice`
/// belongs to is determined by *which list it lives in*
/// (`Registry::normative_slices` vs. `Registry::pragmatic_slices`), not by a
/// field on this type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "registry", derive(serde::Serialize))]
pub struct Slice {
    /// Stable id. For a normative slice, the format's own module identifier;
    /// for a pragmatic slice, whatever id its curator chose.
    pub id: &'static str,
    /// Declared name. For a normative slice, taken from the schema file; for
    /// a pragmatic slice, curator-chosen.
    pub name: &'static str,
    /// Source schema file that declares this slice's constructs. Empty for a
    /// pragmatic slice with no backing schema file.
    pub source_file: &'static str,
    /// Resolvable URL for that file, or for whatever explains the pragmatic
    /// grouping's rationale. Empty if none exists.
    pub url: &'static str,
}

/// One child element a construct's content model permits, flattened out of
/// whatever ordering/choice/group structure the source schema expressed it
/// with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "registry", derive(serde::Serialize))]
pub struct PermittedChild {
    /// Local element name.
    pub name: &'static str,
    /// Whether the schema permits more than one occurrence of this child.
    /// `false` means the schema never allows more than one, though it says
    /// nothing about relative order, since order is exactly what flattening
    /// discards.
    pub repeatable: bool,
}

/// One attribute a construct's content model permits, with its
/// required/optional status *for this element*.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "registry", derive(serde::Serialize))]
pub struct PermittedAttribute {
    /// Local attribute name.
    pub name: &'static str,
    /// Whether the schema requires this attribute on every instance of the
    /// element. `false` covers both "optional" and "one of a choice of
    /// alternatives."
    pub required: bool,
}

/// What a construct permits as content: which child elements, which
/// attributes, and whether character data may appear directly inside it.
/// Only populated for [`ConstructKind::Element`] — attributes have a value
/// type, not a content model, and this registry does not model datatypes.
///
/// Distinct content models are deduplicated in the generated static data:
/// several constructs with the same shape reference the same `ContentModel`
/// value, rather than each carrying its own copy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "registry", derive(serde::Serialize))]
pub struct ContentModel {
    /// Every element name permitted as a direct child, in no particular
    /// order (order is not recorded — see the module docs).
    pub children: &'static [PermittedChild],
    /// Every attribute name this element permits, with required/optional
    /// status.
    pub attributes: &'static [PermittedAttribute],
    /// Whether character data (`#PCDATA` / RELAX NG `<text/>`/`<mixed>`) is
    /// permitted directly inside this element, alongside its children.
    pub mixed: bool,
}

/// One construct the format defines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "registry", derive(serde::Serialize))]
pub struct Construct {
    /// Stable id, `{kind}:{name}`, e.g. `element:sec`.
    pub id: &'static str,
    /// Local name as it appears in a document.
    pub name: &'static str,
    /// Element, attribute, …
    pub kind: ConstructKind,
    /// Ids into `Registry::normative_slices` that declare this construct, in
    /// driver `<include>` order. Empty only when the format's normative
    /// schema publishes no modularization at all; JATS's is never empty.
    /// `normative_slices[0]`, when non-empty, is the stable primary.
    pub normative_slices: &'static [&'static str],
    /// Ids into `Registry::pragmatic_slices` this construct has been
    /// hand-assigned to. Always legitimately empty — no format is required
    /// to have a pragmatic partition, and JATS's pilot leaves this empty for
    /// every construct.
    pub pragmatic_slices: &'static [&'static str],
    /// What this construct permits as content. `Some` for every
    /// [`ConstructKind::Element`] the schema actually defines a body for;
    /// `None` for [`ConstructKind::Attribute`] constructs and for any
    /// element the derivation could not resolve a model for. Points at a
    /// shared, deduplicated `ContentModel` — see the module docs.
    pub content_model: Option<&'static ContentModel>,
}

impl Construct {
    /// Does this element's content model permit `child` as a direct child?
    /// `None` if this construct has no recorded content model at all.
    pub fn permits_child(&self, child: &str) -> Option<bool> {
        self.content_model
            .map(|m| m.children.iter().any(|c| c.name == child))
    }

    /// Does this element's content model require `attr`? `Some(false)` also
    /// covers "permitted but optional"; distinguish via
    /// [`Construct::permits_attribute`] if needed. `None` if this construct
    /// has no recorded content model at all.
    pub fn requires_attribute(&self, attr: &str) -> Option<bool> {
        self.content_model
            .map(|m| m.attributes.iter().any(|a| a.name == attr && a.required))
    }

    /// Does this element's content model permit `attr` at all (required or
    /// optional)? `None` if this construct has no recorded content model.
    pub fn permits_attribute(&self, attr: &str) -> Option<bool> {
        self.content_model
            .map(|m| m.attributes.iter().any(|a| a.name == attr))
    }

    /// The primary normative slice id — the first module to declare this
    /// construct in the driver schema's include order, if the format
    /// publishes a normative modularization at all.
    pub fn primary_normative_slice(&self) -> Option<&'static str> {
        self.normative_slices.first().copied()
    }

    /// The primary pragmatic slice id, if this construct has been assigned
    /// to any pragmatic grouping.
    pub fn primary_pragmatic_slice(&self) -> Option<&'static str> {
        self.pragmatic_slices.first().copied()
    }
}

/// The full spec-derived catalog for one format/version/profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "registry", derive(serde::Serialize))]
pub struct Registry {
    /// Schema version of the *registry document format* itself, so a
    /// consumer can tell v1 from v2 from v3. `3` added
    /// [`Construct::content_model`]; `2` introduced the normative/pragmatic
    /// slice split; `1` had a single `slices` field and no content models.
    pub registry_version: u32,
    /// Which format this describes.
    pub format: FormatInfo,
    /// Where it came from.
    pub provenance: Provenance,
    /// How to cite an individual construct.
    pub citation: Citation,
    /// The format's own published modularization. May be empty for a format
    /// whose normative schema publishes no modularization (e.g. DocBook);
    /// when empty, `normative_slices_absent_reason` should say why.
    pub normative_slices: &'static [Slice],
    /// Why `normative_slices` is empty, when it is. `None` when
    /// `normative_slices` is non-empty.
    pub normative_slices_absent_reason: Option<&'static str>,
    /// A hand-curated, explicitly non-normative partition. Always
    /// legitimately empty — a format gains nothing from a pragmatic grouping
    /// nobody asked for. JATS's pilot leaves this empty.
    pub pragmatic_slices: &'static [Slice],
    /// Every construct, sorted by id.
    pub constructs: &'static [Construct],
}

impl Registry {
    /// All constructs, sorted by id.
    pub fn constructs(&self) -> &'static [Construct] {
        self.constructs
    }

    /// The format's own published modules.
    pub fn normative_slices(&self) -> &'static [Slice] {
        self.normative_slices
    }

    /// This registry's hand-curated, explicitly non-normative groupings.
    pub fn pragmatic_slices(&self) -> &'static [Slice] {
        self.pragmatic_slices
    }

    /// Look up a normative slice by id.
    pub fn normative_slice(&self, id: &str) -> Option<&'static Slice> {
        self.normative_slices.iter().find(|s| s.id == id)
    }

    /// Look up a pragmatic slice by id.
    pub fn pragmatic_slice(&self, id: &str) -> Option<&'static Slice> {
        self.pragmatic_slices.iter().find(|s| s.id == id)
    }

    /// Look up a construct by its stable id, e.g. `element:sec`.
    pub fn get(&self, id: &str) -> Option<&'static Construct> {
        self.constructs
            .binary_search_by(|c| c.id.cmp(id))
            .ok()
            .map(|i| &self.constructs[i])
    }

    /// Look up a construct by kind and local name.
    pub fn lookup(&self, kind: ConstructKind, name: &str) -> Option<&'static Construct> {
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
    pub fn elements(&self) -> impl Iterator<Item = &'static Construct> {
        self.of_kind(ConstructKind::Element)
    }

    /// Every attribute the format defines.
    pub fn attributes(&self) -> impl Iterator<Item = &'static Construct> {
        self.of_kind(ConstructKind::Attribute)
    }

    /// Every construct of one kind.
    pub fn of_kind(&self, kind: ConstructKind) -> impl Iterator<Item = &'static Construct> {
        self.constructs.iter().filter(move |c| c.kind == kind)
    }

    /// Every construct declared by a given normative slice.
    pub fn in_normative_slice<'a>(
        &self,
        slice_id: &'a str,
    ) -> impl Iterator<Item = &'static Construct> + 'a {
        self.constructs
            .iter()
            .filter(move |c| c.normative_slices.contains(&slice_id))
    }

    /// Every construct assigned to a given pragmatic slice.
    pub fn in_pragmatic_slice<'a>(
        &self,
        slice_id: &'a str,
    ) -> impl Iterator<Item = &'static Construct> + 'a {
        self.constructs
            .iter()
            .filter(move |c| c.pragmatic_slices.contains(&slice_id))
    }

    /// A resolvable citation URL for a construct, if a template is defined.
    pub fn citation_url(&self, c: &Construct) -> Option<String> {
        let tpl = match c.kind {
            ConstructKind::Element => self.citation.element_url_template?,
            ConstructKind::Attribute => self.citation.attribute_url_template?,
        };
        Some(tpl.replace("{name}", c.name))
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
    ) -> impl Iterator<Item = &'static Construct>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let handled: std::collections::BTreeSet<String> = handled
            .into_iter()
            .map(|s| s.as_ref().to_string())
            .collect();
        self.of_kind(kind)
            .filter(move |c| !handled.contains(c.name))
    }

    /// Count of constructs per normative slice, for a coverage-report-style
    /// summary.
    pub fn counts_by_normative_slice(&self, kind: ConstructKind) -> BTreeMap<&'static str, usize> {
        let mut out = BTreeMap::new();
        for c in self.of_kind(kind) {
            for s in c.normative_slices {
                *out.entry(*s).or_insert(0) += 1;
            }
        }
        out
    }

    /// Count of constructs per pragmatic slice, for a coverage-report-style
    /// summary.
    pub fn counts_by_pragmatic_slice(&self, kind: ConstructKind) -> BTreeMap<&'static str, usize> {
        let mut out = BTreeMap::new();
        for c in self.of_kind(kind) {
            for s in c.pragmatic_slices {
                *out.entry(*s).or_insert(0) += 1;
            }
        }
        out
    }
}

/// The JATS 1.3 Archiving registry: a `'static` reference to committed
/// generated data. No parsing, no allocation, no `OnceLock` — the value is
/// simply already there, compiled into the binary.
pub fn registry() -> &'static Registry {
    &generated::REGISTRY
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn committed_document_is_well_formed() {
        let r = registry();
        assert_eq!(r.registry_version, 3);
        assert_eq!(r.format.id, "jats");
        assert_eq!(r.format.version, "1.3");
        assert_eq!(r.format.profile, Some("archiving"));
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
            assert_eq!(r.get(c.id).map(|x| x.id), Some(c.id));
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
            for s in c.normative_slices {
                assert!(
                    r.normative_slice(s).is_some(),
                    "{} cites undeclared normative slice {s}",
                    c.id
                );
            }
            for s in c.pragmatic_slices {
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
        for d in p.source_digests {
            assert_eq!(d.sha256.len(), 64, "{} digest is not sha-256", d.file);
        }
    }

    #[test]
    fn content_models_are_populated_for_elements() {
        let r = registry();
        let sec = r.lookup(ConstructKind::Element, "sec").unwrap();
        let model = sec.content_model.expect("sec has a content model");
        assert!(
            model.children.iter().any(|c| c.name == "title"),
            "sec-model permits <title>"
        );
        assert!(
            model.children.iter().any(|c| c.name == "p" && c.repeatable),
            "sec-model permits repeatable <p> via para-level"
        );
        assert_eq!(sec.permits_child("title"), Some(true));
        assert_eq!(sec.permits_child("this-child-does-not-exist"), Some(false));
        assert!(
            model.attributes.iter().any(|a| a.name == "sec-type"),
            "sec-atts permits sec-type"
        );
        assert_eq!(sec.permits_attribute("sec-type"), Some(true));
    }

    #[test]
    fn attribute_constructs_have_no_content_model() {
        let r = registry();
        let attr = r.lookup(ConstructKind::Attribute, "id").unwrap();
        assert!(attr.content_model.is_none());
    }

    #[test]
    fn content_models_are_deduplicated() {
        // Regression guard for the dedup this registry's generated data
        // performs: many distinct constructs should point at the *same*
        // `ContentModel` value (same address) when their shape is identical.
        let r = registry();
        let mut seen_addrs: std::collections::BTreeSet<usize> = Default::default();
        let mut constructs_with_model = 0usize;
        for c in r.elements() {
            if let Some(cm) = c.content_model {
                constructs_with_model += 1;
                seen_addrs.insert(cm as *const ContentModel as usize);
            }
        }
        assert!(constructs_with_model > 0);
        assert!(
            seen_addrs.len() < constructs_with_model,
            "expected sharing: {} distinct addresses for {} constructs with a model",
            seen_addrs.len(),
            constructs_with_model
        );
    }

    #[test]
    fn not_handled_is_the_gap_join() {
        let r = registry();
        let gaps: Vec<_> = r
            .not_handled(ConstructKind::Element, ["sec", "p"])
            .map(|c| c.name)
            .collect();
        assert!(!gaps.contains(&"sec"));
        assert!(!gaps.contains(&"p"));
        assert!(gaps.contains(&"article"));
    }
}
