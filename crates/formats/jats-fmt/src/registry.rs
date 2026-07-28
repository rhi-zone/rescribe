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
//! through a RELAX NG validator, not this registry. Recording the full
//! pattern structure (nested choice/group/interleave trees mirroring the
//! source grammar) was considered and rejected for the pilot: it would
//! roughly double the derivation tool's complexity for a question the
//! flattened form doesn't need to answer, and JATS's `<define>`-based
//! customization layer means many patterns are shared/reused across dozens
//! of elements, so a literal per-element grammar tree would also be far more
//! repetitive on disk than the flattened form. Whether a richer,
//! validation-capable content-model representation is worth that cost is
//! left as an open question in `docs/adr/0013-per-format-construct-registry.md`
//! rather than decided here.
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

/// The form of the authoritative schema (or, for [`SourceKind::ScriptedExtraction`],
/// the published artifact) a registry was derived from.
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
    /// The format has no machine-readable schema, so the construct list was
    /// produced by a script that mechanically extracts it from a published
    /// prose artifact (e.g. an HTML element index), rather than by parsing a
    /// grammar.
    ///
    /// This is a first-class, fully permitted source kind — not a marked-down
    /// fallback — because the property this design actually needs is
    /// *reproducibility*, not "came from a schema." A scripted extraction is
    /// re-runnable, diffable against a fresh fetch, and auditable by reading
    /// the script; a hand-typed list is none of those, no matter how careful
    /// the person typing it was. See
    /// `docs/adr/0013-per-format-construct-registry.md`'s 2026-07-28
    /// hand-curation amendment.
    ///
    /// A registry with this `source_kind` must still carry everything
    /// [`Provenance`] requires: `source_base_url` (and, where the extraction
    /// spans several published pages, a `url` per entry in
    /// `source_digests`), `derived_on` as the retrieval date, `derived_by`
    /// naming the extraction script (path and version, e.g.
    /// `scripts/docbook/extract-element-index.py v1`), and a `sha256` +
    /// `bytes` digest of every fetched artifact in `source_digests` — the
    /// same fields a schema-derived registry carries, just describing a
    /// downloaded page instead of a downloaded schema file. Without those,
    /// re-running the extraction to check for drift is not actually
    /// possible, and the reproducibility claim is just asserted, not
    /// delivered.
    ScriptedExtraction,
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

/// A checksum of one source file (a schema module, or, for
/// [`SourceKind::ScriptedExtraction`], one fetched prose page) so staleness
/// is detectable even when the source itself is not present in the checkout.
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SourceDigest {
    /// File name as published, e.g. `JATS-section1-3.ent.rng`.
    pub file: String,
    /// Size in bytes at derivation time.
    pub bytes: u64,
    /// Lowercase hex SHA-256 of the file's bytes at derivation time.
    pub sha256: String,
    /// The exact URL this entry was fetched from, when it differs per entry
    /// rather than sharing `Provenance::source_base_url` (e.g. a scripted
    /// extraction spanning several distinct prose pages). Empty when
    /// `source_base_url` plus `file` already resolves it, which is the
    /// common case for schema-module digests.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub url: String,
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

/// One child element a construct's content model permits, flattened out of
/// whatever ordering/choice/group structure the source schema expressed it
/// with. See the module docs ("Content models: flattened, not full grammar")
/// for why this registry records a permitted-children *set* rather than the
/// full pattern structure.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermittedChild {
    /// Local element name.
    pub name: String,
    /// Whether the schema permits more than one occurrence of this child
    /// (reachable under a `zeroOrMore`/`oneOrMore` — or DTD `*`/`+` — without
    /// crossing into another element's own body first). `false` means the
    /// schema never allows more than one, though it says nothing about
    /// relative order, since order is exactly what flattening discards.
    #[serde(default)]
    pub repeatable: bool,
}

/// One attribute a construct's content model permits, with its
/// required/optional status *for this element* — the same attribute name can
/// be required on one element and optional on another, so this cannot live
/// on the global attribute [`Construct`] and is recorded per element instead.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermittedAttribute {
    /// Local attribute name.
    pub name: String,
    /// Whether the schema requires this attribute on every instance of the
    /// element (reached with no enclosing `optional`/`choice`). `false`
    /// covers both "optional" and "one of a choice of alternatives" — a
    /// choice member is never individually required.
    #[serde(default)]
    pub required: bool,
}

/// What a construct permits as content: which child elements, which
/// attributes, and whether character data may appear directly inside it.
/// Only populated for [`ConstructKind::Element`] — attributes have a value
/// type, not a content model, and this registry does not model datatypes.
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ContentModel {
    /// Every element name permitted as a direct child, in no particular
    /// order (order is not recorded — see the module docs).
    #[serde(default)]
    pub children: Vec<PermittedChild>,
    /// Every attribute name this element permits, with required/optional
    /// status.
    #[serde(default)]
    pub attributes: Vec<PermittedAttribute>,
    /// Whether character data (`#PCDATA` / RELAX NG `<text/>`/`<mixed>`) is
    /// permitted directly inside this element, alongside its children.
    #[serde(default)]
    pub mixed: bool,
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
    /// What this construct permits as content. `Some` for every
    /// [`ConstructKind::Element`] the schema actually defines a body for;
    /// `None` for [`ConstructKind::Attribute`] constructs and for any
    /// element the derivation could not resolve a model for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_model: Option<ContentModel>,
}

impl Construct {
    /// Does this element's content model permit `child` as a direct child?
    /// `None` if this construct has no recorded content model at all.
    pub fn permits_child(&self, child: &str) -> Option<bool> {
        self.content_model
            .as_ref()
            .map(|m| m.children.iter().any(|c| c.name == child))
    }

    /// Does this element's content model require `attr`? `Some(false)` also
    /// covers "permitted but optional"; distinguish via
    /// [`Construct::permits_attribute`] if needed. `None` if this construct
    /// has no recorded content model at all.
    pub fn requires_attribute(&self, attr: &str) -> Option<bool> {
        self.content_model
            .as_ref()
            .map(|m| m.attributes.iter().any(|a| a.name == attr && a.required))
    }

    /// Does this element's content model permit `attr` at all (required or
    /// optional)? `None` if this construct has no recorded content model.
    pub fn permits_attribute(&self, attr: &str) -> Option<bool> {
        self.content_model
            .as_ref()
            .map(|m| m.attributes.iter().any(|a| a.name == attr))
    }
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
    /// consumer can tell v1 from v2 from v3. `3` added
    /// [`Construct::content_model`] (`docs/adr/0013-...`'s 2026-07-28
    /// content-model amendment); `2` introduced the normative/pragmatic slice
    /// split (the same ADR's earlier 2026-07-28 slice amendment); `1` had a
    /// single `slices` field and no content models.
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
        assert_eq!(r.registry_version, 3);
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
    fn content_models_are_populated_for_elements() {
        let r = registry();
        let sec = r.lookup(ConstructKind::Element, "sec").unwrap();
        let model = sec.content_model.as_ref().expect("sec has a content model");
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
