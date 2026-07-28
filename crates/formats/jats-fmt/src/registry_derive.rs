//! Owned, `serde_json`-serializable model of the construct registry, and the
//! codegen that turns it into the committed `src/registry_generated.rs`.
//!
//! This module exists only behind the `registry-derive` feature: it is the
//! *tool-side* representation, used while walking the JATS schema and while
//! reading/writing the human-readable source document
//! (`registry/jats-1.3-archiving.json`). It is deliberately a separate set of
//! types from [`crate::registry`]'s `&'static`-everything runtime API —
//! building a registry from a schema walk needs owned `String`/`Vec` data;
//! *using* a built registry at runtime needs none of that, which is the
//! entire reason this crate ships committed generated Rust statics instead of
//! parsing this JSON at runtime (see `crate::registry`'s module docs).
//!
//! # Why JSON, not YAML
//!
//! The registry previously committed its human-readable source as YAML,
//! parsed at runtime via `serde_yaml`. Two independent problems with that:
//!
//! 1. `serde_yaml` was archived by its author 2024-03-25; the final release
//!    is versioned `0.9.34+deprecated`. It receives no further updates,
//!    including security fixes. No successor fork (`serde_yaml_ng`,
//!    `serde_norway`, the YAML organization's `yaml_serde`) has a comparable
//!    track record yet — each is a young project that could itself stall the
//!    way the original did.
//! 2. Runtime YAML parsing was never actually required once the registry
//!    stops being parsed at runtime at all: this crate's `registry` feature
//!    now exposes committed generated Rust statics (see `crate::registry`),
//!    so no format parser of any kind ships in the compiled artifact. Only
//!    the offline derivation tool needs to read/write the human-readable
//!    source, and that tool is not part of any downstream consumer's
//!    dependency graph.
//!
//! Given that only an offline, dev-only tool needs to parse the source at
//! all, JSON via `serde_json` was chosen over continuing with YAML (even a
//! maintained fork): `serde_json` is a mature, actively maintained crate
//! already present in this workspace (used by `rescribe query`), with a
//! multi-year track record none of the YAML successors can yet match, and
//! JSON parses identically across every language's tooling — arguably more
//! uniformly than YAML, whose parsers are notorious for disagreeing on edge
//! cases across ecosystems. The cost is JSON's lack of comments and its
//! more verbose punctuation; since this file is generated output, never
//! hand-edited prose, that cost is small. Pretty-printed JSON (one array
//! entry per line, `serde_json`'s default `to_string_pretty` shape) is kept
//! rather than compacted to single-line arrays: a one-line-per-entry format
//! means adding or removing one child/attribute is a one-line diff, which
//! matters more for a file whose whole purpose is reviewable regeneration
//! diffs than shaving bytes off a file no one hand-edits.
//!
//! # Two independent drift checks
//!
//! - **Schema → source.** Needs the JATS schema fetched locally
//!   (`scripts/jats/download-spec.sh`); re-derives the registry from the RNG
//!   and diffs against the committed JSON. See `derive-registry --check`.
//! - **Source → generated.** Needs only the committed JSON — no schema
//!   required. Reads `registry/jats-1.3-archiving.json`, regenerates
//!   `src/registry_generated.rs`'s text, and diffs against the committed
//!   file. Cheap enough to run as an ordinary `cargo test` (see
//!   `drift_tests` below), so it runs in CI without anyone needing the
//!   upstream schema.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::registry::{ConstructKind, SourceKind};

/// Owned mirror of [`crate::registry::FormatInfo`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormatInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_name: Option<String>,
}

/// Owned mirror of [`crate::registry::SourceDigest`].
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SourceDigest {
    pub file: String,
    pub bytes: u64,
    pub sha256: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub url: String,
}

/// Owned mirror of [`crate::registry::Provenance`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    pub spec: String,
    pub source_kind: SourceKind,
    pub source_driver: String,
    pub source_base_url: String,
    pub source_license: String,
    pub source_redistributable: bool,
    pub source_vendored: bool,
    pub derived_on: String,
    pub derived_by: String,
    #[serde(default)]
    pub source_digests: Vec<SourceDigest>,
}

/// Owned mirror of [`crate::registry::Citation`].
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Citation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub element_url_template: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribute_url_template: Option<String>,
}

