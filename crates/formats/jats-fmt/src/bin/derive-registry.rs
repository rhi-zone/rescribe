//! Derive `registry/jats-1.3-archiving.json` (and, from it,
//! `src/registry_generated.rs`) from the JATS DTD Suite's published RELAX NG
//! schema.
//!
//! The schema is **not vendored** in this repository, so this tool is not part
//! of a normal build: a developer fetches the schema first
//! (`scripts/jats/download-spec.sh`), then runs this to regenerate or verify
//! the committed registry documents. Everyone else — CI, and every downstream
//! consumer of `jats-fmt` — uses the committed `src/registry_generated.rs`
//! directly and never needs the schema, or any JSON/YAML parser, at all.
//!
//! ```text
//! cargo run -p jats-fmt --features registry-derive --bin derive-registry -- \
//!     --schema-dir <dir> [--check]
//!
//! # Regenerate src/registry_generated.rs from the committed JSON alone,
//! # with no schema involved:
//! cargo run -p jats-fmt --features registry-derive --bin derive-registry -- \
//!     --emit-rust-only
//! ```
//!
//! `--check` re-derives from the schema and diffs against the committed JSON
//! source, exiting non-zero on drift — the "schema → source" check. The
//! separate "source → generated" check (does `src/registry_generated.rs`
//! actually match what regenerating from the committed JSON alone would
//! produce?) needs no schema at all and is wired in as an ordinary test:
//! `crate::registry_derive::drift_tests::generated_rust_matches_committed_source`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use jats_fmt::Node;
use jats_fmt::registry::{ConstructKind, SourceKind};
use jats_fmt::registry_derive::{
    Citation, Construct, ContentModel, FormatInfo, PermittedAttribute, PermittedChild, Provenance,
    Registry, Slice, SourceDigest, emit_rust,
};
use rescribe_format_api::Parse as _;
use sha2::{Digest, Sha256};

const DRIVER: &str = "JATS-archivearticle1-3.rng";
const BASE_URL: &str = "https://jats.nlm.nih.gov/archiving/1.3/rng/";
const TOOL: &str = "jats-fmt derive-registry v3";
const JSON_OUT_REL: &str = "registry/jats-1.3-archiving.json";
const RUST_OUT_REL: &str = "src/registry_generated.rs";

