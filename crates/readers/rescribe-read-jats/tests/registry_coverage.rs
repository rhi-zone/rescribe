//! Coverage gap check: what JATS defines, minus what this reader mentions.
//!
//! This is the consumer that justifies the registry's existence. A hand-written
//! `COVERAGE.md` checklist can only report a ratio against *itself* — the
//! 2026-07-28 audit found JATS's checklist was missing 216 element names, so
//! "108/109 covered" was measuring the list, not the format. Here the
//! denominator comes from the JATS DTD Suite's own RELAX NG schema, so the
//! ratio cannot drift from what JATS actually defines.
//!
//! # Why this reads the reader's source text
//!
//! The registry supplies the denominator; the numerator is "element names the
//! reader mentions at all." There is no runtime way to enumerate a `match`'s
//! arms, so the numerator is extracted from the source text. That is a
//! deliberately *loose* numerator — a name counts as handled if it appears as
//! a string literal anywhere in the reader, whether in `convert_element`, in
//! `is_block_element`, or in a comment. So this test **understates** the gap:
//! everything it reports is definitely unhandled, but a name it considers
//! handled might only be mentioned in passing.
//!
//! That asymmetry is the right one. The test's job is to make gaps
//! impossible to miss, not to grade the reader — and an understated gap that
//! never produces a false alarm is one a maintainer will keep trusting.
//!
//! Support status deliberately lives here rather than in the registry: the
//! registry stays spec-pure so it never churns when reader work lands, and
//! "do we support X" is this join, computed fresh.

use std::collections::BTreeSet;

use jats_fmt::registry::registry;

const READER_SRC: &str = include_str!("../src/lib.rs");

/// Slices whose constructs are not JATS's own vocabulary. JATS embeds MathML
/// and XHTML tables by reference, so their elements are legitimately in the
/// registry — but MathML is raw-preserved wholesale rather than mapped
/// element-by-element, so counting ~180 MathML tags as reader gaps would bury
/// the JATS gaps this test exists to surface.
/// (XHTML's table modules are *not* foreign: `<table>`/`<tr>`/`<td>` are
/// documented JATS elements the reader is expected to map.)
const FOREIGN_SLICES: &[&str] = &["mathml2", "mathml2-qname-1.mod"];

/// Every string literal in the reader source, as a crude "is this name
/// mentioned" oracle. See the module docs for why loose is correct here.
fn names_mentioned_by_reader() -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let bytes = READER_SRC.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'"' {
            let start = i + 1;
            let mut j = start;
            while j < bytes.len() && bytes[j] != b'"' {
                if bytes[j] == b'\\' {
                    j += 1;
                }
                j += 1;
            }
            if j <= bytes.len()
                && let Some(s) = READER_SRC.get(start..j)
            {
                out.insert(s.to_string());
            }
            i = j + 1;
        } else {
            i += 1;
        }
    }
    // Comments mention element names in prose too; treating a documented
    // element as "mentioned" keeps this test from flagging deliberate,
    // recorded decisions as fresh discoveries.
    for tok in READER_SRC.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == ':')) {
        if tok.len() > 1 {
            out.insert(tok.to_string());
        }
    }
    out
}

fn jats_elements() -> Vec<&'static jats_fmt::registry::Construct> {
    registry()
        .elements()
        .filter(|c| {
            !c.normative_slices
                .iter()
                .all(|s| FOREIGN_SLICES.contains(&s.as_str()))
        })
        .collect()
}

/// The registry must actually describe JATS Archiving, not a stale or
/// truncated derivation. If this fails, every other assertion here is
/// measuring against the wrong denominator.
#[test]
fn denominator_is_plausible() {
    let r = registry();
    let n = jats_elements().len();
    assert!(
        (280..=340).contains(&n),
        "JATS Archiving defines ~306 elements; registry yields {n}. \
         Either the derivation is broken or the spec version changed — \
         re-run derive-registry --check before trusting any coverage number."
    );
    assert_eq!(r.provenance.spec, "ANSI/NISO Z39.96-2021 (JATS 1.3)");
    assert!(!r.provenance.source_vendored, "schema is not vendored here");
}

/// The gap query itself. Reports rather than fails: the point is a trustworthy
/// number in front of a maintainer, and a hard failure would just get the
/// threshold bumped.
#[test]
fn report_unmentioned_elements() {
    let mentioned = names_mentioned_by_reader();
    let all = jats_elements();
    let gaps: Vec<_> = all
        .iter()
        .filter(|c| !mentioned.contains(&c.name))
        .collect();

    println!(
        "\nJATS Archiving elements (excluding embedded MathML): {}\n\
         mentioned anywhere in rescribe-read-jats:            {}\n\
         never mentioned:                                     {}\n",
        all.len(),
        all.len() - gaps.len(),
        gaps.len()
    );
    if !gaps.is_empty() {
        println!("Unmentioned, by normative slice:");
        let mut by_slice: std::collections::BTreeMap<&str, Vec<&str>> = Default::default();
        for c in &gaps {
            by_slice
                .entry(c.primary_normative_slice().unwrap_or("(none)"))
                .or_default()
                .push(&c.name);
        }
        for (slice, names) in by_slice {
            let label = registry()
                .normative_slice(slice)
                .map(|s| s.name.as_str())
                .unwrap_or(slice);
            println!("  {label} ({slice}): {}", names.join(", "));
        }
        println!(
            "\nCite any of these as {}",
            registry()
                .citation
                .element_url_template
                .as_deref()
                .unwrap_or("(no citation template)")
        );
    }
}

/// Regression guard for the audit that motivated the registry.
///
/// The 2026-07-28 COVERAGE.md audit named specific elements JATS defines that
/// the checklist had never enumerated. Those names must be in the registry —
/// if a future derivation stops producing them, the registry has silently
/// regressed to the failure mode it was built to prevent.
#[test]
fn registry_contains_the_elements_the_hand_written_checklist_missed() {
    let r = registry();
    // From TODO.md's "DocBook and JATS COVERAGE.md audited" entry.
    let audit_findings = [
        "hr",
        "sub-article",
        "response",
        "ruby",
        "rb",
        "rt",
        "rp",
        "chem-struct",
        "array",
        "index-term",
        "media",
        "alt-text",
    ];
    let missing: Vec<_> = audit_findings
        .iter()
        .filter(|n| !r.contains_element(n))
        .collect();
    assert!(
        missing.is_empty(),
        "registry no longer knows elements the 2026-07-28 audit found by hand: {missing:?}"
    );
}

/// Normative slices must be usable as a work-decomposition axis: a
/// maintainer asking "what's left in the references module" must get a real
/// answer. JATS's normative modularization is never empty (unlike DocBook's
/// would be), so every element must land in at least one.
#[test]
fn normative_slices_partition_the_format() {
    let r = registry();
    assert!(
        r.normative_slices().len() > 10,
        "suspiciously few normative slices"
    );
    for slice in r.normative_slices() {
        let n = r.in_normative_slice(&slice.id).count();
        println!("{:>34}  {:>4}  {}", slice.id, n, slice.name);
    }
    // Every element belongs to at least one declared normative slice.
    for c in r.elements() {
        assert!(
            !c.normative_slices.is_empty(),
            "{} has no normative slice",
            c.id
        );
    }
}