/// Owned mirror of [`crate::registry::Slice`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Slice {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub source_file: String,
    #[serde(default)]
    pub url: String,
}

/// Owned mirror of [`crate::registry::PermittedChild`].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PermittedChild {
    pub name: String,
    #[serde(default)]
    pub repeatable: bool,
}

/// Owned mirror of [`crate::registry::PermittedAttribute`].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PermittedAttribute {
    pub name: String,
    #[serde(default)]
    pub required: bool,
}

/// Owned mirror of [`crate::registry::ContentModel`].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub struct ContentModel {
    #[serde(default)]
    pub children: Vec<PermittedChild>,
    #[serde(default)]
    pub attributes: Vec<PermittedAttribute>,
    #[serde(default)]
    pub mixed: bool,
}

/// Owned mirror of [`crate::registry::Construct`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Construct {
    pub id: String,
    pub name: String,
    pub kind: ConstructKind,
    #[serde(default)]
    pub normative_slices: Vec<String>,
    #[serde(default)]
    pub pragmatic_slices: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_model: Option<ContentModel>,
}

/// Owned mirror of [`crate::registry::Registry`]: what the derivation tool
/// builds from the schema walk, and what `registry/jats-1.3-archiving.json`
/// serializes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Registry {
    pub registry_version: u32,
    pub format: FormatInfo,
    pub provenance: Provenance,
    #[serde(default)]
    pub citation: Citation,
    #[serde(default)]
    pub normative_slices: Vec<Slice>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normative_slices_absent_reason: Option<String>,
    #[serde(default)]
    pub pragmatic_slices: Vec<Slice>,
    pub constructs: Vec<Construct>,
}

impl Registry {
    /// Parse the owned model from the committed JSON source.
    pub fn from_json(src: &str) -> Result<Registry, serde_json::Error> {
        serde_json::from_str(src)
    }