fn main() -> ExitCode {
    let mut schema_dir: Option<PathBuf> = None;
    let mut check = false;
    let mut emit_rust_only = false;
    let mut derived_on: Option<String> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--schema-dir" => schema_dir = args.next().map(PathBuf::from),
            "--check" => check = true,
            "--emit-rust-only" => emit_rust_only = true,
            "--derived-on" => derived_on = args.next(),
            other => {
                eprintln!("unknown argument: {other}");
                return ExitCode::FAILURE;
            }
        }
    }

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let json_out = manifest_dir.join(JSON_OUT_REL);
    let rust_out = manifest_dir.join(RUST_OUT_REL);

    if emit_rust_only {
        let committed_json = match std::fs::read_to_string(&json_out) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("{json_out:?}: {e}");
                return ExitCode::FAILURE;
            }
        };
        let model = match Registry::from_json(&committed_json) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("{JSON_OUT_REL} did not parse: {e}");
                return ExitCode::FAILURE;
            }
        };
        let rust = emit_rust(&model);
        if let Err(e) = std::fs::write(&rust_out, rust) {
            eprintln!("write failed: {e}");
            return ExitCode::FAILURE;
        }
        println!(
            "wrote {RUST_OUT_REL} from {JSON_OUT_REL} ({} constructs)",
            model.constructs.len()
        );
        return ExitCode::SUCCESS;
    }

    let Some(schema_dir) = schema_dir else {
        eprintln!("usage: derive-registry --schema-dir <dir> [--check] [--derived-on YYYY-MM-DD]");
        eprintln!("   or: derive-registry --emit-rust-only");
        eprintln!("fetch the schema first with scripts/jats/download-spec.sh");
        return ExitCode::FAILURE;
    };

    let reg = match derive(&schema_dir, derived_on) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("derivation failed: {e}");
            return ExitCode::FAILURE;
        }
    };

    if check {
        let committed = std::fs::read_to_string(&json_out).unwrap_or_default();
        // Compare the parsed documents, not the bytes: reformatting the
        // committed file by hand should not read as spec drift.
        let a = Registry::from_json(&committed).ok();
        if a.as_ref() == Some(&reg) {
            println!(
                "registry is up to date with {DRIVER} ({} constructs)",
                reg.constructs.len()
            );
            return ExitCode::SUCCESS;
        }
        eprintln!(
            "REGISTRY DRIFT: committed {JSON_OUT_REL} does not match the schema at {schema_dir:?}"
        );
        match a {
            None => eprintln!("  committed document is missing or unparseable"),
            Some(a) => report_drift(&a, &reg),
        }
        eprintln!("  re-run without --check to regenerate");
        return ExitCode::FAILURE;
    }

    let json = match reg.to_json() {
        Ok(j) => format!("{}\n", j),
        Err(e) => {
            eprintln!("serialization failed: {e}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(e) = std::fs::write(&json_out, &json) {
        eprintln!("write failed: {e}");
        return ExitCode::FAILURE;
    }
    let rust = emit_rust(&reg);
    if let Err(e) = std::fs::write(&rust_out, rust) {
        eprintln!("write failed: {e}");
        return ExitCode::FAILURE;
    }
    println!(
        "wrote {JSON_OUT_REL} and {RUST_OUT_REL}: {} constructs across {} normative slices",
        reg.constructs.len(),
        reg.normative_slices.len()
    );
    ExitCode::SUCCESS
}

fn report_drift(old: &Registry, new: &Registry) {
    let ids =
        |r: &Registry| -> BTreeSet<String> { r.constructs.iter().map(|c| c.id.clone()).collect() };
    let (a, b) = (ids(old), ids(new));
    for x in b.difference(&a) {
        eprintln!("  + {x} (in schema, missing from committed registry)");
    }
    for x in a.difference(&b) {
        eprintln!("  - {x} (in committed registry, absent from schema)");
    }
    if a == b {
        eprintln!("  construct set matches; metadata or slice assignment differs");
    }
}

fn derive(dir: &Path, derived_on: Option<String>) -> Result<Registry, String> {
    let driver_path = dir.join(DRIVER);
    let driver = std::fs::read(&driver_path).map_err(|e| format!("{driver_path:?}: {e}"))?;

    // Module order comes from the driver's own <include> list, so the primary
    // slice of a multiply-declared construct is defined by the format, not by
    // this tool's traversal order.
    let (driver_doc, diags) = jats_fmt::JatsDoc::parse(&driver);
    if !diags.is_empty() {
        return Err(format!("{DRIVER} did not parse cleanly: {diags:?}"));
    }
    // Resolved transitively: the tag set embeds XHTML tables and MathML by
    // reference (`JATS-XHTMLtablesetup` includes `xhtml-table-1.mod.rng`,
    // `JATS-mathmlsetup` includes `mathml2.rng`), and `<table>`/`<tr>`/`<td>`
    // and the MathML vocabulary are constructs JATS documents genuinely use.
    // Walking only the driver's direct includes silently loses all of them.
    let mut module_files = Vec::new();
    let mut frontier: Vec<String> = Vec::new();
    collect_includes(&driver_doc.nodes, &mut frontier);
    if frontier.is_empty() {
        return Err(format!("{DRIVER} declares no <include href=…> modules"));
    }
    while let Some(file) = frontier.first().cloned() {
        frontier.remove(0);
        if module_files.contains(&file) {
            continue;
        }
        module_files.push(file.clone());
        let path = dir.join(&file);
        let bytes = std::fs::read(&path).map_err(|e| {
            format!("{path:?}: {e} (re-run scripts/jats/download-spec.sh — it resolves includes transitively)")
        })?;
        let (doc, diags) = jats_fmt::JatsDoc::parse(&bytes);
        if !diags.is_empty() {
            return Err(format!("{file} did not parse cleanly: {diags:?}"));
        }
        let mut nested = Vec::new();
        collect_includes(&doc.nodes, &mut nested);
        frontier.extend(nested);
    }

    let mut digests = vec![digest_of(DRIVER, &driver)];
    let mut normative_slices = Vec::new();
    // name -> (kind, normative slice ids in include order)
    let mut found: BTreeMap<(ConstructKind, String), Vec<String>> = BTreeMap::new();
    // Every parsed document's nodes, retained for the second, content-model
    // resolution pass below: <define> bodies and <element> bodies can (and
    // routinely do, via JATS's customization-layer <ref>s) live in a
    // different module than the <element name="…"> declaration that uses
    // them, so resolution needs the whole schema assembled first.
    let mut all_docs: Vec<Vec<Node>> = vec![driver_doc.nodes.clone()];

    // The driver itself declares constructs too (notably <article>).
    let driver_slice = DRIVER.to_string();
    normative_slices.push(Slice {
        id: driver_slice.clone(),
        name: module_name(&driver_doc.nodes).unwrap_or_else(|| "Archiving Driver".into()),
        source_file: DRIVER.to_string(),
        url: format!("{BASE_URL}{DRIVER}"),
    });
    collect_constructs(&driver_doc.nodes, &driver_slice, &mut found);

    for file in &module_files {
        let path = dir.join(file);
        let bytes = std::fs::read(&path).map_err(|e| format!("{path:?}: {e}"))?;
        let (doc, diags) = jats_fmt::JatsDoc::parse(&bytes);
        if !diags.is_empty() {
            return Err(format!("{file} did not parse cleanly: {diags:?}"));
        }
        digests.push(digest_of(file, &bytes));
        // Slice id drops the `.rng` mirror suffix: the module's identity is
        // the DTD-suite module name, which the RNG is a 1:1 mirror of.
        let id = file.strip_suffix(".rng").unwrap_or(file).to_string();
        normative_slices.push(Slice {
            id: id.clone(),
            name: module_name(&doc.nodes).unwrap_or_else(|| id.clone()),
            source_file: file.clone(),
            url: format!("{BASE_URL}{file}"),
        });
        collect_constructs(&doc.nodes, &id, &mut found);
        all_docs.push(doc.nodes);
    }

    // Content-model resolution: build a global `<define name="…">` table and
    // a global `<element name="…">` body table across every parsed module,
    // then resolve each element's body against the define table. See
    // `resolve_content_model`'s doc comment for the flattening rules.
    let mut defines: BTreeMap<String, Vec<Node>> = BTreeMap::new();
    let mut element_bodies: BTreeMap<String, Vec<Node>> = BTreeMap::new();
    for doc in &all_docs {
        collect_defines(doc, &mut defines);
        collect_element_bodies(doc, &mut element_bodies);
    }

    let mut constructs: Vec<Construct> = found
        .into_iter()
        .map(|((kind, name), normative_slices)| {
            let content_model = match kind {
                ConstructKind::Element => element_bodies
                    .get(&name)
                    .map(|body| resolve_content_model(body, &defines)),
                ConstructKind::Attribute => None,
            };
            Construct {
                id: format!("{}:{}", kind.id_prefix(), name),
                name,
                kind,
                normative_slices,
                // JATS's normative modularization already does the decomposition
                // job; this derivation tool has no basis for inventing a second,
                // pragmatic grouping, so it leaves this empty for every
                // construct rather than guessing one.
                pragmatic_slices: Vec::new(),
                content_model,
            }
        })
        .collect();
    constructs.sort_by(|a, b| a.id.cmp(&b.id));

    Ok(Registry {
        registry_version: 3,
        format: FormatInfo {
            id: "jats".into(),
            name: "JATS (Journal Article Tag Suite)".into(),
            version: "1.3".into(),
            profile: Some("archiving".into()),
            profile_name: Some("Archiving and Interchange Tag Set (green)".into()),
        },
        provenance: Provenance {
            spec: "ANSI/NISO Z39.96-2021 (JATS 1.3)".into(),
            source_kind: SourceKind::Relaxng,
            source_driver: DRIVER.into(),
            source_base_url: BASE_URL.into(),
            source_license: "Public domain (per the JATS DTD Suite module headers). Verbatim \
                 redistribution permitted; the headers additionally state \"Do not modify \
                 the suite directly or redistribute modified versions of the suite.\""
                .into(),
            source_redistributable: true,
            source_vendored: false,
            derived_on: derived_on.unwrap_or_else(|| "unknown".into()),
            derived_by: TOOL.into(),
            source_digests: digests,
        },
        citation: Citation {
            element_url_template: Some(
                "https://jats.nlm.nih.gov/archiving/tag-library/1.3/element/{name}.html".into(),
            ),
            attribute_url_template: Some(
                "https://jats.nlm.nih.gov/archiving/tag-library/1.3/attribute/{name}.html".into(),
            ),
        },
        normative_slices,
        normative_slices_absent_reason: None,
        // JATS's own DTD Suite modularization already provides the
        // decomposition axis; the pilot does not invent a second, pragmatic
        // partition on top of it (see the comment above).
        pragmatic_slices: Vec::new(),
        constructs,
    })
}

/// `<define name="…">…</define>` bodies, keyed by name, across every parsed
/// module. Multiple modules may legitimately declare the same name (JATS's
/// customization layer splits several definitions across files with
/// `combine="interleave"`/`"choice"`); for the *flattened* content model this
/// registry records, that distinction doesn't matter — every occurrence's
/// children are simply unioned into the same bucket.
fn collect_defines(nodes: &[Node], out: &mut BTreeMap<String, Vec<Node>>) {
    for n in nodes {
        if let Node::Element {
            name,
            attrs,
            children,
            ..
        } = n
        {
            let local = name.rsplit(':').next().unwrap_or(name);
            if local == "define"
                && let Some((_, dname)) = attrs.iter().find(|(k, _)| k == "name")
            {
                out.entry(dname.clone())
                    .or_default()
                    .extend(children.clone());
            }
            collect_defines(children, out);
        }
    }
}

/// `<element name="…">…</element>` bodies, keyed by local element name,
/// across every parsed module. A body mixes the element's attribute-list
/// `<ref>` and its content-model `<ref>` side by side (e.g. `<element
/// name="sec"><ref name="sec-attlist"/><ref name="sec-model"/></element>`),
/// so resolving the whole body in one pass picks up both children and
/// attributes without needing to guess at a `-model`/`-attlist` naming
/// convention the schema does not actually guarantee everywhere.
fn collect_element_bodies(nodes: &[Node], out: &mut BTreeMap<String, Vec<Node>>) {
    for n in nodes {
        if let Node::Element {
            name,
            attrs,
            children,
            ..
        } = n
        {
            let local = name.rsplit(':').next().unwrap_or(name);
            if local == "element"
                && let Some((_, ename)) = attrs.iter().find(|(k, _)| k == "name")
            {
                out.entry(ename.clone())
                    .or_default()
                    .extend(children.clone());
            }
            collect_element_bodies(children, out);
        }
    }
}

/// Resolve one element's `<element>` body into a flattened [`ContentModel`]:
/// every permitted direct child element and attribute, whether each is
/// repeatable/required, and whether character data is permitted. Walks
/// `<ref>` through the global `defines` table, transparently descends through
/// `<choice>`/`<group>`/`<interleave>`/`<optional>`/`<zeroOrMore>`/
/// `<oneOrMore>`, and stops at each nested `<element>` boundary — a
/// grandchild element's own permitted content is *its* content model, not
/// this one's, which is exactly what keeps this a one-level, per-element
/// catalog rather than an unrolled document grammar.
///
/// Deliberately **not** recorded: relative order, which children exclude
/// which others (choice), and which children must co-occur (group/
/// interleave) — see the module docs on `jats_fmt::registry` for why the
/// flattened form was chosen over preserving the full pattern structure.
fn resolve_content_model(body: &[Node], defines: &BTreeMap<String, Vec<Node>>) -> ContentModel {
    let mut children: BTreeMap<String, bool> = BTreeMap::new();
    let mut attributes: BTreeMap<String, bool> = BTreeMap::new();
    let mut mixed = false;
    let mut visiting: BTreeSet<String> = BTreeSet::new();
    resolve_body(
        body,
        defines,
        &mut visiting,
        false,
        false,
        &mut children,
        &mut attributes,
        &mut mixed,
    );
    let mut children: Vec<PermittedChild> = children
        .into_iter()
        .map(|(name, repeatable)| PermittedChild { name, repeatable })
        .collect();
    children.sort_by(|a, b| a.name.cmp(&b.name));
    let mut attributes: Vec<PermittedAttribute> = attributes
        .into_iter()
        .map(|(name, required)| PermittedAttribute { name, required })
        .collect();
    attributes.sort_by(|a, b| a.name.cmp(&b.name));
    ContentModel {
        children,
        attributes,
        mixed,
    }
}

/// Recursive worker for [`resolve_content_model`].
///
/// `repeat_ctx` is true once the walk has passed under a `zeroOrMore`/
/// `oneOrMore`, so any `<element>` reached below is recorded as repeatable.
/// `constrained_ctx` is true once the walk has passed under an `optional` or
/// a `choice` (a choice member is never individually required), so any
/// `<attribute>` reached below is recorded as not required. `visiting` guards
/// against `<ref>` cycles: JATS's patterns bottom out at concrete
/// `<element>`/`<attribute>` declarations in practice, but the guard makes
/// that an enforced property of this resolver rather than an assumption
/// about the schema.
#[allow(clippy::too_many_arguments)]
fn resolve_body(
    nodes: &[Node],
    defines: &BTreeMap<String, Vec<Node>>,
    visiting: &mut BTreeSet<String>,
    repeat_ctx: bool,
    constrained_ctx: bool,
    children: &mut BTreeMap<String, bool>,
    attributes: &mut BTreeMap<String, bool>,
    mixed: &mut bool,
) {
    for n in nodes {
        let Node::Element {
            name,
            attrs,
            children: sub,
            ..
        } = n
        else {
            continue;
        };
        let local = name.rsplit(':').next().unwrap_or(name);
        match local {
            "element" => {
                if let Some((_, ename)) = attrs.iter().find(|(k, _)| k == "name") {
                    let entry = children.entry(ename.clone()).or_insert(false);
                    *entry = *entry || repeat_ctx;
                }
                // Do not descend: a nested element's own body is that
                // element's content model, not this one's.
            }
            "attribute" => {
                if let Some((_, aname)) = attrs.iter().find(|(k, _)| k == "name") {
                    let required = !constrained_ctx;
                    let entry = attributes.entry(aname.clone()).or_insert(required);
                    // Required only if every reachable occurrence says so.
                    *entry = *entry && required;
                }
            }
            "text" => *mixed = true,
            "mixed" => {
                *mixed = true;
                resolve_body(
                    sub,
                    defines,
                    visiting,
                    repeat_ctx,
                    constrained_ctx,
                    children,
                    attributes,
                    mixed,
                );
            }
            "ref" => {
                if let Some((_, target)) = attrs.iter().find(|(k, _)| k == "name")
                    && visiting.insert(target.clone())
                {
                    if let Some(def) = defines.get(target) {
                        resolve_body(
                            def,
                            defines,
                            visiting,
                            repeat_ctx,
                            constrained_ctx,
                            children,
                            attributes,
                            mixed,
                        );
                    }
                    visiting.remove(target);
                }
            }
            "optional" => resolve_body(
                sub, defines, visiting, repeat_ctx, true, children, attributes, mixed,
            ),
            "zeroOrMore" => resolve_body(
                sub, defines, visiting, true, true, children, attributes, mixed,
            ),
            "oneOrMore" => resolve_body(
                sub,
                defines,
                visiting,
                true,
                constrained_ctx,
                children,
                attributes,
                mixed,
            ),
            "choice" => resolve_body(
                sub, defines, visiting, repeat_ctx, true, children, attributes, mixed,
            ),
            "group" | "interleave" => resolve_body(
                sub,
                defines,
                visiting,
                repeat_ctx,
                constrained_ctx,
                children,
                attributes,
                mixed,
            ),
            // "empty", "notAllowed", "data", "value", "param", "name",
            // "anyName", "nsName", "except", "list" and anything else carry
            // no child-element/attribute/mixed information for this purpose.
            _ => {}
        }
    }
}

fn digest_of(file: &str, bytes: &[u8]) -> SourceDigest {
    let mut h = Sha256::new();
    h.update(bytes);
    SourceDigest {
        file: file.to_string(),
        bytes: bytes.len() as u64,
        sha256: format!("{:x}", h.finalize()),
        url: String::new(),
    }
}

/// `<include href="…"/>` targets, in document order, deduplicated.
fn collect_includes(nodes: &[jats_fmt::Node], out: &mut Vec<String>) {
    for n in nodes {
        if let jats_fmt::Node::Element {
            name,
            attrs,
            children,
            ..
        } = n
        {
            if (name == "include" || name.ends_with(":include"))
                && let Some((_, href)) = attrs.iter().find(|(k, _)| k == "href")
                && !out.contains(href)
            {
                out.push(href.clone());
            }
            collect_includes(children, out);
        }
    }
}

/// The module's self-declared name, from its leading `<!-- MODULE: … -->`
/// banner comment. This is the format naming its own partition; inventing a
/// name here would defeat the point of sourcing slices from the spec.
fn module_name(nodes: &[jats_fmt::Node]) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    let mut seen = false;
    for n in nodes {
        if let jats_fmt::Node::Comment { content, .. } = n {
            let t = content.trim();
            if let Some(rest) = t.strip_prefix("MODULE:") {
                seen = true;
                parts.push(rest.trim().to_string());
                continue;
            }
            if seen {
                // The banner wraps across several comment lines; keep taking
                // continuation lines until the next labelled field.
                if t.is_empty() || t.chars().all(|c| c == '=' || c == '-') {
                    break;
                }
                if t.contains(':') && t.split(':').next().is_some_and(|k| k == k.to_uppercase()) {
                    break;
                }
                parts.push(t.to_string());
                continue;
            }
        }
    }
    if parts.is_empty() {
        return None;
    }
    Some(
        parts
            .join(" ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" "),
    )
}

/// `<element name="…">` / `<attribute name="…">` declarations.
///
/// Only named declarations count as constructs: `<element>` with a
/// `<nsName>`/`<anyName>` name class is a wildcard, not a construct the
/// format defines.
fn collect_constructs(
    nodes: &[jats_fmt::Node],
    slice: &str,
    out: &mut BTreeMap<(ConstructKind, String), Vec<String>>,
) {
    for n in nodes {
        if let jats_fmt::Node::Element {
            name,
            attrs,
            children,
            ..
        } = n
        {
            let local = name.rsplit(':').next().unwrap_or(name);
            let kind = match local {
                "element" => Some(ConstructKind::Element),
                "attribute" => Some(ConstructKind::Attribute),
                _ => None,
            };
            if let Some(kind) = kind
                && let Some((_, decl)) = attrs.iter().find(|(k, _)| k == "name")
            {
                let entry = out.entry((kind, decl.clone())).or_default();
                if !entry.iter().any(|s| s == slice) {
                    entry.push(slice.to_string());
                }
            }
            collect_constructs(children, slice, out);
        }
    }
}
