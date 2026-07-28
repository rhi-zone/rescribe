//! Derive `registry/jats-1.3-archiving.yaml` from the JATS DTD Suite's
//! published RELAX NG schema.
//!
//! The schema is **not vendored** in this repository, so this tool is not part
//! of a normal build: a developer fetches the schema first
//! (`scripts/jats/download-spec.sh`), then runs this to regenerate or verify
//! the committed registry document. Everyone else — CI, and every downstream
//! consumer of `jats-fmt` — uses the committed YAML and never needs the
//! schema at all. That is why the registry is a committed derived artifact
//! rather than build-time codegen, and why provenance (checksums, dates,
//! canonical URLs) is load-bearing: it is the only way a reader without the
//! schema can judge whether the committed copy has gone stale.
//!
//! ```text
//! cargo run -p jats-fmt --features registry-derive --bin derive-registry -- \
//!     --schema-dir <dir> [--check]
//! ```
//!
//! `--check` re-derives and diffs against the committed document instead of
//! writing, exiting non-zero on drift. This is the verification mode: it is
//! how the "auditable, re-derivable denominator" promise survives the source
//! schema being absent for almost everyone.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use jats_fmt::registry::{
    Citation, Construct, ConstructKind, FormatInfo, Provenance, Registry, Slice, SourceDigest,
    SourceKind,
};
use sha2::{Digest, Sha256};

const DRIVER: &str = "JATS-archivearticle1-3.rng";
const BASE_URL: &str = "https://jats.nlm.nih.gov/archiving/1.3/rng/";
const TOOL: &str = "jats-fmt derive-registry v1";
const OUT_REL: &str = "registry/jats-1.3-archiving.yaml";

fn main() -> ExitCode {
    let mut schema_dir: Option<PathBuf> = None;
    let mut check = false;
    let mut derived_on: Option<String> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--schema-dir" => schema_dir = args.next().map(PathBuf::from),
            "--check" => check = true,
            "--derived-on" => derived_on = args.next(),
            other => {
                eprintln!("unknown argument: {other}");
                return ExitCode::FAILURE;
            }
        }
    }
    let Some(schema_dir) = schema_dir else {
        eprintln!("usage: derive-registry --schema-dir <dir> [--check] [--derived-on YYYY-MM-DD]");
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

    let yaml = match serde_yaml::to_string(&reg) {
        Ok(y) => format!("{}{y}", header()),
        Err(e) => {
            eprintln!("serialization failed: {e}");
            return ExitCode::FAILURE;
        }
    };

    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join(OUT_REL);
    if check {
        let committed = std::fs::read_to_string(&out).unwrap_or_default();
        // Compare the parsed documents, not the bytes: reformatting the
        // committed file by hand should not read as spec drift.
        let a = Registry::from_yaml(&committed).ok();
        if a.as_ref() == Some(&reg) {
            println!(
                "registry is up to date with {DRIVER} ({} constructs)",
                reg.constructs.len()
            );
            return ExitCode::SUCCESS;
        }
        eprintln!(
            "REGISTRY DRIFT: committed {OUT_REL} does not match the schema at {schema_dir:?}"
        );
        match a {
            None => eprintln!("  committed document is missing or unparseable"),
            Some(a) => report_drift(&a, &reg),
        }
        eprintln!("  re-run without --check to regenerate");
        return ExitCode::FAILURE;
    }

    if let Err(e) = std::fs::write(&out, yaml) {
        eprintln!("write failed: {e}");
        return ExitCode::FAILURE;
    }
    println!(
        "wrote {OUT_REL}: {} constructs across {} normative slices",
        reg.constructs.len(),
        reg.normative_slices.len()
    );
    ExitCode::SUCCESS
}

fn report_drift(old: &Registry, new: &Registry) {
    let ids = |r: &Registry| -> std::collections::BTreeSet<String> {
        r.constructs.iter().map(|c| c.id.clone()).collect()
    };
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

fn header() -> String {
    format!(
        "# GENERATED — do not edit by hand.\n\
         #\n\
         # Derived from the JATS DTD Suite RELAX NG schema by `{TOOL}`.\n\
         # Regenerate:\n\
         #   scripts/jats/download-spec.sh\n\
         #   cargo run -p jats-fmt --features registry-derive --bin derive-registry -- \\\n\
         #       --schema-dir spec/jats-1.3-archiving-rng\n\
         # Verify (fails on drift):  … --schema-dir <dir> --check\n\
         #\n\
         # The source schema is NOT vendored here; see the `provenance` block\n\
         # for the canonical URLs and per-file checksums that make staleness\n\
         # detectable without it.\n\n"
    )
}

fn derive(dir: &Path, derived_on: Option<String>) -> Result<Registry, String> {
    let driver_path = dir.join(DRIVER);
    let driver = std::fs::read(&driver_path).map_err(|e| format!("{driver_path:?}: {e}"))?;

    // Module order comes from the driver's own <include> list, so the primary
    // slice of a multiply-declared construct is defined by the format, not by
    // this tool's traversal order.
    let (driver_doc, diags) = jats_fmt::parse(&driver);
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
        let (doc, diags) = jats_fmt::parse(&bytes);
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
        let (doc, diags) = jats_fmt::parse(&bytes);
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
    }

    let mut constructs: Vec<Construct> = found
        .into_iter()
        .map(|((kind, name), normative_slices)| Construct {
            id: format!("{}:{}", kind.id_prefix(), name),
            name,
            kind,
            normative_slices,
            // JATS's normative modularization already does the decomposition
            // job; this derivation tool has no basis for inventing a second,
            // pragmatic grouping, so it leaves this empty for every
            // construct rather than guessing one (ADR 0013's 2026-07-28
            // amendment: an unasked-for pragmatic slice is noise, not value).
            pragmatic_slices: Vec::new(),
        })
        .collect();
    constructs.sort_by(|a, b| a.id.cmp(&b.id));

    Ok(Registry {
        registry_version: 2,
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

fn digest_of(file: &str, bytes: &[u8]) -> SourceDigest {
    let mut h = Sha256::new();
    h.update(bytes);
    SourceDigest {
        file: file.to_string(),
        bytes: bytes.len() as u64,
        sha256: format!("{:x}", h.finalize()),
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