    /// Serialize to the pretty-printed JSON form committed as the
    /// human-readable source. See the module docs for why pretty (one entry
    /// per line) rather than compacted.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

/// Emit the committed `src/registry_generated.rs` text for a registry model,
/// deduplicating identical [`ContentModel`]s into shared named statics.
///
/// This is the only place that decides the generated file's shape, so both
/// `derive-registry` (regenerating from a live schema walk) and the
/// source→generated drift test (regenerating from the committed JSON alone)
/// go through the same code path and cannot disagree by construction.
pub fn emit_rust(reg: &Registry) -> String {
    let mut out = String::new();
    out.push_str(
        "// GENERATED — do not edit by hand.\n\
         //\n\
         // Derived from `registry/jats-1.3-archiving.json` by\n\
         // `jats-fmt`'s `registry-derive` codegen (`src/registry_derive.rs::emit_rust`).\n\
         // Regenerate:\n\
         //   cargo run -p jats-fmt --features registry-derive --bin derive-registry -- \\\n\
         //       --schema-dir spec/jats-1.3-archiving-rng\n\
         // Verify without the schema (source JSON -> this file only):\n\
         //   cargo test -p jats-fmt --features registry-derive registry_derive::drift_tests\n\
         // Verify against a live schema fetch (schema -> source JSON):\n\
         //   … --schema-dir <dir> --check\n\
         #![allow(clippy::all)]\n\n\
         use crate::registry::{\n    \
             Citation, Construct, ConstructKind, ContentModel, FormatInfo, PermittedAttribute,\n    \
             PermittedChild, Provenance, Registry, Slice, SourceDigest, SourceKind,\n\
         };\n\n",
    );

    // --- Deduplicated content models -----------------------------------
    // Key on the model's own data (children+attributes+mixed), not on which
    // construct it belongs to, so two constructs with an identical shape
    // share one static regardless of name.
    let mut model_names: BTreeMap<ModelKey, String> = BTreeMap::new();
    let mut ordered_models: Vec<(&ContentModel, String)> = Vec::new();
    for c in &reg.constructs {
        let Some(cm) = &c.content_model else { continue };
        let key = ModelKey::from(cm);
        if !model_names.contains_key(&key) {
            let idx = model_names.len();
            let ident = format!("CM_{idx}");
            model_names.insert(key, ident.clone());
            ordered_models.push((cm, ident));
        }
    }

    for (cm, ident) in &ordered_models {
        if !cm.children.is_empty() {
            out.push_str(&format!(
                "static {ident}_CHILDREN: &[PermittedChild] = &[\n"
            ));
            for ch in &cm.children {
                out.push_str(&format!(
                    "    PermittedChild {{ name: {}, repeatable: {} }},\n",
                    rust_str(&ch.name),
                    ch.repeatable
                ));
            }
            out.push_str("];\n");
        }
        if !cm.attributes.is_empty() {
            out.push_str(&format!(
                "static {ident}_ATTRS: &[PermittedAttribute] = &[\n"
            ));
            for a in &cm.attributes {
                out.push_str(&format!(
                    "    PermittedAttribute {{ name: {}, required: {} }},\n",
                    rust_str(&a.name),
                    a.required
                ));
            }
            out.push_str("];\n");
        }
        let children_expr = if cm.children.is_empty() {
            "&[]".to_string()
        } else {
            format!("{ident}_CHILDREN")
        };
        let attrs_expr = if cm.attributes.is_empty() {
            "&[]".to_string()
        } else {
            format!("{ident}_ATTRS")
        };
        out.push_str(&format!(
            "static {ident}: ContentModel = ContentModel {{ children: {children_expr}, attributes: {attrs_expr}, mixed: {} }};\n\n",
            cm.mixed
        ));
    }

    // --- Slices -----------------------------------------------------------
    emit_slices(&mut out, "NORMATIVE_SLICES", &reg.normative_slices);
    emit_slices(&mut out, "PRAGMATIC_SLICES", &reg.pragmatic_slices);

    // --- Source digests -----------------------------------------------------
    out.push_str("static SOURCE_DIGESTS: &[SourceDigest] = &[\n");
    for d in &reg.provenance.source_digests {
        out.push_str(&format!(
            "    SourceDigest {{ file: {}, bytes: {}, sha256: {}, url: {} }},\n",
            rust_str(&d.file),
            d.bytes,
            rust_str(&d.sha256),
            rust_str(&d.url)
        ));
    }
    out.push_str("];\n\n");

    // --- Constructs ---------------------------------------------------------
    out.push_str("static CONSTRUCTS: &[Construct] = &[\n");
    for c in &reg.constructs {
        let kind = match c.kind {
            ConstructKind::Element => "ConstructKind::Element",
            ConstructKind::Attribute => "ConstructKind::Attribute",
        };
        let normative = rust_str_slice(&c.normative_slices);
        let pragmatic = rust_str_slice(&c.pragmatic_slices);
        let content_model = match &c.content_model {
            None => "None".to_string(),
            Some(cm) => {
                let ident = &model_names[&ModelKey::from(cm)];
                format!("Some(&{ident})")
            }
        };
        out.push_str(&format!(
            "    Construct {{ id: {}, name: {}, kind: {kind}, normative_slices: {normative}, pragmatic_slices: {pragmatic}, content_model: {content_model} }},\n",
            rust_str(&c.id),
            rust_str(&c.name),
        ));
    }
    out.push_str("];\n\n");

    // --- Registry -------------------------------------------------------
    let profile = rust_opt_str(reg.format.profile.as_deref());
    let profile_name = rust_opt_str(reg.format.profile_name.as_deref());
    let source_kind = match reg.provenance.source_kind {
        SourceKind::Relaxng => "SourceKind::Relaxng",
        SourceKind::Rnc => "SourceKind::Rnc",
        SourceKind::Dtd => "SourceKind::Dtd",
        SourceKind::Xsd => "SourceKind::Xsd",
        SourceKind::Odd => "SourceKind::Odd",
        SourceKind::ScriptedExtraction => "SourceKind::ScriptedExtraction",
    };
    let element_tpl = rust_opt_str(reg.citation.element_url_template.as_deref());
    let attribute_tpl = rust_opt_str(reg.citation.attribute_url_template.as_deref());
    let absent_reason = rust_opt_str(reg.normative_slices_absent_reason.as_deref());

    out.push_str(&format!(
        "pub(crate) static REGISTRY: Registry = Registry {{\n    \
            registry_version: {},\n    \
            format: FormatInfo {{ id: {}, name: {}, version: {}, profile: {profile}, profile_name: {profile_name} }},\n    \
            provenance: Provenance {{\n        \
                spec: {},\n        \
                source_kind: {source_kind},\n        \
                source_driver: {},\n        \
                source_base_url: {},\n        \
                source_license: {},\n        \
                source_redistributable: {},\n        \
                source_vendored: {},\n        \
                derived_on: {},\n        \
                derived_by: {},\n        \
                source_digests: SOURCE_DIGESTS,\n    \
            }},\n    \
            citation: Citation {{ element_url_template: {element_tpl}, attribute_url_template: {attribute_tpl} }},\n    \
            normative_slices: NORMATIVE_SLICES,\n    \
            normative_slices_absent_reason: {absent_reason},\n    \
            pragmatic_slices: PRAGMATIC_SLICES,\n    \
            constructs: CONSTRUCTS,\n\
         }};\n",
        reg.registry_version,
        rust_str(&reg.format.id),
        rust_str(&reg.format.name),
        rust_str(&reg.format.version),
        rust_str(&reg.provenance.spec),
        rust_str(&reg.provenance.source_driver),
        rust_str(&reg.provenance.source_base_url),
        rust_str(&reg.provenance.source_license),
        reg.provenance.source_redistributable,
        reg.provenance.source_vendored,
        rust_str(&reg.provenance.derived_on),
        rust_str(&reg.provenance.derived_by),
    ));

    out
}

fn emit_slices(out: &mut String, ident: &str, slices: &[Slice]) {
    out.push_str(&format!("static {ident}: &[Slice] = &[\n"));
    for s in slices {
        out.push_str(&format!(
            "    Slice {{ id: {}, name: {}, source_file: {}, url: {} }},\n",
            rust_str(&s.id),
            rust_str(&s.name),
            rust_str(&s.source_file),
            rust_str(&s.url)
        ));
    }
    out.push_str("];\n\n");
}

/// The subset of `ContentModel` that determines dedup identity.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ModelKey(Vec<PermittedChild>, Vec<PermittedAttribute>, bool);

impl From<&ContentModel> for ModelKey {
    fn from(cm: &ContentModel) -> Self {
        ModelKey(cm.children.clone(), cm.attributes.clone(), cm.mixed)
    }
}

/// A Rust string literal for `s`, using `format!("{s:?}")`'s escaping (valid
/// Rust string-literal syntax for any `&str`).
fn rust_str(s: &str) -> String {
    format!("{s:?}")
}

fn rust_opt_str(s: Option<&str>) -> String {
    match s {
        Some(s) => format!("Some({})", rust_str(s)),
        None => "None".to_string(),
    }
}

fn rust_str_slice(items: &[String]) -> String {
    if items.is_empty() {
        return "&[]".to_string();
    }
    let inner = items
        .iter()
        .map(|s| rust_str(s))
        .collect::<Vec<_>>()
        .join(", ");
    format!("&[{inner}]")
}

#[cfg(test)]
mod drift_tests {
    use super::*;

    const COMMITTED_JSON: &str = include_str!("../registry/jats-1.3-archiving.json");
    const COMMITTED_GENERATED: &str = include_str!("registry_generated.rs");

    /// Source → generated drift check. Needs only the committed JSON — no
    /// upstream schema required — so this runs as an ordinary test, in CI,
    /// for every contributor, unlike the schema → source check
    /// (`derive-registry --check`), which only a developer holding a local
    /// copy of the JATS schema can run.
    #[test]
    fn generated_rust_matches_committed_source() {
        let model = Registry::from_json(COMMITTED_JSON)
            .expect("committed registry/jats-1.3-archiving.json must parse");
        let regenerated = emit_rust(&model);
        assert_eq!(
            regenerated, COMMITTED_GENERATED,
            "src/registry_generated.rs does not match what regenerating from \
             registry/jats-1.3-archiving.json produces — regenerate it: \
             cargo run -p jats-fmt --features registry-derive --bin derive-registry -- \
             --emit-rust-only"
        );
    }

    /// The committed JSON must actually round-trip through the owned model
    /// (i.e. `Registry::to_json` . `Registry::from_json` is the identity),
    /// so a future hand-edit of the JSON that the model can't represent is
    /// caught here rather than silently accepted.
    #[test]
    fn committed_source_round_trips() {
        let model = Registry::from_json(COMMITTED_JSON).expect("must parse");
        let rewritten = model.to_json().expect("must serialize");
        let reparsed = Registry::from_json(&rewritten).expect("must reparse");
        assert_eq!(model, reparsed);
    }
}
